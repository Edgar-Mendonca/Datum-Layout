//! The plant package: what a P&ID (Datum Schema) hands to the 3D layout. An open, documented JSON
//! format (docs/formats/plant-package.md): equipment with its class (DEXPI names), datasheet values
//! and nozzles; lines with their number, size, class, rating, fluid, insulation, both ends and the
//! components on them in flow order; every object with the P&ID's stable uid.
//!
//! Importing places the equipment not yet in the plant (in a row along the plot, to be arranged),
//! updates what is (nozzle sizes, ratings and services, datasheet values), and puts every line on
//! the line list with its ends and components, so the checks compare the 3D model with the P&ID:
//! lines missing or extra, sizes and classes, and the components' order. `route_listed` then routes
//! listed lines nozzle to nozzle and drops their components in the P&ID's order.

use crate::catalogue::{Catalogue, NPS, RATINGS};
use crate::doc::{apply_op, Op, RouteEnd};
use crate::model::{EquipKind, ItemKind, ListedLine, Nozzle, Project, P3};
use crate::parts::{add, dist, mul, sub, unit};
use serde::Deserialize;
use std::collections::BTreeMap;

pub const FORMAT: &str = "datum-plant-package";

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Package { pub format: String, pub version: u32, pub app: String, pub project: String, pub equipment: Vec<PEquip>, pub lines: Vec<PLine> }
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct PEquip { pub uid: String, pub tag: String, pub class: String, pub service: String, pub data: BTreeMap<String, serde_json::Value>, pub nozzles: Vec<PNozzle> }
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct PNozzle { pub tag: String, pub nps: String, pub rating: u32, pub service: String }
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct PLine { pub uid: String, pub number: String, pub nps: String, pub class: String, pub rating: u32, pub fluid: String, pub insulation_mm: f64, pub from: PEnd, pub to: PEnd, pub components: Vec<PComp>, pub notes: String }
/// A line's end: an equipment nozzle, a branch off another line, or open.
#[derive(Deserialize, Default, Clone)]
#[serde(default)]
pub struct PEnd { pub equipment: String, pub nozzle: String, pub line: String }
#[derive(Deserialize, Default)]
#[serde(default)]
pub struct PComp { pub uid: String, pub kind: String, pub tag: String }

/// The equipment kind for a DEXPI class (and the usual other names).
pub fn kind_of(class: &str) -> Option<EquipKind> {
    let c = class.to_lowercase();
    Some(if c.contains("inline") && c.contains("pump") { EquipKind::InlinePump }
    else if c.contains("pump") { EquipKind::Pump }
    else if c.contains("reciprocating") && c.contains("compressor") { EquipKind::RecipCompressor }
    else if c.contains("compressor") || c.contains("blower") { EquipKind::Compressor }
    else if c.contains("aircooled") || c.contains("air_cooled") || c.contains("air cooled") || c.contains("air cooler") || c.contains("fin fan") { EquipKind::AirCooler }
    else if c.contains("plate") && c.contains("exchanger") { EquipKind::PlateHx }
    else if c.contains("kettle") || c.contains("reboiler") { EquipKind::Kettle }
    else if c.contains("furnace") || c.contains("fired heater") || c.contains("firedheater") { EquipKind::Heater }
    else if c.contains("sphere") || c.contains("spherical") { EquipKind::Sphere }
    else if c.contains("filter") { EquipKind::Filter }
    else if c.contains("agitat") || c.contains("reactor") || c.contains("mixer") { EquipKind::Agitated }
    else if c.contains("package") || c.contains("skid") || c.contains("customequipment") { EquipKind::Package }
    else if c.contains("column") || c.contains("tower") { EquipKind::Column }
    else if c.contains("tank") { EquipKind::Tank }
    else if c.contains("exchanger") || c.contains("cooler") || c.contains("heater") || c.contains("condenser") { EquipKind::Exchanger }
    else if c.contains("horizontal") || c.contains("drum") { EquipKind::HorizontalDrum }
    else if c.contains("vessel") || c.contains("separator") || c.contains("receiver") { EquipKind::VerticalVessel }
    else { return None })
}

