/*
 * repairs/tubes.js: the Gannet tubes' repair (openspec/changes/repair-minigames, design 2; the missile and its tube
 * states are weapons-and-shields section 10).
 *
 * The magazine's breech: above, the tube side on with a Gannet on its ready tray; below left, the breech face; below
 * right, the load panel's three big switches. First clear the jam: turn the locking ring (drag around it, or Left and
 * Right) feeling for each hidden notch. The pawl at the top rises as the ring nears a notch and drops into it once
 * the ring rests there; turning fast runs past it. The lamps beside the face count the notches. With the last notch
 * found the ring lets go and the breech door swings open. Then load and arm, in order: RAIL (the guide rail lowers
 * and the missile runs in), LATCH (only once the missile sits home against its stop; it bounces before it settles),
 * INTERLOCK (the tube arms). Click a switch, or Left and Right to pick and Space to throw. A switch out of order is a
 * fumble: the interlock trips and the latch and interlock fall back off. Later rounds hide more notches in narrower
 * windows, bounce the missile longer and shuffle the switches on the panel. A disabled tube's first step fits a new
 * breech seal: drag it from the crate onto the breech face.
 */
RepairKit.register({
  id: "tubes",
  title: "Gannet tubes and hoist",
  place: "Magazine",
  group: "Weapons and sensors",
  hazard: "Interlock tripped: back to the latch",
  // The how-to card (repair-minigames 6g), drawn by the kit: pictures and a few words, on demand.
  guide: {
    steps: [
      { icon: "turn", text: "Turn the ring slowly" },
      { icon: "hold", text: "Rest where the pawl drops" },
      { icon: "order", text: "RAIL, LATCH, INTERLOCK" },
    ],
    mistake: "A switch out of order: the interlock trips",
    now: (q) => (q.part ? -1 : q.phase === "jam" ? 0 : q.phase === "load" ? 2 : -1),
  },
  down: "The tube cannot load",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const MY = 220;                                     // the missile's axis, side view
    const TUBE = { x0: 600, x1: 1240, top: 180, bot: 260 };
    const M_LEN = 380, M_START = 120, M_HOME = 650;     // the missile's tail x on the tray, and home in the tube
    const FACE = { x: 300, y: 545, r: 150 };            // the breech face
    const SW = { y0: 400, y1: 640, pivot: 520, w: 140 };
    const NAMES = { rail: "RAIL", latch: "LATCH", arm: "INTERLOCK" };
    const DWELL_S = 0.4;                                // seconds at rest in a notch's window for the pawl to drop
    let hadPart = false, part, seal, phase, hard;
    let notches, found, ringA, ringLo, dwell, dragA, ringShake, openT, doorT;
    let slots, focus, on, railT, latchT, mx, mv, tripT, tripName, played, armT;
    const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
    const wrap = (a) => Math.atan2(Math.sin(a), Math.cos(a));
    const slotX = (s) => 730 + s * 200;
    const home = () => mx >= M_HOME - 3;


    /** Throw switch `name`: in order it acts, out of order it trips the interlock. */
    function throwSwitch(name) {
      if (on[name] || played) return;
      const ok = name === "rail" || (name === "latch" && on.rail && railT >= 1 && home()) || (name === "arm" && on.latch);
      if (!ok) {
        on.latch = false; on.arm = false; latchT = Math.min(latchT, 1); tripName = name; tripT = 0.9;
        api.fumble("Interlock tripped: back to the latch");
        return;
      }
      on[name] = true;
      if (name === "latch") mv = 0;
      if (name === "arm") { played = true; armT = 0; api.stepDone(); }
    }

    return {
      step(index, isPart) {
        const r = api.rand();
        if (isPart) hadPart = true;
        part = isPart;
        hard = index;   // the round's level - 1 (repair-minigames 1a)
        seal = { x: 1080, y: 540, held: false, set: false };
        phase = part ? "part" : "jam";
        // Hidden notches, clockwise from the ring's rest.
        const k = 2 + Math.min(2, Math.ceil(hard / 2));
        notches = []; let a = 0;
        for (let i = 0; i < k; i++) { a += 0.6 + r() * 0.9; notches.push(a); }
        found = 0; ringA = 0; ringLo = -0.3; dwell = 0; dragA = null; ringShake = 0; openT = 0; doorT = 0;
        // The panel: in order at first, shuffled once the hands know it.
        slots = ["rail", "latch", "arm"];
        if (hard >= 2) for (let i = 2; i > 0; i--) { const j = Math.floor(r() * (i + 1)); [slots[i], slots[j]] = [slots[j], slots[i]]; }
        focus = 0; on = { rail: false, latch: false, arm: false };
        railT = 0; latchT = 0; mx = M_START; mv = 0; tripT = 0; tripName = ""; played = false; armT = 0;
      },
      /** For tools (shots and tests): the round's state, read only, so a script can play it. */
      peek() { return { phase, part, seal: { ...seal }, FACE, notches: [...notches], found, ringA, dwell, slots: [...slots], slotX: [0, 1, 2].map(slotX), SW, on: { ...on }, railT, home: home(), mx, mv, played }; },
      update(dt, input) {
        ringShake = Math.max(0, ringShake - dt);
        tripT = Math.max(0, tripT - dt);
        if (phase === "part") {
          if (input.pressed && Math.hypot(input.x - seal.x, input.y - seal.y) < 110) seal.held = true;
          if (seal.held && input.down) { seal.x = input.x; seal.y = input.y; }
          if (seal.held && input.released) {
            seal.held = false;
            if (Math.hypot(seal.x - FACE.x, seal.y - FACE.y) < 50) { seal.set = true; seal.x = FACE.x; seal.y = FACE.y; phase = "sealed"; api.stepDone(); }
          }
          return;
        }
        if (phase === "jam") {
          const last = notches[notches.length - 1];
          if (input.pressed) {
            const d = Math.hypot(input.x - FACE.x, input.y - FACE.y);
            if (d > 40 && d < 210) dragA = Math.atan2(input.y - FACE.y, input.x - FACE.x);
          }
          let turn = 0;
          if (dragA !== null && input.down) {
            const a = Math.atan2(input.y - FACE.y, input.x - FACE.x);
            turn = wrap(a - dragA); dragA = a;
          } else dragA = null;
          turn += input.stick.x * 0.9 * dt;
          ringA = clamp(ringA + turn, ringLo, last + 0.35);
          const speed = Math.abs(turn) / Math.max(dt, 1e-3);
          const w = Math.max(0.045, 0.085 - 0.01 * hard);
          const inWin = Math.abs(ringA - notches[found]) < w;
          // The pawl drops only while the ring rests in the window.
          if (inWin && speed < 1.2) dwell += dt; else dwell = Math.max(0, dwell - 2 * dt);
          if (dwell >= DWELL_S) {
            ringA = notches[found]; ringLo = ringA; found++; dwell = 0; ringShake = 0.3;
            if (found === notches.length) { phase = "open"; openT = 0; }
          }
          return;
        }
        if (phase === "open") {
          // The ring lets go and turns to its stop; the breech door swings up.
          openT += dt;
          ringA = Math.min(notches[notches.length - 1] + 0.35, ringA + dt * 0.8);
          doorT = clamp((openT - 0.4) / 0.6, 0, 1);
          if (doorT >= 1) phase = "load";
          return;
        }
        if (phase !== "load") return;
        // The panel.
        if (input.hit.has("ArrowLeft") || input.hit.has("KeyA")) focus = (focus + 2) % 3;
        if (input.hit.has("ArrowRight") || input.hit.has("KeyD")) focus = (focus + 1) % 3;
        if (input.actionPressed) { throwSwitch(slots[focus]); return; }
        if (input.pressed && input.y > SW.y0 && input.y < SW.y1 + 30) {
          for (let s = 0; s < 3; s++) if (Math.abs(input.x - slotX(s)) < SW.w / 2) { focus = s; throwSwitch(slots[s]); return; }
        }
        // The movers: the rail lowers, the missile runs in and bounces at its stop, the latch swings down.
        if (on.rail) railT = Math.min(1, railT + dt / 0.6);
        latchT = on.latch ? Math.min(1, latchT + dt / 0.35) : Math.max(0, latchT - dt / 0.35);
        if (railT >= 1 && !on.latch) {
          if (mx < M_HOME) mv = Math.min(520, mv + (480 + 60 * hard) * dt);
          mx += mv * dt;
          if (mx >= M_HOME) {
            mx = M_HOME;
            mv = mv > 50 ? -mv * Math.min(0.62, 0.32 + 0.1 * hard) : 0;
          }
        }
        if (played) { armT += dt; doorT = Math.max(0, 1 - armT / 0.6); }
      },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        // ---- Side view: the tray, the rail, the tube cut away, the missile, the latch, the breech door.
        D.panel(g, 30, 100, 1220, 240, 18, "#0b111b");
        // The ready tray and its legs.
        g.fillStyle = "#283246"; g.fillRect(80, 262, 500, 14);
        for (const x of [120, 300, 480]) g.fillRect(x, 276, 16, 50);
        for (let x = 100; x < 580; x += 40) D.disc(g, x, 258, 6, "#3a4558");
        // The tube's bore, dark, then the rail inside it.
        g.fillStyle = "#05080d"; g.fillRect(TUBE.x0, TUBE.top, TUBE.x1 - TUBE.x0, TUBE.bot - TUBE.top);
        const railY = 252;
        g.fillStyle = on && on.rail ? "#55617a" : "#3a4558";
        g.fillRect(680, railY, 540, 6);
        // The bridging section: tilted up until thrown, level once down.
        g.save(); g.translate(560, railY + 3); g.rotate(-0.35 * (1 - railT));
        g.fillStyle = railT >= 1 ? "#55617a" : C.amber; g.fillRect(0, -3, 120, 6); g.restore();
        // The missile.
        const shake = ringShake > 0 ? Math.sin(t * 80) * 2 : 0;
        const x = mx;
        g.fillStyle = "#aab3c0"; g.fillRect(x, MY - 26, M_LEN - 50, 52);
        g.beginPath(); g.moveTo(x + M_LEN - 50, MY - 26); g.lineTo(x + M_LEN, MY); g.lineTo(x + M_LEN - 50, MY + 26); g.closePath(); g.fill();
        g.fillStyle = C.amber; g.fillRect(x + M_LEN - 90, MY - 26, 16, 52);
        g.fillStyle = "#6f7a8a";
        for (const sgn of [-1, 1]) {
          g.beginPath(); g.moveTo(x, MY + sgn * 26); g.lineTo(x + 50, MY + sgn * 26); g.lineTo(x + 30, MY + sgn * 38); g.lineTo(x, MY + sgn * 38); g.closePath(); g.fill();
        }
        g.fillStyle = "#3a4558"; g.fillRect(x - 10, MY - 14, 10, 28);
        // The tube's walls drawn over the bore's edges.
        g.fillStyle = "#2a3446"; g.fillRect(TUBE.x0, TUBE.top - 16, TUBE.x1 - TUBE.x0, 16); g.fillRect(TUBE.x0, TUBE.bot, TUBE.x1 - TUBE.x0, 16);
        for (let xx = TUBE.x0 + 60; xx < TUBE.x1; xx += 120) { g.fillStyle = "#364257"; g.fillRect(xx, TUBE.top - 16, 8, 16); g.fillRect(xx, TUBE.bot, 8, 16); }
        // Interlock lamps on the tube: hollow while safe, filled green when armed.
        for (let i = 0; i < 3; i++) {
          const lx = 1100 + i * 40, ly = 140;
          if (on.arm) D.disc(g, lx, ly, 11, C.ok); else D.ring(g, lx, ly, 10, "#3a4558", 4);
        }
        // The breech flange and its door (hinged at the top, swings up and aft, clear of the latch).
        g.fillStyle = C.steel; g.fillRect(588, 150, 24, 140);
        g.save(); g.translate(600, 160); g.rotate(doorT * 1.9);
        g.fillStyle = doorT > 0 && doorT < 1 ? "#8c96a6" : "#5d6879"; g.fillRect(-10, 0, 20, 100); g.restore();
        // The latch: an arm from a pivot above the breech, swinging down behind the tail.
        const la = -0.6 + (Math.PI / 2 + 0.6) * latchT;
        const px = 628, py = 150, lx = px + Math.cos(la) * 70, ly = py + Math.sin(la) * 70;
        g.strokeStyle = on.latch ? C.ok : "#8792a3"; g.lineWidth = 10; g.lineCap = "round";
        g.beginPath(); g.moveTo(px, py); g.lineTo(lx, ly); g.stroke(); g.lineCap = "butt";
        D.disc(g, px, py, 9, "#3a4558");
        // ---- The breech face: the housing, the locking ring, the pawl, the notch lamps.
        const dimFace = phase === "load" || phase === "sealed";
        g.save(); g.globalAlpha = dimFace ? 0.55 : 1;
        g.translate(shake, 0);
        D.disc(g, FACE.x, FACE.y, FACE.r, "#141b27");
        D.ring(g, FACE.x, FACE.y, FACE.r, C.steel, 4);
        for (let i = 0; i < 12; i++) {
          const a = (i / 12) * Math.PI * 2;
          D.disc(g, FACE.x + Math.cos(a) * (FACE.r - 12), FACE.y + Math.sin(a) * (FACE.r - 12), 5, "#4a5568");
        }
        // The locking ring and its lugs, turned by ringA.
        D.ring(g, FACE.x, FACE.y, 112, "#2f3a4d", 40);
        for (let i = 0; i < 8; i++) {
          const a = ringA + (i / 8) * Math.PI * 2;
          g.save(); g.translate(FACE.x + Math.cos(a) * 112, FACE.y + Math.sin(a) * 112); g.rotate(a);
          g.fillStyle = i === 0 ? C.accent : "#56637a"; g.fillRect(-14, -16, 28, 32); g.restore();
        }
        // The bore: the missile's tail when one is home, the new seal's groove on the part step.
        D.disc(g, FACE.x, FACE.y, 80, "#05080d");
        if (part && !seal.set) { g.setLineDash([10, 8]); D.ring(g, FACE.x, FACE.y, 86, C.amber, 4); g.setLineDash([]); }
        if (seal.set) D.ring(g, FACE.x, FACE.y, 86, "#2b2f38", 12);
        if (home()) { D.disc(g, FACE.x, FACE.y, 56, "#6f7a8a"); D.disc(g, FACE.x, FACE.y, 20, "#3a4558"); }
        // The pawl at the top: it rises as the ring nears a notch and drops in at rest.
        if (phase === "jam") {
          const near = clamp(1 - Math.abs(ringA - notches[found]) / 0.6, 0, 1);
          const lift = near * near * 22 * (1 - dwell / DWELL_S);
          const top = FACE.y - 136 - lift;
          g.beginPath(); g.moveTo(FACE.x - 16, top - 26); g.lineTo(FACE.x + 16, top - 26); g.lineTo(FACE.x, top); g.closePath();
          g.fillStyle = dwell > 0 ? C.amber : near > 0.5 ? "#e8c28a" : C.steel; g.fill();
          // The feel: an arc beside the pawl, its length how near the notch is.
          g.beginPath(); g.arc(FACE.x, FACE.y, FACE.r + 18, -Math.PI / 2 - 0.5 * near, -Math.PI / 2 + 0.5 * near);
          g.strokeStyle = `rgba(242,160,70,${0.3 + 0.7 * near})`; g.lineWidth = 8; g.stroke();
          // The way it turns: an arrow on the rim.
          D.turnArrow(g, FACE.x, FACE.y, FACE.r + 18, -Math.PI / 2 + 0.7, -Math.PI / 2 + 1.3, C.dim);
        }
        g.restore();
        // The notch lamps: hollow for one still to find, filled green once found.
        if (!part) {
          notches.forEach((_, i) => {
            const lx2 = 500, ly2 = 440 + i * 46;
            if (i < found) D.disc(g, lx2, ly2, 14, C.ok); else D.ring(g, lx2, ly2, 12, i === found && phase === "jam" ? C.amber : "#3a4558", 4);
          });
        }
        // ---- The load panel, or the crate on the part step.
        if (phase === "part" || (part && seal.set)) {
          if (!seal.set) {
            D.panel(g, 960, 420, 240, 240, 16, "#141b27");
            D.ring(g, seal.x, seal.y, 86, "#2b2f38", 16);
            D.ring(g, seal.x, seal.y, 86, C.amber, 2);
          }
          return;
        }
        const live = phase === "load";
        g.save(); g.globalAlpha = live ? 1 : 0.4;
        D.panel(g, 620, 380, 620, 320, 18, "#0f1620");
        for (let s = 0; s < 3; s++) {
          const name = slots[s], cx = slotX(s);
          const tripped = tripT > 0 && tripName === name;
          D.round(g, cx - SW.w / 2, SW.y0, SW.w, SW.y1 - SW.y0, 14);
          g.fillStyle = tripped ? "#3a1218" : "#161f2c"; g.fill();
          g.lineWidth = 3; g.strokeStyle = live && focus === s ? C.amber : C.line; g.stroke();
          // The lever: up when off, down when thrown.
          const up = on[name] ? 0 : 1;
          const ang = -Math.PI / 2 * up + Math.PI / 2 * (1 - up);
          const ex = cx, ey = SW.pivot + Math.sin(ang) * 85;
          g.strokeStyle = "#8792a3"; g.lineWidth = 16; g.lineCap = "round";
          g.beginPath(); g.moveTo(cx, SW.pivot); g.lineTo(ex, ey); g.stroke(); g.lineCap = "butt";
          D.disc(g, cx, SW.pivot, 20, "#3a4558");
          D.disc(g, ex, ey, 26, on[name] ? C.ok : tripped ? C.danger : "#c9d1dc");
          if (on[name]) {
            g.strokeStyle = "#0b2a18"; g.lineWidth = 5; g.beginPath(); g.moveTo(ex - 11, ey); g.lineTo(ex - 3, ey + 9); g.lineTo(ex + 12, ey - 9); g.stroke();
          } else if (tripped) {
            g.strokeStyle = "#2a0a0e"; g.lineWidth = 5; g.beginPath(); g.moveTo(ex - 9, ey - 9); g.lineTo(ex + 9, ey + 9); g.moveTo(ex + 9, ey - 9); g.lineTo(ex - 9, ey + 9); g.stroke();
          }
          D.text(g, NAMES[name], cx, 670, 20, on[name] ? C.ok : C.fg, "center", 700);
        }
        g.restore();
      },
    };
  },
});
