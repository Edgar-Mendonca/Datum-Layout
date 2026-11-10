// The workspace around the 3D view, after Datum Mesh's shell: the steps bar at the far left (the
// order a plant is laid out in, each step done or still to do), the plant tree (every item of the
// plant in that order, with counts, marks, an eye to show or hide each, a filter and a menu on the
// right button), the context bar under the ribbon (the settings used most, whatever the tab) and
// the tool strip beside the tree (the open tab's most used tools). Nothing here calculates: every
// fact comes from the engine's output.
import { For, Show, JSX, createSignal, createEffect } from "solid-js";
import { project, out, sel, setSel, picked, setPicked, tab, setTab, mode, checked, checkStale, hidden, setHidden, isolated, setIsolated, colourBy, setColourBy, ColourBy, section, setBottom, openMenu, act, elev, setElev, setBackstage, setFocus, setOpts, opts } from "./store";
import { C } from "./cmds";
import { Icon } from "./ui";
import { Help } from "./help";
import { EQUIP, ITEMS, NPS, RATINGS, SUPPORTS, inch, Issue } from "./api";
import { catalogue, CataloguePanel } from "./panels";
import { fmt, toU, fromU, unit } from "./units";

const Ic = (p: { n: string }) => <svg class="tic"><use href={`#i-${p.n}`} /></svg>;
const plural = (n: number, one: string, many = one + "s") => `${n} ${n === 1 ? one : many}`;
const EQ_ICON = Object.fromEntries(EQUIP.map(([k, , i]) => [k, i])) as Record<string, string>;
const EQ_NAME = Object.fromEntries(EQUIP.map(([k, l]) => [k, l])) as Record<string, string>;
const IT_ICON = Object.fromEntries(ITEMS.map(([k, , i]) => [k, i])) as Record<string, string>;
const IT_NAME = Object.fromEntries(ITEMS.map(([k, l]) => [k, l.replace(/ \(.*\)$/, "")])) as Record<string, string>;
const IT_GROUP: Record<string, string> = { gate: "Gate valves", globe: "Globe valves", check: "Check valves", ball: "Ball valves", butterfly: "Butterfly valves", strainer: "Strainers", flange_pair: "Flange pairs", reducer: "Reducers", tee: "Tees", cross: "Crosses", weldolet: "Weldolets", sockolet: "Sockolets", support: "Pipe supports" };

// ------------------------------------------------------------------ the steps
/** The ribbon tab of each step. */
export const STEP_TABS = ["view", "equipment", "piping", "fittings", "check", "lists"];
export type FlowStep = { n: number; t: string; short: string; icon: string; done: boolean; why: string };
/** The six steps of laying out a plant: one source for the steps bar and the tree. */
export const flowSteps = (): FlowStep[] => {
  const p = project(); if (!p) return [];
  const iss = out()?.issues ?? [];
  const routing = iss.filter(i => i.kind === "routing").length;
  const items = p.lines.reduce((s, l) => s + l.items.length, 0);
  const ck = checked();
  return [
    { n: 1, t: "Plot", short: "Plot", icon: "grid", done: p.equipment.length > 0 || p.areas.length > 0, why: "Set the grid and areas (View › Plot)" },
    { n: 2, t: "Equipment", short: "Equip", icon: "vvessel", done: p.equipment.length > 0, why: "Drag equipment from the catalogue onto the plot" },
    { n: 3, t: "Lines", short: "Lines", icon: "route", done: p.lines.length > 0 && !routing, why: p.lines.length ? `${plural(routing, "routing problem")} to fix` : "Route a line from a nozzle (R)" },
    { n: 4, t: "Valves and fittings", short: "Fittings", icon: "valve", done: items > 0, why: "Drag valves and fittings onto the pipes" },
    { n: 5, t: "Check", short: "Check", icon: "clash", done: !!ck && !checkStale() && !ck.length, why: !ck ? "Check clashes and errors (F5)" : checkStale() ? "Changed since the check: check again (F5)" : `${plural(ck.length, "finding")} to look at` },
    { n: 6, t: "Lists and hand-over", short: "Hand over", icon: "pipe", done: false, why: "Line list, take-off, PCF for Datum Pipe, glTF" },
  ];
};
export const goStep = (i: number) => {
  setTab(STEP_TABS[i]); setTreeTab("plant");
  requestAnimationFrame(() => document.querySelectorAll(".tree .step")[i]?.scrollIntoView({ block: "nearest", behavior: "smooth" }));
};

