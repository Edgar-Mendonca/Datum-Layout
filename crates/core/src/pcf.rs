//! Piping component files (.pcf) for Datum Pipe: one line, or every line, as the plain-text
//! component list Datum Pipe's import reads. Coordinates in millimetres with X east, Y north and
//! Z up (the model's own axes); bores are nominal millimetre sizes (DN). Elbows carry their corner
//! as the centre point, tees their centre and branch end. Bolt sets, nozzles and equipment are not
//! part of the file.
//!
//! `reader` is Datum Pipe's own reader (crates/core/src/import.rs there), copied so the tests can
//! prove that Pipe reads Layout's files unchanged: the same nodes, sizes and components.

use crate::catalogue::Catalogue;
use crate::model::{Project, P3};
use crate::route::{build, Part};

/// Nominal sizes as nominal millimetre bores.
pub fn dn(nps: &str) -> u32 {
    match nps {
        "1/2" => 15, "3/4" => 20, "1" => 25, "1-1/4" => 32, "1-1/2" => 40, "2" => 50, "2-1/2" => 65, "3" => 80, "3-1/2" => 90, "4" => 100, "5" => 125, "6" => 150,
        "8" => 200, "10" => 250, "12" => 300, "14" => 350, "16" => 400, "18" => 450, "20" => 500, "22" => 550, "24" => 600, _ => 0,
    }
}

fn pt(p: P3, nps: &str) -> String {
    format!("{:.2} {:.2} {:.2} {}", p[0] * 1000.0, p[1] * 1000.0, p[2] * 1000.0, dn(nps))
}

/// The component keyword and its SKEY for a part, or None for what is not written.
fn keyword(q: &Part) -> Option<(&'static str, &'static str)> {
    Some(match q.kind {
        "pipe" => ("PIPE", ""),
        "elbow90" | "elbow45" => ("ELBOW", "ELBW"),
        "tee" => ("TEE", "TEBW"),
        "reducer" => ("REDUCER-CONCENTRIC", "RCBW"),
        "flange" => ("FLANGE", "FLWN"),
        "blind" => ("FLANGE-BLIND", "FLBL"),
        "gasket" => ("GASKET", "GK"),
        "cap" => ("CAP", "CPBW"),
        "gate" => ("VALVE", "VTFL"),
        "globe" => ("VALVE", "VGFL"),
        "check" => ("VALVE", "CKFL"),
        "ball" => ("VALVE", "VBFL"),
        "strainer" => ("FILTER", "FYFL"),
        _ => return None,
    })
}

/// The file for one line (`line` = Some) or for every line.
pub fn write(p: &Project, cat: &Catalogue, line: Option<u32>) -> Result<String, String> {
    let b = build(p, cat);
    let lines: Vec<_> = p.lines.iter().filter(|l| line.is_none_or(|id| l.id == id)).collect();
    if lines.is_empty() { return Err(if line.is_some() { "That line no longer exists.".into() } else { "The plant has no lines yet.".into() }); }
    let mut s = String::from("ISOGEN-FILES ISOGEN.FLS\nUNITS-BORE MM\nUNITS-CO-ORDS MM\nUNITS-WEIGHT KGS\nUNITS-BOLT-DIA MM\nUNITS-BOLT-LENGTH MM\n");
    for l in lines {
        let number = if l.number.trim().is_empty() { format!("LINE-{}", l.id) } else { l.number.replace(char::is_whitespace, "_") };
        s += &format!("PIPELINE-REFERENCE {number}\n    PIPING-SPEC {}\n", l.class);
        for q in b.parts.iter().filter(|q| q.line == l.id) {
            let Some((key, skey)) = keyword(q) else { continue };
            s += key;
            s += "\n";
            match q.kind {
                "tee" => {
                    s += &format!("    END-POINT {}\n    END-POINT {}\n    CENTRE-POINT {:.2} {:.2} {:.2}\n    BRANCH1-POINT {}\n", pt(q.ports[0].p, &q.ports[0].nps), pt(q.ports[1].p, &q.ports[1].nps), q.corner.unwrap_or_default()[0] * 1000.0, q.corner.unwrap_or_default()[1] * 1000.0, q.corner.unwrap_or_default()[2] * 1000.0, pt(q.ports[2].p, &q.ports[2].nps));
                }
                _ => {
                    for port in &q.ports { s += &format!("    END-POINT {}\n", pt(port.p, &port.nps)); }
                    // One-port parts (caps, blinds) end where they close.
                    if q.ports.len() == 1 {
                        let far = q.ports[0].p;
                        let back = crate::parts::sub(far, crate::parts::mul(q.ports[0].d, q.length));
                        s += &format!("    END-POINT {}\n", pt(back, &q.nps));
                    }
                    if let Some(c) = q.corner { s += &format!("    CENTRE-POINT {:.2} {:.2} {:.2}\n", c[0] * 1000.0, c[1] * 1000.0, c[2] * 1000.0); }
                }
            }
            if !skey.is_empty() { s += &format!("    SKEY {skey}\n"); }
            if q.kind.starts_with("elbow") { s += &format!("    ANGLE {:.0}\n", if q.kind == "elbow90" { 90 } else { 45 }); }
            if q.rating > 0 { s += &format!("    ITEM-DESCRIPTION CLASS {} {}\n", q.rating, q.kind.to_uppercase()); }
        }
    }
    Ok(s)
}

