//! The six parametric equipment types: their shapes from datasheet sizes, and their nozzles.
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
