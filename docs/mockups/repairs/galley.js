/*
 * repairs/galley.js: the galley synthesizer's repair (openspec/changes/repair-minigames, design 2).
 *
 * The recipe card shows four nutrient columns and the band each must reach; the same bands are bracketed on the
 * synthesizer's four glass columns. Hold a column's valve to fill it (the pointer, keys 1 to 4, or the arrows and
 * Space); the feed runs on briefly after the valve is let go, so let go early. All four in their bands lights PURGE:
 * press it (or Space) and the mix runs down into the bowl. A step is one recipe balanced; later steps fill faster,
 * run on longer and narrow the bands. Past a band's top: a fumble, paste everywhere, that column drains. A disabled
 * synthesizer's first step fits the new dispenser nozzle: drag it from the crate onto the dispenser.
 */
RepairKit.register({
  id: "galley",
  title: "Galley synthesizer",
  place: "Mess",
  group: "Life",
  hazard: "Paste everywhere: the column drains",
  // The how-to card (repair-minigames 6g), drawn by the kit: pictures and a few words, on demand.
  guide: {
    steps: [
      { icon: "hold", text: "Hold a valve to fill" },
      { icon: "band", text: "Let go early: it runs on" },
      { icon: "tap", text: "All in band: PURGE" },
    ],
    mistake: "Past a band's top: paste everywhere, that column drains",
    now: (q) => ({ fill: 0, purge: 2 })[q.phase] ?? -1,
  },
  down: "Hunger between missions (design 4)",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const SPILL = "Paste everywhere: the column drains";
    const MACH = { x: 320, y: 100, w: 620, h: 590 };      // the synthesizer
    const COLX = [440, 580, 720, 860], CW = 92;           // the glass columns' centres and width, px
    const TOP = 140, BOT = 500, CH = BOT - TOP;           // a column's glass, px
    const VY = 572, VR = 42, MANI = 650;                  // valves, and the manifold to the dispenser
    const NAMES = ["PRO", "CARB", "FAT", "VIT"];
    const COLS = [C.copper, "#e6d6a8", C.warn, C.lilac];
    const CARD = { x: 40, y: 112, w: 240, h: 290 };       // the recipe card
    const DISP = { x: 40, y: 425, w: 240, h: 265 };       // the dispenser hatch
    const NOZ = { x: 160, y: 470 };                       // its nozzle
    const PURGE = { x: 1160, y: 330, r: 72 };
    const CRATE = { x: 1090, y: 560, w: 140, h: 115 };    // the spare nozzle's crate (part step)
    const DRAIN = 0.7;                                    // a spilled column drains at this, column heights a second
    let s;

    const level = (c, L) => BOT - L * CH;
    const inBand = (col) => Math.abs(col.L - col.c) <= col.w;
    const settled = () => s.cols.every((col) => inBand(col) && col.f < 0.004 && !col.drain);

    function partStep(dt, input) {
      const p = s.noz;
      if (input.stick.x || input.stick.y) { p.x += input.stick.x * 480 * dt; p.y += input.stick.y * 480 * dt; p.pad = true; }
      if (input.pressed && Math.hypot(input.x - p.x, input.y - p.y) < 50) p.held = true;
      if (p.held && input.down) { p.x = input.x; p.y = input.y; }
      if ((p.held && input.released) || (p.pad && input.actionPressed)) {
        p.held = false; p.pad = false;
        if (Math.hypot(p.x - NOZ.x, p.y - NOZ.y) < 40) { p.set = true; p.x = NOZ.x; p.y = NOZ.y; s.phase = "fitted"; api.stepDone(); }
      }
    }

    function fill(dt, input) {
      if (input.hit.has("ArrowLeft") || input.hit.has("KeyA")) { s.sel = (s.sel + 3) % 4; s.keys = true; }
      if (input.hit.has("ArrowRight") || input.hit.has("KeyD")) { s.sel = (s.sel + 1) % 4; s.keys = true; }
      const ready = settled();
      const purge = (input.pressed && Math.hypot(input.x - PURGE.x, input.y - PURGE.y) < PURGE.r) || (ready && input.actionPressed);
      if (purge && ready) { s.phase = "purge"; s.purgeT = 0; api.stepDone(); return; }
      const rate = 0.16 + 0.03 * s.index, tau = 0.22 + 0.06 * s.index;
      for (let i = 0; i < 4; i++) {
        const col = s.cols[i];
        const held = (input.down && Math.hypot(input.x - COLX[i], input.y - VY) < VR + 10) ||
          input.keys.has("Digit" + (i + 1)) || (input.action && !ready && s.sel === i);
        if (input.action) s.keys = true;
        col.open = held && !col.drain;
        if (col.drain) { col.L = Math.max(0, col.L - DRAIN * dt); col.f = 0; if (col.L === 0) col.drain = false; continue; }
        // The feed: up to the rate while held, running on and dying away after.
        col.f = col.open ? col.f + (rate - col.f) * Math.min(1, dt / 0.08) : col.f * Math.exp(-dt / tau);
        col.L += col.f * dt;
        if (col.L > col.c + col.w) {
          col.drain = true; col.f = 0;
          for (let k = 0; k < 16; k++) s.splats.push({ x: COLX[i] + (k - 8) * 4, y: level(col, col.L), vx: (Math.sin(k * 7.1) * 0.5 + (k - 8) / 8) * 260, vy: -150 - 160 * Math.abs(Math.cos(k * 3.3)), c: COLS[i], life: 2.2, land: 0 });
          api.fumble(SPILL); return;
        }
      }
    }

    // ---------------------------------------------------------------- drawing
    function pipe(g, pts, w = 16) {
      g.beginPath(); pts.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y)));
      g.lineJoin = "round"; g.lineCap = "round";
      g.strokeStyle = "#232c3b"; g.lineWidth = w + 6; g.stroke();
      g.strokeStyle = "#3a4658"; g.lineWidth = w; g.stroke();
      g.lineCap = "butt";
    }
    function nozzle(g, x, y) {
      g.fillStyle = C.steel; g.beginPath(); g.moveTo(x - 26, y - 22); g.lineTo(x + 26, y - 22); g.lineTo(x + 12, y + 18); g.lineTo(x - 12, y + 18); g.closePath(); g.fill();
      g.fillStyle = "#4a5566"; g.fillRect(x - 30, y - 30, 60, 10);
      g.fillStyle = "#1a2230"; g.fillRect(x - 6, y + 12, 12, 8);
    }
    function bracket(g, x, y0, y1, side, col) {
      g.beginPath(); g.moveTo(x + side * 14, y0); g.lineTo(x, y0); g.lineTo(x, y1); g.lineTo(x + side * 14, y1);
      g.strokeStyle = col; g.lineWidth = 5; g.stroke();
    }

    return {
      get state() { return s; },   // for tools: the headless checks read the round
      step(index, isPart) {
        const r = api.rand();
        const w = Math.max(0.04, 0.075 - 0.01 * index);
        s = {
          index, phase: isPart ? "part" : "fill", sel: 0, keys: false, purgeT: 0, splats: [],
          noz: { x: CRATE.x + CRATE.w / 2, y: CRATE.y + CRATE.h / 2 + 8, held: false, pad: false, set: !isPart },
          cols: COLX.map(() => ({ c: 0.35 + 0.5 * r(), w, L: 0.02 + 0.1 * r(), f: 0, open: false, drain: false })),
        };
      },
      update(dt, input) {
        for (const p of s.splats) {
          p.life -= dt;
          if (!p.land) { p.vy += 900 * dt; p.x += p.vx * dt; p.y += p.vy * dt; if (p.y > 640 && p.vy > 0) { p.land = 1; p.y = 640 + (p.x % 30); } }
        }
        s.splats = s.splats.filter((p) => p.life > 0);
        if (s.phase === "part") partStep(dt, input);
        else if (s.phase === "fill") fill(dt, input);
        else if (s.phase === "purge") {
          s.purgeT += dt;
          for (const col of s.cols) { col.f = 0; col.open = false; col.L = Math.max(0, col.L - 0.8 * dt); }
        }
      },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        const ready = s.phase === "fill" && settled();
        const purging = s.phase === "purge";
        // The manifold from the columns' feet to the dispenser.
        pipe(g, [[COLX[3], MANI], [DISP.x + DISP.w - 10, MANI]]);
        // The synthesizer's body.
        D.panel(g, MACH.x, MACH.y, MACH.w, MACH.h, 18, "#101722");
        g.fillStyle = "#0d131c"; g.fillRect(MACH.x + 20, MACH.y + 16, MACH.w - 40, 10);
        COLX.forEach((x, i) => {
          const col = s.cols[i], good = inBand(col), over = col.drain;
          // Glass, graduations, the band, the paste.
          D.round(g, x - CW / 2, TOP, CW, CH, 14); g.fillStyle = "#060a10"; g.fill();
          g.fillStyle = "rgba(61,220,132,0.10)"; g.fillRect(x - CW / 2 + 4, level(col, col.c + col.w), CW - 8, col.w * 2 * CH);
          g.save(); D.round(g, x - CW / 2, TOP, CW, CH, 14); g.clip();
          const y = level(col, col.L);
          g.fillStyle = COLS[i]; g.globalAlpha = 0.9; g.fillRect(x - CW / 2, y, CW, BOT - y);
          g.globalAlpha = 1; g.fillStyle = "rgba(255,255,255,0.25)"; g.fillRect(x - CW / 2, y, CW, 4);
          if (col.open || col.f > 0.01) { g.fillStyle = COLS[i]; g.fillRect(x - 5, y, 10, BOT - y); }
          g.restore();
          for (let k = 1; k < 10; k++) { g.fillStyle = "#2a3446"; g.fillRect(x - CW / 2, BOT - (k * CH) / 10, k % 5 ? 10 : 18, 2); }
          D.round(g, x - CW / 2, TOP, CW, CH, 14); g.lineWidth = 3; g.strokeStyle = over ? C.danger : "#3a4658"; g.stroke();
          const bc = over ? C.danger : good ? C.ok : "#2f7a50";
          bracket(g, x - CW / 2 - 8, level(col, col.c + col.w), level(col, col.c - col.w), 1, bc);
          bracket(g, x + CW / 2 + 8, level(col, col.c + col.w), level(col, col.c - col.w), -1, bc);
          // In band and still: a check on the cap. Spilling: a cross.
          D.round(g, x - CW / 2 - 6, TOP - 26, CW + 12, 26, 8); g.fillStyle = C.steel; g.fill();
          D.text(g, NAMES[i], x - 10, TOP - 13, 18, "#0b111b", "center", 700);
          if (good && col.f < 0.004 && !over) {
            D.disc(g, x + 32, TOP - 13, 10, C.ok);
            g.beginPath(); g.moveTo(x + 27, TOP - 13); g.lineTo(x + 31, TOP - 9); g.lineTo(x + 37, TOP - 17); g.strokeStyle = "#0b111b"; g.lineWidth = 3; g.stroke();
          }
          // The valve under it.
          pipe(g, [[x, BOT], [x, MANI]], 12);
          D.disc(g, x, VY, VR, col.open ? COLS[i] : "#1d2738");
          D.ring(g, x, VY, VR, col.open ? "#fff3d6" : C.steel, 4);
          D.disc(g, x, VY, VR - 16, col.open ? "#fff3d6" : "#2a3446");
          if (s.keys && s.sel === i && s.phase === "fill") D.ring(g, x, VY, VR + 9, C.amber, 3);
        });
        // PURGE: the main action, lit once all four sit in their bands.
        D.disc(g, PURGE.x, PURGE.y, PURGE.r + 10, "#0d131c");
        D.disc(g, PURGE.x, PURGE.y, PURGE.r, ready ? C.amber : purging ? "#5a3a1a" : "#1d2738");
        D.ring(g, PURGE.x, PURGE.y, PURGE.r, ready ? "#ffd9a8" : C.steel, 4);
        g.beginPath(); g.moveTo(PURGE.x - 24, PURGE.y - 26); g.lineTo(PURGE.x + 24, PURGE.y - 26); g.lineTo(PURGE.x, PURGE.y + 4); g.closePath();
        g.fillStyle = ready ? "#1a0f04" : "#3a4658"; g.fill();
        D.text(g, "PURGE", PURGE.x, PURGE.y + 30, 22, ready ? "#1a0f04" : C.dim, "center", 700);
        // The recipe card: the four bands at a glance.
        D.round(g, CARD.x, CARD.y, CARD.w, CARD.h, 10); g.fillStyle = "#d9d2bf"; g.fill();
        g.fillStyle = "#8a8170"; g.fillRect(CARD.x + 16, CARD.y + 18, CARD.w - 32, 6);
        s.cols.forEach((col, i) => {
          const x = CARD.x + 30 + i * 52, y0 = CARD.y + 50, h = CARD.h - 80;
          g.fillStyle = "#bdb5a0"; g.fillRect(x, y0, 32, h);
          g.fillStyle = COLS[i]; g.fillRect(x, y0 + h * (1 - col.c - col.w), 32, h * col.w * 2);
          g.strokeStyle = "#3b352a"; g.lineWidth = 2; g.strokeRect(x, y0 + h * (1 - col.c - col.w), 32, h * col.w * 2);
          g.strokeRect(x, y0, 32, h);
        });
        // The dispenser: the nozzle and the bowl.
        D.panel(g, DISP.x, DISP.y, DISP.w, DISP.h, 14, "#0d131c");
        D.round(g, DISP.x + 24, DISP.y + 70, DISP.w - 48, DISP.h - 90, 10); g.fillStyle = "#05080d"; g.fill();
        if (purging) {
          const pour = s.purgeT < 1.4;
          if (pour) { g.fillStyle = "#c9a36a"; g.fillRect(NOZ.x - 6, NOZ.y + 18, 12, 640 - NOZ.y - 18); }
          const m = Math.min(1, s.purgeT / 1.4);
          g.beginPath(); g.ellipse(NOZ.x, 640, 50 * m + 1, 22 * m + 1, 0, Math.PI, 0); g.fillStyle = "#c9a36a"; g.fill();
        }
        g.beginPath(); g.moveTo(NOZ.x - 70, 636); g.lineTo(NOZ.x + 70, 636); g.lineTo(NOZ.x + 52, 670); g.lineTo(NOZ.x - 52, 670); g.closePath();
        g.fillStyle = "#7d8796"; g.fill();
        if (s.noz.set) nozzle(g, NOZ.x, NOZ.y);
        else { D.ring(g, NOZ.x, NOZ.y, 30, C.amber, 4); D.disc(g, NOZ.x, NOZ.y, 26, "#120d08"); }
        // Paste on everything.
        for (const p of s.splats) {
          g.globalAlpha = Math.min(1, p.life);
          if (p.land) { g.beginPath(); g.ellipse(p.x, p.y, 12, 5, 0, 0, Math.PI * 2); g.fillStyle = p.c; g.fill(); }
          else D.disc(g, p.x, p.y, 7, p.c);
          g.globalAlpha = 1;
        }
        if (!s.noz.set) {
          D.panel(g, CRATE.x, CRATE.y, CRATE.w, CRATE.h, 14, "#141b27");
          nozzle(g, s.noz.x, s.noz.y);
          D.ring(g, s.noz.x, s.noz.y - 4, 40, "#f0c08a", 3);
        }
      },
    };
  },
});
