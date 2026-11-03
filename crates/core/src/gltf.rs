//! glTF 2.0 export (binary .glb): every part as a named node with its own mesh, coloured by what
//! it is, in one buffer. glTF is Y up, so the plant's (east, north, up) becomes (x, z → −north,
//! y → up). Metres, as glTF expects.

use crate::model::Project;
use crate::parts::Mesh;
use crate::route::Built;
use serde_json::json;

fn colour(kind: &str) -> [f64; 4] {
    match kind {
        "equipment" => [0.60, 0.66, 0.72, 1.0],
        "nozzle" => [0.48, 0.54, 0.60, 1.0],
        "pipe" | "elbow90" | "elbow45" | "tee" | "reducer" | "cap" => [0.23, 0.35, 0.47, 1.0],
        "flange" | "blind" | "gasket" => [0.42, 0.47, 0.52, 1.0],
        _ => [0.72, 0.47, 0.12, 1.0],
    }
}

fn label(p: &Project, q: &crate::route::Part) -> String {
    if q.line != 0 { format!("{} {}", p.line(q.line).map(|l| l.number.as_str()).unwrap_or(""), q.kind) } else { format!("{} {}", p.equip(q.equip).map(|e| e.tag.as_str()).unwrap_or(""), if q.kind == "nozzle" { &q.note } else { "" }) }.trim().to_string()
}

