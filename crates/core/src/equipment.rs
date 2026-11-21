//! The parametric equipment types (six to 0.2.12, seventeen from 0.2.13): their shapes from datasheet sizes, and their nozzles.
//! Each type's sizes are a few named values (`fields`); nozzles are listed by the designer (new
//! equipment gets typical ones) and drawn as a neck with a weld-neck flange at their face.

use crate::catalogue::Catalogue;
use crate::model::{Dims, EquipKind, Equipment, Nozzle, P3};
use crate::parts::{add, mul, rot_z, sub, Port, Shape};
use serde::Serialize;
use std::f64::consts::TAU;

/// What each size means for a kind: (field key, label). The window shows only these.
pub fn fields(k: EquipKind) -> Vec<(&'static str, &'static str)> {
    match k {
        EquipKind::VerticalVessel => vec![("d", "Diameter"), ("h", "Height, tangent to tangent"), ("elev", "Bottom tangent above base (skirt)")],
        EquipKind::Column => vec![("d", "Diameter"), ("h", "Height, tangent to tangent"), ("elev", "Bottom tangent above base (skirt)")],
        EquipKind::HorizontalDrum => vec![("d", "Diameter"), ("l", "Length, tangent to tangent"), ("elev", "Centreline elevation above base")],
        EquipKind::Tank => vec![("d", "Diameter"), ("h", "Shell height")],
        EquipKind::Pump => vec![("d", "Casing diameter"), ("h", "Shaft centreline above base"), ("l", "Baseplate length"), ("w", "Baseplate width"), ("d2", "Motor diameter"), ("l2", "Motor length")],
        EquipKind::Exchanger => vec![("d", "Shell diameter"), ("l", "Shell length"), ("l2", "Channel length"), ("elev", "Centreline elevation above base")],
        EquipKind::Sphere => vec![("d", "Diameter"), ("elev", "Centre above base")],
        EquipKind::Filter => vec![("d", "Diameter"), ("h", "Height, tangent to tangent"), ("elev", "Bottom tangent above base (legs)")],
        EquipKind::Agitated => vec![("d", "Diameter"), ("h", "Height, tangent to tangent"), ("elev", "Bottom tangent above base"), ("d2", "Drive diameter"), ("l2", "Drive height")],
        EquipKind::Heater => vec![("l", "Radiant box length"), ("w", "Radiant box width"), ("h", "Radiant box height"), ("d", "Stack diameter"), ("l2", "Stack height")],
        EquipKind::InlinePump => vec![("d", "Casing diameter"), ("h", "Pipe centreline above base"), ("d2", "Motor diameter"), ("l2", "Motor length")],
        EquipKind::Compressor => vec![("d", "Casing diameter"), ("h", "Shaft centreline above base"), ("l", "Baseplate length"), ("w", "Baseplate width"), ("d2", "Driver diameter"), ("l2", "Driver length")],
        EquipKind::RecipCompressor => vec![("l", "Frame length"), ("w", "Frame width"), ("h", "Frame height"), ("d", "Cylinder diameter"), ("d2", "Motor diameter"), ("l2", "Motor length")],
        EquipKind::Kettle => vec![("d", "Bundle (channel) diameter"), ("d2", "Kettle diameter"), ("l", "Kettle length"), ("l2", "Channel length"), ("elev", "Channel centreline above base")],
        EquipKind::PlateHx => vec![("l", "Frame length"), ("w", "Frame width"), ("h", "Frame height")],
        EquipKind::AirCooler => vec![("l", "Tube length"), ("w", "Bay width"), ("h", "Bundle above base"), ("d", "Fan diameter")],
        EquipKind::Package => vec![("l", "Skid length"), ("w", "Skid width"), ("h", "Height")],
    }
}

