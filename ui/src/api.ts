// Calls into the Rust shell and the shapes it returns (they mirror crates/core/src/model.rs,
// route.rs, scene.rs and lists.rs).
declare global {
  interface Window { __TAURI__?: { core: { invoke: (cmd: string, args?: object) => Promise<any> }; event: { listen: (e: string, f: (x: { payload: any }) => void) => Promise<() => void> } } }
}

export const invoke = <T = any>(cmd: string, args?: object): Promise<T> =>
  window.__TAURI__ ? window.__TAURI__.core.invoke(cmd, args) : Promise.reject("Datum Layout must run inside its desktop window.");
export const listen = (e: string, f: (p: any) => void) => window.__TAURI__?.event.listen(e, x => f(x.payload));

export type V3 = [number, number, number];
export interface Plot { grid: number; e: [number, number]; n: [number, number]; north: number; ground: number }
export interface Area { id: number; uid: string; name: string; unit: string; min: V3; max: V3 }
export type EquipKind = "vertical_vessel" | "horizontal_drum" | "tank" | "column" | "pump" | "exchanger";
export interface Nozzle { tag: string; nps: string; rating: number; facing: string; pos: V3; dir: V3; proj: number; service: string }
export interface Dims { d: number; l: number; h: number; elev: number; d2: number; l2: number; w: number }
export interface Equipment { id: number; uid: string; tag: string; kind: EquipKind; pos: V3; rot: number; dims: Dims; nozzles: Nozzle[]; data: [string, string][]; area: number }
export type Term = "open" | "cap" | "blind";
export type End = { kind: "point"; term: Term } | { kind: "nozzle"; equip: number; nozzle: string } | { kind: "branch"; line: number; item: number; port?: number };
export type ItemKind = "gate" | "globe" | "check" | "ball" | "butterfly" | "strainer" | "flange_pair" | "tee" | "reducer" | "cross" | "weldolet" | "sockolet" | "support";
export type SupportKind = "rest" | "guide" | "stop" | "anchor";
export const SUPPORTS: [SupportKind, string][] = [["rest", "Rest (shoe)"], ["guide", "Guide"], ["stop", "Line stop"], ["anchor", "Anchor"]];
export type Flat = "none" | "top" | "bottom";
export interface Item { id: number; uid: string; kind: ItemKind; seg: number; at: number; nps2: string; dir: V3; tag: string; support: SupportKind; flat: Flat }
export interface Line { id: number; uid: string; number: string; nps: string; class: string; rating: number; fluid: string; insulation: number; radius: "long" | "short"; from: End; to: End; pts: V3[]; items: Item[]; area: number }
export interface Settings { clearance: number; nps: string; class: string; rating: number }
export interface Project { format: number; app: string; name: string; notes: string; plant: string; unit: string; units: "SI" | "US"; plot: Plot; areas: Area[]; equipment: Equipment[]; lines: Line[]; settings: Settings; next: number; views: SavedView[]; line_list: ListedLine[] }
export interface SavedView { name: string; eye: V3; target: V3; colour_by: string; hidden: number[]; isolated: number[] | null; section: [V3, V3] | null }
export interface ListedLine { number: string; nps: string; class: string; rating: number; fluid: string; insulation: number; from: string; to: string; notes: string; uid: string; components: string[] }
export interface Issue { kind: "routing" | "clash" | "open" | "mismatch" | "class" | "number" | "list" | "rule"; text: string; at: V3; line: number; item: number; equip: number }
export interface Out { p: Project; path: string | null; dirty: boolean; undo: boolean; redo: boolean; created: number[]; warnings: string[]; issues: Issue[]; paths: [number, V3[]][] }

export interface PartRef { kind: string; line: number; equip: number; item: number; seg: number; nps: string; t0: number; t1: number; lo: V3; hi: V3 }
export interface PortRef { p: V3; d: V3; nps: string; kind: "nozzle" | "end" | "branch"; equip: number; tag: string; line: number; item: number; used: boolean; branch: number }
export interface Scene { pos: string; nor: string; idx: string; parts: PartRef[]; ins_pos: string; ins_nor: string; ins_idx: string; ports: PortRef[]; triangles: number }

export interface PipeRow { nps: string; dn: number; od: number; schedule: string; wall: number; source: string }
export interface BranchRule { header_min?: string | null; header_max?: string | null; branch?: string | null; branch_min?: string | null; branch_max?: string | null; fitting: string }
export interface PipeClass { branches?: BranchRule[]; name: string; description: string; material: string; ratings: number[]; sizes: string[]; schedules: [string, string][]; elbow: string; components: { kind: string; min: string | null; max: string | null }[]; source: string }
export interface Catalogue { pipes: PipeRow[]; fittings: any[]; flanges: any[]; valves: any[]; tees: { nps: string; nps_branch: string; c: number; m: number; source: string }[]; reducers: { nps: string; nps_small: string; h: number; source: string }[]; olets: { kind: string; same: boolean; nps_branch: string; height: number; base_od: number; top_od: number; weight_lb: number | null; source: string }[]; weights: WeightRow[]; classes: PipeClass[]; packs: string[]; warnings: string[] }
export interface CatalogueOut { own?: string[]; base?: string[]; cat: Catalogue; fields: [EquipKind, string, [string, string][]][] }