/// A component kind in the package's words (gate, globe, check, ball, strainer, reducer, tee, flange_pair).
pub fn item_kind(k: &str) -> Option<ItemKind> {
    let k = k.to_lowercase().replace(['-', ' '], "_");
    Some(match k.as_str() {
        "gate" | "gate_valve" | "gatevalve" => ItemKind::Gate,
        "globe" | "globe_valve" | "globevalve" => ItemKind::Globe,
        "check" | "check_valve" | "checkvalve" | "swing_check_valve" => ItemKind::Check,
        "ball" | "ball_valve" | "ballvalve" => ItemKind::Ball,
        "butterfly" | "butterfly_valve" | "butterflyvalve" => ItemKind::Butterfly,
        "strainer" | "y_strainer" | "ystrainer" => ItemKind::Strainer,
        "reducer" | "concentric_reducer" => ItemKind::Reducer,
        "tee" => ItemKind::Tee,
        "cross" => ItemKind::Cross,
        "weldolet" => ItemKind::Weldolet,
        "sockolet" => ItemKind::Sockolet,
        "flange_pair" | "flanged_joint" | "spectacle_blind" => ItemKind::FlangePair,
        _ => return None,
    })
}
pub fn kind_word(k: ItemKind) -> &'static str {
    match k { ItemKind::Gate => "gate", ItemKind::Globe => "globe", ItemKind::Check => "check", ItemKind::Ball => "ball", ItemKind::Butterfly => "butterfly", ItemKind::Strainer => "strainer", ItemKind::Reducer => "reducer", ItemKind::Tee => "tee", ItemKind::Cross => "cross", ItemKind::Weldolet => "weldolet", ItemKind::Sockolet => "sockolet", ItemKind::FlangePair => "flange_pair", ItemKind::Support => "support" }
}

fn size(s: &str) -> Option<&'static str> {
    let t = s.trim().trim_end_matches('"').replace(' ', "-");
    if let Some(dn) = t.to_uppercase().strip_prefix("DN") { return crate::pcf::import::nps_of(dn.trim().parse().ok()?, false); }
    NPS.iter().copied().find(|n| *n == t).or_else(|| crate::pcf::import::nps_of(t.parse().ok()?, true))
}

pub fn read(json: &str) -> Result<Package, String> {
    let pk: Package = serde_json::from_str(json).map_err(|e| format!("This is not a plant package Datum Layout can read: {e}"))?;
    if pk.format != FORMAT { return Err(format!("This file is not a plant package (its format is \"{}\", not \"{FORMAT}\").", pk.format)); }
    if pk.version > 1 { return Err(format!("This plant package is version {}; this Datum Layout reads version 1. Update Datum Layout.", pk.version)); }
    Ok(pk)
}

/// What an import did, for the window to say.
#[derive(Default, Debug, serde::Serialize)]
pub struct Report { pub placed: Vec<String>, pub updated: Vec<String>, pub listed: usize, pub warnings: Vec<String> }

