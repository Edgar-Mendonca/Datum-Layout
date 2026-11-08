//! The catalogue: dimension tables and pipe classes read from plain CSV and JSON. The built-in
//! tables are the files in `catalogue/` (compiled in); plug-in packs (a folder the user adds) are
//! read by the same functions and their rows add to or replace the built-in ones. Every row keeps
//! its source; rows that cannot be read are reported, never fatal.

use serde::{Deserialize, Serialize};

pub const IN: f64 = 0.0254;

/// The flange pressure classes the catalogue covers.
pub const RATINGS: [u32; 6] = [150, 300, 600, 900, 1500, 2500];

/// Nominal sizes in order (the catalogue covers these).
pub const NPS: [&str; 21] = ["1/2", "3/4", "1", "1-1/4", "1-1/2", "2", "2-1/2", "3", "3-1/2", "4", "5", "6", "8", "10", "12", "14", "16", "18", "20", "22", "24"];

/// The nominal size as inches ("1-1/2" → 1.5).
pub fn nps_inches(nps: &str) -> Option<f64> {
    let frac = |s: &str| -> Option<f64> {
        match s.split_once('/') {
            Some((a, b)) => Some(a.parse::<f64>().ok()? / b.parse::<f64>().ok().filter(|b| *b != 0.0)?),
            None => s.parse().ok(),
        }
    };
    match nps.split_once('-') {
        Some((w, f)) => Some(w.parse::<f64>().ok()? + frac(f)?),
        None => frac(nps),
    }
}

