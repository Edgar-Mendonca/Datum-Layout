//! Hardening (FAMILY §11): failed edits change nothing; random plants and random edits never
//! panic and keep their rules; damaged files open as far as possible or give a clear error; a
//! plant with 2,000 components builds and checks quickly; hostile values are clamped on load.

use crate::catalogue::{Catalogue, NPS};
use crate::checks;
use crate::doc::{Doc, Op, RouteEnd};
use crate::examples;
use crate::model::{self, EquipKind, ItemKind, Project, Term};
use crate::route::build;
use std::panic;

/// A small random generator (no extra crate).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 }
    fn f(&mut self, lo: f64, hi: f64) -> f64 { lo + (hi - lo) * (self.next() % 10_000) as f64 / 10_000.0 }
    fn pick<T: Copy>(&mut self, v: &[T]) -> T { v[(self.next() % v.len() as u64) as usize] }
}

const KINDS: [ItemKind; 13] = [ItemKind::Butterfly, ItemKind::Weldolet, ItemKind::Sockolet, ItemKind::Gate, ItemKind::Globe, ItemKind::Check, ItemKind::Ball, ItemKind::Strainer, ItemKind::FlangePair, ItemKind::Tee, ItemKind::Reducer, ItemKind::Cross, ItemKind::Support];

fn random_op(r: &mut Rng, p: &Project) -> Op {
    let lines: Vec<u32> = p.lines.iter().map(|l| l.id).collect();
    let eqs: Vec<u32> = p.equipment.iter().map(|e| e.id).collect();
    let pt = |r: &mut Rng| [r.f(-5.0, 40.0), r.f(-5.0, 30.0), r.f(-1.0, 12.0)];
    let line = |r: &mut Rng| if lines.is_empty() { 999 } else { r.pick(&lines) };
    match r.next() % 11 {
        0 => Op::AddEquipment { kind: r.pick(&EquipKind::ALL), pos: [r.f(0.0, 30.0), r.f(0.0, 20.0), 0.0] },
        1 => {
            let e = if eqs.is_empty() { None } else { p.equip(r.pick(&eqs)) };
            let from = match e { Some(e) if !e.nozzles.is_empty() => RouteEnd::Nozzle { equip: e.id, nozzle: e.nozzles[(r.next() % e.nozzles.len() as u64) as usize].tag.clone() }, _ => RouteEnd::Point { p: pt(r) } };
            Op::Route { from, via: vec![pt(r)], to: RouteEnd::Point { p: pt(r) }, nps: Some(r.pick(&NPS).to_string()) }
        }
        2 | 3 => Op::Drop { line: line(r), kind: r.pick(&KINDS), at: pt(r), dir: [r.f(-1.0, 1.0), r.f(-1.0, 1.0), r.f(-1.0, 1.0)], nps2: String::new() },
        4 => Op::Resize { line: line(r), nps: r.pick(&NPS).to_string() },
        5 => Op::MoveRun { line: line(r), seg: (r.next() % 6) as usize, delta: [r.f(-3.0, 3.0), r.f(-3.0, 3.0), r.f(-3.0, 3.0)] },
        6 => {
            let ids: Vec<u32> = p.lines.iter().flat_map(|l| l.items.iter().map(|i| i.id)).collect();
            if ids.is_empty() { Op::Delete { ids: vec![] } } else { let item = r.pick(&ids); Op::MoveItem { line: p.lines.iter().find(|l| l.items.iter().any(|i| i.id == item)).map_or(0, |l| l.id), item, at: pt(r) } }
        }
        7 => Op::SetTerm { line: line(r), start: r.next() % 2 == 0, term: r.pick(&[Term::Open, Term::Cap, Term::Blind]) },
        8 => { let all: Vec<u32> = eqs.iter().copied().chain(lines.iter().copied()).collect(); Op::Delete { ids: if all.is_empty() { vec![] } else { vec![r.pick(&all)] } } }
        9 => {
            let mut e = if eqs.is_empty() { return Op::Delete { ids: vec![] } } else { p.equip(r.pick(&eqs)).cloned().unwrap_or_default() };
            e.pos = pt(r);
            e.rot = r.f(-720.0, 720.0);
            e.dims.d = r.f(-1.0, 5.0);
            Op::SetEquipment(e)
        }
        _ => Op::Route { from: RouteEnd::Point { p: pt(r) }, via: vec![], to: RouteEnd::Point { p: pt(r) }, nps: None },
    }
}

#[test]
fn a_failed_edit_leaves_the_plant_exactly_as_it_was() {
    let cat = Catalogue::builtin();
    for ex in examples::list() {
        let mut d = Doc::new(examples::get(ex.id).unwrap());
        let before = d.p.clone();
        let bad = [
            Op::Resize { line: d.p.lines[0].id, nps: "7".into() },
            Op::MoveRun { line: d.p.lines[0].id, seg: 99, delta: [0.0, 0.0, 1.0] },
            Op::Drop { line: 999_999, kind: ItemKind::Gate, at: [0.0; 3], dir: [0.0; 3], nps2: String::new() },
            Op::Batch(vec![Op::AddEquipment { kind: EquipKind::Tank, pos: [0.0; 3] }, Op::Delete { ids: vec![] }, Op::Resize { line: 0, nps: "6".into() }]),
            Op::SetMeta { name: " ".into(), notes: String::new(), plant: String::new(), unit: String::new(), units: "SI".into() },
        ];
        for op in bad {
            assert!(d.apply(op, &cat).is_err());
            assert_eq!(d.p, before, "{}", ex.name);
            assert!(!d.dirty);
        }
    }
}

