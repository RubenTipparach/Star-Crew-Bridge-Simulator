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
 * - a seeded random stream per game and step, so a shot is the same every time;
 * - the how-to guide (design 6g): the ? button in the bar, F1 or a pad's Back, and the card it lays over the game,
 *   drawn here from each game's `guide` data with the icon set below, so every game's card looks the same;
 * - fluid through pipe tiles (flood): the breadth-first fill the shower and the coolant pipes share.
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
 *   guide: { steps: [{ icon, text, cover? }], mistake, now? } the how-to card (design 6g): up to four steps in the
 *     order they come, each an icon from KIT.icons and about three to six words; `cover: true` marks the step lit
 *     while the cover's screws are worked; `mistake` is one line, cause and cost; now(q) (optional) takes the game's
 *     peek() (or its state) and returns the step it is in (0-based), or -1, so the card lights it,
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
      /**
       * Pipes with a bore, the shower's look and every pipe game's (owner, 2026-10-09: "use the better pipes from the
       * shower mini game"): a dark casing, a metal `body`, and down the middle a bore, `dry` where nothing runs and
       * `fluid` as far as it has got. `runs` are polylines, or { pts, fill } with `fill` (0-1) how far from pts[0] the
       * fluid has run. Each layer is drawn for every run before the next, so runs that meet join cleanly.
       */
      pipe(g, runs, { w = 22, body = "#3a4658", casing = "#232c3b", bore = 0.4, dry = "#141b27", fluid = null, fluidAlpha = 1, cap = "round" } = {}) {
        const list = runs.map((r) => (Array.isArray(r) ? { pts: r, fill: 1 } : r));
        const stroke = (pts) => { g.beginPath(); pts.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y))); g.stroke(); };
        g.lineJoin = "round"; g.lineCap = cap;
        for (const [lw, col] of [[w + 6, casing], [w, body], [w * bore, dry]]) { g.strokeStyle = col; g.lineWidth = lw; for (const r of list) stroke(r.pts); }
        if (fluid) {
          g.globalAlpha = fluidAlpha; g.strokeStyle = fluid; g.lineWidth = w * bore;
          for (const r of list) if (r.fill > 0) stroke(r.fill >= 1 ? r.pts : KIT.polyCut(r.pts, r.fill));
          g.globalAlpha = 1;
        }
        g.lineCap = "butt"; g.lineJoin = "miter";
      },
      /** The hub where a tile's arms meet: a boss on the body, its bore dry or holding `fluid`. */
      pipeHub(g, x, y, { r = 14, body = "#4a5566", dry = "#141b27", fluid = null, fluidAlpha = 1 } = {}) {
        KIT.draw.disc(g, x, y, r, body);
        KIT.draw.disc(g, x, y, r * 0.43, dry);
        if (fluid) { g.globalAlpha = fluidAlpha; KIT.draw.disc(g, x, y, r * 0.43, fluid); g.globalAlpha = 1; }
      },
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
      /** A curved arrow along a rim from a0 to a1, the head at a1: the way a ring or a crank turns. a1 > a0 runs
       *  clockwise on screen, a1 < a0 anticlockwise (the arc takes the short way, the head points along it). */
      turnArrow(g, x, y, r, a0, a1, color) {
        g.beginPath(); g.arc(x, y, r, a0, a1, a1 < a0); g.strokeStyle = color; g.lineWidth = 4; g.stroke();
        const hx = x + Math.cos(a1) * r, hy = y + Math.sin(a1) * r, d = a1 + (a1 < a0 ? -1 : 1) * Math.PI / 2;
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
    /**
     * A fluid's reach through connected pieces (the shower's tiles, the coolant pipes'): a breadth-first fill from the
     * inlets, each piece numbered by how many pieces the fluid passed to reach it, so a valve or FILL can wet them in that
     * order, tile by tile (owner, 2026-10-09: "pipes on coolant need work to actually function like the shower thing").
     *   starts: [{ key, ... }] the pieces the fluid enters first, at distance 0;
     *   visit(node, dist, reach): called once a piece, nearest first; reach(next) carries the fluid on into `next`
     *     ({ key, ... }) at dist + 1 and returns false when that piece was already wet (by this or another inlet).
     * Returns { dist: Map key -> distance, far: the furthest distance reached }.
     */
    flood(starts, visit) {
      const dist = new Map(), queue = [];
      for (const st of starts) if (!dist.has(st.key)) { dist.set(st.key, 0); queue.push([st, 0]); }
      let far = 0;
      for (let qi = 0; qi < queue.length; qi++) {
        const [node, d] = queue[qi];
        far = Math.max(far, d);
        visit(node, d, (next) => { if (dist.has(next.key)) return false; dist.set(next.key, d + 1); queue.push([next, d + 1]); return true; });
      }
      return { dist, far };
    },
  };
  window.RepairKit = KIT;

  // ------------------------------------------------------------------ the guide's icons (design 6g)
  // One small picture a move, drawn in a box 100 px across centred on the origin, so every game's card speaks the same
  // picture language. A finger is the blue dot with a white rim; a target is amber; good is green, a mistake red.
  const FINGER = (g, x, y) => { KIT.draw.disc(g, x, y, 10, C.accent); KIT.draw.ring(g, x, y, 10, "#e8f7ff", 2.5); };
  const arrowHead = (g, x, y, a, col, s = 10) => {
    g.beginPath(); g.moveTo(x + Math.cos(a) * s, y + Math.sin(a) * s);
    g.lineTo(x + Math.cos(a + 2.5) * s, y + Math.sin(a + 2.5) * s); g.lineTo(x + Math.cos(a - 2.5) * s, y + Math.sin(a - 2.5) * s);
    g.closePath(); g.fillStyle = col; g.fill();
  };
  const line = (g, pts, col, w = 4, dash = null) => {
    g.beginPath(); pts.forEach(([x, y], i) => (i ? g.lineTo(x, y) : g.moveTo(x, y)));
    if (dash) g.setLineDash(dash); g.strokeStyle = col; g.lineWidth = w; g.lineCap = "round"; g.lineJoin = "round"; g.stroke();
    g.setLineDash([]); g.lineCap = "butt";
  };
  const ICONS = {
    /** Tap a target. */
    tap(g, t) {
      for (let k = 0; k < 2; k++) { const ph = (t * 0.9 + k / 2) % 1; KIT.draw.ring(g, 0, 0, 14 + ph * 26, `rgba(242,160,70,${0.9 * (1 - ph)})`, 3); }
      KIT.draw.ring(g, 0, 0, 16, C.amber, 4); FINGER(g, 0, 0);
    },
    /** Drag a part onto its place. */
    drag(g, t) {
      g.setLineDash([6, 5]); KIT.draw.round(g, 14, -38, 30, 30, 6); g.strokeStyle = C.amber; g.lineWidth = 3; g.stroke(); g.setLineDash([]);
      const u = (t * 0.5) % 1, x = -30 + 59 * u, y = 22 - 45 * u;
      line(g, [[-30, 22], [22, -16]], "rgba(232,238,246,0.35)", 3, [5, 6]);
      KIT.draw.round(g, x - 14, y - 14, 28, 28, 6); g.fillStyle = C.steel; g.fill();
      FINGER(g, x + 6, y + 8);
    },
    /** Hold a press while a ring fills. */
    hold(g, t) {
      KIT.draw.disc(g, 0, 0, 26, "#262e42"); KIT.draw.ring(g, 0, 0, 34, "#2a3446", 6);
      g.beginPath(); g.arc(0, 0, 34, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * ((t * 0.4) % 1)); g.strokeStyle = C.ok; g.lineWidth = 6; g.stroke();
      FINGER(g, 0, 0);
    },
    /** Turn round a knob, a wheel or a crank. */
    turn(g, t) {
      KIT.draw.ring(g, 0, 0, 22, C.steel, 6);
      g.save(); g.rotate(t * 2); g.strokeStyle = "#c9d3e0"; g.lineWidth = 4;
      for (let k = 0; k < 3; k++) { const b = (k * Math.PI * 2) / 3; g.beginPath(); g.moveTo(0, 0); g.lineTo(Math.cos(b) * 20, Math.sin(b) * 20); g.stroke(); }
      g.restore();
      KIT.draw.turnArrow(g, 0, 0, 38, -2.6, 0.4, C.amber);
      const a = t * 2; FINGER(g, Math.cos(a) * 38, Math.sin(a) * 38);
    },
    /** A screw: turn it a full loop. */
    screw(g, t) {
      KIT.draw.disc(g, 0, 0, 18, "#b7c0cc"); KIT.draw.ring(g, 0, 0, 18, "#7a8494", 2);
      g.save(); g.rotate(-t * 2); g.strokeStyle = "#2a313c"; g.lineWidth = 4; g.beginPath(); g.moveTo(-9, 0); g.lineTo(9, 0); g.moveTo(0, -9); g.lineTo(0, 9); g.stroke(); g.restore();
      KIT.draw.turnArrow(g, 0, 0, 34, 0.4, -2.6, C.amber);
    },
    /** Swap a broken piece for a new one. */
    swap(g) {
      KIT.draw.round(g, -44, -18, 36, 36, 6); g.fillStyle = "#1d1410"; g.fill(); g.strokeStyle = C.danger; g.lineWidth = 2; g.stroke();
      line(g, [[-32, -12], [-24, -2], [-30, 4], [-20, 14]], C.danger, 3);
      KIT.draw.round(g, 8, -18, 36, 36, 6); g.fillStyle = C.steel; g.fill(); g.strokeStyle = C.ok; g.lineWidth = 2; g.stroke();
      line(g, [[4, 28], [-6, 28]], C.fg, 4); arrowHead(g, -6, 28, Math.PI, C.fg);
      line(g, [[-26, -28], [26, -28]], "rgba(232,238,246,0.3)", 2, [4, 4]);
    },
    /** Sweep a tool along a line. */
    sweep(g, t) {
      const pts = Array.from({ length: 21 }, (_, i) => [-40 + i * 4, 14 * Math.sin(i * 0.35)]);
      line(g, pts, "rgba(232,238,246,0.35)", 3, [5, 5]);
      const n = 1 + Math.floor(((t * 0.6) % 1) * 20);
      line(g, pts.slice(0, n + 1), C.amber, 6);
      FINGER(g, pts[n][0], pts[n][1]);
    },
    /** Act on the beat: a pulse crossing the line. */
    rhythm(g, t) {
      const off = ((t * 30) % 34);
      line(g, [[-46, 16], [46, 16]], "#4a5568", 2);
      for (let k = -2; k <= 2; k++) {
        const x = -40 + k * 34 + off; if (x < -46 || x > 46) continue;
        const on = Math.abs(x) < 6;
        line(g, [[x - 8, 16], [x, -8], [x + 8, 16]], on ? C.ok : C.fg, 3);
      }
      line(g, [[0, -24], [0, 30]], C.amber, 3);
    },
    /** Keep a needle in its green band. */
    band(g, t) {
      g.beginPath(); g.arc(0, 14, 40, Math.PI * 1.05, Math.PI * 1.95); g.strokeStyle = "#2a3446"; g.lineWidth = 10; g.stroke();
      g.beginPath(); g.arc(0, 14, 40, Math.PI * 1.4, Math.PI * 1.62); g.strokeStyle = C.ok; g.lineWidth = 10; g.stroke();
      KIT.draw.hatchArc(g, 0, 14, 35, 45, Math.PI * 1.8, Math.PI * 1.95, C.danger);
      const a = Math.PI * 1.51 + 0.07 * Math.sin(t * 3);
      line(g, [[0, 14], [Math.cos(a) * 36, 14 + Math.sin(a) * 36]], C.fg, 4);
      KIT.draw.disc(g, 0, 14, 6, C.steel);
    },
    /** Work things in their numbered order. */
    order(g, t) {
      const P = [[-30, 26], [22, 26], [-4, -18]];
      line(g, [P[0], P[1]], "rgba(232,238,246,0.3)", 2, [4, 4]); line(g, [P[1], P[2]], "rgba(232,238,246,0.3)", 2, [4, 4]);
      const next = Math.floor(t * 0.8) % 3;
      P.forEach(([x, y], i) => { KIT.draw.disc(g, x, y, 8, "#b7c0cc"); KIT.draw.orderBadge(g, x, y, 8, i + 1, i === next, t); });
    },
    /** Slide a handle along its track. */
    slider(g, t) {
      line(g, [[-40, 0], [40, 0]], "#2a3446", 10);
      line(g, [[-6, 0], [14, 0]], C.ok, 10);
      const x = 4 + 12 * Math.sin(t * 2);
      KIT.draw.round(g, x - 9, -20, 18, 40, 6); g.fillStyle = "#c9d3e0"; g.fill();
      arrowHead(g, -46, 0, Math.PI, C.amber, 9); arrowHead(g, 46, 0, 0, C.amber, 9);
    },
    /** Lay a live trace on its reference. */
    match(g, t) {
      const wave = (ph, amp) => Array.from({ length: 25 }, (_, i) => [-44 + i * 3.67, amp * Math.sin(i * 0.5 + ph)]);
      line(g, wave(0, 18), "rgba(61,220,132,0.4)", 9);
      line(g, wave(0.6 * Math.sin(t * 1.5), 18 + 4 * Math.sin(t)), C.fg, 3);
    },
    /** A valve: open or shut it. */
    valve(g) {
      line(g, [[-46, 10], [46, 10]], "#3a4658", 12);
      g.beginPath(); g.moveTo(-18, -4); g.lineTo(18, 24); g.lineTo(18, -4); g.lineTo(-18, 24); g.closePath(); g.fillStyle = "#c0392b"; g.fill(); g.strokeStyle = "#ff8a80"; g.lineWidth = 2; g.stroke();
      line(g, [[0, 10], [0, -22]], "#c9d3e0", 4); line(g, [[-14, -22], [14, -22]], "#c9d3e0", 5);
      FINGER(g, 10, -22);
    },
    /** Turn a pipe tile a quarter to join the run. */
    pipes(g, t) {
      KIT.draw.round(g, -30, -30, 60, 60, 8); g.fillStyle = "#0f1520"; g.fill(); g.strokeStyle = "#2a3446"; g.lineWidth = 2; g.stroke();
      g.save(); g.rotate((Math.floor(t * 0.7) % 4) * Math.PI / 2 + Math.min(1, (t * 0.7 % 1) * 4) * Math.PI / 2);
      g.beginPath(); g.moveTo(0, -30); g.quadraticCurveTo(0, 0, 30, 0); g.strokeStyle = "#4a586d"; g.lineWidth = 14; g.stroke();
      g.restore();
      KIT.draw.turnArrow(g, 0, 0, 42, -2.2, -0.9, C.amber);
    },
    /** Let the fluid run through what is built. */
    flow(g, t) {
      line(g, [[-46, 0], [46, 0]], "#263142", 22);
      const u = (t * 0.5) % 1;
      line(g, [[-46, 0], [-46 + 92 * u, 0]], "#ff6a4d", 14);
      for (let x = -38; x < -46 + 92 * u - 6; x += 18) line(g, [[x - 4, -6], [x + 3, 0], [x - 4, 6]], "#fff3e0", 2.5);
      KIT.draw.round(g, -26, 18, 52, 24, 12); g.fillStyle = "#1f6b45"; g.fill();
    },
    /** Aim at a mark. */
    aim(g, t) {
      KIT.draw.ring(g, 0, 0, 30, "#4a5568", 2); line(g, [[-40, 0], [40, 0]], "#4a5568", 2); line(g, [[0, -40], [0, 40]], "#4a5568", 2);
      const r = 6 + 10 * Math.abs(Math.sin(t)); KIT.draw.disc(g, 4 * Math.sin(t * 1.3), 3 * Math.cos(t), r, "rgba(242,160,70,0.8)");
    },
    /** Pick the right tool from the tray. */
    tool(g, t) {
      for (let i = 0; i < 3; i++) {
        const x = -34 + i * 34, on = i === 1;
        KIT.draw.round(g, x - 14, -4, 28, 34, 6); g.fillStyle = on ? "#3a2a12" : "#141c28"; g.fill();
        g.lineWidth = on ? 3 : 2; g.strokeStyle = on ? C.amber : "#2a3446"; g.stroke();
        line(g, [[x - 5, 20], [x + 5, 4]], on ? C.fg : C.steel, 4);
      }
      FINGER(g, 4, 16 + 3 * Math.sin(t * 3));
      arrowHead(g, 0, -20, Math.PI / 2, C.amber, 10);
    },
    /** A plug into the socket of its own shape. */
    plug(g, t) {
      const x = -6 - 10 * Math.abs(Math.sin(t * 1.5));
      KIT.draw.round(g, 14, -18, 32, 36, 6); g.fillStyle = "#141c28"; g.fill(); g.strokeStyle = C.steel; g.lineWidth = 3; g.stroke();
      g.beginPath(); g.moveTo(30, -9); g.lineTo(39, 6); g.lineTo(21, 6); g.closePath(); g.strokeStyle = C.amber; g.lineWidth = 2.5; g.stroke();
      line(g, [[-46, 0], [x - 14, 0]], C.lilac, 6);
      KIT.draw.round(g, x - 14, -12, 24, 24, 4); g.fillStyle = C.lilac; g.fill();
      g.beginPath(); g.moveTo(x - 2, -7); g.lineTo(x + 5, 5); g.lineTo(x - 9, 5); g.closePath(); g.fillStyle = "#1d1430"; g.fill();
    },
  };
  /** A warning triangle: the mistake's line. */
  function warnGlyph(g, x, y, s) {
    g.beginPath(); g.moveTo(x, y - s); g.lineTo(x + s * 1.1, y + s * 0.85); g.lineTo(x - s * 1.1, y + s * 0.85); g.closePath();
    g.fillStyle = C.danger; g.fill();
    g.fillStyle = "#1a0508"; g.fillRect(x - 2.5, y - s * 0.45, 5, s * 0.75); g.fillRect(x - 2.5, y + s * 0.45, 5, 5);
  }
  KIT.icons = ICONS;

  /**
   * Words wrapped to lines no wider than `w` px in the current font. Two lines are balanced (the break that makes the
   * longer line shortest), so a step never leaves one word alone on its second line.
   */
  function wrap(g, s, w) {
    const words = s.split(/\s+/), out = [];
    let cur = "";
    for (const word of words) {
      const next = cur ? cur + " " + word : word;
      if (cur && g.measureText(next).width > w) { out.push(cur); cur = word; } else cur = next;
    }
    if (cur) out.push(cur);
    if (out.length !== 2) return out;
    let best = out, bw = Infinity;
    for (let i = 1; i < words.length; i++) {
      const a = words.slice(0, i).join(" "), b = words.slice(i).join(" ");
      const m = Math.max(g.measureText(a).width, g.measureText(b).width);
      if (m <= w && m < bw) { bw = m; best = [a, b]; }
    }
    return best;
  }
  /** The guide's ? button in the bar, beside the fumble pips: canvas px. 48 px across, a 26 CSS px target on a phone. */
  const GUIDE_BTN = { x: 1234, y: 31, r: 24 };
  /** The card's frame, over the game below the bar. */
  const CARD = { x: 120, y: 176, w: 1040, h: 400 };
  /**
   * The guide card (design 6g): numbered steps left to right, each a picture and its few words, the one the game is in
   * lit; the mistake under them on one line; a cross in the corner. Tap anywhere, ?, F1, Escape or Back closes it.
   */
  function drawGuide(g, guide, now, t) {
    const D = KIT.draw, steps = guide.steps.slice(0, 4), n = steps.length;
    g.fillStyle = "rgba(4,6,10,0.72)"; g.fillRect(0, BAR_H, W, H - BAR_H);
    D.panel(g, CARD.x, CARD.y, CARD.w, CARD.h, 22, "#0e151f", "#33445a");
    // The cross: the card closes on a tap anywhere, the cross only says so.
    const cx = CARD.x + CARD.w - 34, cy = CARD.y + 34;
    g.strokeStyle = C.dim; g.lineWidth = 4; g.lineCap = "round";
    g.beginPath(); g.moveTo(cx - 10, cy - 10); g.lineTo(cx + 10, cy + 10); g.moveTo(cx + 10, cy - 10); g.lineTo(cx - 10, cy + 10); g.stroke();
    g.lineCap = "butt";
    const colW = Math.min(240, (CARD.w - 80) / Math.max(1, n)), x0 = CARD.x + (CARD.w - colW * n) / 2, tile = 150, ty = CARD.y + 52;
    steps.forEach((st, i) => {
      const x = x0 + i * colW + colW / 2, lit = i === now;
      // The picture.
      D.round(g, x - tile / 2, ty, tile, tile, 18); g.fillStyle = lit ? "#1d2a3c" : "#121a25"; g.fill();
      g.lineWidth = lit ? 4 : 2; g.strokeStyle = lit ? C.amber : "#2a3446"; g.stroke();
      g.save(); g.translate(x, ty + tile / 2); g.scale(1.25, 1.25);
      g.beginPath(); g.rect(-58, -58, 116, 116); g.clip();
      (ICONS[st.icon] || ICONS.tap)(g, t);
      g.restore();
      // Its number, in the order the steps come.
      D.disc(g, x - tile / 2 + 4, ty + 4, 18, lit ? C.amber : "#26324a");
      D.ring(g, x - tile / 2 + 4, ty + 4, 18, lit ? "#ffe2bd" : "#4a5568", 2);
      D.text(g, String(i + 1), x - tile / 2 + 4, ty + 5, 24, lit ? "#1a0f02" : C.fg, "center", 700);
      // Between steps: a chevron, the way the job runs.
      if (i < n - 1) { const ax = x0 + (i + 1) * colW; g.strokeStyle = "#4a5568"; g.lineWidth = 4; g.beginPath(); g.moveTo(ax - 6, ty + tile / 2 - 12); g.lineTo(ax + 4, ty + tile / 2); g.lineTo(ax - 6, ty + tile / 2 + 12); g.stroke(); }
      // Its words.
      g.font = `600 24px ${FONT}`;
      wrap(g, st.text, colW - 22).slice(0, 3).forEach((ln, k) => D.text(g, ln, x, ty + tile + 30 + k * 28, 24, lit ? C.fg : "#c4cfdc", "center", 600));
    });
    // The mistake: what causes it and what it costs, one line.
    if (guide.mistake) {
      const my = CARD.y + CARD.h - 52;
      D.round(g, CARD.x + 30, my - 28, CARD.w - 60, 56, 14); g.fillStyle = "#1c0f14"; g.fill(); g.strokeStyle = "rgba(255,71,87,0.55)"; g.lineWidth = 2; g.stroke();
      warnGlyph(g, CARD.x + 66, my + 1, 16);
      g.font = `600 24px ${FONT}`;
      let s = guide.mistake;
      const max = CARD.w - 150;
      if (g.measureText(s).width > max) { while (s.length > 4 && g.measureText(s + "…").width > max) s = s.slice(0, -1); s = s.trimEnd() + "…"; }
      D.text(g, s, CARD.x + 96, my + 1, 24, "#ffd2d6", "left", 600);
    }
  }
  /** The ? button: lit while the card is up. */
  function drawGuideButton(g, open) {
    const D = KIT.draw, b = GUIDE_BTN;
    D.disc(g, b.x, b.y, b.r, open ? C.accent : "#262e42");
    D.ring(g, b.x, b.y, b.r, open ? "#e8f7ff" : "#4a5a70", 2);
    D.text(g, "?", b.x, b.y + 2, 30, open ? "#04131c" : C.fg, "center", 700);
  }
  /** Whether a player has seen a game's card: browser storage where it works, this page's memory where it does not. */
  const seenHere = new Set();
  const GUIDE_KEY = (id) => "starcrew.repairs.guide." + id;
  function guideSeen(id) {
    if (seenHere.has(id)) return true;
    try { return localStorage.getItem(GUIDE_KEY(id)) === "1"; } catch (_) { return false; }
  }
  function markGuideSeen(id) {
    seenHere.add(id);
    try { localStorage.setItem(GUIDE_KEY(id), "1"); } catch (_) { /* private window or blocked storage: memory only */ }
  }

  // ------------------------------------------------------------------ access panels (screws)
  // The owner, 2026-10-08: "some of the panel ones would be cool to like screw or unscrew stuff". A game that names a
  // `panel` ({ x, y, w, h, screws }) has its machine behind a cover plate: the job opens by unscrewing it (turn each
  // screw anticlockwise: drag round it, roll the wheel over it, or hold Left; Tab picks the next) and ends by screwing
  // it back (clockwise, Right). The plate hides the machine until it is off. Nothing here is a fumble: a screw only
  // turns while the pointer goes round it.
  // One full loop frees a screw or drives it home (owner, 2026-10-09: the screws were not "properly detecting when I do
  // a full loop"; it took 2.5).
  const SCREW_TURNS = 1;                        // turns to free a screw, and to drive it home
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
  /** A polyline's length, px. */
  KIT.polyLen = (pts) => { let L = 0; for (let i = 1; i < pts.length; i++) L += Math.hypot(pts[i][0] - pts[i - 1][0], pts[i][1] - pts[i - 1][1]); return L; };
  /** The point `u` (0-1) of the way along a polyline by length, and the direction it runs there: [x, y, angle]. */
  KIT.polyAt = (pts, u) => {
    let d = Math.max(0, Math.min(1, u)) * KIT.polyLen(pts);
    for (let i = 1; i < pts.length; i++) {
      const [x0, y0] = pts[i - 1], [x1, y1] = pts[i], L = Math.hypot(x1 - x0, y1 - y0);
      if (d <= L || i === pts.length - 1) { const k = L > 0 ? Math.min(1, d / L) : 0; return [x0 + (x1 - x0) * k, y0 + (y1 - y0) * k, Math.atan2(y1 - y0, x1 - x0)]; }
      d -= L;
    }
    return [pts[0][0], pts[0][1], 0];
  };
  /** The first `u` (0-1) of a polyline by length, as a polyline. */
  KIT.polyCut = (pts, u) => {
    let d = Math.max(0, Math.min(1, u)) * KIT.polyLen(pts);
    const out = [pts[0]];
    for (let i = 1; i < pts.length; i++) {
      const [x0, y0] = pts[i - 1], [x1, y1] = pts[i], L = Math.hypot(x1 - x0, y1 - y0);
      if (d >= L) { out.push(pts[i]); d -= L; continue; }
      const k = L > 0 ? d / L : 0; out.push([x0 + (x1 - x0) * k, y0 + (y1 - y0) * k]); break;
    }
    return out;
  };
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
      // Every pointer sample since the last frame, not one a frame: a quick loop close round the head sweeps more than
      // half a turn between frames, which a once-a-frame angle reads as the other way round and throws away.
      const sc = p.screws[p.grab], pts = input.path.length ? input.path : [[input.x, input.y]];
      let wrong = 0;
      for (const [px, py] of pts) {
        const dx = px - sc.x, dy = py - sc.y;
        if (Math.hypot(dx, dy) <= 6) continue;                 // through the middle: no angle to read
        const a = Math.atan2(dy, dx);
        if (p.prevA !== null) { let d = a - p.prevA; d = Math.atan2(Math.sin(d), Math.cos(d)); turn(sc, d); if (d * dir < 0) wrong -= d * dir; }
        p.prevA = a;
      }
      // Turning it the wrong way does nothing; after a third of a turn of it the arrow flashes and says so.
      p.wrongAcc = (p.wrongAcc || 0) * Math.max(0, 1 - dt) + wrong;
      if (p.wrongAcc > 2) { p.wrongAcc = 0; p.wrongT = 1.2; p.wrongSay = true; }
    }
    p.wrongT = Math.max(0, (p.wrongT || 0) - dt);
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
        // The progress ring fills the way the screw turns: anticlockwise coming off, clockwise going home.
        const pdir = p.phase === "open" ? -1 : 1;
        g.beginPath(); g.arc(sc.x, sc.y, SCREW_R + 9, -Math.PI / 2, -Math.PI / 2 + (pdir * Math.PI * 2 * sc.turn) / goal, pdir < 0);
        g.strokeStyle = C.amber; g.lineWidth = 4; g.stroke();
        if (i === p.sel) D.ring(g, sc.x, sc.y, SCREW_R + 15 + Math.sin(t * 5) * 2, "rgba(232,238,246,0.35)", 2);
      } else D.ring(g, sc.x, sc.y, SCREW_R + 9, C.ok, 3);
    }
    // Which way they turn, once, by the selected screw: a turn arrow (anticlockwise off, clockwise on).
    const sc = p.screws[p.sel];
    if (sc && sc.turn < goal) {
      const dir = p.phase === "open" ? -1 : 1;
      const wrong = p.wrongT > 0 && Math.sin(t * 18) > -0.3;   // turned the wrong way: the arrow flashes red
      D.turnArrow(g, sc.x, sc.y, SCREW_R + 26, dir < 0 ? 0.2 : -1.6, dir < 0 ? -1.6 : 0.2, wrong ? C.danger : "rgba(232,238,246,0.6)");
    }
    g.restore();
  }

  // ------------------------------------------------------------------ the runner (the page calls RepairKit.start)
  KIT.start = function (canvas, ui) {
    const g = canvas.getContext("2d");
    const input = { x: -1, y: -1, path: [], down: false, pressed: false, released: false, keys: new Set(), hit: new Set(), wheel: 0,
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
    // Every sample of a move while pressed goes on input.path (the browser's coalesced events between two frames), for
    // games that follow a stroke's shape rather than where it ended (the screw panels).
    canvas.addEventListener("pointermove", (e) => {
      if (foreign(e)) return;
      if (input.down) { const all = e.getCoalescedEvents ? e.getCoalescedEvents() : []; for (const q of all.length ? all : [e]) input.path.push(toCanvas(q)); }
      [input.x, input.y] = toCanvas(e); if (input.down) e.preventDefault();
    });
    canvas.addEventListener("pointerdown", (e) => {
      e.preventDefault();
      if (foreign(e)) return;
      owner = e.pointerId;
      [input.x, input.y] = toCanvas(e); input.down = true; input.pressed = true; input.path = [[input.x, input.y]];
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
    addEventListener("keydown", (e) => { if (!input.keys.has(e.code)) input.hit.add(e.code); input.keys.add(e.code); if (["Space", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "F1"].includes(e.code)) e.preventDefault(); });
    addEventListener("keyup", (e) => input.keys.delete(e.code));

    // The guide card (design 6g). While it is up, and until the press or key that closed it lets go, the game and the
    // job's clock wait: nothing the player does to read or dismiss it reaches the game.
    const guide = { open: false, hold: false, keys: null };
    const IDLE = { x: -1, y: -1, path: [], down: false, pressed: false, released: false, keys: new Set(), hit: new Set(), wheel: 0,
      stick: { x: 0, y: 0 }, action: false, actionPressed: false };
    function showGuide() { if (run && run.game.guide) { guide.open = true; guide.hold = true; } }
    function hideGuide() {
      if (!guide.open) return;
      guide.open = false; guide.keys = new Set(input.keys);
      if (run) markGuideSeen(run.game.id);
    }
    /** The step the game is in, for the card to light: the cover's step while its screws are worked, else the game's. */
    function guideNow() {
      const gd = run.game.guide, P = run.panel;
      if (P && !run.auto && P.phase !== "work") return gd.steps.findIndex((st) => st.cover);
      if (!gd.now) return -1;
      try { const i = run.inst; const q = i.peek ? i.peek() : i.state; return q ? gd.now(q) : -1; } catch (_) { return -1; }
    }
    // A pad's Back (the standard mapping's button 8) toggles the card; A or B (0, 1) closes it.
    const padPrev = [];
    function padEdges() {
      let back = false, shut = false;
      let pads = [];
      try { pads = (navigator.getGamepads && navigator.getGamepads()) || []; } catch (_) { pads = []; }
      for (let i = 0; i < pads.length; i++) {
        const p = pads[i]; if (!p) continue;
        const now = [0, 1, 8].map((b) => !!(p.buttons[b] && p.buttons[b].pressed)), was = padPrev[i] || [false, false, false];
        if (now[2] && !was[2]) back = true;
        if ((now[0] && !was[0]) || (now[1] && !was[1])) shut = true;
        padPrev[i] = now;
      }
      return { back, shut };
    }

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
      // The guide shows on its own the first time a player opens a game (until they close it once); a rating has
      // nothing to play, so it waits for the ? there.
      guide.open = guide.hold = !run.auto && !!game.guide && !guideSeen(game.id);
      guide.keys = null;
      if (ui && ui.onOpen) ui.onOpen(game, run);
      location.hash = game.id;
    }

    function update(dt) {
      if (!run) return;
      input.stick.x = (input.keys.has("ArrowRight") || input.keys.has("KeyD") ? 1 : 0) - (input.keys.has("ArrowLeft") || input.keys.has("KeyA") ? 1 : 0);
      input.stick.y = (input.keys.has("ArrowDown") || input.keys.has("KeyS") ? 1 : 0) - (input.keys.has("ArrowUp") || input.keys.has("KeyW") ? 1 : 0);
      input.action = input.keys.has("Space") || input.keys.has("Enter");
      input.actionPressed = input.hit.has("Space") || input.hit.has("Enter");
      // The guide: ?, F1 or Back toggles it; while it is up any press or key closes it, and the game waits.
      const pad = padEdges();
      const onBtn = input.pressed && Math.hypot(input.x - GUIDE_BTN.x, input.y - GUIDE_BTN.y) <= GUIDE_BTN.r + 10;
      const toggle = input.hit.has("F1") || pad.back || onBtn;
      if (guide.open) {
        if (toggle || input.pressed || pad.shut || input.hit.has("Escape") || input.actionPressed) hideGuide();
      } else if (toggle) showGuide();
      if (guide.hold) {
        const keysUp = !guide.keys || ![...guide.keys].some((k) => input.keys.has(k));
        if (guide.open || input.down || !keysUp) return;
        guide.hold = false;
      }
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
          if (P.wrongSay) { P.wrongSay = false; run.note = "Other way"; run.noteT = 1.2; }
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
      // The guide's button, beside the pips (design 6g).
      if (run.game.guide) drawGuideButton(g, guide.open);
      if (run.noteT > 0) D.text(g, run.note, 1240, 62, 16, run.done ? C.ok : C.danger, "right", 700);
    }

    let last = performance.now(), guideT = 0;
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
        } else run.inst.draw(g, run.t, dt, guide.hold ? IDLE : input);
        if (run.panel && !run.auto) drawPanel(g, run.panel, run.t);
        g.restore();
        if (run.done) { g.fillStyle = "rgba(4,6,10,0.55)"; g.fillRect(0, BAR_H, W, H - BAR_H); KIT.draw.text(g, (run.game.doneWord || "Repaired").toUpperCase(), W / 2, H / 2, 64, C.ok, "center", 700); }
        if (run.flash > 0) { g.fillStyle = `rgba(255,71,87,${0.25 * run.flash})`; g.fillRect(0, BAR_H, W, H - BAR_H); }
        drawBar();
      }
      g.restore();
      // The guide card, still while the ship shakes, over the game and never over the bar.
      if (run && guide.open) { guideT += dt; drawGuide(g, run.game.guide, guideNow(), guideT); }
      input.pressed = false; input.released = false; input.hit.clear(); input.wheel = 0; input.path = [];
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
      /** The guide card, for tools: whether it is up, and a way to raise or drop it as the ? would. */
      guide: { get open() { return guide.open; }, show: showGuide, hide: hideGuide },
    };
  };
})();
