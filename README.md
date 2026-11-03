<p align="center"><img src="docs/brand/datum-layout-icon.svg" width="96" alt="Datum Layout"></p>

<h1 align="center">Datum Layout</h1>

<p align="center">3D plant layout and piping: place equipment, route lines, check and hand over.</p>

<p align="center">
  <a href="https://github.com/Edgar-Mendonca/Datum-Layout/releases"><img src="https://img.shields.io/github/v/release/Edgar-Mendonca/Datum-Layout?include_prereleases&color=3A5A78" alt="Release"></a>
  <a href="https://github.com/Edgar-Mendonca/Datum-Layout/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Edgar-Mendonca/Datum-Layout/ci.yml?label=CI" alt="CI"></a>
  <a href="https://github.com/Edgar-Mendonca/Datum-Layout/releases"><img src="https://img.shields.io/github/downloads/Edgar-Mendonca/Datum-Layout/total?color=3A5A78" alt="Downloads"></a>
  <img src="https://img.shields.io/badge/license-MIT-blue" alt="MIT">
  <img src="https://img.shields.io/badge/platform-Windows%2010%2F11-0C1A2B" alt="Windows">
  <img src="https://img.shields.io/badge/Rust-engine-B7410E" alt="Rust">
  <img src="https://img.shields.io/badge/Tauri-2-24C8DB" alt="Tauri 2">
  <img src="https://img.shields.io/badge/SolidJS-window-2C4F7C" alt="SolidJS">
</p>

<p align="center">
  <a href="https://github.com/Edgar-Mendonca/Datum-Layout/releases/latest"><img src="https://img.shields.io/badge/Download_for_Windows-3A5A78?style=for-the-badge" alt="Download for Windows"></a>
  <a href="CHANGELOG.md"><img src="https://img.shields.io/badge/What's_new-0C1A2B?style=for-the-badge" alt="What's new"></a>
  <a href="docs/MEMORY.md"><img src="https://img.shields.io/badge/Memory_&_roadmap-5B6878?style=for-the-badge" alt="Memory and roadmap"></a>
</p>

![Datum Layout: a tank, a duty and standby pump pair and a discharge header in 3D, with the line list below](docs/images/pumps-3d.png)

Place equipment from its datasheet sizes, route pipes between nozzles, and the elbows, tees, flanges and valves appear and size themselves from the line's size and pipe class. Drag a valve onto a pipe and it snaps into place. Then check for clashes, list every line and its materials, and send each line to Datum Pipe for its isometric and stress analysis. It is for piping and layout designers, mechanical and process engineers, and students learning plant design. It is free and open source (MIT), Windows first, installs in seconds and updates itself. More on the family site: [edgar-mendonca.github.io/Datum/apps/layout](https://edgar-mendonca.github.io/Datum/apps/layout).

## What it does

- **Place the equipment.** Vertical vessels, horizontal drums, storage tanks, columns, centrifugal pumps with motor and baseplate, and shell-and-tube exchangers, drawn from a few datasheet sizes, each with its nozzles (size, rating, facing, position, direction).
- **Route the lines.** Click from nozzle to nozzle with orthogonal guides: spec elbows at every turn (long or short radius), reducers where sizes change, weld-neck flanges, gaskets and bolt sets at flanged nozzles.
- **Drop the fittings.** Valves, strainers, flange pairs, tees and reducers snap onto the pipe by their ports, take the line's size and class, and split the pipe. A tee starts a branch. Resize a line and everything on it re-sizes.
- **Check.** Clashes (pipe–pipe, pipe–equipment, pipe–ground) with a clearance, open ends, mismatches, class rules and missing line numbers, each one a click away.
- **Hand over.** Line list, equipment and nozzle list, material take-off (CSV); a piping component file (.pcf) per line for Datum Pipe; glTF 2.0 for any 3D viewer; print to PDF.

Every part is **generated** from a dimension table and a shape rule, never downloaded: each table row names its source ([catalogue/SOURCES.md](catalogue/SOURCES.md)), and plug-in packs add your company's pipe classes and parts. Check dimensions against your supplier's data before ordering or building.

## The Datum family

Datum Layout works with **Datum Schema** (P&IDs; its plant package arrives in Layout 0.2) and **Datum Pipe** (isometrics and stress). See [docs/MEMORY.md](docs/MEMORY.md) for the roadmap and how it is built, and the family's design guide in [Edgar-Mendonca/Datum](https://github.com/Edgar-Mendonca/Datum/blob/HEAD/FAMILY.md).

## Build from source

```bash
npm ci
npm run build:ui
cargo test --workspace
npx tauri build --bundles nsis
```

MIT licence, © 2026 Edgar Mendonca. Product and file-format names are trademarks of their owners and are used only to describe file compatibility.
