#!/usr/bin/env node
/*
 * parity.mjs: the console mockup's side of the console parity check (openspec/changes/console-parity, design 4).
 *
 * For each named shot of docs/mockups/consoles.html this sets the shot up (its clock frozen), captures the 1280 x 720
 * console at one layout point per pixel, and saves window.consoleState() beside it. The engine then draws the same
 * state (sc-client --console-fixture <json> --shots <dir>) and tools/consoles/compare.py lays the two side by side.
 *
 * Usage: node tools/consoles/parity.mjs [--out docs/screenshots/parity] [--only helm,tactical]
 */
import { createRequire } from "node:module";
import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const require = createRequire(import.meta.url);
function loadPlaywright() {
  try { return require("playwright"); } catch (_) { /* the global install */ }
  return require(path.join(execSync("npm root -g").toString().trim(), "playwright"));
}
const { chromium } = loadPlaywright();
const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const args = process.argv.slice(2);
const opt = (name, dflt) => { const i = args.indexOf(name); return i >= 0 ? args.splice(i, 2)[1] : dflt; };
const out = path.resolve(ROOT, opt("--out", "docs/screenshots/parity"));
// Every named shot of every console, at rest, in use and in combat.
const SHOTS = ["helm", "helm-stick", "helm-orient", "tactical", "engineering", "engineering-cooling", "engineering-cooling-breaker",
  "science", "captain", "red-helm-evading", "red-tactical", "red-science-target-feed", "red-science-shields", "red-engineering",
  "red-captain", "red-captain-ship", "scram-engineering", "red-tactical-turned"];
const only = opt("--only", null);
const shots = only ? SHOTS.filter((s) => only.split(",").includes(s)) : SHOTS;

fs.mkdirSync(out, { recursive: true });
let browser;
try { browser = await chromium.launch(); }
catch (_) { browser = await chromium.launch({ executablePath: "/opt/pw-browsers/chromium" }); }
// The page's bar is 44 px; below it the stage fits the 1280 x 720 console at a scale of exactly 1.
const page = await browser.newPage({ viewport: { width: 1280, height: 764 }, deviceScaleFactor: 1 });
const errors = [];
page.on("pageerror", (e) => errors.push(String(e)));
page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
await page.goto(pathToFileURL(path.join(ROOT, "docs/mockups/consoles.html")).href);
await page.waitForFunction(() => window.MOCKUP_READY === true, null, { timeout: 60000 });
for (const name of shots) {
  await page.evaluate((n) => window.MOCKUP_SHOTS.find((s) => s.name === n).setup(), name);
  await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
  const box = await page.evaluate(() => { const r = document.getElementById("console").getBoundingClientRect(); return { x: r.left, y: r.top, width: r.width, height: r.height }; });
  if (Math.abs(box.width - 1280) > 0.5 || Math.abs(box.height - 720) > 0.5) throw new Error(`console is ${box.width} x ${box.height}, not 1280 x 720`);
  await page.screenshot({ path: path.join(out, `${name}-mockup.png`), clip: box });
  const state = await page.evaluate(() => window.consoleState());
  fs.writeFileSync(path.join(out, `${name}.json`), JSON.stringify(state, null, 1) + "\n");
  console.log(`${name}: ${path.relative(ROOT, path.join(out, `${name}-mockup.png`))} and its state`);
}
await browser.close();
if (errors.length) { console.error(errors.join("\n")); process.exit(1); }