/// A new item of a kind: typical sizes, nozzles and datasheet lines.
pub fn new(kind: EquipKind, tag: &str) -> Equipment {
    let n = |tag: &str, nps: &str, pos: P3, dir: P3, service: &str| Nozzle { tag: tag.into(), nps: nps.into(), rating: 150, facing: "RF".into(), pos, dir, proj: 0.2, service: service.into() };
    let (dims, nozzles, data): (Dims, Vec<Nozzle>, Vec<(&str, &str)>) = match kind {
        EquipKind::VerticalVessel => {
            let d = Dims { d: 1.6, h: 4.0, elev: 1.5, ..Dims::default() };
            let top = d.elev + d.h + d.d / 4.0;
            (d.clone(), vec![n("N1", "6", [0.0, 0.0, top + 0.2], [0.0, 0.0, 1.0], "Inlet"), n("N2", "6", [d.d / 2.0 + 0.2, 0.0, d.elev + 0.4], [1.0, 0.0, 0.0], "Outlet"), n("N3", "2", [-(d.d / 2.0 + 0.2), 0.0, d.elev + d.h - 0.5], [-1.0, 0.0, 0.0], "Vent")],
             vec![("Design pressure", "10 bar"), ("Design temperature", "120 °C"), ("Material", "Carbon steel"), ("Volume", "9 m³")])
        }
        EquipKind::Column => {
            let d = Dims { d: 1.2, h: 16.0, elev: 2.5, ..Dims::default() };
            let top = d.elev + d.h + d.d / 4.0;
            (d.clone(), vec![n("N1", "8", [0.0, 0.0, top + 0.2], [0.0, 0.0, 1.0], "Overhead vapour"), n("N2", "4", [d.d / 2.0 + 0.2, 0.0, d.elev + d.h / 2.0], [1.0, 0.0, 0.0], "Feed"), n("N3", "3", [d.d / 2.0 + 0.2, 0.0, d.elev + d.h - 1.0], [1.0, 0.0, 0.0], "Reflux"), n("N4", "6", [-(d.d / 2.0 + 0.2), 0.0, d.elev + 0.5], [-1.0, 0.0, 0.0], "Bottoms")],
             vec![("Design pressure", "6 bar"), ("Design temperature", "180 °C"), ("Trays", "24"), ("Material", "Carbon steel")])
        }
        EquipKind::HorizontalDrum => {
            let d = Dims { d: 1.8, l: 6.0, elev: 2.0, ..Dims::default() };
            (d.clone(), vec![n("N1", "6", [-d.l / 2.0 + 0.6, 0.0, d.elev + d.d / 2.0 + 0.2], [0.0, 0.0, 1.0], "Inlet"), n("N2", "6", [d.l / 2.0 - 0.6, 0.0, d.elev - d.d / 2.0 - 0.2], [0.0, 0.0, -1.0], "Outlet"), n("N3", "3", [d.l / 2.0 - 0.6, 0.0, d.elev + d.d / 2.0 + 0.2], [0.0, 0.0, 1.0], "Vapour out")],
             vec![("Design pressure", "8 bar"), ("Design temperature", "90 °C"), ("Material", "Carbon steel"), ("Volume", "17 m³")])
        }
        EquipKind::Tank => {
            let d = Dims { d: 6.0, h: 6.0, ..Dims::default() };
            (d.clone(), vec![n("N1", "8", [d.d / 2.0 + 0.25, 0.0, 0.6], [1.0, 0.0, 0.0], "Outlet"), n("N2", "6", [-(d.d / 2.0 + 0.25), 0.0, d.h - 0.6], [-1.0, 0.0, 0.0], "Inlet"), n("N3", "4", [0.0, 1.0, d.h + d.d * 0.08 + 0.3], [0.0, 0.0, 1.0], "Vent")],
             vec![("Design pressure", "Atmospheric"), ("Design temperature", "60 °C"), ("Capacity", "170 m³"), ("Material", "Carbon steel")])
        }
        EquipKind::Pump => {
            let d = Dims { d: 0.5, h: 0.6, l: 1.8, w: 0.7, d2: 0.45, l2: 0.8, elev: 0.0 };
            let x0 = -d.l / 2.0 + d.d * 0.6;
            (d.clone(), vec![n("S", "6", [x0 - d.d * 0.25 - 0.2, 0.0, d.h], [-1.0, 0.0, 0.0], "Suction"), n("D", "4", [x0, 0.0, d.h + d.d / 2.0 + 0.2], [0.0, 0.0, 1.0], "Discharge")],
             vec![("Flow", "50 m³/h"), ("Differential head", "45 m"), ("Motor", "11 kW"), ("Speed", "2950 rpm")])
        }
        EquipKind::Exchanger => {
            let d = Dims { d: 0.6, l: 4.5, l2: 0.8, elev: 1.4, ..Dims::default() };
            let top = d.elev + d.d / 2.0 + 0.2;
            let bot = d.elev - d.d / 2.0 - 0.2;
            let xc = d.l / 2.0 + d.l2 / 2.0;
            (d.clone(), vec![n("N1", "6", [-d.l / 2.0 + 0.5, 0.0, top], [0.0, 0.0, 1.0], "Shell in"), n("N2", "6", [d.l / 2.0 - 0.5, 0.0, bot], [0.0, 0.0, -1.0], "Shell out"), n("N3", "4", [xc, 0.0, top], [0.0, 0.0, 1.0], "Tube in"), n("N4", "4", [xc, 0.0, bot], [0.0, 0.0, -1.0], "Tube out")],
             vec![("Duty", "850 kW"), ("Shell side design", "10 bar, 150 °C"), ("Tube side design", "6 bar, 60 °C"), ("Area", "48 m²")])
        }
        EquipKind::Sphere => {
            let d = Dims { d: 10.0, elev: 7.0, ..Dims::default() };
            let r = d.d / 2.0;
            (d.clone(), vec![n("N1", "6", [0.0, 0.0, d.elev + r + 0.2], [0.0, 0.0, 1.0], "Inlet"), n("N2", "8", [0.0, 0.0, d.elev - r - 0.2], [0.0, 0.0, -1.0], "Outlet"), n("N3", "4", [r * 0.3, 0.0, d.elev + r + 0.25], [0.0, 0.0, 1.0], "Relief")],
             vec![("Design pressure", "17 bar"), ("Design temperature", "50 °C"), ("Capacity", "520 m³"), ("Material", "Carbon steel")])
        }
        EquipKind::Filter => {
            let d = Dims { d: 0.6, h: 1.2, elev: 0.8, ..Dims::default() };
            let r = d.d / 2.0;
            (d.clone(), vec![n("N1", "4", [r + 0.2, 0.0, d.elev + d.h - 0.3], [1.0, 0.0, 0.0], "Inlet"), n("N2", "4", [-(r + 0.2), 0.0, d.elev + 0.3], [-1.0, 0.0, 0.0], "Outlet"), n("N3", "1", [0.0, 0.0, d.elev - d.d / 4.0 - 0.2], [0.0, 0.0, -1.0], "Drain")],
             vec![("Design pressure", "16 bar"), ("Design temperature", "80 °C"), ("Element", "Cartridge, 10 µm"), ("Material", "Carbon steel")])
        }
        EquipKind::Agitated => {
            let d = Dims { d: 2.0, h: 2.5, elev: 1.0, d2: 0.4, l2: 0.8, ..Dims::default() };
            let r = d.d / 2.0;
            let top = d.elev + d.h + d.d / 4.0;
            (d.clone(), vec![n("N1", "4", [r * 0.6, 0.0, top + 0.2], [0.0, 0.0, 1.0], "Feed"), n("N2", "3", [-r * 0.6, 0.0, top + 0.2], [0.0, 0.0, 1.0], "Vent"), n("N3", "4", [r + 0.2, 0.0, d.elev + 0.3], [1.0, 0.0, 0.0], "Outlet")],
             vec![("Design pressure", "4 bar"), ("Design temperature", "150 °C"), ("Volume", "9 m³"), ("Agitator", "15 kW, 90 rpm")])
        }
        EquipKind::Heater => {
            let d = Dims { l: 8.0, w: 5.0, h: 8.0, d: 1.2, l2: 10.0, ..Dims::default() };
            (d.clone(), vec![n("N1", "6", [0.0, d.w * 0.3 + 0.2, d.h + 1.0], [0.0, 1.0, 0.0], "Process in"), n("N2", "6", [d.l / 2.0 + 0.2, 0.0, 1.0], [1.0, 0.0, 0.0], "Process out")],
             vec![("Duty", "12 MW"), ("Fuel", "Fuel gas"), ("Coil design", "40 bar, 400 °C")])
        }
        EquipKind::InlinePump => {
            let d = Dims { d: 0.35, h: 0.6, d2: 0.35, l2: 0.6, ..Dims::default() };
            (d.clone(), vec![n("S", "4", [-(d.d + 0.2), 0.0, d.h], [-1.0, 0.0, 0.0], "Suction"), n("D", "4", [d.d + 0.2, 0.0, d.h], [1.0, 0.0, 0.0], "Discharge")],
             vec![("Flow", "30 m³/h"), ("Differential head", "35 m"), ("Motor", "5.5 kW")])
        }
        EquipKind::Compressor => {
            let d = Dims { d: 1.4, h: 1.4, l: 6.0, w: 2.2, d2: 1.1, l2: 1.8, elev: 0.0 };
            let xc = -d.l / 2.0 + d.d * 0.9;
            (d.clone(), vec![n("S", "12", [xc - d.d * 0.25, 0.0, d.h + d.d / 2.0 + 0.3], [0.0, 0.0, 1.0], "Suction"), n("D", "10", [xc + d.d * 0.25, 0.0, d.h + d.d / 2.0 + 0.3], [0.0, 0.0, 1.0], "Discharge")],
             vec![("Flow", "12 000 Nm³/h"), ("Pressure ratio", "3.2"), ("Driver", "1.2 MW motor")])
        }
        EquipKind::RecipCompressor => {
            let d = Dims { l: 2.5, w: 1.6, h: 1.4, d: 0.5, d2: 0.9, l2: 1.4, elev: 0.0 };
            let (yc, zc) = (d.w / 2.0 + 1.0, d.h * 0.75);
            (d.clone(), vec![n("S", "6", [0.0, yc - 0.4, zc + d.d / 2.0 + 0.25], [0.0, 0.0, 1.0], "Suction"), n("D", "4", [0.0, -(yc + 0.2), zc], [0.0, -1.0, 0.0], "Discharge")],
             vec![("Flow", "1 500 Nm³/h"), ("Stages", "1, double acting"), ("Driver", "250 kW motor")])
        }
        EquipKind::Kettle => {
            let d = Dims { d: 0.6, d2: 1.4, l: 5.0, l2: 0.8, elev: 1.6, ..Dims::default() };
            let ck = d.elev + (d.d2 - d.d) / 2.0;
            let xc = d.l / 2.0 + d.l2 / 2.0;
            (d.clone(), vec![n("N1", "6", [xc, 0.0, d.elev + d.d / 2.0 + 0.2], [0.0, 0.0, 1.0], "Heating medium in"), n("N2", "4", [xc, 0.0, d.elev - d.d / 2.0 - 0.2], [0.0, 0.0, -1.0], "Condensate out"),
                n("N3", "10", [0.0, 0.0, ck + d.d2 / 2.0 + 0.2], [0.0, 0.0, 1.0], "Vapour out"), n("N4", "6", [-d.l / 2.0 + 0.6, 0.0, d.elev - d.d / 2.0 - 0.2], [0.0, 0.0, -1.0], "Liquid in"), n("N5", "4", [-d.l / 2.0 - d.d2 / 4.0 - 0.2, 0.0, ck], [-1.0, 0.0, 0.0], "Bottoms out")],
             vec![("Duty", "2.4 MW"), ("Shell side design", "10 bar, 200 °C"), ("Tube side design", "12 bar, 220 °C")])
        }
        EquipKind::PlateHx => {
            let d = Dims { l: 2.2, w: 0.7, h: 1.6, ..Dims::default() };
            let x = -d.l / 2.0 - 0.2;
            (d.clone(), vec![n("N1", "4", [x, d.w * 0.28, d.h * 0.8], [-1.0, 0.0, 0.0], "Hot in"), n("N2", "4", [x, d.w * 0.28, d.h * 0.3], [-1.0, 0.0, 0.0], "Hot out"), n("N3", "4", [x, -d.w * 0.28, d.h * 0.3], [-1.0, 0.0, 0.0], "Cold in"), n("N4", "4", [x, -d.w * 0.28, d.h * 0.8], [-1.0, 0.0, 0.0], "Cold out")],
             vec![("Duty", "600 kW"), ("Plates", "80"), ("Design", "16 bar, 120 °C")])
        }
        EquipKind::AirCooler => {
            let d = Dims { l: 9.0, w: 3.0, h: 4.0, d: 3.0, ..Dims::default() };
            let xh = d.l / 2.0 + 0.15;
            (d.clone(), vec![n("N1", "8", [-xh, 0.0, d.h + 0.45 + 0.2], [0.0, 0.0, 1.0], "Inlet"), n("N2", "6", [xh, 0.0, d.h - 0.15 - 0.2], [0.0, 0.0, -1.0], "Outlet")],
             vec![("Duty", "3 MW"), ("Fans", "2, 22 kW each"), ("Design", "20 bar, 180 °C")])
        }
        EquipKind::Package => {
            let d = Dims { l: 4.0, w: 2.5, h: 2.5, ..Dims::default() };
            (d.clone(), vec![n("N1", "3", [-(d.l / 2.0 + 0.2), 0.0, d.h * 0.5], [-1.0, 0.0, 0.0], "Inlet"), n("N2", "3", [d.l / 2.0 + 0.2, 0.0, d.h * 0.5], [1.0, 0.0, 0.0], "Outlet")],
             vec![("Package", "Chemical injection"), ("Supplier", "")])
        }
    };
    Equipment { kind, tag: tag.into(), dims, nozzles, data: data.into_iter().map(|(a, b)| (a.into(), b.into())).collect(), ..Default::default() }
}

