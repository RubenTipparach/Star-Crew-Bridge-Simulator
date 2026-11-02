#!/usr/bin/env node
/*
 * join_test.mjs: the browser joins a crew by its code through the matchmaker (openspec/changes/matchmaker design
 * 10.7, task 1.5).
 *
 * On this machine it starts a matchmaker (sc-matchmaker), a drill server listed with it (sc-server --list), and the
 * web build in headless Chromium. It clicks MULTIPLAYER, types `join:CODE@<the local matchmaker>` into the server
 * field and clicks JOIN, as a player would. The page makes its offer with the browser's own RTCPeerConnection, POSTs it
 * to the matchmaker, and applies the server's answer. It passes when the server says the browser's officer joined,
 * and the page's client says it was welcomed into the crew (`sc-client: round` once a phase arrives), with no page
 * error.
 *
 * Usage: node tools/web/join_test.mjs [build dir, default build/web] [--timeout-s 600] [--shot file.png]
 * Needs: target/release/sc-matchmaker and sc-server (cargo build --release -p sc-matchmaker -p sc-server), the built
 * page (tools/web/build.sh), Playwright and Chromium.
 */
import fs from "node:fs";
import os from "node:os";
import http from "node:http";
import path from "node:path";
import { spawn, execSync } from "node:child_process";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
function loadPlaywright() {
  try { return require("playwright"); } catch (_) { /* the global install */ }
  return require(path.join(execSync("npm root -g").toString().trim(), "playwright"));
}
const { chromium } = loadPlaywright();

const args = process.argv.slice(2);
const opt = (name) => (args.indexOf(name) >= 0 ? args[args.indexOf(name) + 1] : undefined);
const timeoutS = Number(opt("--timeout-s") ?? 600);
const shot = opt("--shot");
const optionArgs = new Set(["--timeout-s", "--shot"]);
const dir = path.resolve(args.find((a, i) => !a.startsWith("--") && !optionArgs.has(String(args[i - 1]))) || "build/web");
const bin = (name) => path.resolve("target/release", name);
for (const f of [path.join(dir, "index.html"), bin("sc-matchmaker"), bin("sc-server")]) {
  if (!fs.existsSync(f)) {
    console.error(`join_test: ${f} is missing`);
    process.exit(2);
  }
}

const children = [];
const logs = { mm: [], server: [] };
function start(name, file, argv) {
  const c = spawn(file, argv, { stdio: ["ignore", "pipe", "pipe"] });
  const take = (b) => b.toString().split("\n").filter(Boolean).forEach((l) => { logs[name].push(l); });
  c.stdout.on("data", take);
  c.stderr.on("data", take);
  children.push(c);
  return c;
}
const stopAll = () => children.forEach((c) => { try { c.kill("SIGTERM"); } catch (_) {} });
process.on("exit", stopAll);

const free = () => new Promise((ok) => { const s = http.createServer(); s.listen(0, "127.0.0.1", () => { const p = s.address().port; s.close(() => ok(p)); }); });
const mmPort = await free();
const svPort = await free();
const mmUrl = `http://127.0.0.1:${mmPort}`;
start("mm", bin("sc-matchmaker"), ["--root", ".", "--listen", `127.0.0.1:${mmPort}`]);
await new Promise((ok) => setTimeout(ok, 1500));
start("server", bin("sc-server"), ["--root", ".", "--listen", `127.0.0.1:${svPort}`, "--host-ip", "127.0.0.1",
  "--matchmaker", mmUrl, "--list", "Browser join test"]);

const t0 = Date.now();
const late = () => (Date.now() - t0) / 1000 > timeoutS;
let code = null;
while (!code) {
  if (late()) throw new Error(`no join code from the server: ${logs.server.slice(-5).join(" | ")}`);
  const l = logs.server.find((x) => / join code ([0-9A-Z]{6})/.test(x));
  if (l) code = l.match(/ join code ([0-9A-Z]{6})/)[1];
  await new Promise((ok) => setTimeout(ok, 250));
}
console.log(`join_test: the server is listed with code ${code} at ${mmUrl}`);

