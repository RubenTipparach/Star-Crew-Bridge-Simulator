/*
 * repairs/hull.js: hull plating, a damaged wall section repaired from inside (openspec/changes/hull-repair, design 3).
 *
 * One wall bay between two ribs, in the crew finish (dark grey steel, rivets), its middle panel buckled, scorched and
 * torn. The job, in the kit's step rule:
 * - Cut out: trace the torch round the buckled panel's outline, either way round; what the torch passes over is cut.
 *   Drifting off the line only pauses. Too fast scores a thin line instead of cutting through: go over it again.
 * - Fit: drag the new plate from the trolley onto the opening (it snaps in), then turn it, dragging round its rim, until
 *   its bolt holes meet the frame's.
 * - Weld: run the bead along each of the plate's four seams. The torch's heat gauge shows the speed: too slow and the
 *   heat climbs into the red and burns through (the fumble "Burned through: a hiss of air", 2 HP of burns, and a hole in
 *   the seam to go over again); too fast leaves a cold bead (grey gaps) to go over again.
 * - Bolt: drive the frame's bolts in star order (KIT.starOrder), the next one lit.
 * A damaged section's job has three steps, so its last step welds and then bolts; a disabled one has five: a part step
 * first (fetch a plate from the stack onto the trolley), then the four.
 *
 * Keys: hold Right or Left (or D, A) to run the torch along the line; the arrows carry the plate and Space drops it;
 * Q and E (or Left and Right) turn it; arrows pick a bolt and Space drives it.
 */
