// Datum Layout window: the catalogue on the left, the 3D view in the centre with the Lines,
// Equipment, Clashes and Checks tabs below it, the Inspector on the right; title bar, ribbon,
// status bar, keyboard shortcuts, start-up and recovery.
import { render } from "solid-js/web";
import { For, Show, onMount, createSignal } from "solid-js";
import "@fontsource/ibm-plex-sans/latin-400.css";
import "@fontsource/ibm-plex-sans/latin-500.css";
import "@fontsource/ibm-plex-sans/latin-600.css";
import "@fontsource/ibm-plex-sans/latin-700.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import "@fontsource/ibm-plex-mono/latin-500.css";
import "@fontsource/ibm-plex-mono/latin-600.css";
import "./styles.css";
import { invoke, listen, Out, Options, Shown, Typed } from "./api";
import { picked, out, setOut, backstage, setBackstage, setOpts, opts, dialogs, toast, loadOut, setThemeRaw, applyTheme, sel, project, mode, elev, scene, checked } from "./store";
import { C } from "./cmds";
import { TitleBar, Ribbon, palette } from "./ribbon";
import { Backstage, update, checkUpdate } from "./backstage";
import { View3D } from "./view3d";
import { CataloguePanel, Inspector, BottomPanel } from "./panels";
import { Menu, Toast } from "./ui";
import { setUnitTable, fmt } from "./units";
import * as D from "./dialogs";

const Status = () => (
  <footer class="status">
    <span>{mode() === "route" ? "Routing" : mode() === "place" ? "Placing" : mode() === "measure" ? "Measuring" : "Ready"}</span>
    <span class="muted">{project() ? `${project()!.equipment.length} equipment · ${project()!.lines.length} lines · ${scene()?.parts.filter(p => p.line).length ?? 0} piping parts` : ""}</span>
    <Show when={checked()}><span class="muted">{checked()!.length ? `${checked()!.length} findings` : "Checked: clean"}</span></Show>
    <span class="grow" />
    <span class="muted">Working elevation {fmt("coord", elev())}</span>
    <span class="muted">{project()?.units === "US" ? "US units" : "SI units"}</span>
    <span class="muted">{picked().length ? `${picked().length} selected` : sel() ? sel()!.kind : ""}</span>
  </footer>
);

const App = () => {
  const [bh, setBh] = createSignal(210);
  onMount(async () => {
    // For the UI tests: open an example by id (the speed check opens a large plant this way).
    (window as any).__open = (id: string) => C.newProject(id);
    try {
      const o = await invoke<Options>("options"); setOpts(o); setThemeRaw(o.theme || "auto"); applyTheme();
      setUnitTable(await invoke<[Shown[], Shown[], Typed[]]>("unit_table"));
      setOut(await invoke<Out>("state"));
      const start = await invoke<string | null>("startup_file");
      if (start) await loadOut("open", { path: start });
      const rec = await invoke<{ name: string; saved: string } | null>("recovery");
      if (rec) D.recoverDialog(rec.name, rec.saved, () => setBackstage(null));
      checkUpdate(true);
      setInterval(() => { if (opts().autosave > 0) invoke("autosave").catch(() => {}); }, Math.max(1, opts().autosave) * 60000);
      listen("close-requested", () => D.confirmClose(() => invoke("quit")));
    } catch (e) { toast(String(e)); }
  });
  const keys = (e: KeyboardEvent) => {
    const ctrl = e.ctrlKey || e.metaKey, k = e.key.toLowerCase();
    if (dialogs().length) return;
    const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement;
    const grid = project()?.plot.grid ?? 1;
    if ((ctrl && k === "k") || (e.altKey && k === "q")) { e.preventDefault(); palette(); }
    else if (ctrl && k === "s") { e.preventDefault(); C.save(e.shiftKey); }
    else if (ctrl && k === "o") { e.preventDefault(); C.open(); }
    else if (ctrl && k === "n") { e.preventDefault(); setBackstage("home"); }
    else if (ctrl && k === "p") { e.preventDefault(); C.print(); }
    else if (ctrl && k === "z" && !typing) { e.preventDefault(); C.undo(); }
    else if (ctrl && k === "y" && !typing) { e.preventDefault(); C.redo(); }
    else if (e.key === "F1") { e.preventDefault(); D.shortcuts(); }
    else if (e.key === "F5") { e.preventDefault(); C.check(); }
    else if (e.key === "Escape" && backstage() && out()) setBackstage(null);
    else if (e.key === "Escape" && mode() !== "select") C.cancelTool();
    else if (ctrl && k === "a" && !typing && !backstage()) { e.preventDefault(); C.selectAll(); }
    else if (ctrl && k === "d" && !typing && !backstage()) { e.preventDefault(); C.copyGroup(); }
    else if (typing || ctrl || backstage()) return;
    else if (e.key === "Delete") { e.preventDefault(); C.del(); }
    else if (e.key === "PageUp") { e.preventDefault(); C.elevation(grid); }
    else if (e.key === "PageDown") { e.preventDefault(); C.elevation(-grid); }
    else if (k === "r") C.route();
    else if (k === "m") C.measure();
    else if (k === "h") { if (e.shiftKey) C.showAll(); else C.hide(); }
    else if (k === "i") C.isolate();
    else if (k === "j") C.jog();
    else if (k === "f") C.fit();
    else if ("1234".includes(k) && k.length === 1) C.std(["iso", "plan", "front", "side"][+k - 1]);
  };
  const drag = (e: PointerEvent) => {
    const y0 = e.clientY, h0 = bh();
    const move = (m: PointerEvent) => setBh(Math.max(90, Math.min(window.innerHeight * 0.6, h0 - (m.clientY - y0))));
    const up = () => { removeEventListener("pointermove", move); removeEventListener("pointerup", up); };
    addEventListener("pointermove", move); addEventListener("pointerup", up);
  };
  return (
    <div class="app" ref={() => document.addEventListener("keydown", keys)}>
      <TitleBar update={update()?.available ? update()!.latest : null} />
      <Ribbon />
      <main class="work">
        <Show when={out()}>
          <CataloguePanel />
          <div class="center">
            <View3D />
            <div class="bhandle" onPointerDown={drag} />
            <div style={{ height: `${bh()}px`, flex: "none", display: "flex" }}><BottomPanel /></div>
          </div>
          <Inspector />
        </Show>
      </main>
      <Status />
      <Show when={backstage()}><Backstage /></Show>
      <For each={dialogs()}>{d => d()}</For>
      <Menu /><Toast />
    </div>
  );
};

render(() => <App />, document.getElementById("app")!);
