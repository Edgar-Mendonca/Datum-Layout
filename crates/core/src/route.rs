//! Routing: turns the model into parts. Each line's path (its corner points) becomes straight
//! pipes and spec elbows at every turn (long or short radius, with exact cut-backs from the
//! elbow table), reducers where sizes change, flanges, gaskets and bolt sets where the line meets
//! a flanged nozzle, and the components placed on it (valves, strainers, flange pairs, tees,
//! reducers), each taking the line's size and class. Turns that are not 90° or 45°, turns too
//! tight for their elbows and components that do not fit are reported and not drawn.
//!
//! It also finds the point on a line nearest a dropped component (`snap`) and builds an
//! orthogonal path between two ports (`orthogonal`).

use crate::catalogue::{nps_index, Catalogue, NPS};
use crate::equipment;
use crate::model::{End, Item, ItemKind, Line, Project, Radius, Term, P3};
use crate::parts::{add, cross, dist, dot, frame, mul, norm, rotate, sub, unit, Port, Shape};
use serde::Serialize;
use std::collections::HashMap;

/// A generated part with its ports.
#[derive(Clone, Debug, Serialize)]
pub struct Part {
    /// pipe, elbow90, elbow45, tee, reducer, cap, flange, blind, gasket, bolts, gate, globe,
    /// check, ball, strainer; equipment and nozzle for equipment.
    pub kind: &'static str,
    pub line: u32,
    pub equip: u32,
    pub item: u32,
    /// The path segment it lies on (-1: not on a segment); for elbows, the segment before it.
    pub seg: i32,
    pub nps: String,
    pub nps2: String,
    pub rating: u32,
    pub ports: Vec<Port>,
    pub shapes: Vec<Shape>,
    /// Centreline length (m).
    pub length: f64,
    /// Elbows: the corner point; tees: the centre point.
    pub corner: Option<P3>,
    /// Bolt sets: number of studs, stud diameter and length (m).
    pub bolts: Option<(u32, f64, f64)>,
    /// Elbows: "LR" or "SR"; nozzles: the nozzle tag.
    pub note: String,
    pub insulation: f64,
}

impl Part {
    fn new(kind: &'static str, line: &Line) -> Part {
        Part { kind, line: line.id, equip: 0, item: 0, seg: -1, nps: line.nps.clone(), nps2: String::new(), rating: 0, ports: vec![], shapes: vec![], length: 0.0, corner: None, bolts: None, note: String::new(), insulation: line.insulation }
    }
    pub fn is_pipe_like(&self) -> bool {
        !matches!(self.kind, "equipment" | "nozzle" | "bolts")
    }
}

/// Something the checks or the routing found.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Issue {
    /// routing, clash, open, mismatch, class, number.
    pub kind: &'static str,
    pub text: String,
    pub at: P3,
    pub line: u32,
    pub item: u32,
    pub equip: u32,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct Built {
    pub parts: Vec<Part>,
    /// Routing problems (turns, fits, unknown sizes); the checks add theirs.
    pub issues: Vec<Issue>,
}

/// Where a size's dimensions are missing from the catalogue, these stand in (and are reported).
struct Dimens { od: f64, elbow: f64, tee: f64, red: f64, cap: f64, fl_od: f64, fl_t: f64, fl_y: f64, gasket: f64, bolts: (u32, f64) }

fn dimens(cat: &Catalogue, nps: &str, class: &str, rating: u32, radius: Radius) -> Dimens {
    let od = cat.pipe(nps, class).map(|p| p.od).unwrap_or_else(|| cat.od(nps));
    let f = cat.fitting(nps);
    let lr = f.map(|f| f.elbow_lr).unwrap_or(od * 1.5);
    let elbow = match radius { Radius::Long => lr, Radius::Short => f.and_then(|f| f.elbow_sr).unwrap_or(lr) };
    let fl = cat.flange(nps, rating);
    Dimens {
        od,
        elbow,
        tee: f.map(|f| f.tee_c).unwrap_or(od),
        red: f.map(|f| f.reducer_h).unwrap_or(od),
        cap: f.map(|f| f.cap_e).unwrap_or(od * 0.6),
        fl_od: fl.map(|f| f.od).unwrap_or(od * 1.8),
        fl_t: fl.map(|f| f.thick).unwrap_or(od * 0.15),
        fl_y: fl.map(|f| f.wn_length).unwrap_or(od * 0.5),
        gasket: fl.map(|f| f.gasket).unwrap_or(0.003),
        bolts: fl.map(|f| (f.bolts, f.bolt)).unwrap_or((4, 0.016)),
    }
}

/// The face-to-face length of an in-line valve or strainer.
fn f2f(cat: &Catalogue, kind: ItemKind, nps: &str, rating: u32, od: f64) -> f64 {
    let key = match kind { ItemKind::Strainer => "globe", k => k.key() };
    cat.valve(key, nps, rating).map(|v| v.f2f).unwrap_or(od * 2.5)
}

