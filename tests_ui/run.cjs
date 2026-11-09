// UI tests: the built window (ui/dist) in Chromium, with every call the window makes answered
// by the real engine (crates/core/examples/bridge.rs), so these tests route lines, drop
// components, resize, check and export for real. Run: npm run test:ui (it builds the window and
// the bridge first). With SHOTS=dir it also writes the README and site screenshots (1440×900).
const { chromium } = require("playwright");
const { spawn, execSync } = require("child_process");
const fs = require("fs"), http = require("http"), path = require("path"), crypto = require("crypto");
const root = path.join(__dirname, "../ui/dist");
const types = { ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".woff2": "font/woff2", ".woff": "font/woff", ".html": "text/html", ".png": "image/png" };
const srv = http.createServer((q, r) => {
  const f = path.join(root, q.url === "/" ? "index.html" : decodeURIComponent(q.url.split("?")[0]));
  fs.readFile(f, (e, b) => { if (e) { r.writeHead(404); return r.end(); } r.writeHead(200, { "content-type": types[path.extname(f)] ?? "application/octet-stream" }); r.end(b); });
});
let failed = 0;
const check = (name, ok, info = "") => { console.log(`${ok ? "ok  " : "FAIL"} ${name}${ok ? "" : "  " + info}`); if (!ok) failed++; };
const shots = process.env.SHOTS;
// Names of commercial products the window must never show, as SHA-256 of their lower-case words
// (FAMILY §13: the names themselves are not written in this repository).
const BANNED = new Set(["2059091bf70e773d18642b704b559a5f79923be9fef170c1c5d729e6a5db20a8","72f52ad8fa80e59e61829b725b12b74495c4de6b29637a51c5115d9254a050a3","69d41313de163800ecb1cc42536748a50e41f0fc469daeac7641f01efc26870c","981309a55b1a205ad3b78139f9b8004d9817e5e60c726cdcc3d44792b8e5de1c","50f08e64c5a6192c58961a72ca0b5fcb06f1a09d49e3d012619c2050b84c9b00","51f5772b82560b51e5e1c03cc6e0e32fabda60428071ecd7710202b0b7281c78","6265160eb4e6e01a5247526509838426a201c1df0284d1f9824cd3ab3bf8b0c7","203b8ade4aaee21728c5ecc2b53aadbe0cd25c7337c205da91f0686542ad3478","063de0773639dd9f2b3047465a825bea1c752c53177f51acd573cf1e17accdbc","78921cb5cc3ef08ee9dd5d3067ebad142e0cacf32ed977945b8094a9e241e027","de4e9e60784006b242783eff92c8efc449ffb1ded63b3ba993acbe7af2800f49","5f37acd72c2cc038391bde05c11697a168667aa4a27c886638faecfd25b1bdd6","ff8934b2aac299dee7b686a6c453f8113e3b3f81c42944015c2f9b3e4cd41dfc","9fe6746863d6b21cff9dcfaf974e0192858b0c582888685307228527cb695b32","866091acb5ff40f25f6d72b9ed3152c7dfb7fb605d5f39b4dec2a2e9f3e99202","559054694ada46f586fa9ad4cc7874eed1077162eae10e0d66d70671915d219f","320d597682ed208cdaa58f77d17df6037e29f3c48323d6d16b23e0ca18f97788","c94665182cc3adf28fa8c2f11e941f6908db52a41fe7102ceedeee255d32be02","1c0b78f5409512d7d91ac065f5dc1c59892453e5505b9469fe61a6bafbefbc58","e342f3f3eeb5cb47c675f100cd7387d929a9db753c795391e6c102c599337ec2","5971b0dc06256600737ca8ba133808b5d8122016a777948e998535036594a95b","7cd583f44e170eb7137cbfaaa3c8e868c7a119d8da93c19c77d3a30cc16bd47b","9f391892bce8757edad9c7ce4f911f650f1620182a93a422c908e9f39ad2dfe0","a12db2a95422f699ba538fa667f5dc7f9f94c81e6072272e3cd0f03fb2ad174c","8552cf7aa7d5f69b9d38aef8356b8ad6a861850a3ec5b9d4ace25f138cd6fef3","6de058d07d56cf5bbec3e8f030a0a29031c42d5c75cb731066208a2f0207c99d","3d6c9bd164fea2b89f9101f42ea2dcc8a53c57fe301437483337ea3965b6a477","0ea39640478cc196478eff2cea4681f3d0fc39d868c199e8b9197d03a8f1b36d","bba94d0b766589e9a868e659fe69d0250eddcae940501895ad95601417dfd7bb","44df2b768f9be5af5eaf1c6f21b2fb9b7258f2d540c9fec2b776d4631b1e93d4","6ca001b66e30dcdad3db0deeeff6116a4bda219f6be7fcd6e2dd0c089dd06d76"]);
const sha = s => crypto.createHash("sha256").update(s).digest("hex");
const named = text => { const w = text.toLowerCase().match(/[a-z0-9]+/g) ?? []; const hits = []; for (let i = 0; i < w.length; i++) { if (BANNED.has(sha(w[i]))) hits.push(w[i]); if (i + 1 < w.length && BANNED.has(sha(w[i] + " " + w[i + 1]))) hits.push(w[i] + " " + w[i + 1]); } return hits; };

// The engine bridge: one call per line.
execSync("cargo build -q -p datum-core --example bridge", { cwd: path.join(__dirname, ".."), stdio: "inherit" });
const bridge = spawn(path.join(__dirname, "../target/debug/examples/bridge"), [], { stdio: ["pipe", "pipe", "inherit"] });
let buf = "", waiting = [];
bridge.stdout.on("data", d => { buf += d; let i; while ((i = buf.indexOf("\n")) >= 0) { const line = buf.slice(0, i); buf = buf.slice(i + 1); waiting.shift()?.(JSON.parse(line)); } });
let chain = Promise.resolve();
const ask = (cmd, args) => (chain = chain.then(() => new Promise(ok => { waiting.push(ok); bridge.stdin.write(JSON.stringify({ cmd, args: args ?? {} }) + "\n"); })));

