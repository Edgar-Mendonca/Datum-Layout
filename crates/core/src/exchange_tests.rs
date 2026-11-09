//! Exchange tests (brief §4): each example's PCF opens in Datum Pipe's reader (copied into
//! `pcf::reader`) with the same nodes, sizes and components; the glTF export is well formed.

use crate::catalogue::Catalogue;
use crate::examples;
use crate::gltf;
use crate::pcf::{self, reader};
use crate::route::build;
use std::collections::BTreeMap;

#[test]
fn each_examples_pcf_reads_in_datum_pipe_with_the_same_nodes_sizes_and_components() {
    let cat = Catalogue::builtin();
    for ex in examples::list() {
        let p = examples::get(ex.id).unwrap();
        let b = build(&p, &cat);
        for l in &p.lines {
            let text = pcf::write(&p, &cat, Some(l.id)).unwrap();
            let r = reader::pcf(&text).unwrap_or_else(|e| panic!("{} {}: {e}", ex.name, l.number));
            assert_eq!(r.name, l.number);
            // Pipe warns that the file has no supports (0.1 has none), and reads a cap as a short
            // plain pipe (its reader has no cap yet: a known limit, noted for Datum Pipe).
            let caps = b.parts.iter().filter(|q| q.line == l.id && q.kind == "cap").count();
            let sups = b.parts.iter().filter(|q| q.line == l.id && q.kind == "support").count();
            let mut want = if sups == 0 { vec!["The file has no supports: add anchors and supports before analysing.".to_string()] } else { vec![] };
            if caps > 0 { want.insert(0, format!("{caps} × CAP: read as plain pipe where it has two ends, otherwise left out.")); }
            assert_eq!(r.warnings, want, "{} {}", ex.name, l.number);
            let ours: Vec<_> = b.parts.iter().filter(|q| q.line == l.id && q.kind != "bolts").collect();
            // Nodes: every distinct port point (to 1 mm), plus elbow corners and tee centres.
            let mut pts: Vec<[f64; 3]> = vec![];
            let mut add = |p: [f64; 3]| { let q = [p[0], p[2], -p[1]]; if !pts.iter().any(|x| (0..3).all(|k| (x[k] - q[k]).abs() < 1e-3)) { pts.push(q); } };
            for q in &ours {
                for pt in &q.ports { add(pt.p); }
                if let Some(c) = q.corner { add(c); }
                // A one-port part (cap, blind) is written with its closed end as a second point.
                if q.ports.len() == 1 && !matches!(q.kind, "weldolet" | "sockolet") { add(crate::parts::sub(q.ports[0].p, crate::parts::mul(q.ports[0].d, q.length))); }
            }
            assert_eq!(r.nodes.len(), pts.len(), "{} {}: nodes", ex.name, l.number);
            for n in &r.nodes { assert!(pts.iter().any(|x| (0..3).all(|k| (x[k] - n[k]).abs() < 2e-5)), "{} {}: node {n:?} not ours", ex.name, l.number); }
            // Components: elbows → bends with our radius; tees; valves and flanges as rigid pieces.
            let elbows: Vec<_> = ours.iter().filter(|q| q.kind.starts_with("elbow")).collect();
            assert_eq!(r.bends.len(), elbows.len());
            for (e, (_, rr)) in elbows.iter().zip(&r.bends) {
                let want = crate::parts::dist(e.ports[0].p, e.corner.unwrap()) / (std::f64::consts::FRAC_PI_4 * if e.kind == "elbow90" { 1.0 } else { 0.5 }).tan();
                assert!((rr - want).abs() < 1e-4, "bend radius {rr} vs {want}");
            }
            assert_eq!(r.tees.len(), ours.iter().filter(|q| matches!(q.kind, "tee" | "cross" | "weldolet" | "sockolet")).count());
            let count = |k: reader::Rigid| r.pieces.iter().filter(|x| x.rigid == Some(k)).count();
            assert_eq!(count(reader::Rigid::Valve), ours.iter().filter(|q| matches!(q.kind, "gate" | "globe" | "check" | "ball" | "butterfly")).count());
            assert_eq!(count(reader::Rigid::Flange), ours.iter().filter(|q| matches!(q.kind, "flange" | "blind")).count());
            assert_eq!(count(reader::Rigid::Rigid), ours.iter().filter(|q| q.kind == "strainer").count());
            assert_eq!(r.pieces.iter().filter(|x| x.to_nps.is_some()).count(), ours.iter().filter(|q| q.kind == "reducer").count());
            // Sizes: the same pipe length in each size.
            let mut mine: BTreeMap<String, f64> = BTreeMap::new();
            for q in ours.iter().filter(|q| q.kind == "pipe") { *mine.entry(q.nps.clone()).or_default() += q.length; }
            for (nps, len) in mine {
                let theirs: f64 = r.pieces.iter().filter(|x| x.nps == Some(nps.as_str()) && x.rigid.is_none() && x.to_nps.is_none()).map(|x| crate::parts::dist(r.nodes[x.a], r.nodes[x.b])).sum();
                // Pipe also reads gaskets, elbow halves and tee arms as plain pieces of the same size.
                assert!(theirs + 1e-6 >= len, "{} {}: {nps}\" {theirs} < {len}", ex.name, l.number);
            }
        }
        // The whole plant in one file reads too.
        let all = pcf::write(&p, &cat, None).unwrap();
        assert!(reader::pcf(&all).is_ok());
    }
}