/// The size after a reducer: its own `nps2`, or the next size down.
pub fn reducer_to(it: &Item, nps: &str) -> String {
    if !it.nps2.is_empty() && it.nps2 != nps { return it.nps2.clone(); }
    let i = nps_index(nps);
    NPS[i.saturating_sub(1).min(NPS.len() - 1)].to_string()
}

/// The length an item takes along its line.
pub fn item_len(cat: &Catalogue, l: &Line, it: &Item, nps: &str) -> f64 {
    let d = dimens(cat, nps, &l.class, l.rating, l.radius);
    match it.kind {
        ItemKind::Tee => 2.0 * d.tee,
        ItemKind::Reducer => dimens(cat, &larger(nps, &reducer_to(it, nps)), &l.class, l.rating, l.radius).red,
        ItemKind::FlangePair => 2.0 * d.fl_y + d.gasket,
        k => f2f(cat, k, nps, l.rating, d.od) + 2.0 * (d.fl_y + d.gasket),
    }
}

fn larger(a: &str, b: &str) -> String {
    if nps_index(a) >= nps_index(b) { a.into() } else { b.into() }
}

/// The resolved path of a line (ends moved to the nozzle or tee they connect to).
pub fn path(p: &Project, cat: &Catalogue, l: &Line) -> Vec<P3> {
    let mut pts = l.pts.clone();
    if let Some(e) = end_port(p, cat, &l.from) { if let Some(f) = pts.first_mut() { *f = e.p; } }
    if let Some(e) = end_port(p, cat, &l.to) { if let Some(f) = pts.last_mut() { *f = e.p; } }
    pts.dedup_by(|a, b| dist(*a, *b) < 1e-6);
    pts
}

/// The port a line end connects to: a nozzle's face or a tee's branch end.
pub fn end_port(p: &Project, cat: &Catalogue, e: &End) -> Option<Port> {
    match e {
        End::Point { .. } => None,
        End::Nozzle { equip, nozzle } => {
            let eq = p.equip(*equip)?;
            let n = eq.nozzles.iter().find(|n| n.tag == *nozzle)?;
            Some(equipment::nozzle(eq, n, cat).port)
        }
        End::Branch { line, item } => {
            let l = p.line(*line)?;
            let it = l.items.iter().find(|i| i.id == *item)?;
            let pts = path_plain(p, cat, l);
            let (a, b) = (*pts.get(it.seg)?, *pts.get(it.seg + 1)?);
            let u = unit(sub(b, a));
            let nps = size_at(l, it.seg, it.at);
            let c = add(a, mul(u, it.at));
            let bd = branch_dir(u, it.dir);
            let d = dimens(cat, &nps, &l.class, l.rating, l.radius);
            Some(Port { p: add(c, mul(bd, d.tee)), d: bd, nps: branch_size(p, l, it), rating: 0, end: "BW" })
        }
    }
}

/// A line's path with nozzle ends resolved (no tee ends, so branches of branches never loop).
fn path_plain(p: &Project, cat: &Catalogue, l: &Line) -> Vec<P3> {
    let mut pts = l.pts.clone();
    if let End::Nozzle { .. } = l.from { if let (Some(e), Some(f)) = (end_port(p, cat, &l.from), pts.first_mut()) { *f = e.p; } }
    if let End::Nozzle { .. } = l.to { if let (Some(e), Some(f)) = (end_port(p, cat, &l.to), pts.last_mut()) { *f = e.p; } }
    pts.dedup_by(|a, b| dist(*a, *b) < 1e-6);
    pts
}

/// The branch size of a tee: the connected branch line's size, else its own `nps2`, else the run's.
pub fn branch_size(p: &Project, l: &Line, it: &Item) -> String {
    let conn = p.lines.iter().find(|b| matches!(b.from, End::Branch { line, item } if line == l.id && item == it.id) || matches!(b.to, End::Branch { line, item } if line == l.id && item == it.id));
    let run = size_at(l, it.seg, it.at);
    let s = conn.map(|b| b.nps.clone()).unwrap_or_else(|| if it.nps2.is_empty() { run.clone() } else { it.nps2.clone() });
    // A tee's branch is never larger than its run.
    if nps_index(&s) > nps_index(&run) { run } else { s }
}

/// A tee's branch direction: its own, made square to the run; by default up (or east on a riser).
pub fn branch_dir(u: P3, want: P3) -> P3 {
    let w = sub(want, mul(u, dot(want, u)));
    if norm(w) > 1e-6 { return unit(w); }
    if u[2].abs() < 0.9 { [0.0, 0.0, 1.0] } else { [1.0, 0.0, 0.0] }
}

/// The line's size at a place along its path (after any reducers before it).
pub fn size_at(l: &Line, seg: usize, at: f64) -> String {
    let mut items: Vec<&Item> = l.items.iter().collect();
    items.sort_by(|a, b| (a.seg, a.at).partial_cmp(&(b.seg, b.at)).unwrap_or(std::cmp::Ordering::Equal));
    let mut cur = l.nps.clone();
    for it in items {
        if (it.seg, it.at) >= (seg, at) { break; }
        if matches!(it.kind, ItemKind::Reducer) { cur = reducer_to(it, &cur); }
    }
    cur
}

