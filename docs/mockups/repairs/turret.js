/*
 * repairs/turret.js: the twin pulse cannon's repair (openspec/changes/repair-minigames, design 2; the cannon is
 * weapons-and-shields section 2).
 *
 * The turret's access room: one barrel's emitter on its optical bench, a test beam through two lenses onto a target
 * plate (seen face on, on the right), and the capacitor bank (24 MJ) below with its coupling. A lens round: slide the
 * two lenses along the rail (drag them, or keys: Up and Down pick a lens, Left and Right move it) until the spot on
 * the plate is smallest and on the crosshair, and hold it there. The lenses work together: each one moves both the
 * focus and the aim, so neither can be set alone. Every other round reseats the bank's coupling: drag it home along
 * its guide (or push it with Right) and arrive inside the speed band. Too fast slams it: a fumble, the bank arcs and
 * loses its charge. Too slow and it does not latch and rolls back. Later rounds shrink the target, add shimmer to the
 * beam and narrow the band. A disabled turret's first step fits a new focus lens: drag it from the crate into the
 * empty carrier.
 */
RepairKit.register({
  id: "turret",
  title: "Twin pulse cannon",
  place: "Turret access room",
  group: "Weapons and sensors",
  hazard: "The bank arcs: 10 HP, charge lost",
  // The how-to card (repair-minigames 6g), drawn by the kit: pictures and a few words, on demand.
  guide: {
    steps: [
      { icon: "slider", text: "Slide the two lenses" },
      { icon: "aim", text: "Smallest spot on the cross" },
      { icon: "hold", text: "Hold it there" },
      { icon: "drag", text: "Coupling: push home steady" },
    ],
    mistake: "The coupling slammed home: the bank arcs, 10 HP",
    now: (q) => (q.part ? -1 : q.kind === "coupling" ? 3 : 0),
  },
  down: "The turret does not fire",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const AXIS = 250;                                   // the beam's axis on the bench, y
    const EMIT_X = 250;                                 // the emitter's muzzle, x
    const LENS = [{ x0: 320, x1: 500 }, { x0: 560, x1: 740 }];   // each lens's travel on the rail
    const PLATE_X = 830;                                // the target plate, side on
    const VIEW = { x: 1060, y: 250, r: 150 };           // the plate seen face on
    const GUIDE = { y: 600, x0: 330, home: 862 };       // the coupling's guide; home is plug-in-socket
    const VMAX_GAUGE = 800;                             // the speed gauge's full scale, px/s
    const HOLD_S = 1.0;                                 // seconds the spot must stay on to finish a lens round
    let kind, hadPart = false, part, lensPart;
    let lens, sel, grab, target, dir, shimmer, tolC, tolR, hold;
    let plug, band, charge, arc, latchT, played, clock = 0;
    const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
    const lensX = (i) => LENS[i].x0 + (LENS[i].x1 - LENS[i].x0) * lens[i];

    /** The spot on the plate from the two lens positions: radius and centre offset, px in the face-on view. */
    function spot(t) {
      const ea = lens[0] - target[0], eb = lens[1] - target[1];
      const focus = ea + 0.6 * eb, aim = 0.7 * ea - eb;
      const sh = shimmer * Math.sin(t * 2.3) + shimmer * 0.6 * Math.sin(t * 5.1 + 1);
      const shy = shimmer * Math.cos(t * 1.9 + 2) * 0.8;
      const r = clamp(7 + 260 * Math.abs(focus) + Math.abs(sh) * 0.3, 7, 125);
      const off = clamp(300 * aim, -140, 140);
      return { r, x: Math.cos(dir) * off + sh, y: Math.sin(dir) * off + shy };
    }


    return {
      step(index, isPart) {
        const r = api.rand();
        if (isPart) hadPart = true;
        part = isPart;
        const n = index - (hadPart ? 1 : 0);              // rounds since the part
        kind = n % 2 === 0 ? "lens" : "coupling";
        lensPart = { x: 190, y: 475, held: false, set: false };
        target = [0.2 + 0.6 * r(), 0.2 + 0.6 * r()];
        // Start the lenses well off their marks.
        lens = target.map((v) => (v > 0.5 ? clamp(v - 0.3 - 0.15 * r(), 0, 1) : clamp(v + 0.3 + 0.15 * r(), 0, 1)));
        dir = r() * Math.PI * 2;
        const hard = Math.max(0, n);
        shimmer = 1.2 + 1.3 * hard;
        tolC = Math.max(9, 15 - 1.5 * hard);
        tolR = Math.max(13, 19 - 1.5 * hard);
        sel = 0; grab = -1; hold = 0;
        // The coupling: out at the start of its guide on a coupling round, home otherwise.
        const coupling = kind === "coupling" && !part;
        plug = { x: coupling ? GUIDE.x0 : GUIDE.home, v: 0, held: false, last: 0, seated: !coupling, back: 0 };
        const lo = 150 + 15 * hard, hi = Math.max(lo + 120, 430 - 35 * hard);
        band = [lo, hi];
        charge = coupling ? 0 : 1;
        arc = 0; latchT = 0; played = false;
      },
      /** For tools (shots and tests): the round's state, read only, so a script can play it. */
      peek() { return { kind, part, partSet: lensPart.set, lensPart: { ...lensPart }, lens: [...lens], target: [...target], lensX: [lensX(0), lensX(1)], LENS, AXIS, GUIDE, plug: { ...plug }, band: [...band], hold, played, spot: spot(clock) }; },
      update(dt, input) {
        clock += dt;
        arc = Math.max(0, arc - dt);
        if (plug.seated) charge = Math.min(1, charge + dt * 0.8);
        if (part && !lensPart.set) {
          // The part step: carry the new focus lens to its carrier.
          if (input.pressed && Math.hypot(input.x - lensPart.x, input.y - lensPart.y) < 60) lensPart.held = true;
          if (lensPart.held && input.down) { lensPart.x = input.x; lensPart.y = input.y; }
          if (lensPart.held && input.released) {
            lensPart.held = false;
            if (Math.hypot(lensPart.x - lensX(0), lensPart.y - AXIS) < 45) {
              lensPart.set = true; lensPart.x = lensX(0); lensPart.y = AXIS; api.stepDone();
            }
          }
          return;
        }
        if (played) return;
        if (kind === "lens") {
          // Pick a lens by pointer or keys, slide it along its travel.
          if (input.pressed) {
            // The nearer lens within 40 px across (two at the ends of their travels are 60 px apart).
            if (input.y > AXIS - 90 && input.y < AXIS + 130) {
              const i = KIT.nearest([0, 1].map((k) => [lensX(k), input.y]), input.x, input.y, 40);
              if (i >= 0) { grab = i; sel = i; }
            }
          }
          if (grab >= 0 && input.down) {
            lens[grab] = clamp((input.x - LENS[grab].x0) / (LENS[grab].x1 - LENS[grab].x0), 0, 1);
          }
          if (input.released) grab = -1;
          if (input.hit.has("ArrowUp") || input.hit.has("KeyW") || input.hit.has("ArrowDown") || input.hit.has("KeyS")) sel = 1 - sel;
          if (input.stick.x) lens[sel] = clamp(lens[sel] + input.stick.x * 0.22 * dt, 0, 1);
          const s = spot(clock);
          const on = Math.hypot(s.x, s.y) < tolC && s.r < tolR;
          hold = on ? hold + dt : Math.max(0, hold - 2 * dt);
          if (hold >= HOLD_S) { hold = HOLD_S; played = true; api.stepDone(); }
          return;
        }
        // The coupling round.
        if (plug.seated) return;
        if (plug.back > 0) {
          // Rolled back from the socket: it eases out a short way and stops.
          plug.back = Math.max(0, plug.back - dt);
          plug.x = Math.max(GUIDE.x0, plug.x - 260 * dt * (plug.back / 0.5));
          plug.v = 0;
          return;
        }
        if (input.pressed && Math.abs(input.x - plug.x) < 60 && Math.abs(input.y - GUIDE.y) < 50) { plug.held = true; plug.last = input.x; }
        if (plug.held && input.down) {
          const nx = clamp(input.x, GUIDE.x0, GUIDE.home);
          const raw = (nx - plug.x) / Math.max(dt, 1e-3);
          plug.v += (raw - plug.v) * Math.min(1, dt / 0.08);
          plug.x = nx;
        } else {
          if (input.released) plug.held = false;
          if (input.stick.x > 0) plug.v += 520 * dt;
          else if (input.stick.x < 0) plug.v -= 520 * dt;
          else plug.v *= Math.exp(-1.6 * dt);
          plug.x += plug.v * dt;
          if (plug.x < GUIDE.x0) { plug.x = GUIDE.x0; plug.v = 0; }
        }
        if (plug.x >= GUIDE.home - 0.5) {
          const v = plug.v;
          plug.held = false;
          if (v > band[1]) {
            // Slammed: the bank arcs and dumps its charge; the coupling kicks back out.
            arc = 0.9; charge = 0; plug.x = GUIDE.home - 6; plug.back = 0.8; plug.v = 0;
            api.fumble("The bank arcs: 10 HP, charge lost");
          } else if (v < band[0]) {
            plug.x = GUIDE.home - 4; plug.back = 0.5; plug.v = 0;     // too soft: the latch does not catch
          } else {
            plug.x = GUIDE.home; plug.v = 0; plug.seated = true; latchT = 1; played = true; api.stepDone();
          }
        }
      },
      draw(g, t, dt) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        latchT = Math.max(0, latchT - dt * 1.5);
        const lensRound = kind === "lens" || part;
        const benchDim = !lensRound;
        // ---- The optical bench: the emitter, the rail, the lenses, the beam.
        g.save(); g.globalAlpha = benchDim ? 0.45 : 1;
        D.panel(g, 40, 110, 840, 290, 18, "#0b111b");
        // The emitter's housing: two barrels' breeches, the upper one on test.
        D.round(g, 70, 160, 180, 180, 14); g.fillStyle = "#1a2333"; g.fill(); g.lineWidth = 3; g.strokeStyle = C.steel; g.stroke();
        for (const yy of [AXIS - 40, AXIS + 40]) { D.round(g, 210, yy - 22, 50, 44, 8); g.fillStyle = "#222d40"; g.fill(); g.strokeStyle = C.steel; g.lineWidth = 2; g.stroke(); }
        for (let i = 0; i < 5; i++) { g.fillStyle = "#2a3446"; g.fillRect(90, 180 + i * 30, 90, 12); }
        D.disc(g, 245, AXIS - 40, 9, part ? "#2a3446" : C.accent);
        D.text(g, "EMITTER", 160, 360, 15, C.dim, "center", 700);
        // The rail under the lenses, and each lens's travel.
        g.fillStyle = "#283246"; g.fillRect(EMIT_X + 20, AXIS + 80, PLATE_X - EMIT_X - 10, 12);
        for (let i = 0; i < 2; i++) {
          D.round(g, LENS[i].x0 - 8, AXIS + 100, LENS[i].x1 - LENS[i].x0 + 16, 14, 7);
          g.fillStyle = "#121a26"; g.fill(); g.strokeStyle = C.line; g.lineWidth = 2; g.stroke();
        }
        // The plate, side on.
        g.fillStyle = C.steel; g.fillRect(PLATE_X, AXIS - 120, 12, 240);
        // The beam: collimated from the emitter, bent by each lens, onto the plate.
        const s = spot(clock);
        if (!part) {
          const xa = lensX(0), xb = lensX(1);
          const hb = 30 * (0.45 + 0.6 * Math.abs(lens[0] - target[0])) + 6;
          const hp = s.r * 0.45, cp = AXIS - 40 + s.y * 0.35;
          g.beginPath();
          g.moveTo(EMIT_X, AXIS - 40 - 14); g.lineTo(xa, AXIS - 40 - 26); g.lineTo(xb, AXIS - 40 - hb + s.y * 0.15); g.lineTo(PLATE_X, cp - hp);
          g.lineTo(PLATE_X, cp + hp); g.lineTo(xb, AXIS - 40 + hb + s.y * 0.15); g.lineTo(xa, AXIS - 40 + 26); g.lineTo(EMIT_X, AXIS - 40 + 14);
          g.closePath(); g.fillStyle = benchDim ? "rgba(79,195,247,0.08)" : "rgba(79,195,247,0.22)"; g.fill();
        }
        // The lenses on their carriers (the beam runs through the upper barrel's line).
        for (let i = 0; i < 2; i++) {
          const x = lensX(i);
          const empty = part && i === 0 && !lensPart.set;
          g.fillStyle = "#30394c"; g.fillRect(x - 10, AXIS - 40 + 60, 20, AXIS + 80 - (AXIS + 20));
          D.round(g, x - 26, AXIS + 70, 52, 18, 6); g.fillStyle = "#3a4558"; g.fill();
          if (empty) {
            g.setLineDash([8, 6]); g.beginPath(); g.ellipse(x, AXIS - 40, 16, 62, 0, 0, Math.PI * 2);
            g.strokeStyle = C.amber; g.lineWidth = 3; g.stroke(); g.setLineDash([]);
          } else {
            g.beginPath(); g.ellipse(x, AXIS - 40, 14, 60, 0, 0, Math.PI * 2);
            g.fillStyle = "rgba(160,220,255,0.35)"; g.fill();
            g.lineWidth = 4; g.strokeStyle = lensRound && !part && sel === i ? C.amber : C.steel; g.stroke();
          }
        }
        g.restore();
        // ---- The plate face on: crosshair, the target ring, the spot, the hold.
        const dimView = !lensRound || part;
        D.disc(g, VIEW.x, VIEW.y, VIEW.r, "#0b1018");
        D.ring(g, VIEW.x, VIEW.y, VIEW.r, C.line, 4);
        g.strokeStyle = "#2b3a50"; g.lineWidth = 2;
        g.beginPath(); g.moveTo(VIEW.x - VIEW.r + 10, VIEW.y); g.lineTo(VIEW.x + VIEW.r - 10, VIEW.y);
        g.moveTo(VIEW.x, VIEW.y - VIEW.r + 10); g.lineTo(VIEW.x, VIEW.y + VIEW.r - 10); g.stroke();
        for (const rr of [50, 100]) D.ring(g, VIEW.x, VIEW.y, rr, "#18222f", 2);
        if (!part) {
          const on = Math.hypot(s.x, s.y) < tolC && s.r < tolR;
          const sx = VIEW.x + s.x, sy = VIEW.y + s.y;
          const grad = g.createRadialGradient(sx, sy, 0, sx, sy, s.r);
          const a = dimView ? 0.25 : 1;
          grad.addColorStop(0, `rgba(230,250,255,${a})`); grad.addColorStop(0.4, `rgba(79,195,247,${0.8 * a})`); grad.addColorStop(1, "rgba(79,195,247,0)");
          g.beginPath(); g.arc(sx, sy, s.r, 0, Math.PI * 2); g.fillStyle = grad; g.fill();
          // The target ring: dashed while off, solid green while on.
          if (!on) g.setLineDash([6, 6]);
          D.ring(g, VIEW.x, VIEW.y, tolC + 6, on && !played ? C.ok : played ? C.ok : C.amber, 3);
          g.setLineDash([]);
          if (kind === "lens" && hold > 0) {
            g.beginPath(); g.arc(VIEW.x, VIEW.y, VIEW.r + 12, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * (hold / HOLD_S));
            g.strokeStyle = C.ok; g.lineWidth = 8; g.stroke();
          }
        }
        // ---- The capacitor bank and the coupling's guide.
        const couplingRound = kind === "coupling" && !part;
        g.save(); g.globalAlpha = couplingRound ? 1 : 0.45;
        D.panel(g, 900, 440, 340, 250, 16, "#111926");
        D.text(g, "BANK", 1070, 466, 16, C.dim, "center", 700);
        for (let i = 0; i < 8; i++) {
          const x = 935 + i * 36, y0 = 490, h = 170;
          D.round(g, x, y0, 26, h, 5); g.fillStyle = "#1a2232"; g.fill();
          const lit = clamp(charge * 8 - i, 0, 1);
          if (lit > 0) { D.round(g, x, y0 + h * (1 - lit), 26, h * lit, 5); g.fillStyle = C.accent; g.fill(); }
        }
        // The socket on the bank's face.
        D.round(g, 880, GUIDE.y - 34, 30, 68, 6); g.fillStyle = plug.seated ? "#1f3a2a" : "#2a1c10"; g.fill();
        g.lineWidth = 3; g.strokeStyle = plug.seated ? C.ok : C.amber; g.stroke();
        // The guide rails.
        g.fillStyle = "#283246"; g.fillRect(GUIDE.x0 - 40, GUIDE.y - 42, GUIDE.home - GUIDE.x0 + 60, 6); g.fillRect(GUIDE.x0 - 40, GUIDE.y + 36, GUIDE.home - GUIDE.x0 + 60, 6);
        // The cable from the bulkhead to the plug.
        D.round(g, 40, GUIDE.y - 60, 60, 120, 10); g.fillStyle = "#1a2333"; g.fill(); g.strokeStyle = C.steel; g.lineWidth = 2; g.stroke();
        g.beginPath(); g.moveTo(100, GUIDE.y); g.bezierCurveTo(180, GUIDE.y + 30, plug.x - 160, GUIDE.y + 20, plug.x - 40, GUIDE.y);
        g.strokeStyle = C.copper; g.lineWidth = 12; g.stroke();
        // The plug.
        D.round(g, plug.x - 50, GUIDE.y - 30, 70, 60, 8); g.fillStyle = "#3b4558"; g.fill();
        g.lineWidth = 3; g.strokeStyle = plug.held ? C.amber : C.steel; g.stroke();
        for (const yy of [-14, 0, 14]) { g.fillStyle = "#c9a26b"; g.fillRect(plug.x + 20, GUIDE.y + yy - 3, 14, 6); }
        if (latchT > 0) D.ring(g, 895, GUIDE.y, 40 + 40 * (1 - latchT), `rgba(61,220,132,${latchT})`, 4);
        // The speed gauge: the band bracketed, past it hatched red, a marker for now.
        const gx0 = GUIDE.x0 - 40, gx1 = 860, gy = 668;
        const vx = (v) => gx0 + (gx1 - gx0) * clamp(v / VMAX_GAUGE, 0, 1);
        D.round(g, gx0, gy - 10, gx1 - gx0, 20, 10); g.fillStyle = "#151d2a"; g.fill();
        g.fillStyle = "rgba(61,220,132,0.35)"; g.fillRect(vx(band[0]), gy - 10, vx(band[1]) - vx(band[0]), 20);
        D.hatch(g, vx(band[1]), gy - 10, gx1 - vx(band[1]), 20, "rgba(255,71,87,0.7)");
        g.strokeStyle = C.ok; g.lineWidth = 3;
        for (const [bx, d] of [[vx(band[0]), 1], [vx(band[1]), -1]]) {
          g.beginPath(); g.moveTo(bx + 8 * d, gy - 16); g.lineTo(bx, gy - 16); g.lineTo(bx, gy + 16); g.lineTo(bx + 8 * d, gy + 16); g.stroke();
        }
        const mv = Math.max(0, plug.v);
        g.beginPath(); g.moveTo(vx(mv), gy - 12); g.lineTo(vx(mv) - 9, gy - 26); g.lineTo(vx(mv) + 9, gy - 26); g.closePath();
        g.fillStyle = mv > band[1] ? C.danger : mv >= band[0] ? C.ok : C.fg; g.fill();
        g.restore();
        // The arc: a jagged discharge from socket to bank.
        if (arc > 0) {
          g.strokeStyle = `rgba(190,159,230,${Math.min(1, arc * 1.5)})`; g.lineWidth = 4;
          for (let k = 0; k < 4; k++) {
            g.beginPath(); g.moveTo(880, GUIDE.y);
            let x = 880, y = GUIDE.y;
            for (let j = 0; j < 7; j++) { x += 30 + Math.random() * 20; y = GUIDE.y - 120 + Math.random() * 240; g.lineTo(x, clamp(y, 450, 690)); }
            g.stroke();
          }
        }
        // The part step: the crate with the new lens.
        if (part && !lensPart.set) {
          D.panel(g, 120, 408, 140, 134, 14, "#141b27");
          g.beginPath(); g.ellipse(lensPart.x, lensPart.y, 14, 60, 0, 0, Math.PI * 2);
          g.fillStyle = "rgba(160,220,255,0.5)"; g.fill(); g.lineWidth = 4; g.strokeStyle = "#cfe9ff"; g.stroke();
        }
      },
    };
  },
});