/** The steps bar at the far left: the steps in order, then the tools any step may need. */
export const Flow = () => (
  <nav class="flow" aria-label="Steps of the work">
    <span class="fcap">Steps</span>
    <For each={flowSteps()}>{(f, i) => (
      <button classList={{ on: tab() === STEP_TABS[i()], done: f.done }} onClick={() => goStep(i())} title={`${f.n}. ${f.t}${f.done ? ": done" : `: ${f.why}`}`}>
        <i class="fn">{f.n}</i><Ic n={f.icon} /><span>{f.short}</span>
      </button>)}</For>
    <span class="fgrow" />
    <span class="fcap">Tools</span>
    <button class="fextra" data-tool="catalogue" classList={{ on: treeTab() === "catalogue" }} title="The catalogue: equipment, valves, fittings, pipe classes and packs" onClick={() => setTreeTab(treeTab() === "catalogue" ? "plant" : "catalogue")}><Ic n="list" /><span>Catalogue</span></button>
    <button class="fextra" data-tool="library" title="The library: every pipe class and parts table, with its source" onClick={() => import("./library").then(k => k.library())}><Ic n="library" /><span>Library</span></button>
    <button class="fextra" title="Options" onClick={() => setBackstage("options")}><Ic n="opts" /><span>Options</span></button>
  </nav>
);

// ------------------------------------------------------------------ the tree
export const [treeTab, setTreeTab] = createSignal<"plant" | "catalogue">("plant");
type Mark = { ok: boolean; why?: string } | null;
const Branch = (p: { t: string; icon: string; n?: number; note?: string; mark?: Mark; help?: string; add?: () => void; addText?: string; open?: boolean; eye?: number[]; children?: JSX.Element }) => (
  <details class="tgrp" open={p.open ?? (p.n === undefined || p.n <= 15)}>
    <summary title={p.mark && !p.mark.ok ? p.mark.why : undefined}><Ic n={p.icon} /><span class="tt">{p.t}</span><Show when={p.n !== undefined}><span class="pill">{p.n}</span></Show>
      <Show when={p.note}><small class="tnote2">{p.note}</small></Show><span class="grow" />
      <Show when={p.mark}>{mk => <i class={`tmark ${mk().ok ? "ok" : "need"}`} title={mk().ok ? "Done" : mk().why}>{mk().ok ? "✓" : "!"}</i>}</Show>
      {p.help && <Help k={p.help} />}{p.add && <button class="tadd" title={`Add: ${p.t}`} onClick={e => { e.preventDefault(); p.add!(); }}>+</button>}
      <Show when={p.eye?.length}><Eye ids={p.eye!} what={p.t.toLowerCase()} /></Show></summary>
    <div class="tlist">{p.children}<Show when={p.add && p.addText && !p.n}><button class="ti ghost" onClick={() => p.add!()}><Ic n="new" /><span>{p.addText}</span></button></Show></div>
  </details>
);
/** Whether an equipment item or line is drawn. */
const shownId = (id: number) => { const iso = isolated(); return iso ? iso.includes(id) : !hidden().includes(id); };
/** The eye at the end of a row: shows or hides those items in 3D, nothing else. */
const Eye = (p: { ids: number[]; what: string }) => {
  const on = () => p.ids.some(shownId);
  const toggle = () => {
    const iso = isolated();
    if (iso) { setIsolated(on() ? iso.filter(i => !p.ids.includes(i)) : [...new Set([...iso, ...p.ids])]); return; }
    setHidden(on() ? [...new Set([...hidden(), ...p.ids])] : hidden().filter(i => !p.ids.includes(i)));
  };
  return <span class="teye" role="button" tabindex="0" classList={{ off: !on() }} title={on() ? `Hide ${p.what}` : `Show ${p.what}`} aria-label={on() ? `Hide ${p.what}` : `Show ${p.what}`}
    onClick={e => { e.stopPropagation(); e.preventDefault(); toggle(); }} onKeyDown={e => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); toggle(); } }}>
    <svg class="tic"><use href={on() ? "#i-eye" : "#i-eyeoff"} /></svg></span>;
};
/** A row for an equipment item or line: click selects (Ctrl+click adds to the group), right-click
 *  for more, the eye shows or hides it. */