/// Axes turned `a` radians about Z: along the radius, across it, up.
fn turned(a: f64) -> [P3; 3] { [[a.cos(), a.sin(), 0.0], [-a.sin(), a.cos(), 0.0], [0.0, 0.0, 1.0]] }

/// A skirt to the bottom tangent line, standing on its base ring with anchor-bolt chairs.
fn skirt(v: &mut Vec<Shape>, r: f64, top: f64) {
    v.push(Shape::tube([0.0; 3], [0.0, 0.0, 0.03], r + 0.12));
    v.push(Shape::tube([0.0, 0.0, 0.03], [0.0, 0.0, top], r));
    let n = if r > 1.0 { 12 } else { 8 };
    for k in 0..n {
        let ax = turned((k as f64 + 0.5) * TAU / n as f64);
        v.push(Shape::Cuboid { centre: add(mul(ax[0], r + 0.05), [0.0, 0.0, 0.15]), axes: ax, half: [0.05, 0.06, 0.12] });
        v.push(Shape::Cuboid { centre: add(mul(ax[0], r + 0.05), [0.0, 0.0, 0.28]), axes: ax, half: [0.06, 0.08, 0.012] });
    }
}

/// A manway: a short neck from `at` along `dir` and its bolted cover.
fn manway(v: &mut Vec<Shape>, at: P3, dir: P3, rm: f64) {
    let e = add(at, mul(dir, 0.25));
    v.push(Shape::tube(at, e, rm));
    v.push(Shape::tube(e, add(e, mul(dir, 0.06)), rm * 1.3));
    v.push(Shape::tube(add(e, mul(dir, 0.06)), add(e, mul(dir, 0.1)), rm * 1.3));
}

/// A saddle at `xs` under a horizontal shell of radius `r` on centreline `cz`: base plate, web
/// to the shell, and ribs.
fn saddle(v: &mut Vec<Shape>, xs: f64, cz: f64, r: f64) {
    let ax = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let top = cz - r * 0.6;
    v.push(Shape::Cuboid { centre: [xs, 0.0, 0.015], axes: ax, half: [0.14, r * 0.9, 0.015] });
    v.push(Shape::Cuboid { centre: [xs, 0.0, (0.03 + top) / 2.0], axes: ax, half: [0.008, r * 0.8, ((top - 0.03) / 2.0).max(0.01)] });
    for y in [-0.75, -0.3, 0.3, 0.75] {
        let t = cz - r * (1.0f64 - y * y).sqrt();
        v.push(Shape::Cuboid { centre: [xs, y * r, (0.03 + t) / 2.0], axes: ax, half: [0.13, 0.008, ((t - 0.03) / 2.0).max(0.01)] });
    }
}

