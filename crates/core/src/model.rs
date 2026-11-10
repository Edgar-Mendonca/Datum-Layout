//! The plant model: the project, its plot and areas, equipment with nozzles, and lines with their
//! routed path and the components placed on them. Every object has a stable id (and a uid for
//! exchange), after Datum Schema's object model; names follow DEXPI where there is one. The model
//! holds only what the designer decides; parts, ports and checks are derived from it by `route`.
//!
//! Coordinates are metres: X east, Y north, Z up (elevation). `.dlayout` files are this model as
//! JSON with a `format` number; older formats are migrated and damaged files are read item by
//! item (`from_json`), keeping what can be read and listing what could not.

use serde::{Deserialize, Serialize};

pub type P3 = [f64; 3];
pub const FORMAT: u32 = 1;
/// No plant is larger than this (m) in any direction; positions beyond it are clamped on load.
pub const HORIZON: f64 = 100_000.0;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Plot {
    /// Grid spacing (m) for snapping and the drawn grid.
    pub grid: f64,
    /// The plot's extent: east and north from–to (m).
    pub e: [f64; 2],
    pub n: [f64; 2],
    /// Plant north, degrees clockwise from the +Y axis (0: plant north is grid north).
    pub north: f64,
    /// The ground elevation (m).
    pub ground: f64,
}
impl Default for Plot {
    fn default() -> Self { Self { grid: 1.0, e: [0.0, 40.0], n: [0.0, 30.0], north: 0.0, ground: 0.0 } }
}

/// An area of the plot (a box), the level below the plant in the hierarchy plant › unit › area.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Area { pub id: u32, pub uid: String, pub name: String, pub unit: String, pub min: P3, pub max: P3 }

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EquipKind {
    #[default]
    VerticalVessel,
    HorizontalDrum,
    Tank,
    Column,
    Pump,
    Exchanger,
}

impl EquipKind {
    pub const ALL: [EquipKind; 6] = [EquipKind::VerticalVessel, EquipKind::HorizontalDrum, EquipKind::Tank, EquipKind::Column, EquipKind::Pump, EquipKind::Exchanger];
    pub fn name(self) -> &'static str {
        match self {
            EquipKind::VerticalVessel => "Vertical vessel",
            EquipKind::HorizontalDrum => "Horizontal drum",
            EquipKind::Tank => "Storage tank",
            EquipKind::Column => "Column",
            EquipKind::Pump => "Centrifugal pump",
            EquipKind::Exchanger => "Shell-and-tube exchanger",
        }
    }
    /// The DEXPI class name.
    pub fn class(self) -> &'static str {
        match self {
            EquipKind::VerticalVessel => "Vessel",
            EquipKind::HorizontalDrum => "Vessel",
            EquipKind::Tank => "Tank",
            EquipKind::Column => "ProcessColumn",
            EquipKind::Pump => "CentrifugalPump",
            EquipKind::Exchanger => "TubularHeatExchanger",
        }
    }
    pub fn prefix(self) -> &'static str {
        match self {
            EquipKind::VerticalVessel | EquipKind::HorizontalDrum => "V",
            EquipKind::Tank => "T",
            EquipKind::Column => "C",
            EquipKind::Pump => "P",
            EquipKind::Exchanger => "E",
        }
    }
}

/// An equipment nozzle: its flange face point and outward direction in the equipment's own axes
/// (before its rotation), size, flange rating and facing.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Nozzle {
    pub tag: String,
    pub nps: String,
    pub rating: u32,
    pub facing: String,
    pub pos: P3,
    pub dir: P3,
    /// Neck length from the shell to the flange face (m), for the drawing.
    pub proj: f64,
    pub service: String,
}
impl Default for Nozzle {
    fn default() -> Self { Self { tag: "N1".into(), nps: "4".into(), rating: 150, facing: "RF".into(), pos: [0.0; 3], dir: [1.0, 0.0, 0.0], proj: 0.2, service: String::new() } }
}

