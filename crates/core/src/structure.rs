//! Free-standing platforms and stairs (0.3.8), and the access check: a valve's handwheel or lever
//! within reach of where an operator stands (grade, a platform, or a platform on equipment).
//! A platform (a "deck" in the code, to keep it apart from equipment platforms) is a rectangle of
//! grating at a floor elevation on legs, with edge beams, joists, toe plates and handrails; a
//! stair runs straight from its foot to its top, with stringers, treads and handrails.

use crate::catalogue::Catalogue;
use crate::model::{Deck, Member, MemberRole, Project, Stair, P3};
use crate::parts::{add, cross, mul, norm, sub, unit, Shape};

/// Grating thickness: the floor is its top; the steel's top is this far below.
pub const GRATING: f64 = 0.03;
/// How high above where an operator stands a handwheel or lever can be (m).
pub const REACH_UP: f64 = 2.0;
/// How far out from a floor's edge an operator reaches (m).
pub const REACH_OUT: f64 = 0.6;

/// The deck's plan rectangle: (min x, min y, max x, max y).
pub fn rect(d: &Deck) -> (f64, f64, f64, f64) {
    (d.a[0].min(d.b[0]), d.a[1].min(d.b[1]), d.a[0].max(d.b[0]), d.a[1].max(d.b[1]))
}

/// Positions from `lo` to `hi` at most `step` apart, ends included.
fn stations(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    let n = (((hi - lo) / step).ceil() as usize).max(1);
    (0..=n).map(|k| lo + (hi - lo) * k as f64 / n as f64).collect()
}

/// The deck's steel: legs at the corners and at most 6 m apart along its edges, edge beams, and
/// joists across its shorter side at most 1.5 m apart (the grating's span).
pub fn deck_members(d: &Deck, cat: &Catalogue) -> Vec<Member> {
    let (x0, y0, x1, y1) = rect(d);
    let z0 = d.a[2];
    let hb = cat.steel_section(&d.beam).map(|s| s.h).unwrap_or(0.2);
    let zb = d.el - GRATING - hb / 2.0;
    let mut out: Vec<Member> = vec![];
    let push = |out: &mut Vec<Member>, role: MemberRole, section: &str, a: P3, b: P3| {
        let id = out.len() as u32 + 1;
        out.push(Member { id, tag: format!("{}/{}", d.tag, id), role, section: section.into(), a, b, ..Default::default() });
    };
    let xs = stations(x0, x1, 6.0);
    let ys = stations(y0, y1, 6.0);
    let mut legs: Vec<[f64; 2]> = vec![];
    for x in &xs { legs.push([*x, y0]); legs.push([*x, y1]); }
    for y in &ys[1..ys.len() - 1] { legs.push([x0, *y]); legs.push([x1, *y]); }
    let top = d.el - GRATING;
    if top - z0 > 0.3 { for l in &legs { push(&mut out, MemberRole::Column, &d.leg, [l[0], l[1], z0], [l[0], l[1], top]); } }
    for w in xs.windows(2) { for y in [y0, y1] { push(&mut out, MemberRole::Beam, &d.beam, [w[0], y, zb], [w[1], y, zb]); } }
    for w in ys.windows(2) { for x in [x0, x1] { push(&mut out, MemberRole::Beam, &d.beam, [x, w[0], zb], [x, w[1], zb]); } }
    // Joists across the shorter side.
    let along_x = x1 - x0 >= y1 - y0;
    let js = if along_x { stations(x0, x1, 1.5) } else { stations(y0, y1, 1.5) };
    for j in &js[1..js.len() - 1] {
        if along_x { push(&mut out, MemberRole::Beam, &d.beam, [*j, y0, zb], [*j, y1, zb]); } else { push(&mut out, MemberRole::Beam, &d.beam, [x0, *j, zb], [x1, *j, zb]); }
    }
    out
}