/// An electric motor along +X from `x0`, shaft at `z`: end bell, finned frame, terminal box, fan
/// cowl, and feet down to `base`.
fn motor(v: &mut Vec<Shape>, x0: f64, len: f64, z: f64, r: f64, base: f64) {
    let ax = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let (b0, b1) = (x0 + len * 0.06, x0 + len * 0.8);
    v.push(Shape::tube([x0, 0.0, z], [b0, 0.0, z], r * 0.9));
    v.push(Shape::tube([b0, 0.0, z], [b1, 0.0, z], r));
    for k in 0..12 {
        let a = (k as f64 + 0.5) * TAU / 12.0;
        let (c, sn) = (a.cos(), a.sin());
        v.push(Shape::Cuboid { centre: [(b0 + b1) / 2.0, r * c, z + r * sn], axes: [[1.0, 0.0, 0.0], [0.0, c, sn], [0.0, -sn, c]], half: [(b1 - b0) / 2.0, r * 0.08, r * 0.025] });
    }
    v.push(Shape::Cuboid { centre: [x0 + len * 0.35, 0.0, z + r * 1.12], axes: ax, half: [r * 0.35, r * 0.3, r * 0.2] });
    v.push(Shape::tube([b1, 0.0, z], [x0 + len * 0.97, 0.0, z], r * 0.96));
    v.push(Shape::Head { base: [x0 + len * 0.97, 0.0, z], dir: [1.0, 0.0, 0.0], r: r * 0.96, depth: len * 0.03 });
    let fz = z - r * 0.75;
    if fz > base + 0.01 {
        v.push(Shape::Cuboid { centre: [(b0 + b1) / 2.0, 0.0, (base + fz) / 2.0], axes: ax, half: [(b1 - b0) * 0.4, r * 0.8, (fz - base) / 2.0] });
    }
}

/// A coupling guard over the shaft from `xa` to `xb` at `z`: a box with a rounded top.
fn guard(v: &mut Vec<Shape>, xa: f64, xb: f64, z: f64, rg: f64) {
    let ax = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    v.push(Shape::tube([xa, 0.0, z], [xb, 0.0, z], rg));
    v.push(Shape::Cuboid { centre: [(xa + xb) / 2.0, 0.0, z - rg / 2.0], axes: ax, half: [(xb - xa) / 2.0, rg, rg / 2.0] });
}

