/*
 * repairs/pump.js: a coolant pump's repair (openspec/changes/repair-minigames, design 6h; reactor-cooling 6a opens it
 * for the core pumps, the radiator pumps and the makeup pump).
 *
 * The pump from the side: the volute (the snail-shell casing) on the left with its impeller behind a window, the
 * coupling, the motor on two feet on the right. A guard over the coupling is screwed on (the kit's cover). Two rounds,
 * alternating:
 *   align: a laser on the pump's shaft throws two dots on two targets on the motor, near and far. Drag each foot's shim
 *     handle up or down (W and S on the selected foot, Tab or A and D to the other): the front foot moves the near dot
 *     most, the rear foot the far one, and each pulls the other's dot a little. Both dots held in their rings: aligned.
 *     A dot left off its target 1.2 s: the fumble "Coupling knocks: the bearings heat".
 *   start: raise the speed lever (drag it, or W and S) to the band at 100% and hold it there, keeping the suction
 *     gauge's needle out of the red. Speed lowers suction, and a fast ramp drops it further while it ramps. The needle in
 *     the red: the fumble "Cavitation: the impeller pits", bubbles in the window and the speed knocked back.
 * A disabled pump's first step fits a new impeller: drag it from the crate onto the shaft, its vanes curving away from
 * the casing's rotation arrow; tap it (or F) to turn it over. Fitted the wrong way round: "Impeller backwards: no flow".
 */
