//! Structural steel (0.3.0): members (columns, beams, braces) drawn as their rolled section from
//! the steel table, along the line between their two ends. The section stands with its web
//! vertical on a sloping or level member, and its web along east on a vertical one; `roll` turns
//! it about its own axis.

use crate::catalogue::Catalogue;
use crate::model::{Member, MemberRole, P3};
use crate::parts::{add, cross, dot, mul, norm, sub, unit, Shape};

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

/// The rolled section along the member from `s0` to `s1` (distances from end a): web, two
/// flanges and the four root fillets (as 45° fills); a box when the section is not in the table.
pub fn section(m: &Member, cat: &Catalogue, s0: f64, s1: f64) -> Vec<Shape> {
    let (u, d, f) = frame(m);
    let l = s1 - s0;
    if l < 1e-6 { return vec![]; }
    let mid = add(m.a, mul(u, (s0 + s1) / 2.0));
    let Some(s) = cat.steel_section(&m.section) else {
        return vec![Shape::Cuboid { centre: mid, axes: [u, d, f], half: [l / 2.0, 0.1, 0.1] }];
    };
    let off = s.h / 2.0 - s.tf / 2.0;
    let mut v = vec![
        Shape::Cuboid { centre: mid, axes: [u, d, f], half: [l / 2.0, (s.h / 2.0 - s.tf).max(0.001), s.tw / 2.0] },
        Shape::Cuboid { centre: add(mid, mul(d, off)), axes: [u, d, f], half: [l / 2.0, s.tf / 2.0, s.b / 2.0] },
        Shape::Cuboid { centre: add(mid, mul(d, -off)), axes: [u, d, f], half: [l / 2.0, s.tf / 2.0, s.b / 2.0] },
    ];
    if s.r > 0.0 {
        // A block on the open side of each web-to-flange corner, its outer face the fillet's chord.
        let k = std::f64::consts::FRAC_1_SQRT_2;
        for (sd, sf) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
            let c = add(mid, add(mul(d, sd * (s.h / 2.0 - s.tf)), mul(f, sf * s.tw / 2.0)));
            let w = unit(sub(mul(f, sf), mul(d, sd)));
            let n = unit(add(mul(d, sd), mul(f, sf)));
            v.push(Shape::Cuboid { centre: add(c, mul(w, s.r * k / 2.0)), axes: [u, w, n], half: [l / 2.0, s.r * k / 2.0, s.r * k] });
        }
    }
    v
}

/// How far a member's end `at` stands back from the face of a column (or any other member whose
/// axis passes through it, other than itself), measured along the member: the other section's half
/// size in this member's direction. Zero at a free end.
fn setback(m: &Member, at: P3, all: &[Member], cat: &Catalogue) -> f64 {
    let u = unit(sub(m.b, m.a));
    let mut best: f64 = 0.0;
    for o in all.iter().filter(|o| o.id != m.id && (o.role == MemberRole::Column || o.role == MemberRole::Beam && m.role == MemberRole::Brace)) {
        let ou = sub(o.b, o.a);
        let ll = dot(ou, ou);
        if ll < 1e-12 { continue; }
        let t = (dot(sub(at, o.a), ou) / ll).clamp(0.0, 1.0);
        if norm(sub(add(o.a, mul(ou, t)), at)) > 1e-3 { continue; }
        // Ends meeting end to end (a column on a column) do not stand back.
        if dot(unit(ou), u).abs() > 0.95 { continue; }
        let Some(s) = cat.steel_section(&o.section) else { continue };
        let (_, od, of) = frame(o);
        best = best.max(dot(u, od).abs() * s.h / 2.0 + dot(u, of).abs() * s.b / 2.0);
    }
    best
}

/// The member as built: its section, stopped at the faces of the members it frames into, with an
/// end plate at each end of a beam or brace; a column has a base plate with four anchor bolts at
/// its foot and a cap plate on top. The plates are for the picture (the weight is the section's).
pub fn shapes(m: &Member, all: &[Member], cat: &Catalogue) -> Vec<Shape> {
    let l = length(m);
    if l < 1e-6 { return vec![]; }
    let (u, d, f) = frame(m);
    let Some(s) = cat.steel_section(&m.section).cloned() else { return section(m, cat, 0.0, l) };
    let vertical = u[2].abs() > 0.95;
    let (mut s0, mut s1) = (0.0, l);
    let mut v = vec![];
    let plate = |c: P3, n: P3, t: f64, hd: f64, hf: f64| Shape::Cuboid { centre: c, axes: [n, d, f], half: [t / 2.0, hd, hf] };
    if m.role == MemberRole::Column || vertical {
        // Foot (the lower end) and top.
        let (foot, top, up) = if m.a[2] <= m.b[2] { (m.a, m.b, u) } else { (m.b, m.a, mul(u, -1.0)) };
        let (tb, ext) = (0.02, 0.075);
        v.push(plate(add(foot, mul(up, tb / 2.0)), up, tb, s.h / 2.0 + ext, s.b / 2.0 + ext));
        for (i, j) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
            let c = add(foot, add(mul(d, i * (s.h / 2.0 + ext - 0.04)), mul(f, j * (s.b / 2.0 + ext - 0.04))));
            v.push(Shape::tube(add(c, mul(up, tb)), add(c, mul(up, tb + 0.06)), 0.012));
            v.push(Shape::tube(add(c, mul(up, tb)), add(c, mul(up, tb + 0.02)), 0.022));
        }
        v.push(plate(sub(top, mul(up, 0.0075)), up, 0.015, s.h / 2.0 + 0.01, s.b / 2.0 + 0.01));
    } else {
        s0 = setback(m, m.a, all, cat);
        s1 = l - setback(m, m.b, all, cat);
        if s1 - s0 < 0.05 { s0 = 0.0; s1 = l; }
        let t = 0.015;
        v.push(plate(add(m.a, mul(u, s0 + t / 2.0)), u, t, s.h / 2.0 + 0.01, s.b / 2.0));
        v.push(plate(add(m.a, mul(u, s1 - t / 2.0)), u, t, s.h / 2.0 + 0.01, s.b / 2.0));
    }
    let mut out = section(m, cat, s0, s1);
    out.extend(v);
    out
}

