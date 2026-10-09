/*
 * repairs/pylons.js: a warp pylon's coil repair, outside on EVA (openspec/changes/repair-minigames, design 2 and 5).
 *
 * The hull and a pylon side on against the stars, the warp coil along the pylon's top with one segment dark and
 * cracked. You are the suited figure at the dorsal airlock on two tether clips. Click a handhold within reach and you
 * swing to it with one clip open; its hook swings past the rail, and you clip it home as it crosses (click, or Space).
 * Only one clip is ever open, and an open clip has a few seconds: miss them and the tether snaps you back to the last
 * hold. At the damaged segment the view closes in: unbolt its plate in a star order (across, then round; every bolt
 * shows its number and the next is lit; a wrong bolt slips and is lost), let the old segment go, and push the new one up
 * into the gap against the drift until it seats square. Later steps go further out along the coil, with shorter clip
 * times, more bolts and a stronger drift. A disabled pylon's first step fits the new coil driver into its socket at the
 * pylon's root. In combat an EVA is refused (design 5): nothing here can be played.
 * Keys: arrows choose a handhold or a bolt and push the segment, Space moves, clips and unbolts.
 */
RepairKit.register({
  id: "pylons",
  title: "Warp pylons",
  place: "Outside, the dorsal pylon (EVA only)",
  group: "Outside and doors",
  hazard: "The tether snaps you back",
  // The how-to card (repair-minigames 6g), drawn by the kit: pictures and a few words, on demand.
  guide: {
    steps: [
      { icon: "tap", text: "Tap a handhold in reach" },
      { icon: "rhythm", text: "Clip the hook as it passes" },
      { icon: "order", text: "Unbolt in the lit order" },
      { icon: "drag", text: "Push the new segment in" },
    ],
    mistake: "A clip missed: the tether snaps you back",
    now: (q) => (q.phase === "climb" ? (q.mode === "open" ? 1 : 0) : q.phase === "bolts" ? 2 : q.phase === "release" || q.phase === "swap" ? 3 : -1),
  },
  down: "No warp jump; not repairable in combat (design 5)",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const HULL = { x: 600, y: 3000, r: 2400 };           // the hull's curve, side on
    const COIL = { x0: 380, y0: 172, h: 88, seg: 120, n: 7 };
    const REACH = 175;                                   // how far a suited arm and lanyard reach, px
    const hullY = (x) => HULL.y - Math.sqrt(HULL.r * HULL.r - (x - HULL.x) * (x - HULL.x));
    const HATCH = { x: 232, y: hullY(232) };
    const SOCKET = { x: 512, y: 548 };                   // the coil driver's socket, low on the pylon
    const CRATE = { x: 120, y: hullY(120) - 26 };
    const near = (ax, ay, bx, by, r) => Math.hypot(ax - bx, ay - by) < r;
    const wrapI = (i, n) => ((i % n) + n) % n;
    const sr = KIT.rng(KIT.hash("pylons:stars"));
    const stars = Array.from({ length: 170 }, () => ({ x: sr() * 1280, y: 80 + sr() * 640, r: 0.6 + sr() * 1.6, p: sr() * 6 }));
    let phase, holds, at, mode, from, to, openT, openMax, omega, swingT, miss, fig, cam, dmg, bolts, order, wrong,
      lost, rel, seg, kf, fi, drv, index;

    /** The fastener that comes next: the kit's star order from the first (one order, shown at rest, KIT.starOrder). */
    function nextInStar(done, n) {
      const S = KIT.starOrder(n);
      return done.length < n ? [S[done.length]] : [];
    }
    /** Fastener i's number in the order, from 1. */
    const orderNo = (i, n) => KIT.starOrder(n).indexOf(i) + 1;
    const segX = (k) => COIL.x0 + k * COIL.seg;
    const seat = () => ({ x: segX(dmg) + COIL.seg / 2, y: COIL.y0 + COIL.h / 2 });
    const toWorld = (x, y) => [(x - 640) / cam.s + cam.x, (y - 400) / cam.s + cam.y];
    const reachable = () => holds.map((h, i) => i).filter((i) => i !== at && near(holds[i].x, holds[i].y, holds[at].x, holds[at].y, REACH))
      .sort((a, b) => holds[a].x - holds[b].x);
    /** Where the open clip's hook is: swinging across its hold with the suit's drift. */
    function hook() {
      const ph = omega * swingT + Math.PI / 2;
      return { x: holds[to].x + 32 * Math.sin(ph), y: holds[to].y + 9 * Math.sin(2 * ph) };
    }
    function arrive() {
      mode = "safe";
      if (at === holds.length - 1 && phase === "climb") { phase = "bolts"; fi = 0; }
    }
    function unbolt(i) {
      const b = bolts[i];
      if (b.out) return;
      if (nextInStar(order, bolts.length).includes(i)) {
        b.out = true; order.push(i);
        if (order.length === bolts.length) { phase = "release"; rel = 0; }
      } else {
        wrong = { i, t: 0.7 };
        lost.push({ x: fig.x + 10, y: fig.y + 20, vx: 30 + 20 * Math.sin(order.length), vy: 46, a: 0, t: 0 });
        api.fumble("A dropped bolt is lost");
      }
    }

    return {
      step(i, isPart) {
        index = i;
        const r = api.rand();
        mode = "safe"; at = 0; openT = 0; miss = 0; kf = false; fi = 0; order = []; wrong = null; lost = [];
        rel = 0; cam = { s: 1, x: 640, y: 400 };
        openMax = Math.max(1.8, 3.4 - 0.4 * index);
        omega = Math.PI * 2 / Math.max(0.9, 1.4 - 0.12 * index);
        dmg = Math.min(6, 2 + index + Math.floor(r() * 2));
        // The route: along the hull from the hatch, up the pylon, along the coil's underside to the damaged segment,
        // a hold every 100-125 px (always within reach), and a few holds off the route.
        const way = [[HATCH.x, HATCH.y - 18], [472, hullY(472) - 20], [636, 290], [segX(dmg) + COIL.seg / 2, 290]];
        holds = [{ x: way[0][0], y: way[0][1], kind: "hull" }];
        for (let w = 1; w < way.length; w++) {
          const [ax, ay] = way[w - 1], [bx, by] = way[w], len = Math.hypot(bx - ax, by - ay);
          const n = Math.max(1, Math.ceil(len / (100 + 25 * r())));
          for (let k = 1; k <= n; k++) {
            const f = k / n, last = w === way.length - 1 && k === n;
            const kind = w === 1 ? "hull" : w === 2 ? "pylon" : "under";
            const jx = last ? 0 : (r() - 0.5) * 18, jy = last ? 0 : (r() - 0.5) * 14;
            let x = ax + (bx - ax) * f + jx, y = ay + (by - ay) * f + jy;
            if (kind === "hull") y = hullY(x) - 20;
            if (kind === "under" && !last) y = 290 + jy;
            holds.push({ x, y, kind });
          }
        }
        const target = holds.pop();
        for (let k = 0; k < 4; k++) {
          const over = k < 2, x = over ? segX(1) + r() * (segX(dmg) - segX(1) + 60) : 560 + r() * 260;
          const h = { x, y: over ? COIL.y0 - 22 : hullY(x) - 20, kind: over ? "over" : "hull" };
          if (holds.every((q) => !near(q.x, q.y, h.x, h.y, 70)) && !near(target.x, target.y, h.x, h.y, 70)) holds.push(h);
        }
        holds.push(target);
        fig = { x: holds[0].x, y: holds[0].y, a: Math.PI * 0.66 };
        bolts = [];
        const n = index === 0 ? 6 : 8, x0 = segX(dmg), y0 = COIL.y0, L = x0 + 16, R = x0 + COIL.seg - 16, T = y0 + 14, B = y0 + COIL.h - 14;
        const M = x0 + COIL.seg / 2, V = y0 + COIL.h / 2;
        const pts = n === 6 ? [[L, T], [M, T], [R, T], [R, B], [M, B], [L, B]] : [[L, T], [M, T], [R, T], [R, V], [R, B], [M, B], [L, B], [L, V]];
        bolts = pts.map(([x, y]) => ({ x, y, out: false, t: 0 }));
        const side = r() < 0.5 ? -1 : 1;
        seg = { x: side * (30 + 30 * r()), y: 150, vx: 0, vy: 0, ph: r() * 6, bias: side * -(4 + 3 * index), bump: 0 };
        drv = { x: CRATE.x, y: CRATE.y, held: false, set: false, gx: 0, gy: 0 };
        phase = isPart ? "part" : "climb";
      },
      update(dt, input) {
        if (api.combat) return;                           // design 5: no EVA in combat
        const h = input.hit, ease = Math.min(1, dt * 10);
        const dir = (h.has("ArrowRight") || h.has("KeyD") || h.has("ArrowDown") || h.has("KeyS") ? 1 : 0)
          - (h.has("ArrowLeft") || h.has("KeyA") || h.has("ArrowUp") || h.has("KeyW") ? 1 : 0);
        if (dir || input.actionPressed) kf = true;
        if (input.pressed) kf = false;
        if (wrong && (wrong.t -= dt) <= 0) wrong = null;
        miss = Math.max(0, miss - dt);
        for (const b of lost) { b.t += dt; b.x += b.vx * dt; b.y += b.vy * dt; b.a += dt * 5; }
        lost = lost.filter((b) => b.t < 3);
        for (const b of bolts) if (b.out) b.t = Math.min(1, b.t + dt / 0.35);
        const zoom = phase === "bolts" || phase === "release" || phase === "swap" || (phase === "done" && !drv.set);
        const tc = zoom ? { s: 2.3, x: seat().x, y: 296 } : { s: 1, x: 640, y: 400 };
        for (const k of ["s", "x", "y"]) cam[k] += (tc[k] - cam[k]) * Math.min(1, dt * 4);
        const goal = holds[mode === "open" ? to : at];
        fig.x += (goal.x - fig.x) * ease; fig.y += (goal.y - fig.y) * ease;
        const lean = { hull: 0.66, over: 1, pylon: 0.6, under: 0 }[goal.kind] * Math.PI;   // floating off the surface it holds
        fig.a += (lean - fig.a) * Math.min(1, dt * 5);

        if (phase === "part") {
          const [wx, wy] = toWorld(input.x, input.y);
          if (input.pressed && Math.abs(wx - drv.x) < 34 && Math.abs(wy - drv.y) < KIT.TOUCH_R + 4) { drv.held = true; drv.gx = wx - drv.x; drv.gy = wy - drv.y; }
          if (drv.held && input.down) { drv.x = wx - drv.gx; drv.y = wy - drv.gy; }
          const fit = () => { drv.set = true; drv.held = false; drv.x = SOCKET.x; drv.y = SOCKET.y; phase = "done"; api.stepDone(); };
          if (drv.held && input.released) { drv.held = false; if (near(drv.x, drv.y, SOCKET.x, SOCKET.y, 40)) return fit(); }
          if (!drv.held) { drv.x += (CRATE.x - drv.x) * ease; drv.y += (CRATE.y - drv.y) * ease; }
          if (kf && input.actionPressed) fit();
          return;
        }
        if (phase === "climb") {
          if (mode === "safe") {
            const reach = reachable();
            let go = -1;
            if (kf) { fi = wrapI(fi + dir, reach.length); if (input.actionPressed) go = reach[fi]; }
            else if (input.pressed) {
              const [wx, wy] = toWorld(input.x, input.y);
              // The nearest reachable hold within a fingertip's reach (KIT.nearest; holds are 70 px or more apart).
              const k = KIT.nearest(reach.map((i) => holds[i]), wx, wy, KIT.TOUCH_R + 10);
              go = k >= 0 ? reach[k] : -1;
            }
            if (go >= 0) { mode = "open"; from = at; to = go; openT = 0; swingT = 0; fi = 0; }
            return;
          }
          // One clip open: clip it as its hook crosses the hold, before the tether takes up.
          openT += dt; swingT += dt;
          if (input.pressed || input.actionPressed) {
            const k = hook();
            if (near(k.x, k.y, holds[to].x, holds[to].y, 13)) { at = to; return arrive(); }
            miss = 0.25;
          }
          if (openT >= openMax) { mode = "safe"; at = from; api.fumble("The tether snaps you back"); }
          return;
        }
        if (phase === "bolts") {
          if (kf) { fi = wrapI(fi + dir, bolts.length); if (input.actionPressed) unbolt(fi); }
          else if (input.pressed) {
            const [wx, wy] = toWorld(input.x, input.y);
            // The bolts are 44 px apart in the world, seen at 2.3x: the nearest one within 20 world px, 46 on screen.
            const i = KIT.nearest(bolts.map((b) => (b.out ? null : b)), wx, wy, 20);
            if (i >= 0) unbolt(i);
          }
          return;
        }
        if (phase === "release") { if ((rel += dt / 1.2) >= 1) { rel = 1; phase = "swap"; } return; }
        if (phase === "swap") {
          // The new segment: offset from its seat, pushed by the drift, the pointer's pull or the stick.
          const tol = Math.max(4, 8 - index);
          let ax = (16 + 7 * index) * Math.sin(seg.ph) + seg.bias, ay = 0;
          seg.ph += dt * 0.8;
          if (input.down && !kf) {
            const [wx, wy] = toWorld(input.x, input.y), s = seat();
            ax += (wx - (s.x + seg.x)) * 6 - seg.vx * 3; ay += (wy - (s.y + seg.y)) * 6 - seg.vy * 3;
          } else if (input.stick.x || input.stick.y) {
            ax += input.stick.x * 70 - seg.vx * 1.5; ay += input.stick.y * 70 - seg.vy * 1.5;
          }
          seg.vx += ax * dt; seg.vy += ay * dt;
          seg.vx *= 1 - 0.3 * dt; seg.vy *= 1 - 0.3 * dt;
          const prevY = seg.y;
          seg.x += seg.vx * dt; seg.y += seg.vy * dt;
          seg.bump = Math.max(0, seg.bump - dt);
          if (seg.y < COIL.h && prevY >= COIL.h && Math.abs(seg.x) > tol) {
            seg.y = COIL.h; seg.vy = Math.abs(seg.vy) * 0.4 + 25; seg.vx *= 0.5; seg.bump = 0.4;    // caught on the flange
          }
          if (seg.y < COIL.h) seg.x = Math.max(-tol, Math.min(tol, seg.x));
          seg.y = Math.min(seg.y, 260);
          if (seg.y <= 0) { seg.x = 0; seg.y = 0; phase = "done"; api.stepDone(); }
        }
      },
      /** For tools: the round's state, read only, so a script can play it. */
      peek() {
        return { phase, mode, at, to, holds, REACH, hook: mode === "open" ? hook() : null, bolts, seg, seat: seat(), drv, SOCKET,
          tol: Math.max(4, 8 - index), toScreen: (x, y) => [(x - cam.x) * cam.s + 640, (y - cam.y) * cam.s + 400] };
      },
      draw(g, t) {
        g.fillStyle = "#020409"; g.fillRect(0, api.BAR_H, api.W, api.H);
        for (const s of stars) D.disc(g, s.x, s.y, s.r, `rgba(232,238,246,${0.35 + 0.3 * Math.sin(t * 0.8 + s.p)})`);
        g.save();
        g.translate(640, 400); g.scale(cam.s, cam.s); g.translate(-cam.x, -cam.y);
        drawHull(g, t);
        drawCoil(g, t);
        drawHolds(g, t);
        drawSuit(g, t);
        for (const b of lost) { g.save(); g.translate(b.x, b.y); g.rotate(b.a); g.globalAlpha = 1 - b.t / 3; hex(g, 0, 0, 6); g.fillStyle = "#c9d3df"; g.fill(); g.restore(); }
        g.restore();
        if (api.combat) {
          g.fillStyle = "rgba(4,6,10,0.6)"; g.fillRect(0, api.BAR_H, api.W, api.H);
          D.panel(g, 250, 330, 780, 120, 20, "#2a0c10", C.danger);
          D.ring(g, 330, 390, 34, C.danger, 7);
          g.beginPath(); g.moveTo(306, 366); g.lineTo(354, 414); g.strokeStyle = C.danger; g.lineWidth = 7; g.stroke();
          D.text(g, "EVA REFUSED: IN COMBAT", 680, 392, 46, C.danger, "center", 700);
        }
      },
    };

    function hex(g, x, y, r) {
      g.beginPath();
      for (let k = 0; k < 6; k++) { const a = Math.PI / 6 + k * Math.PI / 3; g[k ? "lineTo" : "moveTo"](x + r * Math.cos(a), y + r * Math.sin(a)); }
      g.closePath();
    }
    /** The hull's curve, its seams and the dorsal airlock; the pylon on it; the coil driver's socket. */
    function drawHull(g) {
      g.beginPath(); g.arc(HULL.x, HULL.y, HULL.r, Math.PI * 1.2, Math.PI * 1.8);
      g.fillStyle = "#151d2a"; g.fill(); g.strokeStyle = C.steel; g.lineWidth = 3; g.stroke();
      g.strokeStyle = "#0d131c"; g.lineWidth = 2;
      for (let x = -40; x < 1340; x += 150) {
        const y = hullY(x), nx = (x - HULL.x) / HULL.r, ny = (y - HULL.y) / HULL.r;
        g.beginPath(); g.moveTo(x - nx * 2, y - ny * 2); g.lineTo(x - nx * 140, y - ny * 140); g.stroke();
      }
      D.round(g, HATCH.x - 40, HATCH.y - 7, 80, 16, 8); g.fillStyle = "#0b0f15"; g.fill();
      g.strokeStyle = api.combat ? C.danger : C.amber; g.lineWidth = 3; g.stroke();
      // The pylon: a swept strut from the hull to the coil, with its ribs and the warp feed.
      const pts = [[398, hullY(398) + 6], [548, hullY(548) + 6], [702, COIL.y0 + COIL.h], [584, COIL.y0 + COIL.h]];
      g.beginPath(); pts.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y))); g.closePath();
      g.fillStyle = "#222b3a"; g.fill(); g.strokeStyle = C.steel; g.lineWidth = 2; g.stroke();
      g.strokeStyle = "#2c3a52"; g.lineWidth = 2;
      for (let f = 0.15; f < 1; f += 0.17) {
        const ax = pts[0][0] + (pts[3][0] - pts[0][0]) * f, ay = pts[0][1] + (pts[3][1] - pts[0][1]) * f;
        const bx = pts[1][0] + (pts[2][0] - pts[1][0]) * f, by = pts[1][1] + (pts[2][1] - pts[1][1]) * f;
        g.beginPath(); g.moveTo(ax, ay); g.lineTo(bx, by); g.stroke();
      }
      g.strokeStyle = "rgba(190,159,230,0.35)"; g.lineWidth = 4;
      g.beginPath(); g.moveTo(500, hullY(500)); g.lineTo(660, COIL.y0 + COIL.h); g.stroke();
      if (phase === "part" || drv.set) {
        if (!drv.set) { g.setLineDash([6, 5]); g.strokeStyle = C.amber; g.lineWidth = 3; g.strokeRect(SOCKET.x - 30, SOCKET.y - 20, 60, 40); g.setLineDash([]); }
        if (!drv.set) { g.fillStyle = "#141b27"; g.fillRect(CRATE.x - 44, CRATE.y - 26, 88, 52); g.strokeStyle = C.line; g.lineWidth = 2; g.strokeRect(CRATE.x - 44, CRATE.y - 26, 88, 52); }
        D.round(g, drv.x - 28, drv.y - 18, 56, 36, 6); g.fillStyle = "#24314a"; g.fill(); g.strokeStyle = C.accent; g.lineWidth = 2; g.stroke();
        g.fillStyle = C.copper; g.fillRect(drv.x - 18, drv.y - 4, 36, 8);
      }
    }
    function drawSeg(g, x, y, kind, t) {
      D.round(g, x + 3, y, COIL.seg - 6, COIL.h, 10);
      g.fillStyle = kind === "dmg" ? "#121720" : "#1b2433"; g.fill(); g.strokeStyle = "#2c3a52"; g.lineWidth = 2; g.stroke();
      for (let k = 0; k < 6; k++) {
        g.fillStyle = kind === "dmg" ? "#241d2a" : `rgba(190,159,230,${0.5 + 0.25 * Math.sin(t * 2 + k + x * 0.02)})`;
        g.fillRect(x + 14 + k * 17.6, y + 8, 8, COIL.h - 16);
      }
      if (kind === "dmg") {
        g.strokeStyle = C.danger; g.lineWidth = 3; g.beginPath();
        g.moveTo(x + 34, y + 4); g.lineTo(x + 52, y + 30); g.lineTo(x + 44, y + 46); g.lineTo(x + 70, y + 64); g.lineTo(x + 62, y + 84); g.stroke();
        if (Math.sin(t * 7) > 0.3) {
          g.strokeStyle = C.warn; g.lineWidth = 2;
          for (let k = 0; k < 4; k++) { const a = k * 1.6 + t * 3; g.beginPath(); g.moveTo(x + 48, y + 36); g.lineTo(x + 48 + 14 * Math.cos(a), y + 36 + 14 * Math.sin(a)); g.stroke(); }
        }
      }
    }
    /** The warp coil: its segments between flanges, the damaged one (or its gap, or its replacement), the bolts. */
    function drawCoil(g, t) {
      const { x0, y0, h, seg: w, n } = COIL, xe = x0 + n * w;
      D.round(g, x0 - 40, y0 + 6, 46, h - 12, 12); g.fillStyle = "#2a3446"; g.fill(); g.strokeStyle = C.steel; g.lineWidth = 2; g.stroke();
      g.beginPath(); g.moveTo(xe, y0); g.lineTo(xe + 54, y0 + 30); g.lineTo(xe + 54, y0 + h - 30); g.lineTo(xe, y0 + h); g.closePath();
      g.fillStyle = "#2a3446"; g.fill(); g.stroke();
      for (let k = 0; k < n; k++) {
        if (k !== dmg) { drawSeg(g, segX(k), y0, "ok", t); continue; }
        if (phase === "climb" || phase === "bolts" || phase === "part" || (phase === "done" && drv.set) || phase === "release") {
          g.save();
          if (phase === "release") {
            g.globalAlpha = 1 - rel; g.translate(segX(k) + w / 2 + rel * 90, y0 + h / 2 - rel * 160); g.rotate(rel * 0.5); g.translate(-(segX(k) + w / 2), -(y0 + h / 2));
          }
          drawSeg(g, segX(k), y0, "dmg", t);
          if (phase !== "release") drawBolts(g, t);
          g.restore();
        }
      }
      for (let k = 0; k <= n; k++) { g.fillStyle = C.steel; g.fillRect(x0 + k * w - 5, y0 - 8, 10, h + 16); }
      if (phase === "swap" || (phase === "done" && !drv.set)) {
        const s = seat(), x = s.x + seg.x - w / 2, y = s.y + seg.y - h / 2, tol = Math.max(4, 8 - index), square = Math.abs(seg.x) <= tol;
        // The gap's mouth: brackets, solid and green when the segment is square to it.
        g.strokeStyle = phase === "done" ? C.ok : square ? C.ok : seg.bump > 0 ? C.danger : C.amber; g.lineWidth = 3;
        g.setLineDash(square || phase === "done" ? [] : [5, 4]);
        for (const sx of [-1, 1]) {
          const bx = s.x + sx * (w / 2 - 3), by = y0 + h + 4;
          g.beginPath(); g.moveTo(bx - sx * 14, by + 10); g.lineTo(bx, by + 10); g.lineTo(bx, by - 8); g.stroke();
        }
        g.setLineDash([]);
        g.strokeStyle = "rgba(201,211,223,0.5)"; g.lineWidth = 1.5; g.beginPath(); g.moveTo(fig.x, fig.y + 30); g.lineTo(x + w / 2, y + h); g.stroke();
        drawSeg(g, x, y, "new", t);
      }
    }
    function drawBolts(g, t) {
      const loose = phase === "bolts";
      bolts.forEach((b, i) => {
        if (b.out) { D.disc(g, b.x, b.y, 5, "#05070b"); D.ring(g, b.x, b.y, 6, "#4a5568", 1.5); return; }
        hex(g, b.x, b.y, 6); g.fillStyle = "#c9d3df"; g.fill(); g.strokeStyle = "#2a3446"; g.lineWidth = 1.5; g.stroke();
        // Every bolt's number in the star, the next one lit (the owner's note on the fighter, the same here).
        if (loose) D.orderBadge(g, b.x, b.y, 6, orderNo(i, bolts.length), nextInStar(order, bolts.length)[0] === i, t);
        if (loose && kf && fi === i) { g.setLineDash([3, 3]); D.ring(g, b.x, b.y, 11, C.amber, 1.5); g.setLineDash([]); }
        if (wrong && wrong.i === i) {
          g.strokeStyle = C.danger; g.lineWidth = 2.5; g.beginPath();
          g.moveTo(b.x - 9, b.y - 9); g.lineTo(b.x + 9, b.y + 9); g.moveTo(b.x + 9, b.y - 9); g.lineTo(b.x - 9, b.y + 9); g.stroke();
        }
      });
      // A bolt on its way out to the pouch.
      for (const b of bolts) if (b.out && b.t < 1) { g.globalAlpha = 1 - b.t; hex(g, b.x + (fig.x - b.x) * b.t, b.y + (fig.y + 20 - b.y) * b.t, 6); g.fillStyle = "#c9d3df"; g.fill(); g.globalAlpha = 1; }
    }
    /** The handholds: rails on posts. In reach, ringed; the reach itself, dashed round the suit. */
    function drawHolds(g, t) {
      const climbing = phase === "climb" && mode === "safe";
      const reach = climbing ? reachable() : [];
      if (climbing) { g.setLineDash([6, 8]); D.ring(g, holds[at].x, holds[at].y, REACH, "rgba(79,195,247,0.18)", 2); g.setLineDash([]); }
      holds.forEach((hd, i) => {
        g.strokeStyle = C.steel; g.lineWidth = 3;
        const top = hd.kind === "under" ? COIL.y0 + COIL.h : hd.kind === "over" ? COIL.y0 - 8 : null;
        if (top !== null) { g.beginPath(); g.moveTo(hd.x - 9, hd.y); g.lineTo(hd.x - 9, top); g.moveTo(hd.x + 9, hd.y); g.lineTo(hd.x + 9, top); g.stroke(); }
        if (hd.kind === "hull") { g.beginPath(); g.moveTo(hd.x - 9, hd.y); g.lineTo(hd.x - 9, hd.y + 18); g.moveTo(hd.x + 9, hd.y); g.lineTo(hd.x + 9, hd.y + 18); g.stroke(); }
        D.round(g, hd.x - 13, hd.y - 4, 26, 8, 4); g.fillStyle = "#c9a640"; g.fill();
        if (reach.includes(i)) {
          D.ring(g, hd.x, hd.y, 17, C.accent, 2.5);
          if (kf && reach[fi] === i) { g.setLineDash([5, 4]); D.ring(g, hd.x, hd.y, 25, C.amber, 2.5); g.setLineDash([]); }
        }
      });
    }
    /** The suit: hands on the hold, two lanyards to two clips; one open hook when moving, with its time running out. */
    function drawSuit(g, t) {
      const x = fig.x, y = fig.y, open = mode === "open" && phase === "climb";
      const sway = fig.a + 0.1 * Math.sin(t * 0.9) + (open ? 0.18 * Math.sin(omega * swingT) : 0);
      const waist = { x: x - Math.sin(sway) * 44, y: y + Math.cos(sway) * 44 };
      const anchor = holds[open ? from : at], k = open ? hook() : null;
      // The lanyards.
      g.strokeStyle = "#c9d3df"; g.lineWidth = 2;
      g.beginPath(); g.moveTo(waist.x, waist.y); g.lineTo(anchor.x - 4, anchor.y + 2); g.stroke();
      g.beginPath(); g.moveTo(waist.x, waist.y); g.lineTo(open ? k.x : anchor.x + 4, open ? k.y : anchor.y + 2); g.stroke();
      D.ring(g, anchor.x - 4, anchor.y + 2, 5, C.ok, 3);
      if (!open) D.ring(g, anchor.x + 4, anchor.y + 2, 5, C.ok, 3);
      g.save(); g.translate(x, y); g.rotate(sway);
      // Side on: backpack behind, sleeves up to the gloves on the rail, the helmet's gold visor to the right.
      g.fillStyle = "#3a4456"; D.round(g, -17, 20, 12, 28, 4); g.fill();
      g.strokeStyle = "#d9dee6"; g.lineWidth = 8; g.lineCap = "round";
      g.beginPath(); g.moveTo(-6, 26); g.lineTo(-3, 6); g.moveTo(6, 26); g.lineTo(4, 6); g.stroke();
      g.beginPath(); g.moveTo(-4, 50); g.lineTo(-7, 72); g.moveTo(5, 50); g.lineTo(10, 70); g.stroke(); g.lineCap = "butt";
      D.disc(g, -3, 3, 4.5, "#4a5568"); D.disc(g, 4, 3, 4.5, "#4a5568");
      D.round(g, -10, 20, 22, 34, 8); g.fillStyle = "#d9dee6"; g.fill();
      D.disc(g, 2, 17, 11, "#d9dee6");
      g.beginPath(); g.ellipse(7, 17, 5, 7, 0, 0, Math.PI * 2); g.fillStyle = "#d8a23a"; g.fill();
      g.fillStyle = C.accent; g.fillRect(-4, 32, 12, 4);
      g.restore();
      if (open) {
        // The open hook: a C, amber, ringed by the time it has left.
        const left = 1 - openT / openMax;
        g.beginPath(); g.arc(k.x, k.y, 6, 0.6, Math.PI * 2 - 0.6); g.strokeStyle = miss > 0 ? C.danger : C.amber; g.lineWidth = 3; g.stroke();
        g.beginPath(); g.arc(k.x, k.y, 12, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * left);
        g.strokeStyle = left < 0.3 ? C.danger : C.amber; g.lineWidth = 3; g.stroke();
      }
    }
  },
});
