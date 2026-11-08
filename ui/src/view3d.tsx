// The 3D view (three.js, MIT), after Datum Pipe's: the plot (ground, grid, coordinates and the
// plant north arrow), areas, and every part the engine generated, drawn from one buffer with a
// colour per part (one draw call, so large plants stay smooth); picking by triangle; standard
// views and fit; the routing tool (orthogonal guides at the working elevation, ports to start and
// end on); components dropped from the catalogue with a snap marker; arrows to move equipment
// and runs along an axis. The engine owns all geometry: this file only draws what it sends.
import { onMount, onCleanup, createEffect, on, Show } from "solid-js";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { V3, f32, u32, PartRef, PortRef, Scene, inch, ITEMS, invoke } from "./api";
import { scene, project, out, sel, setSel, opts, themeTick, mode, setMode, route, setRoute, elev, act, toast, focus, setFocus, openMenu, selLine, placing, setPlacing, loaded, measure, setMeasure } from "./store";
import { C, setFit, setStdView, finishRoute } from "./cmds";
import { fmt } from "./units";

/** Plant (east, north, up) → three.js (x, y up, z towards the viewer = south). */
const W = (p: V3) => new THREE.Vector3(p[0], p[2], -p[1]);
const E = (v: THREE.Vector3): V3 => [v.x, -v.z, v.y];
const css = (v: string) => getComputedStyle(document.documentElement).getPropertyValue(v).trim() || "#888";