const Leaf = (p: { id: number; kind: "equip" | "line"; label: string; sub?: string; icon: string; bad?: string }) => {
  const on = () => picked().includes(p.id) || ((sel()?.kind === "equip" || sel()?.kind === "nozzle" || sel()?.kind === "line" || sel()?.kind === "run" || sel()?.kind === "corner") && (sel() as { id: number }).id === p.id);
  const pick = (e: MouseEvent) => { if (e.ctrlKey || e.metaKey || e.shiftKey) { C.pick(p.id); return; } setPicked([]); setSel(p.kind === "equip" ? { kind: "equip", id: p.id } : { kind: "line", id: p.id }); };
  const menu = (e: MouseEvent) => { if (!on()) pick(e); openMenu(e, [
    { label: "Zoom to it", icon: "fit", run: () => zoomTo(p.id) },
    { label: shownId(p.id) ? "Hide" : "Show", icon: shownId(p.id) ? "hide" : "showall", keys: "H", run: () => shownId(p.id) ? setHidden([...hidden(), p.id]) : (setHidden(hidden().filter(i => i !== p.id)), isolated() && setIsolated([...isolated()!, p.id])) },
    { label: "Show only this (isolate)", icon: "isolate", keys: "I", run: () => { setHidden([]); setIsolated([p.id]); } },
    "-",
    ...(p.kind === "line" ? [{ label: "Export for Datum Pipe (.pcf)", icon: "export", run: () => C.exportPcf(p.id) }, "-" as const] : []),
    { label: "Delete", icon: "del", keys: "Del", run: () => C.del() },
  ]); };
  return <button class="ti" classList={{ on: on(), dim: !shownId(p.id), bad: !!p.bad }} data-id={p.id} onClick={pick} onDblClick={() => zoomTo(p.id)} onContextMenu={menu} title={p.bad}>
    <Ic n={p.icon} /><span>{p.label}</span>{p.sub && <small>{p.sub}</small>}<Show when={p.bad}><i class="tmark need">!</i></Show><Eye ids={[p.id]} what={p.label} /></button>;
};
const Row2 = (p: { icon: string; label: string; sub?: string; run?: () => void; on?: boolean; dim?: boolean; title?: string }) => (
  <button class="ti" classList={{ on: !!p.on, dim: !!p.dim }} title={p.title} onClick={() => p.run?.()}><Ic n={p.icon} /><span>{p.label}</span>{p.sub && <small>{p.sub}</small>}</button>
);
/** Zooms the view to an item's middle. */
const zoomTo = (id: number) => {
  const p = project(); if (!p) return;
  const e = p.equipment.find(x => x.id === id);
  if (e) return setFocus({ at: [e.pos[0], e.pos[1], e.pos[2] + 1], r: 4 });
  const pts = out()?.paths.find(x => x[0] === id)?.[1]; if (!pts?.length) return;
  const lo = [0, 1, 2].map(k => Math.min(...pts.map(q => q[k]))), hi = [0, 1, 2].map(k => Math.max(...pts.map(q => q[k])));
  setFocus({ at: [(lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, (lo[2] + hi[2]) / 2], r: Math.max(2, Math.hypot(hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]) / 2) });
};
const Step = (p: { n: number; f?: FlowStep; count?: number; eye?: number[]; add?: () => void; children?: JSX.Element }) => (
  <section class="step" classList={{ here: tab() === STEP_TABS[p.n - 1], done: !!p.f?.done }}>
    <div class="sthead" role="button" tabindex="0" onClick={() => setTab(STEP_TABS[p.n - 1])} title={p.f?.done ? `Step ${p.n}: done` : p.f?.why}>
      <b class="stn">{p.f?.done ? "✓" : p.n}</b><Ic n={p.f?.icon ?? "list"} /><span>{p.f?.t}</span>
      <Show when={p.count !== undefined} fallback={<Show when={p.f && !p.f.done && p.n < 6}><small>{p.f!.why}</small></Show>}><small class="cnt">{p.count}</small></Show>
      {p.add && <button class="tadd" title="Add" onClick={e => { e.stopPropagation(); p.add!(); }}>+</button>}
      <Show when={p.eye?.length}><Eye ids={p.eye!} what={`all ${p.f?.t.toLowerCase()}`} /></Show></div>
    <div class="stbody">{p.children}</div>
  </section>
);