#[test]
fn random_plants_and_edits_never_panic_and_keep_their_rules() {
    let cat = Catalogue::builtin();
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    for round in 0..30 {
        let mut d = Doc::new(if round % 3 == 0 { Project::new() } else { examples::get(examples::list()[round % 4].id).unwrap() });
        for step in 0..25 {
            let op = random_op(&mut r, &d.p);
            let res = panic::catch_unwind(panic::AssertUnwindSafe(|| { let _ = d.apply(op.clone(), &cat); }));
            assert!(res.is_ok(), "round {round} step {step}: {op:?} panicked");
            if step % 5 == 0 { d.undo(); }
            // Rules: ids unique, items on segments that exist, and the plant builds, checks and round-trips.
            let mut ids = std::collections::HashSet::new();
            assert!(d.p.equipment.iter().map(|e| e.id).chain(d.p.lines.iter().flat_map(|l| std::iter::once(l.id).chain(l.items.iter().map(|i| i.id)))).all(|i| ids.insert(i)));
            for l in &d.p.lines { assert!(l.pts.len() >= 2 && l.items.iter().all(|i| i.seg + 1 < l.pts.len())); }
            let b = panic::catch_unwind(|| build(&d.p, &cat)).unwrap_or_else(|_| panic!("build panicked, round {round} step {step}"));
            let _ = panic::catch_unwind(|| checks::run(&d.p, &cat, &b)).unwrap_or_else(|_| panic!("checks panicked, round {round} step {step}"));
            let (back, w) = model::from_json(&model::to_json(&d.p)).unwrap();
            assert_eq!(back, d.p, "{w:?}");
        }
    }
}

#[test]
fn damaged_files_never_panic_they_open_or_explain() {
    let text = model::to_json(&examples::get("pumps").unwrap());
    let bytes = text.as_bytes();
    let mut tries: Vec<String> = (0..60).map(|k| String::from_utf8_lossy(&bytes[..bytes.len() * k / 60]).into_owned()).collect();
    let mut r = Rng(7);
    for _ in 0..200 {
        let mut b = bytes.to_vec();
        for _ in 0..4 { let i = (r.next() % b.len() as u64) as usize; b[i] = (r.next() % 256) as u8; }
        tries.push(String::from_utf8_lossy(&b).into_owned());
    }
    // Hostile values: huge, negative, not-a-number positions and sizes; loops of references.
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    v["equipment"][0]["pos"] = serde_json::json!([1e308, -1e308, 0]);
    v["equipment"][0]["dims"]["d"] = serde_json::json!(-5);
    v["lines"][0]["pts"] = serde_json::json!([[0, 0, 0]]);
    v["lines"][1]["from"] = serde_json::json!({ "kind": "branch", "line": v["lines"][1]["id"], "item": 1 });
    v["lines"][2]["nps"] = serde_json::json!("99");
    v["plot"]["grid"] = serde_json::json!(0);
    v["next"] = serde_json::json!(u32::MAX);
    tries.push(v.to_string());
    let cat = Catalogue::builtin();
    for t in &tries {
        let r = panic::catch_unwind(|| model::from_json(t));
        let Ok(r) = r else { panic!("the reader panicked") };
        if let Ok((p, _)) = r {
            let b = panic::catch_unwind(|| build(&p, &cat)).expect("build panicked on a damaged file");
            let _ = panic::catch_unwind(|| checks::run(&p, &cat, &b)).expect("checks panicked on a damaged file");
            assert!(p.equipment.iter().all(|e| e.pos.iter().all(|x| x.abs() <= model::HORIZON) && e.dims.d > 0.0));
        }
    }
}

#[test]
fn a_plant_with_2000_components_builds_and_checks_quickly() {
    let cat = Catalogue::builtin();
    // Twenty-four copies of the pump example side by side: over 2,000 parts.
    let p = examples::repeated("pumps", 24).unwrap();
    let t = std::time::Instant::now();
    let b = build(&p, &cat);
    let issues = checks::run(&p, &cat, &b);
    let s = crate::scene::scene(&p, &cat, &b, 1);
    let ms = t.elapsed().as_millis();
    assert!(b.parts.len() >= 2000, "{} parts", b.parts.len());
    assert!(issues.is_empty(), "{:?}", issues.first());
    let limit = if cfg!(debug_assertions) { 8000 } else { 1500 };
    assert!(ms < limit, "{} parts built, checked and meshed in {ms} ms (limit {limit})", b.parts.len());
    eprintln!("speed gate: {} parts, {} triangles, {ms} ms", b.parts.len(), s.triangles);
}