#[test]
fn the_gltf_export_is_well_formed() {
    let cat = Catalogue::builtin();
    for ex in examples::list() {
        let p = examples::get(ex.id).unwrap();
        let b = build(&p, &cat);
        let glb = gltf::glb(&p, &b, 1);
        let (nodes, tris) = gltf::verify(&glb).unwrap();
        assert_eq!(nodes, b.parts.iter().filter(|q| !q.shapes.is_empty()).count());
        assert!(tris > 1000);
    }
    // An empty plant still writes a valid file.
    let p = crate::model::Project::new();
    assert!(gltf::verify(&gltf::glb(&p, &build(&p, &cat), 1)).is_ok());
}

#[test]
fn each_examples_pcf_reads_back_into_the_same_lines() {
    use crate::doc::{Doc, Op};
    use crate::model::{End, ItemKind};
    use crate::parts::dist;
    let cat = Catalogue::builtin();
    for ex in examples::list() {
        let p = examples::get(ex.id).unwrap();
        let text = pcf::write(&p, &cat, None).unwrap();
        let mut empty = p.clone();
        empty.lines.clear();
        let mut d = Doc::new(empty);
        let made = d.apply(Op::ImportPcf { text }, &cat).unwrap_or_else(|e| panic!("{}: {e}", ex.name));
        assert_eq!(made.len(), p.lines.len(), "{}", ex.name);
        let num = |q: &crate::model::Project, id: u32| q.line(id).map(|l| l.number.clone()).unwrap_or_default();
        for l in &p.lines {
            let m = d.p.lines.iter().find(|x| x.number == l.number).unwrap_or_else(|| panic!("{}: {} not read", ex.name, l.number));
            let want = crate::route::path(&p, &cat, l);
            let mut tidy = l.clone(); tidy.pts = want.clone(); crate::doc::tidy(&mut tidy);
            assert_eq!(m.pts.len(), tidy.pts.len(), "{} {}: {:?} vs {:?}", ex.name, l.number, m.pts, tidy.pts);
            for (a, b) in m.pts.iter().zip(&tidy.pts) { assert!(dist(*a, *b) < 1e-4, "{} {}", ex.name, l.number); }
            assert_eq!((m.nps.as_str(), m.class.as_str(), m.rating), (l.nps.as_str(), l.class.as_str(), l.rating), "{} {}", ex.name, l.number);
            for (a, b) in [(&m.from, &l.from), (&m.to, &l.to)] {
                match (a, b) {
                    (End::Branch { line: x, .. }, End::Branch { line: y, .. }) => assert_eq!(num(&d.p, *x), num(&p, *y)),
                    _ => assert_eq!(a, b, "{} {}", ex.name, l.number),
                }
            }
            let kinds = |items: &[crate::model::Item]| { let mut k: Vec<ItemKind> = items.iter().map(|i| i.kind).collect(); k.sort_by_key(|k| format!("{k:?}")); k };
            assert_eq!(kinds(&m.items), kinds(&l.items), "{} {}", ex.name, l.number);
        }
        // The same parts and the same findings.
        let (b0, b1) = (build(&p, &cat), build(&d.p, &cat));
        assert_eq!(b0.parts.len(), b1.parts.len(), "{}", ex.name);
        assert_eq!(b0.issues.len(), b1.issues.len(), "{}: {:?}", ex.name, b1.issues);
        let c0 = crate::checks::connection_issues(&p, &cat, &b0).len();
        assert_eq!(crate::checks::connection_issues(&d.p, &cat, &b1).len(), c0, "{}", ex.name);
    }
    // Not a piping component file.
    let mut d = Doc::new(examples::get("pumps").unwrap());
    assert!(d.apply(Op::ImportPcf { text: "hello\nworld".into() }, &cat).is_err());
}