/// Datum Pipe's reader (the parsing half of its `import::pcf`), copied unchanged except that it
/// returns what it read (points, pieces, bends, tees) instead of building Pipe's model.
pub mod reader {
    use std::collections::HashMap;
    type P3 = [f64; 3];

    pub const MM_NPS: &[(f64, &str)] = &[
        (15.0, "1/2"), (20.0, "3/4"), (25.0, "1"), (32.0, "1-1/4"), (40.0, "1-1/2"), (50.0, "2"), (65.0, "2-1/2"), (80.0, "3"), (90.0, "3-1/2"),
        (100.0, "4"), (125.0, "5"), (150.0, "6"), (200.0, "8"), (250.0, "10"), (300.0, "12"), (350.0, "14"), (400.0, "16"), (450.0, "18"), (500.0, "20"), (600.0, "24"),
    ];

    struct Component { kind: String, attrs: Vec<(String, Vec<String>)> }
    impl Component {
        fn all(&self, key: &str) -> Vec<&Vec<String>> { self.attrs.iter().filter(|a| a.0 == key).map(|a| &a.1).collect() }
        fn first(&self, key: &str) -> Option<&Vec<String>> { self.attrs.iter().find(|a| a.0 == key).map(|a| &a.1) }
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Rigid { Valve, Flange, Rigid }

    #[derive(Debug, Clone)]
    pub struct Piece { pub a: usize, pub b: usize, pub nps: Option<&'static str>, pub to_nps: Option<&'static str>, pub rigid: Option<Rigid> }

    /// What Datum Pipe made of the file (in Pipe's axes: Y up, Z south).
    #[derive(Debug, Default)]
    pub struct Read { pub name: String, pub nodes: Vec<P3>, pub pieces: Vec<Piece>, pub bends: Vec<(usize, f64)>, pub tees: Vec<usize>, pub warnings: Vec<String>, pub components: usize }

