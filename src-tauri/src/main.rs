//! Datum Layout desktop shell: owns the open plant, the catalogue (built-in and plug-in packs),
//! files, options and recovery, and talks to the window. All geometry and rules are in
//! `datum-core`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod update;

use datum_core::catalogue::Catalogue;
use datum_core::doc::{Doc, Op};
use datum_core::model::{self, Project};
use datum_core::route::{self, Issue};
use datum_core::{checks, equipment, examples, gltf, lists, pcf, scene, units};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_dialog::DialogExt;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Options {
    /// Minutes between recovery saves; 0 turns them off.
    autosave: u32,
    beta: bool,
    /// auto, light or dark.
    theme: String,
    open_last: bool,
    /// Units for new plants: SI or US.
    units: String,
    /// Level of detail of the 3D view: 0 low, 1 normal, 2 high.
    lod: u32,
    /// Clash clearance for new plants (m).
    clearance: f64,
    /// Show tags and line numbers in the 3D view.
    labels: bool,
    paper: String,
    landscape: bool,
    company: String,
    /// Plug-in pack folders.
    packs: Vec<String>,
}

impl Default for Options {
    fn default() -> Self {
        Self { autosave: 5, beta: false, theme: "auto".into(), open_last: false, units: "SI".into(), lod: 1, clearance: 0.025, labels: true, paper: "A4".into(), landscape: true, company: String::new(), packs: vec![] }
    }
}

struct App {
    doc: Doc,
    opts: Options,
    cat: Catalogue,
}

struct St(Mutex<App>);

impl St {
    /// A command that panicked must not lock every later one out, so a poisoned lock is reused.
    fn lock(&self) -> std::sync::MutexGuard<'_, App> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}
type R<T> = Result<T, String>;

/// Runs an action so that an internal error (a panic) becomes a message instead of closing the
/// app. Edits work on a copy of the plant, so the plant is left as it was.
fn guard<T>(f: impl FnOnce() -> R<T>) -> R<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|_| {
        Err("Datum Layout met an internal error and stopped that action; your plant is as it was. Details are in crash.log; please report it (Help › Report an Issue).".into())
    })
}

/// Reads a file of at most `max` bytes (invalid UTF-8 is replaced, not fatal).
fn read_limited(path: &Path, max: u64) -> R<String> {
    let len = std::fs::metadata(path).map_err(|e| format!("Could not open {}: {e}", path.display()))?.len();
    if len > max {
        return Err(format!("{} is {} MB; Datum Layout opens files up to {} MB.", path.display(), len / 1_048_576, max / 1_048_576));
    }
    std::fs::read(path).map(|b| String::from_utf8_lossy(&b).into_owned()).map_err(|e| format!("Could not open {}: {e}", path.display()))
}

/// Writes a file so that a crash or a full disk never leaves it half-written.
fn write_atomic(path: &Path, bytes: &[u8]) -> R<()> {
    use std::io::Write;
    let fail = |e: std::io::Error| format!("Could not save {}: {e}", path.display());
    let tmp = path.with_extension(format!("{}.saving", path.extension().and_then(|e| e.to_str()).unwrap_or("tmp")));
    let mut f = std::fs::File::create(&tmp).map_err(fail)?;
    if let Err(e) = f.write_all(bytes).and_then(|_| f.sync_all()) {
        drop(f);
        let _ = std::fs::remove_file(&tmp);
        return Err(fail(e));
    }
    drop(f);
    std::fs::rename(&tmp, path).map_err(|e| { let _ = std::fs::remove_file(&tmp); fail(e) })
}

/// What the window needs after every change: the plant, the document's state and the routing
/// and connection findings (clashes run on request).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Out {
    p: Project,
    path: Option<String>,
    dirty: bool,
    undo: bool,
    redo: bool,
    created: Vec<u32>,
    warnings: Vec<String>,
    issues: Vec<Issue>,
    /// Each line's resolved path (ends moved to their nozzles and tees).
    paths: Vec<(u32, Vec<model::P3>)>,
}

fn out(a: &App) -> Out {
    let b = route::build(&a.doc.p, &a.cat);
    let mut issues = b.issues.clone();
    issues.extend(checks::connection_issues(&a.doc.p, &a.cat, &b));
    let paths = a.doc.p.lines.iter().map(|l| (l.id, route::path(&a.doc.p, &a.cat, l))).collect();
    Out { p: a.doc.p.clone(), path: a.doc.path.clone(), dirty: a.doc.dirty, undo: a.doc.can_undo(), redo: a.doc.can_redo(), created: vec![], warnings: vec![], issues, paths }
}

