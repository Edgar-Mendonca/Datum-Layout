# Brief: a standard-parts library for Datum Layout

You are helping build the parts catalogue of **Datum Layout**, a free, open-source (MIT) desktop app for 3D plant layout and piping. The app **generates** every part from a few dimensions (a shape rule in code plus one row of a table), so what we need from you is **dimension tables**, not 3D models. Your output will be checked by a program and by an engineer, then dropped into the app as a *plug-in pack*.

Please read the whole brief before starting. The exact file formats in Part A matter: the app rejects rows that do not match.

---

## 0. Rules (read first)

1. **Every row needs a `source`.** Name where the numbers come from as precisely as you can: a manufacturer's public catalogue (company, catalogue title, edition or year, page or table if known), or a public reference book or website. Example: `Acme Fittings, Butt-Weld Fittings Catalogue 2019, p. 12`.
2. **Do not invent numbers.** If you are not sure of a value, still give your best value but start the source with `UNVERIFIED:` (for example `UNVERIFIED: typical value, recalled`). We would rather have an honest "unverified" than a confident wrong number. Never leave a value blank to hide uncertainty; blank only where the part does not exist in that size.
3. **Do not copy a standard's text or reproduce its tables as a document.** Give dimensions only, as they appear in public manufacturers' catalogues. You may name standards by number in the `source` (e.g. "dimensions per ASME B16.9 as listed in …").
4. **Do not name or compare with any commercial plant-design, CAD or 3D software**, anywhere in the files.
5. **Units: inches** for every dimension (columns end in `_in`), exactly as in the existing files. Nominal sizes are written `1/2`, `3/4`, `1`, `1-1/4`, `1-1/2`, `2`, `2-1/2`, `3`, `3-1/2`, `4`, `5`, `6`, `8`, `10`, `12`, `14`, `16`, `18`, `20`, `22`, `24` (the app reads only these today; see §B.9 for larger sizes).
6. **CSV format:** comma-separated, a header row, one part per row, no blank rows, no units inside cells, decimal point `.`, no thousands separators. Put a cell in double quotes if it contains a comma. UTF-8.
7. **Deliver each file in its own code block** headed by its file name (for example `### flanges.csv`), complete, with no "…" or "rows omitted". If a file is too long for one answer, split it into parts `flanges.csv (part 1 of 3)` with the header only in part 1, and I will join them.

---

## Part A: files the app reads today (exact formats)

The pack is a folder:

```
pack/
  pipe.csv
  fittings.csv
  flanges.csv
  valves.csv
  classes/
    CS150.json
    SS316-150.json
    ...
```

Rows in a pack **add to or replace** the built-in rows with the same key (size + schedule; size; size + rating; kind + rating + size).

### A.1 `pipe.csv`: pipe outside diameter and wall per schedule

```
nps,dn,od_in,schedule,wall_in,source
6,150,6.625,40,0.280,Datum Pipe library: published nominal dimensions of wrought steel pipe (ASME B36.10M)
```

- `dn`: the metric nominal size (15, 20, 25, 32, 40, 50, 65, 80, 90, 100, 125, 150, 200, 250, 300, 350, 400, 450, 500, 550, 600).
- `schedule`: `5S`, `10S`, `10`, `20`, `30`, `40`, `40S`, `STD`, `60`, `80`, `80S`, `XS`, `100`, `120`, `140`, `160`, `XXS`.
- Check: `wall_in × 2 < od_in`.
- **Wanted:** every schedule that exists for every size ½"–24", carbon steel (B36.10M) and stainless (B36.19M, the `S` schedules). The built-in table already has most of these; fill gaps and correct anything wrong.

### A.2 `fittings.csv`: butt-weld fittings

```
nps,elbow_lr_in,elbow_sr_in,tee_c_in,reducer_h_in,cap_e_in,source
6,9,6,5.62,5.5,3.5,Typical butt-weld fitting dimensions from manufacturers' public catalogues
```