RepairKit.register({
  id: "pump",
  title: "Coolant pump",
  place: "Engineering, the core and radiator pumps",
  group: "Engineering",
  hazard: "Cavitation: the impeller pits",
  // The how-to card (repair-minigames 6g), drawn by the kit: pictures and a few words, on demand.
  guide: {
    steps: [
      { icon: "screw", text: "Unscrew the guard", cover: true },
      { icon: "swap", text: "Vanes away from the arrow" },
      { icon: "drag", text: "Feet: both dots in rings" },
      { icon: "slider", text: "Speed up slowly to the band" },
    ],
    mistake: "Too fast: the needle hits red, the pump cavitates",
    now: (q) => ({ part: 1, align: 2, start: 3 })[q.phase] ?? -1,
  },
  down: "Half the loop's flow for each core pump down",
  panel: { x: 410, y: 150, w: 220, h: 250, screws: 4 },   // the coupling guard (kit: access panels)
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const KNOCK = "Coupling knocks: the bearings heat", CAVITATE = "Cavitation: the impeller pits", BACKWARDS = "Impeller backwards: no flow";
    // The machine (canvas px).
    const VOL = { x: 290, y: 275, r: 108 };             // the volute; its window shows the impeller
    const WIN_R = 64;
    const SHAFT_Y = VOL.y;                              // the pump's shaft line
    const CPL_X = 520;                                  // the coupling's face
    const MOTOR = { x0: 560, x1: 900, h: 150 };         // the motor's body, along the shaft
    const FOOT = [640, 840];                            // the front and rear feet, x
    const SPAN = FOOT[1] - FOOT[0];
    const BASE_Y = 420;                                 // the skid's top
    const TRACK = { y: 560, half: 78 };                 // the shim handles' tracks, under each foot
    const KNOB = 1.6;                                   // px of handle per px of foot height
    const H_MAX = 46;                                   // a foot's travel either way, px
    // The targets: near and far, with the rings the dots must sit in.
    const TGT = [{ x: 1010, y: 270 }, { x: 1160, y: 270 }], PLATE_R = 62, DOT_K = 1.5, PULL = 0.35;
    const OFF_S = 1.2, HOLD_S = 0.6;
    // The start: the lever and the suction gauge.
    const LEVER = { x: 1190, y0: 640, y1: 140, w: 100 };  // 0 speed at y0, V_MAX at y1
    const V_MAX = 1.2, BAND = [0.95, 1.05], HOLD_RUN_S = 1.0;
    const GAUGE = { x: 1000, y: 540, r: 96 };
    const A0 = 0.8 * Math.PI, SWEEP = 1.4 * Math.PI;
    const LAG_S = 0.8, DROP = 0.6;
    // The part step's crate and the impeller in it.
    const CRATE = { x: 700, y: 480, w: 200, h: 190 };
    const IMP_R = 52;
    const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
    let s;

    /** The dots' readings: the motor's error at each target, from the feet's heights against where they belong. */
    const dots = () => {
      const eF = s.h[0] - s.want[0], eR = s.h[1] - s.want[1];
      return [eF + PULL * eR, eR + PULL * eF].map((e) => e * DOT_K);
    };
    /** Where the motor's shaft line is at x (it tilts with the feet): canvas y. */
    const motorY = (x) => {
      const eF = s.h[0] - s.want[0], eR = s.h[1] - s.want[1];
      return SHAFT_Y - (eF + ((eR - eF) * (x - FOOT[0])) / SPAN);
    };
    const ringR = () => Math.max(10, 20 - 3 * s.index);
    const leverY = (v) => LEVER.y0 + ((LEVER.y1 - LEVER.y0) * v) / V_MAX;
    const red = () => Math.min(0.36, 0.25 + 0.03 * s.index);

    let partJob = false;   // this job began by fitting a part, so its rounds count from step 1
    function step(index, isPart) {
      const r = api.rand();
      if (index === 0) partJob = isPart;
      const k = index - (partJob ? 1 : 0);
      // The motor sits true (feet where they belong) except while it is being aligned.
      s = { index, t: 0, phase: isPart ? "part" : k % 2 === 0 ? "align" : "start", sel: 0, keys: false, spin: 0, bubbles: 0, knock: 0, done: false,
        h: [0, 0], want: [0, 0], off: [0, 0], grab: -1 };
      if (s.phase === "part") {
        // The new impeller waits in the crate, the right way round or not, at random.
        s.imp = { x: CRATE.x + CRATE.w / 2, y: CRATE.y + CRATE.h / 2, sweep: r() < 0.5 ? 1 : -1, held: false, moved: 0, seated: false };
        return;
      }
      if (s.phase === "align") {
        // Where the feet belong, and where they start: both dots on their plates, outside their rings.
        s.want = [(r() - 0.5) * 30, (r() - 0.5) * 30];
        for (let tries = 0; tries < 50; tries++) {
          s.h = [clamp(s.want[0] + (r() < 0.5 ? -1 : 1) * (16 + r() * 16), -H_MAX, H_MAX), clamp(s.want[1] + (r() < 0.5 ? -1 : 1) * (16 + r() * 16), -H_MAX, H_MAX)];
          const d = dots();
          if (d.every((v) => Math.abs(v) > ringR() + 6 && Math.abs(v) < PLATE_R - 12)) break;
        }
        s.grab = -1; s.off = [0, 0]; s.hold = 0;
        return;
      }
      s.v = 0; s.vPrev = 0; s.rate = 0; s.pLag = 1; s.p = 1; s.grab = false; s.hold = 0;
    }

    // ---------------------------------------------------------------- the rounds
    function partUpdate(dt, input) {
      const m = s.imp;
      if (m.seated) return;
      if (input.hit.has("KeyF")) m.sweep = -m.sweep;
      if (input.stick.x || input.stick.y) { s.keys = true; m.x += input.stick.x * 480 * dt; m.y += input.stick.y * 480 * dt; }
      if (input.pressed && Math.hypot(input.x - m.x, input.y - m.y) < IMP_R + 14) { m.held = true; m.moved = 0; m.ox = input.x - m.x; m.oy = input.y - m.y; m.px = input.x; m.py = input.y; }
      if (m.held && input.down) { m.moved += Math.hypot(input.x - m.px, input.y - m.py); m.px = input.x; m.py = input.y; m.x = input.x - m.ox; m.y = input.y - m.oy; }
      const drop = (m.held && input.released) || (s.keys && input.actionPressed);
      if (!drop) return;
      const wasTap = m.held && m.moved < 12;
      m.held = false;
      if (wasTap) { m.sweep = -m.sweep; m.x = CRATE.x + CRATE.w / 2; m.y = CRATE.y + CRATE.h / 2; return; }
      if (Math.hypot(m.x - VOL.x, m.y - VOL.y) < 60) {
        // Backward-curved vanes (sweep -1) suit the casing's clockwise turn; the other way round pumps nothing.
        if (m.sweep < 0) { m.seated = true; m.x = VOL.x; m.y = VOL.y; api.stepDone(); return; }
        api.fumble(BACKWARDS);
      }
      m.x = CRATE.x + CRATE.w / 2; m.y = CRATE.y + CRATE.h / 2;
    }

    function alignUpdate(dt, input) {
      if (s.done) return;
      // Keys: the selected foot up and down, Tab or A and D to the other.
      for (const code of ["Tab", "KeyA", "KeyD", "ArrowLeft", "ArrowRight"]) if (input.hit.has(code)) { s.keys = true; s.sel = 1 - s.sel; }
      const ky = (input.keys.has("KeyW") || input.keys.has("ArrowUp") ? 1 : 0) - (input.keys.has("KeyS") || input.keys.has("ArrowDown") ? 1 : 0);
      if (ky) { s.keys = true; s.h[s.sel] = clamp(s.h[s.sel] + ky * 30 * dt, -H_MAX, H_MAX); }
      if (input.pressed) {
        const i = KIT.nearest(FOOT.map((x, k) => [x, TRACK.y - s.h[k] * KNOB]), input.x, input.y, KIT.TOUCH_R + 20);
        if (i >= 0) { s.grab = i; s.sel = i; s.keys = false; s.gy = input.y + s.h[i] * KNOB; }
      }
      if (s.grab >= 0 && input.down) s.h[s.grab] = clamp((s.gy - input.y) / KNOB, -H_MAX, H_MAX);
      if (s.grab >= 0 && input.released) s.grab = -1;
      const d = dots();
      // A dot off its plate too long: the coupling knocks.
      for (let k = 0; k < 2; k++) {
        s.off[k] = Math.abs(d[k]) > PLATE_R ? s.off[k] + dt : 0;
        if (s.off[k] > OFF_S) { s.off = [0, 0]; s.knock = 1; api.fumble(KNOCK); return; }
      }
      s.hold = d.every((v) => Math.abs(v) <= ringR()) && s.grab < 0 ? s.hold + dt : 0;
      if (s.hold >= HOLD_S) { s.done = true; api.stepDone(); }
    }

    function startUpdate(dt, input) {
      if (s.grab === -1) s.grab = false;
      if (!s.done) {
        const ky = (input.keys.has("KeyW") || input.keys.has("ArrowUp") ? 1 : 0) - (input.keys.has("KeyS") || input.keys.has("ArrowDown") ? 1 : 0);
        if (ky) { s.keys = true; s.v = clamp(s.v + ky * 0.3 * dt, 0, V_MAX); }
        if (input.pressed && Math.abs(input.x - LEVER.x) < LEVER.w / 2 + 20 && Math.abs(input.y - leverY(s.v)) < 50) { s.grab = true; s.keys = false; s.gy = input.y - leverY(s.v); }
        if (s.grab && input.down) s.v = clamp(((input.y - s.gy - LEVER.y0) * V_MAX) / (LEVER.y1 - LEVER.y0), 0, V_MAX);
        if (s.grab && input.released) s.grab = false;
      }
      // Suction: lower with speed, followed with a lag, and lower still while the speed rises.
      const rate = Math.max(0, (s.v - s.vPrev) / Math.max(dt, 1e-3));
      s.vPrev = s.v;
      s.rate += (rate - s.rate) * Math.min(1, dt * 8);
      s.pLag += (1 - 0.55 * s.v * s.v - s.pLag) * Math.min(1, dt / LAG_S);
      s.p = s.pLag - DROP * s.rate;
      s.spin += dt * s.v * 14;
      s.bubbles = Math.max(0, s.bubbles - dt);
      if (s.done) return;
      if (s.p < red() && s.v > 0.2) {
        s.v = 0.4; s.vPrev = 0.4; s.rate = 0; s.grab = false; s.bubbles = 1.4; s.hold = 0;
        api.fumble(CAVITATE);
        return;
      }
      s.hold = s.v >= BAND[0] && s.v <= BAND[1] && s.p >= red() ? s.hold + dt : 0;
      if (s.hold >= HOLD_RUN_S) { s.done = true; api.stepDone(); }
    }

    // ---------------------------------------------------------------- drawing
    /** An impeller: a hub and six vanes curving `sweep` (-1 backward for the clockwise casing), turned `a`. */
    function impeller(g, x, y, r, a, sweep, col = "#c9d3e0") {
      D.disc(g, x, y, r, "#1b2433"); D.ring(g, x, y, r, "#4a5566", 3);
      g.strokeStyle = col; g.lineWidth = Math.max(3, r * 0.09); g.lineCap = "round";
      for (let k = 0; k < 6; k++) {
        const b = a + (k * Math.PI) / 3;
        g.beginPath();
        for (let j = 0; j <= 8; j++) { const u = j / 8, rr = r * (0.28 + 0.66 * u), th = b + sweep * 1.1 * u; const px = x + Math.cos(th) * rr, py = y + Math.sin(th) * rr; j ? g.lineTo(px, py) : g.moveTo(px, py); }
        g.stroke();
      }
      g.lineCap = "butt";
      D.disc(g, x, y, r * 0.24, "#7d8796"); D.disc(g, x, y, r * 0.09, "#2a313c");
    }
    function volute(g, t, open) {
      // The suction pipe in from the left, the discharge up from the top of the scroll.
      // Coolant stands in them, brighter while the pump moves it; a flange where each meets the casing.
      const water = s.phase === "start" && s.v > 0.05 ? "#4fa8f7" : "#2a5a86";
      D.pipe(g, [[[40, VOL.y + 30], [VOL.x - VOL.r + 10, VOL.y + 30]]], { w: 40, bore: 0.6, fluid: water, cap: "butt" });
      D.pipe(g, [[[VOL.x + 20, VOL.y - VOL.r + 10], [VOL.x + 20, 90]]], { w: 40, bore: 0.6, fluid: water, cap: "butt" });
      g.fillStyle = "#5a6a82"; g.fillRect(VOL.x - VOL.r - 6, VOL.y + 30 - 32, 14, 64); g.fillRect(VOL.x + 20 - 32, VOL.y - VOL.r - 6, 64, 14);
      // The scroll: a spiral, wider towards the discharge.
      g.beginPath();
      for (let j = 0; j <= 40; j++) { const u = j / 40, th = -Math.PI / 2 + u * Math.PI * 2, rr = VOL.r * (0.82 + 0.18 * u); const px = VOL.x + Math.cos(th) * rr, py = VOL.y + Math.sin(th) * rr; j ? g.lineTo(px, py) : g.moveTo(px, py); }
      g.closePath(); g.fillStyle = "#2a3446"; g.fill(); g.lineWidth = 4; g.strokeStyle = "#4a586d"; g.stroke();
      // The rotation arrow on the casing: clockwise.
      D.turnArrow(g, VOL.x, VOL.y, VOL.r - 14, -2.4, -1.0, C.amber);
      D.disc(g, VOL.x, VOL.y, WIN_R + 6, "#3a4658");
      D.disc(g, VOL.x, VOL.y, WIN_R, open ? "#070b12" : "#0d1520");
    }
    function machine(g, t) {
      const live = s.phase !== "part";
      volute(g, t, s.phase === "part");
      if (live) impeller(g, VOL.x, VOL.y, WIN_R - 6, s.spin, -1);
      else if (s.imp.seated) impeller(g, VOL.x, VOL.y, WIN_R - 6, 0, -1);
      else { g.setLineDash([8, 6]); D.ring(g, VOL.x, VOL.y, WIN_R - 6, C.amber, 3); g.setLineDash([]); D.disc(g, VOL.x, VOL.y, 12, "#7d8796"); }
      if (s.phase === "start" && s.bubbles > 0) {
        for (let j = 0; j < 14; j++) { const a = j * 2.4 + t * 3, rr = 14 + ((j * 7) % 40); D.ring(g, VOL.x + Math.cos(a) * rr, VOL.y + Math.sin(a) * rr, 3 + (j % 3), `rgba(232,246,255,${(0.8 * s.bubbles).toFixed(2)})`, 2); }
      }
      // The skid and the pump's pedestal.
      D.round(g, 90, BASE_Y, 870, 26, 6); g.fillStyle = "#2a3446"; g.fill();
      D.round(g, VOL.x - 70, VOL.y + VOL.r - 8, 140, BASE_Y - VOL.y - VOL.r + 10, 6); g.fillStyle = "#222c3a"; g.fill();
      // The pump's shaft to its coupling half.
      g.fillStyle = "#9aa6b6"; g.fillRect(VOL.x + WIN_R + 6, SHAFT_Y - 9, CPL_X - VOL.x - WIN_R - 26, 18);
      D.round(g, CPL_X - 34, SHAFT_Y - 34, 34, 68, 6); g.fillStyle = "#5a6a82"; g.fill();
      // The motor, tilted and lifted by its feet.
      const e0 = motorY(MOTOR.x0), e1 = motorY(MOTOR.x1), a = Math.atan2(e1 - e0, MOTOR.x1 - MOTOR.x0);
      g.save(); g.translate(MOTOR.x0, e0); g.rotate(a);
      g.fillStyle = "#5a6a82"; D.round(g, -36, -34, 34, 68, 6); g.fill();       // the motor's coupling half
      g.fillStyle = "#9aa6b6"; g.fillRect(-6, -9, 12, 18);
      D.round(g, 0, -MOTOR.h / 2, MOTOR.x1 - MOTOR.x0, MOTOR.h, 16); g.fillStyle = "#33405a"; g.fill(); g.lineWidth = 3; g.strokeStyle = "#5a6a82"; g.stroke();
      for (let x = 30; x < MOTOR.x1 - MOTOR.x0 - 20; x += 22) { g.fillStyle = "#283247"; g.fillRect(x, -MOTOR.h / 2 + 8, 10, MOTOR.h - 16); }
      g.restore();
      // The feet, the shims under them, and the handles that move them.
      FOOT.forEach((fx, k) => {
        const fy = motorY(fx) + MOTOR.h / 2, sel = s.phase === "align" && (s.keys ? s.sel === k : s.grab === k);
        g.fillStyle = "#4a586d"; g.fillRect(fx - 40, fy - 4, 80, 18);
        const shim = BASE_Y - (fy + 14);
        if (shim > 0) { g.fillStyle = C.copper; g.fillRect(fx - 34, fy + 14, 68, shim); }
        if (s.phase !== "align") return;
        const ky = TRACK.y - s.h[k] * KNOB;
        D.round(g, fx - 9, TRACK.y - TRACK.half, 18, TRACK.half * 2, 9); g.fillStyle = "#141c28"; g.fill();
        D.round(g, fx - 46, ky - 22, 92, 44, 12); g.fillStyle = sel ? "#2a3a52" : "#1f2a3a"; g.fill();
        g.lineWidth = 3; g.strokeStyle = sel ? C.amber : "#5a6a82"; g.stroke();
        g.strokeStyle = C.fg; g.lineWidth = 3;
        g.beginPath(); g.moveTo(fx - 10, ky - 6); g.lineTo(fx, ky - 14); g.lineTo(fx + 10, ky - 6); g.moveTo(fx - 10, ky + 6); g.lineTo(fx, ky + 14); g.lineTo(fx + 10, ky + 6); g.stroke();
      });
    }
    function targets(g, t) {
      const d = dots(), rr = ringR();
      // The beam from the pump's coupling to the targets.
      g.strokeStyle = "rgba(255,71,87,0.35)"; g.lineWidth = 2; g.setLineDash([6, 6]);
      g.beginPath(); g.moveTo(CPL_X, SHAFT_Y); g.lineTo(TGT[1].x, TGT[1].y); g.stroke(); g.setLineDash([]);
      TGT.forEach((tg, k) => {
        D.disc(g, tg.x, tg.y, PLATE_R + 6, "#0c121a");
        D.disc(g, tg.x, tg.y, PLATE_R, "#1a2230"); D.ring(g, tg.x, tg.y, PLATE_R, s.off[k] > 0 ? C.danger : "#3a4658", 3);
        g.strokeStyle = "#3a4658"; g.lineWidth = 1.5;
        g.beginPath(); g.moveTo(tg.x - PLATE_R, tg.y); g.lineTo(tg.x + PLATE_R, tg.y); g.moveTo(tg.x, tg.y - PLATE_R); g.lineTo(tg.x, tg.y + PLATE_R); g.stroke();
        const inRing = Math.abs(d[k]) <= rr;
        D.ring(g, tg.x, tg.y, rr, inRing ? C.ok : "rgba(232,238,246,0.6)", 3);
        const dy = clamp(d[k], -PLATE_R - 14, PLATE_R + 14);
        D.disc(g, tg.x, tg.y - dy, 11, "rgba(255,71,87,0.35)"); D.disc(g, tg.x, tg.y - dy, 6, "#ff4757");
        D.text(g, k ? "FAR" : "NEAR", tg.x, tg.y + PLATE_R + 26, 18, C.dim, "center", 700);
      });
      if (s.hold > 0) { g.beginPath(); g.arc(1085, 120, 16, -Math.PI / 2, -Math.PI / 2 + (Math.PI * 2 * s.hold) / HOLD_S); g.strokeStyle = C.ok; g.lineWidth = 5; g.stroke(); }
    }
    function startPanel(g, t) {
      // The suction gauge: green above the red band, red (hatched) at the bottom of its sweep.
      const G = GAUGE, ang = (v) => A0 + SWEEP * clamp(v, 0, 1);
      D.disc(g, G.x, G.y, G.r + 10, "#0c121a"); D.disc(g, G.x, G.y, G.r, "#141c28");
      g.lineWidth = 16;
      g.beginPath(); g.arc(G.x, G.y, G.r - 16, ang(red()), ang(1)); g.strokeStyle = "rgba(61,220,132,0.45)"; g.stroke();
      D.hatchArc(g, G.x, G.y, G.r - 24, G.r - 8, ang(0), ang(red()), C.danger);
      const na = ang(s.p);
      g.strokeStyle = s.p < red() ? C.danger : C.fg; g.lineWidth = 5; g.lineCap = "round";
      g.beginPath(); g.moveTo(G.x, G.y); g.lineTo(G.x + Math.cos(na) * (G.r - 22), G.y + Math.sin(na) * (G.r - 22)); g.stroke(); g.lineCap = "butt";
      D.disc(g, G.x, G.y, 10, "#5a6a82");
      D.text(g, "SUCTION", G.x, G.y + G.r + 30, 20, C.dim, "center", 700);
      // The speed lever: the biggest control. The band at 100% is green.
      const L = LEVER;
      D.round(g, L.x - 18, L.y1 - 10, 36, L.y0 - L.y1 + 20, 18); g.fillStyle = "#141c28"; g.fill();
      const b0 = leverY(BAND[1]), b1 = leverY(BAND[0]);
      g.fillStyle = "rgba(61,220,132,0.35)"; g.fillRect(L.x - L.w / 2 - 14, b0, L.w + 28, b1 - b0);
      g.strokeStyle = C.ok; g.lineWidth = 2; g.strokeRect(L.x - L.w / 2 - 14, b0, L.w + 28, b1 - b0);
      const ky = leverY(s.v), on = s.grab || s.keys;
      D.round(g, L.x - L.w / 2, ky - 30, L.w, 60, 14); g.fillStyle = on ? "#2a3a52" : "#243045"; g.fill();
      g.lineWidth = 3; g.strokeStyle = on ? C.amber : "#7d8796"; g.stroke();
      g.strokeStyle = C.fg; g.lineWidth = 4; g.beginPath(); g.moveTo(L.x - 26, ky); g.lineTo(L.x + 26, ky); g.stroke();
      D.text(g, "SPEED", L.x, L.y0 + 40, 20, C.dim, "center", 700);
      if (s.hold > 0) { g.beginPath(); g.arc(L.x, L.y1 - 40, 16, -Math.PI / 2, -Math.PI / 2 + (Math.PI * 2 * s.hold) / HOLD_RUN_S); g.strokeStyle = C.ok; g.lineWidth = 5; g.stroke(); }
    }
    function partPanel(g, t) {
      D.panel(g, CRATE.x, CRATE.y, CRATE.w, CRATE.h, 12, "#141c28", "#3a4658");
      const m = s.imp;
      if (!m.seated) {
        impeller(g, m.x, m.y, IMP_R, 0, m.sweep);
        // Tap to turn it over: a mirror mark under it (a line with a head at each end), not a turning arrow.
        if (!m.held) {
          const cx = CRATE.x + CRATE.w / 2, y = CRATE.y + CRATE.h - 16;
          g.strokeStyle = "rgba(232,238,246,0.6)"; g.lineWidth = 3; g.beginPath(); g.moveTo(cx - 34, y); g.lineTo(cx + 34, y); g.stroke();
          g.fillStyle = "rgba(232,238,246,0.6)";
          for (const d of [-1, 1]) { g.beginPath(); g.moveTo(cx + d * 44, y); g.lineTo(cx + d * 32, y - 7); g.lineTo(cx + d * 32, y + 7); g.closePath(); g.fill(); }
        }
      }
    }

    return {
      get state() { return s; },
      step,
      /** For tools (shots and tests): the round's state, read only. */
      peek() {
        const base = { phase: s.phase, index: s.index, done: !!s.done };
        if (s.phase === "part") return { ...base, imp: { x: s.imp.x, y: s.imp.y, sweep: s.imp.sweep, seated: s.imp.seated }, shaft: [VOL.x, VOL.y], crate: [CRATE.x + CRATE.w / 2, CRATE.y + CRATE.h / 2] };
        if (s.phase === "align") return { ...base, h: s.h.slice(), want: s.want.slice(), dots: dots(), ring: ringR(), plate: PLATE_R, feet: FOOT.map((x, k) => [x, TRACK.y - s.h[k] * KNOB]), knob: KNOB, pull: PULL, dotK: DOT_K };
        return { ...base, v: s.v, p: s.p, red: red(), band: BAND, lever: [LEVER.x, leverY(s.v)], y0: LEVER.y0, y1: LEVER.y1, vMax: V_MAX, hold: s.hold };
      },
      update(dt, input) {
        s.t += dt; s.knock = Math.max(0, s.knock - dt * 2);
        if (s.phase === "part") partUpdate(dt, input);
        else if (s.phase === "align") alignUpdate(dt, input);
        else startUpdate(dt, input);
      },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        const shake = s.knock > 0 ? Math.sin(t * 60) * 4 * s.knock : 0;
        g.save(); g.translate(shake, 0);
        machine(g, t);
        g.restore();
        if (s.phase === "align") targets(g, t);
        else if (s.phase === "start") startPanel(g, t);
        else partPanel(g, t);
      },
    };
  },
});
