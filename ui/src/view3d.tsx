// The 3D view (three.js, MIT), after Datum Pipe's: the plot (ground, grid, coordinates and the
// plant north arrow), areas, and every part the engine generated, drawn from one buffer with a
// colour per part (one draw call, so large plants stay smooth); picking by triangle; standard
// views and fit; the routing tool (orthogonal guides at the working elevation, ports to start and
// end on); components dropped from the catalogue with a snap marker; arrows to move equipment
// and runs along an axis. The engine owns all geometry: this file only draws what it sends.
import { onMount, onCleanup, createEffect, on, Show, For, createSignal } from "solid-js";
import { Portal } from "solid-js/web";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { V3, f32, u32, PartRef, PortRef, Scene, inch, ITEMS, invoke, NPS, Line } from "./api";
import { scene, project, out, sel, setSel, opts, themeTick, mode, setMode, route, setRoute, elev, act, toast, focus, setFocus, openMenu, selLine, placing, setPlacing, loaded, measure, setMeasure, picked, setPicked, colourBy, hidden, isolated, section, setSection, hintSlot, checked, checkStale } from "./store";
import { C, setFit, setStdView, setCamHook, finishRoute, branchSize } from "./cmds";
import { fmt, toU, fromU, unit } from "./units";

/** Plant (east, north, up) → three.js (x, y up, z towards the viewer = south). */
const W = (p: V3) => new THREE.Vector3(p[0], p[2], -p[1]);
const E = (v: THREE.Vector3): V3 => [v.x, -v.z, v.y];
const css = (v: string) => getComputedStyle(document.documentElement).getPropertyValue(v).trim() || "#888";

const COLOURS: Record<string, string> = {
  equipment: "#A7B3BF", nozzle: "#8B97A4", pipe: "#4F6F8F", elbow90: "#4F6F8F", elbow45: "#4F6F8F", tee: "#4F6F8F", cross: "#4F6F8F", weldolet: "#4F6F8F", sockolet: "#4F6F8F", reducer: "#4F6F8F", cap: "#4F6F8F",
  flange: "#6E7A87", blind: "#6E7A87", gasket: "#2E2E2E", gate: "#B8742A", globe: "#B8742A", check: "#B8742A", ball: "#B8742A", butterfly: "#B8742A", strainer: "#9C6B3A", support: "#7C8B5A",
};
const SELECTED = new THREE.Color("#2F8CFF"), PROBLEM = new THREE.Color("#D64545");
/** Colours for "colour by": distinct, readable on light and dark, none close to the selection blue or the problem red. */
const PALETTE = ["#D9822B", "#3A9D5D", "#8E5CB5", "#1FA2A0", "#B8A02A", "#D35D9B", "#6B7F2A", "#A0522D", "#5A6ACF", "#2E8B57", "#C97B9F", "#7F8C3D"];
/** What a line part is grouped by. */
const keyOf = (by: string, l: Line, q: PartRef) => by === "line" ? l.number || "(no number)" : by === "class" ? l.class : by === "fluid" ? l.fluid || "(none)" : inch(q.nps || l.nps);

/** The part under a triangle (parts are in triangle order). */
const partAt = (parts: PartRef[], tri: number) => { let lo = 0, hi = parts.length - 1; while (lo < hi) { const m = (lo + hi + 1) >> 1; if (parts[m].t0 <= tri) lo = m; else hi = m - 1; } return parts[lo]; };

export let snapshot: () => string = () => "";
/** Frame times measured while orbiting (ms), for the speed check. */
export const measureOrbit = async (frames = 120) => (window as any).__orbit?.(frames);

