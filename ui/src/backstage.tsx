// File (backstage): home with examples and recent plants, export, print, options, roadmap, updates.
import { For, Show, createSignal, createResource, onMount, JSX } from "solid-js";
import { PAPERS, PrintPage } from "./print";
import { RoadmapPage } from "./roadmap";
import { Options, Out, Example, listen, invoke } from "./api";
import { backstage, setBackstage, call, out, setOut, opts, setOpts, toast, setThemeRaw, applyTheme } from "./store";
import { C } from "./cmds";
import * as D from "./dialogs";
import { Icon, Text, Pick, Check } from "./ui";

type Info = { current: string; latest: string | null; beta: boolean; available: boolean; notes: string; published: string; url: string | null; sums: string | null; size: number; page: string };
export const [update, setUpdate] = createSignal<Info | null>(null);
export const [checking, setChecking] = createSignal(false), [checkedAt, setCheckedAt] = createSignal("");
export const checkUpdate = async (quiet: boolean) => {
  setChecking(true);
  try { setUpdate(await invoke<Info>("check_update")); setCheckedAt(new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })); if (!quiet) toast(update()?.available ? `Version ${update()!.latest} is available.` : "You have the latest version.", true); }
  catch (e) { if (!quiet) toast(`Could not check for updates: ${e}`); }
  finally { setChecking(false); }
};
const GH = "https://github.com/Edgar-Mendonca/Datum-Layout";
export const GitHubLinks = () => (
  <div class="ghlinks">
    <For each={[["", "GitHub", "The project and its code"], ["/releases", "Releases", "Every version and its change log"], ["/issues/new", "Report an issue", "Tell us about a bug or an idea"]]}>{([path, l, t]) =>
      <button class="btn" title={t} onClick={() => call("open_url", { url: GH + path })}><svg class="ic"><use href={path === "/issues/new" ? "#i-bug" : path ? "#i-tag" : "#i-github"} /></svg>{l}</button>}</For>
  </div>
);
const md = (s: string) => s.split("\n").map(l => {
  const e = l.replace(/[&<>]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]!).replace(/\*\*(.+?)\*\*/g, "<b>$1</b>").replace(/`(.+?)`/g, "<code>$1</code>");
  return l.startsWith("## ") ? `<h3>${e.slice(3)}</h3>` : l.startsWith("- ") ? `<li>${e.slice(2)}</li>` : e.trim() ? `<p>${e}</p>` : "";
}).join("");

/** A small drawing for each example tile. */
const ExampleArt = (p: { id: string }) => {
  const s = { fill: "none", stroke: "var(--blue)", "stroke-width": 2, "stroke-linecap": "round" as const, "stroke-linejoin": "round" as const };
  const pics: Record<string, string> = {
    blank: "M10 50h80M10 50l14-10h80M24 40V10",
    pumps: "M6 14h20v36H6zM26 44H52Q58 44 58 38V30M52 30h12M58 30V16H90M70 50h14v-8H70z",
    exchanger: "M30 26h44v12H30zM74 26h8v12h-8zM40 26V12H10M64 38v12h26M20 12v-6M20 12V22H84V26",
    rack: "M8 50V14M92 50V14M8 22H92M8 30H92M40 22v28M60 22v28M4 50h92",
    clashes: "M8 22H92M50 6V48M44 16l12 12M56 16L44 28",
  };
  return <svg viewBox="0 0 100 60" class="exart"><path d={pics[p.id] ?? pics.blank} {...s} /></svg>;
};

const Home = () => {
  const [ex] = createResource(() => call<Example[]>("examples_list"));
  const [recent] = createResource(() => call<{ path: string; name: string; modified: string }[]>("recent"));
  return (
    <div class="home">
      <div>
        <h1>Good {new Date().getHours() < 12 ? "morning" : new Date().getHours() < 18 ? "afternoon" : "evening"}</h1>
        <div class="tiles">
          <button class="tile" onClick={() => C.newProject(null)}><div class="pic"><ExampleArt id="blank" /></div><b>Empty plot</b><span>A new plant: place equipment and route lines.</span></button>
          <For each={ex() ?? []}>{t => <button class="tile" onClick={() => C.newProject(t.id)}><div class="pic"><ExampleArt id={t.id} /></div><b>{t.name}</b><span>{t.what}</span></button>}</For>
          <button class="tile" onClick={() => setBackstage("roadmap")}><div class="pic"><svg class="ic"><use href="#i-list" /></svg></div><b>Roadmap</b><span>Everything Datum Layout will do, and when.</span></button>
        </div>
        <h2>Recent</h2>
        <table class="recent"><tbody>
          <For each={recent() ?? []} fallback={<tr><td class="muted">Plants you open or save appear here.</td></tr>}>{r => <tr class="f" onClick={() => C.openPath(r.path)}><td><b>{r.name}</b><div class="p">{r.path}</div></td><td class="p">{r.modified}</td></tr>}</For>
        </tbody></table>
      </div>
      <UpdateCard />
    </div>
  );
};

