# Brief 2: more standard parts for Datum Layout (one big pass)

You are helping build the parts catalogue of **Datum Layout**, a free, open-source (MIT) desktop app for 3D plant layout and piping. The app **generates** every part from a few dimensions (a shape rule in code plus one row of a table), so what is needed from you is **dimension tables with sources**, not 3D models. A program and an engineer check your files before they go into the app.

You already delivered the first pack (pipe, butt-weld fittings, weld-neck and blind flanges class 150–2500, gate/globe/check/ball valves). This second pack covers what the next releases need. **Do all of it in one answer**, in the order below (most important first). If the answer gets long, split files into parts as described in rule 7, and keep going until every file is complete.

---

## 0. Rules (same as before; read first)

1. **Every row needs a `source`**: a manufacturer's public catalogue (company, catalogue title, edition or year, page or table), or a public reference. Example: `Acme Fittings, Forged Fittings Catalogue 2019, p. 12`.
2. **Do not invent numbers.** If unsure, still give your best value but start the source with `UNVERIFIED:` (e.g. `UNVERIFIED: typical value, recalled`). An honest "unverified" is better than a confident wrong number. Leave a cell blank only where the part does not exist in that size.
3. **Do not copy a standard's text or reproduce its tables as a document.** Give dimensions as they appear in public manufacturers' catalogues. You may name standards by number in `source` ("dimensions per ASME B16.11 as listed in …").
4. **Never name or compare with any commercial plant-design, CAD or 3D software**, anywhere.
5. **Units: inches** for lengths (columns end `_in`), **pounds** for weights (columns end `_lb`). Nominal sizes are written exactly `1/2`, `3/4`, `1`, `1-1/4`, `1-1/2`, `2`, `2-1/2`, `3`, `3-1/2`, `4`, `5`, `6`, `8`, `10`, `12`, `14`, `16`, `18`, `20`, `22`, `24`. Flange classes are `150`, `300`, `600`, `900`, `1500`, `2500`; forged small-bore classes are `3000`, `6000`.
6. **CSV**: comma-separated, one header row exactly as given, one part per row, no blank rows, no units in cells, decimal point `.`, no thousands separators, double quotes around a cell containing a comma, UTF-8.
7. **Deliver each file in its own code block** headed by its file name (`### olets.csv`), complete, with no "…" or "rows omitted". If a file is too long, split it as `olets.csv (part 1 of 2)` with the header only in part 1.

---

## 1. `olets.csv`: branch outlets (highest priority)

Weldolets and sockolets: small branches welded onto a larger header without cutting it into a tee.

Header: `kind,nps_branch,run_min,run_max,height_in,base_od_in,bore_in,source`

- `kind`: `weldolet` (butt-weld outlet, branch ½"–12") or `sockolet` (socket-weld outlet, class 3000, branch ½"–2").
- `run_min`, `run_max`: the range of header sizes the outlet fits (as manufacturers list them, e.g. `3` to `4`).
- `height_in`: from the header's outside surface to the outlet's end (the "A" dimension in most catalogues).
- `base_od_in`: the outlet's largest diameter where it sits on the header.
- `bore_in`: for weldolets the branch bore matched to standard weight; for sockolets the socket bore.

## 2. `tees_reducing.csv`: reducing tees

Header: `nps,nps_branch,c_in,m_in,source`

- Every run size 1"–24" with every branch size the catalogue lists for it (branch smaller than the run).
- `c_in`: centre to end of the run; `m_in`: centre to end of the branch.

## 3. `reducers.csv`: concentric and eccentric reducers by size pair

Header: `nps,nps_small,h_in,source`

- Every large × small pair the catalogue lists, ¾"–24". `h_in`: end to end. (Concentric and eccentric have the same length; one table serves both.)

## 4. `valves_other.csv`: more valves

Header: `kind,rating,nps,face_to_face_in,height_in,handwheel_in,source`

- `kind`: `butterfly_wafer`, `butterfly_lug`, `check_wafer` (dual-plate), `plug`, `ball_short` (short-pattern ball), `gate_short` (short-pattern gate).
- Classes 150 and 300 (and 600 where the catalogue lists them), sizes 2"–24" (plug and short-pattern ball from ½").
- `height_in`: pipe centreline to top of stem or operator (valve open); `handwheel_in`: handwheel diameter or lever length (blank if gear-operated and not listed).

## 5. `weights.csv`: weights, for the stress analysis (new)

Datum Pipe uses component weights in its stress analysis. One table for everything already in the catalogue.

Header: `part,rating,nps,nps2,weight_lb,source`

- `part`: `pipe_per_ft` (with `rating` blank and `nps2` = the schedule: `40`, `80`, `STD`, `XS`, `10S`, `40S`), `elbow90_lr`, `elbow45_lr`, `tee`, `reducer` (with `nps2` = small size), `cap`, `flange_wn`, `flange_blind`, `gate`, `globe`, `check`, `ball`, `strainer`.
- Butt-weld fittings: standard weight wall, `rating` blank. Flanges and valves: by class 150, 300, 600 (and 900–2500 for flanges if listed).
- Sizes ½"–24" as the catalogue lists them.

## 6. `blinds_spectacle.csv`: spectacle blinds, spades and spacers

Header: `nps,rating,type,thickness_in,od_in,web_length_in,source`

- `type`: `spectacle` (figure-8), `spade`, `spacer`. Classes 150, 300, 600; sizes ½"–24".
- `od_in`: the outside diameter of the blank or spacer disc; `web_length_in`: centre-to-centre of a spectacle's two discs (blank for spade and spacer).

## 7. `shoes.csv`: pipe shoes for supports

Header: `nps,shoe_height_in,shoe_length_in,shoe_width_in,source`

- Typical welded T-shoes for rests and guides, ½"–24", for bare pipe and for insulation up to 3" (give the height that clears 3" of insulation; say so in `source` if the catalogue lists several).