fn issue(kind: &'static str, text: String, at: P3, line: u32, item: u32) -> Issue {
    Issue { kind, text, at, line, item, equip: 0 }
}

fn name(l: &Line) -> String {
    if l.number.trim().is_empty() { format!("(line without a number, {}\")", l.nps) } else { l.number.clone() }
}

/// Builds every part of the project.
pub fn build(p: &Project, cat: &Catalogue) -> Built {
    let mut b = Built::default();
    for e in &p.equipment {
        let (body, noz) = equipment::shapes(e, cat);
        b.parts.push(Part { kind: "equipment", line: 0, equip: e.id, item: 0, seg: -1, nps: String::new(), nps2: String::new(), rating: 0, ports: vec![], shapes: body, length: 0.0, corner: None, bolts: None, note: e.tag.clone(), insulation: 0.0 });
        for n in noz {
            b.parts.push(Part { kind: "nozzle", line: 0, equip: e.id, item: 0, seg: -1, nps: n.port.nps.clone(), nps2: String::new(), rating: n.port.rating, ports: vec![n.port], shapes: n.shapes, length: 0.0, corner: None, bolts: None, note: n.tag, insulation: 0.0 });
        }
    }
    for l in &p.lines {
        line(p, cat, l, &mut b);
    }
    b
}

/// One flanged joint's parts going along +u from s0: [flange?] gasket [flange?]. Used by the
/// builder through `Run`.
struct Run<'a> {
    l: &'a Line,
    a: P3,
    u: P3,
    seg: i32,
    out: &'a mut Vec<Part>,
}

impl Run<'_> {
    fn at(&self, s: f64) -> P3 { add(self.a, mul(self.u, s)) }
    fn port(&self, s: f64, back: bool, nps: &str, rating: u32, end: &'static str) -> Port {
        Port { p: self.at(s), d: if back { mul(self.u, -1.0) } else { self.u }, nps: nps.into(), rating, end }
    }
    fn push(&mut self, mut part: Part, item: u32) {
        part.seg = self.seg;
        part.item = item;
        self.out.push(part);
    }
    fn pipe(&mut self, s0: f64, s1: f64, d: &Dimens, nps: &str) {
        if s1 - s0 < 1e-6 { return; }
        let mut q = Part::new("pipe", self.l);
        q.nps = nps.into();
        q.shapes = vec![Shape::tube(self.at(s0), self.at(s1), d.od / 2.0)];
        q.ports = vec![self.port(s0, true, nps, 0, "BW"), self.port(s1, false, nps, 0, "BW")];
        q.length = s1 - s0;
        self.push(q, 0);
    }
    /// A weld-neck flange from s0 to s0 + Y, its face at the `face_back` end.
    fn flange(&mut self, s0: f64, d: &Dimens, nps: &str, face_at_start: bool, item: u32) {
        let (s1, r) = (s0 + d.fl_y, d.od / 2.0);
        let mut q = Part::new("flange", self.l);
        q.nps = nps.into();
        q.rating = self.l.rating;
        let (face, weld) = if face_at_start { (s0, s1) } else { (s1, s0) };
        let toward = if face_at_start { 1.0 } else { -1.0 };
        let hub_end = face + toward * d.fl_t;
        q.shapes = vec![Shape::tube(self.at(face), self.at(hub_end), d.fl_od / 2.0), Shape::Frustum { a: self.at(hub_end), b: self.at(weld), r1: (r * 1.35).min(d.fl_od / 2.0), r2: r * 1.02 }];
        q.ports = vec![self.port(face, face_at_start, nps, self.l.rating, "FL"), self.port(weld, !face_at_start, nps, 0, "BW")];
        q.length = d.fl_y;
        self.push(q, item);
    }
    /// A gasket from s0 to s0 + g, and the joint's bolt set.
    fn gasket(&mut self, s0: f64, d: &Dimens, nps: &str, item: u32) {
        let mut q = Part::new("gasket", self.l);
        q.nps = nps.into();
        q.rating = self.l.rating;
        q.shapes = vec![Shape::tube(self.at(s0), self.at(s0 + d.gasket), (d.fl_od / 2.0) * 0.72)];
        q.ports = vec![self.port(s0, true, nps, self.l.rating, "FL"), self.port(s0 + d.gasket, false, nps, self.l.rating, "FL")];
        q.length = d.gasket;
        self.push(q, item);
        let mut bt = Part::new("bolts", self.l);
        bt.nps = nps.into();
        bt.rating = self.l.rating;
        // Stud length: two flanges, the gasket and two nuts (about one diameter each) and threads.
        let len = 2.0 * d.fl_t + d.gasket + 2.0 * d.bolts.1 + 0.012;
        let len = (len / 0.005).ceil() * 0.005;
        bt.bolts = Some((d.bolts.0, d.bolts.1, len));
        self.push(bt, item);
    }
}