/// Applies a package to the plant (see the module notes). Ids of new equipment go to `created`.
pub fn import(p: &mut Project, pk: &Package, created: &mut Vec<u32>) -> Result<Report, String> {
    let mut r = Report::default();
    // Where new equipment goes: a row along the south of the plot, left to right.
    let mut x = p.plot.e[0] + 4.0;
    let y = p.plot.n[0] + (p.plot.n[1] - p.plot.n[0]) * 0.35;
    for e in &p.equipment { x = x.max(e.pos[0] + 6.0); }
    for pe in &pk.equipment {
        if pe.tag.trim().is_empty() { r.warnings.push("An equipment item without a tag was left out.".into()); continue; }
        let existing = p.equipment.iter().position(|e| e.tag == pe.tag || (!pe.uid.is_empty() && e.uid == pe.uid));
        let idx = match existing {
            Some(i) => { r.updated.push(pe.tag.clone()); i }
            None => {
                let Some(kind) = kind_of(&pe.class) else { r.warnings.push(format!("{} ({}): no 3D shape for this class yet; place it by hand.", pe.tag, pe.class)); continue };
                let mut e = crate::equipment::new(kind, &pe.tag);
                let half = match kind { EquipKind::HorizontalDrum | EquipKind::Exchanger | EquipKind::Kettle | EquipKind::AirCooler | EquipKind::Heater | EquipKind::Package | EquipKind::PlateHx => e.dims.l / 2.0 + 1.0, EquipKind::Pump | EquipKind::Compressor | EquipKind::RecipCompressor => e.dims.l / 2.0 + 0.5, _ => e.dims.d / 2.0 };
                x += half;
                e.id = p.id();
                e.pos = [x, y, p.plot.ground];
                x += half + 3.0;
                created.push(e.id);
                r.placed.push(pe.tag.clone());
                p.equipment.push(e);
                p.equipment.len() - 1
            }
        };
        let e = &mut p.equipment[idx];
        if !pe.uid.is_empty() { e.uid = pe.uid.clone(); }
        for (k, v) in &pe.data {
            let v = match v { serde_json::Value::String(s) => s.clone(), other => other.to_string() };
            match e.data.iter_mut().find(|d| d.0 == *k) { Some(d) => d.1 = v, None => e.data.push((k.clone(), v)) }
        }
        if !pe.service.is_empty() && !e.data.iter().any(|d| d.0 == "Service") { e.data.insert(0, ("Service".into(), pe.service.clone())); }
        // Nozzles: matched by tag; those the 3D type does not have are added on top, to be placed.
        let top = e.nozzles.iter().map(|n| n.pos[2]).fold(0.0f64, f64::max);
        let mut extra = 0.0;
        for pn in &pe.nozzles {
            let nps = size(&pn.nps);
            if !pn.nps.is_empty() && nps.is_none() { r.warnings.push(format!("{} nozzle {}: size \"{}\" not read.", pe.tag, pn.tag, pn.nps)); }
            let n = match e.nozzles.iter_mut().find(|n| n.tag == pn.tag) {
                Some(n) => n,
                None => {
                    extra += 0.6;
                    r.warnings.push(format!("{} nozzle {} was added on top of the item: move it to its place in the Inspector.", pe.tag, pn.tag));
                    e.nozzles.push(Nozzle { tag: pn.tag.clone(), pos: [extra - 0.3, 0.6, top + 0.3], dir: [0.0, 0.0, 1.0], ..Default::default() });
                    e.nozzles.last_mut().unwrap()
                }
            };
            if let Some(s) = nps { n.nps = s.into(); }
            if RATINGS.contains(&pn.rating) { n.rating = pn.rating; }
            if !pn.service.is_empty() { n.service = pn.service.clone(); }
        }
    }
    // Lines: onto the line list (replacing a listed line of the same number).
    for pl in &pk.lines {
        if pl.number.trim().is_empty() { r.warnings.push("A line without a number was left out.".into()); continue; }
        let mut comps = vec![];
        for c in &pl.components {
            match item_kind(&c.kind) { Some(k) => comps.push(kind_word(k).to_string()), None => r.warnings.push(format!("Line {}: component \"{}\" has no 3D part yet.", pl.number, c.kind)) }
        }
        let end = |e: &PEnd| if !e.equipment.is_empty() { format!("{} / {}", e.equipment, e.nozzle) } else if !e.line.is_empty() { format!("Branch of {}", e.line) } else { "Open end".into() };
        let row = ListedLine {
            number: pl.number.clone(), nps: size(&pl.nps).unwrap_or("").into(), class: pl.class.clone(), rating: if RATINGS.contains(&pl.rating) { pl.rating } else { 0 },
            fluid: pl.fluid.clone(), insulation: (pl.insulation_mm.max(0.0) / 1000.0).min(1.0), from: end(&pl.from), to: end(&pl.to), notes: pl.notes.clone(),
            uid: pl.uid.clone(), components: comps,
        };
        match p.line_list.iter_mut().find(|x| x.number == row.number) { Some(x) => *x = row, None => p.line_list.push(row) }
        r.listed += 1;
    }
    Ok(r)
}

/// "TAG / NOZZLE" back to its parts.
fn nozzle_end(s: &str) -> Option<(String, String)> { let (a, b) = s.split_once(" / ")?; Some((a.trim().into(), b.trim().into())) }

