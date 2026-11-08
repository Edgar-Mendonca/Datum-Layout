// Dialogs: saving changes, closing, recovery, and the help pages (shortcuts, getting started,
// glossary, catalogue sources).
import { For, createSignal, createResource } from "solid-js";
import { Out, CatalogueOut } from "./api";
import { openDialog, closeDialog, call, out, setOut } from "./store";
import { C } from "./cmds";
import { Dialog } from "./ui";

export const keepChanges = (then: () => unknown) => {
  if (!out()?.dirty) { then(); return; }
  openDialog(() => (
    <Dialog title="Save changes?" okText="Save" cancel="Cancel" ok={async () => { await C.save(); if (out()?.dirty) return false; setTimeout(then); }}
      extra={<button class="btn" onClick={() => { closeDialog(); setTimeout(then); }}>Don't save</button>}>
      <p>“{out()?.p.name}” has changes that are not saved.</p>
    </Dialog>
  ));
};

export const confirmClose = (quit: () => void) => openDialog(() => (
  <Dialog title="Save changes?" okText="Save" cancel="Cancel" ok={async () => { await C.save(); if (!out()?.dirty) quit(); }}
    extra={<button class="btn" onClick={() => { closeDialog(); quit(); }}>Don't save</button>}>
    <p>“{out()?.p.name}” has changes that are not saved.</p>
  </Dialog>
));

export const shortcuts = () => openDialog(() => (
  <Dialog title="Keyboard shortcuts">
    <table class="tbl keys"><tbody><For each={[
      ["Save / Save As", "Ctrl+S / Ctrl+Shift+S"], ["Open", "Ctrl+O"], ["New", "Ctrl+N"], ["Print", "Ctrl+P"], ["Tell me what you want to do", "Ctrl+K"],
      ["Undo / Redo", "Ctrl+Z / Ctrl+Y"], ["Delete what is selected", "Del"], ["Route a line", "R"], ["Working elevation up / down one grid step", "PgUp / PgDn"],
      ["Cancel the tool", "Esc"], ["Check clashes and errors", "F5"], ["Fit the plant in the view", "F"], ["Standard views: iso, plan, front, side", "1 2 3 4"],
      ["Orbit / pan / zoom", "Left drag / right or middle drag / wheel"], ["Select one run of the selected line", "Click its pipe (or Shift+click)"], ["This list", "F1"],
    ] as [string, string][]}>{([a, b]) => <tr><td>{a}</td><td><kbd>{b}</kbd></td></tr>}</For></tbody></table>
  </Dialog>
));

export const gettingStarted = () => openDialog(() => (
  <Dialog title="Getting started" wide>
    <div class="helpcard">
      <p>Datum Layout lays out one plant in 3D: you place the equipment from its datasheet sizes, route the pipes between nozzles, and the elbows, tees, flanges and valves appear and size themselves from the line's size and pipe class. Then you check for clashes, list every line and hand each one to Datum Pipe for its isometric and stress analysis.</p>
      <h4>1. Place the equipment</h4>
      <ol>
        <li>Drag a vessel, drum, tank, column, pump or exchanger from the <b>catalogue</b> on the left onto the plot. The plot's grid shows east (E) and north (N) in metres; the arrow is plant north.</li>
        <li>Select it: the <b>Inspector</b> on the right holds its tag (P-101A), position, rotation, sizes, nozzles and datasheet lines. Drag its arrows in the 3D view to move it.</li>
      </ol>
      <h4>2. Route the lines</h4>
      <ol>
        <li><b>Piping › Route Line</b> (R). Click a nozzle (an amber dot), then click the corners: each one is square to the last. Click another nozzle to end, or double-click to end at a free point.</li>
        <li>The engine places long-radius elbows at every turn, a reducer where the nozzle and line sizes differ, and a weld-neck flange, gasket and bolt set at each flanged nozzle.</li>
        <li>Drag valves, strainers, flange pairs, tees and reducers from the catalogue onto a pipe: they snap, take the line's size and class, and split the pipe. A tee starts a branch.</li>
        <li>Select a line to change its size (every component re-sizes), number, class, rating, fluid and insulation. Click one of its runs and drag the arrows to move it; the elbows follow.</li>
      </ol>
      <h4>3. Check and hand over</h4>
      <ol>
        <li><b>Check</b> (F5) lists clashes (pipe–pipe, pipe–equipment, pipe–ground, with a 25 mm clearance), open ends, mismatches, class problems and lines without a number. Click one to go to it.</li>
        <li>The bottom panel has the <b>line list</b>, the <b>equipment list</b> with nozzles and the <b>material take-off</b>; each exports to CSV.</li>
        <li><b>To Datum Pipe</b> writes a line as a piping component file (.pcf) that Datum Pipe imports for the isometric and stress analysis. The whole model also exports as glTF for any 3D viewer.</li>
      </ol>
      <h4>Good to know</h4>
      <ul>
        <li>Every part is generated from a dimension table and shape rule: check dimensions against your supplier's data before building. Help › Sources lists every table's source.</li>
        <li>Every tool has a small ? beside it. File › Examples has three worked plants and one with planted problems to find.</li>
      </ul>
    </div>
  </Dialog>
));

export const glossary = () => openDialog(() => {
  const [q, setQ] = createSignal("");
  const G: [string, string][] = [
    ["Plot", "The site the plant stands on, with a coordinate grid: east (E), north (N) and elevation (EL), in metres."],
    ["Plant north", "The direction the plant's drawings call north; it may differ from grid north by an angle."],
    ["Area", "A box on the plot that groups equipment and lines (plant › unit › area)."],
    ["Nozzle", "A flanged connection on equipment where a line starts or ends."],
    ["Line", "One pipeline with its own line number, size, class and fluid, from one end to the other."],
    ["Line number", "The line's name, usually size, fluid code, a serial number and the class: 6\"-P-1001-CS1."],
    ["Pipe class (piping specification)", "The rules for a line: material, wall schedule by size, flange ratings and the components allowed."],
    ["NPS", "Nominal pipe size in inches (½\" to 24\"); the outside diameter is fixed for each size."],
    ["Schedule", "The pipe's wall thickness series (40, 80, STD…)."],
    ["Run", "One straight leg of a line, between two corners."],
    ["Long radius elbow (LR)", "An elbow whose centreline radius is 1.5 × the pipe size; short radius (SR) is 1 ×."],
    ["Centre-to-end", "From an elbow's or tee's centre point to its end face: how far it takes up each run."],
    ["Weld-neck flange", "A flange with a tapered hub that is butt-welded to the pipe."],
    ["Rating (class 150 to 2500)", "The flange pressure class; flanges that meet must have the same rating."],
    ["Face-to-face", "The length of a flanged valve between its two flange faces."],
    ["Reducer", "A cone fitting that joins two sizes; concentric keeps the centreline."],
    ["Tee", "A fitting with a branch at 90°; a reducing tee has a smaller branch."],
    ["Bolt set", "The studs and nuts of one flanged joint, listed by number, diameter and length."],
    ["Clash", "Two parts, or a part and the ground, closer than the clearance."],
    ["Open end", "A free line end without a cap or a blind flange."],
    ["Material take-off (MTO)", "The list of pipe, fittings, flanges, valves, gaskets and bolts a line needs."],
    ["PCF", "A plain-text piping component file: each component with its end points and size; Datum Pipe reads it."],
    ["glTF", "An open 3D file format that any 3D viewer opens."],
    ["Port", "A connection point of a part: where it is, which way it faces, its size, rating and end type."],
  ];
  return <Dialog title="Glossary"><input placeholder="Search" value={q()} onInput={e => setQ(e.currentTarget.value)} style={{ width: "100%", "margin-bottom": "8px" }} />
    <dl class="gloss"><For each={G.filter(g => (g[0] + g[1]).toLowerCase().includes(q().toLowerCase()))}>{([t, d]) => <><dt>{t}</dt><dd>{d}</dd></>}</For></dl></Dialog>;
});

/** Help › Sources: where each catalogue table comes from. */
export const sources = () => openDialog(() => {
  const [c] = createResource(() => call<CatalogueOut>("catalogue"));
  const rows = () => {
    const k = c()?.cat; if (!k) return [];
    const uniq = (v: string[]) => [...new Set(v)];
    return [["Pipe", uniq(k.pipes.map(r => r.source))], ["Fittings", uniq(k.fittings.map((r: any) => r.source))], ["Flanges", uniq(k.flanges.map((r: any) => r.source))], ["Valves", uniq(k.valves.map((r: any) => r.source))], ["Pipe classes", uniq(k.classes.map(r => `${r.name}: ${r.source}`))]] as [string, string[]][];
  };
  return <Dialog title="Catalogue sources" wide>
    <p>Every part is generated from these tables: no 3D model is downloaded or bundled. No standard's tables or text are reproduced. <b>Check dimensions against your supplier's data before ordering or building.</b></p>
    <table class="tbl"><tbody><For each={rows()}>{([t, s]) => <tr><td><b>{t}</b></td><td><For each={s}>{x => <div>{x}</div>}</For></td></tr>}</For></tbody></table>
  </Dialog>;
});

/** Recover unsaved work after a crash. */
export const recoverDialog = (name: string, saved: string, done: () => void) => openDialog(() => (
  <Dialog title="Recover unsaved work?" okText="Recover" cancel="Not now" ok={async () => { const r = await call<Out>("recover", { keep: true }); if (r) { setOut(r); done(); } }}
    extra={<button class="btn" onClick={() => { call("recover", { keep: false }); closeDialog(); }}>Discard</button>}>
    <p>Datum Layout closed before “{name}” was saved. A copy from {saved} was kept.</p>
  </Dialog>
));
