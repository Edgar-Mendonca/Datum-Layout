//! Lists: the line list, the equipment list with nozzles, and the material take-off per line,
//! all from the generated parts; each exports to CSV.

use crate::catalogue::Catalogue;
use crate::model::{End, Project, Term, P3};
use crate::route::{build, Part};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize, Clone, Debug)]
pub struct LineRow { pub id: u32, pub number: String, pub from: String, pub to: String, pub nps: String, pub class: String, pub rating: u32, pub fluid: String, pub insulation: f64, pub length: f64, pub components: usize }
#[derive(Serialize, Clone, Debug)]
pub struct NozzleRow { pub tag: String, pub nps: String, pub rating: u32, pub facing: String, pub service: String, pub at: P3, pub dir: P3, pub line: String }
#[derive(Serialize, Clone, Debug)]
pub struct EquipRow { pub id: u32, pub tag: String, pub kind: String, pub at: P3, pub rot: f64, pub sizes: Vec<(String, f64)>, pub data: Vec<(String, String)>, pub nozzles: Vec<NozzleRow> }
#[derive(Serialize, Clone, Debug)]
pub struct MtoRow { pub line: String, pub group: &'static str, pub what: String, pub nps: String, pub qty: f64, pub unit: &'static str }
#[derive(Serialize, Clone, Debug, Default)]
pub struct Lists { pub lines: Vec<LineRow>, pub equipment: Vec<EquipRow>, pub mto: Vec<MtoRow> }

fn end_text(p: &Project, e: &End) -> String {
    match e {
        End::Point { term: Term::Open } => "Open end".into(),
        End::Point { term: Term::Cap } => "Cap".into(),
        End::Point { term: Term::Blind } => "Blind flange".into(),
        End::Nozzle { equip, nozzle } => format!("{} / {nozzle}", p.equip(*equip).map(|e| e.tag.clone()).unwrap_or_default()),
        End::Branch { line, .. } => format!("Branch of {}", p.line(*line).map(|l| l.number.clone()).unwrap_or_default()),
    }
}

/// Plain words for a part in the take-off: (group, description).
fn describe(q: &Part, class: &str, sched: &str, material: &str) -> Option<(&'static str, String)> {
    let r = q.rating;
    Some(match q.kind {
        "pipe" => ("Pipe", format!("Pipe, {material}, sch {sched}, bevelled ends")),
        "elbow90" => ("Fittings", format!("Elbow 90° {}, butt weld, sch {sched}", q.note)),
        "elbow45" => ("Fittings", format!("Elbow 45° {}, butt weld, sch {sched}", q.note)),
        "tee" if q.nps2 != q.nps => ("Fittings", format!("Tee, reducing {}\" × {}\", butt weld", q.nps, q.nps2)),
        "tee" => ("Fittings", "Tee, equal, butt weld".to_string()),
        "reducer" => ("Fittings", format!("Reducer, concentric {}\" × {}\", butt weld", q.nps, q.nps2)),
        "cap" => ("Fittings", "Cap, butt weld".into()),
        "flange" => ("Flanges", format!("Flange, weld neck, class {r}, raised face")),
        "blind" => ("Flanges", format!("Flange, blind, class {r}, raised face")),
        "gasket" => ("Gaskets", format!("Gasket, spiral wound, class {r}")),
        "bolts" => { let (n, d, l) = q.bolts?; ("Bolts", format!("Stud bolt set: {n} × {:.0} mm dia × {:.0} mm with nuts", d * 1000.0, l * 1000.0)) }
        "gate" => ("Valves", format!("Gate valve, class {r}, flanged RF")),
        "globe" => ("Valves", format!("Globe valve, class {r}, flanged RF")),
        "check" => ("Valves", format!("Check valve, swing, class {r}, flanged RF")),
        "ball" => ("Valves", format!("Ball valve, class {r}, flanged RF")),
        "strainer" => ("Valves", format!("Strainer, Y-type, class {r}, flanged RF")),
        _ => { let _ = class; return None }
    })
}

