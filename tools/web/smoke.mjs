#!/usr/bin/env node
/*
 * smoke.mjs: board the browser build and fail when it cannot play the game (engine-stack design 10b;
 * browser-single-player design 8).
 *
 * It serves the built folder (build/web by default) on a local port, opens it in headless Chromium with WebGL 2
 * (SwiftShader), waits for the front menu, presses Enter (Single Player) and Enter again (Beam aboard), and reads the
 * client's console. Beam aboard hosts the drill in the page on the local link: the test passes when the page says it
 * hosts single player, the bridge for the crew on foot is loaded, and, once the body has walked up the corridor on the
 * keys and sat at the first seat in reach, and START MISSION is clicked where the engine drew it (the round never
 * starts by itself: openspec/changes/start-mission design 1), the round runs as far as Engage, with no
 * fallback and no page error. "no ship (" means the client fell back to first light's test room (a missing data
 * file did exactly that on 2026-10-13, and the release still went out, because nothing ran the page before pushing
 * it); "no bridge to walk" or "no walk on foot" means the game came up without the ship to walk.
 *
 * tools/web/release.sh runs it after the build and before the push. It needs Playwright and Chromium, which the
 * deploy workflow already installs for the deck export.
 *
 * Usage: node tools/web/smoke.mjs [build dir, default build/web] [--timeout-s 900] [--shot file.png] [--touch [DIR]]
 *   [--fit [DIR]]
 * --shot saves the page once the round is engaged.
 *
 * --touch plays it as a phone held sideways instead (openspec/changes/touch-controls design 7): 844 x 390, touch only,
 * no key and no mouse. It taps Single Player and Beam aboard, walks the body from where it boards up the corridor to
 * the bridge on the move stick (the thumb held at its rim, so it runs), turns the view with a drag, sits at the first
 * seat in reach with USE, waits for Engage, and stands up again by holding STAND. It passes when the client says the
 * touch layer came on, that the body sat and that it stood up. DIR saves a shot of each step
 * (docs/screenshots/web/touch/).
 *
 * --fit checks the page fits the game at every size and pixel ratio, with the mouse (openspec/changes/browser-page-fit
 * design 3): itch.io's full screen once left the engine drawing a frame of the old size and reading the mouse at it.
 * At each size of design 5 (1920 x 1080 down to a phone's 844 x 390), at a pixel ratio of 1 and of 2, and in full
 * screen, it resizes the page and fails unless the engine's window is the canvas's size and its pixels the drawing
 * buffer's; then it clicks SINGLE PLAYER and BACK where the engine says it drew them (`sc-client: ui <label> at x y w
 * h`), and fails if either is outside the page or the click does not land (`sc-client: ui clicked <label>`). Then it
 * beams aboard, opens the game menu with Esc and clicks Options and Back and Resume the same way. DIR saves a shot at
 * each size (docs/screenshots/web/fit/).
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
const timeoutS = Number(opt("--timeout-s") ?? 900);
const shot = opt("--shot");
const touchAt = args.indexOf("--touch");
const touch = touchAt >= 0;
const touchShots = touch && args[touchAt + 1] && !args[touchAt + 1].startsWith("--") ? path.resolve(args[touchAt + 1]) : null;
const fitAt = args.indexOf("--fit");
const fit = fitAt >= 0;
const fitShots = fit && args[fitAt + 1] && !args[fitAt + 1].startsWith("--") ? path.resolve(args[fitAt + 1]) : null;
const optionArgs = new Set(["--timeout-s", "--shot", "--touch", "--fit"]);
const dir = path.resolve(args.find((a, i) => !a.startsWith("--") && !optionArgs.has(String(args[i - 1]))) || "build/web");
if (touchShots) fs.mkdirSync(touchShots, { recursive: true });
if (fitShots) fs.mkdirSync(fitShots, { recursive: true });
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
  // The front menu's buttons where the engine says it drew them (browser-page-fit design 3): Single Player, then Beam
  // aboard (at Helm). A tap at a hand-typed place broke when the layout moved.
  const tapAt = async (label) => {
    const re = new RegExp(`^sc-client: ui ${label} at (-?\\d+) (-?\\d+) (\\d+) (\\d+)$`);
    let r = null;
    for (let i = 0; i < 40 && !r; i++) {
      const l = lines.filter((x) => re.test(x)).pop();
      r = l ? l.match(re).slice(1, 5).map(Number) : null;
      if (!r) await page.waitForTimeout(250);
    }
    if (!r) throw new Error(`the engine never said where ${label} is`);
    await page.touchscreen.tap(r[0] + r[2] / 2, r[1] + r[3] / 2);
  };
  await tapAt("SINGLE PLAYER");
  await page.waitForTimeout(2000);
  await snap("2-single-player.png");
  await tapAt("BEAM ABOARD");
  await wait(() => seen("single player on the local server") && seen("the bridge for the crew on foot"), "the game to board");
  await page.waitForTimeout(8000);
  await snap("3-aboard.png");
  // The move stick: a thumb lands low on the left and pushes forward to the rim; held there it runs. The body walks up
  // the corridor to the bridge in short pushes until a seat is in reach (the client says so: "prompt: Helm [E] Sit
  // down" or "[E] Relieve ..."); the crew walking the same corridor push it about, so the walk is steered by what the client says,
  // not timed.
  const prompt = () => lines.filter((l) => l.startsWith("sc-client: prompt: ")).pop() || "";
  const inReach = () => /prompt: .* \[E\] (Sit down|Relieve )/.test(prompt());
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
  // The round waits for START MISSION (start-mission 1): a tap on the chip where the engine drew it in the title band.
  await page.waitForTimeout(1500);
  await snap("5b-start-mission.png");
  let started = false;
  for (let tries = 0; tries < 3 && !started; tries++) {
    const m = lines.length;
    await tapAt("START MISSION");
    for (let i = 0; i < 20 && !after(m, "sc-client: ui clicked START MISSION"); i++) await page.waitForTimeout(250);
    started = after(m, "sc-client: ui clicked START MISSION");
  }
  if (!started) throw new Error("a tap on START MISSION did not land");
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

/** The fit run (--fit): every size and pixel ratio, and full screen, by the mouse where the engine drew. */
async function playFit() {
  // The engine's last word on where a control is, window coordinates (CSS px): [x, y, w, h].
  // `since`: only what it said from that line on (a page or size just changed, so an older word is stale).
  const where = (label, since = 0) => {
    const re = new RegExp(`^sc-client: ui ${label.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")} at (-?\\d+) (-?\\d+) (\\d+) (\\d+)$`);
    const l = lines.slice(since).filter((x) => re.test(x)).pop();
    return l ? l.match(re).slice(1, 5).map(Number) : null;
  };
  const after = (n, s) => lines.slice(n).some((l) => l.includes(s));
  const SIZES = [[1920, 1080], [1280, 720], [1366, 768], [2560, 1440], [1024, 768], [2560, 1080], [985, 455], [844, 390]];
  // A pixel ratio is a browser context's, so each ratio boots the page once and resizes it through every size.
  for (const ratio of [1, 2]) {
    const ctx = await browser.newContext({ viewport: { width: 1280, height: 720 }, deviceScaleFactor: ratio });
    const pg = await ctx.newPage();
    pg.on("console", (m) => lines.push(m.text()));
    pg.on("pageerror", (e) => lines.push("pageerror: " + e.message));
    const snap = async (name) => { if (fitShots) await pg.screenshot({ path: path.join(fitShots, name) }); };
    const state = async () => pg.evaluate(() => {
      const c = document.querySelector("canvas"), r = c.getBoundingClientRect();
      return { w: Math.round(r.width), h: Math.round(r.height), bw: c.width, bh: c.height, ratio: devicePixelRatio, vw: innerWidth, vh: innerHeight };
    });
    /** The engine's window follows the page: its size the canvas's CSS size, its pixels the drawing buffer. */
    const follows = async (what) => {
      let p, ok = false;
      for (let i = 0; i < 60 && !ok; i++) {
        await pg.waitForTimeout(250);
        p = await state();
        const say = lines.filter((l) => l.startsWith("sc-client: window ")).pop() || "";
        const m = say.match(/window (\d+) x (\d+), (\d+) x (\d+) pixels/);
        ok = m && +m[1] === p.w && +m[2] === p.h && +m[3] === p.bw && +m[4] === p.bh && p.bw === Math.round(p.w * p.ratio);
      }
      if (!ok) throw new Error(`${what}: the engine's window does not follow the page (${JSON.stringify(p)}; ${lines.filter((l) => l.startsWith("sc-client: window ")).pop()})`);
    };
    /** Click `label` where the engine drew it; fail if it is off the page or the click does not land. */
    // The line from which the engine's word on where things are is fresh: set before a resize and before each click
    // that changes the page. The engine says where a control is only when it moves, and a frame at 5120 x 2160 pixels
    // under software GL takes longer than the 0.6 s stillness, so the word from before the change looked settled
    // (the PR #34 deploy clicked BACK where the last size had it).
    let mark = 0;
    const click = async (label, what) => {
      // Where it was said since the mark, once it has stayed put for 0.6 s: a page that just changed size is still
      // being laid out. A control that did not move since (Resume, back on the game menu) says nothing new, so after
      // 4 s with no fresh word its older one stands.
      let r = null;
      for (let i = 0, still = 0; i < 100 && still < 3; i++) {
        await pg.waitForTimeout(200);
        const now = where(label, mark) || (i >= 20 ? where(label) : null);
        still = now && r && now.join() === r.join() ? still + 1 : 0;
        r = now;
      }
      if (!r) throw new Error(`${what}: the engine never said where ${label} is`);
      const p = await state();
      const [x, y, w, h] = r;
      if (x < 0 || y < 0 || x + w > p.vw + 1 || y + h > p.vh + 1) throw new Error(`${what}: ${label} at ${r} is off the ${p.vw} x ${p.vh} page`);
      const n = lines.length;
      mark = n;
      await pg.mouse.click(x + w / 2, y + h / 2);
      for (let i = 0; i < 20 && !after(n, `sc-client: ui clicked ${label}`); i++) await pg.waitForTimeout(150);
      if (!after(n, `sc-client: ui clicked ${label}`)) throw new Error(`${what}: a click at the centre of ${label} (${r}) did not land`);
    };
    const n0 = lines.length;
    await pg.goto(url, { waitUntil: "domcontentloaded", timeout: 180000 });
    await wait(() => after(n0, "sc-client:"), "the client to start");
    await pg.waitForTimeout(3000);
    for (const [w, h] of SIZES) {
      mark = lines.length;
      await pg.setViewportSize({ width: w, height: h });
      const what = `${w} x ${h} at ${ratio}`;
      await follows(what);
      await click("SINGLE PLAYER", what);
      await snap(`${w}x${h}-${ratio}x-single-player.png`);
      await click("BACK", what);
    }
    if (ratio === 2) {
      // Full screen, as itch's button does: the page fills the screen and the game follows.
      mark = lines.length;
      await pg.setViewportSize({ width: 1280, height: 720 });
      await pg.evaluate(() => document.documentElement.requestFullscreen().catch(() => {}));
      await follows("full screen");
      await click("SINGLE PLAYER", "full screen");
      await click("BEAM ABOARD", "full screen");
      await wait(() => seen("the bridge for the crew on foot"), "the game to board");
      await pg.waitForTimeout(4000);
      await snap("fullscreen-aboard.png");
      await pg.evaluate(() => document.exitFullscreen().catch(() => {}));
      // In the game, at itch's embed: the game menu by Esc and the mouse.
      mark = lines.length;
      await pg.setViewportSize({ width: 985, height: 455 });
      await follows("in the game at 985 x 455");
      await pg.keyboard.press("Escape");
      await click("Options", "the game menu");
      await snap("985x455-2x-game-options.png");
      await click("Back", "the game menu's Options");
      await click("Resume", "the game menu");
    }
    await ctx.close();
  }
}

