/*
 * repairs/chiller.js: the heat exchanger's repair, the chiller between the coolant loop and the radiators
 * (openspec/changes/repair-minigames, design 6h; reactor-cooling 6a opens it from the chiller's FIX).
 *
 * A plate exchanger: a pack of seven plates between a fixed frame and a pressure plate, held by a top and a bottom tie
 * bolt. Two rounds, alternating:
 *   scrub: the fouled plate on the bench, face on, grey scale over its chevrons, a black rubber gasket round its edge
 *     and its two ports. Drag the brush over it (or move it with the arrows and hold Space): the scale under it comes
 *     off. The brush held on the rubber 0.3 s: the fumble "Torn gasket: coolant weeps". 95% off: clean.
 *   restack: the pack's plates must alternate, chevrons up and down: tap a plate hung the wrong way to turn it over.
 *     Then close the pack to its mark: tap a tie bolt's nut to turn it a quarter, which moves its end of the pressure
 *     plate in. The two ends more than two quarters apart: the fumble "Skewed pack: a plate cracks", and that nut backs
 *     off. The nuts do not turn until the plates alternate.
 * A disabled chiller's first step fits a new plate: drag it from the crate into the gap in the pack.
 */
RepairKit.register({
  phases: ["scrub", "restack"],
  id: "chiller",
  title: "Heat exchanger (chiller)",
  place: "Engineering, the chiller",
  group: "Engineering",
  hazard: "Torn gasket: coolant weeps",
  // The how-to card (repair-minigames 6g), drawn by the kit: pictures and a few words, on demand.
  guide: {
    steps: [
      { icon: "sweep", text: "Brush the scale off" },
      { icon: "rubber", text: "Keep off the black rubber" },
      { icon: "alternate", text: "Plates alternate: up, down" },
      { icon: "turn", text: "Nuts in turn, to the mark" },
    ],
    mistake: "Brush on the rubber tears it; one nut far ahead cracks a plate",
    now: (q) => (q.phase === "scrub" ? (q.onGasket ? 1 : 0) : q.phase === "restack" ? (q.alternate ? 3 : 2) : -1),
  },
  down: "The radiators can't take the loop's heat",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const TORN = "Torn gasket: coolant weeps", SKEW = "Skewed pack: a plate cracks";
    // ---- scrub: the plate on the bench (canvas px).
    const PL = { x: 470, y: 102, w: 340, h: 586 };
    const GASKET_IN = 18, GASKET_HIT = 10;             // the gasket's line, inset from the plate's edge; the brush on it within this
    const PORTS = [[PL.x + 78, PL.y + 80], [PL.x + PL.w - 78, PL.y + PL.h - 80]], PORT_R = 34, PORT_GASKET = 48;
    const CELL = 12, BRUSH_R = 36, CLEAN = 0.95, TEAR_S = 0.3;
    // ---- restack: the pack from the side.
    const FRAME_X = 200, BOLT_Y = [168, 572], PLATE_Y = [214, 526];
    const N = 7, Q = 8, Q0 = 6, SKEW_MAX = 2;           // plates; px a quarter turn; quarters to close; the most the ends may differ
    const MARK = 760;                                  // the pressure plate's mark when the pack is closed
    const PW = 40;                                     // a plate's width, side on
    const CRATE = { x: 1020, y: 190, w: 170, h: 400 };
    const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
    let s;

    // ---------------------------------------------------------------- scrub
    /** How far a point is from the gasket's line (its edge run and its two port rings), px. */
    function gasketDist(x, y) {
      const x0 = PL.x + GASKET_IN, x1 = PL.x + PL.w - GASKET_IN, y0 = PL.y + GASKET_IN, y1 = PL.y + PL.h - GASKET_IN;
      const inside = x > x0 && x < x1 && y > y0 && y < y1;
      const edge = inside ? Math.min(x - x0, x1 - x, y - y0, y1 - y) : Math.hypot(Math.max(x0 - x, 0, x - x1), Math.max(y0 - y, 0, y - y1));
      return Math.min(edge, ...PORTS.map(([px, py]) => Math.abs(Math.hypot(x - px, y - py) - PORT_GASKET)));
    }
    function scrubStep(index, r) {
      // Scale: blobs over the plate's field, more of them in later steps, kept clear of the gasket by a brush's reach.
      const cols = Math.floor((PL.w - 2 * 36) / CELL), rows = Math.floor((PL.h - 2 * 36) / CELL);
      const blobs = Array.from({ length: 8 + 2 * index }, () => ({ x: PL.x + 40 + r() * (PL.w - 80), y: PL.y + 40 + r() * (PL.h - 80), r: 56 + r() * 50 }));
      const cells = [];
      for (let j = 0; j < rows; j++) for (let i = 0; i < cols; i++) {
        const x = PL.x + 36 + (i + 0.5) * CELL, y = PL.y + 36 + (j + 0.5) * CELL;
        if (PORTS.some(([px, py]) => Math.hypot(x - px, y - py) < PORT_GASKET + 14)) continue;
        if (gasketDist(x, y) < 18) continue;
        const k = blobs.reduce((m, b) => Math.max(m, 1 - Math.hypot(x - b.x, y - b.y) / b.r), 0);
        if (k > 0) cells.push({ x, y, on: true, shade: 0.55 + 0.45 * Math.min(1, k * 2) });
      }
      s = { phase: "scrub", index, cells, total: cells.length, left: cells.length, cur: { x: PL.x + PL.w / 2, y: PL.y + PL.h / 2 }, keys: false,
        brushing: false, onGasket: false, tearT: 0, tears: [], lock: false, done: false, shine: 0 };
    }
    function scrubAt(x, y) {
      for (const c of s.cells) if (c.on && Math.hypot(c.x - x, c.y - y) <= BRUSH_R) { c.on = false; s.left--; }
    }
    function scrubUpdate(dt, input) {
      if (s.done) { s.shine = Math.min(1, s.shine + dt * 2); return; }
      let pts = [];
      if (input.stick.x || input.stick.y) { s.keys = true; s.cur.x = clamp(s.cur.x + input.stick.x * 420 * dt, PL.x, PL.x + PL.w); s.cur.y = clamp(s.cur.y + input.stick.y * 420 * dt, PL.y, PL.y + PL.h); }
      if (s.keys && input.action) pts = [[s.cur.x, s.cur.y]];
      if (input.down) { s.keys = false; s.cur = { x: input.x, y: input.y }; pts = input.path.length ? input.path : [[input.x, input.y]]; }
      if (!input.down && !input.action) s.lock = false;
      s.brushing = pts.length > 0 && !s.lock;
      if (!s.brushing) { s.onGasket = false; s.tearT = Math.max(0, s.tearT - dt); return; }
      for (const [x, y] of pts) scrubAt(x, y);
      // The brush on the rubber: a moment is a scuff, held there it tears.
      const [lx, ly] = pts[pts.length - 1];
      s.onGasket = gasketDist(lx, ly) < GASKET_HIT;
      s.tearT = s.onGasket ? s.tearT + dt : Math.max(0, s.tearT - dt);
      if (s.tearT >= TEAR_S) { s.tearT = 0; s.lock = true; s.tears.push([lx, ly]); api.fumble(TORN); return; }
      if (s.left <= s.total * (1 - CLEAN)) { for (const c of s.cells) c.on = false; s.left = 0; s.done = true; api.stepDone(); }
    }
    function scrubDraw(g, t) {
      // The plate: steel, its chevron pressing, the gasket, the ports.
      D.round(g, PL.x, PL.y, PL.w, PL.h, 18); g.fillStyle = "#5d6b80"; g.fill(); g.lineWidth = 3; g.strokeStyle = "#8796aa"; g.stroke();
      g.save(); D.round(g, PL.x + 30, PL.y + 30, PL.w - 60, PL.h - 60, 10); g.clip();
      g.strokeStyle = "#71809a"; g.lineWidth = 5;
      for (let y = PL.y - 200; y < PL.y + PL.h + 200; y += 26) { g.beginPath(); g.moveTo(PL.x, y); g.lineTo(PL.x + PL.w / 2, y + 60); g.lineTo(PL.x + PL.w, y); g.stroke(); }
      g.restore();
      // The scale: grey crust over the steel, left where the brush has not been.
      for (const c of s.cells) if (c.on) D.disc(g, c.x, c.y, CELL * 0.78, `rgba(186,180,164,${c.shade.toFixed(2)})`);
      for (const [px, py] of PORTS) { D.disc(g, px, py, PORT_R, "#0b1018"); }
      // The gasket: black rubber, red where it is being scrubbed.
      const hot = s.onGasket && s.brushing;
      g.lineWidth = 12; g.strokeStyle = hot ? C.danger : "#111418";
      D.round(g, PL.x + GASKET_IN, PL.y + GASKET_IN, PL.w - 2 * GASKET_IN, PL.h - 2 * GASKET_IN, 10); g.stroke();
      for (const [px, py] of PORTS) { g.beginPath(); g.arc(px, py, PORT_GASKET, 0, Math.PI * 2); g.stroke(); }
      for (const [x, y] of s.tears) { g.strokeStyle = C.danger; g.lineWidth = 3; g.beginPath(); g.moveTo(x - 8, y - 8); g.lineTo(x + 8, y + 8); g.moveTo(x + 8, y - 8); g.lineTo(x - 8, y + 8); g.stroke(); }
      if (s.shine > 0) { g.fillStyle = `rgba(232,246,255,${(0.25 * (1 - Math.abs(s.shine * 2 - 1))).toFixed(2)})`; D.round(g, PL.x, PL.y, PL.w, PL.h, 18); g.fill(); }
      // How much is left: a bar beside the plate, the one number a player reads.
      const k = s.total ? 1 - s.left / s.total : 1;
      D.round(g, PL.x + PL.w + 60, PL.y + 40, 36, PL.h - 80, 18); g.fillStyle = "#141c28"; g.fill();
      const hh = (PL.h - 88) * Math.min(1, k / CLEAN);
      D.round(g, PL.x + PL.w + 64, PL.y + PL.h - 44 - hh, 28, hh, 14); g.fillStyle = k >= CLEAN ? C.ok : C.accent; g.fill();
      D.text(g, "CLEAN", PL.x + PL.w + 78, PL.y + 18, 18, C.dim, "center", 700);
      // The brush.
      if (s.brushing || s.keys) {
        const { x, y } = s.cur;
        D.ring(g, x, y, BRUSH_R, s.onGasket ? C.danger : "rgba(232,238,246,0.7)", 3);
        g.strokeStyle = "rgba(242,160,70,0.8)"; g.lineWidth = 2;
        for (let k2 = 0; k2 < 10; k2++) { const a = (k2 / 10) * Math.PI * 2 + t * (s.brushing ? 8 : 0); g.beginPath(); g.moveTo(x, y); g.lineTo(x + Math.cos(a) * (BRUSH_R - 6), y + Math.sin(a) * (BRUSH_R - 6)); g.stroke(); }
      }
    }

    // ---------------------------------------------------------------- restack and part
    const alternate = () => s.plates.every((p, i) => i === 0 || p.up !== s.plates[i - 1].up);
    const endX = (k) => MARK + Q * s.q[k];               // the pressure plate's top (0) and bottom (1) faces
    function plateX(i) {
      const right = Math.min(endX(0), endX(1)) - 34;
      return FRAME_X + 70 + (i * (right - FRAME_X - 70 - PW)) / (N - 1);
    }
    const nutXY = (k) => [Math.max(endX(0), endX(1)) + 52, BOLT_Y[k]];
    function restackStep(index, r, isPart) {
      const start = r() < 0.5;
      const plates = Array.from({ length: N }, (_, i) => ({ up: (i % 2 === 0) === start, flip: 0 }));
      s = { phase: isPart ? "part" : "restack", index, plates, q: [isPart ? 0 : Q0, isPart ? 0 : Q0], focus: 0, keys: false, kick: 0, done: false, turn: [0, 0] };
      if (isPart) {
        s.gap = 1 + Math.floor(r() * (N - 2));
        s.newPlate = { x: CRATE.x + CRATE.w / 2, y: CRATE.y + CRATE.h / 2, held: false };
        return;
      }
      // One or two plates hung the wrong way.
      const wrong = Math.min(2, 1 + Math.floor(index / 2));
      const picked = new Set();
      while (picked.size < wrong) picked.add(Math.floor(r() * N));
      for (const i of picked) plates[i].up = !plates[i].up;
      if (alternate()) plates[0].up = !plates[0].up;
    }
    function tapPlate(i) { s.plates[i].up = !s.plates[i].up; s.plates[i].flip = 1; }
    function tapNut(k) {
      if (!alternate()) { s.kick = 1; api.say("Plates first: up, down, up"); return; }
      if (s.q[k] <= 0) return;
      s.q[k]--; s.turn[k] += Math.PI / 2;
      if (Math.abs(s.q[0] - s.q[1]) > SKEW_MAX) { s.q[k]++; s.kick = 1; api.fumble(SKEW); return; }
      if (s.q[0] === 0 && s.q[1] === 0) { s.done = true; api.stepDone(); }
    }
    function restackUpdate(dt, input) {
      s.kick = Math.max(0, s.kick - dt * 3);
      for (const p of s.plates) p.flip = Math.max(0, p.flip - dt * 4);
      if (s.done) return;
      const items = N + 2;
      for (const [code, d] of [["ArrowLeft", -1], ["KeyA", -1], ["ArrowRight", 1], ["KeyD", 1], ["Tab", 1]]) if (input.hit.has(code)) { s.keys = true; s.focus = (s.focus + d + items) % items; }
      if (input.actionPressed) { s.keys = true; if (s.focus < N) tapPlate(s.focus); else tapNut(s.focus - N); return; }
      if (!input.pressed) return;
      const nut = KIT.nearest([nutXY(0), nutXY(1)], input.x, input.y, KIT.TOUCH_R + 14);
      if (nut >= 0) { s.keys = false; s.focus = N + nut; tapNut(nut); return; }
      for (let i = 0; i < N; i++) {
        const x = plateX(i);
        if (input.x >= x - 10 && input.x <= x + PW + 10 && input.y >= PLATE_Y[0] - 10 && input.y <= PLATE_Y[1] + 10) { s.keys = false; s.focus = i; tapPlate(i); return; }
      }
    }
    function partUpdate(dt, input) {
      const m = s.newPlate;
      if (s.done) return;
      if (input.stick.x || input.stick.y) { s.keys = true; m.x += input.stick.x * 480 * dt; m.y += input.stick.y * 480 * dt; }
      if (input.pressed && Math.abs(input.x - m.x) < PW && Math.abs(input.y - m.y) < 120) { m.held = true; m.ox = input.x - m.x; m.oy = input.y - m.y; }
      if (m.held && input.down) { m.x = input.x - m.ox; m.y = input.y - m.oy; }
      if (!((m.held && input.released) || (s.keys && input.actionPressed))) return;
      m.held = false;
      const gx = plateX(s.gap) + PW / 2, gy = (PLATE_Y[0] + PLATE_Y[1]) / 2;
      if (Math.abs(m.x - gx) < 50 && Math.abs(m.y - gy) < 140) { s.gap = -1; s.done = true; api.stepDone(); return; }
      m.x = CRATE.x + CRATE.w / 2; m.y = CRATE.y + CRATE.h / 2;
    }
    /** A plate side on, with its chevron: up (a peak) or down (a valley), and its colour so the alternation reads. */
    function plateGlyph(g, x, up, sel, flip) {
      const y0 = PLATE_Y[0], y1 = PLATE_Y[1], cx = x + PW / 2, sq = 1 - 0.8 * flip;
      D.round(g, x + (PW * (1 - sq)) / 2, y0, PW * sq, y1 - y0, 6); g.fillStyle = up ? "#5d6b80" : "#4a586d"; g.fill();
      g.lineWidth = sel ? 4 : 2; g.strokeStyle = sel ? C.amber : "#8796aa"; g.stroke();
      g.strokeStyle = up ? C.accent : C.amber; g.lineWidth = 5; g.lineCap = "round"; g.lineJoin = "round";
      for (const yy of [y0 + 70, (y0 + y1) / 2, y1 - 70]) { g.beginPath(); g.moveTo(cx - 12, yy + (up ? 10 : -10)); g.lineTo(cx, yy + (up ? -10 : 10)); g.lineTo(cx + 12, yy + (up ? 10 : -10)); g.stroke(); }
      g.lineCap = "butt"; g.lineJoin = "miter";
    }
    function packDraw(g, t) {
      const shake = s.kick > 0 ? Math.sin(t * 60) * 5 * s.kick : 0;
      g.save(); g.translate(shake, 0);
      // The fixed frame on the left, the tie bolts along the top and bottom.
      D.round(g, FRAME_X - 40, 120, 70, 500, 10); g.fillStyle = "#2a3446"; g.fill(); g.lineWidth = 3; g.strokeStyle = "#4a586d"; g.stroke();
      const boltEnd = Math.max(endX(0), endX(1)) + 110;
      for (const y of BOLT_Y) { g.fillStyle = "#9aa6b6"; g.fillRect(FRAME_X, y - 6, boltEnd - FRAME_X, 12); }
      // The ports' pipes into the frame: hot in, cold out.
      D.pipe(g, [[[40, 260], [FRAME_X - 40, 260]]], { w: 30, fluid: "#ff6a4d" });
      D.pipe(g, [[[40, 480], [FRAME_X - 40, 480]]], { w: 30, fluid: "#4fa8f7" });
      // The mark the pressure plate closes to.
      g.setLineDash([10, 8]); g.strokeStyle = C.ok; g.lineWidth = 3;
      g.beginPath(); g.moveTo(MARK, 110); g.lineTo(MARK, 630); g.stroke(); g.setLineDash([]);
      // The plates.
      const alt = alternate();
      s.plates.forEach((p, i) => { if (s.phase === "part" && i === s.gap) return; plateGlyph(g, plateX(i), p.up, s.keys && s.focus === i, p.flip); });
      if (s.phase === "part" && s.gap >= 0) { g.setLineDash([8, 6]); D.round(g, plateX(s.gap), PLATE_Y[0], PW, PLATE_Y[1] - PLATE_Y[0], 6); g.lineWidth = 3; g.strokeStyle = C.amber; g.stroke(); g.setLineDash([]); }
      // The pressure plate: its top and bottom ends where their nuts have brought them.
      const xT = endX(0), xB = endX(1);
      g.beginPath(); g.moveTo(xT, 130); g.lineTo(xT + 40, 130); g.lineTo(xB + 40, 610); g.lineTo(xB, 610); g.closePath();
      g.fillStyle = "#33405a"; g.fill(); g.lineWidth = 3; g.strokeStyle = Math.abs(s.q[0] - s.q[1]) === SKEW_MAX ? C.warn : "#5a6a82"; g.stroke();
      // The nuts: a hexagon each, locked (dim, with a bar) until the plates alternate.
      for (let k = 0; k < 2; k++) {
        const [nx, ny] = nutXY(k), sel = s.keys && s.focus === N + k, on = alt && s.phase === "restack";
        g.save(); g.translate(nx, ny); g.rotate(s.turn[k]);
        g.beginPath(); for (let j = 0; j < 6; j++) { const a = (j / 6) * Math.PI * 2; j ? g.lineTo(Math.cos(a) * 30, Math.sin(a) * 30) : g.moveTo(Math.cos(a) * 30, Math.sin(a) * 30); } g.closePath();
        g.fillStyle = on ? "#b7c0cc" : "#4a5566"; g.fill(); g.lineWidth = sel ? 4 : 2; g.strokeStyle = sel ? C.amber : "#2a313c"; g.stroke();
        D.disc(g, 0, 0, 10, "#2a313c");
        g.restore();
        if (on && s.q[k] > 0) D.turnArrow(g, nx, ny, 44, -2.2, -0.9, "rgba(232,238,246,0.6)");
        // How many quarters this end still has to go: a short row of ticks under the nut.
        for (let j = 0; j < s.q[k]; j++) D.disc(g, nx - 30 + j * 12, ny + (k ? 46 : -46), 4, on ? C.fg : "#4a5566");
      }
      g.restore();
      if (s.phase === "part") {
        D.panel(g, CRATE.x, CRATE.y, CRATE.w, CRATE.h, 12, "#141c28", "#3a4658");
        const m = s.newPlate;
        if (s.gap >= 0) {
          g.save(); g.translate(m.x - PW / 2, m.y - (PLATE_Y[1] - PLATE_Y[0]) / 2 - PLATE_Y[0]);
          plateGlyph(g, 0, true, m.held, 0);
          g.restore();
        }
      }
    }

    return {
      get state() { return s; },
      step(index, isPart, phase) {
        const r = api.rand();
        // A round scrubs the plate, then restacks the pack (repair-minigames 1a); the part opens the first round.
        if (isPart || phase === "restack") restackStep(index, r, isPart); else scrubStep(index, r);
      },
      /** For tools (shots and tests): the round's state, read only. */
      peek() {
        if (s.phase === "scrub") return { phase: s.phase, done: s.done, left: s.left, total: s.total, onGasket: s.onGasket,
          cells: s.cells.filter((c) => c.on).map((c) => [c.x, c.y]), plate: PL, gasketIn: GASKET_IN, ports: PORTS, portGasket: PORT_GASKET, brush: BRUSH_R };
        const base = { phase: s.phase, done: s.done, alternate: alternate(), ups: s.plates.map((p) => p.up), q: s.q.slice(),
          plates: s.plates.map((_, i) => [plateX(i) + PW / 2, (PLATE_Y[0] + PLATE_Y[1]) / 2]), nuts: [nutXY(0), nutXY(1)] };
        if (s.phase === "part") return { ...base, gap: s.gap, newPlate: { x: s.newPlate.x, y: s.newPlate.y } };
        return base;
      },
      update(dt, input) {
        if (s.phase === "scrub") scrubUpdate(dt, input);
        else if (s.phase === "part") partUpdate(dt, input);
        else restackUpdate(dt, input);
      },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        if (s.phase === "scrub") scrubDraw(g, t); else packDraw(g, t);
      },
    };
  },
});
