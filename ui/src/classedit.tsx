// The pipe class editor (0.3.0): a class written or changed in the Library, kept in the plant's
// file. The engine checks it (every size has a pipe in its schedule, every component and fitting
// is one it makes) and says what is wrong; nothing here decides a rule.
import { For, Show, createSignal, createMemo, JSX } from "solid-js";
import { createStore, produce } from "solid-js/store";
import { Catalogue, PipeClass, BranchRule, Out, inch, NPS, invoke } from "./api";
import { act, setOut } from "./store";

const RATINGS = [150, 300, 400, 600, 900, 1500, 2500];
const KINDS: [string, string][] = [["pipe", "Pipe"], ["elbow90", "Elbow 90°"], ["elbow45", "Elbow 45°"], ["tee", "Tee"], ["reducer", "Reducer"], ["cap", "Cap"], ["flange", "Weld-neck flange"], ["blind", "Blind flange"], ["gasket", "Gasket"], ["bolts", "Stud bolts"],
  ["gate", "Gate valve"], ["globe", "Globe valve"], ["check", "Check valve"], ["ball", "Ball valve"], ["butterfly", "Butterfly valve"], ["strainer", "Strainer"], ["cross", "Cross"], ["weldolet", "Weldolet"], ["sockolet", "Sockolet"], ["support", "Pipe support"]];
const FITTINGS: [string, string][] = [["tee", "Tee"], ["reducing_tee", "Reducing tee"], ["weldolet", "Weldolet"], ["sockolet", "Sockolet"]];
const kindName = (k: string) => KINDS.find(x => x[0] === k)?.[1] ?? k;
const byNps = (a: string, b: string) => NPS.indexOf(a) - NPS.indexOf(b);
/** A copy safe to edit, with every list present. */
const draftOf = (c: PipeClass): PipeClass => JSON.parse(JSON.stringify({ ...c, branches: c.branches ?? [], components: c.components ?? [], schedules: c.schedules ?? [] }));

/** A select of sizes with an "any" choice (null) and, for a branch bound, the words the engine knows. */
const SizeSel = (p: { value: string | null | undefined; set: (v: string | null) => void; sizes: string[]; any?: string; extra?: [string, string][]; title?: string }) => (
  <select value={p.value ?? ""} title={p.title} onChange={e => p.set(e.currentTarget.value || null)}>
    <option value="">{p.any ?? "any"}</option>
    <For each={p.extra ?? []}>{([v, l]) => <option value={v}>{l}</option>}</For>
    <For each={p.sizes}>{s => <option value={s}>{inch(s)}</option>}</For>
  </select>
);

