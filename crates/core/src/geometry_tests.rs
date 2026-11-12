//! Geometry and routing tests (brief §4): parts match their tables, connected ports coincide and
//! face each other, routed lines have no gaps or overlaps, lengths add up, turns of 90° and 45°
//! work, tight turns are reported and not drawn, and resizing re-sizes everything.

use crate::catalogue::{Catalogue, NPS};
use crate::checks::{connection_issues, connections};
use crate::doc::{Doc, Op, RouteEnd};
use crate::model::{End, EquipKind, Item, ItemKind, Line, Project, Radius, Term, P3};
use crate::parts::{dist, dot, norm, sub, Shape};
use crate::route;
use crate::route::{build, Built};

fn line(nps: &str, pts: Vec<P3>, items: Vec<Item>) -> Project {
    let mut p = Project::new();
    p.lines.push(Line { id: 1, number: format!("{nps}\"-P-1-CS1"), nps: nps.into(), pts, items, ..Default::default() });
    p.normalize();
    p
}
fn item(id: u32, kind: ItemKind, seg: usize, at: f64) -> Item {
    Item { id, kind, seg, at, ..Default::default() }
}
fn of<'a>(b: &'a Built, kind: &str) -> Vec<&'a crate::route::Part> { b.parts.iter().filter(|q| q.kind == kind).collect() }

/// Every connection: positions within 0.01 mm and directions opposite.
fn joints_ok(b: &Built) {
    for (&(i, j), &(a, c)) in &connections(b) {
        let (x, y) = (&b.parts[i].ports[j], &b.parts[a].ports[c]);
        assert!(dist(x.p, y.p) <= 1e-5, "ports apart");
        assert!(dot(x.d, y.d) < -1.0 + 1e-9, "{} and {} meet at an angle: {:?} {:?} {:?} line {} {}", b.parts[i].kind, b.parts[a].kind, x.p, x.d, y.d, b.parts[i].line, b.parts[a].line);
    }
}

#[test]
fn every_generated_part_matches_its_table() {
    let cat = Catalogue::builtin();
    for (k, nps) in NPS.iter().enumerate() {
        let f = cat.fitting(nps).unwrap();
        let od = cat.pipe(nps, "CS1").unwrap().od;
        // A line east 10 m, north 10 m, with a gate valve, a tee and a reducer on the first run.
        let mut its = vec![item(10, ItemKind::Gate, 0, 3.0), item(11, ItemKind::Tee, 0, 6.0)];
        if k > 0 { its.push(item(12, ItemKind::Reducer, 1, 6.0)); }
        let p = line(nps, vec![[0.0, 0.0, 3.0], [10.0, 0.0, 3.0], [10.0, 10.0, 3.0]], its);
        let b = build(&p, &cat);
        assert!(b.issues.is_empty(), "{nps}: {:?}", b.issues);
        // Pipe: outside diameter.
        for q in of(&b, "pipe").iter().filter(|q| q.nps == *nps) {
            let Shape::Frustum { r1, r2, .. } = q.shapes[0] else { panic!() };
            assert!((2.0 * r1 - od).abs() < 1e-12 && r1 == r2);
        }
        // 90° elbow: centre-to-end equals the long radius from the table; arc radius too.
        let e = of(&b, "elbow90")[0];
        let corner = e.corner.unwrap();
        for pt in &e.ports { assert!((dist(pt.p, corner) - f.elbow_lr).abs() < 1e-9, "{nps} elbow centre-to-end"); }
        let Shape::Torus { start, r, .. } = e.shapes[0] else { panic!() };
        assert!((crate::parts::norm(start) - f.elbow_lr).abs() < 1e-9 && (2.0 * r - od).abs() < 1e-12);
        // Tee: centre to each end C.
        let t = of(&b, "tee")[0];
        for pt in &t.ports { assert!((dist(pt.p, t.corner.unwrap()) - f.tee_c).abs() < 1e-9, "{nps} tee"); }
        // Gate valve: face-to-face; flanges: outside diameter, length through hub.
        let v = of(&b, "gate")[0];
        assert!((dist(v.ports[0].p, v.ports[1].p) - cat.valve("gate", nps, 150).unwrap().f2f).abs() < 1e-9);
        let fl = cat.flange(nps, 150).unwrap();
        for q in of(&b, "flange") {
            assert!((dist(q.ports[0].p, q.ports[1].p) - fl.wn_length).abs() < 1e-9);
            let Shape::Frustum { r1, .. } = q.shapes[0] else { panic!() };
            assert!((2.0 * r1 - fl.od).abs() < 1e-12);
        }
        assert_eq!(of(&b, "gasket").len(), 2);
        assert_eq!(of(&b, "bolts")[0].bolts.unwrap().0, fl.bolts);
        if k > 0 {
            let r = of(&b, "reducer")[0];
            let big = NPS[k];
            assert!((dist(r.ports[0].p, r.ports[1].p) - cat.fitting(big).unwrap().reducer_h).abs() < 1e-9);
            assert_eq!((r.ports[0].nps.as_str(), r.ports[1].nps.as_str()), (*nps, NPS[k - 1]));
        }
        joints_ok(&b);
    }
}

/// The path's centreline length: the polyline less each drawn elbow's two cut-backs plus its arc.
fn centreline(b: &Built, pts: &[P3]) -> f64 {
    let poly: f64 = pts.windows(2).map(|w| dist(w[0], w[1])).sum();
    poly - b.parts.iter().filter(|q| q.kind.starts_with("elbow")).map(|q| 2.0 * dist(q.ports[0].p, q.corner.unwrap()) - q.length).sum::<f64>()
}

#[test]
fn a_routed_line_has_no_gaps_or_overlaps_and_its_lengths_add_up() {
    let cat = Catalogue::builtin();
    // Turns of 90° in every direction and two of 45°, with components on several runs.
    let pts = vec![[0.0, 0.0, 2.0], [6.0, 0.0, 2.0], [6.0, 5.0, 2.0], [6.0, 5.0, 6.0], [6.0, 9.0, 6.0], [9.0, 12.0, 6.0], [9.0, 16.0, 6.0], [2.0, 16.0, 6.0]];
    let items = vec![item(10, ItemKind::Globe, 0, 3.0), item(11, ItemKind::FlangePair, 1, 2.5), item(12, ItemKind::Check, 6, 3.5), item(13, ItemKind::Strainer, 2, 2.0)];
    let mut p = line("4", pts.clone(), items);
    p.lines[0].to = End::Point { term: Term::Cap };
    p.lines[0].from = End::Point { term: Term::Blind };
    let b = build(&p, &cat);
    assert!(b.issues.is_empty(), "{:?}", b.issues);
    assert_eq!(of(&b, "elbow90").len(), 4);
    assert_eq!(of(&b, "elbow45").len(), 2);
    joints_ok(&b);
    // Every port meets another: nothing is open, and no two parts overlap along the line.
    let conn = connections(&b);
    for (i, q) in b.parts.iter().enumerate() { for j in 0..q.ports.len() { assert!(conn.contains_key(&(i, j)), "{} port {j} is open", q.kind); } }
    assert!(connection_issues(&p, &cat, &b).is_empty(), "{:?}", connection_issues(&p, &cat, &b));
    let total: f64 = b.parts.iter().map(|q| q.length).sum();
    assert!((total - centreline(&b, &pts)).abs() < 1e-9, "{total} vs {}", centreline(&b, &pts));
}

