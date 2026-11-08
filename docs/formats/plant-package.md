# The plant package (version 1)

What a P&ID hands to the 3D layout: the plant's equipment and lines, with the P&ID's stable ids, so
the 3D model can be built from it and checked against it. Datum Schema writes it; Datum Layout reads
it (Equipment › Plant Package, or Lists › Import). The format is open: anyone may read or write it.

A plant package is one UTF-8 JSON file (`.json`). A worked example: `pump-transfer.package.json`
in this folder.

## The file

| Key | Type | Meaning |
| --- | --- | --- |
| `format` | text | Always `"datum-plant-package"`. |
| `version` | number | `1`. A reader refuses a newer version. |
| `app` | text | The program and version that wrote it (for messages only). |
| `project` | text | The project's name. |
| `units` | text | `"SI"`. Sizes are nominal pipe sizes; insulation is in millimetres. |
| `equipment` | list | The equipment items (below). |
| `lines` | list | The line numbers (below). |

Unknown keys are ignored, so a writer may add its own.

## Equipment

| Key | Type | Meaning |
| --- | --- | --- |
| `uid` | text | The P&ID's stable id. Kept on the 3D item; a later package with the same `uid` or `tag` updates it. |
| `tag` | text | The tag, e.g. `P-201A`. Required. |
| `class` | text | The DEXPI class name: `CentrifugalPump`, `Vessel`, `Tank`, `ProcessColumn`, `TubularHeatExchanger`… (also understood: names containing pump, column/tower, tank, exchanger/cooler/heater/condenser, horizontal/drum, vessel/separator/receiver). |
| `service` | text | What it does, e.g. "Crude transfer". |
| `data` | object | Datasheet values, name → value as text with its unit (`"Flow": "60 m³/h"`). |
| `nozzles` | list | `{ "tag", "nps", "rating", "service" }`: tag as on the P&ID (`N1`, `S`, `D`), nominal size (`"6"`, `"1-1/2"`, `"DN150"`), flange class (150, 300, 600, 900, 1500, 2500). |

## Lines

| Key | Type | Meaning |
| --- | --- | --- |
| `uid` | text | The P&ID's stable id for the line number. |
| `number` | text | The line number, e.g. `4"-P-2002-CS300`. Required. |
| `nps` | text | Nominal size. |
| `class` | text | Pipe class (piping specification), e.g. `CS300`. |
| `rating` | number | Flange class. |
| `fluid` | text | Fluid code. |
| `insulation_mm` | number | Insulation thickness, mm (0 for none). |
| `from`, `to` | object | Where the line starts and ends, in flow direction: `{ "equipment": "P-201", "nozzle": "D" }`, or `{ "line": "<number>" }` for a branch off another line, or `{}` for an open end. |
| `components` | list | The in-line components in flow order: `{ "uid", "kind", "tag" }` with `kind` one of `gate`, `globe`, `check`, `ball`, `strainer`, `reducer`, `tee`, `flange_pair`. |
| `notes` | text | Free text. |

## What Datum Layout does with it

1. Equipment not in the plant is placed in a row along the plot, to be arranged. Equipment already there (same tag or uid) keeps its place and gets the package's nozzle sizes, ratings and services and its datasheet values. Nozzles the 3D type does not have are added on top of the item, to be moved to their place.
2. Every line goes on the line list with its ends and components.
3. **Route Listed Lines** routes each listed line that runs nozzle to nozzle, and places its components along it in the package's order. Branches and open ends are routed by hand.
4. The checks compare the model with the list: lines missing or extra, size, class, rating, fluid, insulation, and the components' order.
