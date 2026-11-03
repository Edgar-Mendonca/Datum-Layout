# Datum Layout: project memory

> **Paste this file at the start of each new session**, or just open the repo: `CLAUDE.md` tells Claude to read it.
> It records what exists, why, and what comes next. Update it at the end of every session.

| | |
| --- | --- |
| **Product** | Datum Layout: 3D plant layout and piping (place equipment from datasheet sizes, route lines between nozzles with spec elbows, reducers and flanges, drop fittings that snap, check clashes, list lines and materials, hand each line to Datum Pipe) |
| **Current version** | 0.1.3 (2026-11-03) |
| **Repo** | `Edgar-Mendonca/Datum-Layout` · branch `claude/hopeful-edison-d99owj` (the first branch pushed, so the default branch) |
| **Stack** | Rust engine (`crates/core`, parry3d for clashes), Tauri 2, SolidJS + strict TypeScript, three.js for the 3D view; no Python |
| **Identifier** | `com.edgarmendonca.datumlayout` · files `.dlayout` (JSON, `format` 1) · installer `DatumLayout-x.y.z-setup.exe` + `SHA256SUMS.txt` · publisher "Datum" |
| **Accent** | Steel `#3A5A78` (`--blue2` `#2E4861`, `--sky` `#B9CCDF`, `--pale` `#EEF3F8`, `--sel` `#DFE8F1`, `--sel2` `#C2D3E4`; dark `#8FB3D9`; File sidebar dark `#1F3247`) |
| **Brief** | `Edgar-Mendonca/Datum` › `briefs/DATUM-LAYOUT.md` (hub session 31): every decision made up front |
| **Reference repos** | `Edgar-Mendonca/Datum` (FAMILY, MEMORY, brain), `Datum-Plan` (shell, updater, hardening), `Datum-Pipe` (3D view, pipe library, PCF reader, units, the skeleton copied), `Datum-Mesh` (3D techniques), `Datum-Schema` (object model, plant package), `Datum-Calc` (units) |

---

## 1. Architecture

`crates/core` is the engine (pure Rust, no Tauri). The model holds only what the designer decides (equipment with sizes and nozzles; lines with a path of corner points and components placed on a segment at a distance); `route::build` derives every part with its shapes and ports from it, so resizing, moving and undo never leave stale geometry. `src-tauri` is the shell: commands, files, options, recovery, plug-in packs, the updater. `ui` is the window: it draws what the engine sends and sends edits as operations.

Coordinates: metres, X east, Y north, Z up, in the engine, the file and PCF; the 3D view turns its group so Z is up in three.js's Y-up world. Units: the engine's display table (`units::shown`) gives every quantity in SI (m, mm, bar, °C) or US (ft, in, psi, °F).

### Files

