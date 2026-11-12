//! The parametric equipment types (six to 0.2.12, seventeen from 0.2.13): their shapes from datasheet sizes, and their nozzles.
//! Each type's sizes are a few named values (`fields`); nozzles are listed by the designer (new
//! equipment gets typical ones) and drawn as a neck with a weld-neck flange at their face.

use crate::catalogue::Catalogue;
use crate::model::{Dims, EquipKind, Equipment, Nozzle, P3};
use crate::parts::{add, mul, rot_z, sub, Port, Shape};
use serde::Serialize;

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

/// The body of an item (in its own axes): shapes, without nozzles.
fn body(e: &Equipment) -> Vec<Shape> {
    let d = &e.dims;
    let r = d.d / 2.0;
    let x: P3 = [1.0, 0.0, 0.0];
    let z: P3 = [0.0, 0.0, 1.0];
    let ax = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let saddle = |xs: f64, cz: f64, w: f64| Shape::Cuboid { centre: [xs, 0.0, cz / 2.0], axes: ax, half: [0.1, w / 2.0, cz / 2.0] };
    match e.kind {
        EquipKind::VerticalVessel | EquipKind::Column => vec![
            Shape::tube([0.0; 3], [0.0, 0.0, d.elev], r),
            Shape::tube([0.0, 0.0, d.elev], [0.0, 0.0, d.elev + d.h], r),
            Shape::Head { base: [0.0, 0.0, d.elev + d.h], dir: z, r, depth: d.d / 4.0 },
            Shape::Head { base: [0.0, 0.0, d.elev], dir: [0.0, 0.0, -1.0], r: r * 0.999, depth: (d.d / 4.0).min(d.elev * 0.9) },
        ],
        EquipKind::HorizontalDrum => vec![
            Shape::tube([-d.l / 2.0, 0.0, d.elev], [d.l / 2.0, 0.0, d.elev], r),
            Shape::Head { base: [d.l / 2.0, 0.0, d.elev], dir: x, r, depth: d.d / 4.0 },
            Shape::Head { base: [-d.l / 2.0, 0.0, d.elev], dir: [-1.0, 0.0, 0.0], r, depth: d.d / 4.0 },
            saddle(-d.l * 0.3, d.elev, d.d * 0.8),
            saddle(d.l * 0.3, d.elev, d.d * 0.8),
        ],
        EquipKind::Tank => vec![
            Shape::tube([0.0; 3], [0.0, 0.0, d.h], r),
            Shape::Frustum { a: [0.0, 0.0, d.h], b: [0.0, 0.0, d.h + d.d * 0.08], r1: r, r2: r * 0.05 },
        ],
        EquipKind::Pump => {
            let x0 = -d.l / 2.0 + d.d * 0.6;
            let xm = d.l / 2.0 - d.l2 - 0.05;
            vec![
                Shape::Cuboid { centre: [0.0, 0.0, 0.075], axes: ax, half: [d.l / 2.0, d.w / 2.0, 0.075] },
                Shape::tube([x0 - d.d * 0.25, 0.0, d.h], [x0 + d.d * 0.25, 0.0, d.h], r),
                Shape::Cuboid { centre: [x0, 0.0, (d.h + 0.15) / 2.0], axes: ax, half: [d.d * 0.2, d.d * 0.3, ((d.h - 0.15) / 2.0).max(0.01)] },
                Shape::tube([x0 + d.d * 0.25, 0.0, d.h], [xm, 0.0, d.h], d.d * 0.18),
                Shape::tube([xm, 0.0, d.h], [xm + d.l2, 0.0, d.h], d.d2 / 2.0),
                Shape::Cuboid { centre: [xm + d.l2 / 2.0, 0.0, (d.h - d.d2 / 2.0 + 0.15) / 2.0], axes: ax, half: [d.l2 * 0.35, d.d2 * 0.4, ((d.h - d.d2 / 2.0 - 0.15) / 2.0).max(0.01)] },
            ]
        }
        EquipKind::Exchanger => vec![
            Shape::tube([-d.l / 2.0, 0.0, d.elev], [d.l / 2.0, 0.0, d.elev], r),
            Shape::Head { base: [-d.l / 2.0, 0.0, d.elev], dir: [-1.0, 0.0, 0.0], r, depth: d.d / 4.0 },
            Shape::tube([d.l / 2.0, 0.0, d.elev], [d.l / 2.0 + 0.06, 0.0, d.elev], r * 1.18),
            Shape::tube([d.l / 2.0 + 0.06, 0.0, d.elev], [d.l / 2.0 + d.l2, 0.0, d.elev], r),
            Shape::tube([d.l / 2.0 + d.l2, 0.0, d.elev], [d.l / 2.0 + d.l2 + 0.06, 0.0, d.elev], r * 1.18),
            saddle(-d.l * 0.3, d.elev, d.d * 0.8),
            saddle(d.l * 0.3, d.elev, d.d * 0.8),
        ],
        EquipKind::Sphere => {
            let c = [0.0, 0.0, d.elev];
            let mut v = vec![Shape::Head { base: c, dir: z, r, depth: r }, Shape::Head { base: c, dir: [0.0, 0.0, -1.0], r, depth: r }];
            for k in 0..6 { let a = k as f64 * std::f64::consts::TAU / 6.0; let (x0, y0) = (r * a.cos(), r * a.sin()); v.push(Shape::tube([x0, y0, 0.0], [x0, y0, d.elev], d.d * 0.025)); }
            v
        }
        EquipKind::Filter => {
            let mut v = vec![
                Shape::tube([0.0, 0.0, d.elev], [0.0, 0.0, d.elev + d.h], r),
                Shape::tube([0.0, 0.0, d.elev + d.h], [0.0, 0.0, d.elev + d.h + 0.08], r * 1.25),
                Shape::Head { base: [0.0, 0.0, d.elev + d.h + 0.08], dir: z, r, depth: d.d / 4.0 },
                Shape::Head { base: [0.0, 0.0, d.elev], dir: [0.0, 0.0, -1.0], r, depth: d.d / 4.0 },
            ];
            for k in 0..3 { let a = (k as f64 + 0.25) * std::f64::consts::TAU / 3.0; let (x0, y0) = (r * 0.85 * a.cos(), r * 0.85 * a.sin()); v.push(Shape::tube([x0, y0, 0.0], [x0, y0, d.elev + 0.1], 0.03)); }
            v
        }
        EquipKind::Agitated => {
            let top = d.elev + d.h + d.d / 4.0;
            vec![
                Shape::tube([0.0; 3], [0.0, 0.0, d.elev], r),
                Shape::tube([0.0, 0.0, d.elev], [0.0, 0.0, d.elev + d.h], r),
                Shape::Head { base: [0.0, 0.0, d.elev + d.h], dir: z, r, depth: d.d / 4.0 },
                Shape::Head { base: [0.0, 0.0, d.elev], dir: [0.0, 0.0, -1.0], r: r * 0.999, depth: (d.d / 4.0).min(d.elev * 0.9) },
                Shape::Cuboid { centre: [0.0, 0.0, top + 0.15], axes: ax, half: [d.d2 * 0.7, d.d2 * 0.6, 0.15] },
                Shape::tube([0.0, 0.0, top + 0.3], [0.0, 0.0, top + 0.3 + d.l2], d.d2 / 2.0),
            ]
        }
        EquipKind::Heater => vec![
            Shape::Cuboid { centre: [0.0, 0.0, d.h / 2.0], axes: ax, half: [d.l / 2.0, d.w / 2.0, d.h / 2.0] },
            Shape::Cuboid { centre: [0.0, 0.0, d.h + 1.0], axes: ax, half: [d.l * 0.35, d.w * 0.3, 1.0] },
            Shape::Frustum { a: [0.0, 0.0, d.h + 2.0], b: [0.0, 0.0, d.h + 2.6], r1: d.w * 0.3, r2: d.d / 2.0 },
            Shape::tube([0.0, 0.0, d.h + 2.6], [0.0, 0.0, d.h + 2.6 + d.l2], d.d / 2.0),
        ],
        EquipKind::InlinePump => vec![
            Shape::Cuboid { centre: [0.0, 0.0, (d.h - r) / 2.0], axes: ax, half: [d.d * 0.6, d.d * 0.5, ((d.h - r) / 2.0).max(0.02)] },
            Shape::tube([-d.d, 0.0, d.h], [d.d, 0.0, d.h], r * 0.55),
            Shape::tube([0.0, 0.0, d.h - r], [0.0, 0.0, d.h + r], r),
            Shape::tube([0.0, 0.0, d.h + r], [0.0, 0.0, d.h + r + 0.25], d.d * 0.18),
            Shape::tube([0.0, 0.0, d.h + r + 0.25], [0.0, 0.0, d.h + r + 0.25 + d.l2], d.d2 / 2.0),
        ],
        EquipKind::Compressor => {
            let xc = -d.l / 2.0 + d.d * 0.9;
            let xm = d.l / 2.0 - d.l2 - 0.1;
            vec![
                Shape::Cuboid { centre: [0.0, 0.0, 0.15], axes: ax, half: [d.l / 2.0, d.w / 2.0, 0.15] },
                Shape::tube([xc - d.d * 0.6, 0.0, d.h], [xc + d.d * 0.6, 0.0, d.h], r),
                Shape::Cuboid { centre: [xc, 0.0, (d.h + 0.3) / 2.0], axes: ax, half: [d.d * 0.5, d.d * 0.4, ((d.h - 0.3) / 2.0).max(0.02)] },
                Shape::tube([xc + d.d * 0.6, 0.0, d.h], [xm, 0.0, d.h], d.d * 0.12),
                Shape::tube([xm, 0.0, d.h], [xm + d.l2, 0.0, d.h], d.d2 / 2.0),
                Shape::Cuboid { centre: [xm + d.l2 / 2.0, 0.0, (d.h - d.d2 / 2.0 + 0.3) / 2.0], axes: ax, half: [d.l2 * 0.4, d.d2 * 0.4, ((d.h - d.d2 / 2.0 - 0.3) / 2.0).max(0.02)] },
            ]
        }
        EquipKind::RecipCompressor => {
            let zc = d.h * 0.75;
            vec![
                Shape::Cuboid { centre: [0.0, 0.0, d.h / 2.0], axes: ax, half: [d.l * 0.25, d.w / 2.0, d.h / 2.0] },
                Shape::tube([0.0, d.w / 2.0, zc], [0.0, d.w / 2.0 + 1.0, zc], d.d / 2.0),
                Shape::tube([0.0, -d.w / 2.0, zc], [0.0, -(d.w / 2.0 + 1.0), zc], d.d / 2.0),
                Shape::tube([d.l * 0.25, 0.0, d.h * 0.45], [d.l * 0.25 + d.l2, 0.0, d.h * 0.45], d.d2 / 2.0),
            ]
        }
        EquipKind::Kettle => {
            let ck = d.elev + (d.d2 - d.d) / 2.0;
            let rk = d.d2 / 2.0;
            vec![
                Shape::tube([-d.l / 2.0, 0.0, ck], [d.l / 2.0 - 0.9, 0.0, ck], rk),
                Shape::Head { base: [-d.l / 2.0, 0.0, ck], dir: [-1.0, 0.0, 0.0], r: rk, depth: d.d2 / 4.0 },
                Shape::Frustum { a: [d.l / 2.0 - 0.9, 0.0, ck], b: [d.l / 2.0 - 0.4, 0.0, d.elev], r1: rk, r2: r },
                Shape::tube([d.l / 2.0 - 0.4, 0.0, d.elev], [d.l / 2.0, 0.0, d.elev], r),
                Shape::tube([d.l / 2.0, 0.0, d.elev], [d.l / 2.0 + 0.06, 0.0, d.elev], r * 1.18),
                Shape::tube([d.l / 2.0 + 0.06, 0.0, d.elev], [d.l / 2.0 + d.l2, 0.0, d.elev], r),
                saddle(-d.l * 0.3, ck, d.d2 * 0.8),
                saddle(d.l * 0.25, d.elev, d.d * 0.8),
            ]
        }
        EquipKind::PlateHx => vec![
            Shape::Cuboid { centre: [-d.l / 2.0 + 0.075, 0.0, d.h / 2.0], axes: ax, half: [0.075, d.w / 2.0, d.h / 2.0] },
            Shape::Cuboid { centre: [-d.l / 2.0 + 0.15 + d.l * 0.25, 0.0, d.h / 2.0 + 0.05], axes: ax, half: [d.l * 0.25, d.w * 0.4, d.h * 0.42] },
            Shape::Cuboid { centre: [-d.l / 2.0 + 0.15 + d.l * 0.5 + 0.06, 0.0, d.h / 2.0], axes: ax, half: [0.06, d.w / 2.0, d.h / 2.0] },
            Shape::tube([-d.l / 2.0, 0.0, d.h - 0.05], [d.l / 2.0, 0.0, d.h - 0.05], 0.04),
            Shape::tube([-d.l / 2.0, 0.0, 0.1], [d.l / 2.0, 0.0, 0.1], 0.04),
            Shape::Cuboid { centre: [d.l / 2.0 - 0.06, 0.0, d.h / 2.0], axes: ax, half: [0.06, 0.12, d.h / 2.0] },
        ],
        EquipKind::AirCooler => {
            let mut v = vec![
                Shape::Cuboid { centre: [0.0, 0.0, d.h + 0.15], axes: ax, half: [d.l / 2.0, d.w / 2.0, 0.15] },
                Shape::Cuboid { centre: [-(d.l / 2.0 + 0.15), 0.0, d.h + 0.15], axes: ax, half: [0.15, d.w / 2.0, 0.3] },
                Shape::Cuboid { centre: [d.l / 2.0 + 0.15, 0.0, d.h + 0.15], axes: ax, half: [0.15, d.w / 2.0, 0.3] },
            ];
            for (sx, sy) in [(-1.0, -1.0), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] { v.push(Shape::tube([sx * d.l * 0.45, sy * d.w * 0.45, 0.0], [sx * d.l * 0.45, sy * d.w * 0.45, d.h], 0.08)); }
            for sx in [-1.0, 1.0] {
                let x0 = sx * d.l / 4.0;
                v.push(Shape::Frustum { a: [x0, 0.0, d.h - 0.9], b: [x0, 0.0, d.h], r1: d.d / 2.0 * 0.85, r2: (d.w / 2.0).min(d.d / 2.0) });
                v.push(Shape::tube([x0, 0.0, d.h - 1.05], [x0, 0.0, d.h - 0.9], d.d / 2.0 * 0.85));
            }
            v
        }
        EquipKind::Package => vec![
            Shape::Cuboid { centre: [0.0, 0.0, 0.1], axes: ax, half: [d.l / 2.0, d.w / 2.0, 0.1] },
            Shape::Cuboid { centre: [0.0, 0.0, 0.2 + (d.h - 0.2) / 2.0], axes: ax, half: [d.l * 0.42, d.w * 0.42, (d.h - 0.2) / 2.0] },
        ],
    }
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
pub fn shapes(e: &Equipment, cat: &Catalogue) -> (Vec<Shape>, Vec<PlacedNozzle>) {
    let body = body(e).iter().map(|s| place(s, e.pos, e.rot)).collect();
    (body, e.nozzles.iter().map(|n| nozzle(e, n, cat)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
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
