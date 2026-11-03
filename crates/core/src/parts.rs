//! Parts: shape rules and their triangle meshes, and ports. Every part Datum Layout draws is a
//! list of simple shapes (tube, cone frustum, torus segment, disc, heads, box) generated from a
//! catalogue row, with ports: the points where it connects (position, outward direction, size,
//! rating and end type). The same shapes give the meshes, the clash shapes and the exports.

use crate::model::P3;
use serde::Serialize;

// ---------------------------------------------------------------- vector helpers
pub fn add(a: P3, b: P3) -> P3 { [a[0] + b[0], a[1] + b[1], a[2] + b[2]] }
pub fn sub(a: P3, b: P3) -> P3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
pub fn mul(a: P3, s: f64) -> P3 { [a[0] * s, a[1] * s, a[2] * s] }
pub fn dot(a: P3, b: P3) -> f64 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }
pub fn cross(a: P3, b: P3) -> P3 { [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] }
pub fn norm(a: P3) -> f64 { dot(a, a).sqrt() }
pub fn unit(a: P3) -> P3 { let n = norm(a); if n > 1e-15 { mul(a, 1.0 / n) } else { [0.0, 0.0, 0.0] } }
pub fn dist(a: P3, b: P3) -> f64 { norm(sub(a, b)) }
/// Two unit vectors square to `d` (and to each other).
pub fn frame(d: P3) -> (P3, P3) {
    let d = unit(d);
    let r = if d[2].abs() < 0.9 { [0.0, 0.0, 1.0] } else { [1.0, 0.0, 0.0] };
    let u = unit(cross(r, d));
    (u, cross(d, u))
}
/// Rotates a vector about the vertical by `deg` degrees (anticlockwise seen from above).
pub fn rot_z(v: P3, deg: f64) -> P3 {
    let (s, c) = deg.to_radians().sin_cos();
    [v[0] * c - v[1] * s, v[0] * s + v[1] * c, v[2]]
}

/// A simple shape in plant coordinates (m).
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Shape {
    /// A cone frustum (a cylinder when r1 = r2) from a to b, closed at both ends.
    Frustum { a: P3, b: P3, r1: f64, r2: f64 },
    /// A bent tube: the arc about `centre` in the plane with normal `axis`, starting at
    /// `centre + start` and turning `angle` radians, with tube radius `r`.
    Torus { centre: P3, axis: P3, start: P3, angle: f64, r: f64 },
    /// A vessel head on the end of a cylinder at `base`, bulging along `dir` by `depth` (an
    /// ellipsoid of revolution; depth = r is hemispherical).
    Head { base: P3, dir: P3, r: f64, depth: f64 },
    /// A box: centre, half sizes along the three unit axes.
    Cuboid { centre: P3, axes: [P3; 3], half: P3 },
}

impl Shape {
    pub fn tube(a: P3, b: P3, r: f64) -> Shape { Shape::Frustum { a, b, r1: r, r2: r } }
    /// The shape's bounding box.
    pub fn aabb(&self) -> (P3, P3) {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        let mut put = |p: P3, r: f64| for k in 0..3 { lo[k] = lo[k].min(p[k] - r); hi[k] = hi[k].max(p[k] + r); };
        // A circle of radius r square to unit d reaches r·√(1 − d_k²) along axis k.
        let disc = |lo: &mut P3, hi: &mut P3, c: P3, d: P3, r: f64| for k in 0..3 { let e = r * (1.0 - d[k] * d[k]).max(0.0).sqrt(); lo[k] = lo[k].min(c[k] - e); hi[k] = hi[k].max(c[k] + e); };
        match self {
            Shape::Frustum { a, b, r1, r2 } => { let d = unit(sub(*b, *a)); disc(&mut lo, &mut hi, *a, d, *r1); disc(&mut lo, &mut hi, *b, d, *r2); }
            Shape::Torus { centre, start, r, angle, axis } => {
                let rr = norm(*start);
                for i in 0..=8 { put(add(*centre, rotate(*start, *axis, angle * i as f64 / 8.0)), r + rr * (1.0 - (angle / 16.0).cos())); }
            }
            Shape::Head { base, dir, r, depth } => { let d = unit(*dir); disc(&mut lo, &mut hi, *base, d, *r); disc(&mut lo, &mut hi, add(*base, mul(d, *depth)), d, *r * 0.0); for t in 1..8 { let a = std::f64::consts::FRAC_PI_2 * t as f64 / 8.0; disc(&mut lo, &mut hi, add(*base, mul(d, depth * a.sin())), d, r * a.cos()); } }
            Shape::Cuboid { centre, axes, half } => {
                for s in 0..8 {
                    let mut p = *centre;
                    for k in 0..3 { p = add(p, mul(axes[k], if s >> k & 1 == 1 { half[k] } else { -half[k] })); }
                    put(p, 0.0);
                }
            }
        }
        (lo, hi)
    }
}

