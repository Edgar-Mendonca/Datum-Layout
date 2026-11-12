// The side and bottom panels: the catalogue on the left (search, categories, drag source), the
// Inspector on the right (whatever is selected), and the bottom tabs (Lines, Equipment, Clashes,
// Checks), each issue clickable.
import { For, Show, createSignal, createResource, createMemo, createEffect, on, JSX } from "solid-js";
import { SUPPORTS, SupportKind, CatalogueOut, DEXPI, EQUIP, ITEMS, PART_GROUPS, PART_WORDS, ItemKind, NPS, RATINGS, inch, Equipment, Nozzle, Line, V3, Lists, Issue, EquipKind } from "./api";
import { picked, setPicked, call, project, out, sel, setSel, selLine, selEquip, selItem, act, toast, openDialog, setPartsOpen, recent, useRecent, bottom, setBottom, bottomOpen, setBottomOpen, BottomTab, checked, checkStale, setFocus } from "./store";
import { C } from "./cmds";
import { Help, helpCard } from "./help";
import { Icon, Dialog } from "./ui";
import { fmt, toU, fromU, unit, parseQ, Q } from "./units";

// ------------------------------------------------------------ the catalogue
export const [catalogue, { refetch: refetchCatalogue }] = createResource(() => call<CatalogueOut>("catalogue"));

const dragStart = (type: "item" | "equip", kind: string) => (e: DragEvent) => {
  (window as any).__drag = { type, kind };
  e.dataTransfer?.setData("text/plain", `${type}:${kind}`);
  if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
};

/** Where a component's dimensions are in the Library (equipment: its help). */
const LIB_PAGE: Record<string, string> = { gate: "valves", globe: "valves", check: "valves", ball: "valves", butterfly: "valves", strainer: "valves", flange_pair: "flanges", reducer: "reducers", tee: "tees", cross: "tees", weldolet: "olets", sockolet: "olets" };
/** A free spot for equipment placed by a double-click: the plot's middle, stepped east past what is there. */
const freeSpot = (kind: string): V3 => {
  const p = project()!; const { e, n, ground } = p.plot;
  const big = (k: string) => ["sphere", "heater", "air_cooler", "tank"].includes(k) ? 7 : ["compressor", "kettle", "horizontal_drum", "exchanger", "package", "column"].includes(k) ? 4.5 : 3;
  const free = (x: number, y: number) => p.equipment.every(q => Math.hypot(q.pos[0] - x, q.pos[1] - y) >= big(q.kind) + big(kind));
  const cx = (e[0] + e[1]) / 2, cy = (n[0] + n[1]) / 2;
  // Rings outward from the plot's middle, on the grid of 2 m.
  for (let r = 0; r <= Math.max(e[1] - e[0], n[1] - n[0]); r += 2) for (let a = 0; a < 16; a++) {
    const x = Math.round((cx + r * Math.cos(a * Math.PI / 8)) / 2) * 2, y = Math.round((cy + r * Math.sin(a * Math.PI / 8)) / 2) * 2;
    if (x >= e[0] + 2 && x <= e[1] - 2 && y >= n[0] + 2 && y <= n[1] - 2 && free(x, y)) return [x, y, ground];
  }
  return [cx, cy, ground];
};
/** The Parts panel (0.2.12, after Datum Mesh's parts panel): what can be placed, docked beside the
 *  tree and open while you work. Drag a part in, click it (equipment: then click the plot; a
 *  component: onto the selected line), or double-click equipment to place it at once. */
