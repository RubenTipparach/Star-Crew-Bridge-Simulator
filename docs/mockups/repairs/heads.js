/*
 * repairs/heads.js: the toilets' and showers' repair (openspec/changes/repair-minigames, design 2; the fixtures
 * themselves are openspec/changes/ship-interactables, "toilets that you can flush, showers you can turn on").
 *
 * The id stays "heads" (the hub loads repairs/heads.js by it); the screen says "Toilets and showers". Rounds alternate
 * by step: even steps are a toilet, odd steps a shower.
 *
 * Toilet: a clogged toilet seen from the front, lid up, a plunger standing in the bowl. Plunge it: drag the plunger
 * down and up in the bowl (or Down then Up on the keys, or tap Space for a whole stroke). Each full stroke pushes the
 * clog along the trap, drawn in the cutaway beside the toilet; the ring on the plunger's knob shows the safe rhythm
 * (dashed and filling: too soon; solid: go). A stroke too soon sloshes the water up the bowl, and water over the rim is
 * the fumble "Overflow: the floor is wet" (a puddle, a wet floor sign); the level drains back slowly. Pressing the
 * flush on a clog fills the bowl to overflowing too. Once the clog goes the culprit pops up (a rubber duck), and the
 * flush handle glows: press it (click it, Enter or Space) and the water swirls away and refills clean. Later toilets:
 * a tougher clog (more strokes), a narrower safe rhythm (a longer wait between strokes, a bigger slosh, a slower
 * drain, and suction lost sooner if you dawdle).
 *
 * Shower: a pipe puzzle in the deckhead. Water comes down the main on the left; the shower waits on the right. Click
 * a tile to turn it a quarter (or move the cursor with the arrows and turn it with Space) until one run joins the
 * main to the shower with no open end anywhere along it, then open the valve on the main (click it, or Enter): the
 * water runs through and the shower runs. Later showers have bigger grids. Opening the valve on an unfinished run: a
 * fumble, "Loose joint: you are soaked".
 *
 * A disabled toilet's first step fits the new flush valve (the flapper): the cistern is open, drag the flapper from
 * the crate onto its seat at the cistern's bottom (or move it with the arrows and drop it with Space).
 */
