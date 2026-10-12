/*
 * repairs/boxterm.js: a conduit box's repair (openspec/changes/repair-minigames, design 2a; power-routing 1: a box in
 * every room).
 *
 * The box open on the wall: the incoming conduit's gland at the top, its wire tails hanging loose, each with a numbered
 * sleeve, and the new terminal strip on its rail. Land each tail in the terminal its sleeve numbers (drag the tail's
 * ferrule onto the terminal, or left and right to choose and Space to pick and drop), then torque each terminal's screw
 * with the wrench (hold the screw, or hold Space on the chosen one) and let go while the gauge's needle is in its green
 * band. A wire in the wrong terminal is a fumble: a spark, 5 HP, and the tail springs back. Past the red the screw
 * strips: a fumble, and it is torqued again. Short of the band it is only loose: torque it again. Each level has more
 * wires (4 to 8), colours that repeat from level 3 (read the sleeve, not the colour), a narrower green band and a faster
 * gauge. A disabled box's first round fits the new strip: drag it from the crate onto the rail.
 */
RepairKit.register({
  id: "boxterm",
  title: "Conduit box",
  place: "Every room's conduit box",
  group: "Engineering",
  hazard: "Wrong terminal: a spark, 5 HP",
  guide: {
    steps: [
      { icon: "screw", text: "Unscrew the cover", cover: true },
      { icon: "match", text: "Each wire to its number" },
      { icon: "hold", text: "Torque into the green" },
    ],
    mistake: "A wire in the wrong terminal: a spark, 5 HP; past the red, the screw strips",
    now: (q) => ({ land: 1, torque: 2 })[q.phase] ?? -1,
  },
  down: "Its room's loads and lights are cut (power-routing)",
  panel: { x: 120, y: 100, w: 640, h: 590, screws: 4 },
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const BOX = { x: 120, y: 100, w: 640, h: 590 };
    const GLAND = { x: 440, y: 150 };
    const RAIL = { x0: 170, x1: 710, y: 520 };
    const TERM = { w: 52, top: 432, h: 104 };
    const GAUGE = { x: 1030, y: 300, r: 120 };
    const CRATE = { x: 900, y: 552, w: 300, h: 140 };
    const COLOURS = [C.danger, C.accent, C.ok, C.warn, C.lilac, "#e8eef6", C.copper, "#9aa7b8"];
    const A0 = (-220 * Math.PI) / 180, A1 = (40 * Math.PI) / 180;
    let phase, part, strip, n, wires, terms, held, sel, needle, torquing, band, rate, flash, spark, time;

    const termX = (i) => RAIL.x0 + 40 + (i * (RAIL.x1 - RAIL.x0 - 80)) / Math.max(1, n - 1);
    const screw = (i) => ({ x: termX(i), y: TERM.top + 16 });
    const entry = (i) => ({ x: termX(i), y: TERM.top + 46 });
    const ang = (k) => A0 + (A1 - A0) * k;

    function wirePath(g, w, ex, ey, width) {
      g.beginPath(); g.moveTo(GLAND.x + w.off, GLAND.y + 30);
      g.bezierCurveTo(GLAND.x + w.off, GLAND.y + 140, ex, ey - 150, ex, ey);
      g.strokeStyle = "#0b0f15"; g.lineWidth = width + 4; g.lineCap = "round"; g.stroke();
      g.strokeStyle = w.c; g.lineWidth = width; g.stroke(); g.lineCap = "butt";
    }
    function sleeve(g, w, x, y, hot) {
      // The ferrule (bare copper end), then the numbered sleeve above it.
      g.fillStyle = "#c9ced6"; g.fillRect(x - 5, y, 10, 16);
      D.round(g, x - 17, y - 34, 34, 32, 6); g.fillStyle = "#eef1f4"; g.fill();
      g.lineWidth = hot ? 3 : 1.5; g.strokeStyle = hot ? C.amber : "#7d8796"; g.stroke();
      D.text(g, String(w.num), x, y - 18, 20, "#10151c", "center", 700);
    }

    return {
      step(index, isPart) {
        const r = api.rand(), level = api.level;
        part = isPart; strip = { x: CRATE.x + 150, y: CRATE.y + 75, held: false, set: !isPart };
        phase = isPart ? "part" : "land";
        n = Math.min(8, 3 + level);
        // From level 3 the colours repeat: three colours for all the wires, so the sleeve's number is what tells them.
        const ncol = level >= 3 ? 3 : n;
        const nums = [...Array(n).keys()].map((i) => i + 1);
        for (let i = nums.length - 1; i > 0; i--) { const j = Math.floor(r() * (i + 1)); [nums[i], nums[j]] = [nums[j], nums[i]]; }
        terms = [...Array(n).keys()].map((i) => ({ num: i + 1, wire: -1, torqued: false, loose: false }));
        wires = nums.map((num, i) => {
          const hx = 190 + (i * 500) / Math.max(1, n - 1) + (r() - 0.5) * 30;
          return { num, c: COLOURS[(num - 1) % ncol], off: (i - (n - 1) / 2) * 7, x: hx, y: 300 + r() * 70, hx, hy: 0, landed: -1 };
        });
        wires.forEach((w) => { w.hy = w.y; });
        held = -1; sel = 0; needle = 0; torquing = -1; flash = 0; spark = null; time = 0;
        band = { w: Math.max(0.08, 0.18 - 0.02 * (level - 1)) };
        band.lo = 0.42 + r() * (0.86 - 0.42 - band.w);
        rate = 0.45 + 0.08 * level;
      },
      peek() { return { phase, n, landed: wires.filter((w) => w.landed >= 0).length, torqued: terms.filter((t) => t.torqued).length, wires: wires.map((w) => ({ num: w.num, x: w.x, y: w.y })), termX, screw, band: { ...band }, strip: { ...strip } }; },
      update(dt, input) {
        time += dt;
        flash = Math.max(0, flash - dt * 1.8);
        if (spark) { spark.t -= dt; if (spark.t <= 0) spark = null; }
        if (phase === "part") {
          // The part: the new terminal strip, from the crate onto the rail.
          if (input.pressed && Math.abs(input.x - strip.x) < 150 && Math.abs(input.y - strip.y) < 50) strip.held = true;
          if (strip.held && input.down) { strip.x = input.x; strip.y = input.y; }
          if (input.actionPressed) { strip.x = (RAIL.x0 + RAIL.x1) / 2; strip.y = RAIL.y; strip.held = false; strip.set = true; }
          if (strip.held && input.released) {
            strip.held = false;
            if (Math.abs(strip.y - RAIL.y) < 70 && strip.x > RAIL.x0 && strip.x < RAIL.x1) strip.set = true;
          }
          if (strip.set) { phase = "land"; api.stepDone(); }
          return;
        }
        if (phase === "land") {
          const loose = wires.map((w, i) => (w.landed < 0 ? i : -1)).filter((i) => i >= 0);
          if (input.hit.has("ArrowRight") || input.hit.has("KeyD")) sel = (sel + 1) % n;
          if (input.hit.has("ArrowLeft") || input.hit.has("KeyA")) sel = (sel + n - 1) % n;
          if (input.pressed) {
            const i = KIT.nearest(wires.map((w) => (w.landed < 0 ? [w.x, w.y] : null)), input.x, input.y, 46);
            if (i >= 0) held = i;
          }
          if (held >= 0 && input.down) { wires[held].x = input.x; wires[held].y = input.y; }
          // Keys: Space picks the first loose wire, then drops it on the chosen terminal.
          let drop = -1, w0 = -1;
          if (input.actionPressed) {
            if (held < 0 && loose.length) { held = loose[0]; wires[held].x = termX(sel); wires[held].y = TERM.top - 60; }
            else if (held >= 0) { w0 = held; drop = sel; }
          }
          if (held >= 0 && input.released) {
            w0 = held;
            drop = KIT.nearest(terms.map((t, i) => (t.wire < 0 ? [entry(i).x, entry(i).y - 30] : null)), input.x, input.y, 50);
          }
          if (w0 < 0) return;
          held = -1;
          const w = wires[w0];
          if (drop < 0) { w.x = w.hx; w.y = w.hy; return; }
          if (terms[drop].num === w.num) { w.landed = drop; terms[drop].wire = w0; sel = drop; }
          else {
            // The wrong terminal: a spark, and the tail springs back.
            spark = { x: entry(drop).x, y: entry(drop).y, t: 0.5 }; flash = 1; w.x = w.hx; w.y = w.hy;
            api.fumble("Wrong terminal: a spark, 5 HP");
          }
          if (wires.every((x) => x.landed >= 0)) { phase = "torque"; sel = 0; }
          return;
        }
        if (phase === "torque") {
          if (input.hit.has("ArrowRight") || input.hit.has("KeyD")) sel = (sel + 1) % n;
          if (input.hit.has("ArrowLeft") || input.hit.has("KeyA")) sel = (sel + n - 1) % n;
          if (torquing < 0) {
            if (input.pressed) { const i = KIT.nearest(terms.map((t, k) => (t.torqued ? null : [screw(k).x, screw(k).y])), input.x, input.y, 40); if (i >= 0) { torquing = i; sel = i; needle = 0; } }
            if (input.actionPressed && !terms[sel].torqued) { torquing = sel; needle = 0; }
          }
          if (torquing < 0) return;
          const holding = input.down || input.action;
          if (holding) {
            needle = Math.min(1.05, needle + rate * dt * (1 + 0.6 * needle));
            if (needle >= 1.0) {
              // Past the red: the screw strips.
              terms[torquing].loose = false; torquing = -1; needle = 0; flash = 0.7;
              api.fumble("Over-torqued: the screw strips");
            }
            return;
          }
          // Let go: in the band it is done; short of it, loose.
          const t = terms[torquing];
          if (needle >= band.lo && needle <= band.lo + band.w) { t.torqued = true; t.loose = false; }
          else t.loose = true;
          torquing = -1;
          if (terms.every((x) => x.torqued)) { phase = "done"; api.stepDone(); }
          else { const next = terms.findIndex((x) => !x.torqued); if (next >= 0 && terms[sel].torqued) sel = next; }
        }
      },
      draw(g, t, dt, input) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        // The box: steel, its back plate, the scorch where the old strip burnt.
        D.panel(g, BOX.x, BOX.y, BOX.w, BOX.h, 10, "#1b2433", "#3a4656");
        D.round(g, BOX.x + 26, BOX.y + 26, BOX.w - 52, BOX.h - 52, 6); g.fillStyle = "#121821"; g.fill();
        const sc = g.createRadialGradient(440, 500, 10, 440, 500, 260);
        sc.addColorStop(0, "rgba(0,0,0,0.85)"); sc.addColorStop(0.5, "rgba(36,20,10,0.6)"); sc.addColorStop(1, "rgba(36,20,10,0)");
        g.fillStyle = sc; g.fillRect(BOX.x + 26, 260, BOX.w - 52, 400);
        // The gland: the incoming conduit.
        g.fillStyle = "#3a4656"; g.fillRect(GLAND.x - 46, BOX.y, 92, 34); D.round(g, GLAND.x - 34, GLAND.y - 20, 68, 52, 6); g.fillStyle = "#566273"; g.fill();
        D.text(g, "FROM PANEL", GLAND.x, BOX.y + 18, 14, C.dim, "center", 700);
        // The rail and the strip.
        g.fillStyle = "#7d8796"; g.fillRect(RAIL.x0, RAIL.y - 8, RAIL.x1 - RAIL.x0, 16);
        for (let x = RAIL.x0 + 14; x < RAIL.x1; x += 28) { g.fillStyle = "#4a5566"; g.fillRect(x, RAIL.y - 3, 14, 6); }
        if (strip.set) {
          for (let i = 0; i < n; i++) {
            const x = termX(i), tm = terms[i], sel_ = (phase === "land" || phase === "torque") && sel === i;
            D.round(g, x - TERM.w / 2, TERM.top, TERM.w, TERM.h, 5); g.fillStyle = "#c8c2b4"; g.fill();
            g.lineWidth = sel_ ? 3 : 1.5; g.strokeStyle = sel_ ? C.accent : "#7d7564"; g.stroke();
            D.disc(g, x, TERM.top + 46, 9, "#1b1f26");                         // the wire's entry
            // The screw: its slot turned as it is torqued; green once done, amber when loose.
            const sx = screw(i).x, sy = screw(i).y;
            D.disc(g, sx, sy, 13, tm.torqued ? C.ok : tm.loose ? C.amber : "#9aa3ae");
            g.save(); g.translate(sx, sy); g.rotate(tm.torqued ? 1.2 : torquing === i ? needle * 5 : 0.3);
            g.fillStyle = "#2a2f36"; g.fillRect(-9, -2, 18, 4); g.restore();
            // Its number on a white tag under it.
            D.round(g, x - 18, TERM.top + TERM.h + 6, 36, 26, 4); g.fillStyle = "#eef1f4"; g.fill();
            D.text(g, String(tm.num), x, TERM.top + TERM.h + 20, 18, "#10151c", "center", 700);
          }
        } else {
          g.setLineDash([12, 9]); D.round(g, RAIL.x0 + 10, TERM.top, RAIL.x1 - RAIL.x0 - 20, TERM.h, 8);
          g.lineWidth = 3; g.strokeStyle = C.amber; g.stroke(); g.setLineDash([]);
        }
        // The wires: landed ones run into their terminals; loose ones hang with their sleeves.
        for (let i = 0; i < wires.length; i++) {
          const w = wires[i];
          if (w.landed >= 0) { const e = entry(w.landed); wirePath(g, w, e.x, e.y, 8); }
        }
        for (let i = 0; i < wires.length; i++) {
          const w = wires[i];
          if (w.landed >= 0) continue;
          wirePath(g, w, w.x, w.y, 8);
          sleeve(g, w, w.x, w.y, held === i);
        }
        if (spark) {
          for (let k = 0; k < 10; k++) {
            const a = (k / 10) * Math.PI * 2 + time * 7, rr = 14 + 30 * (1 - spark.t / 0.5);
            g.strokeStyle = k % 2 ? "#fff6c2" : C.warn; g.lineWidth = 3; g.beginPath();
            g.moveTo(spark.x + Math.cos(a) * 6, spark.y + Math.sin(a) * 6); g.lineTo(spark.x + Math.cos(a) * rr, spark.y + Math.sin(a) * rr); g.stroke();
          }
        }
        // The torque wrench's gauge.
        D.panel(g, 820, 120, 420, 340, 18, "#0c121a");
        D.text(g, "TORQUE", 850, 150, 18, C.dim, "left", 700);
        g.beginPath(); g.arc(GAUGE.x, GAUGE.y, GAUGE.r, A0, A1); g.strokeStyle = "#1d2738"; g.lineWidth = 26; g.stroke();
        g.beginPath(); g.arc(GAUGE.x, GAUGE.y, GAUGE.r, ang(band.lo), ang(band.lo + band.w)); g.strokeStyle = C.ok; g.lineWidth = 26; g.stroke();
        D.hatchArc(g, GAUGE.x, GAUGE.y, GAUGE.r - 13, GAUGE.r + 13, ang(0.92), A1, C.danger);
        const na = ang(Math.min(1, needle));
        g.beginPath(); g.moveTo(GAUGE.x, GAUGE.y); g.lineTo(GAUGE.x + Math.cos(na) * (GAUGE.r - 6), GAUGE.y + Math.sin(na) * (GAUGE.r - 6));
        g.strokeStyle = torquing >= 0 ? C.fg : C.steel; g.lineWidth = 5; g.lineCap = "round"; g.stroke(); g.lineCap = "butt";
        D.disc(g, GAUGE.x, GAUGE.y, 12, "#566273");
        if (phase === "torque" || phase === "done") D.text(g, `T${terms[torquing >= 0 ? torquing : sel].num}`, GAUGE.x, GAUGE.y + 70, 34, C.amber, "center", 700);
        // The crate with the spare strip.
        if (part) {
          D.panel(g, CRATE.x, CRATE.y, CRATE.w, CRATE.h, 10, "#2b2014", "#5a4426");
          D.text(g, "SPARE STRIP", CRATE.x + CRATE.w / 2, CRATE.y + 22, 16, C.dim, "center", 700);
          if (!strip.set) {
            D.round(g, strip.x - 140, strip.y - 34, 280, 68, 6); g.fillStyle = "#c8c2b4"; g.fill();
            for (let k = 0; k < 8; k++) { g.fillStyle = "#7d7564"; g.fillRect(strip.x - 130 + k * 34, strip.y - 26, 4, 52); }
          }
        }
        // The counts: what is landed and torqued (the round's progress, not instructions).
        D.text(g, `${wires.filter((w) => w.landed >= 0).length} / ${n} LANDED`, 850, 490, 20, C.fg, "left", 700);
        D.text(g, `${terms.filter((x) => x.torqued).length} / ${n} TORQUED`, 850, 518, 20, C.fg, "left", 700);
        if (flash > 0) { g.fillStyle = `rgba(255,236,170,${0.35 * flash})`; g.fillRect(0, api.BAR_H, api.W, api.H); }
      },
    };
  },
});
