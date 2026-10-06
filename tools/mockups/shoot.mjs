#!/usr/bin/env node
/*
 * shoot.mjs: screenshot every three.js mockup headless, so a mockup is seen
 * before it is shown (CLAUDE.md section 11).
 *
 * Each page sets window.MOCKUP_READY after its first frame and may register
 * named shots with ShipKit.registerShots([{ name, setup }]). For every page this
 * captures the default view, then calls each shot's setup, lets the scene run
 * for a moment and captures again. Console errors fail the run.
 *
 * Rendering is software (SwiftShader) in a cloud session: the shots show what a
 * mockup looks like, never how fast it runs.
 *
 * CDN requests (three.js from jsDelivr) are served from tools/mockups/.cache, which is filled on
 * first use with curl (it honours the session's proxy); a flaky CDN then cannot fail a run.
 *
 * Usage: node tools/mockups/shoot.mjs [page.html ...] [--out docs/screenshots/mockups]
 *        [--size 1440x900] [--only shotName] [--wait seconds]
 */
import { createRequire } from "node:module";
import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const require = createRequire(import.meta.url);
function loadPlaywright() {
  try { return require("playwright"); } catch (_) { /* fall through to the global install */ }
  const root = execSync("npm root -g").toString().trim();
  return require(path.join(root, "playwright"));
}
const { chromium } = loadPlaywright();

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const args = process.argv.slice(2);
const opt = (name, dflt) => { const i = args.indexOf(name); return i >= 0 ? args.splice(i, 2)[1] : dflt; };
const outDir = path.resolve(ROOT, opt("--out", "docs/screenshots/mockups"));
const [W, H] = opt("--size", "1440x900").split("x").map(Number);
const only = opt("--only", null);
// Seconds to wait for a page to set MOCKUP_READY: a page that bakes several rooms on a shared CPU needs more.
const readyS = Number(opt("--wait", "60"));
let pages = args.length ? args.map((p) => path.resolve(p)) :
  fs.readdirSync(path.join(ROOT, "docs/mockups")).filter((f) => f.endsWith(".html")).map((f) => path.join(ROOT, "docs/mockups", f));

fs.mkdirSync(outDir, { recursive: true });
const launchOpts = { args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"] };
let browser;
try { browser = await chromium.launch(launchOpts); }
catch (_) { browser = await chromium.launch({ ...launchOpts, executablePath: "/opt/pw-browsers/chromium" }); }

const CACHE = path.join(ROOT, "tools", "mockups", ".cache");
async function serveCdnFromCache(page) {
  await page.route("https://cdn.jsdelivr.net/npm/**", async (route) => {
    const url = route.request().url();
    const rel = url.replace("https://cdn.jsdelivr.net/npm/", "").split("?")[0];
    const file = path.join(CACHE, rel);
    if (!fs.existsSync(file)) {
      fs.mkdirSync(path.dirname(file), { recursive: true });
      try { execSync(`curl -sSfL --retry 4 -o ${JSON.stringify(file)} ${JSON.stringify(url)}`); }
      catch (_) { return route.continue(); }
    }
    const type = file.endsWith(".js") ? "text/javascript" : "application/octet-stream";
    return route.fulfill({ status: 200, contentType: type, headers: { "access-control-allow-origin": "*" }, body: fs.readFileSync(file) });
  });
}
const frames = (page, n) => page.evaluate((n) => new Promise((r) => { let i = 0; const f = () => (++i >= n ? r() : requestAnimationFrame(f)); requestAnimationFrame(f); }), n);
let failed = false;
for (const file of pages) {
  const name = path.basename(file, ".html");
  // A cloud session reaches the CDN through a TLS-inspecting proxy whose CA Chromium does not carry.
  const page = await browser.newPage({ viewport: { width: W, height: H }, ignoreHTTPSErrors: true });
  const errors = [];
  await serveCdnFromCache(page);
  page.on("pageerror", (e) => errors.push(String(e)));
  page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
  await page.goto(pathToFileURL(file).href);
  try { await page.waitForFunction(() => window.MOCKUP_READY === true, null, { timeout: readyS * 1000 }); }
  catch (_) { errors.push("MOCKUP_READY was never set"); }
  await frames(page, 20);
  const shots = await page.evaluate(() => (window.MOCKUP_SHOTS || []).map((s) => s.name));
  const taken = [];
  if (!only) {
    const out = path.join(outDir, `${name}.png`);
    await page.screenshot({ path: out });
    taken.push(path.relative(ROOT, out));
  }
  for (let i = 0; i < shots.length; i++) {
    if (only && shots[i] !== only) continue;
    await page.evaluate((i) => Promise.resolve(window.MOCKUP_SHOTS[i].setup()), i);
    await frames(page, 45);
    const out = path.join(outDir, `${name}-${shots[i]}.png`);
    await page.screenshot({ path: out });
    taken.push(path.relative(ROOT, out));
  }
  const budget = await page.evaluate(() => (window.MOCKUP_BUDGET ? window.MOCKUP_BUDGET() : null));
  console.log(`${path.relative(ROOT, file)}: ${taken.length} shots` + (budget ? `, ${budget.triangles} triangles, ${budget.drawCalls} draw calls` : ""));
  for (const t of taken) console.log(`  ${t}`);
  for (const e of errors) { console.log(`  ERROR ${e}`); failed = true; }
  await page.close();
}
await browser.close();
process.exit(failed ? 1 : 0);