let verdict = 0;
try {
  if (fit) {
    await playFit();
    const broken = ["no ship (", "no bridge to walk", "no walk on foot", "pageerror:"].some(seen);
    if (broken) throw new Error("the game came up broken");
    console.log(`smoke: ok, the game follows the page at every size, pixel ratio and full screen, and the mouse lands (${((Date.now() - t0) / 1000).toFixed(0)} s)`);
  } else if (touch) {
    await page.goto(url, { waitUntil: "domcontentloaded", timeout: 180000 });
    await playByTouch();
    const broken = ["no ship (", "no bridge to walk", "no walk on foot", "pageerror:"].some(seen);
    if (broken) throw new Error("the game came up broken");
    console.log(`smoke: ok, played by touch: boarded, walked to a seat, sat, stood (${((Date.now() - t0) / 1000).toFixed(0)} s)`);
  } else {
    await page.goto(url, { waitUntil: "domcontentloaded", timeout: 180000 });
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
    // On foot the round waits for the player to sit (browser-page-fit design 6): walk up to the bridge with the keys,
    // running, until a seat is in reach, and sit with E. The crew walking the same corridor push the body about, so
    // the walk is steered by what the client says, not timed.
    await wait(() => hosted() || broken(), "the game to board");
    await page.waitForTimeout(4000);
    await page.mouse.click(640, 400);
    const prompt = () => lines.filter((l) => l.startsWith("sc-client: prompt: ")).pop() || "";
    const inReach = () => /prompt: .* \[E\] (Sit down|Relieve )/.test(prompt());
    for (let i = 0; !inReach() && !broken(); i++) {
      if (i > 300) throw new Error("the keys never walked the body to a seat");
      await page.keyboard.down("Shift");
      await page.keyboard.down("w");
      await page.waitForTimeout(500);
      await page.keyboard.up("w");
      await page.keyboard.up("Shift");
    }
    const n = lines.length;
    for (let tries = 0; tries < 3 && !lines.slice(n).some((l) => l.includes("sc-client: sat at ")); tries++) {
      await page.keyboard.press("e");
      await page.waitForTimeout(4000);
    }
    // The round waits for START MISSION (start-mission 1): click it where the engine drew it.
    const where = (label) => {
      const re = new RegExp(`^sc-client: ui ${label} at (-?\\d+) (-?\\d+) (\\d+) (\\d+)$`);
      const l = lines.filter((x) => re.test(x)).pop();
      return l ? l.match(re).slice(1, 5).map(Number) : null;
    };
    for (let tries = 0; tries < 6 && !lines.some((l) => l.includes("sc-client: ui clicked START MISSION")) && !broken(); tries++) {
      await page.waitForTimeout(2000);
      const r = where("START MISSION");
      if (r) await page.mouse.click(r[0] + r[2] / 2, r[1] + r[3] / 2);
    }
    if (!lines.some((l) => l.includes("sc-client: ui clicked START MISSION"))) throw new Error("START MISSION was never pressed");
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