fn line(p: &Project, cat: &Catalogue, l: &Line, b: &mut Built) {
    let pts = path(p, cat, l);
    if p.class_missing(cat, &l.class) {
        b.issues.push(issue("class", format!("Line {}: the pipe class \"{}\" is not in the catalogue; dimensions are approximate.", name(l), l.class), pts.first().copied().unwrap_or_default(), l.id, 0));
    }
    let n = pts.len().saturating_sub(1);
    if n == 0 {
        b.issues.push(issue("routing", format!("Line {}: its two ends are at the same point.", name(l)), pts.first().copied().unwrap_or_default(), l.id, 0));
        return;
    }
    let lens: Vec<f64> = (0..n).map(|i| dist(pts[i], pts[i + 1])).collect();
    let dirs: Vec<P3> = (0..n).map(|i| unit(sub(pts[i + 1], pts[i]))).collect();
    // Sizes: what each item sees, and the size at each vertex and at the end.
    let mut items: Vec<&Item> = l.items.iter().filter(|i| i.seg < n).collect();
    items.sort_by(|a, b| (a.seg, a.at).partial_cmp(&(b.seg, b.at)).unwrap_or(std::cmp::Ordering::Equal));
    let mut cur = l.nps.clone();
    let mut item_nps: HashMap<u32, (String, String)> = HashMap::new();
    let mut vert_nps = vec![l.nps.clone(); n + 1];
    let mut k = 0;
    for v in 0..=n {
        while k < items.len() && items[k].seg < v {
            let it = items[k];
            let after = if matches!(it.kind, ItemKind::Reducer) { reducer_to(it, &cur) } else { cur.clone() };
            item_nps.insert(it.id, (cur.clone(), after.clone()));
            cur = after;
            k += 1;
        }
        vert_nps[v] = cur.clone();
    }
    let dim = |nps: &str| dimens(cat, nps, &l.class, l.rating, l.radius);

    // End allowances.
    let end_len = |e: &End, nps: &str| -> f64 {
        let d = dim(nps);
        match e {
            End::Point { term: Term::Open } | End::Branch { .. } => 0.0,
            End::Point { term: Term::Cap } => d.cap,
            End::Point { term: Term::Blind } => d.fl_t + d.gasket + d.fl_y,
            End::Nozzle { .. } => match end_port(p, cat, e) {
                Some(np) => { let dn = dim(&np.nps); dn.gasket + dn.fl_y + if np.nps != nps { dim(&larger(&np.nps, nps)).red } else { 0.0 } }
                None => 0.0,
            },
        }
    };
    let mut s0 = vec![0.0; n];
    let mut s1 = vec![0.0; n];
    s0[0] = end_len(&l.from, &vert_nps[0]);
    s1[n - 1] = end_len(&l.to, &vert_nps[n]);
    if n == 1 && s0[0] + s1[0] > lens[0] + 1e-9 {
        b.issues.push(issue("routing", format!("Line {}: too short for the fittings at its ends.", name(l)), pts[0], l.id, 0));
    }
    // Elbows.
    let mut elbows: Vec<Option<(f64, f64, &'static str)>> = vec![None; n + 1];
    for v in 1..n {
        let (ui, uo) = (dirs[v - 1], dirs[v]);
        let th = dot(ui, uo).clamp(-1.0, 1.0).acos();
        let deg = th.to_degrees();
        if deg < 0.5 { continue; }
        let kind = if (deg - 90.0).abs() <= 0.5 { "elbow90" } else if (deg - 45.0).abs() <= 0.5 { "elbow45" } else {
            b.issues.push(issue("routing", format!("Line {}: a turn of {:.1}° at point {}; the class has 90° and 45° elbows only, so no elbow is drawn there.", name(l), deg, v + 1), pts[v], l.id, 0));
            continue;
        };
        let d = dim(&vert_nps[v]);
        let a = d.elbow * (th / 2.0).tan();
        // Room on both sides: what the segment before has left after its start, and what the
        // segment after has before its end fittings (the next turn checks its own room).
        let room_before = lens[v - 1] - s0[v - 1];
        let room_after = lens[v] - s1[v];
        if a > room_before + 1e-9 || a > room_after + 1e-9 {
            b.issues.push(issue("routing", format!("Line {}: the turn at point {} is too tight for a {} {} elbow (needs {:.0} mm each side); it is not drawn.", name(l), v + 1, if kind == "elbow90" { "90°" } else { "45°" }, if l.radius == Radius::Short { "short-radius" } else { "long-radius" }, a * 1000.0), pts[v], l.id, 0));
            continue;
        }
        s1[v - 1] = a;
        s0[v] = a;
        elbows[v] = Some((a, th, kind));
    }

    // Each segment: end pieces, items, pipes between.
    for i in 0..n {
        let mut run = Run { l, a: pts[i], u: dirs[i], seg: i as i32, out: &mut b.parts };
        let mut cursor = 0.0;
        let nps_here = vert_nps[i].clone();
        // The start of the line.
        if i == 0 {
            cursor = start_end(p, cat, &mut run, &l.from, &nps_here, false);
        } else {
            cursor = s0[i].max(cursor);
        }
        let end_room = lens[i] - s1[i];
        let mut size = nps_here.clone();
        for it in items.iter().filter(|it| it.seg == i) {
            let (nin, nout) = item_nps[&it.id].clone();
            let len = item_len(cat, l, it, &nin);
            let (x0, x1) = (it.at - len / 2.0, it.at + len / 2.0);
            if x0 < cursor - 1e-9 || x1 > end_room + 1e-9 {
                b.issues.push(issue("routing", format!("Line {}: the {} does not fit where it is ({:.0} mm long; move it or lengthen the run); it is not drawn.", name(l), it.kind.name().to_lowercase(), len * 1000.0), run.at(it.at), l.id, it.id));
                size = nout;
                continue;
            }
            run.pipe(cursor, x0, &dim(&size), &size);
            place_item(p, cat, &mut run, it, x0, &nin, &nout);
            cursor = x1;
            size = nout;
        }
        let end_size = size.clone();
        if i == n - 1 {
            // Pipe up to the end fittings, then the end fittings.
            let e0 = lens[i] - s1[i];
            run.pipe(cursor, e0, &dim(&end_size), &end_size);
            let mut back = Run { l, a: pts[n], u: mul(dirs[i], -1.0), seg: i as i32, out: run.out };
            start_end(p, cat, &mut back, &l.to, &end_size, true);
        } else {
            run.pipe(cursor, end_room, &dim(&end_size), &end_size);
        }
        // The elbow at the end of this segment.
        if i + 1 < n {
            if let Some((a, th, kind)) = elbows[i + 1] {
                let v = i + 1;
                let d = dim(&vert_nps[v]);
                let (ui, uo) = (dirs[i], dirs[v]);
                let s = sub(pts[v], mul(ui, a));
                let e = add(pts[v], mul(uo, a));
                let inward = unit(sub(uo, mul(ui, dot(uo, ui))));
                let c = add(s, mul(inward, d.elbow));
                let ax = unit(cross(ui, uo));
                let mut q = Part::new(kind, l);
                q.nps = vert_nps[v].clone();
                q.shapes = vec![Shape::Torus { centre: c, axis: ax, start: sub(s, c), angle: th, r: d.od / 2.0 }];
                q.ports = vec![Port { p: s, d: mul(ui, -1.0), nps: q.nps.clone(), rating: 0, end: "BW" }, Port { p: e, d: uo, nps: q.nps.clone(), rating: 0, end: "BW" }];
                q.length = d.elbow * th;
                q.corner = Some(pts[v]);
                q.note = if l.radius == Radius::Short && cat.fitting(&q.nps).and_then(|f| f.elbow_sr).is_some() { "SR".into() } else { "LR".into() };
                q.seg = i as i32;
                b.parts.push(q);
            }
        }
    }
}

/// The fittings at a line end, going inward along the run's +u from its start. Returns where the
/// pipe starts.
fn start_end(p: &Project, cat: &Catalogue, run: &mut Run, e: &End, nps: &str, _back: bool) -> f64 {
    let l = run.l;
    let d = dimens(cat, nps, &l.class, l.rating, l.radius);
    match e {
        End::Point { term: Term::Open } | End::Branch { .. } => 0.0,
        End::Point { term: Term::Cap } => {
            let mut q = Part::new("cap", l);
            q.nps = nps.into();
            q.shapes = vec![Shape::tube(run.at(d.cap * 0.35), run.at(d.cap), d.od / 2.0), Shape::Head { base: run.at(d.cap * 0.35), dir: mul(run.u, -1.0), r: d.od / 2.0, depth: d.cap * 0.35 }];
            q.ports = vec![run.port(d.cap, false, nps, 0, "BW")];
            q.length = d.cap;
            run.push(q, 0);
            d.cap
        }
        End::Point { term: Term::Blind } => {
            let mut q = Part::new("blind", l);
            q.nps = nps.into();
            q.rating = l.rating;
            q.shapes = vec![Shape::tube(run.at(0.0), run.at(d.fl_t), d.fl_od / 2.0)];
            q.ports = vec![run.port(d.fl_t, false, nps, l.rating, "FL")];
            q.length = d.fl_t;
            run.push(q, 0);
            run.gasket(d.fl_t, &d, nps, 0);
            run.flange(d.fl_t + d.gasket, &d, nps, true, 0);
            d.fl_t + d.gasket + d.fl_y
        }
        End::Nozzle { .. } => {
            let Some(np) = end_port(p, cat, e) else { return 0.0 };
            let dn = dimens(cat, &np.nps, &l.class, l.rating, l.radius);
            run.gasket(0.0, &dn, &np.nps, 0);
            run.flange(dn.gasket, &dn, &np.nps, true, 0);
            let mut s = dn.gasket + dn.fl_y;
            if np.nps != nps {
                let h = dimens(cat, &larger(&np.nps, nps), &l.class, l.rating, l.radius).red;
                reducer(cat, run, s, h, &np.nps, nps, 0);
                s += h;
            }
            s
        }
    }
}

fn reducer(cat: &Catalogue, run: &mut Run, s0: f64, h: f64, a: &str, b: &str, item: u32) {
    let (oa, ob) = (cat.od(a), cat.od(b));
    let mut q = Part::new("reducer", run.l);
    q.nps = larger(a, b);
    q.nps2 = if q.nps == a { b.into() } else { a.into() };
    let flat = h * 0.2;
    q.shapes = vec![Shape::tube(run.at(s0), run.at(s0 + flat), oa / 2.0), Shape::Frustum { a: run.at(s0 + flat), b: run.at(s0 + h - flat), r1: oa / 2.0, r2: ob / 2.0 }, Shape::tube(run.at(s0 + h - flat), run.at(s0 + h), ob / 2.0)];
    q.ports = vec![run.port(s0, true, a, 0, "BW"), run.port(s0 + h, false, b, 0, "BW")];
    q.length = h;
    run.push(q, item);
}

fn place_item(p: &Project, cat: &Catalogue, run: &mut Run, it: &Item, x0: f64, nin: &str, nout: &str) {
    let l = run.l;
    let d = dimens(cat, nin, &l.class, l.rating, l.radius);
    match it.kind {
        ItemKind::Reducer => {
            let h = item_len(cat, l, it, nin);
            reducer(cat, run, x0, h, nin, nout, it.id);
        }
        ItemKind::Tee => {
            let c = x0 + d.tee;
            let bd = branch_dir(run.u, it.dir);
            let bn = branch_size(p, l, it);
            let ob = cat.od(&bn);
            let mut q = Part::new("tee", l);
            q.nps = nin.into();
            q.nps2 = bn.clone();
            let cp = run.at(c);
            q.shapes = vec![Shape::tube(run.at(x0), run.at(x0 + 2.0 * d.tee), d.od / 2.0), Shape::tube(cp, add(cp, mul(bd, d.tee)), ob / 2.0)];
            q.ports = vec![run.port(x0, true, nin, 0, "BW"), run.port(x0 + 2.0 * d.tee, false, nin, 0, "BW"), Port { p: add(cp, mul(bd, d.tee)), d: bd, nps: bn, rating: 0, end: "BW" }];
            q.length = 2.0 * d.tee;
            q.corner = Some(cp);
            run.push(q, it.id);
        }
        ItemKind::FlangePair => {
            run.flange(x0, &d, nin, false, it.id);
            run.gasket(x0 + d.fl_y, &d, nin, it.id);
            run.flange(x0 + d.fl_y + d.gasket, &d, nin, true, it.id);
        }
        k => {
            let body = f2f(cat, k, nin, l.rating, d.od);
            run.flange(x0, &d, nin, false, it.id);
            run.gasket(x0 + d.fl_y, &d, nin, it.id);
            let v0 = x0 + d.fl_y + d.gasket;
            let mut q = Part::new(k.key(), l);
            q.nps = nin.into();
            q.rating = l.rating;
            q.shapes = valve_shapes(run, k, v0, body, &d, it.dir);
            q.ports = vec![run.port(v0, true, nin, l.rating, "FL"), run.port(v0 + body, false, nin, l.rating, "FL")];
            q.length = body;
            run.push(q, it.id);
            run.gasket(v0 + body, &d, nin, it.id);
            run.flange(v0 + body + d.gasket, &d, nin, true, it.id);
        }
    }
}

/// A valve or strainer as a simplified body: flanged ends, a body, and a handwheel, lever or cover.
fn valve_shapes(run: &Run, k: ItemKind, v0: f64, len: f64, d: &Dimens, want: P3) -> Vec<Shape> {
    let r = d.od / 2.0;
    let (fr, ft) = (d.fl_od / 2.0, d.fl_t.min(len * 0.25));
    let mid = v0 + len / 2.0;
    let stem = branch_dir(run.u, if norm(want) > 0.0 { want } else { [0.0, 0.0, 1.0] });
    let c = run.at(mid);
    let mut s = vec![
        Shape::tube(run.at(v0), run.at(v0 + ft), fr),
        Shape::tube(run.at(v0 + len - ft), run.at(v0 + len), fr),
        Shape::Frustum { a: run.at(v0 + ft), b: c, r1: r * 1.05, r2: r * 1.6 },
        Shape::Frustum { a: c, b: run.at(v0 + len - ft), r1: r * 1.6, r2: r * 1.05 },
    ];
    match k {
        ItemKind::Gate | ItemKind::Globe => {
            let h = (r * 4.0).max(0.15);
            s.push(Shape::Frustum { a: add(c, mul(stem, r)), b: add(c, mul(stem, h * 0.8)), r1: r * 0.9, r2: r * 0.35 });
            s.push(Shape::tube(add(c, mul(stem, h * 0.8)), add(c, mul(stem, h * 1.05)), (r * 0.08).max(0.006)));
            let (u1, _) = frame(stem);
            s.push(Shape::Torus { centre: add(c, mul(stem, h * 1.05)), axis: stem, start: mul(u1, (r * 1.1).max(0.06)), angle: std::f64::consts::TAU, r: (r * 0.08).max(0.006) });
        }
        ItemKind::Ball => {
            s.push(Shape::tube(add(c, mul(stem, r)), add(c, mul(stem, r * 1.6)), (r * 0.12).max(0.006)));
            let lever = rotate(run.u, stem, 0.0);
            s.push(Shape::Cuboid { centre: add(add(c, mul(stem, r * 1.65)), mul(lever, r * 1.6)), axes: [lever, stem, cross(lever, stem)], half: [r * 1.8, (r * 0.06).max(0.004), (r * 0.15).max(0.006)] });
        }
        ItemKind::Check => {
            s.push(Shape::Cuboid { centre: add(c, mul(stem, r * 1.4)), axes: [run.u, stem, cross(run.u, stem)], half: [len * 0.22, r * 0.25, r * 0.9] });
        }
        ItemKind::Strainer => {
            let leg = unit(add(mul(stem, -1.0), mul(run.u, 0.8)));
            s.push(Shape::Frustum { a: c, b: add(c, mul(leg, r * 3.2)), r1: r * 0.9, r2: r * 0.8 });
        }
        _ => {}
    }
    s
}

impl Project {
    fn class_missing(&self, cat: &Catalogue, class: &str) -> bool {
        cat.class(class).is_none()
    }
}

impl Catalogue {
    /// The built-in catalogue, made once (for places that only need standard outside diameters).
    pub fn builtin_cached() -> &'static Catalogue {
        static C: std::sync::OnceLock<Catalogue> = std::sync::OnceLock::new();
        C.get_or_init(Catalogue::builtin)
    }
}