/// Rotates `v` about the unit axis `k` by `a` radians (Rodrigues).
pub fn rotate(v: P3, k: P3, a: f64) -> P3 {
    let (s, c) = a.sin_cos();
    add(add(mul(v, c), mul(cross(k, v), s)), mul(k, dot(k, v) * (1.0 - c)))
}

/// A connection point of a part.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Port {
    pub p: P3,
    /// Outward direction (away from the part).
    pub d: P3,
    pub nps: String,
    /// Flange rating for flanged ends; 0 for weld ends.
    pub rating: u32,
    /// "BW" (butt weld) or "FL" (flanged face).
    pub end: &'static str,
}

/// A triangle mesh: positions, normals (one per vertex) and triangles.
#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub pos: Vec<f32>,
    pub nor: Vec<f32>,
    pub idx: Vec<u32>,
}

impl Mesh {
    fn v(&mut self, p: P3, n: P3) -> u32 {
        self.pos.extend([p[0] as f32, p[1] as f32, p[2] as f32]);
        self.nor.extend([n[0] as f32, n[1] as f32, n[2] as f32]);
        (self.pos.len() / 3 - 1) as u32
    }
    pub fn triangles(&self) -> usize { self.idx.len() / 3 }
    pub fn vertices(&self) -> usize { self.pos.len() / 3 }

    /// A ring strip between rings of points `a` and `b` (same count, closed loops).
    fn strip(&mut self, a: &[(P3, P3)], b: &[(P3, P3)]) {
        let n = a.len();
        let i0 = self.vertices() as u32;
        for &(p, q) in a.iter().chain(b) { self.v(p, q); }
        for i in 0..n as u32 {
            let j = (i + 1) % n as u32;
            self.idx.extend([i0 + i, i0 + j, i0 + n as u32 + i, i0 + j, i0 + n as u32 + j, i0 + n as u32 + i]);
        }
    }
    /// A flat disc closing a ring, facing `n`.
    fn cap(&mut self, c: P3, ring: &[P3], n: P3) {
        let i0 = self.vertices() as u32;
        self.v(c, n);
        for &p in ring { self.v(p, n); }
        let m = ring.len() as u32;
        for i in 0..m {
            let (a, b) = (i0 + 1 + i, i0 + 1 + (i + 1) % m);
            // Wind so the face points along n.
            let pa = ring[i as usize];
            let pb = ring[((i + 1) % m) as usize];
            if dot(cross(sub(pa, c), sub(pb, c)), n) >= 0.0 { self.idx.extend([i0, a, b]); } else { self.idx.extend([i0, b, a]); }
        }
    }

