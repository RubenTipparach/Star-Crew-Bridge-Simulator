/*
 * repairs/heads.js: the heads and showers' repair (openspec/changes/repair-minigames, design 2).
 *
 * A pipe puzzle in the deckhead. Water comes down the main on the left; the fixture, a shower or a toilet, waits on
 * the right. Click a tile to turn it a quarter (or move the cursor with the arrows and turn it with Space) until one
 * run joins the main to the fixture with no open end anywhere along it, then open the valve on the main (click it, or
 * Enter): the water runs through to the fixture. Tiles that join the main show brighter. A step is one fixture's run;
 * later steps have bigger grids. Opening the valve on an unfinished run: a fumble, a loose joint, you are soaked. A
 * disabled run's first step fits the new valve body: drag it from the crate onto the main.
 */
RepairKit.register({
  id: "heads",
  title: "Heads and showers",
  place: "Crew quarters",
  group: "Life",
  hazard: "Loose joint: you are soaked",
  down: "Comfort and morale (design 4)",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const SOAKED = "Loose joint: you are soaked";
    const T = 84;                                         // a tile, px
    const SIZES = [[5, 4], [6, 4], [6, 5], [7, 5]];       // columns and rows by step
    const DIRS = [{ b: 1, dc: 0, dr: -1 }, { b: 2, dc: 1, dr: 0 }, { b: 4, dc: 0, dr: 1 }, { b: 8, dc: -1, dr: 0 }];  // N E S W
    const OPP = { 1: 4, 2: 8, 4: 1, 8: 2 };
    const RISER = 100, VX = 196, VR = 36;                 // the main's riser and its valve
    const FIX = { x: 990, y: 100, w: 250, h: 540 };       // the fixture's wall
    const CRATE = { x: 30, y: 612, w: 140, h: 96 };       // the spare valve's crate (part step)
    const FLOW = 6;                                       // water runs this many tiles a second
    let s;

    const turn = (m, k) => { for (let i = 0; i < ((k % 4) + 4) % 4; i++) m = ((m << 1) | (m >> 3)) & 15; return m; };
    const open = (tile) => turn(tile.base, tile.k);
    const at = (c, r) => (c >= 0 && r >= 0 && c < s.cols && r < s.rows ? s.tiles[r * s.cols + c] : null);
    const centre = (c, r) => [s.gx + c * T + T / 2, s.gy + r * T + T / 2];

    /** A random self-avoiding run from the main's tile to the fixture's, leaning right. */
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

    function partStep(dt, input) {
      const p = s.valve;
      if (input.stick.x || input.stick.y) { p.x += input.stick.x * 480 * dt; p.y += input.stick.y * 480 * dt; p.pad = true; }
      if (input.pressed && Math.hypot(input.x - p.x, input.y - p.y) < 50) p.held = true;
      if (p.held && input.down) { p.x = input.x; p.y = input.y; }
      if ((p.held && input.released) || (p.pad && input.actionPressed)) {
        p.held = false; p.pad = false;
        if (Math.hypot(p.x - VX, p.y - s.yMain) < 40) { p.set = true; s.phase = "fitted"; api.stepDone(); }
      }
    }

    function openValve() {
      const net = network();
      s.net = net;
      if (net.whole) { s.phase = "flow"; s.flow = 0; return; }
      s.spray = 1.4;
      api.fumble(SOAKED);
    }

    // ---------------------------------------------------------------- drawing
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
    function fixture(g, t, running) {
      D.panel(g, FIX.x, FIX.y, FIX.w, FIX.h, 14, "#0f151f");
      for (let y = FIX.y + 30; y < FIX.y + FIX.h; y += 30) { g.fillStyle = "#141c28"; g.fillRect(FIX.x + 6, y, FIX.w - 12, 2); }
      const y1 = centre(s.cols - 1, s.r1)[1];
      if (s.shower) {
        pipe(g, [[FIX.x - 20, y1], [FIX.x + 40, y1], [FIX.x + 40, 150], [1150, 150], [1150, 168]], 16, running);
        g.fillStyle = C.steel; g.beginPath(); g.moveTo(1120, 168); g.lineTo(1180, 168); g.lineTo(1196, 196); g.lineTo(1104, 196); g.closePath(); g.fill();
        g.fillStyle = "#4a5566"; for (let k = 0; k < 6; k++) g.fillRect(1110 + k * 15, 196, 6, 4);
        if (running) for (let j = 0; j < 30; j++) {
          const ph = (t * 1.6 + ((j * 0.618) % 1)) % 1, x = 1110 + (j % 10) * 8 + ((j % 10) - 4.5) * ph * 7;
          g.fillStyle = "rgba(79,195,247,0.8)"; g.fillRect(x, 200 + ph * 400, 3, 14);
        }
        g.fillStyle = "#3a4658"; g.fillRect(1050, 606, 180, 18);
        g.fillStyle = running ? "rgba(79,195,247,0.5)" : "#1a2230"; g.fillRect(1058, 600, 164, 6);
      } else {
        pipe(g, [[FIX.x - 20, y1], [FIX.x + 40, y1], [FIX.x + 40, 300], [1070, 300]], 16, running);
        D.round(g, 1070, 250, 140, 110, 10); g.fillStyle = "#c9d2dc"; g.fill(); g.lineWidth = 3; g.strokeStyle = "#7d8796"; g.stroke();
        g.fillStyle = "#7d8796"; g.fillRect(1180, 262, 18, 8);
        if (running) { const f = Math.min(1, s.runT / 1.5); g.fillStyle = "rgba(79,195,247,0.6)"; g.fillRect(1078, 352 - 80 * f, 124, 80 * f); }
        g.fillStyle = "#9aa6b6"; g.fillRect(1120, 360, 30, 70);
        g.beginPath(); g.moveTo(1060, 440); g.lineTo(1230, 440); g.quadraticCurveTo(1230, 540, 1160, 556); g.lineTo(1170, 630); g.lineTo(1110, 630); g.lineTo(1116, 556);
        g.quadraticCurveTo(1060, 540, 1060, 440); g.closePath(); g.fillStyle = "#c9d2dc"; g.fill(); g.lineWidth = 3; g.strokeStyle = "#7d8796"; g.stroke();
        g.fillStyle = "#7d8796"; g.fillRect(1052, 430, 186, 12);
        if (running) { g.beginPath(); g.ellipse(1145, 470, 58 + 6 * Math.sin(t * 6), 14, 0, 0, Math.PI * 2); g.fillStyle = "rgba(79,195,247,0.7)"; g.fill(); }
      }
    }

    return {
      get state() { return s; },   // for tools: the headless checks read the round
      step(index, isPart) {
        const r = api.rand();
        const [cols, rows] = SIZES[Math.min(index, SIZES.length - 1)];
        s = { index, cols, rows, phase: isPart ? "part" : "play", tiles: [], cur: { c: 0, r: 0 }, keys: false, flow: 0, runT: 0, spray: 0, net: null, va: 0 };
        s.gx = 640 - (cols * T) / 2; s.gy = 410 - (rows * T) / 2;
        s.r0 = Math.floor(r() * rows); s.r1 = Math.floor(r() * rows);
        s.yMain = centre(0, s.r0)[1];
        s.shower = r() < 0.5;
        s.valve = { x: CRATE.x + CRATE.w / 2, y: CRATE.y + CRATE.h / 2, held: false, pad: false, set: !isPart };
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
      },
      update(dt, input) {
        s.spray = Math.max(0, s.spray - dt);
        for (const tile of s.tiles) tile.ang += ((tile.k * Math.PI) / 2 - tile.ang) * Math.min(1, dt * 16);
        if (s.phase === "part") { partStep(dt, input); return; }
        if (s.phase === "flow") {
          s.flow += FLOW * dt; s.va = Math.min(Math.PI * 1.5, s.va + dt * 6);
          if (s.flow > s.net.far + 1.5) { if (!s.running) { s.running = true; api.stepDone(); } s.runT += dt; }
          return;
        }
        if (s.phase !== "play") return;
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
      },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        // The deckhead behind the pipes: a dark frame round the grid.
        const flowing = s.phase === "flow", wetAll = s.spray > 0;
        const net = flowing || wetAll ? s.net : s.phase === "play" ? network() : { cells: new Map() };
        D.panel(g, s.gx - 14, s.gy - 14, s.cols * T + 28, s.rows * T + 28, 16, "#0b1018");
        // The main: down from the deckhead, along to the grid, through its valve.
        pipe(g, [[RISER, api.BAR_H], [RISER, s.yMain], [s.gx, s.yMain]], 22, flowing || wetAll);
        if (s.valve.set) valveGlyph(g, VX, s.yMain, s.va, flowing);
        else { D.disc(g, VX, s.yMain, VR - 6, "#120d08"); D.ring(g, VX, s.yMain, VR - 6, C.amber, 4); }
        // The run out to the fixture, and the fixture.
        const yEnd = centre(s.cols - 1, s.r1)[1];
        pipe(g, [[s.gx + s.cols * T, yEnd], [FIX.x - 20, yEnd]], 22, s.running);
        fixture(g, t, s.running);
        // The tiles.
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
        if (s.phase === "part") {
          D.panel(g, CRATE.x, CRATE.y, CRATE.w, CRATE.h, 14, "#141b27");
          if (!s.valve.set) { valveGlyph(g, s.valve.x, s.valve.y, 0, false); D.ring(g, s.valve.x, s.valve.y, VR + 4, "#f0c08a", 3); }
        }
      },
    };
  },
});
