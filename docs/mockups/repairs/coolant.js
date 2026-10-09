/*
 * repairs/coolant.js: the coolant balance, the reactor operator's duty (openspec/changes/reactor-cooling, design 2, 3
 * and 5; the loop's numbers are power-grid 10).
 *
 * Not a repair: the engineer's hand on the loop when the automation's is not good enough. The game proper has no steps
 * (it is the Engineering console's E4 panel); the mockup plays it as a job so it can be tried alone, one reactor state a
 * step: cruise, a combat surge to 100%, a core pump lost (damaged); a leak, overdrive at 120%, a radiator pump down, a
 * surge (disabled, after its part step fits core pump B's new cartridge).
 *
 * The picture: the loop as a ring, the core at the top and the chiller (the heat exchanger) at the bottom, the cold leg
 * up the left side (blue, dots) and the hot leg down the right (red to orange with its temperature, chevrons), the two
 * tanks and the loop's inventory at the left, the radiators beyond the chiller. Two gauges: the legs' temperatures (two
 * needles against their bands, the warning and the scram), and the core's heat against what the flow carries at the
 * design's 15 K rise. Four levers: core pump speed (the biggest), the chiller valve (how much of the hot leg goes through
 * the exchanger; the rest bypasses it), the radiator pumps' speed, and the makeup valve from the tanks. Exact values on
 * hover, or while a lever is held.
 *
 * The goal, whatever the reactor does: the hot leg 335-350 K, the cold leg 320-335 K and the loop over 80% full. Held in
 * band, the hold ring round the core fills; it is sized to the step's share at the repairer's rate (as gravity.js), so
 * play and the bar end together. The hot leg past 370 K is the fumble "Loop over-temperature warning"; past 380 K for
 * 2 s the reactor scrams (a second fumble) and restarts.
 *
 * The physics, simplified but honest (one lumped loop): heat in is 40 MW at full throttle; the hot leg is the cold leg
 * plus heat / (flow x 3.6 kJ/(kg K)), followed with a short lag; the radiators reject
 * 40 MW x chiller valve x radiator pumps x (T_hot^4 - 4^4) / (330^4 - 4^4); the cold leg moves by (heat in - rejected) /
 * (3.6 kJ/(kg K) x the loop's kilograms). Flow is the pumps' speed x their capability, cut by cavitation under 80%
 * inventory. A leak drains the loop; the makeup valve refills it from the tanks. The mockup runs the loop TIME_X ship
 * seconds a played second so a step's drift shows inside its share; the console runs at 1.
 *
 * Keys: Left and Right pick a lever, Up and Down move it.
 */