    /// Adds a shape at `seg` segments round a circle.
    pub fn shape(&mut self, s: &Shape, seg: usize) {
        let seg = seg.clamp(6, 64);
        let circle = |c: P3, d: P3, r: f64| -> Vec<(P3, P3)> {
            let (u, v) = frame(d);
            (0..seg).map(|i| { let t = std::f64::consts::TAU * i as f64 / seg as f64; let n = add(mul(u, t.cos()), mul(v, t.sin())); (add(c, mul(n, r)), n) }).collect()
        };
        match s {
            Shape::Frustum { a, b, r1, r2 } => {
                let d = unit(sub(*b, *a));
                if norm(sub(*b, *a)) < 1e-9 { return; }
                let slope = (r1 - r2) / dist(*a, *b);
                let ra = circle(*a, d, *r1);
                let rb = circle(*b, d, *r2);
                let fix = |r: &Vec<(P3, P3)>| -> Vec<(P3, P3)> { r.iter().map(|&(p, n)| (p, unit(add(n, mul(d, slope))))).collect() };
                self.strip(&fix(&ra), &fix(&rb));
                self.cap(*a, &ra.iter().map(|x| x.0).collect::<Vec<_>>(), mul(d, -1.0));
                self.cap(*b, &rb.iter().map(|x| x.0).collect::<Vec<_>>(), d);
            }
            Shape::Torus { centre, axis, start, angle, r } => {
                let k = unit(*axis);
                let steps = ((angle.abs() / std::f64::consts::FRAC_PI_2) * (seg as f64 / 2.0)).ceil().max(2.0) as usize;
                let mut prev: Option<Vec<(P3, P3)>> = None;
                let mut first = vec![];
                let mut last = vec![];
                for i in 0..=steps {
                    let t = angle * i as f64 / steps as f64;
                    let rad = rotate(*start, k, t);
                    let p = add(*centre, rad);
                    let tan = unit(cross(k, rad));
                    let ring = circle(p, tan, *r);
                    if i == 0 { first = ring.clone(); }
                    if i == steps { last = ring.clone(); }
                    if let Some(pr) = &prev { self.strip(pr, &ring); }
                    prev = Some(ring);
                }
                let p0 = add(*centre, *start);
                let p1 = add(*centre, rotate(*start, k, *angle));
                self.cap(p0, &first.iter().map(|x| x.0).collect::<Vec<_>>(), mul(unit(cross(k, *start)), -1.0));
                self.cap(p1, &last.iter().map(|x| x.0).collect::<Vec<_>>(), unit(cross(k, sub(p1, *centre))));
            }
            Shape::Head { base, dir, r, depth } => {
                let d = unit(*dir);
                let rings = (seg / 4).max(3);
                let mut prev = circle(*base, d, *r);
                for i in 1..=rings {
                    let t = std::f64::consts::FRAC_PI_2 * i as f64 / rings as f64;
                    let (rr, h) = (r * t.cos(), depth * t.sin());
                    let ring: Vec<(P3, P3)> = circle(add(*base, mul(d, h)), d, rr.max(1e-6)).into_iter().map(|(p, n)| (p, unit(add(mul(n, t.cos() / r), mul(d, t.sin() / depth.max(1e-9)))))).collect();
                    let prev_n: Vec<(P3, P3)> = prev.iter().map(|&(p, n)| (p, n)).collect();
                    self.strip(&prev_n, &ring);
                    prev = ring;
                }
            }
            Shape::Cuboid { centre, axes, half } => {
                for k in 0..3 {
                    for sgn in [-1.0, 1.0] {
                        let n = mul(axes[k], sgn);
                        let (a, b) = (axes[(k + 1) % 3], axes[(k + 2) % 3]);
                        let (ha, hb) = (half[(k + 1) % 3], half[(k + 2) % 3]);
                        let c = add(*centre, mul(n, half[k]));
                        let pts = [add(add(c, mul(a, -ha)), mul(b, -hb)), add(add(c, mul(a, ha)), mul(b, -hb)), add(add(c, mul(a, ha)), mul(b, hb)), add(add(c, mul(a, -ha)), mul(b, hb))];
                        let i0 = self.vertices() as u32;
                        for p in pts { self.v(p, n); }
                        if dot(cross(sub(pts[1], pts[0]), sub(pts[2], pts[0])), n) >= 0.0 { self.idx.extend([i0, i0 + 1, i0 + 2, i0, i0 + 2, i0 + 3]); } else { self.idx.extend([i0, i0 + 2, i0 + 1, i0, i0 + 3, i0 + 2]); }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meshes_are_closed_and_where_their_shapes_are() {
        let shapes = [
            Shape::tube([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], 0.1),
            Shape::Frustum { a: [0.0, 0.0, 0.0], b: [0.0, 0.0, 0.3], r1: 0.1, r2: 0.05 },
            Shape::Torus { centre: [0.0; 3], axis: [0.0, 0.0, 1.0], start: [0.2, 0.0, 0.0], angle: std::f64::consts::FRAC_PI_2, r: 0.05 },
            Shape::Head { base: [0.0; 3], dir: [0.0, 0.0, 1.0], r: 1.0, depth: 0.5 },
            Shape::Cuboid { centre: [0.0; 3], axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], half: [1.0, 2.0, 3.0] },
        ];
        for s in &shapes {
            let mut m = Mesh::default();
            m.shape(s, 16);
            assert!(m.triangles() > 0 && m.idx.iter().all(|&i| (i as usize) < m.vertices()));
            let (lo, hi) = s.aabb();
            for p in m.pos.chunks(3) {
                for k in 0..3 { assert!(p[k] as f64 >= lo[k] - 1e-6 && p[k] as f64 <= hi[k] + 1e-6, "{s:?}"); }
            }
            assert!(m.nor.chunks(3).all(|n| ((n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt() - 1.0).abs() < 1e-3));
        }
        // A torus quarter ends where the arc ends.
        let p = rotate([0.2, 0.0, 0.0], [0.0, 0.0, 1.0], std::f64::consts::FRAC_PI_2);
        assert!(dist(p, [0.0, 0.2, 0.0]) < 1e-12);
    }
}
