#!/usr/bin/env node
/*
 * bake_ship.mjs: bake the whole Tern in the deck plan and report it (openspec/changes/light-baking, design section 15).
 *
 * It opens docs/mockups/deck-plan.html headless, waits until the page's bake (lightbake.js, every compartment alone,
 * three lighting states from one set of rays, data/lighting/bake.json's final settings) has finished every room, and
 * writes docs/benchmarks/<date>-tern-bake/report.json and report.md: per compartment its floor area, lamps (on the
 * emergency bus), strips, triangles (the shell before and after its split to mockup.cell_m, the room in all) against
 * its ceiling, vertices baked, rays, the time of each step and the bake's digest. Then it shoots fixed views in the
 * three states, and the same views with the quick light the page showed before (?bake=0), into
 * docs/screenshots/mockups/bake/, with one comparison strip a view (quick, normal, red alert, emergency power).
 *
 * A measurement instrument (CLAUDE.md section 4): it changes nothing. Times are this machine's CPU in one browser
 * thread with software rendering: never a Pi 5 number, never the engine baker's (CLAUDE.md 12).
 *
 * Usage: node tools/mockups/bake_ship.mjs [--date YYYY-MM-DD] [--views a,b] [--no-shots] [--timeout-min 60]
 */
import { createRequire } from "node:module";
import { execSync, execFileSync } from "node:child_process";
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
const PAGE = path.join(ROOT, "docs", "mockups", "deck-plan.html");
const args = process.argv.slice(2);
const opt = (name, dflt) => { const i = args.indexOf(name); return i >= 0 ? args.splice(i, 2)[1] : dflt; };
const flag = (name) => { const i = args.indexOf(name); if (i >= 0) args.splice(i, 1); return i >= 0; };
const date = opt("--date", new Date().toISOString().slice(0, 10));
const views = opt("--views", "walk-bridge-forward,walk-bridge-helm-chairs,walk-bridge-captain,walk-eng-mezzanine,walk-eng-lower,walk-corridor-B").split(",");
const timeoutMin = Number(opt("--timeout-min", "60")), noShots = flag("--no-shots");
const OUT = path.join(ROOT, "docs", "benchmarks", `${date}-tern-bake`), SHOTS = path.join(ROOT, "docs", "screenshots", "mockups", "bake");
const STATES = ["normal", "red_alert", "emergency"];
// Each compartment's triangle ceiling (engine-stack design section 5, "Content"; engineering's from engineering-fitout
// design section 7). The report compares; the tables are the source.
const CEILING = { bridge: 30000, engineering: 30000 }, CEILING_DEFAULT = 8000;

async function open(browser, query) {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.stack || e.message));
  await page.route("https://cdn.jsdelivr.net/npm/**", (route) => {
    const rel = route.request().url().replace("https://cdn.jsdelivr.net/npm/", "").split("?")[0];
    route.fulfill({ body: fs.readFileSync(path.join(CACHE, rel)), contentType: "application/javascript" });
  });
  await page.goto("file://" + PAGE + (query || ""));
  for (let i = 0; i < 120; i++) { if (await page.evaluate(() => window.MOCKUP_READY)) break; await page.waitForTimeout(1000); }
  if (errors.length) throw new Error("deck-plan.html: " + errors[0]);
  return { page, errors };
}
async function shoot(page, name, file) {
  await page.evaluate((n) => { const s = (window.MOCKUP_SHOTS || []).find((x) => x.name === n); if (!s) throw new Error("no shot " + n); s.setup(); }, name);
  await page.waitForTimeout(1200);
}