const UpdateCard = () => {
  const [pct, setPct] = createSignal<number | null>(null);
  const [ver] = createResource(() => call<string>("version"));
  onMount(() => { listen("update-progress", (p: number) => setPct(p)); if (!update()) checkUpdate(true); });
  const install = async () => { const u = update(); if (!u?.url) return; setPct(0); await call("install_update", { url: u.url, sums: u.sums }); setPct(null); };
  return (
    <div class="card">
      <div class="brand"><img src="logo.svg" alt="" /><div><b>Datum Layout</b><span class="muted">Version {ver()}{opts().beta ? " · beta channel" : ""}</span></div></div>
      <Show when={update()} fallback={<><span class="pill">{checking() ? "Checking for updates…" : "Not checked yet"}</span><button class="btn wide" disabled={checking()} onClick={() => checkUpdate(false)}>{checking() ? <><i class="spin" /> Checking…</> : "Check for updates"}</button><GitHubLinks /></>}>{u => <>
        <Show when={u().available} fallback={<span class="pill ok">You have the latest version</span>}><span class="pill warn">Version {u().latest}{u().beta ? " (beta)" : ""} is ready</span></Show>
        <Show when={u().available && u().url}><button class="btn pri wide" disabled={pct() !== null} onClick={install}>{pct() === null ? `Update now (${Math.round(u().size / 1048576)} MB)` : `Downloading… ${pct()}%`}</button></Show>
        <Show when={pct() !== null}><div class="prog"><i style={{ width: `${pct()}%` }} /></div></Show>
        <button class="btn wide" disabled={checking()} onClick={() => checkUpdate(false)}>{checking() ? <><i class="spin" /> Checking…</> : "Check again"}</button>
        <Show when={checkedAt() && !checking()}><span class="muted small">Checked at {checkedAt()}</span></Show>
        <GitHubLinks />
        <div class="notes"><h4>What's new{u().latest ? ` in ${u().latest}` : ""}</h4><div innerHTML={md(u().notes || "Release notes will appear here.")} /></div>
      </>}</Show>
    </div>
  );
};

const Export = () => (
  <div><h1>Export</h1>
    <div class="tiles">
      <button class="tile" onClick={() => C.exportPcf()}><div class="pic"><svg class="ic"><use href="#i-pipe" /></svg></div><b>Every line for Datum Pipe</b><span>A piping component file (.pcf) with every line, for Datum Pipe's isometric and stress analysis. One line: select it, then Home › To Datum Pipe.</span></button>
      <button class="tile" onClick={C.exportGltf}><div class="pic"><svg class="ic"><use href="#i-cube" /></svg></div><b>3D model (glTF)</b><span>The whole plant as a glTF 2.0 file (.glb) that opens in any 3D viewer.</span></button>
      <button class="tile" onClick={() => C.exportCsv("lines")}><div class="pic"><svg class="ic"><use href="#i-csv" /></svg></div><b>Line list (CSV)</b><span>From, to, size, class, length and components of every line.</span></button>
      <button class="tile" onClick={() => C.exportCsv("equipment")}><div class="pic"><svg class="ic"><use href="#i-csv" /></svg></div><b>Equipment list (CSV)</b><span>Every item with its nozzles, sizes, ratings and positions.</span></button>
      <button class="tile" onClick={() => C.exportCsv("mto")}><div class="pic"><svg class="ic"><use href="#i-csv" /></svg></div><b>Material take-off (CSV)</b><span>Pipe, fittings, flanges, valves, gaskets and bolts per line.</span></button>
      <button class="tile" onClick={() => setBackstage("print")}><div class="pic"><svg class="ic"><use href="#i-print" /></svg></div><b>Print or save as PDF</b><span>The 3D view and the lists, with a header and footer.</span></button>
    </div>
  </div>
);

const Opt = (p: { l: string; hint?: string; children: JSX.Element }) => <div class="opt"><div><b>{p.l}</b>{p.hint && <small>{p.hint}</small>}</div><div class="ctl">{p.children}</div></div>;
const Sec = (p: { t: string; about: string; children: JSX.Element }) => <section class="osec"><h3>{p.t}</h3><p class="muted">{p.about}</p>{p.children}</section>;

