/*
 * repairs/pipes.js: the reactor's coolant pipes (openspec/changes/reactor-cooling, design 1, 2 and 4).
 *
 * A damaged loop segment, repaired in three rounds, one a step: isolate, rebuild, refill and bleed (a disabled
 * segment adds a part step first, and a second, larger rebuild). More than the shower's puzzle (heads.js): two legs,
 * valves, and the order matters.
 *
 * Isolate: the engineering pipe run as a one-line plan. The core is at the top, the chiller at the bottom, the hot leg
 * down the right (red, chevrons), the cold leg up the left (blue, dots), each with its valves and a bypass round it.
 * One segment drips. Close the two valves either side of it (tap a valve; tap again to open it): the leg's bypass
 * opens on its own and the drip stops. The main valves at the core and the chiller (boxed) carry the core's feed;
 * closing one is the fumble "Core starved: the blanket heats", and it springs back open.
 *
 * Rebuild: the segment's run as a tile grid with two pairs of ends: hot (red, chevrons) left to right, cold (blue,
 * dots) top to bottom. Tap a tile to turn it a quarter. Join each pair with no open end and without the legs meeting;
 * crossover tiles let one leg pass over the other. Cracked tiles drip: drag a new piece from the tray onto each before
 * it carries anything. While building, the tiles each leg would reach show its tint and pattern faintly. Then FILL
 * (reactor-cooling 4, "Coolant flows through it", as in the shower's run, through the kit's KIT.flood): coolant enters
 * at both inlets at once and fills the connected tiles tile by tile, hot red with chevrons and cold blue with dots. A
 * leg reaching its outlet fills the outlet pipe; one stopping at an open end jets there; a cracked tile carrying sprays;
 * the legs meeting throw up steam. The first of those the coolant reaches is the fumble (a scalding spray, or hot into
 * cold); the coolant drains back and the build goes on. Both legs at their outlets, unmixed, nothing cracked carrying:
 * the round is played. The second rebuild is larger, with more crossings.
 *
 * Refill and bleed: the new segment between its two valves, over a high point with a bleed valve on top. Open the
 * downstream valve first, then the upstream one, then hold the bleed valve until the gauge's needle settles (air out,
 * coolant in). Upstream first is the fumble "Water hammer: the joint jumps".
 *
 * Part step (disabled): fetch the new spool from the stores rack and drop it in the gap; the spool for the right leg
 * (hot: red bands and chevrons; cold: blue bands and dots).
 *
 * Colour is never alone: hot pipe carries chevrons, cold pipe dots, a shut valve is solid with a bar across.
 * Keys: arrows pick (a valve, a tile), Space works it (hold it on the bleed valve), Q and E pick a tray piece (Space on a
 * cracked tile fits it), Enter fills.
 */
