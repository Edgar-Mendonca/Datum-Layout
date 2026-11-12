// Datum Layout window: the catalogue on the left, the 3D view in the centre with the Lines,
// Equipment, Clashes and Checks tabs below it, the Inspector on the right; title bar, ribbon,
// status bar, keyboard shortcuts, start-up and recovery.
import { render } from "solid-js/web";
import { For, Show, onMount } from "solid-js";
import "@fontsource/ibm-plex-sans/latin-400.css";
import "@fontsource/ibm-plex-sans/latin-500.css";
import "@fontsource/ibm-plex-sans/latin-600.css";
import "@fontsource/ibm-plex-sans/latin-700.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import "@fontsource/ibm-plex-mono/latin-500.css";
import "@fontsource/ibm-plex-mono/latin-600.css";
import "./styles.css";
import { invoke, listen, Out, Options, Shown, Typed } from "./api";
import { partsOpen, setHintSlot, checkStale, setBottom, setSel, setPicked, picked, out, setOut, backstage, setBackstage, setOpts, opts, dialogs, toast, loadOut, setThemeRaw, applyTheme, sel, project, mode, scene, checked } from "./store";
import { C } from "./cmds";
import { TitleBar, Ribbon, palette } from "./ribbon";
import { Backstage, update, checkUpdate } from "./backstage";
import { View3D } from "./view3d";
import { Inspector, BottomPanel, PartsPanel } from "./panels";
import { Flow, Tree, CtxBar, ToolStrip, grip, treeW, inspW, popShown } from "./shell";
import { Menu, Toast } from "./ui";
import { setUnitTable } from "./units";
import * as D from "./dialogs";

/** Status badges (after Datum Mesh's): the check, routing and the line list, each in words and
 *  kept apart; a click opens what it is about. No badge claims more than its check found. */
const Badges = () => {
  const iss = () => out()?.issues ?? [];
  const routing = () => iss().filter(i => i.kind === "routing").length;
  const list = () => iss().filter(i => i.kind === "list").length;
  const ck = () => checked();
  // One function each (not chained in the markup), so a cleared check never reads as a list.
  const checkTone = (): "ok" | "warn" | "bad" | "idle" => { const c = ck(); return !c ? "idle" : checkStale() ? "warn" : c.length ? "bad" : "ok"; };
  const checkText = () => { const c = ck(); return !c ? "not run" : checkStale() ? "out of date" : c.length ? `${c.length} findings` : "clean"; };
  const B = (p: { k: string; tone: "ok" | "warn" | "bad" | "idle"; label: string; text: string; run: () => void; title: string }) =>
    <button class={`badge ${p.tone}`} data-badge={p.k} title={p.title} onClick={p.run}><i />{p.label} <b>{p.text}</b></button>;
  return <span class="badges">
    <B k="check" label="Check" tone={checkTone()} text={checkText()} title="Clashes, open ends, mismatches, class and design rules (F5 checks again)" run={() => { const c = ck(); if (!c || checkStale()) C.check(); else setBottom(c.some(i => i.kind === "clash") ? "clashes" : "checks"); }} />
    <B k="routing" label="Routing" tone={routing() ? "bad" : "ok"} text={routing() ? `${routing()} to fix` : "square"} title="Lines left skewed, turns without an elbow, components that do not fit" run={() => setBottom("checks")} />
    <Show when={project()!.line_list.length}><B k="list" label="P&ID" tone={list() ? "warn" : "ok"} text={list() ? `${list()} differences` : "matches"} title="The model against the line list or plant package" run={() => setBottom("checks")} /></Show>
  </span>;
};

const Status = () => (
  <footer class="status">
    <span class="smode">{mode() === "route" ? "Routing" : mode() === "place" ? "Placing" : mode() === "measure" ? "Measuring" : "Ready"}</span>
    <span class="hintslot" ref={setHintSlot} />
    <span class="muted">{project() ? `${project()!.equipment.length} equipment · ${project()!.lines.length} lines · ${scene()?.parts.filter(p => p.line).length ?? 0} piping parts` : ""}</span>
    <span class="grow" />
    <Show when={project()}><Badges /></Show>
    <Show when={picked().length}><span class="muted">{picked().length} selected</span></Show>
  </footer>
);

const App = () => {
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
    else if (e.key === "Escape" && (window as any).__cancelDrag?.()) { e.preventDefault(); }
    else if (e.key === "Escape" && mode() !== "select") C.cancelTool();
    else if (e.key === "Escape" && !typing && (sel() || picked().length)) { setSel(null); setPicked([]); }
    else if (ctrl && k === "a" && !typing && !backstage()) { e.preventDefault(); C.selectAll(); }
    else if (ctrl && k === "d" && !typing && !backstage()) { e.preventDefault(); C.copyGroup(); }
    else if (typing || ctrl || backstage()) return;
    else if (e.key === "Delete") { e.preventDefault(); C.del(); }
    else if (e.key.startsWith("Arrow") && (sel() || picked().length)) { e.preventDefault(); C.nudge(e.key.slice(5).toLowerCase() as "up" | "down" | "left" | "right", e.shiftKey); }
    else if (e.key === "PageUp") { e.preventDefault(); C.elevation(grid); }
    else if (e.key === "PageDown") { e.preventDefault(); C.elevation(-grid); }
    else if (k === "r") C.route();
    else if (k === "m") C.measure();
    else if (k === "n") C.nextClash();
    else if (k === "h") { if (e.shiftKey) C.showAll(); else C.hide(); }
    else if (k === "i") C.isolate();
    else if (k === "j") C.jog();
    else if (k === "f") C.fit();
    else if ("1234".includes(k) && k.length === 1) C.std(["iso", "plan", "front", "side"][+k - 1]);
  };
  return (
    <div class="app" ref={() => document.addEventListener("keydown", keys)}>
      <TitleBar update={update()?.available ? update()!.latest : null} />
      <Ribbon />
      <Show when={out()}><CtxBar /></Show>
      <main class="work">
        <Show when={out()}>
          <Flow />
          <div class="content" style={{ "--tree-w": `${treeW()}px`, "--insp-w": `${inspW()}px` }}>
            <Tree />
            <div class="vgrip" onPointerDown={e => grip(e, "tree")} title="Drag to widen or narrow the tree" />
            <ToolStrip />
            <Show when={partsOpen()}><PartsPanel /></Show>
            <div class="center">
              <div class="viewwrap">
                <View3D />
                <Show when={popShown()}><div class="ipop"><div class="vgrip" onPointerDown={e => grip(e, "insp")} title="Drag to widen or narrow this panel" /><Inspector /></div></Show>
              </div>
            </div>
          </div>
        </Show>
      </main>
      <Show when={out()}><BottomPanel /></Show>
      <Status />
      <Show when={backstage()}><Backstage /></Show>
      <For each={dialogs()}>{d => d()}</For>
      <Menu /><Toast />
    </div>
  );
};

render(() => <App />, document.getElementById("app")!);
