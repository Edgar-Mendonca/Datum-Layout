# Datum Layout: project memory

> **Paste this file at the start of each new session**, or just open the repo: `CLAUDE.md` tells Claude to read it.
> It records what exists, why, and what comes next. Update it at the end of every session.

| | |
| --- | --- |
| **Product** | Datum Layout: 3D plant layout and piping (place equipment from datasheet sizes, route lines between nozzles with spec elbows, reducers and flanges, drop fittings that snap, check clashes, list lines and materials, hand each line to Datum Pipe) |
| **Current version** | 0.1.10 (2026-11-08) |
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
| `ui/src/shell.tsx` | The workspace after Datum Mesh: steps bar (`Flow`), plant tree (`Tree`: steps, branches, counts, marks, eyes, filter, menus), context bar (`CtxBar`), tool strip (`ToolStrip`), pane grips and the properties card |
| `ui/src/panels.tsx` | Catalogue (the tree's second tab: search, drag source, classes, packs), Inspector (the properties card) (equipment with nozzles and datasheet, line, component, area, plot), bottom panel (Lines with take-off, Equipment, Clashes, Checks) |
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
| 0.1.5 | The catalogue tables were gathered by an outside assistant from the brief `docs/briefs/CATALOGUE-PACK.md`, then read by the app's own checker, compared with the earlier tables and sanity-checked (sizes and classes growing, duplicates, sources) before merging; disagreements were settled against more than one source (the ¾" long-radius elbow kept at 1⅛") | Quick to gather, but every number is still traceable to a named catalogue page |
| 0.1.5 | Ratings are any of 150, 300, 600, 900, 1500, 2500 (`catalogue::RATINGS`); a valve size missing from the table is interpolated by nominal size between its neighbours of the same kind and class, never a rough guess | Several classes have no 3½" or 22" row; interpolation keeps lengths realistic |
| 0.1.6 | Group edits are engine ops on id lists (`MoveGroup`, `CopyGroup`, `RotateGroup`, `MirrorGroup`, `Delete`); a mirror is a turn to 180° − θ (or −θ) plus nozzles mirrored in their own y, because every equipment body is symmetric about its own x axis; a copy reconnects ends among the copies and frees the others (Point Open) | One rule for every group edit; nothing mirrored that cannot be built |
| 0.1.6 | UI: `picked` (Ctrl+click) is separate from `sel` (the single Inspector item); commands act on `picked` when it is not empty, else on `sel` | Every single-item command keeps working unchanged |
| 0.1.7 | Corner and jog edits are engine ops on the path (index of corner or run); a jog refuses to turn within an elbow's room of a component; `tidy` merges straight corners and zero runs keeping components in place | Routes change without routing again, and nothing ends up on an elbow |
| 0.1.7 | Hiding collapses a part's triangles to a point in the one index buffer (order kept, so picking by triangle still works); the section box uses six clipping planes and picking skips hits outside it; both are view state only, never saved | Keeps one draw call and leaves the model untouched |
| 0.1.8 | Imports read the file's text in the window (a file input; nothing leaves the computer) and send it to the engine as an op (`ImportLineList`, `ImportPcf`), so they undo like any edit and need no new shell command | Small shell, and the bridge tests drive them unchanged |
| 0.1.8 | The line list is matched by line number, or by the number after its size ("8\"-P-1001" matches "10\"-P-1001") since the model renames a line when it is resized | A resized line is reported as a difference, not as two strangers |
| 0.1.8 | PCF import walks each pipeline's end-point graph from a free end, takes elbows' centre points as corners and keeps valves, strainers, tees and reducers as items; flanges, gaskets and bolts are generated again; a reducer next to an end that lands on a free nozzle is the nozzle's own and is dropped | Round trip of every example gives the same lines, parts and findings |
| 0.1.8 (user) | The window is to follow Datum Mesh's shell (steps bar, plant tree with filter, marks and eyes, context bar, tool strip, resizable panes): next release | The user: "improvements on the UI is one of the biggest tasks… look at Datum Mesh and do the same here" |
| 0.1.9 (user) | The window copies Datum Mesh's shell (steps bar, tree with steps, branches, counts, marks and eyes, filter, context bar, tool strip, properties card over the view with grips) into `ui/src/shell.tsx`, adapted to the six steps of a layout (Plot, Equipment, Lines, Fittings, Check, Hand over); the catalogue became the tree's second tab | The user: Mesh's panes look like a commercial app; make Layout the same |
| 0.1.9 | The properties card hides while a tool is active (route, place, measure) and Esc closes it; tree eyes and View › Hide share one hidden list | The card never covers the nozzle being clicked; one idea of "hidden" |
| 0.1.9 | Roadmap re-ordered as PLAN-0.2 proposed: 0.2 pipe classes and fittings (with the valve list, weld count and design rules), 0.3 steel and supports, 0.4 drawings, 0.5 From Schema, 0.6 CAD exchange, 0.7 big models | The work an engineer meets first comes first; Schema's package can wait for its format to settle |
| 0.1.10 | Flow runs from a line's start to its end unless a pump says otherwise (away from its discharge D, into its suction S); the swing-check rule uses that | The examples draw discharge lines from the header to the pump; the rule must not flag correct valves |
| 0.1.10 | A pipe piece under 6 mm between fittings counts as fittings welded straight together; 6 to 50 mm is reported as too short to weld | The router leaves a few millimetres where fittings meet; a real short spool is what matters |
| 0.1.10 | The valve list, weld count and design rules (planned for 0.2) came first as 0.1.10; 0.2 keeps the class editor, eccentric reducers, crosses and sloped lines | Small releases, each finished and tested |
| 0.1.4 (2026-11-08) | **Parked: a shared "Datum Library"** (data only: tables with sources, checks, a browsable web page; no generator, no installer). The tables keep being built now in Layout's catalogue, in the plug-in-pack format, so they can move there unchanged. Build the library repo when a second app needs the same table (Schema's pipe classes, Pipe's flange and valve weights, Mesh's fasteners); each app then copies a fixed version into its own build (fully offline). An in-app download and update channel only if the data grows large (vendor models) | The user, after weighing it: today the overlap is mostly pipe sizes, apps release often enough to carry data updates, and the download channel is the costly part. Generators stay in each app (each needs its own form: ports, solids, elements) |
| 0.1.0 | Everything built for 0.1 shipped in 0.1.0, because the engine came together in one piece; 0.1.1–0.1.4 polish and prove each part of the plan in its order | Releasing a deliberately cut-down 0.1.0 would only hide working code |

