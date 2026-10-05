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
 * Usage: node tools/mockups/kit_report.mjs [ship id] [--json]
 * three.js comes from tools/mockups/.cache (filled by shoot.mjs) or the global npm install.
 */
import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const args = process.argv.slice(2);
const json = args.includes("--json");
const ship = args.find((a) => !a.startsWith("--")) || "tern";
const cached = path.join(ROOT, "tools/mockups/.cache/three@0.169.0/build/three.module.js");
if (!fs.existsSync(cached)) { console.error(`missing ${cached}: run node tools/mockups/shoot.mjs once to fill the cache`); process.exit(2); }
const THREE = await import(pathToFileURL(cached).href);

const read = (p) => fs.readFileSync(path.join(ROOT, p), "utf8");
const scripts = {
  "ship-layout": read(`data/ships/${ship}/layout.json`),
  "ship-data-detailing": read(`data/ships/${ship}/detailing.json`),
};
const sandbox = { window: {}, document: { getElementById: (id) => (id in scripts ? { textContent: scripts[id] } : null) }, Math, JSON, Map, Set, Array, Object, Number, Error };
vm.runInNewContext(read("docs/mockups/lib/shipkit.js"), sandbox);
const K = sandbox.window.ShipKit;
const L = K.layout();

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