RepairKit.register({
  id: "heads",
  title: "Toilets and showers",
  place: "Crew quarters",
  group: "Life",
  hazard: "Overflow: the floor is wet",
  down: "Comfort and morale (design 4)",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const OVERFLOW = "Overflow: the floor is wet";
    const SOAKED = "Loose joint: you are soaked";
    const clamp01 = (v) => Math.max(0, Math.min(1, v));
    const ease = (u) => { u = clamp01(u); return u * u * (3 - 2 * u); };
    const lerp = (a, b, u) => a + (b - a) * u;
    let s;

    // ================================================================ the toilet
    const TX = 590, FLOOR = 652;                          // the toilet's centre line and the floor, px
    const RIM = { y: 388, rx: 186, ry: 66 };              // the bowl's rim
    const HOLE = { y: 392, rx: 136, ry: 44 };             // the seat's opening
    const CIS = { x: TX - 136, y: 112, w: 272, h: 150 };  // the cistern
    const PIV = { x: TX + 124, y: 150 }, LEVER = 66;      // the flush handle: pivot, lever length
    const HANDLE_REST = 0.08, HANDLE_DOWN = 0.62;         // its angle at rest and pressed, rad
    const TRAVEL = 120;                                   // pointer travel for a whole stroke, px
    const SEAT = { x: TX - 34, y: 244 };                  // the flapper's seat in the open cistern (part step)
    const CRATE = { x: 960, y: 520, w: 190, h: 120 };     // the spare flapper's crate (part step)
    const COL = { x: 290, y: 300, w: 34, h: 250 };        // the bowl's level column
    // The clog's tuning by toilet (the 1st, 2nd, 3rd ...): strokes to clear, the safe gap between strokes (s), the
    // slosh of a stroke too soon and of a good one, the drain back (level a second). Level 1 is the rim.
    const TUNE = [
      { need: 6, minGap: 0.5, maxGap: 3.0, slosh: 0.27, bump: 0.05, drain: 0.16 },
      { need: 9, minGap: 0.65, maxGap: 2.5, slosh: 0.31, bump: 0.06, drain: 0.13 },
      { need: 12, minGap: 0.8, maxGap: 2.1, slosh: 0.35, bump: 0.07, drain: 0.11 },
    ];
    const CLOGGED = 0.58, CLEAR = 0.45;                   // the bowl's resting level clogged and clear

    // The trap in cutaway (the clog gauge): from the bowl's outlet, under and over the weir, down to the floor.
    const TRAP_PTS = [[946, 296], [944, 370], [958, 432], [996, 466], [1036, 450], [1058, 404], [1090, 374],
      [1128, 386], [1146, 436], [1150, 520], [1150, 588]];
    const TRAP = (() => {
      const pts = [];
      for (let i = 0; i < TRAP_PTS.length - 1; i++) {
        const p0 = TRAP_PTS[Math.max(0, i - 1)], p1 = TRAP_PTS[i], p2 = TRAP_PTS[i + 1], p3 = TRAP_PTS[Math.min(TRAP_PTS.length - 1, i + 2)];
        for (let j = 0; j < 12; j++) {
          const u = j / 12, u2 = u * u, u3 = u2 * u;
          pts.push([0, 1].map((k) => 0.5 * (2 * p1[k] + (-p0[k] + p2[k]) * u + (2 * p0[k] - 5 * p1[k] + 4 * p2[k] - p3[k]) * u2 + (-p0[k] + 3 * p1[k] - 3 * p2[k] + p3[k]) * u3)));
        }
      }
      pts.push(TRAP_PTS[TRAP_PTS.length - 1]);
      const len = [0];
      for (let i = 1; i < pts.length; i++) len.push(len[i - 1] + Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]));
      return { pts, len, total: len[len.length - 1] };
    })();
    /** A point a fraction u along the trap. */
    function along(u) {
      const want = clamp01(u) * TRAP.total;
      let i = 1;
      while (i < TRAP.len.length - 1 && TRAP.len[i] < want) i++;
      const a = TRAP.pts[i - 1], b = TRAP.pts[i], k = (want - TRAP.len[i - 1]) / Math.max(1e-6, TRAP.len[i] - TRAP.len[i - 1]);
      return [lerp(a[0], b[0], k), lerp(a[1], b[1], k)];
    }
    const LUMP_END = 0.86;                                // the lump sits this far along when the last stroke lands

    function toiletStep(index, isPart, r) {
      const tune = TUNE[Math.min(Math.floor(index / 2), TUNE.length - 1)];
      s = {
        kind: isPart ? "part" : "toilet", index, r, tune,
        phase: isPart ? "part" : "plunge",
        level: isPart ? CLEAR : CLOGGED, rest: isPart ? CLEAR : CLOGGED, wob: 0, rising: false, spill: false, spillT: 0,
        puddle: 0, puddleT: 0, prog: 0, strokes: 0, last: -99, clock: 0, clearT: 0, out: 0,
        ft: 0, swirl: 0, flushed: false, press: 0, hAng: HANDLE_REST, fresh: 0,
        plunger: { d: 0, target: 0, grab: false, gy: 0, gd: 0, auto: false, bottom: false },
        drops: [],
        // The wad's outline, jittered once a step so it is the same in every shot.
        wad: Array.from({ length: 11 }, () => 0.75 + 0.45 * r()),
        flapper: { x: CRATE.x + CRATE.w / 2, y: CRATE.y + CRATE.h / 2 + 6, held: false, pad: false, set: false },
      };
    }
    function splash(n, x, y, up = 1) {
      for (let i = 0; i < n; i++) {
        s.drops.push({ x: x + (s.r() - 0.5) * 120, y, vx: (s.r() - 0.5) * 260, vy: -(160 + s.r() * 260) * up, life: 0.7 + s.r() * 0.4 });
      }
    }
    const handleTip = (a = s.hAng) => [PIV.x + Math.cos(a) * LEVER, PIV.y + Math.sin(a) * LEVER];
    function onHandle(x, y) {
      const [hx, hy] = handleTip();
      return Math.hypot(x - hx, y - hy) < 46 || Math.hypot(x - (PIV.x + hx) / 2, y - (PIV.y + hy) / 2) < 30;
    }

    function stroke() {
      const k = s.tune, gap = s.clock - s.last, fast = s.strokes > 0 && gap < k.minGap;
      s.last = s.clock; s.strokes++; s.wob = 1;
      if (fast) { s.level += k.slosh; s.prog += 0.5; splash(7, TX, HOLE.y - 10); }
      else { s.level += k.bump; s.prog += 1; }
      if (s.prog >= k.need) { s.prog = k.need; s.phase = "ready"; s.rest = CLEAR; s.clearT = 0; s.plunger.grab = false; }
    }

    function toiletUpdate(dt, input) {
      const k = s.tune, P = s.plunger;
      s.clock += dt;
      s.wob = Math.max(0, s.wob - dt * 1.4);
      s.press = Math.max(0, s.press - dt);
      s.hAng += ((s.press > 0 ? HANDLE_DOWN : HANDLE_REST) - s.hAng) * Math.min(1, dt * 14);
      s.puddleT = Math.min(s.puddle, s.puddleT + dt * 1.5);
      s.spillT = Math.max(0, s.spillT - dt);
      for (const p of s.drops) { p.vy += 900 * dt; p.x += p.vx * dt; p.y += p.vy * dt; p.life -= dt; }
      s.drops = s.drops.filter((p) => p.life > 0 && p.y < FLOOR + 30);
      if (s.phase !== "plunge") s.out = Math.min(1, s.out + dt * 2);
      if (s.phase === "ready") s.clearT += dt;

      if (s.phase === "part") { partUpdate(dt, input); return; }

      // The water: it rises while a clogged flush fills it, drains back to rest, and is the flush's own in a flush.
      if (s.phase === "flush") {
        s.ft += dt; s.swirl += dt * (3 + s.ft * 5);
        if (s.ft < 1.5) s.level = lerp(s.from, 0.03, ease(s.ft / 1.5));
        else s.level = lerp(0.03, CLEAR, ease((s.ft - 1.6) / 1.3));
        if (s.ft >= 1.5 && !s.flushed) { s.flushed = true; api.stepDone(); }
        if (s.ft >= 3.0) { s.phase = "fresh"; s.level = CLEAR; }
        return;
      }
      if (s.phase === "fresh") { s.fresh += dt; return; }
      if (s.rising) s.level += 0.55 * dt;
      else if (s.level > s.rest) s.level = Math.max(s.rest, s.level - (s.phase === "ready" ? 0.6 : k.drain) * dt);
      else s.level = Math.min(s.rest, s.level + 0.6 * dt);
      if (s.spill && s.level < 0.9) s.spill = false;
      if (s.level > 1 && !s.spill) {
        // Over the rim: the fumble. The level holds at the rim and drains back slowly.
        s.spill = true; s.rising = false; s.level = 1; s.puddle++; s.spillT = 1.6;
        splash(10, TX, RIM.y);
        api.fumble(OVERFLOW);
        return;                                          // a third fumble restarts the step: s is a new round now
      }

      const flushIt = (input.pressed && onHandle(input.x, input.y)) || input.hit.has("Enter") || (s.phase === "ready" && input.hit.has("Space"));
      if (flushIt) {
        s.press = 0.45;
        if (s.phase === "ready") { s.phase = "flush"; s.ft = 0; s.from = s.level; return; }
        if (!s.spill) s.rising = true;                   // flushing a clog: it only fills the bowl
        if (!input.hit.has("Enter")) return;
      }
      if (s.phase !== "plunge") return;

      // The plunger: the pointer drags it, the keys and Space drive it.
      const zone = input.x > TX - 160 && input.x < TX + 160 && input.y > 110 && input.y < HOLE.y + 80;
      if (input.pressed && zone) { P.grab = true; P.gy = input.y; P.gd = P.d; P.auto = false; }
      if (!input.down) P.grab = false;
      if (P.grab) P.d = clamp01(P.gd + (input.y - P.gy) / TRAVEL);
      else {
        if (input.hit.has("Space")) { P.auto = true; P.target = 1; }
        if (input.stick.y > 0) { P.target = 1; P.auto = false; }
        if (input.stick.y < 0) { P.target = 0; P.auto = false; }
        if (P.auto && P.d >= 0.97) { P.target = 0; P.auto = false; }
        P.d += Math.max(-8 * dt, Math.min(8 * dt, P.target - P.d));
      }
      if (P.d > 0.8) P.bottom = true;
      if (P.bottom && P.d < 0.25) { P.bottom = false; stroke(); }
      // Dawdle past the safe gap and the suction goes: the clog creeps back.
      if (s.phase === "plunge" && s.strokes > 0 && s.clock - s.last > k.maxGap) s.prog = Math.max(0, s.prog - 0.4 * dt);
    }

    function partUpdate(dt, input) {
      const f = s.flapper;
      if (f.set) return;
      if (input.stick.x || input.stick.y) { f.x += input.stick.x * 480 * dt; f.y += input.stick.y * 480 * dt; f.pad = true; }
      if (input.pressed && Math.hypot(input.x - f.x, input.y - f.y) < 56) f.held = true;
      if (f.held && input.down) { f.x = input.x; f.y = input.y; }
      if ((f.held && input.released) || (f.pad && input.actionPressed)) {
        f.held = false; f.pad = false;
        if (Math.hypot(f.x - SEAT.x, f.y - SEAT.y) < 42) { f.set = true; f.x = SEAT.x; f.y = SEAT.y; s.phase = "fitted"; api.stepDone(); }
      }
    }

    // ---------------------------------------------------------------- toilet drawing
    function porcelain(g, x0, x1) {
      const gr = g.createLinearGradient(x0, 0, x1, 0);
      gr.addColorStop(0, "#f2f5f8"); gr.addColorStop(0.55, "#d3dae3"); gr.addColorStop(1, "#97a3b2");
      return gr;
    }
    function room(g) {
      g.fillStyle = "#0b1119"; g.fillRect(0, api.BAR_H, api.W, FLOOR - api.BAR_H);
      g.fillStyle = "#111a26";
      for (let y = api.BAR_H + 44; y < FLOOR; y += 48) g.fillRect(0, y, api.W, 2);
      for (let x = 24; x < api.W; x += 48) g.fillRect(x, api.BAR_H, 2, FLOOR - api.BAR_H);
      g.fillStyle = "#1a2433"; g.fillRect(0, FLOOR - 16, api.W, 16);                       // the skirting
      g.fillStyle = "#0e141d"; g.fillRect(0, FLOOR, api.W, api.H - FLOOR);
      g.fillStyle = "#121a25";
      for (let x = 0; x < api.W; x += 64) g.fillRect(x, FLOOR, 2, api.H - FLOOR);
    }
    function cistern(g, open) {
      const { x, y, w, h } = CIS;
      g.fillStyle = "rgba(0,0,0,0.35)"; D.round(g, x + 8, y + 8, w, h, 14); g.fill();
      D.round(g, x, y, w, h, 14); g.fillStyle = porcelain(g, x, x + w); g.fill();
      g.lineWidth = 3; g.strokeStyle = "#7d8796"; g.stroke();
      if (open) {
        // The front cut away: inside, the water, the overflow tube, the fill valve and the empty flapper seat.
        D.round(g, x + 14, y + 10, w - 28, h - 22, 8); g.fillStyle = "#16202c"; g.fill();
        g.fillStyle = "rgba(79,170,230,0.35)"; g.fillRect(x + 14, y + 52, w - 28, h - 64);
        g.fillStyle = "rgba(160,215,245,0.5)"; g.fillRect(x + 14, y + 52, w - 28, 3);
        g.fillStyle = "#8d98a8"; g.fillRect(TX + 14, y + 26, 14, h - 50);                  // the overflow tube
        g.fillStyle = "#6b7686"; g.fillRect(TX + 86, y + 30, 18, h - 54);                  // the fill valve
        D.round(g, TX + 76, y + 64, 38, 26, 6); g.fillStyle = "#4a5566"; g.fill();         // its float
        g.strokeStyle = "#b7c0cc"; g.lineWidth = 5;                                         // the lever arm
        g.beginPath(); g.moveTo(PIV.x - 4, PIV.y); g.lineTo(SEAT.x + 4, PIV.y + 6); g.stroke();
        // The chain, hanging to the seat (taut once the flapper is on it).
        g.strokeStyle = "#9aa6b6"; g.lineWidth = 2; g.setLineDash([4, 3]);
        g.beginPath(); g.moveTo(SEAT.x + 4, PIV.y + 6);
        if (s.flapper.set) g.lineTo(SEAT.x + 2, SEAT.y - 12);
        else g.quadraticCurveTo(SEAT.x + 26, SEAT.y - 30, SEAT.x + 18, SEAT.y - 14);
        g.stroke(); g.setLineDash([]);
        g.beginPath(); g.ellipse(SEAT.x, SEAT.y + 2, 30, 9, 0, 0, Math.PI * 2); g.fillStyle = "#0a0e14"; g.fill();
        g.lineWidth = 3; g.strokeStyle = "#5a6577"; g.stroke();
      } else {
        D.round(g, x - 8, y - 14, w + 16, 20, 8); g.fillStyle = porcelain(g, x - 8, x + w + 8); g.fill();
        g.lineWidth = 2; g.strokeStyle = "#7d8796"; g.stroke();
      }
    }
    function handle(g, t, ready) {
      const [hx, hy] = handleTip();
      if (ready) {
        // The main action: the handle glows and shows which way it goes.
        const pulse = 0.5 + 0.5 * Math.sin(t * 5);
        D.disc(g, hx, hy, 44 + 6 * pulse, `rgba(242,160,70,${0.12 + 0.12 * pulse})`);
        D.ring(g, hx, hy, 44 + 6 * pulse, C.amber, 4);
        D.turnArrow(g, PIV.x, PIV.y, LEVER + 30, 0.05, 0.75, C.amber);
      }
      g.lineCap = "round";
      g.strokeStyle = "#5b6574"; g.lineWidth = 16;
      g.beginPath(); g.moveTo(PIV.x, PIV.y); g.lineTo(hx, hy); g.stroke();
      g.strokeStyle = "#d5dce5"; g.lineWidth = 10;
      g.beginPath(); g.moveTo(PIV.x, PIV.y); g.lineTo(hx, hy); g.stroke();
      g.lineCap = "butt";
      D.disc(g, hx, hy, 11, "#e8eef6"); D.ring(g, hx, hy, 11, "#7d8796", 2);
      D.disc(g, PIV.x, PIV.y, 15, "#b7c0cc"); D.ring(g, PIV.x, PIV.y, 15, "#6b7686", 3);
    }
    function bowlBody(g) {
      g.beginPath();
      g.moveTo(TX - RIM.rx, RIM.y);
      g.bezierCurveTo(TX - RIM.rx, RIM.y + 110, TX - 92, RIM.y + 148, TX - 80, RIM.y + 176);
      g.lineTo(TX - 100, FLOOR); g.lineTo(TX + 100, FLOOR); g.lineTo(TX + 80, RIM.y + 176);
      g.bezierCurveTo(TX + 92, RIM.y + 148, TX + RIM.rx, RIM.y + 110, TX + RIM.rx, RIM.y);
      g.closePath();
      g.fillStyle = porcelain(g, TX - RIM.rx, TX + RIM.rx); g.fill();
      g.lineWidth = 3; g.strokeStyle = "#7d8796"; g.stroke();
      g.fillStyle = "rgba(0,0,0,0.12)"; g.fillRect(TX - 100, FLOOR - 10, 200, 10);
    }
    function waterShape(L) { return { y: HOLE.y + 22 - 26 * L, rx: 30 + 104 * L, ry: 10 + 33 * L }; }
    function wad(g, x, y, sc) {
      g.beginPath();
      s.wad.forEach((k, i) => { const a = (i / s.wad.length) * Math.PI * 2; const px = x + Math.cos(a) * 30 * sc * k, py = y + Math.sin(a) * 18 * sc * k; i ? g.lineTo(px, py) : g.moveTo(px, py); });
      g.closePath(); g.fillStyle = "#e9e4d6"; g.fill(); g.lineWidth = 2; g.strokeStyle = "#b3ab97"; g.stroke();
      g.strokeStyle = "#c9c2ae"; g.lineWidth = 1.5;
      for (let i = 0; i < 3; i++) { g.beginPath(); g.moveTo(x - 18 * sc + i * 12 * sc, y - 8 * sc); g.lineTo(x - 10 * sc + i * 12 * sc, y + 6 * sc); g.stroke(); }
    }
    /** The culprit: a rubber duck. */
    function duck(g, x, y, sc, rot = 0) {
      g.save(); g.translate(x, y); g.rotate(rot); g.scale(sc, sc);
      g.beginPath(); g.ellipse(0, 0, 26, 15, 0, 0, Math.PI * 2); g.fillStyle = "#ffd23f"; g.fill();
      g.beginPath(); g.moveTo(-24, -2); g.lineTo(-36, -12); g.lineTo(-22, -10); g.closePath(); g.fill();      // the tail
      D.disc(g, 14, -16, 12, "#ffd23f");
      g.beginPath(); g.moveTo(24, -18); g.lineTo(36, -14); g.lineTo(24, -11); g.closePath(); g.fillStyle = "#f2803a"; g.fill();
      D.disc(g, 17, -19, 2.6, "#1a1a1a");
      g.beginPath(); g.ellipse(-4, 2, 12, 6, -0.3, 0, Math.PI * 2); g.fillStyle = "#f0bd2a"; g.fill();      // a wing
      g.restore();
    }
    function bowl(g, t) {
      // The rim, the seat, the opening, the water and what is in it.
      g.beginPath(); g.ellipse(TX, RIM.y, RIM.rx, RIM.ry, 0, 0, Math.PI * 2); g.fillStyle = "#e6ebf0"; g.fill();
      g.lineWidth = 3; g.strokeStyle = "#7d8796"; g.stroke();
      const lidDown = s.kind === "part";
      if (lidDown) {
        g.beginPath(); g.ellipse(TX, RIM.y - 4, RIM.rx - 6, RIM.ry - 4, 0, 0, Math.PI * 2); g.fillStyle = "#c6d0dc"; g.fill();
        g.lineWidth = 3; g.strokeStyle = "#8a96a6"; g.stroke();
        g.beginPath(); g.ellipse(TX, RIM.y - 10, RIM.rx - 30, RIM.ry - 18, 0, Math.PI * 1.1, Math.PI * 1.9); g.strokeStyle = "rgba(255,255,255,0.5)"; g.stroke();
        return;
      }
      g.beginPath(); g.ellipse(TX, RIM.y + 2, RIM.rx - 6, RIM.ry - 6, 0, 0, Math.PI * 2); g.fillStyle = "#c6d0dc"; g.fill();
      g.lineWidth = 2; g.strokeStyle = "#8a96a6"; g.stroke();
      // Near the rim, the rim is hatched red: the water is about to go over.
      g.save();
      g.beginPath(); g.ellipse(TX, HOLE.y, HOLE.rx, HOLE.ry, 0, 0, Math.PI * 2); g.clip();
      const gr = g.createRadialGradient(TX, HOLE.y + 22, 8, TX, HOLE.y, HOLE.rx);
      gr.addColorStop(0, "#4d5763"); gr.addColorStop(1, "#d6dce3");
      g.fillStyle = gr; g.fillRect(TX - HOLE.rx, HOLE.y - HOLE.ry, HOLE.rx * 2, HOLE.ry * 2);
      g.beginPath(); g.ellipse(TX, HOLE.y + 24, 26, 8, 0, 0, Math.PI * 2); g.fillStyle = "#151b23"; g.fill();
      const clogged = s.phase === "plunge";
      // The clog under the water, smaller as it goes, the duck jammed in it.
      if (clogged) {
        const sc = 1 - 0.55 * (s.prog / s.tune.need);
        wad(g, TX + 4, HOLE.y + 16, sc);
        duck(g, TX + 70, HOLE.y + 10, 0.78, 2.3);   // nose down in the drain, tail up beside the cup
      }
      const L = Math.min(1.02, s.level), w = waterShape(L), wob = s.wob * Math.sin(t * 15);
      const clean = s.phase === "flush" ? s.ft > 0.4 : !clogged;
      g.beginPath(); g.ellipse(TX, w.y + wob * 3, w.rx * (1 + 0.05 * wob), w.ry * (1 - 0.05 * wob), 0, 0, Math.PI * 2);
      g.fillStyle = clean ? "rgba(79,170,230,0.72)" : "rgba(112,132,104,0.82)"; g.fill();
      g.beginPath(); g.ellipse(TX - w.rx * 0.2, w.y - w.ry * 0.3 + wob * 3, w.rx * 0.5, w.ry * 0.25, 0, 0, Math.PI * 2);
      g.strokeStyle = clean ? "rgba(200,235,255,0.55)" : "rgba(190,200,170,0.4)"; g.lineWidth = 2; g.stroke();
      if (s.phase === "flush") {
        // The swirl: spiral arms turning down into the drain.
        g.strokeStyle = "rgba(230,245,255,0.75)"; g.lineWidth = 3;
        for (let arm = 0; arm < 4; arm++) {
          g.beginPath();
          for (let j = 0; j <= 18; j++) {
            const f = j / 18, a = s.swirl + arm * (Math.PI / 2) + f * 3.4, rr = (1 - f) * 0.92;
            const px = TX + Math.cos(a) * w.rx * rr, py = w.y + Math.sin(a) * w.ry * rr;
            j ? g.lineTo(px, py) : g.moveTo(px, py);
          }
          g.stroke();
        }
        D.disc(g, TX, w.y + 2, 8 + 10 * Math.min(1, s.ft), "rgba(20,40,60,0.6)");
      }
      // The duck: up from the drain when the clog goes, afloat, then away down the swirl.
      if (s.phase === "ready") {
        const up = ease(s.clearT / 0.6), bob = Math.sin(t * 3) * 3;
        duck(g, TX + 10, lerp(HOLE.y + 22, w.y - 10, up) + bob, lerp(0.4, 0.9, up), Math.sin(t * 2) * 0.12);
        if (s.clearT < 0.9) D.ring(g, TX, w.y, 20 + s.clearT * 90, `rgba(220,240,255,${0.8 - s.clearT * 0.85})`, 3);
      }
      if (s.phase === "flush" && s.ft < 1.3) {
        const f = s.ft / 1.3, a = s.swirl * 1.1, rr = (1 - f) * w.rx * 0.6;
        duck(g, TX + Math.cos(a) * rr, w.y - 10 + Math.sin(a) * rr * 0.32 + f * 14, 0.9 * (1 - 0.75 * f), a * 0.8);
      }
      g.restore();
      if (s.level > 0.82 && s.phase === "plunge") {
        g.save();
        g.beginPath(); g.ellipse(TX, RIM.y + 2, RIM.rx - 4, RIM.ry - 4, 0, 0, Math.PI * 2);
        g.ellipse(TX, HOLE.y, HOLE.rx + 6, HOLE.ry + 4, 0, 0, Math.PI * 2); g.clip("evenodd");
        D.hatch(g, TX - RIM.rx, RIM.y - RIM.ry, RIM.rx * 2, RIM.ry * 2 + 6, `rgba(255,71,87,${0.35 + 0.5 * clamp01((s.level - 0.82) / 0.18)})`);
        g.restore();
      }
      // Fresh: a squeak-clean glint or two.
      if (s.phase === "fresh" || (s.phase === "flush" && s.ft > 2.2)) {
        for (const [gx, gy, ph] of [[TX - 120, RIM.y - 30, 0], [TX + 140, RIM.y + 10, 1.7], [TX - 40, RIM.y + 120, 3.1], [TX + 60, CIS.y + 30, 4.4]]) {
          const k = 0.5 + 0.5 * Math.sin(t * 4 + ph), r = 6 + 10 * k;
          g.fillStyle = `rgba(255,255,255,${0.4 + 0.5 * k})`;
          g.beginPath(); g.moveTo(gx, gy - r); g.lineTo(gx + r * 0.25, gy - r * 0.25); g.lineTo(gx + r, gy); g.lineTo(gx + r * 0.25, gy + r * 0.25);
          g.lineTo(gx, gy + r); g.lineTo(gx - r * 0.25, gy + r * 0.25); g.lineTo(gx - r, gy); g.lineTo(gx - r * 0.25, gy - r * 0.25); g.closePath(); g.fill();
        }
      }
    }
    function lidUp(g) {
      g.beginPath(); g.ellipse(TX, 262, 116, 98, 0, 0, Math.PI * 2);
      g.fillStyle = "#bcc6d2"; g.fill(); g.lineWidth = 3; g.strokeStyle = "#8a96a6"; g.stroke();
      g.beginPath(); g.ellipse(TX, 262, 96, 80, 0, 0, Math.PI * 2); g.strokeStyle = "rgba(255,255,255,0.35)"; g.lineWidth = 2; g.stroke();
      g.fillStyle = "#8a96a6"; g.fillRect(TX - 60, 352, 24, 10); g.fillRect(TX + 36, 352, 24, 10);   // the hinges
    }
    function spillover(g, t) {
      if (s.spillT <= 0) return;
      const a = Math.min(1, s.spillT);
      g.strokeStyle = `rgba(79,195,247,${0.75 * a})`; g.lineWidth = 6; g.lineCap = "round";
      for (const [x0, sway] of [[-150, -1], [-80, -0.5], [10, 0.2], [96, 0.6], [160, 1]]) {
        g.beginPath(); g.moveTo(TX + x0, RIM.y + 40);
        for (let y = RIM.y + 40; y <= FLOOR; y += 12) g.lineTo(TX + x0 * (1 - (y - RIM.y) / 520) + sway * 18 + Math.sin(t * 9 + y * 0.05) * 3, y);
        g.stroke();
      }
      g.lineCap = "butt";
    }
    function plunger(g, t) {
      const P = s.plunger, sq = clamp01((P.d - 0.65) / 0.35);
      const e = ease(s.out);
      const cx = lerp(TX, TX + 236, e), cupY = lerp(HOLE.y + 8 + P.d * 24, FLOOR - 4, e), ang = lerp(0, 0.12, e);
      const w = 96 * (1 + 0.22 * sq), h = 42 * (1 - 0.3 * sq), stick = 205;
      if (e < 0.2) {
        // Ripples where the cup meets the water.
        const wy = waterShape(Math.min(1, s.level)).y;
        g.beginPath(); g.ellipse(TX, wy + 6, w / 2 + 12 + 6 * Math.sin(t * 6), 10, 0, 0, Math.PI * 2);
        g.strokeStyle = "rgba(200,235,255,0.35)"; g.lineWidth = 2; g.stroke();
      }
      g.save(); g.translate(cx, cupY); g.rotate(ang);
      g.fillStyle = "#a4733f"; g.fillRect(-7, -h - stick, 14, stick);
      g.fillStyle = "rgba(0,0,0,0.18)"; g.fillRect(2, -h - stick, 5, stick);
      D.disc(g, 0, -h - stick - 6, 14, "#7a4f28");
      g.beginPath(); g.moveTo(-w / 2, 0); g.bezierCurveTo(-w / 2, -h * 1.1, w / 2, -h * 1.1, w / 2, 0); g.closePath();
      g.fillStyle = "#d23c3c"; g.fill(); g.lineWidth = 2; g.strokeStyle = "#8e2626"; g.stroke();
      g.beginPath(); g.ellipse(0, 0, w / 2, 8, 0, 0, Math.PI * 2); g.fillStyle = "#9e2a2a"; g.fill();
      g.beginPath(); g.ellipse(-w * 0.18, -h * 0.55, w * 0.12, h * 0.16, -0.5, 0, Math.PI * 2); g.fillStyle = "rgba(255,255,255,0.25)"; g.fill();
      g.restore();
      return [cx, cupY - h - stick - 6];
    }
    function rhythm(g, t, kx, ky) {
      // The safe rhythm on the knob: dashed and filling, too soon; solid, go; grey dashed, the suction is going.
      const k = s.tune, gap = s.clock - s.last;
      g.save();
      if (s.strokes === 0 || (gap >= k.minGap && gap <= k.maxGap)) D.ring(g, kx, ky, 28, C.ok, 5);
      else if (gap < k.minGap) {
        D.ring(g, kx, ky, 28, "#2a3446", 5);
        g.setLineDash([7, 5]); g.beginPath(); g.arc(kx, ky, 28, -Math.PI / 2, -Math.PI / 2 + (Math.PI * 2 * gap) / k.minGap);
        g.strokeStyle = C.amber; g.lineWidth = 5; g.stroke();
      } else {
        g.setLineDash([4, 6]); D.ring(g, kx, ky, 28 + 2 * Math.sin(t * 8), C.steel, 4);
      }
      g.restore();
      if (s.strokes === 0) {
        // Up and down, once, beside the handle (a shape, not words), until the first stroke.
        const a = 0.35 + 0.35 * Math.sin(t * 4), x = kx - 58, y = ky + 80;
        g.fillStyle = `rgba(232,238,246,${a})`;
        for (const [dy, dir] of [[-26, -1], [26, 1]]) {
          g.beginPath(); g.moveTo(x - 14, y + dy - dir * 7); g.lineTo(x, y + dy + dir * 9); g.lineTo(x + 14, y + dy - dir * 7); g.closePath(); g.fill();
        }
      }
    }
    function levelColumn(g) {
      const { x, y, w, h } = COL, top = 1.12;
      const yAt = (v) => y + h - (h * v) / top;
      D.round(g, x - 6, y - 6, w + 12, h + 12, 10); g.fillStyle = "#0c121a"; g.fill(); g.lineWidth = 2; g.strokeStyle = C.line; g.stroke();
      D.hatch(g, x, yAt(1.12), w, yAt(0.85) - yAt(1.12), "rgba(255,71,87,0.5)");
      const v = Math.min(top, s.level);
      g.fillStyle = s.phase === "plunge" ? "rgba(112,132,104,0.95)" : "rgba(79,170,230,0.9)";
      g.fillRect(x, yAt(v), w, y + h - yAt(v));
      g.fillStyle = C.danger; g.fillRect(x - 10, yAt(1) - 2, w + 20, 4);                          // the rim
      // A bowl glyph at the foot, so the column reads as the bowl's water.
      g.beginPath(); g.moveTo(x - 6, y + h + 18); g.lineTo(x + w + 6, y + h + 18); g.quadraticCurveTo(x + w / 2, y + h + 46, x - 6, y + h + 18);
      g.fillStyle = "#c6d0dc"; g.fill();
    }
    function trapGauge(g, t) {
      D.panel(g, 868, 196, 352, 428, 18, "#0b1018");
      // The bowl in section, feeding the trap.
      g.beginPath(); g.moveTo(888, 226); g.lineTo(1006, 226); g.quadraticCurveTo(1000, 300, 958, 300); g.quadraticCurveTo(900, 296, 888, 226);
      g.fillStyle = "#c6d0dc"; g.fill();
      g.beginPath(); g.moveTo(900, 238); g.lineTo(994, 238); g.quadraticCurveTo(988, 290, 954, 290); g.quadraticCurveTo(910, 288, 900, 238);
      g.fillStyle = s.phase === "plunge" ? "rgba(112,132,104,0.9)" : "rgba(79,170,230,0.8)"; g.fill();
      const path = (u0, u1) => { g.beginPath(); for (let u = u0; u <= u1 + 1e-6; u += 0.01) { const [x, y] = along(u); u === u0 ? g.moveTo(x, y) : g.lineTo(x, y); } };
      path(0, 1);
      g.lineJoin = "round"; g.lineCap = "round";
      g.strokeStyle = "#c6d0dc"; g.lineWidth = 40; g.stroke();
      g.strokeStyle = "#0d131c"; g.lineWidth = 26; g.stroke();
      g.fillStyle = "#3a4658"; g.fillRect(1112, 588, 76, 12);                                   // the floor flange
      const need = s.tune.need, plunging = s.phase === "plunge";
      const lu = plunging ? (s.prog / need) * LUMP_END : LUMP_END + (1 - LUMP_END) * clamp01(s.clearT / 0.7);
      if (plunging) { path(0, lu); g.strokeStyle = "rgba(112,132,104,0.95)"; g.lineWidth = 18; g.stroke(); }
      else {
        path(0, 1); g.strokeStyle = "rgba(79,170,230,0.85)"; g.lineWidth = 18; g.stroke();
        g.setLineDash([10, 14]); g.lineDashOffset = -t * 90; g.strokeStyle = "rgba(220,240,255,0.7)"; g.lineWidth = 4; g.stroke(); g.setLineDash([]);
      }
      g.lineCap = "butt";
      // A mark a stroke along the trap: hollow ahead, filled behind the clog.
      for (let i = 0; i < need; i++) {
        const [x, y] = along(((i + 1) / need) * LUMP_END);
        if (i < Math.floor(s.prog) || !plunging) D.disc(g, x, y, 6, C.ok); else D.ring(g, x, y, 6, "#6f7f94", 2);
      }
      if (plunging || s.clearT < 0.7) {
        const [x, y] = along(lu);
        g.save(); g.globalAlpha = plunging ? 1 : 1 - clamp01(s.clearT / 0.7); wad(g, x, y, 0.8); g.restore();
      }
    }
    function puddleAndSign(g, t) {
      if (s.puddle <= 0) return;
      const k = s.puddleT;
      g.beginPath(); g.ellipse(TX, FLOOR + 26, 150 + 70 * k, 20 + 6 * k, 0, 0, Math.PI * 2);
      g.fillStyle = "rgba(79,195,247,0.3)"; g.fill();
      g.beginPath(); g.ellipse(TX - 60, FLOOR + 22, 60 + 20 * k, 4, 0, 0, Math.PI * 2); g.fillStyle = "rgba(220,240,255,0.25)"; g.fill();
      // The wet floor sign: an A-frame with a figure slipping, no words.
      const x = 150, y = FLOOR + 30;
      g.fillStyle = "#e0b100";
      g.beginPath(); g.moveTo(x - 48, y); g.lineTo(x - 18, y - 150); g.lineTo(x + 18, y - 150); g.lineTo(x + 48, y); g.closePath(); g.fill();
      g.lineWidth = 3; g.strokeStyle = "#7a6000"; g.stroke();
      g.beginPath(); g.moveTo(x - 30, y - 70); g.lineTo(x, y - 122); g.lineTo(x + 30, y - 70); g.closePath(); g.fillStyle = "#1a1a1a"; g.fill();
      g.beginPath(); g.moveTo(x - 24, y - 74); g.lineTo(x, y - 115); g.lineTo(x + 24, y - 74); g.closePath(); g.fillStyle = "#e0b100"; g.fill();
      g.save(); g.translate(x, y - 88); g.rotate(-0.5 + 0.05 * Math.sin(t * 3));
      g.strokeStyle = "#1a1a1a"; g.lineWidth = 3; g.lineCap = "round";
      D.disc(g, 4, -16, 4, "#1a1a1a");
      g.beginPath(); g.moveTo(2, -11); g.lineTo(-2, 2); g.moveTo(-2, 2); g.lineTo(-10, 10); g.moveTo(-2, 2); g.lineTo(9, 7);
      g.moveTo(1, -7); g.lineTo(-9, -12); g.moveTo(1, -7); g.lineTo(11, -4); g.stroke();
      g.restore(); g.lineCap = "butt";
      g.fillStyle = "#1a1a1a"; g.fillRect(x - 22, y - 22, 44, 5);
    }
    function crateAndFlapper(g, t) {
      D.panel(g, CRATE.x, CRATE.y, CRATE.w, CRATE.h, 14, "#141b27");
      D.hatch(g, CRATE.x + 14, CRATE.y + CRATE.h - 22, CRATE.w - 28, 10, "rgba(242,160,70,0.5)");
      const f = s.flapper;
      if (!f.set) {
        // The empty seat waits, ringed amber.
        D.ring(g, SEAT.x, SEAT.y + 2, 40 + 3 * Math.sin(t * 5), C.amber, 3);
      }
      flapperGlyph(g, f.x, f.y, f.set);
      if (f.set) D.ring(g, SEAT.x, SEAT.y + 2, 40, C.ok, 4);
      else if (!f.held) D.ring(g, f.x, f.y, 52, "#f0c08a", 3);
    }
    function flapperGlyph(g, x, y, seated) {
      g.beginPath(); g.ellipse(x, y, 32, 11, 0, 0, Math.PI * 2); g.fillStyle = "#b23232"; g.fill();
      g.beginPath(); g.ellipse(x, y - 4, 22, 13, 0, Math.PI, Math.PI * 2); g.fillStyle = "#d23c3c"; g.fill();
      g.fillStyle = "#8e2626"; g.fillRect(x - 40, y - 4, 12, 7); g.fillRect(x + 28, y - 4, 12, 7);            // the hinge ears
      D.ring(g, x + 2, y - 18, 5, "#9aa6b6", 2);                                                             // the chain loop
      if (!seated) { g.beginPath(); g.ellipse(x - 8, y - 9, 7, 3, -0.3, 0, Math.PI * 2); g.fillStyle = "rgba(255,255,255,0.3)"; g.fill(); }
    }
    function leaningLid(g) {
      // The cistern's lid, set down on the floor out of the way.
      D.round(g, 120, FLOOR - 4, 288, 26, 8); g.fillStyle = porcelain(g, 120, 408); g.fill(); g.lineWidth = 2; g.strokeStyle = "#7d8796"; g.stroke();
      g.fillStyle = "rgba(255,255,255,0.45)"; g.fillRect(132, FLOOR, 264, 3);
    }
    function toiletDraw(g, t) {
      room(g);
      const part = s.kind === "part";
      if (part) leaningLid(g);
      cistern(g, part);
      handle(g, t, s.phase === "ready");
      if (!part) lidUp(g);
      bowlBody(g);
      bowl(g, t);
      spillover(g, t);
      puddleAndSign(g, t);
      if (part) { crateAndFlapper(g, t); return; }
      const [kx, ky] = plunger(g, t);
      if (s.phase === "plunge") rhythm(g, t, kx, ky);
      for (const p of s.drops) D.disc(g, p.x, p.y, 4, "rgba(79,195,247,0.85)");
      levelColumn(g);
      trapGauge(g, t);
    }

    // ================================================================ the shower (a pipe puzzle)
    const T = 84;                                         // a tile, px
    const SIZES = [[5, 4], [6, 4], [6, 5], [7, 5]];       // columns and rows by shower (the 1st, 2nd ...)
    const DIRS = [{ b: 1, dc: 0, dr: -1 }, { b: 2, dc: 1, dr: 0 }, { b: 4, dc: 0, dr: 1 }, { b: 8, dc: -1, dr: 0 }];  // N E S W
    const OPP = { 1: 4, 2: 8, 4: 1, 8: 2 };
    const RISER = 100, VX = 196, VR = 36;                 // the main's riser and its valve
    const FIX = { x: 990, y: 100, w: 250, h: 540 };       // the shower stall
    const HEAD = { x: 1140, y: 190 };                     // the shower head's face
    const FLOW = 6;                                       // water runs this many tiles a second

    const turn = (m, k) => { for (let i = 0; i < ((k % 4) + 4) % 4; i++) m = ((m << 1) | (m >> 3)) & 15; return m; };
    const open = (tile) => turn(tile.base, tile.k);
    const at = (c, r) => (c >= 0 && r >= 0 && c < s.cols && r < s.rows ? s.tiles[r * s.cols + c] : null);
    const centre = (c, r) => [s.gx + c * T + T / 2, s.gy + r * T + T / 2];

    /** A random self-avoiding run from the main's tile to the shower's, leaning right. */
    function route(r) {
      const seen = new Set(), path = [];
      let budget = 4000;
      const go = (c, rr) => {
        if (--budget < 0) return false;
        seen.add(rr * s.cols + c); path.push([c, rr]);
        if (c === s.cols - 1 && rr === s.r1) return true;
        const opts = DIRS.map((d) => ({ d, w: r() + (d.dc > 0 ? 0.35 : 0) })).sort((a, b) => b.w - a.w);
        for (const { d } of opts) {
          const nc = c + d.dc, nr = rr + d.dr;
          if (nc < 0 || nr < 0 || nc >= s.cols || nr >= s.rows || seen.has(nr * s.cols + nc)) continue;
          if (go(nc, nr)) return true;
        }
        path.pop(); return false;
      };
      if (go(0, s.r0)) return path;
      const p = [];   // the budget ran out: straight along, then down or up
      for (let c = 0; c < s.cols; c++) p.push([c, s.r0]);
      for (let rr = s.r0; rr !== s.r1; rr += Math.sign(s.r1 - s.r0)) p.push([s.cols - 1, rr + Math.sign(s.r1 - s.r0)]);
      return p;
    }

    /** The water's reach from the main: the tiles it fills, their distance, its open ends, and whether it is whole. */
    function network() {
      const first = at(0, s.r0), yMain = centre(0, s.r0)[1];
      if (!(open(first) & 8)) return { cells: new Map(), leaks: [[s.gx, yMain, 2]], whole: false, far: 0 };
      const cells = new Map([[s.r0 * s.cols, 0]]), queue = [[0, s.r0]], leaks = [];
      let reach = false, far = 0;
      while (queue.length) {
        const [c, r] = queue.shift(), m = open(at(c, r)), d0 = cells.get(r * s.cols + c);
        far = Math.max(far, d0);
        for (const d of DIRS) {
          if (!(m & d.b)) continue;
          if (c === 0 && r === s.r0 && d.b === 8) continue;
          if (c === s.cols - 1 && r === s.r1 && d.b === 2) { reach = true; continue; }
          const n = at(c + d.dc, r + d.dr);
          const [x, y] = centre(c, r);
          if (!n || !(open(n) & OPP[d.b])) { leaks.push([x + (d.dc * T) / 2, y + (d.dr * T) / 2, d.b]); continue; }
          const key = (r + d.dr) * s.cols + c + d.dc;
          if (!cells.has(key)) { cells.set(key, d0 + 1); queue.push([c + d.dc, r + d.dr]); }
        }
      }
      return { cells, leaks, whole: reach && !leaks.length, far };
    }

    function showerStep(index, r) {
      const [cols, rows] = SIZES[Math.min(Math.floor(index / 2), SIZES.length - 1)];
      s = { kind: "shower", index, r, cols, rows, phase: "play", tiles: [], cur: { c: 0, r: 0 }, keys: false, flow: 0, runT: 0, spray: 0, net: null, va: 0, running: false };
      s.gx = 600 - (cols * T) / 2; s.gy = 410 - (rows * T) / 2;
      s.r0 = Math.floor(r() * rows); s.r1 = Math.floor(r() * rows);
      s.yMain = centre(0, s.r0)[1];
      // Decoys first, then the run laid over them; then every tile turned at random.
      for (let i = 0; i < cols * rows; i++) { const q = r(); s.tiles.push({ base: q < 0.45 ? 3 : q < 0.8 ? 5 : 7, k: 0, ang: 0, run: false }); }
      const path = route(r);
      path.forEach(([c, rr], i) => {
        const dirTo = (a, b) => DIRS.find((d) => a[0] + d.dc === b[0] && a[1] + d.dr === b[1]).b;
        const prev = i ? dirTo(path[i], path[i - 1]) : 8;
        const next = i < path.length - 1 ? dirTo(path[i], path[i + 1]) : 2;
        const m = prev | next, tile = at(c, rr);
        tile.base = m === 5 || m === 10 ? 5 : 3; tile.run = true;
        tile.sol = [0, 1, 2, 3].find((k) => turn(tile.base, k) === m);
      });
      for (const tile of s.tiles) { tile.k = Math.floor(r() * 4); tile.ang = (tile.k * Math.PI) / 2; }
      if (network().whole) { const tile = at(0, s.r0); tile.k++; tile.ang = (tile.k * Math.PI) / 2; }
    }

    function openValve() {
      const net = network();
      s.net = net;
      if (net.whole) { s.phase = "flow"; s.flow = 0; return; }
      s.spray = 1.4;
      api.fumble(SOAKED);
    }

    function showerUpdate(dt, input) {
      s.spray = Math.max(0, s.spray - dt);
      for (const tile of s.tiles) tile.ang += ((tile.k * Math.PI) / 2 - tile.ang) * Math.min(1, dt * 16);
      if (s.phase === "flow") {
        s.flow += FLOW * dt; s.va = Math.min(Math.PI * 1.5, s.va + dt * 6);
        if (s.flow > s.net.far + 1.5) { if (!s.running) { s.running = true; api.stepDone(); } s.runT += dt; }
        return;
      }
      // Keys: a cursor, Space turns, Enter opens the valve.
      for (const [code, dc, dr] of [["ArrowLeft", -1, 0], ["KeyA", -1, 0], ["ArrowRight", 1, 0], ["KeyD", 1, 0], ["ArrowUp", 0, -1], ["KeyW", 0, -1], ["ArrowDown", 0, 1], ["KeyS", 0, 1]]) {
        if (input.hit.has(code)) { s.keys = true; s.cur.c = Math.max(0, Math.min(s.cols - 1, s.cur.c + dc)); s.cur.r = Math.max(0, Math.min(s.rows - 1, s.cur.r + dr)); }
      }
      if (input.hit.has("Space")) { s.keys = true; at(s.cur.c, s.cur.r).k++; }
      if (input.hit.has("Enter") && s.spray === 0) { openValve(); return; }
      if (input.pressed) {
        if (Math.hypot(input.x - VX, input.y - s.yMain) < VR + 8) { if (s.spray === 0) openValve(); return; }
        const c = Math.floor((input.x - s.gx) / T), rr = Math.floor((input.y - s.gy) / T), tile = at(c, rr);
        if (tile) { tile.k++; s.cur = { c, r: rr }; }
      }
    }

    // ---------------------------------------------------------------- shower drawing
    function pipe(g, pts, w = 22, wet = false) {
      g.beginPath(); pts.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y)));
      g.lineJoin = "round"; g.lineCap = "round";
      g.strokeStyle = "#232c3b"; g.lineWidth = w + 6; g.stroke();
      g.strokeStyle = "#3a4658"; g.lineWidth = w; g.stroke();
      g.strokeStyle = wet ? C.accent : "#141b27"; g.lineWidth = w * 0.4; g.stroke();
      g.lineCap = "butt";
    }
    function valveGlyph(g, x, y, a, on) {
      D.disc(g, x, y, VR, "#141b27");
      D.ring(g, x, y, VR - 5, on ? C.ok : "#b0473f", 10);
      g.strokeStyle = "#9aa6b6"; g.lineWidth = 7;
      for (let k = 0; k < 3; k++) { const b = a + (k * Math.PI * 2) / 3; g.beginPath(); g.moveTo(x, y); g.lineTo(x + Math.cos(b) * (VR - 10), y + Math.sin(b) * (VR - 10)); g.stroke(); }
      D.disc(g, x, y, 10, "#2a3446"); D.ring(g, x, y, 10, "#9aa6b6", 2);
    }
    function spray(g, x, y, b, t) {
      const d = DIRS.find((q) => q.b === b) || DIRS[1];
      for (let j = 0; j < 16; j++) {
        const ph = (t * 2.2 + j / 16) % 1, side = (j % 8) - 3.5;
        const px = x + d.dc * ph * 80 + (d.dc ? 0 : side * ph * 9);
        const py = y + d.dr * ph * 80 + (d.dc ? side * ph * 9 : 0) + ph * ph * 60;
        D.disc(g, px, py, 5 - 3 * ph, "rgba(79,195,247,0.9)");
      }
    }
    function stall(g, t, running) {
      // The stall: tiled walls, a curtain bunched on its rail, the head, the tray, and a duck on the tray.
      D.panel(g, FIX.x, FIX.y, FIX.w, FIX.h, 14, "#101a24");
      g.fillStyle = "#16222f";
      for (let y = FIX.y + 34; y < FIX.y + FIX.h - 30; y += 34) g.fillRect(FIX.x + 6, y, FIX.w - 12, 2);
      for (let x = FIX.x + 40; x < FIX.x + FIX.w; x += 40) g.fillRect(x, FIX.y + 6, 2, FIX.h - 40);
      const y1 = centre(s.cols - 1, s.r1)[1];
      pipe(g, [[FIX.x - 20, y1], [FIX.x + 34, y1], [FIX.x + 34, 150], [HEAD.x, 150], [HEAD.x, 166]], 16, running);
      g.fillStyle = "#9aa6b6"; g.fillRect(FIX.x + 10, 116, FIX.w - 20, 6);                                  // the rail
      g.fillStyle = C.lilac;
      for (let k = 0; k < 4; k++) { D.round(g, FIX.x + FIX.w - 52 + k * 10, 122, 12, 440, 6); g.fill(); }
      g.fillStyle = "rgba(0,0,0,0.25)"; for (let k = 0; k < 4; k++) g.fillRect(FIX.x + FIX.w - 44 + k * 10, 126, 3, 430);
      // The head: a rose on its arm, angled down.
      g.fillStyle = C.steel;
      g.beginPath(); g.moveTo(HEAD.x - 34, 166); g.lineTo(HEAD.x + 34, 166); g.lineTo(HEAD.x + 46, HEAD.y); g.lineTo(HEAD.x - 46, HEAD.y); g.closePath(); g.fill();
      g.fillStyle = "#4a5566"; for (let k = 0; k < 7; k++) g.fillRect(HEAD.x - 39 + k * 12, HEAD.y, 5, 4);
      // The tray and its drain.
      g.fillStyle = "#3a4658"; g.fillRect(FIX.x + 14, 604, FIX.w - 28, 20);
      g.beginPath(); g.ellipse(HEAD.x - 20, 606, 18, 4, 0, 0, Math.PI * 2); g.fillStyle = "#151b23"; g.fill();
      if (running) {
        const k = Math.min(1, s.runT / 0.5);
        g.fillStyle = `rgba(79,195,247,${0.45 * k})`; g.fillRect(FIX.x + 20, 598, FIX.w - 40, 8);
        // The fall: a cone of streaks from the rose, splashes on the tray, a little steam.
        for (let j = 0; j < 60; j++) {
          const ph = (t * 1.7 + ((j * 0.618) % 1)) % 1, lane = ((j * 7) % 23) / 22 - 0.5, y = HEAD.y + 6 + ph * 400 * k;
          const x = HEAD.x - 6 + lane * (84 + (y - HEAD.y) * 0.2);
          g.fillStyle = `rgba(120,205,250,${0.5 + 0.4 * (1 - ph)})`; g.fillRect(x - 1.5, y, 3, 18);
        }
        for (let j = 0; j < 12; j++) {
          const ph = (t * 2.4 + j / 12) % 1, sx = HEAD.x - 10 + (((j * 0.7) % 1) - 0.5) * 150, side = j % 2 ? 1 : -1;
          D.disc(g, sx + side * ph * 30, 598 - Math.sin(ph * Math.PI) * 26, 3, "rgba(160,220,250,0.8)");
        }
        for (let j = 0; j < 5; j++) {
          const ph = (t * 0.25 + j / 5) % 1;
          D.disc(g, FIX.x + 50 + j * 36 + Math.sin(t + j) * 10, 560 - ph * 380, 18 + ph * 26, `rgba(220,230,240,${0.04 * (1 - ph)})`);
        }
      }
      const bob = running ? Math.sin(t * 5) * 3 : 0;
      duck(g, FIX.x + 56, 588 + bob, 0.7, running ? Math.sin(t * 4) * 0.15 : 0);
    }

    function showerDraw(g, t) {
      g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
      const flowing = s.phase === "flow", wetAll = s.spray > 0;
      const net = flowing || wetAll ? s.net : network();
      D.panel(g, s.gx - 14, s.gy - 14, s.cols * T + 28, s.rows * T + 28, 16, "#0b1018");
      // The main: down from the deckhead, along to the grid, through its valve.
      pipe(g, [[RISER, api.BAR_H], [RISER, s.yMain], [s.gx, s.yMain]], 22, flowing || wetAll);
      valveGlyph(g, VX, s.yMain, s.va, flowing);
      const yEnd = centre(s.cols - 1, s.r1)[1];
      pipe(g, [[s.gx + s.cols * T, yEnd], [FIX.x - 20, yEnd]], 22, s.running);
      stall(g, t, s.running);
      for (let rr = 0; rr < s.rows; rr++) for (let c = 0; c < s.cols; c++) {
        const tile = at(c, rr), [x, y] = centre(c, rr), key = rr * s.cols + c;
        const inNet = net.cells.has(key), wet = (flowing && inNet && net.cells.get(key) < s.flow) || (wetAll && inNet);
        D.round(g, x - T / 2 + 3, y - T / 2 + 3, T - 6, T - 6, 10); g.fillStyle = inNet && s.phase === "play" ? "#121c2a" : "#0d131c"; g.fill();
        g.save(); g.translate(x, y); g.rotate(tile.ang);
        g.lineCap = "round";
        for (const [w, col] of [[28, "#232c3b"], [22, inNet ? "#5a6a82" : "#3a4658"], [9, wet ? C.accent : "#141b27"]]) {
          g.strokeStyle = col; g.lineWidth = w;
          for (const d of DIRS) if (tile.base & d.b) { g.beginPath(); g.moveTo(0, 0); g.lineTo((d.dc * T) / 2, (d.dr * T) / 2); g.stroke(); }
        }
        g.lineCap = "butt";
        D.disc(g, 0, 0, 14, inNet ? "#6b7c95" : "#4a5566"); D.disc(g, 0, 0, 6, wet ? C.accent : "#141b27");
        g.restore();
      }
      if (s.keys && s.phase === "play") { const [x, y] = centre(s.cur.c, s.cur.r); D.round(g, x - T / 2 + 2, y - T / 2 + 2, T - 4, T - 4, 10); g.lineWidth = 4; g.strokeStyle = C.amber; g.stroke(); }
      if (wetAll) for (const [x, y, b] of s.net.leaks) spray(g, x, y, b, t);
    }

    return {
      get state() { return s; },   // for tools: the headless checks read the round
      /** Even steps a toilet, odd steps a shower; a part step is the toilet's flapper. */
      step(index, isPart) {
        const r = api.rand();
        if (isPart || index % 2 === 0) toiletStep(index, isPart, r); else showerStep(index, r);
      },
      update(dt, input) { if (s.kind === "shower") showerUpdate(dt, input); else toiletUpdate(dt, input); },
      draw(g, t) { if (s.kind === "shower") showerDraw(g, t); else toiletDraw(g, t); },
    };
  },
});