---

## 3. Roadmap

| Version | Scope | State |
| --- | --- | --- |
| 0.1.0 | Shell (renamed, steel, logo, publisher "Datum", updater, CI, release); plot with grid, coordinates and north; 3D view; engine (units, catalogue, parts, ports, six equipment types); placing, routing, Inspector; drag-and-drop fittings, resize, branches; checks, lists, CSV, PCF, glTF; examples | ✅ |
| 0.1.1 | Lines follow moved equipment and tees (`doc::follow_ends`); Route Again (`Op::Reroute`); the square-to-nozzle check | ✅ |
| 0.1.2 | Exact snap preview while dragging (`doc::snap`, shell `snap`), slide a component with its arrow (`MoveItem`), tee → branch routing tested, double-click fix, resize and oversized-branch tests | ✅ |
| 0.1.3 | PCF read and solved by Datum Pipe's own engine (`scripts/verify.sh`, CI job "exchange", docs/verification/0.1.3.md); glTF through the Khronos validator; each check's failing and passing case; take-off test | ✅ |
| 0.1.4 | Examples, print, ? cards, screenshots, README and site, final check | ✅ |
| 0.1.5 | Catalogue merged from the outside helper's tables (flanges 150–2500 with `blind_thick_in`, valves in all classes, fittings); missing valve sizes interpolated (`valve_length`); all six ratings in the model and window; eight example classes; measure tool | ✅ |
| 0.1.6 | Multi-select (Ctrl+click, Select All), group move, copy (Ctrl+D), turn ±90°, mirror east–west / north–south and delete (`Op::MoveGroup`, `CopyGroup`, `RotateGroup`, `MirrorGroup`) | ✅ |
| 0.1.7 | Corners (`Op::MoveCorner`) and jogs (`Op::Jog`, `Op::RemoveJog`, `doc::tidy`); colour by line, class, fluid or size with a key; hide, isolate, show all; section box (clipping planes, picking honours it) | ✅ |
| 0.1.8 | Saved views (`Project.views`, `Op::SetViews`); line list import and check (`Project.line_list`, `ImportLineList`, `UseListedLine`, `checks::line_list_issues`); PCF import (`pcf::import`, `Op::ImportPcf`), proved by reading every example back | ✅ |
| 0.1.9 | The workspace after Datum Mesh's shell (`ui/src/shell.tsx`): steps bar, plant tree (filter, counts, marks, eyes, menus), context bar, tool strip, properties card over the view, resizable panes; site.json and roadmap brought up to date | ✅ |
| 0.1.10 | Valve list (`lists::ValveRow`, CSV "valves"); butt welds counted in the take-off; design rules (`checks::design_rules`: stem down, swing check flowing down, short pipe pieces) | ✅ |
| 0.2 Pipe classes and fittings | Class editor with branch table; eccentric reducers, crosses, sloped lines | 🔜 |
| 0.3 Steel and supports | Racks, platforms, ladders, stairs; pipe supports sent to Datum Pipe with the PCF | ⬜ |
| 0.4 Drawings and quantities | Plot plan and GA drawings; full take-off; IFC | ⬜ |
| 0.5 From Schema | Import the plant package; place and route from it; 3D-against-P&ID check | ⬜ |
| 0.6 CAD exchange | Optional engine on first use; STEP import and export | ⬜ |
| 0.7 Big models and teams | Level of detail, areas on demand, project database with history, several users | ⬜ |
| Parked | Datum Library: the shared data repo for every app's tables (see Decisions, 2026-11-08) | 💭 |
| 1.0 | Catalogue validated against its sources, user guide, signed installer | ⬜ |