// ---------------------------------------------------------------- snapping and routing helpers

/// The nearest place on a line's path to a point: (segment, distance from its start, the point).
pub fn nearest(p: &Project, cat: &Catalogue, l: &Line, q: P3) -> Option<(usize, f64, P3)> {
    let pts = path(p, cat, l);
    let mut best: Option<(f64, usize, f64, P3)> = None;
    for i in 0..pts.len().saturating_sub(1) {
        let (a, b) = (pts[i], pts[i + 1]);
        let ab = sub(b, a);
        let len = norm(ab);
        if len < 1e-9 { continue; }
        let t = (dot(sub(q, a), ab) / (len * len)).clamp(0.0, 1.0);
        let x = add(a, mul(ab, t));
        let dd = dist(x, q);
        if best.is_none_or(|bb| dd < bb.0) { best = Some((dd, i, t * len, x)); }
    }
    best.map(|b| (b.1, b.2, b.3))
}

/// The parts of one line.
pub fn build_line(p: &Project, cat: &Catalogue, l: &Line) -> Vec<Part> {
    let mut b = Built::default();
    line(p, cat, l, &mut b);
    b.parts
}

/// Where a component of length `len` can sit on segment `seg` near `at`: the nearest free place
/// between the segment's elbows, end fittings and other components (snapped to `step` metres),
/// or None when no gap is long enough. `skip` is a component being moved (its own place is free).
pub fn free_place(p: &Project, cat: &Catalogue, l: &Line, seg: usize, at: f64, len: f64, step: f64, skip: u32) -> Option<f64> {
    let pts = path(p, cat, l);
    let (a, e) = (*pts.get(seg)?, *pts.get(seg + 1)?);
    let u = unit(sub(e, a));
    let seg_len = dist(a, e);
    let on_axis = |q: P3| { let s = dot(sub(q, a), u); (dist(q, add(a, mul(u, s))) < 1e-6 && (-1e-6..=seg_len + 1e-6).contains(&s)).then_some(s) };
    let mut iv: Vec<(f64, f64)> = vec![];
    for q in build_line(p, cat, l).iter().filter(|q| !matches!(q.kind, "pipe" | "bolts") && (q.item == 0 || q.item != skip)) {
        let elbow = q.kind.starts_with("elbow");
        if !(q.seg == seg as i32 || (elbow && q.seg + 1 == seg as i32)) { continue; }
        let ss: Vec<f64> = q.ports.iter().filter_map(|pt| on_axis(pt.p)).collect();
        if ss.is_empty() { continue; }
        let lo = ss.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = ss.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        iv.push(if elbow { if q.seg == seg as i32 { (lo, seg_len) } else { (0.0, hi) } } else if ss.len() == 1 { if lo < seg_len / 2.0 { (0.0, hi) } else { (lo, seg_len) } } else { (lo, hi) });
    }
    iv.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut gaps = vec![];
    let mut c = 0.0;
    for (lo, hi) in iv { if lo > c { gaps.push((c, lo)); } c = c.max(hi); }
    if seg_len > c { gaps.push((c, seg_len)); }
    let snap = |x: f64| if step > 0.0 { (x / step).round() * step } else { x };
    let mut best: Option<f64> = None;
    for (g0, g1) in gaps {
        if g1 - g0 + 1e-9 < len { continue; }
        let (lo, hi) = (g0 + len / 2.0, g1 - len / 2.0);
        let mut x = snap(at.clamp(lo, hi));
        if x < lo - 1e-9 || x > hi + 1e-9 { x = at.clamp(lo, hi); }
        if best.is_none_or(|b| (x - at).abs() < (b - at).abs()) { best = Some(x); }
    }
    best
}

