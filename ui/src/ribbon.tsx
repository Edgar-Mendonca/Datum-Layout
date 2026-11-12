// Title bar, quick access toolbar, Tell Me and the ribbon (Home · Equipment · Piping · Fittings ·
// Check · Lists · View · Help, and the contextual Format tab for a selected line or equipment).
// Copied from Datum Plan's ribbon by way of Datum Pipe.
import { For, Show, JSX, createSignal, createMemo, onMount } from "solid-js";
import { colourBy, setColourBy, ColourBy, section, hidden, isolated, picked, call, out, project, tab, setTab, setBackstage, openMenu, openDialog, closeDialog, theme, setTheme, sel, opts, setOpts, selLine, selEquip, mode, setBottom, checked, act } from "./store";
import { C, ALL_CMDS } from "./cmds";
import * as D from "./dialogs";
import { Icon } from "./ui";
import { Help, helpCard, HELP } from "./help";
import { invoke, Example, EQUIP, ITEMS, NPS, PART_GROUPS, inch } from "./api";
import { fmt, toU, fromU, unit } from "./units";

const GH = "https://github.com/Edgar-Mendonca/Datum-Layout";

export const B = (p: { i: string; l: string; run?: (e: MouseEvent) => void; big?: boolean; on?: boolean; off?: boolean; dd?: (e: MouseEvent) => void; title?: string }) => p.big && p.run && p.dd ? (
  <div class="rbw big splitbig" classList={{ on: p.on }}>
    <button class="rb top" title={p.title ?? p.l} disabled={p.off} onClick={e => p.run!(e)}><Icon n={p.i} /></button>
    <button class="rb bot" title={`${p.l} options`} onClick={e => p.dd!(e)}><span>{p.l}</span><b class="car">▾</b></button>
  </div>
) : (
  <div class="rbw" classList={{ big: p.big }}>
    <button class="rb" classList={{ big: p.big, on: p.on }} disabled={p.off} title={p.title ?? p.l} onClick={e => (p.run ?? p.dd)?.(e)}><Icon n={p.i} />{p.big ? <span>{p.l}{p.dd && !p.run ? " ▾" : ""}</span> : <span>{p.l}</span>}{!p.big && p.dd && !p.run && <b class="car">▾</b>}</button>
    {p.run && p.dd && <button class="rb split" onClick={e => p.dd!(e)}>▾</button>}
  </div>
);
export const G = (p: { l: string; h?: string; children: JSX.Element }) => <div class="grp"><div class="items">{p.children}</div><div class="cap">{p.l}{p.h && <Help k={p.h} />}</div></div>;
export const S = (p: { children: JSX.Element }) => <div class="stack">{p.children}</div>;
/** A planned tool: shown dimmed with the release it arrives in. */
export const Planned = (p: { i: string; l: string; v: string; big?: boolean }) => <B i={p.i} l={p.l} big={p.big} off title={`${p.l}: planned for ${p.v}`} />;

/** Tell Me / command search (Ctrl+K). */
export const palette = () => openDialog(() => {
  const [q, setQ] = createSignal(""), [k, setK] = createSignal(0);
  const list = createMemo(() => { const s = q().toLowerCase(); return ALL_CMDS.filter(c => c[0].toLowerCase().includes(s)).slice(0, 14); });
  const go = (i: number) => { const c = list()[i]; if (c) { closeDialog(); setTimeout(() => c[1]()); } };
  let input!: HTMLInputElement;
  onMount(() => input.focus());
  return (
    <div class="ov top" onMouseDown={e => e.target === e.currentTarget && closeDialog()}>
      <div class="pal">
        <input ref={input} placeholder="Tell me what you want to do" value={q()} onInput={e => { setQ(e.currentTarget.value); setK(0); }}
          onKeyDown={e => { if (e.key === "ArrowDown") { e.preventDefault(); setK(Math.min(list().length - 1, k() + 1)); } else if (e.key === "ArrowUp") { e.preventDefault(); setK(Math.max(0, k() - 1)); } else if (e.key === "Enter") { e.preventDefault(); go(k()); } else if (e.key === "Escape") closeDialog(); }} />
        <div class="pl"><For each={list()}>{(c, i) => <button classList={{ on: i() === k() }} onMouseEnter={() => setK(i())} onClick={() => go(i())}><span>{c[0]}</span>{c[2] && <kbd>{c[2]}</kbd>}</button>}</For></div>
      </div>
    </div>
  );
});

