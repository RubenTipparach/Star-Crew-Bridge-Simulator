/*
 * repairs/breakers.js: a breaker's repair at the main switchboard (openspec/changes/repair-minigames, design 2).
 *
 * The switchboard's cubicle with its draw-out breaker, the bus's rating plate above it, the synchroscope and the
 * close button on the right, the fuse tray below them. Rack the breaker out (drag its handle down, or hold the down
 * arrow), and its fuse cover opens on the blown cartridge. Fit the cartridge whose rating matches the bus (drag it
 * from the tray, or up and down to choose and Space to fit it), rack the breaker back in (drag up, or hold up), and
 * close it (the button, or Space) while the synchroscope's needle sits in its green wedge and the lamp shows green.
 * A step is one breaker; each next one has more cartridges, a faster needle and a narrower wedge. Closing on red is a
 * fumble: an arc flash, 10 HP. A wrong cartridge is a fumble too: it blows as it seats. A disabled board's first step
 * fits the new breaker: drag it off the trolley into the empty cubicle.
 */
RepairKit.register({
  id: "breakers",
  title: "Switchboard and breakers",
  place: "Main switchboard",
  group: "Engineering",
  hazard: "Arc flash: 10 HP",
  down: "power-grid",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const CAB = { x: 120, y: 92, w: 660, h: 610 };       // the switchboard cabinet
    const CUB = { x: 200, y: 196, w: 500, h: 460 };      // the cubicle
    const UNIT = { w: 440, h: 236 };                     // the draw-out breaker
    const UX = CUB.x + (CUB.w - UNIT.w) / 2, UY0 = CUB.y + 12, TRAVEL = 196;
    const FUSE = { dx: 140, dy: 64, w: 160, h: 70 };     // the fuse holder on the breaker's face
    const SYN = { x: 965, y: 250, r: 112 };              // the synchroscope
    const LAMP = { x: 1168, y: 170, r: 32 };
    const BTN = { x: 1168, y: 318, r: 56 };              // the close button: the biggest control
    const TRAY = { x: 820, y: 432, w: 430, h: 268 };
    const RATINGS = [16, 25, 40, 63, 100];
    const BAND = { 16: C.lilac, 25: C.accent, 40: C.warn, 63: C.ok, 100: C.danger };
    let part, unitP, phase, rack, grab, grab0, carts, sel, held, bus, fuse, theta, omega, win, closed, flash, time, busName;
    const wrap = (a) => Math.atan2(Math.sin(a), Math.cos(a));
    const unitY = () => UY0 + rack * TRAVEL;
    const handle = () => ({ x: UX + UNIT.w / 2 - 70, y: unitY() + UNIT.h - 46, w: 140, h: 30 });
    const holder = () => ({ x: UX + FUSE.dx, y: unitY() + FUSE.dy, w: FUSE.w, h: FUSE.h });
    const cartBox = (i) => ({ x: TRAY.x + 60, y: TRAY.y + 26 + i * 60, w: 310, h: 44 });
    const green = () => Math.abs(wrap(theta)) < win;
    const inBox = (b, x, y, m = 0) => x >= b.x - m && x <= b.x + b.w + m && y >= b.y - m && y <= b.y + b.h + m;

    return {
      step(index, isPart) {
        const r = api.rand();
        part = isPart;
        unitP = { x: 1035, y: 590, held: false, set: false };
        phase = isPart ? "part" : "out";
        rack = 0; grab = false; held = -1; sel = 0; closed = false; flash = 0; time = 0;
        const k = Math.min(4, 3 + Math.floor(index / 2));
        const pool = [...RATINGS];
        for (let i = pool.length - 1; i > 0; i--) { const j = Math.floor(r() * (i + 1)); [pool[i], pool[j]] = [pool[j], pool[i]]; }
        carts = pool.slice(0, k).map((a) => ({ a, gone: false }));
        bus = carts[Math.floor(r() * k)].a;
        busName = "ABCD"[Math.floor(r() * 4)];
        fuse = "blown";
        theta = r() * Math.PI * 2;
        omega = 1.5 + 0.35 * index;
        win = Math.max(0.16, 0.34 - 0.04 * index);
      },
      /** For tools (shots and tests): the round's state, read only. */
      peek() { return { phase, rack, bus, carts: carts.map((c) => c.a), green: green(), partSet: unitP.set, handle: handle(), holder: holder(), cart: cartBox, btn: BTN, unitP: { ...unitP } }; },
      update(dt, input) {
        time += dt;
        flash = Math.max(0, flash - dt * 1.6);
        theta += omega * (1 + 0.3 * Math.sin(time * 0.5)) * dt;
        if (phase === "part") {
          // The part step: wheel the new breaker into the cubicle.
          if (unitP.set) return;
          if (input.pressed && Math.abs(input.x - unitP.x) < UNIT.w * 0.25 && Math.abs(input.y - unitP.y) < UNIT.h * 0.25) unitP.held = true;
          if (unitP.held && input.down) { unitP.x = input.x; unitP.y = input.y; }
          if (unitP.held && input.released) {
            unitP.held = false;
            if (inBox(CUB, unitP.x, unitP.y)) { unitP.set = true; api.stepDone(); }
          }
          return;
        }
        if (phase === "out" || phase === "in") {
          // Racking: drag the handle, or hold the arrow, the way this phase goes.
          const dir = phase === "out" ? 1 : -1;
          if (input.pressed && inBox(handle(), input.x, input.y, 16)) { grab = true; grab0 = input.y - rack * TRAVEL; }
          if (grab && input.down) rack = Math.max(0, Math.min(1, (input.y - grab0) / TRAVEL));
          if (!input.down) grab = false;
          if (input.stick.y * dir > 0) rack = Math.max(0, Math.min(1, rack + dir * 0.7 * dt));
          if (phase === "out" && rack >= 0.98) { rack = 1; grab = false; phase = "fuse"; }
          if (phase === "in" && rack <= 0.02) { rack = 0; grab = false; phase = "close"; }
          return;
        }
        if (phase === "fuse") {
          const live = carts.map((c, i) => i).filter((i) => !carts[i].gone);
          if (input.hit.has("ArrowUp") || input.hit.has("KeyW")) sel = live[(live.indexOf(sel) + live.length - 1) % live.length] ?? live[0];
          if (input.hit.has("ArrowDown") || input.hit.has("KeyS")) sel = live[(live.indexOf(sel) + 1) % live.length] ?? live[0];
          if (carts[sel] && carts[sel].gone) sel = live[0];
          if (input.pressed) for (const i of live) if (inBox(cartBox(i), input.x, input.y)) { held = i; sel = i; }
          let fit = -1;
          if (held >= 0 && input.released) { if (inBox(holder(), input.x, input.y, 40)) fit = held; held = -1; }
          if (input.actionPressed) fit = sel;
          if (fit < 0) return;
          if (carts[fit].a === bus) { carts[fit].gone = true; fuse = "new"; phase = "in"; }
          else {
            carts[fit].gone = true; flash = 0.5;
            api.fumble("Wrong rating: the fuse blows");
          }
          return;
        }
        if (phase === "close") {
          const press = input.actionPressed || (input.pressed && Math.hypot(input.x - BTN.x, input.y - BTN.y) < BTN.r + 8);
          if (!press) return;
          if (green()) { closed = true; phase = "done"; api.stepDone(); }
          else { flash = 1; api.fumble("Arc flash: 10 HP"); }
        }
      },
      draw(g, t, dt, input) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);

        // The cabinet: steel, its bolts, the rating plate, the cubicle and its rails.
        D.panel(g, CAB.x, CAB.y, CAB.w, CAB.h, 10, "#1b2433", "#3a4656");
        for (const [x, y] of [[CAB.x + 22, CAB.y + 22], [CAB.x + CAB.w - 22, CAB.y + 22], [CAB.x + 22, CAB.y + CAB.h - 22], [CAB.x + CAB.w - 22, CAB.y + CAB.h - 22]]) D.disc(g, x, y, 7, "#3a4656");
        D.panel(g, 300, 112, 300, 62, 8, "#0c121a", "#566273");
        D.text(g, `BUS ${busName}`, 330, 143, 26, C.dim, "left", 700);
        D.text(g, `${bus} A`, 570, 143, 34, C.amber, "right", 700);
        D.round(g, CUB.x, CUB.y, CUB.w, CUB.h, 6); g.fillStyle = "#06090e"; g.fill(); g.lineWidth = 3; g.strokeStyle = "#2b3646"; g.stroke();
        for (const x of [CUB.x + 8, CUB.x + CUB.w - 20]) { g.fillStyle = "#3a4656"; g.fillRect(x, CUB.y + 8, 12, CUB.h - 16); }
        for (let y = CUB.y + 24; y < CUB.y + 120; y += 22) { g.fillStyle = "#2b2014"; g.fillRect(CUB.x + 120, y, 260, 8); g.fillStyle = C.copper; g.fillRect(CUB.x + 120, y, 260, 3); }
        // The racked position: a slot on the cabinet's side with a marker.
        g.fillStyle = "#0c121a"; g.fillRect(CUB.x + CUB.w + 26, UY0 + 40, 10, TRAVEL);
        const my = UY0 + 40 + rack * TRAVEL;
        g.beginPath(); g.moveTo(CUB.x + CUB.w + 22, my); g.lineTo(CUB.x + CUB.w + 40, my - 10); g.lineTo(CUB.x + CUB.w + 40, my + 10); g.closePath();
        g.fillStyle = phase === "out" || phase === "in" ? C.amber : C.steel; g.fill();
        D.text(g, "IN", CUB.x + CUB.w + 48, UY0 + 40, 16, C.dim, "left", 700);
        D.text(g, "OUT", CUB.x + CUB.w + 48, UY0 + 40 + TRAVEL, 16, C.dim, "left", 700);

        if (phase !== "part") unit(g, UX, unitY(), 1);
        else if (unitP.set) unit(g, UX, UY0, 1);
        else { g.setLineDash([12, 9]); D.round(g, UX, UY0, UNIT.w, UNIT.h, 10); g.lineWidth = 3; g.strokeStyle = C.amber; g.stroke(); g.setLineDash([]); }

        // The synchroscope: a dial, a green wedge at the top, the needle turning with the slip.
        D.panel(g, 820, 96, 430, 314, 18, "#0c121a");
        D.disc(g, SYN.x, SYN.y, SYN.r + 10, "#121a25"); D.ring(g, SYN.x, SYN.y, SYN.r + 10, "#3a4656", 3);
        g.beginPath(); g.moveTo(SYN.x, SYN.y); g.arc(SYN.x, SYN.y, SYN.r, -Math.PI / 2 - win, -Math.PI / 2 + win); g.closePath();
        g.fillStyle = "rgba(61,220,132,0.4)"; g.fill();
        g.save(); g.beginPath(); g.moveTo(SYN.x, SYN.y); g.arc(SYN.x, SYN.y, SYN.r, -Math.PI / 2 + win, -Math.PI / 2 - win + Math.PI * 2); g.closePath(); g.clip();
        g.strokeStyle = "rgba(255,71,87,0.18)"; g.lineWidth = 3;
        for (let x = -SYN.r * 2; x < SYN.r * 2; x += 14) { g.beginPath(); g.moveTo(SYN.x + x, SYN.y + SYN.r); g.lineTo(SYN.x + x + SYN.r, SYN.y - SYN.r); g.stroke(); }
        g.restore();
        for (let k = 0; k < 36; k++) {
          const a = (k / 36) * Math.PI * 2;
          g.beginPath(); g.moveTo(SYN.x + Math.cos(a) * (SYN.r - 10), SYN.y + Math.sin(a) * (SYN.r - 10)); g.lineTo(SYN.x + Math.cos(a) * SYN.r, SYN.y + Math.sin(a) * SYN.r);
          g.strokeStyle = "#566273"; g.lineWidth = 2; g.stroke();
        }
        const live = phase !== "part";
        const ok = live && green();
        const na = theta - Math.PI / 2;
        g.beginPath(); g.moveTo(SYN.x - Math.cos(na) * 20, SYN.y - Math.sin(na) * 20); g.lineTo(SYN.x + Math.cos(na) * (SYN.r - 8), SYN.y + Math.sin(na) * (SYN.r - 8));
        g.strokeStyle = live ? C.fg : "#3a4656"; g.lineWidth = 6; g.lineCap = "round"; g.stroke(); g.lineCap = "butt";
        D.disc(g, SYN.x, SYN.y, 12, "#566273");
        // The lamp: green with a tick, red with a cross.
        D.disc(g, LAMP.x, LAMP.y, LAMP.r + 6, "#121a25");
        if (!live) D.disc(g, LAMP.x, LAMP.y, LAMP.r, "#1d2738");
        else if (ok) { D.disc(g, LAMP.x, LAMP.y, LAMP.r, C.ok); D.text(g, "✓", LAMP.x, LAMP.y + 2, 34, "#04140a", "center", 700); }
        else {
          D.disc(g, LAMP.x, LAMP.y, LAMP.r, C.danger);
          g.beginPath(); g.moveTo(LAMP.x - 12, LAMP.y - 12); g.lineTo(LAMP.x + 12, LAMP.y + 12); g.moveTo(LAMP.x + 12, LAMP.y - 12); g.lineTo(LAMP.x - 12, LAMP.y + 12);
          g.strokeStyle = "#2a0a0e"; g.lineWidth = 6; g.stroke();
        }
        // The close button: a mushroom head, lit when the breaker is ready to close.
        const ready = phase === "close";
        D.disc(g, BTN.x, BTN.y, BTN.r + 8, "#121a25"); D.ring(g, BTN.x, BTN.y, BTN.r + 8, ready ? C.amber : "#2b3646", 4);
        D.disc(g, BTN.x, BTN.y + 4, BTN.r, ready ? "#7a2f38" : "#262e42");
        D.disc(g, BTN.x, BTN.y, BTN.r - 2, closed ? C.ok : ready ? C.danger : "#2b3646");
        D.text(g, "CLOSE", BTN.x, BTN.y, 22, ready || closed ? "#fff" : C.dim, "center", 700);

        // The fuse tray: one cartridge a rating, each with its colour band and label.
        D.panel(g, TRAY.x, TRAY.y, TRAY.w, TRAY.h, 18, "#0c121a");
        if (phase === "part") {
          // The trolley with the new breaker on it.
          g.fillStyle = "#2b3646"; g.fillRect(TRAY.x + 40, TRAY.y + TRAY.h - 50, TRAY.w - 80, 14);
          for (const x of [TRAY.x + 80, TRAY.x + TRAY.w - 80]) D.disc(g, x, TRAY.y + TRAY.h - 26, 14, "#566273");
          if (!unitP.set) unit(g, unitP.x - UNIT.w * 0.25, unitP.y - UNIT.h * 0.25, 0.5);
        } else {
          carts.forEach((c, i) => {
            if (c.gone || i === held) return;
            cartridge(g, cartBox(i), c.a, phase === "fuse" && i === sel);
          });
          if (held >= 0 && input) {
            const b = cartBox(held);
            cartridge(g, { x: input.x - b.w * 0.25, y: input.y - b.h * 0.5, w: b.w * 0.5, h: b.h }, carts[held].a, true);
          }
        }

        // The arc flash.
        if (flash > 0) {
          const h = holder();
          g.strokeStyle = `rgba(200,230,255,${flash})`; g.lineWidth = 4;
          for (let k = 0; k < 8; k++) {
            const x0 = h.x + h.w / 2, y0 = h.y + h.h / 2;
            g.beginPath(); g.moveTo(x0, y0);
            for (let j = 1; j < 6; j++) g.lineTo(x0 + Math.cos(k * 0.8 + t * 17) * j * 30 + Math.sin(t * 50 + j) * 12, y0 + Math.sin(k * 0.8 + t * 13) * j * 26);
            g.stroke();
          }
          g.fillStyle = `rgba(220,235,255,${0.35 * flash})`; g.fillRect(0, api.BAR_H, api.W, api.H);
        }
      },
    };

    /** The draw-out breaker at (x, y), at a scale, with its face: state window, fuse holder, racking handle. */
    function unit(g, x, y, s) {
      g.save(); g.translate(x, y); g.scale(s, s);
      D.panel(g, 0, 0, UNIT.w, UNIT.h, 10, "#2b3646", "#566273");
      g.fillStyle = "#222c3c"; g.fillRect(10, 10, UNIT.w - 20, 40);
      // The state window: O open (green), I closed (red).
      D.round(g, 24, 16, 64, 28, 6); g.fillStyle = "#0c121a"; g.fill();
      if (closed) { g.fillStyle = C.danger; g.fillRect(52, 20, 8, 20); }
      else D.ring(g, 56, 30, 9, C.ok, 4);
      for (let k = 0; k < 5; k++) D.disc(g, 300 + k * 24, 30, 6, "#3a4656");
      // The fuse holder: covered by the interlock until the breaker is out.
      const fx = FUSE.dx, fy = FUSE.dy;
      D.round(g, fx - 6, fy - 6, FUSE.w + 12, FUSE.h + 12, 8); g.fillStyle = "#121a25"; g.fill();
      const open = phase === "fuse" || (phase === "in" && rack > 0.02) || (phase === "out" && rack >= 0.98);
      if (open || phase === "part") {
        if (phase !== "part") {
          if (fuse === "blown") {
            cartridge(g, { x: fx + 8, y: fy + 13, w: FUSE.w - 16, h: FUSE.h - 26 }, 0, false, true);
          } else cartridge(g, { x: fx + 8, y: fy + 13, w: FUSE.w - 16, h: FUSE.h - 26 }, bus, false);
        }
      } else {
        g.save(); D.round(g, fx, fy, FUSE.w, FUSE.h, 6); g.clip();
        g.fillStyle = "#3a4656"; g.fillRect(fx, fy, FUSE.w, FUSE.h);
        g.strokeStyle = "#2b3646"; g.lineWidth = 6;
        for (let k = -FUSE.h; k < FUSE.w; k += 18) { g.beginPath(); g.moveTo(fx + k, fy + FUSE.h); g.lineTo(fx + k + FUSE.h, fy); g.stroke(); }
        g.restore();
      }
      if (phase === "fuse") { D.round(g, fx - 6, fy - 6, FUSE.w + 12, FUSE.h + 12, 8); g.lineWidth = 3; g.strokeStyle = C.amber; g.stroke(); }
      // Vents.
      for (let k = 0; k < 6; k++) { g.fillStyle = "#1b2433"; g.fillRect(24, 70 + k * 14, 80, 6); g.fillRect(UNIT.w - 104, 70 + k * 14, 80, 6); }
      // The racking handle.
      const hx = UNIT.w / 2 - 70, hy = UNIT.h - 46;
      const racking = phase === "out" || phase === "in";
      g.fillStyle = "#566273"; g.fillRect(UNIT.w / 2 - 6, hy - 10, 12, 14);
      D.round(g, hx, hy, 140, 30, 10); g.fillStyle = racking ? (grab ? C.amber : "#c9d3e0") : "#7d8796"; g.fill();
      g.fillStyle = "#3a4656"; g.fillRect(hx + 20, hy + 12, 100, 6);
      if (racking) {
        const dir = phase === "out" ? 1 : -1, ay = dir > 0 ? hy + 44 : hy - 26;
        g.beginPath(); g.moveTo(UNIT.w / 2 - 16, ay - dir * 8); g.lineTo(UNIT.w / 2 + 16, ay - dir * 8); g.lineTo(UNIT.w / 2, ay + dir * 8); g.closePath();
        g.fillStyle = C.amber; g.fill();
      }
      g.restore();
    }

    /** A fuse cartridge: brass caps, a ceramic body, the rating's band and label; blown is black and cracked. */
    function cartridge(g, b, amps, on, blown) {
      const cap = Math.min(30, b.w * 0.14);
      D.round(g, b.x, b.y, cap, b.h, 4); g.fillStyle = blown ? "#4a3a24" : "#c9a45a"; g.fill();
      D.round(g, b.x + b.w - cap, b.y, cap, b.h, 4); g.fill();
      g.fillStyle = blown ? "#1a1512" : "#e7e2d6"; g.fillRect(b.x + cap, b.y + 3, b.w - cap * 2, b.h - 6);
      if (blown) {
        g.beginPath(); g.moveTo(b.x + b.w * 0.42, b.y + 3); g.lineTo(b.x + b.w * 0.5, b.y + b.h * 0.45); g.lineTo(b.x + b.w * 0.45, b.y + b.h * 0.6); g.lineTo(b.x + b.w * 0.55, b.y + b.h - 3);
        g.strokeStyle = C.danger; g.lineWidth = 3; g.stroke();
      } else {
        g.fillStyle = BAND[amps]; g.fillRect(b.x + cap + 6, b.y + 3, 12, b.h - 6);
        D.text(g, `${amps} A`, b.x + b.w / 2 + 8, b.y + b.h / 2 + 1, Math.min(26, b.h * 0.6), "#111821", "center", 700);
      }
      if (on) { D.round(g, b.x - 6, b.y - 5, b.w + 12, b.h + 10, 8); g.lineWidth = 3; g.strokeStyle = C.amber; g.stroke(); }
    }
  },
});
