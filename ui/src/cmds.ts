// Commands: what the ribbon, menus, keys and Tell Me run.
import { Out, V3, ItemKind, EquipKind, Issue, Lists, inch, NPS } from "./api";
import { picked, setPicked, out, setOut, act, call, sel, setSel, setBackstage, loadOut, toast, project, setMode, setRoute, route, setPlacing, setChecked, setCheckStale, setBottom, selLine, selEquip, setFocus, mode, elev, setElev } from "./store";
import * as D from "./dialogs";
import { RouteFrom } from "./store";

let fitHook: () => void = () => {};
export const setFit = (f: () => void) => { fitHook = f; };
let viewHook: (v: string) => void = () => {};
export const setStdView = (f: (v: string) => void) => { viewHook = f; };

/** Ends the route being drawn at a nozzle, a tee's branch or a free point, and adds the line. */
export const finishRoute = async (to: RouteFrom, keepLast = false) => {
  const r = route(); if (!r?.from) return;
  const via = keepLast ? r.via.slice(0, -1) : r.via;
  const end = to.kind === "point" ? { kind: "point", p: to.p } : to;
  const o = await act({ Route: { from: r.from, via, to: end, nps: r.nps } });
  if (o?.created.length) { setSel({ kind: "line", id: o.created[0] }); toast(`Line ${o.p.lines.find(l => l.id === o.created[0])?.number} routed: elbows, flanges and reducers are placed for you.`, true); }
  setRoute(null); setMode("select");
};

