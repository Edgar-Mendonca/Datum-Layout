# Brief for Datum Schema: export the plant package

**For:** a Datum Schema session (Schema roadmap 0.9.0, "Plant package export for 3D modelling").
**From:** Datum Layout 0.2.1, which already reads the package and has a worked example.

## Goal

File › Export › Plant Package writes the project's equipment and process lines as one JSON file in the
format of `Edgar-Mendonca/Datum-Layout` › `docs/formats/plant-package.md` (version 1). Datum Layout
reads it, places the equipment, lists the lines, routes them and checks the 3D model against the
P&ID. Copy the format document into Schema's own docs; never link code between the apps.

## What to write, from Schema's model

- **Equipment:** every object whose class is an equipment class: `uid`, `tag`, `class` (the DEXPI
  name Schema already uses), service, datasheet values as text with units (SI), and its nozzles
  (tag, size, flange class, service) where the datasheet or symbol ports give them.
- **Lines:** every `PipingNetworkSystem` object: `uid`, line number (tag), size, spec (as `class`),
  rating, fluid code, insulation (mm). Ends: walk the line's drawn pieces in flow direction (a → b,
  unless `flip`) to the first and last symbol port: an equipment nozzle (`equipment` + `nozzle`), a
  branch (`line`), or open (`{}`).
- **Components:** the valves, strainers, reducers, tees and spectacle blinds on the line, in flow
  order, with `uid`, `kind` (gate, globe, check, ball, strainer, reducer, tee, flange_pair) and tag.
  Symbols with no 3D part yet may be written with their own kind; Layout reports and skips them.

## Done when

- The pump transfer example exports a package that Datum Layout imports with no warnings, and its
  Route Listed Lines leaves only branches to route by hand.
- A test in Schema writes each example's package and reads it back with serde (the keys and types
  of the format document).
- The format version is 1; a change that breaks readers bumps it.
