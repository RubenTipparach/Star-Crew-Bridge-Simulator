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
 * Usage: node tools/web/smoke.mjs [build dir, default build/web] [--timeout-s 240] [--shot file.png] [--touch [DIR]]
 * --shot saves the page once the round is engaged.
 *
 * --touch plays it as a phone held sideways instead (openspec/changes/touch-controls design 7): 844 x 390, touch only,
 * no key and no mouse. It taps Single Player and Beam aboard, walks the body from where it boards up the corridor to
 * the bridge on the move stick (the thumb held at its rim, so it runs), turns the view with a drag, sits at the first
 * seat in reach with USE, waits for Engage, and stands up again by holding STAND. It passes when the client says the
 * touch layer came on, that the body sat and that it stood up. DIR saves a shot of each step
 * (docs/screenshots/web/touch/).
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
const touchAt = args.indexOf("--touch");
const touch = touchAt >= 0;
const touchShots = touch && args[touchAt + 1] && !args[touchAt + 1].startsWith("--") ? path.resolve(args[touchAt + 1]) : null;
const dir = path.resolve(args.find((a, i) => !a.startsWith("--") && !String(args[i - 1]).startsWith("--")) || "build/web");
if (touchShots) fs.mkdirSync(touchShots, { recursive: true });
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
const page = touch
  ? await (await browser.newContext({ viewport: { width: 844, height: 390 }, hasTouch: true, isMobile: true, deviceScaleFactor: 2 })).newPage()
  : await browser.newPage({ viewport: { width: 1280, height: 800 } });
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
/** The phone's run (--touch): touch only, from the front menu to a seat and back on foot. */
async function playByTouch() {
  const cdp = await page.context().newCDPSession(page);
  const fingers = (type, pts) => cdp.send("Input.dispatchTouchEvent", { type, touchPoints: pts.map(([x, y, id]) => ({ x, y, id })) });
  const snap = async (name) => { if (touchShots) await page.screenshot({ path: path.join(touchShots, name) }); };
  await wait(() => seen("sc-client:"), "the client to start");
  await page.waitForTimeout(3000);
  await snap("1-front-menu.png");
  // The front menu's buttons where they draw on a 390 px tall screen: Single Player, then Beam aboard (at Helm).
  await page.touchscreen.tap(422, 170);
  await page.waitForTimeout(2000);
  await snap("2-single-player.png");
  await page.touchscreen.tap(422, 292);
  await wait(() => seen("single player on the local server") && seen("the bridge for the crew on foot"), "the game to board");
  await page.waitForTimeout(8000);
  await snap("3-aboard.png");
  // The move stick: a thumb lands low on the left and pushes forward to the rim; held there it runs. The body walks up
  // the corridor to the bridge in short pushes until a seat is in reach (the client says so: "prompt: E Sit at ..." or
  // "E Take ..."); the crew walking the same corridor push it about, so the walk is steered by what the client says,
  // not timed.
  const prompt = () => lines.filter((l) => l.startsWith("sc-client: prompt: ")).pop() || "";
  const inReach = () => /prompt: E (Sit at|Take) /.test(prompt());
  let pushes = 0, waits = 0;
  while (!inReach()) {
    // A seat in reach that a crew member is still walking to is theirs: stand and wait for them to sit.
    if (/ is on the way to /.test(prompt())) {
      if (++waits > 120) throw new Error("the crew member walking to the seat in reach never sat");
      await page.waitForTimeout(500);
      continue;
    }
    if (++pushes > 40) throw new Error("the stick never walked the body to a seat");
    await fingers("touchStart", [[150, 300, 1]]);
    for (let i = 1; i <= 6; i++) { await fingers("touchMove", [[150, 300 - i * 10, 1]]); await page.waitForTimeout(20); }
    await page.waitForTimeout(400);
    if (pushes === 3) await snap("4-walking-on-the-stick.png");
    await fingers("touchEnd", []);
    await page.waitForTimeout(150);
    // On the bridge, a drag on the right turns the view toward the seats.
    if (pushes === 5) {
      await fingers("touchStart", [[600, 200, 2]]);
      for (let i = 1; i <= 10; i++) { await fingers("touchMove", [[600 + i * 12, 200, 2]]); await page.waitForTimeout(30); }
      await fingers("touchEnd", []);
    }
  }
  await page.waitForTimeout(500);
  await snap("5-a-seat-in-reach.png");
  // USE, bottom right: sit (or take the seat from the NPC in it). The body boards seated and stands, so only a sit
  // said after the tap counts; a tap the server refuses (the walk it holds a moment behind the client's) is tapped again.
  const after = (n, s) => lines.slice(n).some((l) => l.includes(s));
  let sat = false;
  for (let tries = 0; tries < 3 && !sat; tries++) {
    const n = lines.length;
    await page.touchscreen.tap(776, 322);
    for (let i = 0; i < 20 && !after(n, "sc-client: sat at "); i++) await page.waitForTimeout(250);
    sat = after(n, "sc-client: sat at ");
  }
  if (!sat) throw new Error("USE never sat the body down");
  await wait(() => lines.some((l) => /sc-client: round \d+ Engage/.test(l)), "the round to reach Engage");
  await page.waitForTimeout(2000);
  await snap("6-seated-the-console.png");
  // STAND, in the bar beside the console, held well past the seats' stand_hold_s: the hold counts game time, which
  // a slow frame caps, so a loaded runner needs longer than a phone does.
  const held = lines.length;
  await fingers("touchStart", [[37, 36, 3]]);
  await page.waitForTimeout(3000);
  await fingers("touchEnd", []);
  await wait(() => after(held, "sc-client: stood up"), "holding STAND to stand the body up");
  await page.waitForTimeout(1500);
  await snap("7-stood-up.png");
  if (!seen("sc-client: touch layer on")) throw new Error("the touch layer never came on");
}

let verdict = 0;
try {
  await page.goto(url);
  if (touch) {
    await playByTouch();
    const broken = ["no ship (", "no bridge to walk", "no walk on foot", "pageerror:"].some(seen);
    if (broken) throw new Error("the game came up broken");
    console.log(`smoke: ok, played by touch: boarded, walked to a seat, sat, stood (${((Date.now() - t0) / 1000).toFixed(0)} s)`);
  } else {
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
  }
} catch (e) {
  verdict = 1;
  console.error("smoke: FAIL, " + e.message);
}
if (verdict) for (const l of lines.filter((l) => /sc-client|pageerror|error/i.test(l)).slice(-20)) console.error("  " + l);
await browser.close();
server.close();
process.exit(verdict);
