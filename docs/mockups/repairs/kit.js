/*
 * repairs/kit.js: the repair mini-games' shared kit (openspec/changes/repair-minigames, design sections 1 and 6).
 *
 * It owns what every game shares, so no game re-implements it (CLAUDE.md 6.1):
 * - the job: integrity from where the damage left it to 100%, filled at the repairer's rate (damage-control 6a: an
 *   officer 1.8% a second, a rating 0.6%), cut into steps; a step completes only when its round is played AND its
 *   share has filled at the rate, so no play beats the board's preview (design 1);
 * - fumbles: 5% of the job back and the game's hazard; three in a step restart it;
 * - input: pointer (mouse or touch) in canvas pixels, keys, a "stick" from the arrows or W A S D, one action button
 *   (Space or Enter), and the wheel;
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

  const games = [];
  const KIT = {
    W, H, BAR_H, C, FONT, RATES, rng, hash,
    /**
     * Register a game:
     *   id: short id (the URL hash), title: the system's name, place: where its repair point is,
     *   group: the menu's column, hazard: what a fumble does, down: what the ship loses while it is down,
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

  // ------------------------------------------------------------------ the runner (the page calls RepairKit.start)
  KIT.start = function (canvas, ui) {
    const g = canvas.getContext("2d");
    const input = { x: -1, y: -1, down: false, pressed: false, released: false, keys: new Set(), hit: new Set(), wheel: 0,
      stick: { x: 0, y: 0 }, action: false, actionPressed: false };
    const opts = { who: "officer", state: "damaged", combat: false };
    let run = null;
    const toCanvas = (e) => { const r = canvas.getBoundingClientRect(); return [(e.clientX - r.left) * (W / r.width), (e.clientY - r.top) * (H / r.height)]; };
    canvas.addEventListener("pointermove", (e) => { [input.x, input.y] = toCanvas(e); });
    canvas.addEventListener("pointerdown", (e) => { [input.x, input.y] = toCanvas(e); input.down = true; input.pressed = true; canvas.setPointerCapture(e.pointerId); });
    canvas.addEventListener("pointerup", (e) => { [input.x, input.y] = toCanvas(e); input.down = false; input.released = true; });
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
      const cap = () => job.start + share() * (run.step + 1);
      run.api = {
        W, H, BAR_H, C, KIT,
        /** This game's and step's random stream. */
        rand: () => rng(hash(game.id + ":" + run.step + ":" + opts.state)),
        /** The round is played: the step completes once its share has filled at the rate. */
        stepDone() { run.roundDone = true; },
        /** A mistake: 5% of the job back and the hazard said. */
        fumble(what) {
          run.value = Math.max(job.start - share(), run.value - (job.fumble ?? FUMBLE_SHARE));
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
        if (run.auto) run.roundDone = true;
        else if (run.inst.update) run.inst.update(dt, input);
        if (run.roundDone && run.value >= run.cap() - 1e-6) {
          run.roundDone = false; run.fumbles = 0; run.step++;
          if (run.step >= job.steps) { run.done = true; run.value = job.target; run.note = run.game.doneWord || "Repaired"; run.noteT = 99; }
          else run.startStep();
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
        g.restore();
        if (run.done) { g.fillStyle = "rgba(4,6,10,0.55)"; g.fillRect(0, BAR_H, W, H - BAR_H); KIT.draw.text(g, (run.game.doneWord || "Repaired").toUpperCase(), W / 2, H / 2, 64, C.ok, "center", 700); }
        if (run.flash > 0) { g.fillStyle = `rgba(255,71,87,${0.25 * run.flash})`; g.fillRect(0, BAR_H, W, H - BAR_H); }
        drawBar();
      }
      g.restore();
      input.pressed = false; input.released = false; input.hit.clear(); input.wheel = 0;
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
