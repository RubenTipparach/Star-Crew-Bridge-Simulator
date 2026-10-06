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
 * With --panels it measures the opt-in panel dressing instead (openspec/changes/wall-panels,
 * ceilings-and-trims, floor-panels): for each compartment the triangles of its flat walls, ceilings,
 * floors and trims today and dressed (data/materials/panels.json), the corridor runner the walkway
 * retires, its bays and cells, and the modules the rules give them.
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
  // Triangles of the dressed surfaces, before any bake subdivision: walls (the tiled "wall" role
  // against the "panel" role), ceilings and floors (one polygon against cells), trims (ribs, beams,
  // coves, baseboards, frames: the same boxes, split for pillars and deep faces), and the corridor
  // runners the walkway retires.
  const TRIM = ["rib", "beam", "cove", "baseboard", "frame", "window_frame"];
  const tri = (parts, roles) => roles.reduce((a, r) => a + (parts[r] ? parts[r].position.length / 9 : 0), 0);
  const out = [], mods = {};
  const tot = { wall0: 0, wall1: 0, ceil0: 0, ceil1: 0, floor0: 0, floor1: 0, trim0: 0, trim1: 0, runner: 0, bays: 0, ccells: 0, fcells: 0, walkway: 0, pillars: 0 };
  for (const c of L.compartments) {
    const a = K.buildCompartment(THREE, L, c, {}), b = K.buildCompartment(THREE, L, c, { panels: true });
    const r = {
      poi: c.poi, id: c.id, finish: c.finish,
      wall0: tri(a.parts, ["wall"]), wall1: tri(b.parts, ["panel"]),
      ceil0: tri(a.parts, ["ceiling"]), ceil1: tri(b.parts, ["ceiling"]),
      floor0: tri(a.parts, ["floor"]), floor1: tri(b.parts, ["floor"]),
      trim0: tri(a.parts, TRIM), trim1: tri(b.parts, TRIM), runner: tri(a.parts, ["runner"]) - tri(b.parts, ["runner"]),
      bays: b.panels.bays, ccells: b.panels.ceiling.cells, fcells: b.panels.floor.cells, walkway: b.panels.floor.walkway, pillars: b.panels.trims.pillars,
    };
    for (const [kind, st] of [["walls", { modules: b.panels.modules }], ["ceiling", b.panels.ceiling], ["floor", b.panels.floor]]) {
      const m = mods[`${c.finish} ${kind}`] || (mods[`${c.finish} ${kind}`] = {});
      for (const [k, n] of Object.entries(st.modules)) m[k] = (m[k] || 0) + n;
    }
    for (const k of Object.keys(tot)) tot[k] += r[k];
    out.push(r);
  }
  out.sort((a, b) => a.poi - b.poi);
  if (json) { console.log(JSON.stringify({ ship, rows: out, modules: mods, total: tot }, null, 1)); process.exit(0); }
  const pad = (s, n) => String(s).padStart(n);
  const pair = (x, y) => pad(`${x}>${y}`, 10);
  console.log(`${L.ship.name}: triangles today > dressed (walls, ceilings, floors, trims), before bake subdivision`);
  console.log(`  poi  ${"compartment".padEnd(16)} ${"finish".padEnd(8)} ${pad("walls", 10)} ${pad("ceiling", 10)} ${pad("floor", 10)} ${pad("trims", 10)} ${pad("runner", 7)} ${pad("added", 6)} ${pad("bays", 5)} ${pad("cells c/f", 10)} ${pad("walk", 5)}`);
  const added = (r) => r.wall1 - r.wall0 + r.ceil1 - r.ceil0 + r.floor1 - r.floor0 + r.trim1 - r.trim0 - r.runner;
  for (const r of out) console.log(`  ${pad(r.poi, 3)}  ${r.id.padEnd(16)} ${r.finish.padEnd(8)} ${pair(r.wall0, r.wall1)} ${pair(r.ceil0, r.ceil1)} ${pair(r.floor0, r.floor1)} ${pair(r.trim0, r.trim1)} ${pad(-r.runner, 7)} ${pad(added(r), 6)} ${pad(r.bays, 5)} ${pad(`${r.ccells}/${r.fcells}`, 10)} ${pad(r.walkway, 5)}`);
  console.log(`       ${"total".padEnd(16)} ${"".padEnd(8)} ${pair(tot.wall0, tot.wall1)} ${pair(tot.ceil0, tot.ceil1)} ${pair(tot.floor0, tot.floor1)} ${pair(tot.trim0, tot.trim1)} ${pad(-tot.runner, 7)} ${pad(added(tot), 6)} ${pad(tot.bays, 5)} ${pad(`${tot.ccells}/${tot.fcells}`, 10)} ${pad(tot.walkway, 5)}`);
  console.log(`  bays are counted per module band (a tall wall's second band counts again); cells per ceiling and floor; ${tot.pillars} ribs carry a base and a capital`);
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
