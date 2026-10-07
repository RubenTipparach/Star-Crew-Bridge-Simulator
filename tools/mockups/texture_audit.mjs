#!/usr/bin/env node
/*
 * texture_audit.mjs: what each surface of a mockup is textured with, headless.
 *
 * The owner, 2026-10-07: "did you get rid of all the initial textures from our very first prototype?",
 * after "basically everything using the metal tile grid needs to get replaced with custom textures". The first
 * prototype's textures are the eleven tiling Material Maker layers of data/materials/materials.json (layers 0-10:
 * bulkhead, deck_tiles, deck_plate, ceiling, trim, hazard, light_panel, machinery, crate, hull, hull_dark). This
 * reads every triangle the page draws (window.MOCKUP_SCENE), with the layer its vertices sample (surfLayer), and
 * sums the area by kind:
 *   prototype   a materials.json layer (0-10): the tiling "metal tile grid";
 *   panels      a layer baked by the panel build (data/materials/panels.json, first_layer and up);
 *   atlas       a prop's own baked atlas (the layers loadPanels adds after the panel layers, and the big arrays);
 *   untextured  a mesh with no surfLayer: flat or vertex colour (screens, glass, markers, stand-in blocks).
 * Area is world area in m^2, so a big wall counts for more than a bolt. Grouped by compartment (the mesh's
 * ancestor named after a layout compartment, else the mesh path) and by role.
 *
 * A measurement instrument (CLAUDE.md section 4): it changes nothing.
 *
 * Usage: node tools/mockups/texture_audit.mjs [page.html] [--setup "js run in the page first"] [--only <compartment>]
 *        [--json out.json]. Default page: docs/mockups/deck-plan.html.
 */
import { createRequire } from "node:module";
import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
function loadPlaywright() {
  try { return require("playwright"); } catch (_) { /* fall through to the global install */ }
  return require(path.join(execSync("npm root -g").toString().trim(), "playwright"));
}
const { chromium } = loadPlaywright();
const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const CACHE = path.join(ROOT, "tools", "mockups", ".cache");
const args = process.argv.slice(2);
const opt = (name, dflt) => { const i = args.indexOf(name); return i >= 0 ? args.splice(i, 2)[1] : dflt; };
const setup = opt("--setup", null), only = opt("--only", null), jsonOut = opt("--json", null);
const file = path.resolve(args[0] || path.join(ROOT, "docs/mockups/deck-plan.html"));

const MATS = JSON.parse(fs.readFileSync(path.join(ROOT, "data/materials/materials.json"), "utf8")).materials;
const PROTO = Object.fromEntries(Object.entries(MATS).map(([k, m]) => [m.layer, k]));
const PANELS = JSON.parse(fs.readFileSync(path.join(ROOT, "data/materials/panels.json"), "utf8"));
const panelLayers = [];
(function walk(o) {
  if (Array.isArray(o)) o.forEach(walk);
  else if (o && typeof o === "object") { if (Number.isInteger(o.layer)) panelLayers.push(o.layer); Object.values(o).forEach(walk); }
})(PANELS);
const PANEL_MIN = PANELS.layers.first_layer, PANEL_MAX = Math.max(...panelLayers);
// Compartments as the deck plan draws them: the layout, then each patch's compartments replacing or adding by id.
const COMP_BRUSHES = new Map();
for (const f of ["layout.json", "command_suite.json", "deck_access.json", "engineering.json"]) {
  const d = JSON.parse(fs.readFileSync(path.join(ROOT, "data/ships/tern", f), "utf8"));
  for (const c of d.compartments || []) if (c.brushes) COMP_BRUSHES.set(c.id, c.brushes);
}
const COMPS = new Set(COMP_BRUSHES.keys());
const inPoly = (x, z, poly) => { let o = false; for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) { const [xi, zi] = poly[i], [xj, zj] = poly[j]; if ((zi > z) !== (zj > z) && x < ((xj - xi) * (z - zi)) / (zj - zi) + xi) o = !o; } return o; };
/** The compartment whose brush holds a point (0.35 m of slack up and down for floors, ceilings and wall faces). */
function compAt(x, y, z) {
  for (const [id, bs] of COMP_BRUSHES) for (const b of bs) if (y >= b.y[0] - 0.35 && y <= b.y[1] + 0.35 && inPoly(x, z, b.poly)) return id;
  for (const [id, bs] of COMP_BRUSHES) for (const b of bs) if (y >= b.y[0] - 0.35 && y <= b.y[1] + 0.35)
    for (const [dx, dz] of [[0.35, 0], [-0.35, 0], [0, 0.35], [0, -0.35]]) if (inPoly(x + dx, z + dz, b.poly)) return id;
  return null;
}

function kindOf(layer) {
  if (layer === null) return "untextured";
  const l = layer % 1000;
  if (layer < 1000 && PROTO[l] !== undefined) return "prototype";
  if (layer < 1000 && l >= PANEL_MIN && l <= PANEL_MAX) return "panels";
  return "atlas";
}

