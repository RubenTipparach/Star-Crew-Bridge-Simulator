/*
 * repairs/boxmegger.js: an insulation test on the forward switchboard or the emergency bus (openspec/changes/
 * repair-minigames, design 2a; power-routing 1).
 *
 * The bus open: its isolator and the LIVE lamp at the top, its outgoing ways below (each a bushing with a lug), and the
 * insulation tester on the right, its probe on a lead. Isolate the bus (drag the isolator down, or Space) and see the
 * lamp go dark. Put the probe on a way's lug (drag it, or left and right) and hold TEST (the button, or hold Space)
 * until the needle settles: the reading is taken there, a good way high, a faulty one low. Scrub the carbon tracking off a low way's bushing
 * (drag back and forth across it, a few strokes a track, or hold Space), then test it again. When every way reads high, close the bus (drag the isolator up,
 * or Space). Testing before the bus is dead is a fumble: a shock, 5 HP. Closing up on a low or untested way is a
 * fumble: a flash, 5 HP. Each level has more ways (5 to 8), two faults from level 3, a needle that takes longer to
 * settle and more tracking. A disabled bus's first round fits a new bushing: drag it from the crate onto the empty way.
 */
// The guide's pictures (repair-minigames 6g): this game's own parts, in the card's 100 px box about the origin.
/** Isolate: the isolator's handle comes down to OPEN and the LIVE lamp goes dark. */
function boxmeggerPicIsolate(g, t, K) {
  const D = K.draw, u = Math.min(1, ((t * 0.55) % 1) * 1.4), y = -22 + 44 * u;
  D.round(g, -44, -46, 46, 92, 7); g.fillStyle = "#0b1017"; g.fill(); g.lineWidth = 2; g.strokeStyle = "#3a4656"; g.stroke();
  D.round(g, -38, y - 11, 34, 22, 5); g.fillStyle = u >= 1 ? "#c8c2b4" : K.C.danger; g.fill();
  D.disc(g, 26, -18, 14, u >= 1 ? "#2a2f36" : K.C.danger); D.ring(g, 26, -18, 14, "#566273", 2);
  D.text(g, u >= 1 ? "DEAD" : "LIVE", 26, 10, 12, u >= 1 ? K.C.dim : K.C.danger, "center", 700);
  D.finger(g, -21, y);
}
/** Test: the red probe goes onto a way's copper lug, the finger holds TEST, the needle climbs and settles. */
function boxmeggerPicTest(g, t, K) {
  const D = K.draw, u = (t * 0.4) % 1, m = Math.min(1, u * 3), k = u > 0.35 ? Math.min(0.85, (u - 0.35) * 2.2) : 0;
  g.fillStyle = "#3a2a1a"; g.fillRect(-36, -50, 6, 22); D.disc(g, -33, -26, 7, K.C.copper);
  for (let j = 0; j < 3; j++) { D.round(g, -46, -16 + j * 15, 26, 12, 6); g.fillStyle = "#d8cfb8"; g.fill(); }
  const px = -33 + 30 * (1 - m), py = -26 - 4 + 20 * (1 - m);
  D.round(g, px - 4, py - 22, 8, 18, 3); g.fillStyle = K.C.danger; g.fill(); D.line(g, [[px, py - 4], [px, py + 2]], "#e8eef6", 2);
  const A0 = (-210 * Math.PI) / 180, A1 = (30 * Math.PI) / 180, ang = (q) => A0 + (A1 - A0) * q;
  g.beginPath(); g.arc(20, -14, 24, A0, A1); g.strokeStyle = "#1d2738"; g.lineWidth = 7; g.stroke();
  g.beginPath(); g.arc(20, -14, 24, ang(0.75), A1); g.strokeStyle = K.C.ok; g.lineWidth = 7; g.stroke();
  D.line(g, [[20, -14], [20 + Math.cos(ang(k)) * 20, -14 + Math.sin(ang(k)) * 20]], K.C.fg, 3);
  D.disc(g, 22, 34, 15, k > 0 ? "#7a1a22" : K.C.danger); D.text(g, "TEST", 22, 34, 9, "#fff", "center", 700);
  if (u > 0.33) D.finger(g, 22, 34);
}
/** Scrub: the finger rubs back and forth over a LOW way's bushing and the black tracking fades. */
function boxmeggerPicScrub(g, t, K) {
  const D = K.draw, u = (t * 0.35) % 1, fade = Math.min(1, u * 1.3);
  for (let j = 0; j < 6; j++) { D.round(g, -18 + (j % 2) * 3, -46 + j * 15, 36 - (j % 2) * 6, 12, 6); g.fillStyle = "#d8cfb8"; g.fill(); }
  g.globalAlpha = 1 - 0.85 * fade;
  D.line(g, [[-6, -34], [2, -24], [-4, -12], [4, 0], [0, 12]], "#120c06", 4);
  g.globalAlpha = 1;
  D.round(g, 22, 34, 30, 16, 4); g.fillStyle = K.C.danger; g.fill(); D.text(g, "LOW", 37, 42, 10, "#06090e", "center", 700);
  D.finger(g, 14 * Math.sin(t * 9), -26 + 34 * ((t * 0.7) % 1));
}
/** Close up: every way reads OK, the isolator's handle goes back up. */
function boxmeggerPicClose(g, t, K) {
  const D = K.draw, u = Math.min(1, ((t * 0.55) % 1) * 1.4), y = 22 - 44 * u;
  D.round(g, -44, -46, 46, 92, 7); g.fillStyle = "#0b1017"; g.fill(); g.lineWidth = 2; g.strokeStyle = "#3a4656"; g.stroke();
  D.round(g, -38, y - 11, 34, 22, 5); g.fillStyle = u >= 1 ? K.C.danger : "#c8c2b4"; g.fill();
  for (let j = 0; j < 3; j++) { D.round(g, 12, -38 + j * 26, 34, 18, 4); g.fillStyle = K.C.ok; g.fill(); D.text(g, "OK", 29, -29 + j * 26, 10, "#06090e", "center", 700); }
  D.finger(g, -21, y);
}

