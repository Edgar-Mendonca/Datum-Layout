//! The checks: clashes (pipe–pipe, pipe–equipment, pipe–ground, with a clearance), open ends,
//! size, rating and end-type mismatches where parts meet, components the line's class does not
//! allow, and lines without a number. Parts meet where their ports coincide (to 0.01 mm).
//! Clashes use parry3d: each shape becomes a convex hull, pairs are found by sweeping bounding
//! boxes and measured with exact convex distance.

use crate::catalogue::Catalogue;
use crate::model::{Project, P3};
use crate::parts::{add, dist, dot, mul, rotate, unit, Shape};
use crate::route::{Built, Issue, Part};
use parry3d_f64::math::{Pose, Vector};
use parry3d_f64::shape::{ConvexPolyhedron, Cuboid};
use std::collections::{HashMap, HashSet};

/// Ports that meet: (part, port) → (part, port), both ways.
pub fn connections(b: &Built) -> HashMap<(usize, usize), (usize, usize)> {
    const Q: f64 = 1e-5;
    let key = |p: P3| [(p[0] / Q).round() as i64, (p[1] / Q).round() as i64, (p[2] / Q).round() as i64];
    let mut grid: HashMap<[i64; 3], Vec<(usize, usize)>> = HashMap::new();
    for (i, q) in b.parts.iter().enumerate() {
        for (j, pt) in q.ports.iter().enumerate() { grid.entry(key(pt.p)).or_default().push((i, j)); }
    }
    let mut out = HashMap::new();
    for (i, q) in b.parts.iter().enumerate() {
        for (j, pt) in q.ports.iter().enumerate() {
            if out.contains_key(&(i, j)) { continue; }
            let k = key(pt.p);
            'find: for dx in -1..=1 { for dy in -1..=1 { for dz in -1..=1 {
                if let Some(v) = grid.get(&[k[0] + dx, k[1] + dy, k[2] + dz]) {
                    for &(a, c) in v {
                        if a == i || out.contains_key(&(a, c)) { continue; }
                        if dist(b.parts[a].ports[c].p, pt.p) <= Q {
                            out.insert((i, j), (a, c));
                            out.insert((a, c), (i, j));
                            break 'find;
                        }
                    }
                }
            } } }
        }
    }
    out
}

fn kind_name(kind: &str) -> &str {
    match kind {
        "elbow90" => "90° elbow", "elbow45" => "45° elbow", "flange" => "flange", "gasket" => "gasket", "pipe" => "pipe", "tee" => "tee", "reducer" => "reducer",
        "cap" => "cap", "blind" => "blind flange", "gate" => "gate valve", "globe" => "globe valve", "check" => "check valve", "ball" => "ball valve", "strainer" => "strainer",
        "nozzle" => "nozzle", "equipment" => "", k => k,
    }
}

fn label(p: &Project, q: &Part) -> String {
    let what = kind_name(q.kind);
    if q.line != 0 {
        let n = p.line(q.line).map(|l| if l.number.is_empty() { "(no number)".to_string() } else { l.number.clone() }).unwrap_or_default();
        format!("{n} {what}")
    } else {
        let t = p.equip(q.equip).map(|e| e.tag.clone()).unwrap_or_default();
        if q.kind == "nozzle" { format!("{t} nozzle {}", q.note) } else { t }
    }
}

fn mm(v: f64) -> String { format!("{:.0} mm", v * 1000.0) }
fn at(p: P3) -> String { format!("E {:.3} N {:.3} EL {:.3}", p[0], p[1], p[2]) }

/// Every check's findings (the routing issues first).
pub fn run(p: &Project, cat: &Catalogue, b: &Built) -> Vec<Issue> {
    let mut out = b.issues.clone();
    out.extend(connection_issues(p, cat, b));
    out.extend(clashes(p, b));
    out
}