#[test]
fn a_turn_too_tight_for_its_elbow_is_reported_not_drawn() {
    let cat = Catalogue::builtin();
    // 8" long radius needs 305 mm each side; the middle run is only 400 mm.
    let p = line("8", vec![[0.0, 0.0, 3.0], [5.0, 0.0, 3.0], [5.0, 0.45, 3.0], [9.0, 0.45, 3.0]], vec![]);
    let b = build(&p, &cat);
    assert_eq!(of(&b, "elbow90").len(), 1);
    assert_eq!(b.issues.len(), 1);
    assert!(b.issues[0].text.contains("too tight") && b.issues[0].text.contains("point 3"), "{}", b.issues[0].text);
    // Short radius fits (203 mm each side).
    let mut p2 = p.clone();
    p2.lines[0].radius = Radius::Short;
    let b2 = build(&p2, &cat);
    assert!(b2.issues.is_empty() && of(&b2, "elbow90").len() == 2 && of(&b2, "elbow90")[0].note == "SR");
    // A 60° turn is not a spec elbow.
    let p3 = line("4", vec![[0.0, 0.0, 3.0], [5.0, 0.0, 3.0], [7.5, 4.33, 3.0]], vec![]);
    let b3 = build(&p3, &cat);
    assert!(of(&b3, "elbow90").is_empty() && b3.issues[0].text.contains("60.0°"));
    // A component that does not fit is reported.
    let p4 = line("6", vec![[0.0, 0.0, 3.0], [0.5, 0.0, 3.0]], vec![item(5, ItemKind::Globe, 0, 0.25)]);
    let b4 = build(&p4, &cat);
    assert!(of(&b4, "globe").is_empty() && b4.issues[0].text.contains("does not fit"));
}

#[test]
fn routing_between_nozzles_places_flanges_reducers_and_elbows_and_resizing_resizes_all() {
    let cat = Catalogue::builtin();
    let mut d = Doc::new(Project::new());
    let pump = d.apply(Op::AddEquipment { kind: EquipKind::Pump, pos: [5.0, 5.0, 0.0] }, &cat).unwrap()[0];
    let drum = d.apply(Op::AddEquipment { kind: EquipKind::HorizontalDrum, pos: [15.0, 12.0, 0.0] }, &cat).unwrap()[0];
    // Pump discharge (4") to the drum inlet (6"): a 6" line, so a reducer at the pump.
    let lid = d.apply(Op::Route { from: RouteEnd::Nozzle { equip: pump, nozzle: "D".into() }, via: vec![[5.5, 5.0, 4.5]], to: RouteEnd::Nozzle { equip: drum, nozzle: "N1".into() }, nps: Some("6".into()) }, &cat).unwrap()[0];
    let b = build(&d.p, &cat);
    assert!(b.issues.is_empty(), "{:?}", b.issues);
    joints_ok(&b);
    let ours: Vec<_> = b.parts.iter().filter(|q| q.line == lid).collect();
    assert_eq!(ours.iter().filter(|q| q.kind == "flange").count(), 2);
    assert_eq!(ours.iter().filter(|q| q.kind == "reducer").count(), 1);
    assert!(ours.iter().filter(|q| q.kind == "elbow90").count() >= 2);
    assert!(connection_issues(&d.p, &cat, &b).is_empty(), "{:?}", connection_issues(&d.p, &cat, &b));
    // Drop a gate valve on the line: it snaps, splits the pipe and nothing is open.
    let pts = &d.p.lines[0].pts;
    let k = (0..pts.len() - 1).max_by(|&a, &b| dist(pts[a], pts[a + 1]).total_cmp(&dist(pts[b], pts[b + 1]))).unwrap();
    let mid = [(pts[k][0] + pts[k + 1][0]) / 2.0, (pts[k][1] + pts[k + 1][1]) / 2.0 + 0.3, (pts[k][2] + pts[k + 1][2]) / 2.0];
    let vid = d.apply(Op::Drop { line: lid, kind: ItemKind::Gate, at: mid, dir: [0.0; 3], nps2: String::new() }, &cat);
    let b = build(&d.p, &cat);
    let vid = vid.unwrap_or_else(|e| panic!("{e}: {:?}", d.p.lines[0].pts))[0];
    assert!(b.parts.iter().any(|q| q.item == vid && q.kind == "gate"));
    assert!(connection_issues(&d.p, &cat, &b).is_empty());
    joints_ok(&b);
    // Resize to 8": every part of the line is 8" except the pump's own 4" flange end.
    d.apply(Op::Resize { line: lid, nps: "8".into() }, &cat).unwrap();
    let b = build(&d.p, &cat);
    assert!(b.issues.is_empty(), "{:?}", b.issues);
    for q in b.parts.iter().filter(|q| q.line == lid && q.item == vid) { assert_eq!(q.nps, "8"); }
    for q in b.parts.iter().filter(|q| q.line == lid && q.kind.starts_with("elbow")) { assert_eq!(q.nps, "8"); }
    assert!(connection_issues(&d.p, &cat, &b).is_empty(), "{:?}", connection_issues(&d.p, &cat, &b));
    joints_ok(&b);
    assert!(d.p.lines[0].number.starts_with("8\"-"));
}

#[test]
fn a_tee_starts_a_branch_that_meets_it_port_to_port() {
    let cat = Catalogue::builtin();
    let mut d = Doc::new(line("6", vec![[0.0, 0.0, 4.0], [12.0, 0.0, 4.0]], vec![]));
    let tee = d.apply(Op::Drop { line: 1, kind: ItemKind::Tee, at: [6.0, 0.0, 4.0], dir: [0.0, 1.0, 0.0], nps2: String::new() }, &cat).unwrap()[0];
    let br = d.apply(Op::Route { from: RouteEnd::Branch { line: 1, item: tee, port: 0 }, via: vec![], to: RouteEnd::Point { p: [6.0, 6.0, 4.0] }, nps: Some("4".into()) }, &cat).unwrap()[0];
    let b = build(&d.p, &cat);
    assert!(b.issues.is_empty(), "{:?}", b.issues);
    let t = b.parts.iter().find(|q| q.kind == "tee").unwrap();
    assert_eq!(t.nps2, "4", "a reducing tee for the 4\" branch");
    joints_ok(&b);
    let conn = connections(&b);
    let ti = b.parts.iter().position(|q| q.kind == "tee").unwrap();
    let (other, _) = conn[&(ti, 2)];
    assert_eq!(b.parts[other].line, br);
    // Only the free ends are open.
    let open = connection_issues(&d.p, &cat, &b).into_iter().filter(|i| i.kind == "open").count();
    assert_eq!(open, 3);
}

#[test]
fn moving_equipment_drags_its_lines_along_and_keeps_them_square() {
    let cat = Catalogue::builtin();
    for id in ["pumps", "exchanger", "rack"] {
        let n = crate::examples::get(id).unwrap().equipment.len();
        for k in 0..n {
            for delta in [[0.3, 0.0, 0.0], [0.0, -0.4, 0.0], [0.5, 0.5, 0.0], [-0.6, 0.0, 0.0]] {
                let mut d = Doc::new(crate::examples::get(id).unwrap());
                let e = d.p.equipment[k].clone();
                let mut moved = e.clone();
                moved.pos = crate::parts::add(e.pos, delta);
                d.apply(Op::SetEquipment(moved), &cat).unwrap();
                let b = build(&d.p, &cat);
                // Nothing comes apart: every joint meets square, nothing is open or mismatched.
                let conn: Vec<_> = connection_issues(&d.p, &cat, &b).into_iter().map(|i| i.text).collect();
                assert!(conn.is_empty(), "{id}: moving {} by {delta:?}: {conn:?}", e.tag);
                if b.issues.is_empty() { joints_ok(&b); }
                // A move the legs cannot take is reported (a tight turn, or a skewed end to route
                // again), never silently drawn wrong.
                assert!(b.issues.iter().all(|i| i.text.contains("too tight") || i.text.contains("a turn of") || i.text.contains("does not fit") || i.text.contains("route it again")), "{id}: {} {delta:?}: {:?}", e.tag, b.issues);
            }
        }
    }
    // Moves the legs can take leave nothing to report.
    for (id, tag, delta) in [("pumps", "T-101", [-0.6, 0.0, 0.0]), ("pumps", "T-101", [0.0, 0.5, 0.0]), ("rack", "V-301", [0.3, 0.0, 0.0]), ("rack", "V-302", [0.6, 0.0, 0.0]), ("exchanger", "E-201", [0.5, 0.5, 0.0])] {
        let mut d = Doc::new(crate::examples::get(id).unwrap());
        let mut e = d.p.equipment.iter().find(|e| e.tag == tag).unwrap().clone();
        e.pos = crate::parts::add(e.pos, delta);
        d.apply(Op::SetEquipment(e), &cat).unwrap();
        let b = build(&d.p, &cat);
        assert!(b.issues.is_empty(), "{id} {tag}: {:?}", b.issues);
        joints_ok(&b);
    }
}