## 8. Pipe classes with a branch table (JSON)

The app's classes are JSON files like this one (existing format, **plus a new `branches` list**):

```json
{
  "name": "CS300",
  "description": "Carbon steel, butt-weld fittings, class 300 raised-face weld-neck flanges; process service to about 50 bar",
  "material": "Carbon steel",
  "ratings": [300],
  "sizes": ["1/2", "3/4", "1", "1-1/2", "2", "3", "4", "6", "8", "10", "12", "14", "16", "18", "20", "24"],
  "schedules": [["1/2", "80"], ["2", "40"], ["8", "STD"]],
  "elbow": "long",
  "components": [
    { "kind": "pipe" }, { "kind": "elbow90" }, { "kind": "elbow45" }, { "kind": "tee" }, { "kind": "cross" }, { "kind": "reducer" },
    { "kind": "cap" }, { "kind": "strainer" }, { "kind": "flange" }, { "kind": "blind" }, { "kind": "gasket" }, { "kind": "bolts" },
    { "kind": "gate", "min": "2", "max": "24" }, { "kind": "globe", "min": "1/2", "max": "12" },
    { "kind": "check", "min": "2", "max": "24" }, { "kind": "ball", "min": "1/2", "max": "12" },
    { "kind": "weldolet" }, { "kind": "sockolet", "max": "1-1/2" }, { "kind": "butterfly_wafer", "min": "3" }
  ],
  "branches": [
    { "header_min": "1/2", "header_max": "24", "branch": "same", "fitting": "tee" },
    { "header_min": "2", "header_max": "24", "branch_min": "1/2", "branch_max": "1-1/2", "fitting": "sockolet" },
    { "header_min": "3", "header_max": "24", "branch_min": "2", "branch_max": "half", "fitting": "weldolet" },
    { "header_min": "2", "header_max": "24", "branch_min": "half_up", "branch_max": "one_smaller", "fitting": "reducing_tee" }
  ],
  "source": "Example class written for Datum Layout; not any company's specification"
}
```

- `branches` says which fitting makes each header × branch size combination, the way a real specification's branch table does. `branch` may be a size, `same` (equal to the header), `half` (half the header size or less), `half_up` (more than half), `one_smaller` (one size below the header). The first rule that matches wins.
- `components[].kind` may now also be: `cross`, `weldolet`, `sockolet`, and any `kind` from `valves_other.csv`.
- **Wanted:** rewrite the 8 example classes you made before with `branches` and the new kinds (CS150, CS300, CS600, CS900/1500, LT150, SS150 [316], UW150 [utility water], CS1 [kept as is, add `branches`]), **and add** `SS300` (stainless 316, class 300) and `ST300` (carbon steel steam, class 300, schedule 80 to 2", with globe valves allowed to 12"). Each one clearly an example, not any company's specification, with a one-sentence reason for its choices in `description`.

## 9. Small-bore forged fittings (if there is room)

`small_bore.csv` with header `kind,rating,nps,a_in,b_in,od_in,weight_lb,source`: `kind` = `elbow90`, `elbow45`, `tee`, `coupling`, `half_coupling`, `cap`, `union`; socket-weld, classes 3000 and 6000, ½"–2". `a` centre to end (or end to end for couplings and caps), `b` socket depth.

---

## Before you hand over, check

1. Every row has a non-empty `source`; rows from memory start with `UNVERIFIED:`.
2. Sizes and classes spelled exactly as in rule 5; headers exactly as given.
3. Dimensions and weights grow with size within each table, and with class (a higher class is never thinner or lighter).
4. Reducing tees: `m_in ≤ c_in`; reducers: `nps_small` smaller than `nps`; olets: `run_min ≤ run_max` and the branch smaller than the run.
5. No duplicate keys (same kind + class + size(s)).
6. JSON files are valid JSON (no comments, no trailing commas).
7. Nothing names a commercial design program.

At the end, a short **report**: rows per file, how many `UNVERIFIED`, what you left out and why, and any doubts.