| Column | Meaning |
| --- | --- |
| `elbow_lr_in` | 90° long-radius elbow, centre to end (= bend radius) |
| `elbow_sr_in` | 90° short-radius elbow, centre to end (blank below 1") |
| `tee_c_in` | Equal tee, centre to end of run (C) |
| `reducer_h_in` | Concentric reducer, end to end (H), for a reducer whose large end is this size |
| `cap_e_in` | Cap, length (E) |

One row per size ½"–24". (45° elbows are worked out from the radius, so no column.)

### A.3 `flanges.csv`: weld-neck and blind flanges

```
nps,rating,od_in,thick_in,wn_length_in,raised_face_in,bolts,bolt_in,gasket_in,source
6,150,11.00,1.00,3.50,8.50,8,0.75,0.125,Typical flange dimensions from manufacturers' public catalogues
```

| Column | Meaning |
| --- | --- |
| `rating` | Pressure class: 150, 300, 600, 900, 1500, 2500 |
| `od_in` | Flange outside diameter |
| `thick_in` | Flange thickness (also used for the blind flange) |
| `wn_length_in` | Weld-neck length through hub (Y) |
| `raised_face_in` | Raised-face diameter |
| `bolts` | Number of bolts |
| `bolt_in` | Bolt (stud) diameter |
| `gasket_in` | Gasket thickness (0.125 for a typical spiral-wound gasket) |

Checks: `wn_length_in > thick_in`, `raised_face_in < od_in`, `bolts` between 4 and 64.
**Wanted:** classes **150, 300, 600, 900, 1500, 2500**, every size ½"–24" that exists in each class. (150 and 300 exist built in; please check them too.)

### A.4 `valves.csv`: flanged valve face-to-face lengths

```
kind,rating,nps,face_to_face_in,source
gate,150,6,10.5,Typical face-to-face lengths of flanged valves from manufacturers' public catalogues
```

- `kind` is one of **`gate`, `globe`, `check`, `ball`** (the app reads only these four today; other kinds go in Part B). `check` = swing check. `ball` = long-pattern flanged ball valve.
- **Wanted:** all four kinds × classes 150, 300, 600, 900, 1500, 2500 × every size ½"–24" that a manufacturer actually makes in that class (leave out combinations that do not exist rather than guessing).

### A.5 `classes/*.json`: pipe classes (piping specifications)

```json
{
  "name": "CS150",
  "description": "Carbon steel, butt-weld fittings, class 150 raised-face weld-neck flanges",
  "material": "Carbon steel",
  "ratings": [150],
  "sizes": ["1/2", "3/4", "1", "1-1/2", "2", "3", "4", "6", "8", "10", "12", "14", "16", "18", "20", "24"],
  "schedules": [["1/2", "80"], ["2", "40"], ["8", "STD"]],
  "elbow": "long",
  "components": [
    { "kind": "pipe" }, { "kind": "elbow90" }, { "kind": "elbow45" }, { "kind": "tee" }, { "kind": "reducer" },
    { "kind": "cap" }, { "kind": "strainer" }, { "kind": "flange" }, { "kind": "blind" }, { "kind": "gasket" }, { "kind": "bolts" },
    { "kind": "gate", "min": "2", "max": "24" }, { "kind": "globe", "min": "1/2", "max": "12" },
    { "kind": "check", "min": "2", "max": "24" }, { "kind": "ball", "min": "1/2", "max": "12" }
  ],
  "source": "Example class written for Datum Layout; not any company's specification"
}
```

- `schedules`: `[from size, schedule]` pairs; each applies from that size up to the next pair.
- `components[].kind` must be one of: `pipe`, `elbow90`, `elbow45`, `tee`, `reducer`, `cap`, `strainer`, `flange`, `blind`, `gasket`, `bolts`, `gate`, `globe`, `check`, `ball`. `min`/`max` limit the sizes.
- **Wanted:** about 8 **generic, realistic example classes** (clearly labelled as examples, not copied from any company): carbon steel 150 / 300 / 600; low-temperature carbon steel 150; stainless 316 150 / 300; a utility water class 150; a steam class 300. Choose schedules and size limits as a typical specification would, and explain each choice in one sentence in `description`.

---

## Part B: new part types (the app will add shape rules for these)

Same rules (inches, a `source` on every row). Propose the columns below; if a part needs another dimension to be drawn correctly, add a column and explain it in a short note under the file. I will write the shape rules and ports.

| # | File | Rows | Columns |
| --- | --- | --- | --- |
| B.1 | `elbows_45_3d.csv` | per size | `nps, elbow45_lr_in, elbow3d_90_in, elbow5d_90_in, source` (45° LR centre-to-end; 3D and 5D bends) |
| B.2 | `tees_reducing.csv` | per run × branch size | `nps, nps_branch, c_in, m_in, source` (run C, branch M) |
| B.3 | `crosses.csv` | per size | `nps, c_in, source` |
| B.4 | `reducers_ecc.csv` | per large × small size | `nps, nps_small, h_in, source` (eccentric reducer; the offset follows from the diameters) |
| B.5 | `stub_ends.csv` | per size | `nps, length_short_in, length_long_in, lap_od_in, source` |
| B.6 | `flanges_other.csv` | per type × size × rating | `type, nps, rating, od_in, thick_in, hub_length_in, raised_face_in, bolts, bolt_in, source` with `type` = `slip_on`, `socket_weld`, `threaded`, `lap_joint`, `orifice`, `reducing` (add `nps_bore` for reducing) |
| B.7 | `blinds_spectacle.csv` | per size × rating | `nps, rating, type, thickness_in, od_in, web_length_in, source` with `type` = `spectacle`, `spade`, `spacer` |
| B.8 | `valves_other.csv` | per kind × rating × size | `kind, rating, nps, face_to_face_in, height_in, handwheel_in, source` with `kind` = `butterfly_wafer`, `butterfly_lug`, `plug`, `needle`, `diaphragm`, `check_wafer`, `check_lift`, `ball_short`, `gate_short`, `control_globe`, `relief` (relief: add `outlet_nps`, `inlet_to_outlet_in`) |
| B.9 | `pipe_large.csv`, `fittings_large.csv`, `flanges_large.csv` | sizes 26"–48" | Same columns as A.1–A.3 (the app needs a small change to read them; keep them in separate files) |
| B.10 | `small_bore.csv` | socket-weld and threaded fittings ½"–2", classes 3000 / 6000 | `kind, rating, nps, a_in, b_in, od_in, source` with `kind` = `elbow90`, `elbow45`, `tee`, `coupling`, `half_coupling`, `cap`, `union`; `a` centre-to-end, `b` socket depth |
| B.11 | `olets.csv` | per run size range × branch size | `kind, nps_branch, run_min, run_max, height_in, base_od_in, source` with `kind` = `weldolet`, `sockolet`, `threadolet`, `elbolet`, `nipolet` |
| B.12 | `strainers.csv` | per type × rating × size | `type, rating, nps, face_to_face_in, leg_length_in, leg_angle_deg, source` with `type` = `y`, `t`, `basket` |
| B.13 | `metric_en.csv` | DN 15–600, PN 10/16/25/40 | `dn, pn, pipe_od_mm, flange_od_mm, flange_thick_mm, wn_length_mm, bolts, bolt_mm, source` (metric flanges; this file is in **millimetres**, say so in the header comment) |
| B.14 | `gaskets.csv`, `bolting.csv` | per size × rating | gaskets: `type, nps, rating, id_in, od_in, thick_in, source` (`spiral_wound`, `ring_joint`, `flat`); bolting: `nps, rating, stud_length_in, nut_size_in, source` |

**Equipment (optional, lower priority):** `motors.csv` with typical frame dimensions of standard electric motors by power (kW and hp: `power_kw, frame, length_in, diameter_in, shaft_height_in, source`), and `pumps_end_suction.csv` with typical dimensions of end-suction centrifugal pumps by suction × discharge × impeller size (`suction_nps, discharge_nps, impeller_in, casing_dia_in, shaft_height_in, base_length_in, base_width_in, source`).

---

## Part C: before you hand over, check

Please run these checks yourself (mentally or with code) and fix what fails; then list in your answer which checks you ran.

1. Every row has a non-empty `source`; rows from memory start with `UNVERIFIED:`.
2. Every size is spelled exactly as in rule 5; every rating is one of the listed classes.
3. Dimensions grow with size within each table (a larger size never has a smaller outside diameter, centre-to-end or face-to-face), and with rating (a higher class never has a thinner flange).
4. Pipe: `wall × 2 < od`. Flanges: `wn_length > thick`, `raised_face < od`. Long-radius elbow = 1.5 × size for ¾" and larger (½" = 1.5").
5. No duplicate keys in a file (same size + schedule, size + rating, kind + rating + size).
6. Header rows exactly as given; no extra spaces in headers; no empty lines.
7. Nothing names a commercial design program.

At the end, add a short **report**: how many rows per file, how many are `UNVERIFIED`, which sizes or classes you left out and why, and any doubts.

---

## What happens next

The files come back to Datum Layout's session. There, a checker reads them (`catalogue.rs`: each row is validated and every rejected row is listed), compares overlapping rows with the built-in tables, and checks the rules in Part C. Part A files work at once as a plug-in pack (Catalogue panel › Add a pack…); Part B files need new shape rules, which are added and tested part by part (each generated part is tested against its table row). Rows marked `UNVERIFIED` stay marked in the app until checked against a catalogue.
