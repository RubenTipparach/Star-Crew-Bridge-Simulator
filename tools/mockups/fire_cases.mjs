#!/usr/bin/env node
/*
 * fire_cases.mjs: damage-control's fire cases (openspec/changes/damage-control design 3, the table) run through the
 * mockups' simulation with fire-spread's cell model, beside the table as it stands.
 *
 * A measurement instrument (CLAUDE.md section 4) for openspec/changes/fire-spread design 4 and 6a: it runs
 * docs/mockups/lib/shipkit.js, firespread.js and shipsystems.js in node on the Tern's layout and data files, with the
 * rooms furnished as the mockups furnish them (data/ships/tern/crew_rooms.json and the prop sets' manifests), seeds each
 * case's fire, steps 900 s and prints the table's columns. Each case is run with the room model alone (the cell model off:
 * the line the table was measured with, a check that this harness is that table's) and with the cell model; the
 * suppression cases twice with the cell model, the extinguisher aimed by a careful crew member (sweeping the burning
 * cells nearest the door) and by a careless one (at the room's centre). It also runs the captain's vent with a crew
 * member who stays (design 6a): seconds to 0 HP and to -50 HP, and the vent preview's time to empty beside the real one.
 * It changes nothing but the report it is asked to write, and in that only the part between its markers, so the
 * written notes around the table stay.
 *
 * Usage: node tools/mockups/fire_cases.mjs [--md openspec/changes/fire-spread/fire-cases.md] [--only name,name]
 */
import fs from "node:fs";
import path from "node:path";
import vm from "node:vm";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const args = process.argv.slice(2);
const opt = (name) => { const i = args.indexOf(name); return i >= 0 ? args.splice(i, 2)[1] : null; };
const mdOut = opt("--md"), only = opt("--only");
const read = (p) => JSON.parse(fs.readFileSync(path.join(ROOT, p), "utf8"));

// The libraries, as the pages carry them (classic scripts on window).
globalThis.window = globalThis;
for (const f of ["shipkit.js", "firespread.js", "shipsystems.js"]) vm.runInThisContext(fs.readFileSync(path.join(ROOT, "docs/mockups/lib", f), "utf8"), { filename: f });
const K = globalThis.ShipKit, FireSpread = globalThis.FireSpread, ShipSystems = globalThis.ShipSystems;
const L = read("data/ships/tern/layout.json"), PW = read("data/ships/tern/power.json"), AT = read("data/ships/tern/atmosphere.json"), DM = read("data/ships/tern/damage.json");
const FURN = read("data/ships/tern/crew_rooms.json").furnishings, HEALTH = read("data/crew/health.json");
const RECS = {};
for (const set of ["machinery", "suite", "engineering"]) Object.assign(RECS, read(`assets/models/${set}/props.json`).props);
const ITEMS = FireSpread.placements(L, AT.fire, FURN, RECS);

// Where each case's fire starts: on a bunk on the quarters' forward wall, on a medbay bed, else the room's centre.
const bunk = ITEMS.find((it) => it.room === "quarters" && it.prop === "bunk" && it.back[0] === 7.3);
const bed = ITEMS.find((it) => it.room === "medbay" && it.prop === "med_bed");
const mid = (poly) => [poly.reduce((s, p) => s + p[0], 0) / poly.length, poly.reduce((s, p) => s + p[1], 0) / poly.length];
const SEED = { quarters: mid(bunk.poly), medbay: mid(bed.poly) };

