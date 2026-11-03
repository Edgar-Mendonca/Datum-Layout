//! File › Examples: three worked plants, each opening with no clashes and no errors, and a copy
//! of the third with three planted problems for demonstrations. Built in code through the same
//! operations the window uses, so they always match the engine.

use crate::catalogue::Catalogue;
use crate::doc::{Doc, Op};
use crate::model::{Area, End, EquipKind, Item, ItemKind, Line, Nozzle, Project, Term, P3};
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct Example { pub id: &'static str, pub name: &'static str, pub what: &'static str }

pub fn list() -> Vec<Example> {
    vec![
        Example { id: "pumps", name: "Pump suction and discharge", what: "A tank, a duty and standby pump pair with suction strainers, discharge check and gate valves, to a header." },
        Example { id: "exchanger", name: "Exchanger hook-up", what: "A shell-and-tube exchanger with inlet and outlet lines, isolation valves and a bypass with a globe valve." },
        Example { id: "rack", name: "Small pipe rack run", what: "Two lines on a rack between two vessels, with elevation changes and a tee branch." },
        Example { id: "clashes", name: "Find the clashes", what: "The pipe rack run with three planted problems: run Check › Clashes to find them." },
    ]
}

pub fn get(id: &str) -> Option<Project> {
    let cat = Catalogue::builtin();
    let p = match id {
        "pumps" => pumps(&cat),
        "exchanger" => exchanger(&cat),
        "rack" => rack(&cat, false),
        "clashes" => rack(&cat, true),
        _ => return None,
    };
    Some(p)
}

struct B<'a> { d: Doc, cat: &'a Catalogue }

impl B<'_> {
    fn op(&mut self, op: Op) -> Vec<u32> {
        let what = format!("{op:?}");
        self.d.apply(op, self.cat).unwrap_or_else(|e| panic!("example step failed: {e}\n{what}"))
    }
    fn equip(&mut self, kind: EquipKind, tag: &str, pos: P3, edit: impl FnOnce(&mut crate::model::Equipment)) -> u32 {
        let id = self.op(Op::AddEquipment { kind, pos })[0];
        let mut e = self.d.p.equip(id).cloned().unwrap_or_default();
        e.tag = tag.into();
        edit(&mut e);
        self.op(Op::SetEquipment(e));
        id
    }
    /// A line with an explicit path (its nozzle and tee ends are snapped to their ports).
    fn line(&mut self, number: &str, nps: &str, from: End, to: End, pts: Vec<P3>) -> u32 {
        let id = self.d.p.id();
        let fluid = number.split('-').nth(1).unwrap_or("P").to_string();
        self.d.p.lines.push(Line { id, number: number.into(), nps: nps.into(), fluid, from, to, pts, ..Default::default() });
        self.d.p.normalize();
        id
    }
    fn drop(&mut self, line: u32, kind: ItemKind, at: P3, dir: P3) -> u32 {
        self.op(Op::Drop { line, kind, at, dir, nps2: String::new() })[0]
    }
    fn port(&self, e: End) -> P3 {
        crate::route::end_port(&self.d.p, self.cat, &e).map(|p| p.p).unwrap_or_default()
    }
    fn done(mut self, name: &str, notes: &str) -> Project {
        self.d.p.name = name.into();
        self.d.p.notes = notes.into();
        self.d.p.normalize();
        self.d.p
    }
}

fn noz(equip: u32, n: &str) -> End { End::Nozzle { equip, nozzle: n.into() } }
fn blind() -> End { End::Point { term: Term::Blind } }
fn cap() -> End { End::Point { term: Term::Cap } }
fn br(line: u32, item: u32) -> End { End::Branch { line, item } }

fn new_b(cat: &Catalogue) -> B<'_> {
    let mut p = Project::new();
    p.plot.e = [0.0, 30.0];
    p.plot.n = [0.0, 20.0];
    B { d: Doc::new(p), cat }
}

