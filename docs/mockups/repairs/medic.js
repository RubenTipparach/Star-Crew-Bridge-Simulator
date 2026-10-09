/*
 * repairs/medic.js: the medic's treatment (openspec/changes/repair-minigames, design 3; rates from medical-officer 2).
 *
 * One body on one screen. The vitals strip runs along the top (heart rate, blood oxygen, HP, the medkit's doses); the
 * body chart on the left marks each wound by kind and size (a cut, a burn, a fracture, a bleed, smoke in the lungs);
 * the close-up on the right is the chosen wound, big, with its target drawn on it; the tool tray is along the bottom
 * (click a slot, or keys 1 to 5, Q and E). Each wound takes its own tool:
 * - sealer, a cut: press on or near the cut and sweep along it, either way; what the tip passes over seals, drifting
 *   off only pauses;
 * - gel, a burn: paint it with a wide brush until it is covered; gel off the burn is wasted, not a slip;
 * - knitter, a fracture: grab the loose bone, drag it to its socket (it snaps in), hold while it knits; letting go
 *   pauses; yanking it well away once knitting has started is the slip;
 * - clamp then sealer, a bleed: the ring round the bleed shows the spurt (hatched red) and the gap (green); clamp in
 *   the gap, then seal the tear like a cut; a clamp into a spurt is the slip;
 * - inhaler, smoke: puff while the chest rises (the gauge climbs, green); a puff on the out-breath is the slip.
 * The tool in hand shows at the cursor: greyed with a red cross when it does nothing here, and a press says "Wrong
 * tool". A press only treats when it starts inside the close-up, so a click on the tray or the chart never touches the
 * wound. A wound is one step; HP comes back at the medic's field rate (4.0 HP/s to 75 HP), the bar waiting on the rate
 * as in every repair, and a dose goes per 25 HP given. A slip: the patient flinches, 2 HP (the job's fumble). The
 * arrows or W A S D move a cursor and Space presses, for a pad.
 */
