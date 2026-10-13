/*
 * repairs/boxload.js: a distribution panel's repair (openspec/changes/repair-minigames, design 2a; power-routing 1: the
 * drive's, the hangars', the turrets', each deck's and life support's panels).
 *
 * The panel open: its main switch at the top, a row of ways below it (each a breaker with the load it feeds and its
 * current), and the panel's ammeter on the right with its trip line. Put the main switch up (drag it, or Space), then
 * close the ways one at a time (tap each, or left and right to choose and Space). Every way closes with an inrush: the
 * needle jumps to a few times the way's load and settles back. Close the next before the last has settled and the
 * spikes stack. Past the trip line the panel trips, every way opens, and that is a fumble. From level 3 one way is
 * burnt (its label scorched): tag it out (its tag), never close it; closing it is a fumble, a spark, 5 HP. Each level
 * has more ways (4 to 8), a bigger inrush and a slower settling. A disabled panel's first round fits a new main switch:
 * drag it from the crate into its slot.
 */
// The guide's pictures (repair-minigames 6g): this game's own parts, in the card's 100 px box about the origin.
/** MAIN up: the switch's handle rises in its slot under the finger and turns green. */
function boxloadPicMain(g, t, K) {
  const D = K.draw, u = Math.min(1, ((t * 0.6) % 1) * 1.4), y = 26 - 52 * u;
  D.round(g, -26, -48, 52, 96, 8); g.fillStyle = "#0b1017"; g.fill(); g.lineWidth = 2; g.strokeStyle = "#3a4656"; g.stroke();
  D.round(g, -20, y - 12, 40, 24, 5); g.fillStyle = u >= 1 ? K.C.ok : "#c8c2b4"; g.fill();
  D.arrowHead(g, 40, -30, -Math.PI / 2, K.C.amber, 9); D.line(g, [[40, 30], [40, -24]], K.C.amber, 3);
  D.finger(g, 0, y);
}
/** A way closed: its toggle flips up, the needle steps up, and an amber arc rides above it and shrinks away. */
function boxloadPicWay(g, t, K) {
  const D = K.draw, u = (t * 0.45) % 1, on = u > 0.15, surge = on ? Math.exp(-(u - 0.15) * 6) : 0;
  const A0 = (-220 * Math.PI) / 180, A1 = (40 * Math.PI) / 180, ang = (k) => A0 + (A1 - A0) * k;
  const cx = 10, cy = -6, r = 34, base = 0.3, load = on ? 0.48 : base;
  g.beginPath(); g.arc(cx, cy, r, A0, A1); g.strokeStyle = "#1d2738"; g.lineWidth = 10; g.stroke();
  g.beginPath(); g.arc(cx, cy, r, A0, ang(load)); g.strokeStyle = "rgba(232,238,246,0.35)"; g.lineWidth = 10; g.stroke();
  if (surge > 0.03) { g.beginPath(); g.arc(cx, cy, r, ang(load), ang(load + 0.4 * surge)); g.strokeStyle = K.C.amber; g.lineWidth = 10; g.stroke(); }
  D.line(g, [[cx, cy], [cx + Math.cos(ang(load)) * (r - 4), cy + Math.sin(ang(load)) * (r - 4)]], K.C.fg, 3);
  D.round(g, -48, 14, 22, 36, 4); g.fillStyle = "#d8cfb8"; g.fill();
  D.round(g, -43, on ? 18 : 32, 12, 14, 3); g.fillStyle = on ? K.C.ok : "#1b1f26"; g.fill();
  if (u < 0.3) D.finger(g, -37, 32);
}
/** The scorched way: tap its TAG and leave it off. */
function boxloadPicTag(g, t, K) {
  const D = K.draw, tagged = (t * 0.5) % 1 > 0.5;
  const sc = g.createRadialGradient(-8, -8, 2, -8, -8, 30); sc.addColorStop(0, "rgba(0,0,0,0.9)"); sc.addColorStop(1, "rgba(0,0,0,0)");
  D.round(g, -24, -40, 32, 56, 5); g.fillStyle = "#d8cfb8"; g.fill(); g.fillStyle = sc; g.fillRect(-34, -48, 52, 72);
  D.round(g, -18, -10, 20, 18, 3); g.fillStyle = "#1b1f26"; g.fill();
  D.round(g, -30, 26, 44, 20, 4); g.fillStyle = tagged ? K.C.danger : "#0c121a"; g.fill(); g.lineWidth = 2; g.strokeStyle = K.C.danger; g.stroke();
  D.text(g, "TAG", -8, 36, 12, tagged ? "#fff" : K.C.danger, "center", 700);
  if (!tagged) D.finger(g, -8, 36);
}