const browser = await chromium.launch({ args: ["--use-gl=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"] });
const page = await browser.newPage({ viewport: { width: 800, height: 500 } });
await page.route("https://cdn.jsdelivr.net/npm/**", (route) => {
  const rel = route.request().url().replace("https://cdn.jsdelivr.net/npm/", "").split("?")[0];
  route.fulfill({ body: fs.readFileSync(path.join(CACHE, rel)), contentType: "application/javascript" });
});
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));
await page.goto("file://" + file);
for (let i = 0; i < 100; i++) { if (await page.evaluate(() => window.MOCKUP_READY)) break; await page.waitForTimeout(3000); }
if (errors.length) throw new Error(`${path.basename(file)}: ${errors[0]}`);
if (setup) await page.evaluate(setup);
const rows = await page.evaluate((comps) => {
  const C = new Set(comps), out = [];
  window.MOCKUP_SCENE.updateMatrixWorld(true);
  window.MOCKUP_SCENE.traverse((o) => {
    if (!o.isMesh || !o.geometry || !o.geometry.attributes.position) return;
    for (let v = o; v; v = v.parent) if (!v.visible) return;
    let comp = null;
    for (let v = o; v && !comp; v = v.parent) if (C.has(v.name)) comp = v.name;
    for (let v = o; v && !comp; v = v.parent) if (v.userData && C.has(v.userData.compartment)) comp = v.userData.compartment;
    const g = o.geometry, pos = g.attributes.position, lay = g.attributes.surfLayer, idx = g.index;
    const roles = g.userData && g.userData.roles ? Object.entries(g.userData.roles) : [];
    const mats = [];
    if (o.isInstancedMesh) for (let i = 0; i < o.count; i++) { const im = new o.matrixWorld.constructor(); o.getMatrixAt(i, im); mats.push(im.premultiply(o.matrixWorld)); }
    else mats.push(o.matrixWorld);
    const n = idx ? idx.count : pos.count, V = new o.position.constructor(), W = new o.position.constructor(), X = new o.position.constructor();
    const acc = new Map();
    for (const m of mats) for (let t = 0; t < n; t += 3) {
      const a = idx ? idx.getX(t) : t, b = idx ? idx.getX(t + 1) : t + 1, c = idx ? idx.getX(t + 2) : t + 2;
      V.fromBufferAttribute(pos, a).applyMatrix4(m); W.fromBufferAttribute(pos, b).applyMatrix4(m); X.fromBufferAttribute(pos, c).applyMatrix4(m);
      const area = W.clone().sub(V).cross(X.clone().sub(V)).length() / 2;
      const r = roles.find(([, [k, cnt]]) => k <= t && t < k + cnt);
      const layer = lay ? Math.round(lay.getX(a)) : null;
      const cx = Math.round((V.x + W.x + X.x) / 3 / 0.5), cy = Math.round((V.y + W.y + X.y) / 3 / 0.5), cz = Math.round((V.z + W.z + X.z) / 3 / 0.5);
      const mat = Array.isArray(o.material) ? o.material[0] : o.material;
      const key = (r ? r[0] : "unnamed") + "|" + layer + "|" + cx + "," + cy + "," + cz + "|" + (mat.type || "") + (mat.color ? ":" + mat.color.getHexString() : "");
      acc.set(key, (acc.get(key) || 0) + area);
    }
    let pathName = o.name || o.type;
    for (let v = o.parent; v && v.parent; v = v.parent) if (v.name) { pathName = v.name + "/" + pathName; break; }
    for (const [key, area] of acc) { const [role, layer, cell, mat] = key.split("|"); out.push({ comp, mesh: pathName, role, layer: layer === "null" ? null : Number(layer), cell: cell.split(",").map((v) => Number(v) * 0.5), mat, area }); }
  });
  return out;
}, [...COMPS]);
await browser.close();
for (const r of rows) if (!r.comp) r.comp = compAt(...r.cell) || "(outside rooms)";

const sel = rows.filter((r) => !only || r.comp === only);
const by = (keyf) => { const m = new Map(); sel.forEach((r) => { const k = keyf(r); m.set(k, (m.get(k) || 0) + r.area); }); return m; };
const total = sel.reduce((s, r) => s + r.area, 0);
const fmt = (a) => a.toFixed(1).padStart(9);
console.log(`${path.basename(file)}${only ? " (" + only + ")" : ""}: ${total.toFixed(0)} m^2 drawn`);
for (const [k, a] of [...by((r) => kindOf(r.layer))].sort((x, y) => y[1] - x[1])) console.log(`  ${k.padEnd(11)} ${fmt(a)} m^2  ${(100 * a / total).toFixed(1)} %`);
console.log("\nPrototype layers (materials.json 0-10), by compartment, role and layer:");
const proto = sel.filter((r) => kindOf(r.layer) === "prototype");
const pk = new Map();
proto.forEach((r) => { const k = `${r.comp}|${r.role}|${PROTO[r.layer % 1000]}`; pk.set(k, (pk.get(k) || 0) + r.area); });
for (const [k, a] of [...pk].sort((x, y) => y[1] - x[1])) { const [c, role, l] = k.split("|"); console.log(`  ${fmt(a)} m^2  ${c.padEnd(28)} ${role.padEnd(22)} ${l}`); }
console.log("\nPrototype layers by compartment:");
const pc = new Map(); proto.forEach((r) => pc.set(r.comp, (pc.get(r.comp) || 0) + r.area));
for (const [c, a] of [...pc].sort((x, y) => y[1] - x[1])) console.log(`  ${fmt(a)} m^2  ${c}`);
console.log("\nUntextured, by compartment and role (flat or vertex colour):");
const uk = new Map();
sel.filter((r) => r.layer === null).forEach((r) => { const k = `${r.comp}|${r.role} ${r.mesh} ${r.mat}`; uk.set(k, (uk.get(k) || 0) + r.area); });
for (const [k, a] of [...uk].sort((x, y) => y[1] - x[1]).slice(0, 40)) { const [c, role] = k.split("|"); console.log(`  ${fmt(a)} m^2  ${c.padEnd(28)} ${role}`); }
if (jsonOut) fs.writeFileSync(jsonOut, JSON.stringify(rows, null, 1));
