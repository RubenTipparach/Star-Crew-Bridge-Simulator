/*
 * repairs/fighter.js: a Swift's avionics repair on its hangar cradle (openspec/changes/repair-minigames, design 2; the
 * Swift is shuttle-bay-and-fighters section 7).
 *
 * The Swift sits on its cradle at the left with its dorsal avionics bay marked; the bay fills the rest of the screen.
 * Lift the access panel off (drag it away), plug each avionics lead into the socket of the same shape and stripe, then
 * torque the panel's fasteners back in a star order: each one across from the last, working round. Every fastener
 * shows its number in the order and the next one is lit, so the order is known before the first is picked (the
 * owner: "needs some way to tell me what the next screw is"). A plug forced into the wrong socket sparks (5 HP) and
 * comes back bent; a fastener out of order is refused with a red cross. Later steps have more leads, fewer shapes to tell them
 * apart by (the stripe decides), and more fasteners. A disabled fighter's first step fits the new avionics unit: drag
 * it from the crate into the empty rack slot. Keys: arrows choose, Space lifts, picks up, places and turns.
 */
RepairKit.register({
  id: "fighter",
  title: "Fighters",
  place: "Hangar, a Swift on its cradle",
  group: "Hangar",
  hazard: "Spark: 5 HP, a bent plug",
  down: "The fighter cannot launch",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const BAY = { x: 520, y: 140, w: 560, h: 290 };     // the avionics bay's opening, close up
    const PLATE = { x: 490, y: 112, w: 620, h: 346 };   // the access panel over it
    const FRAME = { x: 420, y: 92, w: 836, h: 608 };    // the close-up's frame
    const REST = 610;                                    // how far aside the lifted panel sits, px
    const GROMMET = { x: 800, y: 418 };                  // where the leads come up out of the bay floor
    const SLOT = { x: 800, y: 290 };                     // the part step's empty rack slot
    const CRATE = { x: 1170, y: 590 };
    const SWIFT = { x: 214, y: 380 };                    // the fighter's picture, nose up
    const SHAPES = ["circle", "square", "triangle", "diamond"];
    const STRIPES = [C.accent, C.amber, C.lilac];       // stripe i is drawn as i + 1 bars, so it reads without colour
    let phase, panel, plugs, sockets, fast, order, wrong, unit, kf, fi, carry, held, spark;
    const near = (ax, ay, bx, by, r) => Math.hypot(ax - bx, ay - by) < r;
    const wrapI = (i, n) => ((i % n) + n) % n;
    const shuffle = (a, r) => { for (let i = a.length - 1; i > 0; i--) { const j = Math.floor(r() * (i + 1)); [a[i], a[j]] = [a[j], a[i]]; } return a; };

    /** The fastener seats round the panel's rim, in ring order (clockwise from the top left). */
    function ringPts(n) {
      const i = 22, L = PLATE.x + i, R = PLATE.x + PLATE.w - i, T = PLATE.y + i, B = PLATE.y + PLATE.h - i;
      const M = PLATE.x + PLATE.w / 2, V = PLATE.y + PLATE.h / 2;
      return n === 6 ? [[L, T], [M, T], [R, T], [R, B], [M, B], [L, B]]
        : [[L, T], [M, T], [R, T], [R, V], [R, B], [M, B], [L, B], [L, V]];
    }
    /** The fastener that comes next: the kit's star order from the first (one order, shown at rest, KIT.starOrder). */
    function nextInStar(done, n) {
      const S = KIT.starOrder(n);
      return done.length < n ? [S[done.length]] : [];
    }
    /** Fastener i's number in the order, from 1. */
    const orderNo = (i, n) => KIT.starOrder(n).indexOf(i) + 1;
    function shape(g, kind, x, y, r) {
      g.beginPath();
      if (kind === "circle") g.arc(x, y, r, 0, Math.PI * 2);
      else if (kind === "square") g.rect(x - r * 0.85, y - r * 0.85, r * 1.7, r * 1.7);
      else if (kind === "triangle") { g.moveTo(x, y - r); g.lineTo(x + r, y + r * 0.8); g.lineTo(x - r, y + r * 0.8); g.closePath(); }
      else { g.moveTo(x, y - r * 1.1); g.lineTo(x + r, y); g.lineTo(x, y + r * 1.1); g.lineTo(x - r, y); g.closePath(); }
    }
    function hex(g, x, y, r) {
      g.beginPath();
      for (let k = 0; k < 6; k++) { const a = Math.PI / 6 + k * Math.PI / 3; g[k ? "lineTo" : "moveTo"](x + r * Math.cos(a), y + r * Math.sin(a)); }
      g.closePath();
    }

    function lift() { phase = "plugs"; panel.tx = REST; panel.ty = 0; fi = 0; }
    function place(p, s) {
      if (p.shape === s.shape && p.stripe === s.stripe) {
        p.seat = s; s.plug = p;
        if (plugs.every((q) => q.seat)) { phase = "close"; panel.tx = 0; panel.ty = 0; }
        return;
      }
      p.bent = true; spark = { x: s.x, y: s.y, t: 0.5 };
      api.fumble("Spark: 5 HP, a bent plug");
    }
    function turn(i) {
      if (fast[i].on) return;
      if (nextInStar(order, fast.length).includes(i)) {
        fast[i].on = true; order.push(i);
        if (order.length === fast.length) { phase = "done"; api.stepDone(); }
      } else { wrong = { i, t: 0.7 }; api.say("Out of order"); }
    }
    function seatUnit() { unit.set = true; unit.held = false; unit.x = SLOT.x; unit.y = SLOT.y; phase = "done"; api.stepDone(); }

    return {
      step(index, isPart) {
        const r = api.rand();
        kf = false; fi = 0; carry = null; held = null; wrong = null; spark = null;
        panel = { dx: 0, dy: 0, tx: 0, ty: 0, held: false, gx: 0, gy: 0 };
        plugs = []; sockets = []; order = [];
        fast = ringPts(index === 0 ? 6 : 8).map(([x, y]) => ({ x, y, on: false, t: 0 }));
        if (isPart) {
          phase = "part"; panel.dx = panel.tx = REST;
          unit = { x: CRATE.x, y: CRATE.y, held: false, set: false, gx: 0, gy: 0 };
          return;
        }
        phase = "panel";
        // More leads each step, told apart by fewer shapes, so the stripe has to be read.
        const n = Math.min(5, 3 + index), kinds = Math.max(2, 4 - index);
        const shapes = shuffle(SHAPES.slice(), r).slice(0, kinds);
        let combos;
        if (n <= kinds) combos = shapes.slice(0, n).map((s) => [s, Math.floor(r() * 3)]);
        else {
          combos = [];
          for (const s of shapes) for (let c = 0; c < 3; c++) combos.push([s, c]);
          combos = shuffle(combos, r).slice(0, n);
        }
        sockets = combos.map(([s, c], i) => ({ shape: s, stripe: c, x: BAY.x + (i + 0.5) * BAY.w / n, y: 250, plug: null }));
        const homes = shuffle(sockets.map((_, i) => i), r);
        plugs = sockets.map((s, i) => ({
          shape: s.shape, stripe: s.stripe, hx: 590 + homes[i] * (420 / (n - 1)), hy: 590,
          x: GROMMET.x, y: GROMMET.y + 40, held: false, seat: null, bent: false, gx: 0, gy: 0,
        }));
      },
      update(dt, input) {
        const ease = Math.min(1, dt * 12), h = input.hit;
        if (wrong && (wrong.t -= dt) <= 0) wrong = null;
        if (spark && (spark.t -= dt) <= 0) spark = null;
        const dir = (h.has("ArrowRight") || h.has("KeyD") || h.has("ArrowDown") || h.has("KeyS") ? 1 : 0)
          - (h.has("ArrowLeft") || h.has("KeyA") || h.has("ArrowUp") || h.has("KeyW") ? 1 : 0);
        if (dir || input.actionPressed) kf = true;
        if (input.pressed) { kf = false; if (carry) { carry.held = false; carry = null; } }
        if (!panel.held) { panel.dx += (panel.tx - panel.dx) * ease; panel.dy += (panel.ty - panel.dy) * ease; }
        if (phase !== "panel") for (const p of plugs) if (!p.held) {
          const tx = p.seat ? p.seat.x : p.hx, ty = p.seat ? p.seat.y + 14 : p.hy;
          p.x += (tx - p.x) * ease; p.y += (ty - p.y) * ease;
        }
        for (const f of fast) if (f.on) f.t = Math.min(1, f.t + dt / 0.3);

        if (phase === "part") {
          if (input.pressed && Math.abs(input.x - unit.x) < 52 && Math.abs(input.y - unit.y) < 92) { unit.held = true; unit.gx = input.x - unit.x; unit.gy = input.y - unit.y; }
          if (unit.held && input.down) { unit.x = input.x - unit.gx; unit.y = input.y - unit.gy; }
          if (unit.held && input.released) { unit.held = false; if (near(unit.x, unit.y, SLOT.x, SLOT.y, 50)) return seatUnit(); }
          if (!unit.held) { unit.x += (CRATE.x - unit.x) * ease; unit.y += (CRATE.y - unit.y) * ease; }
          if (kf && input.actionPressed) seatUnit();
          return;
        }
        if (phase === "panel") {
          const x = input.x - PLATE.x, y = input.y - PLATE.y;
          if (input.pressed && x > 0 && x < PLATE.w && y > 0 && y < PLATE.h) { panel.held = true; panel.gx = input.x; panel.gy = input.y; }
          if (panel.held && input.down) { panel.dx = input.x - panel.gx; panel.dy = input.y - panel.gy; }
          if (panel.held && input.released) { panel.held = false; if (Math.hypot(panel.dx, panel.dy) > 110) lift(); }
          if (kf && input.actionPressed) lift();
          return;
        }
        if (phase === "plugs") {
          const free = plugs.filter((p) => !p.seat), open = sockets.filter((s) => !s.plug);
          if (kf && !carry) {
            fi = wrapI(fi + dir, free.length);
            if (input.actionPressed) { carry = free[fi]; carry.held = true; fi = 0; }
          } else if (kf && carry) {
            fi = wrapI(fi + dir, open.length);
            const s = open[fi];
            carry.x += (s.x - carry.x) * ease; carry.y += (s.y + 84 - carry.y) * ease;
            if (input.actionPressed) { const p = carry; carry = null; p.held = false; fi = 0; place(p, s); }
          } else {
            if (input.pressed) {
              const p = free.slice().reverse().find((q) => Math.abs(input.x - q.x) < 32 && Math.abs(input.y - q.y) < 40);
              if (p) { held = p; p.held = true; p.gx = input.x - p.x; p.gy = input.y - p.y; }
            }
            if (held && input.down) { held.x = input.x - held.gx; held.y = input.y - held.gy; }
            if (held && input.released) {
              const p = held; held = null; p.held = false;
              const s = open.find((q) => near(p.x, p.y - 14, q.x, q.y, 46));
              if (s) place(p, s);
            }
          }
          return;
        }
        if (phase === "close") { if (Math.abs(panel.dx) < 2) { phase = "torque"; panel.dx = 0; panel.dy = 0; fi = 0; } return; }
        if (phase === "torque") {
          if (kf) { fi = wrapI(fi + dir, fast.length); if (input.actionPressed) turn(fi); }
          else if (input.pressed) { const i = KIT.nearest(fast, input.x, input.y, KIT.TOUCH_R + 10); if (i >= 0) turn(i); }
        }
      },
      /** For tools: the round's state, read only, so a script can play it. */
      peek() { return { phase, sockets, plugs, fast, PLATE, SLOT, unit }; },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        drawHangar(g);
        // The close up: the hull skin round the bay.
        g.save();
        D.round(g, FRAME.x, FRAME.y, FRAME.w, FRAME.h, 18); g.fillStyle = "#151d2a"; g.fill(); g.clip();
        g.strokeStyle = "#0d131c"; g.lineWidth = 3;
        for (const x of [462, 1150]) { g.beginPath(); g.moveTo(x, FRAME.y); g.lineTo(x, FRAME.y + FRAME.h); g.stroke(); }
        g.beginPath(); g.moveTo(FRAME.x, 492); g.lineTo(FRAME.x + FRAME.w, 492); g.stroke();
        g.fillStyle = "#2a3446";
        for (let x = 440; x < 1256; x += 28) { D.disc(g, x, 482, 2.5, "#2a3446"); D.disc(g, x, 502, 2.5, "#2a3446"); }
        drawBay(g);
        if (phase === "part") drawRack(g);
        else if (phase !== "panel") {
          for (const p of plugs) if (!p.held) cable(g, p);
          for (const p of plugs) if (!p.held) drawPlug(g, p);
        }
        drawPlate(g, t);
        if (phase === "part") drawCrate(g);
        for (const p of plugs) if (p.held) { cable(g, p); drawPlug(g, p); }
        // Keys: the chosen item, dashed (only once a key has been used).
        if (kf && phase !== "done") {
          let fx = null, fy = 0, fr = 40;
          if (phase === "panel") { fx = PLATE.x + PLATE.w / 2; fy = PLATE.y + PLATE.h / 2; fr = 70; }
          else if (phase === "plugs" && !carry) { const p = plugs.filter((q) => !q.seat)[fi]; if (p) { fx = p.x; fy = p.y; fr = 48; } }
          else if (phase === "plugs" && carry) { const s = sockets.filter((q) => !q.plug)[fi]; if (s) { fx = s.x; fy = s.y; fr = 32; } }
          else if (phase === "torque") { fx = fast[fi].x; fy = fast[fi].y; fr = 28; }
          else if (phase === "part") { fx = SLOT.x; fy = SLOT.y; fr = 70; }
          if (fx !== null) { g.setLineDash([8, 6]); D.ring(g, fx, fy, fr, C.amber, 3); g.setLineDash([]); }
        }
        if (spark) {
          const k = spark.t / 0.5;
          g.strokeStyle = C.warn; g.lineWidth = 3;
          for (let i = 0; i < 9; i++) {
            const a = i * 0.7 + t * 9, r0 = 10, r1 = 20 + 40 * (1 - k);
            g.beginPath(); g.moveTo(spark.x + r0 * Math.cos(a), spark.y + r0 * Math.sin(a)); g.lineTo(spark.x + r1 * Math.cos(a), spark.y + r1 * Math.sin(a)); g.stroke();
          }
        }
        g.restore();
        D.round(g, FRAME.x, FRAME.y, FRAME.w, FRAME.h, 18); g.lineWidth = 2; g.strokeStyle = C.line; g.stroke();
      },
    };

    /** The hangar, from above: the Swift on its cradle, its avionics bay marked and called out to the close up. */
    function drawHangar(g) {
      D.panel(g, 24, 92, 380, 608, 18, "#0a0f17");
      g.strokeStyle = "#111925"; g.lineWidth = 2;
      for (let y = 132; y < 700; y += 60) { g.beginPath(); g.moveTo(26, y); g.lineTo(402, y); g.stroke(); }
      for (let x = 64; x < 404; x += 60) { g.beginPath(); g.moveTo(x, 94); g.lineTo(x, 698); g.stroke(); }
      const poly = (pts, fill, stroke) => {
        g.beginPath(); pts.forEach(([px, py], i) => (i ? g.lineTo(px, py) : g.moveTo(px, py))); g.closePath();
        g.fillStyle = fill; g.fill(); if (stroke) { g.strokeStyle = stroke; g.lineWidth = 2; g.stroke(); }
      };
      g.save(); g.translate(SWIFT.x, SWIFT.y);
      // The cradle: a centre rail and two arms with their pads.
      g.fillStyle = "#1e2533"; g.fillRect(-8, -236, 16, 480);
      for (const [ay, aw] of [[-104, 84], [178, 96]]) {
        D.round(g, -aw, ay - 9, aw * 2, 18, 6); g.fillStyle = "#3a4456"; g.fill();
        for (const s of [-1, 1]) { g.fillStyle = C.amber; g.fillRect(s * aw - (s > 0 ? 14 : 0), ay - 13, 14, 26); }
      }
      for (const s of [-1, 1]) {
        poly([[s * 30, -40], [s * 150, 112], [s * 150, 142], [s * 34, 150]], "#222b3a", C.steel);
        poly([[s * 22, -150], [s * 62, -118], [s * 62, -106], [s * 26, -110]], "#222b3a", C.steel);
        g.fillStyle = C.amber; g.fillRect(s > 0 ? 136 : -150, 116, 14, 22);
      }
      poly([[0, -250], [14, -214], [24, -150], [30, -60], [34, 60], [36, 150], [30, 200], [-30, 200], [-36, 150], [-34, 60],
        [-30, -60], [-24, -150], [-14, -214]], "#2a3446", C.steel);
      g.fillStyle = "#121821"; g.fillRect(-26, 200, 20, 18); g.fillRect(6, 200, 20, 18);
      g.strokeStyle = "#3a4558"; g.lineWidth = 2; g.beginPath(); g.moveTo(0, -105); g.lineTo(0, 190); g.stroke();
      g.beginPath(); g.ellipse(0, -150, 11, 36, 0, 0, Math.PI * 2); g.fillStyle = "#1d3b52"; g.fill(); g.strokeStyle = C.accent; g.stroke();
      g.fillStyle = "rgba(242,160,70,0.25)"; g.fillRect(-18, -70, 36, 80);
      g.strokeStyle = C.amber; g.lineWidth = 3; g.strokeRect(-18, -70, 36, 80);
      g.restore();
      D.text(g, "SWIFT 1", SWIFT.x, 668, 18, C.dim, "center", 700);
      // The call out from the bay to the close up.
      g.strokeStyle = "rgba(242,160,70,0.3)"; g.lineWidth = 2;
      g.beginPath(); g.moveTo(SWIFT.x + 18, SWIFT.y - 70); g.lineTo(FRAME.x, FRAME.y + 20);
      g.moveTo(SWIFT.x + 18, SWIFT.y + 10); g.lineTo(FRAME.x, FRAME.y + FRAME.h - 20); g.stroke();
    }
    /** The bay's recess, its rails and (on a normal step) the avionics units with their sockets. */
    function drawBay(g) {
      g.fillStyle = "#05080d"; g.fillRect(BAY.x, BAY.y, BAY.w, BAY.h);
      g.strokeStyle = "#000"; g.lineWidth = 8; g.strokeRect(BAY.x + 4, BAY.y + 4, BAY.w - 8, BAY.h - 8);
      g.strokeStyle = "#2a3446"; g.lineWidth = 2; g.strokeRect(BAY.x, BAY.y, BAY.w, BAY.h);
      g.fillStyle = "#1a212d"; g.fillRect(BAY.x + 8, BAY.y + 18, BAY.w - 16, 8); g.fillRect(BAY.x + 8, BAY.y + BAY.h - 26, BAY.w - 16, 8);
      if (phase !== "part") {
        const bw = BAY.w / sockets.length - 18;
        for (const s of sockets) {
          D.round(g, s.x - bw / 2, 166, bw, 236, 6); g.fillStyle = "#1b2433"; g.fill(); g.strokeStyle = "#2c3a52"; g.lineWidth = 2; g.stroke();
          g.fillStyle = "#121821"; for (let k = 0; k < 4; k++) g.fillRect(s.x - bw / 2 + 12, 366 + k * 8, bw - 24, 3);
          shape(g, s.shape, s.x, s.y, 20); g.fillStyle = "#020306"; g.fill(); g.strokeStyle = "#8a96a8"; g.lineWidth = 2; g.stroke();
          for (let k = 0; k <= s.stripe; k++) { g.fillStyle = STRIPES[s.stripe]; g.fillRect(s.x - 22, 320 + k * 9, 44, 5); }
        }
      }
      D.disc(g, GROMMET.x, GROMMET.y, 14, "#0e1219"); D.ring(g, GROMMET.x, GROMMET.y, 14, "#3a4456", 3);
    }
    function cable(g, p) {
      const ex = p.x, ey = p.y + 38;
      g.beginPath(); g.moveTo(GROMMET.x, GROMMET.y);
      g.quadraticCurveTo((GROMMET.x + ex) / 2, Math.max(GROMMET.y, ey) + (p.seat ? 24 : 50), ex, ey);
      g.strokeStyle = "#0e1219"; g.lineWidth = 11; g.stroke(); g.strokeStyle = "#3a4456"; g.lineWidth = 5; g.stroke();
    }
    function drawPlug(g, p) {
      const { x, y } = p;
      D.round(g, x - 30, y - 38, 60, 76, 10); g.fillStyle = "#2a3446"; g.fill(); g.lineWidth = 2; g.strokeStyle = C.steel; g.stroke();
      shape(g, p.shape, x, y - 14, 15); g.fillStyle = "#c9d3df"; g.fill();
      for (let k = 0; k <= p.stripe; k++) { g.fillStyle = STRIPES[p.stripe]; g.fillRect(x - 22, y + 10 + k * 8, 44, 5); }
      if (p.bent) {
        g.strokeStyle = C.danger; g.lineWidth = 3; g.beginPath();
        g.moveTo(x - 10, y - 38); g.lineTo(x - 16, y - 48); g.lineTo(x - 6, y - 56);
        g.moveTo(x + 8, y - 38); g.lineTo(x + 16, y - 46); g.stroke();
      }
      if (p.seat) { g.fillStyle = C.ok; g.fillRect(x - 14, y - 46, 28, 7); }
    }
    /** The access panel, wherever it is: seated, held, or set aside at the frame's edge. */
    function drawPlate(g, t) {
      const x = PLATE.x + panel.dx, y = PLATE.y + panel.dy, ox = panel.dx, oy = panel.dy;
      D.round(g, x + 8, y + 10, PLATE.w, PLATE.h, 14); g.fillStyle = "rgba(0,0,0,0.35)"; g.fill();
      D.round(g, x, y, PLATE.w, PLATE.h, 14); g.fillStyle = "#2b3546"; g.fill(); g.lineWidth = 3; g.strokeStyle = C.steel; g.stroke();
      g.strokeStyle = "#3a4558"; g.lineWidth = 2; g.strokeRect(x + 44, y + 44, PLATE.w - 88, PLATE.h - 88);
      D.round(g, x + PLATE.w / 2 - 70, y + PLATE.h / 2 - 14, 140, 28, 14); g.fillStyle = "#1a212d"; g.fill(); g.strokeStyle = "#4a5568"; g.stroke();
      D.text(g, "AV 2", x + 70, y + 70, 22, "#4a5568", "left", 700);
      const loose = phase === "torque" || phase === "done" || phase === "close";
      fast.forEach((f, i) => {
        const fx = f.x + ox, fy = f.y + oy;
        if (!loose) { D.disc(g, fx, fy, 10, "#0b0f15"); D.ring(g, fx, fy, 10, "#4a5568", 2); return; }
        hex(g, fx, fy, 14);
        if (f.on) {
          g.fillStyle = "#c9d3df"; g.fill();
          g.beginPath(); g.arc(fx, fy, 21, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * f.t); g.strokeStyle = C.ok; g.lineWidth = 4; g.stroke();
        } else { g.fillStyle = "#0b0f15"; g.fill(); g.strokeStyle = "#c9d3df"; g.lineWidth = 2; g.stroke(); }
        g.strokeStyle = f.on ? "#2a3446" : "#c9d3df"; g.lineWidth = 3;
        g.beginPath(); g.moveTo(fx - 6, fy); g.lineTo(fx + 6, fy); g.stroke();
        // Every fastener's number in the star, the next one lit (owner: tell me the next screw before I pick one).
        if (phase === "torque" && !f.on) D.orderBadge(g, fx, fy, 14, orderNo(i, fast.length), nextInStar(order, fast.length)[0] === i, t);
        if (wrong && wrong.i === i) {
          g.strokeStyle = C.danger; g.lineWidth = 5; g.beginPath();
          g.moveTo(fx - 18, fy - 18); g.lineTo(fx + 18, fy + 18); g.moveTo(fx + 18, fy - 18); g.lineTo(fx - 18, fy + 18); g.stroke();
        }
      });
    }
    /** The part step: the rack with one unit missing. */
    function drawRack(g) {
      for (let i = 0; i < 5; i++) {
        const cx = BAY.x + (i + 0.5) * BAY.w / 5;
        if (i === 2) {
          if (unit.set) continue;
          g.setLineDash([10, 7]); g.strokeStyle = C.amber; g.lineWidth = 3; g.strokeRect(cx - 46, 200, 92, 180); g.setLineDash([]);
          continue;
        }
        D.round(g, cx - 46, 200, 92, 180, 6); g.fillStyle = "#1b2433"; g.fill(); g.strokeStyle = "#2c3a52"; g.lineWidth = 2; g.stroke();
        g.fillStyle = "#121821"; for (let k = 0; k < 4; k++) g.fillRect(cx - 34, 340 + k * 8, 68, 3);
        D.disc(g, cx, 230, 5, "#2c3a52");
      }
    }
    function drawCrate(g) {
      D.panel(g, CRATE.x - 62, CRATE.y - 104, 124, 208, 12, "#141b27");
      const x = unit.x, y = unit.y;
      D.round(g, x - 46, y - 90, 92, 180, 6); g.fillStyle = "#24314a"; g.fill(); g.strokeStyle = C.accent; g.lineWidth = 2; g.stroke();
      g.fillStyle = C.copper; g.fillRect(x - 30, y - 82, 60, 8);
      g.fillStyle = "#121821"; for (let k = 0; k < 4; k++) g.fillRect(x - 34, y + 50 + k * 8, 68, 3);
      D.disc(g, x, y - 60, 5, unit.set ? C.ok : "#2c3a52");
    }
  },
});