/// The deck as built: its steel, the grating, toe plates and handrails with posts.
pub fn deck_shapes(d: &Deck, cat: &Catalogue) -> Vec<Shape> {
    let ms = deck_members(d, cat);
    let mut v: Vec<Shape> = ms.iter().flat_map(|m| crate::steel::shapes(m, &ms, cat)).collect();
    let (x0, y0, x1, y1) = rect(d);
    let ax = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    v.push(Shape::Cuboid { centre: [(x0 + x1) / 2.0, (y0 + y1) / 2.0, d.el - GRATING / 2.0], axes: ax, half: [(x1 - x0) / 2.0, (y1 - y0) / 2.0, GRATING / 2.0] });
    let corners = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
    for k in 0..4 {
        let (a, b) = (corners[k], corners[(k + 1) % 4]);
        let (pa, pb) = ([a[0], a[1], d.el], [b[0], b[1], d.el]);
        let len = norm(sub(pb, pa));
        let u = unit(sub(pb, pa));
        let n = cross([0.0, 0.0, 1.0], u);
        let mid = mul(add(pa, pb), 0.5);
        v.push(Shape::Cuboid { centre: add(mid, [0.0, 0.0, 0.05]), axes: [u, n, [0.0, 0.0, 1.0]], half: [len / 2.0, 0.004, 0.05] });
        for h in [1.1, 0.55] { v.push(Shape::tube(add(pa, [0.0, 0.0, h]), add(pb, [0.0, 0.0, h]), 0.022)); }
        for s in stations(0.0, len, 1.8) { v.push(Shape::tube(add(pa, mul(u, s)), add(add(pa, mul(u, s)), [0.0, 0.0, 1.1]), 0.022)); }
    }
    v
}

/// A stair's plan direction, plan run, rise, and slope (degrees).
pub fn stair_geom(s: &Stair) -> (P3, f64, f64, f64) {
    let d = [s.b[0] - s.a[0], s.b[1] - s.a[1], 0.0];
    let run = norm(d);
    let rise = s.b[2] - s.a[2];
    let u = if run > 1e-9 { mul(d, 1.0 / run) } else { [1.0, 0.0, 0.0] };
    (u, run, rise, rise.atan2(run).to_degrees())
}

/// The stair as built: two stringers, treads about 180 mm apart in height (the top one is the
/// landing it arrives at), and a handrail with posts each side.
pub fn stair_shapes(s: &Stair) -> Vec<Shape> {
    let (u, run, rise, _) = stair_geom(s);
    if run < 0.1 || rise < 0.1 { return vec![]; }
    let z: P3 = [0.0, 0.0, 1.0];
    let side = cross(z, u);
    let slope = unit(sub(s.b, s.a));
    let up = cross(slope, side);
    let len = norm(sub(s.b, s.a));
    let w = s.width;
    let mut v = vec![];
    let mid = mul(add(s.a, s.b), 0.5);
    for sg in [-1.0, 1.0] {
        v.push(Shape::Cuboid { centre: add(add(mid, mul(side, sg * (w / 2.0 + 0.006))), mul(up, -0.04)), axes: [slope, side, up], half: [len / 2.0, 0.006, 0.1] });
        let rail = |t: f64| add(add(add(s.a, mul(sub(s.b, s.a), t)), mul(side, sg * (w / 2.0 + 0.03))), [0.0, 0.0, 0.9]);
        v.push(Shape::tube(rail(0.0), rail(1.0), 0.022));
        for t in [0.0, 0.5, 1.0] { v.push(Shape::tube(sub(rail(t), [0.0, 0.0, 0.9]), rail(t), 0.022)); }
    }
    let n = ((rise / 0.18).round() as usize).max(1);
    let (g, r) = (run / n as f64, rise / n as f64);
    for k in 1..n {
        let c = add(add(s.a, mul(u, (k as f64 - 0.5) * g)), [0.0, 0.0, k as f64 * r - 0.015]);
        v.push(Shape::Cuboid { centre: c, axes: [u, side, z], half: [g / 2.0 + 0.015, w / 2.0, 0.015] });
    }
    v
}

/// A floor an operator stands on: grade, a deck, a stair's flights, an equipment platform.
enum Floor { Grade(f64), Rect(f64, f64, f64, f64, f64), Ring { c: P3, rot: f64, r0: f64, r1: f64, from: f64, to: f64, el: f64 } }