export const View3D = () => {
  let host!: HTMLDivElement, labels!: HTMLDivElement;
  const renderer = new THREE.WebGLRenderer({ antialias: true, preserveDrawingBuffer: true });
  const sc = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(45, 1, 0.05, 5000);
  camera.position.set(30, 25, 30);
  let controls: OrbitControls;
  const world = new THREE.Group(); // in plant axes: rotated so Z is up
  world.rotation.x = -Math.PI / 2;
  sc.add(world);
  const plotG = new THREE.Group(), partsG = new THREE.Group(), toolG = new THREE.Group();
  world.add(plotG, partsG, toolG);
  sc.add(new THREE.HemisphereLight(0xffffff, 0x8899aa, 1.6));
  const sun = new THREE.DirectionalLight(0xffffff, 1.4); sun.position.set(30, 60, 20); sc.add(sun);
  const sun2 = new THREE.DirectionalLight(0xffffff, 0.5); sun2.position.set(-30, 20, -40); sc.add(sun2);
  let mesh: THREE.Mesh | null = null, insMesh: THREE.Mesh | null = null, colours: Float32Array | null = null, sceneData: Scene | null = null, idx0: Uint32Array | null = null;
  const [legend, setLegend] = createSignal<[string, string][]>([]);
  const [legendFolded, setLegendFolded] = createSignal(false);
  /** Whether a part is drawn (hidden and isolated items are not). */
  const shown = (q: PartRef) => { const id = q.line || q.equip, iso = isolated(); return iso ? iso.includes(id) : !hidden().includes(id); };
  // The section box: six clipping planes (three.js axes: plant east = x, north = −z, up = y).
  const clip = [0, 1, 2, 3, 4, 5].map(() => new THREE.Plane());
  const boxHelper = new THREE.Box3Helper(new THREE.Box3(), new THREE.Color("#2F8CFF"));
  boxHelper.visible = false;
  const inSection = (p: V3) => { const s = section(); return !s || [0, 1, 2].every(k => p[k] >= s.lo[k] - 1e-6 && p[k] <= s.hi[k] + 1e-6); };
  let base: Float32Array | null = null;
  const ray = new THREE.Raycaster();
  // Lines (the arrows' shafts) are picked only within a few centimetres, not the default metre.
  ray.params.Line = { threshold: 0.04 };
  let dirty = true;
  const draw = () => { dirty = true; };

  // ------------------------------------------------------------ plot: ground, grid, north, areas
  const buildPlot = () => {
    plotG.clear();
    renderer.setClearColor(new THREE.Color(css("--bg2")));
    {
      const cv = document.createElement("canvas"); cv.width = 2; cv.height = 256;
      const g2 = cv.getContext("2d")!, gr = g2.createLinearGradient(0, 0, 0, 256);
      gr.addColorStop(0, css("--vtop") || "#F2F5F7"); gr.addColorStop(1, css("--vbot") || "#D3DCE3"); g2.fillStyle = gr; g2.fillRect(0, 0, 2, 256);
      const old = sc.background as THREE.Texture | null; if (old && (old as THREE.Texture).isTexture) old.dispose();
      const tex = new THREE.CanvasTexture(cv); tex.colorSpace = THREE.SRGBColorSpace; sc.background = tex;
    }
    const p = project(); if (!p) return;
    const { e, n, grid, ground, north } = p.plot;
    const g = ground;
    const plane = new THREE.Mesh(new THREE.PlaneGeometry(e[1] - e[0], n[1] - n[0]), new THREE.MeshBasicMaterial({ color: new THREE.Color(css("--plot")), transparent: true, opacity: 0.7, depthWrite: false }));
    plane.position.set((e[0] + e[1]) / 2, (n[0] + n[1]) / 2, g - 0.002); plotG.add(plane);
    const minor: number[] = [], major: number[] = [];
    const step = Math.max(grid, (e[1] - e[0]) / 400);
    const maj = step * (step >= 1 ? 5 : 10);
    for (let x = Math.ceil(e[0] / step) * step; x <= e[1] + 1e-9; x += step) (Math.abs(x / maj - Math.round(x / maj)) < 1e-6 ? major : minor).push(x, n[0], g, x, n[1], g);
    for (let y = Math.ceil(n[0] / step) * step; y <= n[1] + 1e-9; y += step) (Math.abs(y / maj - Math.round(y / maj)) < 1e-6 ? major : minor).push(e[0], y, g, e[1], y, g);
    const lines = (a: number[], c: string, o: number) => { const geo = new THREE.BufferGeometry(); geo.setAttribute("position", new THREE.Float32BufferAttribute(a, 3)); plotG.add(new THREE.LineSegments(geo, new THREE.LineBasicMaterial({ color: new THREE.Color(c), transparent: true, opacity: o }))); };
    lines(minor, css("--line"), 0.7); lines(major, css("--line3"), 0.9);
    // Plant north arrow at the plot's north-west corner.
    const sz = Math.max(1.5, Math.min(e[1] - e[0], n[1] - n[0]) * 0.06);
    const shape = new THREE.Shape(); shape.moveTo(0, sz); shape.lineTo(sz * 0.38, -sz * 0.5); shape.lineTo(0, -sz * 0.22); shape.lineTo(-sz * 0.38, -sz * 0.5); shape.closePath();
    const arrow = new THREE.Mesh(new THREE.ShapeGeometry(shape), new THREE.MeshBasicMaterial({ color: new THREE.Color(css("--blue")), side: THREE.DoubleSide }));
    arrow.position.set(e[0] - sz * 1.4, n[1] - sz, g + 0.01); arrow.rotation.z = -north * Math.PI / 180; plotG.add(arrow);
    tag(`north`, [e[0] - sz * 1.4 + Math.sin(north * Math.PI / 180) * sz * 1.5, n[1] - sz + Math.cos(north * Math.PI / 180) * sz * 1.5, g], "N", "nlabel north");
    // Coordinates along the south and west edges, at a round step giving about ten labels.
    const nice = (span: number) => { const r = span / 10, m = Math.pow(10, Math.floor(Math.log10(r))); return [1, 2, 5, 10].map(k => k * m).find(k => k >= r) ?? r; };
    const span = (a: number[]) => a[1] - a[0];
    const le = Math.max(maj, span(e) > 25 ? 10 : nice(span(e))), ln = Math.max(maj, span(n) > 25 ? 10 : nice(span(n)));
    // E along the south edge; N along the west edge, but not at the south-west corner (E's first label is there) nor beside the north arrow.
    for (let x = Math.ceil(e[0] / le) * le; x <= e[1] + 1e-9; x += le) tag(`e${x}`, [x, n[0] - sz * 0.4, g], `E ${+x.toFixed(2)}`, "nlabel coord");
    for (let y = Math.ceil(n[0] / ln) * ln; y <= n[1] + 1e-9; y += ln) if (y - n[0] > ln * 0.5 && n[1] - y > sz * 2.2) tag(`n${y}`, [e[0] - sz * 0.5, y, g], `N ${+y.toFixed(2)}`, "nlabel coord");
    // Areas as boxes.
    for (const a of p.areas) {
      const box = new THREE.Box3(new THREE.Vector3(...a.min), new THREE.Vector3(...a.max));
      const h = new THREE.Box3Helper(box, new THREE.Color(sel()?.kind === "area" && sel()!.id === a.id ? "#2F8CFF" : css("--blue")));
      (h.material as THREE.LineBasicMaterial).transparent = true; (h.material as THREE.LineBasicMaterial).opacity = 0.6;
      plotG.add(h);
      tag(`area${a.id}`, [a.min[0], a.max[1], a.max[2]], a.name, "nlabel area");
    }
  };

  // ------------------------------------------------------------ labels (HTML over the canvas)
  const tags = new Map<string, { el: HTMLDivElement; p: V3 }>();
  let liveTags = new Set<string>();
  const tag = (key: string, p: V3, text: string, cls: string) => {
    liveTags.add(key);
    let t = tags.get(key);
    if (!t) { const el = document.createElement("div"); labels.append(el); t = { el, p }; tags.set(key, t); }
    t.p = p; t.el.className = cls; t.el.textContent = text;
  };
  const placeTags = () => {
    const w = host.clientWidth, h = host.clientHeight;
    for (const { el, p } of tags.values()) {
      const v = W(p).project(camera);
      if (v.z > 1 || v.x < -1.1 || v.x > 1.1 || v.y < -1.1 || v.y > 1.1) { el.style.display = "none"; continue; }
      el.style.display = ""; el.style.transform = `translate(${(v.x + 1) / 2 * w}px, ${(1 - v.y) / 2 * h}px)`;
    }
  };

  // ------------------------------------------------------------ the parts
  const buildParts = () => {
    partsG.clear();
    mesh = null;
    const s = scene(); sceneData = s; if (!s) return;
    const pos = f32(s.pos), nor = f32(s.nor), idx = u32(s.idx);
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.BufferAttribute(pos, 3));
    geo.setAttribute("normal", new THREE.BufferAttribute(nor, 3));
    colours = new Float32Array(pos.length);
    geo.setAttribute("color", new THREE.BufferAttribute(colours, 3));
    geo.setIndex(new THREE.BufferAttribute(idx, 1));
    geo.computeBoundingSphere();
    // Each part's colour by kind; kept so a selection can be undone.
    const c = new THREE.Color();
    const by = colourBy(), lines = new Map((project()?.lines ?? []).map(l => [l.id, l] as [number, Line]));
    const keys = new Map<string, string>();
    if (by !== "kind") {
      const all = [...new Set(s.parts.filter(q => q.line && lines.has(q.line) && q.kind !== "gasket" && q.kind !== "bolts").map(q => keyOf(by, lines.get(q.line)!, q)))];
      const nps = (k: string) => NPS.findIndex(n => inch(n) === k);
      all.sort((a, b) => by === "size" ? nps(a) - nps(b) : a.localeCompare(b, undefined, { numeric: true }));
      all.forEach((k, i) => keys.set(k, PALETTE[i % PALETTE.length]));
    }
    setLegend([...keys.entries()]);
    for (const q of s.parts) {
      const l = q.line ? lines.get(q.line) : undefined;
      c.set(by !== "kind" && l && q.kind !== "gasket" && q.kind !== "bolts" ? keys.get(keyOf(by, l, q)) ?? "#4F6F8F" : COLOURS[q.kind] ?? "#4F6F8F");
      for (let t = q.t0 * 3; t < q.t1 * 3; t++) { const v = idx[t]; colours[v * 3] = c.r; colours[v * 3 + 1] = c.g; colours[v * 3 + 2] = c.b; } }
    base = colours.slice();
    idx0 = idx.slice();
    mesh = new THREE.Mesh(geo, new THREE.MeshStandardMaterial({ vertexColors: true, metalness: 0.25, roughness: 0.55, clippingPlanes: clip, clipShadows: true }));
    partsG.add(mesh);
    if (s.ins_idx.length > 4) {
      const ig = new THREE.BufferGeometry();
      ig.setAttribute("position", new THREE.BufferAttribute(f32(s.ins_pos), 3)); ig.setAttribute("normal", new THREE.BufferAttribute(f32(s.ins_nor), 3)); ig.setIndex(new THREE.BufferAttribute(u32(s.ins_idx), 1));
      const ins = new THREE.Mesh(ig, new THREE.MeshStandardMaterial({ color: 0xd8dee6, transparent: true, opacity: 0.35, depthWrite: false, clippingPlanes: clip }));
      ins.raycast = () => {};
      partsG.add(ins); insMesh = ins;
    } else insMesh = null;
    applyShown(false);
    applySection();
    paintSelection();
    buildLabels();
    buildPorts();
  };

  /** Which parts the selection covers. */
  const selected = (q: PartRef) => {
    const g = picked();
    if (g.length) return g.includes(q.equip) || g.includes(q.line);
    const s = sel(); if (!s) return false;
    if (s.kind === "equip") return q.equip === s.id;
    if (s.kind === "nozzle") return q.equip === s.id && q.kind === "nozzle";
    if (s.kind === "line") return q.line === s.id;
    if (s.kind === "run") return q.line === s.id && q.seg === s.seg && q.kind === "pipe";
    if (s.kind === "corner") return q.line === s.id && q.kind.startsWith("elbow") && q.seg === s.v - 1;
    if (s.kind === "item") return q.line === s.line && q.item === s.id;
    return false;
  };
  /** The clashes found by the last check, while it still stands. */
  const clashes = () => checkStale() ? [] : (checked() ?? []).filter(i => i.kind === "clash");
  /** Whether a part is one of a clash's two, at the clash (a line's pipe pieces: only the one there). */
  const inClash = (i: { at: V3; line: number; item: number; equip: number; line2?: number; item2?: number }, q: PartRef) => {
    const near = [0, 1, 2].every(k => i.at[k] >= q.lo[k] - 0.15 && i.at[k] <= q.hi[k] + 0.15);
    if (!near) return false;
    if (q.line) return (q.line === i.line && q.item === i.item) || (!!i.line2 && q.line === i.line2 && q.item === (i.item2 ?? 0));
    return !!i.equip && q.equip === i.equip && q.kind === "equipment";
  };
  const paintSelection = () => {
    if (!mesh || !colours || !base || !sceneData) return;
    colours.set(base);
    const idx = (mesh.geometry.index!.array) as Uint32Array;
    const bad = new Set((out()?.issues ?? []).filter(i => i.kind === "routing" && i.item).map(i => i.item));
    const cl = clashes();
    for (const q of sceneData.parts) {
      const on = selected(q), wrong = (q.item && bad.has(q.item)) || cl.some(i => inClash(i, q));
      if (!on && !wrong) continue;
      const c = on ? SELECTED : PROBLEM;
      for (let t = q.t0 * 3; t < q.t1 * 3; t++) { const v = idx[t]; colours[v * 3] = c.r; colours[v * 3 + 1] = c.g; colours[v * 3 + 2] = c.b; }
    }
    (mesh.geometry.getAttribute("color") as THREE.BufferAttribute).needsUpdate = true;
    buildGizmo();
    draw();
  };

  /** Hidden parts: their triangles collapse to a point (the buffer and the triangle order stay). */
  const applyShown = (redraw = true) => {
    if (!mesh || !idx0 || !sceneData) return;
    const idx = mesh.geometry.index!.array as Uint32Array;
    idx.set(idx0);
    const all = !hidden().length && !isolated();
    if (!all) for (const q of sceneData.parts) if (!shown(q)) { const v = idx0[q.t0 * 3]; idx.fill(v, q.t0 * 3, q.t1 * 3); }
    mesh.geometry.index!.needsUpdate = true;
    if (insMesh) insMesh.visible = all;
    if (redraw) { buildLabels(); buildGizmo(); draw(); }
  };
  const applySection = () => {
    const s = section();
    renderer.localClippingEnabled = !!s;
    if (s) {
      clip[0].set(new THREE.Vector3(1, 0, 0), -s.lo[0]); clip[1].set(new THREE.Vector3(-1, 0, 0), s.hi[0]);
      clip[2].set(new THREE.Vector3(0, 0, -1), -s.lo[1]); clip[3].set(new THREE.Vector3(0, 0, 1), s.hi[1]);
      clip[4].set(new THREE.Vector3(0, 1, 0), -s.lo[2]); clip[5].set(new THREE.Vector3(0, -1, 0), s.hi[2]);
      boxHelper.box.set(new THREE.Vector3(...s.lo), new THREE.Vector3(...s.hi));
    } else for (const c of clip) c.set(new THREE.Vector3(0, 1, 0), 1e9);
    boxHelper.visible = !!s;
    draw();
  };

  const buildLabels = () => {
    liveTags = new Set([...liveTags].filter(k => !k.startsWith("eq") && !k.startsWith("ln") && !k.startsWith("cl")));
    clashes().forEach((i, k) => tag(`cl${k}`, i.at, String(k + 1), "nlabel clashl"));
    const p = project();
    if (p && opts().labels) {
      const vis = (id: number) => { const iso = isolated(); return iso ? iso.includes(id) : !hidden().includes(id); };
      for (const e of (p.equipment.length <= 60 ? p.equipment : []).filter(e => vis(e.id))) {
        const parts = sceneData?.parts.filter(q => q.equip === e.id && q.kind === "equipment") ?? [];
        const top = parts.length ? Math.max(...parts.map(q => q.hi[2])) : e.pos[2] + 2;
        tag(`eq${e.id}`, [e.pos[0], e.pos[1], top + 0.4], e.tag, "nlabel tagl");
      }
      for (const [id, pts] of (out()?.paths ?? []).length <= 40 ? out()?.paths ?? [] : []) {
        const l = p.lines.find(x => x.id === id); if (!l || pts.length < 2 || !vis(id)) continue;
        if (!(sel()?.kind === "line" && sel()!.id === id) && !picked().includes(id)) continue;
        // On the longest run, in its middle.
        let k = 0, best = -1; for (let i = 0; i + 1 < pts.length; i++) { const d = Math.hypot(pts[i + 1][0] - pts[i][0], pts[i + 1][1] - pts[i][1], pts[i + 1][2] - pts[i][2]); if (d > best) { best = d; k = i; } }
        tag(`ln${id}`, [(pts[k][0] + pts[k + 1][0]) / 2, (pts[k][1] + pts[k + 1][1]) / 2, (pts[k][2] + pts[k + 1][2]) / 2 + 0.3], l.number || "(no number)", "nlabel linel");
      }
    }
    for (const [k, t] of tags) if (!liveTags.has(k)) { t.el.remove(); tags.delete(k); }
  };

  // ------------------------------------------------------------ ports, the route preview, snap marker
  const portG = new THREE.Group(); toolG.add(portG);
  let portList: PortRef[] = [];
  const buildPorts = () => {
    portG.clear();
    portList = [];
    if (mode() !== "route" || !sceneData) { draw(); return; }
    portList = sceneData.ports.filter(pt => !pt.used || pt.kind === "branch");
    for (const pt of portList) {
      const m = new THREE.Mesh(new THREE.SphereGeometry(0.09, 14, 10), new THREE.MeshBasicMaterial({ color: pt.kind === "nozzle" ? 0xb7791f : 0x2f8cff }));
      m.position.set(...pt.p); portG.add(m);
    }
    draw();
  };
  const rubber = new THREE.Line(new THREE.BufferGeometry(), new THREE.LineDashedMaterial({ color: 0x2f8cff, dashSize: 0.25, gapSize: 0.12 }));
  // The measure tool: a line between the two points and its readout.
  const measureLine = new THREE.Line(new THREE.BufferGeometry(), new THREE.LineBasicMaterial({ color: 0xb7791f, depthTest: false }));
  measureLine.renderOrder = 5;
  const measureDots = new THREE.Group();
  toolG.add(measureLine, measureDots);
  const marker = new THREE.Mesh(new THREE.SphereGeometry(0.12, 16, 12), new THREE.MeshBasicMaterial({ color: 0x2f8cff }));
  marker.visible = false; rubber.visible = false;
  toolG.add(rubber, marker);
  let cursor: V3 | null = null;
  let snapSeq = 0;

  const setRubber = (pts: V3[]) => {
    rubber.geometry.dispose();
    rubber.geometry = new THREE.BufferGeometry().setFromPoints(pts.map(p => new THREE.Vector3(...p)));
    rubber.computeLineDistances();
    rubber.visible = pts.length > 1;
    draw();
  };

  // ------------------------------------------------------------ mouse to plant
  const ndc = (e: { clientX: number; clientY: number }) => { const r = host.getBoundingClientRect(); return new THREE.Vector2((e.clientX - r.left) / r.width * 2 - 1, -(e.clientY - r.top) / r.height * 2 + 1); };
  /** Where the mouse meets the horizontal plane at elevation z (plant axes). */
  const onPlane = (e: { clientX: number; clientY: number }, z: number): V3 | null => {
    ray.setFromCamera(ndc(e), camera);
    const hit = new THREE.Vector3();
    return ray.ray.intersectPlane(new THREE.Plane(new THREE.Vector3(0, 1, 0), -z), hit) ? E(hit) : null;
  };
  const pickPart = (e: { clientX: number; clientY: number }): [PartRef, V3] | null => {
    if (!mesh || !sceneData) return null;
    ray.setFromCamera(ndc(e), camera);
    const hit = ray.intersectObject(mesh, false).find(h => inSection(E(h.point)));
    if (!hit || hit.faceIndex == null) return null;
    return [partAt(sceneData.parts, hit.faceIndex), E(hit.point)];
  };
  const nearPort = (e: { clientX: number; clientY: number }): PortRef | null => {
    const r = host.getBoundingClientRect();
    let best: PortRef | null = null, bd = 14;
    for (const pt of portList) {
      const v = W(pt.p).project(camera); if (v.z > 1) continue;
      const d = Math.hypot((v.x + 1) / 2 * r.width - (e.clientX - r.left), (1 - v.y) / 2 * r.height - (e.clientY - r.top));
      if (d < bd) { bd = d; best = pt; }
    }
    return best;
  };
  const snap = (v: number, s: number) => Math.round(v / s) * s;
  const step = () => { const g = project()?.plot.grid ?? 1; return g >= 1 ? g / 10 : g; };
  /** The next route point: on the working plane, square to the last point along its larger offset. */
  const guided = (e: MouseEvent): V3 | null => {
    const r = route(); const z = elev();
    const last = r?.via.length ? r.via[r.via.length - 1] : r?.start ?? null;
    const p = onPlane(e, last && !e.shiftKey ? last[2] : z); if (!p) return null;
    const s = step();
    let q: V3 = [snap(p[0], s), snap(p[1], s), last && !e.shiftKey ? last[2] : z];
    if (last) { const dx = Math.abs(q[0] - last[0]), dy = Math.abs(q[1] - last[1]); q = dx >= dy ? [q[0], last[1], q[2]] : [last[0], q[1], q[2]]; }
    return q;
  };

  // ------------------------------------------------------------ move arrows (gizmo)
  const gizmo = new THREE.Group(), corners = new THREE.Group(); toolG.add(gizmo, corners, boxHelper);
  let giz: { axis: V3; at: V3; kind: "equip" | "run" | "item" | "group" | "corner" } | null = null;
  const buildGizmo = () => {
    gizmo.clear(); corners.clear(); giz = null;
    const s = sel(), p = project(); if (!p || mode() !== "select") return;
    let at: V3 | null = null, axes: V3[] = [[1, 0, 0], [0, 1, 0], [0, 0, 1]], kind: "equip" | "run" | "item" | "group" | "corner" = "equip";
    // A selected line shows its corners as dots to pick and drag.
    if (!picked().length && s && (s.kind === "line" || s.kind === "run" || s.kind === "corner")) {
      const pts = out()?.paths.find(x => x[0] === s.id)?.[1] ?? [];
      const r = 0.1;
      for (let v = 1; v + 1 < pts.length; v++) {
        const on = s.kind === "corner" && s.v === v;
        const m = new THREE.Mesh(new THREE.SphereGeometry(r * (on ? 1.4 : 1), 14, 10), new THREE.MeshBasicMaterial({ color: on ? 0x2f8cff : 0xb7791f, depthTest: false }));
        m.position.set(...pts[v]); m.renderOrder = 4; m.userData = { corner: v, line: s.id };
        corners.add(m);
      }
      if (s.kind === "corner" && pts[s.v]) { at = pts[s.v]; kind = "corner"; }
    }
    const g = picked();
    if (g.length) {
      // The group's arrows sit above the middle of everything picked.
      const parts = sceneData?.parts.filter(q => g.includes(q.equip) || g.includes(q.line)) ?? [];
      if (!parts.length) return;
      const lo = [0, 1, 2].map(k => Math.min(...parts.map(q => q.lo[k]))), hi = [0, 1, 2].map(k => Math.max(...parts.map(q => q.hi[k])));
      at = [(lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2, hi[2] + 0.3]; kind = "group";
    } else if (!s) return;
    else if (s.kind === "item") {
      const l = p.lines.find(x => x.id === s.line), it = l?.items.find(i => i.id === s.id), pts = out()?.paths.find(x => x[0] === s.line)?.[1];
      if (!it || !pts || it.seg + 1 >= pts.length) return;
      const a = pts[it.seg], b = pts[it.seg + 1], L = Math.hypot(b[0] - a[0], b[1] - a[1], b[2] - a[2]) || 1;
      const u: V3 = [(b[0] - a[0]) / L, (b[1] - a[1]) / L, (b[2] - a[2]) / L];
      at = [a[0] + u[0] * it.at, a[1] + u[1] * it.at, a[2] + u[2] * it.at]; axes = [u]; kind = "item";
    }
    if (!g.length && s?.kind === "equip") { const e = p.equipment.find(x => x.id === s.id); if (!e) return; const parts = sceneData?.parts.filter(q => q.equip === e.id) ?? []; at = [e.pos[0], e.pos[1], parts.length ? Math.max(...parts.map(q => q.hi[2])) + 0.2 : e.pos[2] + 1]; }
    if (!g.length && s?.kind === "run") {
      const pts = out()?.paths.find(x => x[0] === s.id)?.[1]; if (!pts || s.seg + 1 >= pts.length) return;
      const a = pts[s.seg], b = pts[s.seg + 1]; at = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2, (a[2] + b[2]) / 2]; kind = "run";
      const u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]], k = u.map(Math.abs).indexOf(Math.max(...u.map(Math.abs)));
      axes = axes.filter((_, i) => i !== k);
    }
    if (!at) return;
    const len = 1.2;
    for (const ax of axes) {
      const col = kind === "item" ? 0xb7791f : Math.abs(ax[0]) > 0.5 ? 0xc0392b : Math.abs(ax[1]) > 0.5 ? 0x1f7a4d : 0x2f8cff;
      for (const sg of [1, -1]) {
        const d = new THREE.Vector3(ax[0] * sg, ax[1] * sg, ax[2] * sg);
        const arr = new THREE.ArrowHelper(d, new THREE.Vector3(...at), len, col, 0.35, 0.22);
        arr.userData = { axis: ax };
        arr.traverse(o => { o.userData = { axis: ax }; });
        gizmo.add(arr);
      }
    }
    giz = { axis: [0, 0, 0], at, kind };
  };
  let dragAxis: { axis: V3; from: V3; start: { clientX: number; clientY: number }; delta: number } | null = null;
  /** Distance along an axis through `at` closest to the mouse ray. */
  const alongAxis = (e: { clientX: number; clientY: number }, at: V3, axis: V3) => {
    ray.setFromCamera(ndc(e), camera);
    const o = W(at), d = W(axis).sub(W([0, 0, 0])).normalize();
    const r0 = ray.ray.origin, rd = ray.ray.direction;
    const w0 = o.clone().sub(r0), a = d.dot(d), b = d.dot(rd), c = rd.dot(rd), dd = d.dot(w0), ee = rd.dot(w0);
    const den = a * c - b * b; if (Math.abs(den) < 1e-9) return 0;
    return (b * ee - c * dd) / den;
  };

  // ------------------------------------------------------------ camera
  const bounds = (): THREE.Box3 => {
    const b = new THREE.Box3();
    const p = project();
    if (sceneData?.parts.length) for (const q of sceneData.parts) { if (q.t1 > q.t0 && shown(q)) { b.expandByPoint(W(q.lo)); b.expandByPoint(W(q.hi)); } }
    if (b.isEmpty() && p) { b.expandByPoint(W([p.plot.e[0], p.plot.n[0], 0])); b.expandByPoint(W([p.plot.e[1], p.plot.n[1], 3])); }
    return b;
  };
  const frame = (b: THREE.Box3, dir?: THREE.Vector3) => {
    const c = b.getCenter(new THREE.Vector3()), r = Math.max(1, b.getSize(new THREE.Vector3()).length() / 2);
    const d = (dir ?? camera.position.clone().sub(controls.target)).normalize();
    const fov = Math.min(camera.fov, 2 * THREE.MathUtils.radToDeg(Math.atan(Math.tan(THREE.MathUtils.degToRad(camera.fov / 2)) * camera.aspect)));
    const dist = r / Math.sin(THREE.MathUtils.degToRad(fov / 2)) * 0.8;
    camera.position.copy(c.clone().add(d.multiplyScalar(dist)));
    camera.near = Math.max(0.02, dist / 2000); camera.far = dist * 50; camera.updateProjectionMatrix();
    controls.target.copy(c); controls.update(); draw();
  };
  const fit = () => frame(bounds());
  /** Zooms towards (f < 1) or away from (f > 1) the point the view turns about. */
  const zoom = (f: number) => { if (!controls) return; const t = controls.target; camera.position.sub(t).multiplyScalar(f).add(t); controls.update(); draw(); };
  // ------------------------------------------------------------ the view cube (after Datum Mesh's)
  // Drawn in SVG from the camera each frame and hit-tested by our own code: a face looks straight
  // at it, an edge from between its two faces, a corner from the three (an isometric view).
  let cubeEl: SVGSVGElement | undefined;
  let cubePolys: { k: number; pts: [number, number][]; depth: number; a: number; u: number; w: number }[] = [];
  const [cubeZone, setCubeZone] = createSignal<{ pts: string; dir: number[]; name: string } | null>(null);
  // Faces in three.js axes (x east, y up, z south), named for the plant.
  const FACES: [string, number[]][] = [["South", [0, 0, 1]], ["North", [0, 0, -1]], ["East", [1, 0, 0]], ["West", [-1, 0, 0]], ["Top", [0, 1, 0]], ["Bottom", [0, -1, 0]]];
  const lookFrom = (d: number[]) => frame(bounds(), new THREE.Vector3(d[0], d[1], d[2] + (d[1] && !d[0] && !d[2] ? 0.0001 : 0)));
  const camR = () => { const e = camera.matrixWorldInverse.elements; return (i: number, j: number) => e[j * 4 + i]; };
  const zoneAt = (e: PointerEvent) => {
    if (!cubeEl) return null;
    const b = cubeEl.getBoundingClientRect(), x = (e.clientX - b.left) * 64 / b.width, y = (e.clientY - b.top) * 64 / b.height;
    const inside = (p: [number, number][]) => { let c = false; for (let i = 0, j = p.length - 1; i < p.length; j = i++) if ((p[i][1] > y) !== (p[j][1] > y) && x < (p[j][0] - p[i][0]) * (y - p[i][1]) / (p[j][1] - p[i][1]) + p[i][0]) c = !c; return c; };
    const hit = [...cubePolys].sort((a, c) => c.depth - a.depth).find(f => inside(f.pts));
    if (!hit) return null;
    const [p0, p1, , p3] = hit.pts, ex = [p1[0] - p0[0], p1[1] - p0[1]], ey = [p3[0] - p0[0], p3[1] - p0[1]], det = ex[0] * ey[1] - ex[1] * ey[0];
    if (Math.abs(det) < 1e-9) return null;
    const dx = x - p0[0], dy = y - p0[1], s = (dx * ey[1] - dy * ey[0]) / det * 2 - 1, t = (ex[0] * dy - ex[1] * dx) / det * 2 - 1;
    const su = Math.abs(s) > 0.62 ? Math.sign(s) : 0, sw = Math.abs(t) > 0.62 ? Math.sign(t) : 0;
    const n = FACES[hit.k][1], dir = [n[0], n[1], n[2]]; dir[hit.u] += su; dir[hit.w] += sw;
    const rng = (c: number): [number, number] => (c === 0 ? [-0.62, 0.62] : c > 0 ? [0.62, 1] : [-1, -0.62]);
    const [u0, u1] = rng(su), [w0, w1] = rng(sw);
    const at = (p: number, q: number) => { const v = [0, 0, 0]; v[hit.a] = n[hit.a]; v[hit.u] = p; v[hit.w] = q; return v; };
    const R = camR(), pr = (v: number[]) => [0, 1].map(i => R(i, 0) * v[0] + R(i, 1) * v[1] + R(i, 2) * v[2]);
    const pts = [at(u0, w0), at(u1, w0), at(u1, w1), at(u0, w1)].map(v => { const r = pr(v); return `${(32 + 17 * r[0]).toFixed(1)},${(32 - 17 * r[1]).toFixed(1)}`; }).join(" ");
    const name = su === 0 && sw === 0 ? `From the ${FACES[hit.k][0].toLowerCase()}` : su !== 0 && sw !== 0 ? "Corner (isometric)" : "Edge";
    return { pts, dir, name };
  };
  const drawCube = () => {
    if (!cubeEl) return;
    const R = camR(), rot = (v: number[]) => [0, 1, 2].map(i => R(i, 0) * v[0] + R(i, 1) * v[1] + R(i, 2) * v[2]);
    const order: [number, number][] = [], gs = FACES.map(([name]) => cubeEl!.querySelector(`[data-face="${name}"]`) as SVGGElement);
    cubePolys = [];
    FACES.forEach(([, n], k) => {
      const a = n.findIndex(c => c !== 0), [u, w] = [0, 1, 2].filter(i => i !== a);
      const corners = [[-1, -1], [1, -1], [1, 1], [-1, 1]];
      const pts = corners.map(([p, q]) => { const v = [0, 0, 0]; v[a] = n[a]; v[u] = p; v[w] = q; return rot(v); });
      const g = gs[k]; if (!g) return;
      const depth = rot(n)[2];
      g.style.display = depth > 0.02 ? "" : "none";
      const xy = pts.map(p => [32 + 17 * p[0], 32 - 17 * p[1]] as [number, number]);
      g.querySelector("polygon.outer")!.setAttribute("points", xy.map(p => `${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(" "));
      if (depth > 0.02) cubePolys.push({ k, pts: xy, depth, a, u, w });
      const inner = corners.map(([p, q]) => { const v = [0, 0, 0]; v[a] = n[a]; v[u] = p * 0.62; v[w] = q * 0.62; const r = rot(v); return `${(32 + 17 * r[0]).toFixed(1)},${(32 - 17 * r[1]).toFixed(1)}`; });
      g.querySelector("polygon.inner")!.setAttribute("points", inner.join(" "));
      const tx = g.querySelector("text")!; tx.setAttribute("x", (32 + 17 * rot(n)[0]).toFixed(1)); tx.setAttribute("y", (35 - 17 * rot(n)[1]).toFixed(1));
      tx.style.opacity = String(Math.min(1, depth * 1.6));
      order.push([depth, k]);
    });
    order.sort((x, y) => x[0] - y[0]).forEach(([, k]) => gs[k] && cubeEl!.appendChild(gs[k]));
  };

  const std = (v: string) => {
    const dirs: Record<string, THREE.Vector3> = { iso: new THREE.Vector3(1, 0.9, 1.1), plan: new THREE.Vector3(0, 1, 0.0001), front: new THREE.Vector3(0, 0.0001, 1), side: new THREE.Vector3(1, 0.0001, 0), back: new THREE.Vector3(0, 0.0001, -1), west: new THREE.Vector3(-1, 0.0001, 0) };
    frame(bounds(), (dirs[v] ?? dirs.iso).clone());
  };

  // ------------------------------------------------------------ dragging equipment (0.2.3)
  // A selected item (or the group picked) can be dragged across the plot with the mouse: its own
  // vertices move with the pointer as a preview, the move snaps to a tenth of the grid, and on
  // release the engine moves it (the lines connected follow). Esc while dragging puts it back.
  let bodyDrag: { ids: number[]; z: number; from: V3; start: { x: number; y: number }; delta: V3; moving: boolean; verts: Uint32Array; base: Float32Array } | null = null;
  const [dragInfo2, setDragInfo2] = createSignal<string | null>(null);
  const dragIds = (q: PartRef): number[] | null => {
    const g = picked();
    if (g.length && (g.includes(q.equip) || g.includes(q.line))) return g;
    const s = sel();
    if (q.line === 0 && q.equip && s && (s.kind === "equip" || s.kind === "nozzle") && s.id === q.equip) return [q.equip];
    return null;
  };
  const vertsOf = (ids: number[]) => {
    if (!mesh || !sceneData) return new Uint32Array();
    const idx = mesh.geometry.index!.array as Uint32Array, set = new Set<number>();
    for (const q of sceneData.parts) if (ids.includes(q.equip) || ids.includes(q.line)) for (let t = q.t0 * 3; t < q.t1 * 3; t++) set.add(idx0 ? idx0[t] : idx[t]);
    return Uint32Array.from(set);
  };
  const previewDelta = (d: V3) => {
    if (!mesh || !bodyDrag) return;
    const pos = mesh.geometry.getAttribute("position") as THREE.BufferAttribute, a = pos.array as Float32Array;
    bodyDrag.verts.forEach((v, i) => { a[v * 3] = bodyDrag!.base[i * 3] + d[0]; a[v * 3 + 1] = bodyDrag!.base[i * 3 + 1] + d[1]; a[v * 3 + 2] = bodyDrag!.base[i * 3 + 2] + d[2]; });
    pos.needsUpdate = true; mesh.geometry.computeBoundingSphere(); draw();
  };
  const endBodyDrag = (commit: boolean) => {
    if (!bodyDrag) return;
    const b = bodyDrag; bodyDrag = null; controls.enabled = true; setDragInfo2(null);
    if (!b.moving) return;
    if (!commit || Math.hypot(...b.delta) < 1e-9) { previewDelta([0, 0, 0]); return; }
    const s = sel();
    if (b.ids.length === 1 && s && (s.kind === "equip" || s.kind === "nozzle")) { const eq = project()?.equipment.find(x => x.id === b.ids[0]); if (eq) act({ SetEquipment: { ...eq, pos: [eq.pos[0] + b.delta[0], eq.pos[1] + b.delta[1], eq.pos[2]] } }).then(o => { if (!o) previewDelta([0, 0, 0]); else C.squareAfterMove(o); }); }
    else act({ MoveGroup: { ids: b.ids, delta: b.delta } }).then(o => { if (!o) previewDelta([0, 0, 0]); else C.squareAfterMove(o); });
  };
  (window as any).__cancelDrag = () => {
    if (bodyDrag) { endBodyDrag(false); return true; }
    if (dragAxis) { dragAxis = null; controls.enabled = true; gizmo.position.set(0, 0, 0); host.title = ""; draw(); return true; }
    return false;
  };

  // ------------------------------------------------------------ events
  let down: { x: number; y: number; b: number } | null = null;
  const onDown = (e: PointerEvent) => {
    host.focus();
    down = { x: e.clientX, y: e.clientY, b: e.button };
    // A press on the selected equipment (or the picked group) starts a drag across the plot.
    if (e.button === 0 && mode() === "select" && !e.ctrlKey && !e.metaKey) {
      const giHit = gizmo.children.length ? (ray.setFromCamera(ndc(e), camera), ray.intersectObjects(gizmo.children.filter(c => c.visible), true)[0]) : undefined;
      // The selected item anywhere under the pointer (a pipe running over it does not get in the way).
      let hit: [PartRef, V3] | null = null;
      if (!giHit && mesh && sceneData) {
        ray.setFromCamera(ndc(e), camera);
        for (const h of ray.intersectObject(mesh, false)) { if (h.faceIndex == null || !inSection(E(h.point))) continue; const q = partAt(sceneData.parts, h.faceIndex); if (dragIds(q)) { hit = [q, E(h.point)]; break; } }
      }
      const ids = hit ? dragIds(hit[0]) : null;
      if (hit && ids) {
        const z = hit[0].line === 0 ? (project()?.equipment.find(x => x.id === hit[0].equip)?.pos[2] ?? 0) : hit[1][2];
        const from = onPlane(e, z);
        if (from) {
          controls.enabled = false;
          const verts = vertsOf(ids), a = (mesh!.geometry.getAttribute("position") as THREE.BufferAttribute).array as Float32Array, base = new Float32Array(verts.length * 3);
          verts.forEach((v, i) => { base[i * 3] = a[v * 3]; base[i * 3 + 1] = a[v * 3 + 1]; base[i * 3 + 2] = a[v * 3 + 2]; });
          bodyDrag = { ids, z, from, start: { x: e.clientX, y: e.clientY }, delta: [0, 0, 0], moving: false, verts, base };
          (e.target as Element).setPointerCapture?.(e.pointerId);
          return;
        }
      }
    }
    if (e.button === 0 && gizmo.children.length && giz) {
      ray.setFromCamera(ndc(e), camera);
      const hit = ray.intersectObjects(gizmo.children.filter(c => c.visible), true)[0];
      if (hit) {
        controls.enabled = false;
        dragAxis = { axis: hit.object.userData.axis, from: giz.at, start: { clientX: e.clientX, clientY: e.clientY }, delta: 0 };
        (e.target as Element).setPointerCapture?.(e.pointerId);
      }
    }
  };
  const onMove = (e: PointerEvent) => {
    if (bodyDrag) {
      if (!bodyDrag.moving && Math.hypot(e.clientX - bodyDrag.start.x, e.clientY - bodyDrag.start.y) < 4) return;
      bodyDrag.moving = true;
      const q = onPlane(e, bodyDrag.z); if (!q) return;
      const st = e.shiftKey ? step() / 10 : step();
      bodyDrag.delta = [snap(q[0] - bodyDrag.from[0], st), snap(q[1] - bodyDrag.from[1], st), 0];
      // Three.js axes inside the rotated world group are the plant's own, so the delta applies as is.
      previewDelta(bodyDrag.delta);
      setDragInfo2(`Move ΔE ${fmt("len", bodyDrag.delta[0])} · ΔN ${fmt("len", bodyDrag.delta[1])}`);
      return;
    }
    if (dragAxis) {
      const t0 = alongAxis(dragAxis.start, dragAxis.from, dragAxis.axis), t = alongAxis(e, dragAxis.from, dragAxis.axis);
      dragAxis.delta = snap(t - t0, step());
      gizmo.position.set(dragAxis.axis[0] * dragAxis.delta, dragAxis.axis[1] * dragAxis.delta, dragAxis.axis[2] * dragAxis.delta);
      host.title = `Move ${fmt("len", dragAxis.delta)}`;
      draw(); return;
    }
    if (mode() === "route") {
      const r = route();
      const pt = nearPort(e);
      cursor = pt ? pt.p : guided(e);
      marker.visible = !!cursor; if (cursor) marker.position.set(...cursor);
      const start = r?.start; if (start && cursor) setRubber([start, ...r!.via, cursor]); else setRubber(r?.start ? [r.start, ...r.via] : []);
      draw();
    }
  };
  const onUp = (e: PointerEvent) => {
    if (bodyDrag) {
      const moved = bodyDrag.moving;
      endBodyDrag(true);
      if (moved) { down = null; return; }
    }
    if (dragAxis) {
      const { axis, delta } = dragAxis; dragAxis = null; controls.enabled = true; gizmo.position.set(0, 0, 0); host.title = "";
      if (Math.abs(delta) > 1e-9) {
        const d: V3 = [axis[0] * delta, axis[1] * delta, axis[2] * delta], s = sel();
        if (giz?.kind === "corner" && s?.kind === "corner") { act({ MoveCorner: { line: s.id, corner: s.v, delta: d } }); draw(); return; }
        if (giz?.kind === "group") { act({ MoveGroup: { ids: picked(), delta: d } }); draw(); return; }
        if (s?.kind === "equip") { const eq = project()?.equipment.find(x => x.id === s.id); if (eq) act({ SetEquipment: { ...eq, pos: [eq.pos[0] + d[0], eq.pos[1] + d[1], eq.pos[2] + d[2]] } }); }
        if (s?.kind === "run") act({ MoveRun: { line: s.id, seg: s.seg, delta: d } });
        if (s?.kind === "item" && giz) act({ MoveItem: { line: s.line, item: s.id, at: [giz.at[0] + d[0], giz.at[1] + d[1], giz.at[2] + d[2]] } });
      }
      draw(); return;
    }
    if (!down || Math.hypot(e.clientX - down.x, e.clientY - down.y) > 4 || down.b !== 0) { down = null; return; }
    down = null;
    if (mode() === "route") return routeClick(e);
    if (mode() === "measure") {
      const hit = pickPart(e);
      const q = hit ? hit[1] : onPlane(e, project()?.plot.ground ?? 0);
      if (!q) return;
      const m = measure();
      setMeasure(m.length >= 2 ? [q] : [...m, q]);
      return;
    }
    if (mode() === "place") { const k = placing(); const p = onPlane(e, project()?.plot.ground ?? 0); if (k && p) { const s = project()?.plot.grid ?? 1; act({ AddEquipment: { kind: k, pos: [snap(p[0], s / 2), snap(p[1], s / 2), project()?.plot.ground ?? 0] } }).then(o => { if (o?.created.length) setSel({ kind: "equip", id: o.created[0] }); }); } setMode("select"); setPlacing(null); return; }
    if (corners.children.length) {
      ray.setFromCamera(ndc(e), camera);
      const c = ray.intersectObjects(corners.children, false)[0];
      if (c) { setSel({ kind: "corner", id: c.object.userData.line, v: c.object.userData.corner }); return; }
    }
    const hit = pickPart(e);
    if (!hit) { setSel(null); setPicked([]); return; }
    const [q] = hit;
    // Ctrl+click builds a group of equipment and lines.
    if (e.ctrlKey || e.metaKey) { C.pick(q.line || q.equip); return; }
    setPicked([]);
    if (q.line === 0) setSel(q.kind === "nozzle" && e.altKey ? { kind: "equip", id: q.equip } : { kind: "equip", id: q.equip });
    else if (q.item) setSel({ kind: "item", id: q.item, line: q.line });
    else if (q.kind === "pipe" && (sel()?.kind === "line" || sel()?.kind === "run" || sel()?.kind === "corner") && (sel() as { id: number }).id === q.line) setSel({ kind: "run", id: q.line, seg: q.seg });
    else if (q.kind === "pipe" && e.shiftKey) setSel({ kind: "run", id: q.line, seg: q.seg });
    else setSel({ kind: "line", id: q.line });
  };
  const routeClick = (e: MouseEvent) => {
    const r = route() ?? { from: null, start: null, via: [], nps: project()?.settings.nps ?? "6" };
    const pt = nearPort(e);
    if (!r.start) {
      if (pt?.kind === "nozzle") setRoute({ ...r, from: { kind: "nozzle", equip: pt.equip, nozzle: pt.tag }, start: pt.p, nps: pt.nps });
      else if (pt?.kind === "branch") setRoute({ ...r, from: { kind: "branch", line: pt.line, item: pt.item, port: pt.branch ?? 0 }, start: pt.p, nps: branchSize(pt.nps) });
      else { const q = cursor ?? guided(e as PointerEvent); if (q) setRoute({ ...r, from: { kind: "point", p: q }, start: q }); }
      buildPorts(); return;
    }
    if (pt && pt.kind === "nozzle") return finishRoute({ kind: "nozzle", equip: pt.equip, nozzle: pt.tag });
    const q = cursor ?? guided(e as PointerEvent); if (!q) return;
    const last = r.via[r.via.length - 1];
    if (last && Math.hypot(q[0] - last[0], q[1] - last[1], q[2] - last[2]) < 1e-6) return;
    setRoute({ ...r, via: [...r.via, q] });
  };
  const onDbl = (e: MouseEvent) => {
    // A double-click ends the route at the point clicked (its clicks already added that corner).
    if (mode() === "route") { const r = route(); if (r?.start && r.via.length) finishRoute({ kind: "point", p: r.via[r.via.length - 1] }, true); return; }
    if (mode() !== "select") return;
    const hit = pickPart(e); if (!hit) return fit();
    const [q] = hit;
    const b = new THREE.Box3(W(q.lo), W(q.hi)); b.expandByScalar(1.5); frame(b);
  };
  const onContext = (e: MouseEvent) => {
    e.preventDefault();
    if (mode() === "route") { if (route()?.start) finishRoute({ kind: "point", p: route()!.via.length ? route()!.via[route()!.via.length - 1] : cursor ?? route()!.start! }, true); else C.cancelTool(); return; }
    const hit = pickPart(e); if (!hit) return;
    const [q, at] = hit;
    if (q.line) {
      openMenu(e, [
        { label: "Select the line", icon: "pipe", run: () => setSel({ kind: "line", id: q.line }) },
        { label: "Select this run", icon: "route", run: () => setSel({ kind: "run", id: q.line, seg: q.seg }), off: q.kind !== "pipe" },
        { label: "Add a jog here", icon: "jog", keys: "J", off: q.kind !== "pipe", run: () => { const pts = out()?.paths.find(x => x[0] === q.line)?.[1]; const a = pts?.[q.seg]; setSel({ kind: "run", id: q.line, seg: q.seg }); C.jog(a ? Math.hypot(at[0] - a[0], at[1] - a[1], at[2] - a[2]) : undefined); } },
        { label: "Take out this jog", icon: "straight", off: q.kind !== "pipe", run: () => { setSel({ kind: "run", id: q.line, seg: q.seg }); C.removeJog(); } },
        "-",
        { label: "Add a component here", icon: "valve", sub: ITEMS.map(([k, l]) => ({ label: l, run: () => C.drop(q.line, k, at) })) },
        { label: "Export this line for Datum Pipe (.pcf)", icon: "export", run: () => C.exportPcf(q.line) },
        "-",
        { label: "Delete", icon: "del", keys: "Del", run: () => { setSel(q.item ? { kind: "item", id: q.item, line: q.line } : { kind: "line", id: q.line }); C.del(); } },
      ]);
    } else {
      const pick = () => { setPicked([]); setSel({ kind: "equip", id: q.equip }); };
      openMenu(e, [
        { label: "Select", icon: "selectall", run: pick },
        { label: "Move to…", icon: "measure", keys: "↑↓←→", run: () => { pick(); C.moveTo(); } },
        { label: "Turn 90°", icon: "rotate", run: () => { pick(); C.rotate(90); } },
        { label: "Turn −90°", icon: "rotate", run: () => { pick(); C.rotate(-90); } },
        { label: "Copy beside it", icon: "copy", keys: "Ctrl+D", run: () => { pick(); C.copyGroup(); } },
        "-",
        { label: "Hide", icon: "hide", keys: "H", run: () => { pick(); C.hide(); } },
        { label: "Show only this", icon: "isolate", keys: "I", run: () => { pick(); C.isolate(); } },
        "-",
        { label: "Delete", icon: "del", keys: "Del", run: () => { pick(); C.del(); } },
      ]);
    }
  };

  // Drag and drop from the catalogue: components snap onto a pipe, equipment onto the plot.
  const dragInfo = () => (window as any).__drag as { type: "item" | "equip"; kind: string } | undefined;
  const onDragOver = (e: DragEvent) => {
    const d = dragInfo(); if (!d) return;
    e.preventDefault();
    if (d.type === "item") {
      const hit = pickPart(e);
      if (hit && hit[0].line) {
        // Ask the engine where it would snap (the free place on that run), or why it cannot.
        const line = hit[0].line, n = ++snapSeq;
        invoke<V3>("snap", { line, kind: d.kind, at: hit[1] }).then(pt => {
          if (n !== snapSeq) return;
          marker.visible = true; marker.position.set(...pt); (marker.material as THREE.MeshBasicMaterial).color.set(0x2f8cff);
          host.title = `Drop on ${project()?.lines.find(l => l.id === line)?.number ?? "the line"} (${inch(hit[0].nps)})`; draw();
        }).catch(err => {
          if (n !== snapSeq) return;
          marker.visible = true; marker.position.set(...hit[1]); (marker.material as THREE.MeshBasicMaterial).color.set(0xd64545);
          host.title = String(err); draw();
        });
        if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
      } else { marker.visible = false; if (e.dataTransfer) e.dataTransfer.dropEffect = "none"; host.title = ""; }
    } else {
      const p = onPlane(e, project()?.plot.ground ?? 0);
      marker.visible = !!p; if (p) marker.position.set(p[0], p[1], p[2]);
    }
    draw();
  };
  const onDrop = (e: DragEvent) => {
    const d = dragInfo(); (window as any).__drag = undefined; snapSeq++; marker.visible = false; host.title = ""; draw();
    if (!d) return;
    e.preventDefault();
    if (d.type === "item") {
      const hit = pickPart(e);
      if (!hit || !hit[0].line) return toast("Drop components onto a pipe.");
      C.drop(hit[0].line, d.kind as any, hit[1]);
    } else {
      const p = onPlane(e, project()?.plot.ground ?? 0); if (!p) return;
      const s = project()?.plot.grid ?? 1;
      act({ AddEquipment: { kind: d.kind, pos: [snap(p[0], s / 2), snap(p[1], s / 2), project()?.plot.ground ?? 0] } }).then(o => { if (o?.created.length) setSel({ kind: "equip", id: o.created[0] }); });
    }
  };
  const onDragLeave = () => { snapSeq++; marker.visible = false; host.title = ""; draw(); };

  onMount(() => {
    host.prepend(renderer.domElement);
    renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
    controls = new OrbitControls(camera, renderer.domElement);
    controls.enableDamping = false;
    controls.mouseButtons = { LEFT: THREE.MOUSE.ROTATE, MIDDLE: THREE.MOUSE.PAN, RIGHT: THREE.MOUSE.PAN };
    controls.addEventListener("change", draw);
    const resize = () => { const w = host.clientWidth, h = host.clientHeight; if (!w || !h) return; renderer.setSize(w, h, false); camera.aspect = w / h; camera.updateProjectionMatrix(); draw(); };
    const ro = new ResizeObserver(resize); ro.observe(host); resize();
    let raf = 0;
    // Arrows pointing along the line of sight cannot be dragged and hide the item under them.
    const gizmoFacing = () => { const v = W([0, 0, 0]).sub(W([0, 0, 0])); camera.getWorldDirection(v); for (const c of gizmo.children) { const ax = c.userData.axis as V3 | undefined; if (ax) c.visible = Math.abs(W(ax).sub(W([0, 0, 0])).normalize().dot(v)) < 0.92; } };
    const loop = () => { raf = requestAnimationFrame(loop); if (!dirty) return; dirty = false; gizmoFacing(); renderer.render(sc, camera); placeTags(); drawCube(); };
    loop();
    setFit(fit); setStdView(std);
    setCamHook({
      get: () => ({ eye: E(camera.position), target: E(controls.target) }),
      set: (eye, target) => { camera.position.copy(W(eye)); controls.target.copy(W(target)); const d = camera.position.distanceTo(controls.target); camera.near = Math.max(0.02, d / 2000); camera.far = d * 50; camera.updateProjectionMatrix(); controls.update(); draw(); },
    });
    snapshot = () => { renderer.render(sc, camera); return renderer.domElement.toDataURL("image/png"); };
    // The speed check: orbit through `frames` frames, rendering each, and give the mean and worst frame times.
    (window as any).__orbit = (frames: number) => {
      const times: number[] = []; const t = controls.target.clone(), p0 = camera.position.clone(); const px = new Uint8Array(4);
      for (let i = 0; i < frames; i++) {
        const a = (i / frames) * Math.PI * 2, r = p0.clone().sub(t);
        camera.position.copy(t.clone().add(r.applyAxisAngle(new THREE.Vector3(0, 1, 0), a / frames * 4))); camera.lookAt(t);
        const s = performance.now(); renderer.render(sc, camera); const gl = renderer.getContext(); gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, px); times.push(performance.now() - s);
      }
      camera.position.copy(p0); controls.update(); draw();
      return { mean: times.reduce((a, b) => a + b, 0) / frames, worst: Math.max(...times), triangles: sceneData?.triangles ?? 0 };
    };
    // For the UI tests: where a plant point is on screen.
    (window as any).__screen = (p: V3) => { const r = host.getBoundingClientRect(); const v = W(p).project(camera); return { x: r.left + (v.x + 1) / 2 * r.width, y: r.top + (1 - v.y) / 2 * r.height }; };
    const el = renderer.domElement;
    el.addEventListener("pointerdown", onDown); el.addEventListener("pointermove", onMove); el.addEventListener("pointerup", onUp);
    el.addEventListener("dblclick", onDbl); el.addEventListener("contextmenu", onContext);
    host.addEventListener("dragover", onDragOver); host.addEventListener("drop", onDrop); host.addEventListener("dragleave", onDragLeave);
    onCleanup(() => { cancelAnimationFrame(raf); ro.disconnect(); renderer.dispose(); controls.dispose(); });
    buildPlot(); buildParts(); fit();
  });

  let firstScene = true;
  createEffect(on(scene, () => { buildParts(); if (firstScene && scene()) { firstScene = false; fit(); } }, { defer: true }));
  createEffect(on(() => [project()?.plot, project()?.areas, themeTick(), sel()?.kind === "area" ? sel() : null], () => { liveTags = new Set([...liveTags].filter(k => k.startsWith("eq") || k.startsWith("ln"))); buildPlot(); buildLabels(); draw(); }, { defer: true }));
  createEffect(on([sel, picked], paintSelection, { defer: true }));
  createEffect(on(colourBy, () => buildParts(), { defer: true }));
  createEffect(on([hidden, isolated], () => applyShown(), { defer: true }));
  createEffect(on(section, applySection, { defer: true }));
  createEffect(on(() => opts().labels, () => { buildLabels(); draw(); }, { defer: true }));
  createEffect(on(() => [checked(), checkStale()], () => { buildLabels(); paintSelection(); }, { defer: true }));
  createEffect(on(() => [sel()?.kind === "line" ? sel()!.id : 0, picked().length], () => { buildLabels(); draw(); }, { defer: true }));
  createEffect(on(mode, () => { buildPorts(); buildGizmo(); if (mode() !== "route") { setRubber([]); marker.visible = false; } host.style.cursor = mode() === "select" ? "" : "crosshair"; if (mode() !== "measure") setMeasure([]); }, { defer: true }));
  createEffect(on(measure, () => {
    const m = measure();
    measureLine.geometry.dispose();
    measureLine.geometry = new THREE.BufferGeometry().setFromPoints(m.map(p => new THREE.Vector3(...p)));
    measureLine.visible = m.length === 2;
    measureDots.clear();
    for (const p of m) { const d = new THREE.Mesh(new THREE.SphereGeometry(0.06, 12, 8), new THREE.MeshBasicMaterial({ color: 0xb7791f, depthTest: false })); d.position.set(...p); d.renderOrder = 6; measureDots.add(d); }
    draw();
  }, { defer: true }));
  createEffect(on(route, () => { const r = route(); setRubber(r?.start ? [r.start, ...r.via] : []); }, { defer: true }));
  createEffect(on(focus, () => { const f = focus(); if (!f) return; const b = new THREE.Box3(W(f.at), W(f.at)); b.expandByScalar(f.r); frame(b); setFocus(null); }, { defer: true }));
  // A new plant: fit it.
  createEffect(on(loaded, () => { firstScene = true; }, { defer: true }));

  const p = () => project();
  return (
    <div class="v3d" ref={host} tabindex="0">
      <div class="nlabels" ref={labels} />
      <div class="vcube" on:pointerdown={(e: PointerEvent) => e.stopPropagation()} on:pointermove={(e: PointerEvent) => { e.stopPropagation(); setCubeZone(zoneAt(e)); }} on:pointerleave={() => setCubeZone(null)} on:pointerup={(e: PointerEvent) => { e.stopPropagation(); const z = zoneAt(e); if (z) lookFrom(z.dir); }}>
        <button class="vhome" aria-label="Home: the isometric view of the whole plant" title="Home: fit the plant, isometric (1)" onClick={() => std("iso")}><svg class="ic"><use href="#i-home" /></svg></button>
        <svg class="cube" ref={cubeEl} viewBox="0 0 64 64" width="84" height="84">
          {FACES.map(([n]) => <g class={`cf cf-${n.toLowerCase()}`} data-face={n} aria-label={`Look from the ${n.toLowerCase()}`}><polygon class="outer" /><polygon class="inner" /><text text-anchor="middle">{n === "Bottom" ? "Under" : n}</text></g>)}
          <Show when={cubeZone()}>{z => <polygon class="zone" points={z().pts}><title>{z().name}</title></polygon>}</Show>
        </svg>
      </div>
      <Show when={p() && !p()!.equipment.length && !p()!.lines.length && mode() === "select"}>
        <div class="empty"><b>An empty plot</b><span>Drag equipment from the catalogue on the left onto the plot, or choose it on the Equipment tab. Then route lines between nozzles with Piping › Route Line.</span>
          <span class="ibtns"><button class="btn" onClick={() => C.place("vertical_vessel")}>Place a vessel</button><button class="btn" onClick={() => C.newProject("pumps")}>Open an example</button></span></div>
      </Show>
      <Show when={hintSlot()}>{slot => <Portal mount={slot()}>
      <Show when={dragInfo2()}><div class="toolhint">{dragInfo2()}<span class="muted"> · Shift: finer · Esc puts it back</span></div></Show>
      <Show when={mode() === "route"}><div class="toolhint">{route()?.start ? "Click to add a corner (orthogonal to the last point) · click a nozzle to end there · double-click or right-click to end at a free point · Esc to cancel" : "Click a nozzle, a tee's branch end or a point on the plot to start the line"}<span class="muted"> · Elevation {fmt("coord", elev())} · Shift: use the working elevation</span></div></Show>
      <Show when={mode() === "measure"}><div class="toolhint">{measure().length === 2 ? (() => { const [a, b] = measure(); const d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]]; return <span><b>{fmt("len", Math.hypot(d[0], d[1], d[2]))}</b> · ΔE {fmt("len", d[0])} · ΔN {fmt("len", d[1])} · ΔEL {fmt("len", d[2])} · horizontal {fmt("len", Math.hypot(d[0], d[1]))}</span>; })() : measure().length === 1 ? "Click the second point" : "Measure: click a point on a part or on the plot"}<span class="muted"> · Esc to finish</span></div></Show>
      <Show when={mode() === "place"}><div class="toolhint">Click on the plot to place the {placing()?.replace("_", " ")} · Esc to cancel</div></Show>
      <Show when={!dragInfo2() && (sel()?.kind === "equip" || picked().length > 0) && mode() === "select"}><div class="toolhint sub">Drag it across the plot (or its arrows) · arrow keys nudge it a grid step · right-click to turn or type a position</div></Show>
      <Show when={selLine() && sel()?.kind === "line"}><div class="toolhint sub">Click a straight run to move it · click an amber corner dot to move the corner · Piping › Add Jog steps a run over an obstacle · drag components from the catalogue onto the pipe</div></Show>
      <Show when={sel()?.kind === "corner"}><div class="toolhint sub">Corner {(sel() as { v: number }).v + 1}: drag its arrows to move it; the runs on both sides stay square · Esc or click elsewhere to finish</div></Show>
      <Show when={hidden().length || isolated()}><div class="hidnote">{isolated() ? `Showing ${isolated()!.length} item${isolated()!.length === 1 ? "" : "s"} only` : `${hidden().length} hidden`} · <a onClick={() => C.showAll()}>Show all</a></div></Show>
      </Portal>}</Show>
      <div class="zoomctl"><button title="Zoom out" aria-label="Zoom out" onClick={() => zoom(1.25)}>−</button><button title="Fit the whole plant in view (F)" onClick={() => C.fit()}>Fit</button><button title="Zoom in" aria-label="Zoom in" onClick={() => zoom(0.8)}>+</button></div>
      <Show when={legend().length}><div class="legend lcard" classList={{ folded: legendFolded() }}><button class="lfold" title={legendFolded() ? "Show the key" : "Fold the key away"} onClick={() => setLegendFolded(!legendFolded())}>{legendFolded() ? "▸ Key" : "▾"}</button><b>{({ line: "Line", class: "Pipe class", fluid: "Fluid", size: "Size" } as Record<string, string>)[colourBy()]}</b><For each={legend().slice(0, 16)}>{([k, c]) => <span><i style={{ background: c }} />{k}</span>}</For>{legend().length > 16 && <span class="muted">+{legend().length - 16} more</span>}</div></Show>
      <Show when={section()}><div class="secbox"><b>Section box</b>
        <For each={[["E", 0], ["N", 1], ["EL", 2]] as [string, number][]}>{([n, k]) => <label>{n}<input type="number" step="0.5" value={+toU("coord", section()!.lo[k]).toFixed(2)} onChange={e => { const v = fromU("coord", +e.currentTarget.value), s = section()!; if (Number.isFinite(v) && v < s.hi[k]) { const lo = [...s.lo] as V3; lo[k] = v; setSection({ lo, hi: s.hi }); } }} />to<input type="number" step="0.5" value={+toU("coord", section()!.hi[k]).toFixed(2)} onChange={e => { const v = fromU("coord", +e.currentTarget.value), s = section()!; if (Number.isFinite(v) && v > s.lo[k]) { const hi = [...s.hi] as V3; hi[k] = v; setSection({ lo: s.lo, hi }); } }} /><span class="unit">{unit("coord")}</span></label>}</For>
        <button class="btn" onClick={() => setSection(null)}>Turn off</button></div></Show>
    </div>
  );
};
