#!/usr/bin/env node
/*
 * export_deck.mjs: the ship as the deck plan builds it, for the engine's deck compiler (openspec/changes/deck-pipeline,
 * design section 13: the first deck in the engine).
 *
 * It opens docs/mockups/deck-plan.html headless, waits until every compartment is lit (from the bake cache, or baked
 * live where the cache does not match), calls the page's window.MOCKUP_EXPORT_DECK() and writes build/deck/<ship>/:
 * export.json (the index) and one .bin file per array. `cargo run -p sc-tools -- deckc` packs them into
 * compiled/<ship>.deck with sc-core's vertex packer. The geometry is the kit's (docs/mockups/lib/shipkit.js), the one
 * implementation of the deck rules until deckc grows its own (task 2.x); this tool only carries it across.
 *
 * Usage: node tools/deck/export_deck.mjs [--timeout-min 30]
 */
import { createRequire } from "node:module";
import { execSync } from "node:child_process";
import fs from "node:fs";
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
await browser.close();

const dir = path.join(ROOT, "build", "deck", ex.ship);
fs.rmSync(dir, { recursive: true, force: true });
fs.mkdirSync(dir, { recursive: true });
const put = (name, b64) => { fs.writeFileSync(path.join(dir, name), Buffer.from(b64, "base64")); return name; };
const index = { schema: "starcrew.deck-export/1", ship: ex.ship, panel_first: ex.panel_first, panel_glow: ex.panel_glow,
  textures: { main: { size_px: ex.textures.main.size_px, layers: ex.textures.main.layers, rgba: put("tex-main.bin", ex.textures.main.rgba) },
    big: ex.textures.big.map((b, k) => ({ base: b.base, size_px: b.size_px, layers: b.layers, rgba: put(`tex-big${k}.bin`, b.rgba) })) },
  rooms: ex.rooms.map((r) => {
    const f = {};
    for (const k of ["position", "normal", "uv", "layer", "c0", "c1", "c2"]) f[k] = put(`${r.id}.${k}.bin`, r[k]);
    return { id: r.id, name: r.name, deck: r.deck, vertices: r.vertices, files: f };
  }) };
fs.writeFileSync(path.join(dir, "export.json"), JSON.stringify(index, null, 1) + "\n");
const mb = fs.readdirSync(dir).reduce((s, f) => s + fs.statSync(path.join(dir, f)).size, 0) / 1e6;
console.log(`export: ${path.relative(ROOT, dir)} (${index.rooms.length} rooms, ${index.rooms.reduce((s, r) => s + r.vertices, 0)} vertices, ${mb.toFixed(1)} MB)`);