/// The body of an item (in its own axes): shapes, without nozzles. Drawn as built (0.3.3): skirts
/// with base rings, saddles with ribs, manways, a pump's casing, bearing frame, coupling guard
/// and finned motor; the proportions are for the picture only.
fn body(e: &Equipment) -> Vec<Shape> {
    let d = &e.dims;
    let r = d.d / 2.0;
    let x: P3 = [1.0, 0.0, 0.0];
    let z: P3 = [0.0, 0.0, 1.0];
    let ax = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let bx = |c: P3, h: P3| Shape::Cuboid { centre: c, axes: ax, half: h };
    let tx = |x0: f64, x1: f64, zz: f64, rr: f64| Shape::tube([x0, 0.0, zz], [x1, 0.0, zz], rr);
    let tz = |z0: f64, z1: f64, rr: f64| Shape::tube([0.0, 0.0, z0], [0.0, 0.0, z1], rr);
    let rm = (r * 0.4).clamp(0.15, 0.3);
    let mut v = vec![];
    match e.kind {
        EquipKind::VerticalVessel | EquipKind::Column => {
            skirt(&mut v, r, d.elev);
            v.push(tz(d.elev, d.elev + d.h, r));
            v.push(Shape::Head { base: [0.0, 0.0, d.elev + d.h], dir: z, r, depth: d.d / 4.0 });
            v.push(Shape::Head { base: [0.0, 0.0, d.elev], dir: [0.0, 0.0, -1.0], r: r * 0.999, depth: (d.d / 4.0).min(d.elev * 0.9) });
            let at: Vec<f64> = if e.kind == EquipKind::Column { vec![d.elev + 1.0, d.elev + d.h / 2.0 - 1.0, d.elev + d.h - 1.5] } else { vec![d.elev + d.h * 0.5] };
            for zm in at { manway(&mut v, [0.0, -r * 0.9, zm], [0.0, -1.0, 0.0], rm); }
            // A lifting lug each side of the top head.
            for sx in [-1.0, 1.0] { v.push(bx([0.0, sx * r * 0.55, d.elev + d.h + d.d * 0.2], [0.03, 0.06, 0.1])); }
        }
        EquipKind::HorizontalDrum => {
            v.push(tx(-d.l / 2.0, d.l / 2.0, d.elev, r));
            v.push(Shape::Head { base: [d.l / 2.0, 0.0, d.elev], dir: x, r, depth: d.d / 4.0 });
            v.push(Shape::Head { base: [-d.l / 2.0, 0.0, d.elev], dir: [-1.0, 0.0, 0.0], r, depth: d.d / 4.0 });
            saddle(&mut v, -d.l * 0.3, d.elev, r);
            saddle(&mut v, d.l * 0.3, d.elev, r);
            manway(&mut v, [0.0, 0.0, d.elev + r * 0.9], z, rm);
        }
        EquipKind::Tank => {
            v.push(tz(0.0, 0.02, r + 0.06));
            v.push(tz(0.02, d.h, r));
            // Shell courses (about 2 m), the top curb angle, and the cone roof.
            let n = (d.h / 2.0).floor() as i32;
            for k in 1..n.max(1) { let zc = d.h * k as f64 / n as f64; if zc < d.h - 0.3 { v.push(tz(zc - 0.006, zc + 0.006, r + 0.006)); } }
            v.push(tz(d.h - 0.08, d.h, r + 0.05));
            v.push(Shape::Frustum { a: [0.0, 0.0, d.h], b: [0.0, 0.0, d.h + d.d * 0.08], r1: r + 0.05, r2: r * 0.05 });
            manway(&mut v, [0.0, -(r - 0.05), 0.75], [0.0, -1.0, 0.0], 0.3);
            manway(&mut v, [-r * 0.5, -r * 0.3, d.h], z, 0.3);
        }
        EquipKind::Pump => {
            // End-suction pump: casing, cover, bearing frame, coupling guard, motor, on a baseplate.
            let bt = 0.15;
            v.push(bx([0.0, 0.0, 0.06], [d.l / 2.0, d.w / 2.0, 0.06]));
            v.push(bx([0.0, 0.0, 0.135], [d.l / 2.0 - 0.03, d.w / 2.0 - 0.03, 0.015]));
            let x0 = -d.l / 2.0 + d.d * 0.6;
            let xm = d.l / 2.0 - d.l2 - 0.05;
            let (c0, c1) = (x0 - d.d * 0.25, x0 + d.d * 0.2);
            v.push(tx(c0, c1, d.h, r));
            v.push(tx(c1, c1 + 0.03, d.h, r * 1.06));
            v.push(Shape::Frustum { a: [c1 + 0.03, 0.0, d.h], b: [c1 + 0.08, 0.0, d.h], r1: r * 0.8, r2: r * 0.5 });
            v.push(bx([x0, 0.0, (d.h - r * 0.6 + bt) / 2.0], [d.d * 0.16, d.d * 0.32, ((d.h - r * 0.6 - bt) / 2.0).max(0.01)]));
            let xa = c1 + 0.08;
            let xb = xa + (xm - xa) * 0.6;
            v.push(Shape::Frustum { a: [xa, 0.0, d.h], b: [xb, 0.0, d.h], r1: r * 0.5, r2: r * 0.4 });
            v.push(bx([xb - 0.06, 0.0, (d.h - r * 0.4 + bt) / 2.0], [0.04, r * 0.55, ((d.h - r * 0.4 - bt) / 2.0).max(0.01)]));
            guard(&mut v, xb, xm, d.h, d.d * 0.2);
            motor(&mut v, xm, d.l2, d.h, d.d2 / 2.0, bt);
        }
        EquipKind::Exchanger => {
            v.push(tx(-d.l / 2.0, d.l / 2.0, d.elev, r));
            v.push(tx(-d.l / 2.0 - 0.06, -d.l / 2.0, d.elev, r * 1.18));
            v.push(Shape::Head { base: [-d.l / 2.0 - 0.06, 0.0, d.elev], dir: [-1.0, 0.0, 0.0], r, depth: d.d / 4.0 });
            v.push(tx(d.l / 2.0, d.l / 2.0 + 0.06, d.elev, r * 1.18));
            v.push(tx(d.l / 2.0 + 0.06, d.l / 2.0 + d.l2, d.elev, r));
            v.push(tx(d.l / 2.0 + d.l2, d.l / 2.0 + d.l2 + 0.06, d.elev, r * 1.18));
            v.push(tx(d.l / 2.0 + d.l2 + 0.06, d.l / 2.0 + d.l2 + 0.1, d.elev, r * 1.18));
            saddle(&mut v, -d.l * 0.3, d.elev, r);
            saddle(&mut v, d.l * 0.3, d.elev, r);
        }
        EquipKind::Sphere => {
            let c = [0.0, 0.0, d.elev];
            v.push(Shape::Head { base: c, dir: z, r, depth: r });
            v.push(Shape::Head { base: c, dir: [0.0, 0.0, -1.0], r, depth: r });
            // Legs to the equator, with base plates and crossed tie rods between them.
            let n = 6;
            let leg = |k: i32| { let a = k as f64 * TAU / n as f64; [r * a.cos(), r * a.sin()] };
            for k in 0..n {
                let [x0, y0] = leg(k);
                v.push(Shape::tube([x0, y0, 0.0], [x0, y0, d.elev], d.d * 0.025));
                v.push(bx([x0, y0, 0.02], [d.d * 0.05, d.d * 0.05, 0.02]));
                let [x1, y1] = leg(k + 1);
                let (z0, z1) = (0.4, d.elev * 0.62);
                v.push(Shape::tube([x0, y0, z0], [x1, y1, z1], 0.02));
                v.push(Shape::tube([x0, y0, z1], [x1, y1, z0], 0.02));
            }
            v.push(Shape::tube([0.0, 0.0, d.elev - 0.06], [0.0, 0.0, d.elev + 0.06], r + 0.02));
        }
        EquipKind::Filter => {
            v.push(tz(d.elev, d.elev + d.h, r));
            v.push(tz(d.elev + d.h, d.elev + d.h + 0.04, r * 1.25));
            v.push(tz(d.elev + d.h + 0.04, d.elev + d.h + 0.08, r * 1.25));
            v.push(Shape::Head { base: [0.0, 0.0, d.elev + d.h + 0.08], dir: z, r, depth: d.d / 4.0 });
            v.push(Shape::Head { base: [0.0, 0.0, d.elev], dir: [0.0, 0.0, -1.0], r, depth: d.d / 4.0 });
            // The closure's davit: a post and arm to lift the cover.
            v.push(Shape::tube([-r * 1.35, -r * 0.2, d.elev + d.h - 0.4], [-r * 1.35, -r * 0.2, d.elev + d.h + 0.4], 0.025));
            v.push(Shape::tube([-r * 1.35, -r * 0.2, d.elev + d.h + 0.4], [0.0, -r * 0.2, d.elev + d.h + 0.4], 0.02));
            for k in 0..3 {
                let a = (k as f64 + 0.25) * TAU / 3.0;
                let (x0, y0) = (r * 0.85 * a.cos(), r * 0.85 * a.sin());
                v.push(Shape::tube([x0, y0, 0.02], [x0, y0, d.elev + 0.1], 0.03));
                v.push(bx([x0, y0, 0.01], [0.07, 0.07, 0.01]));
            }
        }
        EquipKind::Agitated => {
            let top = d.elev + d.h + d.d / 4.0;
            skirt(&mut v, r, d.elev);
            v.push(tz(d.elev, d.elev + d.h, r));
            v.push(Shape::Head { base: [0.0, 0.0, d.elev + d.h], dir: z, r, depth: d.d / 4.0 });
            v.push(Shape::Head { base: [0.0, 0.0, d.elev], dir: [0.0, 0.0, -1.0], r: r * 0.999, depth: (d.d / 4.0).min(d.elev * 0.9) });
            // Mounting flange, gearbox, motor stool and motor.
            v.push(tz(top - 0.1, top + 0.04, d.d2 * 0.9));
            v.push(bx([0.0, 0.0, top + 0.17], [d.d2 * 0.7, d.d2 * 0.6, 0.13]));
            v.push(Shape::Frustum { a: [0.0, 0.0, top + 0.3], b: [0.0, 0.0, top + 0.38], r1: d.d2 * 0.45, r2: d.d2 * 0.55 });
            v.push(tz(top + 0.38, top + 0.38 + d.l2 * 0.85, d.d2 / 2.0));
            v.push(tz(top + 0.38 + d.l2 * 0.85, top + 0.38 + d.l2, d.d2 / 2.0 * 0.95));
            v.push(bx([d.d2 * 0.55, 0.0, top + 0.38 + d.l2 * 0.3], [0.06, 0.07, 0.06]));
            manway(&mut v, [0.0, -r * 0.5, d.elev + d.h + d.d / 4.0 * 0.8], z, rm.min(0.25));
        }
        EquipKind::Heater => {
            // Radiant box with corner columns, convection section, breeching, stack and cap ring.
            v.push(bx([0.0, 0.0, d.h / 2.0], [d.l / 2.0, d.w / 2.0, d.h / 2.0]));
            for (sx, sy) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] { v.push(bx([sx * (d.l / 2.0 + 0.1), sy * (d.w / 2.0 + 0.1), d.h / 2.0], [0.1, 0.1, d.h / 2.0])); }
            for k in 1..4 { let xk = -d.l / 2.0 + d.l * k as f64 / 4.0; for sy in [-1.0, 1.0] { v.push(bx([xk, sy * (d.w / 2.0 + 0.06), d.h / 2.0], [0.08, 0.06, d.h / 2.0])); } }
            v.push(bx([0.0, 0.0, d.h + 1.0], [d.l * 0.35, d.w * 0.3, 1.0]));
            v.push(Shape::Frustum { a: [0.0, 0.0, d.h + 2.0], b: [0.0, 0.0, d.h + 2.6], r1: d.w * 0.3, r2: d.d / 2.0 });
            v.push(tz(d.h + 2.6, d.h + 2.6 + d.l2, d.d / 2.0));
            v.push(tz(d.h + 2.6 + d.l2 * 0.7, d.h + 2.6 + d.l2 * 0.7 + 0.08, d.d / 2.0 + 0.08));
            v.push(tz(d.h + 2.6 + d.l2 - 0.1, d.h + 2.6 + d.l2, d.d / 2.0 + 0.05));
        }
        EquipKind::InlinePump => {
            v.push(bx([0.0, 0.0, (d.h - r) / 2.0], [d.d * 0.6, d.d * 0.5, ((d.h - r) / 2.0).max(0.02)]));
            v.push(tx(-d.d, d.d, d.h, r * 0.55));
            v.push(tz(d.h - r, d.h + r, r));
            v.push(tz(d.h + r, d.h + r + 0.03, r * 1.08));
            // Motor stool (open lantern) and a vertical motor with its cowl and terminal box.
            v.push(Shape::Frustum { a: [0.0, 0.0, d.h + r + 0.03], b: [0.0, 0.0, d.h + r + 0.25], r1: r * 0.6, r2: d.d2 * 0.45 });
            let m0 = d.h + r + 0.25;
            v.push(tz(m0, m0 + d.l2 * 0.8, d.d2 / 2.0));
            v.push(tz(m0 + d.l2 * 0.8, m0 + d.l2, d.d2 / 2.0 * 0.95));
            v.push(Shape::Head { base: [0.0, 0.0, m0 + d.l2], dir: z, r: d.d2 / 2.0 * 0.95, depth: 0.03 });
            v.push(bx([d.d2 * 0.58, 0.0, m0 + d.l2 * 0.35], [0.06, 0.07, 0.06]));
        }
        EquipKind::Compressor => {
            // Horizontally split casing with bearing housings, coupling guard and driver.
            let bt = 0.3;
            v.push(bx([0.0, 0.0, 0.15], [d.l / 2.0, d.w / 2.0, 0.15]));
            let xc = -d.l / 2.0 + d.d * 0.9;
            let xm = d.l / 2.0 - d.l2 - 0.1;
            v.push(tx(xc - d.d * 0.6, xc + d.d * 0.6, d.h, r));
            v.push(bx([xc, 0.0, d.h], [d.d * 0.62, r * 1.1, 0.03]));
            v.push(bx([xc, 0.0, (d.h + bt) / 2.0], [d.d * 0.5, d.d * 0.4, ((d.h - bt) / 2.0).max(0.02)]));
            v.push(tx(xc - d.d * 0.6 - 0.25, xc - d.d * 0.6, d.h, r * 0.35));
            v.push(tx(xc + d.d * 0.6, xc + d.d * 0.6 + 0.25, d.h, r * 0.35));
            guard(&mut v, xc + d.d * 0.6 + 0.25, xm, d.h, d.d * 0.15);
            motor(&mut v, xm, d.l2, d.h, d.d2 / 2.0, bt);
        }
        EquipKind::RecipCompressor => {
            let zc = d.h * 0.75;
            v.push(bx([0.0, 0.0, d.h / 2.0], [d.l * 0.25, d.w / 2.0, d.h / 2.0]));
            v.push(bx([0.0, 0.0, d.h + 0.03], [d.l * 0.2, d.w * 0.4, 0.03]));
            for sy in [-1.0, 1.0] {
                // Distance piece, the cylinder with its flanged heads, and a valve cap on top.
                let y = |t: f64| [0.0, sy * (d.w / 2.0 + t), zc];
                v.push(Shape::tube(y(0.0), y(0.4), d.d * 0.3));
                v.push(Shape::tube(y(0.4), y(0.45), d.d / 2.0 * 1.12));
                v.push(Shape::tube(y(0.45), y(0.95), d.d / 2.0));
                v.push(Shape::tube(y(0.95), y(1.0), d.d / 2.0 * 1.12));
            }
            // The flywheel, then the motor.
            let zm = d.h * 0.45;
            v.push(tx(d.l * 0.25, d.l * 0.25 + 0.1, zm, (d.d2 * 0.7).min(zm - 0.05)));
            motor(&mut v, d.l * 0.25 + 0.12, d.l2, zm, d.d2 / 2.0, 0.0);
        }
        EquipKind::Kettle => {
            let ck = d.elev + (d.d2 - d.d) / 2.0;
            let rk = d.d2 / 2.0;
            v.push(tx(-d.l / 2.0, d.l / 2.0 - 0.9, ck, rk));
            v.push(Shape::Head { base: [-d.l / 2.0, 0.0, ck], dir: [-1.0, 0.0, 0.0], r: rk, depth: d.d2 / 4.0 });
            v.push(Shape::Frustum { a: [d.l / 2.0 - 0.9, 0.0, ck], b: [d.l / 2.0 - 0.4, 0.0, d.elev], r1: rk, r2: r });
            v.push(tx(d.l / 2.0 - 0.4, d.l / 2.0, d.elev, r));
            v.push(tx(d.l / 2.0, d.l / 2.0 + 0.06, d.elev, r * 1.18));
            v.push(tx(d.l / 2.0 + 0.06, d.l / 2.0 + d.l2, d.elev, r));
            v.push(tx(d.l / 2.0 + d.l2, d.l / 2.0 + d.l2 + 0.06, d.elev, r * 1.18));
            v.push(tx(d.l / 2.0 + d.l2 + 0.06, d.l / 2.0 + d.l2 + 0.1, d.elev, r * 1.18));
            saddle(&mut v, -d.l * 0.3, ck, rk);
            saddle(&mut v, d.l * 0.25, d.elev, r);
        }
        EquipKind::PlateHx => {
            // Fixed frame plate, the plate pack, pressure plate, guide bars, tie bolts and end post.
            v.push(bx([-d.l / 2.0 + 0.075, 0.0, d.h / 2.0], [0.075, d.w / 2.0, d.h / 2.0]));
            v.push(bx([-d.l / 2.0 + 0.15 + d.l * 0.25, 0.0, d.h / 2.0 + 0.05], [d.l * 0.25, d.w * 0.4, d.h * 0.42]));
            v.push(bx([-d.l / 2.0 + 0.15 + d.l * 0.5 + 0.06, 0.0, d.h / 2.0], [0.06, d.w / 2.0, d.h / 2.0]));
            v.push(tx(-d.l / 2.0, d.l / 2.0, d.h - 0.05, 0.04));
            v.push(tx(-d.l / 2.0, d.l / 2.0, 0.1, 0.04));
            v.push(bx([d.l / 2.0 - 0.06, 0.0, d.h / 2.0], [0.06, 0.12, d.h / 2.0]));
            let xe = -d.l / 2.0 + 0.15 + d.l * 0.5 + 0.2;
            for zb in [0.3, 0.7] { for sy in [-1.0, 1.0] { v.push(Shape::tube([-d.l / 2.0 - 0.04, sy * (d.w / 2.0 + 0.03), d.h * zb], [xe, sy * (d.w / 2.0 + 0.03), d.h * zb], 0.018)); } }
        }
        EquipKind::AirCooler => {
            v.push(bx([0.0, 0.0, d.h + 0.15], [d.l / 2.0, d.w / 2.0, 0.15]));
            v.push(bx([-(d.l / 2.0 + 0.15), 0.0, d.h + 0.15], [0.15, d.w / 2.0, 0.3]));
            v.push(bx([d.l / 2.0 + 0.15, 0.0, d.h + 0.15], [0.15, d.w / 2.0, 0.3]));
            // Legs (H sections drawn as boxes) braced along the length.
            for (sx, sy) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] { v.push(bx([sx * d.l * 0.45, sy * d.w * 0.45, d.h / 2.0], [0.08, 0.08, d.h / 2.0])); }
            for sy in [-1.0, 1.0] { v.push(bx([0.0, sy * d.w * 0.45, d.h - 1.1], [d.l * 0.45, 0.05, 0.08])); }
            for sx in [-1.0, 1.0] {
                // Plenum, fan ring, fan hub and the drive motor below.
                let x0 = sx * d.l / 4.0;
                v.push(Shape::Frustum { a: [x0, 0.0, d.h - 0.9], b: [x0, 0.0, d.h], r1: d.d / 2.0 * 0.85, r2: (d.w / 2.0).min(d.d / 2.0) });
                v.push(Shape::tube([x0, 0.0, d.h - 1.05], [x0, 0.0, d.h - 0.9], d.d / 2.0 * 0.85));
                v.push(Shape::tube([x0, 0.0, d.h - 1.15], [x0, 0.0, d.h - 1.05], 0.15));
                v.push(Shape::tube([x0, 0.0, d.h - 1.75], [x0, 0.0, d.h - 1.2], 0.18));
            }
        }
        EquipKind::Package => {
            // Skid of two main beams and cross members, the equipment enclosure, a control panel.
            for sy in [-1.0, 1.0] { v.push(bx([0.0, sy * (d.w / 2.0 - 0.1), 0.1], [d.l / 2.0, 0.1, 0.1])); }
            for k in 0..5 { let xk = -d.l / 2.0 + 0.05 + (d.l - 0.1) * k as f64 / 4.0; v.push(bx([xk, 0.0, 0.08], [0.05, d.w / 2.0 - 0.2, 0.08])); }
            v.push(bx([0.0, 0.0, 0.21], [d.l / 2.0, d.w / 2.0, 0.01]));
            v.push(bx([-d.l * 0.08, 0.0, 0.22 + (d.h - 0.22) / 2.0], [d.l * 0.34, d.w * 0.42, (d.h - 0.22) / 2.0]));
            v.push(bx([d.l * 0.38, 0.0, 0.22 + (d.h * 0.75 - 0.22) / 2.0], [0.1, d.w * 0.25, (d.h * 0.75 - 0.22) / 2.0]));
        }
    }
    v
}

