/*
 * repairs/shuttle.js: the Petrel's fuel line repair, a pressure test (openspec/changes/repair-minigames, design 2; the
 * Petrel is shuttle-bay-and-fighters section 8).
 *
 * The Petrel from above, nose right, its fuel lines drawn on the hull: the tank amidships feeding the two main engines
 * aft and the four RCS quads. The line is under test pressure and leaks somewhere. Close a valve and the gauge says
 * which side of it the leak is: holding (DOWN: the leak is past the valve, cut off) or falling (UP: on the tank's side
 * of it, or in another branch). Work out from the tank one valve at a time: the next valve along the line from the last
 * one that held. A valve that falls clears its branch (it greys, with a tick); the trail to the leak shows amber. Once
 * the leak is boxed in (the valves round its section all shut) its mist shows: drag the patch onto it, then pump the
 * line back up and hold the needle in the green band for 3 s. A valve out of order (past an untested valve, or on a
 * cleared branch) vents fuel mist into the bay. Later steps have more valves, a deeper leak and a narrower band. A
 * disabled shuttle's first step fits the new isolation valve into the gap in the main line.
 * Keys: arrows choose a valve, Space closes it, fits the patch and pumps (held).
 */
RepairKit.register({
  id: "shuttle",
  title: "Shuttle",
  place: "Hangar, the Petrel on its pad",
  group: "Hangar",
  hazard: "Fuel mist: the bay's fire risk rises",
  down: "The Petrel cannot launch",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    // The fuel lines on the Petrel from above. A valve names its parent (-1: the tank) and the line from the parent to
    // it; a valve with no children also names the lines on to what it feeds.
    const AFT = [450, 410], FWD = [630, 410];
    const TO_V2 = [[395, 410], [340, 410], [340, 320], [290, 320]], TO_V3 = [[395, 410], [340, 410], [340, 500], [290, 500]];
    const FWD_P = [[690, 410], [740, 410], [740, 300], [800, 300]], FWD_S = [[690, 410], [740, 410], [740, 520], [800, 520]];
    const FULL = [
      { parent: -1, route: [AFT, [395, 410]] },
      { parent: 0, route: TO_V2 },
      { parent: 0, route: TO_V3 },
      { parent: 1, route: [[290, 320], [180, 320]], out: [[[180, 320], [72, 320]]] },
      { parent: 2, route: [[290, 500], [180, 500]], out: [[[180, 500], [72, 500]]] },
      { parent: 1, route: [[290, 320], [250, 320], [250, 275]], out: [[[250, 275], [250, 238]]] },
      { parent: 2, route: [[290, 500], [250, 500], [250, 545]], out: [[[250, 545], [250, 582]]] },
      { parent: -1, route: [FWD, [690, 410]] },
      { parent: 7, route: FWD_P, out: [[[800, 300], [840, 300], [840, 270]]] },
      { parent: 7, route: FWD_S, out: [[[800, 520], [840, 520], [840, 550]]] },
    ];
    const SIMPLE = [
      { parent: -1, route: [AFT, [395, 410]] },
      { parent: 0, route: TO_V2, out: [[[290, 320], [72, 320]], [[290, 320], [250, 320], [250, 238]]] },
      { parent: 0, route: TO_V3, out: [[[290, 500], [72, 500]], [[290, 500], [250, 500], [250, 582]]] },
      { parent: -1, route: [FWD, [690, 410]], out: [[...FWD_P, [840, 300], [840, 270]], [...FWD_S, [840, 520], [840, 550]]] },
    ];
    const HULL = [[968, 410], [946, 322], [892, 262], [760, 222], [210, 210], [92, 222], [60, 272], [60, 548], [92, 598],
      [210, 610], [760, 598], [892, 558], [946, 498]];
    const RCS = [[250, 230], [250, 590], [840, 262], [840, 558]];
    const G = { x: 1128, y: 250, r: 96 };               // the gauge
    const PUMP = { x: 1128, y: 524, r: 58 };
    const KITBOX = { x: 1128, y: 640 };                  // the patch kit (and the part step's crate)
    const BAND = 0.6;                                    // the hold band's centre, of the gauge's full scale
    let valves, phase, partStep, current, res, closed, busy, lamp, leak, p, flow, hold, hw, patch, puffs, kf, fi, isolated, decay, clock;
    const near = (ax, ay, bx, by, r) => Math.hypot(ax - bx, ay - by) < r;
    const wrapI = (i, n) => ((i % n) + n) % n;
    const pos = (v) => v.route[v.route.length - 1];
    const kids = (i) => valves.map((v, k) => k).filter((k) => valves[k].parent === i);
    /** Is region `owner` (a valve's outflow, or -1 the tank's) past valve `v`? */
    const past = (owner, v) => { for (let o = owner; o !== -1; o = valves[o].parent) if (o === v) return true; return false; };
    /** The lines a valve's outflow holds: its children's lines and what it feeds. */
    const region = (o) => [...kids(o).map((k) => valves[k].route), ...(o >= 0 ? valves[o].out || [] : [])];
    /** A point a fraction of the way along a polyline, and whether that stretch runs up and down. */
    function along(line, f) {
      const lens = line.slice(1).map((q, i) => Math.hypot(q[0] - line[i][0], q[1] - line[i][1]));
      let d = f * lens.reduce((a, b) => a + b, 0);
      for (let i = 0; i < lens.length; i++) {
        if (d <= lens[i] || i === lens.length - 1) {
          const k = Math.min(1, d / lens[i]), a = line[i], b = line[i + 1];
          return { x: a[0] + (b[0] - a[0]) * k, y: a[1] + (b[1] - a[1]) * k, vert: Math.abs(b[0] - a[0]) < 1 };
        }
        d -= lens[i];
      }
    }
    function puff(x, y) { puffs.push({ x, y, t: 0 }); }

    /** Close valve i: in order, it tests; out of order, it vents. */
    function close(i) {
      if (busy > 0 || res[i]) return;
      if (valves[i].parent !== current) { const [x, y] = pos(valves[i]); puff(x, y); api.fumble("Fuel mist: the bay's fire risk rises"); return; }
      closed = i; busy = 0.6; lamp = null;
    }
    /** The closed valve's verdict, once the pressure has had a moment to show it. */
    function verdict() {
      const i = closed;
      if (past(leak.owner, i)) { res[i] = "down"; lamp = "down"; current = i; }
      else { res[i] = "up"; lamp = "up"; }
      const ks = kids(current);
      if (ks.length === 0 || ks.every((k) => res[k] === "up")) {
        isolated = [current, ...ks].filter((k) => k >= 0);
        phase = "patch"; fi = 0;
      }
    }

    return {
      step(index, isPart) {
        const r = api.rand();
        valves = isPart || index === 0 ? SIMPLE : FULL;
        partStep = isPart; clock = 0;
        current = -1; res = {}; closed = -1; busy = 0; lamp = null; isolated = [];
        p = 0.7; flow = 0; hold = 0; hw = Math.max(0.045, 0.09 - 0.015 * index); decay = 0.1 + 0.025 * index;
        puffs = []; kf = false; fi = 0;
        patch = { x: KITBOX.x, y: KITBOX.y, held: false, set: false, gx: 0, gy: 0 };
        if (isPart) { phase = "part"; p = 0; return; }
        phase = "find";
        // The leak: a section at least `index` valves deep (as deep as the lines go).
        const depth = (o) => { let n = 0; for (; o !== -1; o = valves[o].parent) n++; return n; };
        const owners = [-1, ...valves.map((v, k) => k)];
        const maxD = Math.max(...owners.map(depth));
        const pool = owners.filter((o) => depth(o) >= Math.min(index, maxD));
        const owner = pool[Math.floor(r() * pool.length)];
        const lines = region(owner);
        const at = along(lines[Math.floor(r() * lines.length)], 0.3 + 0.4 * r());
        leak = { owner, ...at };
      },
      update(dt, input) {
        const h = input.hit, ease = Math.min(1, dt * 12);
        const dir = (h.has("ArrowRight") || h.has("KeyD") || h.has("ArrowDown") || h.has("KeyS") ? 1 : 0)
          - (h.has("ArrowLeft") || h.has("KeyA") || h.has("ArrowUp") || h.has("KeyW") ? 1 : 0);
        if (dir || input.actionPressed) kf = true;
        if (input.pressed) kf = false;
        clock += dt;
        for (const q of puffs) q.t += dt;
        puffs = puffs.filter((q) => q.t < 1.4);
        const dragPatch = (tx, ty, onDrop) => {
          if (input.pressed && Math.abs(input.x - patch.x) < 44 && Math.abs(input.y - patch.y) < 30) { patch.held = true; patch.gx = input.x - patch.x; patch.gy = input.y - patch.y; }
          if (patch.held && input.down) { patch.x = input.x - patch.gx; patch.y = input.y - patch.gy; }
          if (patch.held && input.released) { patch.held = false; if (near(patch.x, patch.y, tx, ty, 44)) return onDrop(); }
          if (!patch.held) { patch.x += (KITBOX.x - patch.x) * ease; patch.y += (KITBOX.y - patch.y) * ease; }
          if (kf && input.actionPressed) onDrop();
        };

        if (phase === "part") {
          const [x, y] = pos(valves[0]);
          dragPatch(x, y, () => { patch.set = true; patch.x = x; patch.y = y; phase = "done"; api.stepDone(); });
          return;
        }
        if (phase === "find") {
          // The test rig tops the line up as a valve shuts; with the leak still on the tank's side it falls again.
          const falling = closed < 0 || !past(leak.owner, closed);
          if (busy > 0) p += (0.7 - p) * dt * 6;
          else if (falling) p = Math.max(0.15, p - 0.1 * dt);
          else p += (0.7 - p) * dt * 2;
          if (busy > 0 && (busy -= dt) <= 0) return verdict();
          if (kf) { fi = wrapI(fi + dir, valves.length); if (input.actionPressed) close(fi); }
          // The nearest valve within a fingertip's reach; no two valves are closer than 60 px (KIT.nearest).
          else if (input.pressed) { const i = KIT.nearest(valves.map(pos), input.x, input.y, KIT.TOUCH_R + 6); if (i >= 0) close(i); }
          return;
        }
        if (phase === "patch") {
          p += (0.7 - p) * dt * 2;
          dragPatch(leak.x, leak.y, () => { patch.set = true; patch.held = false; patch.x = leak.x; patch.y = leak.y; phase = "hold"; p = 0.2; closed = -1; });
          return;
        }
        if (phase === "hold") {
          const pumping = input.keys.has("Space") || input.keys.has("Enter") || input.keys.has("ArrowUp") || input.keys.has("KeyW")
            || (input.down && near(input.x, input.y, PUMP.x, PUMP.y, PUMP.r));
          flow += ((pumping ? 0.36 : 0) - flow) * Math.min(1, dt * 4);
          p = Math.max(0, Math.min(1, p + (flow - decay - 0.03 * Math.sin(clock * 1.1)) * dt));
          if (Math.abs(p - BAND) <= hw) hold += dt; else hold = 0;
          if (hold >= 3) { hold = 3; phase = "done"; api.stepDone(); }
        }
      },
      /** For tools: the round's state, read only, so a script can play it. */
      peek() { return { phase, partStep, valves, current, res, leak, p, hold, band: BAND, hw, patch, PUMP, KITBOX }; },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        drawHull(g);
        // The lines: grey when cleared, amber along the trail to the leak, red dashed round it once boxed in.
        const cleared = (o) => Object.keys(res).some((k) => res[k] === "up" && past(o, +k));
        const trail = new Set(); for (let o = current; o !== -1; o = valves[o].parent) trail.add(o);
        const lineOf = (pts, color, w = 6, dash) => {
          g.beginPath(); pts.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y)));
          g.setLineDash(dash || []); g.strokeStyle = color; g.lineWidth = w; g.lineJoin = "round"; g.stroke(); g.setLineDash([]);
        };
        const boxed = phase === "patch" || ((phase === "hold" || phase === "done") && !patch.set);
        valves.forEach((v, i) => {
          const owner = v.parent, lit = trail.has(i);
          const col = cleared(owner) ? "#1f2836" : lit ? C.amber : "#4a5a70";
          // The part step: a gap in the main line where the new valve goes.
          const route = !partStep || patch.set ? v.route : i === 0 ? [AFT, [417, 410]] : v.parent === 0 ? [[373, 410], ...v.route.slice(1)] : v.route;
          lineOf(route, col);
          for (const o of v.out || []) lineOf(o, cleared(i) ? "#1f2836" : "#4a5a70");
        });
        if (boxed) for (const l of region(leak.owner)) lineOf(l, C.danger, 6, [12, 8]);
        // The tank.
        D.round(g, 450, 372, 180, 76, 38); g.fillStyle = "#1d2738"; g.fill(); g.strokeStyle = C.steel; g.lineWidth = 3; g.stroke();
        D.text(g, "FUEL", 540, 410, 20, C.dim, "center", 700);
        // The valves: a bow tie, hollow open, solid shut.
        valves.forEach((v, i) => {
          const [x, y] = pos(v), prev = v.route[v.route.length - 2], vert = Math.abs(prev[0] - x) < 1;
          if (partStep && i === 0 && !patch.set) {
            g.setLineDash([6, 5]); bowtie(g, x, y, vert, null, C.amber); g.setLineDash([]);
            return;
          }
          const shut = (closed === i && phase === "find") || (boxed && isolated.includes(i));
          bowtie(g, x, y, vert, shut ? "#c9d3df" : "#0b111b", res[i] === "up" ? "#3a4558" : "#c9d3df");
          if (res[i] === "up") tick(g, x + 16, y - 20);
          if (closed === i && busy > 0) D.ring(g, x, y, 22 + 6 * Math.sin(t * 12), C.amber, 2);
          if (kf && phase === "find" && fi === i) { g.setLineDash([6, 5]); D.ring(g, x, y, 26, C.amber, 3); g.setLineDash([]); }
        });
        // The leak's mist, once it is boxed in, until it is patched.
        if (boxed && !patch.set) for (let k = 0; k < 7; k++) {
          const a = t * 1.3 + k * 0.9, rr = 10 + ((t * 30 + k * 13) % 40);
          D.disc(g, leak.x + Math.cos(a) * rr * 0.8, leak.y + Math.sin(a) * rr * 0.6 - rr * 0.4, 7 + rr * 0.25, `rgba(255,197,66,${0.35 * (1 - rr / 50)})`);
        }
        for (const q of puffs) for (let k = 0; k < 6; k++) {
          const a = k * 1.05, rr = 8 + q.t * 50;
          D.disc(g, q.x + Math.cos(a) * rr, q.y + Math.sin(a) * rr, 6 + q.t * 10, `rgba(255,197,66,${0.4 * (1 - q.t / 1.4)})`);
        }
        drawPanel(g, t);
        if (partStep) { if (!patch.set) drawValvePart(g, patch.x, patch.y); }
        else drawPatch(g, patch.x, patch.y, patch.set ? leak.vert : false);
      },
    };

    function bowtie(g, x, y, vert, fill, stroke) {
      g.beginPath();
      if (vert) { g.moveTo(x - 12, y - 14); g.lineTo(x + 12, y - 14); g.lineTo(x - 12, y + 14); g.lineTo(x + 12, y + 14); }
      else { g.moveTo(x - 14, y - 12); g.lineTo(x - 14, y + 12); g.lineTo(x + 14, y - 12); g.lineTo(x + 14, y + 12); }
      g.closePath();
      if (fill) { g.fillStyle = fill; g.fill(); }
      g.strokeStyle = stroke; g.lineWidth = 3; g.lineJoin = "round"; g.stroke();
    }
    function tick(g, x, y) {
      D.disc(g, x, y, 10, "#0b111b");
      g.strokeStyle = C.ok; g.lineWidth = 3; g.beginPath(); g.moveTo(x - 5, y); g.lineTo(x - 1, y + 4); g.lineTo(x + 6, y - 5); g.stroke();
    }
    /** The Petrel from above: hull, cockpit, engines, RCS quads, side hatch. */
    function drawHull(g) {
      g.beginPath(); HULL.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y))); g.closePath();
      g.fillStyle = "#0f1622"; g.fill(); g.strokeStyle = "#2c3a52"; g.lineWidth = 3; g.stroke();
      g.strokeStyle = "#151e2c"; g.lineWidth = 2;
      for (const x of [330, 700]) { g.beginPath(); g.moveTo(x, 214); g.lineTo(x, 606); g.stroke(); }
      for (const y of [-1, 1]) {
        g.beginPath(); g.moveTo(60, 410 + y * 60); g.lineTo(40, 410 + y * 40); g.lineTo(40, 410 + y * 140); g.lineTo(60, 410 + y * 120); g.closePath();
        g.fillStyle = "#1b2433"; g.fill(); g.strokeStyle = C.steel; g.stroke();
      }
      g.fillStyle = "#1d3b52";
      for (const [y0, y1] of [[350, 392], [428, 470]]) { g.beginPath(); g.moveTo(912, y0); g.lineTo(944, y0 + 12); g.lineTo(944, y1 - 12); g.lineTo(912, y1); g.closePath(); g.fill(); }
      for (const [x, y] of RCS) {
        g.fillStyle = "#1b2433"; g.fillRect(x - 9, y - 9, 18, 18); g.strokeStyle = C.steel; g.lineWidth = 2; g.strokeRect(x - 9, y - 9, 18, 18);
      }
      g.fillStyle = "#080c12"; g.fillRect(600, 224, 64, 8);
      D.text(g, "PETREL", 520, 650, 18, C.dim, "center", 700);
    }
    /** The test panel: the gauge, the two lamps, the pump, the patch kit. */
    function drawPanel(g, t) {
      D.panel(g, 1000, 92, 256, 608, 18, "#0a0f17");
      const ang = (v) => (150 + 240 * v) * Math.PI / 180;
      D.disc(g, G.x, G.y, G.r, "#0b111b"); D.ring(g, G.x, G.y, G.r, C.steel, 3);
      for (let k = 0; k <= 10; k++) {
        const a = ang(k / 10), r0 = G.r - (k % 5 ? 10 : 18);
        g.beginPath(); g.moveTo(G.x + Math.cos(a) * r0, G.y + Math.sin(a) * r0); g.lineTo(G.x + Math.cos(a) * (G.r - 4), G.y + Math.sin(a) * (G.r - 4));
        g.strokeStyle = "#4a5a70"; g.lineWidth = 2; g.stroke();
      }
      g.setLineDash([5, 4]); g.beginPath(); g.arc(G.x, G.y, G.r - 12, ang(0.9), ang(1)); g.strokeStyle = C.danger; g.lineWidth = 8; g.stroke(); g.setLineDash([]);
      if (phase === "hold" || phase === "done") {
        g.beginPath(); g.arc(G.x, G.y, G.r - 14, ang(BAND - hw), ang(BAND + hw)); g.strokeStyle = C.ok; g.lineWidth = 14; g.stroke();
        g.beginPath(); g.arc(G.x, G.y, G.r + 10, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * hold / 3); g.strokeStyle = C.ok; g.lineWidth = 5; g.stroke();
      }
      const a = ang(p);
      g.beginPath(); g.moveTo(G.x, G.y); g.lineTo(G.x + Math.cos(a) * (G.r - 16), G.y + Math.sin(a) * (G.r - 16));
      g.strokeStyle = C.fg; g.lineWidth = 4; g.lineCap = "round"; g.stroke(); g.lineCap = "butt";
      D.disc(g, G.x, G.y, 9, C.steel);
      // The lamps: which side of the closed valve the leak is.
      for (const [x, which, up] of [[1072, "up", true], [1184, "down", false]]) {
        const on = lamp === which;
        D.panel(g, x - 50, 372, 100, 48, 12, on ? "#3a2a12" : "#0b111b", on ? C.amber : C.line);
        g.beginPath();
        if (up) { g.moveTo(x - 38, 405); g.lineTo(x - 26, 387); g.lineTo(x - 14, 405); } else { g.moveTo(x - 38, 387); g.lineTo(x - 26, 405); g.lineTo(x - 14, 387); }
        g.closePath(); g.fillStyle = on ? C.amber : "#2a3446"; g.fill();
        D.text(g, up ? "UP" : "DOWN", x + 14, 396, 18, on ? C.amber : C.dim, "center", 700);
      }
      // The pump: the round's one action once the patch is on.
      const live = phase === "hold";
      D.disc(g, PUMP.x, PUMP.y, PUMP.r, live ? "#262e42" : "#121822");
      D.ring(g, PUMP.x, PUMP.y, PUMP.r, live && flow > 0.18 ? C.amber : live ? C.steel : C.line, 4);
      D.text(g, "PUMP", PUMP.x, PUMP.y, 24, live ? C.fg : "#3a4558", "center", 700);
      D.panel(g, KITBOX.x - 80, KITBOX.y - 36, 160, 72, 12, "#141b27");
    }
    function drawPatch(g, x, y, vert) {
      g.save(); g.translate(x, y); if (vert) g.rotate(Math.PI / 2);
      D.round(g, -30, -13, 60, 26, 8); g.fillStyle = C.copper; g.fill(); g.strokeStyle = "#f0c08a"; g.lineWidth = 2; g.stroke();
      g.fillStyle = "#7a4a22"; g.fillRect(-20, -13, 6, 26); g.fillRect(14, -13, 6, 26);
      g.restore();
    }
    function drawValvePart(g, x, y) {
      g.fillStyle = C.steel; g.fillRect(x - 24, y - 16, 6, 32); g.fillRect(x + 18, y - 16, 6, 32);
      bowtie(g, x, y, false, "#c9d3df", C.accent);
    }
  },
});