RepairKit.register({
  id: "boxload",
  title: "Distribution panel",
  place: "Each deck's distribution panels",
  group: "Electrical",
  hazard: "Over the trip line: the panel trips",
  guide: {
    steps: [
      { icon: "screw", text: "Unscrew the cover", cover: true },
      { pic: boxloadPicMain, text: "Drag MAIN up" },
      { pic: boxloadPicWay, text: "Tap a way, wait for the amber to go" },
      { pic: boxloadPicTag, text: "Scorched way: tap TAG, leave it off" },
    ],
    mistake: "Amber past 100 A: the panel trips, all off. A scorched way: a spark, 5 HP",
    now: (q) => ({ main: 1, ways: q.burnt >= 0 && !q.tagged && q.closed >= q.n - 2 ? 3 : 2 })[q.phase] ?? -1,
  },
  down: "The rooms and systems it feeds lose their power (power-routing)",
  panel: { x: 100, y: 96, w: 660, h: 604, screws: 6 },
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const BOX = { x: 100, y: 96, w: 660, h: 604 };
    const MAIN = { x: 210, y0: 300, y1: 190, w: 110, h: 150 };   // the main switch: its handle's travel, down to up
    const WAYS = { x0: 150, x1: 720, top: 404, w: 58, h: 132 };
    const METER = { x: 1010, y: 330, r: 150 };
    const CRATE = { x: 840, y: 560, w: 360, h: 130 };
    const NAMES = ["LIGHTS", "AIR", "CONSOLES", "PUMP", "TURRET", "HOIST", "GALLEY", "MEDBAY"];
    const T = 100, MAXA = 125;                                   // the trip line and the meter's end, A
    const A0 = (-220 * Math.PI) / 180, A1 = (40 * Math.PI) / 180;
    let phase, part, slot, main, mainY, grab, n, ways, burnt, tagged, sel, m, tau, amps, load, trips, flash, spark, time;

    const wayX = (i) => WAYS.x0 + 30 + (i * (WAYS.x1 - WAYS.x0 - 60)) / Math.max(1, n - 1);
    const ang = (a) => A0 + (A1 - A0) * Math.min(1, a / MAXA);
    const steady = () => ways.reduce((s, w) => s + (w.on ? w.a : 0), 0);
    const surge = () => ways.reduce((s, w) => s + (w.on ? w.a * (m - 1) * Math.exp(-w.since / tau) : 0), 0);
    const tagBox = (i) => ({ x: wayX(i) - 26, y: WAYS.top + WAYS.h + 46, w: 52, h: 30 });

    function trip() {
      for (const w of ways) { w.on = false; w.since = 0; }
      main = false; mainY = MAIN.y0; trips++; flash = 1; phase = "main";
      api.fumble("Over the trip line: the panel trips");
    }

    return {
      step(index, isPart) {
        const r = api.rand(), level = api.level;
        part = isPart; slot = { x: CRATE.x + 180, y: CRATE.y + 70, held: false, set: !isPart };
        phase = isPart ? "part" : "main";
        n = Math.min(8, 3 + level);
        m = 2.0 + 0.3 * (level - 1);
        tau = (0.8 + 0.2 * (level - 1)) / 3;
        // The loads: they sum to 72 A, each small enough that its inrush fits under the line once the rest settled.
        const total = 72, cap = ((T - total) / (m - 1)) * 0.95;
        let a = [...Array(n)].map(() => 0.6 + r() * 0.8);
        for (let k = 0; k < 20; k++) {
          const s = a.reduce((x, y) => x + y, 0);
          a = a.map((x) => Math.min(cap, (x * total) / s));
        }
        const s = a.reduce((x, y) => x + y, 0);
        a = a.map((x) => (x * total) / s);
        const names = [...NAMES];
        for (let i = names.length - 1; i > 0; i--) { const j = Math.floor(r() * (i + 1)); [names[i], names[j]] = [names[j], names[i]]; }
        ways = a.map((x, i) => ({ a: Math.round(x * 10) / 10, name: names[i], on: false, since: 0 }));
        burnt = level >= 3 ? Math.floor(r() * n) : -1;
        tagged = false; main = false; mainY = MAIN.y0; grab = false; sel = 0; amps = 0; load = 0; trips = 0; flash = 0; spark = null; time = 0;
      },
      peek() { return { phase, n, closed: ways.filter((w) => w.on).length, amps, T, burnt, tagged, surging: surge() > 1, wayX, tagBox, main, ways: ways.map((w) => ({ a: w.a, on: w.on })) }; },
      update(dt, input) {
        time += dt;
        flash = Math.max(0, flash - dt * 1.6);
        if (spark) { spark.t -= dt; if (spark.t <= 0) spark = null; }
        for (const w of ways) if (w.on) w.since += dt;
        amps = main ? steady() + surge() : 0;
        // The needle reads the settled load and only climbs as ways close (owner, 2026-10-12: "needle is moving the
        // wrong way": a needle that jumped to the inrush and sank back read as the load falling); the inrush rides
        // above it as an amber arc that shrinks away. The trip still counts both.
        const want = main ? steady() : 0;
        load = want < load ? want : load + (want - load) * (1 - Math.exp(-dt / 0.12));
        if (amps > T) { trip(); return; }
        if (phase === "part") {
          // The part: the new main switch, from the crate into its slot.
          if (input.pressed && Math.abs(input.x - slot.x) < 70 && Math.abs(input.y - slot.y) < 60) slot.held = true;
          if (slot.held && input.down) { slot.x = input.x; slot.y = input.y; }
          if (input.actionPressed) slot.set = true;
          if (slot.held && input.released) { slot.held = false; if (Math.abs(slot.x - (MAIN.x + MAIN.w / 2)) < 90 && Math.abs(slot.y - 250) < 110) slot.set = true; }
          if (slot.set) { phase = "main"; api.stepDone(); }
          return;
        }
        if (phase === "main") {
          // Drag the handle up, or Space.
          const hx = MAIN.x + MAIN.w / 2;
          if (input.pressed && Math.abs(input.x - hx) < 70 && Math.abs(input.y - mainY) < 50) grab = true;
          if (grab && input.down) mainY = Math.max(MAIN.y1, Math.min(MAIN.y0, input.y));
          if (!input.down) grab = false;
          if (input.actionPressed || input.stick.y < 0) mainY = Math.max(MAIN.y1, mainY - (input.actionPressed ? 999 : 300 * dt));
          if (mainY <= MAIN.y1 + 2) { mainY = MAIN.y1; main = true; grab = false; phase = "ways"; }
          return;
        }
        if (phase === "ways") {
          if (input.hit.has("ArrowRight") || input.hit.has("KeyD")) sel = (sel + 1) % n;
          if (input.hit.has("ArrowLeft") || input.hit.has("KeyA")) sel = (sel + n - 1) % n;
          let close = -1, tag = false;
          if (input.pressed) {
            if (burnt >= 0) { const b = tagBox(burnt); if (input.x >= b.x - 12 && input.x <= b.x + b.w + 12 && input.y >= b.y - 12 && input.y <= b.y + b.h + 12) tag = true; }
            if (!tag) close = KIT.nearest(ways.map((w, i) => (w.on ? null : [wayX(i), WAYS.top + WAYS.h / 2])), input.x, input.y, 46);
          }
          if (input.actionPressed) { if (sel === burnt && !tagged) tag = true; else if (!ways[sel].on) close = sel; }
          if (tag && burnt >= 0) tagged = true;
          if (close >= 0) {
            sel = close;
            if (close === burnt) {
              spark = { x: wayX(close), y: WAYS.top + 30, t: 0.5 }; flash = 0.8;
              api.fumble("A burnt way: a spark, 5 HP");
            } else { ways[close].on = true; ways[close].since = 0; }
          }
          const good = ways.every((w, i) => i === burnt || w.on);
          if (good && (burnt < 0 || tagged) && surge() < 0.5) { phase = "done"; api.stepDone(); }
        }
      },
      draw(g, t, dt, input) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        // The panel: steel, its back plate, the busbar behind the ways.
        D.panel(g, BOX.x, BOX.y, BOX.w, BOX.h, 10, "#1b2433", "#3a4656");
        D.round(g, BOX.x + 24, BOX.y + 24, BOX.w - 48, BOX.h - 48, 6); g.fillStyle = "#121821"; g.fill();
        g.fillStyle = "#2b2014"; g.fillRect(WAYS.x0, WAYS.top - 34, WAYS.x1 - WAYS.x0, 14); g.fillStyle = C.copper; g.fillRect(WAYS.x0, WAYS.top - 34, WAYS.x1 - WAYS.x0, 5);
        // The main switch: its slot, its handle (up is on), its word.
        D.round(g, MAIN.x, MAIN.y1 - 40, MAIN.w, MAIN.y0 - MAIN.y1 + 90, 8); g.fillStyle = "#0b1017"; g.fill(); g.lineWidth = 2; g.strokeStyle = "#3a4656"; g.stroke();
        if (slot.set) {
          D.round(g, MAIN.x + 18, mainY - 26, MAIN.w - 36, 52, 6); g.fillStyle = main ? C.ok : "#c8c2b4"; g.fill();
          g.fillStyle = "#10151c"; g.fillRect(MAIN.x + 30, mainY - 4, MAIN.w - 60, 8);
        } else { g.setLineDash([10, 8]); D.round(g, MAIN.x + 8, MAIN.y1 - 30, MAIN.w - 16, MAIN.y0 - MAIN.y1 + 70, 6); g.lineWidth = 3; g.strokeStyle = C.amber; g.stroke(); g.setLineDash([]); }
        D.text(g, "MAIN", MAIN.x + MAIN.w + 20, MAIN.y1 - 6, 18, C.dim, "left", 700);
        D.text(g, main ? "ON" : "OFF", MAIN.x + MAIN.w + 20, MAIN.y1 + 22, 28, main ? C.ok : C.dim, "left", 700);
        D.text(g, `${T} A`, MAIN.x + MAIN.w + 20, MAIN.y1 + 56, 20, C.amber, "left", 700);
        // The ways: a breaker each, its toggle up when closed, its load and current on a tag; the burnt one scorched.
        for (let i = 0; i < n; i++) {
          const x = wayX(i), w = ways[i], isB = i === burnt, chosen = phase === "ways" && sel === i;
          D.round(g, x - WAYS.w / 2, WAYS.top, WAYS.w, WAYS.h, 6); g.fillStyle = "#d7d2c6"; g.fill();
          g.lineWidth = chosen ? 3 : 1.5; g.strokeStyle = chosen ? C.accent : "#8a8473"; g.stroke();
          const ty = w.on ? WAYS.top + 26 : WAYS.top + 66;
          D.round(g, x - 12, WAYS.top + 22, 24, 84, 5); g.fillStyle = "#3a3f47"; g.fill();
          D.round(g, x - 14, ty - 4, 28, 40, 5); g.fillStyle = w.on ? C.ok : "#1b1f26"; g.fill();
          if (isB) {
            const sc = g.createRadialGradient(x, WAYS.top + 70, 4, x, WAYS.top + 70, 60);
            sc.addColorStop(0, "rgba(0,0,0,0.9)"); sc.addColorStop(1, "rgba(30,16,6,0)");
            g.fillStyle = sc; g.fillRect(x - 60, WAYS.top + 10, 120, 120);
          }
          D.round(g, x - 30, WAYS.top + WAYS.h + 8, 60, 32, 4); g.fillStyle = isB ? "#3a2a1a" : "#eef1f4"; g.fill();
          D.text(g, w.name, x, WAYS.top + WAYS.h + 18, 11, isB ? "#9a7a5a" : "#10151c", "center", 700);
          D.text(g, `${w.a.toFixed(0)} A`, x, WAYS.top + WAYS.h + 32, 13, isB ? "#9a7a5a" : "#10151c", "center", 700);
          if (isB) {
            const b = tagBox(i);
            D.round(g, b.x, b.y, b.w, b.h, 4); g.fillStyle = tagged ? C.danger : "#1b1f26"; g.fill(); g.lineWidth = 2; g.strokeStyle = C.danger; g.stroke();
            D.text(g, tagged ? "OUT" : "TAG", b.x + b.w / 2, b.y + b.h / 2 + 1, 15, tagged ? "#fff" : C.danger, "center", 700);
          }
        }
        if (spark) {
          for (let k = 0; k < 10; k++) {
            const a = (k / 10) * Math.PI * 2 + time * 7, rr = 14 + 30 * (1 - spark.t / 0.5);
            g.strokeStyle = k % 2 ? "#fff6c2" : C.warn; g.lineWidth = 3; g.beginPath();
            g.moveTo(spark.x + Math.cos(a) * 6, spark.y + Math.sin(a) * 6); g.lineTo(spark.x + Math.cos(a) * rr, spark.y + Math.sin(a) * rr); g.stroke();
          }
        }
        // The ammeter: its scale, the trip line and the red past it, the load, the inrush, the needle, the reading.
        D.panel(g, 800, 110, 420, 420, 18, "#0c121a");
        D.text(g, "PANEL CURRENT", 830, 140, 18, C.dim, "left", 700);
        g.beginPath(); g.arc(METER.x, METER.y, METER.r, A0, A1); g.strokeStyle = "#1d2738"; g.lineWidth = 24; g.stroke();
        D.hatchArc(g, METER.x, METER.y, METER.r - 12, METER.r + 12, ang(T), A1, C.danger);
        for (let a = 0; a <= MAXA; a += 25) {
          const q = ang(a);
          g.beginPath(); g.moveTo(METER.x + Math.cos(q) * (METER.r - 16), METER.y + Math.sin(q) * (METER.r - 16)); g.lineTo(METER.x + Math.cos(q) * (METER.r + 16), METER.y + Math.sin(q) * (METER.r + 16));
          g.strokeStyle = C.steel; g.lineWidth = 2; g.stroke();
          D.text(g, String(a), METER.x + Math.cos(q) * (METER.r - 40), METER.y + Math.sin(q) * (METER.r - 40), 15, C.dim, "center", 600);
        }
        const tq = ang(T);
        g.beginPath(); g.moveTo(METER.x + Math.cos(tq) * (METER.r - 22), METER.y + Math.sin(tq) * (METER.r - 22)); g.lineTo(METER.x + Math.cos(tq) * (METER.r + 22), METER.y + Math.sin(tq) * (METER.r + 22));
        g.strokeStyle = C.danger; g.lineWidth = 5; g.stroke();
        // The settled load as a fill along the scale, the inrush as an amber arc on from it (wait for it to go).
        if (load > 0.2) { g.beginPath(); g.arc(METER.x, METER.y, METER.r, A0, ang(load)); g.strokeStyle = "rgba(232,238,246,0.35)"; g.lineWidth = 24; g.stroke(); }
        if (amps - load > 0.5) { g.beginPath(); g.arc(METER.x, METER.y, METER.r, ang(load), ang(amps)); g.strokeStyle = amps > T * 0.9 ? C.danger : C.amber; g.lineWidth = 24; g.stroke(); }
        const nq = ang(load);
        g.beginPath(); g.moveTo(METER.x, METER.y); g.lineTo(METER.x + Math.cos(nq) * (METER.r - 8), METER.y + Math.sin(nq) * (METER.r - 8));
        g.strokeStyle = C.fg; g.lineWidth = 5; g.lineCap = "round"; g.stroke(); g.lineCap = "butt";
        D.disc(g, METER.x, METER.y, 12, "#566273");
        D.text(g, `${load.toFixed(0)} A`, METER.x, METER.y + 78, 38, C.fg, "center", 700);
        D.text(g, `${ways.filter((w) => w.on).length} / ${n - (burnt >= 0 ? 1 : 0)} WAYS`, 830, 505, 20, C.fg, "left", 700);
        // The crate with the spare main switch.
        if (part) {
          D.panel(g, CRATE.x, CRATE.y, CRATE.w, CRATE.h, 10, "#2b2014", "#5a4426");
          D.text(g, "SPARE MAIN SWITCH", CRATE.x + CRATE.w / 2, CRATE.y + 20, 16, C.dim, "center", 700);
          if (!slot.set) { D.round(g, slot.x - 50, slot.y - 40, 100, 80, 8); g.fillStyle = "#c8c2b4"; g.fill(); g.fillStyle = "#3a3f47"; g.fillRect(slot.x - 30, slot.y - 6, 60, 12); }
        }
        if (flash > 0) { g.fillStyle = `rgba(255,236,170,${0.3 * flash})`; g.fillRect(0, api.BAR_H, api.W, api.H); }
      },
    };
  },
});
