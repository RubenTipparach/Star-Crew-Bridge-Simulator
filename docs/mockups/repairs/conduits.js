/*
 * repairs/conduits.js: an electrical conduit's repair (openspec/changes/repair-minigames, design 2).
 *
 * Two kinds of step. A splice: the conduit is open where it burnt, its wires charred along the middle. Strip the burnt
 * length (drag along it, or hold Space or the right arrow), then join the cut ends colour to colour, each colour with
 * its own tip shape (drag a left end onto a right end, or up and down to choose and Space to pick and drop) before the
 * clamp's jaws close; a clamp that shuts first pulls the joins apart. A reroute: the conduit map, with the run's
 * junction box burnt out; draw a new run from the switchboard to the load around it and the bulkheads (drag through
 * the cells, or the arrows). Steps are splices with a reroute third and last; each splice has more wires and a quicker
 * clamp, each map more bulkheads. A crossed pair is a fumble: a spark, 5 HP, and the breaker trips. A disabled
 * conduit's first step fits a new length: drag it from the crate into the gap.
 */
RepairKit.register({
  id: "conduits",
  title: "Electrical conduits",
  place: "Anywhere a conduit runs",
  group: "Engineering",
  hazard: "Crossed pair: spark, 5 HP, breaker tripped",
  // The how-to card (repair-minigames 6g), drawn by the kit: pictures and a few words, on demand.
  guide: {
    steps: [
      { icon: "sweep", text: "Drag along to strip burns" },
      { icon: "plug", text: "Join colours before the clamp" },
      { icon: "drag", text: "Reroute: draw round the burn" },
    ],
    mistake: "A crossed pair: a spark, 5 HP, the breaker trips",
    now: (q) => (q.mode === "part" ? -1 : q.mode === "reroute" ? 2 : q.stripped < 1 ? 0 : 1),
  },
  down: "Loads beyond it are cut (power-grid)",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    // The splice's picture.
    const PIPE = { y: 330, h: 120, l: 345, r: 935 };     // the conduit; open between l and r
    const MID = PIPE.y + PIPE.h / 2;
    const BURN = { x0: 470, x1: 810 };                   // the burnt length
    const BINS = 34;
    const WIRES = [
      { c: C.danger, shape: "circle" }, { c: C.accent, shape: "square" }, { c: C.ok, shape: "triangle" },
      { c: C.warn, shape: "diamond" }, { c: C.lilac, shape: "bar" }, { c: "#e8eef6", shape: "ring" },
    ];
    // The reroute's map.
    const COLS = 11, ROWS = 6, CELL = 80, GX = 200, GY = 150;
    let mode, part, len, n, rightOrder, stripped, head, joins, held, selL, selR, clamp, clampT, spark, done;
    let grid, src, load, burnt, oldRun, run, deny, time;

    const rowY = (i) => MID + (i - (n - 1) / 2) * 26;
    const cellOf = (x, y) => { const c = Math.floor((x - GX) / CELL), r = Math.floor((y - GY) / CELL); return c >= 0 && c < COLS && r >= 0 && r < ROWS ? [c, r] : null; };
    const key = (c, r) => r * COLS + c;
    const cx = (c) => GX + c * CELL + CELL / 2, cy = (r) => GY + r * CELL + CELL / 2;

    function tip(g, w, x, y, s = 11) {
      g.fillStyle = w.c; g.strokeStyle = w.c; g.lineWidth = 4;
      if (w.shape === "circle") D.disc(g, x, y, s, w.c);
      else if (w.shape === "square") g.fillRect(x - s, y - s, s * 2, s * 2);
      else if (w.shape === "triangle") { g.beginPath(); g.moveTo(x, y - s - 2); g.lineTo(x + s + 2, y + s); g.lineTo(x - s - 2, y + s); g.closePath(); g.fill(); }
      else if (w.shape === "diamond") { g.beginPath(); g.moveTo(x, y - s - 3); g.lineTo(x + s + 3, y); g.lineTo(x, y + s + 3); g.lineTo(x - s - 3, y); g.closePath(); g.fill(); }
      else if (w.shape === "bar") g.fillRect(x - 4, y - s - 2, 8, s * 2 + 4);
      else D.ring(g, x, y, s - 2, w.c, 5);
    }

    function buildMap(r, index) {
      for (let tries = 0; tries < 200; tries++) {
        const r0 = Math.floor(r() * ROWS), r1 = Math.floor(r() * ROWS), bc = 4 + Math.floor(r() * 3);
        src = [0, r0]; load = [COLS - 1, r1]; burnt = [bc, r0];
        oldRun = [];
        for (let c = 0; c <= 7; c++) oldRun.push([c, r0]);
        for (let rr = r0; rr !== r1; rr += Math.sign(r1 - r0)) oldRun.push([7, rr + Math.sign(r1 - r0)]);
        for (let c = 8; c < COLS; c++) oldRun.push([c, r1]);
        grid = new Array(COLS * ROWS).fill(0);
        grid[key(...burnt)] = 2;
        // Bulkheads: never on the old run (it is still there, but for its box), always beside the burnt box.
        const onOld = new Set(oldRun.map(([c, rr]) => key(c, rr)));
        for (const dr of [-1, 1]) if (r0 + dr >= 0 && r0 + dr < ROWS && !onOld.has(key(bc, r0 + dr))) grid[key(bc, r0 + dr)] = 1;
        const want = Math.min(24, 10 + 3 * index);
        let placed = 0, guard = 0;
        while (placed < want && guard++ < 500) {
          const c = 1 + Math.floor(r() * (COLS - 2)), rr = Math.floor(r() * ROWS);
          if (grid[key(c, rr)] === 0 && !onOld.has(key(c, rr))) { grid[key(c, rr)] = 1; placed++; }
        }
        // A run must exist from the switchboard to the load.
        const seen = new Set([key(...src)]), q = [src];
        while (q.length) {
          const [c, rr] = q.shift();
          for (const [dc, dr] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
            const nc = c + dc, nr = rr + dr;
            if (nc < 0 || nc >= COLS || nr < 0 || nr >= ROWS || grid[key(nc, nr)] || seen.has(key(nc, nr))) continue;
            seen.add(key(nc, nr)); q.push([nc, nr]);
          }
        }
        if (seen.has(key(...load))) return;
      }
      grid.fill(0); grid[key(...burnt)] = 2;
    }

    function tryMove(c, r) {
      const last = run[run.length - 1];
      const prev = run[run.length - 2];
      if (prev && prev[0] === c && prev[1] === r) { run.pop(); return; }
      const at = run.findIndex(([a, b]) => a === c && b === r);
      if (at >= 0) { run.length = at + 1; return; }
      if (Math.abs(c - last[0]) + Math.abs(r - last[1]) !== 1) return;
      if (grid[key(c, r)]) { deny = { c, r, t: 0.5 }; return; }
      run.push([c, r]);
      if (c === load[0] && r === load[1]) { done = true; api.stepDone(); }
    }

    return {
      step(index, isPart) {
        const r = api.rand();
        part = isPart; time = 0; spark = null; done = false; deny = null;
        len = { x: 1130, y: 620, held: false, set: false };
        mode = isPart ? "part" : index % 3 === 2 || index === api.steps - 1 ? "reroute" : "splice";
        // The splice.
        n = Math.min(6, 3 + index);
        rightOrder = Array.from({ length: n }, (_, i) => i);
        do { for (let i = n - 1; i > 0; i--) { const j = Math.floor(r() * (i + 1)); [rightOrder[i], rightOrder[j]] = [rightOrder[j], rightOrder[i]]; } }
        while (rightOrder.every((w, i) => w === i));
        stripped = new Array(BINS).fill(false);
        head = BURN.x0;
        joins = new Array(n).fill(-1);
        held = -1; selL = 0; selR = 0;
        clamp = Math.max(8, 13 - index); clampT = 0;
        // The reroute.
        if (mode === "reroute") { buildMap(r, index); run = [src]; }
      },
      /** For tools (shots and tests): the round's state, read only. */
      peek() {
        return { mode, partSet: len.set, stripped: stripped.filter(Boolean).length / BINS, n, rightOrder: [...rightOrder], joins: [...joins], done,
          rowY: rightOrder.map((_, i) => rowY(i)), rx: BURN.x1 - 4, lx: BURN.x0 + 4,
          grid: grid && [...grid], src, load, run: run && run.map((x) => [...x]), cell: [GX, GY, CELL, COLS, ROWS] };
      },
      update(dt, input) {
        time += dt;
        if (spark) { spark.t -= dt; if (spark.t <= 0) spark = null; }
        if (deny) { deny.t -= dt; if (deny.t <= 0) deny = null; }
        if (mode === "part") {
          // The part step: carry the new length into the gap.
          if (len.set) return;
          if (input.pressed && Math.abs(input.x - len.x) < 90 && Math.abs(input.y - len.y) < 40) len.held = true;
          if (len.held && input.down) { len.x = input.x; len.y = input.y; }
          if (len.held && input.released) {
            len.held = false;
            if (len.x > PIPE.l && len.x < PIPE.r && Math.abs(len.y - MID) < 70) { len.set = true; api.stepDone(); }
          }
          return;
        }
        if (done) return;
        if (mode === "reroute") {
          if (input.down) { const cell = cellOf(input.x, input.y); if (cell) tryMove(...cell); }
          const last = run[run.length - 1];
          const mv = (input.hit.has("ArrowRight") || input.hit.has("KeyD")) ? [1, 0] : (input.hit.has("ArrowLeft") || input.hit.has("KeyA")) ? [-1, 0]
            : (input.hit.has("ArrowDown") || input.hit.has("KeyS")) ? [0, 1] : (input.hit.has("ArrowUp") || input.hit.has("KeyW")) ? [0, -1] : null;
          if (mv) { const c = last[0] + mv[0], r = last[1] + mv[1]; if (c >= 0 && c < COLS && r >= 0 && r < ROWS) tryMove(c, r); }
          return;
        }
        // The splice: strip first.
        if (stripped.some((s) => !s)) {
          const mark = (x) => { const b = Math.floor(((x - BURN.x0) / (BURN.x1 - BURN.x0)) * BINS); if (b >= 0 && b < BINS) stripped[b] = true; };
          if (input.down && Math.abs(input.y - MID) < 90) mark(input.x);
          if (input.action || input.stick.x > 0) { head = Math.min(BURN.x1, head + 260 * dt); for (let x = BURN.x0; x <= head; x += 5) mark(x); }
          return;
        }
        // Then join, before the clamp closes.
        clampT += dt;
        if (clampT >= clamp) {
          joins.fill(-1); held = -1; clampT = 0;
          api.say("Clamp shut on loose ends");
          return;
        }
        const ry = rightOrder.map((w, i) => rowY(i));
        const lx = BURN.x0 + 4, rx = BURN.x1 - 4;
        const up = input.hit.has("ArrowUp") || input.hit.has("KeyW"), dn = input.hit.has("ArrowDown") || input.hit.has("KeyS");
        if (held < 0) {
          if (up) selL = (selL + n - 1) % n;
          if (dn) selL = (selL + 1) % n;
          // The ends are 26 px apart: the nearest free one within a fingertip's reach (KIT.nearest).
          if (input.pressed) { const i = KIT.nearest(joins.map((j, k) => (j < 0 ? [lx, rowY(k)] : null)), input.x, input.y, KIT.TOUCH_R + 10); if (i >= 0) { held = i; selL = i; } }
          if (input.actionPressed && joins[selL] < 0) { held = selL; selR = rightOrder.findIndex((w, i) => !joins.includes(i)); }
          return;
        }
        if (up) selR = (selR + n - 1) % n;
        if (dn) selR = (selR + 1) % n;
        // Dragged onto a right end, or tapped there after tapping a left end; let go elsewhere, the wire drops back.
        let drop = -1;
        if (input.pressed || input.released) {
          drop = KIT.nearest(ry.map((y) => [rx, y]), input.x, input.y, KIT.TOUCH_R + 10);
          if (drop < 0 && input.released && Math.hypot(input.x - lx, input.y - rowY(held)) > 30) { held = -1; return; }
        }
        if (input.actionPressed) drop = selR;
        if (drop < 0) return;
        if (joins.includes(drop)) { held = -1; return; }
        if (rightOrder[drop] === held) {
          joins[held] = drop; held = -1;
          selL = joins.findIndex((j) => j < 0); if (selL < 0) selL = 0;
          if (joins.every((j) => j >= 0)) { done = true; api.stepDone(); }
        } else {
          spark = { x: rx, y: ry[drop], t: 0.6 }; held = -1;
          api.fumble("Crossed pair: spark, 5 HP, breaker tripped");
        }
      },
      draw(g, t, dt, input) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        if (mode === "reroute") return drawMap(g, t);
        drawSplice(g, t, input);
      },
    };

    function pipe(g, x0, x1) {
      const grad = g.createLinearGradient(0, PIPE.y, 0, PIPE.y + PIPE.h);
      grad.addColorStop(0, "#3a4656"); grad.addColorStop(0.45, "#566273"); grad.addColorStop(1, "#1b2433");
      D.round(g, x0, PIPE.y, x1 - x0, PIPE.h, 14); g.fillStyle = grad; g.fill();
      for (let x = x0 + 30; x < x1 - 10; x += 70) { g.fillStyle = "rgba(0,0,0,0.25)"; g.fillRect(x, PIPE.y + 4, 6, PIPE.h - 8); }
    }
    function flange(g, x) { D.round(g, x - 14, PIPE.y - 22, 28, PIPE.h + 44, 6); g.fillStyle = "#7d8796"; g.fill(); for (const y of [PIPE.y - 10, PIPE.y + PIPE.h + 10]) D.disc(g, x, y, 5, "#3a4656"); }

    function drawSplice(g, t, input) {
      // The conduit's two good ends and the open span between them, with the tray under it.
      g.fillStyle = "#0e151f"; g.fillRect(PIPE.l, PIPE.y + 10, PIPE.r - PIPE.l, PIPE.h - 20);
      g.strokeStyle = "#1f2a37"; g.lineWidth = 2;
      for (let x = PIPE.l + 10; x < PIPE.r; x += 24) { g.beginPath(); g.moveTo(x, PIPE.y + 10); g.lineTo(x, PIPE.y + PIPE.h - 10); g.stroke(); }
      pipe(g, 20, PIPE.l); pipe(g, PIPE.r, 1260);
      if (mode === "part") {
        if (len.set) { pipe(g, PIPE.l, PIPE.r); }
        else {
          g.setLineDash([10, 8]); D.round(g, PIPE.l + 8, PIPE.y, PIPE.r - PIPE.l - 16, PIPE.h, 14); g.lineWidth = 2; g.strokeStyle = C.amber; g.stroke(); g.setLineDash([]);
          D.panel(g, 1020, 570, 220, 110, 14, "#141b27");
          g.save(); g.translate(len.x, len.y); g.scale(0.32, 0.32); g.translate(-640, -MID); pipe(g, 640 - 260, 640 + 260); g.restore();
        }
        flange(g, PIPE.l); flange(g, PIPE.r);
        return;
      }
      const stripDone = !stripped.some((s) => !s);
      // The wires: whole from the flanges to the burnt length; charred inside it until stripped.
      for (let i = 0; i < n; i++) {
        const y = rowY(i), w = WIRES[i];
        g.strokeStyle = w.c; g.lineWidth = 9;
        g.beginPath(); g.moveTo(PIPE.l, y); g.lineTo(BURN.x0, y); g.stroke();
        const wr = WIRES[rightOrder[i]];
        g.strokeStyle = wr.c;
        g.beginPath(); g.moveTo(BURN.x1, y); g.lineTo(PIPE.r, y); g.stroke();
      }
      if (!stripDone) {
        const bw = (BURN.x1 - BURN.x0) / BINS;
        for (let b = 0; b < BINS; b++) {
          if (stripped[b]) continue;
          const x = BURN.x0 + b * bw;
          g.fillStyle = "rgba(20,16,14,0.9)"; g.fillRect(x, rowY(0) - 18, bw + 0.5, rowY(n - 1) - rowY(0) + 36);
          for (let i = 0; i < n; i++) { g.fillStyle = (b + i) % 3 ? "#2e2620" : "#3a2f27"; g.fillRect(x, rowY(i) - 5 + ((b * 5 + i) % 3) - 1, bw + 0.5, 10); }
          if ((b * 7) % 5 === 0) D.disc(g, x + bw / 2, rowY((b * 3) % n) + 4 * Math.sin(t * 3 + b), 3, `rgba(255,120,60,${0.5 + 0.4 * Math.sin(t * 5 + b)})`);
        }
        if (input && input.down && Math.abs(input.y - MID) < 90 && input.x > BURN.x0 - 40 && input.x < BURN.x1 + 40) stripper(g, input.x);
        else stripper(g, head);
      }
      // The clamp's jaws above and below the span, closing as the time runs out.
      const k = stripDone ? (done ? 1 : clampT / clamp) : 0;
      const top = 120 + k * (rowY(0) - 30 - 120 - 24), bot = 640 - k * (640 - (rowY(n - 1) + 30));
      for (const [y, dir] of [[top, 1], [bot, -1]]) {
        D.round(g, BURN.x0 - 40, y - (dir > 0 ? 24 : 0), BURN.x1 - BURN.x0 + 80, 24, 6);
        g.fillStyle = done ? "#2f6e4c" : k > 0.75 ? "#7a2f38" : "#566273"; g.fill();
        for (let x = BURN.x0 - 30; x < BURN.x1 + 30; x += 22) {
          g.beginPath(); g.moveTo(x, y); g.lineTo(x + 11, y + dir * 12); g.lineTo(x + 22, y); g.closePath(); g.fillStyle = "#3a4656"; g.fill();
        }
        g.fillStyle = "#2b3646"; g.fillRect((BURN.x0 + BURN.x1) / 2 - 10, dir > 0 ? api.BAR_H : y, 20, dir > 0 ? y - 24 - api.BAR_H : api.H - y);
      }
      if (!stripDone) return;
      // The ends: a tip shape and colour each; joins as curves with a sleeve.
      const lx = BURN.x0 + 4, rx = BURN.x1 - 4;
      for (let i = 0; i < n; i++) {
        const j = joins[i];
        if (j >= 0) {
          const y0 = rowY(i), y1 = rowY(j);
          g.beginPath(); g.moveTo(lx, y0); g.bezierCurveTo(lx + 120, y0, rx - 120, y1, rx, y1);
          g.strokeStyle = WIRES[i].c; g.lineWidth = 8; g.stroke();
          const mx = (lx + rx) / 2, my = (y0 + y1) / 2;
          D.round(g, mx - 22, my - 9, 44, 18, 6); g.fillStyle = "#b8c2d0"; g.fill();
        }
      }
      for (let i = 0; i < n; i++) {
        tip(g, WIRES[i], lx, rowY(i));
        tip(g, WIRES[rightOrder[i]], rx, rowY(i));
      }
      // The keyboard's choice, and the wire in the hand.
      if (held < 0 && joins[selL] < 0) { D.ring(g, lx, rowY(selL), 18, "rgba(242,160,70,0.6)", 2); }
      if (held >= 0) {
        const y0 = rowY(held);
        const usingPtr = input && input.down;
        const ex = usingPtr ? input.x : rx, ey = usingPtr ? input.y : rowY(selR);
        g.beginPath(); g.moveTo(lx, y0); g.bezierCurveTo(lx + 100, y0, ex - 100, ey, ex, ey);
        g.strokeStyle = WIRES[held].c; g.lineWidth = 8; g.globalAlpha = 0.8; g.stroke(); g.globalAlpha = 1;
        tip(g, WIRES[held], ex, ey, 13);
        if (!usingPtr) D.ring(g, rx, rowY(selR), 20, C.amber, 3);
      }
      if (spark) {
        g.strokeStyle = `rgba(255,240,180,${spark.t / 0.6})`; g.lineWidth = 3;
        for (let a = 0; a < 10; a++) {
          const b = a * 0.63 + t * 20;
          g.beginPath(); g.moveTo(spark.x, spark.y); g.lineTo(spark.x + Math.cos(b) * 50, spark.y + Math.sin(b) * 50); g.stroke();
        }
        D.disc(g, spark.x, spark.y, 16, `rgba(255,255,255,${spark.t / 0.6})`);
      }
      flange(g, PIPE.l); flange(g, PIPE.r);
    }

    function stripper(g, x) {
      // The wire stripper: two jaws closing on the bundle.
      const y0 = rowY(0) - 26, y1 = rowY(n - 1) + 26;
      D.round(g, x - 10, y0 - 60, 20, 60, 6); g.fillStyle = C.danger; g.fill();
      D.round(g, x - 10, y1, 20, 60, 6); g.fillStyle = C.danger; g.fill();
      g.fillStyle = "#b8c2d0"; g.fillRect(x - 6, y0 - 4, 12, 10); g.fillRect(x - 6, y1 - 6, 12, 10);
    }

    function drawMap(g, t) {
      // The conduit map: the deck's cells, bulkheads hatched, the old run dashed, the burnt box, the new run.
      D.panel(g, GX - 30, GY - 30, COLS * CELL + 60, ROWS * CELL + 60, 18, "#0b1119");
      for (let r = 0; r < ROWS; r++) for (let c = 0; c < COLS; c++) {
        const x = GX + c * CELL, y = GY + r * CELL, v = grid[key(c, r)];
        g.fillStyle = "#101824"; g.fillRect(x + 2, y + 2, CELL - 4, CELL - 4);
        if (v === 1) {
          g.save(); g.beginPath(); g.rect(x + 2, y + 2, CELL - 4, CELL - 4); g.clip();
          g.fillStyle = "#1f2733"; g.fillRect(x, y, CELL, CELL);
          g.strokeStyle = "#384354"; g.lineWidth = 3;
          for (let k = -CELL; k < CELL; k += 14) { g.beginPath(); g.moveTo(x + k, y + CELL); g.lineTo(x + k + CELL, y); g.stroke(); }
          g.restore();
        }
        if (deny && deny.c === c && deny.r === r) { g.fillStyle = `rgba(255,71,87,${deny.t})`; g.fillRect(x + 2, y + 2, CELL - 4, CELL - 4); }
      }
      g.setLineDash([8, 8]); g.beginPath();
      oldRun.forEach(([c, r], i) => (i ? g.lineTo(cx(c), cy(r)) : g.moveTo(cx(c), cy(r))));
      g.strokeStyle = "rgba(111,127,148,0.45)"; g.lineWidth = 6; g.stroke(); g.setLineDash([]);
      // The burnt junction box: red, crossed, sooted.
      const bx = cx(burnt[0]), by = cy(burnt[1]);
      D.disc(g, bx, by, 36, "rgba(30,24,22,0.9)");
      D.round(g, bx - 24, by - 24, 48, 48, 6); g.fillStyle = "#3a1a1e"; g.fill(); g.lineWidth = 3; g.strokeStyle = C.danger; g.stroke();
      g.beginPath(); g.moveTo(bx - 14, by - 14); g.lineTo(bx + 14, by + 14); g.moveTo(bx + 14, by - 14); g.lineTo(bx - 14, by + 14);
      g.strokeStyle = C.danger; g.lineWidth = 5; g.stroke();
      // The new run.
      g.beginPath(); run.forEach(([c, r], i) => (i ? g.lineTo(cx(c), cy(r)) : g.moveTo(cx(c), cy(r))));
      g.strokeStyle = done ? C.ok : C.amber; g.lineWidth = 12; g.lineJoin = "round"; g.lineCap = "round"; g.stroke(); g.lineJoin = "miter"; g.lineCap = "butt";
      // The switchboard and the load.
      const sx = cx(src[0]), sy = cy(src[1]);
      D.round(g, sx - 28, sy - 28, 56, 56, 8); g.fillStyle = "#2b3646"; g.fill(); g.lineWidth = 3; g.strokeStyle = C.amber; g.stroke();
      g.beginPath(); g.moveTo(sx + 4, sy - 18); g.lineTo(sx - 10, sy + 3); g.lineTo(sx, sy + 3); g.lineTo(sx - 4, sy + 18); g.lineTo(sx + 10, sy - 3); g.lineTo(sx, sy - 3); g.closePath();
      g.fillStyle = C.amber; g.fill();
      const lx = cx(load[0]), ly = cy(load[1]);
      D.disc(g, lx, ly, 28, "#2b3646"); D.ring(g, lx, ly, 28, done ? C.ok : "#566273", 3);
      D.disc(g, lx, ly, 13, done ? `rgba(255,214,120,${0.8 + 0.2 * Math.sin(t * 6)})` : "#3a4656");
      D.text(g, "SWBD", sx, GY + ROWS * CELL + 14, 16, C.dim, "center", 700);
      D.text(g, "LOAD", lx, GY + ROWS * CELL + 14, 16, C.dim, "center", 700);
      const [hc, hr] = run[run.length - 1];
      if (!done) { if (run.length > 1) D.disc(g, cx(hc), cy(hr), 14, C.amber); D.ring(g, cx(hc), cy(hr), 36, "rgba(242,160,70,0.5)", 3); }
    }
  },
});
