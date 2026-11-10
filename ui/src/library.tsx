// The Library (0.2.0, after Datum Mesh's materials library): every pipe class and every parts
// table the catalogue holds, browsable and searchable, each row with its source. Nothing here is
// calculated: the window shows the catalogue the engine read (built-in tables and plug-in packs).
import { For, Show, createSignal, createMemo, createResource, JSX } from "solid-js";
import { CatalogueOut, PipeClass, inch, NPS, ITEMS } from "./api";
import { call, openDialog, project, act } from "./store";
import { ClassEditor } from "./classedit";
import { Dialog } from "./ui";
import { fmt } from "./units";

type Page = { k: string; label: string; group: string; sub?: string };
const mm = (m: number) => fmt("dim", m);
const IT_NAME = { pipe: "Pipe", elbow90: "Elbow 90°", elbow45: "Elbow 45°", cap: "Cap", flange: "Weld-neck flange", blind: "Blind flange", gasket: "Gasket", bolts: "Stud bolts", ...Object.fromEntries(ITEMS.map(([k, l]) => [k, l.replace(/ \(.*\)$/, "")])) } as Record<string, string>;
const VALVE: Record<string, string> = { gate: "Gate", globe: "Globe", check: "Check (swing)", ball: "Ball", butterfly: "Butterfly (triple offset, short pattern)", strainer: "Strainer (Y)" };
/** The class's schedule for a size: the last "from this size" entry not above it (the engine's rule). */
const schedOf = (c: PipeClass, s: string) => [...c.schedules].reverse().find(([from]) => NPS.indexOf(from) <= NPS.indexOf(s))?.[1] ?? "STD";
const byNps = (a: { nps: string }, b: { nps: string }) => NPS.indexOf(a.nps) - NPS.indexOf(b.nps);

/** A table with a sticky header; rows filtered by the search box. */
const Tbl = (p: { head: string[]; rows: (string | JSX.Element)[][]; right?: number[] }) => (
  <div class="libtbl"><table class="tbl"><thead><tr><For each={p.head}>{(h, i) => <th classList={{ r: p.right?.includes(i()) }}>{h}</th>}</For></tr></thead>
    <tbody><For each={p.rows} fallback={<tr><td colSpan={p.head.length} class="muted">Nothing matches the search.</td></tr>}>{r => <tr><For each={r}>{(c, i) => <td classList={{ r: p.right?.includes(i()), mono: p.right?.includes(i()) }}>{c}</td>}</For></tr>}</For></tbody></table></div>
);

/** Help › Library (or the steps bar's Library): the pipe classes and the parts tables. */
const WKIND: Record<string, string> = { elbow90_lr: "Elbow 90° long radius", elbow90_sr: "Elbow 90° short radius", elbow45_lr: "Elbow 45°", tee: "Tee, equal", reducing_tee: "Tee, reducing", reducer: "Reducer", flange_wn: "Flange, weld neck", flange_blind: "Flange, blind", gate: "Gate valve", globe: "Globe valve", check: "Check valve", ball: "Ball valve" };

