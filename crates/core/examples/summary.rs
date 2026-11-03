//! Each example's parts by kind and its findings, for checking examples by hand.
use datum_core::{catalogue::Catalogue, checks, examples, route};
fn main() {
    let cat = Catalogue::builtin();
    for ex in examples::list() {
        let p = examples::get(ex.id).unwrap();
        let b = route::build(&p, &cat);
        let mut k: std::collections::BTreeMap<&str, usize> = Default::default();
        for q in &b.parts { *k.entry(q.kind).or_default() += 1; }
        println!("{}: {} lines, {} equipment, {k:?}", ex.name, p.lines.len(), p.equipment.len());
        for i in checks::run(&p, &cat, &b) { println!("   {} {}", i.kind, i.text); }
    }
}
