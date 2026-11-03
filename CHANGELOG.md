# What's new in Datum Layout

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
