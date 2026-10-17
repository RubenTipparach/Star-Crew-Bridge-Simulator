#!/usr/bin/env node
/*
 * bake_pages.mjs: bake the rooms of mockup pages fresh and keep them, so each page opens lit (lib/roombake.js;
 * light-baking design 15). For every page named it opens it headless with ?bake=rooms (its rooms fresh, any other bake from its cache), waits for its rooms' bake
 * (window.MOCKUP_BAKING), runs each of its shots and waits for the rooms each one builds, writes the light to docs/mockups/cache/<name>-bake.bin (tools/mockups/bakecache.mjs; the name
 * is the page's RoomBake.page name, the page's own unless it keeps another cache already) and inlines it into the page
 * (its <!-- INLINE bakecache:<name>-bake --> markers). Run it after changing anything a
 * page's rooms bake from (their geometry, props or lamps, data/lighting, lightbake.js or roombake.js); a stale cache
 * only makes the page bake live. The deck plan keeps its own tool (bake_ship.mjs), the spacedock its own
 * (bake_spacedock.mjs). Documentation tooling (CLAUDE.md 4): times are this machine's.
 *
 * Usage: node tools/mockups/bake_pages.mjs docs/mockups/bridge.html [more pages] [--timeout-min 60]
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
const args = process.argv.slice(2), ti = args.indexOf("--timeout-min"), timeoutMin = ti >= 0 ? Number(args.splice(ti, 2)[1]) : 60;
const pages = args.map((a) => path.resolve(a));
if (!pages.length) { console.error("usage: node tools/mockups/bake_pages.mjs docs/mockups/<page>.html [...]"); process.exit(2); }

const launch = { args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"] };
const browser = await chromium.launch({ ...launch, executablePath: "/opt/pw-browsers/chromium" }).catch(() => chromium.launch(launch));
let failed = false;
for (const file of pages) {
  const name = path.basename(file, ".html"), t0 = Date.now();
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.stack || e.message));
  // three.js from tools/mockups/.cache, filled on first use (as shoot.mjs).
  await page.route("https://cdn.jsdelivr.net/npm/**", async (route) => {
    const url = route.request().url(), f = path.join(CACHE, url.replace("https://cdn.jsdelivr.net/npm/", "").split("?")[0]);
    if (!fs.existsSync(f)) {
      fs.mkdirSync(path.dirname(f), { recursive: true });
      try { execSync(`curl -sSfL --retry 4 -o ${JSON.stringify(f)} ${JSON.stringify(url)}`); } catch (_) { return route.continue(); }
    }
    return route.fulfill({ status: 200, contentType: "text/javascript", body: fs.readFileSync(f) });
  });
  try {
    await page.goto("file://" + file + "?bake=rooms");
    await page.waitForFunction(() => window.MOCKUP_READY === true, null, { timeout: 300000 });
    if (!(await page.evaluate(() => !!window.MOCKUP_ROOMBAKE))) throw new Error("the page bakes no rooms (no lib:roombake page)");
    // The rooms the page opens with, then every room a shot shows (a page that builds rooms on demand, the concept ships).
    const all = page.evaluate(async () => {
      await window.MOCKUP_BAKING();
      const shots = window.MOCKUP_SHOTS || [];
      for (const sh of shots) { await Promise.resolve(sh.setup()); await new Promise((r) => setTimeout(r, 300)); await window.MOCKUP_BAKING(); }
      return shots.length;
    });
    const n = await Promise.race([all, new Promise((_, no) => setTimeout(() => no(new Error(`not baked after ${timeoutMin} min`)), timeoutMin * 60000))]);
    console.log(`${path.relative(ROOT, file)}: the opening view and ${n} shots baked`);
    if (errors.length) throw new Error(errors[0]);
    const e = await page.evaluate(() => window.MOCKUP_ROOMBAKE.export()), cacheName = await page.evaluate(() => window.MOCKUP_ROOMBAKE.name);
    const out = writeBakeCache(ROOT, cacheName + "-bake", { date: new Date().toISOString().slice(0, 10), rooms: e.rooms }, [Buffer.from(e.b64, "base64")]);
    console.log(`${path.relative(ROOT, file)}: ${Object.keys(e.rooms).length} rooms baked in ${((Date.now() - t0) / 1000).toFixed(0)} s; ` +
      `${path.relative(ROOT, out)} ${(fs.statSync(out).size / 1e6).toFixed(2)} MB`);
    // inline.py also reports every other page's caches; its exit status is not this page's.
    try { execFileSync("python3", [path.join(ROOT, "tools", "mockups", "inline.py"), file], { stdio: "ignore" }); } catch (_) { /* reported above */ }
  } catch (err) {
    failed = true;
    console.error(`${path.relative(ROOT, file)}: FAIL, ${err.message}`);
  }
  await page.close();
}
await browser.close();
process.exit(failed ? 1 : 0);
