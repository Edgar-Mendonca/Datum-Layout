// File › Print: settings on the left, a live page preview on the right (FAMILY §8). Pages are laid
// out at real size and printed with the WebView's print dialog, where "Save as PDF" makes a PDF.
// Built on Datum Plan's print page (paper sizes, header and footer slots and codes, @page), by way
// of Datum Pipe. The current 3D view prints on its own page; the lists flow over as many pages as
// they need, with the header and footer on every page.
import { For, Show, JSX, createSignal, createMemo, createEffect, on, onMount } from "solid-js";
import { Portal } from "solid-js/web";
import { createStore } from "solid-js/store";
import { Project, Lists, inch } from "./api";
import { out, opts, call, toast, project, setBackstage } from "./store";
import { Help } from "./help";
import { Pick, Check } from "./ui";
import { snapshot } from "./view3d";
import { toU, unit } from "./units";

export const PAPERS: [string, number, number][] = [["A4", 210, 297], ["A3", 297, 420], ["A2", 420, 594], ["Letter", 215.9, 279.4], ["Legal", 215.9, 355.6], ["Tabloid", 279.4, 431.8]];
export const MM = 96 / 25.4;
type Slots = [string, string, string];
type Setup = { paper: string; landscape: boolean; margins: [number, number, number, number]; header: Slots; footer: Slots; view: boolean; lines: boolean; equipment: boolean; mto: boolean };
export const CODES: [string, string][] = [["&[Page]", "Page number"], ["&[Pages]", "Total pages"], ["&[Date]", "Today's date"], ["&[Plant]", "Plant name"], ["&[Company]", "Company"], ["&[File]", "File name"]];
export const [ps, setPs] = createStore<Setup>({
  paper: "A4", landscape: true, margins: [12, 12, 12, 12], header: ["&[Company]", "&[Plant]", "&[Date]"], footer: ["&[File]", "Page &[Page] of &[Pages]", ""],
  view: true, lines: true, equipment: true, mto: false,
});
const codes = (t: string, page: number, pages: number) => t.replace(/&\[Page\]/g, String(page)).replace(/&\[Pages\]/g, String(pages)).replace(/&\[Date\]/g, new Date().toLocaleDateString())
  .replace(/&\[Plant\]/g, out()?.p.name ?? "").replace(/&\[Company\]/g, opts().company).replace(/&\[File\]/g, out()?.path?.split(/[\\/]/).pop() ?? "not saved");

export const [version, setVersion] = createSignal("");

/** The title block of a printed view. */
export const TitleBlock = (p: { p: Project; title: string }) => (
  <table class="tblock"><tbody>
    <tr><td colSpan={2}><small>Plant</small><b>{p.p.name}</b></td><td><small>Plant / unit</small>{p.p.plant} / {p.p.unit}</td><td><small>Date</small>{new Date().toLocaleDateString()}</td></tr>
    <tr><td><small>Document</small>{p.title}</td><td><small>Units</small>{p.p.units === "US" ? "ft, in, psi, °F" : "m, mm, bar, °C"}</td><td><small>Equipment / lines</small>{p.p.equipment.length} / {p.p.lines.length}</td><td><small>Datum Layout</small>{version()}</td></tr>
  </tbody></table>
);

/** Rows of a long table cut into blocks that each fit a page. */
const tableBlocks = (title: string, head: string[], rows: string[][], per = 26) => {
  const out: (() => JSX.Element)[] = [];
  for (let i = 0; i < Math.max(rows.length, 1); i += per) {
    const chunk = rows.slice(i, i + per);
    out.push(() => <div class="pblock">{i === 0 && <h3>{title}</h3>}<table class="tbl ptab"><thead><tr><For each={head}>{h => <th>{h}</th>}</For></tr></thead><tbody><For each={chunk} fallback={<tr><td colSpan={head.length} class="muted">None.</td></tr>}>{r => <tr><For each={r}>{c => <td>{c}</td>}</For></tr>}</For></tbody></table></div>);
  }
  return out;
};

