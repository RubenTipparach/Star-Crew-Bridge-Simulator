/*
 * repairs/kit.js: the repair mini-games' shared kit (openspec/changes/repair-minigames, design sections 1 and 6).
 *
 * It owns what every game shares, so no game re-implements it (CLAUDE.md 6.1):
 * - the job: integrity from where the damage left it to 100%, filled at the repairer's rate (damage-control 6a: an
 *   officer 1.8% a second, a rating 0.6%), cut into steps; a step completes only when its round is played AND its
 *   share has filled at the rate, so no play beats the board's preview (design 1);
 * - fumbles: 5% of the job back and the game's hazard; three in a step restart it;
 * - input: pointer (mouse or touch) in canvas pixels, keys, a "stick" from the arrows or W A S D, one action button
 *   (Space or Enter), and the wheel; every action has a pointer path, so a phone plays with touch alone (keys and the
 *   wheel are a pad's and a desk's extras), and only the finger that started a press drives it;
 * - touch targets: TOUCH_R, the smallest reach a target is given, and nearest(), which picks the closest of packed
 *   targets, so a fingertip on a phone (one canvas px is about half a CSS px there) still lands;
 * - the frame: a 1280 x 720 canvas, the job's bar along the top 72 px, the combat shake;
 * - a seeded random stream per game and step, so a shot is the same every time.
 *
 * A game is a classic script that calls RepairKit.register({...}) (no modules: the page opens from disk). See
 * register() for the fields. A game draws into the area below the bar (y >= 80) and calls api.stepDone() when its round
 * is played, api.fumble("what happened") on a mistake.
 */
