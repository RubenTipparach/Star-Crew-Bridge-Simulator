#!/usr/bin/env node
/*
 * console_capture.mjs: one console of a console mockup, its #console element only (1920 x 1080), after a named shot's
 * setup, for pages that show a console as a picture (tools/mockups/concord_ui.py).
 *
 * Usage: node tools/mockups/console_capture.mjs <page.html> <shot name> <out.png>
 */
import { createRequire } from "node:module"; import { execSync } from "node:child_process"; import path from "node:path"; import { pathToFileURL } from "node:url";
const require = createRequire(import.meta.url);
let pw; try { pw = require("playwright"); } catch (_) { pw = require(path.join(execSync("npm root -g").toString().trim(), "playwright")); }
const [page_, shotName, out] = process.argv.slice(2);
const b = await pw.chromium.launch({ args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"] }).catch(() => pw.chromium.launch({ executablePath: "/opt/pw-browsers/chromium" }));
const p = await b.newPage({ viewport: { width: 1920, height: 1200 }, deviceScaleFactor: 1 });
await p.goto(pathToFileURL(path.resolve(page_)).href);
await p.waitForFunction(() => window.MOCKUP_READY === true, null, { timeout: 120000 });
await p.evaluate(() => { const s = window.MOCKUP_SHOTS.find((x) => x.name === shotName); s && s.setup(); });
await p.waitForTimeout(2500);
await p.locator("#console").screenshot({ path: out });
await b.close();