export const PartsPanel = () => {
  const [q, setQ] = createSignal("");
  const templates = () => (catalogue()?.cat.templates ?? []).filter(t => { const w = q().trim().toLowerCase(); return !w || `${t.name} ${t.equip.kind.replace("_", " ")} template`.toLowerCase().includes(w); });
  const all = (): [string, string, string, "equip" | "item"][] => [...EQUIP.map(([k, l, i]) => [k, l, i, "equip"] as [string, string, string, "equip"]), ...ITEMS.map(([k, l, i]) => [k, l, i, "item"] as [string, string, string, "item"])];
  const part = (k: string) => all().find(x => x[0] === k);
  const words = (k: string, g = "", sub = "") => `${part(k)?.[1] ?? k} ${PART_WORDS[k] ?? ""} ${g} ${sub}`.toLowerCase();
  const has = (k: string, g = "", sub = "") => { const t = q().trim().toLowerCase(); return !t || t.split(/\s+/).every(w => words(k, g, sub).includes(w)); };
  const use = (k: string) => {
    const x = part(k); if (!x) return; useRecent(k);
    if (x[3] === "equip") C.place(k as EquipKind); else C.dropOnSelected(k as ItemKind);
  };
  const Tile = (t: { k: string }) => {
    const x = () => part(t.k)!;
    const lib = () => LIB_PAGE[t.k];
    return <Show when={part(t.k)}>
      <button class="cattile" data-kind={t.k} draggable="true" onDragStart={e => { useRecent(t.k); dragStart(x()[3], t.k)(e); }} onDragEnd={() => { (window as any).__drag = undefined; }}
        onClick={() => use(t.k)} onDblClick={() => { if (x()[3] === "equip" && project()) { C.cancelTool(); act({ AddEquipment: { kind: t.k, pos: freeSpot(t.k) } }).then(o => o?.created.length && setSel({ kind: "equip", id: o.created[0] })); } }}
        title={x()[3] === "equip" ? `${x()[1]}: drag onto the plot, click then click the plot, or double-click to place it now` : `${x()[1]}: drag onto a pipe (it snaps and takes the line's size and class), or select a line and click`}>
        <Icon n={x()[2]} size={22} /><span>{x()[1].replace(/ \(.*\)$/, "")}</span>
        <i class="cinfo" role="button" title={lib() ? "Its dimensions in the Library" : "How it is drawn"} onClick={e => { e.stopPropagation(); if (lib()) import("./library").then(k => k.library(lib())); else helpCard(x()[3] === "equip" ? "equipment" : "fittings"); }}>i</i>
      </button></Show>;
  };
  const addPack = async () => { const w = await call<string[] | null>("add_pack"); if (w) { refetchCatalogue(); toast(w.length ? `Pack added with ${w.length} warning(s): ${w[0]}` : "Pack added to the catalogue.", !w.length); } };
  return (
    <aside class="catp partsp">
      <header>Parts <Help k="catalogue" /><span class="grow" /><button class="x" title="Close the Parts panel (Tools › Parts opens it again)" onClick={() => setPartsOpen(false)}>×</button></header>
      <div class="catbody">
        <input class="catsearch" type="search" placeholder="Search: pump, isolation, olet…" value={q()} onInput={e => setQ(e.currentTarget.value)} />
        {/* A fixed height (one row), so a click that updates it never moves the parts under the pointer. */}
        <Show when={!q()}>
          <div class="catgrp recentgrp"><div class="csub">Recent</div><div class="catgrid"><For each={recent().filter(k => part(k)).slice(0, 3)} fallback={<p class="muted small rempty">The parts you use appear here.</p>}>{k => <Tile k={k} />}</For></div></div>
        </Show>
        <Show when={templates().length}>
          <details class="catgrp tmplgrp" open><summary>My templates</summary>
            <div class="catgrid"><For each={templates()}>{t => {
              const icon = () => EQUIP.find(x => x[0] === t.equip.kind)?.[2] ?? "vvessel";
              const put = (pos?: V3) => { if (!project()) return; C.cancelTool(); act({ AddFromTemplate: { equip: t.equip, pos: pos ?? freeSpot(t.equip.kind) } }).then(o => o?.created.length && setSel({ kind: "equip", id: o.created[o.created.length - 1] })); };
              return <button class="cattile tmpl" data-template={t.name} draggable="true" onDragStart={e => { (window as any).__drag = { type: "template", kind: t.equip.kind, template: t.equip }; e.dataTransfer?.setData("text/plain", `template:${t.name}`); }} onDragEnd={() => { (window as any).__drag = undefined; }}
                onClick={() => put()} title={`${t.name}: ${EQUIP.find(x => x[0] === t.equip.kind)?.[1] ?? ""}, ${t.equip.nozzles.length} nozzles${t.origin && t.origin !== "My templates" ? ` · from the pack ${t.origin}` : ""}. Click to place it in a free spot, or drag it onto the plot.`}>
                <Icon n={icon()} size={22} /><span>{t.name}</span>
                <Show when={t.origin === "My templates"}><i class="cinfo tdel" role="button" title="Delete this template" onClick={async e => { e.stopPropagation(); await call("delete_template", { name: t.name }); refetchCatalogue(); toast(`Template ${t.name} deleted.`, true); }}>×</i></Show>
              </button>; }}</For></div>
          </details>
        </Show>
        <For each={PART_GROUPS}>{g => <Show when={g.subs.some(([sub, ks]) => ks.some(k => has(k, g.g, sub)))}>
          <details class="catgrp" open><summary>{g.g}</summary>
            <For each={g.subs}>{([sub, ks]) => <Show when={ks.some(k => has(k, g.g, sub))}>
              <div class="csub">{sub}</div>
              <div class="catgrid"><For each={ks.filter(k => has(k, g.g, sub))}>{k => <Tile k={k} />}</For></div>
            </Show>}</For>
          </details></Show>}</For>
        <Show when={q() && !all().some(([k]) => has(k))}><p class="muted small">Nothing called that. Try a kind (pump, valve, branch) or a use (isolation, non-return).</p></Show>
        <Show when={!q()}>
          <details class="catgrp"><summary>Pipe classes and packs</summary>
          <For each={catalogue()?.cat.classes ?? []}>{c => <div class="catclass" role="button" title="Open in the Library" onClick={() => import("./library").then(k => k.library(`class:${c.name}`))}><b>{c.name}</b><span>{c.description}</span><small>{inch(c.sizes[0] ?? "")}–{inch(c.sizes[c.sizes.length - 1] ?? "")} · class {c.ratings.join(", ")}</small></div>}</For>
          <h4>Plug-in packs <Help k="packs" /></h4>
          <For each={catalogue()?.cat.packs ?? []} fallback={<p class="muted">Company pipe classes and extra parts, as a folder of CSV and JSON files.</p>}>{p => <div class="catpack"><span>{p}</span><button class="x" title="Remove the pack" onClick={async () => { await call("remove_pack", { name: p }); refetchCatalogue(); }}>×</button></div>}</For>
          <Show when={catalogue()?.cat.warnings.length}><details class="catwarn"><summary>{catalogue()!.cat.warnings.length} row(s) not used</summary><For each={catalogue()!.cat.warnings}>{w => <p>{w}</p>}</For></details></Show>
          <button class="btn sm" onClick={addPack}>Add a pack…</button>
          </details>
        </Show>
      </div>
    </aside>
  );
};