#[test]
fn a_line_left_skewed_by_a_rotation_routes_square_again() {
    let cat = Catalogue::builtin();
    let mut d = Doc::new(crate::examples::get("pumps").unwrap());
    let mut e = d.p.equipment.iter().find(|e| e.tag == "T-101").unwrap().clone();
    e.rot = -90.0;
    d.apply(Op::SetEquipment(e), &cat).unwrap();
    assert!(build(&d.p, &cat).issues.iter().any(|i| i.text.contains("route it again")));
    let line = d.p.lines.iter().find(|l| l.number.contains("1001")).unwrap().id;
    d.apply(Op::Reroute { line }, &cat).unwrap();
    // Its tee moved with it, so the branch routes again too.
    let branch = d.p.lines.iter().find(|l| l.number.contains("1002")).unwrap().id;
    if build(&d.p, &cat).issues.iter().any(|i| i.line == branch) { d.apply(Op::Reroute { line: branch }, &cat).unwrap(); }
    let b = build(&d.p, &cat);
    assert!(b.issues.is_empty(), "{:?}", b.issues);
    assert!(connection_issues(&d.p, &cat, &b).is_empty(), "{:?}", connection_issues(&d.p, &cat, &b));
}

#[test]
fn dropped_components_snap_clear_of_fittings_and_move_between_runs() {
    let cat = Catalogue::builtin();
    let mut d = Doc::new(line("6", vec![[0.0, 0.0, 3.0], [6.0, 0.0, 3.0], [6.0, 5.0, 3.0]], vec![]));
    // Dropped right at the corner: it lands clear of the elbow, ports touching it at most.
    let v = d.apply(Op::Drop { line: 1, kind: ItemKind::Gate, at: [5.95, 0.1, 3.0], dir: [0.0; 3], nps2: String::new() }, &cat).unwrap()[0];
    let b = build(&d.p, &cat);
    assert!(b.issues.is_empty(), "{:?}", b.issues);
    joints_ok(&b);
    assert!(connection_issues(&d.p, &cat, &b).iter().all(|i| i.kind == "open"));
    // The snap preview says the same place the drop used.
    let (seg, at, _) = crate::doc::snap(&d.p, &cat, d.p.line(1).unwrap(), ItemKind::Globe, [1.0, 0.0, 3.0], "").unwrap();
    assert_eq!(seg, 0);
    assert!(at > 0.3 && at < 2.0, "{at}");
    // Move the valve onto the other run.
    d.apply(Op::MoveItem { line: 1, item: v, at: [6.0, 3.0, 3.0] }, &cat).unwrap();
    let it = d.p.lines[0].items.iter().find(|i| i.id == v).unwrap();
    assert_eq!(it.seg, 1);
    assert!((it.at - 3.0).abs() < 0.011, "{}", it.at);
    assert!(build(&d.p, &cat).issues.is_empty());
    // No room on a short run: a clear message, nothing changed.
    let mut short = Doc::new(line("12", vec![[0.0, 0.0, 3.0], [0.6, 0.0, 3.0]], vec![]));
    let before = short.p.clone();
    let e = short.apply(Op::Drop { line: 1, kind: ItemKind::Globe, at: [0.3, 0.0, 3.0], dir: [0.0; 3], nps2: String::new() }, &cat).unwrap_err();
    assert!(e.contains("No room for a 12\" globe valve"), "{e}");
    assert_eq!(short.p, before);
}

#[test]
fn resizing_keeps_reducers_and_an_oversized_branch_is_a_mismatch() {
    let cat = Catalogue::builtin();
    let mut d = Doc::new(line("6", vec![[0.0, 0.0, 3.0], [12.0, 0.0, 3.0]], vec![]));
    d.apply(Op::Drop { line: 1, kind: ItemKind::Reducer, at: [6.0, 0.0, 3.0], dir: [0.0; 3], nps2: "4".into() }, &cat).unwrap();
    d.apply(Op::Resize { line: 1, nps: "8".into() }, &cat).unwrap();
    let b = build(&d.p, &cat);
    let r = b.parts.iter().find(|q| q.kind == "reducer").unwrap();
    assert_eq!((r.ports[0].nps.as_str(), r.ports[1].nps.as_str()), ("8", "4"));
    assert!(b.issues.is_empty());
    joints_ok(&b);
    // A 6" branch on a 4" run part: the tee stays 4" and the joint is reported.
    let tee = d.apply(Op::Drop { line: 1, kind: ItemKind::Tee, at: [10.0, 0.0, 3.0], dir: [0.0, 1.0, 0.0], nps2: String::new() }, &cat).unwrap()[0];
    d.apply(Op::Route { from: RouteEnd::Branch { line: 1, item: tee, port: 0 }, via: vec![], to: RouteEnd::Point { p: [10.0, 5.0, 3.0] }, nps: Some("6".into()) }, &cat).unwrap();
    let b = build(&d.p, &cat);
    assert_eq!(b.parts.iter().find(|q| q.kind == "tee").unwrap().nps2, "4");
    assert!(connection_issues(&d.p, &cat, &b).iter().any(|i| i.kind == "mismatch" && i.text.contains("4\"") && i.text.contains("6\"")));
}

#[test]
fn every_check_has_a_failing_and_a_passing_case() {
    let cat = Catalogue::builtin();
    let kinds = |p: &Project| { let b = build(p, &cat); crate::checks::run(p, &cat, &b).into_iter().map(|i| (i.kind, i.text)).collect::<Vec<_>>() };
    // Open end: fails open, passes capped.
    let mut p = line("4", vec![[0.0, 0.0, 2.0], [5.0, 0.0, 2.0]], vec![]);
    assert_eq!(kinds(&p).iter().filter(|k| k.0 == "open").count(), 2);
    p.lines[0].from = End::Point { term: Term::Cap };
    p.lines[0].to = End::Point { term: Term::Blind };
    assert!(kinds(&p).is_empty(), "{:?}", kinds(&p));
    // A line without a number.
    let mut q = p.clone();
    q.lines[0].number.clear();
    assert!(kinds(&q).iter().any(|k| k.0 == "number"));
    // Class rule: a 16" ball valve is not in CS1 (ball valves to 12"); a gate valve is.
    let mut r = line("16", vec![[0.0, 0.0, 3.0], [8.0, 0.0, 3.0]], vec![item(5, ItemKind::Ball, 0, 4.0)]);
    r.lines[0].from = End::Point { term: Term::Blind };
    r.lines[0].to = End::Point { term: Term::Blind };
    assert!(kinds(&r).iter().any(|k| k.0 == "class" && k.1.contains("16\" ball")), "{:?}", kinds(&r));
    r.lines[0].items[0].kind = ItemKind::Gate;
    assert!(kinds(&r).is_empty(), "{:?}", kinds(&r));
    // Rating mismatch at a nozzle: a class 300 nozzle on a class 150 line.
    let mut d = Doc::new(Project::new());
    let v = d.apply(Op::AddEquipment { kind: EquipKind::VerticalVessel, pos: [5.0, 5.0, 0.0] }, &cat).unwrap()[0];
    d.apply(Op::Route { from: RouteEnd::Nozzle { equip: v, nozzle: "N2".into() }, via: vec![], to: RouteEnd::Point { p: [10.0, 9.0, 1.9] }, nps: None }, &cat).unwrap();
    d.apply(Op::SetTerm { line: d.p.lines[0].id, start: false, term: Term::Blind }, &cat).unwrap();
    assert!(kinds(&d.p).is_empty(), "{:?}", kinds(&d.p));
    let mut e = d.p.equip(v).unwrap().clone();
    e.nozzles[1].rating = 300;
    d.apply(Op::SetEquipment(e), &cat).unwrap();
    assert!(kinds(&d.p).iter().any(|k| k.0 == "mismatch" && k.1.contains("class 300 meets class 150") || k.1.contains("class 150 meets class 300")), "{:?}", kinds(&d.p));
    // Clash with the ground: fails at 10 mm, passes at 1 m (see also "Find the clashes").
    let mut g = line("2", vec![[0.0, 0.0, 0.03], [5.0, 0.0, 0.03]], vec![]);
    g.lines[0].from = End::Point { term: Term::Cap };
    g.lines[0].to = End::Point { term: Term::Cap };
    assert!(kinds(&g).iter().any(|k| k.0 == "clash" && k.1.starts_with("Pipe–ground")));
    for pt in &mut g.lines[0].pts { pt[2] = 1.0; }
    assert!(kinds(&g).is_empty());
    // Size mismatch: tested with an oversized branch (resizing test); pipe–pipe and
    // pipe–equipment clashes: "Find the clashes" (examples).
}