/// Position of a size in [`NPS`] (sizes not in the list sort last).
pub fn nps_index(nps: &str) -> usize {
    NPS.iter().position(|n| *n == nps).unwrap_or(NPS.len())
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct PipeRow { pub nps: String, pub dn: u32, pub od: f64, pub schedule: String, pub wall: f64, pub source: String }
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct FittingRow { pub nps: String, pub elbow_lr: f64, pub elbow_sr: Option<f64>, pub tee_c: f64, pub reducer_h: f64, pub cap_e: f64, pub source: String }
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct FlangeRow { pub nps: String, pub rating: u32, pub od: f64, pub thick: f64, pub wn_length: f64, pub raised_face: f64, pub bolts: u32, pub bolt: f64, pub gasket: f64, pub source: String,
    /// The blind flange's thickness where it differs from the weld-neck's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blind_thick: Option<f64> }
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ValveRow { pub kind: String, pub rating: u32, pub nps: String, pub f2f: f64, pub source: String }

/// One component a class allows, with the sizes it is allowed in.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct ClassItem { pub kind: String, pub min: Option<String>, pub max: Option<String> }

/// A pipe class (piping specification): material, flange ratings, schedules and allowed components.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct PipeClass {
    pub name: String,
    pub description: String,
    pub material: String,
    pub ratings: Vec<u32>,
    pub sizes: Vec<String>,
    /// (from size, schedule): the schedule applies from that size up to the next entry.
    pub schedules: Vec<(String, String)>,
    pub elbow: String,
    pub components: Vec<ClassItem>,
    pub source: String,
}

impl PipeClass {
    pub fn schedule(&self, nps: &str) -> String {
        let i = nps_index(nps);
        self.schedules.iter().rfind(|(from, _)| nps_index(from) <= i).map(|s| s.1.clone()).unwrap_or_else(|| "STD".into())
    }
    /// Whether the class allows `kind` in size `nps`.
    pub fn allows(&self, kind: &str, nps: &str) -> bool {
        let i = nps_index(nps);
        self.sizes.iter().any(|s| s == nps)
            && self.components.iter().any(|c| c.kind == kind && c.min.as_deref().is_none_or(|m| nps_index(m) <= i) && c.max.as_deref().is_none_or(|m| i <= nps_index(m)))
    }
}

/// The whole catalogue: every table and class, and what could not be read.
#[derive(Clone, Debug, Serialize, Default)]
pub struct Catalogue {
    pub pipes: Vec<PipeRow>,
    pub fittings: Vec<FittingRow>,
    pub flanges: Vec<FlangeRow>,
    pub valves: Vec<ValveRow>,
    pub classes: Vec<PipeClass>,
    pub packs: Vec<String>,
    pub warnings: Vec<String>,
}

/// Splits one CSV line (quotes allowed, `""` inside quotes).
fn split(line: &str) -> Vec<String> {
    let (mut out, mut cur, mut q, mut chars) = (vec![], String::new(), false, line.chars().peekable());
    while let Some(c) = chars.next() {
        match c {
            '"' if q && chars.peek() == Some(&'"') => { cur.push('"'); chars.next(); }
            '"' => q = !q,
            ',' if !q => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out.into_iter().map(|s| s.trim().to_string()).collect()
}

/// Reads a CSV table with a header into maps of column → text; bad rows become warnings.
fn table(name: &str, text: &str, need: &[&str], warnings: &mut Vec<String>) -> Vec<std::collections::HashMap<String, String>> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'));
    let Some(head) = lines.next() else { warnings.push(format!("{name}: the file is empty.")); return vec![] };
    let head: Vec<String> = split(head.trim_start_matches('\u{feff}')).into_iter().map(|h| h.to_lowercase()).collect();
    if let Some(m) = need.iter().find(|n| !head.iter().any(|h| h == *n)) {
        warnings.push(format!("{name}: the column \"{m}\" is missing, so the file was not used."));
        return vec![];
    }
    let mut rows = vec![];
    for (i, l) in lines.enumerate().take(100_000) {
        let cells = split(l);
        let row: std::collections::HashMap<String, String> = head.iter().cloned().zip(cells.into_iter().chain(std::iter::repeat(String::new()))).collect();
        if row.get("source").is_none_or(|s| s.is_empty()) {
            warnings.push(format!("{name}, row {}: no source given, so the row was not used.", i + 2));
            continue;
        }
        rows.push(row);
    }
    rows
}

fn f(row: &std::collections::HashMap<String, String>, k: &str) -> Option<f64> {
    row.get(k)?.parse::<f64>().ok().filter(|v| v.is_finite() && *v > 0.0)
}
fn size_ok(n: &str) -> bool {
    NPS.contains(&n)
}

impl Catalogue {
    /// The built-in catalogue (the files in `catalogue/`).
    pub fn builtin() -> Catalogue {
        let mut c = Catalogue::default();
        c.add_table("pipe.csv", include_str!("../../../catalogue/pipe.csv"));
        c.add_table("fittings.csv", include_str!("../../../catalogue/fittings.csv"));
        c.add_table("flanges.csv", include_str!("../../../catalogue/flanges.csv"));
        c.add_table("valves.csv", include_str!("../../../catalogue/valves.csv"));
        c.add_class("classes/CS1.json", include_str!("../../../catalogue/classes/CS1.json"));
        c.add_class("classes/CS150.json", include_str!("../../../catalogue/classes/CS150.json"));
        c.add_class("classes/CS300.json", include_str!("../../../catalogue/classes/CS300.json"));
        c.add_class("classes/CS600.json", include_str!("../../../catalogue/classes/CS600.json"));
        c.add_class("classes/CS1500.json", include_str!("../../../catalogue/classes/CS1500.json"));
        c.add_class("classes/LT150.json", include_str!("../../../catalogue/classes/LT150.json"));
        c.add_class("classes/SS150.json", include_str!("../../../catalogue/classes/SS150.json"));
        c.add_class("classes/UW150.json", include_str!("../../../catalogue/classes/UW150.json"));
        c
    }

    /// Adds one table by its file name; rows replace built-in rows of the same key.
    pub fn add_table(&mut self, name: &str, text: &str) {
        let base = name.rsplit(['/', '\\']).next().unwrap_or(name).to_lowercase();
        let w = &mut self.warnings;
        let bad = |w: &mut Vec<String>, i: usize, why: &str| w.push(format!("{name}, row {}: {why}, so the row was not used.", i + 2));
        match base.as_str() {
            "pipe.csv" => for (i, r) in table(name, text, &["nps", "od_in", "schedule", "wall_in", "source"], w).into_iter().enumerate() {
                let (n, od, wall) = (r["nps"].clone(), f(&r, "od_in"), f(&r, "wall_in"));
                match (size_ok(&n), od, wall) {
                    (true, Some(od), Some(t)) if t * 2.0 < od => {
                        let row = PipeRow { dn: r.get("dn").and_then(|d| d.parse().ok()).unwrap_or(0), nps: n, od: od * IN, schedule: r["schedule"].to_uppercase(), wall: t * IN, source: r["source"].clone() };
                        self.pipes.retain(|p| !(p.nps == row.nps && p.schedule == row.schedule));
                        self.pipes.push(row);
                    }
                    _ => bad(w, i, "the size, diameter or wall is not valid"),
                }
            },
            "fittings.csv" => for (i, r) in table(name, text, &["nps", "elbow_lr_in", "tee_c_in", "reducer_h_in", "cap_e_in", "source"], w).into_iter().enumerate() {
                match (size_ok(&r["nps"]), f(&r, "elbow_lr_in"), f(&r, "tee_c_in"), f(&r, "reducer_h_in"), f(&r, "cap_e_in")) {
                    (true, Some(lr), Some(c), Some(h), Some(e)) => {
                        let row = FittingRow { nps: r["nps"].clone(), elbow_lr: lr * IN, elbow_sr: f(&r, "elbow_sr_in").map(|v| v * IN), tee_c: c * IN, reducer_h: h * IN, cap_e: e * IN, source: r["source"].clone() };
                        self.fittings.retain(|p| p.nps != row.nps);
                        self.fittings.push(row);
                    }
                    _ => bad(w, i, "a size or dimension is not valid"),
                }
            },
            "flanges.csv" => for (i, r) in table(name, text, &["nps", "rating", "od_in", "thick_in", "wn_length_in", "raised_face_in", "bolts", "bolt_in", "source"], w).into_iter().enumerate() {
                let rating = r["rating"].parse::<u32>().ok().filter(|v| *v > 0 && *v <= 2500);
                let bolts = r["bolts"].parse::<u32>().ok().filter(|v| (2..=64).contains(v));
                match (size_ok(&r["nps"]), rating, f(&r, "od_in"), f(&r, "thick_in"), f(&r, "wn_length_in"), f(&r, "raised_face_in"), bolts, f(&r, "bolt_in")) {
                    (true, Some(rating), Some(od), Some(t), Some(y), Some(rf), Some(nb), Some(bd)) if y > t && rf < od => {
                        let row = FlangeRow { nps: r["nps"].clone(), rating, od: od * IN, thick: t * IN, wn_length: y * IN, raised_face: rf * IN, bolts: nb, bolt: bd * IN, gasket: f(&r, "gasket_in").unwrap_or(0.125) * IN, source: r["source"].clone(), blind_thick: f(&r, "blind_thick_in").map(|v| v * IN) };
                        self.flanges.retain(|p| !(p.nps == row.nps && p.rating == row.rating));
                        self.flanges.push(row);
                    }
                    _ => bad(w, i, "a size, rating or dimension is not valid"),
                }
            },
            "valves.csv" => for (i, r) in table(name, text, &["kind", "rating", "nps", "face_to_face_in", "source"], w).into_iter().enumerate() {
                let kind = r["kind"].to_lowercase();
                match (["gate", "globe", "check", "ball"].contains(&kind.as_str()), r["rating"].parse::<u32>().ok(), size_ok(&r["nps"]), f(&r, "face_to_face_in")) {
                    (true, Some(rating), true, Some(l)) => {
                        let row = ValveRow { kind, rating, nps: r["nps"].clone(), f2f: l * IN, source: r["source"].clone() };
                        self.valves.retain(|v| !(v.kind == row.kind && v.rating == row.rating && v.nps == row.nps));
                        self.valves.push(row);
                    }
                    _ => bad(w, i, "the kind, rating, size or length is not valid"),
                }
            },
            _ => w.push(format!("{name}: not a catalogue table Datum Layout knows (pipe, fittings, flanges or valves .csv).")),
        }
    }

    /// Adds one pipe class (JSON); a class of the same name is replaced.
    pub fn add_class(&mut self, name: &str, text: &str) {
        match serde_json::from_str::<PipeClass>(text) {
            Ok(mut c) if !c.name.trim().is_empty() => {
                c.sizes.retain(|s| size_ok(s));
                c.ratings.retain(|r| *r > 0 && *r <= 2500);
                if c.ratings.is_empty() { c.ratings.push(150); }
                if c.source.trim().is_empty() { self.warnings.push(format!("{name}: the class has no source; it was used, marked \"source not given\".")); c.source = "source not given".into(); }
                self.classes.retain(|x| x.name != c.name);
                self.classes.push(c);
            }
            Ok(_) => self.warnings.push(format!("{name}: the class has no name, so it was not used.")),
            Err(e) => self.warnings.push(format!("{name}: not a pipe class that can be read ({e}).")),
        }
    }

    /// Adds a plug-in pack: (relative file name, text) pairs from its folder.
    pub fn add_pack(&mut self, pack: &str, files: &[(String, String)]) {
        let before = self.warnings.len();
        for (name, text) in files {
            let lower = name.to_lowercase();
            if lower.ends_with(".json") { self.add_class(&format!("{pack}/{name}"), text); }
            else if lower.ends_with(".csv") { self.add_table(&format!("{pack}/{name}"), text); }
        }
        if files.is_empty() { self.warnings.push(format!("{pack}: the pack has no .csv or .json files.")); }
        self.packs.push(format!("{pack}{}", if self.warnings.len() > before { " (with warnings)" } else { "" }));
    }

    pub fn class(&self, name: &str) -> Option<&PipeClass> {
        self.classes.iter().find(|c| c.name == name)
    }
    /// Outside diameter and wall of a size in a class's schedule (falling back to STD, then any).
    pub fn pipe(&self, nps: &str, class: &str) -> Option<&PipeRow> {
        let sch = self.class(class).map(|c| c.schedule(nps)).unwrap_or_else(|| "STD".into());
        let rows = self.pipes.iter().filter(|p| p.nps == nps);
        self.pipes.iter().find(|p| p.nps == nps && p.schedule == sch).or_else(|| self.pipes.iter().find(|p| p.nps == nps && p.schedule == "STD")).or_else(|| rows.into_iter().next())
    }
    pub fn od(&self, nps: &str) -> f64 {
        self.pipes.iter().find(|p| p.nps == nps).map(|p| p.od).unwrap_or_else(|| nps_inches(nps).unwrap_or(6.0) * IN * 1.1)
    }
    pub fn fitting(&self, nps: &str) -> Option<&FittingRow> {
        self.fittings.iter().find(|p| p.nps == nps)
    }
    pub fn flange(&self, nps: &str, rating: u32) -> Option<&FlangeRow> {
        self.flanges.iter().find(|p| p.nps == nps && p.rating == rating)
    }
    pub fn valve(&self, kind: &str, nps: &str, rating: u32) -> Option<&ValveRow> {
        self.valves.iter().find(|v| v.kind == kind && v.nps == nps && v.rating == rating)
    }
    /// A valve's face-to-face length: from the table, or where the table has no row for that size,
    /// interpolated by nominal size between the nearest sizes listed for the same kind and class.
    pub fn valve_length(&self, kind: &str, nps: &str, rating: u32) -> Option<f64> {
        if let Some(v) = self.valve(kind, nps, rating) { return Some(v.f2f); }
        let x = nps_inches(nps)?;
        let mut pts: Vec<(f64, f64)> = self.valves.iter().filter(|v| v.kind == kind && v.rating == rating).filter_map(|v| Some((nps_inches(&v.nps)?, v.f2f))).collect();
        pts.sort_by(|a, b| a.0.total_cmp(&b.0));
        let lo = pts.iter().rev().find(|p| p.0 < x)?;
        let hi = pts.iter().find(|p| p.0 > x)?;
        Some(lo.1 + (hi.1 - lo.1) * (x - lo.0) / (hi.0 - lo.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_builtin_catalogue_reads_cleanly_and_covers_every_size() {
        let c = Catalogue::builtin();
        assert!(c.warnings.is_empty(), "{:?}", c.warnings);
        for n in NPS {
            assert!(c.pipe(n, "CS1").is_some(), "pipe {n}");
            assert!(c.fitting(n).is_some(), "fittings {n}");
            for r in [150, 300] {
                assert!(c.flange(n, r).is_some(), "flange {n} {r}");
                for k in ["gate", "check"] { assert!(c.valve(k, n, r).is_some() || n == "3-1/2", "{k} {n} {r}"); }
            }
            // The higher classes exist in every size their flanges are made in.
            for r in RATINGS { if c.flange(n, r).is_none() { assert!(["3-1/2", "22", "14", "16", "18", "20", "24"].contains(&n), "flange {n} {r}"); } }
        }
        // 6" Std: 168.3 mm × 7.11 mm.
        let p = c.pipe("6", "CS1").unwrap();
        assert!((p.od - 0.168_275).abs() < 1e-9 && (p.wall - 0.007_112).abs() < 1e-9 && p.schedule == "40");
        let cs1 = c.class("CS1").unwrap();
        assert_eq!(cs1.schedule("1"), "80");
        assert_eq!(cs1.schedule("4"), "40");
        assert_eq!(cs1.schedule("12"), "STD");
        assert!(cs1.allows("gate", "6") && !cs1.allows("gate", "1") && !cs1.allows("ball", "16") && cs1.allows("elbow90", "24"));
        assert!(c.pipes.iter().all(|r| !r.source.is_empty()));
        assert_eq!(c.classes.len(), 8);
        // Every class's schedules exist in the pipe table for its sizes (so no part falls back silently).
        for cl in &c.classes { for s in &cl.sizes { assert!(c.pipes.iter().any(|p| p.nps == *s && p.schedule == cl.schedule(s)), "{} {s} {}", cl.name, cl.schedule(s)); } }
        // Flanges and valves for every class 150–2500 that is made; the blind may be thicker.
        assert!(c.flange("12", 2500).is_some() && c.flange("24", 1500).is_some() && c.flange("14", 2500).is_none());
        assert!(c.flanges.iter().all(|f| f.blind_thick.is_none_or(|b| b >= f.thick * 0.5)));
        assert!(c.valve("gate", "6", 900).is_some() && c.valve("ball", "5", 600).is_some());
        // A missing size is interpolated between its neighbours: 3-1/2" gate, class 300, between 3" and 4".
        let v = c.valve_length("gate", "3-1/2", 300).unwrap() / IN;
        assert!(v > 11.12 && v < 12.0, "{v}");
    }

    #[test]
    fn a_bad_pack_is_used_as_far_as_it_can_be() {
        let mut c = Catalogue::builtin();
        let n = c.flanges.len();
        let files = vec![
            ("flanges.csv".to_string(), "nps,rating,od_in,thick_in,wn_length_in,raised_face_in,bolts,bolt_in,source\n6,150,11.5,1.0,3.5,8.5,8,0.75,Vendor X catalogue\n6,150,abc,1,1,1,8,1,src\n7,150,1,1,2,0.5,4,1,src\n6,600,13,1,2,8,12,1,\n".to_string()),
            ("classes/bad.json".to_string(), "{ nope".to_string()),
            ("valves.csv".to_string(), "kind,rating\n".to_string()),
            ("readme.txt".to_string(), "hello".to_string()),
        ];
        c.add_pack("MyPack", &files);
        assert_eq!(c.flanges.len(), n, "the good row replaced the built-in one");
        assert!((c.flange("6", 150).unwrap().od - 11.5 * IN).abs() < 1e-12);
        assert_eq!(c.flange("6", 150).unwrap().source, "Vendor X catalogue");
        assert_eq!(c.warnings.len(), 5, "{:?}", c.warnings);
        assert_eq!(c.packs, ["MyPack (with warnings)"]);
        // Garbage of every kind never panics.
        for t in ["", "\u{feff}", "\"", "a,b,c\n\"\"\"", "nps\n,,,,,,,,,,,,,,,,"] {
            for name in ["pipe.csv", "fittings.csv", "flanges.csv", "valves.csv", "x.json"] {
                let mut c = Catalogue::builtin();
                if name.ends_with("json") { c.add_class(name, t) } else { c.add_table(name, t) }
            }
        }
    }
}