export const library = (start?: string) => openDialog(() => {
  const [c, { refetch }] = createResource(() => call<CatalogueOut>("catalogue"));
  /** The class being edited (the editor shows in place of its page). */
  const [editing, setEditing] = createSignal<{ start: PipeClass; was: string | null } | null>(null);
  const own = (n: string) => !!c()?.own?.includes(n);
  const builtin = (n: string) => !!c()?.base?.includes(n);
  const blank = (): PipeClass => ({ name: "", description: "", material: "Carbon steel", ratings: [150], sizes: NPS.slice(0, 13), schedules: [["1/2", "80"], ["2", "40"]], elbow: "long", components: ["pipe", "elbow90", "elbow45", "tee", "reducer", "cap", "flange", "blind", "gasket", "gate", "check"].map(kind => ({ kind, min: null, max: null })), branches: [{ branch: "same", fitting: "tee" }, { fitting: "reducing_tee" }], source: "" });
  const copyName = (n: string) => { let k = 1, s = `${n}-A`; while (c()?.cat.classes.some(x => x.name === s)) s = `${n}-${String.fromCharCode(65 + k++)}`; return s; };
  const [q, setQ] = createSignal("");
  const [cur, setCur] = createSignal(start ?? "");
  const cat = () => c()?.cat;
  const pages = createMemo((): Page[] => {
    const k = cat(); if (!k) return [];
    return [
      ...k.classes.map(x => ({ k: `class:${x.name}`, label: x.name, group: "Pipe classes", sub: own(x.name) ? (builtin(x.name) ? "edited · this plant" : "this plant") : x.material })),
      { k: "pipe", label: "Pipe", group: "Parts tables", sub: `${k.pipes.length} sizes` },
      { k: "fittings", label: "Butt-weld fittings", group: "Parts tables", sub: `${k.fittings.length} sizes` },
      { k: "flanges", label: "Flanges", group: "Parts tables", sub: `${k.flanges.length} rows` },
      { k: "tees", label: "Reducing tees", group: "Parts tables", sub: `${k.tees.length} pairs` },
      { k: "reducers", label: "Reducers", group: "Parts tables", sub: `${k.reducers.length} pairs` },
      { k: "olets", label: "Branch outlets", group: "Parts tables", sub: `${k.olets.length} rows` },
      { k: "valves", label: "Valves and strainers", group: "Parts tables", sub: `${k.valves.length} rows` },
      { k: "weights", label: "Weights", group: "Parts tables", sub: `${k.weights.length} rows` },
    ];
  });
  const shownPages = () => { const t = q().toLowerCase(); return pages().filter(x => !t || `${x.label} ${x.sub ?? ""} ${x.group}`.toLowerCase().includes(t) || !x.k.startsWith("class:")); };
  const page = () => cur() || pages()[0]?.k || "";
  const cls = (): PipeClass | undefined => cat()?.classes.find(x => `class:${x.name}` === page());
  const has = (...v: (string | number)[]) => { const t = q().trim().toLowerCase(); return !t || v.join(" ").toLowerCase().includes(t); };
  const used = (name: string) => (project()?.lines ?? []).filter(l => l.class === name).length;
  return (
    <Dialog title="Library" wide help="library">
      <div class="mgr lib libx">
        <div class="mlist"><input class="libsearch" placeholder="Search: CS300, 6, gate, 2500…" value={q()} onInput={e => setQ(e.currentTarget.value)} />
          <Show when={cat()} fallback={<p class="muted">Loading…</p>}>
            <For each={[...new Set(shownPages().map(x => x.group))]}>{g => <><small class="fam">{g}</small>
              <For each={shownPages().filter(x => x.group === g)}>{x => <button classList={{ on: x.k === page() && !editing() }} onClick={() => { setEditing(null); setCur(x.k); }}><span>{x.label}</span>{x.sub && <small>{x.sub}</small>}</button>}</For>
              <Show when={g === "Pipe classes"}><button class="libnew" disabled={!project()} title={project() ? "Write a new pipe class for this plant" : "Open or start a plant first: a class is kept in the plant's file"} onClick={() => setEditing({ start: blank(), was: null })}>+ New pipe class</button></Show></>}</For>
            <Show when={cat()!.packs.length}><small class="fam">Plug-in packs</small><For each={cat()!.packs}>{p => <div class="libpack">{p}</div>}</For></Show>
          </Show></div>
        <div class="form libpage">
          <Show when={editing()} keyed>{e => <ClassEditor start={e.start} was={e.was} cat={cat()!} own={!!e.was && own(e.was)} builtin={!!e.was && builtin(e.was)} done={async name => { setEditing(null); await refetch(); if (name) setCur(`class:${name}`); }} />}</Show>
          <Show when={!editing() && cls()}>{x => <>
            <h3>{x().name} <small class="muted">{x().description}</small></h3>
            <p class="libmeta"><span><b>Material</b> {x().material}</span><span><b>Flange ratings</b> {x().ratings.map(r => `Class ${r}`).join(", ")}</span><span><b>Sizes</b> {inch(x().sizes[0] ?? "")} to {inch(x().sizes[x().sizes.length - 1] ?? "")}</span><span><b>Elbows</b> {x().elbow === "short" ? "short radius" : "long radius"}</span><span><b>Lines using it</b> {used(x().name)}</span></p>
            <div class="libcols">
              <section><h4>Wall by size</h4>
                <Tbl head={["Size", "Schedule", "OD", "Wall"]} right={[2, 3]} rows={x().sizes.filter(s => has(s, inch(s))).map(s => { const sch = schedOf(x(), s); const p = cat()!.pipes.find(r => r.nps === s && r.schedule === sch); return [inch(s), sch, p ? mm(p.od) : "—", p ? mm(p.wall) : "not in the pipe table"]; })} /></section>
              <section><h4>Components allowed</h4>
                <Tbl head={["Component", "From", "To"]} rows={x().components.filter(k => has(k.kind, IT_NAME[k.kind] ?? "")).map(k => [IT_NAME[k.kind] ?? k.kind, k.min ? inch(k.min) : inch(x().sizes[0] ?? ""), k.max ? inch(k.max) : inch(x().sizes[x().sizes.length - 1] ?? "")])} />
                <h4>Branch table</h4>
                <Show when={x().branches?.length} fallback={<p class="small muted">No branch table: any tee is accepted. A branch larger than its header is reported by the checks.</p>}>
                  <Tbl head={["Header", "Branch", "Fitting"]} rows={x().branches!.map(r => [r.header_min || r.header_max ? `${r.header_min ? inch(r.header_min) : "any"} to ${r.header_max ? inch(r.header_max) : "any"}` : "any", r.branch === "same" ? "the same size" : r.branch_min || r.branch_max ? `${r.branch_min ? (r.branch_min === "half" ? "over half the header" : inch(r.branch_min)) : "any"} to ${r.branch_max ? (r.branch_max === "half" ? "half the header" : r.branch_max === "one_smaller" ? "one size smaller" : inch(r.branch_max)) : "any"}` : "any other", ({ tee: "Tee", reducing_tee: "Reducing tee", weldolet: "Weldolet", sockolet: "Sockolet" } as Record<string, string>)[r.fitting] ?? r.fitting])} />
                  <p class="small muted">The first row that fits gives the fitting. Routing a branch off a tee makes that fitting; the checks report any branch made otherwise.</p>
                </Show></section>
            </div>
            <p class="small libsrc"><b>Source:</b> {x().source}</p>
            <p class="ibtns"><button class="btn" disabled={!project()} title="Change this class for this plant (the plant keeps its own copy)" onClick={() => setEditing({ start: x(), was: x().name })}>Edit…</button><button class="btn" disabled={!project()} title="A new class starting from this one" onClick={() => setEditing({ start: { ...x(), name: copyName(x().name), source: "" }, was: null })}>Copy as a new class…</button><span class="grow" /><button class="btn pri" disabled={!project()} onClick={() => { const p = project(); if (p) act({ SetSettings: { ...p.settings, class: x().name, rating: x().ratings.includes(p.settings.rating) ? p.settings.rating : x().ratings[0] } }); }}>Use for new lines</button></p>
          </>}</Show>
          <Show when={!editing() && page() === "pipe"}>
            <h3>Pipe <small class="muted">outside diameter and wall by size and schedule</small></h3>
            <Tbl head={["Size", "DN", "Schedule", "OD", "Wall", "Source"]} right={[1, 3, 4]} rows={[...(cat()?.pipes ?? [])].sort(byNps).filter(r => has(r.nps, inch(r.nps), r.dn, r.schedule)).map(r => [inch(r.nps), String(r.dn), r.schedule, mm(r.od), mm(r.wall), <small class="muted src" title={r.source}>{r.source}</small>])} /></Show>
          <Show when={!editing() && page() === "fittings"}>
            <h3>Butt-weld fittings <small class="muted">centre-to-end and end-to-end</small></h3>
            <Tbl head={["Size", "Elbow LR", "Elbow SR", "Tee", "Reducer", "Cap", "Source"]} right={[1, 2, 3, 4, 5]} rows={[...(cat()?.fittings ?? [])].sort(byNps).filter((r: any) => has(r.nps, inch(r.nps))).map((r: any) => [inch(r.nps), mm(r.elbow_lr), r.elbow_sr ? mm(r.elbow_sr) : "—", mm(r.tee_c), mm(r.reducer_h), mm(r.cap_e), <small class="muted src" title={r.source}>{r.source}</small>])} /></Show>
          <Show when={!editing() && page() === "flanges"}>
            <h3>Flanges <small class="muted">weld-neck and blind, by size and class</small></h3>
            <Tbl head={["Size", "Class", "OD", "Thickness", "Blind", "Length", "Bolts", "Gasket", "Source"]} right={[2, 3, 4, 5, 7]} rows={[...(cat()?.flanges ?? [])].sort((a: any, b: any) => a.rating - b.rating || byNps(a, b)).filter((r: any) => has(r.nps, inch(r.nps), r.rating, `class ${r.rating}`)).map((r: any) => [inch(r.nps), String(r.rating), mm(r.od), mm(r.thick), r.blind_thick ? mm(r.blind_thick) : "—", mm(r.wn_length), `${r.bolts} × ${mm(r.bolt)}`, mm(r.gasket), <small class="muted src" title={r.source}>{r.source}</small>])} /></Show>
          <Show when={!editing() && page() === "tees"}>
            <h3>Reducing tees <small class="muted">run × branch: centre to end of the run (C) and of the branch (M)</small></h3>
            <Tbl head={["Run", "Branch", "C", "M", "Source"]} right={[2, 3]} rows={(cat()?.tees ?? []).filter(r => has(r.nps, inch(r.nps), r.nps_branch, inch(r.nps_branch))).map(r => [inch(r.nps), inch(r.nps_branch), mm(r.c), mm(r.m), <small class="muted src" title={r.source}>{r.source}</small>])} /></Show>
          <Show when={!editing() && page() === "reducers"}>
            <h3>Reducers <small class="muted">large × small: end-to-end length, concentric and eccentric alike</small></h3>
            <Tbl head={["Large end", "Small end", "Length", "Source"]} right={[2]} rows={(cat()?.reducers ?? []).filter(r => has(r.nps, inch(r.nps), r.nps_small, inch(r.nps_small))).map(r => [inch(r.nps), inch(r.nps_small), mm(r.h), <small class="muted src" title={r.source}>{r.source}</small>])} /></Show>
          <Show when={!editing() && page() === "olets"}>
            <h3>Branch outlets <small class="muted">weldolets (standard weight) and sockolets (class 3000): height above the header (A), base and outlet diameters, weight</small></h3>
            <Tbl head={["Kind", "Fits", "Branch", "Height", "Base", "Outlet", "Weight", "Source"]} right={[3, 4, 5, 6]} rows={(cat()?.olets ?? []).filter(r => has(r.kind, r.nps_branch, inch(r.nps_branch))).map(r => [r.kind === "weldolet" ? "Weldolet" : "Sockolet", r.same ? "size on size" : "smaller than the header", inch(r.nps_branch), mm(r.height), mm(r.base_od), mm(r.top_od), r.weight_lb != null ? fmt("mass", r.weight_lb * 0.45359237) : "—", <small class="muted src" title={r.source}>{r.source}</small>])} /></Show>
          <Show when={!editing() && page() === "valves"}>
            <h3>Valves and strainers <small class="muted">face to face, flanged, by size and class</small></h3>
            <Tbl head={["Type", "Size", "Class", "Face to face", "Source"]} right={[3]} rows={[...(cat()?.valves ?? [])].sort((a: any, b: any) => a.kind.localeCompare(b.kind) || a.rating - b.rating || byNps(a, b)).filter((r: any) => has(r.kind, VALVE[r.kind] ?? "", r.nps, inch(r.nps), r.rating)).map((r: any) => [VALVE[r.kind] ?? r.kind, inch(r.nps), String(r.rating), mm(r.f2f), <small class="muted src" title={r.source}>{r.source}</small>])} /></Show>
          <Show when={!editing() && page() === "weights"}>
            <h3>Weights <small class="muted">fittings by wall (STD, XS), flanges and valves by class; pipe is weighed from its diameter and wall</small></h3>
            <p class="small muted">A schedule whose wall is neither STD nor XS is scaled by its wall from the nearer of the two (marked ≈ in the take-off). Weights are the makers' approximate figures: check them with your supplier.</p>
            <Tbl head={["Part", "Size", "Wall or class", "Weight", "Source"]} right={[3]} rows={(cat()?.weights ?? []).filter(r => has(WKIND[r.kind] ?? r.kind, r.nps, inch(r.nps), r.nps_small, r.wall, r.rating || "")).map(r => [WKIND[r.kind] ?? r.kind, r.nps_small ? `${inch(r.nps)} × ${inch(r.nps_small)}` : inch(r.nps), r.wall || `class ${r.rating}`, fmt("mass", r.kg), <small class="muted src" title={r.source}>{r.source}</small>])} /></Show>
          <p class="muted small">Every part is generated from these tables: nothing is downloaded or bundled, and no standard's tables or text are reproduced. Check dimensions against your supplier's data before ordering or building.</p>
        </div>
      </div>
    </Dialog>
  );
});
