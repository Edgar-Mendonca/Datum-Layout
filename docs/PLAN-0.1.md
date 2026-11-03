# Datum Layout 0.1: plan

From the brief (`Edgar-Mendonca/Datum` › `briefs/DATUM-LAYOUT.md`, hub session 31). Every decision is made there; this file says how 0.1 is built and in what order.

## Feedback → diagnosis
- The family's plant workflow is four apps: Schema (P&ID, plant package) → **Layout** (3D layout of one plant) → Pipe (isometrics, stress) and Sim (dynamics). Layout models geometry only.
- Open parts libraries are GPL/LGPL or licensed per asset, and vendor models cannot be redistributed, so every part is **generated** from a dimension table and a shape rule.

## Proposal
1. **Engine first** (`crates/core`): units (from Pipe/Calc), catalogue (CSV/JSON tables with sources, plug-in packs), parts (shapes → meshes, ports), the six equipment types, the model (objects with ids and uids after Schema, `.dlayout` with format and repair), edits as operations with undo, routing (path → straights + spec elbows with exact cut-backs, reducers, end flanges; drop components by snapping to the nearest free place; branch tees), checks (parry3d clashes, open ends, mismatches, class rules, numbers), lists and take-off, PCF (writer, plus Pipe's reader for the tests) and glTF.
2. **Window**: Pipe's shell (itself Plan's) with Plan's hardening; a catalogue panel on the left (drag source), the 3D view (one buffer, colour per part, picking by triangle, orthogonal routing guides, snap marker while dragging, axis arrows to move), the Inspector, and the Lines · Equipment · Clashes · Checks tabs.
3. **Tests**: parts against their tables, ports coincide and face each other, no gaps or overlaps, lengths add up, 90°/45°/too-tight turns, resizing; each example's PCF read by Pipe's reader; glTF verified; hardening (failed edits, random edits, damaged files, 2,000-part speed gate); UI tests driving the real engine through a bridge.

## Size
About 5,000 lines of Rust and 3,000 of TypeScript, in one run.

## Releases
| Release | Content |
| --- | --- |
| 0.1.0 | Shell, plot, 3D view, and the engine as built (equipment, routing, fittings, checks, lists, exports, examples) |
| 0.1.1 | Placing, routing and the Inspector, polished from use |
| 0.1.2 | Drag and drop, resize, branches: refined and tested |
| 0.1.3 | Checks, lists, PCF proved against Datum Pipe itself, glTF validated |
| 0.1.4 | Examples, print, help, screenshots, README and site, final check |

## Questions
None: the run is unattended. Anything unclear was decided simply and recorded in `docs/MEMORY.md` (Decisions).
