//! Edits: every change to the model is a named operation, applied to a copy and kept only when it
//! succeeds, so a failed edit leaves the model exactly as it was. Undo and redo keep the model as
//! it was before each operation.

use crate::catalogue::{Catalogue, NPS};
use crate::equipment;
use crate::model::{Area, End, EquipKind, Equipment, Item, ItemKind, Line, ListedLine, Plot, Project, SavedView, Settings, Term, P3};
use crate::parts::{add, dot, mul, norm, sub, unit};
use crate::route;
use serde::{Deserialize, Serialize};

/// One end of a new route.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum RouteEnd {
    Nozzle { equip: u32, nozzle: String },
    Branch { line: u32, item: u32 },
    Point { p: P3 },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Op {
    SetMeta { name: String, notes: String, plant: String, unit: String, units: String },
    SetPlot(Plot),
    SetSettings(Settings),
    /// A new equipment item of a kind, its base at `pos`, with the next free tag.
    AddEquipment { kind: EquipKind, pos: P3 },
    SetEquipment(Equipment),
    AddArea(Area),
    SetArea(Area),
    /// A new line routed orthogonally from one end through the via points to the other.
    Route { from: RouteEnd, via: Vec<P3>, to: RouteEnd, nps: Option<String> },
    SetLine(Line),
    /// Moves one straight run square to itself; the runs on either side stretch and the elbows follow.
    MoveRun { line: u32, seg: usize, delta: P3 },
    /// Moves one corner (index into the line's path): the runs on both sides stay parallel to
    /// themselves, so the route stays square. A corner next to a nozzle or tee only slides along
    /// the run that leaves it.
    MoveCorner { line: u32, corner: usize, delta: P3 },
    /// Offsets the part of run `seg` between `s0` and `s1` (metres from its start) by `delta`
    /// (square to the run): four new corners make a jog, e.g. to pass over or round an obstacle.
    Jog { line: u32, seg: usize, s0: f64, s1: f64, delta: P3 },
    /// Takes a jog out again: run `seg` moves back in line by the shorter of its two legs, and
    /// corners left in a straight line are removed.
    RemoveJog { line: u32, seg: usize },
    /// A component dropped near `at`: it snaps onto the nearest free place on the line, takes the
    /// line's size and class, and splits the pipe.
    Drop { line: u32, kind: ItemKind, at: P3, dir: P3, nps2: String },
    MoveItem { line: u32, item: u32, at: P3 },
    SetItem { line: u32, item: Item },
    /// Sets a line's size: every component on it re-sizes.
    Resize { line: u32, nps: String },
    SetTerm { line: u32, start: bool, term: Term },
    /// Routes a line square again between its ends, through its corners away from the ends;
    /// components keep their run where it still exists.
    Reroute { line: u32 },
    Delete { ids: Vec<u32> },
    /// Moves equipment and lines together by `delta`; lines connected to them follow.
    MoveGroup { ids: Vec<u32>, delta: P3 },
    /// Copies equipment and lines (with their components) by `delta`: lines between copied
    /// equipment connect to the copies; ends to anything not copied become free ends.
    CopyGroup { ids: Vec<u32>, delta: P3 },
    /// Turns equipment and lines about a vertical axis through `about` by `deg` degrees.
    RotateGroup { ids: Vec<u32>, about: P3, deg: f64 },
    /// Mirrors equipment and lines in a vertical plane through `about`: across north–south
    /// (east becomes west) when `east_west` is false, across east–west otherwise.
    MirrorGroup { ids: Vec<u32>, about: P3, east_west: bool },
    /// The plant's saved views (replaces the list).
    SetViews(Vec<SavedView>),
    /// Reads a line list (CSV text with a header row) and keeps it with the plant, replacing any
    /// earlier list; the checks compare the model's lines with it.
    ImportLineList { csv: String },
    /// Gives a line the number, size, class, rating, fluid and insulation of a listed line.
    UseListedLine { line: u32, number: String },
    /// Adds the lines of a piping component file (.pcf).
    ImportPcf { text: String },
    Batch(Vec<Op>),
}

pub struct Doc {
    pub p: Project,
    pub path: Option<String>,
    pub dirty: bool,
    undo: Vec<Project>,
    redo: Vec<Project>,
}

const UNDO: usize = 200;

impl Doc {
    pub fn new(p: Project) -> Doc {
        Doc { p, path: None, dirty: false, undo: vec![], redo: vec![] }
    }
    pub fn can_undo(&self) -> bool { !self.undo.is_empty() }
    pub fn can_redo(&self) -> bool { !self.redo.is_empty() }
    pub fn undo(&mut self) {
        if let Some(p) = self.undo.pop() { self.redo.push(std::mem::replace(&mut self.p, p)); self.dirty = true; }
    }
    pub fn redo(&mut self) {
        if let Some(p) = self.redo.pop() { self.undo.push(std::mem::replace(&mut self.p, p)); self.dirty = true; }
    }

    /// Applies an operation; on error the model is unchanged. Returns the ids it created.
    pub fn apply(&mut self, op: Op, cat: &Catalogue) -> Result<Vec<u32>, String> {
        let mut p = self.p.clone();
        let mut created = vec![];
        apply(&mut p, op, cat, &mut created)?;
        p.normalize();
        follow_ends(&mut p, cat);
        if p != self.p {
            self.undo.push(std::mem::replace(&mut self.p, p));
            if self.undo.len() > UNDO { self.undo.remove(0); }
            self.redo.clear();
            self.dirty = true;
        }
        Ok(created)
    }
}