const browser = await chromium.launch({ executablePath: "/opt/pw-browsers/chromium", args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"] })
  .catch(() => chromium.launch({ args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"] }));
const t0 = Date.now();
const { page, errors } = await open(browser, "");
let last = -1;
for (;;) {
  const s = await page.evaluate(() => window.MOCKUP_BAKE());
  if (errors.length) throw new Error("deck-plan.html: " + errors[0]);
  if (s.baked !== last) { process.stdout.write(`baked ${s.baked}/${s.total} after ${((Date.now() - t0) / 1000).toFixed(0)} s\n`); last = s.baked; }
  if (s.done) break;
  if (Date.now() - t0 > timeoutMin * 60000) throw new Error(`bake not done after ${timeoutMin} min`);
  await page.waitForTimeout(2000);
}
const bake = await page.evaluate(() => window.MOCKUP_BAKE());
const wall = (Date.now() - t0) / 1000;

// ------------------------------------------------------------------ the report
const rows = Object.entries(bake.rooms).map(([id, r]) => {
  const ceiling = CEILING[id] || CEILING_DEFAULT;
  return Object.assign({ id, ceiling, overCeiling: r.roomTriangles > ceiling, splitAdded: r.splitAfter - r.splitBefore }, r);
});
const sum = (k) => rows.reduce((s, r) => s + (r[k] || 0), 0);
const totals = { compartments: rows.length, floor_m2: +sum("floor_m2").toFixed(1), lamps: sum("lamps"), emergencyLamps: sum("emergencyLamps"), strips: sum("strips"),
  roomTriangles: sum("roomTriangles"), splitAdded: sum("splitAdded"), vertices: sum("vertices"), cachePoints: sum("cachePoints"),
  rays: rows.reduce((s, r) => s + (r.rays ? r.rays.shadow + r.rays.gather + r.rays.ao : 0), 0), bakeMs: sum("ms"), wall_s: +wall.toFixed(1) };
fs.mkdirSync(OUT, { recursive: true });
const settings = JSON.parse(fs.readFileSync(path.join(ROOT, "data", "lighting", "bake.json"), "utf8"));
const report = { schema: "starcrew.bake-report/1", date, page: "docs/mockups/deck-plan.html", generator: "tools/mockups/bake_ship.mjs",
  machine: "Claude Code cloud container, headless Chromium with SwiftShader, one browser thread; not a Pi 5 and not the engine baker",
  settings: { seed: settings.seed, reference_lux: settings.reference_lux, shadow_samples: settings.shadow_samples, ao_rays: settings.ao_rays,
    bounces: settings.bounces, cache_spacing_m: settings.cache_spacing_m, cache_gather_rays: settings.cache_gather_rays, cell_m: settings.mockup.cell_m },
  totals, compartments: rows };
fs.writeFileSync(path.join(OUT, "report.json"), JSON.stringify(report, null, 1) + "\n");
const fmt = (n) => (typeof n === "number" ? n.toLocaleString("en-US") : n);
const md = [
  `# Tern bake, ${date}`, "",
  "Written by `tools/mockups/bake_ship.mjs` (openspec/changes/light-baking, design section 15): the deck plan baking every compartment with",
  "`lightbake.js`, alone, doors closed, the three lighting states from one set of rays, at `data/lighting/bake.json`'s final settings,",
  `the shell split to ${settings.mockup.cell_m} m cells. ${report.machine}. **Times say nothing about a Pi 5 or the engine baker.**`, "",
  `Totals: ${totals.compartments} compartments, ${fmt(totals.floor_m2)} m^2 of floor, ${totals.lamps} lamps (${totals.emergencyLamps} on the emergency bus), ${totals.strips} cove strips;`,
  `${fmt(totals.roomTriangles)} triangles in the rooms, ${fmt(totals.splitAdded)} of them added by the split; ${fmt(totals.vertices)} vertices baked; ${fmt(totals.rays)} rays;`,
  `${(totals.bakeMs / 1000).toFixed(0)} s of baking (${totals.wall_s} s from opening the page).`, "",
  "| Compartment | Floor m^2 | Lamps (emergency) | Strips | Triangles / ceiling | Split added | Vertices | Cache points | Rays | Time s (scene, cache, vertices) | Digest |",
  "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |",
  ...rows.map((r) => `| ${r.name} | ${r.floor_m2.toFixed(1)} | ${r.lamps} (${r.emergencyLamps}) | ${r.strips} | ${fmt(r.roomTriangles)} / ${fmt(r.ceiling)}${r.overCeiling ? " **over**" : ""} | ${fmt(r.splitAdded)} | ${fmt(r.vertices)} | ${fmt(r.cachePoints)} | ${fmt(r.rays.shadow + r.rays.gather + r.rays.ao)} | ${(r.ms / 1000).toFixed(1)} (${(r.sceneMs / 1000).toFixed(1)}, ${(r.cacheMs / 1000).toFixed(1)}, ${(r.verticesMs / 1000).toFixed(1)}) | \`${r.digest}\` |`),
  "", "A room's triangles are the deck plan's whole room (shell, detail, props, the split), against its ceiling in `engine-stack`'s table",
  "(engineering's from `engineering-fitout`). The split stands in for the engine's adaptive subdivision (design section 3), which spends fewer.", "",
];
fs.writeFileSync(path.join(OUT, "report.md"), md.join("\n"));
console.log(`report: ${path.relative(ROOT, OUT)}/report.md`);

// ------------------------------------------------------------------ the shots
if (!noShots) {
  fs.mkdirSync(SHOTS, { recursive: true });
  for (const v of views) {
    for (const st of STATES) {
      await shoot(page, v);
      await page.evaluate((k) => window.MOCKUP_LIGHT(k), st);
      await page.waitForTimeout(600);
      await page.screenshot({ path: path.join(SHOTS, `${v}-${st}.png`) });
    }
  }
  await page.close();
  const quick = await open(browser, "?bake=0");
  for (const v of views) {
    await shoot(quick.page, v);
    await quick.page.screenshot({ path: path.join(SHOTS, `${v}-quick.png`) });
  }
  await quick.page.close();
  // One strip a view: the quick light, then the bake in the three states.
  execFileSync("python3", ["-c", `
import sys
from PIL import Image, ImageDraw
d, views = sys.argv[1], sys.argv[2].split(",")
labels = [("quick", "before: quick light"), ("normal", "baked: normal"), ("red_alert", "baked: red alert"), ("emergency", "baked: emergency power")]
for v in views:
    ims = [Image.open(f"{d}/{v}-{k}.png").convert("RGB").resize((720, 450)) for k, _ in labels]
    out = Image.new("RGB", (1440, 900), (20, 22, 26))
    for i, (im, (_, t)) in enumerate(zip(ims, labels)):
        x, y = (i % 2) * 720, (i // 2) * 450
        out.paste(im, (x, y))
        dr = ImageDraw.Draw(out)
        dr.rectangle((x + 8, y + 8, x + 8 + 9 * len(t), y + 30), fill=(8, 11, 16))
        dr.text((x + 14, y + 12), t, fill=(232, 236, 242))
    out.save(f"{d}/{v}-compare.png")
`, SHOTS, views.join(",")]);
  console.log(`shots: ${path.relative(ROOT, SHOTS)}/ (${views.length} views x 3 states, quick, compare)`);
}
await browser.close();