// ------------------------------------------------------------ the Inspector
/** A number with its unit, typed in display units (values with units are understood). */
export const NumIn = (p: { q: Q; v: number; set: (si: number) => void; min?: number; title?: string }) => (
  <span class="numin"><input class="mono" value={+toU(p.q, p.v).toFixed(4)} title={p.title}
    onChange={e => { const r = parseQ(e.currentTarget.value, p.q); if (typeof r === "string") { toast(r); e.currentTarget.value = String(+toU(p.q, p.v).toFixed(4)); return; } const si = fromU(p.q, r); if (p.min !== undefined && si < p.min) { toast("That value is too small."); return; } p.set(si); }} />
    <small>{unit(p.q)}</small></span>
);
const Row = (p: { l: string; children: JSX.Element; h?: string }) => <div class="iprow"><label>{p.l}{p.h && <Help k={p.h} />}</label>{p.children}</div>;
const V3In = (p: { v: V3; set: (v: V3) => void; q?: Q; labels?: [string, string, string] }) => (
  <div class="v3"><For each={[0, 1, 2]}>{i => <label class="v3l"><small>{(p.labels ?? ["E", "N", "EL"])[i]}</small><NumIn q={p.q ?? "coord"} v={p.v[i]} set={x => { const n = [...p.v] as V3; n[i] = x; p.set(n); }} /></label>}</For></div>
);
const SizeSel = (p: { v: string; set: (s: string) => void }) => <select onChange={e => p.set(e.currentTarget.value)}><For each={NPS}>{s => <option value={s} selected={s === p.v}>{inch(s)}</option>}</For></select>;
const endText = (e: Line["from"]) => { const pr = project(); return e.kind === "nozzle" ? `${pr?.equipment.find(x => x.id === e.equip)?.tag} / ${e.nozzle}` : e.kind === "branch" ? `Branch of ${pr?.lines.find(l => l.id === e.line)?.number}` : e.term === "open" ? "Open end" : e.term === "cap" ? "Cap" : "Blind flange"; };

/** Keeps an item as one of the user's templates (asks its name). */
const saveTemplate = (e: Equipment) => {
  let name = `${EQUIP.find(x => x[0] === e.kind)?.[1] ?? "Item"} ${e.tag}`;
  openDialog(() => (
    <Dialog title="Save as a template" okText="Save" cancel="Cancel" help="templates" ok={async () => {
      const w = await call<string[]>("save_template", { name, equip: e });
      if (w) { refetchCatalogue(); setPartsOpen(true); toast(w.length ? w[0] : `Saved: ${name.trim()} is in Parts › My templates, for every plant.`, !w.length); }
    }}>
      <p class="muted small">Its type, sizes, nozzles and datasheet are kept. Place it again from Parts › My templates in any plant; its tag is given there.</p>
      <label class="fld">Name<input value={name} maxLength={60} onInput={x => { name = x.currentTarget.value; }} /></label>
    </Dialog>
  ));
};
/** The equipment card's tab (kept while moving between items). */
const [eqTab, setEqTab] = createSignal<"general" | "nozzles" | "datasheet">("general");
let lastEq = 0;
const EquipInsp = (p: { e: Equipment }) => {
  const fields = () => catalogue()?.fields.find(f => f[0] === p.e.kind)?.[2] ?? [];
  const set = (e: Partial<Equipment>) => act({ SetEquipment: { ...p.e, ...e } });
  const setN = (i: number, n: Partial<Nozzle>) => set({ nozzles: p.e.nozzles.map((x, j) => j === i ? { ...x, ...n } : x) });
  const addN = () => { let k = p.e.nozzles.length + 1; while (p.e.nozzles.some(n => n.tag === `N${k}`)) k++; set({ nozzles: [...p.e.nozzles, { tag: `N${k}`, nps: "2", rating: 150, facing: "RF", pos: [0, 0, 1], dir: [1, 0, 0], proj: 0.2, service: "" }] }); };
  const dirs: [string, V3][] = [["East (+E)", [1, 0, 0]], ["West (−E)", [-1, 0, 0]], ["North (+N)", [0, 1, 0]], ["South (−N)", [0, -1, 0]], ["Up", [0, 0, 1]], ["Down", [0, 0, -1]]];
  const dirKey = (d: V3) => dirs.find(x => x[1].every((v, i) => Math.abs(v - d[i]) < 1e-6))?.[0] ?? "Other";
  // A picked nozzle opens the Nozzles tab.
  createEffect(on(() => sel()?.kind === "nozzle", n => { if (n) setEqTab("nozzles"); }));
  // Another item opens on General.
  createEffect(on(() => p.e.id, id => { if (id !== lastEq && sel()?.kind !== "nozzle") setEqTab("general"); lastEq = id; }));
  return (<>
    <nav class="itabs" role="tablist"><For each={[["general", "General"], ["nozzles", `Nozzles · ${p.e.nozzles.length}`], ["datasheet", "Datasheet"]] as [typeof eqTab extends () => infer T ? T : never, string][]}>{([k, l]) => <button role="tab" classList={{ on: eqTab() === k }} onClick={() => setEqTab(k)}>{l}</button>}</For></nav>
    <Show when={eqTab() === "general"}>
    <Row l="Tag"><input value={p.e.tag} onChange={e => set({ tag: e.currentTarget.value })} /></Row>
    <p class="muted iwhat">{EQUIP.find(x => x[0] === p.e.kind)?.[1]} · DEXPI class {DEXPI[p.e.kind]}</p>
    <h4>Position <Help k="move" /></h4>
    <V3In v={p.e.pos} set={v => set({ pos: v })} />
    <Row l="Rotation"><NumIn q="angle" v={p.e.rot} set={v => set({ rot: v })} /></Row>
    <h4>Sizes <Help k="equipment" /></h4>
    <For each={fields()}>{([k, l]) => <Row l={l}><NumIn q="coord" v={(p.e.dims as any)[k]} min={0.01} set={v => set({ dims: { ...p.e.dims, [k]: v } })} /></Row>}</For>
    <p class="ibtns"><button class="btn sm" title="Keep this item (type, sizes, nozzles, datasheet) as a template to place again in any plant, from Parts › My templates" onClick={() => saveTemplate(p.e)}>Save as a template…</button></p>
    </Show>
    <Show when={eqTab() === "nozzles"}>
    <h4>Nozzles <Help k="nozzles" /></h4>
    <table class="ntab"><thead><tr><th>Tag</th><th>Size</th><th>Class</th><th>Facing</th><th /></tr></thead><tbody>
      <For each={p.e.nozzles}>{(n, i) => <>
        <tr classList={{ on: sel()?.kind === "nozzle" && (sel() as any).tag === n.tag }} onClick={() => setSel({ kind: "nozzle", id: p.e.id, tag: n.tag })}>
          <td><input value={n.tag} onChange={e => setN(i(), { tag: e.currentTarget.value })} /></td>
          <td><SizeSel v={n.nps} set={s => setN(i(), { nps: s })} /></td>
          <td><select onChange={e => setN(i(), { rating: +e.currentTarget.value })}><For each={RATINGS}>{r => <option value={r} selected={n.rating === r}>{r}</option>}</For></select></td>
          <td>{n.facing}</td>
          <td><button class="x" title="Remove the nozzle" onClick={e => { e.stopPropagation(); set({ nozzles: p.e.nozzles.filter((_, j) => j !== i()) }); }}>×</button></td>
        </tr>
        <Show when={sel()?.kind === "nozzle" && (sel() as any).tag === n.tag}><tr class="ndet"><td colSpan={5}>
          <Row l="Service"><input value={n.service} onChange={e => setN(i(), { service: e.currentTarget.value })} /></Row>
          <Row l="Face position (in the item's own axes)"><V3In v={n.pos} set={v => setN(i(), { pos: v })} labels={["X", "Y", "Z"]} /></Row>
          <Row l="Facing direction"><select onChange={e => { const d = dirs.find(x => x[0] === e.currentTarget.value); if (d) setN(i(), { dir: d[1] }); }}><For each={dirs}>{d => <option selected={dirKey(n.dir) === d[0]}>{d[0]}</option>}</For><Show when={dirKey(n.dir) === "Other"}><option selected>Other</option></Show></select></Row>
          <Row l="Projection (shell to face)"><NumIn q="dim" v={n.proj} min={0} set={v => setN(i(), { proj: v })} /></Row>
        </td></tr></Show>
      </>}</For>
    </tbody></table>
    <button class="btn sm" onClick={addN}>Add a nozzle</button>
    </Show>
    <Show when={eqTab() === "datasheet"}>
    <h4>Datasheet</h4>
    <For each={p.e.data}>{([k, v], i) => <div class="dsrow"><input value={k} onChange={e => set({ data: p.e.data.map((x, j) => j === i() ? [e.currentTarget.value, x[1]] : x) })} /><input value={v} onChange={e => set({ data: p.e.data.map((x, j) => j === i() ? [x[0], e.currentTarget.value] : x) })} /><button class="x" onClick={() => set({ data: p.e.data.filter((_, j) => j !== i()) })}>×</button></div>}</For>
    <button class="btn sm" onClick={() => set({ data: [...p.e.data, ["Value", ""]] })}>Add a datasheet line</button>
    </Show>
  </>);
};

