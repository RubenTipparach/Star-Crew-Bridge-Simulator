/*
 * repairs/medic.js: the medic's treatment (openspec/changes/repair-minigames, design 3; rates from medical-officer 2).
 *
 * One body on one screen. The vitals strip runs along the top (heart rate, blood oxygen, HP, the medkit's doses); the
 * body chart on the left marks each wound by kind and size (a cut, a burn, a fracture, a bleed, smoke in the lungs);
 * the close-up on the right shows the chosen wound; the tool tray is along the bottom (click a tool, or keys 1 to 5,
 * Q and E). Each wound takes its own tool and does nothing for another:
 * - sealer, a cut: trace along it from the sealed end, inside its line;
 * - gel, a burn: paint it until it is covered;
 * - knitter, a fracture: drag the loose end onto the other and hold it there, against the tremor, while it knits;
 * - clamp then sealer, a bleed: clamp between the spurts, then seal the tear;
 * - inhaler, smoke: press as the chest rises, a puff a breath.
 * A wound is one step; HP comes back at the medic's field rate (4.0 HP/s to 75 HP), the bar waiting on the rate as in
 * every repair, and a dose goes per 25 HP given. A slip (the sealer off the line, gel off the burn, the bone jerked
 * apart, a clamp into a spurt, a puff on the out-breath): the patient flinches, 2 HP, the wound reopens a little. The
 * arrows or W A S D move a cursor and Space presses, for a pad.
 */
