#!/usr/bin/env node
/*
 * bake_spacedock.mjs: bake the spacedock mockup's round bay (E) fresh and keep it, so the page opens lit
 * (openspec/changes/spacedock design 8a; light-baking design 16). It opens docs/mockups/spacedock.html headless with
 * ?bake=fresh, waits for the bay's bake (the inside's three colour sets and its light probes), writes them to
 * docs/mockups/cache/spacedock-bake.bin (tools/mockups/bakecache.mjs) and inlines it into the page. Run it after
 * changing anything the bay's bake reads (its geometry or fixtures, data/lighting, lightbake.js); a stale cache only
 * makes the page bake live. Documentation tooling (CLAUDE.md 4): the time it prints is this machine's, not a Pi 5's.
 *
 * Usage: node tools/mockups/bake_spacedock.mjs [--timeout-min 30]
 */
import fs from "node:fs";
import path from "node:path";
import { createRequire } from "node:module";
import { execSync, execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { writeBakeCache } from "./bakecache.mjs";

const require = createRequire(import.meta.url);
function loadPlaywright() {
  try { return require("playwright"); } catch (_) { /* the global install */ }
  return require(path.join(execSync("npm root -g").toString().trim(), "playwright"));
}
const { chromium } = loadPlaywright();
const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", ".."), CACHE = path.join(ROOT, "tools", "mockups", ".cache");
const PAGE = path.join(ROOT, "docs", "mockups", "spacedock.html");
const args = process.argv.slice(2), ti = args.indexOf("--timeout-min"), timeoutMin = ti >= 0 ? Number(args[ti + 1]) : 30;

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
await page.goto("file://" + PAGE + "?bake=fresh");
for (;;) {
  if (errors.length) throw new Error("spacedock.html: " + errors[0]);
  const s = await page.evaluate(() => window.SPACEDOCK_BAKE || null);
  if (s) { console.log(`baked in ${((Date.now() - t0) / 1000).toFixed(0)} s: ${JSON.stringify(s)}`); break; }
  if (Date.now() - t0 > timeoutMin * 60000) throw new Error(`the bay's bake not done after ${timeoutMin} min`);
  await page.waitForTimeout(5000);
}
const e = await page.evaluate(() => window.SPACEDOCK_BAKE_EXPORT());
if (!e) throw new Error("the page has no live bake to export");
const out = writeBakeCache(ROOT, "spacedock-bake", { date: new Date().toISOString().slice(0, 10), bays: { E: e.entry } }, [Buffer.from(e.b64, "base64")]);
await browser.close();
execFileSync("python3", [path.join(ROOT, "tools", "mockups", "inline.py"), PAGE], { stdio: "inherit" });
console.log(`cache: ${path.relative(ROOT, out)} (${(fs.statSync(out).size / 1e6).toFixed(2)} MB)`);
