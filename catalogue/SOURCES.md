# Where the catalogue comes from

Every part Datum Layout draws is **generated**: a shape rule in code (`crates/core/src/parts.rs`) plus a row of a dimension table here, plus ports. No 3D model is downloaded or bundled. Every table row carries its source in its `source` column; this file explains them.

| File | What it holds | Source |
| --- | --- | --- |
| `pipe.csv` | Nominal pipe sizes ½"–24": outside diameter and the wall of each schedule (inches) | The published nominal dimensions of wrought steel pipe (ASME B36.10M), as listed in Datum Pipe's library (`crates/core/src/library.rs` there) |
| `fittings.csv` | Butt-weld fittings: long- and short-radius elbow centre-to-end (90°), tee centre-to-end, reducer length, cap length | A manufacturer's public fitting catalogue, named on each row. Two values kept from the earlier table and noted on their rows: the ¾" long-radius elbow (1⅛" = 1.5 × NPS; the source row repeats the ½" value) and the ½" reducer length |
| `flanges.csv` | Weld-neck and blind flanges, class 150, 300, 600, 900, 1500 and 2500, ½"–24" where made: outside diameter, thickness, length through hub, raised-face diameter, number and size of studs, gasket thickness, and the blind's thickness where it differs (`blind_thick_in`) | Manufacturers' public flange data sheets and catalogue pages, named on each row |
| `valves.csv` | Flanged gate, globe, swing-check and ball valves, class 150 to 2500: face-to-face length | Manufacturers' public valve catalogues, named on each row with page; rows not fully confirmed start with `UNVERIFIED:`. A size not listed is interpolated between its neighbours of the same kind and class |
| `classes/*.json` | Example pipe classes: CS1 (used by the examples), CS150, CS300, CS600, CS1500, LT150, SS150, UW150, each with material, ratings, schedules by size, and the components it allows with their size ranges | Examples written for Datum Layout; not any company's class |

Tables gathered in November 2026 with the help of an outside assistant working from manufacturers' public catalogues; every row was read by the app's checker and compared with the earlier tables before it was merged. No standard's tables or text are reproduced: the files hold dimensions only, as they appear in public catalogues, and are checked against those catalogues before 1.0 (roadmap). **Check every dimension against your supplier's data before ordering or building.**

## Plug-in packs

A pack is a folder (or a .zip of one) the user adds in the Catalogue panel. It may hold any of the files above with the same columns (its rows add to or replace the built-in rows of the same size and rating), and `classes/*.json` for more pipe classes. Every row needs a `source`. Rows that cannot be read are listed, and the rest of the pack is used.