#[test]
fn the_take_off_counts_what_example_1_holds() {
    let cat = Catalogue::builtin();
    let p = crate::examples::get("pumps").unwrap();
    let l = crate::lists::lists(&p, &cat);
    let count = |g: &str, w: &str| l.mto.iter().filter(|r| r.group == g && r.what.contains(w)).map(|r| r.qty).sum::<f64>();
    assert_eq!(count("Valves", "Gate"), 2.0);
    assert_eq!(count("Valves", "Check"), 2.0);
    assert_eq!(count("Valves", "Strainer"), 2.0);
    assert_eq!(count("Fittings", "Tee"), 3.0);
    assert_eq!(count("Fittings", "Reducer"), 2.0);
    assert_eq!(count("Flanges", "blind"), 1.0);
    assert_eq!(count("Fittings", "Cap"), 1.0);
    // One gasket and one bolt set per flanged joint.
    assert_eq!(count("Gaskets", ""), count("Bolts", ""));
    let b = build(&p, &cat);
    assert_eq!(count("Gaskets", "") as usize, b.parts.iter().filter(|q| q.kind == "gasket").count());
    // Pipe length per line equals the generated pipes'.
    for line in &p.lines {
        let mine: f64 = b.parts.iter().filter(|q| q.line == line.id && q.kind == "pipe").map(|q| q.length).sum();
        let listed: f64 = l.mto.iter().filter(|r| r.line == line.number && r.group == "Pipe").map(|r| r.qty).sum();
        assert!((mine - listed).abs() < 0.002, "{}: {mine} vs {listed}", line.number);
    }
    assert_eq!(l.lines.len(), 5);
    assert_eq!(l.equipment.iter().map(|e| e.nozzles.len()).sum::<usize>(), 7);
    assert!(crate::lists::csv(&l, "mto").unwrap().lines().count() == l.mto.len() + 1);
}

#[test]
fn groups_move_copy_rotate_and_mirror_cleanly() {
    let cat = Catalogue::builtin();
    let clean = |d: &Doc| { let b = build(&d.p, &cat); assert!(b.issues.is_empty(), "{:?}", b.issues); assert!(connection_issues(&d.p, &cat, &b).is_empty(), "{:?}", connection_issues(&d.p, &cat, &b)); joints_ok(&b); };
    let base = crate::examples::get("pumps").unwrap();
    let all: Vec<u32> = base.equipment.iter().map(|e| e.id).chain(base.lines.iter().map(|l| l.id)).collect();
    // Move the whole plant.
    let mut d = Doc::new(base.clone());
    d.apply(Op::MoveGroup { ids: all.clone(), delta: [3.0, -2.0, 0.5] }, &cat).unwrap();
    clean(&d);
    // Copy it: twice as many items, all connected among the copies.
    let mut d = Doc::new(base.clone());
    let made = d.apply(Op::CopyGroup { ids: all.clone(), delta: [0.0, 20.0, 0.0] }, &cat).unwrap();
    assert_eq!(made.len(), all.len());
    assert_eq!(d.p.equipment.len(), 6);
    assert_eq!(d.p.lines.len(), 10);
    clean(&d);
    // Turn it a quarter and a half turn.
    for deg in [90.0, 180.0, -90.0] {
        let mut d = Doc::new(base.clone());
        d.apply(Op::RotateGroup { ids: all.clone(), about: [10.0, 10.0, 0.0], deg }, &cat).unwrap();
        clean(&d);
    }
    // Mirror it both ways: the pumps' nozzles and every line come out the other hand, still clean.
    for ew in [false, true] {
        let mut d = Doc::new(base.clone());
        d.apply(Op::MirrorGroup { ids: all.clone(), about: [15.0, 10.0, 0.0], east_west: ew }, &cat).unwrap();
        clean(&d);
    }
    // Moving only the pumps: the lines connected to them follow.
    let mut d = Doc::new(base.clone());
    let pumps: Vec<u32> = d.p.equipment.iter().filter(|e| e.tag.starts_with('P')).map(|e| e.id).collect();
    d.apply(Op::MoveGroup { ids: pumps, delta: [0.5, 0.0, 0.0] }, &cat).unwrap();
    let b = build(&d.p, &cat);
    assert!(connection_issues(&d.p, &cat, &b).is_empty());
}

#[test]
fn corners_move_and_jogs_come_and_go_cleanly() {
    let cat = Catalogue::builtin();
    let clean = |d: &Doc| { let b = build(&d.p, &cat); assert!(b.issues.is_empty(), "{:?}", b.issues); assert!(connection_issues(&d.p, &cat, &b).is_empty()); joints_ok(&b); };
    let base = crate::examples::get("pumps").unwrap();
    // Every line: jog the longest run up by a metre in its middle third, then take the jog out.
    let mut jogged = 0;
    for l in &base.lines {
        let mut d = Doc::new(base.clone());
        let pts = route::path(&d.p, &cat, &l.clone());
        let (seg, len) = (0..pts.len() - 1).map(|i| (i, norm(sub(pts[i + 1], pts[i])))).fold((0, 0.0), |a, b| if b.1 > a.1 { b } else { a });
        if len < 3.0 { continue; }
        let up = if (pts[seg + 1][2] - pts[seg][2]).abs() > 1e-6 { [1.0, 0.0, 0.0] } else { [0.0, 0.0, 1.0] };
        let items = d.p.line(l.id).unwrap().items.len();
        let before: Vec<P3> = route::build(&d.p, &cat).parts.iter().filter(|q| q.line == l.id && q.item != 0 && !q.ports.is_empty()).map(|q| q.ports[0].p).collect();
        match d.apply(Op::Jog { line: l.id, seg, s0: len / 3.0, s1: 2.0 * len / 3.0, delta: up }, &cat) {
            Err(e) if e.contains("component") => continue,
            r => { r.unwrap(); jogged += 1; }
        }
        clean(&d);
        assert_eq!(d.p.line(l.id).unwrap().pts.len(), pts.len() + 4);
        assert_eq!(d.p.line(l.id).unwrap().items.len(), items);
        d.apply(Op::RemoveJog { line: l.id, seg: seg + 2 }, &cat).unwrap();
        clean(&d);
        assert_eq!(d.p.line(l.id).unwrap().pts.len(), pts.len(), "{}", l.number);
        // Components are back where they were.
        let after: Vec<P3> = route::build(&d.p, &cat).parts.iter().filter(|q| q.line == l.id && q.item != 0 && !q.ports.is_empty()).map(|q| q.ports[0].p).collect();
        assert_eq!(before.len(), after.len());
        for (a, b) in before.iter().zip(&after) { assert!(norm(sub(*a, *b)) < 1e-6, "{a:?} {b:?}"); }
    }
    assert!(jogged >= 2, "{jogged}");
    // Drag a corner of a free-ended line: both runs stay square.
    let mut d = Doc::new(base.clone());
    let l = d.p.lines.iter().find(|l| l.pts.len() >= 4).unwrap().clone();
    let pts = route::path(&d.p, &cat, &l);
    let c = (1..pts.len() - 1).find(|&c| c > 1 && c + 1 < pts.len() - 1).unwrap_or(1);
    let r = d.apply(Op::MoveCorner { line: l.id, corner: c, delta: [0.3, 0.3, 0.3] }, &cat);
    if r.is_ok() {
        let b = build(&d.p, &cat);
        assert!(b.issues.iter().all(|i| !i.text.contains("a turn of") && !i.text.contains("square")), "{:?}", b.issues);
    }
    // A corner can't move off a held end.
    assert!(d.apply(Op::MoveCorner { line: l.id, corner: 0, delta: [1.0, 0.0, 0.0] }, &cat).is_err());
    // Remove a jog where there is none.
    assert!(d.apply(Op::RemoveJog { line: l.id, seg: 0 }, &cat).is_err());
}

