//! Edits: every change to the model is a named operation, applied to a copy and kept only when it
//! succeeds, so a failed edit leaves the model exactly as it was. Undo and redo keep the model as
//! it was before each operation.

use crate::catalogue::{Catalogue, NPS};
use crate::equipment;
use crate::model::{Area, End, EquipKind, Equipment, Item, ItemKind, Line, Plot, Project, Settings, Term, P3};
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
    /// A component dropped near `at`: it snaps onto the nearest free place on the line, takes the
    /// line's size and class, and splits the pipe.
    Drop { line: u32, kind: ItemKind, at: P3, dir: P3, nps2: String },
    MoveItem { line: u32, item: u32, at: P3 },
    SetItem { line: u32, item: Item },
    /// Sets a line's size: every component on it re-sizes.
    Resize { line: u32, nps: String },
    SetTerm { line: u32, start: bool, term: Term },
    Delete { ids: Vec<u32> },
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
        if p != self.p {
            self.undo.push(std::mem::replace(&mut self.p, p));
            if self.undo.len() > UNDO { self.undo.remove(0); }
            self.redo.clear();
            self.dirty = true;
        }
        Ok(created)
    }
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
            let lead = |d: Option<P3>| d.map(|_| {
                let od = cat.od(&nps);
                let lr = cat.fitting(&nps).map(|f| f.elbow_lr).unwrap_or(od * 1.5);
                let fl = cat.flange(&nps, p.settings.rating).map(|f| f.wn_length).unwrap_or(od * 0.5);
                let h = cat.fitting(&nps).map(|f| f.reducer_h).unwrap_or(od);
                ((lr + fl + h + 0.05) / 0.05).ceil() * 0.05
            }).unwrap_or(0.0);
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
        Op::Drop { line, kind, at, dir, nps2 } => {
            let l = p.line(line).ok_or("That line no longer exists.")?.clone();
            let (seg, s, _) = route::nearest(p, cat, &l, at).ok_or("The line has no route to place it on.")?;
            let nps = route::size_at(&l, seg, s);
            let mut it = Item { id: 0, uid: String::new(), kind, seg, at: s, nps2: nps2.clone(), dir, tag: String::new() };
            if kind == ItemKind::Reducer && it.nps2.is_empty() { it.nps2 = route::reducer_to(&it, &nps); }
            if !it.nps2.is_empty() { valid_nps(&it.nps2)?; }
            let len = route::item_len(cat, &l, &it, &nps);
            let at = route::free_place(p, cat, &l, seg, s, len, 0.01, 0).ok_or_else(|| format!("No room for a {nps}\" {} ({:.0} mm) on this run: it needs a straight piece at least that long between fittings.", kind.name().to_lowercase(), len * 1000.0))?;
            it.at = at;
            it.id = p.id();
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
        // Deleting the pump frees the line's end.
        d.apply(Op::Delete { ids: vec![pid] }, &cat).unwrap();
        assert!(matches!(d.p.lines[0].from, End::Point { .. }));
    }
}
