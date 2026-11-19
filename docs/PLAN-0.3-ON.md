# Plan: closing the gaps, 0.3 to 1.0 (agreed 2026-11-19)

After 0.2.14 the user asked where Datum Layout stands against the established plant-design
tools and chose to close the gaps in small numbered steps, built and checked one at a time.
Order agreed: steel, routing, catalogue, drawings, exchange; then big plants and validation.
A real job tried on the user's Windows PC runs alongside; whatever it finds goes first.

## 0.3.x Steel

| Step | What |
| --- | --- |
| 0.3.0 | Steel sections table (IPE, HEA, HEB; sourced, area checked) and single members: column, beam, brace |
| 0.3.1 | Pipe rack as one object (bents, spacing, tiers, width); lines snap to tier elevations |
| 0.3.2 | Supports stand on steel; Check reports a support with nothing under it |
| 0.3.3 | Platforms and ladders on vessels and columns |
| 0.3.4 | Free-standing platforms and stairs; access check (valves reachable from grade or a platform) |
| 0.3.5 | Steel in the take-off with weights; clashes with steel; steel in glTF |

## 0.4.x Routing

| Step | What |
| --- | --- |
| 0.4.0 | Sloped lines (fall written to the PCF) |
| 0.4.1 | Route Again avoids clashes |
| 0.4.2 | Auto-route between two nozzles, along racks |
| 0.4.3 | Move and turn nozzles in 3D; start a line from a nozzle |
| 0.4.4 | Line spacing on racks (flange and insulation) |

## 0.5.x Catalogue gaps (only with manufacturers' data)

| Step | What |
| --- | --- |
| 0.5.0 | Weights for globe, check and ball valves, caps, gaskets, bolts |
| 0.5.1 | Spectacle blinds, spades, spacers |
| 0.5.2 | Control valves and relief valves |
| 0.5.3 | In-line instruments: orifice flanges, meters, thermowells, tappings |
| 0.5.4 | Small bore: socket-weld and threaded fittings and classes |

## 0.6.x Drawings

| Step | What |
| --- | --- |
| 0.6.0 | Plot plan (PDF, SVG) |
| 0.6.1 | General arrangement plans and sections with dimensions |
| 0.6.2 | Title blocks, revisions, drawing list |
| 0.6.3 | Equipment nozzle orientation drawings (isometrics stay in Datum Pipe) |
| 0.6.4 | Full take-off report (PDF) and DXF |

## 0.7.x Exchange and equipment detail

| Step | What |
| --- | --- |
| 0.7.0 | STEP import of vendor equipment (optional engine on first use) |
| 0.7.1 | IFC export |
| 0.7.2 | Equipment detail: skirt openings, saddle positions, pads |

## Later

0.8.x big plants and teams (areas on demand, history, several users); 0.9.x every table checked
against its source, user guide, Windows hardening; then 1.0. From Schema runs alongside (it
waits on Datum Schema's export).