/** The tree's filter: rows whose words do not match are hidden, and the branches holding a match open. */
const [treeFilter, setTreeFilter] = createSignal("");
const applyFilter = (body: HTMLElement, q: string) => {
  const t = q.trim().toLowerCase();
  const has = (el: Element | null) => (el?.textContent ?? "").toLowerCase().includes(t);
  const rows = [...body.querySelectorAll<HTMLElement>(".ti")].filter(r => !r.closest("summary"));
  const groups = [...body.querySelectorAll<HTMLElement>("details")];
  rows.forEach(r => { r.style.display = !t || has(r) ? "" : "none"; });
  // Inner groups first: a group shows when its head matches (then all of it) or anything in it does.
  groups.reverse().forEach(d => {
    d.style.display = "";
    if (!t) return;
    const head = has(d.querySelector(":scope > summary"));
    if (head) { d.querySelectorAll<HTMLElement>(".ti, details").forEach(x => { x.style.display = ""; }); return; }
    const inner = [...d.querySelectorAll<HTMLElement>(":scope > .tlist > .ti, :scope > .ti, :scope > .tlist > details, :scope > details")].some(x => x.style.display !== "none");
    d.style.display = inner ? "" : "none";
    if (inner) (d as HTMLDetailsElement).open = true;
  });
};
const KIND_NAME: Record<string, string> = { clash: "Clashes", routing: "Routing", open: "Open ends", mismatch: "Mismatches", class: "Pipe class", number: "Line numbers", list: "Line list", rule: "Design rules" };