/// Main sizes of a parametric equipment item (m); what each means depends on the kind
/// (`equipment::fields`).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Dims { pub d: f64, pub l: f64, pub h: f64, pub elev: f64, pub d2: f64, pub l2: f64, pub w: f64 }
impl Default for Dims {
    fn default() -> Self { Self { d: 1.5, l: 4.0, h: 4.0, elev: 1.0, d2: 0.5, l2: 1.0, w: 1.0 } }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Equipment {
    pub id: u32,
    pub uid: String,
    pub tag: String,
    pub kind: EquipKind,
    /// The equipment's origin on the plot (the centre of its footprint at its base).
    pub pos: P3,
    /// Rotation about the vertical, degrees anticlockwise seen from above.
    pub rot: f64,
    pub dims: Dims,
    pub nozzles: Vec<Nozzle>,
    /// Datasheet values: (label, value with unit), as the designer typed them.
    pub data: Vec<(String, String)>,
    pub area: u32,
}

/// What ends a line at a free point.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Term { #[default] Open, Cap, Blind }

/// Where a line starts or ends.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum End {
    /// A free point (the path's end), open or closed with a cap or a blind flange.
    Point { term: Term },
    /// An equipment nozzle, by equipment id and nozzle tag.
    Nozzle { equip: u32, nozzle: String },
    /// The branch of a tee (or one of a cross's two) placed on another line.
    Branch {
        line: u32,
        item: u32,
        /// A cross's branch: 0 the first (towards its direction), 1 the opposite one.
        #[serde(default)]
        port: u8,
    },
}
impl Default for End {
    fn default() -> Self { End::Point { term: Term::Open } }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    #[default]
    Gate,
    Globe,
    Check,
    Ball,
    Strainer,
    FlangePair,
    Tee,
    Reducer,
    /// A cross: a tee with a second branch opposite the first.
    Cross,
    /// A butt-weld branch outlet on the header (it does not cut the header into a tee).
    Weldolet,
    /// A socket-weld branch outlet (class 3000) for small branches.
    Sockolet,
    /// A pipe support at a point of a run (it takes no length; the pipe is split there so the
    /// stress model has a node).
    Support,
    /// A butterfly valve (double flanged, short pattern): a short body, a disc on a stem, a gearbox.
    Butterfly,
}

/// A reducer's flat side: none (concentric), on top or at the bottom (eccentric).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Flat {
    #[default]
    None,
    Top,
    Bottom,
}

/// What a pipe support holds: a rest carries the weight, a guide also stops sideways movement,
/// a line stop stops movement along the pipe, an anchor holds it fixed.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SupportKind {
    #[default]
    Rest,
    Guide,
    Stop,
    Anchor,
}
impl SupportKind {
    pub const ALL: [SupportKind; 4] = [SupportKind::Rest, SupportKind::Guide, SupportKind::Stop, SupportKind::Anchor];
    pub fn name(self) -> &'static str {
        match self { SupportKind::Rest => "Rest (shoe)", SupportKind::Guide => "Guide", SupportKind::Stop => "Line stop", SupportKind::Anchor => "Anchor" }
    }
    /// The SKEY written to the piping component file (Datum Pipe reads ANC, GUI and LST).
    pub fn skey(self) -> &'static str {
        match self { SupportKind::Rest => "REST", SupportKind::Guide => "GUI", SupportKind::Stop => "LST", SupportKind::Anchor => "ANC" }
    }
}
impl ItemKind {
    pub fn key(self) -> &'static str {
        match self {
            ItemKind::Gate => "gate",
            ItemKind::Globe => "globe",
            ItemKind::Check => "check",
            ItemKind::Ball => "ball",
            ItemKind::Strainer => "strainer",
            ItemKind::FlangePair => "flange",
            ItemKind::Tee => "tee",
            ItemKind::Reducer => "reducer",
            ItemKind::Cross => "cross",
            ItemKind::Weldolet => "weldolet",
            ItemKind::Sockolet => "sockolet",
            ItemKind::Support => "support",
            ItemKind::Butterfly => "butterfly",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            ItemKind::Gate => "Gate valve",
            ItemKind::Globe => "Globe valve",
            ItemKind::Check => "Check valve",
            ItemKind::Ball => "Ball valve",
            ItemKind::Strainer => "Strainer",
            ItemKind::FlangePair => "Flange pair",
            ItemKind::Tee => "Tee",
            ItemKind::Reducer => "Concentric reducer",
            ItemKind::Cross => "Cross",
            ItemKind::Weldolet => "Weldolet",
            ItemKind::Sockolet => "Sockolet",
            ItemKind::Support => "Pipe support",
            ItemKind::Butterfly => "Butterfly valve",
        }
    }
    /// How many branches it has: tees and outlets one, a cross two.
    pub fn branches(self) -> u8 {
        match self { ItemKind::Tee | ItemKind::Weldolet | ItemKind::Sockolet => 1, ItemKind::Cross => 2, _ => 0 }
    }
    pub fn olet(self) -> bool { matches!(self, ItemKind::Weldolet | ItemKind::Sockolet) }
    pub fn valve(self) -> bool {
        matches!(self, ItemKind::Gate | ItemKind::Globe | ItemKind::Check | ItemKind::Ball | ItemKind::Butterfly | ItemKind::Strainer)
    }
    pub fn flanged(self) -> bool {
        self.valve() || self == ItemKind::FlangePair
    }
}

