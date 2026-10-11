#!/usr/bin/env node
/*
 * export_deck.mjs: the ship as the deck plan builds it, for the engine's deck compiler (openspec/changes/deck-pipeline,
 * design section 13: the first deck in the engine).
 *
 * It opens docs/mockups/deck-plan.html headless, waits until every compartment is lit (from the bake cache, or baked
 * live where the cache does not match), calls the page's window.MOCKUP_EXPORT_DECK() and writes build/deck/<ship>/:
 * export.json (the index) and one .bin file per array, the walk's collision triangles and entities among them (13a), and
 * since 13b the console faces and the screens' atlas, the lift's car as a moving room, and the viewscreens. `cargo run -p sc-tools -- deckc` packs them into
 * compiled/<ship>.deck with sc-core's vertex packer. The geometry is the kit's (docs/mockups/lib/shipkit.js), the one
 * implementation of the deck rules until deckc grows its own (task 2.x); this tool only carries it across.
 *
 * Light probes (light-baking design 16): each room's ambient cubes come from the probe cache,
 * tools/deck/cache/deck-plan-probes.bin, when its entry's key matches the room's; any other room's are baked in the
 * page (its bake scene prepared again, then probeCube at each point), and --write-probe-cache writes the cache back.
 * The cache file: gzip of "SCPR", a version (u32), the index's length (u32), the index as JSON ({ version, rooms: { id:
 * { key, origin_m, spacing_m, dims, offset, count } } }), then each room's cubes (54 bytes a probe) and valid flags (a
 * byte a probe) at its offset.
 *
 * Usage: node tools/deck/export_deck.mjs [--timeout-min 30] [--write-probe-cache]
 */
import { createRequire } from "node:module";
import { execSync } from "node:child_process";
import fs from "node:fs";
import zlib from "node:zlib";
import path from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
function loadPlaywright() {
  try { return require("playwright"); } catch (_) { /* the global install */ }
  return require(path.join(execSync("npm root -g").toString().trim(), "playwright"));
}
const { chromium } = loadPlaywright();
const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const CACHE = path.join(ROOT, "tools", "mockups", ".cache");
const args = process.argv.slice(2);
const ti = args.indexOf("--timeout-min"), timeoutMin = ti >= 0 ? Number(args[ti + 1]) : 30;
const writeProbeCache = args.includes("--write-probe-cache");
const PROBE_CACHE = path.join(ROOT, "tools", "deck", "cache", "deck-plan-probes.bin");

/** The probe cache's rooms: id -> { key, origin_m, spacing_m, dims, cubes: Buffer, valid: Buffer }, or {} with none. */
function readProbeCache() {
  if (!fs.existsSync(PROBE_CACHE)) return {};
  const raw = zlib.gunzipSync(fs.readFileSync(PROBE_CACHE));
  if (raw.subarray(0, 4).toString("latin1") !== "SCPR" || raw.readUInt32LE(4) !== 1) throw new Error(`${PROBE_CACHE}: not a version 1 probe cache`);
  const n = raw.readUInt32LE(8), index = JSON.parse(raw.subarray(12, 12 + n).toString("utf8")), data = raw.subarray(12 + n), out = {};
  for (const [id, e] of Object.entries(index.rooms)) {
    out[id] = { key: e.key, origin_m: e.origin_m, spacing_m: e.spacing_m, dims: e.dims,
      cubes: data.subarray(e.offset, e.offset + e.count * 54), valid: data.subarray(e.offset + e.count * 54, e.offset + e.count * 55) };
  }
  return out;
}
/** Write the probe cache from rooms (id -> entry as readProbeCache gives), in sorted id order (stable bytes). */
function writeProbeCacheFile(rooms) {
  const index = { version: 1, rooms: {} }, parts = [];
  let offset = 0;
  for (const id of Object.keys(rooms).sort()) {
    const e = rooms[id], count = e.valid.length;
    index.rooms[id] = { key: e.key, origin_m: e.origin_m, spacing_m: e.spacing_m, dims: e.dims, offset, count };
    parts.push(e.cubes, e.valid);
    offset += count * 55;
  }
  const json = Buffer.from(JSON.stringify(index)), head = Buffer.alloc(12);
  head.write("SCPR", 0, "latin1"); head.writeUInt32LE(1, 4); head.writeUInt32LE(json.length, 8);
  fs.mkdirSync(path.dirname(PROBE_CACHE), { recursive: true });
  fs.writeFileSync(PROBE_CACHE, zlib.gzipSync(Buffer.concat([head, json, ...parts]), { level: 9 }));
}