export const Tree = () => {
  const p = () => project();
  const fs = flowSteps;
  const issuesOf = (id: number) => (out()?.issues ?? []).filter(i => i.line === id || i.equip === id);
  const findings = (): Issue[] => checked() && !checkStale() ? checked()! : out()?.issues ?? [];
  const kinds = () => { const m = new Map<string, number>(); for (const i of findings()) m.set(i.kind, (m.get(i.kind) ?? 0) + 1); return [...m.entries()]; };
  const items = () => (p()?.lines ?? []).flatMap(l => l.items.map(it => ({ l, it })));
  const listIssues = () => (out()?.issues ?? []).filter(i => i.kind === "list");
  // An empty plant opens on the catalogue; the first item placed brings the plant tree back.
  let wasEmpty = false;
  createEffect(() => { const e = !!p() && !p()!.equipment.length && !p()!.lines.length; if (e && !wasEmpty) setTreeTab("catalogue"); if (!e && wasEmpty) setTreeTab("plant"); wasEmpty = e; });
  return (
    <aside class="tree">
      <div class="ttabs" role="tablist"><button role="tab" classList={{ on: treeTab() === "plant" }} onClick={() => setTreeTab("plant")}>Plant</button><button role="tab" classList={{ on: treeTab() === "catalogue" }} onClick={() => setTreeTab("catalogue")}>Catalogue</button><Help k="tree" /></div>
      <div class="tcat" style={{ display: treeTab() === "catalogue" ? undefined : "none" }}><CataloguePanel /></div>
      <Show when={treeTab() === "plant"}><label class="tfilter"><svg class="ic"><use href="#i-search" /></svg><input type="search" placeholder="Filter the tree" value={treeFilter()} onInput={e => setTreeFilter(e.currentTarget.value)} /></label></Show>
      <div class="tbody" ref={el => createEffect(() => { treeFilter(); project(); setTimeout(() => applyFilter(el, treeFilter()), 0); })} style={{ display: treeTab() === "plant" ? undefined : "none" }}>
        <Show when={p()}>{pp => <>
          <div class="troot" title="The plant's name, plot and units" onClick={() => C.selectPlot()}><img src="logo.svg" alt="" /><b>{pp().name}</b><small>{pp().unit} · {pp().units}</small></div>
          <Step n={1} f={fs()[0]}>
            <Row2 icon="grid" label="Grid and plot" sub={`${fmt("coord", pp().plot.grid)} · N ${pp().plot.north}°`} run={() => C.selectPlot()} on={sel()?.kind === "plot"} />
            <Branch t="Areas" icon="area" n={pp().areas.length} help="plot" add={() => C.addArea()} addText="Add an area (View › Add Area)">
              <For each={pp().areas}>{a => <Row2 icon="area" label={a.name} sub={`${fmt("len", a.max[0] - a.min[0])} × ${fmt("len", a.max[1] - a.min[1])}`} on={sel()?.kind === "area" && (sel() as { id: number }).id === a.id} run={() => { setPicked([]); setSel({ kind: "area", id: a.id }); }} />}</For></Branch>
          </Step>
          <Step n={2} f={fs()[1]} count={pp().equipment.length || undefined} eye={pp().equipment.map(e => e.id)} add={() => setTreeTab("catalogue")}>
            <Show when={!pp().equipment.length}><button class="ti ghost" onClick={() => setTreeTab("catalogue")}><Ic n="new" /><span>Drag equipment from the catalogue</span></button></Show>
              <For each={pp().equipment}>{e => <details class="tsub"><summary><Leaf id={e.id} kind="equip" icon={EQ_ICON[e.kind] ?? "vvessel"} label={e.tag} sub={EQ_NAME[e.kind]} bad={issuesOf(e.id)[0]?.text} /></summary>
                <For each={e.nozzles}>{n => { const l = pp().lines.find(x => (x.from.kind === "nozzle" && x.from.equip === e.id && x.from.nozzle === n.tag) || (x.to.kind === "nozzle" && x.to.equip === e.id && x.to.nozzle === n.tag)); return <Row2 icon="flange" label={`${n.tag}${n.service ? ` · ${n.service}` : ""}`} sub={`${inch(n.nps)} · ${l ? l.number : "free"}`} dim={!l} on={sel()?.kind === "nozzle" && (sel() as { id: number; tag: string }).id === e.id && (sel() as { tag: string }).tag === n.tag} run={() => { setPicked([]); setSel({ kind: "nozzle", id: e.id, tag: n.tag }); }} />; }}</For>
              </details>}</For>
          </Step>
          <Step n={3} f={fs()[2]} count={pp().lines.length || undefined} eye={pp().lines.map(l => l.id)} add={() => C.route()}>
            <Show when={!pp().lines.length}><button class="ti ghost" onClick={() => C.route()}><Ic n="new" /><span>Route a line (R)</span></button></Show>
            <For each={pp().lines}>{l => { const its = () => items().filter(x => x.l.id === l.id); return <Show when={its().length} fallback={<Leaf id={l.id} kind="line" icon="pipe" label={l.number || "(no number)"} sub={`${inch(l.nps)} · ${l.class}`} bad={issuesOf(l.id).find(i => i.kind === "routing")?.text} />}>
              <details class="tsub tline"><summary><Leaf id={l.id} kind="line" icon="pipe" label={l.number || "(no number)"} sub={`${inch(l.nps)} · ${l.class}`} bad={issuesOf(l.id).find(i => i.kind === "routing")?.text} /></summary>
                <For each={its()}>{x => <Row2 icon={IT_ICON[x.it.kind]} label={x.it.tag || (x.it.kind === "support" ? (SUPPORTS.find(s => s[0] === x.it.support)?.[1] ?? "Support") : IT_NAME[x.it.kind])} sub={x.it.tag ? IT_NAME[x.it.kind] : undefined} on={sel()?.kind === "item" && (sel() as { id: number }).id === x.it.id} run={() => { setPicked([]); setSel({ kind: "item", id: x.it.id, line: x.l.id }); }} />}</For>
              </details></Show>; }}</For>
            <Show when={pp().line_list.length}><Branch t="Line list" icon="list" n={pp().line_list.length} help="linelist" open={false} mark={{ ok: !listIssues().length, why: `${plural(listIssues().length, "difference")} with the model` }}>
              <Show when={pp().line_list.some(r => !pp().lines.some(l => l.number === r.number))}><button class="ti ghost" onClick={() => C.routeListed()}><Ic n="route" /><span>Route the listed lines</span></button></Show>
              <For each={pp().line_list}>{r => { const m = pp().lines.find(l => l.number === r.number); return <Row2 icon="list" label={r.number} sub={`${r.nps ? inch(r.nps) : "?"} · ${r.class || "—"}${m ? "" : " · not modelled"}`} dim={!m} title={r.components.length ? `${r.from} → ${r.to}: ${r.components.join(", ")}` : `${r.from} → ${r.to}`} run={() => m && (setPicked([]), setSel({ kind: "line", id: m.id }))} />; }}</For></Branch></Show>
          </Step>
          <Step n={4} f={fs()[3]}>
            {/* 0.2.11: the components sit under their lines (step 3); here, one row per kind with its count. */}
            <For each={ITEMS.map(([k]) => k).filter(k => items().some(x => x.it.kind === k))}>{k => (
              <Row2 icon={IT_ICON[k]} label={IT_GROUP[k] ?? IT_NAME[k]} sub={String(items().filter(x => x.it.kind === k).length)} title={`Show the ${(IT_GROUP[k] ?? IT_NAME[k]).toLowerCase()} under their lines`} run={() => setTreeFilter(IT_NAME[k])} />)}</For>
            <Show when={!items().length}><button class="ti ghost" onClick={() => setTreeTab("catalogue")}><Ic n="valve" /><span>Drag valves and fittings onto a pipe</span></button></Show>
          </Step>
          <Step n={5} f={fs()[4]}>
            <Branch t="Findings" icon="warn" n={findings().length} help="checks" mark={{ ok: !findings().length && !!checked() && !checkStale(), why: checked() ? (checkStale() ? "Changed since the check" : "See the Checks tab") : "Not checked yet (F5)" }}>
              <For each={kinds()}>{([k, n]) => <Row2 icon={k === "clash" ? "clash" : "warn"} label={KIND_NAME[k] ?? k} sub={String(n)} run={() => setBottom(k === "clash" ? "clashes" : "checks")} />}</For>
              <Show when={!findings().length}><Row2 icon="ok" label={checked() && !checkStale() ? "No clashes or errors" : "Check (F5) for clashes"} run={() => C.check()} /></Show></Branch>
          </Step>
          <Step n={6} f={fs()[5]}>
            <Branch t="Saved views" icon="views" n={pp().views.length} help="views" add={() => C.saveView()} addText="Save this view (View › Save View)">
              <For each={pp().views}>{v => <Row2 icon="camera" label={v.name} run={() => C.showView(v.name)} />}</For></Branch>
            <Row2 icon="pipe" label="PCF for Datum Pipe" sub="every line" run={() => C.exportPcf()} />
            <Row2 icon="csv" label="Line list" sub="CSV" run={() => C.exportCsv("lines")} />
            <Row2 icon="mto" label="Material take-off" sub="CSV" run={() => C.exportCsv("mto")} />
            <Row2 icon="valve" label="Valve list" sub="CSV" run={() => C.exportCsv("valves")} />
            <Row2 icon="cube" label="3D model" sub="glTF" run={C.exportGltf} />
          </Step>
        </>}</Show>
      </div>
    </aside>
  );
};