fn dir(app: &AppHandle, data: bool) -> Option<PathBuf> {
    let d = if data { app.path().app_data_dir() } else { app.path().app_config_dir() }.ok()?;
    std::fs::create_dir_all(&d).ok()?;
    Some(d)
}

fn fresh(o: &Options, mut p: Project) -> Project {
    if p.equipment.is_empty() && p.lines.is_empty() {
        if o.units == "SI" || o.units == "US" { p.units = o.units.clone(); }
        if o.clearance.is_finite() && o.clearance >= 0.0 { p.settings.clearance = o.clearance; }
    }
    p
}

/// The catalogue: built in, then each plug-in pack folder (its .csv and .json files, and those in
/// a `classes` folder inside it).
fn load_catalogue(packs: &[String]) -> Catalogue {
    let mut c = Catalogue::builtin();
    for dir in packs {
        let d = Path::new(dir);
        let name = d.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| dir.clone());
        let mut files = vec![];
        for sub in [d.to_path_buf(), d.join("classes")] {
            let Ok(rd) = std::fs::read_dir(&sub) else { continue };
            for e in rd.flatten().take(500) {
                let path = e.path();
                let ext = path.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
                if ext == "csv" || ext == "json" {
                    if let Ok(t) = read_limited(&path, 20 * 1024 * 1024) {
                        let rel = if sub == d { path.file_name().unwrap_or_default().to_string_lossy().to_string() } else { format!("classes/{}", path.file_name().unwrap_or_default().to_string_lossy()) };
                        files.push((rel, t));
                    }
                }
            }
        }
        if !d.is_dir() { c.warnings.push(format!("{dir}: the pack's folder was not found.")); continue; }
        c.add_pack(&name, &files);
    }
    c
}

// ---------------------------------------------------------------- the plant

#[tauri::command]
fn state(st: State<St>) -> Out {
    out(&st.lock())
}

#[tauri::command]
fn apply(st: State<St>, op: Op) -> R<Out> {
    guard(|| {
        let mut a = st.lock();
        let cat = a.cat.clone();
        let created = a.doc.apply(op, &cat)?;
        let mut o = out(&a);
        o.created = created;
        Ok(o)
    })
}

#[tauri::command]
fn history(st: State<St>, redo: bool) -> Out {
    let mut a = st.lock();
    if redo { a.doc.redo(); } else { a.doc.undo(); }
    out(&a)
}

/// The 3D view's meshes at the chosen level of detail.
#[tauri::command]
fn scene(st: State<St>) -> R<scene::Scene> {
    guard(|| {
        let a = st.lock();
        let b = route::build(&a.doc.p, &a.cat);
        Ok(scene::scene(&a.doc.p, &a.cat, &b, a.opts.lod))
    })
}

