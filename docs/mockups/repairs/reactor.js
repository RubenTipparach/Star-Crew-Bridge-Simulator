/*
 * repairs/reactor.js: the reactor core's repair (openspec/changes/repair-minigames, design 2).
 *
 * The containment vessel from the front: the plasma ball floats inside the ring, held by four magnet coils (N, E, S,
 * W), while a fifth coil in the upper right slot is swapped out. The core's load pushes the plasma off centre and the
 * push wanders. Trim the coils to hold it in the centre band: the stick (arrows or W A S D) raises the coil on that
 * side and lowers the opposite one, or drag the four coil sliders. A raised coil pulls the plasma towards it. A step is
 * the swap's ring filling, which it does only while the plasma sits in the band; the next step's load is heavier and
 * wanders faster. The plasma touching the wall is a fumble: a heat spike. A disabled core's first step fits the new
 * coil: drag it from the crate into the open slot, with the core cold.
 */
RepairKit.register({
  id: "reactor",
  title: "Reactor core",
  place: "Engineering, deck C/B",
  group: "Engineering",
  hazard: "Plasma touched the wall: heat spike",
  down: "The ship runs on batteries (power-grid)",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const CX = 520, CY = 400;
    const VESSEL = 226;                     // the vessel's outer radius, px
    const WALL = 178;                       // the chamber's inner wall, px
    const BALL = 24;                        // the plasma's radius, px
    const BAND = 46;                        // the centre band the swap needs, px
    const COIL_R = 258;                     // where the coil housings sit, px from the centre
    const PULL = 125;                       // px/s of push for a full trim difference across a pair
    const TRIM_RATE = 0.75;                 // trim a second from the stick
    const LAG = 0.35;                       // s: the plasma's lag behind the field
    const SLOT = -Math.PI / 4;              // the swapped coil's slot: upper right
    const SLOT_X = CX + Math.cos(SLOT) * COIL_R, SLOT_Y = CY + Math.sin(SLOT) * COIL_R;
    const COILS = [
      { name: "N", a: -Math.PI / 2, dx: 0, dy: -1 },
      { name: "E", a: 0, dx: 1, dy: 0 },
      { name: "S", a: Math.PI / 2, dx: 0, dy: 1 },
      { name: "W", a: Math.PI, dx: -1, dy: 0 },
    ];
    const SL = { x: 905, y: 170, w: 58, h: 380, gap: 86 };   // the four coil sliders
    let part, coil, trim, p, v, drift, hold, holdNeed, done, heat, cool, grab, trail, time;

    const sliderAt = (x, y) => {
      for (let i = 0; i < 4; i++) {
        const sx = SL.x + i * SL.gap;
        if (x >= sx - 14 && x <= sx + SL.w + 14 && y >= SL.y - 20 && y <= SL.y + SL.h + 20) return i;
      }
      return -1;
    };
    const load = (t) => {
      const m = drift.m * (1 + 0.35 * Math.sin(1.3 * t + drift.p1));
      const a = drift.a0 + drift.w * t + 0.6 * Math.sin(0.7 * t + drift.p2);
      return [Math.cos(a) * m, Math.sin(a) * m];
    };

    return {
      step(index, isPart) {
        const r = api.rand();
        part = isPart;
        coil = { x: 1080, y: 420, held: false, set: false };
        trim = [0.5, 0.5, 0.5, 0.5];
        p = { x: 0, y: 0 }; v = { x: 0, y: 0 };
        drift = { a0: r() * Math.PI * 2, w: (0.3 + 0.1 * index) * (r() < 0.5 ? -1 : 1), m: 38 + 14 * index, p1: r() * 6.3, p2: r() * 6.3 };
        hold = 0; holdNeed = 8 + 1.5 * index; done = false;
        heat = 0; cool = 0; grab = -1; trail = []; time = 0;
      },
      /** For tools (shots and tests): the round's state, read only. */
      peek() { return { part, p: { ...p }, trim: [...trim], hold: hold / holdNeed, done, coilSet: coil.set }; },
      update(dt, input) {
        heat = Math.max(0, heat - dt * 0.6);
        cool = Math.max(0, cool - dt);
        if (part && !coil.set) {
          // The part step: carry the coil to the open slot.
          if (input.pressed && Math.hypot(input.x - coil.x, input.y - coil.y) < 50) coil.held = true;
          if (coil.held && input.down) { coil.x = input.x; coil.y = input.y; }
          if (coil.held && input.released) {
            coil.held = false;
            if (Math.hypot(coil.x - SLOT_X, coil.y - SLOT_Y) < 50) { coil.set = true; coil.x = SLOT_X; coil.y = SLOT_Y; api.stepDone(); }
          }
          return;
        }
        if (part) return;
        time += dt;
        // Trims: the stick raises the coil on its side and lowers the opposite one; the sliders set one directly.
        const sx = input.stick.x, sy = input.stick.y;
        const clamp = (x) => Math.max(0, Math.min(1, x));
        if (sx) { trim[1] = clamp(trim[1] + sx * TRIM_RATE * dt); trim[3] = clamp(trim[3] - sx * TRIM_RATE * dt); }
        if (sy) { trim[2] = clamp(trim[2] + sy * TRIM_RATE * dt); trim[0] = clamp(trim[0] - sy * TRIM_RATE * dt); }
        if (input.pressed) grab = sliderAt(input.x, input.y);
        if (grab >= 0 && input.down) trim[grab] = clamp(1 - (input.y - SL.y) / SL.h);
        if (input.released || !input.down) grab = -1;
        if (done) {
          // Swapped: the field settles the plasma home.
          p.x *= 1 - Math.min(1, dt * 2); p.y *= 1 - Math.min(1, dt * 2);
          return;
        }
        // The plasma follows the field (the load plus the coils) with a short lag.
        const [lx, ly] = load(time);
        const fx = lx + (trim[1] - trim[3]) * PULL, fy = ly + (trim[2] - trim[0]) * PULL;
        v.x += (fx - v.x) * Math.min(1, dt / LAG); v.y += (fy - v.y) * Math.min(1, dt / LAG);
        p.x += v.x * dt; p.y += v.y * dt;
        trail.push([p.x, p.y]); if (trail.length > 14) trail.shift();
        const off = Math.hypot(p.x, p.y);
        if (off + BALL >= WALL && cool <= 0) {
          heat = 1; cool = 1;
          p.x *= 0.5; p.y *= 0.5; v.x = 0; v.y = 0; trail = [];
          api.fumble("Plasma touched the wall: heat spike");
          return;
        }
        if (off + BALL > WALL) { const k = (WALL - BALL) / off; p.x *= k; p.y *= k; }
        if (off <= BAND) {
          hold += dt;
          if (hold >= holdNeed) { hold = holdNeed; done = true; api.stepDone(); }
        }
      },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        const live = !part;
        const inBand = live && Math.hypot(p.x, p.y) <= BAND;

        // The core's temperature: a column on the left, its top band hatched red.
        const TX = 96, TY = 150, TH = 470;
        D.panel(g, TX - 26, TY - 16, 52, TH + 32, 14, "#0c121a");
        g.save(); D.round(g, TX - 12, TY, 24, TH, 12); g.clip();
        g.fillStyle = "#1a2230"; g.fillRect(TX - 12, TY, 24, TH);
        g.strokeStyle = "rgba(255,71,87,0.55)"; g.lineWidth = 3;
        for (let y = TY - 24; y < TY + TH * 0.2; y += 12) { g.beginPath(); g.moveTo(TX - 12, y + 24); g.lineTo(TX + 12, y); g.stroke(); }
        const temp = live ? 0.42 + 0.06 * Math.sin(t * 0.8) + 0.5 * heat : 0.08;
        const ty = TY + TH * (1 - temp);
        g.fillStyle = temp > 0.8 ? C.danger : temp > 0.6 ? C.warn : C.amber; g.fillRect(TX - 12, ty, 24, TY + TH - ty);
        g.restore();
        D.disc(g, TX, TY + TH + 2, 20, temp > 0.8 ? C.danger : C.amber);

        // The vessel: a thick steel ring with its bolts, the dark chamber inside.
        D.disc(g, CX, CY, VESSEL, "#141c28");
        D.ring(g, CX, CY, VESSEL, "#2b3646", 4);
        for (let i = 0; i < 24; i++) {
          const a = (i / 24) * Math.PI * 2 + Math.PI / 24;
          D.disc(g, CX + Math.cos(a) * (VESSEL - 18), CY + Math.sin(a) * (VESSEL - 18), 5, "#3a4656");
        }
        D.disc(g, CX, CY, WALL + 14, "#0e151f");
        D.disc(g, CX, CY, WALL, "#05080d");
        const wallHot = heat > 0 ? `rgba(255,71,87,${0.35 + 0.65 * heat})` : live ? "#2f4a66" : "#1d2633";
        D.ring(g, CX, CY, WALL, wallHot, 5);

        // The coils: housings outside the vessel, windings glowing with their trim, the field on the inner wall.
        COILS.forEach((c, i) => {
          const hx = CX + Math.cos(c.a) * COIL_R, hy = CY + Math.sin(c.a) * COIL_R;
          const k = live ? trim[i] : 0;
          g.save(); g.translate(hx, hy); g.rotate(c.a + Math.PI / 2);
          D.panel(g, -60, -28, 120, 56, 10, "#1b2433", grab === i ? C.amber : "#34404f");
          for (let s = -48; s <= 44; s += 8) {
            g.fillStyle = `rgba(208,138,74,${0.35 + 0.65 * k})`; g.fillRect(s, -18, 5, 36);
          }
          g.restore();
          g.beginPath(); g.arc(CX, CY, WALL - 6, c.a - 0.45, c.a + 0.45);
          g.strokeStyle = `rgba(79,195,247,${0.08 + 0.55 * k})`; g.lineWidth = 6 + 10 * k; g.stroke();
          const lx = CX + Math.cos(c.a) * (COIL_R + 52), ly = CY + Math.sin(c.a) * (COIL_R + 52);
          if (c.dx) D.text(g, c.name, lx, ly, 22, C.dim, "center", 700);
          else D.text(g, c.name, hx - 82, hy, 22, C.dim, "center", 700);
        });

        // The fifth coil's slot: the swap's ring fills while the plasma is held centred.
        g.save(); g.translate(SLOT_X, SLOT_Y); g.rotate(SLOT + Math.PI / 2);
        D.panel(g, -60, -28, 120, 56, 10, part && !coil.set ? "#120d08" : "#1b2433", part && !coil.set ? C.amber : "#34404f");
        if (!part || coil.set) {
          const fill = part ? 1 : 0.25 + 0.75 * (hold / holdNeed);
          for (let s = -48; s <= 44; s += 8) {
            g.fillStyle = (s + 48) / 96 <= fill ? C.copper : "#3a2a1c"; g.fillRect(s, -18, 5, 36);
          }
        }
        g.restore();
        if (!part) {
          D.ring(g, SLOT_X, SLOT_Y, 74, "#1d2636", 8);
          g.beginPath(); g.arc(SLOT_X, SLOT_Y, 74, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * (hold / holdNeed));
          g.strokeStyle = done ? C.ok : C.amber; g.lineWidth = 8; g.lineCap = "round"; g.stroke(); g.lineCap = "butt";
        }

        // The centre band and the plasma.
        g.setLineDash([10, 8]);
        D.ring(g, CX, CY, BAND, inBand ? C.ok : "#3b4a5e", 3);
        g.setLineDash([]);
        g.strokeStyle = "rgba(111,127,148,0.25)"; g.lineWidth = 1;
        g.beginPath(); g.moveTo(CX - WALL, CY); g.lineTo(CX + WALL, CY); g.moveTo(CX, CY - WALL); g.lineTo(CX, CY + WALL); g.stroke();
        if (live) {
          trail.forEach(([x, y], i) => D.disc(g, CX + x, CY + y, BALL * (i / trail.length) * 0.8, `rgba(190,159,230,${0.05 + 0.12 * (i / trail.length)})`));
          const bx = CX + p.x, by = CY + p.y;
          const flick = 1 + 0.06 * Math.sin(t * 23) + 0.04 * Math.sin(t * 37);
          const glow = g.createRadialGradient(bx, by, 0, bx, by, BALL * 2.6 * flick);
          glow.addColorStop(0, "rgba(255,255,255,1)");
          glow.addColorStop(0.25, "rgba(225,205,255,0.95)");
          glow.addColorStop(0.45, heat > 0 ? "rgba(255,120,120,0.6)" : "rgba(190,159,230,0.55)");
          glow.addColorStop(1, "rgba(79,195,247,0)");
          D.disc(g, bx, by, BALL * 2.6 * flick, glow);
        }

        // The coil sliders: a track each, the trim as a fill and a handle.
        if (live) {
          for (let i = 0; i < 4; i++) {
            const sx = SL.x + i * SL.gap;
            D.round(g, sx, SL.y, SL.w, SL.h, 12); g.fillStyle = "#121a25"; g.fill();
            g.lineWidth = 2; g.strokeStyle = grab === i ? C.amber : C.line; g.stroke();
            const hy = SL.y + SL.h * (1 - trim[i]);
            D.round(g, sx + 8, hy, SL.w - 16, SL.y + SL.h - hy, 8); g.fillStyle = "rgba(208,138,74,0.45)"; g.fill();
            g.fillStyle = "rgba(111,127,148,0.4)"; g.fillRect(sx + 4, SL.y + SL.h / 2 - 1, SL.w - 8, 2);
            D.round(g, sx - 6, hy - 12, SL.w + 12, 24, 8); g.fillStyle = grab === i ? C.amber : "#c9d3e0"; g.fill();
            g.fillStyle = "#1b2433"; g.fillRect(sx + 6, hy - 2, SL.w - 12, 4);
            D.text(g, COILS[i].name, sx + SL.w / 2, SL.y + SL.h + 40, 24, C.fg, "center", 700);
          }
        }

        // The part: the new coil in its crate, or in the hand.
        if (part && !coil.set) {
          D.panel(g, 1000, 350, 160, 140, 14, "#141b27");
          D.disc(g, coil.x, coil.y, 38, C.copper);
          D.ring(g, coil.x, coil.y, 38, "#f0c08a", 3);
          D.disc(g, coil.x, coil.y, 16, "#141b27");
          D.ring(g, coil.x, coil.y, 26, "#7a4a22", 5);
        }
      },
    };
  },
});
