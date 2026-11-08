// Window state: the open plant, the 3D scene, the selection, the routing tool, check results,
// menus, dialogs and messages.
import { createSignal, batch, JSX } from "solid-js";
import { invoke, Out, Options, Project, Scene, Issue, V3, Line, Equipment, Item } from "./api";

export const [out, setOutRaw] = createSignal<Out | null>(null);
export const project = (): Project | undefined => out()?.p;
export const [scene, setScene] = createSignal<Scene | null>(null);
/** The full check (clashes included) and whether the plant changed since it ran. */
export const [checked, setChecked] = createSignal<Issue[] | null>(null);
export const [checkStale, setCheckStale] = createSignal(false);

let sceneSeq = 0;
export const refreshScene = async () => {
  const n = ++sceneSeq;
  try { const s = await invoke<Scene>("scene"); if (n === sceneSeq) setScene(s); } catch (e) { toast(String(e)); }
};

export const setOut = (o: Out | null | undefined) => {
  if (!o) return;
  batch(() => {
    setOutRaw(o);
    // Drop a selection that no longer exists (deleted or undone away).
    const s = sel(), p = o.p;
    if (s) {
      const ok = s.kind === "equip" || s.kind === "nozzle" ? p.equipment.some(e => e.id === s.id) && (s.kind !== "nozzle" || p.equipment.find(e => e.id === s.id)!.nozzles.some(n => n.tag === s.tag))
        : s.kind === "corner" ? p.lines.some(l => l.id === s.id && s.v > 0 && s.v < l.pts.length - 1)
        : s.kind === "line" || s.kind === "run" ? p.lines.some(l => l.id === s.id && (s.kind !== "run" || (s.seg ?? 0) < l.pts.length - 1))
        : s.kind === "item" ? p.lines.some(l => l.id === s.line && l.items.some(i => i.id === s.id))
        : s.kind === "area" ? p.areas.some(a => a.id === s.id) : true;
      if (!ok) setSel(null);
    }
    const ids = new Set([...p.equipment.map(e => e.id), ...p.lines.map(l => l.id)]);
    if (picked().some(i => !ids.has(i))) setPicked(picked().filter(i => ids.has(i)));
  });
  if (checked()) setCheckStale(true);
  refreshScene();
  if (o.warnings?.length) toast(`Opened with repairs: ${o.warnings.join(" ")}`);
};

/** auto, light or dark; auto follows Windows. */
export const [theme, setThemeRaw] = createSignal("auto");
const darkMq = typeof matchMedia === "function" ? matchMedia("(prefers-color-scheme: dark)") : null;
export const [themeTick, setThemeTick] = createSignal(0);
export const applyTheme = () => { document.documentElement.dataset.theme = theme() === "auto" ? (darkMq?.matches ? "dark" : "light") : theme(); setThemeTick(t => t + 1); };
darkMq?.addEventListener("change", () => theme() === "auto" && applyTheme());
export const setTheme = (t: string) => { setThemeRaw(t); applyTheme(); const o = { ...opts(), theme: t }; setOpts(o); invoke("set_options", { opts: o }).catch(() => {}); };
export const [opts, setOpts] = createSignal<Options>({ autosave: 5, beta: false, theme: "auto", openLast: false, units: "SI", lod: 1, clearance: 0.025, labels: true, paper: "A4", landscape: true, company: "", packs: [] });

export const [backstage, setBackstage] = createSignal<string | null>("home");
export const [tab, setTab] = createSignal("home");
/** The bottom panel's tab and whether it is open. */
export const [bottom, setBottom] = createSignal<"lines" | "equipment" | "clashes" | "checks">("lines");

// ---------- selection ----------
/** What is selected: equipment, one of its nozzles, a line, one straight run of it, a component on it, an area, or the plot. */
export type Sel =
  | { kind: "equip"; id: number } | { kind: "nozzle"; id: number; tag: string } | { kind: "line"; id: number }
  | { kind: "run"; id: number; seg: number } | { kind: "corner"; id: number; v: number } | { kind: "item"; id: number; line: number } | { kind: "area"; id: number } | { kind: "plot"; id: 0 };