#[test]
fn the_line_list_reads_back_and_is_checked_against_the_model() {
    use crate::doc::{read_line_list, Doc, Op};
    let cat = Catalogue::builtin();
    let p = examples::get("pumps").unwrap();
    let csv = crate::lists::csv(&crate::lists::lists(&p, &cat), "lines").unwrap();
    let (rows, skipped) = read_line_list(&csv).unwrap();
    assert_eq!((rows.len(), skipped), (p.lines.len(), 0));
    let mut d = Doc::new(p.clone());
    d.apply(Op::ImportLineList { csv }, &cat).unwrap();
    let list = |d: &Doc| crate::checks::line_list_issues(&d.p);
    assert!(list(&d).is_empty(), "{:?}", list(&d));
    // A line made larger is reported; taking the list's values puts it back.
    let l = d.p.lines[0].clone();
    d.apply(Op::Resize { line: l.id, nps: "10".into() }, &cat).unwrap();
    assert_eq!(list(&d).len(), 1, "{:?}", list(&d));
    assert!(list(&d)[0].text.contains("size 8"), "{:?}", list(&d));
    assert!(list(&d)[0].text.contains("number"), "{:?}", list(&d));
    d.apply(Op::UseListedLine { line: l.id, number: l.number.clone() }, &cat).unwrap();
    assert_eq!(d.p.line(l.id).unwrap().nps, l.nps);
    assert!(list(&d).is_empty());
    // Another spreadsheet's columns, sizes and quoting.
    let (rows, skipped) = read_line_list("\u{feff}Line No;Size;Spec;Fluid code;Insulation;Remarks\n\"6\"\"-P-2001\";6\";CS300;HC;50;\"a, b\"\n;4;CS1;P;0;\n2\"-W-1;DN50;UW150;W;0;\n").unwrap();
    assert_eq!((rows.len(), skipped), (2, 1));
    assert_eq!((rows[0].number.as_str(), rows[0].nps.as_str(), rows[0].class.as_str(), rows[0].insulation, rows[0].notes.as_str()), ("6\"-P-2001", "6", "CS300", 0.05, "a, b"));
    assert_eq!(rows[1].nps, "2");
    // Lines missing from the model, and lines not on the list.
    let mut d = Doc::new(p);
    d.apply(Op::ImportLineList { csv: "Line number,Size\n9\"-X-1,8\n".into() }, &cat).unwrap();
    let iss = list(&d);
    assert!(iss.iter().any(|i| i.text.contains("not in the model")));
    assert_eq!(iss.iter().filter(|i| i.text.contains("not on the line list")).count(), d.p.lines.len());
    assert!(read_line_list("Size,Class\n6,CS1\n").is_err());
}

#[test]
fn a_plant_package_places_equipment_lists_lines_and_routes_them_in_the_pids_order() {
    use crate::doc::{Doc, Op};
    let cat = Catalogue::builtin();
    let json = include_str!("../../../docs/formats/pump-transfer.package.json");
    let mut d = Doc::new(crate::model::Project::new());
    let made = d.apply(Op::ImportPackage { json: json.into() }, &cat).unwrap();
    assert_eq!(made.len(), 3);
    assert_eq!(d.p.equipment.iter().map(|e| e.tag.as_str()).collect::<Vec<_>>(), ["T-201", "P-201", "V-201"]);
    let pump = d.p.equipment.iter().find(|e| e.tag == "P-201").unwrap();
    assert_eq!(pump.uid, "eq-pump-0001");
    assert_eq!(pump.nozzles.iter().find(|n| n.tag == "D").map(|n| (n.nps.as_str(), n.rating)), Some(("4", 300)));
    assert!(pump.data.iter().any(|x| x.0 == "Flow" && x.1 == "60 m³/h"));
    assert_eq!(d.p.line_list.len(), 3);
    assert_eq!(d.p.line_list[1].components, ["check", "gate"]);
    // Before routing, every listed line is missing from the model.
    let list = |d: &Doc| crate::checks::line_list_issues(&d.p);
    assert_eq!(list(&d).iter().filter(|i| i.text.contains("not in the model")).count(), 3);
    // Route the listed lines: the two nozzle-to-nozzle lines are made, in the P&ID's order.
    d.apply(Op::RouteListed { numbers: vec![] }, &cat).unwrap();
    assert_eq!(d.p.lines.len(), 2);
    for l in &d.p.lines {
        let r = d.p.line_list.iter().find(|r| r.number == l.number).unwrap();
        assert_eq!(crate::package::order(l), r.components, "{}", l.number);
        assert_eq!((l.class.as_str(), l.rating), (r.class.as_str(), r.rating));
    }
    assert!((d.p.lines[1].insulation - 0.04).abs() < 1e-9);
    // Only the branch is left to route by hand; nothing else differs from the P&ID.
    let iss = list(&d);
    assert_eq!(iss.len(), 1, "{iss:?}");
    assert!(iss[0].text.contains("2003"));
    let b = build(&d.p, &cat);
    assert!(b.issues.iter().all(|i| i.kind != "routing"), "{:?}", b.issues);
    // A valve taken out shows as a difference in the components.
    let l = d.p.lines[1].clone();
    let gate = l.items.iter().find(|i| i.kind == crate::model::ItemKind::Gate).unwrap().id;
    d.apply(Op::Delete { ids: vec![gate] }, &cat).unwrap();
    assert!(list(&d).iter().any(|i| i.text.contains("components check, gate (the model has check)")), "{:?}", list(&d));
    // Importing again updates rather than duplicates.
    let made = d.apply(Op::ImportPackage { json: json.into() }, &cat).unwrap();
    assert!(made.is_empty());
    assert_eq!((d.p.equipment.len(), d.p.line_list.len()), (3, 3));
    // Not a package.
    assert!(d.apply(Op::ImportPackage { json: "{\"format\":\"other\"}".into() }, &cat).is_err());
    assert!(d.apply(Op::ImportPackage { json: "not json".into() }, &cat).is_err());
}