/// The shell radius platforms are built round, for the kinds that take them.
pub fn shell_radius(e: &Equipment) -> Option<f64> {
    match e.kind {
        EquipKind::VerticalVessel | EquipKind::Column | EquipKind::Agitated | EquipKind::Tank => Some(e.dims.d / 2.0),
        _ => None,
    }
}

/// The platforms and ladders of an item, in its own axes (0.3.7): grating in segments round the
/// shell (open where a ladder comes through), a toe plate, handrails with posts, brackets with knee
/// braces back to the shell; and for each platform with a ladder, a caged ladder from the platform
/// below (or grade) up through it.
pub fn platform_shapes(e: &Equipment) -> Vec<Shape> {
    let Some(r) = shell_radius(e) else { return vec![] };
    let mut v = vec![];
    let z: P3 = [0.0, 0.0, 1.0];
    let dir = |a: f64| -> P3 { [a.cos(), a.sin(), 0.0] };
    let tan = |a: f64| -> P3 { [-a.sin(), a.cos(), 0.0] };
    let pt = |rad: f64, a: f64, h: f64| -> P3 { [rad * a.cos(), rad * a.sin(), h] };
    let mut below = 0.0;
    for pl in &e.platforms {
        let (r0, r1) = (r + 0.05, r + 0.05 + pl.width);
        let (a0, a1) = (pl.from.to_radians(), pl.to.to_radians());
        let span = a1 - a0;
        let full = span >= TAU - 1e-6;
        let la = pl.ladder_at.to_radians();
        let rl = r + 0.3;
        let open = 0.35 / rl;
        let ang_off = |a: f64| { let d = (a - la).rem_euclid(TAU); d.min(TAU - d) };
        // Grating and toe plate, about 10° a segment.
        let n = ((span / 10f64.to_radians()).ceil() as usize).max(1);
        let da = span / n as f64;
        for k in 0..n {
            let a = a0 + (k as f64 + 0.5) * da;
            let ax = [dir(a), tan(a), z];
            if !(pl.ladder && ang_off(a) < open + da / 2.0) {
                v.push(Shape::Cuboid { centre: pt((r0 + r1) / 2.0, a, pl.el - 0.015), axes: ax, half: [pl.width / 2.0, r1 * (da / 2.0).tan() + 0.005, 0.015] });
            }
            v.push(Shape::Cuboid { centre: pt(r1 - 0.004, a, pl.el + 0.05), axes: ax, half: [0.004, r1 * (da / 2.0).tan() + 0.005, 0.05] });
        }
        // Handrail (top and knee rails) and posts; brackets and knee braces at the same places.
        for h in [1.1, 0.55] { v.push(Shape::Torus { centre: [0.0, 0.0, pl.el + h], axis: z, start: mul(dir(a0), r1 - 0.025), angle: span, r: 0.022 }); }
        let posts = ((span * r1 / 1.6).ceil() as usize).max(1);
        let m = if full { posts } else { posts + 1 };
        for k in 0..m {
            let a = a0 + span * k as f64 / posts as f64;
            v.push(Shape::tube(pt(r1 - 0.025, a, pl.el), pt(r1 - 0.025, a, pl.el + 1.1), 0.022));
            v.push(Shape::Cuboid { centre: pt((r0 + r1) / 2.0, a, pl.el - 0.08), axes: [dir(a), tan(a), z], half: [pl.width / 2.0, 0.035, 0.05] });
            v.push(Shape::tube(pt(r, a, pl.el - 0.13 - pl.width * 0.8), pt(r1 - 0.1, a, pl.el - 0.12), 0.03));
        }
        // The caged ladder: two stringers, rungs every 300 mm, hoops and straps from 2.2 m up.
        if pl.ladder {
            let (n0, t0) = (dir(la), tan(la));
            let base = mul(n0, rl);
            let at = |s: f64, h: f64| add(add(base, mul(t0, s)), [0.0, 0.0, h]);
            let (z0, z1) = (below, pl.el + 1.1);
            for s in [-0.23, 0.23] { v.push(Shape::tube(at(s, z0), at(s, z1), 0.03)); }
            let mut h = z0 + 0.3;
            while h < pl.el - 0.05 { v.push(Shape::tube(at(-0.23, h), at(0.23, h), 0.012)); h += 0.3; }
            let c0 = z0 + 2.2;
            if z1 > c0 + 0.3 {
                let centre = add(base, mul(n0, 0.36));
                let mut h = c0;
                while h <= z1 + 1e-9 { v.push(Shape::Torus { centre: add(centre, [0.0, 0.0, h]), axis: z, start: mul(t0, -0.36), angle: std::f64::consts::PI, r: 0.02 }); h += 0.9; }
                for k in 0..5 {
                    let b = std::f64::consts::PI * (k as f64 + 0.5) / 5.0;
                    let off = add(mul(t0, -0.36 * b.cos()), mul(n0, 0.36 * b.sin()));
                    v.push(Shape::Cuboid { centre: add(add(centre, off), [0.0, 0.0, (c0 + z1) / 2.0]), axes: [z, n0, t0], half: [(z1 - c0) / 2.0, 0.004, 0.025] });
                }
            }
        }
        below = pl.el;
    }
    v
}

