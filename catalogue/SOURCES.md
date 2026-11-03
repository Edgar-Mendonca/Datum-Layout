# Where the catalogue comes from

Every part Datum Layout draws is **generated**: a shape rule in code (`crates/core/src/parts.rs`) plus a row of a dimension table here, plus ports. No 3D model is downloaded or bundled. Every table row carries its source in its `source` column; this file explains them.

| File | What it holds | Source |
| --- | --- | --- |
| `pipe.csv` | Nominal pipe sizes ½"–24": outside diameter and the wall of each schedule (inches) | The published nominal dimensions of wrought steel pipe (ASME B36.10M), as listed in Datum Pipe's library (`crates/core/src/library.rs` there) |
| `fittings.csv` | Butt-weld fittings: long- and short-radius elbow centre-to-end (90°), tee centre-to-end, reducer length, cap length | Typical values from manufacturers' public fitting catalogues. Elbow radii follow the usual rule (long radius 1.5 × NPS, short radius 1 × NPS; ½" long radius is 1½") |
| `flanges.csv` | Weld-neck and blind flanges, class 150 and 300: outside diameter, thickness, length through hub, raised-face diameter, number and size of bolts, gasket thickness | Typical values from manufacturers' public flange catalogues |
| `valves.csv` | Flanged gate, globe, check and ball valves, class 150 and 300: face-to-face length | Typical values from manufacturers' public valve catalogues |
| `classes/CS1.json` | The one pipe class of 0.1: carbon steel, butt-weld fittings, class 150 or 300 raised-face flanges, schedules by size, the components it allows and their size ranges | An example class written for Datum Layout; not any company's class |

No standard's tables or text are reproduced: the files hold dimensions only, as they appear in public catalogues, and are checked against those catalogues before 1.0 (roadmap). **Check every dimension against your supplier's data before ordering or building.**

## Plug-in packs

A pack is a folder (or a .zip of one) the user adds in the Catalogue panel. It may hold any of the files above with the same columns (its rows add to or replace the built-in rows of the same size and rating), and `classes/*.json` for more pipe classes. Every row needs a `source`. Rows that cannot be read are listed, and the rest of the pack is used.