/// Lines follow what they connect to: when a nozzle or a tee's branch end has moved, the move
/// passes along the route leg by leg, each leg absorbing the part along its own axis (it
/// stretches), until nothing is left, so the route stays square and its corners stay put
/// wherever they can. Tee branches depend on other lines, so this runs until nothing moves.
pub fn follow_ends(p: &mut Project, cat: &Catalogue) {
    let mut changed = std::collections::HashSet::new();
    for _ in 0..4 {
        let mut moved = false;
        for i in 0..p.lines.len() {
            for start in [true, false] {
                let l = &p.lines[i];
                let e = if start { &l.from } else { &l.to };
                let Some(port) = route::end_port(p, cat, e) else { continue };
                let n = l.pts.len();
                if n < 2 { continue; }
                let k = if start { 0 } else { n - 1 };
                let delta = sub(port.p, l.pts[k]);
                if norm(delta) < 1e-9 { continue; }
                let mut pts = l.pts.clone();
                if !start { pts.reverse(); }
                let far = if start { &l.to } else { &l.from };
                let min_len = { let f = cat.fitting(&l.nps); let fl = cat.flange(&l.nps, l.rating).map_or(0.0, |x| x.wn_length); fl + 2.0 * f.map_or(0.0, |x| x.elbow_lr) };
                let left = pass_along(&mut pts, delta, matches!(far, End::Point { .. }), min_len);
                if !start { pts.reverse(); }
                // The move reached the far end and that end is attached: the legs cannot take it
                // without a new jog, so the line stays as it was and its skewed end is reported
                // (route it again).
                if norm(left) > 1e-9 { continue; }
                p.lines[i].pts = pts;
                changed.insert(i);
                moved = true;
            }
        }
        if !moved { break; }
    }
    // Components that no longer fit where they were slide to the nearest free place on their run.
    for i in changed {
        let l = p.lines[i].clone();
        let placed: std::collections::HashSet<u32> = route::build_line(p, cat, &l).iter().map(|q| q.item).collect();
        for it in l.items.iter().filter(|it| !placed.contains(&it.id)) {
            let nps = route::size_at(&l, it.seg, it.at);
            let len = route::item_len(cat, &l, it, &nps);
            if let Some(at) = route::free_place(p, cat, &p.lines[i], it.seg, it.at, len, 0.01, it.id) {
                if let Some(x) = p.lines[i].items.iter_mut().find(|x| x.id == it.id) { x.at = at; }
            }
        }
    }
}

/// Where a component dropped near `at` would go on a line: (segment, distance along it, the
/// point), or why it cannot go there. The window shows this while dragging; `Drop` uses it.
pub fn snap(p: &Project, cat: &Catalogue, l: &Line, kind: ItemKind, at: P3, nps2: &str) -> Result<(usize, f64, P3), String> {
    let (seg, s, _) = route::nearest(p, cat, l, at).ok_or("The line has no route to place it on.")?;
    let nps = route::size_at(l, seg, s);
    let mut it = Item { kind, seg, at: s, nps2: nps2.to_string(), ..Default::default() };
    if kind == ItemKind::Reducer && it.nps2.is_empty() { it.nps2 = route::reducer_to(&it, &nps); }
    if !it.nps2.is_empty() { valid_nps(&it.nps2)?; }
    let len = route::item_len(cat, l, &it, &nps);
    let place = route::free_place(p, cat, l, seg, s, len, 0.01, 0).ok_or_else(|| format!("No room for a {}\" {} ({:.0} mm) on this run: it needs a straight piece at least that long between fittings.", nps, kind.name().to_lowercase(), len * 1000.0))?;
    let pts = route::path(p, cat, l);
    let u = unit(sub(pts[seg + 1], pts[seg]));
    Ok((seg, place, add(pts[seg], mul(u, place))))
}

/// Applies a placement change to the equipment and lines in `ids`: `pt` maps points, `dir` maps
/// directions, `rot` maps an equipment rotation; `mirror` also mirrors nozzles in their own y.
fn group(p: &mut Project, ids: &[u32], pt: impl Fn(P3) -> P3, dir: impl Fn(P3) -> P3, rot: impl Fn(f64) -> f64, mirror: bool) -> Result<(), String> {
    let set: std::collections::HashSet<u32> = ids.iter().copied().collect();
    let mut any = false;
    for e in p.equipment.iter_mut().filter(|e| set.contains(&e.id)) {
        e.pos = pt(e.pos);
        e.rot = (rot(e.rot) % 360.0 + 360.0) % 360.0;
        if mirror { for n in &mut e.nozzles { n.pos[1] = -n.pos[1]; n.dir[1] = -n.dir[1]; } }
        any = true;
    }
    for l in p.lines.iter_mut().filter(|l| set.contains(&l.id)) {
        for q in &mut l.pts { *q = pt(*q); }
        for it in &mut l.items { it.dir = dir(it.dir); }
        any = true;
    }
    if !any { return Err("Select equipment or lines first.".into()); }
    Ok(())
}