/// Example 1: a tank, a duty/standby pump pair, suction strainers, discharge check and gate
/// valves, to a header.
fn pumps(cat: &Catalogue) -> Project {
    let mut b = new_b(cat);
    b.op(Op::AddArea(Area { name: "Pump area".into(), unit: "Unit 100".into(), min: [11.0, 6.0, 0.0], max: [16.0, 14.0, 6.0], ..Default::default() }));
    let t = b.equip(EquipKind::Tank, "T-101", [5.0, 10.0, 0.0], |_| {});
    let pa = b.equip(EquipKind::Pump, "P-101A", [14.0, 12.0, 0.0], |_| {});
    let pb = b.equip(EquipKind::Pump, "P-101B", [14.0, 8.0, 0.0], |_| {});
    // Suction: 8" from the tank to P-101A, with a tee to P-101B; the 6" pump nozzles get reducers.
    let tn = b.port(noz(t, "N1"));
    let sa = b.port(noz(pa, "S"));
    let s1 = b.line("8\"-P-1001-CS1", "8", noz(t, "N1"), noz(pa, "S"), vec![tn, [12.4, tn[1], tn[2]], [12.4, sa[1], sa[2]], sa]);
    let tee = b.drop(s1, ItemKind::Tee, [10.0, 10.0, 0.6], [0.0, -1.0, 0.0]);
    b.drop(s1, ItemKind::Strainer, [12.4, 11.0, 0.6], [0.0, 0.0, 1.0]);
    let sb = b.port(noz(pb, "S"));
    let bp = b.port(br(s1, tee));
    let s2 = b.line("8\"-P-1002-CS1", "8", br(s1, tee), noz(pb, "S"), vec![bp, [bp[0], sb[1], sb[2]], sb]);
    b.drop(s2, ItemKind::Strainer, [bp[0], 9.0, 0.6], [0.0, 0.0, 1.0]);
    // Discharge header: 6", capped at the south end, to a blind flange at the battery limit.
    let h = b.line("6\"-P-1005-CS1", "6", cap(), blind(), vec![[13.4, 6.5, 4.5], [13.4, 15.0, 4.5], [22.0, 15.0, 4.5]]);
    for (pump, y, n) in [(pa, 12.0, "1003"), (pb, 8.0, "1004")] {
        let tee = b.drop(h, ItemKind::Tee, [13.4, y, 4.5], [0.0, 0.0, -1.0]);
        let top = b.port(br(h, tee));
        let d = b.port(noz(pump, "D"));
        let l = b.line(&format!("4\"-P-{n}-CS1"), "4", br(h, tee), noz(pump, "D"), vec![top, d]);
        b.drop(l, ItemKind::Gate, [13.4, y, 3.1], [1.0, 0.0, 0.0]);
        b.drop(l, ItemKind::Check, [13.4, y, 2.0], [1.0, 0.0, 0.0]);
    }
    b.done("Pump suction and discharge", "Example 1. A tank feeds a duty and standby pump pair through 8\" suction lines with strainers; each pump discharges through a check valve and a gate valve to a 6\" header.")
}

/// Example 2: a shell-and-tube exchanger with inlet and outlet lines, isolation valves and a bypass.
fn exchanger(cat: &Catalogue) -> Project {
    let mut b = new_b(cat);
    let e = b.equip(EquipKind::Exchanger, "E-201", [10.0, 10.0, 0.0], |_| {});
    let n1 = b.port(noz(e, "N1"));
    let n2 = b.port(noz(e, "N2"));
    let n3 = b.port(noz(e, "N3"));
    let n4 = b.port(noz(e, "N4"));
    // Shell side in: from the battery limit at 3 m, down onto the shell inlet.
    let inl = b.line("6\"-P-2001-CS1", "6", blind(), noz(e, "N1"), vec![[3.0, 10.0, 3.0], [n1[0], 10.0, 3.0], n1]);
    let t_in = b.drop(inl, ItemKind::Tee, [4.5, 10.0, 3.0], [0.0, 1.0, 0.0]);
    b.drop(inl, ItemKind::Gate, [6.5, 10.0, 3.0], [0.0, 0.0, 1.0]);
    // Shell side out: down, north under the exchanger's end, east, up to the rack and out.
    let out = b.line("6\"-P-2002-CS1", "6", noz(e, "N2"), blind(), vec![n2, [n2[0], 10.0, 0.45], [n2[0], 12.5, 0.45], [16.5, 12.5, 0.45], [16.5, 12.5, 3.0], [19.5, 12.5, 3.0]]);
    b.drop(out, ItemKind::Gate, [16.5, 12.5, 1.8], [1.0, 0.0, 0.0]);
    let t_out = b.drop(out, ItemKind::Tee, [18.0, 12.5, 3.0], [0.0, -1.0, 0.0]);
    // The bypass between the two tees, with a globe valve.
    let a = b.port(br(inl, t_in));
    let z = b.port(br(out, t_out));
    let by = b.line("6\"-P-2003-CS1", "6", br(inl, t_in), br(out, t_out), vec![a, [a[0], 11.6, 3.0], [z[0], 11.6, 3.0], z]);
    b.drop(by, ItemKind::Globe, [11.0, 11.6, 3.0], [0.0, 0.0, 1.0]);
    // Tube side: cooling water supply and return from the south, each with a gate valve.
    let cws = b.line("4\"-CW-2004-CS1", "4", blind(), noz(e, "N3"), vec![[n3[0], 5.0, 3.5], [n3[0], 10.0, 3.5], n3]);
    b.drop(cws, ItemKind::Gate, [n3[0], 7.0, 3.5], [1.0, 0.0, 0.0]);
    let cwr = b.line("4\"-CW-2005-CS1", "4", noz(e, "N4"), blind(), vec![n4, [n4[0], 10.0, 0.5], [n4[0], 5.0, 0.5]]);
    b.drop(cwr, ItemKind::Gate, [n4[0], 7.0, 0.5], [1.0, 0.0, 0.0]);
    b.done("Exchanger hook-up", "Example 2. A shell-and-tube exchanger: the process side enters and leaves through gate valves, with a bypass through a globe valve between two tees; cooling water comes in and out from the south.")
}

