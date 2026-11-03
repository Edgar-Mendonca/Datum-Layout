//! What the 3D view draws: every part's mesh in one buffer (positions, normals and triangles,
//! sent as base64 binary so a large plant stays quick to send), which triangles belong to which
//! part (for picking and colours), insulation sleeves, and the ports a route can start or end at.

use crate::catalogue::Catalogue;
use crate::model::{Project, P3};
use crate::parts::{Mesh, Shape};
use crate::route::{Built, Part};
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
pub struct PartRef {
    pub kind: &'static str,
    pub line: u32,
    pub equip: u32,
    pub item: u32,
    pub seg: i32,
    pub nps: String,
    /// Triangles [t0, t1) in the scene's index buffer.
    pub t0: u32,
    pub t1: u32,
    pub lo: P3,
    pub hi: P3,
}

#[derive(Serialize, Clone, Debug)]
pub struct PortRef {
    pub p: P3,
    pub d: P3,
    pub nps: String,
    /// "nozzle" (with equip and tag), or "end" (an open line end).
    pub kind: &'static str,
    pub equip: u32,
    pub tag: String,
    pub line: u32,
    /// For a tee's branch end: the tee.
    pub item: u32,
    pub used: bool,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Scene {
    pub pos: String,
    pub nor: String,
    pub idx: String,
    pub parts: Vec<PartRef>,
    pub ins_pos: String,
    pub ins_nor: String,
    pub ins_idx: String,
    pub ports: Vec<PortRef>,
    pub triangles: usize,
}

pub fn b64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        s.push(T[(n >> 18) as usize & 63] as char);
        s.push(T[(n >> 12) as usize & 63] as char);
        s.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        s.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    s
}
fn f32s(v: &[f32]) -> String { b64(&v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>()) }
fn u32s(v: &[u32]) -> String { b64(&v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>()) }

/// Above this many parts the view draws at low detail.
pub const LARGE: usize = 1500;

/// Segments round a circle for a level of detail (0 low, 1 normal, 2 high).
pub fn segments(lod: u32) -> usize {
    match lod { 0 => 10, 2 => 28, _ => 18 }
}

/// The mesh of every part, one after another; returns the mesh and each part's triangle range.
pub fn meshes(parts: &[Part], lod: u32) -> (Mesh, Vec<(u32, u32)>) {
    let seg = segments(lod);
    let mut m = Mesh::default();
    let mut ranges = vec![];
    for q in parts {
        let t0 = m.triangles() as u32;
        for s in &q.shapes { m.shape(s, if q.kind == "equipment" { seg + 8 } else { seg }); }
        ranges.push((t0, m.triangles() as u32));
    }
    (m, ranges)
}

pub fn scene(p: &Project, cat: &Catalogue, b: &Built, lod: u32) -> Scene {
    // Large plants draw at low detail so they stay smooth to orbit.
    let lod = if b.parts.len() > LARGE { 0 } else { lod };
    let (m, ranges) = meshes(&b.parts, lod);
    let mut ins = Mesh::default();
    for q in b.parts.iter().filter(|q| q.insulation > 0.0 && matches!(q.kind, "pipe" | "elbow90" | "elbow45" | "tee" | "reducer")) {
        for s in &q.shapes {
            let t = q.insulation;
            let s2 = match s {
                Shape::Frustum { a, b, r1, r2 } => Shape::Frustum { a: *a, b: *b, r1: r1 + t, r2: r2 + t },
                Shape::Torus { centre, axis, start, angle, r } => Shape::Torus { centre: *centre, axis: *axis, start: *start, angle: *angle, r: r + t },
                x => x.clone(),
            };
            ins.shape(&s2, segments(lod.min(1)));
        }
    }
    let parts = b.parts.iter().zip(&ranges).map(|(q, r)| {
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for s in &q.shapes { let (a, c) = s.aabb(); for k in 0..3 { lo[k] = lo[k].min(a[k]); hi[k] = hi[k].max(c[k]); } }
        if q.shapes.is_empty() { lo = [0.0; 3]; hi = [0.0; 3]; }
        PartRef { kind: q.kind, line: q.line, equip: q.equip, item: q.item, seg: q.seg, nps: q.nps.clone(), t0: r.0, t1: r.1, lo, hi }
    }).collect();
    // Ports: every nozzle (used or not) and every open line end.
    let mut ports = vec![];
    let conn = crate::checks::connections(b);
    for (i, q) in b.parts.iter().enumerate() {
        for (j, pt) in q.ports.iter().enumerate() {
            let used = conn.contains_key(&(i, j));
            if q.kind == "nozzle" {
                ports.push(PortRef { p: pt.p, d: pt.d, nps: pt.nps.clone(), kind: "nozzle", equip: q.equip, tag: q.note.clone(), line: 0, item: 0, used });
            } else if !used && q.line != 0 && !matches!(q.kind, "cap" | "blind" | "bolts") {
                ports.push(PortRef { p: pt.p, d: pt.d, nps: pt.nps.clone(), kind: if q.kind == "tee" && j == 2 { "branch" } else { "end" }, equip: 0, tag: String::new(), line: q.line, item: q.item, used });
            }
        }
    }
    let _ = (p, cat);
    Scene { pos: f32s(&m.pos), nor: f32s(&m.nor), idx: u32s(&m.idx), parts, ins_pos: f32s(&ins.pos), ins_nor: f32s(&ins.nor), ins_idx: u32s(&ins.idx), ports, triangles: m.triangles() }
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_matches_the_standard_alphabet() {
        assert_eq!(super::b64(b""), "");
        assert_eq!(super::b64(b"f"), "Zg==");
        assert_eq!(super::b64(b"fo"), "Zm8=");
        assert_eq!(super::b64(b"foobar"), "Zm9vYmFy");
    }
}