export interface LineRow { id: number; number: string; from: string; to: string; nps: string; class: string; rating: number; fluid: string; insulation: number; length: number; components: number; weight: number; unweighed: number }
export interface NozzleRow { tag: string; nps: string; rating: number; facing: string; service: string; at: V3; dir: V3; line: string }
export interface EquipRow { id: number; tag: string; kind: string; at: V3; rot: number; sizes: [string, number][]; data: [string, string][]; nozzles: NozzleRow[] }
export interface MtoRow { line: string; group: string; what: string; nps: string; qty: number; unit: string; unit_kg: number | null; kg: number | null; est: boolean }
export interface WeightRow { kind: string; nps: string; nps_small: string; wall: string; rating: number; kg: number; source: string }
export interface ValveRow { line_id: number; item: number; tag: string; kind: string; nps: string; rating: number; line: string; at: V3; stem: string }
export interface Lists { lines: LineRow[]; equipment: EquipRow[]; mto: MtoRow[]; valves: ValveRow[] }

export interface Example { id: string; name: string; what: string }
export interface Shown { q: string; label: string; factor: number; offset: number; dim?: number[] }
export interface Typed { sym: string; factor: number; offset: number; dim: number[] }
export interface Options {
  autosave: number; beta: boolean; theme: string; openLast: boolean; units: "SI" | "US"; lod: number; clearance: number; labels: boolean;
  paper: string; landscape: boolean; company: string; packs: string[];
}

/** Flange pressure classes. */
export const RATINGS = [150, 300, 600, 900, 1500, 2500];

/** Nominal sizes in order. */
export const NPS = ["1/2", "3/4", "1", "1-1/4", "1-1/2", "2", "2-1/2", "3", "3-1/2", "4", "5", "6", "8", "10", "12", "14", "16", "18", "20", "22", "24"];
/** ½" instead of 1/2". */
export const inch = (n: string) => n.replace("1/2", "½").replace("3/4", "¾").replace("1/4", "¼").replace("-", " ") + "\"";

export const ITEMS: [ItemKind, string, string][] = [
  ["gate", "Gate valve", "valve"], ["globe", "Globe valve", "globe"], ["check", "Check valve", "chkvalve"], ["ball", "Ball valve", "ball"], ["butterfly", "Butterfly valve", "butterfly"],
  ["strainer", "Strainer", "strainer"], ["flange_pair", "Flange pair", "flange"], ["tee", "Tee (starts a branch)", "tee"], ["reducer", "Reducer", "reducer"], ["cross", "Cross (two branches)", "cross"], ["weldolet", "Weldolet (branch outlet)", "weldolet"], ["sockolet", "Sockolet (small branch)", "sockolet"],
  ["support", "Pipe support", "support"],
];
export const EQUIP: [EquipKind, string, string][] = [
  ["vertical_vessel", "Vertical vessel", "vvessel"], ["horizontal_drum", "Horizontal drum", "drum"], ["tank", "Storage tank", "tank"],
  ["column", "Column", "column"], ["pump", "Centrifugal pump", "pump"], ["exchanger", "Shell-and-tube exchanger", "exchanger"],
];

/** A number typed by a person: "−" and "," are understood; empty or not a number gives null. */
export const parseNum = (s: string): number | null => {
  let t = s.trim().replace(/[−–]/g, "-").replace(/\s/g, "");
  if (!t) return null;
  if (t.includes(",")) t = t.includes(".") || /^-?\d{1,3}(,\d{3})+$/.test(t) ? t.replace(/,/g, "") : t.replace(",", ".");
  const v = Number(t);
  return Number.isFinite(v) ? v : null;
};

/** Numbers for people: up to 4 significant digits, no trailing zeros. */
export const num = (v: number, sig = 4) => {
  if (!isFinite(v)) return v > 0 ? "∞" : v < 0 ? "−∞" : "—";
  if (v === 0) return "0";
  const a = Math.abs(v);
  if (a >= 1e7 || a < 1e-3) return v.toExponential(Math.max(0, sig - 1)).replace(/\.?0+e/, "e").replace("-", "−");
  return String(+v.toPrecision(Math.max(sig, Math.ceil(Math.log10(a + 1))))).replace("-", "−");
};

/** Base64 → typed arrays (the scene's buffers). */
const bytes = (b64: string) => { const s = atob(b64); const u = new Uint8Array(s.length); for (let i = 0; i < s.length; i++) u[i] = s.charCodeAt(i); return u; };
export const f32 = (b64: string) => new Float32Array(bytes(b64).buffer);
export const u32 = (b64: string) => new Uint32Array(bytes(b64).buffer);