| File | What it does |
| --- | --- |
| `catalogue/*.csv`, `catalogue/classes/CS1.json`, `catalogue/SOURCES.md` | The tables (pipe ½"–24" with schedules, fittings, flanges class 150/300, valves face-to-face) and the one pipe class, each row with its source |
| `crates/core/src/catalogue.rs` | Reads the tables and classes (built in with `include_str!`, and plug-in packs by the same functions), validates each row, keeps sources, lists what it could not read; `PipeClass::schedule` and `allows` |
| `crates/core/src/parts.rs` | Vector helpers; `Shape` (frustum, torus segment, head, cuboid) with exact bounding boxes; `Port` (position, outward direction, size, rating, BW/FL); `Mesh` (positions, normals, triangles at a level of detail) |
| `crates/core/src/equipment.rs` | The six types: `fields` (which sizes each has), `new` (typical sizes, nozzles, datasheet lines), `shapes` (body and nozzles placed and rotated), `nozzle` (neck + weld-neck flange, port at the face) |
| `crates/core/src/model.rs` | `Project` (plot, areas, equipment, lines, settings, ids), `Equipment`, `Nozzle`, `Line`, `End` (point with open/cap/blind, nozzle, tee branch), `Item` (component on a segment), `normalize` (clamps every value, unique ids and uids, valid references), `.dlayout` read whole or item by item, and write |
| `crates/core/src/doc.rs` | `Op` (SetMeta, SetPlot, SetSettings, AddEquipment, SetEquipment, AddArea, SetArea, Route, SetLine, MoveRun, Drop, MoveItem, SetItem, Resize, SetTerm, Delete, Batch) applied to a copy and kept only on success; snapshot undo/redo |
| `crates/core/src/route.rs` | `build`: per line, the resolved path (ends moved to nozzles and tees), sizes along it (reducers), end fittings (gasket, weld-neck flange, reducer at a nozzle; cap; blind + gasket + flange), spec elbows (R tan θ/2 cut-backs; 90°/45° only; too tight → reported, not drawn), components (valves with flanges, gaskets and bolt sets; flange pairs; tees; reducers; not fitting → reported), pipes between; `nearest`, `free_place` (snapping), `orthogonal` (route legs, never doubling back), `end_port`, `branch_size`, `size_at` |
| `crates/core/src/checks.rs` | `connections` (ports that meet within 0.01 mm), `connection_issues` (open ends, size/end/rating/class mismatches, class rules, missing numbers), `clashes` (convex hulls in parry3d, sweep over bounding boxes, pairs joined within two hops ignored, ground with clearance), `run` |
| `crates/core/src/scene.rs` | One mesh buffer for the view (base64 f32/u32), each part's triangle range, insulation sleeves, ports to route from; large plants (over 1,500 parts) drawn at low detail |
| `crates/core/src/lists.rs` | Line list, equipment and nozzle list, material take-off per line; CSV with the formula guard |
| `crates/core/src/pcf.rs` | PCF writer (per line or all; mm, east–north–up, DN bores, elbow corners, tee centres and branches); `reader`: Datum Pipe's reader, copied for the tests |
| `crates/core/src/gltf.rs` | glTF 2.0 binary writer (a named node and mesh per part, a material per kind, Y up) and `verify` |
| `crates/core/src/examples.rs` | The three examples and "Find the clashes", built through `Doc` operations; `repeated` (the large plant of the speed checks) |
| `crates/core/src/units.rs` | Calc's unit table (NIST SP 811) with Layout's display quantities |
| `crates/core/src/geometry_tests.rs`, `exchange_tests.rs`, `hardening.rs` | Brief §4: parts against tables, ports, gaps, lengths, turns, resizing, branches; PCF read by Pipe's reader, glTF; failed edits, random edits, damaged files, 2,000-part speed gate |
| `crates/core/examples/bridge.rs` | The engine behind the window for the UI tests (stdin/stdout JSON) |
| `crates/core/examples/summary.rs` | Each example's parts and findings, for checking by hand |
| `crates/core/examples/export.rs`, `scripts/verify.sh`, `scripts/pipe-read.rs`, `scripts/gltf-validate.cjs` | The exchange proofs: exports every example; a fresh clone of Datum Pipe reads and solves each line (the reader file is copied into the clone as an example, never linked here); the Khronos validator checks each .glb |
| `src-tauri/src/main.rs` | Commands (state, apply, history, scene, check, lists, examples, catalogue, packs, exports, files, options, recovery, recent, open_url, updates); `guard` (a panic becomes a message), crash.log, atomic saves, read limits |
| `src-tauri/src/update.rs` | Plan's updater (`REPO` = Datum-Layout; requires SHA256SUMS.txt) |
| `ui/src/view3d.tsx` | The 3D view: plot (ground, grid, coordinates, plant north), areas, parts from one buffer with colours per part, picking, selection colours, labels, ports and the route preview, snap marker while dragging, axis arrows (gizmo), standard views and fit, the orbit speed check |
| `ui/src/panels.tsx` | Catalogue panel (search, drag source, classes, packs), Inspector (equipment with nozzles and datasheet, line, component, area, plot), bottom panel (Lines with take-off, Equipment, Clashes, Checks) |
| `ui/src/ribbon.tsx`, `cmds.ts` | Title bar, Tell Me, ribbon (Home · Equipment · Piping · Fittings · Check · Lists · View · Help, contextual Format); every command |
| `ui/src/store.ts`, `api.ts`, `units.ts` | Window state (plant, scene, selection, tools, checks), the engine's shapes, display units |
| `ui/src/backstage.tsx`, `print.tsx`, `dialogs.tsx`, `help.tsx`, `roadmap.tsx`, `ui.tsx`, `styles.css`, `index.html` | File page, print (view + lists), dialogs and help pages, ? cards, roadmap, components, the family styles (steel), the icon sprite |
| `tests_ui/run.cjs` | UI tests with the real engine behind the window; product-name guard; screenshots with `SHOTS=dir` |

### Release flow
Bump both Cargo.toml versions, add the CHANGELOG section, keep `docs/site.json` true, run `release.yml` (workflow_dispatch). It builds `DatumLayout-x.y.z-setup.exe` and `SHA256SUMS.txt` and publishes the GitHub release.

---

## 2. Decisions

