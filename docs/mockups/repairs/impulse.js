/*
 * repairs/impulse.js: an impulse engine's repair (openspec/changes/repair-minigames, design 2).
 *
 * The unit from the side: the combustion chamber, its manifold and eight fuel injectors above it, the nozzle bell
 * behind. Below it the timing scope: each injector's pulse scrolls along the trace towards the firing line, in the
 * rhythm of the unit's tune. Fire (click, tap, or Space) as a pulse crosses the line and that injector is timed. A step
 * is the eight injectors timed; steps alternate between the two units, and each one scrolls faster with a tighter
 * line. A pulse let past the line comes round again. Firing off the line is a fumble: a misfire, soot and heat. A
 * disabled unit's first step fits the new injector: drag it from the crate into its empty seat on the manifold.
 */
RepairKit.register({
  id: "impulse",
  title: "Impulse engines",
  place: "Drive, the two impulse units",
  group: "Engineering",
  hazard: "Misfire: soot and heat",
  down: "Half thrust for each unit down",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const N = 8;
    const INJ_X = (i) => 210 + i * 88;      // injector i's x on the manifold
    const MAN_Y = 156;                      // the manifold pipe's centre, px
    const CH = { x: 130, y: 196, w: 790, h: 120 };   // the combustion chamber
    const SC = { x: 60, y: 392, w: 1160, h: 290 };   // the timing scope
    const BASE = SC.y + SC.h - 70;          // the trace's baseline, px
    const LINE = 300;                       // the firing line's x, px
    const SPIKE = 120;                      // a pulse's height, px
    const MISSING = 3;                      // the part step's empty seat
    let part, inj, pulses, speed, tol, time, unit, heat, soot, kick, fired;

    /** A pulse's x on the scope now. */
    const px = (pl) => LINE + (pl.at - time) * speed;

    return {
      step(index, isPart) {
        const r = api.rand();
        part = isPart;
        inj = { x: 1110, y: 214, held: false, set: false };
        unit = index % 2;
        speed = 240 + 40 * index;
        tol = Math.max(18, 30 - 3 * index);
        time = 0; heat = 0; soot = []; kick = 0; fired = 0;
        // The unit's tune: a short rhythm of long and short gaps, repeated with a little swing.
        const beat = Math.max(0.5, 0.78 - 0.05 * index);
        const tune = [1, 1, 0.5, 1.5, 1, 0.5, 0.5, 1.5].map((b) => b * beat);
        const order = Array.from({ length: N }, (_, i) => i);
        for (let i = N - 1; i > 0; i--) { const j = Math.floor(r() * (i + 1)); [order[i], order[j]] = [order[j], order[i]]; }
        let at = 2.4;
        pulses = order.map((k, i) => { const pl = { inj: k, at, done: false, hitT: 0 }; at += tune[(i + Math.floor(r() * 3)) % tune.length]; return pl; });
      },
      /** For tools (shots and tests): the round's state, read only. */
      peek() { return { part, partSet: inj.set, fired, time, pulses: pulses.map((pl) => ({ inj: pl.inj, x: px(pl), done: pl.done })), tol, line: LINE }; },
      update(dt, input) {
        heat = Math.max(0, heat - dt * 0.5);
        kick = Math.max(0, kick - dt * 3);
        soot = soot.filter((s) => (s.t += dt) < 1.6);
        if (part && !inj.set) {
          // The part step: carry the injector to its seat.
          const sx = INJ_X(MISSING), sy = MAN_Y + 40;
          if (input.pressed && Math.abs(input.x - inj.x) < 40 && Math.abs(input.y - inj.y) < 60) inj.held = true;
          if (inj.held && input.down) { inj.x = input.x; inj.y = input.y; }
          if (inj.held && input.released) {
            inj.held = false;
            if (Math.hypot(inj.x - sx, inj.y - sy) < 45) { inj.set = true; inj.x = sx; inj.y = sy; api.stepDone(); }
          }
          return;
        }
        if (part) return;
        time += dt;
        if (fired < N) {
          // A pulse let past the line comes round again at the back of the trace.
          let last = Math.max(time, ...pulses.map((pl) => pl.at));
          for (const pl of pulses) if (!pl.done && px(pl) < LINE - tol - 40) { last += 1.2; pl.at = last; }
          const fire = input.actionPressed || (input.pressed && input.y > api.BAR_H);
          if (fire) {
            let best = null, bd = Infinity;
            for (const pl of pulses) if (!pl.done) { const d = Math.abs(px(pl) - LINE); if (d < bd) { bd = d; best = pl; } }
            if (best && bd <= tol) {
              best.done = true; best.hitT = time; fired++; kick = 1;
              if (fired === N) api.stepDone();
            } else {
              heat = 1;
              const k = best ? best.inj : 0;
              soot.push({ x: INJ_X(k), y: CH.y + 20, t: 0 });
              api.fumble("Misfire: soot and heat");
              return;
            }
          }
        }
      },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        const live = !part;
        const firedSet = new Set(live ? pulses.filter((pl) => pl.done).map((pl) => pl.inj) : []);
        let next = -1;
        if (live) {
          let bx = Infinity;
          for (const pl of pulses) if (!pl.done) { const x = px(pl); if (x > LINE - tol && x < bx) { bx = x; next = pl.inj; } }
        }

        // The nozzle bell and its plume: brighter with every injector timed.
        const thrust = live ? fired / N : 0;
        const plume = g.createLinearGradient(1040, 0, 1260, 0);
        plume.addColorStop(0, `rgba(79,195,247,${0.15 + 0.6 * thrust + 0.25 * kick})`);
        plume.addColorStop(1, "rgba(79,195,247,0)");
        g.beginPath(); g.moveTo(1040, 170); g.lineTo(1260, 196 + 10 * Math.sin(t * 9)); g.lineTo(1260, 316 - 10 * Math.sin(t * 7)); g.lineTo(1040, 342); g.closePath();
        g.fillStyle = plume; g.fill();
        g.beginPath(); g.moveTo(CH.x + CH.w, CH.y + 14); g.lineTo(1040, 160); g.lineTo(1040, 352); g.lineTo(CH.x + CH.w, CH.y + CH.h - 14); g.closePath();
        g.fillStyle = "#1b2433"; g.fill(); g.lineWidth = 3; g.strokeStyle = "#3a4656"; g.stroke();
        for (let k = 1; k <= 3; k++) {
          const x = CH.x + CH.w + k * 30, f = (x - CH.x - CH.w) / (1040 - CH.x - CH.w);
          g.beginPath(); g.moveTo(x, CH.y + 14 - f * 50); g.lineTo(x, CH.y + CH.h - 14 + f * 50);
          g.strokeStyle = "#2a3444"; g.lineWidth = 2; g.stroke();
        }
        g.fillStyle = `rgba(160,220,255,${0.3 + 0.6 * thrust})`; g.fillRect(1034, 162, 6, 188);

        // The chamber: a ribbed steel cylinder, hot when it misfires.
        D.panel(g, CH.x, CH.y, CH.w, CH.h, 26, "#1b2433", "#3a4656");
        for (let x = CH.x + 40; x < CH.x + CH.w - 20; x += 44) { g.fillStyle = "#222c3c"; g.fillRect(x, CH.y + 4, 10, CH.h - 8); }
        if (heat > 0) { D.round(g, CH.x, CH.y, CH.w, CH.h, 26); g.fillStyle = `rgba(255,71,87,${0.35 * heat})`; g.fill(); }
        const inside = g.createLinearGradient(0, CH.y + 40, 0, CH.y + 80);
        inside.addColorStop(0, "rgba(79,195,247,0)"); inside.addColorStop(0.5, `rgba(79,195,247,${0.1 + 0.45 * thrust})`); inside.addColorStop(1, "rgba(79,195,247,0)");
        g.fillStyle = inside; g.fillRect(CH.x + 30, CH.y + 40, CH.w - 40, 40);
        D.panel(g, CH.x - 50, CH.y + 18, 70, CH.h - 36, 14, "#141c28", "#3a4656");
        D.text(g, unit ? "UNIT 2" : "UNIT 1", CH.x + 30, CH.y + CH.h + 26, 20, C.dim, "left", 700);

        // The manifold and the eight injectors, each with its lamp: hollow waiting, amber ring next, green timed.
        D.round(g, 170, MAN_Y - 12, 650, 24, 12); g.fillStyle = "#2b3646"; g.fill();
        D.round(g, 120, MAN_Y - 8, 60, 16, 8); g.fillStyle = "#2b3646"; g.fill();
        for (let i = 0; i < N; i++) {
          const x = INJ_X(i);
          if (part && i === MISSING && !inj.set) {
            g.setLineDash([6, 5]); D.round(g, x - 15, MAN_Y + 8, 30, CH.y - MAN_Y - 6, 6); g.lineWidth = 2; g.strokeStyle = C.amber; g.stroke(); g.setLineDash([]);
            continue;
          }
          D.round(g, x - 13, MAN_Y + 8, 26, CH.y - MAN_Y - 4, 6); g.fillStyle = "#566273"; g.fill();
          g.fillStyle = "#3a4656"; g.fillRect(x - 17, MAN_Y + 18, 34, 6);
          const ly = MAN_Y - 44;
          if (firedSet.has(i)) { D.disc(g, x, ly, 14, C.ok); D.text(g, "✓", x, ly + 1, 18, "#04140a", "center", 700); }
          else if (i === next) { D.disc(g, x, ly, 14, "#1d2738"); D.ring(g, x, ly, 14, C.amber, 4); }
          else { D.disc(g, x, ly, 14, "#121a25"); D.ring(g, x, ly, 14, "#3b4a5e", 2); }
        }
        for (const s of soot) {
          const k = s.t / 1.6;
          for (let j = 0; j < 5; j++) D.disc(g, s.x + (j - 2) * 14 * (1 + k), s.y - 30 * k - (j % 2) * 10, 10 + 22 * k, `rgba(40,40,44,${0.75 * (1 - k)})`);
        }

        // The timing scope: a grid, the trace, the firing line, the pulses.
        D.panel(g, SC.x, SC.y, SC.w, SC.h, 18, "#060c10", "#1f3a33");
        g.save(); D.round(g, SC.x, SC.y, SC.w, SC.h, 18); g.clip();
        g.strokeStyle = "rgba(61,220,132,0.08)"; g.lineWidth = 1;
        for (let x = SC.x; x < SC.x + SC.w; x += 40) { g.beginPath(); g.moveTo(x, SC.y); g.lineTo(x, SC.y + SC.h); g.stroke(); }
        for (let y = SC.y + 10; y < SC.y + SC.h; y += 40) { g.beginPath(); g.moveTo(SC.x, y); g.lineTo(SC.x + SC.w, y); g.stroke(); }
        g.fillStyle = "rgba(242,160,70,0.14)"; g.fillRect(LINE - tol, SC.y, tol * 2, SC.h);
        g.fillStyle = C.amber; g.fillRect(LINE - 2, SC.y, 4, SC.h);
        g.beginPath(); g.moveTo(LINE - 14, SC.y); g.lineTo(LINE + 14, SC.y); g.lineTo(LINE, SC.y + 18); g.closePath(); g.fill();
        g.beginPath(); g.moveTo(LINE - 14, SC.y + SC.h); g.lineTo(LINE + 14, SC.y + SC.h); g.lineTo(LINE, SC.y + SC.h - 18); g.closePath(); g.fill();
        // The trace: flat between pulses, a sharp spike at each.
        g.beginPath(); g.moveTo(SC.x, BASE);
        const shown = live ? pulses.map((pl) => ({ pl, x: px(pl) })).filter((o) => o.x > SC.x - 40 && o.x < SC.x + SC.w + 40).sort((a, b) => a.x - b.x) : [];
        for (const { x } of shown) { g.lineTo(x - 22, BASE); g.lineTo(x - 8, BASE - SPIKE); g.lineTo(x + 8, BASE - SPIKE); g.lineTo(x + 22, BASE); }
        g.lineTo(SC.x + SC.w, BASE);
        g.strokeStyle = live ? "rgba(61,220,132,0.75)" : "rgba(61,220,132,0.2)"; g.lineWidth = 3; g.stroke();
        for (const { pl, x } of shown) {
          const near = !pl.done && Math.abs(x - LINE) <= tol;
          g.beginPath(); g.moveTo(x - 22, BASE); g.lineTo(x - 8, BASE - SPIKE); g.lineTo(x + 8, BASE - SPIKE); g.lineTo(x + 22, BASE); g.closePath();
          g.fillStyle = pl.done ? "rgba(61,220,132,0.35)" : near ? "rgba(242,160,70,0.45)" : "rgba(61,220,132,0.12)"; g.fill();
          if (pl.done) { D.disc(g, x, BASE - SPIKE - 26, 14, C.ok); D.text(g, "✓", x, BASE - SPIKE - 25, 18, "#04140a", "center", 700); }
          else D.text(g, String(pl.inj + 1), x, BASE - SPIKE - 24, 22, near ? C.amber : C.fg, "center", 700);
        }
        g.restore();

        // The part: the new injector in its crate, or in the hand.
        if (part && !inj.set) {
          D.panel(g, 1040, 120, 140, 190, 14, "#141b27");
          D.round(g, inj.x - 13, inj.y - 40, 26, 80, 6); g.fillStyle = "#8a96a6"; g.fill();
          g.fillStyle = C.copper; g.fillRect(inj.x - 17, inj.y - 30, 34, 6);
          g.beginPath(); g.moveTo(inj.x - 8, inj.y + 40); g.lineTo(inj.x + 8, inj.y + 40); g.lineTo(inj.x, inj.y + 54); g.closePath(); g.fillStyle = "#b8c2d0"; g.fill();
        }
      },
    };
  },
});