/** One printed page: header, body, footer. */
const Sheet = (p: { body: JSX.Element; n: number; pages: number; w: number; h: number }) => (
  <div class="ppage" style={{ width: `${p.w}px`, height: `${p.h}px` }}>
    <div class="pin" style={{ left: `${ps.margins[3] * MM}px`, top: `${ps.margins[0] * MM}px`, width: `${p.w - (ps.margins[1] + ps.margins[3]) * MM}px`, height: `${p.h - (ps.margins[0] + ps.margins[2]) * MM}px` }}>
      <div class="phead"><For each={ps.header}>{(t, i) => <span class={`ps${i()}`}>{codes(t, p.n, p.pages)}</span>}</For></div>
      <div class="pbody">{p.body}</div>
      <div class="pfoot"><For each={ps.footer}>{(t, i) => <span class={`ps${i()}`}>{codes(t, p.n, p.pages)}</span>}</For></div>
    </div>
  </div>
);

/**
 * Splits the report into pages: the report is laid out once off screen in a column as wide as the
 * page, and its blocks (sections, tables, pictures) are packed onto pages by their heights.
 */
const paginate = (el: HTMLElement, h: number): number[][] => {
  const blocks = [...(el.firstElementChild?.children ?? [])] as HTMLElement[];
  const pages: number[][] = [[]];
  let used = 0;
  blocks.forEach((b, i) => {
    const bh = b.getBoundingClientRect().height + 6;
    if (used + bh > h && pages[pages.length - 1].length) { pages.push([]); used = 0; }
    pages[pages.length - 1].push(i);
    used += bh;
  });
  return pages;
};

const [printing, setPrinting] = createSignal(false);