(async () => {
  await new Promise(ok => srv.listen(0, ok));
  const url = `http://localhost:${srv.address().port}/`;
  const b = await chromium.launch({ args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
  const p = await b.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });
  const errors = [];
  p.on("pageerror", e => errors.push(e.message + " @ " + (e.stack || "").split("\n").slice(1, 4).join(" | ")));
  p.on("console", m => { if (m.type() === "error") errors.push(m.text()); });
  await p.exposeFunction("__bridge", (cmd, args) => ask(cmd, args));
  await p.addInitScript(() => {
    window.__calls = [];
    const handlers = {};
    window.__TAURI__ = {
      core: { invoke: async (c, a) => { window.__calls.push([c, a]); const r = await window.__bridge(c, a); if ("err" in r) throw r.err; return r.ok; } },
      event: { listen: async (e, h) => { (handlers[e] ??= []).push(h); return () => {}; } },
    };
  });
  await p.goto(url);
  await p.waitForTimeout(1200);
  const calls = cmd => p.evaluate(c => window.__calls.filter(x => x[0] === c).map(x => x[1]), cmd);
  const state = () => ask("state").then(r => r.ok);
  const settle = (ms = 400) => p.waitForTimeout(ms);
  const open = async id => {
    await p.evaluate(() => document.querySelector(".tabs .file").click()); await settle(200);
    await p.click('.bs aside button:has-text("Home")'); await settle(200);
    const n = id === null ? "Empty plot" : (await ask("examples_list")).ok.find(e => e.id === id).name;
    await p.click(`.home .tile:has(b:text-is("${n}"))`); await settle(900);
    if (await p.$('.dlg header:has-text("Save changes?")')) { await p.click('.dlg button:has-text("Don\'t save")'); await settle(900); }
  };
  const screenOf = pt => p.evaluate(q => window.__screen(q), pt);
  const allText = async () => p.evaluate(() => document.body.innerText);
  const seen = [];
  const shot = async name => { await p.evaluate(() => document.getElementById("toast")?.remove()); await settle(250); await p.screenshot({ path: `${shots}/${name}.png` }); };

  // 1. File page: the examples, the steel sidebar, the update card.
  const tiles = await p.$$eval(".home .tile b", els => els.map(e => e.textContent));
  check("File page shows the empty plot and every example", ["Empty plot", "Pump suction and discharge", "Exchanger hook-up", "Small pipe rack run", "Find the clashes"].every(t => tiles.includes(t)), JSON.stringify(tiles));
  check("File sidebar is the steel accent", (await p.$eval(".bs aside", el => getComputedStyle(el).backgroundColor)) === "rgb(58, 90, 120)");
  check("the update card names Datum Layout", (await p.textContent(".card .brand")).includes("Datum Layout"));
  seen.push(await allText());
  for (const k of ["Export", "Options", "Roadmap", "Updates"]) { await p.click(`.bs aside button:has-text("${k}")`); await settle(150); seen.push(await allText()); }
  check("File › Roadmap lists the next release and From Schema", seen[seen.length - 2].includes("Pipe classes and fittings") && seen[seen.length - 2].includes("From Schema"));

  // 2. An empty plot: grid coordinates and plant north, an empty state that offers the first action.
  await open(null);
  check("the empty plot has a north arrow and coordinates", await p.$eval(".nlabels", el => el.innerText.includes("N") && /E \d/.test(el.innerText)));
  check("the empty state says what to do", (await p.textContent(".v3d .empty")).includes("Drag equipment"));
  check("the window has the eight ribbon tabs", JSON.stringify(await p.$$eval(".tabs > button:not(.file)", els => els.map(e => e.textContent))) === JSON.stringify(["Home", "Equipment", "Piping", "Fittings", "Check", "Lists", "View", "Help"]));
  const bt = await p.$$eval(".btabs > button:not(.btn)", els => els.map(e => e.childNodes[0].textContent));
  check("the bottom panel has Lines, Equipment, Valves, Clashes and Checks", bt.join() === "Lines,Equipment,Valves,Clashes,Checks", bt.join());
  for (const t of ["Home", "Equipment", "Piping", "Fittings", "Check", "Lists", "View", "Help"]) { await p.click(`.tabs > button:has-text("${t}")`); await settle(80); seen.push(await allText()); }

  // 3. Place equipment from the catalogue by drag and drop onto the plot.
  const dropAt = async (type, kind, pt) => {
    await p.evaluate(([t, k]) => { window.__drag = { type: t, kind: k }; }, [type, kind]);
    const s = await screenOf(pt);
    await p.evaluate(([x, y]) => { const el = document.elementFromPoint(x, y); for (const ev of ["dragover", "drop"]) el.dispatchEvent(new DragEvent(ev, { bubbles: true, cancelable: true, clientX: x, clientY: y, dataTransfer: new DataTransfer() })); }, [s.x, s.y]);
    await settle(700);
  };
  await dropAt("equip", "pump", [10, 10, 0]);
  await dropAt("equip", "vertical_vessel", [20, 16, 0]);
  let st = await state();
  check("two items dropped onto the plot", st.p.equipment.length === 2 && st.p.equipment[0].tag === "P-101" && st.p.equipment[1].tag === "V-101", JSON.stringify(st.p.equipment.map(e => e.tag)));
  check("the Inspector shows the dropped vessel and its nozzles", (await p.textContent(".insp")).includes("V-101") && (await p.$$(".insp .ntab tbody tr")).length >= 3);

  // 4. Route a line nozzle to nozzle: Route, click the pump discharge, a corner, the vessel inlet.
  await p.click('.tabs > button:has-text("Piping")'); await p.click('.rb:has-text("Route Line")'); await settle(500);
  const sc = (await ask("scene")).ok;
  const dPort = sc.ports.find(x => x.kind === "nozzle" && x.tag === "D"), vIn = sc.ports.find(x => x.kind === "nozzle" && x.tag === "N2" && x.equip === st.p.equipment[1].id);
  for (const pt of [dPort.p, null, vIn.p]) {
    const s = await screenOf(pt ?? [dPort.p[0], dPort.p[1], 5]);
    await p.mouse.move(s.x, s.y); await settle(120); await p.mouse.click(s.x, s.y); await settle(400);
  }
  st = await state();
  const line = st.p.lines[0];
  check("a line was routed from P-101 D to V-101 N2", st.p.lines.length === 1 && line?.from.kind === "nozzle" && line?.to.kind === "nozzle", JSON.stringify(st.p.lines.map(l => [l.from, l.to])));
  const parts = sc2 => sc2.parts.filter(q => q.line === line.id).map(q => q.kind);
  let s2 = (await ask("scene")).ok;
  check("its elbows, flanges and gaskets were placed by themselves", parts(s2).includes("elbow90") && parts(s2).filter(k => k === "flange").length === 2 && parts(s2).filter(k => k === "gasket").length === 2, JSON.stringify(parts(s2)));
  check("the routed line has no routing problems", st.issues.filter(i => i.line === line.id && i.kind !== "number" && i.kind !== "rule").length === 0, JSON.stringify(st.issues));

  // 5. Drag a gate valve onto the line's longest run: it snaps, splits the pipe, nothing is open.
  const pts = st.paths.find(x => x[0] === line.id)[1];
  let k = 0, best = -1; for (let i = 0; i + 1 < pts.length; i++) { const d = Math.hypot(...[0, 1, 2].map(j => pts[i + 1][j] - pts[i][j])); if (d > best) { best = d; k = i; } }
  const mid = [0, 1, 2].map(j => (pts[k][j] + pts[k + 1][j]) / 2);
  check("the routed line's properties show over the view", !!(await p.$(".ipop .insp")));
  await p.keyboard.press("Escape"); await settle(200);
  check("Esc closes the properties card", !(await p.$(".ipop")));
  await p.keyboard.press("f"); await settle(300);
  await dropAt("item", "gate", mid);
  st = await state();
  const gate = st.p.lines[0].items.find(i => i.kind === "gate");
  check("the gate valve snapped onto the line", !!gate, JSON.stringify(await calls("apply")));
  s2 = (await ask("scene")).ok;
  check("the valve came with flanges, gaskets and bolts", s2.parts.filter(q => q.item === gate?.id).map(q => q.kind).sort().join() === "bolts,bolts,flange,flange,gasket,gasket,gate");
  check("no open ends after the drop", !st.issues.some(i => i.kind === "open"), JSON.stringify(st.issues));

  // 5b. While dragging, the engine says where it would snap.
  await p.evaluate(() => { window.__drag = { type: "item", kind: "globe" }; window.__calls.length = 0; });
  const q1 = [0, 1, 2].map(j => pts[k][j] + (pts[k + 1][j] - pts[k][j]) * 0.2);
  const s1 = await screenOf(q1);
  await p.evaluate(([x, y]) => document.elementFromPoint(x, y).dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, clientX: x, clientY: y, dataTransfer: new DataTransfer() })), [s1.x, s1.y]);
  await settle(400);
  check("dragging over a pipe asks the engine where it snaps", (await calls("snap")).length > 0);
  await p.evaluate(() => { window.__drag = undefined; document.querySelector(".v3d").dispatchEvent(new DragEvent("dragleave", { bubbles: true })); });

  // 5c. A tee dropped on the pipe starts a branch: route it to a free point.
  await dropAt("item", "tee", q1);
  check("dropping a tee starts routing its branch", (await p.textContent(".toolhint")).includes("Click to add a corner"));
  st = await state();
  const teeItem = st.p.lines[0].items.find(i => i.kind === "tee");
  const bport = (await ask("scene")).ok.ports.find(x => x.kind === "branch" && x.item === teeItem?.id);
  const far = await screenOf([bport.p[0] + bport.d[0] * 3, bport.p[1] + bport.d[1] * 3, bport.p[2] + bport.d[2] * 3]);
  await p.mouse.move(far.x, far.y); await settle(150); await p.mouse.dblclick(far.x, far.y); await settle(700);
  st = await state();
  const br = st.p.lines.find(l => l.from.kind === "branch");
  check("the branch line starts at the tee", !!br && br.from.item === teeItem.id, JSON.stringify(st.p.lines.map(l => l.from)));
  check("the branch meets its tee with nothing open there", !st.issues.some(i => i.kind === "open" && i.line !== br?.id) && !st.issues.some(i => i.kind === "mismatch"), JSON.stringify(st.issues.map(i => i.text)));
  await p.keyboard.press("Escape");

  // 6. Resize the line: every component re-sizes.
  await p.click(".btabs > button:has-text(\"Lines\")"); await settle(300);
  await p.click(".bbody tr.f"); await settle(300);
  await p.click('.tabs > button:has-text("Piping")'); await p.click('.rb:has-text("One size larger")'); await settle(700);
  st = await state();
  s2 = (await ask("scene")).ok;
  check("resizing made the 4\" line and its valve 5\"", st.p.lines[0].nps === "5" && s2.parts.filter(q => q.item === gate.id && q.kind === "gate")[0]?.nps === "5", st.p.lines[0].nps);
  await p.click(".tabs .ctxgrp button"); await settle(200); seen.push(await allText());

  // 7. Undo and redo.
  await p.keyboard.press("Control+z"); await settle(500);
  check("undo brings back the 4\" line", (await state()).p.lines[0].nps === "4");
  await p.keyboard.press("Control+y"); await settle(500);

  // 7a. Measure: two clicks give the distance with its parts.
  await p.keyboard.press("m"); await settle(200);
  const ma = await screenOf([2, 2, 0]), mb = await screenOf([8, 2, 0]);
  await p.mouse.click(ma.x, ma.y); await settle(150); await p.mouse.click(mb.x, mb.y); await settle(300);
  const mt = await p.textContent(".toolhint");
  check("measure shows the distance between two clicks", /6(\.0+)? m/.test(mt) && mt.includes("ΔE"), mt);
  await p.keyboard.press("Escape"); await settle(150);
  check("the catalogue lists the example pipe classes", (await p.$$(".catclass")).length === 8);

  // 7c. Groups: Ctrl+A, copy beside, mirror; every copy stays connected.
  await open("pumps");
  await p.keyboard.press("Control+a"); await settle(300);
  check("Ctrl+A selects every item", (await p.textContent(".insp")).includes("8 items selected"), await p.textContent(".insp header"));
  await p.keyboard.press("Control+d"); await settle(900);
  st = await state();
  check("Ctrl+D copies the plant beside itself", st.p.equipment.length === 6 && st.p.lines.length === 10);
  check("the copy is connected and clean", !st.issues.length, JSON.stringify(st.issues.map(i => i.text)));
  await p.click('.tabs > button:has-text("Home")'); await p.click('.rb:has-text("Mirror east–west")'); await settle(900);
  st = await state();
  check("mirroring the copy keeps it clean", !st.issues.length, JSON.stringify(st.issues.map(i => i.text)));
  await p.keyboard.press("Control+z"); await p.keyboard.press("Control+z"); await settle(600);
  check("undo takes the copy away again", (await state()).p.equipment.length === 3);

  // 7f. The workspace (0.1.9, after Datum Mesh): steps bar, plant tree, eyes, filter, catalogue tab.
  await open("pumps");
  check("the steps bar shows six steps, equipment and lines done", (await p.$$(".flow button .fn")).length === 6 && (await p.$$(".flow button.done")).length >= 3, String((await p.$$(".flow button.done")).length));
  await p.click('.flow button:has-text("Fittings")'); await settle(200);
  check("a step opens its ribbon tab", (await p.textContent(".tabs button.on")).includes("Fittings"));
  check("the tree lists the equipment and every line", (await p.$$(".tree .ti[data-id]")).length === 8, String((await p.$$(".tree .ti[data-id]")).length));
  check("the tree lists the valves on the lines", (await p.textContent(".tree")).includes("Gate valves"));
  const lid = (await state()).p.lines.find(l => l.number.includes("1002")).id;
  const row = `.tree .ti[data-id="${lid}"]`;
  await p.click(`${row} > span:not(.teye)`); await settle(300);
  check("clicking a line in the tree selects it and opens its properties", (await p.textContent(".ipop .insp header")).includes("P-1002"));
  await p.click(`${row} .teye`); await settle(300);
  check("its eye hides it in the view", (await p.textContent(".hidnote")).includes("1 hidden") && !!(await p.$(`${row}.dim`)));
  await p.click(`${row} .teye`); await settle(300);
  check("and shows it again", !(await p.$(".hidnote")));
  await p.fill(".tfilter input", "P-101B"); await settle(300);
  const vis = await p.$$eval(".tree .ti[data-id]", els => els.filter(e => e.offsetParent !== null).map(e => e.textContent));
  check("the filter keeps only matching rows", vis.length === 1 && vis[0].includes("P-101B"), JSON.stringify(vis));
  await p.fill(".tfilter input", ""); await settle(200);
  await p.click('.ttabs button:has-text("Catalogue")'); await settle(200);
  check("the Catalogue tab holds the parts to drag", (await p.$$(".tree .cattile")).length >= 10 && await p.isVisible(".tree .cattile"));
  await p.click('.ttabs button:has-text("Plant")'); await settle(200);
  await p.selectOption("select.ctxunits", "US"); await settle(500);
  check("the context bar switches units", (await state()).p.units === "US");
  await p.selectOption("select.ctxunits", "SI"); await settle(400);
  if (shots) { const eq = (await state()).p.equipment.find(e => e.tag === "P-101A").id; await p.click(`.tree .ti[data-id="${eq}"] > span:not(.teye)`); await settle(300); await shot("workspace"); }

  // 7h. The Library (0.2.0): classes and parts tables with their sources.
  await open("pumps");
  await p.click('.flow button[data-tool="library"]'); await settle(600);
  check("the Library lists every pipe class and the eight parts tables", (await p.$$('.libx .mlist > button')).length === 16, String((await p.$$('.libx .mlist > button')).length));
  await p.click('.libx .mlist > button:has-text("CS300")'); await settle(300);
  const lt = await p.textContent(".libpage");
  check("a class page shows its wall by size and allowed components", lt.includes("Class 300") && /½"\s*80/.test(lt) && lt.includes("Gate valve"), lt.slice(0, 300));
  if (shots) await shot("library-class");
  await p.click('.libx .mlist > button:has-text("Use for new lines")').catch(() => {});
  await p.click('.libpage button:has-text("Use for new lines")'); await settle(500);
  check("Use for new lines sets the class", (await state()).p.settings.class === "CS300");
  await p.click('.libx .mlist > button:has-text("Flanges")'); await settle(200);
  await p.fill(".libsearch", "2500"); await settle(300);
  const fr = await p.$$eval(".libtbl tbody tr", rs => rs.map(r => r.children[1]?.textContent));
  check("search narrows the flange table to class 2500", fr.length > 5 && fr.every(x => x === "2500"), JSON.stringify(fr.slice(0, 5)));
  if (shots) await shot("library-flanges");
  await p.keyboard.press("Escape"); await settle(200);

  // 7i. From the P&ID (0.2.1): a plant package placed, listed, routed and checked.
  await open(null);
  const pkg = require("fs").readFileSync(require("path").join(__dirname, "../docs/formats/pump-transfer.package.json"), "utf8");
  await p.click('.tabs > button:has-text("Equipment")');
  const [pfc] = await Promise.all([p.waitForEvent("filechooser"), p.click('.rb:has-text("Plant Package")')]);
  await pfc.setFiles({ name: "pump-transfer.package.json", mimeType: "application/json", buffer: Buffer.from(pkg) }); await settle(900);
  st = await state();
  check("the plant package places its equipment and lists its lines", st.p.equipment.length === 3 && st.p.line_list.length === 3, `${st.p.equipment.length} ${st.p.line_list.length}`);
  check("the checks say the listed lines are not modelled yet", st.issues.filter(i => i.kind === "list").length === 3);
  await p.click('.rb:has-text("Route Listed Lines")'); await settle(1200);
  st = await state();
  check("Route Listed Lines routes the nozzle-to-nozzle lines with their components", st.p.lines.length === 2 && st.p.lines.every(l => l.items.length === 2), JSON.stringify(st.p.lines.map(l => [l.number, l.items.length])));
  check("only the branch is left, and nothing else differs from the P&ID", st.issues.filter(i => i.kind === "list").length === 1 && st.issues.some(i => i.kind === "list" && i.text.includes("2003")), JSON.stringify(st.issues.filter(i => i.kind === "list").map(i => i.text)));
  if (shots) { await p.keyboard.press("Escape"); await p.keyboard.press("f"); await settle(500); await shot("from-pid"); }

  // 7j. Pipe supports (0.2.2): on the rack, placed, changed and written for Datum Pipe.
  await open("rack");
  st = await state();
  const nsup = st.p.lines.reduce((n, l) => n + l.items.filter(i => i.kind === "support").length, 0);
  check("the rack example has its rests, guides and anchors", nsup === 9 && (await p.textContent(".tree")).includes("Pipe supports"), String(nsup));
  const la = st.p.lines.find(l => l.number.includes("3001"));
  await p.click(".btabs > button:has-text(\"Lines\")"); await settle(200);
  await p.click(`.bbody tr.f:has-text(${JSON.stringify(la.number)})`); await settle(300);
  await p.click('.tabs > button:has-text("Piping")'); await p.click('.rb:has-text("Rest")'); await settle(700);
  st = await state();
  const added = st.p.lines.find(l => l.id === la.id).items.filter(i => i.kind === "support");
  check("Piping › Rest puts a support on the selected line", added.length === 5, String(added.length));
  await p.selectOption(".ipop select.supkind", "anchor"); await settle(600);
  st = await state();
  check("the properties change what it holds", st.p.lines.find(l => l.id === la.id).items.filter(i => i.kind === "support" && i.support === "anchor").length === 2);
  const pcfA = (await ask("export_pcf", { line: la.id })).ok;
  check("supports go to Datum Pipe in the PCF", (pcfA.match(/^SUPPORT$/gm) ?? []).length === 5 && pcfA.includes("SKEY ANC") && pcfA.includes("SKEY GUI"), pcfA.slice(-400));
  check("supports raise no clash or class finding", !st.issues.some(i => i.kind === "class" || i.kind === "clash"), JSON.stringify(st.issues.map(i => i.text)));
  if (shots) { await p.keyboard.press("Escape"); await settle(200); await p.evaluate(() => (window).__screen); await p.keyboard.press("f"); await settle(400); await shot("supports"); }

  // 7k. Moving made easy (0.2.3): drag equipment across the plot, Esc, arrow keys, the view cube, badges.
  await open("pumps");
  await p.keyboard.press("2"); await settle(400); // plan view: the drag reads plainly east and north
  st = await state();
  const pA = st.p.equipment.find(e => e.tag === "P-101A");
  await p.click(`.tree .ti[data-id="${pA.id}"] > span:not(.teye)`); await settle(300);
  const bodyPt = [pA.pos[0] + 0.55, pA.pos[1] + 0.12, pA.pos[2] + 0.5]; // the motor, clear of the arrows
  const s0 = await screenOf(bodyPt), sB = await screenOf([bodyPt[0] + 2, bodyPt[1], bodyPt[2]]);
  await p.mouse.move(s0.x, s0.y); await p.mouse.down(); await p.mouse.move((s0.x + sB.x) / 2, (s0.y + sB.y) / 2, { steps: 4 }); await p.mouse.move(sB.x, sB.y, { steps: 4 }); await settle(150);
  check("dragging shows the move as it goes",(await p.textContent(".v3d")).includes("ΔE"));
  await p.mouse.up(); await settle(800);
  st = await state();
  const pA2 = st.p.equipment.find(e => e.tag === "P-101A");
  check("dragging the selected pump moves it across the plot", Math.abs(pA2.pos[0] - pA.pos[0] - 2) < 0.35 && Math.abs(pA2.pos[1] - pA.pos[1]) < 0.35, JSON.stringify([pA.pos, pA2.pos]));
  check("its lines follow it, connected and routed square again", !st.issues.some(i => i.kind === "open" || i.kind === "mismatch" || i.kind === "routing"), JSON.stringify(st.issues.map(i => i.text)));
  // Esc while dragging puts it back.
  const t0 = await screenOf([pA2.pos[0] + 0.55, pA2.pos[1] + 0.12, pA2.pos[2] + 0.5]), t1 = await screenOf([pA2.pos[0] + 0.55, pA2.pos[1] - 2.88, pA2.pos[2] + 0.5]);
  await p.mouse.move(t0.x, t0.y); await p.mouse.down(); await p.mouse.move(t1.x, t1.y, { steps: 6 }); await settle(100);
  await p.keyboard.press("Escape"); await p.mouse.up(); await settle(500);
  check("Esc while dragging puts it back", JSON.stringify((await state()).p.equipment.find(e => e.tag === "P-101A").pos) === JSON.stringify(pA2.pos));
  // Arrow keys: one grid step, along a plant axis.
  await p.keyboard.press("ArrowRight"); await settle(600);
  const pA3 = (await state()).p.equipment.find(e => e.tag === "P-101A");
  const dd = [pA3.pos[0] - pA2.pos[0], pA3.pos[1] - pA2.pos[1]];
  check("an arrow key nudges it one grid step along a plant axis", Math.abs(Math.hypot(dd[0], dd[1]) - 1) < 1e-6 && (dd[0] === 0 || dd[1] === 0), JSON.stringify(dd));
  // The view cube: a corner gives the isometric view.
  const before = await screenOf([0, 0, 0]);
  // The top face's south-east corner (the view looks down): an isometric view.
  const cb = await p.$eval(".v3d .cube", el => { const r = el.getBoundingClientRect(); return { x: r.left + r.width * 60 / 84, y: r.top + r.height * 60 / 84 }; });
  await p.mouse.move(cb.x, cb.y); await settle(100); await p.mouse.click(cb.x, cb.y); await settle(400);
  const after = await screenOf([0, 0, 0]);
  check("the view cube turns the view", Math.hypot(after.x - before.x, after.y - before.y) > 5, JSON.stringify([before, after]));
  check("the cube shows the plant's faces", (await p.textContent(".v3d .cube")).includes("Top"));
  if (shots) { await p.mouse.move(cb.x + 30, cb.y - 30); await settle(150); await shot("viewcube"); }
  // Badges.
  check("the check badge says it has not run", (await p.textContent('.badge[data-badge="check"]')).includes("not run"));
  await p.keyboard.press("F5"); await settle(1200);
  check("after F5 the check badge reports the check", /clean|findings/.test(await p.textContent('.badge[data-badge="check"]')), await p.textContent(".status"));

  // 7l. Eccentric reducers (0.2.4): flat on top, from the properties; concentric again.
  await open("pumps");
  st = await state();
  const hdr = st.p.lines.find(l => l.number.includes("1005"));
  await p.click(".btabs > button:has-text(\"Lines\")"); await settle(200);
  await p.click(`.bbody tr.f:has-text(${JSON.stringify(hdr.number)})`); await settle(300);
  await p.click('.tabs > button:has-text("Fittings")'); await p.click('.rb:has-text("Reducer")'); await settle(700);
  await p.selectOption(".ipop select.flatside", "top"); await settle(700);
  st = await state();
  const er = st.p.lines.find(l => l.id === hdr.id).items.find(i => i.kind === "reducer");
  check("a reducer's flat side can be set on top", er?.flat === "top", JSON.stringify(er));
  check("the eccentric reducer routes cleanly", !st.issues.some(i => i.kind === "routing" || i.kind === "mismatch"), JSON.stringify(st.issues.map(i => i.text)));
  check("the properties name it eccentric", (await p.textContent(".ipop .insp")).includes("flat on top"));
  check("it goes to Datum Pipe as an eccentric reducer", (await ask("export_pcf", { line: hdr.id })).ok.includes("REDUCER-ECCENTRIC"));
  if (shots) { await p.keyboard.press("Escape"); await settle(200); await shot("eccentric"); }
  await p.keyboard.press("Control+z"); await settle(600);
  check("undo makes it concentric again", (await state()).p.lines.find(l => l.id === hdr.id).items.find(i => i.kind === "reducer")?.flat === "none");

  // 7m. Crosses (0.2.5): two branches, the second routed from the properties.
  await open("pumps");
  st = await state();
  const hd = st.p.lines.find(l => l.number.includes("1005"));
  await p.click(".btabs > button:has-text(\"Lines\")"); await settle(200);
  await p.click(`.bbody tr.f:has-text(${JSON.stringify(hd.number)})`); await settle(300);
  await p.click('.tabs > button:has-text("Fittings")'); await p.click('.rb:has-text("Cross")'); await settle(800);
  check("dropping a cross starts routing its first branch", (await p.textContent(".toolhint")).includes("Click to add a corner"));
  await p.keyboard.press("Escape"); await settle(300);
  st = await state();
  const cx = st.p.lines.find(l => l.id === hd.id).items.find(i => i.kind === "cross");
  const bports = (await ask("scene")).ok.ports.filter(x => x.kind === "branch" && x.item === cx?.id);
  check("a cross has two branch ends, opposite", bports.length === 2 && Math.abs(bports[0].d[0] + bports[1].d[0]) + Math.abs(bports[0].d[1] + bports[1].d[1]) + Math.abs(bports[0].d[2] + bports[1].d[2]) < 1e-6, JSON.stringify(bports.map(b => b.d)));
  await p.click(".btabs > button:has-text(\"Lines\")"); await settle(200);
  await p.click(`.bbody tr.f:has-text(${JSON.stringify(hd.number)})`); await settle(300);
  await p.click(`.ipop .ti:has-text("Cross")`).catch(() => {}); await settle(300);
  if (!(await p.$('.ipop button:has-text("Route the second branch")'))) { await p.click(`.tree .ti:has-text("Cross")`).catch(() => {}); await settle(300); }
  await p.click('.ipop button:has-text("Route the second branch")'); await settle(400);
  const b2 = bports.find(x => x.branch === 1);
  const fp = await screenOf([b2.p[0] + b2.d[0] * 3, b2.p[1] + b2.d[1] * 3, b2.p[2] + b2.d[2] * 3]);
  await p.mouse.move(fp.x, fp.y); await settle(150); await p.mouse.dblclick(fp.x, fp.y); await settle(800);
  st = await state();
  const b2l = st.p.lines.find(l => l.from.kind === "branch" && l.from.item === cx.id && l.from.port === 1);
  check("the second branch is routed from the cross", !!b2l, JSON.stringify(st.p.lines.map(l => l.from)));
  check("the cross and its branch route cleanly", !st.issues.some(i => i.kind === "routing" || i.kind === "mismatch" || i.kind === "class"), JSON.stringify(st.issues.map(i => i.text)));
  check("the cross goes to Datum Pipe with both branch points", (await ask("export_pcf", { line: hd.id })).ok.includes("BRANCH2-POINT"));
  if (shots) { await p.keyboard.press("Escape"); await settle(200); await shot("cross"); }

  // 7n. Branch outlets (0.2.7): a 2" branch off the 6" header's tee becomes a weldolet, as the class says.
  await open("pumps");
  st = await state();
  const hdr2 = st.p.lines.find(l => l.number.includes("1005"));
  await p.click(".btabs > button:has-text(\"Lines\")"); await settle(200);
  await p.click(`.bbody tr.f:has-text(${JSON.stringify(hdr2.number)})`); await settle(300);
  await p.click('.tabs > button:has-text("Piping")'); await p.selectOption('select.ctxsize', "2"); await settle(300);
  await p.click('.tabs > button:has-text("Fittings")'); await p.click('.rb:has-text("Tee")'); await settle(800);
  const tp2 = (await ask("scene")).ok.ports.filter(x => x.kind === "branch" && !x.used).slice(-1)[0];
  const fp2 = await screenOf([tp2.p[0] + tp2.d[0] * 2, tp2.p[1] + tp2.d[1] * 2, tp2.p[2] + tp2.d[2] * 2]);
  await p.mouse.move(fp2.x, fp2.y); await settle(150); await p.mouse.dblclick(fp2.x, fp2.y); await settle(800);
  st = await state();
  const olet = st.p.lines.find(l => l.id === hdr2.id).items.find(i => i.kind === "weldolet");
  check("routing a 2\" branch off the 6\" header's tee makes a weldolet", !!olet, JSON.stringify(st.p.lines.find(l => l.id === hdr2.id).items.map(i => i.kind)));
  check("the outlet and its branch check cleanly", !st.issues.some(i => ["routing", "mismatch", "class"].includes(i.kind)), JSON.stringify(st.issues.map(i => i.text)));
  check("it goes to Datum Pipe as an OLET", (await ask("export_pcf", { line: hdr2.id })).ok.includes("OLET"));
  if (shots) { await p.keyboard.press("Escape"); await settle(200); await shot("weldolet"); }

  // 7o. Butterfly valves (0.2.9): from the ribbon onto the selected 6" line; short, stem up, written as VYFL.
  await open("pumps");
  st = await state();
  const bl = st.p.lines.find(l => l.number.includes("1005"));
  await p.click(".btabs > button:has-text(\"Lines\")"); await settle(200);
  await p.click(`.bbody tr.f:has-text(${JSON.stringify(bl.number)})`); await settle(300);
  await p.click('.tabs > button:has-text("Fittings")'); await p.click('.rb:has-text("Butterfly")'); await settle(800);
  st = await state();
  const bfv = st.p.lines.find(l => l.id === bl.id).items.find(i => i.kind === "butterfly");
  check("a butterfly valve drops onto the selected line from the ribbon", !!bfv, JSON.stringify(st.p.lines.find(l => l.id === bl.id).items.map(i => i.kind)));
  const bpcf = (await ask("export_pcf", { line: bl.id })).ok;
  const vb = /VALVE\n    END-POINT (\S+) (\S+) (\S+) \S+\n    END-POINT (\S+) (\S+) (\S+) \S+\n    SKEY VYFL/.exec(bpcf);
  const blen = vb ? Math.hypot(vb[4] - vb[1], vb[5] - vb[2], vb[6] - vb[3]) : 0;
  check("it is short pattern (140 mm for 6\" class 150) and checks cleanly", Math.abs(blen - 140) < 1 && !st.issues.some(i => ["class", "routing"].includes(i.kind)), `${blen} ${JSON.stringify(st.issues.map(i => i.text))}`);
  check("it goes to Datum Pipe as a butterfly VALVE", bpcf.includes("SKEY VYFL"));
  if (shots) { await p.keyboard.press("Escape"); await settle(200); await shot("butterfly"); }

  // 7g. The valve list, butt welds in the take-off, design rules (0.1.10).
  await open("pumps");
  const nv = (await ask("lists")).ok.valves.length;
  await p.click('.tabs > button:has-text("Lists")'); await p.click('.rb:has-text("Valve List")'); await settle(500);
  check("the valve list shows every valve", nv === 6 && (await p.$$(".bbody tr.f")).length === nv, `${nv} ${(await p.$$(".bbody tr.f")).length}`);
  check("the take-off counts butt welds", (await ask("lists")).ok.mto.some(r => r.group === "Welds" && r.qty > 0));
  await p.click('.bbody tr.f:has-text("Gate")'); await settle(400);
  await p.selectOption('.ipop .iprow:has(label:has-text("Stem")) select', { label: "Down" }); await settle(600);
  st = await state();
  check("a valve stem turned down breaks a design rule", st.issues.some(i => i.kind === "rule" && i.text.includes("stem points down")), JSON.stringify(st.issues.map(i => i.text)));
  await p.click(".btabs > button:has-text(\"Checks\")"); await settle(300);
  check("the Checks tab lists it as a design rule", (await p.textContent(".bbody")).includes("Design rule"));
  await p.keyboard.press("Control+z"); await settle(500);
  check("undo puts the stem back and clears the rule", !(await state()).issues.some(i => i.kind === "rule"));

  // 7d. Corners, jogs and what the view shows (0.1.7).
  await open("pumps");
  st = await state();
  // A level run of 3 m or more with nothing on it, whose middle is in view.
  let cand = null;
  for (const l of st.p.lines) {
    const ps = st.paths.find(x => x[0] === l.id)[1];
    for (let i = 0; i + 1 < ps.length && !cand; i++) {
      const L = Math.hypot(...[0, 1, 2].map(j => ps[i + 1][j] - ps[i][j]));
      if (L >= 3 && Math.abs(ps[i + 1][2] - ps[i][2]) < 1e-6 && !l.items.some(it => it.seg === i)) cand = { l, i, ps, mid: [0, 1, 2].map(j => (ps[i][j] + ps[i + 1][j]) / 2) };
    }
  }
  check("example 1 has a clear level run to jog", !!cand);
  await p.click(".btabs > button:has-text(\"Lines\")"); await settle(300);
  await p.click(`.bbody tr.f:has-text(${JSON.stringify(cand.l.number)})`); await settle(300);
  const ms = await screenOf(cand.mid);
  await p.mouse.click(ms.x, ms.y); await settle(300);
  await p.keyboard.press("j"); await settle(800);
  st = await state();
  const jl = st.p.lines.find(l => l.id === cand.l.id);
  check("J adds a jog with four corners to the selected run", st.paths.find(x => x[0] === jl.id)[1].length === cand.ps.length + 4, JSON.stringify(await calls("apply")).slice(-300));
  check("the jog routes cleanly", !st.issues.length, JSON.stringify(st.issues.map(i => i.text)));
  await p.click('.tabs > button:has-text("Piping")'); await p.click('.rb:has-text("Remove Jog")'); await settle(800);
  st = await state();
  check("Remove Jog straightens it again", st.paths.find(x => x[0] === jl.id)[1].length === cand.ps.length && !st.issues.length, JSON.stringify(st.issues.map(i => i.text)));
  // A selected line shows its corners; clicking one selects it.
  await p.click(`.bbody tr.f:has-text(${JSON.stringify(cand.l.number)})`); await settle(300);
  const cs = await screenOf(cand.ps[1]);
  await p.mouse.click(cs.x, cs.y); await settle(300);
  check("clicking a corner dot selects the corner", ((await p.$(".toolhint.sub")) && (await p.textContent(".toolhint.sub")).includes("Corner 2")), await p.textContent(".v3d"));
  if (shots) await shot("corner");
  // Colour by line: a key with every line.
  await p.click('.tabs > button:has-text("View")'); await p.selectOption("select.colourby", "line"); await settle(500);
  check("colour by line shows a key with every line", (await p.$$(".legend span")).length === st.p.lines.length, String((await p.$$(".legend span")).length));
  if (shots) await shot("colour-by-line");
  await p.selectOption("select.colourby", "kind"); await settle(300);
  check("colour by part kind has no key", !(await p.$(".legend")));
  // Hide, isolate, show all.
  await p.click(`.bbody tr.f:has-text(${JSON.stringify(cand.l.number)})`); await settle(300);
  await p.keyboard.press("h"); await settle(300);
  check("H hides the selected line", (await p.textContent(".hidnote")).includes("1 hidden"));
  await p.keyboard.press("Shift+H"); await settle(300);
  check("Shift+H shows everything again", !(await p.$(".hidnote")));
  await p.click(`.bbody tr.f:has-text(${JSON.stringify(cand.l.number)})`); await settle(300);
  await p.keyboard.press("i"); await settle(400);
  check("I shows only the selected line", (await p.textContent(".hidnote")).includes("Showing 1 item only"));
  await p.click('.hidnote a'); await settle(300);
  // The section box round the whole plant, then off.
  await p.click('.tabs > button:has-text("View")'); await p.click('.rb:has-text("Section Box")'); await settle(400);
  check("Section Box opens its panel", !!(await p.$(".secbox")));
  if (shots) await shot("section-box");
  await p.click('.secbox button:has-text("Turn off")'); await settle(300);
  check("the section box turns off", !(await p.$(".secbox")));

  // 7e. Saved views, the line list and PCF lines (0.1.8).
  await open("pumps");
  await p.click('.tabs > button:has-text("View")'); await p.selectOption("select.colourby", "line"); await settle(300);
  await p.click('.rb:has-text("Save View")'); await settle(500);
  st = await state();
  check("Save View keeps the view with the plant", st.p.views.length === 1 && st.p.views[0].colour_by === "line", JSON.stringify(st.p.views));
  await p.selectOption("select.colourby", "kind"); await settle(300);
  await p.click('.rb:has-text("Views")'); await settle(200); await p.click('.menu button:has-text("View 1")'); await settle(500);
  check("choosing the saved view brings back its colours", !!(await p.$(".legend")));
  const filePick = async (button, name, text) => {
    const [fc] = await Promise.all([p.waitForEvent("filechooser"), p.click(button)]);
    await fc.setFiles({ name, mimeType: "text/plain", buffer: Buffer.from(text) }); await settle(900);
  };
  let csv = (await ask("export_csv", { kind: "lines" })).ok;
  csv = csv.replace(/^(8""-P-1002-CS1|"8""-P-1002-CS1"),([^,]*),([^,]*),8,/m, (m, a, b, c) => `${a},${b},${c},10,`);
  await p.click('.tabs > button:has-text("Lists")');
  await filePick('.rb[title^="Read a line list"]', "lines.csv", csv);
  st = await state();
  check("Line List reads every line of the exported list", st.p.line_list.length === 5, String(st.p.line_list.length));
  check("the checks show the one difference with the list", st.issues.filter(i => i.kind === "list").length === 1 && st.issues.some(i => i.kind === "list" && i.text.includes("size 10")), JSON.stringify(st.issues.map(i => i.text)));
  check("the difference is listed under Checks", (await p.textContent(".bbody")).includes("Line list"));
  await p.click('.bbody tr.f:has-text("Line list")'); await settle(400);
  await p.selectOption("select.fromlist", { index: 2 }); await settle(700);
  st = await state();
  check("taking the listed line's values clears it", !st.issues.some(i => i.kind === "list"), JSON.stringify(st.issues.map(i => i.text)));
  // PCF: the plant's lines written, taken away and read back in.
  await open("pumps");
  const pcfText = (await ask("export_pcf", { line: null })).ok;
  st = await state();
  await ask("apply", { op: { Delete: { ids: st.p.lines.map(l => l.id) } } });
  await p.click('.tabs > button:has-text("Lists")');
  await filePick('.rb:has-text("PCF Lines")', "plant.pcf", pcfText);
  st = await state();
  check("PCF Lines reads the plant's lines back", st.p.lines.length === 5, String(st.p.lines.length));
  check("the lines read back connect to their nozzles and tees, clean", !st.issues.length && st.p.lines.filter(l => l.from.kind === "nozzle" || l.to.kind === "nozzle").length === 4 && st.p.lines.filter(l => l.from.kind === "branch").length === 3, JSON.stringify(st.issues.map(i => i.text)));
  if (shots) await shot("pcf-import");

  // 7b. Turn the tank in example 1: its suction line is reported skewed; Route Again squares it.
  await open("pumps");
  await p.click(".btabs > button:has-text(\"Equipment\")"); await settle(300);
  await p.click(".bbody tr.f:has-text(\"T-101\")"); await settle(300);
  await p.click('.tabs > button:has-text("Equipment")'); await p.click('.rb:has-text("Rotate −90°")'); await settle(600);
  st = await state();
  check("turning the tank reports its line to route again", st.issues.some(i => i.text.includes("route it again")), JSON.stringify(st.issues.map(i => i.text)));
  await p.click(".btabs > button:has-text(\"Lines\")"); await settle(300);
  await p.click(".bbody tr.f:has-text(\"8\\\"-P-1001\")"); await settle(300);
  await p.click('.insp button:has-text("Route again")'); await settle(600);
  st = await state();
  const sid = st.p.lines.find(l => l.number.includes("1001")).id;
  check("Route again squares the suction line", !st.issues.some(i => i.line === sid), JSON.stringify(st.issues.map(i => i.text)));

  // 8. Example 1: opens clean; the check finds nothing; the lists fill.
  await open("pumps");
  await p.keyboard.press("F5"); await settle(1200);
  check("example 1: the check finds no clashes and no errors", (await p.textContent(".bbody")).includes("No clashes"), await p.textContent(".bbody"));
  await p.click(".btabs > button:has-text(\"Lines\")"); await settle(400);
  check("example 1: the line list has five lines", (await p.$$(".bbody tr.f")).length === 5);
  if (shots) { await p.click(".bbody tr.f >> nth=0"); await settle(400); await p.click(".tabs > button:has-text(\"Lists\")"); await shot("line-list"); }
  await p.click(".btabs > button:has-text(\"Equipment\")"); await settle(400);
  check("example 1: the equipment list shows every nozzle", (await p.textContent(".bbody")).includes("P-101A") && (await p.$$(".bbody tr.f")).length >= 7);
  // 8b. Weights (0.2.8): the line list and the take-off show them; the PCF carries them to Datum Pipe.
  await p.click(".btabs > button:has-text(\"Lines\")"); await settle(300);
  check("the line list shows each line's empty weight", /Weight \(kg\)/.test(await p.textContent(".bbody thead")) && (await p.$$eval(".bbody tr.f", rs => rs.every(r => Number(r.lastElementChild.textContent.replace("+", "")) > 0))));
  await p.click("label:has-text(\"Material take-off\") >> nth=0"); await settle(300);
  const mtoText = await p.textContent(".bbody");
  check("the take-off weighs pipe, elbows, flanges and gate valves", (await p.$$eval(".bbody tbody tr", rs => rs.filter(r => /^(Pipe|Elbow|Flange, weld|Gate)/.test(r.children[2].textContent) && r.lastElementChild.textContent === "—").length)) === 0 && mtoText.includes("Gate valve"));
  if (shots) await shot("take-off-weights");
  await p.click("label:has-text(\"Material take-off\") >> nth=0"); await settle(200);
  check("valves and flanges go to Datum Pipe with their weight", /VALVE\n(?:    .*\n)*?    WEIGHT \d/.test((await ask("export_pcf", {})).ok));

  // 9. Export the discharge line for Datum Pipe.
  await p.click(".btabs > button:has-text(\"Lines\")"); await settle(300);
  await p.click(".bbody tr.f:has-text(\"P-1003\")"); await settle(300);
  await p.click('.tabs > button:has-text("Home")'); await p.click('.rb:has-text("To Datum Pipe")'); await settle(500);
  const saved = (await calls("save_file")).slice(-1)[0];
  check("the line was exported as a piping component file", saved?.name.endsWith(".pcf") && saved.data.includes("PIPELINE-REFERENCE 4\"-P-1003-CS1") && saved.data.includes("VALVE"), saved?.name);
  await p.click('.tabs > button:has-text("Lists")'); await p.click('.rb:has-text("glTF 3D Model")'); await settle(500);
  check("the model exports as glTF", (await calls("save_file")).slice(-1)[0]?.data.startsWith("data:model/gltf-binary;base64,Z2xURg"));
  if (shots) {
    await open("pumps");
    await p.click(".btabs > button:has-text(\"Lines\")"); await settle(300);
    await p.click(".bbody tr.f:has-text(\"4\\\"-P-1003\")"); await settle(500);
    await p.keyboard.press("1"); await settle(300); await p.click(".tabs > button:has-text(\"Home\")");
    await shot("pumps-3d");
    // Dropping a valve onto the discharge header: the snap marker shows where it goes.
    const s3 = await state(); const hdr = s3.paths.find(x => x[0] === s3.p.lines.find(l => l.number.includes("1005")).id)[1];
    await p.evaluate(() => { window.__drag = { type: "item", kind: "gate" }; });
    const at = await screenOf([hdr[0][0], 10.5, hdr[0][2]]);
    await p.evaluate(([x, y]) => { const el = document.elementFromPoint(x, y); el.dispatchEvent(new DragEvent("dragover", { bubbles: true, cancelable: true, clientX: x, clientY: y, dataTransfer: new DataTransfer() })); }, [at.x, at.y]);
    await p.click(".tabs > button:has-text(\"Fittings\")"); await settle(600);
    await shot("drop-valve");
    await p.evaluate(() => { window.__drag = undefined; });
  }

  // 9b. Print: the 3D view on its own page, then the lists.
  await p.keyboard.press("Control+p"); await settle(1500);
  const pages = await p.$$(".pprev .pwrap");
  check("print previews the view and the lists", pages.length >= 2 && !!(await p.$(".pview img")) && (await p.textContent(".pprev")).includes("Line list"), String(pages.length));
  seen.push(await allText());
  await p.keyboard.press("Escape"); await settle(300);

  // 10. Find the clashes: exactly three, each clickable.
  await open("clashes");
  await p.click('.tabs > button:has-text("Check")'); await p.click('.ribbon .rb:has-text("Check") >> nth=0'); await settle(1500);
  const rows = await p.$$(".bbody tr.f");
  check("Find the clashes: three clashes listed", rows.length === 3, String(rows.length));
  if (rows.length) { await rows[0].click(); await settle(500); check("clicking a clash selects it", !!(await p.evaluate(() => document.querySelector(".insp header b")?.textContent))); }
  if (shots) await shot("clashes");
  seen.push(await allText());

  // 11. Speed: a large plant (24 copies of example 1, over 2,000 parts) orbits smoothly.
  // Measured here in Chromium's software renderer (no graphics card) at 1440×900, where a frame
  // takes about ten times as long as on any real graphics card; the gate is set for that.
  await p.evaluate(() => window.__open("big")); await settle(1000);
  if (await p.$('.dlg header:has-text("Save changes?")')) await p.click('.dlg button:has-text("Don\'t save")');
  await settle(4000);
  const perf = await p.evaluate(() => window.__orbit(60));
  console.log(`     orbit: ${perf.triangles} triangles, mean ${perf.mean.toFixed(1)} ms, worst ${perf.worst.toFixed(1)} ms a frame`);
  check("a 2,000-component plant is drawn and orbits quickly", perf.triangles > 100000 && perf.mean < 250, JSON.stringify(perf));

  // 12. Help: every card, the getting started page and the glossary; no product names anywhere.
  await p.click('.tabs > button:has-text("Help")');
  for (const btn of ["Getting Started", "Glossary", "Shortcuts", "Sources"]) { await p.click(`.ribbon .rb:has-text("${btn}")`); await settle(300); seen.push(await allText()); await p.keyboard.press("Escape"); await settle(100); }
  const helpTexts = [];
  for (const tb of ["Home", "Equipment", "Piping", "Fittings", "Check", "Lists", "View", "Help"]) {
    await p.click(`.tabs > button:has-text("${tb}")`); await settle(100);
    helpTexts.push(...await p.evaluate(async () => { const out = []; for (const b of document.querySelectorAll(".help")) { b.click(); await new Promise(r => setTimeout(r, 30)); out.push(document.querySelector(".dlg")?.innerText ?? ""); document.querySelector(".dlg .x")?.click(); await new Promise(r => setTimeout(r, 10)); } return out; }));
  }
  seen.push(...helpTexts);
  check("every ribbon group and panel has a ? card", helpTexts.length >= 12 && helpTexts.every(x => x.length > 40), String(helpTexts.length));
  const hits = seen.flatMap(named);
  check("no commercial product name appears in the window", hits.length === 0, JSON.stringify([...new Set(hits)]));
  check("the window never named Datum Pipe's moss or Plan's blue", !(await p.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--blue"))).includes("4D7C0F"));

  check("no errors in the page", errors.length === 0, errors.slice(0, 3).join(" | "));
  await b.close(); srv.close(); bridge.kill();
  console.log(failed ? `${failed} failed` : "all passed");
  process.exit(failed ? 1 : 0);
})().catch(e => { console.error(e); bridge.kill(); process.exit(1); });