const LineInsp = (p: { l: Line }) => {
  const set = (l: Partial<Line>) => act({ SetLine: { ...p.l, ...l } });
  const len = () => (out()?.paths.find(x => x[0] === p.l.id)?.[1] ?? []).reduce((s, q, i, a) => i ? s + Math.hypot(q[0] - a[i - 1][0], q[1] - a[i - 1][1], q[2] - a[i - 1][2]) : 0, 0);
  const issues = () => (out()?.issues ?? []).filter(i => i.line === p.l.id);
  return (<>
    <Row l="Line number" h="lines"><input value={p.l.number} placeholder="e.g. 6&quot;-P-1001-CS1" onChange={e => set({ number: e.currentTarget.value })} /></Row>
    <Show when={project()?.line_list.length}><Row l="From the line list" h="linelist"><select class="fromlist" onChange={e => { const v = e.currentTarget.value; if (v) act({ UseListedLine: { line: p.l.id, number: v } }); }}>
      <option value="">{project()!.line_list.some(r => r.number === p.l.number) ? "On the list" : "Choose a listed line…"}</option>
      <For each={project()!.line_list}>{r => <option value={r.number}>{r.number}{r.nps ? ` · ${r.nps}"` : ""}{r.class ? ` · ${r.class}` : ""}</option>}</For></select></Row></Show>
    <div class="two">
      <Row l="Size" h="resize"><SizeSel v={p.l.nps} set={s => C.resize(s)} /></Row>
      <Row l="Pipe class" h="spec"><select onChange={e => set({ class: e.currentTarget.value })}><For each={catalogue()?.cat.classes ?? []}>{c => <option selected={c.name === p.l.class}>{c.name}</option>}</For><Show when={!(catalogue()?.cat.classes ?? []).some(c => c.name === p.l.class)}><option selected>{p.l.class}</option></Show></select></Row>
      <Row l="Flange rating"><select onChange={e => set({ rating: +e.currentTarget.value })}><For each={RATINGS}>{r => <option value={r} selected={p.l.rating === r}>Class {r}</option>}</For></select></Row>
      <Row l="Fluid code"><input value={p.l.fluid} onChange={e => set({ fluid: e.currentTarget.value })} /></Row>
      <Row l="Insulation" h="insulation"><NumIn q="dim" v={p.l.insulation} min={0} set={v => set({ insulation: v })} /></Row>
      <Row l="Elbows" h="elbows"><select onChange={e => set({ radius: e.currentTarget.value as any })}><option value="long" selected={p.l.radius === "long"}>Long radius</option><option value="short" selected={p.l.radius === "short"}>Short radius</option></select></Row>
    </div>
    <h4>Ends <Help k="ends" /></h4>
    <p class="iend"><span>From</span><b>{endText(p.l.from)}</b></p>
    <p class="iend"><span>To</span><b>{endText(p.l.to)}</b></p>
    <h4>Route <Help k="route" /></h4>
    <p class="muted">{fmt("len", len())} along {p.l.pts.length - 1} run{p.l.pts.length === 2 ? "" : "s"}. Click a run in the 3D view and drag its arrows to move it; the elbows follow.</p>
    <table class="ntab"><thead><tr><th>#</th><th>E</th><th>N</th><th>EL</th></tr></thead><tbody>
      <For each={out()?.paths.find(x => x[0] === p.l.id)?.[1] ?? p.l.pts}>{(q, i) => <tr><td class="muted">{i() + 1}</td><For each={[0, 1, 2]}>{k => <td class="r mono">{toU("coord", q[k]).toFixed(3)}</td>}</For></tr>}</For>
    </tbody></table>
    <h4>Components</h4>
    <For each={p.l.items} fallback={<p class="muted">None yet: drag valves and fittings from the catalogue onto the pipe.</p>}>{it => <button class="ti" onClick={() => setSel({ kind: "item", id: it.id, line: p.l.id })}>{ITEMS.find(x => x[0] === it.kind)?.[1]}<small>run {it.seg + 1} · {fmt("len", it.at)}</small></button>}</For>
    <Show when={issues().length}><h4>Problems</h4><For each={issues()}>{i => <div class="warnline" onClick={() => C.goto(i)}><Icon n="warn" />{i.text}</div>}</For>
      <Show when={issues().some(i => i.kind === "routing")}><p><button class="btn sm pri" onClick={C.reroute}>Route again</button> <span class="muted">square between its ends, through its corners</span></p></Show></Show>
    <p class="ibtns"><button class="btn sm" onClick={() => C.exportPcf(p.l.id)}>Export for Datum Pipe</button><button class="btn sm" onClick={() => C.del()}>Delete the line</button></p>
  </>);
};