RepairKit.register({
  id: "pipes",
  title: "Coolant pipes",
  place: "Engineering, the reactor's pipe run",
  group: "Engineering",
  hazard: "Scald: a hot coolant spray",
  // The how-to card (repair-minigames 6g), drawn by the kit: pictures and a few words, on demand.
  guide: {
    steps: [
      { icon: "valve", text: "Shut the valves either side" },
      { icon: "pipes", text: "Turn tiles, swap cracked ones" },
      { icon: "flow", text: "FILL: both legs reach the end" },
      { icon: "order", text: "Open out, in, then bleed" },
    ],
    mistake: "Main valve shut: core starves. A spray or legs meeting: scald",
    now: (q) => ({ isolate: 0, bleed: 3 })[q.kind] ?? (q.kind === "rebuild" ? (q.phase === "play" ? 1 : 2) : -1),
  },
  down: "The loop leaks and loses flow; the reactor runs hot (reactor-cooling 2)",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const STARVED = "Core starved: the blanket heats";
    const HAMMER = "Water hammer: the joint jumps";
    const CRACKED = "Cracked pipe: a scalding spray";
    const MIXED = "Legs mixed: hot into cold";
    const OPEN_END = "Open joint: a scalding spray";
    const HOT = "#ff6a4d", COLD = "#4fa8f7";
    const PLAN_DAMAGED = ["isolate", "rebuild", "bleed"];
    const PLAN_DISABLED = ["part", "isolate", "rebuild", "rebuild", "bleed"];
    const REACH = KIT.TOUCH_R + 8;          // px: a valve's tap reach
    const BLANKET_JUMP_K = 20;              // K the blanket heats when the core is starved (design 4)
    const BLEED_SHARE = 0.45, BLEED_MIN_S = 3, BLEED_MAX_S = 7;   // the bleed's hold, a share of the step's time
    const FLOW_TILES_S = 4;                 // coolant advances this many tiles a second once FILL is pressed
    const FAULT_SHOW_S = 1.6;               // s the coolant keeps running after it meets a fault, before it drains
    const DRAIN_S = 0.8;                    // s the spilt coolant takes to drain back out of the run
    const clamp = (x, a, b) => Math.max(a, Math.min(b, x));
    const lerp = (a, b, u) => a + (b - a) * u;
    const shares = {};
    let s = null, job = null;

    // ================================================================ shared drawing
    /** A pipe along a polyline: casing, coolant (or empty), and the leg's pattern running with the flow. */
    function pipe(g, pts, leg, { w = 24, flow = 0, t = 0, dim = false, fill = 1 } = {}) {
      g.beginPath(); pts.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y)));
      g.lineJoin = "round"; g.lineCap = "round";
      g.strokeStyle = "#263142"; g.lineWidth = w + 8; g.stroke();
      g.strokeStyle = dim ? "#161e2b" : leg === "hot" ? "rgba(255,106,77,0.55)" : leg === "cold" ? "rgba(79,168,247,0.55)" : "#3a4658";
      g.lineWidth = w; g.stroke();
      g.lineCap = "butt";
      if (dim || !leg || fill <= 0) return;
      // The pattern: chevrons for hot, dots for cold, every 30 px along the run, moving with the flow.
      const segs = [];
      let total = 0;
      for (let i = 1; i < pts.length; i++) { const L = Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]); segs.push(L); total += L; }
      const gap = 30, off = ((t * flow * 60) % gap + gap) % gap;
      for (let d = off; d < total * fill; d += gap) {
        let k = 0, acc = 0;
        while (k < segs.length - 1 && acc + segs[k] < d) acc += segs[k++];
        const u = (d - acc) / Math.max(1e-6, segs[k]), [x0, y0] = pts[k], [x1, y1] = pts[k + 1];
        const x = lerp(x0, x1, u), y = lerp(y0, y1, u), a = Math.atan2(y1 - y0, x1 - x0);
        g.save(); g.translate(x, y); g.rotate(a);
        if (leg === "hot") { g.beginPath(); g.moveTo(-5, -7); g.lineTo(4, 0); g.lineTo(-5, 7); g.strokeStyle = "#fff3e0"; g.lineWidth = 3; g.lineJoin = "round"; g.stroke(); }
        else D.disc(g, 0, 0, 3.5, "#e8f6ff");
        g.restore();
      }
    }
    /** A one-line valve: a bowtie on the pipe, hollow when open, solid with a bar across when shut. */
    function bowtie(g, x, y, vertical, shut, opts = {}) {
      const r = opts.r || 18;
      g.save(); g.translate(x, y); if (vertical) g.rotate(Math.PI / 2);
      if (opts.main) { D.round(g, -r - 10, -r - 10, 2 * r + 20, 2 * r + 20, 6); g.fillStyle = "#0b1119"; g.fill(); g.lineWidth = 2.5; g.strokeStyle = "#8796aa"; g.stroke(); }
      g.beginPath(); g.moveTo(-r, -r * 0.8); g.lineTo(r, r * 0.8); g.lineTo(r, -r * 0.8); g.lineTo(-r, r * 0.8); g.closePath();
      g.fillStyle = shut ? "#c0392b" : "#0b1119"; g.fill();
      g.lineWidth = 3; g.strokeStyle = shut ? "#ff8a80" : C.ok; g.stroke();
      // The stem and handle: along the pipe when open, across it when shut.
      g.strokeStyle = shut ? "#ffd2cc" : "#c9d3e0"; g.lineWidth = 5; g.lineCap = "round";
      g.beginPath(); if (shut) { g.moveTo(0, -r - 6); g.lineTo(0, r + 6); } else { g.moveTo(-r * 0.6, -r - 4); g.lineTo(r * 0.6, -r - 4); } g.stroke();
      g.lineCap = "butt";
      if (opts.focus) { g.setLineDash([7, 5]); D.ring(g, 0, 0, r + 18, C.amber, 3); g.setLineDash([]); }
      g.restore();
    }
    function drips(g, x, y, t, n = 6) {
      for (let j = 0; j < n; j++) { const ph = (t * 1.5 + j / n) % 1; D.disc(g, x + ((j * 7) % 5 - 2) * 3, y + ph * 60, 4.5 - 2.5 * ph, `rgba(255,170,120,${0.95 - 0.7 * ph})`); }
      for (let j = 0; j < 3; j++) { const ph = (t * 0.45 + j / 3) % 1; D.disc(g, x + 16 + Math.sin(t + j) * 8, y - 10 - ph * 50, 7 + ph * 12, `rgba(220,230,240,${0.13 * (1 - ph)})`); }
    }
    function spray(g, x, y, t, col = "rgba(255,190,150,0.9)") {
      for (let j = 0; j < 18; j++) {
        const a = (j / 18) * Math.PI * 2 + t * 3, ph = (t * 2.4 + j / 18) % 1;
        D.disc(g, x + Math.cos(a) * ph * 60, y + Math.sin(a) * ph * 60 + ph * ph * 30, 5 - 3.5 * ph, col);
      }
    }
    function coreGlyph(g, x, y, r, hot) {
      const rg = g.createRadialGradient(x, y, 0, x, y, r * 1.7);
      rg.addColorStop(0, hot > 0 ? `rgba(255,140,110,${0.5 + 0.4 * hot})` : "rgba(240,230,255,0.75)"); rg.addColorStop(1, "rgba(190,159,230,0)");
      D.disc(g, x, y, r * 1.7, rg);
      D.disc(g, x, y, r, "#141c28"); D.ring(g, x, y, r, hot > 0 ? C.danger : "#4a5568", 4);
      D.disc(g, x, y, r * 0.5, hot > 0 ? "#ffb4a0" : "#e6dcff");
    }
    function chillGlyph(g, x, y, w, h) {
      D.panel(g, x - w / 2, y - h / 2, w, h, 10, "#152030", "#4a5a70");
      for (let k = 0; k < 7; k++) {
        const xx = x - w / 2 + 14 + k * ((w - 28) / 6);
        g.strokeStyle = "rgba(79,168,247,0.8)"; g.lineWidth = 4;
        g.beginPath(); g.moveTo(xx - 3, y - h / 2 + 10); g.lineTo(xx + 3, y - 4); g.lineTo(xx - 3, y + 6); g.lineTo(xx + 3, y + h / 2 - 10); g.stroke();
      }
    }

    // ================================================================ round 1: isolate
    // The plan (canvas px): the core at the top, the chiller at the bottom, a leg down each side.
    const CORE = { x: 640, y: 150, r: 40 }, CHILL = { x: 640, y: 650, w: 130, h: 54 };
    const XR = 1010, XL = 270, YT = 150, YB = 650, BYR = 1140, BYL = 140;
    const VY = [272, 400, 528];             // the inner valves' heights, top to bottom
    const TEE = [205, 595];                 // where each bypass leaves and rejoins its leg
    /** The valves: four main (at the core and the chiller) and three inner a leg; leg, position, main. */
    const VALVES = [
      { id: "c4", leg: "cold", x: 500, y: YT, main: true }, { id: "h0", leg: "hot", x: 780, y: YT, main: true },
      { id: "h1", leg: "hot", x: XR, y: VY[0], i: 1 }, { id: "h2", leg: "hot", x: XR, y: VY[1], i: 2 }, { id: "h3", leg: "hot", x: XR, y: VY[2], i: 3 },
      { id: "h4", leg: "hot", x: 780, y: YB, main: true }, { id: "c0", leg: "cold", x: 500, y: YB, main: true },
      { id: "c1", leg: "cold", x: XL, y: VY[2], i: 1 }, { id: "c2", leg: "cold", x: XL, y: VY[1], i: 2 }, { id: "c3", leg: "cold", x: XL, y: VY[0], i: 3 },
    ];
    /** Segment n (1-4) of a leg, as its pipe points: the hot leg runs core to chiller, the cold chiller to core. */
    function segPts(leg, n) {
      const X = leg === "hot" ? XR : XL;
      const yA = [YT, VY[0], VY[1], VY[2]], yB = [VY[0], VY[1], VY[2], YB];
      if (leg === "hot") {
        if (n === 1) return [[780, YT], [X, YT], [X, VY[0]]];
        if (n === 4) return [[X, VY[2]], [X, YB], [780, YB]];
        return [[X, yA[n - 1]], [X, yB[n - 1]]];
      }
      if (n === 1) return [[500, YB], [X, YB], [X, VY[2]]];
      if (n === 4) return [[X, VY[0]], [X, YT], [500, YT]];
      return [[X, yB[4 - n]], [X, yA[4 - n]]];
    }
    const segMid = (leg, n) => { const p = segPts(leg, n); return [(p[0][0] + p[p.length - 1][0]) / 2, (p[0][1] + p[p.length - 1][1]) / 2]; };

    function isoStep() {
      s = { kind: "isolate", shut: new Set(), spring: [], focus: 0, keys: false, blanket: 0, done: false, doneT: 0, bypass: { hot: 0, cold: 0 } };
    }
    const isoValve = (v) => VALVES.indexOf(v);
    /** The two valves either side of the cracked segment. */
    const isoNeed = () => VALVES.filter((v) => v.leg === job.leg && (v.i === job.seg - 1 || v.i === job.seg)).map(isoValve);
    function isoToggle(i) {
      const v = VALVES[i];
      if (v.main) {
        // The core's feed: it slams shut, the blanket heats, and it springs back open.
        s.spring.push({ i, t: 0.7 }); s.blanket = 1;
        api.fumble(STARVED);
        return;
      }
      if (s.shut.has(i)) s.shut.delete(i); else s.shut.add(i);
      const need = isoNeed();
      if (s.shut.size === 2 && need.every((k) => s.shut.has(k))) { s.done = true; api.stepDone(); }
    }
    function isoUpdate(dt, input) {
      s.blanket = Math.max(0, s.blanket - dt * 0.6);
      s.spring = s.spring.filter((sp) => (sp.t -= dt) > 0);
      for (const leg of ["hot", "cold"]) {
        const want = [...s.shut].some((i) => VALVES[i].leg === leg) ? 1 : 0;
        s.bypass[leg] += (want - s.bypass[leg]) * Math.min(1, dt * 4);
      }
      if (s.done) { s.doneT += dt; return; }
      for (const [code, d] of [["ArrowLeft", -1], ["KeyA", -1], ["ArrowUp", -1], ["KeyW", -1], ["ArrowRight", 1], ["KeyD", 1], ["ArrowDown", 1], ["KeyS", 1], ["Tab", 1]]) {
        if (input.hit.has(code)) { s.keys = true; s.focus = (s.focus + d + VALVES.length) % VALVES.length; }
      }
      if (input.actionPressed) { s.keys = true; isoToggle(s.focus); return; }
      if (input.pressed) {
        const i = KIT.nearest(VALVES, input.x, input.y, REACH);
        if (i >= 0) { s.keys = false; s.focus = i; isoToggle(i); }
      }
    }
    function isoDraw(g, t) {
      D.panel(g, 40, 92, 1200, 616, 18, "#0a0f17");
      const cut = (leg) => [...s.shut].some((i) => VALVES[i].leg === leg);
      const crackIso = s.done;
      // The bypasses: dashed and dry while shut, running once their leg is cut.
      for (const leg of ["hot", "cold"]) {
        const X = leg === "hot" ? XR : XL, B = leg === "hot" ? BYR : BYL, k = s.bypass[leg];
        const pts = [[X, TEE[0]], [B, TEE[0]], [B, TEE[1]], [X, TEE[1]]];
        if (leg === "cold") pts.reverse();
        pipe(g, pts, k > 0.5 ? leg : null, { w: 14, flow: 1.5, t, dim: k < 0.5 });
        if (k < 0.5) { g.setLineDash([10, 8]); g.beginPath(); pts.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y))); g.strokeStyle = "#3a4658"; g.lineWidth = 3; g.stroke(); g.setLineDash([]); }
        bowtie(g, B, 400, true, k < 0.5, { r: 12 });
      }
      // The legs, segment by segment: dry where the bypass has taken the flow.
      for (const leg of ["hot", "cold"]) for (let n = 1; n <= 4; n++) {
        const between = cut(leg) && n >= 2 && n <= 3, outer = cut(leg) && !between;
        pipe(g, segPts(leg, n), leg, { flow: outer ? 0.6 : 1.5, t, dim: between });
      }
      // Main runs: core to the main valves, the main valves to the chiller.
      pipe(g, [[CORE.x + CORE.r, YT], [780, YT]], "hot", { flow: 1.5, t });
      pipe(g, [[780, YB], [CHILL.x + CHILL.w / 2, YB]], "hot", { flow: 1.5, t });
      pipe(g, [[CHILL.x - CHILL.w / 2, YB], [500, YB]], "cold", { flow: 1.5, t });
      pipe(g, [[500, YT], [CORE.x - CORE.r, YT]], "cold", { flow: 1.5, t });
      // The crack: a split mark and a drip, until it is cut off.
      const [mx, my] = segMid(job.leg, job.seg), side = job.leg === "hot" ? 1 : -1;
      g.strokeStyle = C.danger; g.lineWidth = 3;
      g.beginPath(); g.moveTo(mx - 8, my - 18); g.lineTo(mx + 6, my - 6); g.lineTo(mx - 6, my + 4); g.lineTo(mx + 8, my + 18); g.stroke();
      if (!crackIso) drips(g, mx + side * 24, my + 8, t);
      else { const k = Math.max(0, 1 - s.doneT); if (k > 0) drips(g, mx + side * 24, my + 8, t, 2); }
      coreGlyph(g, CORE.x, CORE.y, CORE.r, s.blanket);
      chillGlyph(g, CHILL.x, CHILL.y, CHILL.w, CHILL.h);
      VALVES.forEach((v, i) => {
        const shut = s.shut.has(i) || s.spring.some((sp) => sp.i === i);
        bowtie(g, v.x, v.y, v.x === XR || v.x === XL, shut, { main: v.main, focus: s.keys && s.focus === i });
      });
      if (s.blanket > 0) D.text(g, "BLANKET +20 K", CORE.x, CORE.y + 74, 20, C.danger, "center", 700);
    }

    // ================================================================ round 2: rebuild (the tile grid)
    const N_ = 0, E_ = 1, S_ = 2, W_ = 3;   // ports: north, east, south, west
    const DC = [0, 1, 0, -1], DR = [-1, 0, 1, 0];
    const opp = (p) => (p + 2) % 4;
    const TYPES = { s: [[0, 2]], e: [[0, 1]], t: [[0, 1, 2]], x: [[0, 2], [1, 3]] };
    const TRAY = ["s", "e", "x"];
    const TRAY_BOX = { x: 1020, y: 116, w: 220, h: 400 };
    const FILL_BTN = { x: 1020, y: 556, w: 220, h: 116 };
    const slotXY = (i) => [TRAY_BOX.x + TRAY_BOX.w / 2, TRAY_BOX.y + 70 + i * 130];
    /** A tile's port groups, turned k quarters clockwise. */
    const groups = (tile) => TYPES[tile.type].map((gp) => gp.map((p) => (p + tile.k) % 4));

    /** A random walk from a cell entered by port `pin` to the goal cell left by port `pout`; `ok` vets each move. */
    function walk(r, cols, rows, start, pin, goal, pout, ok, bias, budget = 4000) {
      const seen = new Set(), path = [];
      const key = (c, rr) => rr * cols + c;
      const go = (c, rr, inP) => {
        if (--budget < 0) return false;
        seen.add(key(c, rr)); path.push({ c, r: rr, inP, outP: -1 });
        const outs = ok.forced(c, rr, inP);
        const order = outs.map((p) => ({ p, w: r() + (bias[p] || 0) })).sort((a, b) => b.w - a.w).map((o) => o.p);
        for (const p of order) {
          if (c === goal[0] && rr === goal[1] && p === pout) { path[path.length - 1].outP = p; return true; }
          const nc = c + DC[p], nr = rr + DR[p];
          if (nc < 0 || nr < 0 || nc >= cols || nr >= rows || seen.has(key(nc, nr)) || !ok.enter(nc, nr, opp(p))) continue;
          path[path.length - 1].outP = p;
          if (go(nc, nr, opp(p))) return true;
        }
        seen.delete(key(c, rr)); path.pop(); return false;
      };
      return go(start[0], start[1], pin) ? path : null;
    }
    /** The two legs' runs: hot left to right, cold top to bottom, crossing `want` times on crossover tiles. */
    function layRuns(r, cols, rows, want) {
      for (let tries = 0; tries < 400; tries++) {
        const a = Math.floor(r() * rows), b = Math.floor(r() * rows), c = 1 + Math.floor(r() * (cols - 2)), d = 1 + Math.floor(r() * (cols - 2));
        const free = { forced: (cc, rr, inP) => [0, 1, 2, 3].filter((p) => p !== inP), enter: () => true };
        const hot = walk(r, cols, rows, [0, a], W_, [cols - 1, b], E_, free, want > 1 ? { 0: 0.1, 2: 0.1 } : { 1: 0.4 });
        if (!hot) continue;
        const hotAt = new Map(hot.map((h) => [h.r * cols + h.c, h]));
        const straightAcross = (h) => (h.inP === 1 || h.inP === 3) && h.outP === opp(h.inP);
        const crossing = {
          forced: (cc, rr, inP) => (hotAt.has(rr * cols + cc) ? [opp(inP)] : [0, 1, 2, 3].filter((p) => p !== inP)),
          enter: (cc, rr, inP) => { const h = hotAt.get(rr * cols + cc); return !h || (straightAcross(h) && (inP === 0 || inP === 2)); },
        };
        if (!crossing.enter(c, 0, N_)) continue;
        for (let k = 0; k < 25; k++) {
          const cold = walk(r, cols, rows, [c, 0], N_, [d, rows - 1], S_, crossing, { 2: 0.15 });
          if (!cold) break;
          const x = cold.filter((q) => hotAt.has(q.r * cols + q.c)).length;
          if (x === want) return { hot, cold, ends: { a, b, c, d } };
        }
      }
      return null;
    }

    function rebuildStep(r, round) {
      const [cols, rows, T] = round === 0 ? [5, 4, 88] : [7, 5, 80];
      const want = round === 0 ? 1 : 3, cracks = round === 0 ? 1 : 2;
      let runs = layRuns(r, cols, rows, want);
      if (!runs) runs = layRuns(r, cols, rows, 1);
      s = {
        kind: "rebuild", round, cols, rows, T, tiles: [], cur: { c: 0, r: 0 }, keys: false, sel: 0, held: null, refuse: null,
        phase: "play", front: 0, fumbled: false, afterFault: 0, drainT: 0, ends: runs.ends, net: null,
      };
      s.gx = 540 - (cols * T) / 2; s.gy = 410 - (rows * T) / 2;
      // Decoys first; the runs laid over them; crossings where they meet; then every tile turned at random.
      for (let i = 0; i < cols * rows; i++) { const q = r(); s.tiles.push({ type: q < 0.36 ? "s" : q < 0.8 ? "e" : q < 0.92 ? "t" : "x", k: 0, ang: 0, sol: false, cracked: false }); }
      const lay = (run) => run.forEach((q) => {
        const tile = s.tiles[q.r * cols + q.c], straight = q.outP === opp(q.inP);
        if (tile.sol) { tile.type = "x"; return; }
        tile.sol = true; tile.type = straight ? "s" : "e";
        const m = [q.inP, q.outP];
        tile.solK = [0, 1, 2, 3].find((k) => { const gp = TYPES[tile.type][0].map((p) => (p + k) % 4); return m.every((p) => gp.includes(p)); });
      });
      lay(runs.hot); lay(runs.cold);
      for (const tile of s.tiles) { tile.k = Math.floor(r() * 4); tile.ang = (tile.k * Math.PI) / 2; }
      const cand = s.tiles.filter((tile) => tile.sol && tile.type !== "x");
      for (let n = 0; n < cracks && cand.length; n++) cand.splice(Math.floor(r() * cand.length), 1)[0].cracked = true;
      if (network().ok) { const tile = cand[0] || s.tiles[0]; tile.k++; tile.ang = (tile.k * Math.PI) / 2; }
    }
    const tileAt = (c, r) => (c >= 0 && r >= 0 && c < s.cols && r < s.rows ? s.tiles[r * s.cols + c] : null);
    const cellXY = (c, r) => [s.gx + c * s.T + s.T / 2, s.gy + r * s.T + s.T / 2];
    /** Each leg's ends: its entry cell and port, its exit cell and port. */
    function legEnds() {
      const { a, b, c, d } = s.ends;
      return {
        hot: { start: [0, a], pin: W_, goal: [s.cols - 1, b], pout: E_ },
        cold: { start: [c, 0], pin: N_, goal: [d, s.rows - 1], pout: S_ },
      };
    }
    /**
     * The coolant's reach through the tiles as they lie, both legs filling at once from their inlets through the kit's
     * fill (KIT.flood, the shower's): which leg wets each tile's group and at what distance (in tiles), where each leg
     * reaches its outlet, and the faults the coolant meets, each at the distance it gets there: an open end (it sprays
     * there and goes no further), a cracked tile carrying, the two legs meeting. A tile at distance d wets from its inlet
     * port at d to its far ports at d + 1.
     */
    function network() {
      const ends = legEnds(), legOf = new Map(), from = new Map(), faults = [], reach = { hot: -1, cold: -1 }, starts = [];
      const groupAt = (c, rr, pin) => groups(tileAt(c, rr)).findIndex((gq) => gq.includes(pin));
      const edge = (c, rr, p) => { const [x, y] = cellXY(c, rr); return [x + (DC[p] * s.T) / 2, y + (DR[p] * s.T) / 2]; };
      for (const leg of ["hot", "cold"]) {
        const E = ends[leg], gp = groupAt(E.start[0], E.start[1], E.pin);
        if (gp < 0) { faults.push({ kind: "leak", leg, at: edge(E.start[0], E.start[1], E.pin), dir: opp(E.pin), d: 0 }); continue; }
        const key = `${E.start[0]},${E.start[1]},${gp}`;
        starts.push({ key, c: E.start[0], r: E.start[1], gp, pin: E.pin, leg });
        legOf.set(key, leg); from.set(key, E.pin);
      }
      const { dist, far } = KIT.flood(starts, (n, d, go) => {
        const tile = tileAt(n.c, n.r), E = ends[n.leg], other = ends[n.leg === "hot" ? "cold" : "hot"];
        if (tile.cracked) faults.push({ kind: "crack", leg: n.leg, at: cellXY(n.c, n.r), d: d + 0.5 });
        for (const p of groups(tile)[n.gp]) {
          if (p === n.pin) continue;
          const here = edge(n.c, n.r, p);
          if (n.c === E.goal[0] && n.r === E.goal[1] && p === E.pout) { if (reach[n.leg] < 0) reach[n.leg] = d + 1; continue; }
          if ((n.c === other.goal[0] && n.r === other.goal[1] && p === other.pout) || (n.c === other.start[0] && n.r === other.start[1] && p === other.pin)) {
            faults.push({ kind: "meet", leg: n.leg, at: here, d: d + 1 }); continue;
          }
          const nc = n.c + DC[p], nr = n.r + DR[p];
          const ngp = tileAt(nc, nr) ? groupAt(nc, nr, opp(p)) : -1;
          if (ngp < 0) { faults.push({ kind: "leak", leg: n.leg, at: here, dir: p, d: d + 1 }); continue; }
          const key = `${nc},${nr},${ngp}`;
          if (go({ key, c: nc, r: nr, gp: ngp, pin: opp(p), leg: n.leg })) { legOf.set(key, n.leg); from.set(key, opp(p)); }
          else if (legOf.get(key) !== n.leg) faults.push({ kind: "meet", leg: n.leg, at: here, d: d + 1 });
        }
      });
      faults.sort((a, b) => a.d - b.d);
      const end = Math.max(far + 1, reach.hot, reach.cold, ...faults.map((f) => f.d));
      const ok = reach.hot >= 0 && reach.cold >= 0 && !faults.length;
      return { dist, legOf, from, faults, reach, far, end, ok };
    }
    /** FILL: the coolant runs through what is built (design 4); what it meets is judged as it gets there. */
    function fill() {
      s.net = network(); s.phase = "flow"; s.front = 0; s.fumbled = false; s.afterFault = 0; s.drainT = 0;
    }
    const FAULT_SAY = { crack: CRACKED, meet: MIXED, leak: OPEN_END };
    function fitPart(c, rr, type) {
      const tile = tileAt(c, rr);
      if (!tile || !tile.cracked) { s.refuse = { c, r: rr, t: 0.6 }; return false; }
      tile.type = type; tile.cracked = false; tile.k = 0; tile.ang = 0; tile.fresh = 1;
      return true;
    }
    function rebuildUpdate(dt, input) {
      if (s.refuse && (s.refuse.t -= dt) <= 0) s.refuse = null;
      for (const tile of s.tiles) { tile.ang += ((tile.k * Math.PI) / 2 - tile.ang) * Math.min(1, dt * 16); if (tile.fresh) tile.fresh = Math.max(0, tile.fresh - dt); }
      if (s.phase === "flow") {
        // The front advances tile by tile; the first fault it reaches is the fumble, and it keeps running a moment so
        // every spray it reaches shows, then drains back to the build.
        s.front += dt * FLOW_TILES_S;
        const first = s.net.faults[0];
        if (first && !s.fumbled && s.front >= first.d) { s.fumbled = true; api.fumble(FAULT_SAY[first.kind]); return; }
        if (s.fumbled) {
          s.afterFault += dt;
          if (s.afterFault >= FAULT_SHOW_S || s.front >= s.net.end + 0.5) { s.phase = "drain"; s.drainT = 0; }
        } else if (s.front >= s.net.end + 0.6) { s.phase = "running"; api.stepDone(); }
        return;
      }
      if (s.phase === "running") { s.front += dt * FLOW_TILES_S; return; }
      if (s.phase === "drain") {
        s.drainT += dt;
        if (s.drainT >= DRAIN_S) { s.phase = "play"; s.net = null; s.front = 0; }
        return;
      }
      if (s.phase !== "play") return;
      // Keys: a cursor; Space turns (or fits the picked piece on a cracked tile); Q and E pick a piece; Enter fills.
      for (const [code, dc, dr] of [["ArrowLeft", -1, 0], ["KeyA", -1, 0], ["ArrowRight", 1, 0], ["KeyD", 1, 0], ["ArrowUp", 0, -1], ["KeyW", 0, -1], ["ArrowDown", 0, 1], ["KeyS", 0, 1]]) {
        if (input.hit.has(code)) { s.keys = true; s.cur.c = clamp(s.cur.c + dc, 0, s.cols - 1); s.cur.r = clamp(s.cur.r + dr, 0, s.rows - 1); }
      }
      if (input.hit.has("KeyQ")) { s.keys = true; s.sel = (s.sel + TRAY.length - 1) % TRAY.length; }
      if (input.hit.has("KeyE")) { s.keys = true; s.sel = (s.sel + 1) % TRAY.length; }
      if (input.hit.has("Space")) {
        s.keys = true;
        const tile = tileAt(s.cur.c, s.cur.r);
        if (tile.cracked) fitPart(s.cur.c, s.cur.r, TRAY[s.sel]); else tile.k++;
      }
      if (input.hit.has("Enter")) { fill(); return; }
      // Pointer: a press on the tray picks a piece up; a drag carries it; a tap on a tile turns it; FILL fills.
      if (input.pressed) {
        const i = KIT.nearest(TRAY.map((_, k) => slotXY(k)), input.x, input.y, 62);
        if (i >= 0) { s.held = { type: TRAY[i], x: input.x, y: input.y }; s.sel = i; s.keys = false; return; }
        if (input.x >= FILL_BTN.x && input.x <= FILL_BTN.x + FILL_BTN.w && input.y >= FILL_BTN.y && input.y <= FILL_BTN.y + FILL_BTN.h) { fill(); return; }
        const c = Math.floor((input.x - s.gx) / s.T), rr = Math.floor((input.y - s.gy) / s.T), tile = tileAt(c, rr);
        if (tile) { tile.k++; s.cur = { c, r: rr }; s.keys = false; }
      }
      if (s.held && input.down) { s.held.x = input.x; s.held.y = input.y; }
      if (s.held && !input.down) {
        const c = Math.floor((s.held.x - s.gx) / s.T), rr = Math.floor((s.held.y - s.gy) / s.T);
        if (tileAt(c, rr)) fitPart(c, rr, s.held.type);
        s.held = null;
      }
    }
    /**
     * A tile group's branches as the coolant runs them, from the port it comes in by (`f`, unturned) to the far port(s):
     * each a function of u, 0 at the inlet port and 1 at the far port, to a point in px at the tile's centre.
     */
    function branches(gp, f, T) {
      const P = [(DC[f] * T) / 2, (DR[f] * T) / 2], at2 = (p) => [(DC[p] * T) / 2, (DR[p] * T) / 2];
      const others = gp.filter((p) => p !== f);
      if (gp.length === 3) return others.map((q) => (u) => (u < 0.5 ? [lerp(P[0], 0, u * 2), lerp(P[1], 0, u * 2)] : [lerp(0, at2(q)[0], u * 2 - 1), lerp(0, at2(q)[1], u * 2 - 1)]));
      const Q = at2(others[0]);
      if (opp(f) === others[0]) return [(u) => [lerp(P[0], Q[0], u), lerp(P[1], Q[1], u)]];
      return [(u) => [(1 - u) * (1 - u) * P[0] + u * u * Q[0], (1 - u) * (1 - u) * P[1] + u * u * Q[1]]];   // the elbow: a curve round the centre
    }
    /**
     * One tile's pipes at the origin, unrotated groups turned by `ang`. A group a leg reaches is tinted faintly with its
     * pattern while building (the preview); once FILL runs, the coolant fills it from its inlet port, `wetOf(gi)` (0-1)
     * of the way, in the leg's colour with its chevrons (hot) or dots (cold) moving with the flow.
     */
    function tilePipes(g, type, ang, T, legOf, wetOf, fromOf, t = 0, fade = 1) {
      g.save(); g.rotate(ang);
      TYPES[type].forEach((gp, gi) => {
        const leg = legOf ? legOf(gi) : null, w = leg && wetOf ? wetOf(gi) : 0;
        const col = leg ? (leg === "hot" ? "rgba(255,106,77,0.3)" : "rgba(79,168,247,0.3)") : "#4a586d";
        // A crossover's second group (east-west) bridges over the first: drawn last, with a gap cut under it.
        if (type === "x" && gi === 1) { g.strokeStyle = "#0d131c"; g.lineWidth = 34; g.beginPath(); g.moveTo(-T / 2 + 18, 0); g.lineTo(T / 2 - 18, 0); g.stroke(); }
        const from = fromOf ? fromOf(gi) : null, br = branches(gp, from === null || !gp.includes(from) ? gp[0] : from, T);
        const path = (fn, u1) => { g.beginPath(); for (let k = 0; k <= 16; k++) { const [x, y] = fn((u1 * k) / 16); if (k) g.lineTo(x, y); else g.moveTo(x, y); } g.stroke(); };
        for (const [wd, c2] of [[28, "#232c3b"], [20, col]]) { g.strokeStyle = c2; g.lineWidth = wd; g.lineCap = "round"; for (const fn of br) path(fn, 1); }
        if (w > 0) {
          g.globalAlpha = fade; g.strokeStyle = leg === "hot" ? HOT : COLD; g.lineWidth = 20;
          for (const fn of br) path(fn, Math.min(1, w));
          g.globalAlpha = 1;
        }
        g.lineCap = "butt";
        // The leg's pattern, never colour alone: bright and running where the coolant is, faint where it will go.
        if (leg) {
          const us = gp.length === 3 ? [0.25, 0.75] : [1 / 6, 0.5, 5 / 6], step = 1 / us.length;
          for (const fn of br) for (const u0 of us) {
            const u = w >= 1 ? (u0 + t * 0.9 * step * FLOW_TILES_S / 2) % 1 : u0, wet = u <= w;
            const [x, y] = fn(u), [xa, ya] = fn(Math.max(0, u - 0.02)), [xb, yb] = fn(Math.min(1, u + 0.02));
            g.save(); g.translate(x, y); g.rotate(Math.atan2(yb - ya, xb - xa)); g.globalAlpha = wet ? fade : 1;
            const ink = wet ? (leg === "hot" ? "#fff3e0" : "#e8f6ff") : leg === "hot" ? "rgba(255,210,190,0.45)" : "rgba(210,235,255,0.45)";
            if (leg === "hot") { g.beginPath(); g.moveTo(-4, -6); g.lineTo(3, 0); g.lineTo(-4, 6); g.strokeStyle = ink; g.lineWidth = 2.5; g.stroke(); }
            else D.disc(g, 0, 0, 3, ink);
            g.restore();
          }
        }
      });
      D.disc(g, 0, 0, type === "x" ? 0 : 9, "#5a6a82");
      g.restore();
    }
    /** Coolant jetting out of an open joint, the way it was running (`dir`, a port): a spray, and steam off the hot leg. */
    function jet(g, x, y, dir, t, leg, k = 1) {
      const dx = DC[dir], dy = DR[dir], col = leg === "hot" ? "255,150,110" : "130,195,250";
      for (let j = 0; j < 36; j++) {
        const ph = (t * 2.2 + j / 36) % 1, side = ((j * 7) % 11) / 10 - 0.5;
        const px = x + dx * ph * 120 + (dx ? 0 : side * ph * 70), py = y + dy * ph * 120 + (dy ? 0 : side * ph * 70) + ph * ph * 50;
        D.disc(g, px, py, 9 - 6 * ph, `rgba(${col},${(0.95 * (1 - ph) * k).toFixed(3)})`);
      }
      D.disc(g, x, y, 13, `rgba(${col},${(0.9 * k).toFixed(3)})`);
      if (leg === "hot") for (let j = 0; j < 4; j++) { const ph = (t * 0.7 + j / 4) % 1; D.disc(g, x + dx * 40 + Math.sin(t * 2 + j) * 10, y + dy * 40 - ph * 70, 10 + ph * 18, `rgba(230,236,244,${(0.22 * (1 - ph) * k).toFixed(3)})`); }
    }
    /** The two legs meeting: hot into cold, a burst of steam over a ring half red, half blue, and a hatched core. */
    function meetGlyph(g, x, y, t, k = 1) {
      g.save(); g.globalAlpha = k;
      for (let j = 0; j < 7; j++) { const ph = (t * 0.8 + j / 7) % 1; D.disc(g, x + Math.sin(j * 2.1 + t) * 22 * ph, y - ph * 80, 10 + ph * 24, `rgba(235,240,248,${(0.5 * (1 - ph)).toFixed(3)})`); }
      const r = 24 + 3 * Math.sin(t * 8);
      g.lineWidth = 6;
      g.beginPath(); g.arc(x, y, r, Math.PI / 2, Math.PI * 1.5); g.strokeStyle = HOT; g.stroke();
      g.beginPath(); g.arc(x, y, r, -Math.PI / 2, Math.PI / 2); g.strokeStyle = COLD; g.stroke();
      g.beginPath(); g.arc(x, y, r - 6, 0, Math.PI * 2); g.save(); g.clip(); D.hatch(g, x - r, y - r, 2 * r, 2 * r, "rgba(190,159,230,0.8)"); g.restore();
      g.restore();
    }
    function rebuildDraw(g, t) {
      const T = s.T, net = s.net || network(), flowing = s.phase !== "play", front = s.front;
      const fade = s.phase === "drain" ? Math.max(0, 1 - s.drainT / DRAIN_S) : 1;
      D.panel(g, s.gx - 14, s.gy - 14, s.cols * T + 28, s.rows * T + 28, 16, "#0b1018");
      // The ends: stubs in from outside, hot left to right, cold top to bottom. The inlets stand full behind the grid;
      // an outlet fills once its leg's coolant reaches it.
      const E = legEnds();
      const [, hy0] = cellXY(...E.hot.start), [, hy1] = cellXY(...E.hot.goal), [cx0] = cellXY(...E.cold.start), [cx1] = cellXY(...E.cold.goal);
      const outlet = (leg) => (flowing && net.reach[leg] >= 0 && front >= net.reach[leg] ? clamp((front - net.reach[leg]) / 1.2, 0.05, 1) * fade : 0);
      pipe(g, [[20, hy0], [s.gx, hy0]], "hot", { flow: flowing ? 1 : 0.3, t });
      pipe(g, [[s.gx + s.cols * T, hy1], [s.gx + s.cols * T + 90, hy1]], "hot", { flow: 1, t, dim: !outlet("hot"), fill: outlet("hot") });
      pipe(g, [[cx0, 84], [cx0, s.gy]], "cold", { flow: flowing ? 1 : 0.3, t });
      pipe(g, [[cx1, s.gy + s.rows * T], [cx1, 712]], "cold", { flow: 1, t, dim: !outlet("cold"), fill: outlet("cold") });
      for (const [x, y, v] of [[s.gx - 4, hy0, true], [s.gx + s.cols * T + 4, hy1, true], [cx0, s.gy - 4, false], [cx1, s.gy + s.rows * T + 4, false]]) {
        g.fillStyle = "#8796aa"; if (v) g.fillRect(x - 4, y - 22, 8, 44); else g.fillRect(x - 22, y - 4, 44, 8);
      }
      // The tiles: the groups a leg reaches carry its tint while building, and fill in order of distance once flowing.
      for (let rr = 0; rr < s.rows; rr++) for (let c = 0; c < s.cols; c++) {
        const tile = tileAt(c, rr), [x, y] = cellXY(c, rr), key = (gi) => `${c},${rr},${gi}`;
        D.round(g, x - T / 2 + 3, y - T / 2 + 3, T - 6, T - 6, 10);
        g.fillStyle = tile.cracked ? "#1d1410" : tile.fresh ? `rgba(61,220,132,${0.25 * tile.fresh})` : "#0f1520"; g.fill();
        if (tile.cracked) { g.lineWidth = 2; g.strokeStyle = "rgba(255,71,87,0.6)"; g.setLineDash([6, 5]); g.stroke(); g.setLineDash([]); }
        g.save(); g.translate(x, y);
        const legOf = (gi) => net.legOf.get(key(gi)) || null;
        const wet = (gi) => (flowing && net.dist.has(key(gi)) ? clamp(front - net.dist.get(key(gi)), 0, 1) : 0);
        const fromOf = (gi) => { const f = net.from.get(key(gi)); return f === undefined ? null : (((f - tile.k) % 4) + 4) % 4; };
        tilePipes(g, tile.type, tile.ang, T, legOf, wet, fromOf, t, fade);
        g.restore();
        if (tile.cracked) {
          g.strokeStyle = C.danger; g.lineWidth = 3;
          g.beginPath(); g.moveTo(x - 14, y - 20); g.lineTo(x - 2, y - 6); g.lineTo(x - 12, y + 4); g.lineTo(x + 4, y + 20); g.stroke();
          drips(g, x + 18, y + 10, t + c * 0.3, 4);
        }
        if (s.refuse && s.refuse.c === c && s.refuse.r === rr) {
          g.strokeStyle = C.danger; g.lineWidth = 5; g.beginPath(); g.moveTo(x - 18, y - 18); g.lineTo(x + 18, y + 18); g.moveTo(x + 18, y - 18); g.lineTo(x - 18, y + 18); g.stroke();
        }
      }
      // What the coolant met, each once it gets there: a jet at an open end, a spray from a cracked tile, steam where
      // the legs meet.
      if (flowing) for (const f of net.faults) {
        if (front < f.d) continue;
        const [fx, fy] = f.at;
        if (f.kind === "leak") jet(g, fx, fy, f.dir, t, f.leg, fade);
        else if (f.kind === "crack") { jet(g, fx + 6, fy, E_, t, f.leg, fade * 0.9); jet(g, fx - 6, fy, W_, t + 0.37, f.leg, fade * 0.9); }
        else meetGlyph(g, fx, fy, t, fade);
      }
      if (s.keys && s.phase === "play") { const [x, y] = cellXY(s.cur.c, s.cur.r); D.round(g, x - T / 2 + 1, y - T / 2 + 1, T - 2, T - 2, 10); g.lineWidth = 4; g.strokeStyle = C.amber; g.stroke(); }
      // The tray: three new pieces, picked up by a press and dropped on a cracked tile.
      D.panel(g, TRAY_BOX.x, TRAY_BOX.y, TRAY_BOX.w, TRAY_BOX.h, 16, "#0c121a");
      TRAY.forEach((type, i) => {
        const [x, y] = slotXY(i);
        D.round(g, x - 52, y - 52, 104, 104, 12); g.fillStyle = "#141c28"; g.fill();
        g.lineWidth = 2; g.strokeStyle = s.keys && s.sel === i ? C.amber : "#2a3446"; g.stroke();
        g.save(); g.translate(x, y); tilePipes(g, type, 0, 84, null, null, null); g.restore();
      });
      // FILL: the round's main action, the biggest control.
      const ready = s.phase === "play";
      D.round(g, FILL_BTN.x, FILL_BTN.y, FILL_BTN.w, FILL_BTN.h, 22); g.fillStyle = ready ? "#1f6b45" : "#1a2230"; g.fill();
      g.lineWidth = 3; g.strokeStyle = ready ? C.ok : C.line; g.stroke();
      D.text(g, "FILL", FILL_BTN.x + FILL_BTN.w / 2, FILL_BTN.y + FILL_BTN.h / 2, 40, ready ? C.fg : C.dim, "center", 700);
      if (s.held) { g.save(); g.translate(s.held.x, s.held.y); g.globalAlpha = 0.9; D.round(g, -44, -44, 88, 88, 12); g.fillStyle = "rgba(20,28,40,0.85)"; g.fill(); tilePipes(g, s.held.type, 0, 84, null, null, null); g.restore(); g.globalAlpha = 1; }
    }

    // ================================================================ round 3: refill and bleed
    const RUN = [[40, 500], [420, 500], [500, 330], [780, 330], [860, 500], [1240, 500]];
    const UPV = { x: 250, y: 500 }, DNV = { x: 1040, y: 500 };   // upstream (left) and downstream (right) valves
    const BLEED = { x: 640, y: 330, top: 236 };                   // the bleed valve on the high point
    const GAUGE = { x: 1040, y: 236, r: 76 };
    const WHEEL = 40;                                             // a handwheel's radius, px
    const targets = () => [[UPV.x, UPV.y - 64], [BLEED.x, BLEED.top], [DNV.x, DNV.y - 64]];

    function bleedStep(index) {
      s = { kind: "bleed", up: false, dn: false, upA: 0, dnA: 0, fill: 0, air: 1, holding: false, settled: 0, played: false,
        jolt: 0, focus: 2, keys: false, need: clamp(BLEED_SHARE * shares[index], BLEED_MIN_S, BLEED_MAX_S), spit: 0 };
    }
    function bleedUpdate(dt, input) {
      s.jolt = Math.max(0, s.jolt - dt * 2);
      s.upA += ((s.up ? Math.PI * 1.5 : 0) - s.upA) * Math.min(1, dt * 6);
      s.dnA += ((s.dn ? Math.PI * 1.5 : 0) - s.dnA) * Math.min(1, dt * 6);
      if (s.up && s.dn) s.fill = Math.min(1, s.fill + dt / 1.2);
      const operate = (which) => {
        if (which === 0 && !s.up) {
          if (!s.dn) { s.jolt = 1; api.fumble(HAMMER); return; }   // upstream first: the slug of coolant hits a shut valve
          s.up = true;
        } else if (which === 2 && !s.dn) s.dn = true;
      };
      for (const [code, d] of [["ArrowLeft", -1], ["KeyA", -1], ["ArrowRight", 1], ["KeyD", 1], ["Tab", 1]]) {
        if (input.hit.has(code)) { s.keys = true; s.focus = (s.focus + d + 3) % 3; }
      }
      if (input.actionPressed && s.focus !== 1) { s.keys = true; operate(s.focus); }
      let hold = s.keys && s.focus === 1 && input.action;
      if (input.pressed) {
        const i = KIT.nearest(targets(), input.x, input.y, WHEEL + 14);
        if (i >= 0) { s.keys = false; s.focus = i; if (i !== 1) operate(i); }
      }
      if (!s.keys && s.focus === 1 && input.down && Math.hypot(input.x - BLEED.x, input.y - BLEED.top) < WHEEL + 30) hold = true;
      s.holding = hold;
      if (hold && s.fill >= 1 && s.air > 0) s.air = Math.max(0, s.air - dt / s.need);
      if (hold && s.air <= 0) s.spit = Math.min(1, s.spit + dt * 2);
      else s.spit = Math.max(0, s.spit - dt);
      if (s.air <= 0 && s.fill >= 1) {
        s.settled += dt;
        if (s.settled > 0.4 && !s.played) { s.played = true; api.stepDone(); }
      }
    }
    function wheel(g, x, y, a, open, focus, t) {
      g.strokeStyle = "#8796aa"; g.lineWidth = 6; g.beginPath(); g.moveTo(x, y); g.lineTo(x, y - 64); g.stroke();
      bowtie(g, x, y, false, !open, { r: 20 });
      const wy = y - 64;
      D.ring(g, x, wy, WHEEL - 6, open ? C.ok : "#c0392b", 9);
      g.strokeStyle = "#c9d3e0"; g.lineWidth = 6;
      for (let k = 0; k < 3; k++) { const b = a + (k * Math.PI * 2) / 3; g.beginPath(); g.moveTo(x, wy); g.lineTo(x + Math.cos(b) * (WHEEL - 10), wy + Math.sin(b) * (WHEEL - 10)); g.stroke(); }
      D.disc(g, x, wy, 9, "#2a3446");
      if (focus) { g.setLineDash([7, 5]); D.ring(g, x, wy, WHEEL + 12, C.amber, 3); g.setLineDash([]); }
    }
    function bleedDraw(g, t) {
      const leg = job.leg, jx = s.jolt > 0 ? Math.sin(t * 60) * 10 * s.jolt : 0;
      D.panel(g, 20, 92, 1240, 616, 18, "#0a0f17");
      // Where the run comes from and goes to: the core or the chiller at each end, the flow left to right.
      const src = leg === "hot" ? "core" : "chill";
      pipe(g, [[70, 600], [70, 500]], leg, { w: 22, flow: 1, t });
      pipe(g, [[1210, 500], [1210, 600]], leg, { w: 22, flow: 1, t, dim: !(s.up && s.dn) });
      for (const [x, kind] of [[70, src], [1210, src === "core" ? "chill" : "core"]]) {
        if (kind === "core") coreGlyph(g, x, 620, 34, 0); else chillGlyph(g, x, 620, 96, 52);
      }
      // The run: live up to the upstream valve; the new segment fills as the valves open; live again past downstream.
      pipe(g, [RUN[0], [UPV.x, UPV.y]], leg, { flow: 1, t });
      g.save(); g.translate(jx, 0);
      const mid = [[UPV.x, UPV.y], RUN[1], RUN[2], RUN[3], RUN[4], [DNV.x, DNV.y]];
      pipe(g, mid, leg, { flow: s.up && s.dn ? 1 : 0, t, fill: s.fill, dim: s.fill <= 0 });
      // The air pocket at the high point: what the bleed lets out.
      if (s.fill > 0.6 && s.air > 0) {
        const w = 200 * s.air;
        g.save(); D.round(g, BLEED.x - w / 2, BLEED.y - 11, w, 22, 11); g.fillStyle = "rgba(230,240,250,0.85)"; g.fill();
        for (let k = 0; k < 6; k++) D.disc(g, BLEED.x - w / 2 + 12 + ((k * 37 + t * 40) % Math.max(10, w - 24)), BLEED.y + Math.sin(t * 5 + k) * 3, 4, "rgba(120,140,170,0.8)");
        g.restore();
      }
      // The joints at the new segment's ends: flanges that jump in a water hammer.
      for (const x of [UPV.x + 70, DNV.x - 70]) { g.fillStyle = s.jolt > 0 ? C.danger : "#9aa6b6"; g.fillRect(x - 5, 470, 10, 60); }
      g.restore();
      pipe(g, [[DNV.x, DNV.y], RUN[5]], leg, { flow: s.up && s.dn ? 1 : 0, t, dim: !(s.up && s.dn) });
      // Flow arrows, so up and downstream read without words.
      for (const x of [140, 1140]) { g.fillStyle = "#6f7f94"; g.beginPath(); g.moveTo(x + 16, 548); g.lineTo(x - 10, 534); g.lineTo(x - 10, 562); g.closePath(); g.fill(); }
      wheel(g, UPV.x, UPV.y, s.upA, s.up, s.keys && s.focus === 0, t);
      wheel(g, DNV.x, DNV.y, s.dnA, s.dn, s.keys && s.focus === 2, t);
      // The bleed valve: a petcock on a stem over the high point; held, it hisses air, then spits coolant.
      g.strokeStyle = "#8796aa"; g.lineWidth = 8; g.beginPath(); g.moveTo(BLEED.x, BLEED.y - 12); g.lineTo(BLEED.x, BLEED.top); g.stroke();
      const on = s.holding;
      D.disc(g, BLEED.x, BLEED.top, WHEEL - 4, on ? "#3a2a12" : "#1b2433");
      D.ring(g, BLEED.x, BLEED.top, WHEEL - 4, on ? C.amber : "#8796aa", 5);
      g.save(); g.translate(BLEED.x, BLEED.top); g.rotate(on ? Math.PI / 2 : 0);
      g.fillStyle = on ? C.amber : "#c9d3e0"; D.round(g, -26, -8, 52, 16, 8); g.fill(); g.restore();
      if (s.keys && s.focus === 1) { g.setLineDash([7, 5]); D.ring(g, BLEED.x, BLEED.top, WHEEL + 12, C.amber, 3); g.setLineDash([]); }
      if (!s.played && s.up && s.dn && s.fill >= 1 && !on) D.ring(g, BLEED.x, BLEED.top, WHEEL + 8 + 3 * Math.sin(t * 6), "rgba(79,195,247,0.8)", 3);
      if (on && s.fill > 0) {
        for (let j = 0; j < 12; j++) {
          const ph = (t * 2.5 + j / 12) % 1, a = -Math.PI / 2 + (j % 2 ? 1 : -1) * (0.3 + 0.5 * ((j * 0.37) % 1));
          const x = BLEED.x + 28 + Math.cos(a) * ph * 70, y = BLEED.top - 6 + Math.sin(a) * ph * 70;
          D.disc(g, x, y, 5 + 8 * ph, s.air > 0 ? `rgba(235,242,250,${0.75 * (1 - ph)})` : `rgba(${leg === "hot" ? "255,150,120" : "120,190,250"},${0.9 * (1 - ph)})`);
        }
      }
      // The gauge: the needle wanders while there is air in the run and settles in its band once it is out.
      D.disc(g, GAUGE.x, GAUGE.y, GAUGE.r + 10, "#0b1119"); D.ring(g, GAUGE.x, GAUGE.y, GAUGE.r + 10, "#3a4658", 3);
      const a0 = Math.PI * 0.8, a1 = Math.PI * 2.2, au = (u) => lerp(a0, a1, u);
      g.beginPath(); g.arc(GAUGE.x, GAUGE.y, GAUGE.r - 6, au(0.55), au(0.7)); g.strokeStyle = "rgba(61,220,132,0.8)"; g.lineWidth = 12; g.stroke();
      for (let k = 0; k <= 10; k++) { const a = au(k / 10); g.strokeStyle = "#4a5568"; g.lineWidth = 2; g.beginPath(); g.moveTo(GAUGE.x + Math.cos(a) * (GAUGE.r - 22), GAUGE.y + Math.sin(a) * (GAUGE.r - 22)); g.lineTo(GAUGE.x + Math.cos(a) * (GAUGE.r - 14), GAUGE.y + Math.sin(a) * (GAUGE.r - 14)); g.stroke(); }
      const base = s.fill * 0.62, wob = s.fill >= 1 ? s.air * 0.22 * Math.sin(t * 9) + s.air * 0.1 * Math.sin(t * 23) : 0;
      const na = au(clamp(base + wob, 0, 1));
      g.strokeStyle = s.air <= 0 && s.fill >= 1 ? C.ok : C.fg; g.lineWidth = 5; g.lineCap = "round";
      g.beginPath(); g.moveTo(GAUGE.x, GAUGE.y); g.lineTo(GAUGE.x + Math.cos(na) * (GAUGE.r - 18), GAUGE.y + Math.sin(na) * (GAUGE.r - 18)); g.stroke(); g.lineCap = "butt";
      D.disc(g, GAUGE.x, GAUGE.y, 8, "#8796aa");
      if (s.air <= 0 && s.fill >= 1) { g.strokeStyle = C.ok; g.lineWidth = 5; g.beginPath(); g.moveTo(GAUGE.x - 14, GAUGE.y + 40); g.lineTo(GAUGE.x - 3, GAUGE.y + 51); g.lineTo(GAUGE.x + 16, GAUGE.y + 30); g.stroke(); }
      // The order, as a ghost under each valve: 1 downstream, 2 upstream, 3 the bleed (numbers, no sentences).
      const badge = (x, y, n, next) => D.orderBadge(g, x, y, WHEEL - 14, n, next, t);
      if (!s.dn) badge(DNV.x, DNV.y - 64, 1, true);
      if (!s.up) badge(UPV.x, UPV.y - 64, 2, s.dn);
      if (!s.played) badge(BLEED.x, BLEED.top, 3, s.up && s.dn);
    }

    // ================================================================ the part step: the new spool
    const RACK = { x: 860, y: 120, w: 380, h: 560 };
    const GAP = { x: 420, y: 400, w: 240 };      // the burst section cut out of the run
    const spoolHome = (i) => [RACK.x + RACK.w / 2, RACK.y + 90 + i * 128];
    function partStep(r) {
      // Two of each leg's spool, in a seeded order on the rack.
      const legs = ["hot", "cold", "hot", "cold"];
      for (let i = legs.length - 1; i > 0; i--) { const j = Math.floor(r() * (i + 1)); [legs[i], legs[j]] = [legs[j], legs[i]]; }
      s = { kind: "part", spools: legs.map((leg, i) => ({ leg, x: spoolHome(i)[0], y: spoolHome(i)[1], held: false })), held: -1, focus: 0, keys: false, set: false, refuse: 0, dx: 0, dy: 0 };
    }
    function partUpdate(dt, input) {
      s.refuse = Math.max(0, s.refuse - dt);
      s.spools.forEach((sp, i) => { if (!sp.held && !(s.set && sp.fitted)) { const [hx, hy] = spoolHome(i); sp.x += (hx - sp.x) * Math.min(1, dt * 10); sp.y += (hy - sp.y) * Math.min(1, dt * 10); } });
      if (s.set) return;
      const drop = (i) => {
        const sp = s.spools[i];
        if (sp.leg !== job.leg) { s.refuse = 0.8; api.say("Wrong leg"); return; }
        sp.fitted = true; sp.x = GAP.x + GAP.w / 2; sp.y = GAP.y; s.set = true; api.stepDone();
      };
      for (const [code, d] of [["ArrowUp", -1], ["KeyW", -1], ["ArrowDown", 1], ["KeyS", 1], ["Tab", 1]]) {
        if (input.hit.has(code)) { s.keys = true; s.focus = (s.focus + d + s.spools.length) % s.spools.length; }
      }
      if (input.actionPressed) { s.keys = true; drop(s.focus); return; }
      if (input.pressed) {
        const i = KIT.nearest(s.spools, input.x, input.y, 70);
        if (i >= 0) { s.held = i; s.spools[i].held = true; s.dx = s.spools[i].x - input.x; s.dy = s.spools[i].y - input.y; s.keys = false; }
      }
      if (s.held >= 0 && input.down) { const sp = s.spools[s.held]; sp.x = input.x + s.dx; sp.y = input.y + s.dy; }
      if (s.held >= 0 && !input.down) {
        const i = s.held, sp = s.spools[i]; sp.held = false; s.held = -1;
        if (Math.abs(sp.x - (GAP.x + GAP.w / 2)) < 110 && Math.abs(sp.y - GAP.y) < 70) drop(i);
      }
    }
    function spoolGlyph(g, x, y, leg, w = 220) {
      D.round(g, x - w / 2, y - 20, w, 40, 8); g.fillStyle = "#566375"; g.fill(); g.lineWidth = 2; g.strokeStyle = "#8796aa"; g.stroke();
      g.fillStyle = "#9aa6b6"; g.fillRect(x - w / 2 - 6, y - 30, 12, 60); g.fillRect(x + w / 2 - 6, y - 30, 12, 60);
      g.fillStyle = leg === "hot" ? HOT : COLD;
      for (const bx of [x - w / 2 + 24, x + w / 2 - 44]) g.fillRect(bx, y - 20, 20, 40);
      for (let k = -2; k <= 2; k++) {
        const cx = x + k * 26;
        if (leg === "hot") { g.beginPath(); g.moveTo(cx - 6, y - 9); g.lineTo(cx + 5, y); g.lineTo(cx - 6, y + 9); g.strokeStyle = HOT; g.lineWidth = 4; g.stroke(); }
        else D.disc(g, cx, y, 5, COLD);
      }
    }
    function partDraw(g, t) {
      D.panel(g, 20, 92, 820, 616, 18, "#0a0f17");
      // The run with its burst section cut out: the flanges either side, the drip pan under it.
      pipe(g, [[60, GAP.y], [GAP.x, GAP.y]], null, { dim: true });
      pipe(g, [[GAP.x + GAP.w, GAP.y], [800, GAP.y]], null, { dim: true });
      g.fillStyle = "#9aa6b6"; g.fillRect(GAP.x - 6, GAP.y - 30, 12, 60); g.fillRect(GAP.x + GAP.w - 6, GAP.y - 30, 12, 60);
      if (!s.set) { g.setLineDash([10, 8]); D.round(g, GAP.x + 6, GAP.y - 26, GAP.w - 12, 52, 8); g.strokeStyle = "rgba(232,238,246,0.5)"; g.lineWidth = 3; g.stroke(); g.setLineDash([]); }
      // Which leg: the run's own stripes at both ends (hot chevrons or cold dots).
      for (const x of [140, 700]) {
        for (let k = -1; k <= 1; k++) {
          const cx = x + k * 26;
          if (job.leg === "hot") { g.beginPath(); g.moveTo(cx - 6, GAP.y - 9); g.lineTo(cx + 5, GAP.y); g.lineTo(cx - 6, GAP.y + 9); g.strokeStyle = HOT; g.lineWidth = 4; g.stroke(); }
          else D.disc(g, cx, GAP.y, 5, COLD);
        }
      }
      g.fillStyle = "#141b27"; g.fillRect(GAP.x - 30, 560, GAP.w + 60, 18);
      D.panel(g, RACK.x, RACK.y, RACK.w, RACK.h, 16, "#0c121a");
      for (let i = 0; i < 4; i++) { g.fillStyle = "#1d2636"; g.fillRect(RACK.x + 20, spoolHome(i)[1] + 38, RACK.w - 40, 8); }
      s.spools.forEach((sp, i) => { if (!sp.held) spoolGlyph(g, sp.x, sp.y, sp.leg); if (s.keys && s.focus === i && !s.set) { g.setLineDash([7, 5]); D.round(g, sp.x - 130, sp.y - 44, 260, 88, 12); g.strokeStyle = C.amber; g.lineWidth = 3; g.stroke(); g.setLineDash([]); } });
      s.spools.forEach((sp) => { if (sp.held) spoolGlyph(g, sp.x, sp.y, sp.leg); });
      if (s.refuse > 0) { const x = GAP.x + GAP.w / 2, y = GAP.y; g.strokeStyle = C.danger; g.lineWidth = 6; g.beginPath(); g.moveTo(x - 24, y - 24); g.lineTo(x + 24, y + 24); g.moveTo(x + 24, y - 24); g.lineTo(x - 24, y + 24); g.stroke(); }
    }

    // ================================================================ the job
    return {
      /** For tools (tests and shots): the round's state, read only. */
      peek() {
        if (!s) return null;
        const base = { kind: s.kind, leg: job.leg, seg: job.seg };
        if (s.kind === "isolate") return { ...base, shut: [...s.shut], need: isoNeed(), valves: VALVES.map((v) => ({ id: v.id, x: v.x, y: v.y, main: !!v.main, leg: v.leg })), done: s.done };
        if (s.kind === "rebuild") {
          const net = network();
          return { ...base, round: s.round, cols: s.cols, rows: s.rows, T: s.T, gx: s.gx, gy: s.gy, phase: s.phase,
            tiles: s.tiles.map((tl, i) => ({ type: tl.type, k: tl.k % 4, sol: tl.sol, solK: tl.solK, cracked: tl.cracked,
              leg: [0, 1].map((gi) => net.legOf.get(`${i % s.cols},${Math.floor(i / s.cols)},${gi}`) || null) })),
            tray: TRAY.map((type, i) => ({ type, xy: slotXY(i) })), fill: FILL_BTN, ok: net.ok, ends: s.ends,
            front: s.front, faults: (s.net || net).faults.map((f) => ({ kind: f.kind, leg: f.leg, d: f.d, at: f.at })), reach: { ...(s.net || net).reach }, end: (s.net || net).end };
        }
        if (s.kind === "bleed") return { ...base, up: s.up, dn: s.dn, fill: s.fill, air: s.air, played: s.played, need: s.need, targets: targets() };
        return { ...base, set: s.set, spools: s.spools.map((sp) => ({ leg: sp.leg, x: sp.x, y: sp.y })), gap: [GAP.x + GAP.w / 2, GAP.y] };
      },
      step(index, isPart) {
        const r = api.rand();
        if (!job) job = { leg: r() < 0.5 ? "hot" : "cold", seg: r() < 0.5 ? 2 : 3 };
        if (shares[index] === undefined) shares[index] = (100 - api.value) / Math.max(1, api.steps - index) / (KIT.RATES[api.who] || KIT.RATES.officer);
        const plan = api.steps >= 5 ? PLAN_DISABLED : PLAN_DAMAGED;
        const kind = isPart ? "part" : plan[Math.min(index, plan.length - 1)];
        if (kind === "part") partStep(r);
        else if (kind === "isolate") isoStep();
        else if (kind === "rebuild") rebuildStep(r, plan.slice(0, index).filter((k) => k === "rebuild").length);
        else bleedStep(index);
      },
      update(dt, input) {
        if (s.kind === "isolate") isoUpdate(dt, input);
        else if (s.kind === "rebuild") rebuildUpdate(dt, input);
        else if (s.kind === "bleed") bleedUpdate(dt, input);
        else partUpdate(dt, input);
      },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        if (s.kind === "isolate") isoDraw(g, t);
        else if (s.kind === "rebuild") rebuildDraw(g, t);
        else if (s.kind === "bleed") bleedDraw(g, t);
        else partDraw(g, t);
      },
    };
  },
});
