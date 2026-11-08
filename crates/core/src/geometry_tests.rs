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
    let br = d.apply(Op::Route { from: RouteEnd::Branch { line: 1, item: tee }, via: vec![], to: RouteEnd::Point { p: [6.0, 6.0, 4.0] }, nps: Some("3".into()) }, &cat).unwrap()[0];
    let b = build(&d.p, &cat);
    assert!(b.issues.is_empty(), "{:?}", b.issues);
    let t = b.parts.iter().find(|q| q.kind == "tee").unwrap();
    assert_eq!(t.nps2, "3", "a reducing tee for the 3\" branch");
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
    d.apply(Op::Route { from: RouteEnd::Branch { line: 1, item: tee }, via: vec![], to: RouteEnd::Point { p: [10.0, 5.0, 3.0] }, nps: Some("6".into()) }, &cat).unwrap();
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