// ------------------------------------------------------------------ the context bar
export const CtxBar = () => {
  const p = () => project();
  const sets = (k: string, v: string | number) => p() && act({ SetSettings: { ...p()!.settings, [k]: v } });
  return <div class="ctxbar">
    <span class="ctxtab" data-ctx="newlines" title="Size, class and rating for new lines"><label>New lines</label>
      <select class="ctxsize" onChange={e => sets("nps", e.currentTarget.value)}><For each={NPS}>{s => <option value={s} selected={p()?.settings.nps === s}>{inch(s)}</option>}</For></select>
      <select class="ctxclass" onChange={e => sets("class", e.currentTarget.value)}><For each={catalogue()?.cat.classes ?? []}>{c => <option value={c.name} selected={p()?.settings.class === c.name}>{c.name}</option>}</For></select>
      <select class="ctxrating" onChange={e => sets("rating", +e.currentTarget.value)}><For each={RATINGS}>{r => <option value={r} selected={p()?.settings.rating === r}>Class {r}</option>}</For></select></span>
    <span class="sep" />
    <label title="Clicks in the 3D view while routing land at this elevation (PgUp / PgDn)">EL</label>
    <input class="ctxel" type="number" step="0.1" value={+toU("coord", elev()).toFixed(3)} onChange={e => { const v = +e.currentTarget.value; if (Number.isFinite(v)) setElev(fromU("coord", v)); }} /><small class="unit">{unit("coord")}</small>
    <span class="sep" />
    <label>Colour</label>
    <select class="ctxcolour" onChange={e => setColourBy(e.currentTarget.value as ColourBy)}><For each={[["kind", "Part kind"], ["line", "Line"], ["class", "Pipe class"], ["fluid", "Fluid"], ["size", "Size"]] as [ColourBy, string][]}>{([k, l]) => <option value={k} selected={colourBy() === k}>{l}</option>}</For></select>
    <span class="seg" role="group" aria-label="Show">
      <button classList={{ on: opts().labels }} title="Tags and line numbers" onClick={() => setOpts({ ...opts(), labels: !opts().labels })}><Icon n="tag" /></button>
      <button classList={{ on: !!section() }} title="Section box" onClick={C.sectionBox}><Icon n="section" /></button>
      <button classList={{ on: !!(hidden().length || isolated()) }} title={hidden().length || isolated() ? "Show everything (Shift+H)" : "Nothing hidden"} onClick={C.showAll}><Icon n="showall" /></button>
    </span>
    <span class="sep" />
    <button class="ib" title="Fit the plant in the view (F)" onClick={C.fit}><Icon n="fit" /></button>
    <button class="ib" title="Isometric view (1)" onClick={() => C.std("iso")}><Icon n="cube" /></button>
    <button class="ib" title="Plan, from above (2)" onClick={() => C.std("plan")}><Icon n="plan" /></button>
    <button class="ib" title="Front, from the south (3)" onClick={() => C.std("front")}><Icon n="front" /></button>
    <button class="ib" title="Side, from the east (4)" onClick={() => C.std("side")}><Icon n="side" /></button>
    <span class="grow" />
    <Show when={mode() !== "select"}><span class="ctxmode">{mode() === "route" ? "Routing" : mode() === "place" ? "Placing" : "Measuring"} · Esc to stop</span><span class="sep" /></Show>
    <label>Units</label>
    <select class="ctxunits" value={p()?.units ?? "SI"} onChange={e => C.units(e.currentTarget.value as "SI" | "US")} title="Units shown and typed (the file keeps metres)">
      <option value="SI">SI · m, mm, bar</option><option value="US">US · ft, in, psi</option>
    </select>
  </div>;
};