/// An orthogonal path from a start port (or point) through `via` points to an end: leaves the
/// start along its direction by `lead`, runs east–west, north–south and up–down legs, and arrives
/// at the end along its direction. Leg orders that would double back are avoided. Corner points
/// only; no zero-length legs.
pub fn orthogonal(start: (P3, Option<P3>), via: &[P3], end: (P3, Option<P3>), lead: f64) -> Vec<P3> {
    let mut pts = vec![start.0];
    if let Some(d) = start.1 { pts.push(add(start.0, mul(d, lead))); }
    let end_lead = end.1.map(|d| add(end.0, mul(d, lead)));
    let mut targets: Vec<(P3, bool)> = via.iter().map(|v| (*v, false)).collect();
    if let Some(t) = end_lead { targets.push((t, true)); }
    targets.push((end.0, false));
    const ORDERS: [[usize; 3]; 6] = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];
    let reverses = |a: P3, b: P3| dot(unit(a), unit(b)) < -0.5;
    for (t, last_lead) in targets {
        let c = *pts.last().unwrap_or(&t);
        let last_dir = if pts.len() >= 2 { sub(c, pts[pts.len() - 2]) } else { [0.0; 3] };
        // The leg after this target: into the end along the end's direction.
        let next_dir = if last_lead { end.1.map(|d| mul(d, -1.0)) } else { None };
        let axis = |d: P3| (0..3).max_by(|&i, &j| d[i].abs().total_cmp(&d[j].abs())).unwrap_or(0);
        let score = |o: &[usize; 3]| -> (bool, i32) {
            let mut q = c;
            let mut prev = last_dir;
            let mut ok = true;
            let mut legs = vec![];
            for &k in o { if (t[k] - q[k]).abs() > 1e-9 { let mut n = q; n[k] = t[k]; legs.push(sub(n, q)); q = n; } }
            if let Some(nd) = next_dir { legs.push(nd); }
            for l in &legs { if norm(prev) > 0.0 && reverses(prev, *l) { ok = false; } prev = *l; }
            // Prefer carrying on along the current direction, then the end's axis first.
            let pref = (if norm(last_dir) > 0.0 && o[0] == axis(last_dir) { 2 } else { 0 }) + (if next_dir.is_some_and(|d| o[0] == axis(d)) { 1 } else { 0 });
            (ok, pref)
        };
        let best = ORDERS.iter().max_by_key(|o| { let (ok, pref) = score(o); (ok, pref) }).copied().unwrap_or([0, 1, 2]);
        let mut q = c;
        for k in best { if (t[k] - q[k]).abs() > 1e-9 { q[k] = t[k]; pts.push(q); } }
    }
    pts.dedup_by(|a, b| dist(*a, *b) < 1e-9);
    let mut out: Vec<P3> = vec![];
    for p in pts {
        if out.len() >= 2 {
            let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
            if norm(cross(sub(b, a), sub(p, b))) < 1e-9 && dot(sub(b, a), sub(p, b)) > 0.0 { out.pop(); }
        }
        out.push(p);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orthogonal_routes_never_double_back() {
        // Side nozzle to a top nozzle higher up: out, up, across, down.
        let p = orthogonal(([3.0, 10.0, 1.9], Some([1.0, 0.0, 0.0])), &[], ([8.25, 10.0, 1.9], Some([0.0, 0.0, 1.0])), 0.55);
        assert_eq!(p, vec![[3.0, 10.0, 1.9], [3.55, 10.0, 1.9], [3.55, 10.0, 2.45], [8.25, 10.0, 2.45], [8.25, 10.0, 1.9]]);
        // Facing nozzles offset sideways.
        let p = orthogonal(([0.0, 0.0, 1.0], Some([1.0, 0.0, 0.0])), &[], ([5.0, 2.0, 1.0], Some([-1.0, 0.0, 0.0])), 0.5);
        for w in p.windows(3) { assert!(dot(sub(w[1], w[0]), sub(w[2], w[1])) >= 0.0, "{p:?}"); }
        for w in p.windows(2) { assert_eq!((0..3).filter(|&k| (w[1][k] - w[0][k]).abs() > 1e-12).count(), 1, "{p:?}"); }
    }
}
