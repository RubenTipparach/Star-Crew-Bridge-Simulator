#!/usr/bin/env node
/*
 * zfight.mjs: find z-fighting in a three.js mockup, headless (CLAUDE.md section 8: "No z-fighting, ever").
 *
 * It loads each page, takes every triangle its scene draws (window.MOCKUP_SCENE, in world space, with the role its
 * geometry gives it), and reports pairs of triangles that lie on one plane (each centre within 5 mm of the other's
 * plane), face the same way and overlap by more than 1 cm^2: the surfaces a depth buffer cannot order. Faces pressed
 * back to back face opposite ways and are fine; a faint x-ray overlay (opacity under 0.2) blends rather than fights
 * and is left out, as is an overlay that does not write depth (markers drawn in order), and a plan's own marks
 * overlapping each other in one colour (one mesh, no roles).
 *
 * A measurement instrument and a check (CLAUDE.md section 4): it changes nothing. Rendering is software
 * (SwiftShader); only geometry is read.
 *
 * A flat 2D page (the consoles, CLAUDE.md section 11) sets window.MOCKUP_FLAT and has no geometry to fight: it is
 * reported and skipped.
 *
 * Usage: node tools/mockups/zfight.mjs [page.html ...] [--setup "js run in the page first"] [--max-m2 0.1]
 *        (default: docs/mockups/deck-plan.html). Exits 1 when the fighting area passes --max-m2.
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
const opt = (name, dflt) => { const i = args.indexOf(name); return i >= 0 ? args.splice(i, 2)[1] : dflt; };
const setup = opt("--setup", null), maxM2 = Number(opt("--max-m2", "0.1"));
const pages = args.length ? args.map((p) => path.resolve(p)) : [path.join(ROOT, "docs/mockups/deck-plan.html")];

const PLANE_M = 0.005, MIN_OVERLAP_M2 = 1e-4, CELL_M = 0.5;

/** The page's triangles: [{ mesh, role, p: [9 numbers] }]. */
async function trianglesOf(browser, file) {
  const page = await browser.newPage({ viewport: { width: 800, height: 500 } });
  await page.route("https://cdn.jsdelivr.net/npm/**", (route) => {
    const rel = route.request().url().replace("https://cdn.jsdelivr.net/npm/", "").split("?")[0];
    route.fulfill({ body: fs.readFileSync(path.join(CACHE, rel)), contentType: "application/javascript" });
  });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("file://" + file);
  for (let i = 0; i < 100; i++) { if (await page.evaluate(() => window.MOCKUP_READY)) break; await page.waitForTimeout(3000); }
  if (errors.length) throw new Error(`${path.basename(file)}: ${errors[0]}`);
  if (await page.evaluate(() => !!window.MOCKUP_FLAT)) { await page.close(); return null; }
  if (!(await page.evaluate(() => !!window.MOCKUP_SCENE))) throw new Error(`${path.basename(file)} does not set window.MOCKUP_SCENE`);
  if (setup) await page.evaluate(setup);
  const meshes = await page.evaluate(() => {
    const out = [];
    window.MOCKUP_SCENE.updateMatrixWorld(true);
    window.MOCKUP_SCENE.traverse((o) => {
      if (!o.isMesh || !o.geometry || !o.geometry.attributes.position) return;
      for (let v = o; v; v = v.parent) if (!v.visible) return;
      const mat = Array.isArray(o.material) ? o.material[0] : o.material;
      if (mat.transparent && mat.opacity < 0.2) return;
      if (mat.depthWrite === false) return;   // an overlay drawn in order (markers, glyphs): it cannot fight
      const g = o.geometry.index ? o.geometry.toNonIndexed() : o.geometry, a = g.attributes.position.array;
      const roles = g.userData && g.userData.roles ? Object.entries(g.userData.roles).map(([r, [k, n]]) => [r, k, n]) : [];
      // An instanced mesh: each drawn instance, at its own matrix (its count may be less than its capacity).
      const mats = [];
      if (o.isInstancedMesh) for (let i = 0; i < o.count; i++) { const im = new o.matrixWorld.constructor(); o.getMatrixAt(i, im); mats.push(im.premultiply(o.matrixWorld).elements); }
      else mats.push(o.matrixWorld.elements);
      mats.forEach((m, k) => {
        const p = new Array(a.length);
        for (let i = 0; i < a.length; i += 3) {
          const x = a[i], y = a[i + 1], z = a[i + 2];
          p[i] = m[0] * x + m[4] * y + m[8] * z + m[12]; p[i + 1] = m[1] * x + m[5] * y + m[9] * z + m[13]; p[i + 2] = m[2] * x + m[6] * y + m[10] * z + m[14];
        }
        out.push({ name: `${(o.parent && o.parent.name) || "scene"}/${o.name || "mesh" + out.length}${mats.length > 1 ? "#" + k : ""}`, roles, p });
      });
    });
    return out;
  });
  await page.close();
  const tris = [];
  meshes.forEach((m, mi) => {
    for (let t = 0; t < m.p.length / 9; t++) {
      const v = 3 * t, r = m.roles.find(([, k, n]) => k <= v && v < k + n);
      tris.push({ mi, mesh: m.name, role: r ? r[0] : "unnamed", p: m.p.slice(t * 9, t * 9 + 9) });
    }
  });
  return tris;
}

const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const unit = (a) => { const l = Math.hypot(...a); return [a[0] / l, a[1] / l, a[2] / l]; };