pub fn connection_issues(p: &Project, cat: &Catalogue, b: &Built) -> Vec<Issue> {
    let conn = connections(b);
    let mut out = vec![];
    let iss = |kind, text: String, at: P3, q: &Part| Issue { kind, text, at, line: q.line, item: q.item, equip: q.equip };
    for (i, q) in b.parts.iter().enumerate() {
        for (j, pt) in q.ports.iter().enumerate() {
            match conn.get(&(i, j)) {
                None if q.line != 0 => out.push(iss("open", format!("{}: open end at {}. Connect it, or close it with a cap or a blind flange.", label(p, q), at(pt.p)), pt.p, q)),
                Some(&(a, c)) if (a, c) > (i, j) => {
                    let (r, rp) = (&b.parts[a], &b.parts[a].ports[c]);
                    if pt.nps != rp.nps {
                        out.push(iss("mismatch", format!("Size mismatch: {} ({}\") meets {} ({}\") at {}.", label(p, q), pt.nps, label(p, r), rp.nps, at(pt.p)), pt.p, if q.line != 0 { q } else { r }));
                    } else if pt.end != rp.end {
                        out.push(iss("mismatch", format!("End mismatch: a weld end meets a flange face where {} meets {}.", label(p, q), label(p, r)), pt.p, if q.line != 0 { q } else { r }));
                    } else if pt.end == "FL" && pt.rating != rp.rating {
                        out.push(iss("mismatch", format!("Rating mismatch: class {} meets class {} where {} meets {}.", pt.rating, rp.rating, label(p, q), label(p, r)), pt.p, if q.line != 0 { q } else { r }));
                    } else if q.line != 0 && r.line != 0 && q.line != r.line {
                        let (lc, rc) = (p.line(q.line).map(|l| l.class.clone()), p.line(r.line).map(|l| l.class.clone()));
                        if lc != rc { out.push(iss("mismatch", format!("Class mismatch: {} ({}) meets {} ({}).", label(p, q), lc.unwrap_or_default(), label(p, r), rc.unwrap_or_default()), pt.p, q)); }
                    }
                    let _ = dot(pt.d, rp.d);
                }
                _ => {}
            }
        }
    }
    // Class rules: each component allowed in its size; the line's rating in the class.
    let mut said = HashSet::new();
    for q in b.parts.iter().filter(|q| q.line != 0) {
        let Some(l) = p.line(q.line) else { continue };
        let Some(class) = cat.class(&l.class) else { continue };
        if !class.ratings.contains(&l.rating) && said.insert((l.id, 0, "rating".to_string())) {
            out.push(Issue { kind: "class", text: format!("Line {}: class {} has no class {} flanges.", l.number, l.class, l.rating), at: l.pts[0], line: l.id, item: 0, equip: 0 });
        }
        if !class.allows(q.kind, &q.nps) && said.insert((l.id, q.item, format!("{}{}", q.kind, q.nps))) {
            let pt = q.ports.first().map(|x| x.p).unwrap_or_default();
            out.push(Issue { kind: "class", text: format!("{}: a {}\" {} is not allowed by class {}.", if l.number.is_empty() { "Line without a number" } else { &l.number }, q.nps, kind_name(q.kind), l.class), at: pt, line: l.id, item: q.item, equip: 0 });
        }
    }
    for l in p.lines.iter().filter(|l| l.number.trim().is_empty()) {
        out.push(Issue { kind: "number", text: format!("A {}\" line has no line number.", l.nps), at: l.pts.first().copied().unwrap_or_default(), line: l.id, item: 0, equip: 0 });
    }
    out
}

/// Convex pieces of a shape (a torus is cut into chunks of at most 15°).
fn hulls(s: &Shape) -> Vec<Vec<Vector>> {
    let v = |p: P3| Vector::new(p[0], p[1], p[2]);
    let ring = |c: P3, d: P3, r: f64| -> Vec<Vector> {
        let (a, b) = crate::parts::frame(d);
        (0..16).map(|i| { let t = std::f64::consts::TAU * i as f64 / 16.0; v(add(c, add(mul(a, r * t.cos()), mul(b, r * t.sin())))) }).collect()
    };
    match s {
        Shape::Frustum { a, b, r1, r2 } => { let d = unit(crate::parts::sub(*b, *a)); let mut h = ring(*a, d, *r1); h.extend(ring(*b, d, *r2)); vec![h] }
        Shape::Torus { centre, axis, start, angle, r } => {
            let k = unit(*axis);
            let n = ((angle.abs() / 15f64.to_radians()).ceil() as usize).max(1);
            (0..n).map(|i| {
                let mut h = vec![];
                for t in [angle * i as f64 / n as f64, angle * (i + 1) as f64 / n as f64] {
                    let rad = rotate(*start, k, t);
                    h.extend(ring(add(*centre, rad), unit(crate::parts::cross(k, rad)), *r));
                }
                h
            }).collect()
        }
        Shape::Head { base, dir, r, depth } => {
            let d = unit(*dir);
            let mut h = ring(*base, d, *r);
            for t in 1..=4 { let a = std::f64::consts::FRAC_PI_2 * t as f64 / 4.0; h.extend(ring(add(*base, mul(d, depth * a.sin())), d, (r * a.cos()).max(1e-4))); }
            vec![h]
        }
        Shape::Cuboid { .. } => vec![],
    }
}

