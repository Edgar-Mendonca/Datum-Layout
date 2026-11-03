// The side and bottom panels: the catalogue on the left (search, categories, drag source), the
// Inspector on the right (whatever is selected), and the bottom tabs (Lines, Equipment, Clashes,
// Checks), each issue clickable.
import { For, Show, createSignal, createResource, createMemo, createEffect, on, JSX } from "solid-js";
import { CatalogueOut, EQUIP, ITEMS, NPS, inch, Equipment, Nozzle, Line, V3, Lists, Issue, EquipKind } from "./api";
import { call, project, out, sel, setSel, selLine, selEquip, selItem, act, toast, bottom, setBottom, checked, checkStale, setFocus } from "./store";
import { C } from "./cmds";
import { Help } from "./help";
import { Icon } from "./ui";
import { fmt, toU, fromU, unit, parseQ, Q } from "./units";

// ------------------------------------------------------------ the catalogue
export const [catalogue, { refetch: refetchCatalogue }] = createResource(() => call<CatalogueOut>("catalogue"));

const dragStart = (type: "item" | "equip", kind: string) => (e: DragEvent) => {
  (window as any).__drag = { type, kind };
  e.dataTransfer?.setData("text/plain", `${type}:${kind}`);
  if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
};

export const CataloguePanel = () => {
  const [q, setQ] = createSignal("");
  const has = (s: string) => s.toLowerCase().includes(q().toLowerCase());
  const Tile = (p: { type: "item" | "equip"; kind: string; label: string; icon: string; hint: string; run: () => void }) => (
    <Show when={has(p.label)}>
      <button class="cattile" draggable="true" onDragStart={dragStart(p.type, p.kind)} onDragEnd={() => { (window as any).__drag = undefined; }} onClick={p.run} title={p.hint}>
        <Icon n={p.icon} size={26} /><span>{p.label}</span>
      </button>
    </Show>
  );
  const addPack = async () => { const w = await call<string[] | null>("add_pack"); if (w) { refetchCatalogue(); toast(w.length ? `Pack added with ${w.length} warning(s): ${w[0]}` : "Pack added to the catalogue.", !w.length); } };
  return (
    <aside class="catp">
      <header>Catalogue <Help k="catalogue" /></header>
      <div class="catbody">
        <input class="catsearch" placeholder="Search the catalogue" value={q()} onInput={e => setQ(e.currentTarget.value)} />
        <h4>Equipment</h4>
        <div class="catgrid"><For each={EQUIP}>{([k, l, i]) => <Tile type="equip" kind={k} label={l} icon={i} hint={`${l}: drag onto the plot, or click and then click the plot`} run={() => C.place(k)} />}</For></div>
        <h4>Valves</h4>
        <div class="catgrid"><For each={ITEMS.slice(0, 5)}>{([k, l, i]) => <Tile type="item" kind={k} label={l} icon={i} hint={`${l}: drag onto a pipe; it snaps, takes the line's size and class and splits the pipe`} run={() => C.dropOnSelected(k)} />}</For></div>
        <h4>Fittings</h4>
        <div class="catgrid"><For each={ITEMS.slice(5)}>{([k, l, i]) => <Tile type="item" kind={k} label={l} icon={i} hint={`${l}: drag onto a pipe`} run={() => C.dropOnSelected(k)} />}</For></div>
        <Show when={!q()}>
          <h4>Pipe classes <Help k="spec" /></h4>
          <For each={catalogue()?.cat.classes ?? []}>{c => <div class="catclass"><b>{c.name}</b><span>{c.description}</span><small>{inch(c.sizes[0] ?? "")}–{inch(c.sizes[c.sizes.length - 1] ?? "")} · class {c.ratings.join(", ")}</small></div>}</For>
          <h4>Plug-in packs <Help k="packs" /></h4>
          <For each={catalogue()?.cat.packs ?? []} fallback={<p class="muted">Company pipe classes and extra parts, as a folder of CSV and JSON files.</p>}>{p => <div class="catpack"><span>{p}</span><button class="x" title="Remove the pack" onClick={async () => { await call("remove_pack", { name: p }); refetchCatalogue(); }}>×</button></div>}</For>
          <Show when={catalogue()?.cat.warnings.length}><details class="catwarn"><summary>{catalogue()!.cat.warnings.length} row(s) not used</summary><For each={catalogue()!.cat.warnings}>{w => <p>{w}</p>}</For></details></Show>
          <button class="btn sm" onClick={addPack}>Add a pack…</button>
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

const EquipInsp = (p: { e: Equipment }) => {
  const fields = () => catalogue()?.fields.find(f => f[0] === p.e.kind)?.[2] ?? [];
  const set = (e: Partial<Equipment>) => act({ SetEquipment: { ...p.e, ...e } });
  const setN = (i: number, n: Partial<Nozzle>) => set({ nozzles: p.e.nozzles.map((x, j) => j === i ? { ...x, ...n } : x) });
  const addN = () => { let k = p.e.nozzles.length + 1; while (p.e.nozzles.some(n => n.tag === `N${k}`)) k++; set({ nozzles: [...p.e.nozzles, { tag: `N${k}`, nps: "2", rating: 150, facing: "RF", pos: [0, 0, 1], dir: [1, 0, 0], proj: 0.2, service: "" }] }); };
  const dirs: [string, V3][] = [["East (+E)", [1, 0, 0]], ["West (−E)", [-1, 0, 0]], ["North (+N)", [0, 1, 0]], ["South (−N)", [0, -1, 0]], ["Up", [0, 0, 1]], ["Down", [0, 0, -1]]];
  const dirKey = (d: V3) => dirs.find(x => x[1].every((v, i) => Math.abs(v - d[i]) < 1e-6))?.[0] ?? "Other";
  return (<>
    <Row l="Tag"><input value={p.e.tag} onChange={e => set({ tag: e.currentTarget.value })} /></Row>
    <p class="muted iwhat">{EQUIP.find(x => x[0] === p.e.kind)?.[1]} · DEXPI class {({ vertical_vessel: "Vessel", horizontal_drum: "Vessel", tank: "Tank", column: "ProcessColumn", pump: "CentrifugalPump", exchanger: "TubularHeatExchanger" } as Record<EquipKind, string>)[p.e.kind]}</p>
    <h4>Position <Help k="move" /></h4>
    <V3In v={p.e.pos} set={v => set({ pos: v })} />
    <Row l="Rotation"><NumIn q="angle" v={p.e.rot} set={v => set({ rot: v })} /></Row>
    <h4>Sizes <Help k="equipment" /></h4>
    <For each={fields()}>{([k, l]) => <Row l={l}><NumIn q="coord" v={(p.e.dims as any)[k]} min={0.01} set={v => set({ dims: { ...p.e.dims, [k]: v } })} /></Row>}</For>
    <h4>Nozzles <Help k="nozzles" /></h4>
    <table class="ntab"><thead><tr><th>Tag</th><th>Size</th><th>Class</th><th>Facing</th><th /></tr></thead><tbody>
      <For each={p.e.nozzles}>{(n, i) => <>
        <tr classList={{ on: sel()?.kind === "nozzle" && (sel() as any).tag === n.tag }} onClick={() => setSel({ kind: "nozzle", id: p.e.id, tag: n.tag })}>
          <td><input value={n.tag} onChange={e => setN(i(), { tag: e.currentTarget.value })} /></td>
          <td><SizeSel v={n.nps} set={s => setN(i(), { nps: s })} /></td>
          <td><select onChange={e => setN(i(), { rating: +e.currentTarget.value })}><For each={[150, 300]}>{r => <option value={r} selected={n.rating === r}>{r}</option>}</For></select></td>
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
    <h4>Datasheet</h4>
    <For each={p.e.data}>{([k, v], i) => <div class="dsrow"><input value={k} onChange={e => set({ data: p.e.data.map((x, j) => j === i() ? [e.currentTarget.value, x[1]] : x) })} /><input value={v} onChange={e => set({ data: p.e.data.map((x, j) => j === i() ? [x[0], e.currentTarget.value] : x) })} /><button class="x" onClick={() => set({ data: p.e.data.filter((_, j) => j !== i()) })}>×</button></div>}</For>
    <button class="btn sm" onClick={() => set({ data: [...p.e.data, ["Value", ""]] })}>Add a datasheet line</button>
  </>);
};

const LineInsp = (p: { l: Line }) => {
  const set = (l: Partial<Line>) => act({ SetLine: { ...p.l, ...l } });
  const len = () => (out()?.paths.find(x => x[0] === p.l.id)?.[1] ?? []).reduce((s, q, i, a) => i ? s + Math.hypot(q[0] - a[i - 1][0], q[1] - a[i - 1][1], q[2] - a[i - 1][2]) : 0, 0);
  const issues = () => (out()?.issues ?? []).filter(i => i.line === p.l.id);
  return (<>
    <Row l="Line number" h="lines"><input value={p.l.number} placeholder="e.g. 6&quot;-P-1001-CS1" onChange={e => set({ number: e.currentTarget.value })} /></Row>
    <div class="two">
      <Row l="Size" h="resize"><SizeSel v={p.l.nps} set={s => C.resize(s)} /></Row>
      <Row l="Pipe class" h="spec"><select onChange={e => set({ class: e.currentTarget.value })}><For each={catalogue()?.cat.classes ?? []}>{c => <option selected={c.name === p.l.class}>{c.name}</option>}</For><Show when={!(catalogue()?.cat.classes ?? []).some(c => c.name === p.l.class)}><option selected>{p.l.class}</option></Show></select></Row>
      <Row l="Flange rating"><select onChange={e => set({ rating: +e.currentTarget.value })}><For each={[150, 300]}>{r => <option value={r} selected={p.l.rating === r}>Class {r}</option>}</For></select></Row>
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
      <p class="iwhat"><b>{ITEMS.find(k => k[0] === x().kind)?.[1]}</b> on <a href="#" onClick={e => { e.preventDefault(); setSel({ kind: "line", id: p.l.id }); }}>{p.l.number || "the line"}</a></p>
      <Row l="Tag"><input value={x().tag} placeholder="e.g. HV-101" onChange={e => act({ SetItem: { line: p.l.id, item: { ...x(), tag: e.currentTarget.value } } })} /></Row>
      <Row l="Distance from the start of its run" h="move"><NumIn q="len" v={x().at} set={v => act({ SetItem: { line: p.l.id, item: { ...x(), at: v } } })} /></Row>
      <Show when={x().kind === "reducer"}><Row l="Size after it"><SizeSel v={x().nps2 || p.l.nps} set={s => act({ SetItem: { line: p.l.id, item: { ...x(), nps2: s } } })} /></Row></Show>
      <Show when={x().kind === "tee"}>
        <Row l="Branch direction"><select onChange={e => act({ SetItem: { line: p.l.id, item: { ...x(), dir: JSON.parse(e.currentTarget.value) } } })}><For each={[["Up", [0, 0, 1]], ["Down", [0, 0, -1]], ["East", [1, 0, 0]], ["West", [-1, 0, 0]], ["North", [0, 1, 0]], ["South", [0, -1, 0]]] as [string, V3][]}>{([l, d]) => <option value={JSON.stringify(d)} selected={d.every((v, i) => Math.abs(v - x().dir[i]) < 1e-6)}>{l}</option>}</For></select></Row>
        <Show when={!project()?.lines.some(l => (l.from.kind === "branch" && l.from.item === x().id) || (l.to.kind === "branch" && l.to.item === x().id))}>
          <button class="btn sm" onClick={() => C.routeBranch(p.l.id, x().id)}>Route the branch</button>
        </Show>
      </Show>
      <Show when={x().kind !== "tee" && x().kind !== "reducer"}><Row l="Stem or lever towards"><select onChange={e => act({ SetItem: { line: p.l.id, item: { ...x(), dir: JSON.parse(e.currentTarget.value) } } })}><For each={[["Up", [0, 0, 1]], ["Down", [0, 0, -1]], ["East", [1, 0, 0]], ["West", [-1, 0, 0]], ["North", [0, 1, 0]], ["South", [0, -1, 0]]] as [string, V3][]}>{([l, d]) => <option value={JSON.stringify(d)} selected={d.every((v, i) => Math.abs(v - x().dir[i]) < 1e-6)}>{l}</option>}</For></select></Row></Show>
      <p class="muted">It takes the line's size ({inch(p.l.nps)}), class {p.l.class} and class {p.l.rating} flanges.</p>
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
  const title = () => { const s = sel(); if (!s || s.kind === "plot") return project()?.name ?? "Plant"; if (s.kind === "equip" || s.kind === "nozzle") return selEquip()?.tag ?? ""; if (s.kind === "area") return project()?.areas.find(a => a.id === s.id)?.name ?? "Area"; if (s.kind === "item") return selItem() ? ITEMS.find(x => x[0] === selItem()!.kind)?.[1] ?? "" : ""; return selLine()?.number || "Line"; };
  return (
    <aside class="insp">
      <header>Inspector <b>{title()}</b><Help k="inspector" /></header>
      <div class="ibody">
        <Show when={project()}>
          <Show when={selEquip()} keyed>{e => <EquipInsp e={e} />}</Show>
          <Show when={(sel()?.kind === "line" || sel()?.kind === "run") && selLine()} keyed>{l => <LineInsp l={l} />}</Show>
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
  <tr class="f" onClick={() => C.goto(p.i)}><td><span class={`sev ${p.i.kind}`}>{({ clash: "Clash", routing: "Routing", open: "Open end", mismatch: "Mismatch", class: "Class", number: "Number" } as any)[p.i.kind]}</span></td><td>{p.i.text}</td><td class="r mono muted">{p.i.at.map(v => toU("coord", v).toFixed(2)).join(", ")}</td></tr>
);

export const BottomPanel = () => {
  const [mto, setMto] = createSignal(false);
  createEffect(on(() => out(), () => { if (bottom() === "lines" || bottom() === "equipment") refreshLists(); }));
  createEffect(on(bottom, () => refreshLists()));
  const clashes = createMemo(() => (checked() ?? []).filter(i => i.kind === "clash"));
  const others = createMemo(() => checked() ? checked()!.filter(i => i.kind !== "clash") : (out()?.issues ?? []));
  const tabs: [typeof bottom extends () => infer T ? T : never, string][] = [["lines", "Lines"], ["equipment", "Equipment"], ["clashes", "Clashes"], ["checks", "Checks"]];
  return (
    <section class="bpanel">
      <nav class="btabs">
        <For each={tabs}>{([k, l]) => <button classList={{ on: bottom() === k }} onClick={() => setBottom(k)}>{l}{k === "clashes" && checked() ? <span class={`bcnt ${clashes().length ? "bad" : "ok"}`}>{clashes().length}</span> : k === "checks" ? <span class={`bcnt ${others().length ? "bad" : "ok"}`}>{others().length}</span> : null}</button>}</For>
        <span class="grow" />
        <Show when={bottom() === "lines"}><label class="chk"><input type="checkbox" checked={mto()} onChange={e => setMto(e.currentTarget.checked)} />Material take-off</label><button class="btn sm" onClick={() => C.exportCsv(mto() ? "mto" : "lines")}>CSV</button></Show>
        <Show when={bottom() === "equipment"}><button class="btn sm" onClick={() => C.exportCsv("equipment")}>CSV</button></Show>
        <Show when={bottom() === "clashes" || bottom() === "checks"}><Show when={checkStale()}><span class="muted">The plant changed since the check.</span></Show><button class="btn sm pri" onClick={C.check}>Check now</button></Show>
      </nav>
      <div class="bbody">
        <Show when={bottom() === "lines" && !mto()}>
          <table class="tbl lst"><thead><tr><th>Line number</th><th>From</th><th>To</th><th>Size</th><th>Class</th><th>Fluid</th><th class="r">Length ({unit("len")})</th><th class="r">Components</th></tr></thead><tbody>
            <For each={lists()?.lines ?? []} fallback={<tr><td colSpan={8} class="muted">No lines yet: Piping › Route Line, then click a nozzle and the corners.</td></tr>}>{r => <tr class="f" classList={{ on: selLine()?.id === r.id }} onClick={() => setSel({ kind: "line", id: r.id })}><td>{r.number || <i class="muted">no number</i>}</td><td>{r.from}</td><td>{r.to}</td><td>{inch(r.nps)}</td><td>{r.class} / {r.rating}</td><td>{r.fluid}</td><td class="r mono">{toU("len", r.length).toFixed(2)}</td><td class="r mono">{r.components}</td></tr>}</For>
          </tbody></table>
        </Show>
        <Show when={bottom() === "lines" && mto()}>
          <table class="tbl lst"><thead><tr><th>Line</th><th>Group</th><th>Description</th><th>Size</th><th class="r">Quantity</th><th>Unit</th></tr></thead><tbody>
            <For each={lists()?.mto ?? []}>{r => <tr><td>{r.line}</td><td>{r.group}</td><td>{r.what}</td><td>{inch(r.nps)}</td><td class="r mono">{r.unit === "m" ? toU("len", r.qty).toFixed(2) : r.qty}</td><td>{r.unit === "m" ? unit("len") : r.unit}</td></tr>}</For>
          </tbody></table>
        </Show>
        <Show when={bottom() === "equipment"}>
          <table class="tbl lst"><thead><tr><th>Tag</th><th>Type</th><th>Nozzle</th><th>Size</th><th>Class</th><th>Service</th><th class="r">E</th><th class="r">N</th><th class="r">EL</th><th>Line</th></tr></thead><tbody>
            <For each={lists()?.equipment ?? []} fallback={<tr><td colSpan={10} class="muted">No equipment yet: drag it from the catalogue onto the plot.</td></tr>}>{e => <For each={e.nozzles.length ? e.nozzles : [null]}>{(n, i) => <tr class="f" classList={{ on: selEquip()?.id === e.id }} onClick={() => setSel(n ? { kind: "nozzle", id: e.id, tag: n.tag } : { kind: "equip", id: e.id })}>
              <td>{i() === 0 ? <b>{e.tag}</b> : ""}</td><td>{i() === 0 ? e.kind : ""}</td><td>{n?.tag}</td><td>{n ? inch(n.nps) : ""}</td><td>{n?.rating}</td><td>{n?.service}</td>
              <For each={[0, 1, 2]}>{k => <td class="r mono">{toU("coord", (n ? n.at : e.at)[k]).toFixed(3)}</td>}</For><td>{n?.line}</td></tr>}</For>}</For>
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
    </section>
  );
};
export { setFocus };
