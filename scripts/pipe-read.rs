// Copied into a clone of Datum Pipe as an example and run there by scripts/verify.sh; Datum
// Layout never links Pipe's code.
//! Reads Datum Layout's PCF exports with Datum Pipe's own import (its real crate), then anchors
//! every free end and runs Pipe's stress analysis, to prove the hand-over end to end.
use datum_core::model::{SupKind, Support};
use std::collections::HashMap;
fn main() {
    let dir = std::env::args().nth(1).unwrap();
    let mut names: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "pcf") && p.to_string_lossy().contains("__")).collect();
    names.sort();
    for f in names {
        let text = std::fs::read_to_string(&f).unwrap();
        match datum_core::import::pcf(&text) {
            Err(e) => println!("FAIL {}: {e}", f.display()),
            Ok(r) => {
                let mut m = r.model;
                let mut deg: HashMap<u32, usize> = HashMap::new();
                for e in &m.elements { *deg.entry(e.from).or_default() += 1; *deg.entry(e.to).or_default() += 1; }
                let mut ends: Vec<u32> = deg.iter().filter(|(_, d)| **d == 1).map(|(n, _)| *n).collect();
                ends.sort();
                let mut id = m.max_id() + 1;
                for n in &ends { m.supports.push(Support { id, node: *n, kind: SupKind::Anchor, tag: format!("A{n}"), ..Default::default() }); id += 1; }
                m.next = id;
                let bends = m.elements.iter().filter(|e| matches!(e.comp, datum_core::model::Comp::Bend { .. })).count();
                let tees = m.elements.iter().filter(|e| matches!(e.comp, datum_core::model::Comp::Tee { .. })).count();
                let rigid = m.elements.iter().filter(|e| matches!(e.comp, datum_core::model::Comp::Rigid { .. })).count();
                let reds = m.elements.iter().filter(|e| matches!(e.comp, datum_core::model::Comp::Reducer { .. })).count();
                let res = datum_core::solve::analyze(&m);
                let (ok, imb) = match &res { Ok(r) => (true, r.cases.iter().map(|c| c.imbalance).fold(0.0, f64::max)), Err(e) => { println!("     analysis: {e}"); (false, 0.0) } };
                println!("{} {}: {} components read, {} elements, {} bends, {} tees, {} rigid, {} reducers, warnings {:?}; analysis {} (largest imbalance {:.1e})",
                    if ok { "ok  " } else { "FAIL" }, f.file_name().unwrap().to_string_lossy(), r.components, m.elements.len(), bends, tees, rigid, reds, r.warnings, if ok { "solved" } else { "failed" }, imb);
            }
        }
    }
}