// The table as it stands (damage-control design 3, rerun on layout v2 2026-10-05).
const TABLE = {
  q_shut: ["113", "118", "262", "238", "96 / 181 / 193", "321", "3.71", "307", "195", "12.6"],
  q_open: ["113", "120", "267", "246", "97 / 184 / 196", "burning at 900", "3.95", "327", "167", "13.0"],
  q_ext30: ["", "", "", "", "", "33", "0.18", "24", "102", "20.8"],
  q_ext60: ["", "", "", "", "", "67", "0.40", "31", "104", "20.6"],
  q_ext90: ["", "", "", "", "", "102", "0.71", "45", "109", "20.3"],
  q_ext120: ["113", "118", "364", "324", "96 / 225 / 244", "407", "3.29", "300", "193", "12.6"],
  q_ext2_120: ["113", "118", "", "", "96 / - / -", "129", "1.10", "65", "116", "19.7"],
  q_ext2_180: ["113", "118", "", "327", "96 / 181 / 193", "415", "2.55", "291", "190", "12.7"],
  q_ext3_180: ["113", "118", "", "", "96 / 181 / -", "192", "2.12", "139", "140", "17.6"],
  q_vent60: ["", "", "", "", "66 / 92 / -", "132", "0.63", "30", "104", "18.7"],
  medbay: ["113", "99", "228", "207", "79 / 158 / 170", "284", "2.87", "308", "195", "12.5"],
  swb_inert: ["", "", "", "", "66 / - / -", "127", "0.58", "53", "112", "12.4"],
  mag: ["113", "151", "325", "300", "124 / 222 / 235", "390", "5.64", "310", "196", "12.7"],
  mag_inert: ["", "", "", "", "63 / - / -", "131", "0.63", "33", "105", "12.5"],
  eng_mist: ["at once", "", "", "", "", "22", "1.20", "22", "102", "20.9"],
  eng_nomist: ["at once", "173", "467", "425", "130 / 265 / 283", "576", "15.97", "310", "196", "13.0"],
  hangar: ["66", "", "", "", "", "87", "1.14", "27", "103", "20.7"],
};
const ext = (n, at) => ({ at, n });
const CASES = [
  { id: "q_shut", name: "Quarters, door shut (default)", room: "quarters" },
  { id: "q_open", name: "Quarters, door held open", room: "quarters", open: "p_quarters" },
  { id: "q_ext30", name: "Quarters, one extinguisher at 30 s", room: "quarters", ext: ext(1, 30) },
  { id: "q_ext60", name: "Quarters, one extinguisher at 60 s", room: "quarters", ext: ext(1, 60) },
  { id: "q_ext90", name: "Quarters, one extinguisher at 90 s", room: "quarters", ext: ext(1, 90) },
  { id: "q_ext120", name: "Quarters, one extinguisher at 120 s", room: "quarters", ext: ext(1, 120) },
  { id: "q_ext2_120", name: "Quarters, two extinguishers at 120 s", room: "quarters", ext: ext(2, 120) },
  { id: "q_ext2_180", name: "Quarters, two extinguishers at 180 s", room: "quarters", ext: ext(2, 180) },
  { id: "q_ext3_180", name: "Quarters, three extinguishers at 180 s", room: "quarters", ext: ext(3, 180) },
  { id: "q_vent60", name: "Quarters, vented at 60 s", room: "quarters", vent: 60 },
  { id: "medbay", name: "Medbay, door shut", room: "medbay" },
  { id: "swb_inert", name: "Forward switchboard, inert gas at 30 s", room: "switchboard", inert: 30 },
  { id: "mag", name: "Magazine, no action", room: "magazine" },
  { id: "mag_inert", name: "Magazine, inert gas at 30 s", room: "magazine", inert: 30 },
  { id: "eng_mist", name: "Engineering, 1 MW seed, water mist automatic", room: "engineering", kw: 1000 },
  { id: "eng_nomist", name: "Engineering, 1 MW seed, mist disabled", room: "engineering", kw: 1000, noMist: true },
  { id: "hangar", name: "Hangar, 300 kW seed, mist automatic", room: "hangar", kw: 300 },
].filter((c) => !only || only.split(",").includes(c.id));