export const TABS: [string, string][] = [["home", "Home"], ["equipment", "Equipment"], ["piping", "Piping"], ["fittings", "Fittings"], ["check", "Check"], ["lists", "Lists"], ["view", "View"], ["help", "Help"]];

export const TitleBar = (p: { update: string | null }) => (
  <header class="top">
    <div class="bar">
      <div class="qat">
        <img src="logo.svg" alt="" class="logo" /><b class="appname">Datum Layout</b>
        <button title="New plant (Ctrl+N)" onClick={() => setBackstage("new")}><Icon n="new" /></button>
        <button title="Open (Ctrl+O)" onClick={() => C.open()}><Icon n="open" /></button>
        <button title="Save (Ctrl+S)" onClick={() => C.save()}><Icon n="save" /></button>
        <button title="Undo (Ctrl+Z)" disabled={!out()?.undo} onClick={C.undo}><Icon n="undo" /></button>
        <button title="Redo (Ctrl+Y)" disabled={!out()?.redo} onClick={C.redo}><Icon n="redo" /></button>
        <button title="Print (Ctrl+P)" onClick={C.print}><Icon n="print" /></button>
      </div>
      <div class="title"><Show when={out()} fallback="Datum Layout">{o => <><b>{o().p.name}{o().dirty ? " •" : ""}</b><span class="tsub"> · {o().p.equipment.length} equipment · {o().p.lines.length} {o().p.lines.length === 1 ? "line" : "lines"} · {o().p.units === "US" ? "US" : "SI"}</span></>}</Show></div>
      <button class="search" onClick={palette} title="Tell me what you want to do (Ctrl+K)"><Icon n="search" /><span>Tell me what you want to do</span><kbd>Ctrl+K</kbd></button>
      <Show when={p.update}><button class="upd" onClick={() => setBackstage("about")}>Update {p.update} available</button></Show>
      <button class="theme" title="Theme: Auto follows Windows" onClick={() => setTheme(theme() === "auto" ? "light" : theme() === "light" ? "dark" : "auto")}>Theme · {theme()}</button>
    </div>
    <nav class="tabs">
      <button class="file" onClick={() => setBackstage("home")}>File</button>
      <For each={TABS}>{([k, l]) => <button classList={{ on: tab() === k }} onClick={() => setTab(k)}>{l}</button>}</For>
      <div class="ctxgrp" style={{ visibility: selLine() || selEquip() ? "visible" : "hidden" }}><small>{selLine() ? "Line Tools" : "Equipment Tools"}</small><button classList={{ on: tab() === "format" }} onClick={() => setTab("format")}>Format</button></div>
    </nav>
  </header>
);

/** Shrinks the ribbon in steps instead of scrolling sideways (FAMILY §6). */
const fitRibbon = (el: HTMLElement) => {
  const fit = () => {
    const gs = [...el.querySelectorAll<HTMLElement>(":scope > .grp")];
    el.dataset.fit = "0"; gs.forEach(g => delete g.dataset.fit);
    const over = () => el.scrollWidth > el.clientWidth + 1;
    if (!over()) return;
    el.dataset.fit = "1";
    for (const lvl of ["2", "3"]) for (let i = gs.length - 1; i >= 0 && over(); i--) gs[i].dataset.fit = lvl;
  };
  new ResizeObserver(fit).observe(el);
  new MutationObserver(() => requestAnimationFrame(fit)).observe(el, { childList: true, subtree: false });
};

export const exampleMenu = async (e: MouseEvent) => {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
  const list = await invoke<Example[]>("examples_list").catch(() => [] as Example[]);
  openMenu({ x: r.left, y: r.bottom + 2 }, list.map(x => ({ label: x.name, run: () => C.newProject(x.id) })));
};
const sizeMenu = (e: MouseEvent) => openMenu(e, NPS.map(n => ({ label: inch(n), on: selLine()?.nps === n, run: () => C.resize(n) })));