/// Moves a shape from the item's axes to the plot.
pub fn place(s: &Shape, pos: P3, rot: f64) -> Shape {
    let p = |v: P3| add(pos, rot_z(v, rot));
    let r = |v: P3| rot_z(v, rot);
    match s {
        Shape::Frustum { a, b, r1, r2 } => Shape::Frustum { a: p(*a), b: p(*b), r1: *r1, r2: *r2 },
        Shape::Torus { centre, axis, start, angle, r: rr } => Shape::Torus { centre: p(*centre), axis: r(*axis), start: r(*start), angle: *angle, r: *rr },
        Shape::Head { base, dir, r: rr, depth } => Shape::Head { base: p(*base), dir: r(*dir), r: *rr, depth: *depth },
        Shape::Cuboid { centre, axes, half } => Shape::Cuboid { centre: p(*centre), axes: [r(axes[0]), r(axes[1]), r(axes[2])], half: *half },
    }
}

/// A nozzle on the plot: its port and its shapes (neck and flange).
#[derive(Clone, Debug, Serialize)]
pub struct PlacedNozzle {
    pub tag: String,
    pub port: Port,
    pub shapes: Vec<Shape>,
}

pub fn nozzle(e: &Equipment, n: &Nozzle, cat: &Catalogue) -> PlacedNozzle {
    let p = add(e.pos, rot_z(n.pos, e.rot));
    let d = rot_z(n.dir, e.rot);
    let od = cat.od(&n.nps);
    let (fod, thick, y) = cat.flange(&n.nps, n.rating).map(|f| (f.od, f.thick, f.wn_length)).unwrap_or((od * 2.0, od * 0.2, od * 0.5));
    let neck0 = sub(p, mul(d, n.proj.max(y + 0.01)));
    let shapes = vec![
        Shape::tube(neck0, sub(p, mul(d, y)), od / 2.0),
        Shape::Frustum { a: sub(p, mul(d, y)), b: sub(p, mul(d, thick)), r1: od / 2.0 * 1.05, r2: (od / 2.0 * 1.35).min(fod / 2.0) },
        Shape::tube(sub(p, mul(d, thick)), p, fod / 2.0),
    ];
    PlacedNozzle { tag: n.tag.clone(), port: Port { p, d, nps: n.nps.clone(), rating: n.rating, end: "FL" }, shapes }
}