const fmtT = (t) => (t == null ? "" : String(Math.round(t)));
/** One case: { cols (the table's columns), hp0, hp50, preview } with mode "room" (cells off) or "cells", aim for extinguishers. */
function run(c, mode, aim) {
  const sys = ShipSystems.create(L, PW, AT, DM, { cells: mode === "cells", fireItems: ITEMS, health: HEALTH });
  const i = sys.idx[c.room], st = sys.st;
  sys.addCrew("c1", "Crew", c.room);
  if (c.noMist) st.autoMist = false;
  if (c.open) sys.operateDoor(c.open, true, true);
  const at = SEED[c.room] || null;
  sys.ignite(c.room, c.kw || AT.fire.seed_kw, at);
  const r = { t1: null, t60: null, t300: null, smoke: null, imp: null, unc: null, dead: null, out: null, burning: true, pMW: 0, pT: 0, pP: 0, hp0: null, hp50: null, preview: null, ventAt: null };
  if (sys.st.fire[i].hrr >= 1e6) r.t1 = 0;
  const steps = Math.round(900 / sys.dt);
  let was = sys.st.fire[i].hrr;
  for (let k = 0; k < steps; k++) {
    const t = st.t;
    if (c.ext && Math.abs(t - c.ext.at) < sys.dt / 2) sys.useExtinguisher(c.room, c.ext.n, aim || "careful");
    if (c.inert && Math.abs(t - c.inert) < sys.dt / 2) sys.dischargeInert(c.room);
    if (c.vent != null && Math.abs(t - c.vent) < sys.dt / 2) { r.preview = sys.ventPreview(c.room); r.ventAt = t; sys.ventCompartment(c.room); }
    // The vented case closes the dump once the fire is out (the table's 18.7 % oxygen at the end says it was measured so).
    if (r.ventAt != null && !r.stopped && t > r.ventAt && st.fire[i].hrr < AT.fire.out_below_kw * 1000) { r.stopped = t; sys.stopVent(); }
    sys.step();
    const rd = sys.compartmentReadout(i), crew = st.crew[0], T = st.t;
    const hrr = st.fire[i].hrr;
    if (r.t1 == null && hrr >= 1e6) r.t1 = T;
    if (r.t60 == null && rd.t_k >= 333.15) r.t60 = T;
    if (r.t300 == null && rd.t_k >= 573.15) r.t300 = T;
    if (r.smoke == null && rd.smoke_ppm >= 2000) r.smoke = T;
    if (r.imp == null && crew.status !== "ok") r.imp = T;
    if (r.unc == null && (crew.status === "unconscious" || crew.dead)) r.unc = T;
    if (r.dead == null && crew.dead) r.dead = T;
    if (r.hp0 == null && crew.hp <= 0) r.hp0 = T;
    if (r.hp50 == null && crew.hp <= -50) r.hp50 = T;
    const lim = AT.fire.out_below_kw * 1000;
    if (was >= lim && hrr < lim) r.out = T;
    if (hrr >= lim) r.out = null;
    was = hrr;
    r.pMW = Math.max(r.pMW, hrr / 1e6); r.pT = Math.max(r.pT, rd.t_k - 273.15); r.pP = Math.max(r.pP, rd.p_kpa);
  }
  const rd = sys.compartmentReadout(i), x = (100 * sys.n[0][i]) / sys.ntot[i];
  const first = (v) => (v == null ? "-" : fmtT(v));
  const crewCol = r.imp == null ? "" : `${first(r.imp)} / ${first(r.unc)} / ${first(r.dead)}`;
  const startHot = (c.kw || 0) >= 1000;
  r.cols = [startHot ? "at once" : fmtT(r.t1), fmtT(r.t60), fmtT(r.t300), fmtT(r.smoke), crewCol,
    r.out == null ? "burning at 900" : fmtT(r.out), r.pMW.toFixed(2), r.pT.toFixed(0), r.pP.toFixed(0), x.toFixed(1)];
  r.burntCells = mode === "cells" ? sys.fs.rooms[i].state.reduce((s, v) => s + (v === 3 ? 1 : 0), 0) : null;
  r.p_kpa_end = rd.p_kpa;
  return r;
}
/** The vent with a crew member who stays (design 6a): the quarters, no fire, vented at 10 s. */
function ventStay() {
  const sys = ShipSystems.create(L, PW, AT, DM, { cells: true, fireItems: ITEMS, health: HEALTH });
  const i = sys.idx.quarters, st = sys.st;
  sys.addCrew("c1", "Crew", "quarters");
  let hp20 = null, pre = null, t0 = null, dump = null, empty = null, hp0 = null, hp50 = null, dead = null, cold = null, po2_16 = null, po2_6 = null;
  for (let k = 0; k < Math.round(300 / sys.dt); k++) {
    if (Math.abs(st.t - 10) < sys.dt / 2) { pre = sys.ventPreview("quarters"); t0 = st.t; sys.ventCompartment("quarters"); }
    sys.step();
    if (t0 == null) continue;
    const t = st.t - t0, rd = sys.compartmentReadout(i), c = st.crew[0];
    if (dump == null && st.vent && st.vent.phase === "venting") dump = t;
    if (empty == null && rd.p_kpa < AT.fire.extinct_kpa) empty = t;
    if (po2_16 == null && rd.po2_kpa < 16) po2_16 = t;
    if (po2_6 == null && rd.po2_kpa < 6.3) po2_6 = t;
    if (cold == null && rd.t_k < 278.15) cold = t;
    if (hp20 == null && c.hp < HEALTH.incapacitated_below_hp) hp20 = t;
    if (hp0 == null && c.hp <= 0) hp0 = t;
    if (hp50 == null && c.hp <= -50) hp50 = t;
    if (dead == null && c.dead) dead = t;
  }
  return { hp20, pre, dump, empty, hp0, hp50, dead, cold, po2_16, po2_6, endT: sys.compartmentReadout(i).t_k - 273.15 };
}