/** The editor, in place of the class page. `was` is the class being changed (null: a new one). */
export const ClassEditor = (p: { start: PipeClass; was: string | null; cat: Catalogue; own: boolean; builtin: boolean; done: (name: string | null) => void }) => {
  const [d, setD] = createStore<PipeClass>(draftOf(p.start));
  const [err, setErr] = createSignal("");
  const [busy, setBusy] = createSignal(false);
  const sizes = () => [...d.sizes].sort(byNps);
  /** The schedules the pipe table has for a size. */
  const schedsFor = (s: string) => [...new Set(p.cat.pipes.filter(r => r.nps === s).map(r => r.schedule))];
  const allScheds = createMemo(() => [...new Set(p.cat.pipes.map(r => r.schedule))]);
  const schedOf = (s: string) => [...d.schedules].sort((a, b) => byNps(a[0], b[0])).reverse().find(([from]) => NPS.indexOf(from) <= NPS.indexOf(s))?.[1] ?? "STD";
  const wall = (s: string) => p.cat.pipes.find(r => r.nps === s && r.schedule === schedOf(s));
  const toggleSize = (s: string) => setD("sizes", v => v.includes(s) ? v.filter(x => x !== s) : [...v, s].sort(byNps));
  const toggleRating = (r: number) => setD("ratings", v => v.includes(r) ? v.filter(x => x !== r) : [...v, r].sort((a, b) => a - b));
  const move = (i: number, by: number) => setD("branches", produce(b => { const j = i + by; if (j < 0 || j >= b!.length) return; [b![i], b![j]] = [b![j], b![i]]; }));
  const save = async () => {
    setErr(""); setBusy(true);
    const cls: PipeClass = JSON.parse(JSON.stringify(d));
    cls.schedules = [...cls.schedules].sort((a, b) => byNps(a[0], b[0]));
    try {
      setOut(await invoke<Out>("apply", { op: { SetClass: { class: cls, was: p.was } } }));
      p.done(cls.name.trim());
    } catch (e) { setErr(String(e)); } finally { setBusy(false); }
  };
  const remove = async () => { if (p.was && await act({ DeleteClass: { name: p.was } })) p.done(p.builtin ? p.was : null); };
  const Field = (f: { l: string; children: JSX.Element; hint?: string }) => <label class="cefield"><span>{f.l}</span>{f.children}<Show when={f.hint}><small class="muted">{f.hint}</small></Show></label>;
  return (
    <div class="cedit">
      <h3>{p.was ? <>Edit class <b>{p.was}</b></> : "New pipe class"}<small class="muted">{p.own ? "this plant's own class" : p.builtin && p.was ? "saved as this plant's own copy; the catalogue's stays as it is" : "kept in this plant's file"}</small></h3>
      <section class="cegrid">
        <Field l="Name"><input value={d.name} onInput={e => setD("name", e.currentTarget.value)} maxLength={40} /></Field>
        <Field l="Material"><input value={d.material} onInput={e => setD("material", e.currentTarget.value)} placeholder="Carbon steel" /></Field>
        <Field l="Description"><input value={d.description} onInput={e => setD("description", e.currentTarget.value)} placeholder="What the class is for" /></Field>
        <Field l="Elbows"><select value={d.elbow || "long"} onChange={e => setD("elbow", e.currentTarget.value)}><option value="long">Long radius</option><option value="short">Short radius</option></select></Field>
        <Field l="Source" hint="Who wrote the class, or the specification it follows"><input value={d.source} onInput={e => setD("source", e.currentTarget.value)} placeholder="Written for this plant" /></Field>
      </section>
      <h4>Flange ratings</h4>
      <div class="chips"><For each={RATINGS}>{r => <button classList={{ on: d.ratings.includes(r) }} onClick={() => toggleRating(r)}>Class {r}</button>}</For></div>
      <h4>Sizes <small class="muted">{d.sizes.length ? `${inch(sizes()[0])} to ${inch(sizes()[sizes().length - 1])}, ${d.sizes.length} sizes` : "none chosen"}</small></h4>
      <div class="chips sizes"><For each={NPS}>{s => <button classList={{ on: d.sizes.includes(s) }} onClick={() => toggleSize(s)}>{inch(s)}</button>}</For></div>
      <div class="libcols">
        <section>
          <h4>Schedules <small class="muted">each applies from its size up to the next row</small></h4>
          <table class="tbl cetbl cesched"><thead><tr><th>From size</th><th>Schedule</th><th /></tr></thead><tbody>
            <For each={d.schedules}>{(r, i) => <tr>
              <td><select value={r[0]} onChange={e => setD("schedules", i(), 0, e.currentTarget.value)}><For each={NPS}>{s => <option value={s}>{inch(s)}</option>}</For></select></td>
              <td><select value={r[1]} onChange={e => setD("schedules", i(), 1, e.currentTarget.value)}><For each={schedsFor(r[0]).length ? schedsFor(r[0]) : allScheds()}>{s => <option value={s}>{s}</option>}</For></select></td>
              <td class="r"><button class="ceb" title="Remove this row" onClick={() => setD("schedules", v => v.filter((_, k) => k !== i()))}>✕</button></td></tr>}</For>
          </tbody></table>
          <button class="btn sm" onClick={() => setD("schedules", v => [...v, [sizes().find(s => !v.some(x => x[0] === s)) ?? NPS[0], "STD"]])}>Add a schedule</button>
          <h4>Wall by size</h4>
          <table class="tbl cetbl"><thead><tr><th>Size</th><th>Schedule</th><th class="r">Wall</th></tr></thead><tbody>
            <For each={sizes()}>{s => <tr classList={{ bad: !wall(s) }}><td>{inch(s)}</td><td>{schedOf(s)}</td><td class="r mono">{wall(s) ? `${(wall(s)!.wall * 1000).toFixed(2)} mm` : "no such pipe"}</td></tr>}</For>
          </tbody></table>
        </section>
        <section>
          <h4>Components allowed <small class="muted">and the sizes each is allowed in</small></h4>
          <table class="tbl cetbl"><thead><tr><th>Component</th><th>From</th><th>To</th><th /></tr></thead><tbody>
            <For each={d.components}>{(k, i) => <tr>
              <td><select value={k.kind} onChange={e => setD("components", i(), "kind", e.currentTarget.value)}><For each={KINDS}>{([v, l]) => <option value={v}>{l}</option>}</For></select></td>
              <td><SizeSel value={k.min} set={v => setD("components", i(), "min", v)} sizes={sizes()} any="smallest" /></td>
              <td><SizeSel value={k.max} set={v => setD("components", i(), "max", v)} sizes={sizes()} any="largest" /></td>
              <td class="r"><button class="ceb" title={`Remove ${kindName(k.kind).toLowerCase()}`} onClick={() => setD("components", v => v.filter((_, j) => j !== i()))}>✕</button></td></tr>}</For>
          </tbody></table>
          <button class="btn sm" onClick={() => setD("components", v => [...v, { kind: KINDS.find(([k]) => !v.some(x => x.kind === k))?.[0] ?? "gate", min: null, max: null }])}>Add a component</button>
          <h4>Branch table <small class="muted">the first row that fits gives the fitting</small></h4>
          <table class="tbl cetbl"><thead><tr><th>Header from</th><th>to</th><th>Branch</th><th>from</th><th>to</th><th>Fitting</th><th /></tr></thead><tbody>
            <For each={d.branches ?? []}>{(r: BranchRule, i) => <tr>
              <td><SizeSel value={r.header_min} set={v => setD("branches", i(), "header_min", v)} sizes={sizes()} /></td>
              <td><SizeSel value={r.header_max} set={v => setD("branches", i(), "header_max", v)} sizes={sizes()} /></td>
              <td><select value={r.branch === "same" ? "same" : ""} onChange={e => setD("branches", i(), "branch", e.currentTarget.value || null)}><option value="">a range</option><option value="same">same size</option></select></td>
              <td><Show when={r.branch !== "same"} fallback="—"><SizeSel value={r.branch_min} set={v => setD("branches", i(), "branch_min", v)} sizes={sizes()} extra={[["half", "over half the header"]]} /></Show></td>
              <td><Show when={r.branch !== "same"} fallback="—"><SizeSel value={r.branch_max} set={v => setD("branches", i(), "branch_max", v)} sizes={sizes()} extra={[["half", "half the header"], ["one_smaller", "one size smaller"]]} /></Show></td>
              <td><select value={r.fitting} onChange={e => setD("branches", i(), "fitting", e.currentTarget.value)}><For each={FITTINGS}>{([v, l]) => <option value={v}>{l}</option>}</For></select></td>
              <td class="r nowrap"><button class="ceb" title="Up" disabled={i() === 0} onClick={() => move(i(), -1)}>↑</button><button class="ceb" title="Down" disabled={i() === (d.branches?.length ?? 0) - 1} onClick={() => move(i(), 1)}>↓</button><button class="ceb" title="Remove this rule" onClick={() => setD("branches", v => (v ?? []).filter((_, j) => j !== i()))}>✕</button></td></tr>}</For>
          </tbody></table>
          <button class="btn sm" onClick={() => setD("branches", v => [...(v ?? []), { header_min: null, header_max: null, branch: null, branch_min: null, branch_max: null, fitting: "reducing_tee" }])}>Add a rule</button>
        </section>
      </div>
      <Show when={err()}><p class="ceerr" role="alert">{err()}</p></Show>
      <div class="cefoot">
        <Show when={p.own && p.was}><button class="btn" onClick={remove} title={p.builtin ? "Remove this plant's copy: the catalogue's class applies again" : "Remove the class from this plant"}>{p.builtin ? "Back to the catalogue's" : "Remove from the plant"}</button></Show>
        <span class="grow" />
        <button class="btn" onClick={() => p.done(null)}>Cancel</button>
        <button class="btn pri" disabled={busy()} onClick={save}>Save to this plant</button>
      </div>
    </div>
  );
};