// ------------------------------------------------------------------ the tool strip
type Tl = [string, string, () => void, (() => boolean)?] | "-";
const TOOLS: Record<string, () => Tl[]> = {
  home: () => [["route", "Route a line (R)", C.route, () => mode() === "route"], ["measure", "Measure (M)", C.measure, () => mode() === "measure"], "-", ["copy", "Copy the selection (Ctrl+D)", C.copyGroup], ["rotate", "Turn the selection 90°", () => C.rotateGroup(90)], ["mirror", "Mirror east–west", () => C.mirrorGroup(false)], "-", ["clash", "Check (F5)", C.check]],
  equipment: () => EQUIP.map(([k, l, i]) => [i, `Place a ${l.toLowerCase()}`, () => C.place(k)] as Tl),
  piping: () => [["route", "Route a line (R)", C.route, () => mode() === "route"], ["route", "Route the selected line again", C.reroute], "-", ["jog", "Add a jog to the selected run (J)", () => C.jog()], ["straight", "Remove the jog", C.removeJog], "-", ["up", "One size larger", () => C.step(true)], ["down", "One size smaller", () => C.step(false)], "-", ["support", "Rest on the selected run", () => C.support("rest")]],
  fittings: () => ITEMS.map(([k, l, i]) => [i, `${l}: on the selected line`, () => C.dropOnSelected(k)] as Tl),
  check: () => [["clash", "Check (F5)", C.check], ["list", "Clashes", () => setBottom("clashes")], ["warn", "Checks", () => setBottom("checks")]],
  lists: () => [["list", "Line list", () => setBottom("lines")], ["vvessel", "Equipment list", () => setBottom("equipment")], ["valve", "Valve list", () => setBottom("valves")], "-", ["import", "Import a line list", C.importLineList], ["pipe", "Import PCF lines", C.importPcf], "-", ["csv", "Export the line list", () => C.exportCsv("lines")]],
  view: () => [["fit", "Fit (F)", C.fit], ["cube", "Isometric", () => C.std("iso")], ["plan", "Plan", () => C.std("plan")], "-", ["hide", "Hide (H)", C.hide], ["isolate", "Isolate (I)", C.isolate], ["showall", "Show all", C.showAll], ["section", "Section box", C.sectionBox, () => !!section()], "-", ["camera", "Save this view", C.saveView]],
};
const [tip, setTip] = createSignal<{ text: string; x: number; y: number } | null>(null);
/** Beside the tree: the open tab's most used tools, one click each. */
export const ToolStrip = () => {
  const list = () => (TOOLS[tab()] ?? TOOLS.home)();
  return <div class="toolstrip" role="toolbar" aria-label="Tools of this tab">
    <For each={list()}>{t => t === "-" ? <hr /> : <button classList={{ on: !!t[3]?.() }} aria-label={t[1]} data-tip={t[1]} onClick={() => t[2]()}
      onMouseEnter={e => { const r = e.currentTarget.getBoundingClientRect(); setTip({ text: t[1], x: r.right + 6, y: r.top + r.height / 2 }); }} onMouseLeave={() => setTip(null)}><Icon n={t[0]} /></button>}</For>
    <Show when={tip()}>{x => <div class="striptip" style={{ left: `${x().x}px`, top: `${x().y}px` }}>{x().text}</div>}</Show>
  </div>;
};

/** Drag a pane's edge to widen or narrow it (kept for the session). */
export const [treeW, setTreeW] = createSignal(260);
export const [inspW, setInspW] = createSignal(300);
export const grip = (e: PointerEvent, which: "tree" | "insp") => {
  e.preventDefault();
  const x0 = e.clientX, w0 = which === "tree" ? treeW() : inspW();
  const move = (m: PointerEvent) => { const d = m.clientX - x0; if (which === "tree") setTreeW(Math.max(200, Math.min(520, w0 + d))); else setInspW(Math.max(260, Math.min(560, w0 - d))); };
  const up = () => { removeEventListener("pointermove", move); removeEventListener("pointerup", up); };
  addEventListener("pointermove", move); addEventListener("pointerup", up);
};
/** The properties card over the view: open while something is selected. */
export const popShown = () => mode() === "select" && (!!sel() || picked().length > 0);
export const closePop = () => { setSel(null); setPicked([]); };