/// Routes listed lines not yet in the model (all of them, or those in `numbers`) from nozzle to
/// nozzle, gives each the list's number, size, class, rating, fluid and insulation, and drops its
/// components along it in the list's order. Gives the ids made and what could not be done.
pub fn route_listed(p: &mut Project, cat: &Catalogue, numbers: &[String], created: &mut Vec<u32>) -> Result<Vec<String>, String> {
    let mut notes = vec![];
    let todo: Vec<ListedLine> = p.line_list.iter().filter(|r| (numbers.is_empty() || numbers.contains(&r.number)) && !p.lines.iter().any(|l| l.number == r.number)).cloned().collect();
    if todo.is_empty() { return Err("Every listed line is already in the model.".into()); }
    let mut routed = 0;
    for r in todo {
        let (Some((ft, fnz)), Some((tt, tnz))) = (nozzle_end(&r.from), nozzle_end(&r.to)) else { notes.push(format!("{}: route it by hand (it starts or ends on another line or an open end).", r.number)); continue };
        let id_of = |tag: &str, p: &Project| p.equipment.iter().find(|e| e.tag == tag).map(|e| e.id);
        let (Some(fe), Some(te)) = (id_of(&ft, p), id_of(&tt, p)) else { notes.push(format!("{}: {} or {} is not in the plant.", r.number, ft, tt)); continue };
        let mut made = vec![];
        let nps = if r.nps.is_empty() { None } else { Some(r.nps.clone()) };
        let op = Op::Route { from: RouteEnd::Nozzle { equip: fe, nozzle: fnz.clone() }, via: vec![], to: RouteEnd::Nozzle { equip: te, nozzle: tnz.clone() }, nps };
        if let Err(e) = apply_op(p, op, cat, &mut made) { notes.push(format!("{}: {e}", r.number)); continue; }
        let Some(&lid) = made.first() else { continue };
        routed += 1;
        created.push(lid);
        if let Some(l) = p.lines.iter_mut().find(|l| l.id == lid) {
            l.number = r.number.clone();
            if !r.class.is_empty() { l.class = r.class.clone(); }
            if r.rating != 0 { l.rating = r.rating; }
            if !r.fluid.is_empty() { l.fluid = r.fluid.clone(); }
            l.insulation = r.insulation;
        }
        // Components in the list's order, spread along the route's straight lengths.
        let l = p.lines.iter().find(|l| l.id == lid).cloned().unwrap_or_default();
        let pts = crate::route::path(p, cat, &l);
        let total: f64 = pts.windows(2).map(|w| dist(w[0], w[1])).sum();
        let n = r.components.len();
        let point_at = |s: f64| -> P3 {
            let mut left = s;
            for w in pts.windows(2) { let d = dist(w[0], w[1]); if left <= d { return add(w[0], mul(unit(sub(w[1], w[0])), left)); } left -= d; }
            *pts.last().unwrap_or(&[0.0; 3])
        };
        for (k, c) in r.components.iter().enumerate() {
            let Some(kind) = item_kind(c) else { continue };
            let at = point_at(total * (k as f64 + 1.0) / (n as f64 + 1.0));
            let op = Op::Drop { line: lid, kind, at, dir: [0.0, 0.0, 0.0], nps2: String::new() };
            if let Err(e) = apply_op(p, op, cat, &mut vec![]) { notes.push(format!("{}: the {} could not be placed ({e}); add it by hand.", r.number, c.replace('_', " "))); }
        }
    }
    if routed == 0 && !notes.is_empty() { return Err(notes.join(" ")); }
    Ok(notes)
}

/// The model's components on a line, in flow order (by run, then along it).
pub fn order(l: &crate::model::Line) -> Vec<String> {
    // Supports are the layout's, not the P&ID's.
    let mut v: Vec<_> = l.items.iter().filter(|i| i.kind != ItemKind::Support).collect();
    v.sort_by(|a, b| (a.seg, a.at).partial_cmp(&(b.seg, b.at)).unwrap_or(std::cmp::Ordering::Equal));
    v.into_iter().map(|i| kind_word(i.kind).to_string()).collect()
}