export const Ribbon = () => {
  const p = () => project();
  const n = (k: string) => (checked() ?? []).filter(i => i.kind === k).length;
  return (
    <section class="ribbon" ref={fitRibbon}>
      <Show when={tab() === "home"}>
        <G l="Plant" h="examples"><B big i="new" l="New Plant" run={() => C.newProject(null)} /><B big i="open" l="Examples" dd={exampleMenu} /></G>
        <G l="Place" h="equipment"><B big i="vvessel" l="Equipment" dd={e => openMenu(e, EQUIP.map(([k, l]) => ({ label: l, run: () => C.place(k) })))} /><B big i="route" l="Route Line" on={mode() === "route"} run={C.route} title="Route a line from a nozzle or point (R)" /></G>
        <G l="Edit" h="edit"><S><B i="undo" l="Undo" run={C.undo} off={!out()?.undo} /><B i="redo" l="Redo" run={C.redo} off={!out()?.redo} /><B i="del" l="Delete" run={C.del} off={!sel() && !picked().length} /></S></G>
        <G l="Arrange" h="group"><B big i="copy" l="Copy" run={C.copyGroup} title="Copy the selection beside it (Ctrl+D)" /><S><B i="rotate" l="Turn 90°" run={() => C.rotateGroup(90)} /><B i="rotate" l="Turn −90°" run={() => C.rotateGroup(-90)} /><B i="selectall" l="Select All" run={C.selectAll} /></S><S><B i="mirror" l="Mirror east–west" run={() => C.mirrorGroup(false)} /><B i="mirror" l="Mirror north–south" run={() => C.mirrorGroup(true)} /></S></G>
        <G l="Units" h="units"><S><B i="units" l="SI: m, mm, bar, °C" on={p()?.units === "SI"} run={() => C.units("SI")} /><B i="units" l="US: ft, in, psi, °F" on={p()?.units === "US"} run={() => C.units("US")} /></S></G>
        <G l="Check" h="checks"><B big i="clash" l="Check" run={C.check} title="Find clashes, open ends, mismatches and class problems (F5)" /><B big i="measure" l="Measure" on={mode() === "measure"} run={C.measure} title="Measure a distance between two points (M)" /></G>
        <G l="Hand Over" h="pcf"><B big i="pipe" l="To Datum Pipe" run={() => { const l = selLine(); C.exportPcf(l?.id); }} title="Export the selected line (or every line) as a piping component file for Datum Pipe" /></G>
      </Show>
      <Show when={tab() === "equipment"}>
        <For each={PART_GROUPS[0].subs}>{([sub, ks]) => { const eq = (k: string) => EQUIP.find(x => x[0] === k)!; return <G l={sub.replace(" and ", " & ")} h="equipment">
          <B big i={eq(ks[0])[2]} l={eq(ks[0])[1].replace(/ \(.*\)$/, "")} run={() => C.place(ks[0] as any)} />
          <Show when={ks.length > 1}><S><For each={ks.slice(1)}>{k => <B i={eq(k)[2]} l={eq(k)[1].replace(/ \(.*\)$/, "")} run={() => C.place(k as any)} />}</For></S></Show></G>; }}</For>
        <G l="Arrange" h="move"><S><B i="rotate" l="Rotate 90°" run={() => C.rotate(90)} off={!selEquip()} /><B i="rotate" l="Rotate −90°" run={() => C.rotate(-90)} off={!selEquip()} /><B i="del" l="Delete" run={C.del} off={!sel()} /></S></G>
        <G l="Nozzles" h="nozzles"><S><div class="rnote">Select an item: its nozzles are listed in the Inspector, where you can add, move or remove them.</div></S></G>
        <G l="From the P&ID" h="package"><B big i="import" l="Plant Package" run={C.importPackage} title="Read a plant package from Datum Schema: equipment, nozzles, lines and their components" /><B big i="route" l="Route Listed Lines" run={C.routeListed} off={!p()?.line_list.length} title="Route the listed lines nozzle to nozzle, with their components in the P&ID's order" /></G>
      </Show>
      <Show when={tab() === "piping"}>
        <G l="Route" h="route"><B big i="route" l="Route Line" on={mode() === "route"} run={C.route} title="Click a nozzle or a point, then the corners; end on a nozzle or double-click (R)" /><S><B i="up" l="Up one grid step" run={() => C.elevation(p()?.plot.grid ?? 1)} /><B i="down" l="Down one grid step" run={() => C.elevation(-(p()?.plot.grid ?? 1))} /></S><S><B i="route" l="Route Again" run={C.reroute} off={!selLine()} title="Route the selected line square again between its ends (after equipment moved or turned)" /></S></G>
        <G l="Pipe Classes" h="spec"><B big i="library" l="Library" run={() => import("./library").then(k => k.library(`class:${p()?.settings.class ?? ""}`))} title="The pipe classes and parts tables; edit a class for this plant. The size, class and rating of new lines are on the bar under the ribbon." /></G>
        <G l="Corners" h="corners"><B big i="jog" l="Add Jog" run={() => C.jog()} off={sel()?.kind !== "run"} title="Step the selected run up (or sideways) with four elbows, to pass over or round something (J)" /><S><B i="straight" l="Remove Jog" run={C.removeJog} off={sel()?.kind !== "run"} title="Straighten the jog whose middle run is selected" /><div class="rnote">Select a line to drag its corners</div></S></G>
        <G l="Line Size" h="resize"><B big i="resize" l="Size" dd={sizeMenu} off={!selLine()} title="Change the selected line's size: every component re-sizes" /><S><B i="up" l="One size larger" run={() => C.step(true)} off={!selLine()} /><B i="down" l="One size smaller" run={() => C.step(false)} off={!selLine()} /></S></G>
        <G l="Elbows" h="elbows"><S><B i="elbow" l="Long radius" on={selLine()?.radius === "long"} run={() => C.radius("long")} off={!selLine()} /><B i="elbow" l="Short radius" on={selLine()?.radius === "short"} run={() => C.radius("short")} off={!selLine()} /></S></G>
        <G l="Supports" h="supports"><B big i="support" l="Rest" run={() => C.support("rest")} off={!selLine()} title="A shoe under the pipe on the selected run (or drag Pipe support from the catalogue onto a pipe)" /><S><B i="support" l="Guide" run={() => C.support("guide")} off={!selLine()} /><B i="support" l="Line stop" run={() => C.support("stop")} off={!selLine()} /><B i="support" l="Anchor" run={() => C.support("anchor")} off={!selLine()} /></S></G>
      </Show>
      <Show when={tab() === "fittings"}>
        <G l="Valves" h="fittings"><For each={ITEMS.slice(0, 5)}>{([k, l, i]) => <B big i={i} l={l.replace(" valve", "")} run={() => C.dropOnSelected(k)} title={`${l}: drag it from the catalogue onto a pipe, or select a line and click`} />}</For></G>
        <G l="In-line" h="fittings"><B big i="strainer" l="Strainer" run={() => C.dropOnSelected("strainer")} /><B big i="flange" l="Flange Pair" run={() => C.dropOnSelected("flange_pair")} /><B big i="reducer" l="Reducer" run={() => C.dropOnSelected("reducer")} /></G>
        <G l="Branch" h="tee"><B big i="tee" l="Tee" run={() => C.dropOnSelected("tee")} title="A tee on the selected line starts a branch" /><B big i="cross" l="Cross" run={() => C.dropOnSelected("cross")} title="A cross on the selected line: two branches, opposite each other" /><S><B i="weldolet" l="Weldolet" run={() => C.dropOnSelected("weldolet")} title="A branch outlet on the header (it does not cut it); routing a branch off a tee also makes one where the class says" /><B i="sockolet" l="Sockolet" run={() => C.dropOnSelected("sockolet")} title="A socket-weld outlet for a small branch" /></S></G>
        <G l="Line Ends" h="ends"><S><B i="cap" l="Cap the start" run={() => C.term(true, "cap")} off={!selLine()} /><B i="cap" l="Cap the end" run={() => C.term(false, "cap")} off={!selLine()} /></S><S><B i="blind" l="Blind the start" run={() => C.term(true, "blind")} off={!selLine()} /><B i="blind" l="Blind the end" run={() => C.term(false, "blind")} off={!selLine()} /></S><S><B i="open" l="Open the start" run={() => C.term(true, "open")} off={!selLine()} /><B i="open" l="Open the end" run={() => C.term(false, "open")} off={!selLine()} /></S></G>
      </Show>
      <Show when={tab() === "check"}>
        <G l="Check" h="checks"><B big i="clash" l="Check" run={C.check} title="Run every check (F5)" /><S><B i="list" l={`Clashes${checked() ? ` (${n("clash")})` : ""}`} run={() => setBottom("clashes")} /><B i="warn" l={`Checks${checked() ? ` (${(checked() ?? []).length - n("clash")})` : ""}`} run={() => setBottom("checks")} /></S></G>
        <G l="Clearance" h="clearance"><S><label class="rsel">Clearance<input type="number" min="0" step="5" style={{ width: "64px" }} value={+toU("clear", p()?.settings.clearance ?? 0.025).toFixed(1)} onChange={e => { const v = fromU("clear", +e.currentTarget.value); if (p() && Number.isFinite(v) && v >= 0) act({ SetSettings: { ...p()!.settings, clearance: v } }); }} /><span class="unit">{unit("clear")}</span></label><div class="rnote">Pipe–pipe, pipe–equipment and pipe–ground</div></S></G>
        <G l="Against the P&ID" h="package"><B big i="compare" l="3D vs P&ID" off={!p()?.line_list.length} run={() => { setBottom("checks"); }} title="Lines missing or extra, sizes, classes and the components' order, against the plant package or line list" /></G>
      </Show>
      <Show when={tab() === "lists"}>
        <G l="Lists" h="lists"><B big i="list" l="Line List" run={() => setBottom("lines")} /><B big i="vvessel" l="Equipment" run={() => setBottom("equipment")} /><B big i="valve" l="Valve List" run={() => setBottom("valves")} title="Every valve and strainer: tag, type, size, class, line, position and stem" /><B big i="mto" l="Take-off" run={() => setBottom("mto")} title="The material take-off: pipe, fittings, flanges, valves, gaskets, bolts and welds per line, with weights" /></G>
        <G l="Import" h="import"><B big i="import" l="Plant Package" run={C.importPackage} title="Read a plant package from Datum Schema" /><B big i="import" l="Line List" run={C.importLineList} title="Read a line list (CSV) to check the model against" /><B big i="pipe" l="PCF Lines" run={C.importPcf} title="Add lines from a piping component file (.pcf)" /></G>
        <G l="Export CSV" h="csv"><S><B i="csv" l="Line list" run={() => C.exportCsv("lines")} /><B i="csv" l="Equipment and nozzles" run={() => C.exportCsv("equipment")} /><B i="csv" l="Material take-off" run={() => C.exportCsv("mto")} /><B i="csv" l="Valve list" run={() => C.exportCsv("valves")} /></S></G>
        <G l="Exports" h="pcf"><B big i="pipe" l="PCF for Datum Pipe" run={() => C.exportPcf(selLine()?.id)} /><B big i="cube" l="glTF 3D Model" run={C.exportGltf} /><Planned big i="ifc" l="IFC" v="0.4" /></G>
        <G l="Print" h="print"><B big i="print" l="Print / PDF" run={C.print} /></G>
      </Show>
      <Show when={tab() === "view"}>
        <G l="Zoom" h="view"><B big i="fit" l="Fit" run={C.fit} /><B big i="measure" l="Measure" on={mode() === "measure"} run={C.measure} title="Measure a distance between two points (M)" /></G>
        <G l="Standard Views" h="view"><B big i="cube" l="Isometric" run={() => C.std("iso")} /><S><B i="plan" l="Plan (from above)" run={() => C.std("plan")} /><B i="front" l="Front (from south)" run={() => C.std("front")} /><B i="side" l="Side (from east)" run={() => C.std("side")} /></S></G>
        <G l="Show" h="show"><S><label class="chk"><input type="checkbox" checked={opts().labels} onChange={e => setOpts({ ...opts(), labels: e.currentTarget.checked })} />Tags and line numbers</label>
          <label class="rsel">Colour by<select class="colourby" onChange={e => setColourBy(e.currentTarget.value as ColourBy)}><For each={[["kind", "Part kind"], ["line", "Line"], ["class", "Pipe class"], ["fluid", "Fluid"], ["size", "Size"]] as [ColourBy, string][]}>{([k, l]) => <option value={k} selected={colourBy() === k}>{l}</option>}</For></select></label></S>
          <S><B i="hide" l="Hide" run={C.hide} off={!sel() && !picked().length} title="Hide the selection (H)" /><B i="isolate" l="Isolate" run={C.isolate} off={!sel() && !picked().length} title="Show only the selection (I)" /><B i="showall" l="Show All" run={C.showAll} off={!hidden().length && !isolated()} title="Show everything again (Shift+H)" /></S>
          <B big i="section" l="Section Box" on={!!section()} run={C.sectionBox} title="Draw only what is inside a box round the selection or the whole plant" /></G>
        <G l="Saved Views" h="views"><B big i="camera" l="Save View" run={C.saveView} title="Keep this camera, colours, hidden items and section box under a name" /><B big i="views" l="Views" off={!p()?.views.length} dd={e => { const vs = p()?.views ?? []; openMenu(e, [...vs.map(v => ({ label: v.name, run: () => C.showView(v.name) })), "-", { label: "Rename", sub: vs.map(v => ({ label: v.name, run: () => D.renameView(v.name) })) }, { label: "Delete", sub: vs.map(v => ({ label: v.name, run: () => C.deleteView(v.name) })) }]); }} /></G>
        <G l="Plot" h="plot"><B big i="grid" l="Plot" run={C.selectPlot} title="Grid, extent, north and ground in the Inspector" /><B big i="area" l="Add Area" run={C.addArea} /></G>
        <G l="Walk"><Planned big i="walk" l="Walk Through" v="a later release" /></G>
      </Show>
      <Show when={tab() === "format"}>
        <Show when={selLine()} fallback={<Show when={selEquip()} fallback={<G l="Format"><div class="rnote">Select a line or an equipment item.</div></G>}>
          <G l="Arrange" h="move"><B big i="rotate" l="Rotate 90°" run={() => C.rotate(90)} /><B big i="rotate" l="Rotate −90°" run={() => C.rotate(-90)} /></G>
          <G l="Item"><div class="rnote">{selEquip()!.tag}: sizes, position and nozzles are in the Inspector.</div></G>
        </Show>}>
          <G l="Size" h="resize"><B big i="resize" l={inch(selLine()!.nps)} dd={sizeMenu} /><S><B i="up" l="Larger" run={() => C.step(true)} /><B i="down" l="Smaller" run={() => C.step(false)} /></S></G>
          <G l="Elbows" h="elbows"><S><B i="elbow" l="Long radius" on={selLine()!.radius === "long"} run={() => C.radius("long")} /><B i="elbow" l="Short radius" on={selLine()!.radius === "short"} run={() => C.radius("short")} /></S></G>
          <G l="Insulation" h="insulation"><S><label class="rsel">Thickness<input type="number" min="0" step="10" style={{ width: "64px" }} value={+toU("dim", selLine()!.insulation).toFixed(1)} onChange={e => { const l = selLine(); const v = fromU("dim", +e.currentTarget.value); if (l && Number.isFinite(v) && v >= 0) act({ SetLine: { ...l, insulation: v } }); }} /><span class="unit">{unit("dim")}</span></label></S></G>
          <G l="Hand Over" h="pcf"><B big i="pipe" l="To Datum Pipe" run={() => C.exportPcf(selLine()!.id)} /></G>
          <G l="Length"><div class="rnote">{selLine()!.number || "Line"}: {fmt("len", (out()?.paths.find(x => x[0] === selLine()!.id)?.[1] ?? []).reduce((s, q, i, a) => i ? s + Math.hypot(q[0] - a[i - 1][0], q[1] - a[i - 1][1], q[2] - a[i - 1][2]) : 0, 0))} of route</div></G>
        </Show>
      </Show>
      <Show when={tab() === "help"}>
        <G l="Learn"><B big i="report" l="Getting Started" run={() => D.gettingStarted()} /><B big i="info" l="Glossary" run={() => D.glossary()} /><B big i="help" l="What Is This Tool?" dd={e => openMenu(e, Object.entries(HELP).map(([k, h]) => ({ label: h.t, run: () => helpCard(k) })))} /><B big i="keys" l="Shortcuts" run={() => D.shortcuts()} /><B big i="search" l="Tell Me" run={palette} /></G>
        <G l="Catalogue" h="library"><B big i="library" l="Library" run={() => import("./library").then(k => k.library())} title="Every pipe class and parts table, with its source" /><B big i="info" l="Sources" run={() => D.sources()} title="Where every dimension table comes from" /></G>
        <G l="Datum Layout on GitHub"><B big i="github" l="GitHub" run={() => call("open_url", { url: GH })} /><B big i="tag" l="Releases" run={() => call("open_url", { url: `${GH}/releases` })} /><B big i="bug" l="Report an Issue" run={() => call("open_url", { url: `${GH}/issues/new` })} /></G>
        <G l="Updates"><B big i="upd" l="Check for Updates" run={() => setBackstage("about")} /><B big i="list" l="Roadmap" run={() => setBackstage("roadmap")} /></G>
      </Show>
    </section>
  );
};
