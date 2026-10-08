/*
 * repairs/reactor.js: the reactor core's repair (openspec/changes/repair-minigames, design 2).
 *
 * The containment vessel from the front: the plasma ball floats inside the ring, held by four magnet coils (N, E, S,
 * W), while a fifth coil in the upper right slot is swapped out. The core's load pushes the plasma off centre and the
 * push wanders. Hold it in the centre band with two controls (owner, 2026-10-08: "should just be a vertical and
 * horizontal dial for simplicity"): the horizontal slider pulls the field left or right, the vertical one up or down,
 * the way its handle is pushed (drag them, or the arrows or W A S D). The coils glow with the pull. A step has two
 * stages (owner: "there should be a stage 2 to stabilize plasma ring too"): first the core, then the plasma ring round
 * it, which the load pulls out of round; the same two controls set the ring's width and height, held round in its
 * band. The ring bulging into the wall or collapsing onto the core is the same heat spike. Stage 1 is
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
    const PULL = 125;                       // px/s of push at a control's end stop
    const TRIM_RATE = 1.5;                  // a control's travel a second from the stick (end to end in 1.3 s)
    const LAG = 0.35;                       // s: the plasma's lag behind the field
    const SLOT = -Math.PI / 4;              // the swapped coil's slot: upper right
    const SLOT_X = CX + Math.cos(SLOT) * COIL_R, SLOT_Y = CY + Math.sin(SLOT) * COIL_R;
    const COILS = [
      { name: "N", a: -Math.PI / 2, dx: 0, dy: -1 },
      { name: "E", a: 0, dx: 1, dy: 0 },
      { name: "S", a: Math.PI / 2, dx: 0, dy: 1 },
      { name: "W", a: Math.PI, dx: -1, dy: 0 },
    ];
    // The two controls: a vertical slider and a horizontal one, each centred at rest (-1 to 1).
    const VS = { x: 1010, y: 120, w: 64, h: 400 };
    const HS = { x: 892, y: 590, w: 300, h: 64 };
    let part, coil, ax, p, v, drift, hold, holdNeed, done, heat, cool, grab, trail, time;
    // Stage 2, the plasma ring: its shape error (0 round), smoothed, and its own hold.
    const RING = 112;                       // the ring's round radius, px
    const RING_BAND = 0.22;                 // how far out of round still counts as held
    const RING_GAIN = 0.4;                  // the radius change per unit of shape error (a control at its stop plus the load can breach)
    let stage, ring, ringLoad, ringHold, ringNeed;
    const ringErr = (tt) => [ringLoad.m * Math.sin(ringLoad.w1 * tt + ringLoad.p1) + ax.x, ringLoad.m * Math.cos(ringLoad.w2 * tt + ringLoad.p2) + ax.y];

    const sliderAt = (x, y) => {
      if (x >= VS.x - 16 && x <= VS.x + VS.w + 16 && y >= VS.y - 20 && y <= VS.y + VS.h + 20) return "v";
      if (x >= HS.x - 20 && x <= HS.x + HS.w + 20 && y >= HS.y - 16 && y <= HS.y + HS.h + 16) return "h";
      return null;
    };
    /** Each coil's pull from the two controls (N, E, S, W): it glows with it. */
    const coilPull = () => [(1 - ax.y) / 2, (1 + ax.x) / 2, (1 + ax.y) / 2, (1 - ax.x) / 2];
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
        ax = { x: 0, y: 0 };
        p = { x: 0, y: 0 }; v = { x: 0, y: 0 };
        drift = { a0: r() * Math.PI * 2, w: (0.3 + 0.1 * index) * (r() < 0.5 ? -1 : 1), m: 38 + 14 * index, p1: r() * 6.3, p2: r() * 6.3 };
        hold = 0; holdNeed = 5 + index; done = false;
        stage = 1; ring = { x: 0, y: 0 }; ringHold = 0; ringNeed = 4 + index;
        ringLoad = { m: 0.55 + 0.15 * index, w1: 0.45 + 0.1 * index, w2: 0.37 + 0.09 * index, p1: r() * 6.3, p2: r() * 6.3 };
        heat = 0; cool = 0; grab = null; trail = []; time = 0;
      },
      /** For tools (shots and tests): the round's state, read only. */
      peek() { return { part, stage, p: { ...p }, ax: { ...ax }, ring: { ...ring }, hold: hold / holdNeed, ringHold: ringHold / ringNeed, done, coilSet: coil.set }; },
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
        // The two controls: the stick moves them, a drag sets the one it holds.
        const clamp = (x) => Math.max(-1, Math.min(1, x));
        if (input.stick.x) ax.x = clamp(ax.x + input.stick.x * TRIM_RATE * dt);
        if (input.stick.y) ax.y = clamp(ax.y + input.stick.y * TRIM_RATE * dt);
        if (input.pressed) grab = sliderAt(input.x, input.y);
        if (grab === "v" && input.down) ax.y = clamp((input.y - (VS.y + VS.h / 2)) / (VS.h / 2));
        if (grab === "h" && input.down) ax.x = clamp((input.x - (HS.x + HS.w / 2)) / (HS.w / 2));
        if (input.released || !input.down) grab = null;
        if (done) {
          // Swapped: the field settles the plasma home.
          p.x *= 1 - Math.min(1, dt * 2); p.y *= 1 - Math.min(1, dt * 2);
          return;
        }
        // The plasma follows the field (the load plus the coils) with a short lag; in stage 2 the core is held.
        const [lx, ly] = stage === 1 ? load(time) : [-ax.x * PULL - p.x * 3, -ax.y * PULL - p.y * 3];
        const fx = lx + ax.x * PULL, fy = ly + ax.y * PULL;
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
        if (stage === 1 && off <= BAND) {
          hold += dt;
          // The core is held: on to the ring, the controls back to centre.
          if (hold >= holdNeed) { hold = holdNeed; stage = 2; ax = { x: 0, y: 0 }; time = 0; }
        }
        if (stage === 2) {
          const [ex, ey] = ringErr(time);
          ring.x += (ex - ring.x) * Math.min(1, dt / LAG); ring.y += (ey - ring.y) * Math.min(1, dt / LAG);
          const rx = RING * (1 + RING_GAIN * ring.x), ry = RING * (1 + RING_GAIN * ring.y);
          if ((Math.max(rx, ry) >= WALL - 6 || Math.min(rx, ry) <= BALL * 2) && cool <= 0) {
            heat = 1; cool = 1; ring.x *= 0.4; ring.y *= 0.4;
            api.fumble(Math.max(rx, ry) >= WALL - 6 ? "Plasma ring touched the wall: heat spike" : "Plasma ring collapsed on the core: heat spike");
            return;
          }
          if (Math.abs(ring.x) < RING_BAND && Math.abs(ring.y) < RING_BAND) {
            ringHold += dt;
            if (ringHold >= ringNeed) { ringHold = ringNeed; done = true; api.stepDone(); }
          }
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
        const pull = coilPull();
        COILS.forEach((c, i) => {
          const hx = CX + Math.cos(c.a) * COIL_R, hy = CY + Math.sin(c.a) * COIL_R;
          const k = live ? pull[i] : 0;
          g.save(); g.translate(hx, hy); g.rotate(c.a + Math.PI / 2);
          D.panel(g, -60, -28, 120, 56, 10, "#1b2433", "#34404f");
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
          const fill = part ? 1 : 0.25 + 0.75 * ((hold / holdNeed + ringHold / ringNeed) / 2);
          for (let s = -48; s <= 44; s += 8) {
            g.fillStyle = (s + 48) / 96 <= fill ? C.copper : "#3a2a1c"; g.fillRect(s, -18, 5, 36);
          }
        }
        g.restore();
        if (!part) {
          D.ring(g, SLOT_X, SLOT_Y, 74, "#1d2636", 8);
          // Two halves: the core's hold, then the ring's.
          g.beginPath(); g.arc(SLOT_X, SLOT_Y, 74, -Math.PI / 2, -Math.PI / 2 + Math.PI * ((hold / holdNeed) + (ringHold / ringNeed)));
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
          // Stage 2: the plasma ring, its target band dashed round it (green while held), a pip for each stage.
          if (stage === 2) {
            const held = Math.abs(ring.x) < RING_BAND && Math.abs(ring.y) < RING_BAND;
            g.setLineDash([8, 7]);
            for (const k of [1 - RING_GAIN * RING_BAND, 1 + RING_GAIN * RING_BAND]) D.ring(g, CX, CY, RING * k, held ? C.ok : "#3b4a5e", 2);
            g.setLineDash([]);
            const rx = RING * (1 + RING_GAIN * ring.x), ry = RING * (1 + RING_GAIN * ring.y);
            const wob = 1 + 0.02 * Math.sin(t * 17);
            for (const [w, col] of [[22, "rgba(190,159,230,0.18)"], [12, "rgba(190,159,230,0.55)"], [4, "rgba(240,230,255,0.95)"]]) {
              g.beginPath(); g.ellipse(CX, CY, rx * wob, ry * wob, 0, 0, Math.PI * 2); g.strokeStyle = heat > 0 ? `rgba(255,120,120,${0.4 + 0.5 * heat})` : col; g.lineWidth = w; g.stroke();
            }
          }
          for (let k = 0; k < 2; k++) {
            const on = stage > k + 1 || done, cur = stage === k + 1 && !done;
            D.disc(g, SLOT_X - 14 + k * 28, SLOT_Y + 100, 8, on ? C.ok : cur ? C.amber : "#2a3446");
          }
          const bx = CX + p.x, by = CY + p.y;
          const flick = 1 + 0.06 * Math.sin(t * 23) + 0.04 * Math.sin(t * 37);
          const glow = g.createRadialGradient(bx, by, 0, bx, by, BALL * 2.6 * flick);
          glow.addColorStop(0, "rgba(255,255,255,1)");
          glow.addColorStop(0.25, "rgba(225,205,255,0.95)");
          glow.addColorStop(0.45, heat > 0 ? "rgba(255,120,120,0.6)" : "rgba(190,159,230,0.55)");
          glow.addColorStop(1, "rgba(79,195,247,0)");
          D.disc(g, bx, by, BALL * 2.6 * flick, glow);
        }

        // The two controls: a vertical and a horizontal slider, a centre mark, the pull as a fill from the centre to
        // the handle, arrows at the ends for the way the field pulls.
        if (live) {
          const slider = (S, vertical, val, on) => {
            D.round(g, S.x, S.y, S.w, S.h, 14); g.fillStyle = "#121a25"; g.fill();
            g.lineWidth = 2; g.strokeStyle = on ? C.amber : C.line; g.stroke();
            const len = vertical ? S.h : S.w, mid = len / 2, pos = mid + val * (mid - 22);
            g.fillStyle = "rgba(79,195,247,0.35)";
            if (vertical) g.fillRect(S.x + 12, S.y + Math.min(mid, pos), S.w - 24, Math.abs(pos - mid));
            else g.fillRect(S.x + Math.min(mid, pos), S.y + 12, Math.abs(pos - mid), S.h - 24);
            g.fillStyle = "rgba(232,238,246,0.5)";
            if (vertical) g.fillRect(S.x + 6, S.y + mid - 1, S.w - 12, 3); else g.fillRect(S.x + mid - 1, S.y + 6, 3, S.h - 12);
            // Arrows at both ends: the way the plasma is pulled with the handle there.
            const arrow = (x, y, a) => {
              g.save(); g.translate(x, y); g.rotate(a); g.beginPath(); g.moveTo(10, 0); g.lineTo(-6, -9); g.lineTo(-6, 9); g.closePath();
              g.fillStyle = "#4a5568"; g.fill(); g.restore();
            };
            if (vertical) { arrow(S.x + S.w / 2, S.y - 18, -Math.PI / 2); arrow(S.x + S.w / 2, S.y + S.h + 18, Math.PI / 2); }
            else { arrow(S.x - 18, S.y + S.h / 2, Math.PI); arrow(S.x + S.w + 18, S.y + S.h / 2, 0); }
            // The handle.
            if (vertical) { D.round(g, S.x - 8, S.y + pos - 14, S.w + 16, 28, 9); }
            else { D.round(g, S.x + pos - 14, S.y - 8, 28, S.h + 16, 9); }
            g.fillStyle = on ? C.amber : "#c9d3e0"; g.fill();
            g.fillStyle = "#1b2433";
            if (vertical) g.fillRect(S.x + 6, S.y + pos - 2, S.w - 12, 4); else g.fillRect(S.x + pos - 2, S.y + 6, 4, S.h - 12);
          };
          slider(VS, true, ax.y, grab === "v");
          slider(HS, false, ax.x, grab === "h");
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
