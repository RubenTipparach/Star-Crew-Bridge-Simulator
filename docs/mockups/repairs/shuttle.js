/*
 * repairs/shuttle.js: the Petrel's fuel line repair: find the leak, isolate it, patch it, prove it (openspec/changes/
 * repair-minigames, design 2 and 6f; the Petrel is shuttle-bay-and-fighters section 8).
 *
 * The Petrel from above, nose right, its fuel lines drawn on the hull as pipes full of fuel, flowing out of the tank
 * amidships to the two main engines aft and the four RCS quads. One line leaks somewhere. A round is four stages:
 * 1. Find: drag the leak sniffer along the lines (it rides the nearest line under the finger). Its ring pulses faster,
 *    bigger and warmer (blue to red) the nearer the leak, the panel's signal bars rise with it, and a faint mist shows
 *    once it is within reach. Hold it on the leak a moment and the leak is marked. No mistake is possible here.
 * 2. Isolate: tap the valve on the tank's side of the leak. It shuts (drawn solid), the lines past it empty (drawn
 *    hollow) and the mist stops. A valve that leaves the leak still fed (past the leak, or on another line) vents fuel
 *    mist into the bay, a fumble, and springs back open.
 * 3. Patch: drag the patch from the kit onto the leak.
 * 4. Pressure test: tap the shut valve to open it again, then hold the pump and keep the needle in the green band for
 *    3 s (leaving the band restarts the 3 s).
 * Later steps put the leak deeper in the lines and narrow the band. A disabled shuttle's first step fits the new
 * isolation valve into the gap in the main line.
 * The kit owns the pointer, touch reach (KIT.TOUCH_R, KIT.nearest) and the drawing helpers (KIT.draw); this file owns
 * only the Petrel's lines and this game's rules.
 * Keys: arrows move the sniffer along the lines, then pick a valve; Space closes the valve, fits the patch, opens the
 * valve and pumps while held.
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
    const SIG = { x: 1128, y: 396 };                     // the sniffer's signal bars
    const PUMP = { x: 1128, y: 524, r: 58 };
    const KITBOX = { x: 1128, y: 640 };                  // the patch kit (and the part step's crate)
    const BAND = 0.6;                                    // the hold band's centre, of the gauge's full scale
    const SNIFF = { home: [425, 410], heat: 360, mist: 90, lock: 22, lockS: 0.45, keys: 260 };   // px, px/s, s
    const FUEL = "#b9822f", FUEL_LIT = "#ffd27a", EMPTY = "#070b12", PIPE = "#56657c";
    let valves, phase, partStep, res, closed, spring, drain, refill, leak, p, flow, hold, hw, patch, puffs, kf, fi, sn, lock, decay, clock;
    const near = (ax, ay, bx, by, r) => Math.hypot(ax - bx, ay - by) < r;
    const wrapI = (i, n) => ((i % n) + n) % n;
    const mix = (a, b, k) => a.map((v, i) => Math.round(v + (b[i] - v) * k));
    const pos = (v) => v.route[v.route.length - 1];
    const kids = (i) => valves.map((v, k) => k).filter((k) => valves[k].parent === i);
    /** Is region `owner` (a valve's outflow, or -1 the tank's) past valve `v`? */
    const past = (owner, v) => { for (let o = owner; o !== -1; o = valves[o].parent) if (o === v) return true; return false; };
    /** The lines a valve's outflow holds: its children's lines and what it feeds. */
    const region = (o) => [...kids(o).map((k) => valves[k].route), ...(o >= 0 ? valves[o].out || [] : [])];
    /** Every line on the ship, the sniffer's track (across the tank too, so keys can carry it fore and aft). */
    const allLines = () => [[AFT, FWD], ...valves.flatMap((v) => [v.route, ...(v.out || [])])];
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
    /** The nearest point on any line to (x, y): the sniffer rides the lines. */
    function snap(x, y) {
      let best = { x, y, d: Infinity };
      for (const line of allLines()) for (let i = 1; i < line.length; i++) {
        const [ax, ay] = line[i - 1], [bx, by] = line[i], vx = bx - ax, vy = by - ay, L = vx * vx + vy * vy || 1;
        const k = Math.max(0, Math.min(1, ((x - ax) * vx + (y - ay) * vy) / L)), qx = ax + vx * k, qy = ay + vy * k;
        const d = Math.hypot(x - qx, y - qy);
        if (d < best.d) best = { x: qx, y: qy, d };
      }
      return best;
    }
    /** How near the sniffer is to the leak, 0 (cold) to 1 (on it). */
    const warmth = () => Math.max(0, 1 - Math.hypot(sn.x - leak.x, sn.y - leak.y) / SNIFF.heat);
    function puff(x, y) { puffs.push({ x, y, t: 0 }); }

    /** Close valve i: on the tank's side of the leak it isolates it; anywhere else the leak is still fed. */
    function close(i) {
      if (spring > 0) return;
      const [x, y] = pos(valves[i]);
      if (!past(leak.owner, i)) { closed = i; spring = 0.45; puff(leak.x, leak.y); puff(x, y); api.fumble("Fuel mist: the bay's fire risk rises"); return; }
      closed = i; drain = 0; phase = "drain";
    }
    /** Lines past the shut valve, and how empty they are (0 full, 1 empty). */
    const emptied = (owner) => (closed >= 0 && spring <= 0 && past(owner, closed) ? drain * (1 - refill) : 0);

    return {
      step(index, isPart) {
        const r = api.rand();
        valves = isPart || index === 0 ? SIMPLE : FULL;
        partStep = isPart; clock = 0;
        res = {}; closed = -1; spring = 0; drain = 0; refill = 0;
        p = 0.7; flow = 0; hold = 0; hw = Math.max(0.045, 0.09 - 0.015 * index); decay = 0.1 + 0.025 * index;
        puffs = []; kf = false; fi = 0; lock = 0;
        sn = { x: SNIFF.home[0], y: SNIFF.home[1], held: false };
        patch = { x: KITBOX.x, y: KITBOX.y, held: false, set: false, gx: 0, gy: 0 };
        if (isPart) { phase = "part"; p = 0; leak = { owner: 0, x: -999, y: -999, vert: false }; return; }
        phase = "find";
        // The leak: in a valve's outflow (so some valve can cut it off), deeper in the lines on later steps.
        const depth = (o) => { let n = 0; for (; o !== -1; o = valves[o].parent) n++; return n; };
        const owners = valves.map((v, k) => k).filter((k) => region(k).length);
        const maxD = Math.max(...owners.map(depth));
        const pool = owners.filter((o) => depth(o) >= Math.min(index + 1, maxD));
        const owner = pool[Math.floor(r() * pool.length)];
        // On a straight stretch of one of its lines, clear of the bends (a patch lies flat on a straight pipe).
        const runs = region(owner).flatMap((l) => l.slice(1).map((q, i) => [l[i], q])).filter(([a, b]) => Math.hypot(b[0] - a[0], b[1] - a[1]) >= 36);
        leak = { owner, ...along(runs[Math.floor(r() * runs.length)], 0.35 + 0.3 * r()) };
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
        if (spring > 0 && (spring -= dt) <= 0) { spring = 0; closed = -1; }
        const dragPatch = (tx, ty, onDrop) => {
          if (input.pressed && Math.abs(input.x - patch.x) < 44 && Math.abs(input.y - patch.y) < 30) { patch.held = true; patch.gx = input.x - patch.x; patch.gy = input.y - patch.y; }
          if (patch.held && input.down) { patch.x = input.x - patch.gx; patch.y = input.y - patch.gy; }
          if (patch.held && input.released) { patch.held = false; if (near(patch.x, patch.y, tx, ty, 44)) return onDrop(); }
          if (!patch.held) { patch.x += (KITBOX.x - patch.x) * ease; patch.y += (KITBOX.y - patch.y) * ease; }
          if (kf && input.actionPressed) onDrop();
        };
        // The leak bleeds the line down while it is fed; the section past a shut valve empties.
        if (phase === "find" || phase === "isolate") p = Math.max(0.3, p - 0.04 * dt);

        if (phase === "part") {
          const [x, y] = pos(valves[0]);
          dragPatch(x, y, () => { patch.set = true; patch.x = x; patch.y = y; phase = "done"; api.stepDone(); });
          return;
        }
        if (phase === "find") {
          // Pick the sniffer up anywhere on the ship; it rides the line nearest the finger.
          if (input.pressed && input.x < 990) sn.held = true;
          if (!input.down) sn.held = false;
          if (sn.held) { const q = snap(input.x, input.y); sn.x = q.x; sn.y = q.y; }
          if (input.stick.x || input.stick.y) {
            kf = true;
            const q = snap(sn.x + input.stick.x * SNIFF.keys * dt, sn.y + input.stick.y * SNIFF.keys * dt); sn.x = q.x; sn.y = q.y;
          }
          if (Math.hypot(sn.x - leak.x, sn.y - leak.y) < SNIFF.lock) lock += dt; else lock = Math.max(0, lock - 2 * dt);
          if (lock >= SNIFF.lockS) { lock = SNIFF.lockS; sn.held = false; phase = "isolate"; fi = 0; }
          return;
        }
        if (phase === "isolate") {
          if (kf) { fi = wrapI(fi + dir, valves.length); if (input.actionPressed) close(fi); }
          // The nearest valve within a fingertip's reach; no two valves are closer than 60 px (KIT.nearest).
          else if (input.pressed) { const i = KIT.nearest(valves.map(pos), input.x, input.y, KIT.TOUCH_R + 6); if (i >= 0) close(i); }
          return;
        }
        if (phase === "drain") {
          drain = Math.min(1, drain + dt / 0.7);
          p += (0.05 - p) * dt * 4;
          if (drain >= 1) phase = "patch";
          return;
        }
        if (phase === "patch") {
          p += (0.05 - p) * dt * 4;
          dragPatch(leak.x, leak.y, () => { patch.set = true; patch.held = false; patch.x = leak.x; patch.y = leak.y; phase = "open"; });
          return;
        }
        if (phase === "open") {
          // Open the shut valve again: the line refills behind the patch, and the test begins.
          const [x, y] = pos(valves[closed]);
          if ((kf && input.actionPressed) || (input.pressed && KIT.nearest([[x, y]], input.x, input.y, KIT.TOUCH_R + 6) === 0)) { phase = "hold"; p = 0.2; }
          return;
        }
        if (phase === "hold") {
          refill = Math.min(1, refill + dt / 0.5);
          const pumping = input.keys.has("Space") || input.keys.has("Enter") || input.keys.has("ArrowUp") || input.keys.has("KeyW")
            || (input.down && near(input.x, input.y, PUMP.x, PUMP.y, PUMP.r));
          flow += ((pumping ? 0.36 : 0) - flow) * Math.min(1, dt * 4);
          p = Math.max(0, Math.min(1, p + (flow - decay - 0.03 * Math.sin(clock * 1.1)) * dt));
          if (Math.abs(p - BAND) <= hw) hold += dt; else hold = 0;
          if (hold >= 3) { hold = 3; phase = "done"; api.stepDone(); }
        }
      },
      /** For tools: the round's state, read only, so a script can play it. */
      peek() { return { phase, partStep, valves, closed, spring, leak, sn, lock, warmth: phase === "find" ? warmth() : 0, p, hold, band: BAND, hw, patch, PUMP, KITBOX }; },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        drawHull(g);
        // The lines as pipes: full of fuel flowing out of the tank, or hollow past a shut valve.
        valves.forEach((v, i) => {
          // The part step: a gap in the main line where the new valve goes.
          const route = !partStep || patch.set ? v.route : i === 0 ? [AFT, [417, 410]] : v.parent === 0 ? [[373, 410], ...v.route.slice(1)] : v.route;
          pipe(g, route, emptied(v.parent), t);
          for (const o of v.out || []) pipe(g, o, emptied(i), t);
        });
        // The tank.
        D.round(g, 450, 372, 180, 76, 38); g.fillStyle = "#1d2738"; g.fill(); g.strokeStyle = C.steel; g.lineWidth = 3; g.stroke();
        D.round(g, 458, 380, 164, 60, 30); g.fillStyle = "rgba(185,130,47,0.55)"; g.fill();
        D.text(g, "FUEL", 540, 410, 20, C.fg, "center", 700);
        // The valves: a bow tie, hollow open, solid shut.
        valves.forEach((v, i) => {
          const [x, y] = pos(v), prev = v.route[v.route.length - 2], vert = Math.abs(prev[0] - x) < 1;
          if (partStep && i === 0 && !patch.set) {
            g.setLineDash([6, 5]); bowtie(g, x, y, vert, null, C.amber); g.setLineDash([]);
            return;
          }
          const shut = closed === i && phase !== "hold" && phase !== "done";
          const wrong = shut && spring > 0;
          bowtie(g, x, y, vert, shut ? (wrong ? C.danger : "#c9d3df") : "#0b111b", wrong ? C.danger : "#c9d3df");
          if (phase === "isolate" && kf && fi === i) { g.setLineDash([6, 5]); D.ring(g, x, y, 26, C.amber, 3); g.setLineDash([]); }
          if (phase === "open" && closed === i) {
            D.disc(g, x, y, 30, "rgba(79,195,247,0.18)"); D.ring(g, x, y, 34 + 3 * Math.sin(t * 6), C.accent, 3);
          }
        });
        // The leak: a faint mist once the sniffer is within reach; marked and spraying once found, until it is cut off.
        if (!partStep) {
          const d = Math.hypot(sn.x - leak.x, sn.y - leak.y);
          const fed = phase === "find" || phase === "isolate" || (phase === "drain" && drain < 1);
          const mist = phase === "find" ? Math.min(0.7, Math.max(0, 1 - d / SNIFF.mist) * 1.1) : fed ? 1 - (phase === "drain" ? drain : 0) : 0;
          if (mist > 0) for (let k = 0; k < 7; k++) {
            const a = t * 1.3 + k * 0.9, rr = 10 + ((t * 30 + k * 13) % 40);
            D.disc(g, leak.x + Math.cos(a) * rr * 0.8, leak.y + Math.sin(a) * rr * 0.6 - rr * 0.4, 7 + rr * 0.25, `rgba(255,232,170,${0.6 * mist * (1 - rr / 50)})`);
          }
          if (phase !== "find" && !patch.set) {
            g.setLineDash([7, 6]); D.ring(g, leak.x, leak.y, 20, C.danger, 3); g.setLineDash([]);
            crack(g, leak.x, leak.y, leak.vert);
          }
          if (phase === "patch") D.ring(g, leak.x, leak.y, 30 + 3 * Math.sin(t * 6), C.accent, 3);
        }
        for (const q of puffs) for (let k = 0; k < 6; k++) {
          const a = k * 1.05, rr = 8 + q.t * 50;
          D.disc(g, q.x + Math.cos(a) * rr, q.y + Math.sin(a) * rr, 6 + q.t * 10, `rgba(255,228,160,${0.5 * (1 - q.t / 1.4)})`);
        }
        if (phase === "find") drawSniffer(g, t);
        drawPanel(g, t);
        if (partStep) { if (!patch.set) drawValvePart(g, patch.x, patch.y); }
        else drawPatch(g, patch.x, patch.y, patch.set ? leak.vert : false);
      },
    };

    /** A fuel pipe: a steel wall round fuel that flows outward (moving dashes), or hollow when `empty` reaches 1. */
    function pipe(g, pts, empty, t) {
      const path = () => { g.beginPath(); pts.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y))); };
      g.lineJoin = "round"; g.lineCap = "round";
      path(); g.strokeStyle = PIPE; g.lineWidth = 13; g.stroke();
      path(); g.strokeStyle = EMPTY; g.lineWidth = 7; g.stroke();
      if (empty < 1) {
        g.globalAlpha = 1 - empty;
        path(); g.strokeStyle = FUEL; g.lineWidth = 7; g.stroke();
        g.setLineDash([5, 15]); g.lineDashOffset = -t * 36;
        path(); g.strokeStyle = FUEL_LIT; g.lineWidth = 3; g.stroke();
        g.setLineDash([]); g.lineDashOffset = 0; g.globalAlpha = 1;
      }
      g.lineCap = "butt";
    }
    function bowtie(g, x, y, vert, fill, stroke) {
      g.beginPath();
      if (vert) { g.moveTo(x - 12, y - 14); g.lineTo(x + 12, y - 14); g.lineTo(x - 12, y + 14); g.lineTo(x + 12, y + 14); }
      else { g.moveTo(x - 14, y - 12); g.lineTo(x - 14, y + 12); g.lineTo(x + 14, y - 12); g.lineTo(x + 14, y + 12); }
      g.closePath();
      g.fillStyle = fill || "#0b111b"; g.fill();
      g.strokeStyle = stroke; g.lineWidth = 3; g.lineJoin = "round"; g.stroke();
    }
    /** The split in the pipe wall at the leak. */
    function crack(g, x, y, vert) {
      g.save(); g.translate(x, y); if (vert) g.rotate(Math.PI / 2);
      g.beginPath(); g.moveTo(-9, -6); g.lineTo(-3, 2); g.lineTo(2, -3); g.lineTo(9, 6);
      g.strokeStyle = C.danger; g.lineWidth = 3; g.lineJoin = "round"; g.stroke();
      g.restore();
    }
    /** The sniffer's colour: cold blue far off, yellow nearer, red on the leak. */
    function heat(w) {
      const c = w < 0.5 ? mix([79, 195, 247], [255, 197, 66], w * 2) : mix([255, 197, 66], [255, 71, 87], (w - 0.5) * 2);
      return (a) => `rgba(${c[0]},${c[1]},${c[2]},${a})`;
    }
    /** The leak sniffer: a probe head on the line, its ring pulsing faster, bigger and warmer near the leak. */
    function drawSniffer(g, t) {
      const w = warmth(), col = heat(w), hz = 0.6 + 5.4 * w * w;
      // The pulse: rings run out from the head; nearer the leak they come faster, reach further and burn brighter.
      for (let k = 0; k < 2; k++) {
        const f = (t * hz + k / 2) % 1, r = 18 + f * (26 + 44 * w);
        D.ring(g, sn.x, sn.y, r, col((0.35 + 0.65 * w) * (1 - f)), 3 + 3 * w);
      }
      // The wand down to the hand, then the head.
      g.strokeStyle = "#9aa6b6"; g.lineWidth = 6; g.lineCap = "round";
      g.beginPath(); g.moveTo(sn.x + 10, sn.y + 10); g.lineTo(sn.x + 34, sn.y + 34); g.stroke(); g.lineCap = "butt";
      D.disc(g, sn.x, sn.y, 15, "#1b2433"); D.ring(g, sn.x, sn.y, 15, "#c9d3df", 3);
      D.disc(g, sn.x, sn.y, 7, col(0.5 + 0.5 * w));
      if (lock > 0) {
        g.beginPath(); g.arc(sn.x, sn.y, 22, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * lock / SNIFF.lockS);
        g.strokeStyle = C.danger; g.lineWidth = 5; g.stroke();
      }
      if (kf) { g.setLineDash([6, 5]); D.ring(g, sn.x, sn.y, 28, C.amber, 2); g.setLineDash([]); }
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
    /** The test panel: the gauge, the sniffer's signal, the pump, the patch kit. */
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
      // The sniffer's signal: five bars, more and warmer the nearer the leak (readable under a finger on the probe).
      const live = phase === "find", w = live ? warmth() : 0, col = heat(w), n = live ? Math.ceil(w * 5) : 0;
      D.panel(g, SIG.x - 106, SIG.y - 30, 212, 60, 12, live ? "#111826" : "#0b111b", live && n ? col(0.8) : C.line);
      D.disc(g, SIG.x - 76, SIG.y, 12, "#1b2433"); D.ring(g, SIG.x - 76, SIG.y, 12, live ? "#c9d3df" : "#3a4558", 3);
      D.disc(g, SIG.x - 76, SIG.y, 5, live ? col(0.9) : "#2a3446");
      for (let k = 0; k < 5; k++) {
        const bh = 10 + k * 8, bx = SIG.x - 46 + k * 30;
        g.fillStyle = k < n ? col(1) : "#222b3a"; g.fillRect(bx, SIG.y + 22 - bh, 20, bh);
      }
      // The pump: the round's one action once the valve is open again.
      const on = phase === "hold";
      D.disc(g, PUMP.x, PUMP.y, PUMP.r, on ? "#262e42" : "#121822");
      D.ring(g, PUMP.x, PUMP.y, PUMP.r, on && flow > 0.18 ? C.amber : on ? C.steel : C.line, 4);
      D.text(g, "PUMP", PUMP.x, PUMP.y, 24, on ? C.fg : "#3a4558", "center", 700);
      D.panel(g, KITBOX.x - 80, KITBOX.y - 36, 160, 72, 12, "#141b27", phase === "patch" || phase === "part" ? C.accent : C.line);
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
