#!/usr/bin/env node
/*
 * export_bays.mjs: the spacedock bays as the mockup bakes them, for the engine (openspec/changes/spacedock design 8b).
 *
 * It opens docs/mockups/spacedock.html headless, waits for the named versions' bakes (from the page's bake cache,
 * docs/mockups/cache/spacedock-bake.bin, or baked live where it does not match), calls the page's
 * window.SPACEDOCK_EXPORT_BAYS(versions) and writes build/stations/: export.json (the index) and one .bin file per
 * array: each version's baked inside and its fixtures' faces (positions in the station's frame, normals, texture
 * coordinates, layers, the three states' colours as linear multipliers), the inside's indices, its probe grids, and the
 * texture layers they sample. `cargo run -p sc-tools -- bayc` packs them into compiled/stations.bays. The geometry and
 * the light are the mockup's own (the one implementation until the station generator bakes its own, tasks 2.3); this
 * tool only carries them across, as tools/deck/export_deck.mjs does for the decks.
 *
 * Usage: node tools/stations/export_bays.mjs [--versions E] [--timeout-min 30]
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
const opt = (name, dflt) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : dflt; };
const versions = opt("--versions", "E").split(",").map((v) => v.trim()).filter(Boolean);
const timeoutMin = Number(opt("--timeout-min", "30"));
if (!versions.every((v) => /^[A-E]$/.test(v))) throw new Error(`--versions: ${versions.join(",")}: each is one of A-E`);

const launch = { args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"] };
const browser = await chromium.launch({ ...launch, executablePath: "/opt/pw-browsers/chromium" }).catch(() => chromium.launch(launch));
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
const errors = [];
page.on("pageerror", (e) => errors.push(e.stack || e.message));
// three.js from tools/mockups/.cache, filled on first use (as shoot.mjs).
await page.route("https://cdn.jsdelivr.net/npm/**", async (route) => {
  const url = route.request().url(), file = path.join(CACHE, url.replace("https://cdn.jsdelivr.net/npm/", "").split("?")[0]);
  if (!fs.existsSync(file)) {
    fs.mkdirSync(path.dirname(file), { recursive: true });
    try { execSync(`curl -sSfL --retry 4 -o ${JSON.stringify(file)} ${JSON.stringify(url)}`); } catch (_) { return route.continue(); }
  }
  return route.fulfill({ status: 200, contentType: "text/javascript", body: fs.readFileSync(file) });
});
const t0 = Date.now();
await page.goto("file://" + path.join(ROOT, "docs", "mockups", "spacedock.html"));
for (;;) {
  if (errors.length) throw new Error("spacedock.html: " + errors[0]);
  const s = await page.evaluate((vs) => vs.map((v) => (window.SPACEDOCK_BAKE || {})[v] || null), versions);
  if (s.every(Boolean)) {
    for (const [i, b] of s.entries()) console.log(`${versions[i]}: ${b.vertices} vertices, ${b.probes} probes, ${b.cached ? "from the cache" : "baked live"}`);
    break;
  }
  if (Date.now() - t0 > timeoutMin * 60000) throw new Error(`the bays not baked after ${timeoutMin} min`);
  await page.waitForTimeout(1000);
}
const ex = await page.evaluate((vs) => window.SPACEDOCK_EXPORT_BAYS(vs), versions);
await browser.close();

const dir = path.join(ROOT, "build", "stations");
fs.rmSync(dir, { recursive: true, force: true });
fs.mkdirSync(dir, { recursive: true });
const put = (name, b64) => { fs.writeFileSync(path.join(dir, name), Buffer.from(b64, "base64")); return name; };
const mesh = (prefix, m, keys) => {
  const files = {};
  for (const k of keys) files[k] = put(`${prefix}.${k}.bin`, m[k]);
  return { vertices: m.vertices, indices: m.indices || 0, files };
};
const index = {
  schema: "starcrew.bays-export/1",
  textures: { size_px: ex.size_px, layers: ex.layers, rgba: put("tex.bin", ex.rgba) },
  bays: ex.bays.map((b) => ({
    version: b.version,
    digest: b.digest,
    inside: mesh(`${b.version}.inside`, b.inside, ["position", "normal", "uv", "layer", "index", "c0", "c1", "c2"]),
    glow: mesh(`${b.version}.glow`, b.glow, ["position", "normal", "uv", "layer", "c0", "c1", "c2"]),
    grids: b.grids.map((g, k) => ({ origin_m: g.origin_m, spacing_m: g.spacing_m, dims: g.dims,
      cubes: put(`${b.version}.grid${k}.cubes.bin`, g.cubes), valid: put(`${b.version}.grid${k}.valid.bin`, g.valid) })),
  })),
};
fs.writeFileSync(path.join(dir, "export.json"), JSON.stringify(index, null, 1) + "\n");
const mb = fs.readdirSync(dir).reduce((s, f) => s + fs.statSync(path.join(dir, f)).size, 0) / 1e6;
console.log(`export: ${path.relative(ROOT, dir)} (${index.bays.map((b) => b.version).join(", ")}; ${ex.layers} texture layers; ${mb.toFixed(1)} MB)`);