const download = async (name: string, data: string, what: string) => {
  const path = await call<string | null>("save_file", { name, data });
  if (path) toast(`Saved ${what}.`, true);
};
const safe = (s: string) => (s || "plant").replace(/[\\/:*?"<>|]+/g, "-");

export const C = {
  // ---------- file ----------
  save: async (asNew = false) => { const o = await call<Out | null>("save", { asNew }); if (o) { setOut(o); toast("Saved.", true); } },
  open: () => D.keepChanges(() => loadOut("open", { path: null }).then(o => { if (o) setTimeout(fitHook, 80); })),
  openPath: (path: string) => D.keepChanges(() => loadOut("open", { path }).then(o => { if (o) setTimeout(fitHook, 80); })),
  newProject: (example: string | null) => D.keepChanges(() => loadOut("new_project", { example }).then(o => { if (o) setTimeout(fitHook, 120); })),
  print: () => setBackstage("print"),
  undo: async () => { const o = await call<Out>("history", { redo: false }); if (o) setOut(o); },
  redo: async () => { const o = await call<Out>("history", { redo: true }); if (o) setOut(o); },

  // ---------- groups ----------
  /** Adds an item (equipment or line id) to the group, or takes it out; the single selection joins first. */
  pick: (id: number) => {
    let g = picked();
    const s = sel();
    if (!g.length && s && (s.kind === "equip" || s.kind === "line" || s.kind === "nozzle")) g = [s.id];
    setPicked(g.includes(id) ? g.filter(x => x !== id) : [...g, id]);
    setSel(null);
  },
  selectAll: () => { const p = project(); if (p) { setSel(null); setPicked([...p.equipment.map(e => e.id), ...p.lines.map(l => l.id)]); } },
  /** The group, or the single selected equipment or line. */
  targets: (): number[] => { const g = picked(); if (g.length) return g; const s = sel(); return s && (s.kind === "equip" || s.kind === "line" || s.kind === "nozzle") ? [s.id] : []; },
  /** The middle of the targets on the plan (for turning and mirroring). */
  centre: (ids: number[]): [number, number, number] => {
    const p = project(); if (!p) return [0, 0, 0];
    const pts: number[][] = [...p.equipment.filter(e => ids.includes(e.id)).map(e => e.pos), ...p.lines.filter(l => ids.includes(l.id)).flatMap(l => out()?.paths.find(x => x[0] === l.id)?.[1] ?? l.pts)];
    if (!pts.length) return [0, 0, 0];
    const lo = [0, 1].map(k => Math.min(...pts.map(q => q[k]))), hi = [0, 1].map(k => Math.max(...pts.map(q => q[k])));
    const g = project()?.plot.grid ?? 1;
    return [Math.round((lo[0] + hi[0]) / 2 / g) * g, Math.round((lo[1] + hi[1]) / 2 / g) * g, 0];
  },
  groupOp: (name: string, make: (ids: number[]) => object) => {
    const ids = C.targets(); if (!ids.length) return toast("Select equipment or lines (Ctrl+click adds more), then " + name + ".");
    act(make(ids)).then(o => { if (o && o.created.length) { setSel(null); setPicked(o.created); } });
  },
  copyGroup: () => C.groupOp("copy", ids => {
    const p = project()!, pts: number[][] = [...p.equipment.filter(e => ids.includes(e.id)).map(e => e.pos), ...p.lines.filter(l => ids.includes(l.id)).flatMap(l => l.pts)];
    const w = pts.length ? Math.max(...pts.map(q => q[0])) - Math.min(...pts.map(q => q[0])) : 0;
    const g = p.plot.grid || 1;
    return { CopyGroup: { ids, delta: [Math.ceil((w + 3) / g) * g, 0, 0] } };
  }),
  rotateGroup: (deg: number) => C.groupOp("turn them", ids => ({ RotateGroup: { ids, about: C.centre(ids), deg } })),
  mirrorGroup: (eastWest: boolean) => C.groupOp("mirror them", ids => ({ MirrorGroup: { ids, about: C.centre(ids), east_west: eastWest } })),
  deleteGroup: (): void => { const ids = C.targets(); if (!ids.length) { C.del(); return; } act({ Delete: { ids } }).then(o => { if (o) { setPicked([]); setSel(null); } }); },

  // ---------- equipment ----------
  /** Click on the plot to place an item of this kind. */
  place: (kind: EquipKind) => { setRoute(null); setPlacing(kind); setMode("place"); },
  del: (): void => {
    if (picked().length) { C.deleteGroup(); return; }
    const s = sel(); if (!s) return toast("Select equipment, a line or a component first.");
    if (s.kind === "nozzle") { const e = selEquip(); if (e) { act({ SetEquipment: { ...e, nozzles: e.nozzles.filter(n => n.tag !== s.tag) } }).then(o => o && setSel({ kind: "equip", id: e.id })); return; } }
    if (s.kind === "run") return toast("A run is part of its line: select the line to delete it, or move the run.");
    if (s.kind === "plot") return;
    act({ Delete: { ids: [s.id] } }).then(o => o && setSel(null));
  },
  rotate: (deg: number) => { const e = selEquip(); if (!e) return toast("Select an equipment item first."); act({ SetEquipment: { ...e, rot: ((e.rot + deg) % 360 + 360) % 360 } }); },

  // ---------- piping ----------
  route: () => { setPlacing(null); setRoute({ from: null, start: null, via: [], nps: project()?.settings.nps ?? "6" }); setMode("route"); },
  cancelTool: () => { setRoute(null); setPlacing(null); setMode("select"); },
  measure: () => { setRoute(null); setPlacing(null); setMode(mode() === "measure" ? "select" : "measure"); },
  /** Drops a component onto a line near a point: the engine snaps it to the nearest free place. */
  drop: async (line: number, kind: ItemKind, at: V3) => {
    const o = await act({ Drop: { line, kind, at, dir: [0, 0, 0], nps2: "" } });
    if (!o?.created.length) return;
    const id = o.created[0];
    setSel({ kind: "item", id, line });
    if (kind === "tee") {
      // A tee starts a branch: route from its branch end.
      await C.routeBranch(line, id);
      toast("Tee placed. Route the branch: click its corners, then a nozzle or a free point (Esc leaves the branch for later).", true);
    } else toast(`${kind === "flange_pair" ? "Flange pair" : kind[0].toUpperCase() + kind.slice(1)} placed: it took the line's size and class and split the pipe.`, true);
  },
  /** Starts routing a branch line from a tee's branch end. */
  routeBranch: async (line: number, item: number) => {
    const s = await call<any>("scene");
    const pt = s?.ports.find((x: any) => x.kind === "branch" && x.item === item);
    if (!pt) return toast("That tee's branch already has a line.");
    setPlacing(null);
    setRoute({ from: { kind: "branch", line, item }, start: pt.p, via: [], nps: pt.nps });
    setMode("route");
  },
  /** Drops a component on the selected line or run, near its middle. */
  dropOnSelected: (kind: ItemKind) => {
    const l = selLine(); if (!l) return toast("Select a line (or a run of it) first, or drag the component onto a pipe.");
    const s = sel(); const pts = out()?.paths.find(x => x[0] === l.id)?.[1] ?? l.pts;
    let k = s?.kind === "run" ? s.seg : 0, best = -1;
    if (s?.kind !== "run") for (let i = 0; i + 1 < pts.length; i++) { const d = Math.hypot(pts[i + 1][0] - pts[i][0], pts[i + 1][1] - pts[i][1], pts[i + 1][2] - pts[i][2]); if (d > best) { best = d; k = i; } }
    const a = pts[k], b = pts[k + 1];
    C.drop(l.id, kind, [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2, (a[2] + b[2]) / 2]);
  },
  resize: (nps: string) => { const l = selLine(); if (!l) return toast("Select a line first."); act({ Resize: { line: l.id, nps } }).then(o => o && toast(`${l.number || "The line"} is now ${inch(nps)}: every component re-sized.`, true)); },
  step: (up: boolean) => { const l = selLine(); if (!l) return toast("Select a line first."); const i = NPS.indexOf(l.nps) + (up ? 1 : -1); if (i >= 0 && i < NPS.length) C.resize(NPS[i]); },
  radius: (r: "long" | "short") => { const l = selLine(); if (l) act({ SetLine: { ...l, radius: r } }); },
  term: (start: boolean, term: "open" | "cap" | "blind") => { const l = selLine(); if (l) act({ SetTerm: { line: l.id, start, term } }); },
  reroute: () => { const l = selLine(); if (!l) return toast("Select a line first."); act({ Reroute: { line: l.id } }).then(o => o && toast(`${l.number || "The line"} routed square again.`, true)); },
  elevation: (dz: number) => { setElev(Math.round((elev() + dz) * 1000) / 1000); },

  // ---------- checks and lists ----------
  check: async () => {
    const r = await call<Issue[]>("check");
    if (!r) return;
    setChecked(r); setCheckStale(false);
    const clashes = r.filter(i => i.kind === "clash").length;
    setBottom(clashes || !r.length ? "clashes" : "checks");
    toast(r.length ? `${clashes} clash${clashes === 1 ? "" : "es"} and ${r.length - clashes} other finding${r.length - clashes === 1 ? "" : "s"}. Click one to show it.` : "No clashes and no errors.", !r.length);
  },
  goto: (i: Issue) => {
    if (i.item) setSel({ kind: "item", id: i.item, line: i.line }); else if (i.line) setSel({ kind: "line", id: i.line }); else if (i.equip) setSel({ kind: "equip", id: i.equip });
    setFocus({ at: i.at, r: 1.5 });
  },
  exportPcf: async (line?: number) => {
    const p = project(); if (!p?.lines.length) return toast("The plant has no lines yet.");
    const l = line ? p.lines.find(x => x.id === line) : undefined;
    const text = await call<string>("export_pcf", { line: line ?? null });
    if (text) download(`${safe(l?.number || p.name)}.pcf`, text, `${l ? `line ${l.number}` : "every line"} for Datum Pipe (File › Import there)`);
  },
  exportGltf: async () => { const d = await call<string>("export_gltf"); if (d) download(`${safe(project()?.name ?? "plant")}.glb`, d, "the 3D model (glTF)"); },
  exportCsv: async (kind: "lines" | "equipment" | "mto") => {
    const text = await call<string>("export_csv", { kind });
    if (text) download(`${safe(project()?.name ?? "plant")} ${kind === "mto" ? "material take-off" : kind === "lines" ? "line list" : "equipment list"}.csv`, text, kind === "mto" ? "the material take-off" : kind === "lines" ? "the line list" : "the equipment list");
  },
  lists: () => call<Lists>("lists"),
  units: (u: "SI" | "US") => { const p = project(); if (p && p.units !== u) act({ SetMeta: { name: p.name, notes: p.notes, plant: p.plant, unit: p.unit, units: u } }); },

  // ---------- views ----------
  fit: () => fitHook(),
  std: (v: string) => viewHook(v),
  selectPlot: () => setSel({ kind: "plot", id: 0 }),
  addArea: () => { const p = project(); if (!p) return; const e = p.plot.e, n = p.plot.n; act({ AddArea: { id: 0, uid: "", name: "", unit: p.unit, min: [e[0] + 2, n[0] + 2, 0], max: [Math.min(e[1], e[0] + 12), Math.min(n[1], n[0] + 10), 6] } }).then(o => o?.created.length && setSel({ kind: "area", id: o.created[0] })); },
  planned: (what: string, v: string) => toast(`${what} arrives in ${v}. File › Roadmap shows what comes when.`),
  isRouting: () => mode() === "route",
};

export const ALL_CMDS: [string, () => void, string?][] = [
  ["Save", () => C.save(), "Ctrl+S"], ["Save As", () => C.save(true)], ["Open", C.open, "Ctrl+O"], ["New plant", () => setBackstage("home"), "Ctrl+N"], ["Print / PDF", C.print, "Ctrl+P"],
  ["Undo", C.undo, "Ctrl+Z"], ["Redo", C.redo, "Ctrl+Y"], ["Delete", C.del, "Del"],
  ["Place a vertical vessel", () => C.place("vertical_vessel")], ["Place a horizontal drum", () => C.place("horizontal_drum")], ["Place a storage tank", () => C.place("tank")],
  ["Place a column", () => C.place("column")], ["Place a centrifugal pump", () => C.place("pump")], ["Place a shell-and-tube exchanger", () => C.place("exchanger")],
  ["Rotate the equipment 90°", () => C.rotate(90)], ["Select everything", () => C.selectAll(), "Ctrl+A"], ["Copy the selection", () => C.copyGroup(), "Ctrl+D"],
  ["Turn the selection 90°", () => C.rotateGroup(90)], ["Turn the selection −90°", () => C.rotateGroup(-90)], ["Mirror the selection east–west", () => C.mirrorGroup(false)], ["Mirror the selection north–south", () => C.mirrorGroup(true)], ["Route a line", C.route, "R"],
  ["Add a gate valve to the selected line", () => C.dropOnSelected("gate")], ["Add a globe valve", () => C.dropOnSelected("globe")], ["Add a check valve", () => C.dropOnSelected("check")], ["Add a ball valve", () => C.dropOnSelected("ball")],
  ["Add a strainer", () => C.dropOnSelected("strainer")], ["Add a flange pair", () => C.dropOnSelected("flange_pair")], ["Add a tee and start a branch", () => C.dropOnSelected("tee")], ["Add a reducer", () => C.dropOnSelected("reducer")],
  ["Route the selected line again", () => C.reroute()], ["Make the line one size larger", () => C.step(true)], ["Make the line one size smaller", () => C.step(false)],
  ["Check clashes and errors", C.check, "F5"], ["Export the plant for Datum Pipe (.pcf)", () => C.exportPcf()], ["Export the selected line for Datum Pipe", () => { const l = selLine(); if (l) C.exportPcf(l.id); else toast("Select a line first."); }],
  ["Export the 3D model (glTF)", C.exportGltf], ["Export the line list (CSV)", () => C.exportCsv("lines")], ["Export the equipment list (CSV)", () => C.exportCsv("equipment")], ["Export the material take-off (CSV)", () => C.exportCsv("mto")],
  ["SI units", () => C.units("SI")], ["US units", () => C.units("US")], ["Add an area", C.addArea], ["Plot settings", C.selectPlot],
  ["Measure a distance", () => C.measure(), "M"], ["Fit the plant", C.fit, "F"], ["Isometric view", () => C.std("iso"), "1"], ["Plan view", () => C.std("plan"), "2"], ["Front view (from the south)", () => C.std("front"), "3"], ["Side view (from the east)", () => C.std("side"), "4"],
  ["Options", () => setBackstage("options")], ["Check for updates", () => setBackstage("about")], ["Roadmap", () => setBackstage("roadmap")],
  ["Keyboard shortcuts", () => D.shortcuts(), "F1"], ["Getting started", () => D.gettingStarted()], ["Glossary", () => D.glossary()],
];