RepairKit.register({
  id: "hull",
  title: "Hull plating",
  place: "Any wall, from inside",
  group: "Hull",
  hazard: "Burned through: a hiss of air",
  down: "Nothing yet; one more hit makes a hole (hull-repair 2)",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const BURNED = "Burned through: a hiss of air";
    const PLAN_DAMAGED = ["cut", "fit", "weldbolt"];
    const PLAN_DISABLED = ["part", "cut", "fit", "weld", "bolt"];

    // ---------------------------------------------------------------- the wall (canvas px)
    const OC = { x: 450, y: 396 };            // the opening's centre
    const PANEL_H = 210;                      // the buckled panel's half size
    const CUT_H = 196, CUT_R = 22;            // the cut line: a rounded square inside the panel's seams
    const PLATE_H = 238;                      // the new plate's half size (it laps the frame)
    const HOLE_H = 218;                       // the frame's bolt holes, from the centre
    const RIBS = [70, 786];                   // the bay's ribs (left edge of each, 44 wide)
    const TROLLEY = { x: 940, y: 470, w: 290, h: 214 };
    const STACK = { x: 940, y: 128, w: 290, h: 250 };
    const SMALL = 0.36;                       // the plate's scale on the trolley
    // ---------------------------------------------------------------- feel
    const BAND = 40;                          // px off the line a torch still works
    const BIN = 8;                            // px of line a coverage bin holds
    const TIP = 7;                            // px either side of the tip a pass covers
    const CUT_FAST = 560;                     // px/s: faster than this scores a thin line
    const CUT_DONE = 0.98;                    // share of the line cut through to free the panel
    const WELD_REF = 420;                     // px/s at which the torch's heat would fall to nothing
    const HEAT_TAU = 0.25;                    // s: the heat follows the speed with this lag
    const COLD = 0.35, BURN = 0.85;           // heat under COLD lays a cold bead; over BURN for BURN_S burns through
    const BURN_S = 0.3, TORCH_COOL_S = 0.8;   // s
    const WELD_DONE = 0.97;                   // share of a seam that must be good
    const KEY_SPEED = 300, KEY_WELD = 150;    // px/s the keys run the torch along a line
    const SNAP = 0.06;                        // rad: the plate's holes meet inside this
    const SPEED_EMA = 0.12;                   // s: the speed's smoothing
    const clamp = (x, a, b) => Math.max(a, Math.min(b, x));
    const wrapQ = (a) => { const q = Math.PI / 2; return a - q * Math.round(a / q); };

    /** A line for the torch: points, arc lengths, bins (0 untouched, 1 thin or cold, 2 done, 3 burned). */
    function line(pts, closed) {
      const P = closed ? [...pts, pts[0]] : pts, cum = [0];
      for (let i = 1; i < P.length; i++) cum.push(cum[i - 1] + Math.hypot(P[i][0] - P[i - 1][0], P[i][1] - P[i - 1][1]));
      const len = cum[cum.length - 1], n = Math.ceil(len / BIN);
      return { pts: P, cum, len, closed, n, bins: new Uint8Array(n), lastS: null, speed: 0 };
    }
    function at(L, sv) {
      sv = L.closed ? ((sv % L.len) + L.len) % L.len : clamp(sv, 0, L.len);
      let i = 1;
      while (i < L.cum.length - 1 && L.cum[i] < sv) i++;
      const a = L.pts[i - 1], b = L.pts[i], k = (sv - L.cum[i - 1]) / Math.max(1e-6, L.cum[i] - L.cum[i - 1]);
      return [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k];
    }
    function project(L, x, y) {
      let best = { d: 1e9, s: 0 };
      for (let i = 1; i < L.pts.length; i++) {
        const [ax, ay] = L.pts[i - 1], [bx, by] = L.pts[i], dx = bx - ax, dy = by - ay, l2 = dx * dx + dy * dy || 1;
        const u = clamp(((x - ax) * dx + (y - ay) * dy) / l2, 0, 1), px = ax + dx * u, py = ay + dy * u, d = Math.hypot(x - px, y - py);
        if (d < best.d) best = { d, s: L.cum[i - 1] + u * Math.sqrt(l2) };
      }
      return best;
    }
    /**
     * The torch on a line this frame, at arc length `sv` (or off it: null): the bins it passed over since the last frame,
     * and its speed along the line, px/s. The one rule both the cut and the weld use.
     */
    function sweep(L, sv, dt) {
      if (sv === null) { L.lastS = null; L.speed *= Math.max(0, 1 - dt / SPEED_EMA); return null; }
      let a = sv, b = sv, ds = 0;
      if (L.lastS !== null) {
        ds = sv - L.lastS;
        if (L.closed) { if (ds > L.len / 2) ds -= L.len; if (ds < -L.len / 2) ds += L.len; }
        if (Math.abs(ds) < 160) { a = Math.min(L.lastS, L.lastS + ds); b = Math.max(L.lastS, L.lastS + ds); } else ds = 0;
      }
      L.lastS = sv;
      L.speed += (Math.abs(ds) / Math.max(dt, 1e-3) - L.speed) * Math.min(1, dt / SPEED_EMA);
      const out = [];
      const i0 = Math.floor((a - TIP) / BIN), i1 = Math.floor((b + TIP) / BIN);
      for (let i = i0; i <= i1; i++) {
        const k = L.closed ? ((i % L.n) + L.n) % L.n : i;
        if (k >= 0 && k < L.n && !out.includes(k)) out.push(k);
      }
      return out;
    }
    const count = (L, v) => L.bins.reduce((n, b) => n + (b === v ? 1 : 0), 0);

    /** The cut line: a rounded square round the opening's centre. */
    function cutLine() {
      const h = CUT_H, r = CUT_R, pts = [];
      const corners = [[h - r, -h + r, -Math.PI / 2], [h - r, h - r, 0], [-h + r, h - r, Math.PI / 2], [-h + r, -h + r, Math.PI]];
      for (const [cx, cy, a0] of corners) for (let k = 0; k <= 6; k++) { const a = a0 + (k / 6) * (Math.PI / 2); pts.push([OC.x + cx + Math.cos(a) * r, OC.y + cy + Math.sin(a) * r]); }
      return line(pts, true);
    }
    const seamLines = () => {
      const h = PLATE_H, x0 = OC.x - h, x1 = OC.x + h, y0 = OC.y - h, y1 = OC.y + h;
      return [line([[x0, y0], [x1, y0]], false), line([[x1, y0], [x1, y1]], false), line([[x1, y1], [x0, y1]], false), line([[x0, y1], [x0, y0]], false)];
    };
    /** The frame's bolt holes round the rim, clockwise from the top left (so KIT.starOrder(8) works across). */
    const HOLES = [[-1, -1], [0, -1], [1, -1], [1, 0], [1, 1], [0, 1], [-1, 1], [-1, 0]].map(([u, v]) => ({ u: u * HOLE_H, v: v * HOLE_H }));
    const holeXY = (h, a = 0) => [OC.x + h.u * Math.cos(a) - h.v * Math.sin(a), OC.y + h.u * Math.sin(a) + h.v * Math.cos(a)];

    // ---------------------------------------------------------------- state
    let wall = null, s = null;
    const shares = {};
    function freshWall(r) {
      return {
        cut: false, fall: 1, plate: { x: TROLLEY.x + TROLLEY.w / 2, y: TROLLEY.y + 92, a: (r() < 0.5 ? -1 : 1) * (0.4 + 0.35 * r()), scale: SMALL, placed: false, fitted: false, onTrolley: true },
        welded: false, seams: null, bolts: HOLES.map(() => ({ on: false, t: 0 })), scorch: [0.3 + 0.4 * r(), 0.25 + 0.3 * r()],
        creases: Array.from({ length: 7 }, () => [r(), r(), r(), r()]), stacked: 6,
      };
    }
    function start(index, isPart) {
      const r = api.rand();
      if (!wall) wall = freshWall(r);
      if (shares[index] === undefined) shares[index] = (100 - api.value) / Math.max(1, api.steps - index) / (KIT.RATES[api.who] || KIT.RATES.officer);
      const plan = api.steps >= 5 ? PLAN_DISABLED : PLAN_DAMAGED;
      const kind = isPart ? "part" : plan[Math.min(index, plan.length - 1)];
      s = { kind, phase: kind === "weldbolt" ? "weld" : kind, heat: 0, burnT: 0, cool: 0, hiss: [], sparks: [], cur: 0, keys: false, keyS: 0,
        grab: null, prevA: 0, wrong: null, played: false, carry: null, sel: 0 };
      if (kind === "part") { wall.plate.onTrolley = false; s.carry = { x: STACK.x + STACK.w / 2, y: STACK.y + STACK.h / 2 + 10, held: false, dx: 0, dy: 0 }; }
      if (kind === "cut") s.line = cutLine();
      if (kind === "fit" && !wall.cut) { wall.cut = true; wall.fall = 1; }
      if (s.phase === "weld") { wall.cut = true; wall.plate.placed = wall.plate.fitted = true; wall.plate.a = 0; wall.plate.x = OC.x; wall.plate.y = OC.y; wall.plate.scale = 1; wall.seams = seamLines(); }
      if (s.phase === "bolt") { wall.welded = true; wall.plate.placed = wall.plate.fitted = true; wall.plate.a = 0; wall.bolts.forEach((b) => { b.on = false; b.t = 0; }); }
    }

    // ---------------------------------------------------------------- the rounds
    function torchPos(input, L) {
      if (input.down) { const q = project(L, input.x, input.y); return q.d < BAND ? q.s : null; }
      return null;
    }
    function keyRun(input, L, speed, dt) {
      const dir = (input.keys.has("ArrowRight") || input.keys.has("KeyD") ? 1 : 0) - (input.keys.has("ArrowLeft") || input.keys.has("KeyA") ? 1 : 0);
      if (!dir) return null;
      s.keys = true;
      s.keyS = L.closed ? s.keyS + dir * speed * dt : clamp(s.keyS + dir * speed * dt, 0, L.len);
      return s.keyS;
    }
    function spark(x, y, n, col, speed) {
      for (let i = 0; i < n && s.sparks.length < 300; i++) {
        const a = Math.random() * Math.PI * 2, v = speed * (0.3 + 0.7 * Math.random());
        s.sparks.push({ x, y, vx: Math.cos(a) * v, vy: Math.sin(a) * v - 60, life: 0.4 + 0.3 * Math.random(), col });
      }
    }

    function cutUpdate(dt, input) {
      const L = s.line;
      if (wall.cut) return;
      let sv = keyRun(input, L, KEY_SPEED, dt);
      if (sv === null) sv = torchPos(input, L);
      const hit = sweep(L, sv, dt);
      if (!hit) return;
      const fast = L.speed > CUT_FAST;
      for (const k of hit) { if (fast) { if (L.bins[k] === 0) L.bins[k] = 1; } else L.bins[k] = 2; }
      const [x, y] = at(L, sv);
      s.tip = { x, y, fast };
      spark(x, y, fast ? 1 : 3, fast ? "#ffd27a" : "#ffb347", fast ? 160 : 320);
      if (count(L, 2) >= L.n * CUT_DONE) { L.bins.fill(2); wall.cut = true; wall.fall = 0; s.tip = null; api.stepDone(); }
    }

    function partUpdate(dt, input) {
      const p = s.carry;
      if (!p) return;
      if (input.stick.x || input.stick.y) { p.x += input.stick.x * 480 * dt; p.y += input.stick.y * 480 * dt; s.keys = true; }
      if (input.pressed && Math.abs(input.x - p.x) < 120 && Math.abs(input.y - p.y) < 100) { p.held = true; p.dx = p.x - input.x; p.dy = p.y - input.y; s.keys = false; }
      if (p.held && input.down) { p.x = input.x + p.dx; p.y = input.y + p.dy; }
      const onTrolley = p.x > TROLLEY.x - 20 && p.x < TROLLEY.x + TROLLEY.w + 20 && p.y > TROLLEY.y - 30 && p.y < TROLLEY.y + TROLLEY.h;
      if ((p.held && !input.down) || (s.keys && input.actionPressed)) {
        p.held = false;
        if (onTrolley) { s.carry = null; wall.stacked--; wall.plate.onTrolley = true; api.stepDone(); return; }
      }
      if (!p.held && !s.keys) { const hx = STACK.x + STACK.w / 2, hy = STACK.y + STACK.h / 2 + 10; p.x += (hx - p.x) * Math.min(1, dt * 10); p.y += (hy - p.y) * Math.min(1, dt * 10); }
      p.x = clamp(p.x, 40, api.W - 40); p.y = clamp(p.y, api.BAR_H + 40, api.H - 40);
    }

    function fitUpdate(dt, input) {
      const P = wall.plate;
      if (P.fitted) return;
      const home = [TROLLEY.x + TROLLEY.w / 2, TROLLEY.y + 92];
      // Keys: carry with the arrows and drop with Space; once in, turn with Q and E (or Left and Right).
      if (!P.placed) {
        if (input.stick.x || input.stick.y) { P.x += input.stick.x * 520 * dt; P.y += input.stick.y * 520 * dt; s.keys = true; P.onTrolley = false; }
        if (s.keys && input.actionPressed && Math.hypot(P.x - OC.x, P.y - OC.y) < 120) P.placed = true;
      } else {
        const k = (input.keys.has("KeyE") || input.keys.has("ArrowRight") ? 1 : 0) - (input.keys.has("KeyQ") || input.keys.has("ArrowLeft") ? 1 : 0);
        if (k) { P.a += k * 0.8 * dt; s.keys = true; }
      }
      // Pointer: a press near the plate's middle carries it; a press on its rim, once it is in, turns it.
      if (input.pressed) {
        const d = Math.hypot(input.x - P.x, input.y - P.y), R = PLATE_H * P.scale;
        if (P.placed && d > 70 && d < R + 40) { s.grab = "turn"; s.prevA = Math.atan2(input.y - P.y, input.x - P.x); s.keys = false; }
        else if (d < R + 20) { s.grab = "carry"; s.dx = P.x - input.x; s.dy = P.y - input.y; P.placed = false; P.onTrolley = false; s.keys = false; }
      }
      if (s.grab === "carry" && input.down) { P.x = input.x + s.dx; P.y = input.y + s.dy; }
      if (s.grab === "turn" && input.down) {
        const a = Math.atan2(input.y - P.y, input.x - P.x); let d = a - s.prevA; d = Math.atan2(Math.sin(d), Math.cos(d));
        P.a += d; s.prevA = a;
      }
      if (!input.down && s.grab) {
        if (s.grab === "carry") { if (Math.hypot(P.x - OC.x, P.y - OC.y) < 90) P.placed = true; else P.onTrolley = true; }
        s.grab = null;
      }
      // Where it goes: snapped into the opening, back on the trolley, or in the hand; full size over the wall.
      if (P.placed) { P.x += (OC.x - P.x) * Math.min(1, dt * 12); P.y += (OC.y - P.y) * Math.min(1, dt * 12); }
      else if (P.onTrolley && !s.grab && !s.keys) { P.x += (home[0] - P.x) * Math.min(1, dt * 10); P.y += (home[1] - P.y) * Math.min(1, dt * 10); }
      P.scale += ((P.x < 880 ? 1 : SMALL) - P.scale) * Math.min(1, dt * 8);
      if (P.placed && Math.hypot(P.x - OC.x, P.y - OC.y) < 3 && Math.abs(wrapQ(P.a)) < SNAP) {
        P.a -= wrapQ(P.a); P.x = OC.x; P.y = OC.y; P.scale = 1; P.fitted = true; s.grab = null; api.stepDone();
      }
    }

    function weldUpdate(dt, input) {
      const seams = wall.seams;
      s.cool = Math.max(0, s.cool - dt);
      for (const h of s.hiss) h.t += dt;
      s.hiss = s.hiss.filter((h) => h.t < 2.5);
      if (wall.welded) return;
      // Which seam: the one the torch is on (sticky while it stays near), or the keys' current one.
      let sv = null, L = null;
      if (input.hit.has("Tab")) { s.cur = (s.cur + 1) % 4; s.keyS = 0; }
      if (!seams[s.cur] || count(seams[s.cur], 2) >= seams[s.cur].n * WELD_DONE) {
        const i = seams.findIndex((q) => count(q, 2) < q.n * WELD_DONE); if (i >= 0 && s.keys) { s.cur = i; s.keyS = 0; }
      }
      if (input.keys.has("Space")) {
        // Keys: Space runs the torch along the current seam at a good speed.
        s.keys = true; s.keyS = Math.min(seams[s.cur].len, s.keyS + KEY_WELD * dt); L = seams[s.cur]; sv = s.keyS;
      } else if (input.down) {
        let best = null;
        // The seam under the torch; the one already in hand wins a near tie (a corner), so a pass does not jump seams.
        seams.forEach((q, i) => { const p = project(q, input.x, input.y), d = p.d - (i === s.cur ? 12 : 0); if (p.d < BAND && (!best || d < best.k)) best = { i, k: d, ...p }; });
        if (best) { s.cur = best.i; L = seams[best.i]; sv = best.s; s.keys = false; }
      }
      seams.forEach((q) => { if (q !== L) sweep(q, null, dt); });
      if (s.cool > 0) { if (L) sweep(L, null, dt); s.heat = Math.max(0, s.heat - dt / 0.6); s.tip = null; return; }
      const hit = L ? sweep(L, sv, dt) : null;
      if (!hit) { s.heat = Math.max(0, s.heat - dt / 0.6); s.burnT = 0; s.tip = null; return; }
      // The heat follows the speed: slow is hot, fast is cold.
      const target = clamp(1 - L.speed / WELD_REF, 0, 1);
      s.heat += (target - s.heat) * Math.min(1, dt / HEAT_TAU);
      const [x, y] = at(L, sv);
      s.tip = { x, y };
      if (s.heat > BURN) {
        s.burnT += dt;
        if (s.burnT > BURN_S) {
          // Burned through: a hole in the seam, air hissing out of it, the torch off a moment.
          const k0 = Math.floor(sv / BIN);
          for (let k = k0 - 2; k <= k0 + 2; k++) if (k >= 0 && k < L.n) L.bins[k] = 3;
          s.hiss.push({ x, y, t: 0 }); s.heat = 0.45; s.burnT = 0; s.cool = TORCH_COOL_S; L.lastS = null;
          api.fumble(BURNED);
          return;
        }
      } else s.burnT = 0;
      const v = s.heat < COLD ? 1 : 2;
      for (const k of hit) if (!(v === 1 && L.bins[k] === 2)) L.bins[k] = v;
      spark(x, y, 2, s.heat > BURN ? "#ffffff" : "#ffd27a", 200);
      if (seams.every((q) => count(q, 2) >= q.n * WELD_DONE)) {
        seams.forEach((q) => q.bins.fill(2)); wall.welded = true; s.tip = null;
        if (s.kind === "weldbolt") { s.phase = "bolt"; wall.bolts.forEach((b) => { b.on = false; b.t = 0; }); } else api.stepDone();
      }
    }

    const nextBolt = () => { const S = KIT.starOrder(8); const done = wall.bolts.filter((b) => b.on).length; return done < 8 ? S[done] : -1; };
    function boltUpdate(dt, input) {
      for (const b of wall.bolts) if (b.on) b.t = Math.min(1, b.t + dt / 0.35);
      if (s.wrong && (s.wrong.t -= dt) <= 0) s.wrong = null;
      if (s.played) return;
      const drive = (i) => {
        if (wall.bolts[i].on) return;
        if (i !== nextBolt()) { s.wrong = { i, t: 0.7 }; api.say("Out of order"); return; }
        wall.bolts[i].on = true;
        if (wall.bolts.every((b) => b.on)) { s.played = true; api.stepDone(); }
      };
      for (const [code, d] of [["ArrowLeft", -1], ["KeyA", -1], ["ArrowUp", -1], ["ArrowRight", 1], ["KeyD", 1], ["ArrowDown", 1], ["Tab", 1]]) {
        if (input.hit.has(code)) { s.keys = true; s.sel = (s.sel + d + 8) % 8; }
      }
      if (input.actionPressed) { s.keys = true; drive(s.sel); }
      if (input.pressed) { const i = KIT.nearest(HOLES.map((h) => holeXY(h)), input.x, input.y, KIT.TOUCH_R + 10); if (i >= 0) { s.keys = false; drive(i); } }
    }

    // ---------------------------------------------------------------- drawing
    const PAINT = "#4d4a46", PAINT_HI = "#625e58", PAINT_LO = "#36332f", RIVET = "#8b867d";
    function rivets(g, x0, y0, x1, y1, step = 30) {
      const n = Math.max(1, Math.round(Math.hypot(x1 - x0, y1 - y0) / step));
      for (let i = 0; i <= n; i++) {
        const x = x0 + ((x1 - x0) * i) / n, y = y0 + ((y1 - y0) * i) / n;
        D.disc(g, x + 1, y + 1.5, 3.4, "rgba(0,0,0,0.5)"); D.disc(g, x, y, 3.2, RIVET); D.disc(g, x - 1, y - 1, 1.2, "#cfc8bb");
      }
    }
    function steelPanel(g, x, y, w, h, base = PAINT) {
      const gr = g.createLinearGradient(x, y, x + w * 0.4, y + h);
      gr.addColorStop(0, PAINT_HI); gr.addColorStop(0.5, base); gr.addColorStop(1, PAINT_LO);
      g.fillStyle = gr; g.fillRect(x, y, w, h);
      g.strokeStyle = "rgba(255,255,255,0.05)"; g.lineWidth = 1;
      for (let yy = y + 4; yy < y + h; yy += 6) { g.beginPath(); g.moveTo(x + 2, yy); g.lineTo(x + w - 2, yy); g.stroke(); }
      g.strokeStyle = "#1c1a18"; g.lineWidth = 3; g.strokeRect(x + 1.5, y + 1.5, w - 3, h - 3);
      g.strokeStyle = "rgba(255,255,255,0.08)"; g.lineWidth = 1; g.strokeRect(x + 4, y + 4, w - 8, h - 8);
    }
    function drawWall(g) {
      g.save(); D.round(g, 16, 84, 868, 628, 16); g.clip();
      g.fillStyle = "#2a2826"; g.fillRect(16, 84, 868, 628);
      // Neighbouring panels round the bay: above, below and beside the buckled one.
      const x0 = OC.x - PANEL_H, x1 = OC.x + PANEL_H, y0 = OC.y - PANEL_H, y1 = OC.y + PANEL_H;
      steelPanel(g, RIBS[0] + 44, 124, x0 - RIBS[0] - 44, 548);
      steelPanel(g, x1, 124, RIBS[1] - x1, 548);
      steelPanel(g, x0, 124, x1 - x0, y0 - 124);
      steelPanel(g, x0, y1, x1 - x0, 672 - y1);
      // A vent grille on the left neighbour and a stencil on the right: the crew finish's panels are not all alike.
      for (let k = 0; k < 6; k++) { D.round(g, RIBS[0] + 64, 300 + k * 18, 70, 8, 4); g.fillStyle = "#1c1a18"; g.fill(); }
      D.hatch(g, x1 + 20, 520, RIBS[1] - x1 - 40, 14, "rgba(209,160,36,0.55)");
      // The ribs: a pillar each side with its base and capital, and the trims top and bottom.
      for (const rx of RIBS) {
        const gr = g.createLinearGradient(rx, 0, rx + 44, 0); gr.addColorStop(0, "#2f2c29"); gr.addColorStop(0.5, "#6a655d"); gr.addColorStop(1, "#2a2724");
        g.fillStyle = gr; g.fillRect(rx, 84, 44, 628);
        g.fillStyle = "#57534c"; g.fillRect(rx - 6, 112, 56, 16); g.fillRect(rx - 6, 664, 56, 18);
        rivets(g, rx + 22, 150, rx + 22, 640, 40);
      }
      g.fillStyle = "#3a3733"; g.fillRect(16, 96, 868, 24); g.fillRect(16, 676, 868, 30);
      g.fillStyle = "rgba(255,240,210,0.5)"; g.fillRect(RIBS[0] + 60, 100, RIBS[1] - RIBS[0] - 76, 6);   // the light strip in the trim
      rivets(g, x0 + 10, 136, x1 - 10, 136); rivets(g, x0 + 10, 660, x1 - 10, 660);
      g.restore();
    }
    /** Behind the panel: frame, insulation and a cable tray, seen once the panel is out. */
    function drawCavity(g) {
      const h = CUT_H;
      g.save(); D.round(g, OC.x - h, OC.y - h, 2 * h, 2 * h, CUT_R); g.clip();
      g.fillStyle = "#121110"; g.fillRect(OC.x - h, OC.y - h, 2 * h, 2 * h);
      // Quilted insulation.
      for (let y = OC.y - h; y < OC.y + h; y += 36) for (let x = OC.x - h; x < OC.x + h; x += 36) { D.round(g, x + 3, y + 3, 30, 30, 8); g.fillStyle = "#4a4436"; g.fill(); D.disc(g, x + 18, y + 18, 3, "#2b271f"); }
      // Two frame members and a cable tray across.
      g.fillStyle = "#24282e"; g.fillRect(OC.x - 110, OC.y - h, 26, 2 * h); g.fillRect(OC.x + 84, OC.y - h, 26, 2 * h);
      g.fillStyle = "#30353d"; g.fillRect(OC.x - h, OC.y + 40, 2 * h, 34);
      for (const [c, y] of [["#b0473f", 48], ["#d1a024", 56], ["#4f8fd0", 64]]) { g.fillStyle = c; g.fillRect(OC.x - h, OC.y + y, 2 * h, 4); }
      g.restore();
      g.strokeStyle = "#0b0a09"; g.lineWidth = 4; D.round(g, OC.x - h, OC.y - h, 2 * h, 2 * h, CUT_R); g.stroke();
    }
    /** The buckled panel: dented, creased, scorched, a corner torn back to the frame. */
    function drawBuckled(g, t) {
      const h = PANEL_H;
      g.save();
      if (wall.cut) {
        const f = wall.fall;
        g.translate(OC.x + f * 60, OC.y + f * f * 700); g.rotate(f * 0.5); g.globalAlpha = Math.max(0, 1 - f * 1.2); g.translate(-OC.x, -OC.y);
        g.beginPath(); g.roundRect(OC.x - CUT_H, OC.y - CUT_H, 2 * CUT_H, 2 * CUT_H, CUT_R); g.clip();
      }
      steelPanel(g, OC.x - h, OC.y - h, 2 * h, 2 * h, "#47443f");
      // The buckle: a dent off centre, lit from the upper left.
      const bx = OC.x - 40, by = OC.y + 20;
      const dent = g.createRadialGradient(bx + 30, by + 34, 10, bx, by, 190);
      dent.addColorStop(0, "rgba(0,0,0,0.55)"); dent.addColorStop(0.45, "rgba(0,0,0,0.25)"); dent.addColorStop(0.7, "rgba(255,255,255,0.07)"); dent.addColorStop(1, "rgba(0,0,0,0)");
      g.fillStyle = dent; g.fillRect(OC.x - h, OC.y - h, 2 * h, 2 * h);
      // Creases from the dent outwards: dark folds with a lit edge.
      wall.creases.forEach(([a, b, c, d]) => {
        const ang = a * Math.PI * 2, r0 = 30 + 40 * b, r1 = 130 + 80 * c, bend = (d - 0.5) * 0.6;
        const p0 = [bx + Math.cos(ang) * r0, by + Math.sin(ang) * r0], p1 = [bx + Math.cos(ang + bend) * r1, by + Math.sin(ang + bend) * r1];
        const pm = [bx + Math.cos(ang + bend / 2) * (r0 + r1) / 2 + 10, by + Math.sin(ang + bend / 2) * (r0 + r1) / 2];
        g.lineCap = "round";
        g.strokeStyle = "rgba(15,13,12,0.75)"; g.lineWidth = 4; g.beginPath(); g.moveTo(...p0); g.lineTo(...pm); g.lineTo(...p1); g.stroke();
        g.strokeStyle = "rgba(200,190,175,0.25)"; g.lineWidth = 2; g.beginPath(); g.moveTo(p0[0] - 2, p0[1] - 3); g.lineTo(pm[0] - 2, pm[1] - 3); g.lineTo(p1[0] - 2, p1[1] - 3); g.stroke();
        g.lineCap = "butt";
      });
      // Scorch: soot round where the blast hit, the paint blistered brown at its edge.
      const sx = OC.x - h + wall.scorch[0] * 2 * h, sy = OC.y - h + wall.scorch[1] * 2 * h;
      const sc = g.createRadialGradient(sx, sy, 6, sx, sy, 150);
      sc.addColorStop(0, "rgba(8,6,5,0.92)"); sc.addColorStop(0.35, "rgba(30,20,12,0.7)"); sc.addColorStop(0.6, "rgba(90,52,24,0.35)"); sc.addColorStop(1, "rgba(0,0,0,0)");
      g.fillStyle = sc; g.fillRect(OC.x - h, OC.y - h, 2 * h, 2 * h);
      // Soot carried up from it in soft streaks.
      for (let k = 0; k < 5; k++) {
        const x = sx - 48 + k * 24, top = sy - 170 + ((k * 37) % 50), st = g.createLinearGradient(0, top, 0, sy - 30);
        st.addColorStop(0, "rgba(10,8,6,0)"); st.addColorStop(1, "rgba(10,8,6,0.4)");
        g.fillStyle = st; D.round(g, x - 7, top, 14, sy - 30 - top, 7); g.fill();
      }
      // A corner torn back: jagged, the frame dark behind it, the lip bent up.
      const cx = OC.x + h, cy = OC.y - h;
      g.beginPath(); g.moveTo(cx, cy + 4); g.lineTo(cx - 92, cy + 4); g.lineTo(cx - 70, cy + 22); g.lineTo(cx - 82, cy + 40); g.lineTo(cx - 48, cy + 52); g.lineTo(cx - 40, cy + 78); g.lineTo(cx - 14, cy + 70); g.lineTo(cx - 4, cy + 104); g.lineTo(cx, cy + 104); g.closePath();
      g.fillStyle = "#0d0c0b"; g.fill();
      g.strokeStyle = "#a59d90"; g.lineWidth = 2; g.stroke();
      g.fillStyle = "#6a655d"; g.beginPath(); g.moveTo(cx - 92, cy + 4); g.lineTo(cx - 70, cy + 22); g.lineTo(cx - 104, cy + 30); g.closePath(); g.fill();
      // The panel's rivets, some sprung.
      rivets(g, OC.x - h + 14, OC.y - h + 14, OC.x + h - 110, OC.y - h + 14);
      rivets(g, OC.x - h + 14, OC.y + h - 14, OC.x + h - 14, OC.y + h - 14);
      rivets(g, OC.x - h + 14, OC.y - h + 14, OC.x - h + 14, OC.y + h - 14);
      rivets(g, OC.x + h - 14, OC.y - h + 110, OC.x + h - 14, OC.y + h - 14);
      // A cracked lamp cover hanging loose (design 2's looks for 25-49%).
      g.save(); g.translate(OC.x + 100, OC.y - 150); g.rotate(0.35 + 0.03 * Math.sin(t * 2));
      D.round(g, -50, 0, 100, 22, 6); g.fillStyle = "#c8c0a8"; g.fill(); g.strokeStyle = "#3a3733"; g.lineWidth = 2; g.stroke();
      g.strokeStyle = "#3a3733"; g.beginPath(); g.moveTo(-20, 0); g.lineTo(-6, 12); g.lineTo(10, 4); g.lineTo(22, 22); g.stroke();
      g.restore();
      g.restore();
    }
    /** The new plate: clean steel in the crew finish, its eight bolt holes, a stencil band. */
    function drawPlate(g, P, t) {
      const h = PLATE_H;
      g.save(); g.translate(P.x, P.y); g.rotate(P.a); g.scale(P.scale, P.scale);
      g.fillStyle = "rgba(0,0,0,0.45)"; g.fillRect(-h + 10, -h + 14, 2 * h, 2 * h);
      steelPanel(g, -h, -h, 2 * h, 2 * h, "#57534d");
      D.hatch(g, -h + 40, h - 60, 140, 16, "rgba(209,160,36,0.6)");
      g.fillStyle = "rgba(230,224,210,0.35)"; g.fillRect(h - 150, -h + 34, 110, 10); g.fillRect(h - 150, -h + 52, 70, 10);
      for (const ho of HOLES) {
        const b = wall.bolts[HOLES.indexOf(ho)];
        D.disc(g, ho.u, ho.v, 11, "#141210"); D.ring(g, ho.u, ho.v, 11, "#8b867d", 2);
        if (b && b.on) {
          // A driven bolt: a hex head that spins in as it seats.
          g.save(); g.translate(ho.u, ho.v); g.rotate(b.t * 6);
          g.beginPath(); for (let k = 0; k < 6; k++) { const a = Math.PI / 6 + (k * Math.PI) / 3; g[k ? "lineTo" : "moveTo"](Math.cos(a) * 14, Math.sin(a) * 14); } g.closePath();
          g.fillStyle = "#b7b0a4"; g.fill(); g.strokeStyle = "#4a4640"; g.lineWidth = 2; g.stroke();
          g.restore();
        }
      }
      g.restore();
    }
    function drawSeams(g, t) {
      for (const L of wall.seams) {
        for (let k = 0; k < L.n; k++) {
          const [x0, y0] = at(L, k * BIN), [x1, y1] = at(L, Math.min(L.len, (k + 1) * BIN + 0.5)), b = L.bins[k];
          if (b === 0) { g.strokeStyle = "rgba(242,160,70,0.45)"; g.lineWidth = 2; g.setLineDash([5, 5]); }
          else if (b === 1) { g.strokeStyle = "#7d8796"; g.lineWidth = 6; g.setLineDash([3, 5]); }
          else if (b === 2) { g.strokeStyle = "#d9cfbf"; g.lineWidth = 9; g.setLineDash([]); }
          else { g.strokeStyle = "#050404"; g.lineWidth = 11; g.setLineDash([]); }
          g.beginPath(); g.moveTo(x0, y0); g.lineTo(x1, y1); g.stroke();
          // A good bead's ripples, so done reads by shape as well as tone.
          if (b === 2 && k % 1 === 0) { const mx = (x0 + x1) / 2, my = (y0 + y1) / 2; D.ring(g, mx, my, 3.5, "rgba(120,100,70,0.8)", 1.5); }
        }
        g.setLineDash([]);
      }
    }
    function drawTorch(g, tip, hot, t) {
      if (!tip) return;
      g.save(); g.translate(tip.x, tip.y);
      g.strokeStyle = "#3a4658"; g.lineWidth = 14; g.lineCap = "round"; g.beginPath(); g.moveTo(14, 14); g.lineTo(80, 70); g.stroke();
      g.strokeStyle = "#c08a3e"; g.lineWidth = 8; g.beginPath(); g.moveTo(8, 8); g.lineTo(20, 20); g.stroke(); g.lineCap = "butt";
      const glow = g.createRadialGradient(0, 0, 0, 0, 0, 30);
      glow.addColorStop(0, "rgba(255,255,255,1)"); glow.addColorStop(0.3, hot > BURN ? "rgba(255,120,90,0.9)" : "rgba(150,210,255,0.85)"); glow.addColorStop(1, "rgba(79,195,247,0)");
      D.disc(g, 0, 0, 30 + 3 * Math.sin(t * 40), glow);
      g.restore();
    }
    /** The torch's gauge at the right: speed for the cut (too fast hatched), heat for the weld (cold and burn hatched). */
    function drawGauge(g, kind, v) {
      const G = { x: 1050, y: 150, w: 64, h: 420 };
      D.panel(g, G.x - 70, G.y - 60, G.w + 140, G.h + 120, 18, "#0c121a");
      D.round(g, G.x, G.y, G.w, G.h, 14); g.fillStyle = "#121a25"; g.fill();
      const yAt = (u) => G.y + G.h - u * G.h;
      if (kind === "heat") {
        D.hatch(g, G.x, yAt(COLD), G.w, COLD * G.h, "rgba(79,168,247,0.55)");
        g.fillStyle = "rgba(61,220,132,0.25)"; g.fillRect(G.x, yAt(BURN), G.w, (BURN - COLD) * G.h);
        D.hatch(g, G.x, G.y, G.w, (1 - BURN) * G.h, "rgba(255,71,87,0.7)");
      } else {
        const f = 1 / 1.5;
        g.fillStyle = "rgba(61,220,132,0.25)"; g.fillRect(G.x, yAt(f), G.w, f * G.h);
        D.hatch(g, G.x, G.y, G.w, (1 - f) * G.h, "rgba(242,160,70,0.6)");
      }
      const y = yAt(clamp(v, 0, 1));
      g.fillStyle = C.fg; g.beginPath(); g.moveTo(G.x - 18, y - 12); g.lineTo(G.x - 2, y); g.lineTo(G.x - 18, y + 12); g.closePath(); g.fill();
      g.fillRect(G.x, y - 2, G.w, 4);
      // The torch icon over it and one word under it.
      g.save(); g.translate(G.x + G.w / 2, G.y - 30); g.rotate(-0.7);
      g.fillStyle = "#c9d3e0"; D.round(g, -22, -6, 36, 12, 5); g.fill(); g.fillStyle = "#ffb347"; g.beginPath(); g.moveTo(14, -5); g.lineTo(28, 0); g.lineTo(14, 5); g.closePath(); g.fill();
      g.restore();
      D.text(g, kind === "heat" ? "HEAT" : "SPEED", G.x + G.w / 2, G.y + G.h + 30, 20, C.dim, "center", 700);
    }
    function drawTrolley(g, t) {
      D.panel(g, TROLLEY.x, TROLLEY.y, TROLLEY.w, TROLLEY.h, 16, "#151b25");
      g.fillStyle = "#3a4658"; g.fillRect(TROLLEY.x + 20, TROLLEY.y + TROLLEY.h - 40, TROLLEY.w - 40, 12);
      for (const wx of [TROLLEY.x + 46, TROLLEY.x + TROLLEY.w - 46]) { D.disc(g, wx, TROLLEY.y + TROLLEY.h - 18, 14, "#0b0f15"); D.ring(g, wx, TROLLEY.y + TROLLEY.h - 18, 14, "#5a6577", 3); }
    }
    function drawStack(g, t) {
      D.panel(g, STACK.x, STACK.y, STACK.w, STACK.h, 16, "#151b25");
      D.hatch(g, STACK.x + 14, STACK.y + 14, STACK.w - 28, 12, "rgba(242,160,70,0.5)");
      const n = wall.stacked - (s.carry ? 1 : 0);
      for (let k = 0; k < n; k++) {
        const y = STACK.y + STACK.h - 40 - k * 9;
        g.fillStyle = k % 2 ? "#57534d" : "#4d4a46"; g.fillRect(STACK.x + 60, y, STACK.w - 120, 8);
        g.fillStyle = "#1c1a18"; g.fillRect(STACK.x + 60, y + 7, STACK.w - 120, 1);
      }
    }

    return {
      /** For tools (tests and shots): the round's state, read only. */
      peek() {
        if (!s) return null;
        const P = wall.plate;
        const out = { kind: s.kind, phase: s.phase, OC, PLATE_H, cut: wall.cut, welded: wall.welded, heat: s.heat, cool: s.cool, keys: s.keys,
          plate: { x: P.x, y: P.y, a: P.a, placed: P.placed, fitted: P.fitted, scale: P.scale, onTrolley: P.onTrolley },
          bolts: wall.bolts.map((b, i) => ({ on: b.on, xy: holeXY(HOLES[i]) })), next: nextBolt(), trolley: TROLLEY, stack: STACK, carry: s.carry ? { ...s.carry } : null };
        if (s.line) out.cutLine = { n: s.line.n, cut: count(s.line, 2), thin: count(s.line, 1), len: s.line.len, pts: Array.from({ length: 80 }, (_, i) => at(s.line, (i * s.line.len) / 80)) };
        if (wall.seams) out.seams = wall.seams.map((q) => ({ n: q.n, good: count(q, 2), cold: count(q, 1), burned: count(q, 3), a: q.pts[0], b: q.pts[q.pts.length - 1] }));
        return out;
      },
      step(index, isPart) { start(index, isPart); },
      update(dt, input) {
        if (wall.cut && wall.fall < 1) wall.fall = Math.min(1, wall.fall + dt * 1.4);
        for (const p of s.sparks) { p.vy += 700 * dt; p.x += p.vx * dt; p.y += p.vy * dt; p.life -= dt; }
        s.sparks = s.sparks.filter((p) => p.life > 0);
        if (s.phase === "part") partUpdate(dt, input);
        else if (s.phase === "cut") cutUpdate(dt, input);
        else if (s.phase === "fit") fitUpdate(dt, input);
        else if (s.phase === "weld") weldUpdate(dt, input);
        else if (s.phase === "bolt") boltUpdate(dt, input);
        if (s.phase === "cut" && !input.down && !s.keys) s.tip = null;
      },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        drawWall(g);
        const P = wall.plate;
        if (wall.cut) drawCavity(g);
        if (!wall.cut || wall.fall < 1) drawBuckled(g, t);
        // The frame's bolt holes round the opening, ringed while the plate is to be fitted.
        if (wall.cut && !P.fitted) for (const ho of HOLES) { const [x, y] = holeXY(ho); D.disc(g, x, y, 11, "#0b0a09"); D.ring(g, x, y, 15, "rgba(242,160,70,0.8)", 3); }
        if (s.phase === "cut" && s.line && !wall.cut) {
          // The cut line: dashed where it is still whole, a thin scored line where it went too fast, open where cut.
          const L = s.line;
          for (let k = 0; k < L.n; k++) {
            const [x0, y0] = at(L, k * BIN), [x1, y1] = at(L, (k + 1) * BIN + 0.5), b = L.bins[k];
            g.beginPath(); g.moveTo(x0, y0); g.lineTo(x1, y1);
            if (b === 0) { g.strokeStyle = "rgba(242,160,70,0.85)"; g.lineWidth = 3; g.setLineDash([8, 6]); }
            else if (b === 1) { g.strokeStyle = "#ffd27a"; g.lineWidth = 2; g.setLineDash([]); }
            else { g.strokeStyle = "#050404"; g.lineWidth = 8; g.setLineDash([]); }
            g.stroke();
            if (b === 2) { g.strokeStyle = "rgba(255,140,60,0.5)"; g.lineWidth = 2; g.stroke(); }
          }
          g.setLineDash([]);
        }
        // The plate: on the trolley, in the hand, or in the opening.
        if (s.phase === "fit" || s.phase === "weld" || s.phase === "bolt") {
          if (s.phase === "fit") drawTrolley(g, t);
          drawPlate(g, P, t);
          // The holes meeting: green rings where the plate's holes sit over the frame's.
          if (s.phase === "fit" && P.placed) {
            const near = Math.abs(wrapQ(P.a)) < 0.12;
            for (const ho of HOLES) { const [x, y] = holeXY(ho, P.a); D.ring(g, x, y, 16, near ? C.ok : "rgba(232,238,246,0.5)", 3); }
            // The turn: an arrow round the rim the way that closes the gap.
            if (!P.fitted) { const k = wrapQ(P.a) > 0 ? -1 : 1, R = PLATE_H + 26, a0 = -Math.PI / 4; D.turnArrow(g, OC.x, OC.y, R, a0, a0 + k * 0.5, "rgba(232,238,246,0.75)"); }
          }
        }
        if (s.phase === "part") {
          // The stores: the stack of plates, and the trolley to carry one to the wall.
          drawStack(g, t); drawTrolley(g, t);
          if (P.onTrolley) drawPlate(g, P, t);
          if (s.carry) { const c = s.carry; drawPlate(g, { x: c.x, y: c.y, a: 0, scale: SMALL }, t); D.ring(g, c.x, c.y, 70 + 3 * Math.sin(t * 5), c.held ? C.amber : "rgba(232,238,246,0.35)", 3); }
        }
        if (s.phase === "weld" || (s.phase === "bolt" && wall.seams)) drawSeams(g, t);
        if (s.phase === "weld") {
          for (const h of s.hiss) for (let j = 0; j < 8; j++) { const ph = (h.t * 1.3 + j / 8) % 1; D.disc(g, h.x + Math.sin(j * 2.1) * 8 + ph * 40, h.y - ph * 50, 4 + ph * 12, `rgba(225,235,245,${0.45 * (1 - ph) * Math.max(0, 1 - h.t / 2.5)})`); }
          drawGauge(g, "heat", s.heat);
          drawTorch(g, s.tip, s.heat, t);
        }
        if (s.phase === "cut" && !wall.cut) { drawGauge(g, "speed", s.line.speed / (CUT_FAST * 1.5)); drawTorch(g, s.tip, 0, t); }
        if (s.phase === "bolt") {
          const nx = nextBolt();
          HOLES.forEach((ho, i) => {
            const [x, y] = holeXY(ho), b = wall.bolts[i];
            if (!b.on) D.orderBadge(g, x, y, 13, KIT.starOrder(8).indexOf(i) + 1, i === nx, t);
            if (s.keys && s.sel === i) { g.setLineDash([7, 5]); D.ring(g, x, y, 30, C.amber, 3); g.setLineDash([]); }
            if (s.wrong && s.wrong.i === i) { g.strokeStyle = C.danger; g.lineWidth = 5; g.beginPath(); g.moveTo(x - 16, y - 16); g.lineTo(x + 16, y + 16); g.moveTo(x + 16, y - 16); g.lineTo(x - 16, y + 16); g.stroke(); }
          });
        }
        for (const p of s.sparks) D.disc(g, p.x, p.y, 2.2, p.col);
      },
    };
  },
});