const COLS = ["1 MW", "Air 60 C", "Air 300 C", "Smoke 2,000 ppm", "Crew impaired / unconscious / dead", "Out", "Peak MW", "Peak air C", "Peak kPa", "Oxygen at end %"];
const SUPP = new Set(["q_ext30", "q_ext60", "q_ext90", "q_ext120", "q_ext2_120", "q_ext2_180", "q_ext3_180"]);
const rows = [];
const t0 = Date.now();
for (const c of CASES) {
  const room = run(c, "room");
  const cells = run(c, "cells", "careful");
  const careless = SUPP.has(c.id) ? run(c, "cells", "careless") : null;
  rows.push({ c, room, cells, careless });
  console.log(c.name);
  console.log("  table       " + TABLE[c.id].join(" | "));
  console.log("  room model  " + room.cols.join(" | "));
  console.log("  cells" + (careless ? " (careful)" : "      ") + " " + cells.cols.join(" | ") + (cells.burntCells != null ? `   [${cells.burntCells} cells burnt out]` : ""));
  if (careless) console.log("  cells (careless) " + careless.cols.join(" | "));
  if (c.vent != null) console.log(`  vent: preview ${cells.preview.total_s.toFixed(1)} s (${cells.preview.warning_s} s warning + ${cells.preview.empty_s.toFixed(1)} s to ${AT.fire.extinct_kpa} kPa); crew ${cells.preview.crew.join(", ")}; 0 HP at ${fmtT(cells.hp0 != null ? cells.hp0 - cells.ventAt : null)} s, -50 HP at ${fmtT(cells.hp50 != null ? cells.hp50 - cells.ventAt : null)} s after the command`);
}
const vs = ventStay();
console.log(`Vent, crew stays (quarters, no fire): preview ${vs.pre.total_s.toFixed(1)} s to ${AT.fire.extinct_kpa} kPa, real ${vs.empty != null ? vs.empty.toFixed(1) : "-"} s; dump opens ${vs.dump.toFixed(1)} s; pO2 under 16 kPa ${vs.po2_16 != null ? vs.po2_16.toFixed(1) : "-"} s, under 6.3 kPa ${vs.po2_6 != null ? vs.po2_6.toFixed(1) : "-"} s; air under 5 C ${vs.cold != null ? vs.cold.toFixed(1) : "never"} (end ${vs.endT.toFixed(1)} C); incapacitated (under ${HEALTH.incapacitated_below_hp} HP) ${vs.hp20 != null ? vs.hp20.toFixed(1) : "-"} s, 0 HP ${vs.hp0 != null ? vs.hp0.toFixed(1) : "-"} s, -50 HP ${vs.hp50 != null ? vs.hp50.toFixed(1) : "-"} s, dead ${vs.dead != null ? vs.dead.toFixed(1) : "-"} s`);
console.log(`(${((Date.now() - t0) / 1000).toFixed(1)} s)`);