RepairKit.register({
  id: "coolant",
  title: "Coolant balance",
  place: "Engineering bench",
  group: "Engineering",
  hazard: "Loop over-temperature warning",
  // The how-to card (repair-minigames 6g), drawn by the kit: pictures and a few words, on demand.
  guide: {
    steps: [
      { icon: "band", text: "Both legs in their bands" },
      { icon: "slider", text: "Pump lever: more flow" },
      { icon: "slider", text: "Chiller, radiators: more cooling" },
      { icon: "valve", text: "Makeup keeps the loop full" },
    ],
    mistake: "Hot leg past 370 K: an over-temperature warning",
  },
  down: "The reactor runs hot and scrams; the ship falls back on its batteries (power-grid 3)",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const WARN_TEXT = "Loop over-temperature warning";
    const SCRAM_TEXT = "Scram: the loop tripped the reactor";

    // ---------------------------------------------------------------- the loop (reactor-cooling 2-3, power-grid 2, 10)
    const CP = 3.6e-3;               // MJ/(kg K): water and glycol
    const M_FULL = 11100;            // kg in the loop when full (40 MJ/K / CP)
    const TANK_KG = 2000;            // kg of reserve in each tank
    const FLOW_FULL = 740;           // kg/s at full flow: two core pumps of half the flow each
    const HEAT_FULL_MW = 40;         // MW into the loop at 100% throttle: a 15 K rise at full flow
    const RAD_MW = 40;               // MW the radiators reject at T_RAD, full health and radiator pumps
    const T_RAD = 330, T_SPACE = 4;  // K
    const LEAK_KG_S = 20;            // kg/s from a segment at 0% integrity
    const LEAK_FROM = 75;            // % integrity under which a segment leaks
    const MAKEUP_KG_S = 10;          // kg/s through the makeup valve, full open
    const CAV_LO = 0.6, CAV_HI = 0.8;   // inventory: no flow at 60%, full flow at 80% and over (cavitation)
    const NATURAL = 0.05;            // natural circulation with the pumps stopped (reactor-cooling 6)
    const SPEED_MAX = 1.2;           // pump speed at overdrive (reactor-cooling 6: 0-120%)
    const RAMP_UP = 0.02, RAMP_DOWN = 0.10;   // throttle a ship second (power-grid 2)
    const HOT_TAU_S = 6;             // ship s: the hot leg follows the core outlet with this lag (the core's own water)
    const RISE_DESIGN_K = 15;        // the rise at full flow and full heat: the "carries" needle's scale
    const HOT_BAND = [335, 350], COLD_BAND = [320, 335], INV_MIN = 0.8;
    const WARN_K = 370, WARN_REARM_K = 362, SCRAM_K = 380, SCRAM_HOLD_S = 2;   // K, K, K, ship s
    const SCRAM_RESTART_S = 3;       // played s the reactor sits at 10% after a scram before it ramps back
    const TIME_X = 6;                // ship s a played s (the mockup only; the console runs at 1)
    const TICK = 1 / 60;             // played s: the loop steps on a fixed tick, so a run is the same at any frame rate
    // The hold: a share of the step's time at the repairer's rate (as gravity.js), and what a fumble knocks off it.
    const HOLD_SHARE = 0.6, HOLD_MIN_S = 4, HOLD_MAX_S = 9, FUMBLE_HOLD_S = 1.5;
    const LEVER_RATE = 0.6;          // a lever's travel a second from the keys (0 to 100% in 1.7 s)

    // The reactor's states, one a step. Throttle 0-1.2; pumpB / radB: that pump's capability; seg: the leaking
    // segment's integrity, %.
    const STATES = {
      cruise: { name: "CRUISE", throttle: 0.6 },
      surge: { name: "SURGE", throttle: 1.0 },
      overdrive: { name: "OVERDRIVE", throttle: 1.2 },
      pump: { name: "PUMP B LOST", throttle: 0.85, pumpB: 0 },
      leak: { name: "LEAK", throttle: 0.6, seg: 50 },
      rad: { name: "RAD PUMP DOWN", throttle: 0.7, radB: 0 },
    };
    const ORDER = { damaged: ["cruise", "surge", "pump"], disabled: ["leak", "overdrive", "rad", "surge"] };

    // ---------------------------------------------------------------- the screen (canvas px)
    const RING = { x: 380, y: 400, r: 186 };
    const CORE = { x: RING.x, y: RING.y - RING.r, r: 48 };
    const CHILL = { x: RING.x, y: RING.y + RING.r, w: 132, h: 58 };
    const RADS = { x: 236, y: 654, w: 288, h: 50 };
    const TANKS = [{ x: 34, y: 300, w: 40, h: 190 }, { x: 84, y: 300, w: 40, h: 190 }];
    const LOOPBAR = { x: 34, y: 528, w: 90, h: 26 };
    const PUMPS = [205, 228].map((deg) => ({ a: (deg * Math.PI) / 180 }));   // on the cold leg, upper left
    const LEAK_A = (25 * Math.PI) / 180;                                     // the leaking segment, on the hot leg
    const TDIAL = { x: 768, y: 256, r: 104 };
    const HDIAL = { x: 768, y: 548, r: 92 };
    const T_LO = 300, T_HI = 390;                      // the temperature dial's scale, K
    const H_HI = 60;                                   // the heat dial's scale, MW
    const DIAL_A0 = Math.PI * 0.8, DIAL_A1 = Math.PI * 2.2;
    const LEVERS = [
      { key: "pump", word: "PUMPS", x: 916, y: 168, w: 88, h: 392, max: SPEED_MAX },
      { key: "chill", word: "CHILLER", x: 1030, y: 168, w: 60, h: 392, max: 1 },
      { key: "rad", word: "RADS", x: 1112, y: 168, w: 60, h: 392, max: SPEED_MAX },
      { key: "makeup", word: "MAKEUP", x: 1194, y: 168, w: 60, h: 392, max: 1 },
    ];
    const HIT = 26;                                    // px round each lever a finger still finds (over KIT.TOUCH_R with the lever's half width)
    const CRATE = { x: 30, y: 596, w: 170, h: 108 };   // the part step's crate

    // ---------------------------------------------------------------- state
    let s = null, sim = null, clock = 0, hadPart = false, lastIndex = -1;
    const shares = {};
    const log = [];
    const clamp = (x, a, b) => Math.max(a, Math.min(b, x));
    const lerp = (a, b, u) => a + (b - a) * u;

    function freshLoop(kind) {
      return {
        Tc: 338, Th: 347, inv: kind === "leak" ? 0.84 : 0.97, tanks: kind === "leak" ? [1500, 1400] : [TANK_KG, TANK_KG],
        throttle: STATES[kind].throttle, ctrl: { pump: 1.0, chill: 0.45, rad: 1.0, makeup: 0 },
        scramT: 0, restartT: 0, armed: true, acc: 0,
      };
    }
    /** The loop's numbers now, from its state and the reactor state's damage. */
    function derive(L, st) {
      const Q = HEAT_FULL_MW * L.throttle;
      const pumps = L.ctrl.pump * (1 + (st.pumpB ?? 1)) / 2;
      const cav = clamp((L.inv - CAV_LO) / (CAV_HI - CAV_LO), 0, 1);
      const flow = Math.max(NATURAL, pumps * cav);
      const radP = L.ctrl.rad * (1 + (st.radB ?? 1)) / 2;
      const rejK = RAD_MW * L.ctrl.chill * radP / (T_RAD ** 4 - T_SPACE ** 4);
      const rej = rejK * (L.Th ** 4 - T_SPACE ** 4);
      const leak = st.seg !== undefined && st.seg < LEAK_FROM ? (LEAK_KG_S * (LEAK_FROM - st.seg)) / LEAK_FROM : 0;
      const carry = flow * FLOW_FULL * CP * RISE_DESIGN_K;
      return { Q, pumps, cav, flow, radP, rej, leak, carry, rise: Q / (flow * FLOW_FULL * CP) };
    }
    const inBand = (L) => L.Th >= HOT_BAND[0] && L.Th <= HOT_BAND[1] && L.Tc >= COLD_BAND[0] && L.Tc <= COLD_BAND[1] && L.inv >= INV_MIN;

    function start(index, isPart) {
      if (isPart) hadPart = true;
      if (shares[index] === undefined && !isPart) {
        // The step's share of the job in seconds at this repairer's rate (design 1): (target - now) / steps left / rate.
        shares[index] = (100 - api.value) / Math.max(1, api.steps - index) / (KIT.RATES[api.who] || KIT.RATES.officer);
      }
      const order = hadPart ? ORDER.disabled : ORDER.damaged;
      const round = index - (hadPart ? 1 : 0);
      const kind = isPart ? order[0] : order[Math.min(round, order.length - 1)];
      // A restarted step (three fumbles) restarts the loop too: the reactor came back from a scram.
      if (!sim || index === lastIndex) sim = freshLoop(kind);
      if (index === lastIndex && !isPart) { sim.Tc = 330; sim.Th = 338; sim.throttle = 0.1; sim.restartT = SCRAM_RESTART_S; }
      lastIndex = index;
      s = {
        index, round, kind, part: isPart, phase: isPart ? "part" : "play", hold: 0,
        holdNeed: isPart ? 0 : clamp(HOLD_SHARE * shares[index], HOLD_MIN_S, HOLD_MAX_S),
        grab: null, focus: 0, keys: false, flash: 0,
        cart: { x: CRATE.x + CRATE.w / 2, y: CRATE.y + CRATE.h / 2 + 4, held: false, pad: false, set: !isPart, dx: 0, dy: 0 },
      };
      log.push({ index, kind, part: isPart, start: clock, inBand: null, played: null, holdNeed: s.holdNeed, share: shares[index] ?? null });
    }

    // ---------------------------------------------------------------- the step
    function physics(dt) {
      const L = sim, st = STATES[s.kind], ds = dt * TIME_X;
      // The reactor: its throttle ramps to the state's (power-grid 2), or sits at 10% while it restarts from a scram.
      L.restartT = Math.max(0, L.restartT - dt);
      const want = L.restartT > 0 ? 0.1 : st.throttle;
      L.throttle = want > L.throttle ? Math.min(want, L.throttle + RAMP_UP * ds) : Math.max(want, L.throttle - RAMP_DOWN * ds);
      const d = derive(L, st);
      // The hot leg follows the core outlet; the cold leg is the loop's heat balance over its heat capacity.
      L.Th += (L.Tc + d.rise - L.Th) * Math.min(1, ds / HOT_TAU_S);
      L.Tc += ((d.Q - d.rej) / (CP * L.inv * M_FULL)) * ds;
      // Inventory: the leak out, the makeup in from the tanks (both evenly), never over full.
      let m = L.inv * M_FULL - d.leak * ds;
      const room = Math.max(0, M_FULL - m), have = L.tanks[0] + L.tanks[1];
      const add = Math.min(MAKEUP_KG_S * L.ctrl.makeup * ds, room, have);
      if (add > 0) { const k = add / have; L.tanks = L.tanks.map((x) => x - x * k); m += add; }
      L.inv = clamp(m / M_FULL, 0, 1);
      // The warning (the fumble) and the scram.
      if (L.Th < WARN_REARM_K) L.armed = true;
      if (L.Th > WARN_K && L.armed) { L.armed = false; s.hold = Math.max(0, s.hold - FUMBLE_HOLD_S); s.flash = 1; api.fumble(WARN_TEXT); }
      L.scramT = L.Th > SCRAM_K ? L.scramT + ds : 0;
      if (L.scramT > SCRAM_HOLD_S && L.restartT <= 0) {
        L.scramT = 0; L.restartT = SCRAM_RESTART_S; L.throttle = 0.1; s.hold = Math.max(0, s.hold - FUMBLE_HOLD_S);
        api.fumble(SCRAM_TEXT);
      }
    }

    function leverAt(x, y) {
      return KIT.nearest(LEVERS.map((l) => (x >= l.x - HIT && x <= l.x + l.w + HIT && y >= l.y - HIT && y <= l.y + l.h + HIT ? [l.x + l.w / 2, clamp(y, l.y, l.y + l.h)] : null)), x, y, 200);
    }
    const valueAt = (l, y) => clamp(((l.y + l.h - 20 - y) / (l.h - 40)) * l.max, 0, l.max);
    const leverY = (l, v) => l.y + l.h - 20 - (v / l.max) * (l.h - 40);

    function play(dt, input) {
      // The levers: a press picks the nearest, a drag sets it; the keys pick and move the focused one.
      for (const [code, d] of [["ArrowLeft", -1], ["KeyA", -1], ["ArrowRight", 1], ["KeyD", 1], ["Tab", 1]]) {
        if (input.hit.has(code)) { s.keys = true; s.focus = (s.focus + d + LEVERS.length) % LEVERS.length; }
      }
      if (input.stick.y) { s.keys = true; const l = LEVERS[s.focus]; sim.ctrl[l.key] = clamp(sim.ctrl[l.key] - input.stick.y * LEVER_RATE * l.max * dt, 0, l.max); }
      if (input.pressed) { const i = leverAt(input.x, input.y); if (i >= 0) { s.grab = i; s.focus = i; s.keys = false; } }
      if (s.grab !== null && input.down) { const l = LEVERS[s.grab]; sim.ctrl[l.key] = valueAt(l, input.y); }
      if (!input.down) s.grab = null;
      // The loop, on a fixed tick.
      sim.acc += dt;
      while (sim.acc >= TICK) { sim.acc -= TICK; physics(TICK); }
      if (s.phase === "play" && inBand(sim)) {
        if (log[log.length - 1].inBand === null) log[log.length - 1].inBand = clock;
        s.hold = Math.min(s.holdNeed, s.hold + dt);
        if (s.hold >= s.holdNeed) { s.phase = "held"; log[log.length - 1].played = clock; api.stepDone(); }
      }
    }

    function partStep(dt, input) {
      const p = s.cart, slot = pumpAt(1);
      if (input.stick.x || input.stick.y) { p.x += input.stick.x * 480 * dt; p.y += input.stick.y * 480 * dt; p.pad = true; }
      if (input.pressed && Math.hypot(input.x - p.x, input.y - p.y) < 60) { p.held = true; p.dx = p.x - input.x; p.dy = p.y - input.y; }
      if (p.held && input.down) { p.x = input.x + p.dx; p.y = input.y + p.dy; }
      if ((p.held && input.released) || (p.pad && input.actionPressed)) {
        p.held = false; p.pad = false;
        if (Math.hypot(p.x - slot[0], p.y - slot[1]) < 60) { p.set = true; p.x = slot[0]; p.y = slot[1]; s.phase = "fitted"; log[log.length - 1].played = clock; api.stepDone(); }
      }
      p.x = clamp(p.x, 30, api.W - 30); p.y = clamp(p.y, api.BAR_H + 30, api.H - 30);
    }

    // ---------------------------------------------------------------- drawing
    const ringPt = (a, r = RING.r) => [RING.x + Math.cos(a) * r, RING.y + Math.sin(a) * r];
    const pumpAt = (i) => ringPt(PUMPS[i].a);
    /** The hot leg's colour at a temperature: red at its band's top and over, orange through it, amber below. */
    function hotCol(T) {
      const u = clamp((T - 330) / 30, 0, 1);
      return `rgb(${Math.round(lerp(242, 255, u))},${Math.round(lerp(160, 71, u))},${Math.round(lerp(70, 87, u))})`;
    }
    const COLD = "#4fa8f7";
    /** An arc of pipe: steel casing, the coolant, and a pattern running the flow's way (hot: chevrons; cold: dots). */
    function legArc(g, a0, a1, col, kind, flow, t, dim) {
      g.beginPath(); g.arc(RING.x, RING.y, RING.r, a0, a1, a1 < a0);
      g.strokeStyle = "#263142"; g.lineWidth = 34; g.stroke();
      g.strokeStyle = dim ? "#1b2433" : col; g.globalAlpha = dim ? 1 : 0.55; g.lineWidth = 22; g.stroke(); g.globalAlpha = 1;
      if (dim) return;
      const dir = a1 > a0 ? 1 : -1, span = Math.abs(a1 - a0), step = 0.16, off = ((t * flow * 0.5) % step + step) % step;
      for (let a = off; a < span; a += step) {
        const th = a0 + dir * a, [x, y] = ringPt(th), tang = th + (dir * Math.PI) / 2;
        g.save(); g.translate(x, y); g.rotate(tang);
        if (kind === "hot") {
          g.beginPath(); g.moveTo(-5, -8); g.lineTo(4, 0); g.lineTo(-5, 8);
          g.strokeStyle = "#fff3e0"; g.lineWidth = 3.5; g.lineJoin = "round"; g.stroke();
        } else D.disc(g, 0, 0, 4, "#e8f6ff");
        g.restore();
      }
    }
    function pumpGlyph(g, x, y, on, dead, t, speed) {
      D.disc(g, x, y, 25, "#1b2433"); D.ring(g, x, y, 25, dead ? C.danger : "#8796aa", 3);
      g.save(); g.translate(x, y); g.rotate(on && !dead ? t * 8 * speed : 0.4);
      g.strokeStyle = dead ? "#5a3036" : "#c9d3e0"; g.lineWidth = 4;
      for (let k = 0; k < 4; k++) { g.beginPath(); g.moveTo(0, 0); g.quadraticCurveTo(7, -6, 15, -4); g.stroke(); g.rotate(Math.PI / 2); }
      g.restore();
      D.disc(g, x, y, 5, "#8796aa");
      if (dead) { D.hatch(g, x - 18, y - 18, 36, 36, "rgba(255,71,87,0.5)"); g.strokeStyle = C.danger; g.lineWidth = 4; g.beginPath(); g.moveTo(x - 12, y - 12); g.lineTo(x + 12, y + 12); g.moveTo(x + 12, y - 12); g.lineTo(x - 12, y + 12); g.stroke(); }
    }
    function drawLoop(g, t, d, st) {
      const L = sim, live = s.phase !== "part";
      // The chiller's bypass: a chord over the exchanger; its width is the share that skips it.
      const by = CHILL.y - 46, bx0 = RING.x + 112, bx1 = RING.x - 112;
      g.beginPath(); g.moveTo(...ringPt(Math.PI / 2 - 0.62)); g.lineTo(bx0 - 30, by); g.lineTo(bx1 + 30, by); g.lineTo(...ringPt(Math.PI / 2 + 0.62));
      g.strokeStyle = "#263142"; g.lineWidth = 14; g.lineJoin = "round"; g.stroke();
      const bysh = live ? 1 - L.ctrl.chill : 0;
      if (bysh > 0.02) { g.strokeStyle = hotCol(L.Th); g.globalAlpha = 0.6; g.lineWidth = 3 + 8 * bysh; g.stroke(); g.globalAlpha = 1; }
      // The legs: hot down the right (core to chiller), cold up the left (chiller to core).
      const fl = live ? d.flow * 10 : 0;
      legArc(g, -Math.PI / 2 + 0.26, Math.PI / 2 - 0.36, hotCol(L.Th), "hot", fl, t, !live);
      legArc(g, Math.PI / 2 + 0.36, Math.PI * 1.5 - 0.26, COLD, "cold", fl, t, !live);
      // The leak: a cracked joint on the hot leg, dripping and steaming.
      if (d.leak > 0 && live) {
        const [x, y] = ringPt(LEAK_A, RING.r + 14);
        g.strokeStyle = C.danger; g.lineWidth = 3; g.beginPath(); g.moveTo(x - 10, y - 12); g.lineTo(x - 2, y - 2); g.lineTo(x - 8, y + 6); g.lineTo(x + 2, y + 14); g.stroke();
        for (let j = 0; j < 6; j++) { const ph = (t * 1.6 + j / 6) % 1; D.disc(g, x + 12 + j * 3, y + 10 + ph * 70, 4 - 2 * ph, `rgba(255,170,120,${0.9 - 0.7 * ph})`); }
        for (let j = 0; j < 4; j++) { const ph = (t * 0.5 + j / 4) % 1; D.disc(g, x + 30 + Math.sin(t + j) * 8, y - ph * 60, 8 + ph * 14, `rgba(220,230,240,${0.12 * (1 - ph)})`); }
      }
      // The makeup line: the tanks into the cold leg.
      const [mx, my] = ringPt(Math.PI);
      g.strokeStyle = "#263142"; g.lineWidth = 12; g.beginPath(); g.moveTo(TANKS[1].x + TANKS[1].w, my); g.lineTo(mx - 16, my); g.stroke();
      if (live && L.ctrl.makeup > 0.02 && L.inv < 1 && L.tanks[0] + L.tanks[1] > 1) {
        g.strokeStyle = COLD; g.lineWidth = 2 + 5 * L.ctrl.makeup; g.stroke();
        for (let j = 0; j < 3; j++) { const ph = (t * 1.2 + j / 3) % 1; D.disc(g, lerp(TANKS[1].x + TANKS[1].w, mx - 16, ph), my, 3.5, "#e8f6ff"); }
      }
      // The core: its vessel, its heat as a glow, the throttle as an arc, the hold ring round it.
      const glow = live ? 0.35 + 0.55 * clamp(L.throttle / 1.2, 0, 1) : 0.1;
      const rg = g.createRadialGradient(CORE.x, CORE.y, 0, CORE.x, CORE.y, CORE.r * 1.8);
      rg.addColorStop(0, `rgba(255,240,220,${glow})`); rg.addColorStop(0.4, `rgba(190,159,230,${glow * 0.6})`); rg.addColorStop(1, "rgba(190,159,230,0)");
      D.disc(g, CORE.x, CORE.y, CORE.r * 1.8, rg);
      D.disc(g, CORE.x, CORE.y, CORE.r, "#141c28"); D.ring(g, CORE.x, CORE.y, CORE.r, "#4a5568", 4);
      D.disc(g, CORE.x, CORE.y, CORE.r * 0.55, `rgba(240,230,255,${0.25 + glow * 0.7})`);
      if (live) {
        g.beginPath(); g.arc(CORE.x, CORE.y, CORE.r - 9, Math.PI * 0.75, Math.PI * 0.75 + Math.PI * 1.5 * clamp(L.throttle / 1.2, 0, 1));
        g.strokeStyle = L.throttle > 1.001 ? C.amber : "#be9fe6"; g.lineWidth = 5; g.stroke();
        // The hold: a ring round the core, green while every needle is in its band.
        const held = inBand(L);
        D.ring(g, CORE.x, CORE.y, CORE.r + 20, "#1d2636", 9);
        g.beginPath(); g.arc(CORE.x, CORE.y, CORE.r + 20, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * (s.holdNeed ? s.hold / s.holdNeed : 0));
        g.strokeStyle = s.phase === "held" ? C.ok : held ? C.ok : C.amber; g.lineWidth = 9; g.lineCap = "round"; g.stroke(); g.lineCap = "butt";
      }
      // The pumps on the cold leg (B may be lost; on the part step its housing is open).
      for (let i = 0; i < 2; i++) {
        const [x, y] = pumpAt(i);
        if (s.phase === "part" && i === 1 && !s.cart.set) {
          D.disc(g, x, y, 30, "#120d08"); D.ring(g, x, y, 30, C.amber, 3); g.setLineDash([6, 5]); D.ring(g, x, y, 40 + 3 * Math.sin(t * 5), C.amber, 2); g.setLineDash([]);
          continue;
        }
        const cap = i === 0 ? 1 : (st.pumpB ?? 1);
        pumpGlyph(g, x, y, live, live && cap <= 0, t, L.ctrl.pump);
      }
      // The chiller: plates of the exchanger, cooler where the valve lets more through.
      D.panel(g, CHILL.x - CHILL.w / 2, CHILL.y - CHILL.h / 2, CHILL.w, CHILL.h, 10, "#152030", "#4a5a70");
      for (let k = 0; k < 9; k++) {
        const x = CHILL.x - CHILL.w / 2 + 14 + k * 13;
        g.strokeStyle = live ? `rgba(79,168,247,${0.3 + 0.6 * L.ctrl.chill})` : "#2a3446"; g.lineWidth = 4;
        g.beginPath(); g.moveTo(x, CHILL.y - 19); g.lineTo(x + 5, CHILL.y - 8); g.lineTo(x, CHILL.y + 3); g.lineTo(x + 5, CHILL.y + 14); g.stroke();
      }
      // The radiator loop: down from the chiller to the fins; the fins glow with what they reject.
      g.strokeStyle = "#263142"; g.lineWidth = 10;
      g.beginPath(); g.moveTo(CHILL.x - 40, CHILL.y + CHILL.h / 2); g.lineTo(CHILL.x - 40, RADS.y); g.moveTo(CHILL.x + 40, CHILL.y + CHILL.h / 2); g.lineTo(CHILL.x + 40, RADS.y); g.stroke();
      const rk = live ? clamp(d.rej / 50, 0, 1) : 0;
      for (let k = 0; k < 18; k++) {
        const x = RADS.x + 6 + k * 16, dead = (st.radB ?? 1) <= 0 && k >= 9 && live;
        g.fillStyle = dead ? "#1b2230" : `rgba(${Math.round(lerp(60, 255, rk))},${Math.round(lerp(70, 120, rk))},${Math.round(lerp(90, 80, rk))},1)`;
        g.fillRect(x, RADS.y, 9, RADS.h);
        if (dead) D.hatch(g, x, RADS.y, 9, RADS.h, "rgba(255,71,87,0.45)");
      }
      for (let i = 0; i < 2; i++) {
        const x = CHILL.x + (i ? 40 : -40), y = RADS.y - 26, dead = live && i === 1 && (st.radB ?? 1) <= 0;
        D.disc(g, x, y, 15, "#1b2433"); D.ring(g, x, y, 15, dead ? C.danger : "#8796aa", 2.5);
        g.save(); g.translate(x, y); g.rotate(live && !dead ? t * 6 * L.ctrl.rad : 0.3); g.strokeStyle = dead ? "#5a3036" : "#c9d3e0"; g.lineWidth = 3;
        for (let k = 0; k < 3; k++) { g.beginPath(); g.moveTo(0, 0); g.lineTo(10, -3); g.stroke(); g.rotate((Math.PI * 2) / 3); }
        g.restore();
        if (dead) { g.strokeStyle = C.danger; g.lineWidth = 3; g.beginPath(); g.moveTo(x - 9, y - 9); g.lineTo(x + 9, y + 9); g.moveTo(x + 9, y - 9); g.lineTo(x - 9, y + 9); g.stroke(); }
      }
      // The tanks and the loop's inventory: levels, the loop's 80% line, hatched under it.
      TANKS.forEach((tk, i) => {
        D.round(g, tk.x, tk.y, tk.w, tk.h, 14); g.fillStyle = "#121a25"; g.fill(); g.lineWidth = 2; g.strokeStyle = "#3a4658"; g.stroke();
        const k = L.tanks[i] / TANK_KG, h = (tk.h - 8) * k;
        g.save(); D.round(g, tk.x + 4, tk.y + 4, tk.w - 8, tk.h - 8, 10); g.clip();
        g.fillStyle = "rgba(79,168,247,0.7)"; g.fillRect(tk.x + 4, tk.y + tk.h - 4 - h, tk.w - 8, h);
        g.restore();
      });
      const LB = LOOPBAR;
      D.round(g, LB.x, LB.y, LB.w, LB.h, 8); g.fillStyle = "#121a25"; g.fill();
      D.hatch(g, LB.x, LB.y, LB.w * INV_MIN, LB.h, "rgba(255,71,87,0.28)");
      const low = L.inv < INV_MIN;
      D.round(g, LB.x + 3, LB.y + 3, Math.max(6, (LB.w - 6) * L.inv), LB.h - 6, 6); g.fillStyle = low ? C.danger : COLD; g.fill();
      g.fillStyle = C.fg; g.fillRect(LB.x + LB.w * INV_MIN - 1, LB.y - 5, 3, LB.h + 10);
      D.text(g, "LOOP", LB.x + LB.w / 2, LB.y + LB.h + 18, 16, C.dim, "center", 700);
      D.text(g, "TANKS", TANKS[0].x + 45, TANKS[0].y - 16, 16, C.dim, "center", 700);
      // The state: one word or two, the reactor's throttle beside the core.
      D.text(g, s.part ? "PUMP B" : st.name, 34, 112, 30, s.kind === "cruise" && !s.part ? C.dim : C.amber, "left", 700);
    }

    /** A needle from a dial's centre at angle a. */
    function needle(g, cx, cy, r, a, col, tip) {
      const x = cx + Math.cos(a) * r, y = cy + Math.sin(a) * r;
      g.strokeStyle = col; g.lineWidth = 5; g.lineCap = "round"; g.beginPath(); g.moveTo(cx, cy); g.lineTo(x, y); g.stroke(); g.lineCap = "butt";
      g.save(); g.translate(x, y); g.rotate(a);
      if (tip === "chev") { g.beginPath(); g.moveTo(-14, -11); g.lineTo(2, 0); g.lineTo(-14, 11); g.strokeStyle = col; g.lineWidth = 5; g.lineJoin = "round"; g.stroke(); }
      else if (tip === "dot") D.disc(g, 0, 0, 9, col);
      else if (tip === "flame") { g.beginPath(); g.moveTo(4, 0); g.quadraticCurveTo(-6, -12, -16, 0); g.quadraticCurveTo(-6, 12, 4, 0); g.fillStyle = col; g.fill(); }
      else { g.fillStyle = col; g.fillRect(-12, -7, 14, 14); }
      g.restore();
    }
    const dialA = (u) => lerp(DIAL_A0, DIAL_A1, clamp(u, 0, 1));
    function bandArc(g, cx, cy, r, u0, u1, col, kind) {
      const a0 = dialA(u0), a1 = dialA(u1);
      if (kind === "hatch") { D.hatchArc(g, cx, cy, r - 10, r + 6, a0, a1, col); return; }
      g.beginPath(); g.arc(cx, cy, r, a0, a1); g.strokeStyle = col; g.lineWidth = 12; g.stroke();
      if (kind === "chev" || kind === "dots") {
        for (let a = a0 + 0.06; a < a1 - 0.03; a += 0.09) {
          const x = cx + Math.cos(a) * r, y = cy + Math.sin(a) * r;
          if (kind === "dots") D.disc(g, x, y, 2.5, "#04131c");
          else { g.save(); g.translate(x, y); g.rotate(a + Math.PI / 2); g.beginPath(); g.moveTo(-3, -4); g.lineTo(1, 0); g.lineTo(-3, 4); g.strokeStyle = "#2a0b0e"; g.lineWidth = 2; g.stroke(); g.restore(); }
        }
      }
    }
    function drawDials(g, d) {
      const L = sim, live = s.phase !== "part";
      // The legs' temperatures: two needles, two bands, the warning and the scram hatched.
      const T = TDIAL, tu = (K) => (K - T_LO) / (T_HI - T_LO);
      D.panel(g, T.x - T.r - 26, T.y - T.r - 30, (T.r + 26) * 2, T.r * 2 + 74, 18, "#0b1119");
      D.ring(g, T.x, T.y, T.r, "#1d2636", 14);
      bandArc(g, T.x, T.y, T.r, tu(COLD_BAND[0]), tu(COLD_BAND[1]), "rgba(79,168,247,0.75)", "dots");
      bandArc(g, T.x, T.y, T.r, tu(HOT_BAND[0]), tu(HOT_BAND[1]), "rgba(255,120,90,0.8)", "chev");
      bandArc(g, T.x, T.y, T.r, tu(WARN_K), tu(SCRAM_K), "rgba(242,160,70,0.8)", "hatch");
      bandArc(g, T.x, T.y, T.r, tu(SCRAM_K), 1, "rgba(255,71,87,0.95)", "hatch");
      for (let K = T_LO; K <= T_HI; K += 10) {
        const a = dialA(tu(K)), r0 = T.r - 22, r1 = T.r - 14;
        g.strokeStyle = "#4a5568"; g.lineWidth = 2; g.beginPath(); g.moveTo(T.x + Math.cos(a) * r0, T.y + Math.sin(a) * r0); g.lineTo(T.x + Math.cos(a) * r1, T.y + Math.sin(a) * r1); g.stroke();
      }
      if (live) {
        needle(g, T.x, T.y, T.r - 26, dialA(tu(L.Tc)), COLD, "dot");
        needle(g, T.x, T.y, T.r - 18, dialA(tu(L.Th)), hotCol(L.Th), "chev");
      }
      D.disc(g, T.x, T.y, 10, "#8796aa");
      D.text(g, "LEGS", T.x, T.y + T.r + 26, 18, C.dim, "center", 700);
      // The core's heat against what the flow carries at the design rise: two needles; the gap between hatched red when
      // the heat is ahead.
      const H = HDIAL, hu = (mw) => mw / H_HI;
      D.panel(g, H.x - H.r - 38, H.y - H.r - 26, (H.r + 38) * 2, H.r * 2 + 68, 18, "#0b1119");
      D.ring(g, H.x, H.y, H.r, "#1d2636", 14);
      if (live) {
        if (d.Q > d.carry) bandArc(g, H.x, H.y, H.r, hu(d.carry), hu(d.Q), "rgba(255,71,87,0.85)", "hatch");
        else { g.beginPath(); g.arc(H.x, H.y, H.r, dialA(hu(d.Q)), dialA(hu(Math.min(H_HI, d.carry)))); g.strokeStyle = "rgba(61,220,132,0.35)"; g.lineWidth = 12; g.stroke(); }
        needle(g, H.x, H.y, H.r - 18, dialA(hu(Math.min(H_HI, d.carry))), COLD, "dot");
        needle(g, H.x, H.y, H.r - 12, dialA(hu(d.Q)), C.amber, "flame");
      }
      D.disc(g, H.x, H.y, 10, "#8796aa");
      D.text(g, "HEAT", H.x - 30, H.y + H.r + 22, 18, C.amber, "center", 700);
      D.text(g, "FLOW", H.x + 30, H.y + H.r + 22, 18, COLD, "center", 700);
    }
    function glyph(g, key, x, y, col) {
      g.strokeStyle = col; g.fillStyle = col; g.lineWidth = 3;
      if (key === "pump") { D.ring(g, x, y, 14, col, 3); for (let k = 0; k < 3; k++) { const a = (k * Math.PI * 2) / 3; g.beginPath(); g.moveTo(x, y); g.lineTo(x + Math.cos(a) * 10, y + Math.sin(a) * 10); g.stroke(); } }
      else if (key === "chill") { for (let k = 0; k < 4; k++) { g.beginPath(); g.moveTo(x - 12 + k * 8, y - 12); g.lineTo(x - 8 + k * 8, y - 2); g.lineTo(x - 12 + k * 8, y + 8); g.stroke(); } }
      else if (key === "rad") { for (let k = 0; k < 4; k++) g.fillRect(x - 13 + k * 7, y - 12, 4, 24); }
      else { g.beginPath(); g.moveTo(x, y - 14); g.quadraticCurveTo(x + 11, y, x + 8, y + 6); g.arc(x, y + 5, 8, 0.1, Math.PI - 0.1); g.quadraticCurveTo(x - 11, y, x, y - 14); g.fill(); }
    }
    function drawLevers(g, t, input) {
      const live = s.phase === "play" || s.phase === "held";
      LEVERS.forEach((l, i) => {
        const v = sim.ctrl[l.key], on = s.grab === i, foc = s.keys && s.focus === i;
        D.round(g, l.x, l.y, l.w, l.h, 14); g.fillStyle = "#121a25"; g.fill(); g.lineWidth = 2; g.strokeStyle = on || foc ? C.amber : C.line; g.stroke();
        // Over 100%: the overdrive zone, hatched amber.
        if (l.max > 1) { const y1 = leverY(l, 1); D.hatch(g, l.x + 4, l.y + 4, l.w - 8, y1 - l.y - 4, "rgba(242,160,70,0.35)"); g.fillStyle = "rgba(232,238,246,0.5)"; g.fillRect(l.x + 4, y1 - 1, l.w - 8, 3); }
        const y = leverY(l, v), bot = l.y + l.h - 20;
        g.fillStyle = l.key === "makeup" || l.key === "chill" ? "rgba(79,168,247,0.35)" : "rgba(190,159,230,0.3)";
        g.fillRect(l.x + 12, y, l.w - 24, bot - y);
        D.round(g, l.x - 8, y - 16, l.w + 16, 32, 10); g.fillStyle = live ? (on ? C.amber : "#c9d3e0") : "#4a5568"; g.fill();
        g.fillStyle = "#1b2433"; g.fillRect(l.x + 4, y - 2, l.w - 8, 4);
        glyph(g, l.key, l.x + l.w / 2, l.y - 30, live ? C.fg : C.dim);
        D.text(g, l.word, l.x + l.w / 2, l.y + l.h + 26, l.w > 70 ? 20 : 16, on || foc ? C.amber : C.dim, "center", 700);
      });
      // The exact values: on hover (a mouse), while a lever is held, or on the keys' focus. One tooltip.
      let tip = null;
      if (s.grab !== null) tip = s.grab;
      else if (s.keys) tip = s.focus;
      else if (input && !input.down && input.x >= 0) { const i = leverAt(input.x, input.y); if (i >= 0) tip = i; }
      if (tip !== null && live) {
        const l = LEVERS[tip], d = derive(sim, STATES[s.kind]), v = sim.ctrl[l.key];
        const txt = {
          pump: `${Math.round(v * 100)}%  ${Math.round(d.flow * FLOW_FULL)} kg/s`,
          chill: `${Math.round(v * 100)}% through  ${d.rej.toFixed(1)} MW`,
          rad: `${Math.round(v * 100)}%  ${d.rej.toFixed(1)} MW`,
          makeup: `${Math.round(v * 100)}%  ${(MAKEUP_KG_S * v).toFixed(1)} kg/s`,
        }[l.key];
        tooltip(g, txt, l.x + l.w / 2, leverY(l, v) - 30);
      }
      // Hover on the dials and the loop: the numbers behind the needles.
      if (input && !input.down && s.grab === null && !s.keys && live) {
        const d = derive(sim, STATES[s.kind]);
        if (Math.hypot(input.x - TDIAL.x, input.y - TDIAL.y) < TDIAL.r) tooltip(g, `Hot ${sim.Th.toFixed(1)} K  Cold ${sim.Tc.toFixed(1)} K`, TDIAL.x, TDIAL.y - TDIAL.r - 6);
        else if (Math.hypot(input.x - HDIAL.x, input.y - HDIAL.y) < HDIAL.r) tooltip(g, `Heat ${d.Q.toFixed(1)} MW  Flow carries ${d.carry.toFixed(1)} MW`, HDIAL.x, HDIAL.y - HDIAL.r - 6);
        else if (input.x < 150 && input.y > 280 && input.y < 580) tooltip(g, `Loop ${(sim.inv * 100).toFixed(1)}%  Tanks ${Math.round(sim.tanks[0] + sim.tanks[1])} kg  Leak ${d.leak.toFixed(1)} kg/s`, 300, 290);
      }
    }
    function tooltip(g, txt, x, y) {
      g.font = `600 18px ${KIT.FONT}`;
      const w = g.measureText(txt).width + 24;
      const bx = clamp(x - w / 2, 8, api.W - w - 8);
      D.panel(g, bx, y - 34, w, 32, 10, "#1d2738", "#4a5568");
      D.text(g, txt, bx + w / 2, y - 18, 18, C.fg, "center", 600);
    }

    return {
      /** For tools (tests and shots): the round's state, read only. */
      peek() {
        if (!s) return null;
        const st = STATES[s.kind], d = derive(sim, st);
        return {
          index: s.index, round: s.round, kind: s.kind, part: s.part, phase: s.phase, hold: s.hold, holdNeed: s.holdNeed,
          Tc: sim.Tc, Th: sim.Th, inv: sim.inv, tanks: [...sim.tanks], throttle: sim.throttle, ctrl: { ...sim.ctrl }, d, st: { ...st },
          inBand: inBand(sim), clock, log: log.map((e) => ({ ...e })), cart: { ...s.cart },
          K: { CP, M_FULL, FLOW_FULL, HEAT_FULL_MW, RAD_MW, T_RAD, HOT_BAND, COLD_BAND, INV_MIN, SPEED_MAX, MAKEUP_KG_S, TIME_X },
          layout: { LEVERS, slot: pumpAt(1), CRATE },
        };
      },
      step(index, isPart) { start(index, isPart); },
      update(dt, input) {
        clock += dt;
        s.flash = Math.max(0, s.flash - dt * 2);
        if (s.phase === "part") partStep(dt, input);
        else if (s.phase === "play" || s.phase === "held") play(dt, input);
      },
      draw(g, t, dt, input) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        const st = STATES[s.kind], d = derive(sim, st);
        drawLoop(g, t, d, st);
        drawDials(g, d);
        drawLevers(g, t, input);
        if (s.phase === "part" || s.phase === "fitted") {
          // The new pump cartridge in its crate, or in the hand.
          D.panel(g, CRATE.x, CRATE.y, CRATE.w, CRATE.h, 14, "#141b27");
          D.hatch(g, CRATE.x + 10, CRATE.y + 10, CRATE.w - 20, 10, "rgba(242,160,70,0.5)");
          const p = s.cart;
          if (!p.set) {
            D.disc(g, p.x + 4, p.y + 6, 30, "rgba(0,0,0,0.45)");
            pumpGlyph(g, p.x, p.y, false, false, t, 0);
            D.ring(g, p.x, p.y, 38 + 3 * Math.sin(t * 5), p.held || p.pad ? C.amber : "rgba(232,238,246,0.35)", 3);
          }
        }
      },
    };
  },
});
