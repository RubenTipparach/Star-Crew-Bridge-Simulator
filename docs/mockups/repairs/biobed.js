/*
 * repairs/biobed.js: the medbay biobeds' repair (openspec/changes/repair-minigames, design 2).
 *
 * Sensor calibration. The bed's monitor shows a channel's reference trace (wide, dim) and the bed's live trace over
 * it. Turn the two knobs, GAIN and PHASE (drag round the knob, the wheel over it, or the arrows), until the live trace
 * lies on the reference and turns green, and hold it there 2 s. A step is one sensor channel (ECG, SpO2, breathing,
 * EEG, temperature); later steps tighten the match, add noise and let the patient's signal wander. A knob turned into
 * its red zone over-drives the sensor: a fumble, the bed alarms, the knob falls back. A disabled bed's first step fits
 * the new sensor module: drag it from the crate into its socket on the arch.
 */
RepairKit.register({
  id: "biobed",
  title: "Medbay biobeds",
  place: "Medbay",
  group: "Life",
  hazard: "The bed alarms",
  down: "Beds heal at the unpowered rate",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const ALARM = "The bed alarms";
    const MON = { x: 600, y: 96, w: 640, h: 380 };        // the monitor
    const SX0 = 630, SX1 = 1210, SY = 300, SCALE = 100;   // its trace area: left, right, mid line, px per unit
    const PERIOD = 190;                                   // one beat across the screen, px
    const KX = [760, 980], KY = 588, KR = 56, NAMES = ["GAIN", "PHASE"];
    const RED = 0.9;                                      // a knob past this is over-driven
    const HOLD_S = 2.0;                                   // the match held this long, s: the channel is calibrated
    const ARCH = { x: 310, y: 450, r: 200 };              // the sensor arch over the bed
    const SOCKET = { x: ARCH.x + ARCH.r * Math.cos(-2.09), y: ARCH.y + ARCH.r * Math.sin(-2.09) };
    const CRATE = { x: 1100, y: 560, w: 135, h: 115 };    // the spare module's crate (part step)
    const CHANNELS = ["ECG", "SpO2", "RESP", "EEG", "TEMP"];
    const g2 = (p, m, w) => Math.exp(-(((p - m) / w) ** 2));
    const WAVES = [
      (p) => 0.12 * g2(p, 0.12, 0.03) - 0.15 * g2(p, 0.27, 0.01) + g2(p, 0.3, 0.014) - 0.25 * g2(p, 0.33, 0.012) + 0.3 * g2(p, 0.55, 0.05),
      (p) => 0.9 * g2(p, 0.25, 0.08) + 0.35 * g2(p, 0.5, 0.07) - 0.3,
      (p) => 0.7 * Math.sin(2 * Math.PI * p),
      (p) => 0.4 * Math.sin(6 * Math.PI * p) + 0.3 * Math.sin(14 * Math.PI * p + 1) + 0.2 * Math.sin(26 * Math.PI * p + 2),
      (p) => 0.6 * Math.sin(2 * Math.PI * p) + 0.2 * Math.sin(4 * Math.PI * p),
    ];
    const wrap = (a) => Math.atan2(Math.sin(a), Math.cos(a));
    const ang = (k) => 0.75 * Math.PI + 1.5 * Math.PI * k;
    const frac = (x) => x - Math.floor(x);
    let s, firstCh = 0;

    const targets = () => [
      s.base[0] + s.drift * 0.5 * Math.sin(s.t * 0.45 + s.ph),
      s.base[1] + s.drift * Math.sin(s.t * 0.6 + s.ph * 2),
    ];
    const matched = () => { const tg = targets(); return s.k.every((k, i) => Math.abs(k - tg[i]) <= s.tol); };

    function partStep(dt, input) {
      const p = s.mod;
      if (input.stick.x || input.stick.y) { p.x += input.stick.x * 480 * dt; p.y += input.stick.y * 480 * dt; p.pad = true; }
      if (input.pressed && Math.hypot(input.x - p.x, input.y - p.y) < 50) p.held = true;
      if (p.held && input.down) { p.x = input.x; p.y = input.y; }
      if ((p.held && input.released) || (p.pad && input.actionPressed)) {
        p.held = false; p.pad = false;
        if (Math.hypot(p.x - SOCKET.x, p.y - SOCKET.y) < 40) { p.set = true; p.x = SOCKET.x; p.y = SOCKET.y; s.phase = "fitted"; api.stepDone(); }
      }
    }

    function tune(dt, input) {
      if (input.hit.has("ArrowLeft") || input.hit.has("KeyA")) { s.sel = 0; s.keys = true; }
      if (input.hit.has("ArrowRight") || input.hit.has("KeyD")) { s.sel = 1; s.keys = true; }
      if (input.stick.y) { s.k[s.sel] = Math.max(0, s.k[s.sel] - input.stick.y * 0.25 * dt); s.keys = true; }
      const over = KX.findIndex((x) => Math.hypot(input.x - x, input.y - KY) < KR + 22);
      if (input.pressed && over >= 0) { s.drag = over; s.sel = over; s.lastA = Math.atan2(input.y - KY, input.x - KX[over]); }
      if (s.drag >= 0) {
        if (!input.down) s.drag = -1;
        else {
          const a = Math.atan2(input.y - KY, input.x - KX[s.drag]);
          if (Math.hypot(input.x - KX[s.drag], input.y - KY) > 10) s.k[s.drag] = Math.max(0, s.k[s.drag] + wrap(a - s.lastA) / (1.5 * Math.PI));
          s.lastA = a;
        }
      }
      if (input.wheel && over >= 0) s.k[over] = Math.max(0, s.k[over] - input.wheel * 0.01);
      const hot = s.k.findIndex((k) => k > RED);
      if (hot >= 0) { s.k[hot] = 0.78; s.drag = -1; s.alarm = 1.6; s.hold = 0; api.fumble(ALARM); return; }
      if (matched()) { s.hold += dt; if (s.hold >= HOLD_S) { s.phase = "done"; api.stepDone(); } }
      else s.hold = 0;
    }

    // ---------------------------------------------------------------- drawing
    function trace(g, k0, k1, wave, col, w, noise, t) {
      g.beginPath();
      for (let x = SX0; x <= SX1; x += 3) {
        const p = (x - SX0) / PERIOD - t * 0.5 + k1;
        const n = noise ? noise * Math.sin(x * 0.9 + t * 23) * Math.sin(x * 0.37 - t * 17) : 0;
        const y = SY - SCALE * (0.3 + 1.2 * k0) * wave(frac(p)) + n;
        x === SX0 ? g.moveTo(x, y) : g.lineTo(x, y);
      }
      g.strokeStyle = col; g.lineWidth = w; g.lineJoin = "round"; g.stroke();
    }
    function module(g, x, y) {
      D.round(g, x - 26, y - 20, 52, 40, 8); g.fillStyle = C.steel; g.fill();
      D.disc(g, x, y, 12, "#0b111b"); D.ring(g, x, y, 12, C.accent, 3);
      g.fillStyle = "#4a5566"; g.fillRect(x - 20, y + 20, 40, 6);
    }
    function knob(g, i) {
      const x = KX[i], k = s.k[i], live = s.phase === "tune";
      // The scale: the working sweep, then the red zone, ticked across so it reads without its colour.
      g.lineWidth = 10;
      g.beginPath(); g.arc(x, KY, KR + 18, ang(0), ang(RED)); g.strokeStyle = "#1f2a37"; g.stroke();
      g.beginPath(); g.arc(x, KY, KR + 18, ang(RED), ang(1)); g.strokeStyle = C.danger; g.stroke();
      g.strokeStyle = "#070b12"; g.lineWidth = 3;
      for (let j = 1; j < 4; j++) { const a = ang(RED + (j * (1 - RED)) / 4); g.beginPath(); g.moveTo(x + Math.cos(a) * (KR + 12), KY + Math.sin(a) * (KR + 12)); g.lineTo(x + Math.cos(a) * (KR + 24), KY + Math.sin(a) * (KR + 24)); g.stroke(); }
      for (let j = 0; j <= 9; j++) { const a = ang(j / 10); g.fillStyle = "#3a4658"; g.beginPath(); g.arc(x + Math.cos(a) * (KR + 32), KY + Math.sin(a) * (KR + 32), 2.5, 0, Math.PI * 2); g.fill(); }
      D.disc(g, x, KY, KR, "#1d2738"); D.ring(g, x, KY, KR, live ? "#9aa6b6" : "#4a5566", 4);
      for (let j = 0; j < 12; j++) { const a = (j * Math.PI) / 6 + ang(k); g.fillStyle = "#2a3446"; g.beginPath(); g.arc(x + Math.cos(a) * (KR - 8), KY + Math.sin(a) * (KR - 8), 4, 0, Math.PI * 2); g.fill(); }
      const a = ang(k);
      g.beginPath(); g.moveTo(x + Math.cos(a) * 12, KY + Math.sin(a) * 12); g.lineTo(x + Math.cos(a) * (KR - 6), KY + Math.sin(a) * (KR - 6));
      g.strokeStyle = k > RED - 0.06 ? C.danger : C.fg; g.lineWidth = 6; g.lineCap = "round"; g.stroke(); g.lineCap = "butt";
      if (s.keys && s.sel === i && live) D.ring(g, x, KY, KR + 42, C.amber, 3);
      D.text(g, NAMES[i], x, KY + 98, 18, C.dim, "center", 700);
    }

    return {
      get state() { return s; },   // for tools: the headless checks read the round
      step(index, isPart) {
        const r = api.rand();
        if (isPart) firstCh = 1;
        const base = [0.18 + 0.5 * r(), 0.18 + 0.55 * r()];
        const k = base.map((b) => { const off = 0.18 + 0.12 * r(); return b + off < 0.82 && (r() < 0.5 || b - off < 0.02) ? b + off : b - off; });
        s = {
          index, t: 0, phase: isPart ? "part" : "tune", ch: index % CHANNELS.length, base, k, ph: r() * 6.28,
          tol: Math.max(0.025, 0.045 - 0.006 * index), drift: Math.min(0.05, 0.015 * index), noise: 2 + 2.5 * index,
          hold: 0, sel: 0, drag: -1, lastA: 0, keys: false, alarm: 0,
          mod: { x: CRATE.x + CRATE.w / 2, y: CRATE.y + CRATE.h / 2, held: false, pad: false, set: !isPart },
        };
      },
      update(dt, input) {
        s.t += dt;
        s.alarm = Math.max(0, s.alarm - dt);
        if (s.phase === "part") partStep(dt, input);
        else if (s.phase === "tune") tune(dt, input);
      },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        g.fillStyle = "#0a0f17"; g.fillRect(0, 690, api.W, api.H - 690);
        const live = s.phase === "tune" || s.phase === "done";
        // The cable from the arch to the monitor.
        g.beginPath(); g.moveTo(ARCH.x + ARCH.r, ARCH.y - 10); g.bezierCurveTo(580, 470, 560, 330, MON.x, 330);
        g.strokeStyle = "#2a3446"; g.lineWidth = 8; g.stroke();
        // The bed: pedestal, frame, mattress, the patient under a blanket.
        g.fillStyle = "#1d2738"; g.fillRect(250, 520, 120, 170);
        g.fillStyle = "#2a3446"; g.fillRect(200, 676, 220, 14);
        D.round(g, 60, 492, 500, 34, 8); g.fillStyle = "#3a4658"; g.fill();
        D.round(g, 70, 452, 480, 44, 14); g.fillStyle = "#c9d2dc"; g.fill();
        D.round(g, 80, 430, 82, 30, 12); g.fillStyle = "#e8eef6"; g.fill();
        D.disc(g, 126, 418, 25, "#8a6a58");
        g.beginPath(); g.moveTo(150, 456); g.bezierCurveTo(170, 400, 300, 404, 380, 420); g.bezierCurveTo(450, 430, 500, 426, 540, 456); g.closePath();
        g.fillStyle = "#3d6f8f"; g.fill();
        // The sensor arch, its pads (the channel's lit), the module socket, the alarm lamp.
        g.beginPath(); g.arc(ARCH.x, ARCH.y, ARCH.r, Math.PI, 0); g.strokeStyle = "#141b27"; g.lineWidth = 34; g.stroke();
        g.beginPath(); g.arc(ARCH.x, ARCH.y, ARCH.r, Math.PI, 0); g.strokeStyle = "#2a3446"; g.lineWidth = 24; g.stroke();
        for (let j = 0; j < 5; j++) {
          const a = Math.PI + (Math.PI * (j + 0.5)) / 5 + 0.0, px = ARCH.x + ARCH.r * Math.cos(a), py = ARCH.y + ARCH.r * Math.sin(a);
          if (j === 1) continue;
          D.disc(g, px, py, 9, live && j === [0, 2, 3, 4, 0][s.ch] ? C.accent : "#3d4a5c");
        }
        if (s.mod.set) module(g, SOCKET.x, SOCKET.y);
        else { D.disc(g, SOCKET.x, SOCKET.y, 28, "#120d08"); D.ring(g, SOCKET.x, SOCKET.y, 28, C.amber, 4); }
        const lamp = s.alarm > 0 && Math.sin(t * 18) > 0;
        g.beginPath(); g.moveTo(ARCH.x, ARCH.y - ARCH.r - 40); g.lineTo(ARCH.x - 20, ARCH.y - ARCH.r - 6); g.lineTo(ARCH.x + 20, ARCH.y - ARCH.r - 6); g.closePath();
        g.fillStyle = lamp ? C.danger : "#3a2226"; g.fill();
        if (s.alarm > 0) { g.fillStyle = `rgba(255,71,87,${0.12 * (lamp ? 1 : 0.4)})`; g.fillRect(40, 220, 540, 470); }
        // The monitor: channel tabs, the grid, the reference and the live trace, the hold.
        D.panel(g, MON.x, MON.y, MON.w, MON.h, 16, "#0d131c");
        const n = Math.max(1, api.steps - firstCh);
        for (let i = 0; i < n; i++) {
          const ch = (i + firstCh) % CHANNELS.length, idx = i + firstCh, tw = Math.min(110, (MON.w - 40) / n - 8), x = MON.x + 20 + i * (tw + 8);
          const done = idx < s.index || (idx === s.index && s.phase === "done"), cur = idx === s.index && s.phase !== "part" && s.phase !== "fitted";
          D.round(g, x, MON.y + 10, tw, 28, 8); g.fillStyle = cur ? "#1d2a3c" : "#121a25"; g.fill();
          if (cur) { g.lineWidth = 2; g.strokeStyle = C.accent; g.stroke(); }
          D.text(g, CHANNELS[ch], x + 12, MON.y + 24, 16, done ? C.ok : cur ? C.fg : C.dim, "left", 700);
          if (done) { g.beginPath(); g.moveTo(x + tw - 26, MON.y + 24); g.lineTo(x + tw - 20, MON.y + 30); g.lineTo(x + tw - 10, MON.y + 18); g.strokeStyle = C.ok; g.lineWidth = 3; g.stroke(); }
        }
        g.fillStyle = "#04080c"; g.fillRect(SX0 - 10, MON.y + 48, SX1 - SX0 + 20, MON.h - 70);
        g.strokeStyle = "#0f1a24"; g.lineWidth = 1;
        for (let x = SX0; x <= SX1; x += 40) { g.beginPath(); g.moveTo(x, MON.y + 50); g.lineTo(x, MON.y + MON.h - 24); g.stroke(); }
        for (let y = MON.y + 60; y < MON.y + MON.h - 24; y += 40) { g.beginPath(); g.moveTo(SX0 - 8, y); g.lineTo(SX1 + 8, y); g.stroke(); }
        g.save(); g.beginPath(); g.rect(SX0 - 10, MON.y + 48, SX1 - SX0 + 20, MON.h - 70); g.clip();
        const wave = WAVES[s.ch], tg = targets(), ok = s.phase === "done" || (s.phase === "tune" && matched());
        if (s.phase === "part" || s.phase === "fitted") { g.fillStyle = "#1b2433"; g.fillRect(SX0, SY - 1, SX1 - SX0, 3); }
        else trace(g, tg[0], tg[1], wave, "#24405a", 14, 0, s.t);
        if (s.phase !== "part" && s.phase !== "fitted") trace(g, s.k[0], s.k[1], wave, ok ? C.ok : C.accent, 3, s.phase === "done" ? 0 : s.noise, s.t);
        g.restore();
        // The hold: a bar along the screen's foot.
        const hf = s.phase === "done" ? 1 : s.hold / HOLD_S;
        g.fillStyle = "#121a25"; g.fillRect(SX0, MON.y + MON.h - 18, SX1 - SX0, 8);
        g.fillStyle = C.ok; g.fillRect(SX0, MON.y + MON.h - 18, (SX1 - SX0) * Math.min(1, hf), 8);
        // The knobs.
        D.panel(g, 640, 490, 460, 210, 18, "#0d131c");
        knob(g, 0); knob(g, 1);
        if (!s.mod.set) {
          D.panel(g, CRATE.x, CRATE.y, CRATE.w, CRATE.h, 14, "#141b27");
          module(g, s.mod.x, s.mod.y);
          D.ring(g, s.mod.x, s.mod.y, 40, "#f0c08a", 3);
        }
      },
    };
  },
});