const OptionsPage = () => {
  const [o, setO] = createSignal<Options>({ ...opts() });
  const save = async () => { const r = await call<Out>("set_options", { opts: o() }); if (r) { setOpts(o()); setOut(r); toast("Options saved.", true); } };
  const put = <K extends keyof Options>(k: K, v: Options[K]) => setO({ ...o(), [k]: v });
  return (
    <div class="options"><h1>Options</h1>
      <p class="muted">Settings for Datum Layout on this computer. A plant's own settings (units, clearance, plot) are in the Inspector when nothing is selected.</p>
      <Sec t="General" about="How the window looks.">
        <Opt l="Theme" hint="Auto follows the Windows light or dark setting."><Pick value={o().theme || "auto"} set={v => { put("theme", v); setThemeRaw(v); applyTheme(); }} options={[["auto", "Auto (follow Windows)"], ["light", "Light"], ["dark", "Dark"]]} /></Opt>
        <Opt l="3D detail" hint="How smooth round parts are. Low keeps very large plants quick to orbit."><Pick value={String(o().lod ?? 1)} set={v => put("lod", +v)} options={[["0", "Low"], ["1", "Normal"], ["2", "High"]]} /></Opt>
        <Opt l="Labels" hint="Equipment tags and line numbers in the 3D view."><Check label="Show tags and line numbers" value={o().labels} set={v => put("labels", v)} /></Opt>
      </Sec>
      <Sec t="Start-up and saving" about="What happens when Datum Layout starts, and how your work is protected.">
        <Opt l="When Datum Layout starts" hint="Show the File page, or reopen the plant you used last."><Pick value={o().openLast ? "last" : "home"} set={v => put("openLast", v === "last")} options={[["home", "Show the File page"], ["last", "Reopen the last plant"]]} /></Opt>
        <Opt l="Recovery copy every" hint="A copy is kept while you work, so a crash or power cut loses little. 0 turns it off."><span class="inline"><input type="number" min="0" style={{ width: "70px" }} value={o().autosave} onChange={e => put("autosave", +e.currentTarget.value)} /> minutes</span></Opt>
      </Sec>
      <Sec t="New plants" about="Defaults given to every new plant. Each plant keeps its own settings afterwards.">
        <Opt l="Units" hint="How values are shown and typed. Files always keep metres."><Pick value={o().units} set={v => put("units", v as any)} options={[["SI", "SI: m, mm, bar, °C"], ["US", "US: ft, in, psi, °F"]]} /></Opt>
        <Opt l="Clash clearance" hint="How close parts may come before it is a clash."><span class="inline"><input type="number" min="0" style={{ width: "70px" }} value={Math.round((o().clearance ?? 0.025) * 1000)} onChange={e => put("clearance", Math.max(0, +e.currentTarget.value) / 1000)} /> mm</span></Opt>
      </Sec>
      <Sec t="Printing" about="Defaults for File › Print.">
        <Opt l="Paper" hint="Default paper for printouts."><span class="inline"><Pick value={o().paper || "A4"} set={v => put("paper", v)} options={PAPERS.map(p => p[0])} /><Pick value={o().landscape ? "1" : "0"} set={v => put("landscape", v === "1")} options={[["0", "Portrait"], ["1", "Landscape"]]} /></span></Opt>
        <Opt l="Company" hint="Shown in the header of printouts."><Text value={o().company} set={v => put("company", v)} /></Opt>
      </Sec>
      <Sec t="Updates" about="Datum Layout checks GitHub for new versions when it starts. Updates install from File › Updates.">
        <Opt l="Beta versions" hint="Get new features early. They may have bugs."><Check label="Get beta versions" value={o().beta} set={v => put("beta", v)} /></Opt>
      </Sec>
      <p class="osave"><button class="btn pri" onClick={save}>Save options</button><span class="muted">Datum Layout keeps everything on this computer. Its only connection is the update check to GitHub.</span></p>
    </div>
  );
};

export const Backstage = () => (
  <div class="bs">
    <aside>
      <button class="back" onClick={() => out() && setBackstage(null)} title="Back">←</button>
      <For each={[["home", "home", "Home"], ["new", "new", "New"], ["open", "open", "Open"], ["save", "save", "Save"], ["saveas", "save", "Save As"], ["export", "export", "Export"], ["print", "print", "Print"], ["roadmap", "list", "Roadmap"]]}>{([k, i, l]) =>
        <button classList={{ on: backstage() === k }} onClick={() => {
          if (k === "open") C.open(); else if (k === "save") C.save(); else if (k === "saveas") C.save(true);
          else setBackstage(k === "new" ? "home" : k);
        }}><Icon n={i} size={22} />{l}</button>}</For>
      <span class="grow" />
      <button classList={{ on: backstage() === "options" }} onClick={() => setBackstage("options")}><Icon n="opts" size={22} />Options</button>
      <button classList={{ on: backstage() === "about" }} onClick={() => setBackstage("about")}><Icon n="upd" size={22} />Updates</button>
    </aside>
    <section class="bsmain">
      <Show when={backstage() === "home"}><Home /></Show>
      <Show when={backstage() === "export"}><Export /></Show>
      <Show when={backstage() === "options"}><OptionsPage /></Show>
      <Show when={backstage() === "print"}><PrintPage /></Show>
      <Show when={backstage() === "roadmap"}><RoadmapPage /></Show>
      <Show when={backstage() === "about"}><div class="about"><h1>Updates</h1><UpdateCard /><p><button class="btn" onClick={() => D.shortcuts()}>Keyboard shortcuts</button></p></div></Show>
    </section>
  </div>
);
