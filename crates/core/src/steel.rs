//! Structural steel (0.3.0): members (columns, beams, braces) drawn as their rolled section from
//! the steel table, along the line between their two ends. The section stands with its web
//! vertical on a sloping or level member, and its web along east on a vertical one; `roll` turns
//! it about its own axis.

use crate::catalogue::Catalogue;
use crate::model::{Member, P3};
use crate::parts::{add, cross, mul, norm, sub, unit, Shape};

/// Steel density (kg/m³), for weights from the section's area.
pub const DENSITY: f64 = 7850.0;

/// A member's length (m).
pub fn length(m: &Member) -> f64 {
    norm(sub(m.b, m.a))
}

/// The member's axes: along it, the section's depth (web) direction, and its width (flange) direction.
pub fn frame(m: &Member) -> (P3, P3, P3) {
    let u = unit(sub(m.b, m.a));
    // Depth: the vertical's part square to the member; a vertical member's depth runs east.
    let d0 = if u[2].abs() > 0.95 { [1.0, 0.0, 0.0] } else { unit(sub([0.0, 0.0, 1.0], mul(u, u[2]))) };
    let f0 = cross(u, d0);
    let (s, c) = m.roll.to_radians().sin_cos();
    let d = add(mul(d0, c), mul(f0, s));
    let f = cross(u, d);
    (u, d, f)
}

/// The member's shapes: web and two flanges (a box when its section is not in the table).
pub fn shapes(m: &Member, cat: &Catalogue) -> Vec<Shape> {
    let l = length(m);
    if l < 1e-6 { return vec![]; }
    let (u, d, f) = frame(m);
    let mid = mul(add(m.a, m.b), 0.5);
    let Some(s) = cat.steel_section(&m.section) else {
        return vec![Shape::Cuboid { centre: mid, axes: [u, d, f], half: [l / 2.0, 0.1, 0.1] }];
    };
    let off = s.h / 2.0 - s.tf / 2.0;
    vec![
        Shape::Cuboid { centre: mid, axes: [u, d, f], half: [l / 2.0, (s.h / 2.0 - s.tf).max(0.001), s.tw / 2.0] },
        Shape::Cuboid { centre: add(mid, mul(d, off)), axes: [u, d, f], half: [l / 2.0, s.tf / 2.0, s.b / 2.0] },
        Shape::Cuboid { centre: add(mid, mul(d, -off)), axes: [u, d, f], half: [l / 2.0, s.tf / 2.0, s.b / 2.0] },
    ]
}

/// The member's weight (kg), when its section is in the table.
pub fn weight(m: &Member, cat: &Catalogue) -> Option<f64> {
    cat.steel_section(&m.section).map(|s| s.area * DENSITY * length(m))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MemberRole;

    #[test]
    fn every_section_has_the_area_its_dimensions_give() {
        let cat = Catalogue::builtin();
        assert!(cat.steel.len() >= 56, "{}", cat.steel.len());
        for s in &cat.steel {
            let a = 2.0 * s.b * s.tf + (s.h - 2.0 * s.tf) * s.tw + (4.0 - std::f64::consts::PI) * s.r * s.r;
            assert!((a - s.area).abs() / s.area < 5e-3, "{}: {a} against {}", s.name, s.area);
        }
        // HEB 200: 78.1 cm², 61.3 kg/m.
        let s = cat.steel_section("HEB 200").unwrap();
        assert!((s.area * DENSITY - 61.3).abs() < 0.1, "{}", s.area * DENSITY);
    }

    #[test]
    fn members_are_drawn_along_their_ends_web_up_or_east() {
        let cat = Catalogue::builtin();
        let s = cat.steel_section("IPE 300").unwrap().clone();
        // A level beam running north: depth up, width east.
        let beam = Member { role: MemberRole::Beam, section: "IPE 300".into(), a: [0.0, 0.0, 5.0], b: [0.0, 6.0, 5.0], ..Default::default() };
        let (lo, hi) = bounds(&shapes(&beam, &cat));
        assert!((hi[2] - lo[2] - s.h).abs() < 1e-9 && (hi[0] - lo[0] - s.b).abs() < 1e-9 && (hi[1] - lo[1] - 6.0).abs() < 1e-9, "{lo:?} {hi:?}");
        // Rolled 90°: on its side.
        let side = Member { roll: 90.0, ..beam.clone() };
        let (lo, hi) = bounds(&shapes(&side, &cat));
        assert!((hi[2] - lo[2] - s.b).abs() < 1e-9 && (hi[0] - lo[0] - s.h).abs() < 1e-9);
        // A column: depth east, width north, from its base to its top.
        let col = Member { role: MemberRole::Column, section: "IPE 300".into(), a: [2.0, 2.0, 0.0], b: [2.0, 2.0, 4.0], ..Default::default() };
        let (lo, hi) = bounds(&shapes(&col, &cat));
        assert!((hi[0] - lo[0] - s.h).abs() < 1e-9 && (hi[1] - lo[1] - s.b).abs() < 1e-9 && lo[2].abs() < 1e-9 && (hi[2] - 4.0).abs() < 1e-9);
        // Weight: area × density × length.
        assert!((weight(&col, &cat).unwrap() - s.area * DENSITY * 4.0).abs() < 1e-9);
        // A section not in the table is still drawn (as a box) and has no weight.
        let odd = Member { section: "W 99x9".into(), ..col };
        assert_eq!(shapes(&odd, &cat).len(), 1);
        assert!(weight(&odd, &cat).is_none());
    }

    fn bounds(v: &[Shape]) -> (P3, P3) {
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for s in v { let (a, b) = s.aabb(); for k in 0..3 { lo[k] = lo[k].min(a[k]); hi[k] = hi[k].max(b[k]); } }
        (lo, hi)
    }
}