/// How far a route leaves a nozzle before it turns: flange, reducer and elbow, to 50 mm.
pub fn lead_for(cat: &Catalogue, nps: &str, rating: u32) -> f64 {
    let od = cat.od(nps);
    let lr = cat.fitting(nps).map(|f| f.elbow_lr).unwrap_or(od * 1.5);
    let fl = cat.flange(nps, rating).map(|f| f.wn_length).unwrap_or(od * 0.5);
    let h = cat.fitting(nps).map(|f| f.reducer_h).unwrap_or(od);
    ((lr + fl + h + 0.05) / 0.05).ceil() * 0.05
}

/// Moves pts[0] by `delta` and passes the move along the legs (see `follow_ends`); gives what
/// was left over at the far end (zero when the legs absorbed it all).
fn pass_along(pts: &mut [P3], delta: P3, move_last: bool, min_len: f64) -> P3 {
    let mut d = delta;
    let n = pts.len();
    for k in 0..n {
        if norm(d) < 1e-12 { break; }
        // The leg after this point (its direction before the move) absorbs the part along it.
        let axis = if k + 1 < n { unit(sub(pts[k + 1], pts[k])) } else { [0.0; 3] };
        if k + 1 == n && !move_last { return d; }
        // A leg absorbs the move along it unless that leaves it shorter than its fittings need;
        // then it moves whole and passes the move on.
        let keep = k + 1 < n && norm(axis) > 0.0 && dot(sub(sub(pts[k + 1], pts[k]), d), axis) < min_len.min(dot(sub(pts[k + 1], pts[k]), axis)) - 1e-9;
        pts[k] = add(pts[k], d);
        if norm(axis) > 0.0 && !keep { d = sub(d, mul(axis, dot(d, axis))); }
    }
    // A free far end moved with the rest: nothing is left over.
    if move_last { [0.0; 3] } else { d }
}

/// A line list from CSV text: the header row names the columns (any order, any case): line
/// number, size (inches like 6, 1-1/2 or 6", or DN150), class, rating, fluid, insulation (mm),
/// from, to and notes. Datum Layout's own line list export reads back. Gives the rows and how
/// many rows were left out.
pub fn read_line_list(csv: &str) -> Result<(Vec<ListedLine>, usize), String> {
    let rows = csv_rows(csv);
    let Some((head, body)) = rows.split_first() else { return Err("The line list is empty.".into()) };
    let col = |names: &[&str]| head.iter().position(|h| { let h = h.trim().to_lowercase(); names.iter().any(|n| h == *n || h.starts_with(&format!("{n} ("))) });
    let number = col(&["line number", "line", "number", "line no", "line no."]).ok_or("The line list needs a \"Line number\" column.")?;
    let (size, class, rating, fluid, ins, from, to, notes) = (col(&["size", "nps", "size (in)", "dn"]), col(&["class", "pipe class", "spec", "piping class"]), col(&["rating", "flange rating"]), col(&["fluid", "fluid code", "service"]), col(&["insulation", "insulation (mm)"]), col(&["from"]), col(&["to"]), col(&["notes", "remarks"]));
    let get = |r: &[String], c: Option<usize>| c.and_then(|c| r.get(c)).map(|s| s.trim().trim_start_matches('\'').to_string()).unwrap_or_default();
    let mut out = vec![];
    let mut skipped = 0;
    for r in body {
        let n = get(r, Some(number));
        if n.is_empty() { skipped += 1; continue; }
        let s = get(r, size);
        let nps = list_size(&s);
        out.push(ListedLine {
            number: n, nps: nps.unwrap_or("").into(), class: get(r, class), rating: get(r, rating).trim_start_matches("Class ").trim_start_matches("CL").trim().parse().ok().filter(|v| crate::catalogue::RATINGS.contains(v)).unwrap_or(0),
            fluid: get(r, fluid), insulation: get(r, ins).parse::<f64>().ok().filter(|v| v.is_finite() && *v >= 0.0).map_or(0.0, |v| v / 1000.0),
            from: get(r, from), to: get(r, to), notes: get(r, notes),
        });
    }
    if out.is_empty() { return Err("No lines could be read from the line list.".into()); }
    Ok((out, skipped))
}

/// A size as written in a line list: 6, 6", 1-1/2, 1 1/2, 1.5 or DN150.
fn list_size(s: &str) -> Option<&'static str> {
    let t = s.trim().trim_end_matches('"').trim_end_matches("in").trim().replace(' ', "-");
    if let Some(dn) = t.to_uppercase().strip_prefix("DN") { return crate::pcf::import::nps_of(dn.trim().parse().ok()?, false); }
    if let Some(n) = NPS.iter().copied().find(|n| *n == t) { return Some(n); }
    crate::pcf::import::nps_of(t.parse().ok()?, true)
}

/// CSV rows (quoted cells, doubled quotes, CR LF or LF).
fn csv_rows(s: &str) -> Vec<Vec<String>> {
    let mut rows = vec![];
    let (mut row, mut cell, mut q) = (vec![], String::new(), false);
    let mut it = s.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(c) = it.next() {
        match (c, q) {
            ('"', true) if it.peek() == Some(&'"') => { cell.push('"'); it.next(); }
            ('"', true) => q = false,
            // A quote opens a quoted cell only at its start (6" is a size, not a quote).
            ('"', false) if cell.trim().is_empty() => { cell.clear(); q = true; }
            (',', false) | (';', false) => row.push(std::mem::take(&mut cell)),
            ('\n', false) => { row.push(std::mem::take(&mut cell)); if row.iter().any(|c: &String| !c.trim().is_empty()) { rows.push(std::mem::take(&mut row)); } else { row.clear(); } }
            ('\r', false) => {}
            _ => cell.push(c),
        }
    }
    row.push(cell);
    if row.iter().any(|c| !c.trim().is_empty()) { rows.push(row); }
    rows
}