export const [sel, setSel] = createSignal<Sel | null>(null);
/** A group of equipment and line ids picked with Ctrl+click (or Ctrl+A); empty when none. */
export const [picked, setPicked] = createSignal<number[]>([]);
export const selLine = (): Line | undefined => { const s = sel(), p = project(); if (!s || !p) return; const id = s.kind === "item" ? s.line : s.kind === "line" || s.kind === "run" || s.kind === "corner" ? s.id : -1; return p.lines.find(l => l.id === id); };
export const selEquip = (): Equipment | undefined => { const s = sel(), p = project(); if (!s || !p || (s.kind !== "equip" && s.kind !== "nozzle")) return; return p.equipment.find(e => e.id === s.id); };
export const selItem = (): Item | undefined => { const s = sel(); if (s?.kind !== "item") return; return selLine()?.items.find(i => i.id === s.id); };
/** Something to zoom to (set by the lists and checks; the 3D view follows). */
export const [focus, setFocus] = createSignal<{ at: V3; r: number } | null>(null);

// ---------- what the view shows ----------
/** Parts coloured by their kind, or each line's parts by its number, class, fluid or size. */
export type ColourBy = "kind" | "line" | "class" | "fluid" | "size";
export const [colourBy, setColourBy] = createSignal<ColourBy>("kind");
/** Equipment and line ids hidden; or, when isolated, the only ones shown. */
export const [hidden, setHidden] = createSignal<number[]>([]);
export const [isolated, setIsolated] = createSignal<number[] | null>(null);
/** The section box (plant axes): only what is inside it is drawn. */
export const [section, setSection] = createSignal<{ lo: V3; hi: V3 } | null>(null);

// ---------- tools ----------
/** select: pick and move; route: click a path; place: put the dragged or chosen equipment down. */
export type RouteFrom = { kind: "nozzle"; equip: number; nozzle: string } | { kind: "branch"; line: number; item: number } | { kind: "point"; p: V3 };
export const [mode, setMode] = createSignal<"select" | "route" | "place" | "measure">("select");
/** The measure tool's points (plant axes). */
export const [measure, setMeasure] = createSignal<V3[]>([]);
export const [route, setRoute] = createSignal<{ from: RouteFrom | null; start: V3 | null; via: V3[]; nps: string } | null>(null);
/** The working elevation for clicks in the 3D view (m). */
export const [elev, setElev] = createSignal(3.0);
export const [placing, setPlacing] = createSignal<string | null>(null);

// ---------- editing ----------
export const act = async (op: object | string, quiet = false): Promise<Out | null> => {
  try { const o = await invoke<Out>("apply", { op }); setOut(o); return o; }
  catch (e) { if (!quiet) toast(String(e)); return null; }
};
export const call = async <T = any>(cmd: string, args?: object): Promise<T | undefined> => {
  try { return await invoke<T>(cmd, args); } catch (e) { toast(String(e)); return undefined; }
};
/** Counts plants opened (the 3D view fits each new one). */
export const [loaded, setLoaded] = createSignal(0);
export const loadOut = async (cmd: string, args?: object) => {
  const o = await call<Out | null>(cmd, args);
  if (o) batch(() => { setLoaded(n => n + 1); setPicked([]); setHidden([]); setIsolated(null); setSection(null); setSel(null); setChecked(null); setCheckStale(false); setMode("select"); setRoute(null); setOut(o); setBackstage(null); });
  return o;
};

// ---------- messages, menus, dialogs ----------
export const [msg, setMsg] = createSignal<{ text: string; ok: boolean } | null>(null);
let timer = 0;
export const toast = (text: string, ok = false) => { setMsg({ text, ok }); clearTimeout(timer); timer = window.setTimeout(() => setMsg(null), ok ? 3000 : 7000); };
export type MenuItem = { label: string; run?: () => void; keys?: string; off?: boolean; on?: boolean; sub?: MenuItem[]; icon?: string } | "-";
export type Item_ = MenuItem;
export const [menu, setMenu] = createSignal<{ x: number; y: number; items: MenuItem[] } | null>(null);
export const openMenu = (e: MouseEvent | { x: number; y: number }, items: MenuItem[]) => {
  if ("preventDefault" in e) e.preventDefault();
  const r = (e as MouseEvent).currentTarget instanceof HTMLElement && (e as MouseEvent).type === "click" ? ((e as MouseEvent).currentTarget as HTMLElement).getBoundingClientRect() : null;
  setMenu({ x: r ? r.left : (e as MouseEvent).clientX ?? (e as any).x, y: r ? r.bottom + 2 : (e as MouseEvent).clientY ?? (e as any).y, items });
};
export const [dialogs, setDialogs] = createSignal<(() => JSX.Element)[]>([]);
export const openDialog = (d: () => JSX.Element) => setDialogs(v => [...v, d]);
export const closeDialog = () => setDialogs(v => v.slice(0, -1));
