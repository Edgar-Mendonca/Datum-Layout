// Commands: what the ribbon, menus, keys and Tell Me run.
import { SUPPORTS, SupportKind, SavedView, Out, V3, ItemKind, EquipKind, Issue, Lists, inch, NPS } from "./api";
import { colourBy, setColourBy, ColourBy, isolated, hidden, setHidden, setIsolated, section, setSection, scene, picked, setPicked, out, setOut, act, call, sel, setSel, setBackstage, loadOut, toast, project, setMode, setRoute, route, setPlacing, setChecked, setCheckStale, setBottom, selLine, selEquip, setFocus, mode, elev, setElev } from "./store";
import * as D from "./dialogs";
import { RouteFrom } from "./store";

let fitHook: () => void = () => {};
export const setFit = (f: () => void) => { fitHook = f; };
let viewHook: (v: string) => void = () => {};
/** The 3D view's camera, for saved views. */
let camHook: { get: () => { eye: V3; target: V3 }; set: (eye: V3, target: V3) => void } | null = null;
export const setCamHook = (h: typeof camHook) => { camHook = h; };

/** Asks for a file and gives its text (WebView file input; nothing leaves the computer). */
export const pickText = (accept: string) => new Promise<string | null>(ok => {
  const i = document.createElement("input"); i.type = "file"; i.accept = accept; i.style.display = "none";
  i.onchange = () => { const f = i.files?.[0]; i.remove(); if (!f) return ok(null); if (f.size > 64e6) { toast("That file is too large (over 64 MB)."); return ok(null); } f.text().then(ok, () => ok(null)); };
  document.body.append(i); i.click();
});
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

  // ---------- corners and jogs ----------
  /** The selected straight run: its line, index and end points. */
  runOf: () => {
    const s = sel(); if (s?.kind !== "run") return null;
    const pts = out()?.paths.find(x => x[0] === s.id)?.[1]; if (!pts || s.seg + 1 >= pts.length) return null;
    return { line: s.id, seg: s.seg, a: pts[s.seg], b: pts[s.seg + 1] };
  },
  /** Adds a jog to the selected run, centred `at` metres along it (its middle by default): up a metre on a level run, east on a vertical one. */
  jog: (at?: number) => {
    const r = C.runOf(); if (!r) return toast("Select one straight run of a line (click the line, then the run), then add a jog.");
    const L = Math.hypot(r.b[0] - r.a[0], r.b[1] - r.a[1], r.b[2] - r.a[2]);
    const vertical = Math.abs(r.b[2] - r.a[2]) > L * 0.5;
    const half = Math.min(L * 0.2, 1.5), c = Math.min(Math.max(at ?? L / 2, half + 0.05 * L), L * 0.95 - half);
    act({ Jog: { line: r.line, seg: r.seg, s0: c - half, s1: c + half, delta: vertical ? [1, 0, 0] : [0, 0, 1] } })
      .then(o => { if (o) { setSel({ kind: "run", id: r.line, seg: r.seg + 2 }); toast("Jog added: drag its middle run with the arrows to place it.", true); } });
  },
  removeJog: () => {
    const s = sel(); if (s?.kind !== "run") return toast("Select the middle run of a jog, then take the jog out.");
    act({ RemoveJog: { line: s.id, seg: s.seg } }).then(o => o && setSel({ kind: "line", id: s.id }));
  },

  // ---------- saved views ----------
  saveView: () => {
    const p = project(); if (!p || !camHook) return;
    const c = camHook.get(), s = section();
    let n = p.views.length + 1; while (p.views.some(v => v.name === `View ${n}`)) n++;
    const v: SavedView = { name: `View ${n}`, eye: c.eye, target: c.target, colour_by: colourBy(), hidden: hidden(), isolated: isolated(), section: s ? [s.lo, s.hi] : null };
    act({ SetViews: [...p.views, v] }).then(o => o && toast(`Saved as “${v.name}”. View › Saved Views brings it back.`, true));
  },
  showView: (name: string) => {
    const v = project()?.views.find(x => x.name === name); if (!v) return;
    setColourBy((["kind", "line", "class", "fluid", "size"].includes(v.colour_by) ? v.colour_by : "kind") as ColourBy);
    setHidden(v.hidden ?? []); setIsolated(v.isolated ?? null); setSection(v.section ? { lo: v.section[0], hi: v.section[1] } : null);
    camHook?.set(v.eye, v.target);
  },
  renameView: (name: string, to: string) => { const p = project(); if (!p || !to.trim() || p.views.some(v => v.name === to)) return; act({ SetViews: p.views.map(v => v.name === name ? { ...v, name: to.trim() } : v) }); },
  deleteView: (name: string) => { const p = project(); if (p) act({ SetViews: p.views.filter(v => v.name !== name) }); },

  // ---------- imports ----------
  importLineList: async () => {
    const t = await pickText(".csv,.txt"); if (t == null) return;
    const o = await act({ ImportLineList: { csv: t } });
    if (o) { const n = o.p.line_list.length, iss = o.issues.filter(i => i.kind === "list").length; toast(`${n} line${n === 1 ? "" : "s"} read from the line list. ${iss ? `${iss} difference${iss === 1 ? "" : "s"} with the model are under Checks.` : "The model matches it."}`, true); if (iss) setBottom("checks"); }
  },
  /** A plant package from a P&ID: equipment placed or updated, lines onto the line list. */
  importPackage: async () => {
    const t = await pickText(".json,.dpkg"); if (t == null) return;
    const before = project();
    const o = await act({ ImportPackage: { json: t } });
    if (!o) return;
    const placed = o.created.length, listed = o.p.line_list.length - (before?.line_list.length ?? 0);
    if (placed) { setPicked(o.created); setSel(null); setTimeout(fitHook, 80); }
    toast(`Plant package read: ${placed} equipment item${placed === 1 ? "" : "s"} placed${listed > 0 ? `, ${listed} line${listed === 1 ? "" : "s"} listed` : ""}. Arrange the equipment, then Route Listed Lines.`, true);
  },
  /** Routes the listed lines not yet in the model, nozzle to nozzle, with their components in the P&ID's order. */
  routeListed: async () => {
    const o = await act({ RouteListed: { numbers: [] } });
    if (o?.created.length) { setPicked(o.created); setSel(null); const left = o.issues.filter(i => i.kind === "list" && i.text.includes("not in the model")).length; toast(`${o.created.length} line${o.created.length === 1 ? "" : "s"} routed with their components in the P&ID's order.${left ? ` ${left} left to route by hand (branches or open ends): see Checks.` : ""}`, true); if (left) setBottom("checks"); }
  },
  importPcf: async () => {
    const t = await pickText(".pcf,.txt"); if (t == null) return;
    const o = await act({ ImportPcf: { text: t } });
    if (o?.created.length) { setPicked(o.created); setSel(null); setTimeout(fitHook, 60); toast(`${o.created.length} line${o.created.length === 1 ? "" : "s"} read from the file: elbows, flanges and gaskets are generated from the class.`, true); }
  },

  // ---------- what the view shows ----------
  hide: () => { const ids = C.targets(); if (!ids.length) return toast("Select equipment or lines to hide (Ctrl+click adds more)."); setHidden([...new Set([...hidden(), ...ids])]); setSel(null); setPicked([]); },
  isolate: () => { const ids = C.targets(); if (!ids.length) return toast("Select equipment or lines to show on their own."); setHidden([]); setIsolated(ids); setTimeout(fitHook, 30); },
  showAll: () => { setHidden([]); setIsolated(null); },
  /** A section box around the selection (or the whole plant), or off again. */
  sectionBox: () => {
    if (section()) return setSection(null);
    const ids = C.targets(), parts = (scene()?.parts ?? []).filter(q => q.t1 > q.t0 && (!ids.length || ids.includes(q.line || q.equip)));
    if (!parts.length) return toast("Nothing to put a section box round.");
    const m = 0.5;
    setSection({ lo: [0, 1, 2].map(k => Math.min(...parts.map(q => q.lo[k])) - m) as V3, hi: [0, 1, 2].map(k => Math.max(...parts.map(q => q.hi[k])) + m) as V3 });
  },

  // ---------- moving (0.2.3) ----------
  /** Nudges the selection one grid step (a tenth with `fine`) in a direction on the screen: the
   *  arrow's direction is turned to the plant axis (east, west, north or south) nearest to it as
   *  the view looks now. Equipment, a group, or a line's run. */
  nudge: (screen: "up" | "down" | "left" | "right", fine = false) => {
    const p = project(); if (!p) return;
    const ids = C.targets(), s = sel();
    if (!ids.length && s?.kind !== "run") return;
    const c = camHook?.get(); let f = [0, 1];
    if (c) { const v = [c.target[0] - c.eye[0], c.target[1] - c.eye[1]], l = Math.hypot(v[0], v[1]); if (l > 1e-6) f = [v[0] / l, v[1] / l]; }
    // Up the screen is forward (the view's direction on the plan); right is forward turned clockwise.
    const dir = screen === "up" ? f : screen === "down" ? [-f[0], -f[1]] : screen === "right" ? [f[1], -f[0]] : [-f[1], f[0]];
    const ax: V3 = Math.abs(dir[0]) >= Math.abs(dir[1]) ? [Math.sign(dir[0]), 0, 0] : [0, Math.sign(dir[1]), 0];
    const g = (p.plot.grid || 1) * (fine ? 0.1 : 1), d: V3 = [ax[0] * g, ax[1] * g, 0];
    if (s?.kind === "run" && !ids.length) { act({ MoveRun: { line: s.id, seg: s.seg, delta: d } }, true); return; }
    act({ MoveGroup: { ids, delta: d } }).then(o => C.squareAfterMove(o));
  },
  /** After equipment moved: lines left skewed are routed square again (through their own corners),
   *  as one more step (Ctrl+Z gives back the skewed lines). */
  squareAfterMove: async (o: Out | null) => {
    if (!o) return;
    const bad = [...new Set(o.issues.filter(i => i.kind === "routing" && i.text.includes("no longer leaves")).map(i => i.line))].filter(Boolean);
    if (!bad.length) return;
    const r = await act({ Batch: bad.map(line => ({ Reroute: { line } })) }, true);
    if (r) toast(`${bad.length} line${bad.length === 1 ? "" : "s"} routed square again to follow the move (Ctrl+Z to keep ${bad.length === 1 ? "it" : "them"} as ${bad.length === 1 ? "it was" : "they were"}).`, true);
  },
  /** Moves the selected equipment to a typed position. */
  moveTo: () => { const e = selEquip(); if (!e) return toast("Select an equipment item first."); D.moveTo(e); },

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
  drop: async (line: number, kind: ItemKind, at: V3, support?: SupportKind) => {
    const o = await act({ Drop: { line, kind, at, dir: [0, 0, 0], nps2: "" } });
    if (!o?.created.length) return;
    const id = o.created[0];
    if (kind === "support") {
      const it = o.p.lines.find(l => l.id === line)?.items.find(i => i.id === id);
      if (it && support && support !== it.support) await act({ SetItem: { line, item: { ...it, support } } });
      setSel({ kind: "item", id, line });
      return toast(`${SUPPORTS.find(s => s[0] === (support ?? "rest"))?.[1]} placed on the pipe: it goes to Datum Pipe with the line for the stress analysis.`, true);
    }
    setSel({ kind: "item", id, line });
    if (kind === "tee" || kind === "cross") {
      // A tee (or cross) starts a branch: route from its (first) branch end.
      await C.routeBranch(line, id, 0);
      toast(kind === "cross" ? "Cross placed. Route its first branch now; the second is in its properties (Esc leaves both for later)." : "Tee placed. Route the branch: click its corners, then a nozzle or a free point (Esc leaves the branch for later).", true);
    } else toast(`${kind === "flange_pair" ? "Flange pair" : kind[0].toUpperCase() + kind.slice(1)} placed: it took the line's size and class and split the pipe.`, true);
  },
  /** Starts routing a branch line from a tee's branch end. */
  routeBranch: async (line: number, item: number, port = 0) => {
    const s = await call<any>("scene");
    const pt = s?.ports.find((x: any) => x.kind === "branch" && x.item === item && (x.branch ?? 0) === port && !x.used);
    if (!pt) return toast("That branch already has a line.");
    setPlacing(null);
    setRoute({ from: { kind: "branch", line, item, port }, start: pt.p, via: [], nps: pt.nps });
    setMode("route");
  },
  /** Drops a component on the selected line or run, near its middle. */
  /** A support of a kind on the selected run (or the selected line's longest run), at its middle. */
  support: (k: SupportKind) => C.dropOnSelected("support", k),
  dropOnSelected: (kind: ItemKind, support?: SupportKind) => {
    const l = selLine(); if (!l) return toast("Select a line (or a run of it) first, or drag the component onto a pipe.");
    const s = sel(); const pts = out()?.paths.find(x => x[0] === l.id)?.[1] ?? l.pts;
    let k = s?.kind === "run" ? s.seg : 0, best = -1;
    if (s?.kind !== "run") for (let i = 0; i + 1 < pts.length; i++) { const d = Math.hypot(pts[i + 1][0] - pts[i][0], pts[i + 1][1] - pts[i][1], pts[i + 1][2] - pts[i][2]); if (d > best) { best = d; k = i; } }
    const a = pts[k], b = pts[k + 1];
    C.drop(l.id, kind, [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2, (a[2] + b[2]) / 2], support);
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
  exportCsv: async (kind: "lines" | "equipment" | "mto" | "valves") => {
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
  ["Add a strainer", () => C.dropOnSelected("strainer")], ["Add a flange pair", () => C.dropOnSelected("flange_pair")], ["Add a tee and start a branch", () => C.dropOnSelected("tee")], ["Add a cross (two branches)", () => C.dropOnSelected("cross")], ["Add a reducer", () => C.dropOnSelected("reducer")], ["Make the selected reducer eccentric, flat on top", () => { const s = sel(); const l = selLine(); if (s?.kind === "item" && l) act({ SetEccentric: { line: l.id, item: s.id, flat: "top" } }); else toast("Select a reducer first."); }], ["Add a rest (pipe support)", () => C.support("rest")], ["Add a guide", () => C.support("guide")], ["Add a line stop", () => C.support("stop")], ["Add an anchor", () => C.support("anchor")],
  ["Route the selected line again", () => C.reroute()], ["Make the line one size larger", () => C.step(true)], ["Make the line one size smaller", () => C.step(false)],
  ["Check clashes and errors", C.check, "F5"], ["Export the plant for Datum Pipe (.pcf)", () => C.exportPcf()], ["Export the selected line for Datum Pipe", () => { const l = selLine(); if (l) C.exportPcf(l.id); else toast("Select a line first."); }],
  ["Export the 3D model (glTF)", C.exportGltf], ["Export the line list (CSV)", () => C.exportCsv("lines")], ["Export the equipment list (CSV)", () => C.exportCsv("equipment")], ["Export the material take-off (CSV)", () => C.exportCsv("mto")],
  ["SI units", () => C.units("SI")], ["US units", () => C.units("US")], ["Add an area", C.addArea], ["Plot settings", C.selectPlot],
  ["Add a jog to the selected run", () => C.jog(), "J"], ["Take the jog out", () => C.removeJog()],
  ["Hide the selection", () => C.hide(), "H"], ["Show only the selection (isolate)", () => C.isolate(), "I"], ["Show everything", () => C.showAll(), "Shift+H"], ["Section box", () => C.sectionBox()],
  ["Save this view", () => C.saveView()], ["Import a line list (CSV)", () => C.importLineList()], ["Import lines from a piping component file (.pcf)", () => C.importPcf()], ["Import a plant package from a P&ID", () => C.importPackage()], ["Route the listed lines", () => C.routeListed()],
  ["Measure a distance", () => C.measure(), "M"], ["Fit the plant", C.fit, "F"], ["Isometric view", () => C.std("iso"), "1"], ["Plan view", () => C.std("plan"), "2"], ["Front view (from the south)", () => C.std("front"), "3"], ["Side view (from the east)", () => C.std("side"), "4"],
  ["Options", () => setBackstage("options")], ["Check for updates", () => setBackstage("about")], ["Roadmap", () => setBackstage("roadmap")],
  ["Keyboard shortcuts", () => D.shortcuts(), "F1"], ["Getting started", () => D.gettingStarted()], ["Glossary", () => D.glossary()],
];