RepairKit.register({
  id: "boxmegger",
  title: "Switchboard insulation",
  place: "The forward switchboard and the emergency bus",
  group: "Electrical",
  hazard: "Testing a live bus: a shock, 5 HP",
  guide: {
    steps: [
      { icon: "screw", text: "Unscrew the cover", cover: true },
      { pic: boxmeggerPicIsolate, text: "Drag the isolator down: DEAD" },
      { pic: boxmeggerPicTest, text: "Red probe on a lug, hold TEST" },
      { pic: boxmeggerPicScrub, text: "LOW: rub the black marks off, test again" },
      { pic: boxmeggerPicClose, text: "All OK: isolator back up" },
    ],
    mistake: "TEST while LIVE: a shock, 5 HP. Closing with a LOW way: a flash, 5 HP",
    now: (q) => ({ isolate: 1, test: q.results.some((r) => r === "low") ? 3 : 2, close: 4 })[q.phase] ?? -1,
  },
  down: "The bow's switchboard section or the emergency bus is out (power-routing)",
  panel: { x: 100, y: 96, w: 640, h: 604, screws: 6 },
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const BOX = { x: 100, y: 96, w: 640, h: 604 };
    const ISO = { x: 170, y0: 170, y1: 270, w: 120 };           // the isolator's handle: up (closed) to down (open)
    const LAMP = { x: 360, y: 215, r: 26 };
    const WAYS = { x0: 160, x1: 690, lug: 404, top: 420, h: 190 };
    const METER = { x: 1010, y: 300, r: 140 };
    const TEST = { x: 1150, y: 500, r: 54 };
    const PARK = { x: 900, y: 520 };
    const CRATE = { x: 800, y: 590, w: 260, h: 104 };
    const PORT = { x: 812, y: 452 };                              // where the probe's lead leaves the tester
    const TRACK_R = 20;                                           // how near a track the scrubbing pointer must pass, px
    const A0 = (-210 * Math.PI) / 180, A1 = (30 * Math.PI) / 180;
    let phase, part, bush, n, ways, iso, isoY, grab, dead, deadT, probe, probing, sel, needle, testT, scrubbing, flash, spark, time, tau;
    let scrubPx, scrubS, last, readT = 0;

    const wayX = (i) => WAYS.x0 + 30 + (i * (WAYS.x1 - WAYS.x0 - 60)) / Math.max(1, n - 1);
    // The dial is logarithmic: 0.1 MOhm at its start, 1000 at its end.
    const dialK = (mo) => Math.max(0, Math.min(1, (Math.log10(Math.max(0.1, mo)) + 1) / 4));
    const ang = (k) => A0 + (A1 - A0) * k;
    const good = (w) => w.tracks.every((tr) => tr.clean);
    // Wear a track down: `k` of a full scrub; it is gone at 1.
    const wear = (w, tr, k) => { tr.wear = Math.min(1, tr.wear + k); if (tr.wear >= 1) tr.clean = true; if (good(w)) w.result = null; };

    return {
      step(index, isPart) {
        const r = api.rand(), level = api.level;
        part = isPart;
        n = Math.min(8, 4 + level);
        const nf = level >= 3 ? 2 : 1;
        const faults = new Set();
        while (faults.size < nf) faults.add(Math.floor(r() * n));
        ways = [...Array(n)].map((_, i) => {
          const tracks = [];
          if (faults.has(i)) {
            // Carbon tracking: wandering lines down the bushing, more and longer each level.
            for (let k = 0; k < 2 + level; k++) {
              const pts = []; let y = WAYS.top + 16 + r() * 130, x = -18 + r() * 36;
              for (let s = 0; s < 5 + Math.floor(level / 2); s++) { pts.push([x, y]); x = Math.max(-21, Math.min(21, x + (r() - 0.5) * 20)); y += 7 + r() * 9; }
              tracks.push({ pts, clean: false, wear: 0 });
            }
          }
          return { tracks, result: null, ok: !faults.has(i), high: 300 + r() * 1500 };
        });
        bush = { x: CRATE.x + 130, y: CRATE.y + 66, held: false, set: !isPart, way: Math.floor(r() * n) };
        phase = isPart ? "part" : "isolate";
        iso = true; isoY = ISO.y0; grab = false; dead = false; deadT = 0;
        probe = { x: PARK.x, y: PARK.y, way: -1, held: false };
        probing = false; sel = 0; needle = 0; testT = 0; scrubbing = false; flash = 0; spark = null; time = 0;
        tau = 0.35 + 0.1 * level;
        // A track takes a few strokes (pointer travel near it) or a held Space; both longer each level.
        scrubPx = 120 + 25 * level; scrubS = 0.5 + 0.1 * level; last = null;
      },
      peek() { return { phase, n, dead, probeWay: probe.way, probe: { ...probe }, needle, scrubbing, results: ways.map((w) => w.result), wayX, TEST, iso, ways: ways.map((w) => ({ ok: w.ok, tracks: w.tracks.length, left: w.tracks.filter((tr) => !tr.clean).length })) }; },
      update(dt, input) {
        time += dt;
        flash = Math.max(0, flash - dt * 1.6);
        readT = Math.max(0, readT - dt);
        if (spark) { spark.t -= dt; if (spark.t <= 0) spark = null; }
        if (!iso && !dead) { deadT += dt; if (deadT > 0.4) dead = true; }
        if (phase === "part") {
          if (input.pressed && Math.hypot(input.x - bush.x, input.y - bush.y) < 60) bush.held = true;
          if (bush.held && input.down) { bush.x = input.x; bush.y = input.y; }
          if (input.actionPressed) bush.set = true;
          if (bush.held && input.released) { bush.held = false; if (Math.abs(bush.x - wayX(bush.way)) < 50 && Math.abs(bush.y - (WAYS.top + WAYS.h / 2)) < 110) bush.set = true; }
          if (bush.set) { phase = "isolate"; api.stepDone(); }
          return;
        }
        // The isolator: drag its handle (down opens the bus, up closes it), or Space.
        const hx = ISO.x + ISO.w / 2;
        if ((phase === "isolate" || phase === "close") && input.pressed && Math.abs(input.x - hx) < 70 && Math.abs(input.y - isoY) < 46) grab = true;
        if (grab && input.down) isoY = Math.max(ISO.y0, Math.min(ISO.y1, input.y));
        if (!input.down) grab = false;
        if (phase === "isolate") {
          if (input.actionPressed) isoY = ISO.y1;
          if (isoY >= ISO.y1 - 2) { isoY = ISO.y1; iso = false; grab = false; phase = "test"; }
          return;
        }
        if (phase === "close") {
          if (input.actionPressed) isoY = ISO.y0;
          if (isoY <= ISO.y0 + 2) {
            isoY = ISO.y0; iso = true; dead = false; deadT = 0; grab = false;
            if (ways.every((w) => w.result === "high")) { phase = "done"; api.stepDone(); }
            else {
              // Closed on a low or untested way: a flash, and the bus is opened again.
              flash = 1; spark = { x: LAMP.x, y: LAMP.y, t: 0.5 }; isoY = ISO.y1; iso = false; phase = "test";
              api.fumble("Closed on a low way: a flash, 5 HP");
            }
          }
          return;
        }
        if (phase !== "test") return;
        // The probe: dragged onto a lug, or moved by the arrows.
        if (input.hit.has("ArrowRight") || input.hit.has("KeyD")) { sel = (sel + 1) % n; probe.way = sel; }
        if (input.hit.has("ArrowLeft") || input.hit.has("KeyA")) { sel = (sel + n - 1) % n; probe.way = sel; }
        const onTest = (x, y) => Math.hypot(x - TEST.x, y - TEST.y) < TEST.r + 10;
        if (input.pressed && !onTest(input.x, input.y) && Math.hypot(input.x - probe.x, input.y - (probe.y - 24)) < 44) probe.held = true;
        if (probe.held && input.down) { probe.x = input.x; probe.y = input.y; probe.way = -1; }
        if (probe.held && input.released) {
          probe.held = false;
          const i = KIT.nearest(ways.map((w, k) => [wayX(k), WAYS.lug]), input.x, input.y, 50);
          probe.way = i; if (i >= 0) sel = i; else { probe.x = PARK.x; probe.y = PARK.y; }
        }
        if (probe.way >= 0) { probe.x = wayX(probe.way); probe.y = WAYS.lug; }
        // Scrubbing: drag across a bushing's tracking, or hold Space on a way that read low.
        scrubbing = false;
        const w = probe.way >= 0 ? ways[probe.way] : null;
        if (input.pressed) last = null;
        if (input.down && !probe.held && !onTest(input.x, input.y)) {
          // Every sample the pointer passed this frame wears the tracks it passed near by the distance it moved.
          for (const [x, y] of input.path.length ? input.path : [[input.x, input.y]]) {
            const d = last ? Math.hypot(x - last[0], y - last[1]) : 0;
            last = [x, y];
            const i = KIT.nearest(ways.map((_, k) => [wayX(k), WAYS.top + WAYS.h / 2]), x, y, 44);
            if (i < 0 || ways[i].result !== "low" || good(ways[i])) continue;
            scrubbing = true;
            for (const tr of ways[i].tracks) if (!tr.clean && tr.pts.some(([px, py]) => Math.hypot(wayX(i) + px - x, py - y) < TRACK_R)) wear(ways[i], tr, d / scrubPx);
          }
        } else last = null;
        if (input.action && w && w.result === "low" && !good(w)) {
          scrubbing = true;
          wear(w, w.tracks.find((x) => !x.clean), dt / scrubS);
          return;
        }
        // TEST: held, the needle climbs toward the way's reading; let go once it has settled.
        const holding = (input.down && onTest(input.x, input.y)) || (input.action && w && !(w.result === "low" && !good(w)));
        if (holding && !probing) {
          if (!dead) {
            spark = { x: probe.x, y: probe.y, t: 0.5 }; flash = 1;
            api.fumble("The bus is live: a shock, 5 HP");
            return;
          }
          if (probe.way < 0) return;
          probing = true; testT = 0; needle = 0;
        }
        if (probing) {
          const target = dialK(good(w) ? w.high : 0.2 + 0.4 * Math.abs(Math.sin(probe.way + 1)));
          testT += dt;
          needle += (target - needle) * (1 - Math.exp(-dt / tau)) + Math.sin(time * 23) * 0.004 * Math.exp(-testT);
          // The reading is taken the moment the needle settles, still held (owner, 2026-10-12: "cant complete this
          // puzzle": a release a moment early read nothing and said nothing).
          if (Math.abs(needle - target) < 0.03 && w.result !== (good(w) ? "high" : "low")) { w.result = good(w) ? "high" : "low"; readT = 0.8; }
          if (!holding) { probing = false; needle = 0; }
        }
        if (ways.every((x) => x.result === "high")) phase = "close";
      },
      draw(g, t, dt, input) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        D.panel(g, BOX.x, BOX.y, BOX.w, BOX.h, 10, "#1b2433", "#3a4656");
        D.round(g, BOX.x + 24, BOX.y + 24, BOX.w - 48, BOX.h - 48, 6); g.fillStyle = "#121821"; g.fill();
        // The bus: a copper bar the ways hang from.
        g.fillStyle = "#2b2014"; g.fillRect(WAYS.x0, 330, WAYS.x1 - WAYS.x0, 18); g.fillStyle = C.copper; g.fillRect(WAYS.x0, 330, WAYS.x1 - WAYS.x0, 6);
        // The isolator: its slot and handle, OPEN at the bottom.
        D.round(g, ISO.x, ISO.y0 - 40, ISO.w, ISO.y1 - ISO.y0 + 80, 8); g.fillStyle = "#0b1017"; g.fill(); g.lineWidth = 2; g.strokeStyle = "#3a4656"; g.stroke();
        D.round(g, ISO.x + 16, isoY - 24, ISO.w - 32, 48, 6); g.fillStyle = iso ? C.danger : "#c8c2b4"; g.fill();
        g.fillStyle = "#10151c"; g.fillRect(ISO.x + 28, isoY - 4, ISO.w - 56, 8);
        D.text(g, "CLOSED", ISO.x + ISO.w + 12, ISO.y0, 14, C.dim, "left", 700);
        D.text(g, "OPEN", ISO.x + ISO.w + 12, ISO.y1, 14, C.dim, "left", 700);
        // The LIVE lamp: red while there is voltage, dark once dead.
        D.disc(g, LAMP.x + 70, LAMP.y, LAMP.r, dead ? "#2a2f36" : C.danger);
        D.ring(g, LAMP.x + 70, LAMP.y, LAMP.r, "#566273", 3);
        D.text(g, dead ? "DEAD" : "LIVE", LAMP.x + 110, LAMP.y, 24, dead ? C.dim : C.danger, "left", 700);
        // The ways: bushings with their lugs; tracking on a faulty one; each way's result under it.
        for (let i = 0; i < n; i++) {
          const x = wayX(i), wy = ways[i];
          g.fillStyle = "#3a2a1a"; g.fillRect(x - 4, 348, 8, WAYS.lug - 348);
          D.disc(g, x, WAYS.lug, 11, C.copper);
          const missing = !bush.set && i === bush.way;
          if (missing) { g.setLineDash([8, 6]); D.round(g, x - 26, WAYS.top, 52, WAYS.h, 22); g.lineWidth = 3; g.strokeStyle = C.amber; g.stroke(); g.setLineDash([]); }
          else {
            for (let k = 0; k < 6; k++) { D.round(g, x - 26 + (k % 2) * 4, WAYS.top + k * 31, 52 - (k % 2) * 8, 26, 12); g.fillStyle = "#d8cfb8"; g.fill(); g.lineWidth = 1.5; g.strokeStyle = "#8a8070"; g.stroke(); }
            // Tracking: a black core in a brown scorch, fading as it is scrubbed.
            g.lineJoin = "round"; g.lineCap = "round";
            for (const tr of wy.tracks) {
              if (tr.clean) continue;
              g.globalAlpha = 1 - 0.8 * tr.wear;
              g.beginPath(); tr.pts.forEach(([px, py], k) => (k ? g.lineTo(x + px, py) : g.moveTo(x + px, py)));
              g.strokeStyle = "rgba(92,52,16,0.45)"; g.lineWidth = 10; g.stroke();
              g.strokeStyle = "#120c06"; g.lineWidth = 4; g.stroke();
            }
            g.globalAlpha = 1; g.lineCap = "butt";
          }
          const res = wy.result, ry = WAYS.top + WAYS.h + 26;
          D.round(g, x - 26, ry - 16, 52, 32, 6); g.fillStyle = res === "high" ? C.ok : res === "low" ? C.danger : "#1b1f26"; g.fill();
          g.lineWidth = sel === i && phase === "test" ? 3 : 1.5; g.strokeStyle = sel === i && phase === "test" ? C.accent : "#3a4656"; g.stroke();
          D.text(g, res === "high" ? "OK" : res === "low" ? "LOW" : String(i + 1), x, ry + 1, 15, res ? "#06090e" : C.dim, "center", 700);
        }
        // The tester: its dial (log scale), its needle, TEST, the probe on its lead.
        D.panel(g, 800, 110, 420, 360, 18, "#0c121a");
        D.text(g, "INSULATION  MΩ", 830, 140, 18, C.dim, "left", 700);
        g.beginPath(); g.arc(METER.x, METER.y, METER.r, A0, A1); g.strokeStyle = "#1d2738"; g.lineWidth = 22; g.stroke();
        g.beginPath(); g.arc(METER.x, METER.y, METER.r, ang(dialK(100)), A1); g.strokeStyle = C.ok; g.lineWidth = 22; g.stroke();
        D.hatchArc(g, METER.x, METER.y, METER.r - 11, METER.r + 11, A0, ang(dialK(1)), C.danger);
        for (const mo of [0.1, 1, 10, 100, 1000]) {
          const q = ang(dialK(mo));
          D.text(g, mo >= 1 ? String(mo) : "0.1", METER.x + Math.cos(q) * (METER.r + 32), METER.y + Math.sin(q) * (METER.r + 32), 15, C.dim, "center", 600);
        }
        const nq = ang(needle);
        g.beginPath(); g.moveTo(METER.x, METER.y); g.lineTo(METER.x + Math.cos(nq) * (METER.r - 6), METER.y + Math.sin(nq) * (METER.r - 6));
        g.strokeStyle = probing ? C.fg : C.steel; g.lineWidth = 5; g.lineCap = "round"; g.stroke(); g.lineCap = "butt";
        D.disc(g, METER.x, METER.y, 12, "#566273");
        const press = probing;
        D.disc(g, TEST.x, TEST.y, TEST.r + (press ? 0 : 4), press ? "#7a1a22" : C.danger);
        D.text(g, "TEST", TEST.x, TEST.y + 1, 22, "#fff", "center", 700);
        // The earth clip on the box's frame, and the probe: a pen coming down onto the lug, its lead over the bus bar, clear of the bushings.
        g.lineCap = "round";
        g.strokeStyle = "#1a1a1a"; g.lineWidth = 6; g.beginPath(); g.moveTo(PORT.x, PORT.y + 12); g.quadraticCurveTo(780, 600, 742, 650); g.stroke();
        D.round(g, 726, 640, 24, 20, 4); g.fillStyle = "#2a2f36"; g.fill();
        const top = probe.y - 46;
        g.strokeStyle = "#7a1a22"; g.lineWidth = 6; g.beginPath(); g.moveTo(PORT.x, PORT.y);
        // Over the box (k 1) it climbs over the bus bar; parked beside the tester (k 0) it droops.
        const k = Math.max(0, Math.min(1, (780 - probe.x) / 120)), mix = (a, b) => a + (b - a) * k;
        g.bezierCurveTo(PORT.x - 40 * k, mix(PORT.y + 60, 330), mix(probe.x, probe.x + 80), mix(top - 30, 300), probe.x, top); g.stroke();
        g.lineCap = "butt";
        D.round(g, probe.x - 8, top, 16, 38, 5); g.fillStyle = C.danger; g.fill();
        g.strokeStyle = "#e8eef6"; g.lineWidth = 3; g.beginPath(); g.moveTo(probe.x, top + 38); g.lineTo(probe.x, probe.y - 2); g.stroke();
        D.disc(g, probe.x, probe.y, 3, "#e8eef6");
        D.disc(g, PORT.x, PORT.y, 7, "#2a2f36");
        // The crate with the spare bushing.
        if (part) {
          D.panel(g, CRATE.x, CRATE.y, CRATE.w, CRATE.h, 10, "#2b2014", "#5a4426");
          D.text(g, "SPARE BUSHING", CRATE.x + CRATE.w / 2, CRATE.y + 18, 15, C.dim, "center", 700);
          if (!bush.set) for (let k = 0; k < 3; k++) { D.round(g, bush.x - 24 + (k % 2) * 4, bush.y - 32 + k * 23, 48 - (k % 2) * 8, 20, 9); g.fillStyle = "#d8cfb8"; g.fill(); }
        }
        // The next thing to touch, pulsing as the kit's next fastener does: the isolator and its way, the probe and the
        // lug to put it on, TEST, or a LOW way's tracking to scrub. One at a time.
        const pulse = (x, y, r) => D.ring(g, x, y, r + 5 + 3 * Math.sin(t * 6), C.accent, 3);
        if (phase === "isolate" || phase === "close") {
          const hx = ISO.x + ISO.w / 2, dir = phase === "isolate" ? 1 : -1;
          pulse(hx, isoY, 42);
          if (!grab) KIT.draw.arrowHead(g, hx, isoY + dir * 62, (dir * Math.PI) / 2, C.accent, 12);
        } else if (phase === "test" && dead && !probing && !probe.held) {
          const on = probe.way >= 0 ? ways[probe.way] : null;
          const dirty = ways.findIndex((x) => x.result === "low" && !good(x));
          const next = ways.findIndex((x) => x.result === null);
          if (on && on.result === null) pulse(TEST.x, TEST.y, TEST.r);
          else if (dirty >= 0) {
            g.setLineDash([8, 6]); D.round(g, wayX(dirty) - 32, WAYS.top - 6, 64, WAYS.h + 12, 26);
            g.lineWidth = 3; g.strokeStyle = `rgba(79,195,247,${0.6 + 0.4 * Math.sin(t * 6)})`; g.stroke(); g.setLineDash([]);
          } else if (next >= 0) { pulse(probe.x, probe.y - 24, 24); pulse(wayX(next), WAYS.lug, 12); }
        }
        // A reading just taken: its result box flashes.
        if (readT > 0 && probe.way >= 0) {
          const x = wayX(probe.way), ry = WAYS.top + WAYS.h + 26;
          D.round(g, x - 32, ry - 22, 64, 44, 9); g.lineWidth = 3; g.strokeStyle = `rgba(232,238,246,${readT})`; g.stroke();
        }
        if (spark) {
          for (let k = 0; k < 10; k++) {
            const a = (k / 10) * Math.PI * 2 + time * 7, rr = 14 + 34 * (1 - spark.t / 0.5);
            g.strokeStyle = k % 2 ? "#cfe9ff" : C.accent; g.lineWidth = 3; g.beginPath();
            g.moveTo(spark.x + Math.cos(a) * 6, spark.y + Math.sin(a) * 6); g.lineTo(spark.x + Math.cos(a) * rr, spark.y + Math.sin(a) * rr); g.stroke();
          }
        }
        if (flash > 0) { g.fillStyle = `rgba(200,230,255,${0.3 * flash})`; g.fillRect(0, api.BAR_H, api.W, api.H); }
      },
    };
  },
});