/// Example 3: two lines on a rack between two vessels, with elevation changes and a tee branch.
/// With `planted`: the "Find the clashes" copy.
fn rack(cat: &Catalogue, planted: bool) -> Project {
    let mut b = new_b(cat);
    b.op(Op::AddArea(Area { name: "Pipe rack".into(), unit: "Unit 300".into(), min: [4.0, 9.0, 4.0], max: [24.0, 12.2, 6.5], ..Default::default() }));
    let side4 = Nozzle { tag: "N4".into(), nps: "4".into(), pos: [0.0, 1.0, 4.5], dir: [0.0, 1.0, 0.0], service: "Product out".into(), ..Default::default() };
    let v1 = b.equip(EquipKind::VerticalVessel, "V-301", [3.0, 10.0, 0.0], |e| e.nozzles.push(side4.clone()));
    let v2 = b.equip(EquipKind::VerticalVessel, "V-302", [25.0, 10.0, 0.0], |e| {
        e.nozzles[1].pos = [-1.0, 0.0, 1.9];
        e.nozzles[1].dir = [-1.0, 0.0, 0.0];
        e.nozzles[2].pos = [1.0, 0.0, 5.0];
        e.nozzles[2].dir = [1.0, 0.0, 0.0];
        e.nozzles.push(Nozzle { tag: "N4".into(), nps: "4".into(), pos: [0.0, 1.0, 3.5], dir: [0.0, 1.0, 0.0], service: "Product in".into(), ..Default::default() });
    });
    let a0 = b.port(noz(v1, "N2"));
    let a1 = b.port(noz(v2, "N2"));
    // Line A, 6": up from V-301 to the rack at 5 m, along it, and down into V-302.
    let la = b.line("6\"-P-3001-CS1", "6", noz(v1, "N2"), noz(v2, "N2"), vec![a0, [4.6, 10.0, a0[2]], [4.6, 10.0, 5.0], [23.4, 10.0, 5.0], [23.4, 10.0, a1[2]], a1]);
    b.drop(la, ItemKind::Gate, [4.6, 10.0, 3.2], [1.0, 0.0, 0.0]);
    // Line B, 4": from V-301's north side up to the rack's upper level at 6 m, and down to V-302.
    let b0 = b.port(noz(v1, "N4"));
    let b1 = b.port(noz(v2, "N4"));
    let lb = b.line("4\"-P-3002-CS1", "4", noz(v1, "N4"), noz(v2, "N4"), vec![b0, [3.0, 11.6, b0[2]], [3.0, 11.6, 6.0], [25.0, 11.6, 6.0], [25.0, 11.6, b1[2]], b1]);
    b.drop(lb, ItemKind::Globe, [25.0, 11.6, 4.6], [1.0, 0.0, 0.0]);
    // A 3" branch off line A down to a drain point with a gate valve and a blind flange.
    let t = b.drop(la, ItemKind::Tee, [14.0, 10.0, 5.0], [0.0, -1.0, 0.0]);
    let tp = b.port(br(la, t));
    let low = if planted { 0.01 } else { 1.0 };
    let lc = b.line("3\"-D-3003-CS1", "3", br(la, t), blind(), vec![tp, [14.0, 9.2, 5.0], [14.0, 9.2, low]]);
    b.drop(lc, ItemKind::Gate, [14.0, 9.2, 2.5], [1.0, 0.0, 0.0]);
    if planted {
        // A 2" utility line laid across line A at the rack level.
        b.line("2\"-U-3004-CS1", "2", blind(), blind(), vec![[10.0, 8.5, 5.0], [10.0, 11.0, 5.0]]);
        // A new vessel under line B's upper run.
        b.equip(EquipKind::VerticalVessel, "V-303", [17.0, 12.2, 0.0], |e| { e.dims.d = 1.2; e.dims.h = 4.5; e.nozzles.retain(|n| n.tag != "N1"); e.nozzles[0].pos = [0.8, 0.0, 1.9]; e.nozzles[1].pos = [-0.8, 0.0, 5.0]; });
        return b.done("Find the clashes", "A copy of the pipe rack run with three planted problems. Run Check › Clashes: a 2\" utility line crosses line A on the rack, the drain branch ends 10 mm above the ground, and vessel V-303 stands under line B's upper run.");
    }
    b.done("Small pipe rack run", "Example 3. Two lines run on a rack between two vessels: line A rises from V-301 to the rack at 5 m and drops into V-302; line B runs at 6 m. A 3\" branch drops from line A to a drain point.")
}

