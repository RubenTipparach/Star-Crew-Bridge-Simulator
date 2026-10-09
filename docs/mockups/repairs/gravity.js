/*
 * repairs/gravity.js: the gravity generator's repair, "field alignment" (openspec/changes/repair-minigames,
 * design 2 and 6d).
 *
 * It owns the generator's picture and its round. The field over the deck is a 3D wave surface, a 24 x 24 wireframe
 * in perspective seen from a fixed three-quarter view above, its height the field's ripple. The reference shape (the
 * field the generator should make) is a faint dashed ghost in the same space. Each live line is coloured by how far it
 * lies from the ghost there: green on it, amber off it, red and thick far off it, so the picture says where the field
 * is wrong. Three controls set the live wave: Turn (a geared dial, the crests' direction), Shift (a horizontal slider,
 * the phase) and Stretch (a vertical slider, the wavelength, locked in round 1). Dragging on the surface turns it
 * (sideways) and shifts it (up and down). Keys and pad: the stick turns (x) and shifts (y), Q and E stretch, Tab or the
 * action button picks the ripple.
 *
 * One number drives the round: the RMS difference between the live surface and the ghost over the grid, over the
 * ghost's own RMS. Under HOLD_ERR the field holds and the hold ring on the deck round the graph fills; the round is
 * played when it fills. Above SURGE_ERR the field surges: the fumble, a jolt, loose bolts float, and the drift is
 * kicked to a new heading. All three settings drift (seeded, smoothed noise), faster each round and in combat, so a
 * held field has to be nursed. Round 1 is turn and shift; round 2 adds stretch; round 3 and on add a second, smaller
 * ripple to both the ghost and the live field, and two tabs pick which ripple the controls hold. The hold to fill is
 * sized from the step's share at the repairer's rate, so play and the job's bar end together (design 1). A disabled
 * generator's first step fits the new field coil: carry it from the crate into the hub socket (drag it, or the stick
 * and the action button).
 *
 * The kit features it uses are only those in kit.js (register, api, KIT.draw, KIT.RATES); nothing here is shared.
 */