#[test]
fn tidy_merges_straight_corners_and_keeps_components() {
    let mut l = Line { pts: vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [2.0, 0.0, 0.0], [5.0, 0.0, 0.0], [5.0, 3.0, 0.0]], ..Default::default() };
    l.items.push(Item { id: 9, uid: String::new(), kind: ItemKind::Gate, seg: 2, at: 1.0, nps2: String::new(), dir: [0.0, 0.0, 1.0], tag: String::new(), ..Default::default() });
    crate::doc::tidy(&mut l);
    assert_eq!(l.pts, vec![[0.0, 0.0, 0.0], [5.0, 0.0, 0.0], [5.0, 3.0, 0.0]]);
    assert_eq!((l.items[0].seg, l.items[0].at), (0, 3.0));
}

#[test]
fn design_rules_find_stems_down_check_valves_flowing_down_and_short_pieces() {
    let cat = Catalogue::builtin();
    let rules = |p: &Project| { let b = build(p, &cat); crate::checks::design_rules(p, &cat, &b) };
    // Every example follows the rules.
    for ex in crate::examples::list() { let p = crate::examples::get(ex.id).unwrap(); assert!(rules(&p).is_empty(), "{}: {:?}", ex.name, rules(&p)); }
    // A gate valve with its stem down; a check valve on a run flowing down.
    let mut v = item(5, ItemKind::Gate, 0, 2.5); v.dir = [0.0, 0.0, -1.0];
    let p = line("4", vec![[0.0, 0.0, 3.0], [5.0, 0.0, 3.0], [5.0, 0.0, -3.0]], vec![v, item(6, ItemKind::Check, 1, 3.0)]);
    let r = rules(&p);
    assert_eq!(r.len(), 2, "{r:?}");
    assert!(r.iter().any(|i| i.text.contains("stem points down")) && r.iter().any(|i| i.text.contains("flowing down")));
    // Drawn the other way (flowing up), the check valve is fine.
    let mut v = item(5, ItemKind::Gate, 1, 2.5); v.dir = [0.0, 1.0, 0.0];
    let p = line("4", vec![[5.0, 0.0, -3.0], [5.0, 0.0, 3.0], [0.0, 0.0, 3.0]], vec![item(6, ItemKind::Check, 0, 3.0), v]);
    assert!(rules(&p).is_empty(), "{:?}", rules(&p));
    // Two elbows 20 mm of pipe apart.
    let e = cat.fitting("8").unwrap().elbow_lr * 2.0 + 0.02; // two 8" long-radius elbows and 20 mm of pipe
    let p = line("8", vec![[0.0, 0.0, 0.0], [3.0, 0.0, 0.0], [3.0, e, 0.0], [6.0, e, 0.0]], vec![]);
    let r = rules(&p);
    assert!(r.len() == 1 && r[0].text.contains("too short"), "{r:?}");
}

#[test]
fn the_take_off_counts_butt_welds_and_the_valve_list_lists_valves() {
    let cat = Catalogue::builtin();
    // Pipe, elbow, pipe: two welds; a valve adds its two flanges' welds.
    let p = line("6", vec![[0.0, 0.0, 0.0], [5.0, 0.0, 0.0], [5.0, 5.0, 0.0]], vec![]);
    let welds = |p: &Project| crate::lists::lists(p, &cat).mto.iter().filter(|r| r.group == "Welds").map(|r| r.qty).sum::<f64>();
    assert_eq!(welds(&p), 2.0);
    let mut v = item(5, ItemKind::Gate, 0, 2.0); v.dir = [0.0, 0.0, 1.0]; v.tag = "HV-1".into();
    let p = line("6", vec![[0.0, 0.0, 0.0], [5.0, 0.0, 0.0], [5.0, 5.0, 0.0]], vec![v]);
    assert_eq!(welds(&p), 4.0);
    let l = crate::lists::lists(&p, &cat);
    assert_eq!(l.valves.len(), 1);
    assert_eq!((l.valves[0].tag.as_str(), l.valves[0].kind.as_str(), l.valves[0].nps.as_str(), l.valves[0].stem.as_str()), ("HV-1", "gate", "6", "up"));
    assert!((l.valves[0].at[0] - 2.0).abs() < 1e-6, "{:?}", l.valves[0].at);
    let csv = crate::lists::csv(&l, "valves").unwrap();
    assert!(csv.starts_with("Tag,Type") && csv.contains("HV-1,gate,6,150"), "{csv}");
    // Example 1: every valve listed.
    let p = crate::examples::get("pumps").unwrap();
    let n: usize = p.lines.iter().map(|l| l.items.iter().filter(|i| matches!(i.kind, ItemKind::Gate | ItemKind::Globe | ItemKind::Check | ItemKind::Ball | ItemKind::Strainer)).count()).sum();
    assert_eq!(crate::lists::lists(&p, &cat).valves.len(), n);
}

#[test]
fn eccentric_reducers_offset_the_run_and_switch_back() {
    use crate::model::Flat;
    let cat = Catalogue::builtin();
    // An 8" level line with a reducer to 6" in its first run, then a turn down.
    let red = Item { id: 7, kind: ItemKind::Reducer, seg: 0, at: 3.0, nps2: "6".into(), ..Default::default() };
    let base = line("8", vec![[0.0, 0.0, 3.0], [6.0, 0.0, 3.0], [6.0, 0.0, 0.5]], vec![red]);
    let e = (cat.od("8") - cat.od("6")) / 2.0;
    for (flat, sign) in [(Flat::Top, 1.0), (Flat::Bottom, -1.0)] {
        let mut d = Doc::new(base.clone());
        d.apply(Op::SetEccentric { line: 1, item: 7, flat }, &cat).unwrap();
        let l = d.p.line(1).unwrap().clone();
        assert_eq!(l.pts.len(), 5, "{:?}", l.pts);
        // The run beyond is offset; the corner below takes it up.
        assert!((l.pts[3][2] - (3.0 + sign * e)).abs() < 1e-9, "{:?}", l.pts);
        let b = build(&d.p, &cat);
        assert!(b.issues.is_empty(), "{:?}", b.issues);
        joints_ok(&b);
        let r = of(&b, "reducer");
        assert_eq!(r.len(), 1);
        // Flat side: the tops (or bottoms) of the two ends line up.
        let (pa, pb) = (r[0].ports[0].p, r[0].ports[1].p);
        let (ra, rb) = (cat.od("8") / 2.0, cat.od("6") / 2.0);
        if flat == Flat::Top { assert!((pa[2] + ra - (pb[2] + rb)).abs() < 1e-9); } else { assert!((pa[2] - ra - (pb[2] - rb)).abs() < 1e-9); }
        // It goes to Datum Pipe as an eccentric reducer and reads back the same.
        let text = crate::pcf::write(&d.p, &cat, None).unwrap();
        assert!(text.contains("REDUCER-ECCENTRIC") && text.contains(if flat == Flat::Top { "FLAT-DIRECTION UP" } else { "FLAT-DIRECTION DOWN" }));
        assert!(crate::pcf::reader::pcf(&text).is_ok());
        let mut back = Doc::new(Project::new());
        back.apply(Op::ImportPcf { text }, &cat).unwrap();
        let bl = &back.p.lines[0];
        assert_eq!(bl.pts.len(), l.pts.len());
        assert!(bl.items.iter().any(|i| i.kind == ItemKind::Reducer && i.flat == flat), "{:?}", bl.items);
        assert!(build(&back.p, &cat).issues.is_empty());
        // Concentric again: the line as it was.
        d.apply(Op::SetEccentric { line: 1, item: 7, flat: Flat::None }, &cat).unwrap();
        let l2 = d.p.line(1).unwrap();
        assert_eq!(l2.pts, base.lines[0].pts);
        assert!((l2.items[0].at - 3.0).abs() < 1e-9 && l2.items[0].seg == 0, "{:?}", l2.items);
    }
    // In a vertical run it cannot be eccentric.
    let mut d = Doc::new(line("8", vec![[0.0, 0.0, 6.0], [0.0, 0.0, 0.5]], vec![Item { id: 7, kind: ItemKind::Reducer, seg: 0, at: 3.0, nps2: "6".into(), ..Default::default() }]));
    assert!(d.apply(Op::SetEccentric { line: 1, item: 7, flat: Flat::Top }, &cat).is_err());
}