/// The model as a .glb file.
pub fn glb(p: &Project, b: &Built, lod: u32) -> Vec<u8> {
    let seg = crate::scene::segments(lod);
    let mut bin: Vec<u8> = vec![];
    let (mut views, mut accessors, mut meshes, mut nodes) = (vec![], vec![], vec![], vec![]);
    let kinds: Vec<&str> = {
        let mut k: Vec<&str> = b.parts.iter().map(|q| q.kind).collect();
        k.sort();
        k.dedup();
        k
    };
    let materials: Vec<_> = kinds.iter().map(|k| json!({ "name": k, "pbrMetallicRoughness": { "baseColorFactor": colour(k), "metallicFactor": 0.3, "roughnessFactor": 0.6 } })).collect();
    for q in &b.parts {
        if q.shapes.is_empty() { continue; }
        let mut m = Mesh::default();
        for s in &q.shapes { m.shape(s, seg); }
        if m.idx.is_empty() { continue; }
        // (E, N, U) → (E, U, −N).
        let pos: Vec<f32> = m.pos.chunks(3).flat_map(|v| [v[0], v[2], -v[1]]).collect();
        let nor: Vec<f32> = m.nor.chunks(3).flat_map(|v| [v[0], v[2], -v[1]]).collect();
        let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        for v in pos.chunks(3) { for k in 0..3 { lo[k] = lo[k].min(v[k]); hi[k] = hi[k].max(v[k]); } }
        let mut push = |bytes: Vec<u8>, target: u32| -> usize {
            while bin.len() % 4 != 0 { bin.push(0); }
            let off = bin.len();
            bin.extend(&bytes);
            views.push(json!({ "buffer": 0, "byteOffset": off, "byteLength": bytes.len(), "target": target }));
            views.len() - 1
        };
        let vp = push(pos.iter().flat_map(|x| x.to_le_bytes()).collect(), 34962);
        let vn = push(nor.iter().flat_map(|x| x.to_le_bytes()).collect(), 34962);
        let vi = push(m.idx.iter().flat_map(|x| x.to_le_bytes()).collect(), 34963);
        let n = pos.len() / 3;
        accessors.push(json!({ "bufferView": vp, "componentType": 5126, "count": n, "type": "VEC3", "min": lo, "max": hi }));
        accessors.push(json!({ "bufferView": vn, "componentType": 5126, "count": n, "type": "VEC3" }));
        accessors.push(json!({ "bufferView": vi, "componentType": 5125, "count": m.idx.len(), "type": "SCALAR" }));
        let a = accessors.len();
        let mat = kinds.iter().position(|k| *k == q.kind).unwrap_or(0);
        meshes.push(json!({ "name": label(p, q), "primitives": [{ "attributes": { "POSITION": a - 3, "NORMAL": a - 2 }, "indices": a - 1, "material": mat, "mode": 4 }] }));
        nodes.push(json!({ "name": label(p, q), "mesh": meshes.len() - 1 }));
    }
    while bin.len() % 4 != 0 { bin.push(0); }
    let mut doc = json!({
        "asset": { "version": "2.0", "generator": concat!("Datum Layout ", env!("CARGO_PKG_VERSION")), "copyright": "" },
        "scene": 0,
        "scenes": [{ "name": p.name, "nodes": (0..nodes.len()).collect::<Vec<_>>() }],
        "nodes": nodes, "meshes": meshes, "materials": materials, "accessors": accessors, "bufferViews": views,
    });
    if !bin.is_empty() { doc["buffers"] = json!([{ "byteLength": bin.len() }]); } else if let Some(o) = doc.as_object_mut() { o.remove("bufferViews"); o.remove("accessors"); o.remove("meshes"); }
    if let Some(o) = doc["asset"].as_object_mut() { o.remove("copyright"); }
    let mut js = serde_json::to_vec(&doc).unwrap_or_default();
    while js.len() % 4 != 0 { js.push(b' '); }
    let total = 12 + 8 + js.len() + if bin.is_empty() { 0 } else { 8 + bin.len() };
    let mut out = Vec::with_capacity(total);
    out.extend(b"glTF");
    out.extend(2u32.to_le_bytes());
    out.extend((total as u32).to_le_bytes());
    out.extend((js.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(js);
    if !bin.is_empty() {
        out.extend((bin.len() as u32).to_le_bytes());
        out.extend(b"BIN\0");
        out.extend(bin);
    }
    out
}

/// Reads a .glb back and checks it as a viewer would: header, chunks, every accessor inside its
/// view, every view inside the buffer, every index inside its positions. Used by the tests.
pub fn verify(glb: &[u8]) -> Result<(usize, usize), String> {
    let u = |o: usize| -> Result<u32, String> { glb.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).ok_or("cut short".to_string()) };
    if glb.get(0..4) != Some(b"glTF") || u(4)? != 2 || u(8)? as usize != glb.len() { return Err("bad header".into()); }
    let jl = u(12)? as usize;
    if glb.get(16..20) != Some(b"JSON") || jl % 4 != 0 { return Err("bad JSON chunk".into()); }
    let doc: serde_json::Value = serde_json::from_slice(glb.get(20..20 + jl).ok_or("cut short")?).map_err(|e| e.to_string())?;
    let bin = if glb.len() > 20 + jl { let bl = u(20 + jl)? as usize; if glb.get(24 + jl..28 + jl) != Some(b"BIN\0") { return Err("bad BIN chunk".into()); } glb.get(28 + jl..28 + jl + bl).ok_or("cut short")? } else { &[][..] };
    if doc["asset"]["version"] != "2.0" { return Err("not glTF 2.0".into()); }
    let views = doc["bufferViews"].as_array().cloned().unwrap_or_default();
    for v in &views { if v["byteOffset"].as_u64().unwrap_or(0) + v["byteLength"].as_u64().unwrap_or(0) > bin.len() as u64 { return Err("a view runs past the buffer".into()); } }
    let acc = doc["accessors"].as_array().cloned().unwrap_or_default();
    let mut tris = 0;
    for m in doc["meshes"].as_array().cloned().unwrap_or_default() {
        for pr in m["primitives"].as_array().cloned().unwrap_or_default() {
            let pa = &acc[pr["attributes"]["POSITION"].as_u64().ok_or("no positions")? as usize];
            let ia = &acc[pr["indices"].as_u64().ok_or("no indices")? as usize];
            if pa["min"].as_array().is_none() || pa["max"].as_array().is_none() { return Err("positions need min and max".into()); }
            let n = pa["count"].as_u64().unwrap_or(0);
            let v = &views[ia["bufferView"].as_u64().unwrap_or(0) as usize];
            let off = v["byteOffset"].as_u64().unwrap_or(0) as usize;
            let cnt = ia["count"].as_u64().unwrap_or(0) as usize;
            if cnt * 4 > v["byteLength"].as_u64().unwrap_or(0) as usize || cnt % 3 != 0 { return Err("index count".into()); }
            for k in 0..cnt { let i = u32::from_le_bytes(bin[off + 4 * k..off + 4 * k + 4].try_into().unwrap_or([0; 4])); if i as u64 >= n { return Err("an index past its positions".into()); } }
            tris += cnt / 3;
        }
    }
    Ok((doc["nodes"].as_array().map_or(0, |n| n.len()), tris))
}
