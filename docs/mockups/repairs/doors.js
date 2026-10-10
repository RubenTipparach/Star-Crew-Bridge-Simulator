/*
 * repairs/doors.js: a jammed door's repair (openspec/changes/repair-minigames, design 2).
 *
 * Any door on the ship, seen from the corridor: the leaf in its frame, hung from a track over the opening, driven by
 * a chain from the hand crank on the right; above the crank, the strain gauge with its slip limit hatched red. Turn
 * the crank clockwise (drag round in circles, or alternate Left and Right) and the leaf slides open along its track
 * into the wall's pocket. The leaf fights back in waves: it shudders as a wave builds, and turning hard into a wave
 * drives the gauge up. Past the slip limit the crank slips: a fumble, the leaf drops back part of the way. Ease off
 * through each wave and wind on between them. The lamp over the door shows red and crossed while it is stuck, green
 * and ticked once it is open. Later doors need more turns and fight back harder and more often. A disabled door's
 * first step fits a new drive pinion: drag it from the crate onto the crank's shaft.
 */
RepairKit.register({
  id: "doors",
  title: "Doors",
  place: "Any door, jammed",
  group: "Outside and doors",
  hazard: "The leaf drops back",
  // The how-to card (repair-minigames 6g), drawn by the kit: pictures and a few words, on demand.
  guide: {
    steps: [
      { icon: "turn", text: "Crank round, clockwise" },
      { icon: "band", text: "Ease off when it shudders" },
      { icon: "rhythm", text: "Wind on between waves" },
    ],
    mistake: "The gauge past its limit: the crank slips, the leaf drops",
  },
  down: "The door is stuck",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const OPEN = { x0: 250, x1: 650, y0: 170, y1: 680 };   // the doorway
    const TRAVEL = 380;                                     // how far the leaf slides into its pocket, px
    const HUB = { x: 1060, y: 520 }, WHEEL = 130;           // the crank
    const GAUGE = { x: 1060, y: 250, r: 118, max: 1.3 };    // the strain gauge: 0 to 1.3, slip at 1
    const W_REF = 8;                                        // rad/s that strains the crank to its limit at rest
    let hadPart = false, part, pinion, hard;
    let crank, omega, strain, prog, shown, turns, waves, amp, rt, lock, dragA, lastKey, played, wobble;
    const clamp = (v, a, b) => Math.max(a, Math.min(b, v));
    const wrap = (a) => Math.atan2(Math.sin(a), Math.cos(a));

    /** The leaf's resistance now: 1 at rest, rising through each wave. */
    function resistance(time) {
      let b = 0;
      for (const w of waves) if (time >= w.t && time < w.t + w.d) b = Math.max(b, Math.sin(Math.PI * (time - w.t) / w.d) ** 2);
      return 1 + amp * b;
    }



    /** A toothed wheel: the pinion, and the track's gear. */
    function gear(g, x, y, r, a, teeth, fill, stroke) {
      g.beginPath();
      for (let i = 0; i < teeth * 2; i++) {
        const ang = a + (i / (teeth * 2)) * Math.PI * 2, rr = i % 2 ? r : r + 7;
        const a2 = ang + Math.PI / (teeth * 2);
        g.lineTo(x + Math.cos(ang) * rr, y + Math.sin(ang) * rr); g.lineTo(x + Math.cos(a2) * rr, y + Math.sin(a2) * rr);
      }
      g.closePath(); g.fillStyle = fill; g.fill(); g.strokeStyle = stroke; g.lineWidth = 2; g.stroke();
    }

    return {
      step(index, isPart) {
        const r = api.rand();
        if (isPart) hadPart = true;
        part = isPart;
        hard = index;   // the round's level - 1 (repair-minigames 1a)
        pinion = { x: 835, y: 635, held: false, set: false };
        turns = Math.PI * 2 * (5 + hard);
        amp = 1.8 + 0.6 * hard;
        waves = []; let tt = 1.0 + r();
        for (let i = 0; i < 40; i++) { const d = 1.2 + 0.4 * r(); waves.push({ t: tt, d }); tt += d + Math.max(0.6, 1.4 - 0.2 * hard) + r() * 1.2; }
        crank = 0; omega = 0; strain = 0; prog = 0; shown = 0; rt = 0; lock = 0; dragA = null; lastKey = ""; played = false; wobble = 0;
      },
      /** For tools (shots and tests): the round's state, read only, so a script can play it. */
      peek() { return { part, pinion: { ...pinion }, HUB, prog, strain, lock, omega, R: resistance(rt), played }; },
      update(dt, input) {
        if (part && !pinion.set) {
          if (input.pressed && Math.hypot(input.x - pinion.x, input.y - pinion.y) < 50) pinion.held = true;
          if (pinion.held && input.down) { pinion.x = input.x; pinion.y = input.y; }
          if (pinion.held && input.released) {
            pinion.held = false;
            if (Math.hypot(pinion.x - HUB.x, pinion.y - HUB.y) < 45) { pinion.set = true; pinion.x = HUB.x; pinion.y = HUB.y; api.stepDone(); }
          }
          return;
        }
        shown += (prog - shown) * Math.min(1, dt * 6);
        if (played || part) { strain = Math.max(0, strain - dt * 2); return; }
        rt += dt;
        const R = resistance(rt);
        wobble = R - 1;
        if (lock > 0) { lock -= dt; omega = 0; strain = Math.max(0, strain - dt * 2); return; }
        // The hands: a drag round the hub (clockwise only: the ratchet holds the other way), or alternate keys.
        let turned = 0;
        // A press takes the crank, and so does a hand still on it once a slip's lock or a step change has let it go.
        if ((input.pressed || (dragA === null && input.down)) && Math.hypot(input.x - HUB.x, input.y - HUB.y) < 220) dragA = Math.atan2(input.y - HUB.y, input.x - HUB.x);
        if (dragA !== null && input.down) {
          const a = Math.atan2(input.y - HUB.y, input.x - HUB.x);
          turned = Math.max(0, wrap(a - dragA)); dragA = a;
          omega += (turned / Math.max(dt, 1e-3) - omega) * Math.min(1, dt / 0.1);
        } else {
          dragA = null;
          for (const [k, side] of [["ArrowLeft", "L"], ["KeyA", "L"], ["ArrowRight", "R"], ["KeyD", "R"]]) {
            if (input.hit.has(k) && side !== lastKey) { omega += 1.8; lastKey = side; }
          }
          omega *= Math.exp(-2.5 * dt);
          turned = omega * dt;
        }
        crank += turned;
        prog = Math.min(1, prog + turned / turns);
        // The strain: how hard the hands drive the leaf against its resistance now.
        strain += (R * omega / W_REF - strain) * Math.min(1, dt / 0.12);
        if (strain > 1) {
          // Slipped: the leaf drops back and the crank spins free for a moment.
          prog = Math.max(0, prog - 0.2); lock = 0.8; omega = 0; dragA = null; strain = 1.15;
          api.fumble("The leaf drops back");
          return;
        }
        if (prog >= 1) { played = true; omega = 0; api.stepDone(); }
      },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        // ---- Beyond the door: the next corridor, lit.
        const cg = g.createLinearGradient(0, OPEN.y0, 0, OPEN.y1);
        cg.addColorStop(0, "#1c2a3c"); cg.addColorStop(1, "#0a1018");
        g.fillStyle = cg; g.fillRect(OPEN.x0, OPEN.y0, OPEN.x1 - OPEN.x0, OPEN.y1 - OPEN.y0);
        g.fillStyle = "rgba(255,226,170,0.55)"; g.fillRect(OPEN.x0 + 120, OPEN.y0 + 30, 160, 10);
        g.fillStyle = "#18222f"; g.fillRect(OPEN.x0, OPEN.y1 - 120, OPEN.x1 - OPEN.x0, 120);
        // ---- The leaf, slid along by the progress, shuddering through a wave.
        const sh = !played && !part ? Math.sin(t * 47) * 2.5 * Math.min(1, wobble / 2) : 0;
        const lx = OPEN.x0 - TRAVEL * shown + sh;
        g.fillStyle = "#4a5568"; g.fillRect(lx, OPEN.y0, OPEN.x1 - OPEN.x0, OPEN.y1 - OPEN.y0);
        g.fillStyle = "#56637a"; for (const yy of [260, 420, 560]) g.fillRect(lx + 20, yy, OPEN.x1 - OPEN.x0 - 70, 16);
        D.round(g, lx + 120, 210, 150, 34, 10); g.fillStyle = "#0e1622"; g.fill(); g.strokeStyle = C.steel; g.lineWidth = 3; g.stroke();
        // Hazard chevrons on the leading edge.
        g.save(); g.beginPath(); g.rect(lx + 360, OPEN.y0, 40, OPEN.y1 - OPEN.y0); g.clip();
        g.fillStyle = "#1a1408"; g.fillRect(lx + 360, OPEN.y0, 40, OPEN.y1 - OPEN.y0);
        g.fillStyle = C.amber;
        for (let y = OPEN.y0 - 40; y < OPEN.y1; y += 40) { g.beginPath(); g.moveTo(lx + 360, y); g.lineTo(lx + 400, y + 20); g.lineTo(lx + 400, y + 40); g.lineTo(lx + 360, y + 20); g.closePath(); g.fill(); }
        g.restore();
        // ---- The wall around it, covering the pocket the leaf slides into.
        g.fillStyle = "#141b27";
        g.fillRect(0, api.BAR_H, OPEN.x0, api.H); g.fillRect(OPEN.x1, api.BAR_H, 800 - OPEN.x1, api.H);
        g.fillRect(0, api.BAR_H, 800, OPEN.y0 - api.BAR_H); g.fillRect(0, OPEN.y1, 800, api.H - OPEN.y1);
        g.fillStyle = "#1a2232"; for (let x = 20; x < 800; x += 120) if (x < OPEN.x0 - 30 || x > OPEN.x1 + 10) g.fillRect(x, 200, 6, 470);
        // The frame: jambs and a lintel standing proud of the wall.
        g.strokeStyle = C.steel; g.lineWidth = 6; g.strokeRect(OPEN.x0 - 6, OPEN.y0 - 6, OPEN.x1 - OPEN.x0 + 12, OPEN.y1 - OPEN.y0 + 12);
        // The track over the opening with its hangers and the gear that drives it.
        g.fillStyle = "#2a3446"; g.fillRect(0, 128, 720, 16);
        for (const hx of [lx + 60, lx + 340]) if (hx > 0) { D.disc(g, hx, 136, 10, "#8792a3"); g.fillStyle = "#56637a"; g.fillRect(hx - 3, 136, 6, OPEN.y0 - 136); }
        gear(g, 700, 136, 20, crank * 0.6, 10, "#3a4558", C.steel);
        // The lamp over the door: red and crossed while stuck, green and ticked once open.
        const open = prog >= 1;
        D.disc(g, 450, 108, 16, open ? C.ok : C.danger);
        g.strokeStyle = "#0b0f15"; g.lineWidth = 4; g.beginPath();
        if (open) { g.moveTo(442, 108); g.lineTo(448, 115); g.lineTo(459, 101); } else { g.moveTo(443, 101); g.lineTo(457, 115); g.moveTo(457, 101); g.lineTo(443, 115); }
        g.stroke();
        // ---- The drive: the chain from the track's gear down the wall and across to the crank.
        g.strokeStyle = "#56637a"; g.lineWidth = 8; g.setLineDash([10, 6]); g.lineDashOffset = -crank * 20;
        g.beginPath(); g.moveTo(720, 136); g.lineTo(760, 136); g.lineTo(760, HUB.y); g.lineTo(HUB.x, HUB.y); g.stroke();
        g.setLineDash([]); g.lineDashOffset = 0;
        // ---- The crank.
        D.panel(g, 900, 380, 320, 300, 18, "#0f1620");
        const wheelIn = !part || pinion.set;
        if (wheelIn) {
          D.ring(g, HUB.x, HUB.y, WHEEL, lock > 0 ? C.danger : "#8792a3", 12);
          for (let i = 0; i < 3; i++) {
            const a = crank + (i / 3) * Math.PI * 2;
            g.strokeStyle = "#56637a"; g.lineWidth = 10;
            g.beginPath(); g.moveTo(HUB.x, HUB.y); g.lineTo(HUB.x + Math.cos(a) * WHEEL, HUB.y + Math.sin(a) * WHEEL); g.stroke();
          }
          gear(g, HUB.x, HUB.y, 30, crank, 12, "#3a4558", C.steel);
          const ka = crank;
          D.disc(g, HUB.x + Math.cos(ka) * (WHEEL - 22), HUB.y + Math.sin(ka) * (WHEEL - 22), 22, dragA !== null ? C.amber : "#c9d1dc");
          // The way it turns: an arrow on the rim.
          D.turnArrow(g, HUB.x, HUB.y, WHEEL + 22, -Math.PI / 2 - 0.5, -Math.PI / 2 + 0.4, C.dim);
        } else {
          g.setLineDash([8, 6]); D.ring(g, HUB.x, HUB.y, 40, C.amber, 4); g.setLineDash([]);
          D.disc(g, HUB.x, HUB.y, 12, "#3a4558");
        }
        // ---- The strain gauge: green, amber near the limit, hatched red past it, the needle.
        const ang = (v) => Math.PI + Math.PI * clamp(v / GAUGE.max, 0, 1);
        D.panel(g, 920, 110, 280, 170, 18, "#0f1620");
        g.lineWidth = 22;
        g.beginPath(); g.arc(GAUGE.x, GAUGE.y, GAUGE.r - 20, ang(0), ang(0.75)); g.strokeStyle = "rgba(61,220,132,0.45)"; g.stroke();
        g.beginPath(); g.arc(GAUGE.x, GAUGE.y, GAUGE.r - 20, ang(0.75), ang(1)); g.strokeStyle = "rgba(242,160,70,0.7)"; g.stroke();
        D.hatchArc(g, GAUGE.x, GAUGE.y, GAUGE.r - 31, GAUGE.r - 9, ang(1), ang(GAUGE.max), "rgba(255,71,87,0.9)");
        const la = ang(1);
        g.strokeStyle = C.danger; g.lineWidth = 5;
        g.beginPath(); g.moveTo(GAUGE.x + Math.cos(la) * (GAUGE.r - 36), GAUGE.y + Math.sin(la) * (GAUGE.r - 36)); g.lineTo(GAUGE.x + Math.cos(la) * (GAUGE.r + 2), GAUGE.y + Math.sin(la) * (GAUGE.r + 2)); g.stroke();
        const na = ang(strain);
        g.strokeStyle = strain > 1 ? C.danger : C.fg; g.lineWidth = 5; g.lineCap = "round";
        g.beginPath(); g.moveTo(GAUGE.x, GAUGE.y); g.lineTo(GAUGE.x + Math.cos(na) * (GAUGE.r - 14), GAUGE.y + Math.sin(na) * (GAUGE.r - 14)); g.stroke(); g.lineCap = "butt";
        D.disc(g, GAUGE.x, GAUGE.y, 10, "#3a4558");
        // ---- The part step: the crate with the new pinion.
        if (part && !pinion.set) {
          D.panel(g, 775, 575, 120, 120, 14, "#141b27");
          gear(g, pinion.x, pinion.y, 30, 0, 12, C.copper, "#f0c08a");
          D.disc(g, pinion.x, pinion.y, 9, "#7a4a22");
        }
      },
    };
  },
});