export const PrintPage = () => {
  setPs({ paper: opts().paper || "A4", landscape: opts().landscape !== false });
  const [zoomP, setZoomP] = createSignal(0.55);
  const size = createMemo(() => { const p = PAPERS.find(x => x[0] === ps.paper) ?? PAPERS[0]; const [w, h] = ps.landscape ? [p[2], p[1]] : [p[1], p[2]]; return { pw: w, ph: h, w: w * MM, h: h * MM }; });
  const inner = () => ({ w: size().w - (ps.margins[1] + ps.margins[3]) * MM, h: size().h - (ps.margins[0] + ps.margins[2]) * MM - 84 });
  const [split, setSplit] = createSignal<number[][]>([]);
  const [lists, setLists] = createSignal<Lists | null>(null);
  const [shot, setShot] = createSignal("");
  let measure!: HTMLDivElement;
  const relayout = () => requestAnimationFrame(() => { if (measure) setSplit(paginate(measure, inner().h)); });
  onMount(async () => { setVersion(await call<string>("version") ?? ""); setShot(snapshot()); setLists(await call<Lists>("lists") ?? null); relayout(); });
  const blocks = createMemo(() => {
    const l = lists(), p = project(); if (!l || !p) return [];
    const b: (() => JSX.Element)[] = [];
    if (ps.lines) b.push(...tableBlocks("Line list", ["Line number", "From", "To", "Size", "Class", "Fluid", `Length (${unit("len")})`, "Components"], l.lines.map(r => [r.number, r.from, r.to, inch(r.nps), `${r.class} / ${r.rating}`, r.fluid, toU("len", r.length).toFixed(2), String(r.components)])));
    if (ps.equipment) b.push(...tableBlocks("Equipment and nozzles", ["Tag", "Type", "Nozzle", "Size", "Class", "Service", "E", "N", "EL", "Line"], l.equipment.flatMap(e => (e.nozzles.length ? e.nozzles : [null]).map((n, i) => [i ? "" : e.tag, i ? "" : e.kind, n?.tag ?? "", n ? inch(n.nps) : "", n ? String(n.rating) : "", n?.service ?? "", ...[0, 1, 2].map(k => toU("coord", (n ? n.at : e.at)[k]).toFixed(3)), n?.line ?? ""]))));
    if (ps.mto) b.push(...tableBlocks("Material take-off", ["Line", "Group", "Description", "Size", "Quantity", "Unit", `Weight (${unit("mass")})`], l.mto.map(r => [r.line, r.group, r.what, inch(r.nps), r.unit === "m" ? toU("len", r.qty).toFixed(2) : String(r.qty), r.unit === "m" ? unit("len") : r.unit, r.kg == null ? "—" : `${r.est ? "≈ " : ""}${toU("mass", r.kg).toFixed(1)}`])));
    return b;
  });
  createEffect(on(blocks, relayout, { defer: true }));
  const pages = createMemo<JSX.Element[]>(() => {
    const p = project(); const b = blocks();
    const view = ps.view && p && shot() ? [<div class="pview"><TitleBlock p={p} title="3D view" /><img src={shot()} style={{ "max-width": "100%", "max-height": `${inner().h - 70}px` }} /></div>] : [];
    return [...view, ...split().map(ix => <div class="preport">{ix.map(i => b[i]?.())}</div>)];
  });
  const doPrint = () => {
    setPrinting(true);
    const css = document.createElement("style");
    css.textContent = `@page { size: ${size().pw}mm ${size().ph}mm; margin: 0 }`;
    document.head.append(css);
    document.documentElement.classList.add("printing");
    setTimeout(() => { window.print(); document.documentElement.classList.remove("printing"); css.remove(); setPrinting(false); }, 250);
  };
  return (
    <div class="printpage">
      <div class="pset">
        <h1>Print <Help k="print" /></h1>
        <div class="pbtns"><button class="btn pri big" onClick={doPrint}><svg class="ic"><use href="#i-print" /></svg> Print / Save as PDF</button></div>
        <p class="muted">In the print window choose your printer, or <b>Save as PDF</b>. {pages().length} {pages().length === 1 ? "page" : "pages"} · {ps.paper} {ps.landscape ? "landscape" : "portrait"}.</p>
        <h4>What to print</h4>
        <Check label="The 3D view, as it is on screen" value={ps.view} set={v => setPs("view", v)} />
        <Check label="The line list" value={ps.lines} set={v => { setPs("lines", v); relayout(); }} />
        <Check label="The equipment list with nozzles" value={ps.equipment} set={v => { setPs("equipment", v); relayout(); }} />
        <Check label="The material take-off" value={ps.mto} set={v => { setPs("mto", v); relayout(); }} />
        <div class="prow"><label>Paper<Pick value={ps.paper} set={v => { setPs("paper", v); relayout(); }} options={PAPERS.map(p => p[0])} /></label><label>Orientation<Pick value={ps.landscape ? "1" : "0"} set={v => { setPs("landscape", v === "1"); relayout(); }} options={[["0", "Portrait"], ["1", "Landscape"]]} /></label></div>
        <h4>Header and footer</h4>
        <For each={["header", "footer"] as const}>{k => <div class="pslots"><b>{k === "header" ? "Header" : "Footer"}</b>
          <For each={["Left", "Center", "Right"]}>{(l, i) => <label><span>{l}</span><textarea rows="1" value={ps[k][i()]} onInput={e => setPs(k, i(), e.currentTarget.value)} /></label>}</For></div>}</For>
        <p class="muted pcodes">Codes: <For each={CODES}>{([c, l]) => <code title={l} onClick={() => navigator.clipboard?.writeText(c).then(() => toast(`${c} copied.`, true))}>{c}</code>}</For></p>
        <div class="prow">{(["Top", "Right", "Bottom", "Left"] as const).map((l, i) => <label>{l} (mm)<input type="number" min="0" max="60" value={ps.margins[i]} onChange={e => { setPs("margins", i, +e.currentTarget.value); relayout(); }} /></label>)}</div>
        <p><button class="btn sm" onClick={() => setBackstage(null)}>Back to the plant</button></p>
      </div>
      <div class="pprev">
        <div class="pzoom"><span class="muted">Preview</span><input type="range" min="0.25" max="1.2" step="0.05" value={zoomP()} onInput={e => setZoomP(+e.currentTarget.value)} /><span class="muted">{Math.round(zoomP() * 100)}%</span></div>
        <div class="pmeasure" ref={measure} style={{ width: `${inner().w}px` }}><div class="preport">{blocks().map(b => b())}</div></div>
        <For each={pages()} fallback={<p class="pempty">Nothing chosen to print.</p>}>{(pg, i) =>
          <div class="pwrap" style={{ width: `${size().w * zoomP()}px`, height: `${size().h * zoomP()}px` }}><div style={{ transform: `scale(${zoomP()})`, "transform-origin": "0 0" }}><Sheet body={pg} n={i() + 1} pages={pages().length} w={size().w} h={size().h} /></div><small>Page {i() + 1} of {pages().length}</small></div>}</For>
      </div>
      <Show when={printing()}><Portal mount={document.body}><div id="printroot"><For each={pages()}>{(pg, i) => <Sheet body={pg} n={i() + 1} pages={pages().length} w={size().w} h={size().h} />}</For></div></Portal></Show>
    </div>
  );
};