RepairKit.register({
  id: "medic",
  title: "Medic: treatment",
  place: "Medbay or in the field",
  group: "Life",
  doneWord: "Treated",
  hazard: "The patient flinches: 2 HP",
  down: "Beds and field healing still work without a medic (medical-officer)",
  job: { unit: "HP", start: 30, target: 75, rateBy: { officer: 4.0, rating: 4.0 }, steps: 4, fumble: 2 },
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const FLINCH = "The patient flinches: 2 HP";
    const START_HP = 30, DOSES = 20, HP_PER_DOSE = 25;       // medical-officer 2: the medkit
    const VIT = { x: 16, y: 84, w: 1248, h: 64 };            // the vitals strip
    const CHART = { x: 24, y: 160, w: 390, h: 548 };         // the body chart
    const BX = 219, BY = 176, S = 5.2;                       // the figure: its top centre, px per body unit (100 tall)
    const WORK = { x: 430, y: 160, w: 826, h: 432 };         // the close-up
    const CX = WORK.x + WORK.w / 2, CY = WORK.y + WORK.h / 2 + 10;
    const TRAY = { x: 430, y: 604, h: 104, slot: 158, gap: 9 };
    const TOOLS = ["Sealer", "Gel", "Knitter", "Clamp", "Inhaler"];
    const KINDS = ["cut", "burn", "fracture", "bleed", "smoke"];
    const NAMES = { cut: "Cut", burn: "Burn", fracture: "Fracture", bleed: "Bleed", smoke: "Smoke" };
    const TOOL_FOR = { cut: [0], burn: [1], fracture: [2], bleed: [3, 0], smoke: [4] };
    // Where each kind can be on the body: [u, v, limb angle], body units from the top of the head.
    const ANCHORS = {
      cut: [[-18, 40, 1.45], [18, 40, 1.7], [-6, 62, 1.53], [6, 62, 1.61], [7, 36, 1.2], [-15.5, 24, 1.38], [15.5, 24, 1.76]],
      burn: [[-5, 24], [5, 27], [-11, 18], [11, 18], [-6, 66], [6, 66], [0, 40]],
      fracture: [[-18, 41, 1.45], [18, 41, 1.7], [-6.8, 85, 1.55], [6.8, 85, 1.59], [-15.5, 24, 1.38], [15.5, 24, 1.76]],
      bleed: [[-6, 58], [6, 58], [-15.5, 26], [15.5, 26], [-18, 38], [18, 38]],
      smoke: [[0, 27]],
    };
    const ECG = (p) => { const q = (m, w) => Math.exp(-(((p - m) / w) ** 2)); return 0.12 * q(0.12, 0.03) - 0.15 * q(0.27, 0.01) + q(0.3, 0.014) - 0.25 * q(0.33, 0.012) + 0.3 * q(0.55, 0.05); };
    const frac = (x) => x - Math.floor(x);
    const dist = (ax, ay, bx, by) => Math.hypot(ax - bx, ay - by);
    let s = null;

    /** HP now: the kit's job value (the one bar), read through the runner until the kit hands it to the api. */
    const hpNow = () => (typeof api.value === "number" ? api.value : window.REPAIRS && window.REPAIRS.run ? window.REPAIRS.run.value : START_HP);

    // ---------------------------------------------------------------- the patient
    function trail(r, len, band) {
      const th = (r() - 0.5) * 0.6, amp = 16 + 16 * r(), k = 1 + r(), ph = r() * 6.28, pts = [];
      for (let i = 0; i <= 24; i++) {
        const a = -len / 2 + (len * i) / 24, o = amp * Math.sin((Math.PI * k * i) / 24 + ph);
        pts.push([CX + a * Math.cos(th) - o * Math.sin(th), CY + a * Math.sin(th) + o * Math.cos(th)]);
      }
      const cum = [0];
      for (let i = 1; i < pts.length; i++) cum.push(cum[i - 1] + dist(...pts[i - 1], ...pts[i]));
      return { pts, cum, len: cum[cum.length - 1], band, prog: 0, tracing: false };
    }
    function patient(r) {
      const kinds = KINDS.slice().sort(() => r() - 0.5).slice(0, 4), used = [];
      return kinds.map((kind) => {
        const spots = ANCHORS[kind].filter(([u, v]) => used.every(([a, b]) => Math.hypot(u - a, v - b) > 7));
        const [u, v, la] = spots[Math.floor(r() * spots.length)] || ANCHORS[kind][0];
        used.push([u, v]);
        const size = Math.floor(r() * 3), w = { kind, u, v, la: la || 0, size, done: false };
        if (kind === "cut") w.tr = trail(r, 320 + 110 * size, 26 - 4 * size);
        if (kind === "burn") {
          w.R = 95 + 25 * size; w.a = r() * 6.28; w.b = r() * 6.28; w.cells = [];
          for (let y = CY - w.R * 1.3; y < CY + w.R * 1.3; y += 18) for (let x = CX - w.R * 1.3; x < CX + w.R * 1.3; x += 18) if (inBurn(w, x, y, 0)) w.cells.push([x, y]);
        }
        if (kind === "fracture") { w.B = [CX - 30, CY + 10]; w.E0 = [w.B[0] + 70 + 40 * r(), w.B[1] + (r() < 0.5 ? -1 : 1) * (45 + 35 * r())]; w.tilt = (r() - 0.5) * 0.6; }
        if (kind === "bleed") { w.P = 1.5 - 0.15 * size; w.ph = r() * 2; w.th = (r() - 0.5) * 0.5; w.tr = trail(r, 170, 22); }
        if (kind === "smoke") { w.need = 2 + size; w.B = 3.2 - 0.3 * size; w.ph = r() * 3; }
        reset(w);
        return w;
      });
    }
    function reset(w) {
      if (w.tr) { w.tr.prog = 0; w.tr.tracing = false; }
      if (w.kind === "burn") { w.covered = new Set(); w.stroke = false; }
      if (w.kind === "fracture") { w.E = w.E0.slice(); w.held = false; w.knit = 0; }
      if (w.kind === "bleed") w.clamped = false;
      if (w.kind === "smoke") { w.puffs = 0; w.last = -1; w.mist = 0; }
    }
    function inBurn(w, x, y, pad) {
      const th = Math.atan2(y - CY, x - CX);
      return dist(x, y, CX, CY) < w.R * (1 + 0.16 * Math.sin(3 * th + w.a) + 0.08 * Math.sin(5 * th + w.b)) + pad;
    }
    function project(tr, x, y) {
      let best = { d: 1e9, s: 0 };
      for (let i = 1; i < tr.pts.length; i++) {
        const [ax, ay] = tr.pts[i - 1], [bx, by] = tr.pts[i], L = tr.cum[i] - tr.cum[i - 1];
        const t = Math.max(0, Math.min(1, ((x - ax) * (bx - ax) + (y - ay) * (by - ay)) / (L * L)));
        const d = dist(x, y, ax + (bx - ax) * t, ay + (by - ay) * t);
        if (d < best.d) best = { d, s: tr.cum[i - 1] + L * t };
      }
      return best;
    }
    function at(tr, sv) {
      for (let i = 1; i < tr.pts.length; i++) if (tr.cum[i] >= sv) {
        const t = (sv - tr.cum[i - 1]) / (tr.cum[i] - tr.cum[i - 1] || 1), [ax, ay] = tr.pts[i - 1], [bx, by] = tr.pts[i];
        return [ax + (bx - ax) * t, ay + (by - ay) * t];
      }
      return tr.pts[tr.pts.length - 1];
    }

    // ---------------------------------------------------------------- treatment
    /** The sealer along a line: "slip", "done" or nothing. */
    function seal(tr, p) {
      if (p.pressed) { const q = project(tr, p.x, p.y); if (q.d < tr.band && Math.abs(q.s - tr.prog) < 40) tr.tracing = true; }
      if (tr.tracing && !p.down) tr.tracing = false;
      if (!tr.tracing) return null;
      const q = project(tr, p.x, p.y);
      if (q.d > tr.band) { tr.tracing = false; tr.prog = Math.max(0, tr.prog - 0.15 * tr.len); return "slip"; }
      if (q.s > tr.prog && q.s - tr.prog < 90) tr.prog = q.s;
      return tr.prog >= tr.len - 3 ? "done" : null;
    }
    const TREAT = {
      cut(w, p) { return s.tool === 0 ? seal(w.tr, p) : null; },
      burn(w, p) {
        if (s.tool !== 1) return null;
        if (p.pressed) w.stroke = inBurn(w, p.x, p.y, 10);
        if (!p.down) w.stroke = false;
        if (!w.stroke) return null;
        if (!inBurn(w, p.x, p.y, 45)) {
          w.stroke = false;
          [...w.covered].forEach((i, n) => { if (n % 5 === 0) w.covered.delete(i); });
          return "slip";
        }
        w.cells.forEach(([x, y], i) => { if (dist(x, y, p.x, p.y) < 34) w.covered.add(i); });
        return w.covered.size >= w.cells.length * 0.92 ? "done" : null;
      },
      fracture(w, p, dt) {
        if (s.tool !== 2) { w.held = false; return null; }
        const [ex, ey] = w.E, ang = w.tilt * Math.min(1, dist(ex, ey, ...w.B) / 80);
        if (p.pressed) {
          const fx = ex + Math.cos(ang) * 230, fy = ey + Math.sin(ang) * 230;
          const t = Math.max(0, Math.min(1, ((p.x - ex) * (fx - ex) + (p.y - ey) * (fy - ey)) / (230 * 230)));
          if (dist(p.x, p.y, ex + (fx - ex) * t, ey + (fy - ey) * t) < 34) { w.held = true; w.gx = ex - p.x; w.gy = ey - p.y; }
        }
        if (w.held && !p.down) w.held = false;
        if (!w.held) { const k = Math.min(1, dt * 0.8); w.E = [ex + (w.E0[0] - ex) * k, ey + (w.E0[1] - ey) * k]; return null; }
        const tr = 3 + 2.5 * w.size;
        w.E = [p.x + w.gx + tr * Math.sin(s.t * 7.3), p.y + w.gy + tr * Math.cos(s.t * 5.1)];
        const d = dist(...w.E, ...w.B);
        if (d < 12) w.knit += dt / (1.6 + 0.4 * w.size);
        else if (w.knit > 0 && d > 40) { w.knit = Math.max(0, w.knit - 0.35); w.held = false; return "slip"; }
        return w.knit >= 1 ? "done" : null;
      },
      bleed(w, p) {
        if (!w.clamped) {
          if (s.tool !== 3 || !p.pressed || dist(p.x, p.y, CX, CY) > 80) return null;
          if (spurting(w)) return "slip";
          w.clamped = true; return null;
        }
        return s.tool === 0 ? seal(w.tr, p) : null;
      },
      smoke(w, p) {
        if (s.tool !== 4 || !p.pressed || !inWork(p.x, p.y)) return null;
        const cyc = (s.t + w.ph) / w.B, n = Math.floor(cyc);
        if (frac(cyc) >= 0.45) return "slip";
        if (w.last === n) return null;
        w.last = n; w.puffs++; w.mist = 1;
        return w.puffs >= w.need ? "done" : null;
      },
    };
    const spurting = (w) => !w.clamped && ((s.t + w.ph) % w.P) < 0.45;
    const rising = (w) => frac((s.t + w.ph) / w.B) < 0.45;
    const inWork = (x, y) => x >= WORK.x && x <= WORK.x + WORK.w && y >= WORK.y && y <= WORK.y + WORK.h;
    const chartXY = (w) => [BX + w.u * S, BY + w.v * S];
    const nextOpen = () => { const i = s.wounds.findIndex((w) => !w.done); return i < 0 ? s.cur : i; };

    /** Mouse, touch, or the pad's cursor (the stick moves it, Space presses). */
    function pointer(dt, input) {
      if (input.stick.x || input.stick.y) {
        if (!s.pad) { s.pad = true; s.vc = [CX, CY]; }
        s.vc = [Math.max(0, Math.min(api.W, s.vc[0] + input.stick.x * 420 * dt)), Math.max(api.BAR_H, Math.min(api.H, s.vc[1] + input.stick.y * 420 * dt))];
      }
      if (input.pressed || input.x !== s.mx || input.y !== s.my) s.pad = false;
      s.mx = input.x; s.my = input.y;
      const p = s.pad ? { x: s.vc[0], y: s.vc[1], down: input.action, pressed: input.actionPressed, released: s.act && !input.action }
        : { x: input.x, y: input.y, down: input.down, pressed: input.pressed, released: input.released };
      s.act = input.action;
      return p;
    }

    // ---------------------------------------------------------------- drawing
    function figure(g) {
      const P = (u, v) => [BX + u * S, BY + v * S];
      const limbs = [[[-14, 16], [-17, 32], 5.5], [[-17, 32], [-19, 47], 4.5], [[14, 16], [17, 32], 5.5], [[17, 32], [19, 47], 4.5],
        [[-5.5, 51], [-6.5, 74], 7.5], [[-6.5, 74], [-7, 95], 5.5], [[5.5, 51], [6.5, 74], 7.5], [[6.5, 74], [7, 95], 5.5], [[0, 11], [0, 16], 5]];
      const torso = [[-13.5, 15], [13.5, 15], [12, 30], [10.5, 46], [11, 52], [-11, 52], [-10.5, 46], [-12, 30]];
      for (const [col, o] of [["#6f8aa8", 4], ["#223247", 0]]) {
        g.lineCap = "round"; g.lineJoin = "round"; g.strokeStyle = col; g.fillStyle = col;
        for (const [a, b, w] of limbs) { g.beginPath(); g.moveTo(...P(...a)); g.lineTo(...P(...b)); g.lineWidth = w * S + o; g.stroke(); }
        g.beginPath(); torso.forEach((q, i) => (i ? g.lineTo(...P(...q)) : g.moveTo(...P(...q)))); g.closePath(); g.fill();
        if (o) { g.lineWidth = o; g.stroke(); }
        D.disc(g, ...P(0, 6), 6 * S + o / 2, col);
        for (const sx of [-1, 1]) { D.disc(g, ...P(sx * 19.6, 50), 2.6 * S + o / 2, col); g.beginPath(); g.ellipse(...P(sx * 8.2, 97), 3.4 * S + o / 2, 1.6 * S + o / 2, 0, 0, Math.PI * 2); g.fill(); }
      }
      g.lineCap = "butt";
      g.strokeStyle = "#2c3f57"; g.lineWidth = 2;
      g.beginPath(); g.moveTo(...P(0, 18)); g.lineTo(...P(0, 34)); g.stroke();
    }
    function marker(g, w, i) {
      const [x, y] = chartXY(w), k = 1.35 + 0.4 * w.size;
      g.save(); g.translate(x, y);
      if (w.kind === "cut") { g.rotate(w.la + Math.PI / 2 + 0.5); g.strokeStyle = C.danger; g.lineWidth = 5; g.beginPath(); g.moveTo(-10 * k, 0); g.lineTo(10 * k, 0); g.stroke(); g.lineWidth = 2; for (let j = -1; j <= 1; j++) { g.beginPath(); g.moveTo(j * 6 * k, -5); g.lineTo(j * 6 * k, 5); g.stroke(); } }
      if (w.kind === "burn") { g.beginPath(); for (let j = 0; j < 9; j++) { const a = (j / 9) * Math.PI * 2, rr = 9 * k * (1 + 0.25 * Math.sin(j * 2.3)); j ? g.lineTo(Math.cos(a) * rr, Math.sin(a) * rr) : g.moveTo(rr, 0); } g.closePath(); g.fillStyle = C.amber; g.fill(); g.strokeStyle = "#5a2f0c"; g.lineWidth = 2; for (let j = -1; j <= 1; j++) { g.beginPath(); g.moveTo(-6 * k + j * 5, 6 * k); g.lineTo(6 * k + j * 5, -6 * k); g.stroke(); } }
      if (w.kind === "fracture") { g.rotate(w.la + Math.PI / 2); g.strokeStyle = C.fg; g.lineWidth = 4; g.beginPath(); g.moveTo(-11 * k, 0); g.lineTo(-4 * k, -5); g.lineTo(0, 5); g.lineTo(4 * k, -5); g.lineTo(11 * k, 0); g.stroke(); }
      if (w.kind === "bleed") { g.beginPath(); g.moveTo(0, -12 * k); g.quadraticCurveTo(9 * k, 0, 0, 8 * k); g.quadraticCurveTo(-9 * k, 0, 0, -12 * k); g.fillStyle = C.danger; g.fill(); g.strokeStyle = "#ffd0d4"; g.lineWidth = 2; g.stroke(); }
      if (w.kind === "smoke") for (const [dx, dy, rr] of [[-12, 2, 13], [0, -6, 15], [12, 3, 12], [-4, 10, 11], [8, 12, 10]]) D.disc(g, dx * k * 0.8, dy * k * 0.8, rr * k * 0.8, "rgba(160,170,184,0.75)");
      g.restore();
      if (w.done) { D.disc(g, x + 14, y - 14, 11, C.ok); g.beginPath(); g.moveTo(x + 8, y - 14); g.lineTo(x + 12, y - 9); g.lineTo(x + 20, y - 19); g.strokeStyle = "#0b111b"; g.lineWidth = 3; g.stroke(); }
      else if (i === s.cur) D.ring(g, x, y, 26 + 3 * Math.sin(s.t * 5), C.amber, 3);
    }
    function toolIcon(g, i, x, y, col) {
      g.save(); g.translate(x, y); g.lineCap = "round"; g.lineJoin = "round";
      if (i === 0) { g.rotate(-0.6); D.round(g, -30, -8, 48, 16, 6); g.fillStyle = col; g.fill(); g.fillStyle = "#1d2738"; g.fillRect(-20, -3, 26, 6); g.beginPath(); g.moveTo(18, -6); g.lineTo(32, 0); g.lineTo(18, 6); g.closePath(); g.fillStyle = C.accent; g.fill(); }
      if (i === 1) { g.beginPath(); g.moveTo(-26, -12); g.lineTo(14, -9); g.lineTo(14, 9); g.lineTo(-26, 12); g.closePath(); g.fillStyle = col; g.fill(); D.round(g, 14, -7, 14, 14, 3); g.fillStyle = C.amber; g.fill(); g.fillStyle = "#3a4658"; g.fillRect(-28, -13, 5, 26); }
      if (i === 2) { g.strokeStyle = col; g.lineWidth = 7; g.beginPath(); g.moveTo(-20, -18); g.lineTo(-20, 12); g.lineTo(20, 12); g.lineTo(20, -18); g.stroke(); g.strokeStyle = C.lilac; g.lineWidth = 3; g.beginPath(); g.moveTo(-14, -16); g.lineTo(14, -16); g.stroke(); }
      if (i === 3) { g.strokeStyle = col; g.lineWidth = 4; D.ring(g, -22, 10, 7, col, 4); D.ring(g, -8, 16, 7, col, 4); g.beginPath(); g.moveTo(-17, 5); g.lineTo(24, -16); g.moveTo(-3, 11); g.lineTo(26, -10); g.stroke(); }
      if (i === 4) { D.round(g, -10, -22, 20, 34, 6); g.fillStyle = col; g.fill(); D.round(g, -4, 6, 30, 14, 5); g.fillStyle = col; g.fill(); g.fillStyle = C.accent; g.fillRect(-7, -28, 14, 7); }
      g.restore(); g.lineCap = "butt";
    }
    function bone(g, x0, y0, x1, y1, knob) {
      g.lineCap = "round"; g.strokeStyle = "#8a8170"; g.lineWidth = 44; g.beginPath(); g.moveTo(x0, y0); g.lineTo(x1, y1); g.stroke();
      g.strokeStyle = "#d9d2bf"; g.lineWidth = 38; g.stroke(); g.lineCap = "butt";
      if (knob) { D.disc(g, x0, y0 - 14, 26, "#d9d2bf"); D.disc(g, x0, y0 + 14, 26, "#d9d2bf"); }
    }
    function closeUp(g, w) {
      g.save(); D.round(g, WORK.x, WORK.y, WORK.w, WORK.h, 16); g.clip();
      const skin = () => { g.fillStyle = "#6b4f42"; g.fillRect(WORK.x, WORK.y, WORK.w, WORK.h); g.fillStyle = "rgba(0,0,0,0.18)"; for (let j = 0; j < 7; j++) { g.beginPath(); g.ellipse(WORK.x + 80 + j * 120, WORK.y + 60 + (j % 3) * 140, 70, 30, j, 0, Math.PI * 2); g.fill(); } };
      if (w.kind === "cut" || w.kind === "bleed") skin();
      if (w.kind === "cut") line(g, w.tr, w.done, "#5a0f18");
      if (w.kind === "burn") {
        skin();
        g.beginPath(); for (let j = 0; j <= 72; j++) { const th = (j / 72) * Math.PI * 2, rr = w.R * (1 + 0.16 * Math.sin(3 * th + w.a) + 0.08 * Math.sin(5 * th + w.b)); j ? g.lineTo(CX + Math.cos(th) * rr, CY + Math.sin(th) * rr) : g.moveTo(CX + rr, CY); }
        g.closePath(); g.fillStyle = w.done ? "#8a5f50" : "#a8442e"; g.fill();
        g.setLineDash([10, 8]); g.strokeStyle = w.done ? "#6b4f42" : C.amber; g.lineWidth = 3; g.stroke(); g.setLineDash([]);
        if (!w.done) for (let j = 0; j < 9; j++) D.disc(g, CX + Math.cos(j * 2.4) * w.R * 0.55 * ((j % 3) / 3 + 0.3), CY + Math.sin(j * 2.4) * w.R * 0.5 * ((j % 3) / 3 + 0.3), 7, "#d9826a");
        g.fillStyle = "rgba(120,226,206,0.55)";
        for (const i of w.covered) { const [x, y] = w.cells[i]; g.beginPath(); g.arc(x, y, 13, 0, Math.PI * 2); g.fill(); }
      }
      if (w.kind === "fracture") {
        g.fillStyle = "#0b1622"; g.fillRect(WORK.x, WORK.y, WORK.w, WORK.h);
        g.fillStyle = "rgba(107,79,66,0.35)"; D.round(g, WORK.x + 40, CY - 120, WORK.w - 80, 260, 120); g.fill();
        const [ex, ey] = w.done ? w.B : w.E, ang = w.done ? 0 : w.tilt * Math.min(1, dist(ex, ey, ...w.B) / 80);
        bone(g, w.B[0] - 300, w.B[1], w.B[0], w.B[1], true);
        bone(g, ex + Math.cos(ang) * 300, ey + Math.sin(ang) * 300, ex, ey, true);
        if (!w.done) {
          g.strokeStyle = "#5a5446"; g.lineWidth = 3;
          for (const [x, y, sx] of [[w.B[0], w.B[1], 1], [ex, ey, -1]]) { g.beginPath(); g.moveTo(x, y - 19); g.lineTo(x + sx * 6, y - 6); g.lineTo(x - sx * 4, y + 4); g.lineTo(x + sx * 5, y + 19); g.stroke(); }
          D.ring(g, ...w.B, 12, "rgba(190,159,230,0.5)", 2);
          if (w.knit > 0) { D.ring(g, ...w.B, 30 + 10 * Math.sin(s.t * 12), C.lilac, 4); g.beginPath(); g.arc(...w.B, 46, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * w.knit); g.strokeStyle = C.lilac; g.lineWidth = 6; g.stroke(); }
        } else { g.strokeStyle = C.lilac; g.lineWidth = 3; g.beginPath(); g.moveTo(w.B[0], w.B[1] - 20); g.lineTo(w.B[0], w.B[1] + 20); g.stroke(); }
      }
      if (w.kind === "bleed") {
        g.save(); g.translate(CX, CY); g.rotate(w.th);
        g.strokeStyle = "#5a1620"; g.lineWidth = 46; g.beginPath(); g.moveTo(-500, 0); g.lineTo(500, 0); g.stroke();
        g.strokeStyle = "#7a1f2a"; g.lineWidth = 36; g.stroke();
        if (!w.done) { g.beginPath(); g.ellipse(0, 40, 120, 40, 0, 0, Math.PI * 2); g.fillStyle = "rgba(122,20,32,0.6)"; g.fill(); }
        if (w.clamped || w.done) {
          g.fillStyle = C.steel; g.fillRect(-130, -34, 14, 68);
          g.strokeStyle = C.steel; g.lineWidth = 6; g.beginPath(); g.moveTo(-123, -34); g.lineTo(-170, -90); g.moveTo(-123, 34); g.lineTo(-170, -70); g.stroke();
          D.ring(g, -176, -96, 9, C.steel, 4); D.ring(g, -176, -64, 9, C.steel, 4);
        }
        g.restore();
        if (!w.clamped && !w.done) {
          const sp = spurting(w), ph = ((s.t + w.ph) % w.P) / 0.45;
          D.disc(g, CX, CY, 16, sp ? C.danger : "#a01c2a");
          if (sp) for (let j = 0; j < 18; j++) { const q = (ph + j / 18) % 1, a = -Math.PI / 2 + w.th + ((j % 6) - 2.5) * 0.12; D.disc(g, CX + Math.cos(a) * q * 150, CY + Math.sin(a) * q * 150 + q * q * 90, 7 - 4 * q, C.danger); }
        }
        if (w.clamped || w.done) line(g, w.tr, w.done, "#a01c2a");
      }
      if (w.kind === "smoke") {
        g.fillStyle = "#0b1622"; g.fillRect(WORK.x, WORK.y, WORK.w, WORK.h);
        const up = rising(w), k = 1 + 0.05 * Math.sin(frac((s.t + w.ph) / w.B) * Math.PI * 2 - 0.3);
        g.save(); g.translate(CX, CY + 40); g.scale(k, k);
        D.disc(g, 0, -262, 62, "#223247"); D.ring(g, 0, -262, 62, "#6f8aa8", 3);
        g.fillStyle = "#223247"; g.fillRect(-34, -210, 68, 60); g.strokeStyle = "#6f8aa8"; g.lineWidth = 3; g.strokeRect(-34, -210, 68, 60);
        g.beginPath(); g.moveTo(-230, -150); g.quadraticCurveTo(0, -200, 230, -150); g.lineTo(200, 190); g.lineTo(-200, 190); g.closePath();
        g.fillStyle = "#223247"; g.fill(); g.strokeStyle = "#6f8aa8"; g.lineWidth = 3; g.stroke();
        g.strokeStyle = "#2c3f57"; g.lineWidth = 4; for (let j = 0; j < 5; j++) { g.beginPath(); g.ellipse(0, -90 + j * 50, 170 - j * 6, 26, 0, 0.15, Math.PI - 0.15); g.stroke(); }
        const left = w.done ? 0 : 1 - w.puffs / w.need;
        for (const sx of [-1, 1]) {
          g.beginPath(); g.ellipse(sx * 85, 10, 70, 140, sx * 0.1, 0, Math.PI * 2); g.fillStyle = "#3a2a30"; g.fill(); g.strokeStyle = "#8a6a70"; g.lineWidth = 2; g.stroke();
          for (let j = 0; j < 6; j++) D.disc(g, sx * (70 + (j % 3) * 18), -60 + j * 34, 26, `rgba(150,160,172,${0.8 * left})`);
        }
        g.restore();
        if (w.mist > 0) for (let j = 0; j < 14; j++) D.disc(g, CX + Math.sin(j * 1.7) * 40 * (1 - w.mist), WORK.y + 30 + (1 - w.mist) * 120 + j * 6, 8, `rgba(79,195,247,${0.5 * w.mist})`);
        // The breath: an arrow up while the chest rises (the moment), down and dim as it falls.
        const ax = WORK.x + WORK.w - 70, ay = WORK.y + 90;
        g.beginPath(); if (up) { g.moveTo(ax, ay - 34); g.lineTo(ax - 26, ay + 6); g.lineTo(ax + 26, ay + 6); } else { g.moveTo(ax, ay + 34); g.lineTo(ax - 26, ay - 6); g.lineTo(ax + 26, ay - 6); }
        g.closePath(); g.fillStyle = up ? C.ok : "#2a3446"; g.fill();
        for (let j = 0; j < w.need; j++) D.disc(g, ax - ((w.need - 1) * 22) / 2 + j * 22, ay + 60, 8, j < w.puffs || w.done ? C.accent : "#2a3446");
      }
      g.restore();
      D.round(g, WORK.x, WORK.y, WORK.w, WORK.h, 16); g.lineWidth = 2; g.strokeStyle = C.line; g.stroke();
      // The wound's name, a check once treated.
      D.round(g, WORK.x + 14, WORK.y + 14, 150, 36, 18); g.fillStyle = "rgba(4,6,10,0.75)"; g.fill();
      D.text(g, NAMES[w.kind], WORK.x + 34, WORK.y + 32, 20, w.done ? C.ok : C.fg, "left", 700);
      for (let j = 0; j <= w.size; j++) D.disc(g, WORK.x + 124 + j * 12, WORK.y + 32, 4, w.done ? C.ok : C.amber);
    }
    function line(g, tr, done, raw) {
      g.lineCap = "round"; g.lineJoin = "round";
      if (!done) {
        g.beginPath(); tr.pts.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y)));
        g.strokeStyle = "rgba(255,255,255,0.08)"; g.lineWidth = tr.band * 2; g.stroke();
        g.strokeStyle = raw; g.lineWidth = 9; g.stroke();
        g.strokeStyle = "#e05a64"; g.lineWidth = 3; g.stroke();
      }
      const end = done ? tr.len : tr.prog;
      if (end > 0) {
        g.beginPath(); g.moveTo(...tr.pts[0]);
        for (let i = 1; i < tr.pts.length && tr.cum[i - 1] < end; i++) g.lineTo(...(tr.cum[i] <= end ? tr.pts[i] : at(tr, end)));
        g.strokeStyle = "#d79a8a"; g.lineWidth = 6; g.stroke();
        g.strokeStyle = C.ok; g.lineWidth = 2;
        for (let sv = 10; sv < end; sv += 26) { const [x, y] = at(tr, sv), [x2, y2] = at(tr, Math.min(tr.len, sv + 2)), a = Math.atan2(y2 - y, x2 - x) + Math.PI / 2; g.beginPath(); g.moveTo(x - Math.cos(a) * 8, y - Math.sin(a) * 8); g.lineTo(x + Math.cos(a) * 8, y + Math.sin(a) * 8); g.stroke(); }
      }
      if (!done && !tr.tracing) { const [x, y] = at(tr, tr.prog); D.ring(g, x, y, 16 + 4 * Math.sin(s.t * 6), C.accent, 3); }
      g.lineCap = "butt";
    }
    function vitals(g) {
      const hp = hpNow(), smoke = s.wounds.find((w) => w.kind === "smoke" && !w.done);
      const hr = Math.round(128 - (hp - START_HP) * 0.8 + 30 * s.flinch);
      const spo2 = Math.round(smoke ? 86 + (8 * smoke.puffs) / smoke.need : 97);
      D.panel(g, VIT.x, VIT.y, VIT.w, VIT.h, 12, "#0b1018");
      const y = VIT.y + VIT.h / 2;
      D.text(g, "HR", VIT.x + 20, y, 16, C.dim, "left", 700);
      D.text(g, String(hr), VIT.x + 56, y, 30, hr > 110 ? C.warn : C.ok, "left", 700);
      g.save(); g.beginPath(); g.rect(130, VIT.y + 6, 420, VIT.h - 12); g.clip();
      g.beginPath();
      for (let x = 130; x <= 550; x += 2) { const p = frac((s.t - (550 - x) / 140) * (hr / 60)); const yy = y + 6 - 22 * ECG(p); x === 130 ? g.moveTo(x, yy) : g.lineTo(x, yy); }
      g.strokeStyle = C.ok; g.lineWidth = 2; g.stroke(); g.restore();
      D.text(g, "SpO2", 590, y, 16, C.dim, "left", 700);
      D.text(g, spo2 + "%", 640, y, 30, spo2 < 92 ? C.warn : C.ok, "left", 700);
      if (spo2 < 92) { g.beginPath(); g.moveTo(718, y - 12); g.lineTo(706, y + 10); g.lineTo(730, y + 10); g.closePath(); g.fillStyle = C.warn; g.fill(); }
      D.text(g, "HP", 770, y, 16, C.dim, "left", 700);
      D.text(g, String(Math.round(hp)), 800, y, 30, hp >= 75 ? C.ok : hp >= 25 ? C.warn : C.danger, "left", 700);
      D.round(g, 860, y - 7, 160, 14, 7); g.fillStyle = "#1a2230"; g.fill();
      D.round(g, 860, y - 7, Math.max(14, 1.6 * hp), 14, 7); g.fillStyle = hp >= 75 ? C.ok : C.warn; g.fill();
      g.fillStyle = C.fg; g.fillRect(860 + 1.6 * 75 - 1, y - 11, 3, 22);
      const used = Math.ceil(Math.max(0, hp - START_HP - 1e-6) / HP_PER_DOSE), left = DOSES - used;
      D.text(g, "Doses", 1050, y, 16, C.dim, "left", 700);
      for (let i = 0; i < DOSES; i++) { const x = 1102 + (i % 10) * 15, yy = y - 9 + Math.floor(i / 10) * 18; if (i < left) D.disc(g, x, yy, 5, C.lilac); else D.ring(g, x, yy, 4, "#3a4658", 2); }
    }

    return {
      get state() { return s; },   // for tools: the headless checks read the round
      step(index) {
        if (!s) s = { wounds: patient(api.rand()), t: 0, tool: 0, cur: 0, flinch: 0, pad: false, vc: [CX, CY], mx: -1, my: -1, act: false, index: -1, waiting: false };
        if (index === s.index) s.wounds.forEach((w) => { if (!w.done) reset(w); });   // three slips: the step restarts
        s.index = index; s.waiting = false;
        if (s.wounds[s.cur].done) s.cur = nextOpen();
      },
      update(dt, input) {
        s.t += dt;
        s.flinch = Math.max(0, s.flinch - dt * 1.5);
        for (const w of s.wounds) if (w.mist) w.mist = Math.max(0, w.mist - dt * 1.5);
        const p = pointer(dt, input);
        for (let i = 0; i < 5; i++) if (input.hit.has("Digit" + (i + 1))) s.tool = i;
        if (input.hit.has("KeyQ")) s.tool = (s.tool + 4) % 5;
        if (input.hit.has("KeyE")) s.tool = (s.tool + 1) % 5;
        if (p.pressed) {
          if (p.y >= TRAY.y && p.y <= TRAY.y + TRAY.h) {
            const i = Math.floor((p.x - TRAY.x) / (TRAY.slot + TRAY.gap));
            if (i >= 0 && i < 5 && p.x - TRAY.x - i * (TRAY.slot + TRAY.gap) <= TRAY.slot) { s.tool = i; return; }
          }
          const wi = s.wounds.findIndex((w) => dist(p.x, p.y, ...chartXY(w)) < 28);
          if (wi >= 0) { if (!s.wounds[wi].done && !s.waiting) s.cur = wi; return; }
        }
        const w = s.wounds[s.cur];
        if (s.waiting || !w || w.done) return;
        const out = TREAT[w.kind](w, p, dt);
        if (out === "slip") { s.flinch = 1; api.fumble(FLINCH); return; }
        if (out === "done") { w.done = true; s.waiting = true; api.stepDone(); }
      },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        vitals(g);
        const shake = s.flinch > 0 ? Math.sin(s.t * 50) * 5 * s.flinch : 0;
        D.panel(g, CHART.x, CHART.y, CHART.w, CHART.h, 16, "#0b1018");
        g.save(); g.translate(shake, 0); figure(g); s.wounds.forEach((w, i) => marker(g, w, i)); g.restore();
        const w = s.wounds[s.cur];
        // A leader from the chosen wound on the chart to its close-up.
        const [mx, my] = chartXY(w);
        g.strokeStyle = "rgba(242,160,70,0.35)"; g.lineWidth = 2; g.setLineDash([6, 6]);
        g.beginPath(); g.moveTo(mx + 24, my); g.lineTo(WORK.x, WORK.y + 32); g.stroke(); g.setLineDash([]);
        g.save(); g.translate(shake * 0.5, 0); closeUp(g, w); g.restore();
        // The tray: the chosen tool raised and ringed.
        for (let i = 0; i < 5; i++) {
          const x = TRAY.x + i * (TRAY.slot + TRAY.gap), on = s.tool === i;
          D.panel(g, x, TRAY.y, TRAY.slot, TRAY.h, 14, on ? "#1d2738" : "#0d131c", on ? C.amber : C.line);
          if (on) { D.round(g, x, TRAY.y, TRAY.slot, TRAY.h, 14); g.lineWidth = 4; g.strokeStyle = C.amber; g.stroke(); }
          toolIcon(g, i, x + TRAY.slot / 2, TRAY.y + 40, on ? "#c9d2dc" : C.steel);
          D.text(g, TOOLS[i], x + TRAY.slot / 2, TRAY.y + 84, 18, on ? C.fg : C.dim, "center", 700);
        }
        // The tool in hand over the close-up: bright when it is this wound's, grey when it does nothing here.
        const p = s.pad ? { x: s.vc[0], y: s.vc[1] } : { x: s.mx, y: s.my };
        if (inWork(p.x, p.y) && !w.done) {
          const need = TOOL_FOR[w.kind][w.kind === "bleed" && w.clamped ? 1 : 0];
          toolIcon(g, s.tool, p.x + 36, p.y - 30, s.tool === need ? "#e8eef6" : "#3a4658");
          if (s.pad) { g.strokeStyle = C.fg; g.lineWidth = 2; g.beginPath(); g.moveTo(p.x - 12, p.y); g.lineTo(p.x + 12, p.y); g.moveTo(p.x, p.y - 12); g.lineTo(p.x, p.y + 12); g.stroke(); }
        }
      },
    };
  },
});