enum Solid { Hull(ConvexPolyhedron), Box(Cuboid, Pose) }

fn solids(s: &Shape) -> Vec<Solid> {
    match s {
        Shape::Cuboid { centre, axes, half } => {
            let m = parry3d_f64::math::Matrix::from_cols(Vector::new(axes[0][0], axes[0][1], axes[0][2]), Vector::new(axes[1][0], axes[1][1], axes[1][2]), Vector::new(axes[2][0], axes[2][1], axes[2][2]));
            let rot = parry3d_f64::math::Rotation::from_mat3(&m);
            let pose = Pose::from_parts(Vector::new(centre[0], centre[1], centre[2]), rot);
            vec![Solid::Box(Cuboid::new(Vector::new(half[0], half[1], half[2])), pose)]
        }
        _ => hulls(s).into_iter().filter_map(|h| ConvexPolyhedron::from_convex_hull(&h).map(Solid::Hull)).collect(),
    }
}

fn gap(a: &Solid, b: &Solid) -> f64 {
    let id = Pose::IDENTITY;
    let (pa, sa): (&Pose, &dyn parry3d_f64::shape::Shape) = match a { Solid::Hull(h) => (&id, h), Solid::Box(c, p) => (p, c) };
    let (pb, sb): (&Pose, &dyn parry3d_f64::shape::Shape) = match b { Solid::Hull(h) => (&id, h), Solid::Box(c, p) => (p, c) };
    parry3d_f64::query::distance(pa, sa, pb, sb).map(|d| d.distance).unwrap_or(f64::INFINITY)
}

/// Parts that may touch: the same part, parts joined port to port or through one part between
/// them (around a tight elbow), a nozzle and its own equipment, and the first parts of a line
/// and the equipment whose nozzle it starts at.
fn near_in_graph(b: &Built) -> Vec<HashSet<usize>> {
    let conn = connections(b);
    let n = b.parts.len();
    let mut adj = vec![HashSet::new(); n];
    for (&(i, _), &(a, _)) in &conn { adj[i].insert(a); }
    // Parts of one component (a valve with its flanges) count as joined.
    let mut by_item: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (i, q) in b.parts.iter().enumerate() { if q.item != 0 { by_item.entry((q.line, q.item)).or_default().push(i); } }
    for v in by_item.values() { for &i in v { for &j in v { if i != j { adj[i].insert(j); } } } }
    let mut near = vec![HashSet::new(); n];
    for i in 0..n {
        near[i].insert(i);
        for &a in &adj[i] { near[i].insert(a); for &c in &adj[a] { near[i].insert(c); for &d in &adj[c] { if b.parts[i].kind == "nozzle" || b.parts[d].kind == "nozzle" { near[i].insert(d); } } } }
    }
    // A nozzle's neighbours may touch its equipment.
    let body: HashMap<u32, usize> = b.parts.iter().enumerate().filter(|(_, q)| q.kind == "equipment").map(|(i, q)| (q.equip, i)).collect();
    for i in 0..n {
        if b.parts[i].kind == "nozzle" {
            if let Some(&e) = body.get(&b.parts[i].equip) {
                let around: Vec<usize> = near[i].iter().copied().collect();
                for a in around { near[a].insert(e); near[e].insert(a); }
            }
        }
    }
    near
}

