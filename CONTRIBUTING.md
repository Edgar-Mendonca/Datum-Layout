# Contributing

Ideas and bug reports are welcome as [issues](https://github.com/Edgar-Mendonca/Datum-Layout/issues/new). File › Roadmap in the app (and `docs/site.json`) shows what is planned.

## How the code is laid out

- `crates/core`: the engine in pure Rust: the plant model and its edits, the catalogue (tables and pipe classes), part shapes and ports, the six equipment types, routing, the checks (clashes with parry3d), lists, and the PCF and glTF exports. All geometry and rules live here, with tests.
- `catalogue/`: the dimension tables and the pipe class, as plain CSV and JSON with a source on every row (`catalogue/SOURCES.md`).
- `src-tauri`: the desktop shell (commands, files, options, recovery, plug-in packs, the updater).
- `ui/src`: the window (SolidJS + TypeScript, three.js for the 3D view). It draws what the engine computes and sends edits back as operations.
- `docs/MEMORY.md`: architecture, decisions, roadmap and a log of every session. Read it first.

## Before you send a change

```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace            # engine tests: geometry, routing, exchange, hardening
npm run check:ui                  # type-check the window
npm run test:ui                   # UI tests (Playwright, with the real engine behind the window)
cargo deny check                  # licences, advisories, sources (cargo install cargo-deny)
```

Keep code compact and in the style around it. A new part or rule comes with a test against its table; a new window feature with a UI test where it can. Never copy a standard's tables or text, or any other program's parts library. Never link GPL, LGPL or AGPL code into the app.

By contributing you agree your work is released under the MIT licence.