fn floors(p: &Project) -> Vec<Floor> {
    let mut f = vec![Floor::Grade(p.plot.ground)];
    for d in &p.decks { let (x0, y0, x1, y1) = rect(d); f.push(Floor::Rect(x0, y0, x1, y1, d.el)); }
    for e in &p.equipment {
        let Some(r) = crate::equipment::shell_radius(e) else { continue };
        for pl in &e.platforms { f.push(Floor::Ring { c: e.pos, rot: e.rot, r0: r + 0.05, r1: r + 0.05 + pl.width, from: pl.from, to: pl.to, el: e.pos[2] + pl.el }); }
    }
    f
}

/// How far above the floor under it (within reach in plan) a point is: the smallest height over
/// any floor at or below it.
pub fn above_floor(p: &Project, at: P3) -> f64 {
    let mut best = f64::INFINITY;
    for f in floors(p) {
        let el = match f {
            Floor::Grade(z) => Some(z),
            Floor::Rect(x0, y0, x1, y1, z) => {
                let dx = (x0 - at[0]).max(at[0] - x1).max(0.0);
                let dy = (y0 - at[1]).max(at[1] - y1).max(0.0);
                (dx.hypot(dy) <= REACH_OUT).then_some(z)
            }
            Floor::Ring { c, rot, r0, r1, from, to, el } => {
                let (dx, dy) = (at[0] - c[0], at[1] - c[1]);
                let rr = dx.hypot(dy);
                let a = (dy.atan2(dx).to_degrees() - rot).rem_euclid(360.0);
                let slack = (REACH_OUT / rr.max(0.1)).to_degrees();
                let span = to - from;
                let rel = (a - from).rem_euclid(360.0);
                let on_arc = span >= 360.0 - 1e-6 || rel <= span + slack || rel >= 360.0 - slack;
                (rr >= r0 - REACH_OUT && rr <= r1 + REACH_OUT && on_arc).then_some(el)
            }
        };
        if let Some(z) = el { if z <= at[2] + 1e-9 { best = best.min(at[2] - z); } }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_deck_stands_on_legs_with_its_floor_at_its_elevation() {
        let cat = Catalogue::builtin();
        let d = Deck { id: 1, tag: "PF-1".into(), a: [0.0, 0.0, 0.0], b: [8.0, 3.0, 0.0], el: 2.4, ..Default::default() };
        let m = deck_members(&d, &cat);
        // Legs at x 0, 4, 8 on both long sides; edge beams; joists at most 1.5 m apart.
        assert_eq!(m.iter().filter(|x| x.role == MemberRole::Column).count(), 6);
        assert!(m.iter().filter(|x| x.role == MemberRole::Column).all(|c| c.a[2] == 0.0 && (c.b[2] - (2.4 - GRATING)).abs() < 1e-9));
        let joists = m.iter().filter(|x| x.role == MemberRole::Beam && (x.a[1] - x.b[1]).abs() > 1e-9 && x.a[0] > 0.0 && x.a[0] < 8.0).count();
        assert_eq!(joists, 5, "joists across, 1.33 m apart");
        let (mut hi, mut lo) = (f64::NEG_INFINITY, f64::INFINITY);
        for s in deck_shapes(&d, &cat) { let (a, b) = s.aabb(); hi = hi.max(b[2]); lo = lo.min(a[2]); }
        assert!(lo.abs() < 1e-9 && (hi - (2.4 + 1.1 + 0.022)).abs() < 1e-6);
    }

    #[test]
    fn a_stair_rises_from_its_foot_to_its_top_in_even_steps() {
        let s = Stair { id: 1, tag: "ST-1".into(), a: [0.0, 0.0, 0.0], b: [3.0, 0.0, 2.7], width: 0.9, ..Default::default() };
        let (_, run, rise, slope) = stair_geom(&s);
        assert!((run - 3.0).abs() < 1e-12 && (rise - 2.7).abs() < 1e-12 && (slope - 41.99).abs() < 0.01);
        // 15 risers of 180 mm: 14 treads (the 15th step is the landing).
        let treads = stair_shapes(&s).iter().filter(|x| matches!(x, Shape::Cuboid { half, .. } if (half[2] - 0.015).abs() < 1e-12)).count();
        assert_eq!(treads, 14);
    }

    #[test]
    fn a_handwheel_is_measured_from_the_floor_under_it() {
        let mut p = Project::new();
        assert!((above_floor(&p, [5.0, 5.0, 3.0]) - 3.0).abs() < 1e-12);
        p.decks.push(Deck { id: 1, a: [4.0, 4.0, 0.0], b: [6.0, 6.0, 0.0], el: 1.5, ..Default::default() });
        // Over the deck, and just off its edge (within reach), from its floor; far off, from grade.
        assert!((above_floor(&p, [5.0, 5.0, 3.0]) - 1.5).abs() < 1e-12);
        assert!((above_floor(&p, [6.5, 5.0, 3.0]) - 1.5).abs() < 1e-12);
        assert!((above_floor(&p, [8.0, 5.0, 3.0]) - 3.0).abs() < 1e-12);
        // Under the deck, the floor is grade.
        assert!((above_floor(&p, [5.0, 5.0, 1.0]) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn the_exchangers_valves_are_in_reach_from_its_platforms_and_not_without_them() {
        let cat = Catalogue::builtin();
        let mut p = crate::examples::get("exchanger").unwrap();
        let reach = |p: &Project| crate::checks::access(p, &cat, &crate::route::build(p, &cat)).into_iter().filter(|i| i.text.contains("out of reach")).count();
        assert_eq!(reach(&p), 0);
        p.decks.clear();
        assert_eq!(reach(&p), 3, "the inlet gate, the bypass globe and the cooling water gate");
        // A stair steeper than 45° is reported.
        p.stairs[0].a[0] = p.stairs[0].b[0] - 1.0;
        let b = crate::route::build(&p, &cat);
        assert!(crate::checks::access(&p, &cat, &b).iter().any(|i| i.text.contains("rises at")));
    }

    #[test]
    fn steel_is_weighed_by_section_and_clashes_with_pipe_and_equipment() {
        let cat = Catalogue::builtin();
        let mut p = crate::examples::get("rack").unwrap();
        let rows = crate::lists::steel(&p, &cat);
        // The rack's columns (HEB 300), beams and struts, the platforms' legs and beams.
        let heb = rows.iter().find(|r| r.section == "HEB 300").unwrap();
        assert_eq!(heb.count, 8);
        let s = cat.steel_section("HEB 300").unwrap();
        assert!((heb.kg.unwrap() - s.area * crate::steel::DENSITY * heb.length).abs() < 1e-6);
        assert!(heb.from.contains("PR-1") && rows.iter().any(|r| r.from.contains("PF-1")));
        let csv = crate::lists::csv(&crate::lists::lists(&p, &cat), "steel").unwrap();
        assert!(csv.starts_with("Section,Family,Members") && csv.contains("HEB 300") && csv.lines().last().unwrap().starts_with("Total"));
        // A beam put through line A's run and into V-302 is found against both.
        p.steel.push(Member { id: 9001, tag: "BM-9".into(), role: MemberRole::Beam, section: "IPE 300".into(), a: [20.0, 10.0, 3.0], b: [20.0, 10.0, 6.0], ..Default::default() });
        p.steel.push(Member { id: 9002, tag: "BM-10".into(), role: MemberRole::Beam, section: "IPE 300".into(), a: [24.0, 10.0, 3.0], b: [26.0, 10.0, 3.0], ..Default::default() });
        let b = crate::route::build(&p, &cat);
        let cl: Vec<String> = crate::checks::clashes(&p, &b).into_iter().map(|i| i.text).collect();
        assert!(cl.iter().any(|t| t.starts_with("Pipe–steel") && t.contains("6\"-P-3001") && t.contains("BM-9")), "{cl:?}");
        assert!(cl.iter().any(|t| t.starts_with("Equipment–steel") && t.contains("V-302") && t.contains("BM-10")), "{cl:?}");
        // Supports on the rack, and the rack's own members, raise nothing.
        assert!(!cl.iter().any(|t| t.contains("support") || t.contains("pipe rack PR-1")), "{cl:?}");
    }
}
