# Plan: the rest of 0.2 (researched in-session, 2026-11-09)

The user asked to stop waiting for an outside helper: research the parts tables here (web search of
manufacturers' public catalogues), and keep shipping small releases. The brief
`docs/briefs/CATALOGUE-PACK-2.md` stays as the list of what is wanted; any file the outside helper
sends later is a cross-check.

Rules for researched data (as for every table): every row names its source (company, catalogue,
page or table); values found only on secondary sites are marked `UNVERIFIED:`; no standard's text
or tables are reproduced as a document; each table is read by the app's loader and checked
(growing with size and class, no duplicates) before it is used.

| Release | What | Data researched | Engine and window | Proof |
| --- | --- | --- | --- | --- |
| 0.2.6 | Reducing tees and reducers by size pair | Run C and branch M for every run × branch; reducer H for every large × small pair (butt-weld fittings catalogues) | Tee branch length from M; reducer length by its two sizes | Parts against their rows; PCF read by Datum Pipe |
| 0.2.7 | Weldolets and sockolets, chosen by the class's branch table | Outlet height and base diameter by branch and header range (forged outlet catalogues) | A branch outlet that does not cut the header; `branches` in the class JSON (tee, reducing tee, weldolet, sockolet); a dropped tee becomes what the class says | PCF OLET (Pipe reads it); class check |
| 0.2.8 | Weights for the stress analysis | Pipe weight per foot by schedule; flange and valve weights by size and class | Every part's weight; WEIGHT in the PCF; weights in the take-off | Pipe reads the weights; totals tested |
| 0.2.9 | More valves and blinds | Butterfly (wafer, lug), dual-plate wafer check and plug valve face-to-face; spectacle blinds, spades and spacers | New valve bodies and blinds; drag from the catalogue | Parts against rows; Pipe reads them |
| 0.2.10 | The pipe class editor | Two more example classes (stainless 300, steam 300) | Edit a class (sizes, schedules, components, branch table) in the Library; saved with the plant or as a pack | UI checks; class rules tested |
| (with 0.2.7) | Support shoes at real size | Shoe height and length by size | Shoes drawn from the table | Parts against rows |

Each release: tests, UI checks, screenshots looked at, MEMORY and CHANGELOG, site.json, released
through the updater. Stop and report after 0.2.10 (or sooner if the research does not hold up).