const ItemInsp = (p: { l: Line; id: number }) => {
  const it = () => p.l.items.find(i => i.id === p.id);
  return (
    <Show when={it()}>{x => <>
      <p class="iwhat"><b>{x().kind === "reducer" && x().flat && x().flat !== "none" ? `Eccentric reducer, flat ${x().flat === "top" ? "on top" : "at the bottom"}` : ITEMS.find(k => k[0] === x().kind)?.[1]}</b> on <a href="#" onClick={e => { e.preventDefault(); setSel({ kind: "line", id: p.l.id }); }}>{p.l.number || "the line"}</a></p>
      <Row l="Tag"><input value={x().tag} placeholder="e.g. HV-101" onChange={e => act({ SetItem: { line: p.l.id, item: { ...x(), tag: e.currentTarget.value } } })} /></Row>
      <Row l="Distance from the start of its run" h="move"><NumIn q="len" v={x().at} set={v => act({ SetItem: { line: p.l.id, item: { ...x(), at: v } } })} /></Row>
      <Show when={x().kind === "reducer"}><Row l="Size after it"><SizeSel v={x().nps2 || p.l.nps} set={s => act({ SetItem: { line: p.l.id, item: { ...x(), nps2: s } } })} /></Row>
        <Row l="Flat side" h="eccentric"><select class="flatside" onChange={e => act({ SetEccentric: { line: p.l.id, item: x().id, flat: e.currentTarget.value } })}><For each={[["none", "Concentric (no flat side)"], ["top", "Eccentric, flat on top"], ["bottom", "Eccentric, flat at the bottom"]]}>{([k, l]) => <option value={k} selected={(x().flat ?? "none") === k}>{l}</option>}</For></select></Row></Show>
      <Show when={x().kind === "tee" || x().kind === "cross" || x().kind === "weldolet" || x().kind === "sockolet"}>
        <Row l={x().kind === "cross" ? "First branch towards" : "Branch direction"}><select onChange={e => act({ SetItem: { line: p.l.id, item: { ...x(), dir: JSON.parse(e.currentTarget.value) } } })}><For each={[["Up", [0, 0, 1]], ["Down", [0, 0, -1]], ["East", [1, 0, 0]], ["West", [-1, 0, 0]], ["North", [0, 1, 0]], ["South", [0, -1, 0]]] as [string, V3][]}>{([l, d]) => <option value={JSON.stringify(d)} selected={d.every((v, i) => Math.abs(v - x().dir[i]) < 1e-6)}>{l}</option>}</For></select></Row>
        <For each={x().kind === "cross" ? [0, 1] : [0]}>{k => <Show when={!project()?.lines.some(l => [l.from, l.to].some(e => e.kind === "branch" && e.item === x().id && (e.port ?? 0) === k))}>
          <button class="btn sm" onClick={() => C.routeBranch(p.l.id, x().id, k)}>{x().kind === "cross" ? (k === 0 ? "Route the first branch" : "Route the second branch") : "Route the branch"}</button>
        </Show>}</For>
      </Show>
      <Show when={x().kind === "support"}><Row l="Holds" h="supports"><select class="supkind" onChange={e => act({ SetItem: { line: p.l.id, item: { ...x(), support: e.currentTarget.value as SupportKind } } })}><For each={SUPPORTS}>{([k, l]) => <option value={k} selected={x().support === k}>{l}</option>}</For></select></Row></Show>
      <Show when={!["tee", "cross", "weldolet", "sockolet", "reducer", "support"].includes(x().kind)}><Row l="Stem or lever towards"><select onChange={e => act({ SetItem: { line: p.l.id, item: { ...x(), dir: JSON.parse(e.currentTarget.value) } } })}><For each={[["Up", [0, 0, 1]], ["Down", [0, 0, -1]], ["East", [1, 0, 0]], ["West", [-1, 0, 0]], ["North", [0, 1, 0]], ["South", [0, -1, 0]]] as [string, V3][]}>{([l, d]) => <option value={JSON.stringify(d)} selected={d.every((v, i) => Math.abs(v - x().dir[i]) < 1e-6)}>{l}</option>}</For></select></Row></Show>
      <p class="muted">{x().kind === "support" ? "A support takes no length: the pipe is split at it, so Datum Pipe's stress model has a node there." : `It takes the line's size (${inch(p.l.nps)}), class ${p.l.class} and class ${p.l.rating} flanges.`}</p>
      <p class="ibtns"><button class="btn sm" onClick={() => C.del()}>Delete</button></p>
    </>}</Show>
  );
};