/// Every check, clashes included (runs off the window's thread).
#[tauri::command]
async fn check(st: State<'_, St>) -> R<Vec<Issue>> {
    let (p, cat) = { let a = st.lock(); (a.doc.p.clone(), a.cat.clone()) };
    tauri::async_runtime::spawn_blocking(move || guard(|| { let b = route::build(&p, &cat); Ok(checks::run(&p, &cat, &b)) })).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn lists(st: State<St>) -> R<lists::Lists> {
    guard(|| { let a = st.lock(); Ok(lists::lists(&a.doc.p, &a.cat)) })
}

#[tauri::command]
fn examples_list() -> Vec<examples::Example> {
    examples::list()
}

/// A new plant: an example by id, or an empty plot.
#[tauri::command]
fn new_project(st: State<St>, example: Option<String>) -> R<Out> {
    let mut a = st.lock();
    let p = match example.as_deref() {
        Some(id) => examples::get(id).ok_or("That example does not exist.")?,
        None => Project::new(),
    };
    let p = fresh(&a.opts, p);
    a.doc = Doc::new(p);
    Ok(out(&a))
}

/// An equipment type, its name and its size fields (key, label).
type Kind = (model::EquipKind, &'static str, Vec<(&'static str, &'static str)>);

#[derive(Serialize)]
struct CatalogueOut {
    cat: Catalogue,
    fields: Vec<Kind>,
}

/// The catalogue and the equipment types with their size fields.
#[tauri::command]
fn catalogue(st: State<St>) -> CatalogueOut {
    let a = st.lock();
    CatalogueOut { cat: a.cat.clone(), fields: model::EquipKind::ALL.iter().map(|k| (*k, k.name(), equipment::fields(*k))).collect() }
}

/// Adds a plug-in pack folder (picked by the user) and reloads the catalogue.
#[tauri::command]
async fn add_pack(app: AppHandle, st: State<'_, St>) -> R<Option<Vec<String>>> {
    let Some(dir) = app.dialog().file().blocking_pick_folder().and_then(|f| f.into_path().ok()) else { return Ok(None) };
    let mut a = st.lock();
    let d = dir.display().to_string();
    if !a.opts.packs.contains(&d) { a.opts.packs.push(d); }
    a.cat = load_catalogue(&a.opts.packs);
    save_options(&app, &a.opts);
    Ok(Some(a.cat.warnings.clone()))
}

#[tauri::command]
fn remove_pack(app: AppHandle, st: State<St>, name: String) -> Vec<String> {
    let mut a = st.lock();
    a.opts.packs.retain(|p| !Path::new(p).file_name().is_some_and(|f| name.starts_with(&*f.to_string_lossy())));
    a.cat = load_catalogue(&a.opts.packs);
    save_options(&app, &a.opts);
    a.opts.packs.clone()
}

/// How every quantity is shown in SI and US units (from the engine's unit table).
#[tauri::command]
fn unit_table() -> (Vec<units::Shown>, Vec<units::Shown>, Vec<units::Typed>) {
    (units::shown(false), units::shown(true), units::typed())
}

// ---------------------------------------------------------------- exports

/// A line (or every line, `line` = None) as a piping component file, for Datum Pipe.
#[tauri::command]
fn export_pcf(st: State<St>, line: Option<u32>) -> R<String> {
    guard(|| { let a = st.lock(); pcf::write(&a.doc.p, &a.cat, line) })
}

/// The model as glTF 2.0 binary (a data URL for `save_file`).
#[tauri::command]
fn export_gltf(st: State<St>) -> R<String> {
    guard(|| { let a = st.lock(); let b = route::build(&a.doc.p, &a.cat); Ok(format!("data:model/gltf-binary;base64,{}", scene::b64(&gltf::glb(&a.doc.p, &b, a.opts.lod)))) })
}

/// A list as CSV: lines, equipment (with nozzles) or the material take-off.
#[tauri::command]
fn export_csv(st: State<St>, kind: String) -> R<String> {
    guard(|| { let a = st.lock(); lists::csv(&lists::lists(&a.doc.p, &a.cat), &kind) })
}

// ---------------------------------------------------------------- files

fn pick(app: &AppHandle, name: &str, exts: &[&str]) -> Option<PathBuf> {
    app.dialog().file().add_filter(name, exts).blocking_pick_file().and_then(|f| f.into_path().ok())
}

#[tauri::command]
async fn open(app: AppHandle, st: State<'_, St>, path: Option<String>) -> R<Option<Out>> {
    let Some(path) = path.map(PathBuf::from).or_else(|| pick(&app, "Datum Layout plant", &["dlayout"])) else { return Ok(None) };
    let text = read_limited(&path, model::MAX_FILE)?;
    let (p, warnings) = guard(|| model::from_json(&text))?;
    remember(&app, &path);
    let mut a = st.lock();
    a.doc = Doc::new(p);
    a.doc.path = Some(path.display().to_string());
    let mut o = out(&a);
    o.warnings = warnings;
    Ok(Some(o))
}

#[tauri::command]
async fn save(app: AppHandle, st: State<'_, St>, as_new: bool) -> R<Option<Out>> {
    let (known, name) = {
        let a = st.lock();
        (a.doc.path.clone().filter(|_| !as_new), a.doc.p.name.clone())
    };
    let path = match known {
        Some(p) => PathBuf::from(p),
        None => match app.dialog().file().add_filter("Datum Layout plant", &["dlayout"]).set_file_name(format!("{name}.dlayout")).blocking_save_file() {
            Some(f) => f.into_path().map_err(|e| e.to_string())?,
            None => return Ok(None),
        },
    };
    let mut a = st.lock();
    write_atomic(&path, model::to_json(&a.doc.p).as_bytes())?;
    a.doc.path = Some(path.display().to_string());
    a.doc.dirty = false;
    remember(&app, &path);
    clear_recovery(&app);
    Ok(Some(out(&a)))
}

/// Saves a file the window made (a picture or glTF as a data URL, or CSV/PCF text).
#[tauri::command]
async fn save_file(app: AppHandle, name: String, data: String) -> R<Option<String>> {
    let ext = name.rsplit('.').next().unwrap_or("txt").to_string();
    let Some(path) = app.dialog().file().add_filter(ext.to_uppercase(), &[&ext]).set_file_name(&name).blocking_save_file().and_then(|f| f.into_path().ok()) else { return Ok(None) };
    let bytes = match data.strip_prefix("data:").and_then(|d| d.split_once(";base64,")) {
        Some((_, b64)) => base64_decode(b64)?,
        None => data.into_bytes(),
    };
    write_atomic(&path, &bytes)?;
    Ok(Some(path.display().to_string()))
}

fn base64_decode(s: &str) -> R<Vec<u8>> {
    let val = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in s.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=') {
        acc = acc << 6 | val(c).ok_or("The data is damaged.")? as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- options, recovery, recent

fn options_file(app: &AppHandle) -> Option<PathBuf> {
    dir(app, false).map(|d| d.join("options.json"))
}
fn save_options(app: &AppHandle, o: &Options) {
    if let Some(f) = options_file(app) {
        let _ = write_atomic(&f, serde_json::to_string(o).unwrap_or_default().as_bytes());
    }
}

#[tauri::command]
fn options(st: State<St>) -> Options {
    st.lock().opts.clone()
}

#[tauri::command]
fn set_options(app: AppHandle, st: State<St>, opts: Options) -> Out {
    let mut a = st.lock();
    let packs_changed = opts.packs != a.opts.packs;
    a.opts = opts;
    if packs_changed { a.cat = load_catalogue(&a.opts.packs); }
    save_options(&app, &a.opts);
    out(&a)
}

#[derive(Serialize, Deserialize)]
struct Recovery {
    name: String,
    path: Option<String>,
    saved: String,
}

fn recovery_files(app: &AppHandle) -> Option<(PathBuf, PathBuf)> {
    dir(app, true).map(|d| (d.join("recovery.dlayout"), d.join("recovery.json")))
}

fn clear_recovery(app: &AppHandle) {
    if let Some((a, b)) = recovery_files(app) {
        let _ = std::fs::remove_file(a);
        let _ = std::fs::remove_file(b);
    }
}

#[tauri::command]
fn autosave(app: AppHandle, st: State<St>) {
    let a = st.lock();
    if let (true, Some((file, meta))) = (a.doc.dirty, recovery_files(&app)) {
        let info = Recovery { name: a.doc.p.name.clone(), path: a.doc.path.clone(), saved: chrono::Local::now().format("%d-%b-%y %H:%M").to_string() };
        let _ = write_atomic(&file, model::to_json(&a.doc.p).as_bytes());
        let _ = write_atomic(&meta, serde_json::to_string(&info).unwrap_or_default().as_bytes());
    }
}

#[tauri::command]
fn recovery(app: AppHandle) -> Option<Recovery> {
    let (file, meta) = recovery_files(&app)?;
    file.exists().then_some(())?;
    serde_json::from_str(&read_limited(&meta, 1 << 20).ok()?).ok()
}

#[tauri::command]
fn recover(app: AppHandle, st: State<St>, keep: bool) -> R<Option<Out>> {
    let info = recovery(app.clone());
    let files = recovery_files(&app);
    if !keep {
        clear_recovery(&app);
        return Ok(None);
    }
    let (file, _) = files.ok_or("No recovered plant was found.")?;
    let (p, warnings) = guard(|| model::from_json(&read_limited(&file, model::MAX_FILE)?))?;
    let mut a = st.lock();
    a.doc = Doc::new(p);
    a.doc.path = info.and_then(|i| i.path);
    a.doc.dirty = true;
    let mut o = out(&a);
    o.warnings = warnings;
    Ok(Some(o))
}

fn recent_paths(app: &AppHandle) -> Vec<String> {
    options_file(app).map(|f| f.with_file_name("recent.json")).and_then(|f| read_limited(&f, 1 << 20).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn remember(app: &AppHandle, path: &Path) {
    let p = path.display().to_string();
    let mut v = recent_paths(app);
    v.retain(|x| *x != p);
    v.insert(0, p);
    v.truncate(12);
    if let Some(f) = options_file(app).map(|f| f.with_file_name("recent.json")) {
        let _ = write_atomic(&f, serde_json::to_string(&v).unwrap_or_default().as_bytes());
    }
}

#[derive(Serialize)]
struct Recent {
    path: String,
    name: String,
    modified: String,
}

#[tauri::command]
fn recent(app: AppHandle) -> Vec<Recent> {
    recent_paths(&app)
        .into_iter()
        .filter_map(|p| {
            let m = std::fs::metadata(&p).ok()?.modified().ok()?;
            let modified = chrono::DateTime::<chrono::Local>::from(m).format("%d-%b-%y %H:%M").to_string();
            let name = Path::new(&p).file_stem()?.to_string_lossy().into_owned();
            Some(Recent { path: p, name, modified })
        })
        .collect()
}

#[tauri::command]
fn startup_file(app: AppHandle, st: State<St>) -> Option<String> {
    let last = || if st.lock().opts.open_last { recent_paths(&app).into_iter().find(|p| Path::new(p).exists()) } else { None };
    std::env::args().skip(1).find(|a| a.to_lowercase().ends_with(".dlayout")).or_else(last)
}

/// Opens one of the project's GitHub pages or the family site in the browser (only those).
#[tauri::command]
fn open_url(url: String) -> R<()> {
    let plain = url.chars().all(|c| c.is_ascii_alphanumeric() || "/:._-?=&#".contains(c));
    let ours = ["https://github.com/Edgar-Mendonca/Datum-Layout", "https://edgar-mendonca.github.io/Datum"].iter().any(|p| url == *p || url.starts_with(&format!("{p}/")));
    if !plain || !ours {
        return Err("Only the Datum Layout GitHub pages and the Datum site can be opened.".into());
    }
    #[cfg(windows)]
    let r = std::process::Command::new("explorer").arg(&url).spawn();
    #[cfg(not(windows))]
    let r = std::process::Command::new("xdg-open").arg(&url).spawn();
    r.map(|_| ()).map_err(|e| e.to_string())
}

#[tauri::command]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[tauri::command]
fn quit(app: AppHandle) {
    clear_recovery(&app);
    app.exit(0);
}

#[tauri::command]
async fn check_update(st: State<'_, St>) -> R<update::Info> {
    let beta = st.lock().opts.beta;
    tauri::async_runtime::spawn_blocking(move || update::check(beta)).await.map_err(|e| e.to_string())?
}

#[tauri::command]
async fn install_update(app: AppHandle, url: String, sums: Option<String>) -> R<()> {
    tauri::async_runtime::spawn_blocking(move || update::install(&app, &url, sums.as_deref())).await.map_err(|e| e.to_string())?
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Anything that ever panics is written to crash.log (version, place, message) for an issue report.
            if let Some(log) = dir(app.handle(), true).map(|d| d.join("crash.log")) {
                let default = std::panic::take_hook();
                std::panic::set_hook(Box::new(move |info| {
                    use std::io::Write;
                    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&log) {
                        let at = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
                        let what = info.payload().downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| info.payload().downcast_ref::<String>().cloned()).unwrap_or_default();
                        let _ = writeln!(f, "{} Datum Layout {} panicked at {at}: {what}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), env!("CARGO_PKG_VERSION"));
                    }
                    default(info);
                }));
            }
            let opts: Options = options_file(app.handle()).and_then(|f| read_limited(&f, 1 << 20).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
            let cat = load_catalogue(&opts.packs);
            let p = fresh(&opts, Project::new());
            app.manage(St(Mutex::new(App { doc: Doc::new(p), opts, cat })));
            Ok(())
        })
        .on_window_event(|w, e| {
            if let WindowEvent::CloseRequested { api, .. } = e {
                if w.state::<St>().lock().doc.dirty {
                    api.prevent_close();
                    let _ = w.emit("close-requested", ());
                } else {
                    clear_recovery(w.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            state, apply, history, scene, check, lists, examples_list, new_project, catalogue, add_pack, remove_pack, unit_table, export_pcf, export_gltf, export_csv,
            open, save, save_file, options, set_options, autosave, recovery, recover, recent, startup_file, open_url, version, quit, check_update, install_update
        ])
        .run(tauri::generate_context!())
        .expect("Datum Layout failed to start");
}