const COLOURS: Record<string, string> = {
  equipment: "#A7B3BF", nozzle: "#8B97A4", pipe: "#4F6F8F", elbow90: "#4F6F8F", elbow45: "#4F6F8F", tee: "#4F6F8F", reducer: "#4F6F8F", cap: "#4F6F8F",
  flange: "#6E7A87", blind: "#6E7A87", gasket: "#2E2E2E", gate: "#B8742A", globe: "#B8742A", check: "#B8742A", ball: "#B8742A", strainer: "#9C6B3A",
};
const SELECTED = new THREE.Color("#2F8CFF"), PROBLEM = new THREE.Color("#D64545");

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
  let mesh: THREE.Mesh | null = null, colours: Float32Array | null = null, sceneData: Scene | null = null;
  let base: Float32Array | null = null;
  const ray = new THREE.Raycaster();
  let dirty = true;
  const draw = () => { dirty = true; };

  // ------------------------------------------------------------ plot: ground, grid, north, areas
  const buildPlot = () => {
    plotG.clear();
    renderer.setClearColor(new THREE.Color(css("--bg2")));
    const p = project(); if (!p) return;
    const { e, n, grid, ground, north } = p.plot;
    const g = ground;
    const plane = new THREE.Mesh(new THREE.PlaneGeometry(e[1] - e[0], n[1] - n[0]), new THREE.MeshBasicMaterial({ color: new THREE.Color(css("--panel2")), transparent: true, opacity: 0.55, depthWrite: false }));
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
    const le = Math.max(maj, nice(e[1] - e[0])), ln = Math.max(maj, nice(n[1] - n[0]));
    for (let x = Math.ceil(e[0] / le) * le; x <= e[1] + 1e-9; x += le) tag(`e${x}`, [x, n[0] - sz * 0.4, g], `E ${+x.toFixed(2)}`, "nlabel coord");
    for (let y = Math.ceil(n[0] / ln) * ln; y <= n[1] + 1e-9; y += ln) tag(`n${y}`, [e[0] - sz * 0.5, y, g], `N ${+y.toFixed(2)}`, "nlabel coord");
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
    for (const q of s.parts) { c.set(COLOURS[q.kind] ?? "#4F6F8F"); for (let t = q.t0 * 3; t < q.t1 * 3; t++) { const v = idx[t]; colours[v * 3] = c.r; colours[v * 3 + 1] = c.g; colours[v * 3 + 2] = c.b; } }
    base = colours.slice();
    mesh = new THREE.Mesh(geo, new THREE.MeshStandardMaterial({ vertexColors: true, metalness: 0.25, roughness: 0.55 }));
    partsG.add(mesh);
    if (s.ins_idx.length > 4) {
      const ig = new THREE.BufferGeometry();
      ig.setAttribute("position", new THREE.BufferAttribute(f32(s.ins_pos), 3)); ig.setAttribute("normal", new THREE.BufferAttribute(f32(s.ins_nor), 3)); ig.setIndex(new THREE.BufferAttribute(u32(s.ins_idx), 1));
      const ins = new THREE.Mesh(ig, new THREE.MeshStandardMaterial({ color: 0xd8dee6, transparent: true, opacity: 0.35, depthWrite: false }));
      ins.raycast = () => {};
      partsG.add(ins);
    }
    paintSelection();
    buildLabels();
    buildPorts();
  };

  /** Which parts the selection covers. */
  const selected = (q: PartRef) => {
    const s = sel(); if (!s) return false;
    if (s.kind === "equip") return q.equip === s.id;
    if (s.kind === "nozzle") return q.equip === s.id && q.kind === "nozzle";
    if (s.kind === "line") return q.line === s.id;
    if (s.kind === "run") return q.line === s.id && q.seg === s.seg && q.kind === "pipe";
    if (s.kind === "item") return q.line === s.line && q.item === s.id;
    return false;
  };
  const paintSelection = () => {
    if (!mesh || !colours || !base || !sceneData) return;
    colours.set(base);
    const idx = (mesh.geometry.index!.array) as Uint32Array;
    const bad = new Set((out()?.issues ?? []).filter(i => i.kind === "routing" && i.item).map(i => i.item));
    for (const q of sceneData.parts) {
      const on = selected(q), wrong = q.item && bad.has(q.item);
      if (!on && !wrong) continue;
      const c = on ? SELECTED : PROBLEM;
      for (let t = q.t0 * 3; t < q.t1 * 3; t++) { const v = idx[t]; colours[v * 3] = c.r; colours[v * 3 + 1] = c.g; colours[v * 3 + 2] = c.b; }
    }
    (mesh.geometry.getAttribute("color") as THREE.BufferAttribute).needsUpdate = true;
    buildGizmo();
    draw();
  };

  const buildLabels = () => {
    liveTags = new Set([...liveTags].filter(k => !k.startsWith("eq") && !k.startsWith("ln")));
    const p = project();
    if (p && opts().labels) {
      for (const e of p.equipment.length <= 60 ? p.equipment : []) {
        const parts = sceneData?.parts.filter(q => q.equip === e.id && q.kind === "equipment") ?? [];
        const top = parts.length ? Math.max(...parts.map(q => q.hi[2])) : e.pos[2] + 2;
        tag(`eq${e.id}`, [e.pos[0], e.pos[1], top + 0.4], e.tag, "nlabel tagl");
      }
      for (const [id, pts] of (out()?.paths ?? []).length <= 40 ? out()?.paths ?? [] : []) {
        const l = p.lines.find(x => x.id === id); if (!l || pts.length < 2) continue;
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
    const hit = ray.intersectObject(mesh, false)[0];
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
  const gizmo = new THREE.Group(); toolG.add(gizmo);
  let giz: { axis: V3; at: V3; kind: "equip" | "run" | "item" } | null = null;
  const buildGizmo = () => {
    gizmo.clear(); giz = null;
    const s = sel(), p = project(); if (!s || !p || mode() !== "select") return;
    let at: V3 | null = null, axes: V3[] = [[1, 0, 0], [0, 1, 0], [0, 0, 1]], kind: "equip" | "run" | "item" = "equip";
    if (s.kind === "item") {
      const l = p.lines.find(x => x.id === s.line), it = l?.items.find(i => i.id === s.id), pts = out()?.paths.find(x => x[0] === s.line)?.[1];
      if (!it || !pts || it.seg + 1 >= pts.length) return;
      const a = pts[it.seg], b = pts[it.seg + 1], L = Math.hypot(b[0] - a[0], b[1] - a[1], b[2] - a[2]) || 1;
      const u: V3 = [(b[0] - a[0]) / L, (b[1] - a[1]) / L, (b[2] - a[2]) / L];
      at = [a[0] + u[0] * it.at, a[1] + u[1] * it.at, a[2] + u[2] * it.at]; axes = [u]; kind = "item";
    }
    if (s.kind === "equip") { const e = p.equipment.find(x => x.id === s.id); if (!e) return; const parts = sceneData?.parts.filter(q => q.equip === e.id) ?? []; at = [e.pos[0], e.pos[1], parts.length ? Math.max(...parts.map(q => q.hi[2])) + 0.2 : e.pos[2] + 1]; }
    if (s.kind === "run") {
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
    if (sceneData?.parts.length) for (const q of sceneData.parts) { if (q.t1 > q.t0) { b.expandByPoint(W(q.lo)); b.expandByPoint(W(q.hi)); } }
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
  const std = (v: string) => {
    const dirs: Record<string, THREE.Vector3> = { iso: new THREE.Vector3(1, 0.9, 1.1), plan: new THREE.Vector3(0, 1, 0.0001), front: new THREE.Vector3(0, 0.0001, 1), side: new THREE.Vector3(1, 0.0001, 0), back: new THREE.Vector3(0, 0.0001, -1), west: new THREE.Vector3(-1, 0.0001, 0) };
    frame(bounds(), (dirs[v] ?? dirs.iso).clone());
  };

  // ------------------------------------------------------------ events
  let down: { x: number; y: number; b: number } | null = null;
  const onDown = (e: PointerEvent) => {
    host.focus();
    down = { x: e.clientX, y: e.clientY, b: e.button };
    if (e.button === 0 && gizmo.children.length && giz) {
      ray.setFromCamera(ndc(e), camera);
      const hit = ray.intersectObjects(gizmo.children, true)[0];
      if (hit) {
        controls.enabled = false;
        dragAxis = { axis: hit.object.userData.axis, from: giz.at, start: { clientX: e.clientX, clientY: e.clientY }, delta: 0 };
        (host as any).setPointerCapture?.(e.pointerId);
      }
    }
  };
  const onMove = (e: PointerEvent) => {
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
    if (dragAxis) {
      const { axis, delta } = dragAxis; dragAxis = null; controls.enabled = true; gizmo.position.set(0, 0, 0); host.title = "";
      if (Math.abs(delta) > 1e-9) {
        const d: V3 = [axis[0] * delta, axis[1] * delta, axis[2] * delta], s = sel();
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
    const hit = pickPart(e);
    if (!hit) { setSel(null); return; }
    const [q] = hit;
    if (q.line === 0) setSel(q.kind === "nozzle" && e.altKey ? { kind: "equip", id: q.equip } : { kind: "equip", id: q.equip });
    else if (q.item) setSel({ kind: "item", id: q.item, line: q.line });
    else if (q.kind === "pipe" && sel()?.kind === "line" && sel()!.id === q.line) setSel({ kind: "run", id: q.line, seg: q.seg });
    else if (q.kind === "pipe" && e.shiftKey) setSel({ kind: "run", id: q.line, seg: q.seg });
    else setSel({ kind: "line", id: q.line });
  };
  const routeClick = (e: MouseEvent) => {
    const r = route() ?? { from: null, start: null, via: [], nps: project()?.settings.nps ?? "6" };
    const pt = nearPort(e);
    if (!r.start) {
      if (pt?.kind === "nozzle") setRoute({ ...r, from: { kind: "nozzle", equip: pt.equip, nozzle: pt.tag }, start: pt.p, nps: pt.nps });
      else if (pt?.kind === "branch") setRoute({ ...r, from: { kind: "branch", line: pt.line, item: pt.item }, start: pt.p, nps: pt.nps });
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
        { label: "Select the line", run: () => setSel({ kind: "line", id: q.line }) },
        { label: "Select this run", run: () => setSel({ kind: "run", id: q.line, seg: q.seg }), off: q.kind !== "pipe" },
        "-",
        { label: "Add a component here", sub: ITEMS.map(([k, l]) => ({ label: l, run: () => C.drop(q.line, k, at) })) },
        { label: "Export this line for Datum Pipe (.pcf)", run: () => C.exportPcf(q.line) },
        "-",
        { label: "Delete", run: () => { setSel(q.item ? { kind: "item", id: q.item, line: q.line } : { kind: "line", id: q.line }); C.del(); } },
      ]);
    } else openMenu(e, [{ label: "Select", run: () => setSel({ kind: "equip", id: q.equip }) }, { label: "Delete", run: () => { setSel({ kind: "equip", id: q.equip }); C.del(); } }]);
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
    const loop = () => { raf = requestAnimationFrame(loop); if (!dirty) return; dirty = false; renderer.render(sc, camera); placeTags(); };
    loop();
    setFit(fit); setStdView(std);
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
  createEffect(on(sel, paintSelection, { defer: true }));
  createEffect(on(() => opts().labels, () => { buildLabels(); draw(); }, { defer: true }));
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
      <Show when={p() && !p()!.equipment.length && !p()!.lines.length && mode() === "select"}>
        <div class="empty"><b>An empty plot</b><span>Drag equipment from the catalogue on the left onto the plot, or choose it on the Equipment tab. Then route lines between nozzles with Piping › Route Line.</span>
          <span class="ibtns"><button class="btn" onClick={() => C.place("vertical_vessel")}>Place a vessel</button><button class="btn" onClick={() => C.newProject("pumps")}>Open an example</button></span></div>
      </Show>
      <Show when={mode() === "route"}><div class="toolhint">{route()?.start ? "Click to add a corner (orthogonal to the last point) · click a nozzle to end there · double-click or right-click to end at a free point · Esc to cancel" : "Click a nozzle, a tee's branch end or a point on the plot to start the line"}<span class="muted"> · Elevation {fmt("coord", elev())} · Shift: use the working elevation</span></div></Show>
      <Show when={mode() === "measure"}><div class="toolhint">{measure().length === 2 ? (() => { const [a, b] = measure(); const d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]]; return <span><b>{fmt("len", Math.hypot(d[0], d[1], d[2]))}</b> · ΔE {fmt("len", d[0])} · ΔN {fmt("len", d[1])} · ΔEL {fmt("len", d[2])} · horizontal {fmt("len", Math.hypot(d[0], d[1]))}</span>; })() : measure().length === 1 ? "Click the second point" : "Measure: click a point on a part or on the plot"}<span class="muted"> · Esc to finish</span></div></Show>
      <Show when={mode() === "place"}><div class="toolhint">Click on the plot to place the {placing()?.replace("_", " ")} · Esc to cancel</div></Show>
      <Show when={selLine() && sel()?.kind === "line"}><div class="toolhint sub">Click a straight run of the line to move it along an axis · drag components from the catalogue onto the pipe</div></Show>
    </div>
  );
};
