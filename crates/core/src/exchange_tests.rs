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
            let mut want = vec!["The file has no supports: add anchors and supports before analysing.".to_string()];
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
                if q.ports.len() == 1 { add(crate::parts::sub(q.ports[0].p, crate::parts::mul(q.ports[0].d, q.length))); }
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
            assert_eq!(r.tees.len(), ours.iter().filter(|q| q.kind == "tee").count());
            let count = |k: reader::Rigid| r.pieces.iter().filter(|x| x.rigid == Some(k)).count();
            assert_eq!(count(reader::Rigid::Valve), ours.iter().filter(|q| matches!(q.kind, "gate" | "globe" | "check" | "ball")).count());
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
