#!/usr/bin/env node
/*
 * kit_report.mjs: what the deck kit generates for a ship, counted per compartment and per role.
 *
 * A measurement instrument (CLAUDE.md section 4) for openspec/changes/deck-pipeline sections 5a
 * and 11: it runs docs/mockups/lib/shipkit.js (the mockups' implementation of the detail rules
 * in data/ships/<id>/detailing.json) on the ship's layout outside a browser, and prints the
 * triangles of each compartment's shell and generated detail, by role, and its lamps. Props
 * (stations, systems, fixtures, craft) are drawn by the mockups and are not counted here. It
 * changes nothing. When deckc exists, its --report replaces this.
 *
 * With --panels it measures the opt-in wall dressing instead (openspec/changes/wall-panels): for
 * each compartment the triangles of its flat walls as one tiled surface and as panelled bays and
 * bands (data/materials/panels.json), its bays, and the modules the rule gives them.
 *
 * Usage: node tools/mockups/kit_report.mjs [ship id] [--json] [--panels]
 * three.js comes from tools/mockups/.cache (filled by shoot.mjs) or the global npm install.
 */
import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const args = process.argv.slice(2);
const json = args.includes("--json");
const panels = args.includes("--panels");
const ship = args.find((a) => !a.startsWith("--")) || "tern";
const cached = path.join(ROOT, "tools/mockups/.cache/three@0.169.0/build/three.module.js");
if (!fs.existsSync(cached)) { console.error(`missing ${cached}: run node tools/mockups/shoot.mjs once to fill the cache`); process.exit(2); }
const THREE = await import(pathToFileURL(cached).href);

const read = (p) => fs.readFileSync(path.join(ROOT, p), "utf8");
const scripts = {
  "ship-layout": read(`data/ships/${ship}/layout.json`),
  "ship-data-detailing": read(`data/ships/${ship}/detailing.json`),
};
if (panels) scripts["ship-panels"] = JSON.stringify({ manifest: JSON.parse(read("data/materials/panels.json")) });
const sandbox = { window: {}, document: { getElementById: (id) => (id in scripts ? { textContent: scripts[id] } : null) }, Math, JSON, Map, Set, Array, Object, Number, Error };
vm.runInNewContext(read("docs/mockups/lib/shipkit.js"), sandbox);
const K = sandbox.window.ShipKit;
const L = K.layout();

if (panels) {
  // Wall triangles: the tiled "wall" role today against the "panel" role, before any bake subdivision.
  const out = [], mods = {};
  let w0 = 0, w1 = 0, bays = 0;
  for (const c of L.compartments) {
    const a = K.buildCompartment(THREE, L, c, {}), b = K.buildCompartment(THREE, L, c, { panels: true });
    const t0 = (a.parts.wall ? a.parts.wall.position.length : 0) / 9, t1 = (b.parts.panel ? b.parts.panel.position.length : 0) / 9;
    const m = mods[c.finish] || (mods[c.finish] = {});
    for (const [k, n] of Object.entries(b.panels.modules)) m[k] = (m[k] || 0) + n;
    out.push({ poi: c.poi, id: c.id, finish: c.finish, wall: t0, panel: t1, bays: b.panels.bays });
    w0 += t0; w1 += t1; bays += b.panels.bays;
  }
  out.sort((a, b) => a.poi - b.poi);
  if (json) { console.log(JSON.stringify({ ship, rows: out, modules: mods, total: { wall: w0, panel: w1, bays } }, null, 1)); process.exit(0); }
  const pad = (s, n) => String(s).padStart(n);
  console.log(`${L.ship.name}: flat-wall triangles, tiled today and dressed with panels (before bake subdivision)`);
  console.log(`  poi  ${"compartment".padEnd(16)} ${"finish".padEnd(8)} ${pad("today", 6)} ${pad("panels", 7)} ${pad("added", 6)} ${pad("bays", 5)}`);
  for (const r of out) console.log(`  ${pad(r.poi, 3)}  ${r.id.padEnd(16)} ${r.finish.padEnd(8)} ${pad(r.wall, 6)} ${pad(r.panel, 7)} ${pad(r.panel - r.wall, 6)} ${pad(r.bays, 5)}`);
  console.log(`       ${"total".padEnd(16)} ${"".padEnd(8)} ${pad(w0, 6)} ${pad(w1, 7)} ${pad(w1 - w0, 6)} ${pad(bays, 5)}`);
  console.log("  bays are counted per module band (a tall wall's second band counts again)");
  for (const [f, m] of Object.entries(mods)) console.log(`  ${f}: ` + Object.entries(m).sort((a, b) => b[1] - a[1]).map(([k, n]) => `${k} ${n}`).join(", "));
  process.exit(0);
}

const rows = [];
const total = {};
for (const c of L.compartments) {
  const b = K.buildCompartment(THREE, L, c, {});
  const roles = {};
  let tris = 0;
  for (const [r, p] of Object.entries(b.parts)) {
    const t = p.position.length / 9;
    roles[r] = t; tris += t; total[r] = (total[r] || 0) + t;
  }
  const shell = ["floor", "wall", "ceiling", "cove"].reduce((a, r) => a + (roles[r] || 0), 0);
  rows.push({ poi: c.poi, id: c.id, shell, detail: tris - shell, triangles: tris, lamps: b.lamps.length, emergency: b.lamps.filter((l) => l.emergency).length, roles });
}
rows.sort((a, b) => a.poi - b.poi);
if (json) { console.log(JSON.stringify({ ship, rows, total }, null, 1)); process.exit(0); }
const pad = (s, n) => String(s).padStart(n);
console.log(`${L.ship.name}: shell and generated detail by compartment (props not counted)`);
console.log(`  poi  ${"compartment".padEnd(16)} ${pad("shell", 6)} ${pad("detail", 7)} ${pad("total", 7)} ${pad("lamps", 6)} ${pad("emerg", 6)}`);
let s = 0, d = 0, l = 0, e = 0;
for (const r of rows) {
  console.log(`  ${pad(r.poi, 3)}  ${r.id.padEnd(16)} ${pad(r.shell, 6)} ${pad(r.detail, 7)} ${pad(r.triangles, 7)} ${pad(r.lamps, 6)} ${pad(r.emergency, 6)}`);
  s += r.shell; d += r.detail; l += r.lamps; e += r.emergency;
}
console.log(`       ${"total".padEnd(16)} ${pad(s, 6)} ${pad(d, 7)} ${pad(s + d, 7)} ${pad(l, 6)} ${pad(e, 6)}`);
console.log("  by role: " + Object.entries(total).sort((a, b) => b[1] - a[1]).map(([r, t]) => `${r} ${t}`).join(", "));