/// A component placed on a line: on path segment `seg`, its centre `at` metres from the
/// segment's start.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct Item {
    pub id: u32,
    pub uid: String,
    pub kind: ItemKind,
    pub seg: usize,
    pub at: f64,
    /// Reducers: the size after it. Tees: the branch size when no branch line is connected.
    pub nps2: String,
    /// Tees: the branch direction (unit, square to the run). Valves: where the stem points.
    pub dir: P3,
    pub tag: String,
    /// Supports: what it holds.
    pub support: SupportKind,
    /// Reducers: concentric, or eccentric with its flat side on top or at the bottom. An eccentric
    /// reducer is its own short sloped run of the path (its ends are offset by half the change
    /// in outside diameter).
    pub flat: Flat,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Radius { #[default] Long, Short }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Line {
    pub id: u32,
    pub uid: String,
    /// The line number ("6\"-P-1001-CS1"); empty is reported by the checks.
    pub number: String,
    pub nps: String,
    pub class: String,
    pub rating: u32,
    pub fluid: String,
    /// Insulation thickness (m); 0 for none.
    pub insulation: f64,
    pub radius: Radius,
    pub from: End,
    pub to: End,
    /// The routed path from start to end: the centreline's corner points.
    pub pts: Vec<P3>,
    pub items: Vec<Item>,
    pub area: u32,
}
impl Default for Line {
    fn default() -> Self {
        Self { id: 0, uid: String::new(), number: String::new(), nps: "6".into(), class: "CS1".into(), rating: 150, fluid: "P".into(), insulation: 0.0, radius: Radius::Long, from: End::default(), to: End::default(), pts: vec![], items: vec![], area: 0 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    /// The clash check's clearance (m).
    pub clearance: f64,
    /// Default size, class and rating for new lines.
    pub nps: String,
    pub class: String,
    pub rating: u32,
}
impl Default for Settings {
    fn default() -> Self { Self { clearance: 0.025, nps: "6".into(), class: "CS1".into(), rating: 150 } }
}

/// A saved view: where the camera stood and what the view showed (plant axes, metres).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct SavedView {
    pub name: String,
    pub eye: P3,
    pub target: P3,
    /// "kind", "line", "class", "fluid" or "size".
    pub colour_by: String,
    pub hidden: Vec<u32>,
    pub isolated: Option<Vec<u32>>,
    /// The section box's low and high corners.
    pub section: Option<[P3; 2]>,
}

/// A line from the process line list: what the line should be, checked against the model.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub struct ListedLine {
    pub number: String,
    /// Nominal size; empty when the list's size could not be read.
    pub nps: String,
    pub class: String,
    /// 0 when the list gives none.
    pub rating: u32,
    pub fluid: String,
    /// Insulation thickness (m).
    pub insulation: f64,
    pub from: String,
    pub to: String,
    pub notes: String,
    /// The P&ID's stable id for the line (from a plant package).
    pub uid: String,
    /// The components on it in flow order, as the P&ID has them (gate, check, strainer…); empty
    /// when the list does not say.
    pub components: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Project {
    pub format: u32,
    pub app: String,
    pub name: String,
    pub notes: String,
    pub plant: String,
    pub unit: String,
    pub units: String,
    pub plot: Plot,
    pub areas: Vec<Area>,
    pub equipment: Vec<Equipment>,
    pub lines: Vec<Line>,
    pub settings: Settings,
    pub next: u32,
    pub views: Vec<SavedView>,
    pub line_list: Vec<ListedLine>,
    /// The plant's own pipe classes: new ones, or edited copies of the catalogue's (same name: the
    /// plant's wins). They travel with the file.
    pub classes: Vec<crate::catalogue::PipeClass>,
}
impl Default for Project {
    fn default() -> Self {
        Self { format: FORMAT, app: "Datum Layout".into(), name: "Plant1".into(), notes: String::new(), plant: "Plant".into(), unit: "Unit 100".into(), units: "SI".into(), plot: Plot::default(), areas: vec![], equipment: vec![], lines: vec![], settings: Settings::default(), next: 1, views: vec![], line_list: vec![], classes: vec![] }
    }
}

/// A uid for a new object: the project's id counter mixed with the clock, so copies of a project
/// made at different times never share uids.
pub fn new_uid(id: u32) -> String {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
    let mut x = t ^ ((id as u64) << 32 | id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    format!("{x:016x}-{id}")
}

impl Project {
    pub fn new() -> Project {
        Project::default()
    }
    pub fn equip(&self, id: u32) -> Option<&Equipment> {
        self.equipment.iter().find(|e| e.id == id)
    }
    pub fn line(&self, id: u32) -> Option<&Line> {
        self.lines.iter().find(|l| l.id == id)
    }
    pub fn max_id(&self) -> u32 {
        let ids = self.equipment.iter().map(|e| e.id).chain(self.areas.iter().map(|a| a.id)).chain(self.lines.iter().flat_map(|l| std::iter::once(l.id).chain(l.items.iter().map(|i| i.id))));
        ids.max().unwrap_or(0)
    }
    /// A fresh id.
    pub fn id(&mut self) -> u32 {
        self.next = self.next.max(self.max_id() + 1);
        self.next += 1;
        self.next - 1
    }

    /// Makes every value usable (finite, in range, ids unique, references that exist) and returns
    /// what it changed. Loading, every edit and the hardening tests go through this.
    pub fn normalize(&mut self) -> Vec<String> {
        let mut w = vec![];
        let fin = |v: &mut f64, lo: f64, hi: f64, def: f64| { if !v.is_finite() { *v = def } *v = v.clamp(lo, hi); };
        let p3 = |p: &mut P3| for v in p.iter_mut() { if !v.is_finite() { *v = 0.0 } *v = v.clamp(-HORIZON, HORIZON); };
        fin(&mut self.plot.grid, 0.001, 100.0, 1.0);
        for k in 0..2 { fin(&mut self.plot.e[k], -HORIZON, HORIZON, 0.0); fin(&mut self.plot.n[k], -HORIZON, HORIZON, 0.0); }
        if self.plot.e[1] <= self.plot.e[0] { self.plot.e = [self.plot.e[0], self.plot.e[0] + 40.0]; }
        if self.plot.n[1] <= self.plot.n[0] { self.plot.n = [self.plot.n[0], self.plot.n[0] + 30.0]; }
        fin(&mut self.plot.north, -360.0, 360.0, 0.0);
        fin(&mut self.plot.ground, -HORIZON, HORIZON, 0.0);
        fin(&mut self.settings.clearance, 0.0, 10.0, 0.025);
        if !crate::catalogue::NPS.contains(&self.settings.nps.as_str()) { self.settings.nps = "6".into(); }
        if self.units != "SI" && self.units != "US" { self.units = "SI".into(); }
        // Ids: unique and non-zero; anything that repeats an id gets a new one.
        let mut seen = std::collections::HashSet::new();
        let mut next = self.next.max(1);
        let mut fresh = |id: &mut u32, uid: &mut String, seen: &mut std::collections::HashSet<u32>, w: &mut Vec<String>, what: &str| {
            if *id == 0 || !seen.insert(*id) {
                if *id != 0 { w.push(format!("{what} had an id used twice; it was given a new one.")); }
                while seen.contains(&next) || next == 0 { next += 1; }
                *id = next;
                seen.insert(next);
            }
            if uid.is_empty() { *uid = new_uid(*id); }
        };
        self.views.truncate(500);
        for v in &mut self.views { p3(&mut v.eye); p3(&mut v.target); if let Some(s) = &mut v.section { p3(&mut s[0]); p3(&mut s[1]); } }
        self.line_list.truncate(100_000);
        for r in &mut self.line_list {
            if !r.nps.is_empty() && !crate::catalogue::NPS.contains(&r.nps.as_str()) { r.nps.clear(); }
            if r.rating != 0 && !crate::catalogue::RATINGS.contains(&r.rating) { r.rating = 0; }
            fin(&mut r.insulation, 0.0, 1.0, 0.0);
        }
        for a in &mut self.areas {
            fresh(&mut a.id, &mut a.uid, &mut seen, &mut w, "An area");
            p3(&mut a.min); p3(&mut a.max);
            for k in 0..3 { if a.max[k] < a.min[k] { std::mem::swap(&mut a.max[k], &mut a.min[k]); } }
        }
        for e in &mut self.equipment {
            fresh(&mut e.id, &mut e.uid, &mut seen, &mut w, &format!("Equipment {}", e.tag));
            p3(&mut e.pos);
            fin(&mut e.rot, -3600.0, 3600.0, 0.0);
            let d = &mut e.dims;
            for (v, lo, hi, def) in [(&mut d.d, 0.05, 100.0, 1.5), (&mut d.l, 0.05, 200.0, 4.0), (&mut d.h, 0.05, 200.0, 4.0), (&mut d.elev, 0.0, 100.0, 1.0), (&mut d.d2, 0.02, 50.0, 0.5), (&mut d.l2, 0.02, 50.0, 1.0), (&mut d.w, 0.05, 50.0, 1.0)] { fin(v, lo, hi, def); }
            e.nozzles.truncate(200);
            let mut tags = std::collections::HashSet::new();
            for (i, n) in e.nozzles.iter_mut().enumerate() {
                p3(&mut n.pos);
                for v in n.dir.iter_mut() { if !v.is_finite() { *v = 0.0 } }
                let l = (n.dir[0] * n.dir[0] + n.dir[1] * n.dir[1] + n.dir[2] * n.dir[2]).sqrt();
                n.dir = if l > 1e-9 { [n.dir[0] / l, n.dir[1] / l, n.dir[2] / l] } else { [1.0, 0.0, 0.0] };
                fin(&mut n.proj, 0.0, 20.0, 0.2);
                if !crate::catalogue::NPS.contains(&n.nps.as_str()) { w.push(format!("Nozzle {} of {} had an unknown size; it is now 2\".", n.tag, e.tag)); n.nps = "2".into(); }
                if !crate::catalogue::RATINGS.contains(&n.rating) { n.rating = 150; }
                if n.tag.trim().is_empty() || !tags.insert(n.tag.clone()) { n.tag = format!("N{}", i + 1); while !tags.insert(n.tag.clone()) { n.tag.push('\''); } }
            }
            e.data.truncate(200);
        }
        for l in &mut self.lines {
            fresh(&mut l.id, &mut l.uid, &mut seen, &mut w, &format!("Line {}", l.number));
            if !crate::catalogue::NPS.contains(&l.nps.as_str()) { w.push(format!("Line {} had an unknown size; it is now 6\".", l.number)); l.nps = "6".into(); }
            if !crate::catalogue::RATINGS.contains(&l.rating) { l.rating = 150; }
            fin(&mut l.insulation, 0.0, 1.0, 0.0);
            l.pts.truncate(10_000);
            l.pts.iter_mut().for_each(p3);
            l.items.truncate(10_000);
            for it in &mut l.items {
                fresh(&mut it.id, &mut it.uid, &mut seen, &mut w, "A line component");
                fin(&mut it.at, 0.0, 2.0 * HORIZON, 0.0);
                for v in it.dir.iter_mut() { if !v.is_finite() { *v = 0.0 } }
                if !it.nps2.is_empty() && !crate::catalogue::NPS.contains(&it.nps2.as_str()) { it.nps2.clear(); }
            }
        }
        // Lines need two points; items must sit on a segment that exists.
        for l in &mut self.lines {
            if l.pts.len() < 2 {
                w.push(format!("Line {} had no route; it was given a short one to edit.", if l.number.is_empty() { "without a number" } else { &l.number }));
                let p = l.pts.first().copied().unwrap_or([0.0, 0.0, 1.0]);
                l.pts = vec![p, [p[0] + 1.0, p[1], p[2]]];
            }
            let n = l.pts.len() - 1;
            let before = l.items.len();
            l.items.retain(|it| it.seg < n);
            if l.items.len() < before { w.push(format!("Line {}: {} component(s) were on a part of the route that no longer exists and were removed.", l.number, before - l.items.len())); }
        }
        // References: ends at nozzles, tees and lines that exist.
        let nozzles: std::collections::HashSet<(u32, String)> = self.equipment.iter().flat_map(|e| e.nozzles.iter().map(move |n| (e.id, n.tag.clone()))).collect();
        let tees: std::collections::HashSet<(u32, u32, u8)> = self.lines.iter().flat_map(|l| l.items.iter().filter(|i| i.kind.branches() > 0).flat_map(move |i| (0..i.kind.branches()).map(move |k| (l.id, i.id, k)))).collect();
        let mut used_branch = std::collections::HashSet::new();
        for l in &mut self.lines {
            for end in [&mut l.from, &mut l.to] {
                let ok = match end {
                    End::Point { .. } => true,
                    End::Nozzle { equip, nozzle } => nozzles.contains(&(*equip, nozzle.clone())),
                    End::Branch { line, item, port } => *line != l.id && tees.contains(&(*line, *item, *port)) && used_branch.insert((*line, *item, *port)),
                };
                if !ok {
                    w.push(format!("Line {}: an end pointed to something that does not exist (or is already used); it is now a free end.", l.number));
                    *end = End::Point { term: Term::Open };
                }
            }
        }
        for a in &mut self.areas { if a.name.trim().is_empty() { a.name = format!("Area {}", a.id); } }
        self.next = next.max(self.max_id() + 1);
        self.format = FORMAT;
        self.app = "Datum Layout".into();
        w
    }
}

/// The largest `.dlayout` file read (a 2,000-line plant is a few megabytes).
pub const MAX_FILE: u64 = 200 * 1024 * 1024;

pub fn to_json(p: &Project) -> String {
    serde_json::to_string_pretty(p).unwrap_or_default()
}

/// Reads a `.dlayout` file: whole if it can, otherwise item by item (a damaged equipment item or
/// line is left out and listed). Gives the project and what was changed or left out.
pub fn from_json(text: &str) -> Result<(Project, Vec<String>), String> {
    let v: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| format!("This is not a Datum Layout file that can be read ({e})."))?;
    let obj = v.as_object().ok_or("This is not a Datum Layout file: it holds no project.")?;
    if obj.get("app").and_then(|a| a.as_str()).is_some_and(|a| a != "Datum Layout") && !obj.contains_key("lines") {
        return Err("This file was written by another program, not Datum Layout.".into());
    }
    let format = obj.get("format").and_then(|f| f.as_u64()).unwrap_or(1);
    let mut w = vec![];
    if format > FORMAT as u64 {
        w.push(format!("The file was written by a newer Datum Layout (format {format}); what this version does not know was left out."));
    }
    let mut p: Project = match serde_json::from_value(v.clone()) {
        Ok(p) => p,
        Err(_) => {
            // Item by item.
            let mut p = Project::default();
            macro_rules! field { ($k:literal, $f:ident) => { if let Some(x) = obj.get($k) { match serde_json::from_value(x.clone()) { Ok(v) => p.$f = v, Err(_) => w.push(format!("The file's {} could not be read; the default was used.", $k)) } } }; }
            field!("name", name); field!("notes", notes); field!("plant", plant); field!("unit", unit); field!("units", units); field!("plot", plot); field!("settings", settings); field!("next", next);
            macro_rules! list { ($k:literal, $f:ident, $what:literal) => {
                if let Some(arr) = obj.get($k).and_then(|a| a.as_array()) {
                    for (i, x) in arr.iter().enumerate() {
                        match serde_json::from_value(x.clone()) { Ok(v) => p.$f.push(v), Err(e) => w.push(format!("{} {} could not be read and was left out ({e}).", $what, i + 1)) }
                    }
                }
            }; }
            list!("areas", areas, "Area"); list!("equipment", equipment, "Equipment item"); list!("lines", lines, "Line");
            p
        }
    };
    w.extend(p.normalize());
    Ok((p, w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_round_trips_and_damaged_parts_are_left_out() {
        let mut p = Project::new();
        let id = p.id();
        p.equipment.push(Equipment { id, tag: "V-101".into(), nozzles: vec![Nozzle::default()], ..Default::default() });
        let lid = p.id();
        p.lines.push(Line { id: lid, number: "6\"-P-1".into(), pts: vec![[0.0, 0.0, 1.0], [5.0, 0.0, 1.0]], from: End::Nozzle { equip: id, nozzle: "N1".into() }, ..Default::default() });
        assert!(p.normalize().is_empty());
        let (q, w) = from_json(&to_json(&p)).unwrap();
        assert!(w.is_empty(), "{w:?}");
        assert_eq!(p, q);
        // A broken line is left out; the rest opens.
        let mut v: serde_json::Value = serde_json::from_str(&to_json(&p)).unwrap();
        v["lines"].as_array_mut().unwrap().push(serde_json::json!({ "pts": "garbage" }));
        let (q, w) = from_json(&v.to_string()).unwrap();
        assert_eq!(q.lines.len(), 1);
        assert!(w.iter().any(|x| x.contains("Line 2")), "{w:?}");
        // A dangling reference becomes a free end.
        let mut r = p.clone();
        r.equipment.clear();
        let w = r.normalize();
        assert!(matches!(r.lines[0].from, End::Point { .. }) && !w.is_empty());
        assert!(from_json("[1,2]").is_err() && from_json("").is_err() && from_json("{\"app\":\"Other\"}").is_err());
    }
}