const PlotInsp = () => {
  const p = () => project()!;
  const set = (x: Partial<ReturnType<typeof p>["plot"]>) => act({ SetPlot: { ...p().plot, ...x } });
  return (<>
    <Row l="Plant name"><input value={p().name} onChange={e => act({ SetMeta: { name: e.currentTarget.value, notes: p().notes, plant: p().plant, unit: p().unit, units: p().units } })} /></Row>
    <div class="two"><Row l="Plant"><input value={p().plant} onChange={e => act({ SetMeta: { name: p().name, notes: p().notes, plant: e.currentTarget.value, unit: p().unit, units: p().units } })} /></Row>
      <Row l="Unit"><input value={p().unit} onChange={e => act({ SetMeta: { name: p().name, notes: p().notes, plant: p().plant, unit: e.currentTarget.value, units: p().units } })} /></Row></div>
    <h4>Plot <Help k="plot" /></h4>
    <Row l="Grid spacing"><NumIn q="coord" v={p().plot.grid} min={0.001} set={v => set({ grid: v })} /></Row>
    <Row l="East from–to"><div class="two"><NumIn q="coord" v={p().plot.e[0]} set={v => set({ e: [v, p().plot.e[1]] })} /><NumIn q="coord" v={p().plot.e[1]} set={v => set({ e: [p().plot.e[0], v] })} /></div></Row>
    <Row l="North from–to"><div class="two"><NumIn q="coord" v={p().plot.n[0]} set={v => set({ n: [v, p().plot.n[1]] })} /><NumIn q="coord" v={p().plot.n[1]} set={v => set({ n: [p().plot.n[0], v] })} /></div></Row>
    <Row l="Plant north (clockwise from grid north)"><NumIn q="angle" v={p().plot.north} set={v => set({ north: v })} /></Row>
    <Row l="Ground elevation"><NumIn q="coord" v={p().plot.ground} set={v => set({ ground: v })} /></Row>
    <h4>Checks</h4>
    <Row l="Clash clearance" h="clearance"><NumIn q="clear" v={p().settings.clearance} min={0} set={v => act({ SetSettings: { ...p().settings, clearance: v } })} /></Row>
    <h4>Notes</h4>
    <textarea rows="4" style={{ width: "100%" }} value={p().notes} onChange={e => act({ SetMeta: { name: p().name, notes: e.currentTarget.value, plant: p().plant, unit: p().unit, units: p().units } })} />
    <h4>Areas</h4>
    <For each={p().areas} fallback={<p class="muted">Areas divide the plot into boxes (plant › unit › area).</p>}>{a => <button class="ti" onClick={() => setSel({ kind: "area", id: a.id })}>{a.name}<small>{a.unit}</small></button>}</For>
    <button class="btn sm" onClick={C.addArea}>Add an area</button>
  </>);
};

const AreaInsp = (p: { id: number }) => {
  const a = () => project()?.areas.find(x => x.id === p.id);
  return <Show when={a()}>{x => <>
    <Row l="Name"><input value={x().name} onChange={e => act({ SetArea: { ...x(), name: e.currentTarget.value } })} /></Row>
    <Row l="Unit"><input value={x().unit} onChange={e => act({ SetArea: { ...x(), unit: e.currentTarget.value } })} /></Row>
    <Row l="From (south-west, bottom)"><V3In v={x().min} set={v => act({ SetArea: { ...x(), min: v } })} /></Row>
    <Row l="To (north-east, top)"><V3In v={x().max} set={v => act({ SetArea: { ...x(), max: v } })} /></Row>
    <p class="ibtns"><button class="btn sm" onClick={() => C.del()}>Delete the area</button></p>
  </>}</Show>;
};

export const Inspector = () => {
  const title = () => { if (picked().length) return `${picked().length} items`; const s = sel(); if (!s || s.kind === "plot") return project()?.name ?? "Plant"; if (s.kind === "equip" || s.kind === "nozzle") return selEquip()?.tag ?? ""; if (s.kind === "area") return project()?.areas.find(a => a.id === s.id)?.name ?? "Area"; if (s.kind === "item") return selItem() ? ITEMS.find(x => x[0] === selItem()!.kind)?.[1] ?? "" : ""; return selLine()?.number || "Line"; };
  return (
    <aside class="insp">
      <header><b>{title()}</b><Help k="inspector" /><button class="x" title="Close (Esc)" onClick={() => { setSel(null); setPicked([]); }}>×</button></header>
      <div class="ibody">
        <Show when={project() && picked().length}>
          <p class="iwhat"><b>{picked().length} items selected</b></p>
          <p class="muted">{project()!.equipment.filter(e => picked().includes(e.id)).map(e => e.tag).concat(project()!.lines.filter(l => picked().includes(l.id)).map(l => l.number || "(line)")).join(", ")}</p>
          <p class="muted">Drag the arrows to move them together; lines connected to them follow. Ctrl+click adds or removes an item.</p>
          <p class="ibtns"><button class="btn sm" onClick={C.copyGroup}>Copy</button><button class="btn sm" onClick={() => C.rotateGroup(90)}>Turn 90°</button><button class="btn sm" onClick={() => C.mirrorGroup(false)}>Mirror E–W</button><button class="btn sm" onClick={() => C.mirrorGroup(true)}>Mirror N–S</button><button class="btn sm" onClick={C.deleteGroup}>Delete</button></p>
        </Show>
        <Show when={project() && !picked().length}>
          <Show when={selEquip()} keyed>{e => <EquipInsp e={e} />}</Show>
          <Show when={(sel()?.kind === "line" || sel()?.kind === "run" || sel()?.kind === "corner") && selLine()} keyed>{l => <LineInsp l={l} />}</Show>
          <Show when={sel()?.kind === "item" && selLine()} keyed>{l => <ItemInsp l={l} id={(sel() as any).id} />}</Show>
          <Show when={sel()?.kind === "area"}><AreaInsp id={(sel() as any).id} /></Show>
          <Show when={!sel() || sel()!.kind === "plot"}><PlotInsp /></Show>
        </Show>
      </div>
    </aside>
  );
};