(function () {
  "use strict";
  const W = 1280, H = 720, BAR_H = 72;
  const RATES = { officer: 1.8, rating: 0.6 };          // % of integrity a second (damage-control 6a)
  const FUMBLE_SHARE = 5;                                 // % of the job (design 1; damage.json repair.fumble_share)
  const FUMBLES_PER_STEP = 3;
  const STATES = {                                        // design 2: steps by the job's state
    damaged: { start: 25, steps: 3, label: "Damaged" },
    disabled: { start: 12, steps: 4, part: true, label: "Disabled" },
  };
  const C = {
    bg: "#04060a", panel: "#0c121a", panel2: "#121a25", line: "#1f2a37", fg: "#e8eef6", dim: "#6f7f94",
    ok: "#3ddc84", warn: "#ffc542", danger: "#ff4757", accent: "#4fc3f7", amber: "#f2a046", lilac: "#be9fe6",
    steel: "#7d8796", copper: "#d08a4a",
  };
  const FONT = '"Barlow Semi Condensed", "Arial Narrow", "Roboto Condensed", sans-serif';

  /** A seeded stream (mulberry32): the same game and step draw the same numbers. */
  function rng(seed) {
    let a = seed >>> 0;
    return () => { a = (a + 0x6d2b79f5) >>> 0; let t = a; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
  }
  function hash(s) { let h = 2166136261; for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 16777619); return h >>> 0; }

  /**
   * The smallest hit reach a touch target gets, canvas px: on a phone held sideways the canvas is about 690 CSS px wide,
   * so 30 canvas px is a 16 CSS px fingertip either side of the target. A game widens the hit zone, never the drawing.
   */
  const TOUCH_R = 30;
  /**
   * The nearest target to (x, y) within reach `r`, or -1. `pts` holds [x, y] pairs or { x, y } objects; a null entry
   * (a target that is done or hidden) is skipped. Packed targets closer than two reaches are told apart by distance,
   * so each one's zone reaches to halfway to its neighbour and no further.
   */
  function nearest(pts, x, y, r = TOUCH_R) {
    let best = -1, bd = r;
    pts.forEach((p, i) => {
      if (!p) return;
      const d = Math.hypot(x - (p.x ?? p[0]), y - (p.y ?? p[1]));
      if (d < bd) { bd = d; best = i; }
    });
    return best;
  }

  const games = [];
  const KIT = {
    W, H, BAR_H, C, FONT, RATES, rng, hash, TOUCH_R, nearest,
    /**
     * Register a game:
     *   id: short id (the URL hash), title: the system's name, place: where its repair point is,
     *   group: the menu's column, hazard: what a fumble does, down: what the ship loses while it is down,
     *   panel (optional): { x, y, w, h, screws } a cover plate over the machine, unscrewed before the work and screwed
 *     back after it (4 screws at the corners, or 6),
 *   doneWord (optional): the word when the job is done ("Treated"; default "Repaired"),
 *   job (optional): { unit: "%" | "HP", start, target, rate or rateBy: { officer, rating }, steps, fumble } to
 *     override the integrity job (the medic: HP, its rates and a 2 HP slip),
     *   create(api): returns { draw(g, t, dt), update?(dt, input), step?(index, part) }:
     *     step(index, part) starts round `index` (part: this round fits the spare part, design 2);
     *     update(dt, input) runs the round; draw(g, t, dt) draws it below the bar.
     */
    register(game) { games.push(game); },
    games,
    /** Common drawing helpers, so the games look like one set. */
    draw: {
      text(g, s, x, y, size = 20, color = C.fg, align = "left", weight = 600) {
        g.font = `${weight} ${size}px ${FONT}`; g.fillStyle = color; g.textAlign = align; g.textBaseline = "middle"; g.fillText(s, x, y);
      },
      round(g, x, y, w, h, r) { g.beginPath(); g.roundRect(x, y, w, h, r); },
      panel(g, x, y, w, h, r = 18, fill = C.panel, stroke = C.line) {
        KIT.draw.round(g, x, y, w, h, r); g.fillStyle = fill; g.fill(); g.lineWidth = 2; g.strokeStyle = stroke; g.stroke();
      },
      ring(g, x, y, r, color, w = 4) { g.beginPath(); g.arc(x, y, r, 0, Math.PI * 2); g.strokeStyle = color; g.lineWidth = w; g.stroke(); },
      disc(g, x, y, r, color) { g.beginPath(); g.arc(x, y, r, 0, Math.PI * 2); g.fillStyle = color; g.fill(); },
      /** Diagonal hatching over a rectangle: a stop, a red zone, a gap (colour always with a shape, CLAUDE.md 10). */
      hatch(g, x, y, w, h, color) {
        g.save(); g.beginPath(); g.rect(x, y, w, h); g.clip();
        g.strokeStyle = color; g.lineWidth = 3;
        for (let i = -h; i < w; i += 12) { g.beginPath(); g.moveTo(x + i, y + h); g.lineTo(x + i + h, y); g.stroke(); }
        g.restore();
      },
      /** Diagonal hatching over a band of a ring between radii r0 and r1 and angles a0 and a1 (a gauge's red zone). */
      hatchArc(g, x, y, r0, r1, a0, a1, color) {
        g.save(); g.beginPath(); g.arc(x, y, r1, a0, a1); g.arc(x, y, r0, a1, a0, true); g.closePath(); g.clip();
        g.strokeStyle = color; g.lineWidth = 3;
        for (let i = -300; i < 300; i += 11) { g.beginPath(); g.moveTo(x + i, y + 40); g.lineTo(x + i + 160, y - 160); g.stroke(); }
        g.restore();
      },
      /** A curved arrow along a rim from a0 to a1: the way a ring or a crank turns. */
      turnArrow(g, x, y, r, a0, a1, color) {
        g.beginPath(); g.arc(x, y, r, a0, a1); g.strokeStyle = color; g.lineWidth = 4; g.stroke();
        const hx = x + Math.cos(a1) * r, hy = y + Math.sin(a1) * r, d = a1 + Math.PI / 2;
        g.beginPath(); g.moveTo(hx + Math.cos(d) * 12, hy + Math.sin(d) * 12);
        g.lineTo(hx + Math.cos(a1) * 9, hy + Math.sin(a1) * 9); g.lineTo(hx - Math.cos(a1) * 9, hy - Math.sin(a1) * 9);
        g.closePath(); g.fillStyle = color; g.fill();
      },
      /**
       * A fastener's place in its order: its number beside it, and a bright pulsing ring on the one that comes next, at
       * rest (owner, 2026-10-08: "needs some way to tell me what the next screw is"). `r` is the fastener's radius.
       */
      orderBadge(g, x, y, r, n, next, t) {
        if (next) {
          KIT.draw.ring(g, x, y, r + 9 + 2 * Math.sin(t * 6), C.accent, 3);
          KIT.draw.disc(g, x, y, r + 5, "rgba(79,195,247,0.18)");
        }
        const br = Math.max(13, r * 0.8), bx = x + r + br - 2, by = y - r - br + 4;
        KIT.draw.disc(g, bx, by, br, next ? C.accent : "#1d2738");
        KIT.draw.ring(g, bx, by, br, next ? "#e8f7ff" : "#4a5568", 1.5);
        KIT.draw.text(g, String(n), bx, by + 1, Math.round(br * 1.3), next ? "#04131c" : C.fg, "center", 700);
      },
      /** A button: returns true when the pointer pressed it this frame. */
      button(g, input, label, x, y, w, h, opts = {}) {
        const over = input.x >= x && input.x <= x + w && input.y >= y && input.y <= y + h;
        KIT.draw.round(g, x, y, w, h, h / 2);
        g.fillStyle = opts.fill || (over ? "#46527a" : "#262e42"); g.fill();
        if (opts.on) { g.lineWidth = 3; g.strokeStyle = C.amber; g.stroke(); }
        KIT.draw.text(g, label, x + w / 2, y + h / 2, opts.size || 20, opts.color || C.fg, "center", 700);
        return over && input.pressed;
      },
    },
  };
  window.RepairKit = KIT;

  // ------------------------------------------------------------------ access panels (screws)
  // The owner, 2026-10-08: "some of the panel ones would be cool to like screw or unscrew stuff". A game that names a
  // `panel` ({ x, y, w, h, screws }) has its machine behind a cover plate: the job opens by unscrewing it (turn each
  // screw anticlockwise: drag round it, roll the wheel over it, or hold Left; Tab picks the next) and ends by screwing
  // it back (clockwise, Right). The plate hides the machine until it is off. Nothing here is a fumble: a screw only
  // turns while the pointer goes round it.
  const SCREW_TURNS = 2.5;                      // turns to free a screw, and to drive it home
  const SCREW_R = 15;                           // a screw head's radius, px
  function screwPanel(spec) {
    const n = spec.screws || 4, m = 26, { x, y, w, h } = spec;
    const spots = n >= 6
      ? [[x + m, y + m], [x + w / 2, y + m], [x + w - m, y + m], [x + w - m, y + h - m], [x + w / 2, y + h - m], [x + m, y + h - m]]
      : [[x + m, y + m], [x + w - m, y + m], [x + w - m, y + h - m], [x + m, y + h - m]];
    return { x, y, w, h, phase: "shut", lift: 0, sel: 0, grab: -1, prevA: 0,
      screws: spots.slice(0, n).map(([sx, sy]) => ({ x: sx, y: sy, turn: 0, rot: 0 })) };
  }
  KIT.screwPanel = screwPanel;
  /** The star (cross) order to work n fasteners round a rim, from the first: across, then round. */
  const STARS = { 4: [0, 2, 1, 3], 6: [0, 3, 1, 4, 2, 5], 8: [0, 4, 2, 6, 1, 5, 3, 7] };
  KIT.starOrder = (n) => (STARS[n] || [...Array(n).keys()]).slice();
  /** Run a panel a frame: `dir` -1 unscrews (anticlockwise), +1 drives home. Returns true when every screw is done. */
  function turnScrews(p, input, dt, dir) {
    const goal = SCREW_TURNS * Math.PI * 2;
    const left = p.screws.filter((sc) => sc.turn < goal);
    if (!left.length) return true;
    if (input.hit.has("Tab")) { const i = p.screws.indexOf(left.find((sc) => p.screws.indexOf(sc) > p.sel) || left[0]); p.sel = i; }
    if (p.screws[p.sel].turn >= goal) p.sel = p.screws.indexOf(left[0]);
    const near = (sc) => Math.hypot(input.x - sc.x, input.y - sc.y) < SCREW_R + 22;
    if (input.pressed) { const i = nearest(p.screws.map((sc) => (sc.turn < goal ? sc : null)), input.x, input.y, SCREW_R + 22); if (i >= 0) { p.grab = i; p.sel = i; p.prevA = null; } }
    if (!input.down) p.grab = -1;
    const turn = (sc, d) => { const k = Math.max(0, d * dir); sc.turn = Math.min(goal, sc.turn + k); sc.rot += d; };
    if (p.grab >= 0) {
      const sc = p.screws[p.grab], dx = input.x - sc.x, dy = input.y - sc.y;
      if (Math.hypot(dx, dy) > 6) {
        const a = Math.atan2(dy, dx);
        if (p.prevA !== null) { let d = a - p.prevA; d = Math.atan2(Math.sin(d), Math.cos(d)); turn(sc, d); }
        p.prevA = a;
      }
    }
    if (input.wheel) { const sc = p.screws.find((q) => q.turn < goal && near(q)) || p.screws[p.sel]; turn(sc, -input.wheel * 0.7); }
    const key = (input.keys.has("ArrowRight") || input.keys.has("KeyD") ? 1 : 0) - (input.keys.has("ArrowLeft") || input.keys.has("KeyA") ? 1 : 0);
    if (key) turn(p.screws[p.sel], key * 9 * dt);
    return p.screws.every((sc) => sc.turn >= goal);
  }
  function drawPanel(g, p, t) {
    const D = KIT.draw, goal = SCREW_TURNS * Math.PI * 2;
    if (p.lift >= 1) return;
    g.save();
    g.translate(0, -p.lift * (p.h + 120));
    g.globalAlpha = 1 - p.lift * 0.6;
    // The plate: brushed steel, a stencil, vent slots, a drop shadow.
    g.fillStyle = "rgba(0,0,0,0.45)"; D.round(g, p.x + 8, p.y + 10, p.w, p.h, 12); g.fill();
    const grd = g.createLinearGradient(p.x, p.y, p.x + p.w, p.y + p.h);
    grd.addColorStop(0, "#3a4556"); grd.addColorStop(1, "#262f3d");
    D.round(g, p.x, p.y, p.w, p.h, 12); g.fillStyle = grd; g.fill(); g.lineWidth = 3; g.strokeStyle = "#556275"; g.stroke();
    g.strokeStyle = "rgba(255,255,255,0.04)"; g.lineWidth = 1;
    for (let yy = p.y + 6; yy < p.y + p.h; yy += 5) { g.beginPath(); g.moveTo(p.x + 6, yy); g.lineTo(p.x + p.w - 6, yy); g.stroke(); }
    const vs = Math.min(6, Math.floor((p.w - 120) / 40));
    for (let i = 0; i < vs; i++) { D.round(g, p.x + p.w / 2 - (vs * 40) / 2 + i * 40 + 8, p.y + p.h - 64, 24, 30, 6); g.fillStyle = "#151b25"; g.fill(); }
    D.hatch(g, p.x + 60, p.y + 50, Math.min(140, p.w - 120), 14, "rgba(242,160,70,0.55)");
    for (const [i, sc] of p.screws.entries()) {
      const k = Math.min(1, sc.turn / goal), out = p.phase === "open" ? k : 1 - k;
      // A screw backs out as it turns: it stands proud, its shadow grows.
      D.disc(g, sc.x + 3 + 5 * out, sc.y + 4 + 5 * out, SCREW_R + 1, "rgba(0,0,0,0.5)");
      D.disc(g, sc.x, sc.y, SCREW_R + 2 * out, out > 0.98 && p.phase === "open" ? "#6d7686" : "#b7c0cc");
      D.ring(g, sc.x, sc.y, SCREW_R + 2 * out, "#7a8494", 2);
      g.save(); g.translate(sc.x, sc.y); g.rotate(sc.rot);
      g.strokeStyle = "#2a313c"; g.lineWidth = 4; g.beginPath(); g.moveTo(-8, 0); g.lineTo(8, 0); g.moveTo(0, -8); g.lineTo(0, 8); g.stroke();
      g.restore();
      if (sc.turn < goal) {
        g.beginPath(); g.arc(sc.x, sc.y, SCREW_R + 9, -Math.PI / 2, -Math.PI / 2 + (Math.PI * 2 * sc.turn) / goal);
        g.strokeStyle = C.amber; g.lineWidth = 4; g.stroke();
        if (i === p.sel) D.ring(g, sc.x, sc.y, SCREW_R + 15 + Math.sin(t * 5) * 2, "rgba(232,238,246,0.35)", 2);
      } else D.ring(g, sc.x, sc.y, SCREW_R + 9, C.ok, 3);
    }
    // Which way they turn, once, by the selected screw: a turn arrow (anticlockwise off, clockwise on).
    const sc = p.screws[p.sel];
    if (sc && sc.turn < goal) {
      const dir = p.phase === "open" ? -1 : 1;
      D.turnArrow(g, sc.x, sc.y, SCREW_R + 26, dir < 0 ? 0.2 : -1.6, dir < 0 ? -1.6 : 0.2, "rgba(232,238,246,0.6)");
    }
    g.restore();
  }

  // ------------------------------------------------------------------ the runner (the page calls RepairKit.start)
  KIT.start = function (canvas, ui) {
    const g = canvas.getContext("2d");
    const input = { x: -1, y: -1, down: false, pressed: false, released: false, keys: new Set(), hit: new Set(), wheel: 0,
      stick: { x: 0, y: 0 }, action: false, actionPressed: false };
    const opts = { who: "officer", state: "damaged", combat: false };
    let run = null;
    const toCanvas = (e) => { const r = canvas.getBoundingClientRect(); return [(e.clientX - r.left) * (W / r.width), (e.clientY - r.top) * (H / r.height)]; };
    // The canvas owns the pointer: no native drag of the canvas, no text selection, no page pan, no menu, so a drag in a
    // game never drags the frame instead (owner, 2026-10-08: "the tools dont respond well and just end up dragging the
    // frame").
    canvas.draggable = false;
    canvas.style.userSelect = "none";
    canvas.style.webkitUserSelect = "none";
    canvas.style.touchAction = "none";
    for (const ev of ["dragstart", "selectstart", "contextmenu"]) canvas.addEventListener(ev, (e) => e.preventDefault());
    // One pointer at a time: the one that started the press owns it until it lifts, so a palm or a second finger can
    // neither move a drag nor end it. With nothing pressed, any pointer's move is the hover (a mouse's).
    let owner = null;
    const foreign = (e) => input.down && e.pointerId !== owner;
    canvas.addEventListener("pointermove", (e) => { if (foreign(e)) return; [input.x, input.y] = toCanvas(e); if (input.down) e.preventDefault(); });
    canvas.addEventListener("pointerdown", (e) => {
      e.preventDefault();
      if (foreign(e)) return;
      owner = e.pointerId;
      [input.x, input.y] = toCanvas(e); input.down = true; input.pressed = true;
      try { canvas.setPointerCapture(e.pointerId); } catch (_) { /* a synthetic event has no pointer to capture */ }
    });
    // A tap quicker than a frame (down and up between two frames) still reads as held for the one frame that sees its
    // press, so a game that acts on "pressed while down" (the medic's clamp and inhaler) takes it; it lifts the frame after.
    let lift = false;
    const up = (e) => {
      if (foreign(e)) return;
      [input.x, input.y] = toCanvas(e); owner = null;
      if (input.down && input.pressed) { lift = true; return; }
      if (input.down) input.released = true;
      input.down = false;
    };
    canvas.addEventListener("pointerup", up);
    canvas.addEventListener("pointercancel", up);
    canvas.addEventListener("lostpointercapture", (e) => { if (foreign(e)) return; if (input.down) { input.down = false; input.released = true; } owner = null; });
    canvas.addEventListener("wheel", (e) => { input.wheel += Math.sign(e.deltaY); e.preventDefault(); }, { passive: false });
    addEventListener("keydown", (e) => { if (!input.keys.has(e.code)) input.hit.add(e.code); input.keys.add(e.code); if (["Space", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"].includes(e.code)) e.preventDefault(); });
    addEventListener("keyup", (e) => input.keys.delete(e.code));

    /** Open game `id` with the current options. */
    function open(id) {
      const game = games.find((x) => x.id === id) || games[0];
      if (!game) return;
      const st = STATES[opts.state];
      // A job a game overrides (the medic's HP) keeps its own steps and never has a part step.
      const job = game.job
        ? Object.assign({ unit: "%", start: st.start, target: 100, rate: RATES[opts.who], part: false }, game.job)
        : { unit: "%", start: st.start, target: 100, rate: RATES[opts.who], steps: st.steps + (st.part ? 1 : 0), part: !!st.part };
      if (game.job && game.job.rateBy) job.rate = game.job.rateBy[opts.who];
      run = { game, job, value: job.start, step: 0, fumbles: 0, fumblesTotal: 0, roundDone: false, done: false, t: 0, flash: 0, note: "", noteT: 0, shake: 0 };
      const share = () => (job.target - job.start) / job.steps;
      // Never past the target: after the last step the cover still has to go back on, and the bar holds at 100%.
      const cap = () => Math.min(job.target, job.start + share() * (run.step + 1));
      run.api = {
        W, H, BAR_H, C, KIT,
        /** This game's and step's random stream. */
        rand: () => rng(hash(game.id + ":" + run.step + ":" + opts.state)),
        /** The round is played: the step completes once its share has filled at the rate. */
        stepDone() { run.roundDone = true; },
        /** A mistake: 5% of the job back and the hazard said. */
        fumble(what) {
          // Back at most one share below where the job started, and never below empty (a disabled job starts at 12%).
          run.value = Math.max(0, job.start - share(), run.value - (job.fumble ?? FUMBLE_SHARE));
          run.fumbles++; run.fumblesTotal++; run.flash = 1; run.shake = Math.max(run.shake, 0.6);
          run.note = what || game.hazard; run.noteT = 2.5;
          if (run.fumbles >= FUMBLES_PER_STEP) { run.fumbles = 0; run.roundDone = false; run.note = "Step restarted"; run.noteT = 2; startStep(); }
        },
        /** Say something on the bar briefly (an event, not instructions). */
        say(what) { run.note = what; run.noteT = 2; },
        get step() { return run.step; },
        /** The job's value now: integrity in %, or the patient's HP. */
        get value() { return run.value; },
        get steps() { return job.steps; },
        get part() { return job.part && run.step === 0; },
        get combat() { return opts.combat; },
        get who() { return opts.who; },
      };
      run.inst = game.create(run.api);
      run.panel = game.panel ? Object.assign(screwPanel(game.panel), { phase: "open" }) : null;
      function startStep() { if (run.inst.step) run.inst.step(run.step, job.part && run.step === 0); }
      run.startStep = startStep;
      run.cap = cap;
      // A rating (or a bot) repairs with no game (design 1): the bar fills at its rate, nothing to play.
      run.auto = opts.who === "rating" && !game.job;
      startStep();
      if (ui && ui.onOpen) ui.onOpen(game, run);
      location.hash = game.id;
    }

    function update(dt) {
      if (!run) return;
      input.stick.x = (input.keys.has("ArrowRight") || input.keys.has("KeyD") ? 1 : 0) - (input.keys.has("ArrowLeft") || input.keys.has("KeyA") ? 1 : 0);
      input.stick.y = (input.keys.has("ArrowDown") || input.keys.has("KeyS") ? 1 : 0) - (input.keys.has("ArrowUp") || input.keys.has("KeyW") ? 1 : 0);
      input.action = input.keys.has("Space") || input.keys.has("Enter");
      input.actionPressed = input.hit.has("Space") || input.hit.has("Enter");
      const job = run.job;
      run.t += dt;
      if (!run.done) {
        // The bar fills at the rate up to the current step's share (design 1).
        if (run.value < run.cap()) run.value = Math.min(run.cap(), run.value + job.rate * dt);
        const P = run.panel;
        if (run.auto) { run.roundDone = true; if (P) P.lift = 1; }
        else if (P && P.phase !== "work") {
          // The cover: off before the work (anticlockwise), back on after it (clockwise).
          if (P.phase === "open" && turnScrews(P, input, dt, -1)) { P.phase = "lifting"; }
          if (P.phase === "lifting") { P.lift = Math.min(1, P.lift + dt * 2.5); if (P.lift >= 1) P.phase = "work"; }
          if (P.phase === "lowering") { P.lift = Math.max(0, P.lift - dt * 2.5); if (P.lift <= 0) { P.phase = "close"; P.sel = 0; } }
          if (P.phase === "close" && turnScrews(P, input, dt, 1)) { run.done = true; run.note = run.game.doneWord || "Repaired"; run.noteT = 99; }
        } else if (run.inst.update) run.inst.update(dt, input);
        if (run.roundDone && run.value >= run.cap() - 1e-6) {
          run.roundDone = false; run.fumbles = 0; run.step++;
          if (run.step >= job.steps) {
            run.value = job.target;
            if (P && !run.auto) { P.phase = "lowering"; P.screws.forEach((sc) => { sc.turn = 0; }); }
            else { run.done = true; run.note = run.game.doneWord || "Repaired"; run.noteT = 99; }
          } else run.startStep();
        }
      }
      run.flash = Math.max(0, run.flash - dt * 2.5);
      run.noteT -= dt;
      run.shake = Math.max(0, run.shake - dt * 1.5);
    }

    function drawBar() {
      const D = KIT.draw, job = run.job;
      g.fillStyle = C.panel2; g.fillRect(0, 0, W, BAR_H);
      g.fillStyle = C.line; g.fillRect(0, BAR_H - 2, W, 2);
      D.text(g, run.game.title.toUpperCase(), 24, 26, 26, C.amber, "left", 700);
      D.text(g, run.game.place, 24, 52, 16, C.dim, "left", 500);
      // The job: integrity (or HP) now, the step's cap as a ghost, the target.
      const x0 = 380, x1 = 1000, y = 22, h = 22;
      const f = (v) => x0 + (x1 - x0) * Math.max(0, Math.min(1, v / Math.max(job.target, 100)));
      D.round(g, x0, y, x1 - x0, h, 11); g.fillStyle = "#1a2230"; g.fill();
      D.round(g, x0, y, f(run.cap()) - x0, h, 11); g.fillStyle = "rgba(79,195,247,0.18)"; g.fill();
      const good = run.value >= 75 ? C.ok : run.value >= 25 ? C.warn : C.danger;
      D.round(g, x0, y, Math.max(h, f(run.value) - x0), h, 11); g.fillStyle = run.flash > 0 ? C.danger : good; g.fill();
      D.text(g, `${Math.round(run.value)}${job.unit === "%" ? "%" : " HP"}`, x1 + 14, y + h / 2, 22, C.fg, "left", 700);
      // Steps as dots; the time left at the rate (the board's preview, damage::repair_time).
      for (let i = 0; i < job.steps; i++) {
        const cx = x0 + 10 + i * 26, cy = 58;
        D.disc(g, cx, cy, 7, i < run.step ? C.ok : i === run.step && !run.done ? C.amber : "#2a3446");
      }
      const left = Math.max(0, (job.target - run.value) / job.rate);
      // The time gives way while a note is up, so a long note never overlaps it.
      if (!run.done && run.noteT <= 0) D.text(g, `${left.toFixed(0)} s`, x1 + 14, 58, 16, C.dim, "left", 600);
      // Fumbles this step as three ticks.
      for (let i = 0; i < FUMBLES_PER_STEP; i++) { g.fillStyle = i < run.fumbles ? C.danger : "#2a3446"; g.fillRect(1140 + i * 20, 18, 12, 26); }
      if (run.noteT > 0) D.text(g, run.note, 1240, 58, 16, run.done ? C.ok : C.danger, "right", 700);
    }

    let last = performance.now();
    function frame(now) {
      const dt = Math.min(0.05, (now - last) / 1000); last = now;
      update(dt);
      g.save();
      g.fillStyle = C.bg; g.fillRect(0, 0, W, H);
      if (run) {
        // The combat shake: the ship lurching under fire (ship-frames), and a fumble's jolt.
        const s = (opts.combat ? 5 + 3 * Math.sin(run.t * 1.7) : 0) + run.shake * 10;
        if (s > 0) g.translate((Math.random() - 0.5) * s, (Math.random() - 0.5) * s);
        g.save(); g.beginPath(); g.rect(0, BAR_H, W, H - BAR_H); g.clip();
        if (run.auto) {
          KIT.draw.text(g, "RATING AT WORK", W / 2, H / 2, 40, C.dim, "center", 700);
        } else run.inst.draw(g, run.t, dt, input);
        if (run.panel && !run.auto) drawPanel(g, run.panel, run.t);
        g.restore();
        if (run.done) { g.fillStyle = "rgba(4,6,10,0.55)"; g.fillRect(0, BAR_H, W, H - BAR_H); KIT.draw.text(g, (run.game.doneWord || "Repaired").toUpperCase(), W / 2, H / 2, 64, C.ok, "center", 700); }
        if (run.flash > 0) { g.fillStyle = `rgba(255,71,87,${0.25 * run.flash})`; g.fillRect(0, BAR_H, W, H - BAR_H); }
        drawBar();
      }
      g.restore();
      input.pressed = false; input.released = false; input.hit.clear(); input.wheel = 0;
      if (lift) { lift = false; input.down = false; input.released = true; }
      requestAnimationFrame(frame);
    }
    requestAnimationFrame(frame);
    return {
      open, opts,
      /** For shots: run the open game `seconds` of simulated time with no input. */
      advance(seconds) { const n = Math.round(seconds * 60); for (let i = 0; i < n; i++) update(1 / 60); },
      get run() { return run; },
      input,
    };
  };
})();
