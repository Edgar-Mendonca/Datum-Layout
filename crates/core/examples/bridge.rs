//! The engine behind the window for the UI tests: reads one call per line on stdin
//! (`{"cmd": "...", "args": {...}}`) and answers one line on stdout (`{"ok": ...}` or
//! `{"err": "..."}`), doing what the desktop shell's commands do, minus files and dialogs.
//! `tests_ui/run.cjs` starts it and routes the window's calls to it, so the UI tests drive the
//! real engine: routing, dropping components, resizing, checks, lists and exports.
use datum_core::catalogue::Catalogue;
use datum_core::doc::{Doc, Op};
use datum_core::model::{EquipKind, Project};
use datum_core::{checks, equipment, examples, gltf, lists, pcf, route, scene, units};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

fn out(d: &Doc, cat: &Catalogue, created: Vec<u32>) -> Value {
    let b = route::build(&d.p, cat);
    let mut issues = b.issues.clone();
    issues.extend(checks::connection_issues(&d.p, cat, &b));
    let paths: Vec<_> = d.p.lines.iter().map(|l| (l.id, route::path(&d.p, cat, l))).collect();
    json!({ "p": d.p, "path": d.path, "dirty": d.dirty, "undo": d.can_undo(), "redo": d.can_redo(), "created": created, "warnings": [], "issues": issues, "paths": paths })
}

fn main() {
    let mut base = Catalogue::builtin();
    let mut d = Doc::new(Project::new());
    let mut opts = json!({ "autosave": 0, "beta": false, "theme": "light", "openLast": false, "units": "SI", "lod": 1, "clearance": 0.025, "labels": true, "paper": "A4", "landscape": true, "company": "", "packs": [] });
    let stdin = std::io::stdin();
    let mut so = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let v: Value = serde_json::from_str(&line).unwrap_or(json!({}));
        let cmd = v["cmd"].as_str().unwrap_or("");
        let a = &v["args"];
        // The catalogue as the plant sees it: with the plant's own classes.
        let cat = base.for_project(&d.p).into_owned();
        let r: Result<Value, String> = match cmd {
            "state" => Ok(out(&d, &cat, vec![])),
            "apply" => serde_json::from_value::<Op>(a["op"].clone()).map_err(|e| e.to_string()).and_then(|op| d.apply(op, &cat)).map(|c| out(&d, &base.for_project(&d.p), c)),
            "history" => { if a["redo"].as_bool() == Some(true) { d.redo() } else { d.undo() }; Ok(out(&d, &cat, vec![])) }
            "scene" => Ok(json!(scene::scene(&d.p, &cat, &route::build(&d.p, &cat), opts["lod"].as_u64().unwrap_or(1) as u32))),
            "snap" => (|| -> Result<Value, String> {
                let l = d.p.line(a["line"].as_u64().unwrap_or(0) as u32).ok_or("no line")?;
                let kind = serde_json::from_value(a["kind"].clone()).map_err(|e| e.to_string())?;
                let at = serde_json::from_value(a["at"].clone()).map_err(|e| e.to_string())?;
                datum_core::doc::snap(&d.p, &cat, l, kind, at, "").map(|x| json!(x.2))
            })(),
            "check" => Ok(json!(checks::run(&d.p, &cat, &route::build(&d.p, &cat)))),
            "lists" => Ok(json!(lists::lists(&d.p, &cat))),
            "examples_list" => Ok(json!(examples::list())),
            "new_project" => match a["example"].as_str() {
                Some("big") => { d = Doc::new(examples::repeated("pumps", 24).unwrap_or_default()); Ok(out(&d, &cat, vec![])) }
                Some(id) => examples::get(id).ok_or("no such example".into()).map(|p| { d = Doc::new(p); out(&d, &cat, vec![]) }),
                None => { d = Doc::new(Project::new()); Ok(out(&d, &cat, vec![])) }
            },
            // The user's templates, kept in memory as the app keeps them in templates.json.
            "save_template" => {
                let t = json!([{ "name": a["name"], "equip": a["equip"] }]).to_string();
                let before = base.warnings.len();
                base.add_templates("My templates", &t);
                Ok(json!(base.warnings[before..].to_vec()))
            }
            "delete_template" => { let n = a["name"].as_str().unwrap_or(""); base.templates.retain(|t| t.name != n); Ok(Value::Null) }
            "catalogue" => Ok(json!({ "base": base.classes.iter().map(|c| c.name.clone()).collect::<Vec<_>>(), "own": d.p.classes.iter().map(|c| c.name.clone()).collect::<Vec<_>>(), "cat": cat, "fields": EquipKind::ALL.iter().map(|k| (k, k.name(), equipment::fields(*k))).collect::<Vec<_>>() })),
            "unit_table" => Ok(json!((units::shown(false), units::shown(true), units::typed()))),
            "export_pcf" => pcf::write(&d.p, &cat, a["line"].as_u64().map(|x| x as u32)).map(Value::from),
            "export_gltf" => Ok(json!(format!("data:model/gltf-binary;base64,{}", scene::b64(&gltf::glb(&d.p, &route::build(&d.p, &cat), 1))))),
            "export_csv" => lists::csv(&lists::lists(&d.p, &cat), a["kind"].as_str().unwrap_or("")).map(Value::from),
            "options" => Ok(opts.clone()),
            "set_options" => { opts = a["opts"].clone(); Ok(out(&d, &cat, vec![])) }
            "save" => { d.dirty = false; d.path = Some(format!("{}.dlayout", d.p.name)); Ok(out(&d, &cat, vec![])) }
            "save_file" => Ok(json!(format!("/saved/{}", a["name"].as_str().unwrap_or("file")))),
            "version" => Ok(json!("test")),
            "recent" => Ok(json!([])),
            "recovery" | "startup_file" | "autosave" | "open_url" | "quit" | "remove_pack" | "add_pack" => Ok(Value::Null),
            "check_update" => Err("offline".into()),
            other => Err(format!("the test bridge has no command {other}")),
        };
        let reply = match r { Ok(v) => json!({ "ok": v }), Err(e) => json!({ "err": e }) };
        let _ = writeln!(so, "{reply}");
        let _ = so.flush();
    }
}