/// The item on the plot: body shapes and nozzles.
/// The item's platforms and ladders on the plot.
pub fn platforms(e: &Equipment) -> Vec<Shape> {
    platform_shapes(e).iter().map(|s| place(s, e.pos, e.rot)).collect()
}

pub fn shapes(e: &Equipment, cat: &Catalogue) -> (Vec<Shape>, Vec<PlacedNozzle>) {
    let body = body(e).iter().map(|s| place(s, e.pos, e.rot)).collect();
    (body, e.nozzles.iter().map(|n| nozzle(e, n, cat)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platforms_ring_the_shell_and_ladders_climb_to_them() {
        let mut e = new(EquipKind::Column, "C-1");
        let r = e.dims.d / 2.0;
        e.platforms = vec![
            crate::model::Platform { el: 6.0, ..Default::default() },
            crate::model::Platform { el: 12.0, from: 90.0, to: 270.0, width: 1.2, ladder: true, ladder_at: 180.0 },
        ];
        let s = platform_shapes(&e);
        // Bounds of what is drawn (a torus's box is loose).
        let bounds = |v: &[&Shape]| { let mut m = crate::parts::Mesh::default(); for x in v { m.shape(x, 24); } let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]); for c in m.pos.chunks(3) { for i in 0..3 { lo[i] = lo[i].min(c[i] as f64); hi[i] = hi[i].max(c[i] as f64); } } (lo, hi) };
        let (lo, hi) = bounds(&s.iter().collect::<Vec<_>>());
        // From grade (the first ladder) to the upper handrail; out to the lower platform's edge.
        assert!(lo[2] < 0.05 && (hi[2] - 13.12).abs() < 0.01, "{lo:?} {hi:?}");
        assert!((hi[0] - (r + 1.05)).abs() < 0.02, "{hi:?}");
        // The upper platform covers the west half only: nothing of it east of the shell.
        let upper: Vec<&Shape> = s.iter().filter(|x| { let (a, b) = x.aabb(); a[2] > 11.5 && b[2] < 13.3 }).collect();
        assert!(!upper.is_empty() && bounds(&upper).1[0] < r * 0.2, "the upper platform stays on its arc: {:?}", bounds(&upper));
        // Kinds without a round shell take none.
        let mut p = new(EquipKind::Pump, "P-1");
        p.platforms = vec![crate::model::Platform::default()];
        assert!(platform_shapes(&p).is_empty());
    }
    use crate::parts::{dist, dot};

    #[test]
    fn every_type_has_its_nozzles_outside_its_body_facing_out() {
        let cat = Catalogue::builtin();
        for k in EquipKind::ALL {
            let mut e = new(k, "X-1");
            e.pos = [10.0, 5.0, 0.0];
            e.rot = 90.0;
            let (body, noz) = shapes(&e, &cat);
            assert!(!body.is_empty() && !noz.is_empty(), "{k:?}");
            assert!(!fields(k).is_empty());
            let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
            for s in &body { let (a, b) = s.aabb(); for i in 0..3 { lo[i] = lo[i].min(a[i]); hi[i] = hi[i].max(b[i]); } }
            assert!(lo[2] >= -1e-9, "{k:?} sits on its base");
            for n in &noz {
                // The face is outside the body's box along its direction, and the port points away.
                let c = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0, (lo[2] + hi[2]) / 2.0];
                let outside = body.iter().all(|s| { let (a, b) = s.aabb(); (0..3).any(|i| n.port.p[i] < a[i] - 1e-6 || n.port.p[i] > b[i] + 1e-6) });
                assert!(outside, "{k:?} {} face inside the body", n.tag);
                let q = crate::parts::add(n.port.p, crate::parts::mul(n.port.d, 0.3));
                assert!(body.iter().all(|s| { let (a, b) = s.aabb(); (0..3).any(|i| q[i] < a[i] || q[i] > b[i]) }), "{k:?} {} faces into the body", n.tag);
                let _ = (c, dot(n.port.d, n.port.d));
                assert!(n.port.p[2] > 0.3, "{k:?} {} too near the ground", n.tag);
            }
        }
        // Rotation keeps nozzle distances from the origin.
        let mut e = new(EquipKind::Pump, "P-1");
        let a = nozzle(&e, &e.nozzles[0].clone(), &cat).port.p;
        e.rot = 37.0;
        let b = nozzle(&e, &e.nozzles[0].clone(), &cat).port.p;
        assert!((dist(a, [0.0; 3]) - dist(b, [0.0; 3])).abs() < 1e-12);
    }
}
