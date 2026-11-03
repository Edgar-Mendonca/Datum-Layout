// Display units. The engine and the file use SI base units (m, kg, N, Pa, K); the engine also sends
// how every quantity is shown in SI and US units (crates/core/src/units.rs, from Datum Calc's unit
// table), so this file holds no conversion factors of its own: it only applies them.
import { createSignal } from "solid-js";
import { out } from "./store";
import { num, parseNum, Shown, Typed } from "./api";

export type Q = "coord" | "len" | "dim" | "clear" | "angle" | "mass" | "press" | "temp" | "vol" | "power" | "flow" | "area";

const [tables, setTables] = createSignal<[Shown[], Shown[]] | null>(null);
let typedUnits: Typed[] = [];
export const setUnitTable = (t: [Shown[], Shown[], Typed[]?]) => { setTables([t[0], t[1]]); typedUnits = t[2] ?? []; };

const row = (q: Q, us?: boolean): Shown => {
  const t = tables(); const sys = us ?? out()?.p.units === "US";
  return t?.[sys ? 1 : 0].find(s => s.q === q) ?? { q, label: "", factor: 1, offset: 0 };
};

/** SI value → display value. */
export const toU = (q: Q, v: number) => { const r = row(q); return v / r.factor - r.offset; };
/** Display value → SI value. */
export const fromU = (q: Q, v: number) => { const r = row(q); return (v + r.offset) * r.factor; };
export const unit = (q: Q, us?: boolean) => row(q, us).label;
/** A value with its unit, for labels. */
export const fmt = (q: Q, v: number, sig = 4) => `${num(toU(q, v), sig)}${unit(q) ? " " + unit(q) : ""}`;
/** A temperature difference or rise has no offset. */
export const fmtD = (q: Q, v: number, sig = 4) => `${num(v / row(q).factor, sig)}${unit(q) ? " " + unit(q) : ""}`;

/** A value as a person types it, in the display unit of `q`: a plain number ("20"), or numbers with
 *  units, added up ("20 ft", "6 in", "3' 6\"", "1 ft 6 in", "150 mm", "3.5 bar", "300 °F"). The
 *  units and their factors come from the engine's unit table. Gives the value in display units, or
 *  a reason it cannot be read. */
export const parseQ = (s: string, q: Q): number | string => {
  const plain = parseNum(s);
  if (plain !== null) return plain;
  const t = s.trim().replace(/[′’']/g, " ft ").replace(/[″”"]/g, " in ").replace(/[−–]/g, "-");
  const parts = [...t.matchAll(/(-?\d[\d.,]*(?:[eE]-?\d+)?)\s*([^\d\s-][^\d\s]*)?/g)];
  if (!parts.length || parts.map(p => p[0]).join("").replace(/\s/g, "") !== t.replace(/\s/g, "")) return `“${s}” is not a number.`;
  const r = row(q);
  let si = 0;
  for (const [, n, sym] of parts) {
    const v = parseNum(n);
    if (v === null) return `“${s}” is not a number.`;
    if (!sym) { si += fromU(q, v); continue; }
    const u = typedUnits.find(x => x.sym === sym) ?? typedUnits.find(x => x.sym.toLowerCase() === sym.toLowerCase());
    if (!u) return `“${sym}” is not a unit Datum Layout knows.`;
    if (r.dim && u.dim.some((d, i) => d !== r.dim![i])) return `“${sym}” is not a unit of ${QNAME[q] ?? "this value"}.`;
    si += v * u.factor + (parts.length === 1 ? u.offset : 0);
  }
  return toU(q, si);
};
const QNAME: Partial<Record<Q, string>> = { coord: "length", len: "length", dim: "length", clear: "length", press: "pressure", temp: "temperature", mass: "mass", angle: "angle", area: "area", vol: "volume", power: "power", flow: "volume flow" };