#[test]
fn a_pump_suction_reducer_flat_on_top_moves_the_run_before_it() {
    use crate::model::Flat;
    let cat = Catalogue::builtin();
    let mut d = Doc::new(Project::new());
    let pump = d.apply(Op::AddEquipment { kind: EquipKind::Pump, pos: [10.0, 10.0, 0.0] }, &cat).unwrap()[0];
    let s = route::end_port(&d.p, &cat, &End::Nozzle { equip: pump, nozzle: "S".into() }).unwrap();
    // An 8" line down from 4 m to the 6" suction: the last run is level into the nozzle.
    let made = d.apply(Op::Route { from: RouteEnd::Point { p: [2.0, 10.0, 4.0] }, via: vec![[2.0, 10.0, s.p[2]]], to: RouteEnd::Nozzle { equip: pump, nozzle: "S".into() }, nps: Some("8".into()) }, &cat).unwrap();
    let id = made[0];
    let l = d.p.line(id).unwrap().clone();
    let k = l.pts.len() - 2;
    let r = d.apply(Op::Drop { line: id, kind: ItemKind::Reducer, at: [4.5, 10.0, s.p[2]], dir: [0.0; 3], nps2: "6".into() }, &cat).unwrap()[0];
    let before = d.p.line(id).unwrap().clone();
    assert_eq!(before.items[0].seg, k);
    d.apply(Op::SetEccentric { line: id, item: r, flat: Flat::Top }, &cat).unwrap();
    let after = d.p.line(id).unwrap();
    // The nozzle end stays; the run before the reducer drops by the offset.
    assert!(dist(*after.pts.last().unwrap(), *before.pts.last().unwrap()) < 1e-9);
    let e = (cat.od("8") - cat.od("6")) / 2.0;
    assert!((after.pts[k][2] - (before.pts[k][2] - e)).abs() < 1e-9, "{:?}", after.pts);
    let b = build(&d.p, &cat);
    assert!(b.issues.iter().all(|i| i.kind != "routing"), "{:?}", b.issues);
    // Only the line's free start is open; the reducer, flanges and nozzle all meet.
    assert!(connection_issues(&d.p, &cat, &b).iter().all(|i| i.kind == "open"), "{:?}", connection_issues(&d.p, &cat, &b));
    joints_ok(&b);
    // Route Again makes it concentric and squares the line.
    d.apply(Op::Reroute { line: id }, &cat).unwrap();
    assert!(d.p.line(id).unwrap().items.iter().all(|i| i.flat == Flat::None));
}

