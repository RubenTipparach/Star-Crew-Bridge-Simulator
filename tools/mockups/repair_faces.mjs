#!/usr/bin/env node
/*
 * repair_faces.mjs: the still picture of every repair game that a machine shows behind its cover
 * (openspec/changes/repairs-on-deck, design 3b), captured from the game itself so the picture and the
 * game cannot drift apart.
 *
 * For each game in docs/mockups/repairs.html it opens a damaged job as an officer, takes the kit's
 * own 2D cover off (in the ship the cover is the 3D one), lets the game draw a moment, and keeps the
 * canvas below the kit's bar (1280 x 648), area-averaged to 256 x 130 px. It writes
 * assets/textures/repairs/faces/<game>.png and assets/textures/repairs/faces/faces.json (each
 * file's size and sha256). Rerun it when a game's look changes.
 *
 * Usage: node tools/mockups/repair_faces.mjs [--only pump,coolant]
 */
import { createRequire } from "node:module";
import { execSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const require = createRequire(import.meta.url);
function loadPlaywright() {
  try { return require("playwright"); } catch (_) { /* fall through to the global install */ }
  return require(path.join(execSync("npm root -g").toString().trim(), "playwright"));
}
const { chromium } = loadPlaywright();

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const OUT = path.join(ROOT, "assets", "textures", "repairs", "faces");
const PAGE = path.join(ROOT, "docs", "mockups", "repairs.html");
const FACE = [256, 130];          // px: the game area, 1280 x 648, at a fifth (design 3b)
const args = process.argv.slice(2);
const oi = args.indexOf("--only");
const only = oi >= 0 ? args[oi + 1].split(",") : null;

fs.mkdirSync(OUT, { recursive: true });
let browser;
const launchOpts = { args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"] };
try { browser = await chromium.launch(launchOpts); }
catch (_) { browser = await chromium.launch({ ...launchOpts, executablePath: "/opt/pw-browsers/chromium" }); }
const page = await browser.newPage({ viewport: { width: 1600, height: 1000 } });
const errors = [];
page.on("pageerror", (e) => errors.push(String(e)));
await page.goto(pathToFileURL(PAGE).href);
await page.waitForFunction(() => !!window.REPAIRS && !!window.RepairKit, null, { timeout: 60000 });
const ids = await page.evaluate(() => window.RepairKit.games.map((g) => g.id));
const manifestPath = path.join(OUT, "faces.json");
const faces = fs.existsSync(manifestPath) ? JSON.parse(fs.readFileSync(manifestPath, "utf8")).faces : {};
for (const id of ids) {
  if (only && !only.includes(id)) continue;
  const uri = await page.evaluate(async ([id, fw, fh]) => {
    const R = window.REPAIRS;
    R.opts.state = "damaged"; R.opts.who = "officer"; R.opts.combat = false;
    R.open(id);
    const frames = (n) => new Promise((r) => { let i = 0; const f = () => (++i >= n ? r() : requestAnimationFrame(f)); requestAnimationFrame(f); });
    await frames(2);
    if (R.guide && R.guide.open) R.guide.hide();   // the how-to card is a 2D overlay, not part of the game
    if (R.run && R.run.panel) { R.run.panel.phase = "work"; R.run.panel.lift = 1; }
    await frames(30);
    const src = document.getElementById("c");
    // Area-average the game area (below the kit's 72 px bar) down in two halvings and a last step.
    let c = document.createElement("canvas"); c.width = 1280; c.height = 648;
    c.getContext("2d").drawImage(src, 0, 72, 1280, 648, 0, 0, 1280, 648);
    for (const s of [2, 2]) {
      const n = document.createElement("canvas"); n.width = c.width / s; n.height = Math.round(c.height / s);
      const g = n.getContext("2d"); g.imageSmoothingQuality = "high"; g.drawImage(c, 0, 0, n.width, n.height); c = n;
    }
    const o = document.createElement("canvas"); o.width = fw; o.height = fh;
    const g = o.getContext("2d"); g.imageSmoothingQuality = "high"; g.drawImage(c, 0, 0, fw, fh);
    return o.toDataURL("image/png");
  }, [id, FACE[0], FACE[1]]);
  const file = path.join(OUT, `${id}.png`);
  const buf = Buffer.from(uri.split(",")[1], "base64");
  fs.writeFileSync(file, buf);
  faces[id] = { file: path.relative(ROOT, file), px: FACE, bytes: buf.length, sha256: crypto.createHash("sha256").update(buf).digest("hex") };
  console.log(`  ${faces[id].file}: ${buf.length} bytes`);
}
await browser.close();
const sorted = Object.fromEntries(Object.keys(faces).sort().map((k) => [k, faces[k]]));
fs.writeFileSync(manifestPath, JSON.stringify({ schema: "starcrew.repair-faces/1", source: "docs/mockups/repairs.html",
  built_by: "tools/mockups/repair_faces.mjs", game_area_px: [1280, 648], faces: sorted }, null, 2) + "\n");
if (errors.length) { console.error(errors.join("\n")); process.exit(1); }
