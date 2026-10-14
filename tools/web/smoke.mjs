#!/usr/bin/env node
/*
 * smoke.mjs: board the browser build and fail when it cannot play the game (engine-stack design 10b;
 * browser-single-player design 8).
 *
 * It serves the built folder (build/web by default) on a local port, opens it in headless Chromium with WebGL 2
 * (SwiftShader), waits for the front menu, presses Enter (Single Player) and Enter again (Beam aboard), and reads the
 * client's console. Beam aboard hosts the drill in the page on the local link: the test passes when the page says it
 * hosts single player, the bridge for the crew on foot is loaded, and the round runs as far as Engage, with no
 * fallback and no page error. "no ship (" means the client fell back to first light's test room (a missing data
 * file did exactly that on 2026-10-13, and the release still went out, because nothing ran the page before pushing
 * it); "no bridge to walk" or "no walk on foot" means the game came up without the ship to walk.
 *
 * tools/web/release.sh runs it after the build and before the push. It needs Playwright and Chromium, which the
 * deploy workflow already installs for the deck export.
 *
 * Usage: node tools/web/smoke.mjs [build dir, default build/web] [--timeout-s 240] [--shot file.png]
 * --shot saves the page once the round is engaged.
 */
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { createRequire } from "node:module";
import { execSync } from "node:child_process";

const require = createRequire(import.meta.url);
function loadPlaywright() {
  try { return require("playwright"); } catch (_) { /* the global install */ }
  return require(path.join(execSync("npm root -g").toString().trim(), "playwright"));
}
const { chromium } = loadPlaywright();

const args = process.argv.slice(2);
const opt = (name) => (args.indexOf(name) >= 0 ? args[args.indexOf(name) + 1] : undefined);
const timeoutS = Number(opt("--timeout-s") ?? 240);
const shot = opt("--shot");
const dir = path.resolve(args.find((a, i) => !a.startsWith("--") && !String(args[i - 1]).startsWith("--")) || "build/web");
if (!fs.existsSync(path.join(dir, "index.html"))) {
  console.error(`smoke: no index.html in ${dir}; build it with tools/web/build.sh`);
  process.exit(2);
}

const TYPES = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".gz": "application/gzip" };
const server = http.createServer((req, res) => {
  const rel = decodeURIComponent(new URL(req.url, "http://x").pathname).replace(/^\/+/, "") || "index.html";
  const file = path.join(dir, rel);
  if (!file.startsWith(dir) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) {
    res.writeHead(404).end();
    return;
  }
  res.writeHead(200, { "content-type": TYPES[path.extname(file)] || "application/octet-stream" });
  fs.createReadStream(file).pipe(res);
});
await new Promise((ok) => server.listen(0, "127.0.0.1", ok));
const url = `http://127.0.0.1:${server.address().port}/index.html`;

const launch = { args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"] };
const browser = await chromium.launch(launch).catch(() => chromium.launch({ ...launch, executablePath: "/opt/pw-browsers/chromium" }));
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
const lines = [];
page.on("console", (m) => lines.push(m.text()));
page.on("pageerror", (e) => lines.push("pageerror: " + e.message));

const t0 = Date.now();
const seen = (s) => lines.some((l) => l.includes(s));
const wait = async (pred, what) => {
  while (!pred()) {
    if ((Date.now() - t0) / 1000 > timeoutS) throw new Error(`timed out after ${timeoutS} s waiting for ${what}`);
    await page.waitForTimeout(500);
  }
};
let verdict = 0;
try {
  await page.goto(url);
  // The client is up when it has printed its GL line; give the menu a few frames to draw.
  await wait(() => seen("sc-client:"), "the client to start");
  await page.waitForTimeout(3000);
  const canvas = await page.$("canvas");
  if (canvas) await canvas.click({ position: { x: 10, y: 10 } });
  await page.keyboard.press("Enter");
  await page.waitForTimeout(1500);
  await page.keyboard.press("Enter");
  const broken = () => ["no ship (", "no bridge to walk", "no walk on foot", "pageerror:"].some(seen);
  const hosted = () => seen("single player on the local server") && seen("the bridge for the crew on foot");
  const engaged = () => lines.some((l) => /sc-client: round \d+ Engage/.test(l));
  await wait(() => engaged() || broken(), "the round to reach Engage");
  if (shot) await page.screenshot({ path: shot });
  if (broken() || !hosted() || !engaged()) {
    verdict = 1;
    console.error("smoke: FAIL, the browser build did not host the game and play into a round:");
  } else {
    console.log(`smoke: ok, the browser hosted single player and the round reached Engage in ${((Date.now() - t0) / 1000).toFixed(0)} s`);
  }
} catch (e) {
  verdict = 1;
  console.error("smoke: FAIL, " + e.message);
}
if (verdict) for (const l of lines.filter((l) => /sc-client|pageerror|error/i.test(l)).slice(-20)) console.error("  " + l);
await browser.close();
server.close();
process.exit(verdict);