#[test]
fn a_cross_takes_two_branches_and_goes_to_datum_pipe() {
    let cat = Catalogue::builtin();
    // A 6" level header with a cross in the middle of its run, branches north and south.
    let x = Item { id: 7, kind: ItemKind::Cross, seg: 0, at: 4.0, dir: [0.0, 1.0, 0.0], ..Default::default() };
    let mut d = Doc::new(line("6", vec![[0.0, 0.0, 3.0], [8.0, 0.0, 3.0]], vec![x]));
    let mut ends = vec![];
    for (port, y) in [(0u8, 3.0), (1, -3.0)] {
        let made = d.apply(Op::Route { from: RouteEnd::Branch { line: 1, item: 7, port }, via: vec![], to: RouteEnd::Point { p: [4.0, y, 3.0] }, nps: Some(if port == 0 { "4".into() } else { "3".into() }) }, &cat).unwrap();
        ends.push(made[0]);
    }
    // The two branches leave opposite ways; the second cannot take a line twice.
    assert!(d.apply(Op::Route { from: RouteEnd::Branch { line: 1, item: 7, port: 1 }, via: vec![], to: RouteEnd::Point { p: [4.0, -5.0, 3.0] }, nps: None }, &cat).is_err());
    let b = build(&d.p, &cat);
    assert!(b.issues.is_empty(), "{:?}", b.issues);
    joints_ok(&b);
    let iss = connection_issues(&d.p, &cat, &b);
    assert!(iss.iter().all(|i| i.kind == "open"), "{iss:?}");
    let c = of(&b, "cross");
    assert_eq!(c.len(), 1);
    assert_eq!((c[0].ports.len(), c[0].ports[2].nps.as_str(), c[0].ports[3].nps.as_str()), (4, "4", "3"));
    assert!(c[0].ports[2].d[1] > 0.99 && c[0].ports[3].d[1] < -0.99);
    // The take-off names it; the class allows it where it allows tees.
    let mto = crate::lists::lists(&d.p, &cat).mto;
    assert!(mto.iter().any(|r| r.what.starts_with("Cross, reducing 6\" × 4\"")), "{mto:?}");
    // To Datum Pipe: a CROSS with both branch points; read back, both branches reconnect.
    let text = crate::pcf::write(&d.p, &cat, None).unwrap();
    assert!(text.contains("CROSS") && text.contains("BRANCH2-POINT"));
    assert!(crate::pcf::reader::pcf(&crate::pcf::write(&d.p, &cat, Some(1)).unwrap()).is_ok());
    let mut back = Doc::new(Project::new());
    back.apply(Op::ImportPcf { text }, &cat).unwrap();
    assert_eq!(back.p.lines.len(), 3);
    let ports: Vec<u8> = back.p.lines.iter().filter_map(|l| if let End::Branch { port, .. } = l.from { Some(port) } else { None }).collect();
    assert_eq!(ports.len(), 2, "{:?}", back.p.lines.iter().map(|l| &l.from).collect::<Vec<_>>());
    assert!(ports.contains(&0) && ports.contains(&1));
    assert!(build(&back.p, &cat).issues.is_empty());
    // An old file's branch (no port) reads as the first.
    let e: End = serde_json::from_str(r#"{"kind":"branch","line":1,"item":7}"#).unwrap();
    assert_eq!(e, End::Branch { line: 1, item: 7, port: 0 });
}

#[test]
fn reducing_tees_and_reducers_come_from_their_tables() {
    let cat = Catalogue::builtin();
    assert!(cat.warnings.is_empty(), "{:?}", cat.warnings);
    assert!(cat.tees.len() >= 80 && cat.reducers.len() >= 90, "{} {}", cat.tees.len(), cat.reducers.len());
    let inch = |v: f64| v / 0.0254;
    // Spot values as the catalogue lists them.
    assert!((inch(cat.tee_m("8", "6").unwrap()) - 6.62).abs() < 1e-9);
    assert!((inch(cat.tee_m("24", "18").unwrap()) - 16.50).abs() < 1e-9);
    assert!((inch(cat.reducer_h("6", "4").unwrap()) - 5.50).abs() < 1e-9);
    assert!((inch(cat.reducer_h("22", "24").unwrap()) - 20.0).abs() < 1e-9, "either order");
    // Every reducing tee's run C equals the equal tee's C, M is no longer than C, and both grow with size.
    for t in &cat.tees {
        let c = cat.fitting(&t.nps).unwrap().tee_c;
        assert!((t.c - c).abs() < 0.002, "{} x {}: C {} vs {}", t.nps, t.nps_branch, inch(t.c), inch(c));
        assert!(t.m <= t.c + 1e-9);
    }
    // A 6" × 3" tee: its branch end is M from the run's centreline, not C.
    let tee = Item { id: 5, kind: ItemKind::Tee, seg: 0, at: 3.0, nps2: "3".into(), dir: [0.0, 1.0, 0.0], ..Default::default() };
    let p = line("6", vec![[0.0, 0.0, 3.0], [6.0, 0.0, 3.0]], vec![tee]);
    let b = build(&p, &cat);
    let t = of(&b, "tee")[0];
    assert!((t.ports[2].p[1] - cat.tee_m("6", "3").unwrap()).abs() < 1e-9, "{:?}", t.ports[2].p);
    assert!((inch(cat.tee_m("6", "3").unwrap()) - 4.88).abs() < 1e-9);
}

#[test]
fn branch_outlets_follow_the_class_branch_table() {
    let cat = Catalogue::builtin();
    let inch = |v: f64| v / 0.0254;
    // The outlet rows: heights as the catalogue lists them.
    assert!((inch(cat.olet("weldolet", "2", false).unwrap().height) - 1.5).abs() < 1e-9);
    assert!((inch(cat.olet("sockolet", "1", false).unwrap().height) - 1.3125).abs() < 1e-9);
    let cs1 = cat.class("CS1").unwrap();
    assert_eq!(cs1.branch_fitting("6", "6"), Some("tee"));
    assert_eq!(cs1.branch_fitting("6", "4"), Some("reducing_tee"));
    assert_eq!(cs1.branch_fitting("6", "3"), Some("weldolet"));
    assert_eq!(cs1.branch_fitting("6", "1"), Some("sockolet"));
    assert_eq!(cs1.branch_fitting("2", "1"), Some("reducing_tee"));
    for (branch, want) in [("3", "weldolet"), ("1", "sockolet"), ("4", "tee")] {
        let mut d = Doc::new(line("6", vec![[0.0, 0.0, 4.0], [12.0, 0.0, 4.0]], vec![]));
        let tee = d.apply(Op::Drop { line: 1, kind: ItemKind::Tee, at: [6.0, 0.0, 4.0], dir: [0.0, 1.0, 0.0], nps2: String::new() }, &cat).unwrap()[0];
        d.apply(Op::Route { from: RouteEnd::Branch { line: 1, item: tee, port: 0 }, via: vec![], to: RouteEnd::Point { p: [6.0, 4.0, 4.0] }, nps: Some(branch.into()) }, &cat).unwrap();
        let b = build(&d.p, &cat);
        assert!(b.issues.is_empty(), "{branch}: {:?}", b.issues);
        joints_ok(&b);
        let iss = connection_issues(&d.p, &cat, &b);
        assert!(iss.iter().all(|i| i.kind == "open"), "{branch}: {iss:?}");
        assert!(crate::checks::clashes(&d.p, &b).is_empty(), "{branch}");
        let part = b.parts.iter().find(|q| matches!(q.kind, "tee" | "weldolet" | "sockolet")).unwrap();
        assert_eq!(part.kind, want, "a {branch}\" branch off 6\"");
        if want != "tee" {
            // Its end: half the header's diameter plus the outlet's height above it.
            let h = cat.olet(want, branch, false).unwrap().height;
            assert!((part.ports[0].p[1] - (cat.od("6") / 2.0 + h)).abs() < 1e-9, "{:?}", part.ports[0].p);
            // The header is split at the outlet (Datum Pipe needs a node there), no weld counted.
            assert_eq!(of(&b, "pipe").iter().filter(|q| q.line == 1).count(), 2);
            let text = crate::pcf::write(&d.p, &cat, None).unwrap();
            assert!(text.contains("OLET") && text.contains(if want == "weldolet" { "SKEY WTBW" } else { "SKEY SKSW" }));
            assert!(crate::pcf::reader::pcf(&crate::pcf::write(&d.p, &cat, Some(1)).unwrap()).is_ok());
            let mut back = Doc::new(Project::new());
            back.apply(Op::ImportPcf { text }, &cat).unwrap();
            let kinds: Vec<ItemKind> = back.p.lines.iter().flat_map(|l| l.items.iter().map(|i| i.kind)).collect();
            assert!(kinds.contains(&(if want == "weldolet" { ItemKind::Weldolet } else { ItemKind::Sockolet })), "{kinds:?}");
            assert!(back.p.lines.iter().any(|l| matches!(l.from, End::Branch { .. })));
            assert!(build(&back.p, &cat).issues.is_empty());
            let mto = crate::lists::lists(&d.p, &cat).mto;
            assert!(mto.iter().any(|r| r.what.to_lowercase().starts_with(want)), "{mto:?}");
        }
    }
    // A tee where the class wants a weldolet is reported (here: made by hand).
    let tee = Item { id: 5, kind: ItemKind::Tee, seg: 0, at: 6.0, dir: [0.0, 1.0, 0.0], ..Default::default() };
    let mut p = line("6", vec![[0.0, 0.0, 4.0], [12.0, 0.0, 4.0]], vec![tee]);
    p.lines.push(Line { id: 9, number: "3\"-P-9-CS1".into(), nps: "3".into(), from: End::Branch { line: 1, item: 5, port: 0 }, pts: vec![[6.0, 0.2, 4.0], [6.0, 4.0, 4.0]], ..Default::default() });
    p.normalize();
    let b = build(&p, &cat);
    let iss = connection_issues(&p, &cat, &b);
    assert!(iss.iter().any(|i| i.kind == "class" && i.text.contains("with a weldolet, not a reducing tee")), "{iss:?}");
}

#[test]
fn every_part_has_its_weight() {
    let cat = Catalogue::builtin();
    assert!(cat.warnings.is_empty(), "{:?}", cat.warnings);
    assert!(cat.weights.len() >= 700, "{}", cat.weights.len());
    // Pipe from its diameter and wall agrees with the published weight per metre (wermac.org's
    // B36.10 weight table, kg/m, STD and XS, 1/2" to 2") within 1.5 %.
    for (nps, std, xs) in [("1/2", 1.27, 1.62), ("3/4", 1.69, 2.20), ("1", 2.50, 3.24), ("1-1/4", 3.39, 4.47), ("1-1/2", 4.05, 5.41), ("2", 5.44, 7.48)] {
        for (sch, want) in [("STD", std), ("XS", xs)] {
            let got = cat.pipe_kg_m(nps, sch, 7850.0).unwrap();
            assert!((got / want - 1.0).abs() < 0.015, "{nps} {sch}: {got} vs {want}");
        }
    }
    // Fittings: the STD row for a schedule with the STD wall; others scaled by the wall.
    let lb = crate::catalogue::LB;
    let near = |got: Option<(f64, bool)>, lbs: f64| got.is_some_and(|(k, e)| !e && (k - lbs * lb).abs() < 0.001);
    assert!(near(cat.fitting_kg("elbow90_lr", "6", "", "40"), 24.5));
    assert!(near(cat.fitting_kg("elbow90_lr", "6", "", "80"), 35.0));
    let (thin, est) = cat.fitting_kg("elbow90_lr", "6", "", "10").unwrap();
    assert!(est && thin < 24.5 * lb * 0.6, "{thin}");
    assert!(cat.rated_kg("flange_wn", "6", 150).unwrap() > 11.0 && cat.rated_kg("flange_wn", "6", 300).unwrap() > cat.rated_kg("flange_wn", "6", 150).unwrap());
    // Heavier walls and higher classes weigh more; every row grows with size within its kind.
    for k in crate::catalogue::WEIGHT_KINDS {
        let mut rows: Vec<_> = cat.weights.iter().filter(|w| w.kind == *k && w.nps_small.is_empty()).collect();
        rows.sort_by_key(|w| (w.wall.clone(), w.rating, crate::catalogue::nps_index(&w.nps)));
        for p in rows.windows(2) {
            if p[0].wall == p[1].wall && p[0].rating == p[1].rating { assert!(p[1].kg >= p[0].kg * 0.95, "{k}: {} {} then {} {}", p[0].nps, p[0].kg, p[1].nps, p[1].kg); }
        }
    }
    // In the examples, every pipe, fitting, flange and gate valve has a weight, and the take-off adds up.
    for ex in crate::examples::list() {
        let p = crate::examples::get(ex.id).unwrap();
        let l = crate::lists::lists(&p, &cat);
        for r in &l.mto {
            if matches!(r.group, "Pipe" | "Flanges") || r.what.starts_with("Elbow") || r.what.starts_with("Tee") || r.what.starts_with("Reducer") || r.what.starts_with("Gate") {
                assert!(r.kg.is_some_and(|k| k > 0.0), "{}: {} {} has no weight", ex.id, r.what, r.nps);
            }
        }
        for line in &l.lines {
            let sum: f64 = l.mto.iter().filter(|r| r.line == line.number).filter_map(|r| r.kg).sum();
            assert!((sum - line.weight).abs() < 0.05 * l.mto.iter().filter(|r| r.line == line.number).count() as f64 + 1e-6, "{}: {} vs {}", line.number, sum, line.weight);
            assert!(line.weight > 0.0);
        }
        let pcf = crate::pcf::write(&p, &cat, None).unwrap();
        assert!(pcf.contains("    WEIGHT "), "{}", ex.id);
    }
}

#[test]
fn butterfly_valves_are_short_and_go_to_datum_pipe() {
    let cat = Catalogue::builtin();
    assert!(cat.warnings.is_empty(), "{:?}", cat.warnings);
    // Short pattern from the table; every long-pattern length in its source equals the gate valve's.
    assert!((cat.valve_length("butterfly", "6", 150).unwrap() / 0.0254 - 5.51).abs() < 1e-9);
    assert!(cat.valve_length("butterfly", "6", 150).unwrap() < cat.valve_length("gate", "6", 150).unwrap() * 0.6);
    let mut p = line("6", vec![[0.0, 0.0, 3.0], [8.0, 0.0, 3.0]], vec![Item { dir: [0.0, 0.0, 1.0], ..item(5, ItemKind::Butterfly, 0, 4.0) }]);
    p.lines[0].from = End::Point { term: Term::Blind };
    p.lines[0].to = End::Point { term: Term::Blind };
    let b = build(&p, &cat);
    joints_ok(&b);
    let v = of(&b, "butterfly");
    assert_eq!(v.len(), 1);
    assert!((v[0].length - 5.51 * 0.0254).abs() < 1e-6, "{}", v[0].length);
    assert!(crate::checks::run(&p, &cat, &build(&p, &cat)).iter().all(|i| i.kind != "class"), "CS1 allows 6\" butterfly valves");
    // Stem down is reported, as for other valves with a stem.
    let mut q = p.clone();
    q.lines[0].items[0].dir = [0.0, 0.0, -1.0];
    assert!(crate::checks::run(&q, &cat, &build(&q, &cat)).iter().any(|i| i.text.contains("butterfly valve's stem points down")));
    // PCF: a VALVE with SKEY VYFL, read back as a butterfly valve.
    let pcf = crate::pcf::write(&p, &cat, None).unwrap();
    assert!(pcf.contains("VALVE\n") && pcf.contains("SKEY VYFL"), "{pcf}");
    let mut n = 100;
    let (lines, _) = crate::pcf::import::read(&pcf, &Project::new(), &cat, &mut || { n += 1; n }).unwrap();
    assert!(lines.iter().flat_map(|l| &l.items).any(|i| i.kind == ItemKind::Butterfly));
}

#[test]
fn a_plant_keeps_its_own_pipe_classes() {
    let base = Catalogue::builtin();
    let mut d = Doc::new(crate::examples::get("pumps").unwrap());
    let six = d.p.lines.iter().find(|l| l.nps == "6").unwrap().id;
    let wall = |d: &Doc| base.for_project(&d.p).pipe("6", "CS1").unwrap().schedule.clone();
    assert_eq!(wall(&d), "40");
    // An edited copy of CS1: 6" in schedule 80. The plant's copy wins over the catalogue's.
    let mut cs1 = base.class("CS1").unwrap().clone();
    cs1.schedules.push(("6".into(), "80".into()));
    cs1.schedules.push(("8".into(), "STD".into()));
    d.apply(Op::SetClass { class: cs1.clone(), was: Some("CS1".into()) }, &base.for_project(&d.p)).unwrap();
    assert_eq!(wall(&d), "80");
    let lists = crate::lists::lists(&d.p, &base.for_project(&d.p));
    assert!(lists.mto.iter().any(|r| r.what.contains("sch 80") && r.nps == "6"), "the take-off follows the plant's class");
    // It travels with the file.
    let back: Project = serde_json::from_str(&serde_json::to_string(&d.p).unwrap()).unwrap();
    assert_eq!(back.classes.len(), 1);
    // A class the engine cannot use is refused with the reason.
    let mut bad = cs1.clone();
    bad.name = "BAD".into();
    bad.schedules = vec![("1/2".into(), "99".into())];
    let e = d.apply(Op::SetClass { class: bad, was: None }, &base.for_project(&d.p)).unwrap_err();
    assert!(e.contains("no 1/2\" pipe in schedule 99"), "{e}");
    let mut bad2 = cs1.clone();
    bad2.name = "BAD2".into();
    bad2.branches[0].fitting = "elbow".into();
    assert!(d.apply(Op::SetClass { class: bad2, was: None }, &base.for_project(&d.p)).unwrap_err().contains("not a branch fitting"));
    // A new class, then renamed: its lines follow the new name.
    let mut mine = cs1.clone();
    mine.name = "CS-NEW".into();
    d.apply(Op::SetClass { class: mine.clone(), was: None }, &base.for_project(&d.p)).unwrap();
    d.apply(Op::SetLine(Line { class: "CS-NEW".into(), ..d.p.line(six).unwrap().clone() }), &base.for_project(&d.p)).unwrap();
    let mut renamed = mine.clone();
    renamed.name = "CS-6".into();
    d.apply(Op::SetClass { class: renamed, was: Some("CS-NEW".into()) }, &base.for_project(&d.p)).unwrap();
    assert_eq!(d.p.line(six).unwrap().class, "CS-6");
    assert!(d.apply(Op::DeleteClass { name: "CS-6".into() }, &base.for_project(&d.p)).unwrap_err().contains("uses class CS-6"));
    // Removing the plant's copy of CS1 brings the catalogue's back.
    d.apply(Op::DeleteClass { name: "CS1".into() }, &base.for_project(&d.p)).unwrap();
    assert!(d.p.classes.iter().all(|c| c.name != "CS1"));
}

#[test]
fn templates_place_an_item_as_it_was_set_up() {
    let mut cat = Catalogue::builtin();
    let mut d = Doc::new(crate::examples::get("pumps").unwrap());
    let mut pump = d.p.equipment.iter().find(|e| e.tag == "P-101A").unwrap().clone();
    pump.dims.l = 2.4;
    pump.data.push(("Seal".into(), "Double".into()));
    // Saved by the shell as JSON; read back as a template (position, id and tag dropped).
    let json = serde_json::to_string(&serde_json::json!([{ "name": "Pump 50 m3/h", "equip": pump }])).unwrap();
    cat.add_templates("My templates", &json);
    assert!(cat.warnings.is_empty(), "{:?}", cat.warnings);
    let t = cat.templates.iter().find(|t| t.name == "Pump 50 m3/h").unwrap().clone();
    assert!(t.equip.tag.is_empty() && t.equip.id == 0 && t.equip.pos == [0.0; 3]);
    // Placed: a new tag, the sizes, nozzles and datasheet as saved.
    let n = d.p.equipment.len();
    let made = d.apply(Op::AddFromTemplate { equip: t.equip.clone(), pos: [30.0, 4.0, 0.0] }, &cat).unwrap();
    assert_eq!(d.p.equipment.len(), n + 1);
    let e = d.p.equip(*made.last().unwrap()).unwrap();
    assert!(e.tag.starts_with('P') && e.tag != "P-101A", "{}", e.tag);
    assert_eq!((e.dims.l, e.nozzles.len(), e.pos), (2.4, pump.nozzles.len(), [30.0, 4.0, 0.0]));
    assert!(e.data.iter().any(|(k, v)| k == "Seal" && v == "Double"));
    // From a pack's templates folder; a broken one is reported, not used.
    cat.add_pack("Site pack", &[("templates/site.json".into(), serde_json::to_string(&serde_json::json!({ "name": "Site drum", "equip": crate::equipment::new(EquipKind::HorizontalDrum, "") })).unwrap())]);
    assert!(cat.templates.iter().any(|t| t.name == "Site drum" && t.origin == "Site pack"));
    let mut bad = pump.clone();
    bad.nozzles[0].nps = "7".into();
    cat.add_templates("My templates", &serde_json::to_string(&serde_json::json!([{ "name": "Bad", "equip": bad }])).unwrap());
    assert!(cat.templates.iter().all(|t| t.name != "Bad") && cat.warnings.iter().any(|w| w.contains("Bad")));
}