/** Keep the part of polygon poly on the left of edge a->b (2D). */
function clip(poly, a, b) {
  const out = [];
  for (let i = 0; i < poly.length; i++) {
    const p = poly[i], q = poly[(i + 1) % poly.length];
    const sp = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]), sq = (b[0] - a[0]) * (q[1] - a[1]) - (b[1] - a[1]) * (q[0] - a[0]);
    if (sp >= 0) out.push(p);
    if ((sp >= 0) !== (sq >= 0)) { const t = sp / (sp - sq); out.push([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]); }
  }
  return out;
}
const area2 = (poly) => Math.abs(poly.reduce((s, p, i) => { const q = poly[(i + 1) % poly.length]; return s + p[0] * q[1] - q[0] * p[1]; }, 0)) / 2;
const ccw = (t) => ((t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[1][1] - t[0][1]) * (t[2][0] - t[0][0]) > 0 ? t : [t[0], t[2], t[1]]);

/** Pairs of triangles that fight, summed by the two (mesh, role) sides: [{ a, b, pairs, m2, at }]. */
function fights(tris) {
  const info = tris.map((t) => {
    const P = [t.p.slice(0, 3), t.p.slice(3, 6), t.p.slice(6, 9)], c = cross(sub(P[1], P[0]), sub(P[2], P[0])), a = Math.hypot(...c);
    if (a < 1e-9) return null;
    const n = [c[0] / a, c[1] / a, c[2] / a], ctr = [(P[0][0] + P[1][0] + P[2][0]) / 3, (P[0][1] + P[1][1] + P[2][1]) / 3, (P[0][2] + P[1][2] + P[2][2]) / 3];
    return { P, n, ctr, area: a / 2 };
  });
  const buckets = new Map();
  info.forEach((f, i) => {
    if (!f || f.area < 1e-5) return;
    const nk = f.n.map((v) => Math.round(v * 40)), nr = unit(nk);
    const key = nk.join(",") + "|" + Math.floor(dot(nr, f.ctr) / PLANE_M);
    if (!buckets.has(key)) buckets.set(key, []);
    buckets.get(key).push(i);
  });
  const found = new Map();
  for (const [key, list] of buckets) {
    const [nk, dk] = key.split("|");
    const cand = list.concat(buckets.get(nk + "|" + (Number(dk) + 1)) || []);
    if (cand.length < 2) continue;
    const n = info[cand[0]].n, u = unit(cross(n, Math.abs(n[1]) < 0.9 ? [0, 1, 0] : [1, 0, 0])), w = cross(n, u);
    const P2 = new Map(cand.map((i) => [i, info[i].P.map((p) => [dot(p, u), dot(p, w)])]));
    const grid = new Map();
    for (const i of cand) {
      const xs = P2.get(i).map((p) => p[0]), ys = P2.get(i).map((p) => p[1]);
      for (let gx = Math.floor(Math.min(...xs) / CELL_M); gx <= Math.floor(Math.max(...xs) / CELL_M); gx++)
        for (let gy = Math.floor(Math.min(...ys) / CELL_M); gy <= Math.floor(Math.max(...ys) / CELL_M); gy++) {
          const k = gx + "," + gy; if (!grid.has(k)) grid.set(k, []); grid.get(k).push(i);
        }
    }
    const seen = new Set();
    for (const cell of grid.values()) for (let a = 0; a < cell.length; a++) for (let b = a + 1; b < cell.length; b++) {
      const i = cell[a], j = cell[b], k = i + ":" + j;
      if (seen.has(k)) continue;
      seen.add(k);
      const fi = info[i], fj = info[j];
      if (dot(fi.n, fj.n) < 0.999) continue;
      // Each centre's distance from the other's plane: plane offsets far from the origin would turn a tiny difference
      // in normal into centimetres.
      if (Math.max(Math.abs(dot(fi.n, sub(fj.ctr, fi.P[0]))), Math.abs(dot(fj.n, sub(fi.ctr, fj.P[0])))) > PLANE_M) continue;
      if (tris[i].mi === tris[j].mi && tris[i].role === "unnamed" && tris[j].role === "unnamed") continue;
      const A = ccw(P2.get(i)), B = ccw(P2.get(j));
      let poly = A;
      for (let e = 0; e < 3 && poly.length; e++) poly = clip(poly, B[e], B[(e + 1) % 3]);
      if (poly.length < 3) continue;
      const m2 = area2(poly);
      if (m2 < MIN_OVERLAP_M2) continue;
      const sides = [`${tris[i].mesh} ${tris[i].role}`, `${tris[j].mesh} ${tris[j].role}`].sort(), key2 = sides.join(" | ");
      const f = found.get(key2) || { a: sides[0], b: sides[1], pairs: 0, m2: 0, at: fi.ctr.map((v) => Math.round(v * 100) / 100) };
      f.pairs++; f.m2 += m2; found.set(key2, f);
    }
  }
  return [...found.values()].sort((x, y) => y.m2 - x.m2);
}

const browser = await chromium.launch({ executablePath: "/opt/pw-browsers/chromium", args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"] })
  .catch(() => chromium.launch({ args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader"] }));
let failed = false;
for (const file of pages) {
  const tris = await trianglesOf(browser, file);
  if (!tris) { console.log(`${path.relative(ROOT, file)}: a flat 2D page, no geometry`); continue; }
  const rows = fights(tris), total = rows.reduce((s, r) => s + r.m2, 0);
  console.log(`${path.relative(ROOT, file)}: ${tris.length} triangles, ${rows.length} fighting pairs of surfaces, ${total.toFixed(3)} m2`);
  for (const r of rows) console.log(`  ${r.m2.toFixed(3)} m2 ${String(r.pairs).padStart(4)} pairs  ${r.a}  vs  ${r.b}  at ${JSON.stringify(r.at)}`);
  if (total > maxM2) { failed = true; console.log(`  FAIL: more than ${maxM2} m2`); }
}
await browser.close();
process.exit(failed ? 1 : 0);