// ------------------------------------------------------------ the bottom panel
const [lists, setLists] = createSignal<Lists | null>(null);
export const refreshLists = async () => { const l = await call<Lists>("lists"); if (l) setLists(l); };

const IssueRow = (p: { i: Issue }) => (
  <tr class="f" onClick={() => C.goto(p.i)}><td><span class={`sev ${p.i.kind}`}>{({ clash: "Clash", routing: "Routing", open: "Open end", mismatch: "Mismatch", class: "Class", number: "Number", list: "Line list", rule: "Design rule" } as any)[p.i.kind]}</span></td><td>{p.i.text}</td><td class="r mono muted">{p.i.at.map(v => toU("coord", v).toFixed(2)).join(", ")}</td></tr>
);

export const BottomPanel = () => {
  createEffect(on(() => out(), () => { if (bottomOpen() && ["lines", "mto", "equipment", "valves"].includes(bottom())) refreshLists(); }));
  createEffect(on([bottom, bottomOpen], () => { if (bottomOpen()) refreshLists(); }));
  const mto = () => bottom() === "mto";
  const clashes = createMemo(() => (checked() ?? []).filter(i => i.kind === "clash"));
  const others = createMemo(() => checked() ? checked()!.filter(i => i.kind !== "clash") : (out()?.issues ?? []));
  const tabs: [BottomTab, string][] = [["lines", "Lines"], ["mto", "Take-off"], ["equipment", "Equipment"], ["valves", "Valves"], ["clashes", "Clashes"], ["checks", "Checks"]];
  const store = (k: string, v?: string) => { try { if (v === undefined) return localStorage.getItem(k); localStorage.setItem(k, v); } catch { /* private window */ } return null; };
  const [height, setHeight] = createSignal(Math.min(520, Math.max(110, +(store("dl.console.h") ?? 230) || 230)));
  const grip = (e: PointerEvent) => {
    const y0 = e.clientY, h0 = height(), el = e.currentTarget as HTMLElement; el.setPointerCapture(e.pointerId);
    const mv = (m: PointerEvent) => setHeight(Math.min(window.innerHeight * 0.6, Math.max(90, h0 + y0 - m.clientY)));
    const up = () => { el.removeEventListener("pointermove", mv); el.removeEventListener("pointerup", up); store("dl.console.h", String(Math.round(height()))); window.dispatchEvent(new Event("resize")); };
    el.addEventListener("pointermove", mv); el.addEventListener("pointerup", up);
  };
  return (
    <section class="console bpanel" classList={{ open: bottomOpen() }} style={bottomOpen() ? { height: `${height()}px` } : {}}>
      <Show when={bottomOpen()}><div class="cgrip" onPointerDown={grip} title="Drag to make the panel taller or shorter" /></Show>
      <nav class="btabs">
        <For each={tabs}>{([k, l]) => <button classList={{ on: bottomOpen() && bottom() === k }} onClick={() => setBottom(k)}>{l}{k === "clashes" && checked() ? <span class={`bcnt ${clashes().length ? "bad" : "ok"}`}>{clashes().length}</span> : k === "checks" ? <span class={`bcnt ${others().length ? "bad" : "ok"}`}>{others().length}</span> : null}</button>}</For>
        <span class="grow" />
        <Show when={bottomOpen()}>
          <Show when={bottom() === "lines" || bottom() === "mto"}><button class="btn sm" onClick={() => C.exportCsv(mto() ? "mto" : "lines")}>CSV</button></Show>
          <Show when={bottom() === "equipment"}><button class="btn sm" onClick={() => C.exportCsv("equipment")}>CSV</button></Show>
          <Show when={bottom() === "valves"}><button class="btn sm" onClick={() => C.exportCsv("valves")}>CSV</button></Show>
          <Show when={bottom() === "clashes" || bottom() === "checks"}><Show when={checkStale()}><span class="muted">The plant changed since the check.</span></Show><button class="btn sm pri" onClick={C.check}>Check now</button></Show>
        </Show>
        <button class="cchev" onClick={() => { setBottomOpen(!bottomOpen()); window.dispatchEvent(new Event("resize")); }} title={bottomOpen() ? "Hide the panel" : "Show the panel"} aria-label={bottomOpen() ? "Hide the panel" : "Show the panel"}>{bottomOpen() ? "▾" : "▴"}</button>
      </nav>
      <Show when={bottomOpen()}>
      <div class="bbody">
        <Show when={bottom() === "lines"}>
          <table class="tbl lst"><thead><tr><th>Line number</th><th>From</th><th>To</th><th>Size</th><th>Class</th><th>Fluid</th><th class="r">Length ({unit("len")})</th><th class="r">Components</th><th class="r" title="Empty (dry) weight of pipe, fittings, flanges and valves that have a weight in the catalogue">Weight ({unit("mass")})</th></tr></thead><tbody>
            <For each={lists()?.lines ?? []} fallback={<tr><td colSpan={9} class="muted">No lines yet: Piping › Route Line, then click a nozzle and the corners.</td></tr>}>{r => <tr class="f" classList={{ on: selLine()?.id === r.id }} onClick={() => setSel({ kind: "line", id: r.id })}><td>{r.number || <i class="muted">no number</i>}</td><td>{r.from}</td><td>{r.to}</td><td>{inch(r.nps)}</td><td>{r.class} / {r.rating}</td><td>{r.fluid}</td><td class="r mono">{toU("len", r.length).toFixed(2)}</td><td class="r mono">{r.components}</td><td class="r mono" title={r.unweighed ? `${r.unweighed} part${r.unweighed === 1 ? "" : "s"} (gaskets, bolts, caps or valves without a catalogue weight) not included` : "Every part included"}>{toU("mass", r.weight).toFixed(0)}{r.unweighed ? "+" : ""}</td></tr>}</For>
          </tbody></table>
        </Show>
        <Show when={bottom() === "mto"}>
          <table class="tbl lst"><thead><tr><th>Line</th><th>Group</th><th>Description</th><th>Size</th><th class="r">Quantity</th><th>Unit</th><th class="r">Weight ({unit("mass")})</th></tr></thead><tbody>
            <For each={lists()?.mto ?? []}>{r => <tr><td>{r.line}</td><td>{r.group}</td><td>{r.what}</td><td>{inch(r.nps)}</td><td class="r mono">{r.unit === "m" ? toU("len", r.qty).toFixed(2) : r.qty}</td><td>{r.unit === "m" ? unit("len") : r.unit}</td><td class="r mono" title={r.est ? "Estimated: scaled by the wall from the nearest listed wall" : r.kg == null ? "No catalogue weight" : ""}>{r.kg == null ? "—" : `${r.est ? "≈ " : ""}${toU("mass", r.kg).toFixed(1)}`}</td></tr>}</For>
          </tbody></table>
        </Show>
        <Show when={bottom() === "equipment"}>
          <table class="tbl lst"><thead><tr><th>Tag</th><th>Type</th><th>Nozzle</th><th>Size</th><th>Class</th><th>Service</th><th class="r">E</th><th class="r">N</th><th class="r">EL</th><th>Line</th></tr></thead><tbody>
            <For each={lists()?.equipment ?? []} fallback={<tr><td colSpan={10} class="muted">No equipment yet: drag it from the catalogue onto the plot.</td></tr>}>{e => <For each={e.nozzles.length ? e.nozzles : [null]}>{(n, i) => <tr class="f" classList={{ on: selEquip()?.id === e.id }} onClick={() => setSel(n ? { kind: "nozzle", id: e.id, tag: n.tag } : { kind: "equip", id: e.id })}>
              <td>{i() === 0 ? <b>{e.tag}</b> : ""}</td><td>{i() === 0 ? e.kind : ""}</td><td>{n?.tag}</td><td>{n ? inch(n.nps) : ""}</td><td>{n?.rating}</td><td>{n?.service}</td>
              <For each={[0, 1, 2]}>{k => <td class="r mono">{toU("coord", (n ? n.at : e.at)[k]).toFixed(3)}</td>}</For><td>{n?.line}</td></tr>}</For>}</For>
          </tbody></table>
        </Show>
        <Show when={bottom() === "valves"}>
          <table class="tbl lst"><thead><tr><th>Tag</th><th>Type</th><th>Size</th><th>Class</th><th>Line</th><th class="r">E</th><th class="r">N</th><th class="r">EL</th><th>Stem</th></tr></thead><tbody>
            <For each={lists()?.valves ?? []} fallback={<tr><td colSpan={9} class="muted">No valves yet: drag them from the catalogue onto a pipe.</td></tr>}>{v => <tr class="f" classList={{ on: sel()?.kind === "item" && (sel() as { id: number }).id === v.item }} onClick={() => setSel({ kind: "item", id: v.item, line: v.line_id })}>
              <td>{v.tag || <i class="muted">no tag</i>}</td><td>{({ gate: "Gate", globe: "Globe", check: "Check (swing)", ball: "Ball", butterfly: "Butterfly", strainer: "Strainer (Y)" } as Record<string, string>)[v.kind] ?? v.kind}</td><td>{inch(v.nps)}</td><td>{v.rating}</td><td>{v.line}</td>
              <For each={[0, 1, 2]}>{k => <td class="r mono">{toU("coord", v.at[k]).toFixed(3)}</td>}</For><td>{v.stem}</td></tr>}</For>
          </tbody></table>
        </Show>
        <Show when={bottom() === "clashes"}>
          <Show when={checked()} fallback={<p class="bempty">Check › Check (F5) looks for pipes closer than {fmt("clear", project()?.settings.clearance ?? 0.025)} to each other, to equipment and to the ground.</p>}>
            <table class="tbl lst"><tbody><For each={clashes()} fallback={<tr><td class="okline"><Icon n="ok" />No clashes.</td></tr>}>{i => <IssueRow i={i} />}</For></tbody></table>
          </Show>
        </Show>
        <Show when={bottom() === "checks"}>
          <table class="tbl lst"><tbody><For each={others()} fallback={<tr><td class="okline"><Icon n="ok" />No open ends, mismatches, class problems or lines without a number.</td></tr>}>{i => <IssueRow i={i} />}</For></tbody></table>
        </Show>
      </div>
      </Show>
    </section>
  );
};
export { setFocus };