/// The member's weight (kg), when its section is in the table.
pub fn weight(m: &Member, cat: &Catalogue) -> Option<f64> {
    cat.steel_section(&m.section).map(|s| s.area * DENSITY * length(m))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let (lo, hi) = bounds(&section(&beam, &cat, 0.0, 6.0));
        assert!((hi[2] - lo[2] - s.h).abs() < 1e-9 && (hi[0] - lo[0] - s.b).abs() < 1e-9 && (hi[1] - lo[1] - 6.0).abs() < 1e-9, "{lo:?} {hi:?}");
        // Rolled 90°: on its side.
        let side = Member { roll: 90.0, ..beam.clone() };
        let (lo, hi) = bounds(&section(&side, &cat, 0.0, 6.0));
        assert!((hi[2] - lo[2] - s.b).abs() < 1e-9 && (hi[0] - lo[0] - s.h).abs() < 1e-9);
        // A column: depth east, width north, from its base to its top.
        let col = Member { role: MemberRole::Column, section: "IPE 300".into(), a: [2.0, 2.0, 0.0], b: [2.0, 2.0, 4.0], ..Default::default() };
        let (lo, hi) = bounds(&section(&col, &cat, 0.0, 4.0));
        assert!((hi[0] - lo[0] - s.h).abs() < 1e-9 && (hi[1] - lo[1] - s.b).abs() < 1e-9 && lo[2].abs() < 1e-9 && (hi[2] - 4.0).abs() < 1e-9);
        // Weight: area × density × length.
        assert!((weight(&col, &cat).unwrap() - s.area * DENSITY * 4.0).abs() < 1e-9);
        // A section not in the table is still drawn (as a box) and has no weight.
        let odd = Member { section: "W 99x9".into(), ..col };
        assert_eq!(shapes(&odd, &[], &cat).len(), 1);
        assert!(weight(&odd, &cat).is_none());
    }

    #[test]
    fn a_beam_stops_at_the_column_faces_and_columns_stand_on_base_plates() {
        let cat = Catalogue::builtin();
        let c = cat.steel_section("HEB 200").unwrap().clone();
        let c1 = Member { id: 1, role: MemberRole::Column, section: "HEB 200".into(), a: [0.0, 0.0, 0.0], b: [0.0, 0.0, 3.0], ..Default::default() };
        let c2 = Member { id: 2, a: [6.0, 0.0, 0.0], b: [6.0, 0.0, 3.0], ..c1.clone() };
        let bm = Member { id: 3, role: MemberRole::Beam, section: "IPE 300".into(), a: [0.0, 0.0, 3.0], b: [6.0, 0.0, 3.0], ..Default::default() };
        let all = vec![c1.clone(), c2, bm.clone()];
        // The beam runs east, into the columns' depth (east): it stops half a depth short of each axis.
        let (lo, hi) = bounds(&shapes(&bm, &all, &cat));
        assert!((lo[0] - c.h / 2.0).abs() < 1e-9 && (hi[0] - (6.0 - c.h / 2.0)).abs() < 1e-9, "{lo:?} {hi:?}");
        // Alone, it runs axis to axis.
        let (lo, hi) = bounds(&shapes(&bm, &[], &cat));
        assert!(lo[0].abs() < 1e-9 && (hi[0] - 6.0).abs() < 1e-9);
        // The column's base plate is wider than the section and on the ground; anchor bolts above it.
        let (lo, hi) = bounds(&shapes(&c1, &all, &cat));
        assert!(lo[2].abs() < 1e-9 && (hi[2] - 3.0).abs() < 1e-9 && hi[0] - lo[0] > c.h + 0.1);
    }

    fn bounds(v: &[Shape]) -> (P3, P3) {
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for s in v { let (a, b) = s.aabb(); for k in 0..3 { lo[k] = lo[k].min(a[k]); hi[k] = hi[k].max(b[k]); } }
        (lo, hi)
    }
}