// ---------------------------------------------------------------- the report
/** How far a column moved from the table, as a fraction (numbers only). */
function moved(a, b) {
  const x = parseFloat(a), y = parseFloat(b);
  if (!Number.isFinite(x) || !Number.isFinite(y) || x === 0) return null;
  return (y - x) / x;
}
if (mdOut) {
  const day = new Date().toISOString().slice(0, 10);
  const L_ = [];
  L_.push(`Written by \`node tools/mockups/fire_cases.mjs --md ${mdOut}\` on ${day} (a measurement instrument, CLAUDE.md 4; fire-spread`,
    "design 4 and 6a). Each case of `damage-control` design 3 is run three ways through `docs/mockups/lib/shipsystems.js`:",
    "the table as it stands, the room model alone (the cell model off, which should reproduce the table), and the cell",
    "model (`firespread.js`, the rooms furnished as the mockups furnish them). Suppression cases are run with the cell model",
    "twice: a careful crew member sweeping the burning cells nearest the door, and a careless one aiming at the room's centre.",
    "Seeds: the quarters' fires start on the forward-wall bunk at x 7.3 m, the medbay's on a bed, the rest at the room's",
    "centre. Times in seconds from ignition; the 0 HP and -50 HP times from the vent command.", "");
  L_.push("| Case | Run | " + COLS.join(" | ") + " |");
  L_.push("| --- | --- | " + COLS.map(() => "---:").join(" | ") + " |");
  for (const { c, room, cells, careless } of rows) {
    L_.push(`| ${c.name} | table | ${TABLE[c.id].join(" | ")} |`);
    L_.push(`| | room model | ${room.cols.join(" | ")} |`);
    L_.push(`| | cells${careless ? ", careful" : ""} | ${cells.cols.join(" | ")} |`);
    if (careless) L_.push(`| | cells, careless | ${careless.cols.join(" | ")} |`);
  }
  L_.push("");
  L_.push("## Moved by more than 15% (cell model against the table)", "");
  for (const { c, cells } of rows) {
    const out = [];
    COLS.forEach((name, k) => { if (k === 4) return; const m = moved(TABLE[c.id][k], cells.cols[k]); if (m != null && Math.abs(m) > 0.15) out.push(`${name} ${TABLE[c.id][k]} to ${cells.cols[k]} (${m > 0 ? "+" : ""}${(m * 100).toFixed(0)}%)`); });
    if (out.length) L_.push(`- ${c.name}: ${out.join("; ")}.`);
  }
  L_.push("", "## Venting (design 6a)", "");
  const q = rows.find((r) => r.c.id === "q_vent60");
  if (q) L_.push(`- Quarters vented at 60 s with the fire burning: the preview said ${q.cells.preview.total_s.toFixed(1)} s (${q.cells.preview.warning_s} s of warning and ${q.cells.preview.empty_s.toFixed(1)} s to ${AT.fire.extinct_kpa} kPa); the crew member inside was at 0 HP ${fmtT(q.cells.hp0 != null ? q.cells.hp0 - q.cells.ventAt : null)} s and -50 HP ${fmtT(q.cells.hp50 != null ? q.cells.hp50 - q.cells.ventAt : null)} s after the command.`);
  L_.push(`- Quarters vented with no fire and a crew member who stays: the preview said ${vs.pre.total_s.toFixed(1)} s to ${AT.fire.extinct_kpa} kPa and the room took ${vs.empty != null ? vs.empty.toFixed(1) : "-"} s (the dump opens at ${vs.dump.toFixed(1)} s). Oxygen fell past 16 kPa at ${vs.po2_16 != null ? vs.po2_16.toFixed(1) : "-"} s and past 6.3 kPa at ${vs.po2_6 != null ? vs.po2_6.toFixed(1) : "-"} s; the air ${vs.cold != null ? "fell below 5 C at " + vs.cold.toFixed(1) + " s" : "never fell below 5 C"} (${vs.endT.toFixed(1)} C at 300 s). The crew member was incapacitated (under ${HEALTH.incapacitated_below_hp} HP, crew-on-deck 7) at ${vs.hp20 != null ? vs.hp20.toFixed(1) : "-"} s, at 0 HP at ${vs.hp0 != null ? vs.hp0.toFixed(1) : "-"} s, -50 HP at ${vs.hp50 != null ? vs.hp50.toFixed(1) : "-"} s, dead at ${vs.dead != null ? vs.dead.toFixed(1) : "-"} s.`);
  L_.push("");
  const file = path.join(ROOT, mdOut), BEGIN = "<!-- fire_cases.mjs BEGIN -->", END = "<!-- fire_cases.mjs END -->";
  const body = BEGIN + "\n" + L_.join("\n") + "\n" + END;
  const old = fs.existsSync(file) ? fs.readFileSync(file, "utf8") : "";
  const a = old.indexOf(BEGIN), b = old.indexOf(END);
  fs.writeFileSync(file, a >= 0 && b > a ? old.slice(0, a) + body + old.slice(b + END.length) : body + "\n");
  console.log("wrote " + mdOut);
}