RepairKit.register({
  id: "gravity",
  title: "Gravity generator",
  place: "Engineering, deck C",
  group: "Engineering",
  hazard: "Field surge: everyone near floats for 2 s",
  // The how-to card (repair-minigames 6g), drawn by the kit: pictures and a few words, on demand.
  guide: {
    steps: [
      { icon: "match", text: "Lay the field on the ghost" },
      { icon: "drag", text: "Drag the field to turn it" },
      { icon: "slider", text: "Shift and Stretch slide it" },
      { icon: "hold", text: "Hold it green; the ring fills" },
    ],
    mistake: "Far off the ghost: a field surge, everyone floats 2 s",
    now: (q) => (q.part ? -1 : q.hold > 0 ? 3 : 0),
  },
  down: "Everyone aboard floats, slowly (design 4)",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const SURGE_TEXT = "Field surge: everyone near floats";

    // ---------------------------------------------------------------- the match
    const N = 24;                          // grid cells a side (N + 1 vertices)
    const HOLD_ERR = 0.15;                 // design 6d: under this the field holds
    const SURGE_ERR = 0.85;                // design 6d: above this the field surges
    const ENV = 1.4;                       // the field's envelope over the deck, exp(-r^2 / ENV): a ripple round the hub
    const PH = 1.1;                        // rad of phase at the Shift slider's end stop
    const OCT = 0.6;                       // octaves of wavelength at the Stretch slider's end stop
    const GEAR = 4;                        // the Turn dial turns 4 times for the crests' once: a fine dial
    const COOL_S = 1.5;                    // s after a surge before another can happen
    const SURGE_HOLD_S = 1.0;              // s of hold a surge knocks back
    const HOLD_SHARE = 0.62;               // the hold to fill, as a share of the step's time at the rate (one ripple)
    const HOLD_SHARE_TWO = 0.5;            // with two ripples: catching both takes about twice as long, so less to hold
    const HOLD_MIN_S = 5, HOLD_MAX_S = 9.5, HOLD_PER_ROUND_S = 0.4;
    // The ripples: [big, small]. amp is relative height; lam0 the wavelength at the Stretch slider's centre, deck units.
    const RIPPLES = [{ amp: 1, lam0: 1.15 }, { amp: 0.45, lam0: 0.56 }];
    // Drift, in control units a second (turn rad/s, shift and stretch in slider units/s), and its bounds.
    const DRIFT = { t: { speed: 0.045, bound: 0.45 }, s: { speed: 0.09, bound: 0.45 }, l: { speed: 0.07, bound: 0.4 } };
    const ROUND_SPEEDUP = 0.15, COMBAT_SPEEDUP = 1.5;
    const SMALL_DRIFT = 0.35;              // the small ripple drifts slower, and not in stretch: two to nurse, not two to chase
    // Input rates.
    const KEY_TURN = 0.35, KEY_SHIFT = 0.7, KEY_STRETCH = 0.6;   // per second at a full stick
    const DRAG_TURN = 0.0025, DRAG_SHIFT = 0.006 / PH;            // per canvas px dragged on the surface

    // ---------------------------------------------------------------- the screen (canvas px)
    const GRAPH = { x: 16, y: 84, w: 880, h: 548 };
    const DIAL = { x: 1036, y: 252, r: 104 };
    const SHIFT = { x: 120, y: 650, w: 700, h: 56 };
    const STRETCH = { x: 1188, y: 112, w: 60, h: 470 };
    const TABS = [{ x: 924, y: 452, w: 108, h: 84 }, { x: 1044, y: 452, w: 108, h: 84 }];
    const CRATE = { x: 940, y: 548, w: 200, h: 150 };
    const HIT = 34;                        // extra px round every control, so a finger finds it

    // ---------------------------------------------------------------- the view: a fixed three-quarter view
    const YAW = -0.66, ELEV = 0.7, CAM = 4.3, FOCAL = 840, VX = 456, VY = 292;
    const ZS = 0.32;                       // deck units of height per unit of ripple
    const DECK_Z = -0.4, RING_R = 1.44, PLATE_R = 1.56;
    const cy = Math.cos(YAW), sy = Math.sin(YAW), ce = Math.cos(ELEV), se = Math.sin(ELEV);
    /** Deck coordinates (u right, v away, z up) to canvas px. */
    const proj = (u, v, z) => {
      const x1 = u * cy - v * sy, y1 = u * sy + v * cy;
      const depth = CAM + y1 * ce - z * se, up = z * ce + y1 * se;
      return [VX + (FOCAL * x1) / depth, VY - (FOCAL * up) / depth, depth];
    };
    const GU = [], ENVW = [];
    for (let j = 0; j <= N; j++) for (let i = 0; i <= N; i++) {
      const u = -1 + (2 * i) / N, v = -1 + (2 * j) / N;
      GU.push([u, v]); ENVW.push(Math.exp(-(u * u + v * v) / ENV));
    }
    // The quads far to near, sorted once (heights are small beside the depth).
    const QUADS = [];
    for (let j = 0; j < N; j++) for (let i = 0; i < N; i++) {
      const a = j * (N + 1) + i;
      QUADS.push({ v: [a, a + 1, a + N + 2, a + N + 1], d: proj(-1 + (2 * i + 1) / N, -1 + (2 * j + 1) / N, 0)[2] });
    }
    QUADS.sort((p, q) => q.d - p.d);
    // Loose bolts on the deck plate: they float in a surge.
    const BOLTS = [[1.22, -0.35], [1.2, 0.45], [-0.5, -1.22], [0.35, -1.22], [-1.22, 0.1], [0.2, 1.24]];

    // ---------------------------------------------------------------- state
    let s = null, hadPart = false, clock = 0;
    const shares = {};                     // the step's share in seconds at the rate, by step index (kept over restarts)
    const log = [];                        // for tools: when each step started and its round was played (clock, s)
    const wrap = (a) => Math.atan2(Math.sin(a), Math.cos(a));
    const clamp = (x, a, b) => Math.max(a, Math.min(b, x));

    /** One drifting setting: a smoothed heading that changes now and then and turns back at its bound. */
    function drifter(kind, mult, r) {
      return { kind, x: 0, v: 0, h: 0, timer: 0, speed: DRIFT[kind].speed * mult, bound: DRIFT[kind].bound, r };
    }
    function driftTick(d, dt) {
      d.timer -= dt;
      if (d.timer <= 0) {
        const sign = Math.abs(d.x) > 0.5 * d.bound ? -Math.sign(d.x) : d.r() < 0.5 ? -1 : 1;
        d.h = sign * d.speed * (0.55 + 0.45 * d.r());
        d.timer = 1.4 + 2.2 * d.r();
      }
      if (Math.abs(d.x) > d.bound && Math.sign(d.h) === Math.sign(d.x)) d.h = -d.h;
      d.v += (d.h - d.v) * Math.min(1, dt / 0.7);
      d.x += d.v * dt;
    }
    /** A surge kicks the drift to a new heading, back the way it came and harder. */
    function driftKick(d) { d.h = -(Math.sign(d.v) || 1) * d.speed * 1.4; d.v = d.h * 0.5; d.timer = 1.6 + d.r(); }

    /** The live settings of ripple k: the hand's control plus the drift. */
    const live = (rp) => ({ t: rp.ct + rp.dt.x, s: rp.cs + rp.ds.x, l: rp.cl + rp.dl.x });
    /** The control values that would lay ripple k exactly on the ghost now. */
    const need = (rp) => ({ t: rp.ct + wrap(rp.gt - (rp.ct + rp.dt.x)), s: rp.gs - rp.ds.x, l: rp.gl - rp.dl.x });
    function heights(out, which) {
      out.fill(0);
      s.rip.forEach((rp, k) => {
        const p = which === "ghost" ? { t: rp.gt, s: rp.gs, l: rp.gl } : live(rp);
        const lam = RIPPLES[k].lam0 * Math.pow(2, OCT * p.l), kk = (2 * Math.PI) / lam;
        const ct = Math.cos(p.t), st = Math.sin(p.t), ph = PH * p.s, a = RIPPLES[k].amp;
        for (let i = 0; i < out.length; i++) out[i] += a * Math.cos(kk * (GU[i][0] * ct + GU[i][1] * st) - ph);
      });
      for (let i = 0; i < out.length; i++) out[i] *= ENVW[i];
    }
    const HL = new Float64Array((N + 1) * (N + 1)), HG = new Float64Array((N + 1) * (N + 1)), DV = new Float64Array((N + 1) * (N + 1));
    /** The match: RMS(live - ghost) / RMS(ghost) over the grid; also each vertex's local miss, over the local scale. */
    function measure() {
      heights(HL, "live"); heights(HG, "ghost");
      let num = 0, den = 0;
      const scale = s.rip.reduce((a, _, k) => a + RIPPLES[k].amp, 0);
      for (let i = 0; i < HL.length; i++) {
        const d = HL[i] - HG[i];
        num += d * d; den += HG[i] * HG[i];
        DV[i] = Math.abs(d) / (ENVW[i] * scale);
      }
      return Math.sqrt(num / Math.max(1e-9, den));
    }

    function newRipple(k, round, mult, r, gt) {
      const rp = {
        gt, gs: (r() * 2 - 1) * 0.35, gl: round >= 2 ? (r() * 2 - 1) * 0.3 : 0,
        dt: drifter("t", mult * (k ? SMALL_DRIFT : 1), r), ds: drifter("s", mult * (k ? SMALL_DRIFT : 1), r),
        dl: drifter("l", round >= 2 && !k ? mult : 0, r),
        ct: 0, cs: 0, cl: 0,
      };
      // The hand starts off the mark, by enough to see, never into a surge.
      const sg = () => (r() < 0.5 ? -1 : 1);
      rp.ct = rp.gt + sg() * (k ? 0.16 : 0.12) * (0.8 + 0.4 * r());
      rp.cs = clamp(rp.gs + sg() * (k ? 0.45 : 0.36) * (0.85 + 0.3 * r()), -1, 1);
      rp.cl = round >= 2 ? clamp(rp.gl + sg() * (k ? 0.3 : 0.24) * (0.85 + 0.3 * r()), -1, 1) : 0;
      return rp;
    }

    function start(index, isPart) {
      const r = api.rand();
      if (isPart) hadPart = true;
      if (shares[index] === undefined && !isPart) {
        // The step's share of the job in seconds at this repairer's rate (design 1): (target - now) / steps left / rate.
        shares[index] = (100 - api.value) / Math.max(1, api.steps - index) / (KIT.RATES[api.who] || KIT.RATES.officer);
      }
      const round = index + 1 - (hadPart ? 1 : 0);
      const mult = (1 + ROUND_SPEEDUP * (round - 1)) * (api.combat ? COMBAT_SPEEDUP : 1);
      s = {
        index, round, part: isPart, phase: isPart ? "part" : "play", t: 0,
        rip: [], sel: 0, err: 1, hold: 0, holdNeed: 0, cool: 0, surge: 0, surges: 0,
        grab: null, last: null, acc: 0, r,
        coil: { x: CRATE.x + CRATE.w / 2, y: CRATE.y + CRATE.h / 2 + 8, held: false, pad: false, set: !isPart },
      };
      if (!isPart) {
        const gt = r() * Math.PI * 2;
        s.rip.push(newRipple(0, round, mult, r, gt));
        // The small ripple runs across the big one.
        if (round >= 3) s.rip.push(newRipple(1, round, mult, r, gt + Math.PI / 2 + (r() * 2 - 1) * 0.5));
        s.holdNeed = clamp((round >= 3 ? HOLD_SHARE_TWO : HOLD_SHARE) * shares[index], HOLD_MIN_S, HOLD_MAX_S) + HOLD_PER_ROUND_S * (round - 1);
        s.err = measure();
      }
      log.push({ index, round, part: isPart, start: clock, aligned: null, played: null, holdNeed: s.holdNeed, share: shares[index] ?? null });
    }

    // ---------------------------------------------------------------- input
    const inRect = (R, x, y, pad = 0) => x >= R.x - pad && x <= R.x + R.w + pad && y >= R.y - pad && y <= R.y + R.h + pad;
    function hitTest(x, y) {
      if (s.rip.length > 1) { const k = TABS.findIndex((T) => inRect(T, x, y, 10)); if (k >= 0) return "tab" + k; }
      if (s.round >= 2 && x >= STRETCH.x - HIT && inRect(STRETCH, x, y, HIT)) return "stretch";
      if (inRect(SHIFT, x, y, HIT)) return "shift";
      if (Math.hypot(x - DIAL.x, y - DIAL.y) < DIAL.r + HIT + 6) return "dial";
      if (inRect(GRAPH, x, y)) return "surface";
      return null;
    }
    const shiftAt = (x) => clamp((x - (SHIFT.x + SHIFT.w / 2)) / (SHIFT.w / 2 - 24), -1, 1);
    const stretchAt = (y) => clamp(-(y - (STRETCH.y + STRETCH.h / 2)) / (STRETCH.h / 2 - 24), -1, 1);

    function partStep(dt, input) {
      const p = s.coil, hub = proj(0, 0, DECK_Z);
      if (input.stick.x || input.stick.y) { p.x += input.stick.x * 480 * dt; p.y += input.stick.y * 480 * dt; p.pad = true; }
      if (input.pressed && Math.hypot(input.x - p.x, input.y - p.y) < 64) { p.held = true; p.dx = p.x - input.x; p.dy = p.y - input.y; }
      if (p.held && input.down) { p.x = input.x + p.dx; p.y = input.y + p.dy; }
      if ((p.held && input.released) || (p.pad && input.actionPressed)) {
        p.held = false; p.pad = false;
        if (Math.hypot(p.x - hub[0], p.y - hub[1]) < 56) {
          p.set = true; p.x = hub[0]; p.y = hub[1]; s.phase = "fitted";
          log[log.length - 1].played = clock;
          api.stepDone();
        }
      }
      p.x = clamp(p.x, 30, api.W - 30); p.y = clamp(p.y, api.BAR_H + 30, api.H - 30);
    }

    function play(dt, input) {
      // Picking the ripple: a tab, Tab, or the action button (round 3 and on).
      if (s.rip.length > 1 && (input.hit.has("Tab") || input.actionPressed)) { s.sel = 1 - s.sel; }
      if (input.pressed) {
        s.grab = hitTest(input.x, input.y); s.last = [input.x, input.y];
        if (s.grab === "dial") s.lastA = Math.atan2(input.y - DIAL.y, input.x - DIAL.x);
        if (s.grab && s.grab.startsWith("tab")) { s.sel = +s.grab.slice(3); s.grab = null; }
      }
      if (!input.down) s.grab = null;
      const R = s.rip[s.sel];
      if (s.grab === "shift") R.cs = shiftAt(input.x);
      if (s.grab === "stretch") R.cl = stretchAt(input.y);
      if (s.grab === "dial") {
        if (Math.hypot(input.x - DIAL.x, input.y - DIAL.y) > 14) {
          const a = Math.atan2(input.y - DIAL.y, input.x - DIAL.x);
          R.ct += wrap(a - s.lastA) / GEAR; s.lastA = a;
        }
      }
      if (s.grab === "surface") {
        const dx = input.x - s.last[0], dy = input.y - s.last[1];
        R.ct += dx * DRAG_TURN; R.cs = clamp(R.cs - dy * DRAG_SHIFT, -1, 1);
      }
      s.last = [input.x, input.y];
      // Keys and pad.
      if (input.stick.x) { R.ct += input.stick.x * KEY_TURN * dt; }
      if (input.stick.y) { R.cs = clamp(R.cs - input.stick.y * KEY_SHIFT * dt, -1, 1); }
      const q = (input.keys.has("KeyE") ? 1 : 0) - (input.keys.has("KeyQ") ? 1 : 0);
      if (q && s.round >= 2) { R.cl = clamp(R.cl + q * KEY_STRETCH * dt, -1, 1); }

      // The drift, on a fixed tick so a run is the same whatever the frame rate.
      s.acc += dt;
      while (s.acc >= 1 / 60) { s.acc -= 1 / 60; for (const p of s.rip) { driftTick(p.dt, 1 / 60); driftTick(p.ds, 1 / 60); if (s.round >= 2) driftTick(p.dl, 1 / 60); } }
      s.err = measure();
      if (s.err > SURGE_ERR && s.cool <= 0) {
        // The surge: the fumble, the field dumps half its error, the drift turns.
        s.cool = COOL_S; s.surge = 2; s.surges++; s.hold = Math.max(0, s.hold - SURGE_HOLD_S); s.grab = null;
        for (const p of s.rip) {
          const L = live(p);
          p.dt.x -= 0.5 * wrap(L.t - p.gt); p.ds.x -= 0.5 * (L.s - p.gs); p.dl.x -= 0.5 * (L.l - p.gl);
          driftKick(p.dt); driftKick(p.ds); if (s.round >= 2) driftKick(p.dl);
        }
        s.err = measure();
        api.fumble(SURGE_TEXT);
        return;
      }
      if (s.err < HOLD_ERR) {
        if (log[log.length - 1].aligned === null) log[log.length - 1].aligned = clock;
        s.hold += dt;
        if (s.hold >= s.holdNeed) {
          s.hold = s.holdNeed; s.phase = "held"; s.grab = null;
          log[log.length - 1].played = clock;
          api.stepDone();
        }
      }
    }

    // ---------------------------------------------------------------- drawing
    const mix = (a, b, k) => a.map((x, i) => x + (b[i] - x) * k);
    const RGB = { ok: [61, 220, 132], warn: [255, 197, 66], bad: [255, 71, 87], off: [96, 112, 132] };
    /** A local miss to a line colour and width: green on, amber off, red and thick far off. */
    function missStyle(d) {
      if (d < 0.18) return [RGB.ok, 1.6];
      if (d < 0.5) return [mix(RGB.ok, RGB.warn, (d - 0.18) / 0.32), 2.2];
      if (d < 0.8) return [mix(RGB.warn, RGB.bad, (d - 0.5) / 0.3), 3];
      return [RGB.bad, 4.2];
    }
    const css = (c, a = 1) => `rgba(${c[0] | 0},${c[1] | 0},${c[2] | 0},${a})`;

    function drawDeck(g, t) {
      // The deck plate under the field: a round plate, its near rim's thickness, its own faint grid.
      const n = 72, top = [], low = [];
      for (let k = 0; k < n; k++) { const a = (k / n) * Math.PI * 2; top.push(proj(PLATE_R * Math.cos(a), PLATE_R * Math.sin(a), DECK_Z)); low.push(proj(PLATE_R * Math.cos(a), PLATE_R * Math.sin(a), DECK_Z - 0.1)); }
      for (let k = 0; k < n; k++) {
        const k2 = (k + 1) % n;
        if (top[k][2] > CAM && top[k2][2] > CAM) continue;
        g.beginPath(); g.moveTo(top[k][0], top[k][1]); g.lineTo(top[k2][0], top[k2][1]); g.lineTo(low[k2][0], low[k2][1]); g.lineTo(low[k][0], low[k][1]); g.closePath();
        g.fillStyle = "#121a26"; g.fill(); g.strokeStyle = "#121a26"; g.lineWidth = 1; g.stroke();
      }
      polyline(g, top); g.closePath();
      g.fillStyle = "#0b1119"; g.fill(); g.strokeStyle = "#2a3648"; g.lineWidth = 2; g.stroke();
      g.save(); polyline(g, top); g.closePath(); g.clip();
      g.strokeStyle = "rgba(60,78,100,0.35)"; g.lineWidth = 1;
      for (let k = -6; k <= 6; k++) {
        const w = (k / 6) * PLATE_R;
        const a = proj(w, -PLATE_R, DECK_Z), b = proj(w, PLATE_R, DECK_Z), c = proj(-PLATE_R, w, DECK_Z), d = proj(PLATE_R, w, DECK_Z);
        g.beginPath(); g.moveTo(a[0], a[1]); g.lineTo(b[0], b[1]); g.moveTo(c[0], c[1]); g.lineTo(d[0], d[1]); g.stroke();
      }
      g.restore();
    }
    /** The generator's hub on the deck: on the part step, its open socket. */
    function drawHub(g, t) {
      const hub = proj(0, 0, DECK_Z);
      g.beginPath(); g.ellipse(hub[0], hub[1], 64, 64 * 0.55, 0, 0, Math.PI * 2); g.fillStyle = "#121a26"; g.fill();
      g.strokeStyle = s.part && !s.coil.set ? C.amber : "#3a4a60"; g.lineWidth = 3; g.stroke();
      if (s.part && !s.coil.set) {
        g.beginPath(); g.ellipse(hub[0], hub[1], 40, 22, 0, 0, Math.PI * 2); g.fillStyle = "#120d08"; g.fill();
        g.setLineDash([8, 6]); g.strokeStyle = C.amber; g.lineWidth = 3; g.stroke(); g.setLineDash([]);
        D.ring(g, hub[0], hub[1] - 2, 54 + 3 * Math.sin(t * 5), "rgba(242,160,70,0.35)", 2);
      }
    }

    /** The hold ring on the deck round the graph: the track, the hold filled in green, the surge zone hatched. */
    function ringPts(a0, a1, n) {
      const out = [];
      for (let i = 0; i <= n; i++) { const a = a0 + ((a1 - a0) * i) / n; out.push(proj(RING_R * Math.cos(a), RING_R * Math.sin(a), DECK_Z)); }
      return out;
    }
    const RING_A0 = Math.atan2(cy, sy);              // the fill starts at the far side, the top of the screen
    function polyline(g, pts) { g.beginPath(); pts.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y))); }
    function drawRing(g, t, half) {
      // half: 0 the far half (under the surface), 1 the near half (over it).
      const segs = 96, all = ringPts(RING_A0, RING_A0 + Math.PI * 2, segs);
      const fill = s.phase === "held" || s.phase === "fitted" ? 1 : s.holdNeed ? s.hold / s.holdNeed : 0;
      const holding = s.phase === "play" && s.err < HOLD_ERR;
      const danger = s.phase === "play" && s.err > 0.6;
      for (let i = 0; i < segs; i++) {
        const a = all[i], b = all[i + 1], far = (a[2] + b[2]) / 2 > CAM;
        if ((half === 0) !== far) continue;
        const f = (i + 0.5) / segs;
        g.beginPath(); g.moveTo(a[0], a[1]); g.lineTo(b[0], b[1]);
        g.lineCap = "round";
        if (f <= fill) { g.strokeStyle = holding || fill >= 1 ? C.ok : "#2f9e64"; g.lineWidth = 13; }
        else { g.strokeStyle = danger ? `rgba(255,71,87,${0.35 + 0.3 * Math.sin(t * 9)})` : "#1d2738"; g.lineWidth = 11; }
        g.stroke();
        // The surge zone has a shape as well as a colour: cross ticks on the track.
        if (f > fill && danger && i % 3 === 0) {
          const dx = b[0] - a[0], dy = b[1] - a[1], l = Math.hypot(dx, dy) || 1;
          g.beginPath(); g.moveTo(a[0] - (dy / l) * 10, a[1] + (dx / l) * 10); g.lineTo(a[0] + (dy / l) * 10, a[1] - (dx / l) * 10);
          g.strokeStyle = C.danger; g.lineWidth = 3; g.stroke();
        }
        g.lineCap = "butt";
      }
      if (half === 1 && fill > 0 && fill < 1) {
        // The head of the hold, a bright bead.
        const k = Math.floor(fill * segs), p = all[Math.min(segs, k)];
        D.disc(g, p[0], p[1], 9, holding ? "#c9ffe0" : "#2f9e64");
      }
    }

    function drawSurface(g, t) {
      const jolt = s.surge > 1 ? (s.surge - 1) : 0;
      const zk = 1 + 1.4 * jolt * Math.sin(t * 38);
      const pts = Array.from(HL, (h, i) => proj(GU[i][0], GU[i][1], h * ZS * zk + (jolt ? 0.04 * Math.sin(t * 51 + i) * jolt : 0)));
      const gpts = Array.from(HG, (h, i) => proj(GU[i][0], GU[i][1], h * ZS));
      const held = s.phase === "held";
      // Quads far to near: a shaded fill tinted by the miss, then its edges coloured by the miss.
      for (const q of QUADS) {
        const [a, b, c, d] = q.v;
        const dzu = (HL[b] - HL[a] + HL[c] - HL[d]) / 2, dzv = (HL[d] - HL[a] + HL[c] - HL[b]) / 2;
        const shade = clamp(0.55 + 2.2 * (-dzu * 0.7 + dzv * 0.5) * ZS, 0.2, 1);
        const miss = held ? 0 : (DV[a] + DV[b] + DV[c] + DV[d]) / 4;
        const [col] = missStyle(miss);
        const base = mix([16, 30, 48], col, 0.16);
        g.beginPath(); g.moveTo(pts[a][0], pts[a][1]); g.lineTo(pts[b][0], pts[b][1]); g.lineTo(pts[c][0], pts[c][1]); g.lineTo(pts[d][0], pts[d][1]); g.closePath();
        g.fillStyle = css(base.map((x) => x * (0.6 + 0.9 * shade)), 0.9); g.fill();
        for (const [p, r] of [[a, b], [b, c], [c, d], [d, a]]) {
          const [ec, w] = missStyle(held ? 0 : (DV[p] + DV[r]) / 2);
          g.beginPath(); g.moveTo(pts[p][0], pts[p][1]); g.lineTo(pts[r][0], pts[r][1]);
          g.strokeStyle = jolt > 0.2 ? css(mix(RGB.bad, [255, 214, 220], 0.5 + 0.5 * Math.sin(t * 30)), 1) : css(ec, 0.95); g.lineWidth = jolt > 0.2 ? Math.max(w, 2.6) : w; g.stroke();
        }
      }
      // The ghost: the shape the field should hold, a faint dashed wire over the same space.
      if (!held) {
        g.save(); g.setLineDash([5, 5]); g.strokeStyle = "rgba(210,228,255,0.5)"; g.lineWidth = 1.3;
        for (let j = 0; j <= N; j += 2) { g.beginPath(); for (let i = 0; i <= N; i++) { const p = gpts[j * (N + 1) + i]; i ? g.lineTo(p[0], p[1]) : g.moveTo(p[0], p[1]); } g.stroke(); }
        for (let i = 0; i <= N; i += 2) { g.beginPath(); for (let j = 0; j <= N; j++) { const p = gpts[j * (N + 1) + i]; j ? g.lineTo(p[0], p[1]) : g.moveTo(p[0], p[1]); } g.stroke(); }
        g.restore();
      }
    }

    function drawBolts(g, t, half) {
      const k = s.surge > 0 ? Math.sin(Math.PI * Math.min(1, (2 - s.surge) / 2)) : 0;
      BOLTS.forEach(([u, v], i) => {
        const z = DECK_Z + 0.03 + 0.55 * k * (0.7 + 0.3 * Math.sin(i * 2.1));
        const [x, y, dep] = proj(u, v, z);
        if ((half === 0) !== (dep > CAM)) return;
        if (k > 0) { const [sx, sy2] = proj(u, v, DECK_Z); g.beginPath(); g.ellipse(sx, sy2, 10, 5, 0, 0, Math.PI * 2); g.fillStyle = "rgba(0,0,0,0.5)"; g.fill(); }
        g.save(); g.translate(x, y); g.rotate(k * (t * 3 + i));
        g.fillStyle = i % 2 ? C.steel : "#a0aab8"; g.fillRect(-9, -4, 18, 8);
        g.fillStyle = "#4a5566"; g.fillRect(-4, -4, 3, 8);
        g.restore();
      });
    }

    /** A short curved arrow along a rim from a0 to a1, either way round (the kit's turnArrow goes clockwise only). */
    function arcArrow(g, x, y, r, a0, a1, color) {
      g.beginPath(); g.arc(x, y, r, a0, a1, a1 < a0); g.strokeStyle = color; g.lineWidth = 4; g.stroke();
      const hx = x + Math.cos(a1) * r, hy = y + Math.sin(a1) * r, d = a1 + (a1 < a0 ? -1 : 1) * Math.PI / 2;
      g.beginPath(); g.moveTo(hx + Math.cos(d) * 12, hy + Math.sin(d) * 12);
      g.lineTo(hx + Math.cos(a1) * 9, hy + Math.sin(a1) * 9); g.lineTo(hx - Math.cos(a1) * 9, hy - Math.sin(a1) * 9);
      g.closePath(); g.fillStyle = color; g.fill();
    }
    function drawDial(g, t) {
      const on = s.phase === "play", R = s.rip[s.sel] || { ct: 0 };
      const grab = s.grab === "dial";
      D.disc(g, DIAL.x, DIAL.y, DIAL.r + 22, "#0c121a");
      D.ring(g, DIAL.x, DIAL.y, DIAL.r + 22, grab ? C.amber : C.line, 2);
      // The knob's grip turns with the hand (geared).
      D.disc(g, DIAL.x, DIAL.y, DIAL.r, "#1d2738");
      D.ring(g, DIAL.x, DIAL.y, DIAL.r, on ? "#9aa6b6" : "#4a5566", 4);
      for (let j = 0; j < 24; j++) {
        const a = (j / 24) * Math.PI * 2 + R.ct * GEAR;
        D.disc(g, DIAL.x + Math.cos(a) * (DIAL.r - 12), DIAL.y + Math.sin(a) * (DIAL.r - 12), 5, j === 0 ? (on ? C.fg : "#6f7f94") : "#2a3446");
      }
      // The face: the crests' direction, three lines across.
      D.disc(g, DIAL.x, DIAL.y, DIAL.r - 30, "#0a1018");
      g.save(); g.beginPath(); g.arc(DIAL.x, DIAL.y, DIAL.r - 32, 0, Math.PI * 2); g.clip();
      g.translate(DIAL.x, DIAL.y); g.rotate(-(R.ct + YAW) + Math.PI / 2);
      for (const o of [-26, 0, 26]) {
        g.beginPath(); g.moveTo(-80, o); for (let x = -80; x <= 80; x += 8) g.lineTo(x, o + 5 * Math.sin(x * 0.08)); g.strokeStyle = on ? C.accent : "#3d5a72"; g.lineWidth = 4; g.stroke();
      }
      g.restore();
      // Which ways it turns.
      arcArrow(g, DIAL.x, DIAL.y, DIAL.r + 11, -2.45, -1.85, "rgba(232,238,246,0.45)");
      arcArrow(g, DIAL.x, DIAL.y, DIAL.r + 11, -0.7, -1.3, "rgba(232,238,246,0.45)");
      D.text(g, "TURN", DIAL.x, DIAL.y + DIAL.r + 44, 20, on ? C.dim : "#3a4658", "center", 700);
    }

    function slider(g, S, vertical, val, opts) {
      const { on, locked, grab } = opts;
      D.round(g, S.x, S.y, S.w, S.h, 14); g.fillStyle = "#121a25"; g.fill();
      g.lineWidth = 2; g.strokeStyle = grab ? C.amber : C.line; g.stroke();
      if (locked) {
        D.hatch(g, S.x + 4, S.y + 4, S.w - 8, S.h - 8, "rgba(111,127,148,0.18)");
        // A padlock in the middle.
        const lx = S.x + S.w / 2, ly = S.y + S.h / 2;
        g.beginPath(); g.arc(lx, ly - 10, 10, Math.PI, 0); g.strokeStyle = "#6f7f94"; g.lineWidth = 4; g.stroke();
        D.round(g, lx - 15, ly - 10, 30, 24, 5); g.fillStyle = "#6f7f94"; g.fill();
        D.disc(g, lx, ly + 1, 3.5, "#121a25");
      } else {
        const len = vertical ? S.h : S.w, mid = len / 2, pos = mid + (vertical ? -val : val) * (mid - 24);
        // Ticks along the track, the centre mark brighter.
        g.fillStyle = "#2a3446";
        for (let k = -4; k <= 4; k++) {
          const at = mid + (k / 4) * (mid - 24);
          if (vertical) g.fillRect(S.x + 10, S.y + at - 1, S.w - 20, k ? 2 : 3); else g.fillRect(S.x + at - 1, S.y + 10, k ? 2 : 3, S.h - 20);
        }
        g.fillStyle = on ? "rgba(79,195,247,0.35)" : "rgba(79,195,247,0.12)";
        if (vertical) g.fillRect(S.x + 14, S.y + Math.min(mid, pos), S.w - 28, Math.abs(pos - mid));
        else g.fillRect(S.x + Math.min(mid, pos), S.y + 14, Math.abs(pos - mid), S.h - 28);
        // The handle.
        if (vertical) D.round(g, S.x - 8, S.y + pos - 16, S.w + 16, 32, 9); else D.round(g, S.x + pos - 16, S.y - 8, 32, S.h + 16, 9);
        g.fillStyle = grab ? C.amber : on ? "#c9d3e0" : "#4a5566"; g.fill();
        g.fillStyle = "#1b2433";
        if (vertical) g.fillRect(S.x + 4, S.y + pos - 2, S.w - 8, 4); else g.fillRect(S.x + pos - 2, S.y + 4, 4, S.h - 8);
      }
    }

    function drawShiftGlyph(g, x, y, on) {
      // Crests sliding sideways: a wave and an arrow under it.
      g.beginPath(); for (let k = 0; k <= 36; k++) { const xx = x - 18 + k; k ? g.lineTo(xx, y - 6 + 6 * Math.sin(k * 0.35)) : g.moveTo(xx, y - 6); }
      g.strokeStyle = on ? C.accent : "#3d5a72"; g.lineWidth = 3; g.stroke();
    }

    function drawTabs(g, t) {
      if (s.rip.length < 2) return;
      TABS.forEach((T, k) => {
        const on = s.sel === k;
        D.round(g, T.x, T.y, T.w, T.h, 14); g.fillStyle = on ? "#1d2a3c" : "#101721"; g.fill();
        g.lineWidth = on ? 4 : 2; g.strokeStyle = on ? C.amber : C.line; g.stroke();
        // The ripple's glyph: a big slow wave, or a small quick one.
        const A = k ? 7 : 16, f = k ? 0.42 : 0.16, cx = T.x + T.w / 2, cy2 = T.y + 32;
        g.beginPath(); for (let x = -38; x <= 38; x += 2) { const y = cy2 + A * Math.sin(x * f); x > -38 ? g.lineTo(cx + x, y) : g.moveTo(cx + x, y); }
        g.strokeStyle = on ? C.fg : C.dim; g.lineWidth = k ? 3 : 4; g.stroke();
        D.text(g, k ? "SMALL" : "BIG", cx, T.y + T.h - 17, 18, on ? C.amber : C.dim, "center", 700);
      });
    }

    function drawCoil(g, x, y, r) {
      D.disc(g, x + 4, y + 6, r, "rgba(0,0,0,0.45)");
      D.disc(g, x, y, r, C.copper);
      D.ring(g, x, y, r, "#f0c08a", 3);
      for (let k = 0; k < 3; k++) D.ring(g, x, y, r - 8 - k * 6, "#7a4a22", 3);
      D.disc(g, x, y, r - 28, "#141b27");
    }

    return {
      /** For tools (tests and shots): the round's state, read only. */
      peek() {
        if (!s) return null;
        return {
          index: s.index, round: s.round, part: s.part, phase: s.phase, err: s.err, hold: s.hold, holdNeed: s.holdNeed,
          cool: s.cool, surge: s.surge, surges: s.surges, sel: s.sel, ripples: s.rip.length, coil: { ...s.coil },
          ctrl: s.rip.map((p) => ({ t: p.ct, s: p.cs, l: p.cl })), need: s.rip.map(need), clock, log: log.map((e) => ({ ...e })),
          layout: { GRAPH, DIAL, SHIFT, STRETCH, TABS, CRATE, GEAR, hub: proj(0, 0, DECK_Z).slice(0, 2), surface: proj(0, 0, 0).slice(0, 2) },
        };
      },
      step(index, isPart) { start(index, isPart); },
      update(dt, input) {
        clock += dt;
        s.t += dt;
        s.cool = Math.max(0, s.cool - dt);
        s.surge = Math.max(0, s.surge - dt);
        if (s.phase === "part") partStep(dt, input);
        else if (s.phase === "play") play(dt, input);
        else if (s.phase === "held") {
          // Played: the field settles onto the ghost while the bar catches up.
          for (const p of s.rip) {
            const L = live(p), k = Math.min(1, dt * 3);
            p.dt.x -= k * wrap(L.t - p.gt); p.ds.x -= k * (L.s - p.gs); p.dl.x -= k * (L.l - p.gl);
          }
          s.err = measure();
        }
      },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        // The graph's panel.
        D.panel(g, GRAPH.x, GRAPH.y, GRAPH.w, GRAPH.h, 18, "#080d15", s.phase === "play" && s.err < HOLD_ERR ? "#245a3e" : C.line);
        g.save(); D.round(g, GRAPH.x + 2, GRAPH.y + 2, GRAPH.w - 4, GRAPH.h - 4, 16); g.clip();
        const jx = s.surge > 1 ? Math.sin(t * 47) * 10 * (s.surge - 1) : 0, jy = s.surge > 1 ? Math.cos(t * 41) * 7 * (s.surge - 1) : 0;
        g.translate(jx, jy);
        drawDeck(g, t);
        if (s.phase !== "part" && s.phase !== "fitted") {
          drawHub(g, t);
          drawRing(g, t, 0);
          drawBolts(g, t, 0);
          drawSurface(g, t);
          drawRing(g, t, 1);
          drawBolts(g, t, 1);
        } else {
          // The field is off: the bare deck, the hub open for its coil.
          drawHub(g, t);
          drawBolts(g, t, 0); drawBolts(g, t, 1);
          if (s.coil.set) { const h = proj(0, 0, DECK_Z); drawCoil(g, h[0], h[1], 34); D.ring(g, h[0], h[1], 46, C.ok, 4); }
        }
        if (s.surge > 0) { g.fillStyle = `rgba(255,71,87,${0.12 * Math.min(1, s.surge)})`; g.fillRect(GRAPH.x - 20, GRAPH.y - 20, GRAPH.w + 40, GRAPH.h + 40); }
        g.restore();

        // The controls, the tabs and, on the part step, the crate.
        if (s.phase === "part" || s.phase === "fitted") {
          D.panel(g, CRATE.x, CRATE.y, CRATE.w, CRATE.h, 14, "#141b27");
          D.hatch(g, CRATE.x + 12, CRATE.y + 12, CRATE.w - 24, 12, "rgba(242,160,70,0.5)");
          if (!s.coil.set) {
            drawCoil(g, s.coil.x, s.coil.y, 40);
            D.ring(g, s.coil.x, s.coil.y, 50 + 3 * Math.sin(t * 5), s.coil.held || s.coil.pad ? C.amber : "rgba(232,238,246,0.35)", 3);
          }
          return;
        }
        const on = s.phase === "play", R = s.rip[s.sel];
        drawDial(g, t);
        slider(g, SHIFT, false, R.cs, { on, grab: s.grab === "shift" });
        drawShiftGlyph(g, SHIFT.x - 58, SHIFT.y + 24, on);
        D.text(g, "SHIFT", SHIFT.x - 58, SHIFT.y + 46, 16, on ? C.dim : "#3a4658", "center", 700);
        slider(g, STRETCH, true, R.cl, { on, locked: s.round < 2, grab: s.grab === "stretch" });
        D.text(g, "STRETCH", STRETCH.x + STRETCH.w / 2, STRETCH.y + STRETCH.h + 28, 17, s.round < 2 ? "#3a4658" : on ? C.dim : "#3a4658", "center", 700);
        drawTabs(g, t);
      },
    };
  },
});