---

## 4. Session log

### 2026-11-03: Session 1 (0.1.0)
- Read the hub (CLAUDE, MEMORY, FAMILY, BRAND, status 2026-11-01, brain) and the reference apps (Plan's shell and updater, Pipe's shell, 3D view, library, PCF reader and units, Schema's object model, Mesh).
- Built the engine (catalogue with sources, parts, equipment, model, edits, routing, checks with parry3d, lists, PCF, glTF, examples, hardening): 25 engine tests (each several cases), clippy clean, `cargo deny` clean.
- Built the window from Pipe's shell: catalogue panel, 3D view, Inspector, bottom tabs, ribbon, File page, print, help. UI tests (30 checks) drive the real engine through a bridge.
- Released 0.1.0, then 0.1.1 (lines follow moved equipment; Route Again), 0.1.2 (exact snap preview; slide components; branch and double-click fixes), 0.1.3 (PCF read and solved by Datum Pipe's own engine; glTF validated; every check with a failing and passing case; take-off test), 0.1.4 (print and help checks, screenshots, site.json, final check).
- Releases: the release workflow had to run once on a push to be registered; publishing failed with 403 for 0.1.0 and 0.1.2 (also on re-run), while 0.1.1, 0.1.3 and 0.1.4 published at once; 0.1.4 was published last so it is "latest".

**Summary of the run**
- *Released:* 0.1.1, 0.1.3 and 0.1.4 are published on GitHub Releases (0.1.4 is "latest"). 0.1.0 and 0.1.2 built their installers, but the publish step failed twice with 403 "Resource not accessible by integration" (runs 41709198474 and 41711771070; the 0.1.2 run keeps its installer as the artifact DatumLayout-0.1.2). To fill the gap: check Settings › Actions › General › Workflow permissions is "Read and write", re-run those two runs' failed jobs, then re-run the 0.1.4 run (41716004166) so 0.1.4 is "latest" again.
- *What works:* the plot (grid, coordinates, plant north, areas); the six parametric equipment types with nozzles, placed by drag and drop, moved with arrows, edited in the Inspector; routing with orthogonal guides (spec elbows LR/SR at 90°/45°, reducers at nozzles of another size, weld-neck flange + gasket + bolt set at flanged nozzles; tight turns and odd angles reported, not drawn); valves, strainers, flange pairs, tees and reducers that snap onto pipes by the engine's rule (exact preview while dragging) and split them; branches from tees; resizing; moving runs; lines following moved equipment; Route Again; undo/redo; the clash check (pipe–pipe, pipe–equipment, pipe–ground, 25 mm) and the connection, class and number checks; line list, equipment list, take-off, CSV; PCF per line or all (read and solved by Datum Pipe's own engine); glTF (Khronos validator clean); print; ? cards, getting started, glossary, sources; three examples + "Find the clashes"; plug-in packs (folders).
- *Tests:* 31 engine tests (geometry against the tables, ports, gaps, lengths, turns, resizing, branches, moving equipment, every check, take-off, exchange with Pipe's reader, glTF, hardening with random edits, damaged files and a 2,376-part speed gate at about 60 ms); 37 UI checks driving the real engine; `scripts/verify.sh` with Datum Pipe's real engine; clippy and cargo deny clean.
- *Cut (brief's cut order):* eccentric reducers; the walk view. glTF and 45° elbows were kept. Also not done: zip plug-in packs (folders only), valve weights in the PCF, automatic jogs when a move cannot be taken by the runs (reported; Route Again).
- *Speed:* 2,376 parts draw as 181k triangles (low detail above 1,500 parts); 123 ms a frame in Chromium's software renderer with no graphics card in the test machine; not yet measured on a real PC.
- *2-minute demo:* (1) File › Examples › Pump suction and discharge. (2) Drag a Gate valve from the catalogue onto the 6" header between the two tees: the marker shows where it snaps; drop it. (3) Click the header in the Lines tab, Piping › One size larger: the valve, tees, cap, elbow and blind all become 8". (4) F5 (Check): no clashes. (5) Click 4"-P-1003-CS1 in the Lines tab, Home › To Datum Pipe: save the .pcf; in Datum Pipe, File › Import a piping component file.

---

### 2026-11-08: Session 2 (catalogue data)
- Wrote `docs/briefs/CATALOGUE-PACK.md`, a brief for an outside helper to build the parts tables in the app's exact formats (Part A: tables read today; Part B: new part types; Part C: checks and a report).
- Reviewed the first two files: flanges.csv (115 rows, classes 150–2500, all accepted, classes 150/300 match the built-in table to 0.01") and valves.csv (433 rows, all accepted; real swing-check lengths replace the built-in ones, which were globe lengths; class 300 1¼"/1½" check, 16" globe and mm-converted rounding sent back for checking). Not merged yet: the set is merged together when complete.
- Discussed and parked a shared Datum Library (Decisions).

### 2026-11-08: Session 3 (0.1.5 to 0.2)
- The user handed over 0.1.5 to 0.2.0 to be done unattended, in the order of `docs/PLAN-0.2.md` (industry gap analysis): the catalogue, then working with many items, routing edits, views, imports, lists and checks, then the class editor.
- 0.1.5: merged the outside helper's flanges, valves and fittings (see Decisions); eight example classes; all six ratings; measure tool.
- 0.1.6: multi-select and group move, copy, turn, mirror and delete; engine test `groups_move_copy_rotate_and_mirror_cleanly`; UI checks for Ctrl+A, Ctrl+D, mirror and undo.
- 0.1.10: valve list, butt-weld count, design rules (engine tests with failing and passing cases; UI checks for the list, a rule and undo).
- 0.1.9 (the user's request mid-run): Datum Mesh's shell studied (its UI test run for screenshots) and copied: steps bar, plant tree, context bar, tool strip, properties card; UI checks for each; the filter bug the new check found fixed; site.json brought up to date for 0.1.5–0.1.9.
- 0.1.8: saved views; line list import and check; PCF import (round-trip test over every example); UI checks with real file choosers.
- 0.1.7: move corners, add and remove jogs (engine tests on every example line: components return to the same place); colour by with a key; hide/isolate; section box; UI checks for each.

## 5. Known limits

- Eccentric reducers and the walk view are not in 0.1 (cut list).
- Plug-in packs are folders only (no zip yet).
- Turning equipment, or a move no run can take (a sideways move of a single straight leg), leaves its line skewed and reported until Route Again; Route Again does not yet look for clashes.
- Dimension tables are typical catalogue values; they are checked against their sources before 1.0.
- Datum Pipe's PCF reader has no 22" bore (reads it as 20") and no cap (reads a short pipe): noted for Datum Pipe's chat.
- Not yet run on a real Windows PC by the user; the 3D view was checked in Chromium's software renderer.
- Hub task: add Datum Layout to `apps.json` (id layout, accent #3A5A78), copy `docs/brand/datum-layout-icon.svg` to the hub's `brand/`, and the rest of the hub's list (AppGlyph, phrases, nine-app hero, FAMILY §1, Datum Home's list, brain).

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