/// `copies` copies of an example side by side (six to a row), each with its own tags, numbers
/// and ids: the large plant of the speed checks.
pub fn repeated(id: &str, copies: usize) -> Option<Project> {
    let one = get(id)?;
    let mut p = Project::new();
    p.name = format!("{} × {copies}", one.name);
    for k in 0..copies {
        let mut q = one.clone();
        let (dx, dy) = ((k % 6) as f64 * 30.0, (k / 6) as f64 * 25.0);
        let base = p.max_id() + 1;
        for e in &mut q.equipment { e.id += base; e.uid.clear(); e.pos[0] += dx; e.pos[1] += dy; e.tag = format!("{}-{k}", e.tag); }
        for l in &mut q.lines {
            l.id += base; l.uid.clear(); l.number = format!("{}-{k}", l.number);
            for pt in &mut l.pts { pt[0] += dx; pt[1] += dy; }
            for it in &mut l.items { it.id += base; it.uid.clear(); }
            for e in [&mut l.from, &mut l.to] {
                match e { End::Nozzle { equip, .. } => *equip += base, End::Branch { line, item } => { *line += base; *item += base; } _ => {} }
            }
        }
        p.equipment.extend(q.equipment);
        p.lines.extend(q.lines);
    }
    p.plot.e = [0.0, 180.0];
    p.plot.n = [0.0, 25.0 * copies.div_ceil(6) as f64];
    p.normalize();
    Some(p)
}

/// Keeps items in the order they were placed (for a stable file).
pub fn tidy(p: &mut Project) {
    for l in &mut p.lines { l.items.sort_by(|a: &Item, b: &Item| (a.seg, a.at).partial_cmp(&(b.seg, b.at)).unwrap_or(std::cmp::Ordering::Equal)); }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks;
    use crate::route::build;

    #[test]
    fn every_example_opens_with_no_clashes_and_no_errors() {
        let cat = Catalogue::builtin();
        for ex in list().iter().filter(|e| e.id != "clashes") {
            let p = get(ex.id).unwrap();
            let b = build(&p, &cat);
            let issues = checks::run(&p, &cat, &b);
            assert!(issues.is_empty(), "{}: {:#?}", ex.name, issues);
            assert!(p.lines.len() >= 2 && !p.equipment.is_empty());
        }
    }

    #[test]
    fn find_the_clashes_finds_exactly_its_three_problems() {
        let cat = Catalogue::builtin();
        let p = get("clashes").unwrap();
        let b = build(&p, &cat);
        let issues = checks::run(&p, &cat, &b);
        assert_eq!(issues.len(), 3, "{issues:#?}");
        assert!(issues.iter().all(|i| i.kind == "clash"));
        let text: Vec<&str> = issues.iter().map(|i| i.text.as_str()).collect();
        assert!(text.iter().any(|t| t.starts_with("Pipe–pipe") && t.contains("2\"-U-3004") && t.contains("6\"-P-3001")), "{text:?}");
        assert!(text.iter().any(|t| t.starts_with("Pipe–ground") && t.contains("3\"-D-3003")), "{text:?}");
        assert!(text.iter().any(|t| t.starts_with("Pipe–equipment") && t.contains("V-303") && t.contains("4\"-P-3002")), "{text:?}");
    }
}
