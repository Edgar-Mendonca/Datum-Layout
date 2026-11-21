//! Pipe racks (0.3.5): one object a designer places with two clicks, from which the engine makes
//! the bents (two columns and a beam at each tier), the struts along each side at each tier, and
//! bracing in the end bays. The members are derived, never stored, so changing the spacing, width
//! or tiers rebuilds the rack. Tiers are elevations of the top of steel; a line routed over a rack
//! near a tier is put on it, its shoe standing on the beam.

use crate::catalogue::Catalogue;
use crate::model::{Member, MemberRole, Rack, P3};
use crate::parts::{add, mul, norm, sub};

/// A shoe's depth under a pipe of radius `rp` insulated to `ri`: clear of the insulation by
/// 50 mm, at least 100 mm (the support drawing uses the same).
pub fn shoe_depth(rp: f64, ri: f64) -> f64 {
    (ri - rp + 0.05).max(0.1)
}

/// The pipe centreline on a tier whose top of steel is `tos`.
pub fn on_tier(tos: f64, od: f64, insulation: f64) -> f64 {
    tos + shoe_depth(od / 2.0, od / 2.0 + insulation) + od / 2.0
}

/// The top of steel under a pipe whose centreline is at `z`.
pub fn tier_under(z: f64, od: f64, insulation: f64) -> f64 {
    z - shoe_depth(od / 2.0, od / 2.0 + insulation) - od / 2.0
}

/// The rack's direction along its length (in plan), across it, and its plan length.
pub fn axes(r: &Rack) -> (P3, P3, f64) {
    let d = [r.b[0] - r.a[0], r.b[1] - r.a[1], 0.0];
    let l = norm(d);
    let u = if l > 1e-9 { mul(d, 1.0 / l) } else { [1.0, 0.0, 0.0] };
    (u, [-u[1], u[0], 0.0], l)
}

/// The number of bays and the bay length: the spacing asked for, evened out over the length.
pub fn bays(r: &Rack) -> (usize, f64) {
    let (_, _, l) = axes(r);
    let n = ((l / r.spacing.max(0.5)).round() as usize).max(1);
    (n, l / n as f64)
}

/// The rack's members: columns at every bent, a beam across at every tier, struts along both
/// sides at every tier, and a brace on each side of the end bays when the rack is braced.
pub fn members(r: &Rack, cat: &Catalogue) -> Vec<Member> {
    let (u, v, _) = axes(r);
    let (n, s) = bays(r);
    let z0 = r.a[2];
    let top = r.tiers.iter().cloned().fold(z0 + 0.5, f64::max);
    let h = |name: &str| cat.steel_section(name).map(|x| x.h).unwrap_or(0.3);
    let roll = v[1].atan2(v[0]).to_degrees();
    let mut out: Vec<Member> = vec![];
    let push = |out: &mut Vec<Member>, role: MemberRole, section: &str, a: P3, b: P3, roll: f64| {
        let id = out.len() as u32 + 1;
        out.push(Member { id, tag: format!("{}/{}", r.tag, id), role, section: section.into(), a, b, roll, ..Default::default() });
    };
    let at = |k: usize, side: f64, z: f64| -> P3 { let p = add(add(r.a, mul(u, k as f64 * s)), mul(v, side * r.width / 2.0)); [p[0], p[1], z] };
    for k in 0..=n {
        for side in [-1.0, 1.0] { push(&mut out, MemberRole::Column, &r.column, at(k, side, z0), at(k, side, top), roll); }
        for t in &r.tiers { let z = t - h(&r.beam) / 2.0; push(&mut out, MemberRole::Beam, &r.beam, at(k, -1.0, z), at(k, 1.0, z), 0.0); }
    }
    for t in &r.tiers {
        let z = t - h(&r.strut) / 2.0;
        for side in [-1.0, 1.0] { for k in 0..n { push(&mut out, MemberRole::Beam, &r.strut, at(k, side, z), at(k + 1, side, z), 0.0); } }
    }
    if r.braced {
        let low = r.tiers.iter().cloned().fold(f64::INFINITY, f64::min);
        let zt = if low.is_finite() { low - h(&r.strut) / 2.0 } else { top };
        let mut ends = vec![0];
        if n > 1 { ends.push(n - 1); }
        for k in ends { for side in [-1.0, 1.0] { push(&mut out, MemberRole::Brace, &r.brace, at(k, side, z0), at(k + 1, side, zt), 0.0); } }
    }
    out
}