/// Clashes between pipe-like parts, and with equipment and the ground, closer than the clearance.
pub fn clashes(p: &Project, b: &Built) -> Vec<Issue> {
    let c = p.settings.clearance;
    let ground = p.plot.ground;
    let near = near_in_graph(b);
    let boxes: Vec<(P3, P3)> = b.parts.iter().map(|q| {
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for s in &q.shapes { let (a, z) = s.aabb(); for k in 0..3 { lo[k] = lo[k].min(a[k]); hi[k] = hi[k].max(z[k]); } }
        (lo, hi)
    }).collect();
    let mut out = vec![];
    // The ground: line parts below ground level plus the clearance.
    let mut seen_ground = HashSet::new();
    for (i, q) in b.parts.iter().enumerate() {
        if q.line == 0 || q.shapes.is_empty() { continue; }
        if boxes[i].0[2] < ground + c - 1e-9 && seen_ground.insert((q.line, q.item, q.seg)) {
            let mut pt = [(boxes[i].0[0] + boxes[i].1[0]) / 2.0, (boxes[i].0[1] + boxes[i].1[1]) / 2.0, ground];
            pt[2] = ground;
            out.push(Issue { kind: "clash", text: format!("Pipe–ground: {} is {} from the ground (clearance {}).", label(p, q), if boxes[i].0[2] < ground { format!("{} below", mm(ground - boxes[i].0[2])) } else { mm(boxes[i].0[2] - ground) }, mm(c)), at: pt, line: q.line, item: q.item, equip: 0 });
        }
    }
    // Pairs whose boxes come within the clearance: sweep along east.
    let mut order: Vec<usize> = (0..b.parts.len()).filter(|&i| !b.parts[i].shapes.is_empty()).collect();
    order.sort_by(|&x, &y| boxes[x].0[0].total_cmp(&boxes[y].0[0]));
    let mut solid_cache: HashMap<usize, Vec<(P3, P3, Solid)>> = HashMap::new();
    let mut best: HashMap<(u32, u32, i64, u32, u32, i64), (f64, usize, usize)> = HashMap::new();
    for (oi, &i) in order.iter().enumerate() {
        for &j in &order[oi + 1..] {
            if boxes[j].0[0] > boxes[i].1[0] + c { break; }
            let (qi, qj) = (&b.parts[i], &b.parts[j]);
            if qi.line == 0 && qj.line == 0 { continue; }
            if near[i].contains(&j) { continue; }
            if (0..3).any(|k| boxes[j].0[k] > boxes[i].1[k] + c || boxes[i].0[k] > boxes[j].1[k] + c) { continue; }
            for k in [i, j] {
                solid_cache.entry(k).or_insert_with(|| b.parts[k].shapes.iter().flat_map(|s| { let (lo, hi) = s.aabb(); solids(s).into_iter().map(move |x| (lo, hi, x)) }).collect());
            }
            let mut d = f64::INFINITY;
            for (alo, ahi, a) in &solid_cache[&i] {
                for (blo, bhi, s) in &solid_cache[&j] {
                    if (0..3).any(|k| blo[k] > ahi[k] + c || alo[k] > bhi[k] + c) { continue; }
                    d = d.min(gap(a, s));
                }
            }
            if d < c - 1e-9 {
                // One finding per pair of runs or components.
                let key = |q: &Part| (q.line, q.equip, if q.item != 0 { -(q.item as i64) } else { q.seg as i64 });
                let (ka, kb) = (key(qi), key(qj));
                let k = if ka <= kb { (ka.0, ka.1, ka.2, kb.0, kb.1, kb.2) } else { (kb.0, kb.1, kb.2, ka.0, ka.1, ka.2) };
                let e = best.entry(k).or_insert((d, i, j));
                if d < e.0 { *e = (d, i, j); }
            }
        }
    }
    let mut found: Vec<_> = best.into_values().collect();
    found.sort_by_key(|a| (a.1, a.2));
    for (d, i, j) in found {
        let (qi, qj) = (&b.parts[i], &b.parts[j]);
        let (qi, qj) = if qi.line == 0 { (qj, qi) } else { (qi, qj) };
        let kind = if qj.line == 0 { "Pipe–equipment" } else { "Pipe–pipe" };
        let (bi, bj) = (boxes[i], boxes[j]);
        let mid = |k: usize| (bi.0[k].max(bj.0[k]) + bi.1[k].min(bj.1[k])) / 2.0;
        let pt = [mid(0), mid(1), mid(2)];
        let how = if d <= 1e-6 { "overlap".to_string() } else { format!("are {} apart", mm(d)) };
        out.push(Issue { kind: "clash", text: format!("{kind}: {} and {} {how} (clearance {}).", label(p, qi), label(p, qj), mm(c)), at: pt, line: qi.line, item: qi.item, equip: qj.equip });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convex_distance_is_exact_for_two_tubes() {
        let a = solids(&Shape::tube([0.0, 0.0, 0.0], [2.0, 0.0, 0.0], 0.1));
        let b = solids(&Shape::tube([1.0, -1.0, 0.35], [1.0, 1.0, 0.35], 0.1));
        // Centre lines 0.35 apart, radii 0.1: gap 0.15 (polygonal hulls are a little inside).
        let d = gap(&a[0], &b[0]);
        assert!((d - 0.15).abs() < 0.002, "{d}");
        let c = solids(&Shape::Cuboid { centre: [1.0, 0.0, 1.0], axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], half: [0.5, 0.5, 0.5] });
        assert!((gap(&a[0], &c[0]) - 0.4).abs() < 0.002);
    }
}
