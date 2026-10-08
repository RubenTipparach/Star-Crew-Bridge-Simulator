/*
 * repairs/scrubbers.js: the life support scrubbers' repair (openspec/changes/repair-minigames, design 2).
 *
 * The scrubber cabinet holds one CO2 cartridge. Drag the spent one (dark, hatched) out of its slot into the bin, and a
 * fresh one from the rack into the slot; the blower spins up. Then trim the three gas valves (turn a wheel round its
 * hub, the wheel over it, or the arrows) until each gauge's needle, O2, N2 and CO2, sits in its green band and holds
 * there a moment. A step is one cartridge and one bank trimmed; later steps narrow the bands, the mix wanders more and
 * each valve pulls on the next gauge. A cartridge let go short of the slot or the bin, or a needle left in the red: a
 * fumble, a hiss, the room's CO2 rises. Space moves the next cartridge. A disabled scrubber's first step fits the new
 * blower fan: drag it from the crate into its housing.
 */
RepairKit.register({
  id: "scrubbers",
  title: "Life support scrubbers",
  place: "Life support room",
  group: "Life",
  hazard: "Hiss: the room's CO2 rises",
  down: "The air goes bad (life-support)",
  panel: { x: 90, y: 100, w: 420, h: 589, screws: 4 },   // the cover, screwed on (kit: access panels)
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const HISS = "Hiss: the room's CO2 rises";
    const CAB = { x: 90, y: 100, w: 420, h: 590 };       // the scrubber cabinet
    const SLOT = { x: 225, y: 270, w: 150, h: 320 };      // its cartridge slot
    const RACK = { x: 545, y: 110, w: 150, h: 360 };      // fresh cartridges wait here
    const BIN = { x: 545, y: 500, w: 150, h: 190 };       // spent ones go down here
    const CW = 120, CH = 290;                             // a cartridge, px
    const SEATED = { x: 300, y: 430 }, RACKED = { x: 620, y: 290 }, BINNED = { x: 620, y: 610 };
    const FAN = { x: 300, y: 185, r: 58 };                // the blower's housing
    const CRATE = { x: 1070, y: 565, w: 150, h: 115 };    // the spare fan's crate (part step)
    const GX = [830, 990, 1150], NAMES = ["O2", "N2", "CO2"];
    const GY = 235, GR = 70, VY = 420, VR = 50, HEADER = 600;
    const A0 = 0.8 * Math.PI, SWEEP = 1.4 * Math.PI;      // a gauge's sweep: lower left, over the top, lower right
    const RED = 0.08;                                     // each end of a gauge in the red, as a fraction of the sweep
    const RED_S = 1.2;                                    // a needle left in the red this long, s: a fumble
    const HOLD_S = 1.0;                                   // a needle held in its band this long, s: that gas is set
    const inRect = (r, x, y) => x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h;
    const wrap = (a) => Math.atan2(Math.sin(a), Math.cos(a));
    const clamp01 = (v) => Math.max(0, Math.min(1, v));
    const ang = (v) => A0 + SWEEP * v;
    let s;

    function partStep(dt, input) {
      const f = s.fan;
      if (input.stick.x || input.stick.y) { f.x += input.stick.x * 480 * dt; f.y += input.stick.y * 480 * dt; f.pad = true; }
      if (input.pressed && Math.hypot(input.x - f.x, input.y - f.y) < 50) f.held = true;
      if (f.held && input.down) { f.x = input.x; f.y = input.y; }
      if ((f.held && input.released) || (f.pad && input.actionPressed)) {
        f.held = false; f.pad = false;
        if (Math.hypot(f.x - FAN.x, f.y - FAN.y) < 44) { f.set = true; f.x = FAN.x; f.y = FAN.y; s.phase = "fitted"; api.stepDone(); }
      }
    }

    function swap(input) {
      const spent = s.phase === "out", cart = spent ? s.spent : s.fresh;
      const seat = () => { if (spent) { s.spent.home = BINNED; s.phase = "in"; } else { s.fresh.home = SEATED; s.phase = "trim"; } };
      if (input.actionPressed && !s.held) { seat(); return; }
      if (input.pressed && Math.abs(input.x - cart.x) < CW / 2 && Math.abs(input.y - cart.y) < CH / 2) {
        s.held = cart; s.ox = cart.x - input.x; s.oy = cart.y - input.y;
      }
      if (s.held === cart && input.down) { cart.x = input.x + s.ox; cart.y = input.y + s.oy; }
      if (s.held === cart && input.released) {
        s.held = null;
        if (inRect(spent ? BIN : SLOT, input.x, input.y)) seat();
        else if (!inRect(spent ? SLOT : RACK, input.x, input.y)) { s.hiss = 1.2; api.fumble(HISS); }
      }
    }

    function trim(dt, input) {
      const vs = s.valves;
      if (input.hit.has("ArrowLeft") || input.hit.has("KeyA")) { s.sel = (s.sel + 2) % 3; s.keys = true; }
      if (input.hit.has("ArrowRight") || input.hit.has("KeyD")) { s.sel = (s.sel + 1) % 3; s.keys = true; }
      if (input.stick.y && !vs[s.sel].locked) { vs[s.sel].u = clamp01(vs[s.sel].u - input.stick.y * 0.3 * dt); s.keys = true; }
      // The pointer turns a wheel round its hub: a turn and a half from shut to open.
      const over = GX.findIndex((x) => Math.hypot(input.x - x, input.y - VY) < VR + 14);
      if (input.pressed && over >= 0 && !vs[over].locked) { s.drag = over; s.sel = over; s.lastA = Math.atan2(input.y - VY, input.x - GX[over]); }
      if (s.drag >= 0) {
        if (!input.down || vs[s.drag].locked) s.drag = -1;
        else {
          const a = Math.atan2(input.y - VY, input.x - GX[s.drag]);
          if (Math.hypot(input.x - GX[s.drag], input.y - VY) > 10) vs[s.drag].u = clamp01(vs[s.drag].u + wrap(a - s.lastA) / (1.5 * Math.PI));
          s.lastA = a;
        }
      }
      if (input.wheel && over >= 0 && !vs[over].locked) vs[over].u = clamp01(vs[over].u - input.wheel * 0.02);
      // The gauges: each needle lags its valve, wanders with the mix, and (later steps) feels the valve before it.
      const amp = Math.min(0.04, 0.01 + 0.008 * s.index), pull = 0.05 * Math.min(s.index, 3);
      vs.forEach((g, i) => {
        if (g.locked) { g.v += (g.c - g.v) * Math.min(1, dt * 4); return; }
        const prev = vs[(i + 2) % 3];
        const raw = clamp01(g.u + g.bias + pull * (prev.u - prev.u0) + amp * Math.sin(s.t * g.fr + g.ph));
        g.v += (raw - g.v) * Math.min(1, dt * 3);
        if (Math.abs(g.v - g.c) <= g.w) { g.hold += dt; if (g.hold >= HOLD_S) g.locked = true; }
        else g.hold = Math.max(0, g.hold - dt * 2);
        g.red = g.v < RED || g.v > 1 - RED ? g.red + dt : 0;
      });
      const bad = vs.find((g) => g.red >= RED_S);
      if (bad) { bad.u = bad.u0; bad.red = 0; s.drag = -1; s.hiss = 1.2; api.fumble(HISS); return; }
      if (vs.every((g) => g.locked)) { s.phase = "done"; api.stepDone(); }
    }

    // ---------------------------------------------------------------- drawing
    function pipe(g, pts, w = 18) {
      g.beginPath(); pts.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y)));
      g.lineJoin = "round"; g.lineCap = "round";
      g.strokeStyle = "#232c3b"; g.lineWidth = w + 6; g.stroke();
      g.strokeStyle = "#3a4658"; g.lineWidth = w; g.stroke();
      g.strokeStyle = "#56637a"; g.lineWidth = 3; g.stroke();
      g.lineCap = "butt";
    }
    function fanGlyph(g, x, y, r, spin, live) {
      D.disc(g, x, y, r, "#1d2738");
      for (let i = 0; i < 5; i++) {
        const a = spin + (i * Math.PI * 2) / 5;
        g.beginPath(); g.moveTo(x, y);
        g.arc(x, y, r - 6, a, a + 0.75); g.closePath();
        g.fillStyle = live ? "#7d9ab8" : C.steel; g.fill();
      }
      D.disc(g, x, y, r * 0.25, "#2a3446"); D.ring(g, x, y, r * 0.25, "#9aa6b6", 2);
    }
    function cartridge(g, c, spent) {
      const x = c.x - CW / 2, y = c.y - CH / 2;
      g.save();
      D.round(g, x, y, CW, CH, 16); g.fillStyle = spent ? "#3b342d" : "#8fa3b8"; g.fill();
      g.clip();
      if (spent) { g.strokeStyle = "#28221d"; g.lineWidth = 7; for (let k = -CH; k < CW + CH; k += 20) { g.beginPath(); g.moveTo(x + k, y); g.lineTo(x + k - CH, y + CH); g.stroke(); } }
      else { g.fillStyle = "#a9bccf"; g.fillRect(x + 12, y, 16, CH); }
      g.restore();
      D.round(g, x, y, CW, CH, 16); g.lineWidth = 2; g.strokeStyle = "#0b111b"; g.stroke();
      g.fillStyle = C.steel; D.round(g, x + 6, y - 6, CW - 12, 24, 6); g.fill(); D.round(g, x + 6, y + CH - 18, CW - 12, 24, 6); g.fill();
      g.strokeStyle = "#9aa6b6"; g.lineWidth = 5; g.beginPath(); g.arc(c.x, y - 6, 22, Math.PI, 0); g.stroke();
      // The saturation window: green and clear when fresh, a dark red cross when spent.
      const wy = y + CH * 0.36;
      D.round(g, x + 22, wy, CW - 44, 44, 8); g.fillStyle = spent ? "#4a1c24" : "#0f2a1c"; g.fill();
      if (spent) { g.strokeStyle = C.danger; g.lineWidth = 5; g.beginPath(); g.moveTo(c.x - 12, wy + 10); g.lineTo(c.x + 12, wy + 34); g.moveTo(c.x + 12, wy + 10); g.lineTo(c.x - 12, wy + 34); g.stroke(); }
      else { D.disc(g, c.x, wy + 22, 13, C.ok); }
      D.text(g, "CO2", c.x, y + CH * 0.74, 24, spent ? "#8a7d6e" : "#0b111b", "center", 700);
    }
    function gauge(g, i, live) {
      const v = s.valves[i], x = GX[i];
      D.disc(g, x, GY, GR + 8, "#0d131c"); D.ring(g, x, GY, GR + 8, C.steel, 4);
      D.disc(g, x, GY, GR, "#070b12");
      const arc = (a, b, col, w) => { g.beginPath(); g.arc(x, GY, GR - 14, ang(a), ang(b)); g.strokeStyle = col; g.lineWidth = w; g.stroke(); };
      arc(0, 1, "#1b2433", 14);
      arc(0, RED, C.danger, 14); arc(1 - RED, 1, C.danger, 14);
      // The red ends carry ticks across them, so the red is a shape as well as a colour.
      g.strokeStyle = "#070b12"; g.lineWidth = 3;
      for (const r0 of [0, 1 - RED]) for (let k = 1; k < 4; k++) {
        const a = ang(r0 + (RED * k) / 4); g.beginPath();
        g.moveTo(x + Math.cos(a) * (GR - 22), GY + Math.sin(a) * (GR - 22)); g.lineTo(x + Math.cos(a) * (GR - 6), GY + Math.sin(a) * (GR - 6)); g.stroke();
      }
      arc(v.c - v.w, v.c + v.w, live ? C.ok : "#245c3c", 20);
      // The hold: a ring filling round the dial; set: a check under the hub.
      if (v.hold > 0 && !v.locked) { g.beginPath(); g.arc(x, GY, GR + 16, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * Math.min(1, v.hold / HOLD_S)); g.strokeStyle = C.ok; g.lineWidth = 5; g.stroke(); }
      if (v.locked) D.ring(g, x, GY, GR + 16, C.ok, 5);
      const a = ang(v.v);
      g.beginPath(); g.moveTo(x, GY); g.lineTo(x + Math.cos(a) * (GR - 8), GY + Math.sin(a) * (GR - 8));
      g.strokeStyle = v.red > 0 ? C.danger : C.fg; g.lineWidth = 4; g.lineCap = "round"; g.stroke(); g.lineCap = "butt";
      D.disc(g, x, GY, 9, "#9aa6b6");
      D.text(g, NAMES[i], x, GY + 40, 22, v.locked ? C.ok : C.fg, "center", 700);
      if (v.locked) {
        g.beginPath(); g.moveTo(x - 9, GY + 58); g.lineTo(x - 2, GY + 65); g.lineTo(x + 11, GY + 51);
        g.strokeStyle = C.ok; g.lineWidth = 4; g.stroke();
      }
    }
    function valve(g, i, live) {
      const v = s.valves[i], x = GX[i];
      D.disc(g, x, VY, VR, "#141b27");
      D.ring(g, x, VY, VR - 4, v.locked ? C.ok : live ? C.steel : "#3a4658", 10);
      const a = v.u * 3 * Math.PI;
      g.strokeStyle = live ? "#9aa6b6" : "#4a5566"; g.lineWidth = 7;
      for (let k = 0; k < 3; k++) {
        const b = a + (k * Math.PI * 2) / 3; g.beginPath();
        g.moveTo(x, VY); g.lineTo(x + Math.cos(b) * (VR - 8), VY + Math.sin(b) * (VR - 8)); g.stroke();
      }
      D.disc(g, x, VY, 11, "#2a3446"); D.ring(g, x, VY, 11, "#9aa6b6", 2);
      if (live && s.keys && i === s.sel && !v.locked) D.ring(g, x, VY, VR + 9, C.amber, 3);
    }

    return {
      get state() { return s; },   // for tools: the headless checks read the round
      step(index, isPart) {
        const r = api.rand();
        const w = Math.max(0.05, 0.1 - 0.015 * index);
        s = {
          index, t: 0, phase: isPart ? "part" : "out",
          spent: { x: SEATED.x, y: SEATED.y, home: SEATED },
          fresh: { x: RACKED.x, y: RACKED.y, home: RACKED },
          fan: { x: CRATE.x + CRATE.w / 2, y: CRATE.y + CRATE.h / 2, held: false, pad: false, set: !isPart },
          held: null, ox: 0, oy: 0, sel: 0, drag: -1, lastA: 0, keys: false, spin: 0, hiss: 0,
          valves: GX.map(() => {
            const c = 0.3 + 0.4 * r(), bias = (r() - 0.5) * 0.2, off = w + 0.08 + 0.08 * r();
            let v0 = c + (r() < 0.5 ? -off : off);
            if (v0 < 0.15 || v0 > 0.85) v0 = 2 * c - v0;
            v0 = Math.max(0.15, Math.min(0.85, v0));
            return { c, w, bias, u: v0 - bias, u0: v0 - bias, v: v0, hold: 0, red: 0, locked: false, ph: r() * 6.28, fr: 0.6 + 0.6 * r() };
          }),
        };
      },
      update(dt, input) {
        s.t += dt;
        s.hiss = Math.max(0, s.hiss - dt);
        if (s.phase === "trim" || s.phase === "done") s.spin += dt * (s.phase === "done" ? 12 : 7);
        for (const c of [s.spent, s.fresh]) if (s.held !== c) { const k = Math.min(1, dt * 12); c.x += (c.home.x - c.x) * k; c.y += (c.home.y - c.y) * k; }
        if (s.phase === "part") partStep(dt, input);
        else if (s.phase === "out" || s.phase === "in") swap(input);
        else if (s.phase === "trim") trim(dt, input);
      },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        const live = s.phase === "trim" || s.phase === "done";
        const jolt = s.hiss > 0 ? Math.sin(t * 40) * 4 * s.hiss : 0;
        g.save(); g.translate(jolt, 0);
        // Pipes: the cabinet's outlet behind the bin to the valve bank, each valve up to its gauge.
        pipe(g, [[CAB.x + CAB.w - 10, HEADER], [GX[2], HEADER]]);
        GX.forEach((x) => { pipe(g, [[x, HEADER], [x, VY]], 14); pipe(g, [[x, VY], [x, GY]], 10); });
        // The cabinet: blower on top, the cartridge slot, the intake grille below.
        D.panel(g, CAB.x, CAB.y, CAB.w, CAB.h, 16, "#101722");
        D.disc(g, FAN.x, FAN.y, FAN.r + 10, "#0b111b"); D.ring(g, FAN.x, FAN.y, FAN.r + 10, C.steel, 3);
        if (s.fan.set) fanGlyph(g, FAN.x, FAN.y, FAN.r, s.spin, live);
        else { D.disc(g, FAN.x, FAN.y, FAN.r, "#120d08"); D.ring(g, FAN.x, FAN.y, FAN.r, C.amber, 4); }
        D.ring(g, FAN.x, FAN.y, FAN.r + 4, "#2a3446", 2);
        D.round(g, SLOT.x, SLOT.y, SLOT.w, SLOT.h, 12); g.fillStyle = "#05080d"; g.fill();
        g.lineWidth = s.phase === "in" ? 4 : 2; g.strokeStyle = s.phase === "in" ? C.amber : C.steel; g.stroke();
        for (let k = 0; k < 4; k++) { g.fillStyle = "#1a2230"; g.fillRect(CAB.x + 60, 612 + k * 18, CAB.w - 120, 8); }
        // The run lamp: a green disc running, a red triangle stopped.
        if (live) D.disc(g, CAB.x + CAB.w - 40, CAB.y + 34, 13, C.ok);
        else { g.beginPath(); g.moveTo(CAB.x + CAB.w - 40, CAB.y + 20); g.lineTo(CAB.x + CAB.w - 55, CAB.y + 46); g.lineTo(CAB.x + CAB.w - 25, CAB.y + 46); g.closePath(); g.fillStyle = C.danger; g.fill(); }
        // The rack and the bin.
        D.panel(g, RACK.x, RACK.y, RACK.w, RACK.h, 12, "#0d131c", s.held === s.fresh ? C.amber : C.line);
        D.round(g, BIN.x + 8, BIN.y, BIN.w - 16, 30, 6); g.fillStyle = "#05080d"; g.fill();
        if (s.phase !== "out" || s.held === s.spent) cartridge(g, s.spent, true);
        D.panel(g, BIN.x, BIN.y + 22, BIN.w, BIN.h - 22, 12, "#141b27", s.held === s.spent ? C.amber : C.line);
        g.beginPath(); g.moveTo(BIN.x + 45, BIN.y + 80); g.lineTo(BIN.x + BIN.w / 2, BIN.y + 120); g.lineTo(BIN.x + BIN.w - 45, BIN.y + 80);
        g.strokeStyle = "#3a4658"; g.lineWidth = 10; g.stroke();
        if (s.phase === "out" && s.held !== s.spent) cartridge(g, s.spent, true);
        cartridge(g, s.fresh, false);
        for (const y of [RACK.y + 50, RACK.y + RACK.h - 50]) { g.fillStyle = C.steel; g.fillRect(RACK.x - 6, y - 6, 16, 12); g.fillRect(RACK.x + RACK.w - 10, y - 6, 16, 12); }
        // The valve bank and its gauges.
        for (let i = 0; i < 3; i++) { valve(g, i, s.phase === "trim" || s.phase === "done"); gauge(g, i, live); }
        g.fillStyle = "#0a0f17"; g.fillRect(0, 690, api.W, api.H - 690);
        if (s.phase === "part" || s.phase === "fitted") {
          if (!s.fan.set) {
            D.panel(g, CRATE.x, CRATE.y, CRATE.w, CRATE.h, 14, "#141b27");
            fanGlyph(g, s.fan.x, s.fan.y, 44, 0, false);
            D.ring(g, s.fan.x, s.fan.y, 46, "#f0c08a", 3);
          }
        }
        g.restore();
      },
    };
  },
});
