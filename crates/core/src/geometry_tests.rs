//! Geometry and routing tests (brief §4): parts match their tables, connected ports coincide and
//! face each other, routed lines have no gaps or overlaps, lengths add up, turns of 90° and 45°
//! work, tight turns are reported and not drawn, and resizing re-sizes everything.

use crate::catalogue::{Catalogue, NPS};
use crate::checks::{connection_issues, connections};
use crate::doc::{Doc, Op, RouteEnd};
use crate::model::{End, EquipKind, Item, ItemKind, Line, Project, Radius, Term, P3};
use crate::parts::{dist, dot, Shape};
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
