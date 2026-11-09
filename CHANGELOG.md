# What's new in Datum Layout

## 0.2.3 — 2026-11-09
Moving equipment made easy, the view cube, and status badges.

**Drag it where it goes.** Select equipment (or a group), then drag it across the plot: it follows the mouse as you go, snapping to a tenth of the grid (hold Shift for finer), with the distance shown. Let go and the lines connected follow; any line left skewed is routed square again through its own corners (Ctrl+Z keeps it as it was). Esc while dragging puts it back.

**Nudge and type.** The arrow keys move the selection one grid step the way they point on the screen (Shift: a tenth). Right-click equipment for Move to… (type its position), Turn 90° and Copy beside it.

**Fixed.** Dragging an arrow could lose the mouse part-way, and the arrows took clicks a metre away from them; an arrow pointing straight at you (the vertical one in plan) is now hidden so the item under it can be dragged.

**View cube.** After Datum Mesh's: click a face (Top, South, East…) to look from it, an edge to look from between two faces, or a corner for an isometric view; the house button fits the plant.

**Status badges.** Check (not run, out of date, findings or clean), Routing (lines to fix or square) and P&ID (differences with the line list or plant package), each a click away from what it is about.

## 0.2.2 — 2026-11-08
Pipe supports, handed to Datum Pipe with each line.

**Supports.** Rests (a shoe under the pipe), guides, line stops and anchors. Piping › Supports puts one on the middle of the selected run, or drag Pipe support from the catalogue onto a pipe; choose what it holds in its properties. On a vertical run it is drawn as a clamp. The tree lists them, and the take-off counts them.

**To Datum Pipe.** Each support is written into the piping component file at its point, with what it holds, and the pipe is split there so the stress model has a node. Datum Pipe's own engine reads the pipe rack example's lines with their supports, with no warnings, and solves them. Piping component files read back into Layout keep their supports.

**Example 3.** The pipe rack now has rests, guides and an anchor on both lines.

## 0.2.1 — 2026-11-08
From the P&ID: a plant package brings the P&ID's equipment and lines into the layout, and the 3D model is checked against it.

**Plant package.** Equipment › Plant Package (or Lists › Import) reads the open plant package format (docs/formats/plant-package.md) that Datum Schema will export: equipment with its class, datasheet values and nozzles; lines with their number, size, class, rating, fluid, insulation, both ends and their components in flow order; every object with the P&ID's stable id. New equipment is placed in a row to be arranged; equipment already in the plant is updated. Importing again updates, never duplicates.

**Route Listed Lines.** Each listed line from nozzle to nozzle is routed and its valves and strainers are placed along it in the P&ID's order. Branches and open ends are left to route by hand, and the checks say which.

**3D vs P&ID.** The checks now also compare each line's components and their order with the P&ID, as well as lines missing or extra, size, class, rating, fluid and insulation.

**For Datum Schema.** The format document, a worked example (docs/formats/pump-transfer.package.json) and a brief for Schema's export (docs/briefs/SCHEMA-PLANT-PACKAGE.md).

## 0.2.0 — 2026-11-08
The Library: every pipe class and parts table, browsable, with its source.

**Pipe classes.** Each class's material, flange ratings, sizes and elbows; its wall schedule size by size with the pipe's outside diameter and wall; the components it allows and in which sizes; how many lines use it. "Use for new lines" sets it for the next lines you route.

**Parts tables.** Pipe (267 sizes and schedules), butt-weld fittings, flanges (½" to 24", class 150 to 2500, weld-neck and blind) and valves (466 rows), row by row, each with its source.

**Search.** Find a class, a size (6, 1-1/2), a valve type or a flange class (2500). Open the Library from the steps bar, Help › Library, Piping › Library, or by clicking a pipe class in the catalogue.

## 0.1.10 — 2026-11-08
The valve list, butt welds in the take-off, and design rules in the checks.

**Valve list.** Lists › Valve List (and its CSV): every valve and strainer with its tag, type, size, class, line, position and the way its stem points. Click a row to select the valve.

**Butt welds.** The material take-off counts the butt welds of each line by size: every joint where pipe, fittings and weld-neck flanges meet.

**Design rules.** The checks now also find a valve stem pointing down, a swing check valve in a run flowing down (the flow runs from a line's start to its end, or away from a pump's discharge), and pipe pieces between fittings too short to weld well (6 to 50 mm).

## 0.1.9 — 2026-11-08
A new workspace, laid out like Datum Mesh's: the work in order, everything in the plant at a glance.

**Steps bar.** At the far left: Plot, Equipment, Lines, Fittings, Check and Hand over, each marked when done. A click opens that step's ribbon tab.

