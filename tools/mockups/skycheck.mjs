#!/usr/bin/env node
/*
 * skycheck.mjs: no star draws over a solid in a three.js mockup, headless (openspec/changes/sky-behind-solids 4).
 *
 * A page with a sky lists its sky layer's groups in window.MOCKUP_SKY (stars, glows, planets, star streaks). In the
 * default view and after every named shot's setup it checks the engine's rule, as three.js will draw the frame:
 * every drawable in the sky layer writes no depth and sits in the opaque list (three.js draws its transparent list
 * after every opaque object), and its sort key (the nearest group's renderOrder, then its own) comes before every
 * solid's in the same scene, so the sky is drawn first and every solid covers it, however far the camera stands.
 *
 * A check (CLAUDE.md section 4): it changes nothing; only the scene graph is read.
 *
 * Usage: node tools/mockups/skycheck.mjs [page.html ...]
 *        (default: the pages with a sky: patrol.html, spacedock.html, ship-exteriors.html). Exits 1 on a breach.
 */
import { createRequire } from "node:module";
import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
function loadPlaywright() {
  try { return require("playwright"); } catch (_) { /* fall through to the global install */ }
  return require(path.join(execSync("npm root -g").toString().trim(), "playwright"));
}
const { chromium } = loadPlaywright();
const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const CACHE = path.join(ROOT, "tools", "mockups", ".cache");
const args = process.argv.slice(2);
const pages = (args.length ? args : ["patrol.html", "spacedock.html", "ship-exteriors.html"].map((p) => `docs/mockups/${p}`))
  .map((p) => path.resolve(ROOT, p));

/** In the page: the breaches of the sky rule in the scenes the sky layer is in, as strings. */
function breaches() {
  const sky = window.MOCKUP_SKY || [];
  const inSky = new Set();
  for (const g of sky) g.traverse((o) => inSky.add(o));
  const roots = new Set(sky.map((g) => { let r = g; while (r.parent) r = r.parent; return r; }));
  const key = (o) => {
    let g = o, group = 0;
    while (g) { if (g.isGroup) { group = g.renderOrder; break; } g = g.parent; }
    return [group, o.renderOrder];
  };
  const before = (a, b) => a[0] < b[0] || (a[0] === b[0] && a[1] < b[1]);
  const seen = (o) => { for (let p = o; p; p = p.parent) if (!p.visible) return false; return true; };
  const out = [];
  for (const root of roots) {
    const skyItems = [], solids = [];
    root.traverse((o) => {
      if (!o.material || !seen(o)) return;
      const mats = Array.isArray(o.material) ? o.material : [o.material];
      if (inSky.has(o)) {
        for (const m of mats) {
          if (m.depthWrite) out.push(`${o.type} ${o.name || ""} in the sky writes depth`);
          if (m.transparent) out.push(`${o.type} ${o.name || ""} in the sky is transparent, so it is drawn after the solids`);
        }
        skyItems.push(o);
      } else if (mats.some((m) => m.depthWrite && !m.transparent)) {
        solids.push(o);
      }
    });
    for (const s of skyItems) {
      const late = solids.find((o) => !before(key(s), key(o)));
      if (late) out.push(`${s.type} in the sky is drawn after the solid ${late.type} ${late.name || ""} (${key(s)} vs ${key(late)})`);
    }
    if (!skyItems.length) out.push("the sky layer draws nothing");
  }
  if (!sky.length) out.push("the page sets no window.MOCKUP_SKY");
  return [...new Set(out)];
}

const browser = await chromium.launch({ executablePath: "/opt/pw-browsers/chromium", args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"] })
  .catch(() => chromium.launch({ args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"] }));
let failed = false;
for (const file of pages) {
  const page = await browser.newPage({ viewport: { width: 800, height: 500 } });
  await page.route("https://cdn.jsdelivr.net/npm/**", (route) => {
    const rel = route.request().url().replace("https://cdn.jsdelivr.net/npm/", "").split("?")[0];
    route.fulfill({ body: fs.readFileSync(path.join(CACHE, rel)), contentType: "application/javascript" });
  });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("file://" + file);
  for (let i = 0; i < 100; i++) { if (await page.evaluate(() => window.MOCKUP_READY)) break; await page.waitForTimeout(3000); }
  const name = path.relative(ROOT, file);
  if (errors.length) { console.log(`${name}: FAIL ${errors[0]}`); failed = true; await page.close(); continue; }
  const shots = await page.evaluate(() => (window.MOCKUP_SHOTS || []).map((s) => s.name));
  for (const shot of [null, ...shots]) {
    if (shot !== null) {
      await page.evaluate((n) => window.MOCKUP_SHOTS.find((s) => s.name === n).setup(), shot);
      await page.waitForTimeout(300);
    }
    const bad = await page.evaluate(breaches);
    const where = `${name}${shot ? ` (${shot})` : ""}`;
    if (bad.length) { failed = true; console.log(`${where}: FAIL`); for (const b of bad) console.log(`  ${b}`); }
    else console.log(`${where}: ok, the sky layer is drawn before every solid`);
  }
  await page.close();
}
await browser.close();
process.exit(failed ? 1 : 0);