/// Whether a point lies over the rack in plan (within its width and length).
pub fn over(r: &Rack, p: P3) -> bool {
    let (u, v, l) = axes(r);
    let d = sub(p, r.a);
    let (x, y) = (d[0] * u[0] + d[1] * u[1], d[0] * v[0] + d[1] * v[1]);
    (-0.05..=l + 0.05).contains(&x) && y.abs() <= r.width / 2.0 + 0.05
}

/// The elevation a pipe's centreline should take at `p`: on the nearest tier of a rack it is over,
/// when it is within 0.75 m of it.
pub fn snap(racks: &[Rack], p: P3, od: f64, insulation: f64) -> Option<f64> {
    let mut best: Option<(f64, f64)> = None;
    for r in racks.iter().filter(|r| over(r, p)) {
        for t in &r.tiers {
            let z = on_tier(*t, od, insulation);
            let d = (z - p[2]).abs();
            if d <= 0.75 && best.is_none_or(|(bd, _)| d < bd) { best = Some((d, z)); }
        }
    }
    best.map(|(_, z)| z)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rack() -> Rack {
        Rack { id: 1, tag: "PR-1".into(), a: [0.0, 0.0, 0.0], b: [18.0, 0.0, 0.0], width: 6.0, spacing: 6.0, tiers: vec![5.0, 6.5], ..Default::default() }
    }

    #[test]
    fn a_rack_is_bents_tiers_struts_and_end_bracing() {
        let cat = Catalogue::builtin();
        let r = rack();
        assert_eq!(bays(&r), (3, 6.0));
        let m = members(&r, &cat);
        let count = |role| m.iter().filter(|x| x.role == role).count();
        // 4 bents of 2 columns; a beam per tier per bent, and 2 sides × 3 bays of struts per tier.
        assert_eq!(count(MemberRole::Column), 8);
        assert_eq!(count(MemberRole::Beam), 4 * 2 + 2 * 3 * 2);
        assert_eq!(count(MemberRole::Brace), 4);
        // Columns from grade to the top tier, 3 m either side of the axis; beam tops at the tiers.
        assert!(m.iter().filter(|x| x.role == MemberRole::Column).all(|c| c.a[2] == 0.0 && c.b[2] == 6.5 && c.a[1].abs() == 3.0));
        let hb = cat.steel_section(&r.beam).unwrap().h;
        assert!(m.iter().filter(|x| x.role == MemberRole::Beam && x.a[1] != x.b[1]).all(|b| [5.0, 6.5].iter().any(|t| (b.a[2] + hb / 2.0 - t).abs() < 1e-9)));
        // Every member is drawn (its sections are in the table).
        assert!(m.iter().all(|x| cat.steel_section(&x.section).is_some()));
        // Spacing evens out: 20 m at 6 m is three bays of 6.67 m.
        let r2 = Rack { b: [20.0, 0.0, 0.0], ..rack() };
        assert_eq!(bays(&r2).0, 3);
    }

    #[test]
    fn a_line_over_a_rack_snaps_onto_the_nearest_tier() {
        let racks = vec![rack()];
        let od = 0.1683;
        let z = snap(&racks, [9.0, 1.0, 5.3], od, 0.0).unwrap();
        assert!((tier_under(z, od, 0.0) - 5.0).abs() < 1e-12 && (z - (5.0 + 0.1 + od / 2.0)).abs() < 1e-12);
        // Insulated: the shoe is deeper.
        assert!((snap(&racks, [9.0, 1.0, 5.3], od, 0.1).unwrap() - (5.0 + 0.15 + od / 2.0)).abs() < 1e-12);
        // Off the rack in plan, or far from every tier: no snap.
        assert!(snap(&racks, [9.0, 4.0, 5.3], od, 0.0).is_none());
        assert!(snap(&racks, [9.0, 1.0, 3.0], od, 0.0).is_none());
    }
}
