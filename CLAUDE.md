# Datum Layout: notes for Claude

Read `docs/MEMORY.md` first. It is the project's memory: architecture, decisions, roadmap, and a log of every session. The family design guide is `FAMILY.md` in `Edgar-Mendonca/Datum` (read-only from here; `docs/FAMILY.md` links to it). The brief this app started from is `briefs/DATUM-LAYOUT.md` in the same hub repo.
At the end of every session that changes anything, update `docs/MEMORY.md`:
- tick the roadmap
- add a session-log entry
- record any new decisions
- refresh the architecture section if files moved

User preferences:
- Rust and Tauri 2, small and efficient code, Windows first. The same look, ribbon, help, printing and updater as every Datum app (FAMILY §2–9); copy from the reference apps rather than reinventing: Datum Plan (shell), Datum Pipe (3D view, pipe library, PCF, units), Datum Mesh (3D techniques, verification pattern), Datum Schema (object model, plant package). Every copy becomes Layout's own code; never link shared code.
- Ship in small numbered releases (0.1.0, 0.1.1, …). Every release appears in the in-app updater.
- All geometry and rules belong in `crates/core` (pure Rust, unit-tested), not in the UI. The window only draws what the engine sends and sends edits back as operations (`doc::Op`).
- The UI is SolidJS + Vite in `ui/src/*.tsx` (built to `ui/dist`). State lives in `ui/src/store.ts`. The 3D view uses three.js (MIT).
- The engine and the `.dlayout` file use metres with X east, Y north, Z up. The engine sends the display units (SI and US) from its unit table (`crates/core/src/units.rs`); `ui/src/units.ts` only applies them.
- Parts are generated, never downloaded or bundled: a shape rule in code + a dimension table in `catalogue/` (every row with its source) + ports. Never copy a standard's tables or text, or any other program's parts library.
- Public words (README, CHANGELOG, site.json, help, commits, comments): never name, compare with, or say the app copies or replaces any commercial plant-design, CAD or 3D product (FAMILY §13). Standards and file formats may be named (PCF, glTF, IFC, STEP). The UI test fails if a product name appears in the window (names kept only as hashes in `tests_ui/run.cjs`).
- Datum Layout models geometry; it never simulates the process.
- Licences: MIT/BSD/Apache/MPL only. Record every dependency in `NOTICE`.

Checks before pushing:
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace` (geometry, routing, exchange with Datum Pipe's reader, examples, hardening)
- `npm run check:ui`, then `npm run test:ui` (builds the window and the engine bridge `crates/core/examples/bridge.rs`, then drives the window with the real engine behind it; set `NODE_PATH=$(npm root -g)` if Playwright is installed globally)
- `cargo deny check` (licences, advisories, sources; CI runs it)
- A change you can see: look at it (`SHOTS=dir node tests_ui/run.cjs` writes the screenshots) and add a UI check for every bug.

Releasing:
1. Bump the version in `src-tauri/Cargo.toml` and `crates/core/Cargo.toml`.
2. Add a CHANGELOG section (`## x.y.z — date`).
3. Keep `docs/site.json` true with each release (`about`, `steps`, `highlights`, `roadmap`).
4. Run `release.yml` from the Actions tab (workflow_dispatch; tag pushes are blocked from Claude sessions) or push the tag `vX.Y.Z`. The workflow creates the tag and publishes `DatumLayout-x.y.z-setup.exe` and `SHA256SUMS.txt`.