    fn num(s: &str) -> Option<f64> { s.trim().parse::<f64>().ok().filter(|v| v.is_finite()) }
    fn sub(a: P3, b: P3) -> P3 { [a[0] - b[0], a[1] - b[1], a[2] - b[2]] }
    fn len(a: P3) -> f64 { (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt() }
    fn dot(a: P3, b: P3) -> f64 { a[0] * b[0] + a[1] * b[1] + a[2] * b[2] }

    pub fn pcf(text: &str) -> Result<Read, String> {
        let mut header: Vec<(String, Vec<String>)> = vec![];
        let mut comps: Vec<Component> = vec![];
        for raw in text.lines() {
            if raw.trim().is_empty() { continue; }
            let indented = raw.starts_with(' ') || raw.starts_with('\t');
            let mut words = raw.split_whitespace().map(|w| w.to_string());
            let key = words.next().unwrap_or_default().to_uppercase();
            let rest: Vec<String> = words.collect();
            if indented {
                match comps.last_mut() { Some(c) => c.attrs.push((key, rest)), None => header.push((key, rest)) }
            } else if key.contains("UNITS") || key.ends_with("-FILES") || key == "PIPELINE-REFERENCE" || key.starts_with("PROJECT") || key.starts_with("AREA") || key.starts_with("DATE") || key.starts_with("REVISION") {
                header.push((key, rest));
            } else {
                comps.push(Component { kind: key, attrs: vec![] });
            }
        }
        let head = |k: &str| header.iter().find(|h| h.0 == k).map(|h| h.1.join(" ").to_uppercase());
        let inch = |v: Option<String>| v.is_some_and(|s| s.starts_with("INCH"));
        let coord = if inch(head("UNITS-CO-ORDS")) { 0.0254 } else { 0.001 };
        let bore_in = inch(head("UNITS-BORE"));
        let name = header.iter().find(|h| h.0 == "PIPELINE-REFERENCE").map(|h| h.1.join(" ")).filter(|s| !s.is_empty()).unwrap_or_else(|| "Imported piping".into());
        if !comps.iter().any(|c| c.first("END-POINT").is_some() || c.first("CENTRE-POINT").is_some() || c.first("CO-ORDS").is_some()) {
            return Err("This file has no piping components with coordinates: is it a piping component file (.pcf)?".into());
        }
        let mut warnings: Vec<String> = vec![];
        let point = |v: &[String]| -> Option<(P3, f64)> {
            let (x, y, z) = (num(v.first()?)?, num(v.get(1)?)?, num(v.get(2)?)?);
            let bore = v.get(3).and_then(|b| num(b)).unwrap_or(0.0);
            Some(([x * coord, z * coord, -y * coord], bore))
        };
        let mut nodes: Vec<P3> = vec![];
        let node_at = |p: P3, nodes: &mut Vec<P3>| -> usize {
            if let Some(i) = nodes.iter().position(|q| len(sub(*q, p)) < 1e-3) { return i; }
            nodes.push(p);
            nodes.len() - 1
        };
        let nps_of = |bore: f64| -> Option<&'static str> {
            if bore <= 0.0 { return None; }
            let mm = if bore_in { bore * 25.4 } else { bore };
            MM_NPS.iter().min_by(|a, b| (a.0 - mm).abs().total_cmp(&(b.0 - mm).abs())).filter(|x| (x.0 - mm).abs() <= 0.15 * x.0).map(|x| x.1)
        };
        let mut pieces: Vec<Piece> = vec![];
        let mut bends: Vec<(usize, f64)> = vec![];
        let mut tees: Vec<usize> = vec![];
        let mut sups: Vec<usize> = vec![];
        let mut unknown: HashMap<String, usize> = HashMap::new();
        let mut last_nps: Option<&'static str> = None;
        for c in &comps {
            let ends: Vec<(P3, f64)> = c.all("END-POINT").iter().filter_map(|v| point(v)).collect();
            let centre = c.first("CENTRE-POINT").and_then(|v| point(v));
            let branch = c.first("BRANCH1-POINT").and_then(|v| point(v));
            let mut nps = |b: f64| { let n = nps_of(b).or(last_nps); if n.is_some() { last_nps = n; } n };
            let k = c.kind.as_str();
            match k {
                _ if k == "SUPPORT" || k.starts_with("SUPPORT") => {
                    let Some((p, _)) = c.first("CO-ORDS").and_then(|v| point(v)) else { continue };
                    sups.push(node_at(p, &mut nodes));
                }
                "ELBOW" | "BEND" | "PULLED-BEND" if ends.len() == 2 && centre.is_some() => {
                    let corner = centre.unwrap().0;
                    let (u, v) = (sub(corner, ends[0].0), sub(ends[1].0, corner));
                    let (lu, lv) = (len(u), len(v));
                    let cos = if lu * lv > 0.0 { dot(u, v) / (lu * lv) } else { 1.0 };
                    let n0 = nps(ends[0].1);
                    let (a, m, b) = (node_at(ends[0].0, &mut nodes), node_at(corner, &mut nodes), node_at(ends[1].0, &mut nodes));
                    if cos > 0.9999 || lu < 1e-6 || lv < 1e-6 {
                        pieces.push(Piece { a, b, nps: n0, to_nps: None, rigid: None });
                    } else {
                        let half = 0.5 * cos.clamp(-1.0, 1.0).acos();
                        bends.push((m, 0.5 * (lu + lv) / half.tan()));
                        pieces.push(Piece { a, b: m, nps: n0, to_nps: None, rigid: None });
                        pieces.push(Piece { a: m, b, nps: n0, to_nps: None, rigid: None });
                    }
                }
                _ if (k == "TEE" || k.starts_with("TEE-") || k == "CROSS") && ends.len() == 2 && centre.is_some() => {
                    let ctr = centre.unwrap().0;
                    let n0 = nps(ends[0].1);
                    let m = node_at(ctr, &mut nodes);
                    for e in &ends { let a = node_at(e.0, &mut nodes); pieces.push(Piece { a, b: m, nps: n0, to_nps: None, rigid: None }); }
                    if let Some((bp, bb)) = branch { let b = node_at(bp, &mut nodes); pieces.push(Piece { a: m, b, nps: nps_of(bb).or(n0), to_nps: None, rigid: None }); }
                    tees.push(m);
                }
                _ if k.starts_with("REDUCER") && ends.len() == 2 => {
                    let (a, b) = (node_at(ends[0].0, &mut nodes), node_at(ends[1].0, &mut nodes));
                    let n0 = nps(ends[0].1);
                    let n1 = nps_of(ends[1].1);
                    if n1.is_some() { last_nps = n1; }
                    pieces.push(Piece { a, b, nps: n0, to_nps: n1.filter(|x| Some(*x) != n0), rigid: None });
                }
                _ if ends.len() == 2 => {
                    let (a, b) = (node_at(ends[0].0, &mut nodes), node_at(ends[1].0, &mut nodes));
                    let n0 = nps(ends[0].1);
                    let rigid = if k.starts_with("VALVE") { Some(Rigid::Valve) } else if k.starts_with("FLANGE") { Some(Rigid::Flange) } else if k.starts_with("PIPE") || matches!(k, "COUPLING" | "UNION" | "GASKET" | "WELD" | "BOLT") { None } else if k.starts_with("INSTRUMENT") || k.starts_with("FILTER") || k.starts_with("TRAP") || k.starts_with("MISC") { Some(Rigid::Rigid) } else { *unknown.entry(c.kind.clone()).or_default() += 1; None };
                    pieces.push(Piece { a, b, nps: n0, to_nps: None, rigid });
                }
                _ if !ends.is_empty() || centre.is_some() => { *unknown.entry(c.kind.clone()).or_default() += 1; }
                _ => {}
            }
        }
        for (k, n) in &unknown { warnings.push(format!("{n} × {k}: read as plain pipe where it has two ends, otherwise left out.")); }
        pieces.retain(|p| p.a != p.b && len(sub(nodes[p.b], nodes[p.a])) >= 1e-4);
        let splits: Vec<usize> = sups.iter().copied().chain(tees.iter().copied()).collect();
        for &n in &splits {
            let touches = pieces.iter().any(|p| p.a == n || p.b == n);
            let q = nodes[n];
            if let Some(i) = pieces.iter().position(|p| {
                let (a, b) = (nodes[p.a], nodes[p.b]);
                let d = sub(b, a);
                let t = dot(sub(q, a), d) / dot(d, d);
                t > 1e-6 && t < 1.0 - 1e-6 && len(sub(q, [a[0] + d[0] * t, a[1] + d[1] * t, a[2] + d[2] * t])) < 1e-3 && p.to_nps.is_none()
            }) {
                let p = pieces.remove(i);
                pieces.push(Piece { a: p.a, b: n, nps: p.nps, to_nps: None, rigid: p.rigid });
                pieces.push(Piece { a: n, b: p.b, nps: p.nps, to_nps: None, rigid: p.rigid });
            } else if !touches {
                warnings.push(format!("A support or branch at ({:.0}, {:.0}, {:.0}) mm is not on the piping: left out.", q[0] * 1e3, q[1] * 1e3, q[2] * 1e3));
            }
        }
        if pieces.is_empty() { return Err("No piping could be read from this file.".into()); }
        if pieces.iter().any(|p| p.nps.is_none()) { warnings.push("Some components had no bore that matches a library size: they use 6\" Std.".into()); }
        if sups.is_empty() { warnings.push("The file has no supports: add anchors and supports before analysing.".into()); }
        Ok(Read { name, nodes, pieces, bends, tees, warnings, components: comps.len() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_size_is_a_bore_pipe_reads_except_22_inch() {
        for n in crate::catalogue::NPS {
            let d = dn(n) as f64;
            let back = reader::MM_NPS.iter().min_by(|a, b| (a.0 - d).abs().total_cmp(&(b.0 - d).abs())).filter(|x| (x.0 - d).abs() <= 0.15 * x.0).map(|x| x.1);
            if n == "22" { assert_eq!(back, Some("20"), "Pipe's library has no 22\" bore (a known limit)"); } else { assert_eq!(back, Some(n)); }
        }
    }
}