const browser = await chromium.launch({ executablePath: "/opt/pw-browsers/chromium", args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"] })
  .catch(() => chromium.launch({ args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"] }));
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
const errors = [];
page.on("pageerror", (e) => errors.push(e.stack || e.message));
await page.route("https://cdn.jsdelivr.net/npm/**", (route) => {
  const rel = route.request().url().replace("https://cdn.jsdelivr.net/npm/", "").split("?")[0];
  const local = path.join(CACHE, rel);
  if (fs.existsSync(local)) route.fulfill({ body: fs.readFileSync(local), contentType: "application/javascript" });
  else route.continue();
});
const t0 = Date.now();
await page.goto("file://" + path.join(ROOT, "docs", "mockups", "deck-plan.html"));
for (;;) {
  if (errors.length) throw new Error("deck-plan.html: " + errors[0]);
  const s = await page.evaluate(() => window.MOCKUP_BAKE && window.MOCKUP_BAKE());
  if (s && s.done) { console.log(`lit: ${s.total} rooms after ${((Date.now() - t0) / 1000).toFixed(0)} s`); break; }
  if (Date.now() - t0 > timeoutMin * 60000) throw new Error(`not lit after ${timeoutMin} min`);
  await page.waitForTimeout(1000);
}
const ex = await page.evaluate(() => window.MOCKUP_EXPORT_DECK());
// The probes: from the cache where a room's key matches, else baked in the page now.
const probeCache = readProbeCache(), probeKeys = await page.evaluate(() => window.MOCKUP_PROBE_KEYS()), probes = {};
let probesBaked = 0;
for (const [id, key] of Object.entries(probeKeys)) {
  const hit = probeCache[id];
  if (hit && hit.key === key) { probes[id] = hit; continue; }
  const t1 = Date.now(), o = await page.evaluate((rid) => window.MOCKUP_PROBE_ROOM(rid), id);
  probes[id] = { key: o.key, origin_m: o.origin_m, spacing_m: o.spacing_m, dims: o.dims,
    cubes: Buffer.from(o.cubes, "base64"), valid: Buffer.from(o.valid, "base64") };
  probesBaked++;
  console.log(`probes: ${id}: ${o.dims.join(" x ")}, ${o.invalid} invalid, ${((Date.now() - t1) / 1000).toFixed(1)} s`);
}
await browser.close();
const probeCount = Object.values(probes).reduce((n, e) => n + e.valid.length, 0);
console.log(`probes: ${probeCount} in ${Object.keys(probes).length} rooms, ${probesBaked} rooms baked here, the rest from the cache`);
if (writeProbeCache) { writeProbeCacheFile(probes); console.log(`probes: wrote ${path.relative(ROOT, PROBE_CACHE)}`); }
else if (probesBaked) console.log("probes: the cache is stale; rerun with --write-probe-cache to keep these");

const dir = path.join(ROOT, "build", "deck", ex.ship);
fs.rmSync(dir, { recursive: true, force: true });
fs.mkdirSync(dir, { recursive: true });
const put = (name, b64) => { fs.writeFileSync(path.join(dir, name), Buffer.from(b64, "base64")); return name; };
const index = { schema: "starcrew.deck-export/1", ship: ex.ship, panel_first: ex.panel_first, panel_glow: ex.panel_glow,
  textures: { main: { size_px: ex.textures.main.size_px, layers: ex.textures.main.layers, rgba: put("tex-main.bin", ex.textures.main.rgba) },
    big: ex.textures.big.map((b, k) => ({ base: b.base, size_px: b.size_px, layers: b.layers, rgba: put(`tex-big${k}.bin`, b.rgba) })),
    screens: ex.textures.screens ? { width_px: ex.textures.screens.width_px, height_px: ex.textures.screens.height_px,
      rgba: put("tex-screens.bin", ex.textures.screens.rgba) } : null },
  rooms: ex.rooms.map((r) => {
    const f = {};
    for (const k of ["position", "normal", "uv", "layer", "c0", "c1", "c2"]) f[k] = put(`${r.id}.${k}.bin`, r[k]);
    const room = { id: r.id, name: r.name, deck: r.deck, mover: r.mover || 0, vertices: r.vertices, files: f, faces: null, probes: null };
    const pr = probes[r.id];
    if (pr) {
      fs.writeFileSync(path.join(dir, `${r.id}.probes.bin`), pr.cubes);
      fs.writeFileSync(path.join(dir, `${r.id}.probes_valid.bin`), pr.valid);
      room.probes = { origin_m: pr.origin_m, spacing_m: pr.spacing_m, dims: pr.dims, cubes: `${r.id}.probes.bin`, valid: `${r.id}.probes_valid.bin` };
    }
    if (r.faces) {
      const ff = {};
      for (const k of ["position", "normal", "uv", "c0", "c1", "c2"]) ff[k] = put(`${r.id}.faces.${k}.bin`, r.faces[k]);
      room.faces = { vertices: r.faces.vertices, files: ff };
    }
    return room;
  }),
  views: ex.views || [] };
// The walk world and its entities (deck-pipeline 13a): the triangles in their own file, the rest in the index.
const { triangles, ...walk } = ex.walk;
index.walk = Object.assign({ triangles: put("walk.triangles.bin", triangles) }, walk);
fs.writeFileSync(path.join(dir, "export.json"), JSON.stringify(index, null, 1) + "\n");
const mb = fs.readdirSync(dir).reduce((s, f) => s + fs.statSync(path.join(dir, f)).size, 0) / 1e6;
console.log(`export: ${path.relative(ROOT, dir)} (${index.rooms.length} rooms, ${index.rooms.reduce((s, r) => s + r.vertices, 0)} vertices, ${mb.toFixed(1)} MB)`);
