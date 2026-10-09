/*
 * repairs/sensors.js: the sensor array's repair (openspec/changes/repair-minigames, design 2).
 *
 * The sensor bay with the dish. Left, a small sky map: the dish's aim is the reticle, and the hatched border is its
 * mechanical stops. Beside it the return meter, cold blue to hot amber. Right, the dish itself on its turntable.
 * Below, the scope's trace and three band filters. First sweep: drag on the sky map (the heavy dish slews toward the
 * pointer) or steer with the arrows, read the meter, and rest the reticle on the strongest return until it locks.
 * Pushing the dish into a stop strains it; held there, it jams: a fumble, and the dish will not move for a moment.
 * Then filter: three sliders, LO, MID and HI (drag them, or Left and Right to pick and Up and Down to move), each
 * cancels one band of the noise on the trace (the slow swell, the ripple, the hiss); set all three until the
 * contact's blip stands clean and hold it. Later rounds narrow the return, add a weaker false return and tighten the
 * filters. A disabled array's first step fits a new feed horn: drag it from the crate to the dish's focus.
 */
RepairKit.register({
  id: "sensors",
  title: "Sensors",
  place: "Sensor bay and the dish",
  group: "Weapons and sensors",
  hazard: "The dish jams",
  // The how-to card (repair-minigames 6g), drawn by the kit: pictures and a few words, on demand.
  guide: {
    steps: [
      { icon: "drag", text: "Drag the dish round the sky" },
      { icon: "band", text: "Rest on the strongest return" },
      { icon: "slider", text: "LO, MID, HI clean the trace" },
      { icon: "hold", text: "Hold the blip clean" },
    ],
    mistake: "The dish held against a stop: it jams",
    now: (q) => (q.part ? -1 : q.phase === "sweep" ? (q.lockT > 0 ? 1 : 0) : q.phase === "filter" ? 2 : -1),
  },
  down: "Science and tactical lose range",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const MAP = { x: 40, y: 100, w: 580, h: 370 };      // the sky map's panel
    const R = { x0: 75, x1: 585, y0: 135, y1: 435 };    // the dish's travel inside its stops
    const METER = { x: 648, y: 112, w: 34, h: 346 };
    const SCOPE = { x: 40, y: 495, w: 820, h: 205 };
    const SL = { xs: [950, 1065, 1180], y0: 520, y1: 660 };
    const PIVOT = { x: 930, y: 300 };                   // the dish's elevation pivot on its yoke
    const SLEW = 260;                                   // the dish's slew, px of map a second
    const LOCK = 0.92, LOCK_S = 0.8, CLEAN_S = 0.8, JAM_AT = 0.6, JAM_S = 1.5;
    let hadPart = false, part, horn, phase, hard;
    let dish, steering, src, decoy, sigma, noise, noiseV, lockT, strain, jamT, trail, trailT, specks;
    let bands, sel, grab, cleanT, played, clock = 0;
    const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
    const gauss = (dx, dy, s) => Math.exp(-(dx * dx + dy * dy) / (2 * s * s));
    const signal = (x, y) => Math.max(gauss(x - src.x, y - src.y, sigma), decoy ? 0.58 * gauss(x - decoy.x, y - decoy.y, sigma * 1.2) : 0);
    const residual = (i) => Math.min(1, Math.abs(bands[i].v - bands[i].c) * 3);
    const tol = () => Math.max(0.08, 0.15 - 0.02 * hard);
    const clean = () => [0, 1, 2].every((i) => residual(i) < tol());

    /** The dish's facing from the reticle: elevation from height on the map, azimuth from across it. */
    function pose() {
      const el = (10 + 70 * (R.y1 - dish.y) / (R.y1 - R.y0)) * Math.PI / 180;
      const az = ((dish.x - R.x0) / (R.x1 - R.x0) - 0.5) * 240 * Math.PI / 180;
      const side = az < 0 ? -1 : 1, squash = 0.45 + 0.55 * Math.abs(Math.cos(az * 0.5));
      const dir = { x: Math.cos(el) * side, y: -Math.sin(el) };
      const focus = { x: PIVOT.x + dir.x * 110 * squash, y: PIVOT.y + dir.y * 110 };
      return { el, az, dir, squash, focus };
    }


    return {
      step(index, isPart) {
        const r = api.rand();
        if (isPart) hadPart = true;
        part = isPart;
        hard = index;   // the round's level - 1 (repair-minigames 1a)
        phase = part ? "part" : "sweep";
        horn = { x: 1160, y: 400, held: false, set: false };
        // The return somewhere inside the stops, the dish parked away from it.
        src = { x: R.x0 + 50 + r() * (R.x1 - R.x0 - 100), y: R.y0 + 40 + r() * (R.y1 - R.y0 - 80) };
        decoy = hard >= 1 ? { x: R.x0 + 60 + r() * (R.x1 - R.x0 - 120), y: R.y0 + 40 + r() * (R.y1 - R.y0 - 80) } : null;
        if (decoy && Math.hypot(decoy.x - src.x, decoy.y - src.y) < 180) decoy.x = src.x > 330 ? src.x - 220 : src.x + 220;
        sigma = Math.max(42, 70 - 8 * hard);
        dish = { x: src.x > 330 ? R.x0 + 30 : R.x1 - 30, y: src.y > 285 ? R.y0 + 30 : R.y1 - 30 };
        steering = false; noise = 0; noiseV = 0; lockT = 0; strain = 0; jamT = 0; trail = []; trailT = 0;
        specks = Array.from({ length: 150 }, () => ({ x: MAP.x + 8 + r() * (MAP.w - 16), y: MAP.y + 8 + r() * (MAP.h - 16), p: r() * 6.3, s: 1 + r() * 2 }));
        bands = [0, 1, 2].map(() => { const c = 0.15 + 0.7 * r(); return { c, v: c > 0.5 ? c - 0.3 - 0.2 * r() : c + 0.3 + 0.2 * r(), p: r() * 6.3 }; });
        sel = 0; grab = -1; cleanT = 0; played = false;
      },
      /** For tools (shots and tests): the round's state, read only, so a script can play it. */
      peek() { return { phase, part, horn: { ...horn }, focus: pose().focus, dish: { ...dish }, src: { ...src }, R, MAP, SL, bands: bands.map((b) => ({ ...b })), lockT, jamT, strain, played }; },
      update(dt, input) {
        clock += dt;
        if (phase === "part") {
          if (input.pressed && Math.hypot(input.x - horn.x, input.y - horn.y) < 50) horn.held = true;
          if (horn.held && input.down) { horn.x = input.x; horn.y = input.y; }
          if (horn.held && input.released) {
            horn.held = false;
            const f = pose().focus;
            if (Math.hypot(horn.x - f.x, horn.y - f.y) < 40) { horn.set = true; phase = "fitted"; api.stepDone(); }
          }
          return;
        }
        if (phase === "sweep") {
          jamT = Math.max(0, jamT - dt);
          // Where the hands want the dish: the pointer while steering on the map, or the arrows.
          if (input.pressed && input.x >= MAP.x && input.x <= MAP.x + MAP.w && input.y >= MAP.y && input.y <= MAP.y + MAP.h) steering = true;
          if (!input.down) steering = false;
          let want;
          if (steering) want = { x: input.x, y: input.y };
          else want = { x: dish.x + input.stick.x * 60, y: dish.y + input.stick.y * 60 };
          if (jamT > 0) want = { x: dish.x, y: dish.y };
          const dx = want.x - dish.x, dy = want.y - dish.y, d = Math.hypot(dx, dy);
          const stepLen = Math.min(d, SLEW * dt);
          if (d > 0.01) { dish.x += dx / d * stepLen; dish.y += dy / d * stepLen; }
          // The stops: past them the dish does not go, and pushing on them strains it.
          const pushed = (want.x < R.x0 - 4 && dish.x <= R.x0) || (want.x > R.x1 + 4 && dish.x >= R.x1) ||
            (want.y < R.y0 - 4 && dish.y <= R.y0) || (want.y > R.y1 + 4 && dish.y >= R.y1);
          dish.x = clamp(dish.x, R.x0, R.x1); dish.y = clamp(dish.y, R.y0, R.y1);
          strain = pushed && jamT === 0 ? strain + dt : Math.max(0, strain - 2 * dt);
          if (strain >= JAM_AT) {
            strain = 0; jamT = JAM_S; steering = false;
            dish.x = clamp(dish.x, R.x0 + 16, R.x1 - 16); dish.y = clamp(dish.y, R.y0 + 16, R.y1 - 16);
            api.fumble("The dish jams");
            return;
          }
          // The return: the true signal with the sky's noise over it on the meter.
          noiseV += (Math.sin(clock * 7.3) * 0.5 + Math.sin(clock * 13.1 + 1) * 0.5) * dt * 3 - noiseV * dt * 4;
          noise = clamp(noise + noiseV * dt * 8, -1, 1) * 0.98;
          const s = signal(dish.x, dish.y);
          lockT = s >= LOCK && jamT === 0 ? lockT + dt : Math.max(0, lockT - 2 * dt);
          trailT += dt;
          if (trailT > 0.06) { trailT = 0; trail.push({ x: dish.x, y: dish.y, s }); if (trail.length > 180) trail.shift(); }
          if (lockT >= LOCK_S) { lockT = LOCK_S; dish.x = src.x; dish.y = src.y; phase = "filter"; }
          return;
        }
        if (phase !== "filter" || played) return;
        // The three band filters.
        if (input.pressed) {
          SL.xs.forEach((x, i) => { if (Math.abs(input.x - x) < 45 && input.y > SL.y0 - 30 && input.y < SL.y1 + 30) { grab = i; sel = i; } });
        }
        if (grab >= 0 && input.down) bands[grab].v = clamp((SL.y1 - input.y) / (SL.y1 - SL.y0), 0, 1);
        if (input.released) grab = -1;
        if (input.hit.has("ArrowLeft") || input.hit.has("KeyA")) sel = (sel + 2) % 3;
        if (input.hit.has("ArrowRight") || input.hit.has("KeyD")) sel = (sel + 1) % 3;
        if (input.stick.y) bands[sel].v = clamp(bands[sel].v - input.stick.y * 0.3 * dt, 0, 1);
        cleanT = clean() ? cleanT + dt : Math.max(0, cleanT - 2 * dt);
        if (cleanT >= CLEAN_S) { cleanT = CLEAN_S; played = true; api.stepDone(); }
      },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        const sweep = phase === "sweep";
        // ---- The sky map: noise speckle, the stops hatched, the trail coloured by return, the reticle.
        D.panel(g, MAP.x, MAP.y, MAP.w, MAP.h, 16, "#081019");
        g.save(); g.globalAlpha = sweep ? 1 : 0.5;
        for (const p of specks) {
          const a = 0.25 + 0.25 * Math.sin(t * 3 + p.p * 7) + 0.2 * Math.sin(t * 11 + p.p * 3);
          g.fillStyle = `rgba(160,190,220,${clamp(a, 0, 1)})`; g.fillRect(p.x, p.y, p.s, p.s);
        }
        const stopCol = strain > 0 || jamT > 0 ? `rgba(255,71,87,${jamT > 0 ? 0.9 : 0.3 + 0.6 * strain / JAM_AT})` : "rgba(255,71,87,0.2)";
        D.hatch(g, MAP.x + 2, MAP.y + 2, R.x0 - MAP.x - 14, MAP.h - 4, stopCol);
        D.hatch(g, R.x1 + 12, MAP.y + 2, MAP.x + MAP.w - R.x1 - 14, MAP.h - 4, stopCol);
        D.hatch(g, MAP.x + 2, MAP.y + 2, MAP.w - 4, R.y0 - MAP.y - 14, stopCol);
        D.hatch(g, MAP.x + 2, R.y1 + 12, MAP.w - 4, MAP.y + MAP.h - R.y1 - 14, stopCol);
        g.strokeStyle = "#1b2636"; g.lineWidth = 1;
        for (let i = 1; i < 6; i++) { const x = R.x0 + (R.x1 - R.x0) * i / 6; g.beginPath(); g.moveTo(x, R.y0); g.lineTo(x, R.y1); g.stroke(); }
        for (let i = 1; i < 4; i++) { const y = R.y0 + (R.y1 - R.y0) * i / 4; g.beginPath(); g.moveTo(R.x0, y); g.lineTo(R.x1, y); g.stroke(); }
        for (const p of trail) {
          const hot = p.s;
          g.fillStyle = hot > 0.6 ? `rgba(242,160,70,${0.3 + 0.6 * hot})` : `rgba(79,195,247,${0.2 + 0.5 * hot})`;
          g.beginPath(); g.arc(p.x, p.y, 3 + 4 * hot, 0, Math.PI * 2); g.fill();
        }
        g.restore();
        if (!part) {
          const jam = jamT > 0;
          const rc = jam ? C.danger : phase === "sweep" ? C.fg : C.ok;
          D.ring(g, dish.x, dish.y, 18, rc, 3);
          g.strokeStyle = rc; g.lineWidth = 3; g.beginPath();
          if (jam) { g.moveTo(dish.x - 12, dish.y - 12); g.lineTo(dish.x + 12, dish.y + 12); g.moveTo(dish.x + 12, dish.y - 12); g.lineTo(dish.x - 12, dish.y + 12); }
          else for (const [ax, ay] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) { g.moveTo(dish.x + ax * 24, dish.y + ay * 24); g.lineTo(dish.x + ax * 34, dish.y + ay * 34); }
          g.stroke();
          if (lockT > 0 && sweep) {
            g.beginPath(); g.arc(dish.x, dish.y, 28, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * lockT / LOCK_S); g.strokeStyle = C.ok; g.lineWidth = 5; g.stroke();
          }
        }
        // ---- The return meter: cold to hot, the lock line bracketed.
        const s = part ? 0 : phase === "sweep" ? clamp(signal(dish.x, dish.y) + noise * (0.05 + 0.02 * hard), 0, 1) : 1;
        D.round(g, METER.x, METER.y, METER.w, METER.h, 10); g.fillStyle = "#121a26"; g.fill();
        const grad = g.createLinearGradient(0, METER.y + METER.h, 0, METER.y);
        grad.addColorStop(0, C.accent); grad.addColorStop(0.6, "#9fb7c9"); grad.addColorStop(0.85, C.amber); grad.addColorStop(1, "#ff8a3d");
        const fh = METER.h * s;
        D.round(g, METER.x, METER.y + METER.h - fh, METER.w, Math.max(10, fh), 10); g.fillStyle = grad; g.fill();
        const ly = METER.y + METER.h * (1 - LOCK);
        g.strokeStyle = C.ok; g.lineWidth = 3;
        g.beginPath(); g.moveTo(METER.x - 8, ly - 6); g.lineTo(METER.x - 8, ly); g.lineTo(METER.x + METER.w + 8, ly); g.lineTo(METER.x + METER.w + 8, ly - 6); g.stroke();
        // ---- The dish on its turntable.
        D.panel(g, 710, MAP.y, 530, MAP.h, 16, "#0b111b");
        const P = pose();
        g.fillStyle = "#1a2232"; g.beginPath(); g.ellipse(PIVOT.x, 440, 140, 22, 0, 0, Math.PI * 2); g.fill();
        g.strokeStyle = C.steel; g.lineWidth = 2; g.stroke();
        D.disc(g, PIVOT.x + Math.sin(P.az) * 120, 440 + Math.cos(P.az) * 18, 7, C.amber);
        g.fillStyle = "#2a3446"; g.fillRect(PIVOT.x - 22, PIVOT.y, 44, 140);
        g.fillStyle = "#364257"; g.fillRect(PIVOT.x - 46, PIVOT.y - 8, 92, 22);
        const px = { x: -P.dir.y, y: P.dir.x };
        const rim = (k) => ({ x: PIVOT.x + px.x * 125 * k * P.squash + P.dir.x * 26 * P.squash, y: PIVOT.y + px.y * 125 * k + P.dir.y * 26 });
        const r1 = rim(1), r2 = rim(-1), cp = { x: PIVOT.x - P.dir.x * 80 * P.squash, y: PIVOT.y - P.dir.y * 80 };
        g.beginPath(); g.moveTo(r1.x, r1.y); g.quadraticCurveTo(cp.x, cp.y, r2.x, r2.y); g.closePath();
        g.fillStyle = "#c9d1dc"; g.fill(); g.strokeStyle = "#8792a3"; g.lineWidth = 6; g.stroke();
        g.strokeStyle = "#8792a3"; g.lineWidth = 3;
        g.beginPath(); g.moveTo(r1.x, r1.y); g.lineTo(P.focus.x, P.focus.y); g.moveTo(r2.x, r2.y); g.lineTo(P.focus.x, P.focus.y); g.stroke();
        D.disc(g, PIVOT.x, PIVOT.y, 12, "#3a4558");
        const hornIn = !part || horn.set;
        if (hornIn) { D.disc(g, P.focus.x, P.focus.y, 11, jamT > 0 ? C.danger : C.copper); }
        else { g.setLineDash([6, 5]); D.ring(g, P.focus.x, P.focus.y, 16, C.amber, 3); g.setLineDash([]); }
        if (part && !horn.set) {
          D.panel(g, 1100, 330, 120, 120, 14, "#141b27");
          D.disc(g, horn.x, horn.y, 14, C.copper); D.ring(g, horn.x, horn.y, 14, "#f0c08a", 3);
          g.fillStyle = "#7a4a22"; g.fillRect(horn.x - 6, horn.y + 12, 12, 18);
        }
        // ---- The scope: the trace, the noise by band, the contact's blip.
        const filt = phase === "filter";
        D.panel(g, SCOPE.x, SCOPE.y, SCOPE.w, SCOPE.h, 16, "#081019");
        const base = SCOPE.y + SCOPE.h - 72;
        g.save(); g.beginPath(); g.rect(SCOPE.x + 4, SCOPE.y + 4, SCOPE.w - 8, SCOPE.h - 8); g.clip();
        g.strokeStyle = "#16212f"; g.lineWidth = 1;
        for (let i = 1; i < 8; i++) { const x = SCOPE.x + SCOPE.w * i / 8; g.beginPath(); g.moveTo(x, SCOPE.y + 10); g.lineTo(x, SCOPE.y + SCOPE.h - 10); g.stroke(); }
        g.beginPath(); g.moveTo(SCOPE.x + 10, base); g.lineTo(SCOPE.x + SCOPE.w - 10, base); g.stroke();
        const n = part ? [1, 1, 1] : filt ? [residual(0), residual(1), residual(2)] : [1, 1, 1];
        const blipH = part ? 0 : filt ? 96 : 26;
        const frame = Math.floor(t * 30), N = 260, c = clean() && filt;
        g.beginPath();
        for (let j = 0; j <= N; j++) {
          const u = j / N, x = SCOPE.x + 14 + (SCOPE.w - 28) * u;
          const h = Math.sin(j * 12.9898 + frame * 78.233) * 43758.5453;
          const hiss = (h - Math.floor(h)) * 2 - 1;
          const y = base - blipH * Math.exp(-((u - 0.56) ** 2) / (2 * 0.011 ** 2))
            - n[0] * 34 * Math.sin(u * 9.4 + t * 0.9 + bands[0].p)
            - n[1] * 20 * Math.sin(u * 62 + t * 5 + bands[1].p)
            - n[2] * 16 * hiss;
          if (j === 0) g.moveTo(x, y); else g.lineTo(x, y);
        }
        g.strokeStyle = c ? C.ok : sweep || part ? "#3d6f8f" : C.accent; g.lineWidth = 2.5; g.stroke();
        if (filt) {
          const bx = SCOPE.x + 14 + (SCOPE.w - 28) * 0.56, by = base - blipH - 22;
          g.beginPath(); g.moveTo(bx, by - 10); g.lineTo(bx + 10, by); g.lineTo(bx, by + 10); g.lineTo(bx - 10, by); g.closePath();
          g.fillStyle = c ? C.ok : "transparent"; g.fill(); g.strokeStyle = c ? C.ok : C.amber; g.lineWidth = 3; g.stroke();
          if (cleanT > 0) { g.beginPath(); g.arc(bx, by, 20, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * cleanT / CLEAN_S); g.strokeStyle = C.ok; g.lineWidth = 4; g.stroke(); }
        }
        g.restore();
        // ---- The band filters.
        g.save(); g.globalAlpha = filt ? 1 : 0.4;
        D.panel(g, 890, SCOPE.y, 350, SCOPE.h, 16, "#0f1620");
        ["LO", "MID", "HI"].forEach((label, i) => {
          const x = SL.xs[i];
          D.round(g, x - 7, SL.y0, 14, SL.y1 - SL.y0, 7); g.fillStyle = "#1a2232"; g.fill();
          const ky = SL.y1 - (SL.y1 - SL.y0) * bands[i].v;
          D.round(g, x - 30, ky - 13, 60, 26, 8); g.fillStyle = "#c9d1dc"; g.fill();
          g.lineWidth = 3; g.strokeStyle = filt && sel === i ? C.amber : "#3a4558"; g.stroke();
          D.text(g, label, x, SL.y1 + 26, 18, C.dim, "center", 700);
        });
        g.restore();
      },
    };
  },
});