/// Removes zero-length runs and corners in a straight line; components keep their place.
pub fn tidy(l: &mut Line) {
    let pts = &l.pts;
    if pts.len() < 3 { return; }
    let mut np = vec![pts[0]];
    let mut map = vec![(0usize, 0.0f64); pts.len() - 1];
    for i in 0..pts.len() - 1 {
        let last = *np.last().unwrap_or(&pts[i]);
        let b = pts[i + 1];
        let k = np.len();
        if norm(sub(b, last)) < 1e-6 {
            map[i] = if k >= 2 { (k - 2, norm(sub(last, np[k - 2]))) } else { (0, 0.0) };
        } else if k >= 2 && dot(unit(sub(b, last)), unit(sub(last, np[k - 2]))) > 1.0 - 1e-9 {
            map[i] = (k - 2, norm(sub(last, np[k - 2])));
            np[k - 1] = b;
        } else {
            map[i] = (k - 1, 0.0);
            np.push(b);
        }
    }
    if np.len() < 2 { return; }
    for it in l.items.iter_mut() {
        if let Some(&(s, off)) = map.get(it.seg) { it.seg = s; it.at += off; }
    }
    l.pts = np;
}

fn line_mut(p: &mut Project, id: u32) -> Result<&mut Line, String> {
    p.lines.iter_mut().find(|l| l.id == id).ok_or_else(|| "That line no longer exists.".to_string())
}

/// The next tag for a kind: V-101, V-102…
fn next_tag(p: &Project, kind: EquipKind) -> String {
    let pre = kind.prefix();
    let mut n = 101;
    while p.equipment.iter().any(|e| e.tag == format!("{pre}-{n}")) { n += 1; }
    format!("{pre}-{n}")
}

/// The next line number: 6"-P-1001-CS1, 1002…
pub fn next_number(p: &Project, nps: &str, fluid: &str, class: &str) -> String {
    let mut n = 1001;
    while p.lines.iter().any(|l| l.number.contains(&format!("-{n}-")) || l.number.ends_with(&format!("-{n}"))) { n += 1; }
    format!("{nps}\"-{fluid}-{n}-{class}")
}

fn valid_nps(s: &str) -> Result<(), String> {
    if NPS.contains(&s) { Ok(()) } else { Err(format!("{s}\" is not a size in the catalogue (½\" to 24\").")) }
}