pub fn lists(p: &Project, cat: &Catalogue) -> Lists {
    let b = build(p, cat);
    let mut out = Lists::default();
    let mut by_line: BTreeMap<u32, Vec<&Part>> = BTreeMap::new();
    for q in b.parts.iter().filter(|q| q.line != 0) { by_line.entry(q.line).or_default().push(q); }
    for l in &p.lines {
        let parts = by_line.get(&l.id).cloned().unwrap_or_default();
        out.lines.push(LineRow {
            id: l.id, number: l.number.clone(), from: end_text(p, &l.from), to: end_text(p, &l.to), nps: l.nps.clone(), class: l.class.clone(), rating: l.rating, fluid: l.fluid.clone(), insulation: l.insulation,
            length: parts.iter().map(|q| q.length).sum(),
            components: parts.iter().filter(|q| !matches!(q.kind, "pipe" | "bolts")).count(),
        });
        // Take-off: pipe by length per size, everything else counted by description and size.
        let class = cat.class(&l.class);
        let material = class.map(|c| c.material.to_lowercase()).unwrap_or_else(|| "carbon steel".into());
        let mut rows: BTreeMap<(usize, &'static str, String, usize), (f64, &'static str)> = BTreeMap::new();
        let group_order = |g: &str| ["Pipe", "Fittings", "Flanges", "Valves", "Gaskets", "Bolts"].iter().position(|x| *x == g).unwrap_or(9);
        for q in &parts {
            let sched = class.map(|c| c.schedule(&q.nps)).unwrap_or_else(|| "STD".into());
            let Some((g, what)) = describe(q, &l.class, &sched, &material) else { continue };
            let e = rows.entry((group_order(g), g, what, crate::catalogue::nps_index(&q.nps))).or_insert((0.0, if g == "Pipe" { "m" } else if g == "Bolts" { "set" } else { "ea" }));
            e.0 += if g == "Pipe" { q.length } else { 1.0 };
        }
        for ((_, g, what, k), (qty, unit)) in rows {
            out.mto.push(MtoRow { line: l.number.clone(), group: g, what, nps: crate::catalogue::NPS.get(k).map(|s| s.to_string()).unwrap_or_default(), qty: if unit == "m" { (qty * 1000.0).round() / 1000.0 } else { qty }, unit });
        }
    }
    for e in &p.equipment {
        let fields = crate::equipment::fields(e.kind);
        let get = |k: &str| match k { "d" => e.dims.d, "l" => e.dims.l, "h" => e.dims.h, "elev" => e.dims.elev, "d2" => e.dims.d2, "l2" => e.dims.l2, _ => e.dims.w };
        out.equipment.push(EquipRow {
            id: e.id, tag: e.tag.clone(), kind: e.kind.name().into(), at: e.pos, rot: e.rot,
            sizes: fields.iter().map(|(k, label)| (label.to_string(), get(k))).collect(),
            data: e.data.clone(),
            nozzles: e.nozzles.iter().map(|n| {
                let placed = crate::equipment::nozzle(e, n, cat);
                let me = End::Nozzle { equip: e.id, nozzle: n.tag.clone() };
                let line = p.lines.iter().find(|l| l.from == me || l.to == me).map(|l| l.number.clone()).unwrap_or_default();
                NozzleRow { tag: n.tag.clone(), nps: n.nps.clone(), rating: n.rating, facing: n.facing.clone(), service: n.service.clone(), at: placed.port.p, dir: placed.port.d, line }
            }).collect(),
        });
    }
    out
}

/// One CSV cell: quoted when needed, and text that a spreadsheet would run as a formula is
/// prefixed with ' (FAMILY §11).
pub fn cell(s: &str) -> String {
    let plain_number = s.parse::<f64>().is_ok();
    let s = if !plain_number && s.starts_with(['=', '+', '-', '@']) { format!("'{s}") } else { s.to_string() };
    if s.contains([',', '"', '\n', '\r']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s }
}

fn row(v: &[String]) -> String { v.iter().map(|c| cell(c)).collect::<Vec<_>>().join(",") + "\r\n" }
fn f(v: f64, dp: usize) -> String { format!("{v:.dp$}") }

/// A list as CSV text: "lines", "equipment" or "mto". Lengths in metres, positions in metres.
pub fn csv(l: &Lists, kind: &str) -> Result<String, String> {
    let mut s = String::new();
    match kind {
        "lines" => {
            s += &row(&["Line number", "From", "To", "Size (in)", "Class", "Rating", "Fluid", "Insulation (mm)", "Length (m)", "Components"].map(String::from));
            for r in &l.lines { s += &row(&[r.number.clone(), r.from.clone(), r.to.clone(), r.nps.clone(), r.class.clone(), r.rating.to_string(), r.fluid.clone(), f(r.insulation * 1000.0, 0), f(r.length, 3), r.components.to_string()]); }
        }
        "equipment" => {
            s += &row(&["Tag", "Type", "E (m)", "N (m)", "EL (m)", "Nozzle", "Size (in)", "Rating", "Facing", "Service", "Nozzle E (m)", "Nozzle N (m)", "Nozzle EL (m)", "Line"].map(String::from));
            for e in &l.equipment {
                if e.nozzles.is_empty() { s += &row(&[e.tag.clone(), e.kind.clone(), f(e.at[0], 3), f(e.at[1], 3), f(e.at[2], 3)]); }
                for n in &e.nozzles {
                    s += &row(&[e.tag.clone(), e.kind.clone(), f(e.at[0], 3), f(e.at[1], 3), f(e.at[2], 3), n.tag.clone(), n.nps.clone(), n.rating.to_string(), n.facing.clone(), n.service.clone(), f(n.at[0], 3), f(n.at[1], 3), f(n.at[2], 3), n.line.clone()]);
                }
            }
        }
        "mto" => {
            s += &row(&["Line", "Group", "Description", "Size (in)", "Quantity", "Unit"].map(String::from));
            for r in &l.mto { s += &row(&[r.line.clone(), r.group.into(), r.what.clone(), r.nps.clone(), if r.unit == "m" { f(r.qty, 3) } else { f(r.qty, 0) }, r.unit.into()]); }
        }
        _ => return Err(format!("There is no list called {kind}.")),
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_cells_are_safe() {
        assert_eq!(cell("=HYPERLINK(1)"), "'=HYPERLINK(1)");
        assert_eq!(cell("-12.5"), "-12.5");
        assert_eq!(cell("+P-101"), "'+P-101");
        assert_eq!(cell("6\"-P-1"), "\"6\"\"-P-1\"");
        assert_eq!(cell("a,b"), "\"a,b\"");
    }
}