const TYPES = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".gz": "application/gzip" };
const web = http.createServer((req, res) => {
  const rel = decodeURIComponent(new URL(req.url, "http://x").pathname).replace(/^\/+/, "") || "index.html";
  const file = path.join(dir, rel);
  if (!file.startsWith(dir) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) return res.writeHead(404).end();
  res.writeHead(200, { "content-type": TYPES[path.extname(file)] || "application/octet-stream" });
  fs.createReadStream(file).pipe(res);
});
await new Promise((ok) => web.listen(0, "127.0.0.1", ok));
const url = `http://127.0.0.1:${web.address().port}/index.html`;

const launch = { args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"] };
const browser = await chromium.launch(launch).catch(() => chromium.launch({ ...launch, executablePath: "/opt/pw-browsers/chromium" }));
const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
const lines = [];
page.on("console", (m) => lines.push(m.text()));
page.on("pageerror", (e) => lines.push("pageerror: " + e.message));

const wait = async (pred, what) => {
  while (!pred()) {
    if (late()) throw new Error(`timed out after ${timeoutS} s waiting for ${what}`);
    await page.waitForTimeout(250);
  }
};
const where = (label, since = 0) => {
  const re = new RegExp(`^sc-client: ui ${label} at (-?\\d+) (-?\\d+) (\\d+) (\\d+)$`);
  const l = lines.slice(since).filter((x) => re.test(x)).pop();
  return l ? l.match(re).slice(1, 5).map(Number) : null;
};
/** Click `label` at the middle of where the engine last said it drew it (since line `since`). */
const click = async (label, since = 0) => {
  await wait(() => where(label, since), `${label} to be drawn`);
  await page.waitForTimeout(600);
  const [x, y, w, h] = where(label, since);
  await page.mouse.click(x + w / 2, y + h / 2);
};
/** Type into the server field: click it, clear it, type. */
const typeServer = async (text, since) => {
  await click("SERVER", since);
  // The field takes keys from the frame after the click: wait a few frames, or the first letters are lost.
  await page.waitForTimeout(1000);
  await page.keyboard.press("Control+A");
  await page.keyboard.press("Backspace");
  await page.keyboard.type(text, { delay: 60 });
  await page.waitForTimeout(500);
};

let failed = null;
try {
  await page.goto(url);
  await wait(() => where("MULTIPLAYER"), "the front menu");
  let mark = lines.length;
  await click("MULTIPLAYER");
  await typeServer(`join:${code}@${mmUrl}`, mark);
  mark = lines.length;
  await click("JOIN", 0);
  await wait(() => logs.server.some((l) => / joined from /.test(l) && !l.includes("(bot)")), "the server to welcome the browser's officer");
  await wait(() => lines.some((l) => l.startsWith("sc-client: round ")), "the page to hear the round's phase");
  console.log(`join_test: ${logs.server.find((l) => / joined from /.test(l))}`);
  console.log(`join_test: ${lines.find((l) => l.startsWith("sc-client: round "))}`);
  if (shot) await page.screenshot({ path: shot });
  const errors = lines.filter((l) => l.startsWith("pageerror"));
  if (errors.length) throw new Error(errors.join("\n"));
} catch (e) {
  failed = e;
  try { await page.screenshot({ path: shot || path.join(os.tmpdir(), "join_test_failed.png") }); } catch (_) {}
}
await browser.close();
web.close();
stopAll();
if (failed) {
  console.error(`join_test: FAILED: ${failed.message}`);
  console.error("--- page\n" + lines.filter((l) => !/ ui .* at /.test(l)).slice(-40).join("\n"));
  console.error("--- server\n" + logs.server.slice(-20).join("\n"));
  console.error("--- matchmaker\n" + logs.mm.slice(-10).join("\n"));
  process.exit(1);
}
console.log("join_test: passed: the browser joined the crew by its code through the matchmaker");
