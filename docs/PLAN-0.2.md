# Datum Layout after 0.1: where it stands and what comes next

Written 2026-11-08, after 0.1.4. Compares Layout with what established 3D plant-design systems do (described by capability, not by name), and proposes the next releases. Nothing here is built until the user agrees.

## 1. Feedback → diagnosis

- 0.1 does the core well: equipment from datasheet sizes, spec-driven routing, snapping fittings, clash and spec checks, lists, and a hand-over to Datum Pipe that Pipe's own engine reads and solves.
- It is still a **small-project tool**: one plant, a few dozen lines, one person, one pipe class, no drawings, no supports or steel, no way to bring in a line list or a P&ID.
- The roadmap's next release, **0.2 From Schema, is blocked**: it needs Datum Schema's plant package, which is on Schema's roadmap as 0.8 and not built.
- The hub's rule is **validate before adding**: an app gets real use before more breadth. Layout has not yet run on a real PC or been used on a real job.

Rough maturity against an established single-user plant-design system: **about 10–15%**. The core model and the checks are sound; most of the gap is breadth (parts, drawings, steel, supports) and everyday editing comfort.

## 2. Gap analysis

✅ have · 🟡 partly · ⬜ missing. Priority: **A** needed for real small jobs, **B** expected by designers, **C** large projects and teams.

| Area | Established systems | Layout today | | Priority |
| --- | --- | --- | --- | --- |
| **Editing comfort** | Multi-select, copy, mirror, rotate groups; move with snapping to objects; measure distance; section box and hide/isolate; saved views; colour by line, class or system | Single selection; move one item or run; no measure, no clipping, no saved views | ⬜ | **A** |
| **Routing** | Auto-route around obstacles; sloped lines (fall); pulled bends; route from any point on a pipe; insert and drag corners; minimum straight lengths | Orthogonal click routing, Route Again, move a run | 🟡 | **A** (corners, slope), B (auto-route) |
| **Parts** | Full catalogue: olets, socket-weld and threaded small bore, couplings, unions, eccentric reducers, crosses, 3D/5D bends, stub ends, many flange types, spectacle blinds, butterfly/plug/control/relief valves, instruments | ½"–24" butt-weld, weld-neck flanges 150/300, four valve types, strainer | 🟡 | **A** (from the catalogue work under way) |
| **Pipe classes** | Class editor; branch table (tee or olet by run × branch size); per-size component choice; materials, ratings, corrosion | One example class, no editor, no branch table | 🟡 | **A** |
| **Equipment** | Many types (compressors, air coolers, fired heaters, filters, reactors, agitated tanks, heat exchangers of several kinds); nozzles by angle and elevation; vessel platforms; equipment from vendor models | Six parametric types; nozzles by position and direction | 🟡 | B |
| **Supports** | Support catalogue (shoes, guides, anchors, hangers, trunnions), placed on lines, loads from stress back on the model | None | ⬜ | **A** (with Pipe hand-over) |
| **Steel** | Racks, platforms, ladders, stairs, handrails, structural members from section tables | None | ⬜ | B |
| **Drawings** | Isometrics (Datum Pipe does these), general arrangement plans and sections with automatic dimensions and labels, plot plans, nozzle orientation drawings | 3D view print only | ⬜ | B |
| **Reports** | Line list, valve list, MTO, weld count, spools with cut lengths and field welds, equipment and nozzle lists | Line list, equipment list, MTO | 🟡 | B (valve list, welds, spools) |
| **Checks** | Hard/soft clashes, maintenance and access volumes, hold areas, design rules (straight run before pump suction, valve handwheel height, clearances), consistency with the P&ID | Clashes with clearance, open ends, mismatches, class rules, numbers | 🟡 | B (design rules, access volumes), C (P&ID) |
| **Exchange** | P&ID integration, line list import, PCF in and out, stress hand-over both ways, IFC, STEP, laser-scan point clouds | PCF out (proved with Pipe), glTF out | 🟡 | **A** (line list CSV in, PCF in), B (IFC), C (STEP, scans) |
| **Data and teams** | Object database, revisions and change tracking, multi-user with claims, review mark-ups | One JSON file, undo | ⬜ | C |
| **Scale** | Tens of thousands of lines, areas loaded on demand | Smooth to about 2,400 parts | 🟡 | C |

## 3. Proposal: the next releases

The order puts what makes Layout usable on a real small job first, then the family hand-overs, and moves the Schema import to when Schema can export.

| Release | Title | Scope |
| --- | --- | --- |
| **0.1.5–0.1.9** | **Real use** | Run it on a real Windows PC and fix what that finds; measure tool; multi-select with copy, mirror, rotate and delete; insert, delete and drag route corners; colour by line, class or fluid; section box and hide/isolate; saved views; line list import from CSV (create lines with number, size, class and from/to, to be routed) |
| **0.2** | **Catalogue and classes** | Merge the catalogue tables now being prepared (flanges to class 2500, valves, fittings, pipe); new part types from them: olets, socket-weld and threaded small bore, eccentric reducers, crosses, 3D/5D bends, slip-on and lap-joint flanges, spectacle blinds, butterfly and control valves; a pipe class editor with a branch table; eight example classes; sloped lines |
| **0.3** | **Supports and steel** | Support catalogue placed on lines (shoe, rest, guide, anchor, hanger, trunnion), sent to Datum Pipe in the PCF; Pipe's support loads read back and shown; parametric pipe racks, platforms, ladders and stairs from section tables |
| **0.4** | **Drawings and reports** | Plans and sections with automatic dimensions and labels; plot plan; nozzle orientation drawing; valve list, weld count, spools with cut lengths; IFC export |
| **0.5** | **From Schema** | When Schema 0.8 exports the plant package: import it, place and route from it, and the 3D-against-P&ID check |
| **0.6** | **CAD exchange** | Optional engine on first use: STEP import of vendor equipment, STEP export |
| **0.7** | **Big models and teams** | Areas on demand, a project database with history, claims for several users |
| **1.0** | **Validated** | Catalogue checked against its sources, user guide, signed installer |

Design rules (straight run before pump suctions, handwheel heights, access volumes) and auto-routing around obstacles are candidates to add in 0.3–0.4 once the basics are in daily use.

## 4. Size

- 0.1.5–0.1.9: several small releases, mostly window work on the existing engine.
- 0.2: the largest data job; each new part type is a shape rule, ports and a test against its table.
- 0.3 and 0.4: one or two sessions each.

## 5. Questions for the user

1. Is the new order right: real use and the catalogue first, Schema import after Schema 0.8?
2. Can Layout be tried on a real Windows PC (and a real small job) before 0.2, as the hub's "validate before adding" rule asks?
3. Which matters more for your work after the catalogue: **supports and steel** (0.3), or **drawings and reports** (0.4)?
4. Are imperial classes (150–2500) enough for now, or are metric EN flanges (PN) needed early?