**Plant tree.** Everything in the plant in that order: the plot and areas; equipment with its nozzles (and the line on each); lines and the line list; valves and fittings grouped by kind; the check's findings; saved views and the hand-over files. Click a row to select it (Ctrl+click adds to a group), double-click to zoom to it, right-click to hide, isolate, export or delete it. Each row's eye shows or hides it in the view. A filter finds any tag, line number or valve. The catalogue is the tree's second tab.

**Context bar.** Under the ribbon, whatever the tab: size, class and rating for new lines, the working elevation, colour by, labels, section box, show all, the standard views and the units.

**Tool strip and properties card.** Beside the tree, the open tab's most used tools. The properties of what is selected open on a card over the view, as a compact grid; Esc or × closes it, and it steps aside while routing or placing. The tree and the card can be widened by dragging their edge.

## 0.1.8 — 2026-11-08
Saved views, a line list to check against, and lines read from piping component files.

**Saved views.** View › Save View keeps the camera, the colours, hidden or isolated items and the section box under a name, saved with the plant. View › Views brings one back, renames or deletes it.

**Line list.** Lists › Import › Line List reads a CSV line list (columns found by name: line number, size, class, rating, fluid, insulation, from, to, notes; Datum Layout's own export reads back). The list is kept with the plant, and the checks show lines missing from the model, lines not on the list, and any difference in size, class, rating, fluid or insulation. In the Inspector, "From the line list" gives a line the listed values.

**PCF lines.** Lists › Import › PCF Lines adds the lines of a piping component file: corners, valves, strainers, tees and reducers, with flanges, gaskets and bolts generated from the class. Ends on a free nozzle connect to it, and a run from another line's tee becomes its branch. Every example written to PCF reads back into the same lines.

## 0.1.7 — 2026-11-08
Shape routes without routing them again, and choose what the view shows.

**Move a corner.** Select a line: its corners show as amber dots. Click one and drag its arrows; the runs on both sides stay square and the elbows follow. A corner next to a nozzle or tee slides along the run that leaves it.

**Jogs.** Select a run and choose Piping › Add Jog (J), or right-click a pipe › Add a jog here: the run steps up a metre (sideways on a vertical run) with four elbows, to pass over or round something. Drag the new middle run to place it. Remove Jog straightens it again, and components come back where they were.

**Colour by.** View › Colour by: part kind, or every line's parts by line number, pipe class, fluid or size, with a key in the corner of the view.

**Hide and isolate.** Hide (H) the selection, Isolate (I) to show only the selection, Show All (Shift+H). Hidden items still count in the checks, lists and exports.

**Section box.** View › Section Box draws only what is inside a box round the selection (or the whole plant); set each face in its panel.

## 0.1.6 — 2026-11-08
Work with many items at once: select several, then move, copy, turn, mirror or delete them together.

**Select several.** Ctrl+click equipment and lines in the view to add or remove them; Home › Select All (Ctrl+A) picks everything. The Inspector lists the selection and the status bar counts it.

**Move together.** Drag the selection's arrows: every picked item moves by the same amount, and lines connected to items left behind follow their nozzles.

**Copy, turn and mirror.** Home › Arrange › Copy (Ctrl+D) places a copy beside the selection: lines between copied equipment connect to the copies and get new numbers, the equipment new tags. Turn ±90° turns the selection about its centre; Mirror east–west and north–south make the opposite hand, nozzles and all.

**Delete together.** Delete removes every picked item in one step; Undo brings them back.

## 0.1.5 — 2026-11-08
The catalogue grows: flanges to class 2500, valves in every class, eight example pipe classes, and a measure tool.

**Flanges in six classes.** Weld-neck and blind flanges now come in class 150, 300, 600, 900, 1500 and 2500, ½" to 24" wherever they are made, each row from a named manufacturer's catalogue. A blind flange can have its own thickness where it differs from the weld-neck's.

**Valves in every class.** Gate, globe, swing-check and ball valves in all six classes, from manufacturers' catalogues. Swing-check valves now have their own lengths (they used globe lengths before). Where a size is not listed, its length is worked out between the neighbouring sizes.

**Fittings checked.** Elbows, tees, reducers and caps from a fittings catalogue, with short-radius elbows from 1".

**Eight example pipe classes.** Carbon steel 150, 300, 600 and 900/1500, low-temperature carbon steel, stainless steel 316 and utility water, as well as CS1. Pick one for new lines on the Piping tab, or for a line in the Inspector.

**Measure.** View › Measure (or M): click two points on parts or the plot to see the distance, its east, north and elevation parts, and the horizontal distance.

## 0.1.4 — 2026-11-03
Datum Layout 0.1 is complete: the examples, printing, help and pictures, checked end to end.

**Examples.** File › Examples opens the pump suction and discharge, the exchanger hook-up and the small pipe rack run with no clashes and no errors, and "Find the clashes" with exactly its three planted problems. Every line of every example is read and solved by Datum Pipe.

**Print.** File › Print (Ctrl+P) puts the 3D view as it is on screen on its own page with a title block, then the line list, the equipment list with nozzles and the material take-off, on any paper, with a header and footer; Save as PDF in the print window makes a PDF.

**Help.** Every ribbon group and panel has its ? card; Help › Getting Started walks through placing equipment, routing lines and handing over; the glossary explains the plant-design words; Help › Sources lists where every catalogue table comes from.

**Pictures.** New screenshots of the pump example, a valve being dropped onto the header, the clash list and the line list.

## 0.1.3 — 2026-11-03
The hand-over to Datum Pipe, proved.

**Datum Pipe reads every line.** Every line of every example is exported as a piping component file and read by Datum Pipe's own engine, which then solves it: the same nodes, bends, tees, valves, flanges and reducers. The record is in docs/verification/0.1.3.md, and the check now runs on every change.

**glTF checked.** Every example's 3D model passes the Khronos glTF validator with no errors and no warnings.

**Checks.** Every check has a failing and a passing case in the tests. A component a class does not allow is now named properly ("a 16\" ball valve is not allowed by class CS1").

**Lists.** The material take-off is tested against what example 1 holds: valves, tees, reducers, flanges, one gasket and one bolt set per flanged joint, and the pipe length of each line.

## 0.1.2 — 2026-11-03
Dropping and moving fittings, and branches, made exact.

**See where it snaps.** While you drag a valve or fitting over a pipe, the marker shows exactly where it will land (the nearest free straight length between elbows, flanges and other components), or turns red and says why it cannot go there.

**Slide a component along its run.** Select a valve, strainer, tee or reducer and drag its amber arrow; it moves to the nearest free place, or onto another run of the line.

**Branches.** Dropping a tee starts routing its branch at once; double-click (or right-click) ends the branch at a free point, or click a nozzle. Fixed: a double-click did not end a route.

**Resizing.** A line's reducers keep the size after them when the line is resized, and a branch larger than its run is reported as a mismatch at the tee.

## 0.1.1 — 2026-11-03
Lines now follow the equipment they connect to.

**Move equipment, keep the lines.** When you move a vessel, pump or exchanger, each line on its nozzles follows: the move passes along the line's runs, each run stretching along its own direction, so the route stays square and its corners stay where they can. A run too short for its fittings moves whole instead. Branches follow their tees, and components slide along their run when it gets shorter.

**Route Again.** Turning equipment, or a move no run can take, leaves a line skewed: it is reported ("route it again"), and Piping › Route Again (or the button in the Inspector) routes it square between its ends through its corners, keeping its components on their runs.

**A new check.** A line that no longer leaves its nozzle or tee square to it is reported, even when it has no turn.

## 0.1.0 — 2026-11-03
The first release: lay out a plant in 3D, route its pipes, check them and hand each line to Datum Pipe.

**The plot.** A site grid with east and north coordinates, the plant north arrow, the ground and areas. Grid spacing, extent, north and ground are set in the Inspector.

**Equipment from its datasheet.** Six types: vertical vessel, horizontal drum, storage tank, column, centrifugal pump with motor and baseplate, and shell-and-tube exchanger. Drag one from the catalogue onto the plot, then set its tag, position, rotation, sizes, nozzles (size, rating, facing, position, direction) and datasheet lines. Drag its arrows to move it.

**Routing.** Piping › Route Line: click a nozzle, the corners and the other nozzle. Long-radius elbows appear at every turn (short radius by choice), a reducer where the nozzle and line sizes differ, and a weld-neck flange, gasket and bolt set at each flanged nozzle. Turns that are not 90° or 45°, and turns too tight for their elbow, are reported and not drawn.

**Drag and drop fittings.** Gate, globe, check and ball valves, strainers, flange pairs, tees and reducers snap onto the pipe, take the line's size and class, and split the pipe. A tee starts a branch. Change a line's size and every component re-sizes; move a straight run and the elbows follow; undo everything.

**Checks.** Clashes between pipes, pipes and equipment, and pipes and the ground, with a 25 mm clearance; open ends; size, rating and end mismatches; components the line's class does not allow; lines without a number. Click a finding to go to it.

**Lists and exports.** The line list, the equipment list with nozzles and the material take-off per line, each to CSV. A piping component file (.pcf) per line or for the whole plant that Datum Pipe reads; glTF 2.0 for any 3D viewer; print or save as PDF.

**A catalogue of generated parts.** Pipe ½" to 24", 90° and 45° elbows, equal and reducing tees, concentric reducers, caps, weld-neck and blind flanges in class 150 and 300, gaskets and bolt sets, and four valve types, each from a table whose rows name their source. One pipe class (carbon steel). Plug-in packs add your company's classes and parts.

**Examples.** A pump suction and discharge, an exchanger hook-up with a bypass, a small pipe rack run, and "Find the clashes" with three planted problems.