| Version | Decision | Why |
| --- | --- | --- |
| 0.1.0 | The skeleton is copied from Datum Pipe (whose shell is Plan's), with Plan's newer hardening added: `update.rs`, `guard`, crash.log, atomic saves, read limits, the strict CSP, `panic = "unwind"` | Pipe is the closest relative and already has the three.js view; Plan is the reference for the shell's safety rules |
| 0.1.0 | Print uses the WebView's print dialog (Save as PDF there), as Datum Pipe does, not Plan's direct WebView2 PrintToPdf | Plan's path needs Windows-only crates that cannot be checked from this session; the print page is the family's |
| 0.1.0 | The model stores decisions only (path corner points; components as segment + distance); parts and ports are derived by `route::build` every time | Resizing, moving runs, undo and checks can never disagree with what is drawn |
| 0.1.0 | Coordinates X east, Y north, Z up everywhere in the engine and file | Plant convention and PCF's; the view turns once |
| 0.1.0 | One pipe class, CS1, allowing class 150 and 300 flanges; each line chooses its rating | The brief's "one class, class 150 and 300 flanges"; simplest |
| 0.1.0 | Elbow cut-back is R·tan(θ/2) with R from the table (long radius 1.5 × NPS, ½" 1½"; short radius 1 × NPS from 1"); only 90° and 45° turns | Exact for both angles from one table column |
| 0.1.0 | **Cut:** eccentric reducers (first on the brief's cut list): the catalogue has concentric reducers only | Their offset centreline needs the path to step; later |
| 0.1.0 | **Cut:** the walk view (second on the cut list); shown dimmed on the View tab | Time |
| 0.1.0 | Plug-in packs are folders (CSV + `classes/*.json`), not zip files yet | No zip crate needed for 0.1; the reader functions are the same |
| 0.1.0 | Clashes: each shape as a convex hull (tori cut every 15°) or a box in parry3d, candidates by a sweep over bounding boxes plus clearance, exact convex distance; parts joined port to port or through one part between them (a tight elbow) are never a clash, nor a nozzle's neighbours with its own equipment | parry3d's exact distance with a simple, testable broad phase; 2,376 parts check in about 60 ms |
| 0.1.0 | Routing: a route from a nozzle first leaves along the nozzle by flange + reducer + elbow (rounded up to 50 mm), then orthogonal legs, choosing the leg order that never doubles back and arrives square to the end nozzle | Real routes with few clicks; the engine, not the window, decides |
| 0.1.0 | Dropped components snap to the nearest free straight length (between elbows, end fittings and other components), snapped to 10 mm | "Snaps by its ports, splits the pipe and rejoins it" falls out of the derived parts |
| 0.1.0 | A tee's branch size follows the branch line connected to it (never larger than the run); a reducing tee when smaller | No separate tee size to keep in step |
| 0.1.0 | PCF: valves as VALVE, strainers as FILTER, flanges FLANGE, blinds FLANGE-BLIND, gaskets GASKET (all read by Pipe as rigid or pipe pieces); caps as CAP (Pipe's reader has no cap yet: it reads it as a short pipe and warns); no weights yet | Pipe's reader as it is; the test allows exactly those warnings |
| 0.1.0 | The product-name guard keeps the names only as SHA-256 hashes in `tests_ui/run.cjs` | FAMILY §13: the names may not appear in any file |
| 0.1.0 | UI tests run the window against the real engine (`examples/bridge.rs` over stdin/stdout) instead of a fixed fixture | Routing, dropping, resizing and checking are tested for real, not mocked |
| 0.1.0 | Plants over 1,500 parts draw at low detail; labels thin out past 60 equipment items or 40 lines | Measured: 2,376 parts at low detail = 181k triangles, 123 ms a frame in Chromium's software renderer (no graphics card) in the test; a graphics card is far faster |
| 0.1.0 | `release.yml` ran once on a push to the working branch to be registered (as Datum Calc did), and that trigger was removed at once; later releases use workflow_dispatch | A workflow needs one run before GitHub lists it; tag pushes are blocked from the session |
| 0.1.1 | After every edit `doc::follow_ends` moves each line end to its nozzle or tee and passes the move along the legs: each leg absorbs the part of the move along its own axis, unless that leaves it shorter than flange + two elbows (then it moves whole); a free far end moves too; when the move reaches an attached far end with something left, the line stays and is reported (no automatic jog). Components that no longer fit slide to the nearest free place on their run | Keeps the designer's corners; never silently draws a wrong route; a fully automatic re-route scrambled component positions in trials |
| 0.1.1 | `Op::Reroute` routes orthogonally between the ports through the interior corners, dropping the lead corner next to each attached end (the new route makes its own) | One click fixes a skewed line after equipment turns |
| 0.1.1 | **Publishing releases needs the repository's Actions token to have write access** (Settings › Actions › General › Workflow permissions › Read and write). Until then the release workflow builds the installer and keeps it as a workflow artifact (`DatumLayout-x.y.z`), but the GitHub Release step fails with 403 | Found on the first release run; a repository setting the session cannot change |
| 0.1.2 | `doc::snap` (nearest segment → size there → length → `free_place`) is the one rule for both the drag preview and `Drop` | The marker can never show a place the drop would not use |
| 0.1.2 | A route ends on the window's dblclick event, not on a click with `detail` 2 | Pointer events carry no click count (found by the new branch UI test) |
| 0.1.3 | Datum Pipe's real engine is run against Layout's files in a fresh clone (CI job "exchange"), in addition to the reader copied into the unit tests | The brief: "Layout's export must open in Pipe unchanged"; a clone tests what Pipe does today without linking its code |
| 0.1.0 | Everything built for 0.1 shipped in 0.1.0, because the engine came together in one piece; 0.1.1–0.1.4 polish and prove each part of the plan in its order | Releasing a deliberately cut-down 0.1.0 would only hide working code |

---

## 3. Roadmap

| Version | Scope | State |
| --- | --- | --- |
| 0.1.0 | Shell (renamed, steel, logo, publisher "Datum", updater, CI, release); plot with grid, coordinates and north; 3D view; engine (units, catalogue, parts, ports, six equipment types); placing, routing, Inspector; drag-and-drop fittings, resize, branches; checks, lists, CSV, PCF, glTF; examples | ✅ |
| 0.1.1 | Lines follow moved equipment and tees (`doc::follow_ends`); Route Again (`Op::Reroute`); the square-to-nozzle check | ✅ |
| 0.1.2 | Exact snap preview while dragging (`doc::snap`, shell `snap`), slide a component with its arrow (`MoveItem`), tee → branch routing tested, double-click fix, resize and oversized-branch tests | ✅ |
| 0.1.3 | PCF read and solved by Datum Pipe's own engine (`scripts/verify.sh`, CI job "exchange", docs/verification/0.1.3.md); glTF through the Khronos validator; each check's failing and passing case; take-off test | ✅ |
| 0.1.4 | Examples, print, ? cards, screenshots, README and site, final check | 🔜 |
| 0.2 From Schema | Import the plant package; place and route from it; 3D-against-P&ID check; more classes as packs | ⬜ |
| 0.3 Steel and supports | Racks, platforms, ladders, stairs; pipe supports sent to Datum Pipe with the PCF | ⬜ |
| 0.4 Drawings and quantities | Plot plan and GA drawings; full take-off; IFC | ⬜ |
| 0.5 CAD exchange | Optional engine on first use; STEP import and export | ⬜ |
| 0.6 Big models and teams | Level of detail, areas on demand, project database with history, several users | ⬜ |
| 1.0 | Catalogue validated against its sources, user guide, signed installer | ⬜ |

---

## 4. Session log

### 2026-11-03: Session 1 (0.1.0)
- Read the hub (CLAUDE, MEMORY, FAMILY, BRAND, status 2026-11-01, brain) and the reference apps (Plan's shell and updater, Pipe's shell, 3D view, library, PCF reader and units, Schema's object model, Mesh).
- Built the engine (catalogue with sources, parts, equipment, model, edits, routing, checks with parry3d, lists, PCF, glTF, examples, hardening): 25 engine tests (each several cases), clippy clean, `cargo deny` clean.
- Built the window from Pipe's shell: catalogue panel, 3D view, Inspector, bottom tabs, ribbon, File page, print, help. UI tests (30 checks) drive the real engine through a bridge.
- Released 0.1.0.

---

## 5. Known limits

- Eccentric reducers and the walk view are not in 0.1 (cut list).
- Plug-in packs are folders only (no zip yet).
- Turning equipment, or a move no run can take (a sideways move of a single straight leg), leaves its line skewed and reported until Route Again; Route Again does not yet look for clashes.
- Dimension tables are typical catalogue values; they are checked against their sources before 1.0.
- Datum Pipe's PCF reader has no 22" bore (reads it as 20") and no cap (reads a short pipe): noted for Datum Pipe's chat.
- Not yet run on a real Windows PC by the user; the 3D view was checked in Chromium's software renderer.

---

## 6. How we work

1. The user reviews a release and sends screenshots.
2. Reproduce first.
3. Big asks get a plan file and questions; build after the answers.
4. Engine first (with a test), then the UI.
5. Check every change in the window (`SHOTS=dir node tests_ui/run.cjs`).
6. Add a UI test for every bug.
7. Run all checks: clippy `-D warnings`, `cargo test` (with the geometry, exchange and hardening tests), the UI type-check and UI tests, `cargo deny check`.
8. Release.
9. Report in plain language: what changed, what was checked, what is not done.