RepairKit.register({
  id: "medic",
  title: "Medic: treatment",
  place: "Medbay or in the field",
  group: "Life",
  doneWord: "Treated",
  hazard: "The patient flinches: 2 HP",
  // The how-to card (repair-minigames 6g), drawn by the kit: pictures and a few words, on demand.
  guide: {
    steps: [
      { icon: "tap", text: "Tap a wound on the body" },
      { icon: "tool", text: "Pick the tool it needs" },
      { icon: "sweep", text: "Sweep sealer or gel over it" },
      { icon: "band", text: "Clamp or puff on green" },
    ],
    mistake: "Clamp in a spurt, or puff on the out-breath: a flinch, 2 HP",
    now: (q) => { const w = q.wounds && q.wounds[q.cur]; if (!w || w.done) return 0; const need = w.kind === "bleed" ? (w.clamped ? 0 : 3) : { cut: 0, burn: 1, fracture: 2, smoke: 4 }[w.kind]; return q.tool !== need ? 1 : need >= 3 ? 3 : 2; },
  },
  down: "Beds and field healing still work without a medic (medical-officer)",
  job: { unit: "HP", start: 30, target: 75, rateBy: { officer: 4.0, rating: 4.0 }, steps: 4, fumble: 2 },
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const FLINCH = "The patient flinches: 2 HP";
    const START_HP = 30, DOSES = 20, HP_PER_DOSE = 25;       // medical-officer 2: the medkit
    // Layout, px on the 1280 x 720 canvas.
    const VIT = { x: 16, y: 82, w: 1248, h: 58 };            // the vitals strip
    const CHART = { x: 16, y: 150, w: 300, h: 558 };         // the body chart
    const BX = 166, BY = 172, S = 5.0;                       // the figure: its top centre, px per body unit (100 tall)
    const WORK = { x: 330, y: 150, w: 934, h: 446 };         // the close-up
    const CX = WORK.x + WORK.w / 2, CY = WORK.y + WORK.h / 2;
    const TRAY = { x: 330, y: 606, h: 102, slot: 178, gap: 11 };
    // Feel. Generous on purpose (owner, 2026-10-08: "the tools dont respond well").
    const BAND = 40;          // sealer: how far off the cut a press still works, px
    const BIN = 6;            // sealer: coverage is kept in bins this long along the cut, px
    const TIP = 18;           // sealer: the tip seals this far either side of where it sits on the cut, px
    const SEAL_DONE = 0.95;   // sealer: coverage that closes the cut
    const BRUSH = 50;         // gel: brush radius, px
    const GEL_DONE = 0.85;    // gel: coverage that covers the burn
    const BONE = 300, BONE_W = 40, GRAB = 46;   // knitter: the loose bone's length and width, the grab reach off its axis
    const SNAP_IN = 25, SNAP_OUT = 40, YANK = 80;   // knitter: snaps to the socket inside 25 px; a yank past 80 px slips
    const SPURT = 0.42;       // bleed: a spurt's length, s
    const INHALE = 0.45;      // smoke: the in-breath's share of a breath
    const TOOLS = ["Sealer", "Gel", "Knitter", "Clamp", "Inhaler"];
    const KINDS = ["cut", "burn", "fracture", "bleed", "smoke"];
    const NAMES = { cut: "Cut", burn: "Burn", fracture: "Fracture", bleed: "Bleed", smoke: "Smoke" };
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
    const fxr = KIT.rng(7);   // a cosmetic stream for sparks, so a shot is the same every time
    let s = null;

    // ---------------------------------------------------------------- the patient
    /** A line across the close-up (a cut, a bleed's tear): points, arc lengths, and its sealed bins. */
    function trail(r, len, ang, amp0) {
      const amp = amp0 * (0.5 + 0.5 * r()), k = 1 + r(), ph = r() * 6.28, pts = [];
      for (let i = 0; i <= 32; i++) {
        const a = -len / 2 + (len * i) / 32, o = amp * Math.sin((Math.PI * k * i) / 32 + ph);
        pts.push([CX + a * Math.cos(ang) - o * Math.sin(ang), CY + a * Math.sin(ang) + o * Math.cos(ang)]);
      }
      const cum = [0];
      for (let i = 1; i < pts.length; i++) cum.push(cum[i - 1] + dist(...pts[i - 1], ...pts[i]));
      const n = Math.ceil(cum[cum.length - 1] / BIN);
      return { pts, cum, len: cum[cum.length - 1], n, seal: new Array(n).fill(-1), count: 0, lastS: null };
    }
    function wound(kind, size, r, u = 0, v = 0, la = 0) {
      const w = { kind, u, v, la, size, done: false };
      if (kind === "cut") w.tr = trail(r, 420 + 110 * size, (r() - 0.5) * 0.5, 34);
      if (kind === "burn") {
        w.R = 100 + 22 * size; w.a = r() * 6.28; w.b = r() * 6.28; w.cells = [];
        for (let y = CY - w.R * 1.3; y < CY + w.R * 1.3; y += 14) for (let x = CX - w.R * 1.3; x < CX + w.R * 1.3; x += 14) if (inBurn(w, x, y, 0)) w.cells.push([x, y]);
      }
      if (kind === "fracture") {
        w.B = [CX - 80, CY];
        const side = r() < 0.5 ? -1 : 1;    // the loose end sits above or below the socket, and leans back across it
        w.E0 = [w.B[0] + 110 + 50 * r(), w.B[1] + side * (70 + 40 * r())];
        w.tilt = -side * (0.2 + 0.15 * r());
        w.knitT = 1.4 + 0.3 * size;          // s held in the socket to knit
      }
      if (kind === "bleed") { w.P = 1.6 - 0.15 * size; w.ph = r() * 2; w.th = (r() - 0.5) * 0.5; w.tr = trail(r, 320, w.th, 8); }
      if (kind === "smoke") { w.need = 2 + size; w.B = 3.0 - 0.25 * size; w.ph = r() * 3; }
      reset(w);
      return w;
    }
    function patient(r) {
      const kinds = KINDS.slice().sort(() => r() - 0.5).slice(0, 4), used = [];
      return kinds.map((kind) => {
        const spots = ANCHORS[kind].filter(([u, v]) => used.every(([a, b]) => Math.hypot(u - a, v - b) > 7));
        const [u, v, la] = spots[Math.floor(r() * spots.length)] || ANCHORS[kind][0];
        used.push([u, v]);
        return wound(kind, Math.floor(r() * 3), r, u, v, la || 0);
      });
    }
    function reset(w) {
      if (w.tr) { w.tr.seal.fill(-1); w.tr.count = 0; w.tr.lastS = null; }
      if (w.kind === "burn") { w.cov = new Uint8Array(w.cells.length); w.covN = 0; w.lp = null; w.layer = null; }
      if (w.kind === "fracture") { w.E = w.E0.slice(); w.held = false; w.snap = false; w.seated = false; w.knit = 0; }
      if (w.kind === "bleed") w.clamped = false;
      if (w.kind === "smoke") { w.puffs = 0; w.last = -1; w.mist = 0; }
    }
    function inBurn(w, x, y, pad) {
      const th = Math.atan2(y - CY, x - CX);
      return dist(x, y, CX, CY) < w.R * (1 + 0.16 * Math.sin(3 * th + w.a) + 0.08 * Math.sin(5 * th + w.b)) + pad;
    }
    /** The nearest point on a line: its distance off, and its arc length along. */
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
    const segDist = (px, py, ax, ay, bx, by) => {
      const L2 = (bx - ax) ** 2 + (by - ay) ** 2, t = L2 ? Math.max(0, Math.min(1, ((px - ax) * (bx - ax) + (py - ay) * (by - ay)) / L2)) : 0;
      return dist(px, py, ax + (bx - ax) * t, ay + (by - ay) * t);
    };
    /** The loose bone's angle: it tilts the further it is from its socket, and lies straight in it. */
    const boneAng = (w) => (w.snap || w.done ? 0 : w.tilt * Math.min(1, dist(...w.E, ...w.B) / 80));
    const boneFar = (w) => { const a = boneAng(w); return [w.E[0] + Math.cos(a) * BONE, w.E[1] + Math.sin(a) * BONE]; };
    const spurting = (w) => !w.clamped && ((s.t + w.ph) % w.P) < SPURT;
    const breath = (w) => frac((s.t + w.ph) / w.B);
    /** How full the breath is, 0 to 1: up over the in-breath, down over the out-breath. */
    const lungs = (w) => { const f = breath(w); return f < INHALE ? Math.sin((f / INHALE) * Math.PI / 2) : Math.cos(((f - INHALE) / (1 - INHALE)) * Math.PI / 2); };
    const inWork = (x, y) => x >= WORK.x && x <= WORK.x + WORK.w && y >= WORK.y && y <= WORK.y + WORK.h;
    const chartXY = (w) => [BX + w.u * S, BY + w.v * S];
    const nextOpen = () => { const i = s.wounds.findIndex((w) => !w.done); return i < 0 ? s.cur : i; };
    /** The tool this wound takes now (a bleed: the clamp, then the sealer). */
    const need = (w) => (w.kind === "bleed" ? (w.clamped ? 0 : 3) : { cut: 0, burn: 1, fracture: 2, smoke: 4 }[w.kind]);
    /** How far along the wound is, 0 to 1. */
    function progress(w) {
      if (w.done) return 1;
      const sealed = (tr) => Math.min(1, tr.count / (tr.n * SEAL_DONE));
      if (w.kind === "cut") return sealed(w.tr);
      if (w.kind === "burn") return Math.min(1, w.covN / (w.cells.length * GEL_DONE));
      if (w.kind === "fracture") return Math.min(1, w.knit);
      if (w.kind === "bleed") return w.clamped ? 0.3 + 0.7 * sealed(w.tr) : 0;
      return w.puffs / w.need;
    }

    // ---------------------------------------------------------------- effects (sparks, gel, glow, mist)
    function burst(x, y, n, col, speed, life, opts = {}) {
      for (let i = 0; i < n && s.fx.length < 500; i++) {
        const a = (opts.dir ?? 0) + (opts.spread ?? Math.PI) * (fxr() * 2 - 1), v = speed * (0.4 + 0.6 * fxr());
        s.fx.push({ x, y, vx: Math.cos(a) * v, vy: Math.sin(a) * v, life: life * (0.6 + 0.4 * fxr()), max: life, col, r: opts.r ?? 3, g: opts.grav ?? 0 });
      }
    }
    const pulse = (x, y, col, r = 40) => s.pulses.push({ x, y, col, r, t: 0 });

    // ---------------------------------------------------------------- treatment
    /** The sealer on a line this frame: seals what the tip passes over; off the band it only pauses. */
    function sealAlong(tr, p, on) {
      if (!on) { tr.lastS = null; return false; }
      const q = project(tr, p.x, p.y);
      if (q.d > BAND) { tr.lastS = null; if (p.pressed) s.hint = 1; return false; }
      let a = q.s, b = q.s;
      if (tr.lastS !== null && Math.abs(tr.lastS - q.s) < 240) { a = Math.min(a, tr.lastS); b = Math.max(b, tr.lastS); }
      tr.lastS = q.s;
      const i0 = Math.max(0, Math.floor((a - TIP) / BIN)), i1 = Math.min(tr.n - 1, Math.floor((b + TIP) / BIN));
      for (let i = i0; i <= i1; i++) if (tr.seal[i] < 0) { tr.seal[i] = s.t; tr.count++; }
      const [x, y] = at(tr, q.s);
      burst(x, y, p.pressed ? 14 : 4, fxr() < 0.5 ? "#bfefff" : C.accent, 220, 0.35, { r: 2.5 });
      s.tip = { x, y, t: 0.12 };
      if (p.pressed) pulse(x, y, C.accent, 30);
      return true;
    }
    function paintGel(w, x0, y0, x1, y1) {
      for (let i = 0; i < w.cells.length; i++) if (!w.cov[i] && segDist(w.cells[i][0], w.cells[i][1], x0, y0, x1, y1) < BRUSH) { w.cov[i] = 1; w.covN++; }
      if (!w.layer) { w.layer = document.createElement("canvas"); w.layer.width = WORK.w; w.layer.height = WORK.h; }
      const lg = w.layer.getContext("2d");
      lg.fillStyle = lg.strokeStyle = "#7fe6d2"; lg.lineCap = "round"; lg.lineWidth = BRUSH * 2;
      lg.beginPath(); lg.moveTo(x0 - WORK.x, y0 - WORK.y); lg.lineTo(x1 - WORK.x, y1 - WORK.y); lg.stroke();
      lg.beginPath(); lg.arc(x1 - WORK.x, y1 - WORK.y, BRUSH, 0, Math.PI * 2); lg.fill();
    }
    const TREAT = {
      cut(w, p, dt, on) {
        sealAlong(w.tr, p, on);
        return w.tr.count >= w.tr.n * SEAL_DONE ? "done" : null;
      },
      burn(w, p, dt, on) {
        if (!on) { w.lp = null; return null; }
        const [x0, y0] = w.lp || [p.x, p.y];
        paintGel(w, x0, y0, p.x, p.y);
        w.lp = [p.x, p.y];
        const onBurn = inBurn(w, p.x, p.y, 0);
        burst(p.x, p.y, p.pressed ? 12 : 3, onBurn ? "#a8f5e6" : "#5f8f88", 120, 0.4, { r: 4 });
        if (p.pressed) pulse(p.x, p.y, onBurn ? "#7fe6d2" : C.dim, BRUSH);
        if (p.pressed && !inBurn(w, p.x, p.y, BRUSH)) s.hint = 1;
        return w.covN >= w.cells.length * GEL_DONE ? "done" : null;
      },
      fracture(w, p, dt, on) {
        if (on && p.pressed) {
          if (segDist(p.x, p.y, ...w.E, ...boneFar(w)) < GRAB) {
            w.held = true; w.seated = false; w.gx = w.E[0] - p.x; w.gy = w.E[1] - p.y;
            burst(p.x, p.y, 12, C.lilac, 160, 0.4); pulse(p.x, p.y, C.lilac, 36);
          } else s.hint = 1;
        }
        if (!on) w.held = false;
        if (!w.held) return null;
        const tx = p.x + w.gx, ty = p.y + w.gy, d = dist(tx, ty, ...w.B);
        // The slip: pulled well out of the socket in the same hold that seated it, once knitting has started.
        if (w.seated && w.knit > 0 && d > YANK) {
          w.held = false; w.snap = false; w.seated = false; w.knit = Math.max(0, w.knit - 0.25); w.E = [tx, ty];
          burst(...w.B, 18, C.danger, 260, 0.5);
          return "slip";
        }
        const was = w.snap;
        w.snap = d < SNAP_IN || (w.snap && d < SNAP_OUT);
        w.E = w.snap ? w.B.slice() : [tx, ty];
        if (w.snap) w.seated = true;
        if (w.snap && !was) { burst(...w.B, 16, "#f3eaff", 240, 0.35); pulse(...w.B, C.lilac, 50); }
        if (w.snap) { w.knit += dt / w.knitT; burst(w.B[0], w.B[1] + (fxr() - 0.5) * 40, 2, C.lilac, 90, 0.5, { r: 3.5 }); }
        return w.knit >= 1 ? "done" : null;
      },
      bleed(w, p, dt, on) {
        if (!w.clamped) {
          if (!(on && p.pressed)) return null;
          if (spurting(w)) { burst(CX, CY, 24, C.danger, 300, 0.6, { grav: 300 }); return "slip"; }
          w.clamped = true;
          const [kx, ky] = clampXY(w);
          burst(kx, ky, 20, "#e8eef6", 260, 0.35); pulse(kx, ky, C.ok, 60);
          return null;
        }
        sealAlong(w.tr, p, on);
        return w.tr.count >= w.tr.n * SEAL_DONE ? "done" : null;
      },
      smoke(w, p, dt, on) {
        if (!(on && p.pressed)) return null;
        const n = Math.floor((s.t + w.ph) / w.B);
        if (breath(w) >= INHALE) { burst(MOUTH[0], MOUTH[1], 18, C.danger, 200, 0.5); return "slip"; }
        if (w.last === n) { s.hint = 1; return null; }   // one puff a breath: this one is taken
        w.last = n; w.puffs++; w.mist = 1;
        burst(MOUTH[0], MOUTH[1] - 30, 26, "#9fe0ff", 160, 0.7, { dir: Math.PI / 2, spread: 0.9, r: 6 });
        pulse(MOUTH[0], MOUTH[1], C.accent, 50);
        return w.puffs >= w.need ? "done" : null;
      },
    };
    const clampXY = (w) => [CX - Math.cos(w.th) * 170, CY - Math.sin(w.th) * 170];
    const BUST = [CX - 90, CY + 60];                         // the smoke close-up's chest centre
    const MOUTH = [BUST[0], BUST[1] - 222];

    /** Mouse, touch, or the pad's cursor (the stick moves it, Space presses). */
    function pointer(dt, input) {
      if (input.stick.x || input.stick.y) {
        if (!s.pad) { s.pad = true; s.vc = [s.mx >= 0 ? s.mx : CX, s.my >= 0 ? s.my : CY]; }
        s.vc = [Math.max(0, Math.min(api.W, s.vc[0] + input.stick.x * 420 * dt)), Math.max(api.BAR_H, Math.min(api.H, s.vc[1] + input.stick.y * 420 * dt))];
      }
      if (input.pressed || input.x !== s.mx || input.y !== s.my) s.pad = false;
      s.mx = input.x; s.my = input.y;
      const p = s.pad ? { x: s.vc[0], y: s.vc[1], down: input.action, pressed: input.actionPressed }
        : { x: input.x, y: input.y, down: input.down, pressed: input.pressed };
      return p;
    }

    // ---------------------------------------------------------------- drawing: chart, tools, vitals
    function figure(g) {
      const P = (u, v) => [BX + u * S, BY + v * S];
      const limbs = [[[-14, 16], [-17, 32], 5.5], [[-17, 32], [-19, 47], 4.5], [[14, 16], [17, 32], 5.5], [[17, 32], [19, 47], 4.5],
        [[-5.5, 51], [-6.5, 74], 7.5], [[-6.5, 74], [-7, 95], 5.5], [[5.5, 51], [6.5, 74], 7.5], [[6.5, 74], [7, 95], 5.5], [[0, 11], [0, 16], 5]];
      const torso = [[-13.5, 15], [13.5, 15], [12, 30], [10.5, 46], [11, 52], [-11, 52], [-10.5, 46], [-12, 30]];
      for (const [col, o] of [["#4d6683", 4], ["#1b293b", 0]]) {
        g.lineCap = "round"; g.lineJoin = "round"; g.strokeStyle = col; g.fillStyle = col;
        for (const [a, b, w] of limbs) { g.beginPath(); g.moveTo(...P(...a)); g.lineTo(...P(...b)); g.lineWidth = w * S + o; g.stroke(); }
        g.beginPath(); torso.forEach((q, i) => (i ? g.lineTo(...P(...q)) : g.moveTo(...P(...q)))); g.closePath(); g.fill();
        if (o) { g.lineWidth = o; g.stroke(); }
        D.disc(g, ...P(0, 6), 6 * S + o / 2, col);
        for (const sx of [-1, 1]) { D.disc(g, ...P(sx * 19.6, 50), 2.6 * S + o / 2, col); g.beginPath(); g.ellipse(...P(sx * 8.2, 97), 3.4 * S + o / 2, 1.6 * S + o / 2, 0, 0, Math.PI * 2); g.fill(); }
      }
      g.lineCap = "butt";
    }
    function marker(g, w, i) {
      const [x, y] = chartXY(w), k = 1.5 + 0.4 * w.size, cur = i === s.cur;
      if (cur && !w.done) { D.disc(g, x, y, 30, "rgba(242,160,70,0.18)"); D.ring(g, x, y, 28 + 3 * Math.sin(s.t * 5), C.amber, 3); }
      g.save(); g.translate(x, y);
      if (w.done) g.globalAlpha = 0.45;
      if (w.kind === "cut") { g.rotate(w.la + Math.PI / 2 + 0.5); g.strokeStyle = C.danger; g.lineWidth = 5; g.beginPath(); g.moveTo(-10 * k, 0); g.lineTo(10 * k, 0); g.stroke(); g.lineWidth = 2; for (let j = -1; j <= 1; j++) { g.beginPath(); g.moveTo(j * 6 * k, -5); g.lineTo(j * 6 * k, 5); g.stroke(); } }
      if (w.kind === "burn") { g.beginPath(); for (let j = 0; j < 9; j++) { const a = (j / 9) * Math.PI * 2, rr = 9 * k * (1 + 0.25 * Math.sin(j * 2.3)); j ? g.lineTo(Math.cos(a) * rr, Math.sin(a) * rr) : g.moveTo(rr, 0); } g.closePath(); g.fillStyle = C.amber; g.fill(); g.strokeStyle = "#5a2f0c"; g.lineWidth = 2; for (let j = -1; j <= 1; j++) { g.beginPath(); g.moveTo(-6 * k + j * 5, 6 * k); g.lineTo(6 * k + j * 5, -6 * k); g.stroke(); } }
      if (w.kind === "fracture") { g.rotate(w.la + Math.PI / 2); g.strokeStyle = C.fg; g.lineWidth = 4; g.beginPath(); g.moveTo(-11 * k, 0); g.lineTo(-4 * k, -5); g.lineTo(0, 5); g.lineTo(4 * k, -5); g.lineTo(11 * k, 0); g.stroke(); }
      if (w.kind === "bleed") { g.beginPath(); g.moveTo(0, -12 * k); g.quadraticCurveTo(9 * k, 0, 0, 8 * k); g.quadraticCurveTo(-9 * k, 0, 0, -12 * k); g.fillStyle = C.danger; g.fill(); g.strokeStyle = "#ffd0d4"; g.lineWidth = 2; g.stroke(); }
      if (w.kind === "smoke") for (const [dx, dy, rr] of [[-12, 2, 13], [0, -6, 15], [12, 3, 12], [-4, 10, 11], [8, 12, 10]]) D.disc(g, dx * k * 0.8, dy * k * 0.8, rr * k * 0.8, "rgba(160,170,184,0.75)");
      g.restore();
      if (w.done) check(g, x + 12, y - 12, 12);
    }
    /** A green disc with a tick: treated. */
    function check(g, x, y, r) {
      D.disc(g, x, y, r, C.ok);
      g.beginPath(); g.moveTo(x - r * 0.5, y); g.lineTo(x - r * 0.12, y + r * 0.42); g.lineTo(x + r * 0.55, y - r * 0.42);
      g.strokeStyle = "#0b111b"; g.lineWidth = Math.max(2.5, r * 0.25); g.lineCap = "round"; g.stroke(); g.lineCap = "butt";
    }
    function toolIcon(g, i, x, y, col, tip) {
      g.save(); g.translate(x, y); g.lineCap = "round"; g.lineJoin = "round";
      tip = tip || C.accent;
      if (i === 0) { g.rotate(-0.6); D.round(g, -30, -8, 48, 16, 6); g.fillStyle = col; g.fill(); g.fillStyle = "#1d2738"; g.fillRect(-20, -3, 26, 6); g.beginPath(); g.moveTo(18, -6); g.lineTo(32, 0); g.lineTo(18, 6); g.closePath(); g.fillStyle = tip; g.fill(); }
      if (i === 1) { g.beginPath(); g.moveTo(-26, -12); g.lineTo(14, -9); g.lineTo(14, 9); g.lineTo(-26, 12); g.closePath(); g.fillStyle = col; g.fill(); D.round(g, 14, -7, 14, 14, 3); g.fillStyle = tip === C.accent ? "#7fe6d2" : tip; g.fill(); g.fillStyle = "#3a4658"; g.fillRect(-28, -13, 5, 26); }
      if (i === 2) { g.strokeStyle = col; g.lineWidth = 7; g.beginPath(); g.moveTo(-20, -18); g.lineTo(-20, 12); g.lineTo(20, 12); g.lineTo(20, -18); g.stroke(); g.strokeStyle = tip === C.accent ? C.lilac : tip; g.lineWidth = 3; g.beginPath(); g.moveTo(-14, -16); g.lineTo(14, -16); g.stroke(); }
      if (i === 3) { g.strokeStyle = col; g.lineWidth = 4; D.ring(g, -22, 10, 7, col, 4); D.ring(g, -8, 16, 7, col, 4); g.beginPath(); g.moveTo(-17, 5); g.lineTo(24, -16); g.moveTo(-3, 11); g.lineTo(26, -10); g.stroke(); }
      if (i === 4) { D.round(g, -10, -22, 20, 34, 6); g.fillStyle = col; g.fill(); D.round(g, -4, 6, 30, 14, 5); g.fillStyle = col; g.fill(); g.fillStyle = tip; g.fillRect(-7, -28, 14, 7); }
      g.restore(); g.lineCap = "butt";
    }
    function cross(g, x, y, r, a = 1) {
      g.save(); g.globalAlpha = a; g.lineCap = "round";
      g.strokeStyle = "rgba(4,6,10,0.8)"; g.lineWidth = 10;
      g.beginPath(); g.moveTo(x - r, y - r); g.lineTo(x + r, y + r); g.moveTo(x + r, y - r); g.lineTo(x - r, y + r); g.stroke();
      g.strokeStyle = C.danger; g.lineWidth = 5; g.stroke();
      g.restore();
    }
    function vitals(g) {
      const hp = api.value, smoke = s.wounds.find((w) => w.kind === "smoke" && !w.done);
      const hr = Math.round(128 - (hp - START_HP) * 0.8 + 30 * s.flinch);
      const spo2 = Math.round(smoke ? 86 + (8 * smoke.puffs) / smoke.need : 97);
      D.panel(g, VIT.x, VIT.y, VIT.w, VIT.h, 12, "#0b1018");
      const y = VIT.y + VIT.h / 2;
      D.text(g, "HR", VIT.x + 20, y, 16, C.dim, "left", 700);
      D.text(g, String(hr), VIT.x + 56, y, 30, hr > 110 ? C.warn : C.ok, "left", 700);
      g.save(); g.beginPath(); g.rect(130, VIT.y + 6, 420, VIT.h - 12); g.clip();
      g.beginPath();
      for (let x = 130; x <= 550; x += 2) { const p = frac((s.t - (550 - x) / 140) * (hr / 60)); const yy = y + 8 - 22 * ECG(p); x === 130 ? g.moveTo(x, yy) : g.lineTo(x, yy); }
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
    function tray(g) {
      for (let i = 0; i < 5; i++) {
        const x = TRAY.x + i * (TRAY.slot + TRAY.gap), on = s.tool === i, y = TRAY.y - (on ? 4 : 0);
        D.panel(g, x, y, TRAY.slot, TRAY.h, 14, on ? "#1d2738" : "#0d131c", on ? C.amber : C.line);
        if (on) { D.round(g, x, y, TRAY.slot, TRAY.h, 14); g.lineWidth = 4; g.strokeStyle = C.amber; g.stroke(); }
        toolIcon(g, i, x + TRAY.slot / 2, y + 42, on ? "#d5dde6" : C.steel);
        D.text(g, TOOLS[i], x + TRAY.slot / 2, y + 84, 19, on ? C.fg : C.dim, "center", 700);
        // The hotkey, as a keycap in the corner.
        D.round(g, x + 10, y + 10, 26, 26, 6); g.fillStyle = on ? C.amber : "#1a2230"; g.fill();
        g.lineWidth = 1.5; g.strokeStyle = on ? C.amber : "#3a4658"; g.stroke();
        D.text(g, String(i + 1), x + 23, y + 24, 17, on ? "#0b111b" : C.dim, "center", 700);
      }
    }

    // ---------------------------------------------------------------- drawing: the close-up
    function skin(g) {
      const grd = g.createRadialGradient(CX, CY, 60, CX, CY, WORK.w * 0.62);
      grd.addColorStop(0, "#8a6553"); grd.addColorStop(1, "#4f3a31");
      g.fillStyle = grd; g.fillRect(WORK.x, WORK.y, WORK.w, WORK.h);
      g.fillStyle = "rgba(0,0,0,0.08)";
      for (let j = 0; j < 9; j++) { g.beginPath(); g.ellipse(WORK.x + 70 + j * 105, WORK.y + 60 + (j % 3) * 150, 60, 22, j * 0.7, 0, Math.PI * 2); g.fill(); }
    }
    const path = (g, pts) => { g.beginPath(); pts.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y))); };
    /** A line to seal: its working band, the raw wound, then what is sealed, glowing as it cools. */
    function drawSeal(g, tr, done, raw, edge, live) {
      g.lineCap = "round"; g.lineJoin = "round";
      if (!done) {
        path(g, tr.pts);
        const a = live ? 0.10 + 0.22 * s.hint : 0.05;
        g.strokeStyle = `rgba(79,195,247,${a})`; g.lineWidth = BAND * 2; g.stroke();
        if (live) { g.setLineDash([8, 10]); g.strokeStyle = `rgba(79,195,247,${0.25 + 0.5 * s.hint})`; g.lineWidth = 2; g.stroke(); g.setLineDash([]); }
        g.strokeStyle = raw; g.lineWidth = 14; g.stroke();
        g.strokeStyle = edge; g.lineWidth = 4; g.stroke();
      }
      for (let i = 0; i < tr.n; i++) {
        if (!done && tr.seal[i] < 0) continue;
        const [x0, y0] = at(tr, i * BIN), [x1, y1] = at(tr, Math.min(tr.len, (i + 1) * BIN + 0.5));
        g.beginPath(); g.moveTo(x0, y0); g.lineTo(x1, y1);
        g.strokeStyle = "#d9a294"; g.lineWidth = 10; g.stroke();
        const age = done ? 9 : s.t - tr.seal[i];
        if (age < 0.8) { g.strokeStyle = `rgba(160,235,255,${0.9 * (1 - age / 0.8)})`; g.lineWidth = 16; g.stroke(); }
      }
      // Stitches across the sealed length, every 22 px.
      g.strokeStyle = C.ok; g.lineWidth = 3;
      for (let sv = 11; sv < tr.len; sv += 22) {
        if (!done && tr.seal[Math.min(tr.n - 1, Math.floor(sv / BIN))] < 0) continue;
        const [x, y] = at(tr, sv), [x2, y2] = at(tr, Math.min(tr.len, sv + 2)), a = Math.atan2(y2 - y, x2 - x) + Math.PI / 2;
        g.beginPath(); g.moveTo(x - Math.cos(a) * 9, y - Math.sin(a) * 9); g.lineTo(x + Math.cos(a) * 9, y + Math.sin(a) * 9); g.stroke();
      }
      g.lineCap = "butt";
    }
    function drawBurn(g, w, live) {
      skin(g);
      g.beginPath();
      for (let j = 0; j <= 72; j++) { const th = (j / 72) * Math.PI * 2, rr = w.R * (1 + 0.16 * Math.sin(3 * th + w.a) + 0.08 * Math.sin(5 * th + w.b)); j ? g.lineTo(CX + Math.cos(th) * rr, CY + Math.sin(th) * rr) : g.moveTo(CX + rr, CY); }
      g.closePath();
      const grd = g.createRadialGradient(CX, CY, 10, CX, CY, w.R * 1.2);
      grd.addColorStop(0, w.done ? "#9b6a58" : "#d2583a"); grd.addColorStop(1, w.done ? "#86604f" : "#8e3424");
      g.fillStyle = grd; g.fill();
      g.save(); g.clip();
      if (!w.done) for (let j = 0; j < 14; j++) { const a = j * 2.4, rr = w.R * (0.15 + 0.65 * frac(j * 0.37)); D.disc(g, CX + Math.cos(a) * rr, CY + Math.sin(a) * rr * 0.9, 6 + (j % 3) * 3, "#ec9a7f"); D.ring(g, CX + Math.cos(a) * rr, CY + Math.sin(a) * rr * 0.9, 6 + (j % 3) * 3, "#7a2a1c", 1.5); }
      // Gel on the burn reads strong (it is working); gel off it is a thin, wasted film.
      if (w.layer) { g.globalAlpha = 0.45; g.drawImage(w.layer, WORK.x, WORK.y); g.globalAlpha = 1; }
      g.restore();
      if (w.layer) { g.save(); g.globalAlpha = 0.2; g.drawImage(w.layer, WORK.x, WORK.y); g.restore(); }
      // The target: a dashed outline round the burn, on top of the gel, brighter when a press missed it.
      g.setLineDash([12, 9]); g.strokeStyle = w.done ? "rgba(61,220,132,0.7)" : s.hint > 0 ? C.warn : C.amber; g.lineWidth = 3 + 3 * s.hint; g.stroke(); g.setLineDash([]);
      if (live && !w.done) { const p = cursor(); if (p && inWork(p.x, p.y)) { g.setLineDash([6, 6]); D.ring(g, p.x, p.y, BRUSH, "rgba(168,245,230,0.8)", 2); g.setLineDash([]); } }
    }
    function bone(g, x0, y0, x1, y1, col = "#d9d2bf", rim = "#8a8170") {
      g.lineCap = "round"; g.strokeStyle = rim; g.lineWidth = BONE_W + 6; g.beginPath(); g.moveTo(x0, y0); g.lineTo(x1, y1); g.stroke();
      g.strokeStyle = col; g.lineWidth = BONE_W; g.stroke(); g.lineCap = "butt";
      const a = Math.atan2(y1 - y0, x1 - x0), nx = -Math.sin(a), ny = Math.cos(a);
      D.disc(g, x0 + nx * 16, y0 + ny * 16, 27, rim); D.disc(g, x0 - nx * 16, y0 - ny * 16, 27, rim);
      D.disc(g, x0 + nx * 16, y0 + ny * 16, 24, col); D.disc(g, x0 - nx * 16, y0 - ny * 16, 24, col);
      // The break: a jagged end at (x1, y1).
      g.strokeStyle = "#5a5446"; g.lineWidth = 3; g.beginPath();
      g.moveTo(x1 + nx * 20, y1 + ny * 20); g.lineTo(x1 + nx * 7 + Math.cos(a) * 6, y1 + ny * 7 + Math.sin(a) * 6);
      g.lineTo(x1 - nx * 4 - Math.cos(a) * 4, y1 - ny * 4 - Math.sin(a) * 4); g.lineTo(x1 - nx * 20 + Math.cos(a) * 5, y1 - ny * 20 + Math.sin(a) * 5); g.stroke();
    }
    function drawFracture(g, w, live) {
      g.fillStyle = "#081420"; g.fillRect(WORK.x, WORK.y, WORK.w, WORK.h);
      g.fillStyle = "rgba(107,140,170,0.13)"; D.round(g, WORK.x + 20, CY - 130, WORK.w - 40, 260, 130); g.fill();
      bone(g, w.B[0] - 380, w.B[1], w.B[0], w.B[1]);
      if (w.done) {
        bone(g, w.B[0] + BONE, w.B[1], w.B[0], w.B[1]);
        g.strokeStyle = C.lilac; g.lineWidth = 4; g.beginPath(); g.moveTo(w.B[0], w.B[1] - 22); g.lineTo(w.B[0], w.B[1] + 22); g.stroke();
        return;
      }
      // The target: where the loose bone belongs (a dashed ghost) and its socket, the snap zone.
      const p = cursor(), near = w.held && dist(...w.E, ...w.B) < 90;
      if (!w.snap) {
        g.save(); g.setLineDash([10, 8]); D.round(g, w.B[0], w.B[1] - BONE_W / 2, BONE, BONE_W, BONE_W / 2);
        g.strokeStyle = "rgba(190,159,230,0.55)"; g.lineWidth = 2; g.stroke(); g.restore();
        D.disc(g, ...w.B, SNAP_IN, near ? "rgba(190,159,230,0.4)" : "rgba(190,159,230,0.16)");
        g.setLineDash([5, 5]); D.ring(g, ...w.B, SNAP_IN, C.lilac, near ? 3 : 2); g.setLineDash([]);
        if (!w.held) D.ring(g, ...w.B, SNAP_IN + 6 + 14 * frac(s.t * 0.8), `rgba(190,159,230,${0.6 * (1 - frac(s.t * 0.8))})`, 2);
        if (s.hint > 0) D.ring(g, ...w.B, SNAP_IN + 10 + 20 * (1 - s.hint), `rgba(190,159,230,${s.hint})`, 3);
      }
      const [fx, fy] = boneFar(w), tremor = w.snap && w.held ? 1.2 : 0;
      const ex = w.E[0] + tremor * Math.sin(s.t * 23), ey = w.E[1] + tremor * Math.cos(s.t * 19);
      const hover = live && !w.held && p && segDist(p.x, p.y, ...w.E, fx, fy) < GRAB;
      if (w.held || hover) { g.lineCap = "round"; g.beginPath(); g.moveTo(fx, fy); g.lineTo(ex, ey); g.strokeStyle = w.held ? "rgba(190,159,230,0.55)" : "rgba(190,159,230,0.3)"; g.lineWidth = BONE_W + 22; g.stroke(); g.lineCap = "butt"; }
      bone(g, fx + (ex - w.E[0]), fy + (ey - w.E[1]), ex, ey);
      // The knit: a ring round the join that fills while it is held in the socket.
      if (w.knit > 0 || w.snap) {
        D.ring(g, ...w.B, 52, "rgba(26,34,48,0.9)", 9);
        g.beginPath(); g.arc(...w.B, 52, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * Math.min(1, w.knit)); g.strokeStyle = C.lilac; g.lineWidth = 9; g.stroke();
        if (w.snap && w.held) D.ring(g, ...w.B, 30 + 6 * Math.sin(s.t * 14), "rgba(243,234,255,0.8)", 3);
      }
    }
    function drawBleed(g, w, live) {
      skin(g);
      g.save(); g.translate(CX, CY); g.rotate(w.th);
      g.strokeStyle = "#4a121b"; g.lineWidth = 54; g.beginPath(); g.moveTo(-620, 0); g.lineTo(620, 0); g.stroke();
      g.strokeStyle = "#7a1f2a"; g.lineWidth = 42; g.stroke();
      g.strokeStyle = "rgba(255,120,130,0.25)"; g.lineWidth = 6; g.beginPath(); g.moveTo(-620, -11); g.lineTo(620, -11); g.stroke();
      if (!w.done) { g.beginPath(); g.ellipse(30, 60, 170, 46, 0, 0, Math.PI * 2); g.fillStyle = "rgba(122,20,32,0.55)"; g.fill(); }
      if (w.clamped || w.done) {
        // The clamp across the vessel, upstream of the tear.
        g.fillStyle = "#c9d2dc"; g.fillRect(-176, -40, 14, 80);
        g.strokeStyle = "#c9d2dc"; g.lineWidth = 7; g.beginPath(); g.moveTo(-169, -40); g.lineTo(-220, -110); g.moveTo(-169, 40); g.lineTo(-232, -92); g.stroke();
        D.ring(g, -226, -118, 11, "#c9d2dc", 5); D.ring(g, -242, -96, 11, "#c9d2dc", 5);
      }
      g.restore();
      if (!w.clamped && !w.done) {
        // The rhythm: a hand goes round once a beat; the spurt is the hatched red arc, the gap is green.
        const R = 92, ph = ((s.t + w.ph) % w.P) / w.P, a0 = -Math.PI / 2, aS = a0 + Math.PI * 2 * (SPURT / w.P), sp = spurting(w);
        D.disc(g, CX, CY, R + 14, "rgba(4,6,10,0.35)");
        g.beginPath(); g.arc(CX, CY, R, aS + 0.04, a0 + Math.PI * 2 - 0.04); g.strokeStyle = sp ? "rgba(61,220,132,0.45)" : C.ok; g.lineWidth = sp ? 10 : 16; g.stroke();
        g.beginPath(); g.arc(CX, CY, R, a0, aS); g.strokeStyle = "rgba(255,71,87,0.35)"; g.lineWidth = 18; g.stroke();
        D.hatchArc(g, CX, CY, R - 9, R + 9, a0, aS, C.danger);
        const ah = a0 + Math.PI * 2 * ph;
        g.beginPath(); g.moveTo(CX + Math.cos(ah) * (R - 22), CY + Math.sin(ah) * (R - 22)); g.lineTo(CX + Math.cos(ah) * (R + 22), CY + Math.sin(ah) * (R + 22));
        g.strokeStyle = C.fg; g.lineWidth = 6; g.lineCap = "round"; g.stroke(); g.lineCap = "butt";
        D.disc(g, CX + Math.cos(ah) * R, CY + Math.sin(ah) * R, 8, sp ? C.danger : C.ok);
        D.ring(g, CX + Math.cos(ah) * R, CY + Math.sin(ah) * R, 8, C.fg, 2);
        // The bleed point, and the spurt itself.
        D.disc(g, CX, CY, sp ? 22 : 16, sp ? C.danger : "#a01c2a");
        if (sp) { const q0 = ((s.t + w.ph) % w.P) / SPURT; for (let j = 0; j < 22; j++) { const q = (q0 + j / 22) % 1, a = -Math.PI / 2 + w.th + ((j % 7) - 3) * 0.1; D.disc(g, CX + Math.cos(a) * q * 200, CY + Math.sin(a) * q * 200 + q * q * 120, 8 - 5 * q, C.danger); } }
      }
      if (w.clamped || w.done) drawSeal(g, w.tr, w.done, "#5a0f18", "#ff6b78", live);
    }
    function drawSmoke(g, w) {
      g.fillStyle = "#081420"; g.fillRect(WORK.x, WORK.y, WORK.w, WORK.h);
      const lv = w.done ? 0.5 : lungs(w), up = !w.done && breath(w) < INHALE, k = 1 + 0.07 * lv;
      g.save(); g.translate(...BUST); g.scale(k, 1 + 0.03 * lv);
      D.disc(g, 0, -222, 56, "#1b293b"); D.ring(g, 0, -222, 56, "#4d6683", 3);
      g.fillStyle = "#1b293b"; g.fillRect(-30, -172, 60, 40); g.strokeStyle = "#4d6683"; g.lineWidth = 3; g.strokeRect(-30, -172, 60, 40);
      g.beginPath(); g.moveTo(-230, -132); g.quadraticCurveTo(0, -180, 230, -132); g.lineTo(200, 200); g.lineTo(-200, 200); g.closePath();
      g.fillStyle = "#1b293b"; g.fill(); g.strokeStyle = up ? C.ok : "#4d6683"; g.lineWidth = up ? 4 : 3; g.stroke();
      const left = w.done ? 0 : 1 - w.puffs / w.need;
      for (const sx of [-1, 1]) {
        g.beginPath(); g.ellipse(sx * 82, 20, 68, 130, sx * 0.1, 0, Math.PI * 2); g.fillStyle = "#3a2a30"; g.fill(); g.strokeStyle = "#8a6a70"; g.lineWidth = 2; g.stroke();
        for (let j = 0; j < 6; j++) D.disc(g, sx * (66 + (j % 3) * 18), -50 + j * 34, 26, `rgba(150,160,172,${0.85 * left})`);
      }
      g.strokeStyle = "#2c3f57"; g.lineWidth = 4; for (let j = 0; j < 4; j++) { g.beginPath(); g.ellipse(0, -70 + j * 52, 170 - j * 6, 24, 0, 0.15, Math.PI - 0.15); g.stroke(); }
      g.restore();
      // The breath gauge: it climbs green with up chevrons on the in-breath (the moment), falls hatched grey on the out.
      const gx = WORK.x + WORK.w - 150, gy = WORK.y + 96, gw = 70, gh = 270;
      D.round(g, gx, gy, gw, gh, 14); g.fillStyle = "#101824"; g.fill(); g.lineWidth = 2; g.strokeStyle = C.line; g.stroke();
      const fh = (gh - 8) * lv;
      if (!w.done) {
        g.save(); D.round(g, gx + 4, gy + gh - 4 - fh, gw - 8, fh, 10); g.clip();
        g.fillStyle = up ? C.ok : "#2a3446"; g.fillRect(gx, gy, gw, gh);
        if (!up) D.hatch(g, gx, gy, gw, gh, "#4a5468");
        g.restore();
        const cy = gy + gh - 4 - fh, chevs = up ? [0, 1] : [0];
        for (const j of chevs) {
          const yy = up ? cy - 22 - j * 22 : cy + 26;
          g.beginPath();
          if (up) { g.moveTo(gx + gw / 2 - 18, yy + 10); g.lineTo(gx + gw / 2, yy - 6); g.lineTo(gx + gw / 2 + 18, yy + 10); }
          else { g.moveTo(gx + gw / 2 - 18, yy - 10); g.lineTo(gx + gw / 2, yy + 6); g.lineTo(gx + gw / 2 + 18, yy - 10); }
          g.strokeStyle = up ? C.ok : C.dim; g.lineWidth = 6; g.lineCap = "round"; g.lineJoin = "round"; g.stroke(); g.lineCap = "butt";
        }
        if (s.hint > 0) { D.round(g, gx - 4, gy - 4, gw + 8, gh + 8, 16); g.lineWidth = 4; g.strokeStyle = `rgba(232,238,246,${s.hint})`; g.stroke(); }
      }
      // A pip per puff needed.
      for (let j = 0; j < w.need; j++) {
        const px = gx + gw / 2 - ((w.need - 1) * 24) / 2 + j * 24, py = gy + gh + 30;
        if (j < w.puffs || w.done) D.disc(g, px, py, 9, C.accent); else D.ring(g, px, py, 8, "#3a4658", 3);
      }
      if (w.mist > 0) for (let j = 0; j < 16; j++) D.disc(g, MOUTH[0] + Math.sin(j * 1.7) * 30 * (1 - w.mist), MOUTH[1] - 50 + (1 - w.mist) * 140 + j * 7, 10, `rgba(159,224,255,${0.55 * w.mist})`);
    }
    function closeUp(g, w) {
      const live = !w.done && !s.waiting && s.tool === need(w);
      g.save(); D.round(g, WORK.x, WORK.y, WORK.w, WORK.h, 16); g.clip();
      if (w.kind === "cut") { skin(g); drawSeal(g, w.tr, w.done, "#5a0f18", "#ff6b78", live); }
      if (w.kind === "burn") drawBurn(g, w, live);
      if (w.kind === "fracture") drawFracture(g, w, live);
      if (w.kind === "bleed") drawBleed(g, w, live);
      if (w.kind === "smoke") drawSmoke(g, w);
      // Effects: the tool working, the press landing.
      for (const q of s.pulses) D.ring(g, q.x, q.y, q.r * (0.6 + q.t * 2), q.col, 4 * (1 - q.t / 0.35));
      for (const f of s.fx) { g.globalAlpha = Math.max(0, f.life / f.max); D.disc(g, f.x, f.y, f.r, f.col); }
      g.globalAlpha = 1;
      if (s.tip && s.tip.t > 0) { D.disc(g, s.tip.x, s.tip.y, 16, "rgba(191,239,255,0.35)"); D.disc(g, s.tip.x, s.tip.y, 7, "#f2fbff"); }
      if (s.cross && s.cross.t > 0) cross(g, s.cross.x, s.cross.y, 22, Math.min(1, s.cross.t * 2));
      g.restore();
      D.round(g, WORK.x, WORK.y, WORK.w, WORK.h, 16); g.lineWidth = 2; g.strokeStyle = C.line; g.stroke();
      // The wound's name and size, top left; how far along, top right.
      D.round(g, WORK.x + 14, WORK.y + 14, 168, 40, 20); g.fillStyle = "rgba(4,6,10,0.78)"; g.fill();
      D.text(g, NAMES[w.kind], WORK.x + 34, WORK.y + 34, 22, w.done ? C.ok : C.fg, "left", 700);
      for (let j = 0; j <= w.size; j++) D.disc(g, WORK.x + 138 + j * 13, WORK.y + 34, 4.5, w.done ? C.ok : C.amber);
      const rx = WORK.x + WORK.w - 48, ry = WORK.y + 48;
      D.disc(g, rx, ry, 32, "rgba(4,6,10,0.78)");
      if (w.done) check(g, rx, ry, 24);
      else {
        D.ring(g, rx, ry, 22, "#2a3446", 8);
        const f = progress(w);
        if (f > 0) { g.beginPath(); g.arc(rx, ry, 22, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * f); g.strokeStyle = C.ok; g.lineWidth = 8; g.stroke(); }
      }
    }
    /** Where the cursor is (the pad's or the mouse's), or null before the pointer has been seen. */
    const cursor = () => (s.pad ? { x: s.vc[0], y: s.vc[1] } : s.mx >= 0 ? { x: s.mx, y: s.my } : null);

    return {
      get state() { return s; },   // for tools: the headless checks read the round
      /** For tools: a wound of `kind` and `size`, so a shot can show every kind. */
      wound(kind, size = 1) { return wound(kind, size, KIT.rng(KIT.hash(kind + size)), 0, 27, 1.5); },
      step(index) {
        if (!s) s = { wounds: patient(api.rand()), t: 0, tool: 0, cur: 0, flinch: 0, pad: false, vc: [CX, CY], mx: -1, my: -1, index: -1, waiting: false,
          gest: null, hint: 0, cross: null, tip: null, fx: [], pulses: [] };
        if (index === s.index) s.wounds.forEach((w) => { if (!w.done) reset(w); });   // three slips: the step restarts
        s.index = index; s.waiting = false; s.gest = null;
        if (s.wounds[s.cur].done) s.cur = nextOpen();
      },
      update(dt, input) {
        s.t += dt;
        s.flinch = Math.max(0, s.flinch - dt * 1.5);
        s.hint = Math.max(0, s.hint - dt * 2);
        if (s.cross) s.cross.t -= dt;
        if (s.tip) s.tip.t -= dt;
        for (const w of s.wounds) if (w.mist) w.mist = Math.max(0, w.mist - dt * 1.2);
        for (const f of s.fx) { f.x += f.vx * dt; f.y += f.vy * dt; f.vy += f.g * dt; f.vx *= 0.96; f.vy *= 0.96; f.life -= dt; }
        s.fx = s.fx.filter((f) => f.life > 0);
        for (const q of s.pulses) q.t += dt;
        s.pulses = s.pulses.filter((q) => q.t < 0.35);
        const p = pointer(dt, input);
        for (let i = 0; i < 5; i++) if (input.hit.has("Digit" + (i + 1))) s.tool = i;
        if (input.hit.has("KeyQ")) s.tool = (s.tool + 4) % 5;
        if (input.hit.has("KeyE")) s.tool = (s.tool + 1) % 5;
        const w = s.wounds[s.cur];
        if (p.pressed) {
          s.gest = null;
          // The tray and the chart take their press; neither ever reaches the wound.
          if (p.y >= TRAY.y - 4 && p.y <= TRAY.y + TRAY.h) {
            // The slot under the finger, or the nearer one in the 11 px gap between two.
            const i = Math.floor((p.x - TRAY.x + TRAY.gap / 2) / (TRAY.slot + TRAY.gap));
            if (p.x >= TRAY.x - TRAY.gap && i >= 0 && i < 5) s.tool = i;
          } else if (p.x < CHART.x + CHART.w + 8) {
            let best = -1, bd = 40;
            s.wounds.forEach((q, i) => { const d = dist(p.x, p.y, ...chartXY(q)); if (d < bd) { bd = d; best = i; } });
            if (best >= 0 && !s.wounds[best].done && !s.waiting) { s.cur = best; s.gest = null; }
          } else if (inWork(p.x, p.y) && !w.done && !s.waiting) {
            const ok = s.tool === need(w);
            s.gest = { ok };
            if (!ok) { api.say("Wrong tool"); s.cross = { x: p.x, y: p.y, t: 0.8 }; }
          }
        }
        if (!p.down) s.gest = null;
        if (s.waiting || !w || w.done) return;
        const on = !!(s.gest && s.gest.ok && s.tool === need(w) && p.down);
        const out = TREAT[w.kind](w, p, dt, on);
        if (out === "slip") { s.flinch = 1; s.gest = null; api.fumble(FLINCH); return; }
        if (out === "done") {
          w.done = true; s.waiting = true; s.gest = null; api.stepDone();
          burst(CX, CY, 30, C.ok, 320, 0.6); pulse(CX, CY, C.ok, 120);
        }
      },
      draw(g) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        vitals(g);
        const shake = s.flinch > 0 ? Math.sin(s.t * 50) * 5 * s.flinch : 0;
        D.panel(g, CHART.x, CHART.y, CHART.w, CHART.h, 16, "#0b1018");
        g.save(); g.translate(shake, 0); figure(g); s.wounds.forEach((w, i) => marker(g, w, i)); g.restore();
        const w = s.wounds[s.cur];
        // A leader from the chosen wound on the chart to its close-up.
        const [mx, my] = chartXY(w);
        g.strokeStyle = "rgba(242,160,70,0.45)"; g.lineWidth = 2; g.setLineDash([6, 6]);
        g.beginPath(); g.moveTo(mx + 30, my); g.lineTo(WORK.x, WORK.y + 34); g.stroke(); g.setLineDash([]);
        g.save(); g.translate(shake * 0.5, 0); closeUp(g, w); g.restore();
        tray(g);
        // The tool in hand at the cursor over the close-up: bright when it works here, greyed with a red cross when not.
        const p = cursor();
        if (p && inWork(p.x, p.y) && !w.done && !s.waiting) {
          const ok = s.tool === need(w);
          g.save(); g.translate(p.x + 30, p.y - 30); g.scale(0.85, 0.85);
          D.disc(g, 0, 0, 34, ok ? "rgba(4,6,10,0.35)" : "rgba(4,6,10,0.6)");
          toolIcon(g, s.tool, 0, 0, ok ? "#e8eef6" : "#4a5466", ok ? C.accent : "#4a5466");
          g.restore();
          if (!ok) cross(g, p.x, p.y, 13);
          else D.ring(g, p.x, p.y, 6, C.fg, 2);
        }
        if (s.pad && p) { g.strokeStyle = C.fg; g.lineWidth = 2; g.beginPath(); g.moveTo(p.x - 14, p.y); g.lineTo(p.x + 14, p.y); g.moveTo(p.x, p.y - 14); g.lineTo(p.x, p.y + 14); g.stroke(); }
      },
    };
  },
});