fn apply(p: &mut Project, op: Op, cat: &Catalogue, created: &mut Vec<u32>) -> Result<(), String> {
    match op {
        Op::SetMeta { name, notes, plant, unit, units } => {
            if name.trim().is_empty() { return Err("The project needs a name.".into()); }
            p.name = name; p.notes = notes; p.plant = plant; p.unit = unit; p.units = units;
        }
        Op::SetPlot(plot) => p.plot = plot,
        Op::SetSettings(s) => {
            valid_nps(&s.nps)?;
            p.settings = s;
        }
        Op::AddEquipment { kind, pos } => {
            let tag = next_tag(p, kind);
            let mut e = equipment::new(kind, &tag);
            e.id = p.id();
            e.pos = pos;
            created.push(e.id);
            p.equipment.push(e);
        }
        Op::SetEquipment(e) => {
            if e.tag.trim().is_empty() { return Err("Equipment needs a tag (for example P-101A).".into()); }
            if p.equipment.iter().any(|x| x.id != e.id && x.tag == e.tag) { return Err(format!("{} is already used by another item.", e.tag)); }
            let mut tags = std::collections::HashSet::new();
            for n in &e.nozzles {
                valid_nps(&n.nps)?;
                if !tags.insert(&n.tag) { return Err(format!("Two nozzles are tagged {}.", n.tag)); }
            }
            let old = p.equip(e.id).ok_or("That equipment item no longer exists.")?.clone();
            // A renamed nozzle keeps its lines; a removed one frees them.
            for (i, n) in old.nozzles.iter().enumerate() {
                if e.nozzles.iter().any(|x| x.tag == n.tag) { continue; }
                let renamed = (old.nozzles.len() == e.nozzles.len()).then(|| e.nozzles[i].tag.clone());
                for l in &mut p.lines {
                    for end in [&mut l.from, &mut l.to] {
                        if let End::Nozzle { equip, nozzle } = end {
                            if *equip == e.id && *nozzle == n.tag {
                                match &renamed { Some(t) => *nozzle = t.clone(), None => *end = End::Point { term: Term::Open } }
                            }
                        }
                    }
                }
            }
            let id = e.id;
            *p.equipment.iter_mut().find(|x| x.id == id).ok_or("That equipment item no longer exists.")? = e;
        }
        Op::AddArea(mut a) => {
            a.id = p.id();
            if a.name.trim().is_empty() { a.name = format!("Area {}", p.areas.len() + 1); }
            created.push(a.id);
            p.areas.push(a);
        }
        Op::SetArea(a) => { let id = a.id; *p.areas.iter_mut().find(|x| x.id == id).ok_or("That area no longer exists.")? = a; }
        Op::Route { from, via, to, nps } => {
            let end = |e: &RouteEnd| -> Result<(End, P3, Option<P3>, Option<String>), String> {
                match e {
                    RouteEnd::Point { p: q } => Ok((End::Point { term: Term::Open }, *q, None, None)),
                    RouteEnd::Nozzle { equip, nozzle } => {
                        let me = End::Nozzle { equip: *equip, nozzle: nozzle.clone() };
                        let port = route::end_port(p, cat, &me).ok_or("That nozzle no longer exists.")?;
                        if p.lines.iter().any(|l| l.from == me || l.to == me) { return Err(format!("Nozzle {nozzle} already has a line.")); }
                        Ok((me, port.p, Some(port.d), Some(port.nps)))
                    }
                    RouteEnd::Branch { line, item } => {
                        let me = End::Branch { line: *line, item: *item };
                        let port = route::end_port(p, cat, &me).ok_or("That tee no longer exists.")?;
                        if p.lines.iter().any(|l| l.from == me || l.to == me) { return Err("That tee's branch already has a line.".into()); }
                        Ok((me, port.p, Some(port.d), Some(port.nps)))
                    }
                }
            };
            let (fe, fp, fd, fnps) = end(&from)?;
            let (te, tp, td, _) = end(&to)?;
            if norm(sub(fp, tp)) < 1e-3 && via.is_empty() { return Err("The route starts and ends at the same point.".into()); }
            let nps = nps.or(fnps).unwrap_or_else(|| p.settings.nps.clone());
            valid_nps(&nps)?;
            let class = p.settings.class.clone();
            // Leave each nozzle far enough for its flange, a reducer and an elbow.
            let lead = |d: Option<P3>| d.map(|_| lead_for(cat, &nps, p.settings.rating)).unwrap_or(0.0);
            let l0 = lead(fd).max(lead(td));
            let pts = route::orthogonal((fp, fd), &via, (tp, td), l0);
            if pts.len() < 2 { return Err("The route has no length.".into()); }
            let id = p.id();
            let number = next_number(p, &nps, "P", &class);
            p.lines.push(Line { id, number, nps, class, rating: p.settings.rating, from: fe, to: te, pts, ..Default::default() });
            created.push(id);
        }
        Op::SetLine(l) => {
            valid_nps(&l.nps)?;
            if l.pts.len() < 2 { return Err("A line needs at least two points.".into()); }
            let id = l.id;
            *line_mut(p, id)? = l;
        }
        Op::MoveRun { line, seg, delta } => {
            let pts = route::path(p, cat, p.line(line).ok_or("That line no longer exists.")?);
            let l = line_mut(p, line)?;
            if pts.len() != l.pts.len() { l.pts = pts.clone(); }
            let n = l.pts.len() - 1;
            if seg >= n { return Err("That run no longer exists.".into()); }
            let fixed = |e: &End| !matches!(e, End::Point { .. });
            if (seg == 0 && fixed(&l.from)) || (seg == n - 1 && fixed(&l.to)) {
                return Err("This run ends at a nozzle or a tee, so it cannot move; move the next run, or route the line again.".into());
            }
            let u = unit(sub(l.pts[seg + 1], l.pts[seg]));
            let d = sub(delta, mul(u, dot(delta, u)));
            if norm(d) < 1e-9 { return Err("A run moves square to itself; drag it sideways or up and down.".into()); }
            l.pts[seg] = add(l.pts[seg], d);
            l.pts[seg + 1] = add(l.pts[seg + 1], d);
            // A free end moved with its run gets a new leg only if it is not the run itself.
        }
        Op::MoveCorner { line, corner, delta } => {
            let pts = route::path(p, cat, p.line(line).ok_or("That line no longer exists.")?);
            let l = line_mut(p, line)?;
            l.pts = pts;
            let n = l.pts.len() - 1;
            if corner == 0 || corner >= n { return Err("Line ends move with their nozzle or tee; pick a corner between two runs.".into()); }
            let fixed = |e: &End| !matches!(e, End::Point { .. });
            let (u1, u2) = (unit(sub(l.pts[corner], l.pts[corner - 1])), unit(sub(l.pts[corner + 1], l.pts[corner])));
            let mut d = delta;
            // The corner before or after may be a nozzle or tee: then only slide along that run.
            if corner == 1 && fixed(&l.from) { d = mul(u1, dot(d, u1)); }
            if corner + 1 == n && fixed(&l.to) { d = mul(u2, dot(d, u2)); }
            if norm(d) < 1e-9 { return Err("This corner is held by the nozzles or tees at both ends; add a jog to the run instead.".into()); }
            let perp = |u: P3| sub(d, mul(u, dot(d, u)));
            l.pts[corner - 1] = add(l.pts[corner - 1], perp(u1));
            l.pts[corner] = add(l.pts[corner], d);
            l.pts[corner + 1] = add(l.pts[corner + 1], perp(u2));
            // Components measure from their run's start: the run after the corner starts later or earlier.
            let shift = dot(d, u2);
            let len = norm(sub(l.pts[corner + 1], l.pts[corner]));
            for it in l.items.iter_mut().filter(|i| i.seg == corner) { it.at = (it.at - shift).clamp(0.0, len); }
            if l.pts.windows(2).any(|w| norm(sub(w[1], w[0])) < 1e-6) { return Err("That would fold a run back onto itself.".into()); }
        }
        Op::Jog { line, seg, s0, s1, delta } => {
            let pts = route::path(p, cat, p.line(line).ok_or("That line no longer exists.")?);
            let l = line_mut(p, line)?;
            l.pts = pts;
            if seg + 1 >= l.pts.len() { return Err("That run no longer exists.".into()); }
            let (a, b) = (l.pts[seg], l.pts[seg + 1]);
            let len = norm(sub(b, a));
            let u = unit(sub(b, a));
            let d = sub(delta, mul(u, dot(delta, u)));
            if norm(d) < 1e-3 { return Err("A jog goes square to the run: up, down or sideways.".into()); }
            if !(s0 > 1e-6 && s1 > s0 + 1e-6 && s1 < len - 1e-6) { return Err("The jog must lie inside the run.".into()); }
            // Each turn needs an elbow's room clear of every component on the run.
            let lc = l.clone();
            for it in lc.items.iter().filter(|i| i.seg == seg) {
                let nps = route::size_at(&lc, seg, it.at);
                let room = cat.fitting(&nps).map(|f| f.elbow_lr).unwrap_or(cat.od(&nps) * 1.5);
                let half = route::item_len(cat, &lc, it, &nps) / 2.0 + room;
                let mid = it.at;
                if (mid - s0).abs() < half || (mid - s1).abs() < half {
                    return Err("A component is where the jog would turn; place the jog clear of it.".into());
                }
            }
            let l = line_mut(p, line)?;
            let (p0, p1) = (add(a, mul(u, s0)), add(a, mul(u, s1)));
            l.pts.splice(seg + 1..seg + 1, [p0, add(p0, d), add(p1, d), p1]);
            for it in l.items.iter_mut() {
                if it.seg > seg { it.seg += 4; }
                else if it.seg == seg && it.at > s1 { it.seg += 4; it.at -= s1; }
                else if it.seg == seg && it.at >= s0 { it.seg += 2; it.at -= s0; }
            }
        }
        Op::RemoveJog { line, seg } => {
            let pts = route::path(p, cat, p.line(line).ok_or("That line no longer exists.")?);
            let l = line_mut(p, line)?;
            l.pts = pts;
            let n = l.pts.len() - 1;
            if seg == 0 || seg + 1 >= n { return Err("Pick the middle run of a jog: a run with a turn at both ends.".into()); }
            let (v0, v1, v2, v3) = (l.pts[seg - 1], l.pts[seg], l.pts[seg + 1], l.pts[seg + 2]);
            let (a, b) = (sub(v1, v0), sub(v3, v2));
            if dot(unit(a), unit(b)) > -1.0 + 1e-6 { return Err("This run is not the middle of a jog (its two legs do not go opposite ways).".into()); }
            let m = norm(a).min(norm(b));
            let back = mul(unit(a), -m);
            let short = if norm(a) <= norm(b) { seg - 1 } else { seg + 1 };
            if l.items.iter().any(|i| i.seg == short) { return Err("A component is on the jog's leg; move or delete it first.".into()); }
            l.pts[seg] = add(v1, back);
            l.pts[seg + 1] = add(v2, back);
            tidy(l);
        }
        Op::Drop { line, kind, at, dir, nps2 } => {
            let l = p.line(line).ok_or("That line no longer exists.")?.clone();
            let (seg, s, _) = snap(p, cat, &l, kind, at, &nps2)?;
            let nps = route::size_at(&l, seg, s);
            let mut it = Item { id: p.id(), uid: String::new(), kind, seg, at: s, nps2, dir, tag: String::new() };
            if kind == ItemKind::Reducer && it.nps2.is_empty() { it.nps2 = route::reducer_to(&it, &nps); }
            created.push(it.id);
            line_mut(p, line)?.items.push(it);
        }
        Op::MoveItem { line, item, at } => {
            let l = p.line(line).ok_or("That line no longer exists.")?.clone();
            let it = l.items.iter().find(|i| i.id == item).ok_or("That component no longer exists.")?.clone();
            let (seg, s, _) = route::nearest(p, cat, &l, at).ok_or("The line has no route.")?;
            // Size and length as if it were already there.
            let mut moved = l.clone();
            moved.items.retain(|i| i.id != item);
            let nps = route::size_at(&moved, seg, s);
            let len = route::item_len(cat, &moved, &it, &nps);
            let at = route::free_place(p, cat, &l, seg, s, len, 0.01, item).ok_or("No room there: move it to a longer straight piece.")?;
            let lm = line_mut(p, line)?;
            if let Some(x) = lm.items.iter_mut().find(|i| i.id == item) { x.seg = seg; x.at = at; }
        }
        Op::SetItem { line, item } => {
            if !item.nps2.is_empty() { valid_nps(&item.nps2)?; }
            let l = line_mut(p, line)?;
            let id = item.id;
            *l.items.iter_mut().find(|i| i.id == id).ok_or("That component no longer exists.")? = item;
        }
        Op::Resize { line, nps } => {
            valid_nps(&nps)?;
            let l = line_mut(p, line)?;
            let old = l.nps.clone();
            if l.number.starts_with(&format!("{old}\"-")) { l.number = l.number.replacen(&format!("{old}\"-"), &format!("{nps}\"-"), 1); }
            l.nps = nps;
        }
        Op::SetTerm { line, start, term } => {
            let l = line_mut(p, line)?;
            let e = if start { &mut l.from } else { &mut l.to };
            if !matches!(e, End::Point { .. }) { return Err("Only a free end can have a cap or a blind flange.".into()); }
            *e = End::Point { term };
        }
        Op::Reroute { line } => {
            let l = p.line(line).ok_or("That line no longer exists.")?.clone();
            let (a, b) = (route::end_port(p, cat, &l.from), route::end_port(p, cat, &l.to));
            let n = l.pts.len();
            // Corners next to an attached end are its old lead; the new route makes its own.
            let lo = if a.is_some() { 2 } else { 1 };
            let hi = if b.is_some() { n.saturating_sub(2) } else { n.saturating_sub(1) };
            let via: Vec<P3> = if lo < hi { l.pts[lo..hi].to_vec() } else { vec![] };
            let s0 = a.map(|x| (x.p, Some(x.d))).unwrap_or((l.pts[0], None));
            let s1 = b.map(|x| (x.p, Some(x.d))).unwrap_or((l.pts[n - 1], None));
            let pts = route::orthogonal(s0, &via, s1, lead_for(cat, &l.nps, l.rating));
            if pts.len() < 2 { return Err("The line's ends are at the same point.".into()); }
            let lm = line_mut(p, line)?;
            let segs = pts.len() - 1;
            lm.pts = pts;
            for it in &mut lm.items { it.seg = it.seg.min(segs - 1); }
            let fresh = lm.clone();
            let placed: std::collections::HashSet<u32> = route::build_line(p, cat, &fresh).iter().map(|q| q.item).collect();
            for it in fresh.items.iter().filter(|it| !placed.contains(&it.id)) {
                let nps = route::size_at(&fresh, it.seg, it.at);
                let len = route::item_len(cat, &fresh, it, &nps);
                // The nearest free place on any run.
                let best = (0..segs).filter_map(|s| route::free_place(p, cat, &fresh, s, if s == it.seg { it.at } else { 0.0 }, len, 0.01, it.id).map(|at| (s, at))).min_by_key(|(s, _)| s.abs_diff(it.seg));
                if let Some((s, at)) = best { if let Some(x) = line_mut(p, line)?.items.iter_mut().find(|x| x.id == it.id) { x.seg = s; x.at = at; } }
            }
        }
        Op::MoveGroup { ids, delta } => {
            group(p, &ids, |q| add(q, delta), |v| v, |r| r, false)?;
        }
        Op::RotateGroup { ids, about, deg } => {
            let r = |v: P3| crate::parts::rot_z(v, deg);
            group(p, &ids, |q| add(about, r(sub(q, about))), r, |rot| rot + deg, false)?;
        }
        Op::MirrorGroup { ids, about, east_west } => {
            // Across a north–south plane x' = 2c − x (or y' = 2c − y). Every equipment body is
            // symmetric about its own x axis, so a mirrored item is the item turned to 180° − θ
            // (or −θ) with its nozzles mirrored in their own y.
            let k = if east_west { 1 } else { 0 };
            let m = move |v: P3| { let mut w = v; w[k] = -w[k]; w };
            group(p, &ids, |q| { let mut w = q; w[k] = 2.0 * about[k] - q[k]; w }, m, move |rot| if east_west { -rot } else { 180.0 - rot }, true)?;
        }
        Op::CopyGroup { ids, delta } => {
            let set: std::collections::HashSet<u32> = ids.iter().copied().collect();
            let mut emap = std::collections::HashMap::new();
            let mut lmap = std::collections::HashMap::new();
            let mut imap = std::collections::HashMap::new();
            let eqs: Vec<Equipment> = p.equipment.iter().filter(|e| set.contains(&e.id)).cloned().collect();
            let lines: Vec<Line> = p.lines.iter().filter(|l| set.contains(&l.id)).cloned().collect();
            if eqs.is_empty() && lines.is_empty() { return Err("Select equipment or lines to copy.".into()); }
            for mut e in eqs {
                let id = p.id();
                emap.insert(e.id, id);
                e.id = id;
                e.uid.clear();
                e.tag = next_tag(p, e.kind);
                e.pos = add(e.pos, delta);
                created.push(id);
                p.equipment.push(e);
            }
            for l in &lines { lmap.insert(l.id, p.id()); for it in &l.items { imap.insert(it.id, p.id()); } }
            for mut l in lines {
                l.id = lmap[&l.id];
                l.uid.clear();
                l.number = next_number(p, &l.nps, &l.fluid, &l.class);
                for q in &mut l.pts { *q = add(*q, delta); }
                for it in &mut l.items { it.id = imap[&it.id]; it.uid.clear(); }
                for e in [&mut l.from, &mut l.to] {
                    *e = match e.clone() {
                        End::Nozzle { equip, nozzle } if emap.contains_key(&equip) => End::Nozzle { equip: emap[&equip], nozzle },
                        End::Branch { line, item } if lmap.contains_key(&line) && imap.contains_key(&item) => End::Branch { line: lmap[&line], item: imap[&item] },
                        End::Point { term } => End::Point { term },
                        _ => End::Point { term: Term::Open },
                    };
                }
                created.push(l.id);
                p.lines.push(l);
            }
        }
        Op::SetViews(v) => { p.views = v; }
        Op::ImportLineList { csv } => {
            let (rows, _) = read_line_list(&csv)?;
            p.line_list = rows;
        }
        Op::UseListedLine { line, number } => {
            let r = p.line_list.iter().find(|r| r.number == number).cloned().ok_or("That line is not on the line list.")?;
            // Resizing renames the line by its size; the list's number is set afterwards.
            p.line(line).ok_or("That line no longer exists.")?;
            if !r.nps.is_empty() { apply(p, Op::Resize { line, nps: r.nps.clone() }, cat, created)?; }
            let l = line_mut(p, line)?;
            l.number = r.number;
            if !r.class.is_empty() { l.class = r.class; }
            if r.rating != 0 { l.rating = r.rating; }
            if !r.fluid.is_empty() { l.fluid = r.fluid; }
            l.insulation = r.insulation;
        }
        Op::ImportPcf { text } => {
            let mut next = p.next.max(1);
            let used: std::collections::HashSet<u32> = p.equipment.iter().map(|e| e.id).chain(p.lines.iter().flat_map(|l| std::iter::once(l.id).chain(l.items.iter().map(|i| i.id)))).chain(p.areas.iter().map(|a| a.id)).collect();
            let mut id = || { while used.contains(&next) || next == 0 { next += 1; } next += 1; next - 1 };
            let (lines, _) = crate::pcf::import::read(&text, p, cat, &mut id)?;
            if lines.is_empty() { return Err("The file has no lines that could be read.".into()); }
            p.next = next;
            for mut l in lines {
                l.uid = crate::model::new_uid(l.id);
                for it in &mut l.items { it.uid = crate::model::new_uid(it.id); }
                created.push(l.id);
                p.lines.push(l);
            }
        }
        Op::Delete { ids } => {
            let ids: std::collections::HashSet<u32> = ids.into_iter().collect();
            let free = |e: &mut End, pred: &dyn Fn(&End) -> bool| if pred(e) { *e = End::Point { term: Term::Open } };
            let gone_items: Vec<(u32, u32)> = p.lines.iter().flat_map(|l| l.items.iter().filter(|i| ids.contains(&i.id) || ids.contains(&l.id)).map(move |i| (l.id, i.id))).collect();
            let gone = |e: &End| match e {
                End::Nozzle { equip, .. } => ids.contains(equip),
                End::Branch { line, item } => gone_items.contains(&(*line, *item)),
                End::Point { .. } => false,
            };
            for l in &mut p.lines {
                // Keep the end where it was drawn.
                free(&mut l.from, &gone);
                free(&mut l.to, &gone);
                l.items.retain(|i| !ids.contains(&i.id));
            }
            p.equipment.retain(|e| !ids.contains(&e.id));
            p.lines.retain(|l| !ids.contains(&l.id));
            p.areas.retain(|a| !ids.contains(&a.id));
            for e in &mut p.equipment { if ids.contains(&e.area) { e.area = 0; } }
            for l in &mut p.lines { if ids.contains(&l.area) { l.area = 0; } }
        }
        Op::Batch(ops) => {
            for o in ops { apply(p, o, cat, created)?; }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_edit_changes_nothing_and_undo_restores() {
        let cat = Catalogue::builtin();
        let mut d = Doc::new(Project::new());
        let ids = d.apply(Op::AddEquipment { kind: EquipKind::Pump, pos: [5.0, 5.0, 0.0] }, &cat).unwrap();
        assert_eq!(d.p.equipment[0].tag, "P-101");
        let before = d.p.clone();
        assert!(d.apply(Op::Resize { line: 999, nps: "6".into() }, &cat).is_err());
        assert!(d.apply(Op::Batch(vec![Op::AddEquipment { kind: EquipKind::Tank, pos: [0.0; 3] }, Op::Resize { line: 1, nps: "7".into() }]), &cat).is_err());
        assert_eq!(d.p, before);
        let pid = ids[0];
        let lid = d.apply(Op::Route { from: RouteEnd::Nozzle { equip: pid, nozzle: "D".into() }, via: vec![], to: RouteEnd::Point { p: [12.0, 9.0, 4.0] }, nps: None }, &cat).unwrap()[0];
        assert_eq!(d.p.line(lid).unwrap().nps, "4");
        d.undo();
        assert!(d.p.lines.is_empty());
        d.redo();
        assert_eq!(d.p.lines.len(), 1);
        // Moving the pump keeps its line square: the route follows the nozzle.
        let mut e = d.p.equip(pid).unwrap().clone();
        e.pos = [6.3, 4.2, 0.0];
        d.apply(Op::SetEquipment(e), &cat).unwrap();
        let b = route::build(&d.p, &cat);
        assert!(b.issues.is_empty(), "{:?}", b.issues);
        // Deleting the pump frees the line's end.
        d.apply(Op::Delete { ids: vec![pid] }, &cat).unwrap();
        assert!(matches!(d.p.lines[0].from, End::Point { .. }));
    }
}
