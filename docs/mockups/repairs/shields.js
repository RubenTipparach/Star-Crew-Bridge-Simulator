/*
 * repairs/shields.js: the shield generator's repair (openspec/changes/repair-minigames, design 2).
 *
 * The generator from above: the core, six emitter segments on a ring, one for each shield face, and the field's six
 * arcs outside them. A detuned segment hums out of step with the master wave. Its wave and the master's are laid over
 * each other on the scope; turn the phase dial and the gain dial (drag them, or the arrows: left and right for phase,
 * up and down for gain, or the wheel for gain) until the two lie together, and hold them there while the lock fills.
 * A step is one pair of segments matched; the next step's segments wander faster and the match is tighter. Gain
 * driven into the hatched red zone is a fumble: the emitter crackles and its face drains. A disabled generator's first
 * step fits a new emitter: drag it from the crate into the empty seat on the ring.
 */
RepairKit.register({
  id: "shields",
  title: "Shield generator",
  place: "Deck C, under the drive",
  group: "Engineering",
  hazard: "Emitter crackle: the face drains",
  // The how-to card (repair-minigames 6g), drawn by the kit: pictures and a few words, on demand.
  guide: {
    steps: [
      { icon: "turn", text: "Phase dial moves the wave" },
      { icon: "turn", text: "Gain dial sizes it" },
      { icon: "match", text: "Lay it on the master" },
      { icon: "hold", text: "Hold while the lock fills" },
    ],
    mistake: "Gain in the hatched red: a crackle, the face drains",
  },
  down: "That shield face is down",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const CX = 330, CY = 405, POD_R = 168, FIELD_R = 238;
    const FACES = ["FWD", "STBD", "DOR", "AFT", "VEN", "PORT"];
    const ANG = (i) => -Math.PI / 2 + (i * Math.PI) / 3;
    const SC = { x: 660, y: 100, w: 580, h: 310 };       // the scope
    const PH = { x: 820, y: 575, r: 82 };                 // the phase dial
    const GA = { x: 1090, y: 575, r: 82 };                // the gain dial
    const G_MAX = 2, G_RED = 1.5;                         // gain: the dial's top, the over-drive line (master = 1)
    const G_A0 = Math.PI * 0.75, G_SWEEP = Math.PI * 1.5; // the gain dial's sweep, radians from +x
    const HOLD = 1.2;                                     // s matched to lock a segment
    let part, pod, segs, pair, active, ok, tolP, tolA, wander, grab, lastA, crackle, time;
    const wrap = (a) => Math.atan2(Math.sin(a), Math.cos(a));
    const angAt = (d, x, y) => Math.atan2(y - d.y, x - d.x);

    function matched(s) { return Math.abs(wrap(s.ph)) < tolP && Math.abs(s.amp - 1) < tolA; }

    return {
      step(index, isPart) {
        const r = api.rand();
        part = isPart;
        pod = { x: 575, y: 648, held: false, set: false };
        tolP = Math.max(0.14, 0.24 - 0.025 * index);
        tolA = Math.max(0.08, 0.13 - 0.012 * index);
        wander = 0.08 + 0.06 * index;
        const k = isPart ? 0 : index;
        pair = [(2 * k) % 6, (2 * k + 1) % 6];
        // Faces tuned earlier in the job hold; this step's pair and the later ones are out.
        ok = FACES.map((_, i) => (k >= 3 ? !pair.includes(i) : i < 2 * k));
        segs = FACES.map(() => {
          let ph = (r() < 0.5 ? -1 : 1) * (1.0 + r() * 1.8), amp = 0.35 + r() * 0.95;
          if (Math.abs(amp - 1) < 0.3) amp = r() < 0.5 ? 0.5 : 1.35;
          return { ph, amp, drift: (r() < 0.5 ? -1 : 1) * wander, hold: 0, locked: false, w: 3 + r() * 0.6 };
        });
        active = pair[0];
        grab = null; crackle = 0; time = 0;
      },
      /** For tools (shots and tests): the round's state, read only. */
      peek() { const s = segs[active]; return { part, podSet: pod.set, active, pair: [...pair], ph: wrap(s.ph), amp: s.amp, locked: pair.map((i) => segs[i].locked) }; },
      update(dt, input) {
        crackle = Math.max(0, crackle - dt * 1.5);
        if (part && !pod.set) {
          // The part step: carry the emitter to its seat.
          const sx = CX + Math.cos(ANG(0)) * POD_R, sy = CY + Math.sin(ANG(0)) * POD_R;
          if (input.pressed && Math.hypot(input.x - pod.x, input.y - pod.y) < 50) pod.held = true;
          if (pod.held && input.down) { pod.x = input.x; pod.y = input.y; }
          if (pod.held && input.released) {
            pod.held = false;
            if (Math.hypot(pod.x - sx, pod.y - sy) < 50) { pod.set = true; pod.x = sx; pod.y = sy; api.stepDone(); }
          }
          return;
        }
        if (part) return;
        time += dt;
        const s = segs[active];
        if (s.locked) return;
        // Clicking the other segment of the pair takes it instead.
        if (input.pressed) {
          for (const i of pair) {
            const x = CX + Math.cos(ANG(i)) * POD_R, y = CY + Math.sin(ANG(i)) * POD_R;
            if (!segs[i].locked && Math.hypot(input.x - x, input.y - y) < 50) { active = i; return; }
          }
          if (Math.hypot(input.x - PH.x, input.y - PH.y) < PH.r + 20) { grab = PH; lastA = angAt(PH, input.x, input.y); }
          else if (Math.hypot(input.x - GA.x, input.y - GA.y) < GA.r + 20) { grab = GA; lastA = angAt(GA, input.x, input.y); }
        }
        if (grab && input.down) {
          // The dials turn with the pointer's angle round them, from where they were grabbed.
          const a = angAt(grab, input.x, input.y), d = wrap(a - lastA); lastA = a;
          if (grab === PH) s.ph += d; else s.amp += (d / G_SWEEP) * G_MAX;
        }
        if (!input.down) grab = null;
        s.ph += input.stick.x * 1.4 * dt;
        s.amp += -input.stick.y * 0.5 * dt - input.wheel * 0.05;
        // The segment wanders on its own.
        s.ph += s.drift * dt + 0.05 * Math.sin(time * 1.7) * dt;
        s.amp = Math.max(0, Math.min(G_MAX, s.amp + 0.04 * Math.sin(time * 1.3) * dt));
        if (s.amp > G_RED) {
          crackle = 1; s.amp = 0.4; s.hold = 0; grab = null;
          api.fumble("Emitter crackle: the face drains");
          return;
        }
        if (matched(s)) s.hold += dt; else s.hold = Math.max(0, s.hold - dt * 2);
        if (s.hold >= HOLD) {
          s.locked = true; s.ph = 0; s.amp = 1;
          const rest = pair.find((i) => !segs[i].locked);
          if (rest === undefined) api.stepDone(); else active = rest;
        }
      },
      draw(g, t) {
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        const live = !part;
        const state = (i) => (part ? (i === 0 && !pod.set ? "empty" : "ok") : segs[i].locked || ok[i] ? "ok" : pair.includes(i) ? "pair" : "bad");

        // The field: six arcs, solid when the face holds, broken when it is out.
        FACES.forEach((_, i) => {
          const st = state(i), a = ANG(i);
          g.beginPath(); g.arc(CX, CY, FIELD_R, a - 0.46, a + 0.46);
          if (st === "ok") { g.strokeStyle = "rgba(79,195,247,0.75)"; g.setLineDash([]); }
          else { g.strokeStyle = st === "pair" ? "rgba(242,160,70,0.6)" : "rgba(255,71,87,0.55)"; g.setLineDash([12, 10]); }
          g.lineWidth = 8; g.stroke(); g.setLineDash([]);
        });
        // The ring, the busbars and the core.
        D.ring(g, CX, CY, POD_R, "#1b2433", 30);
        D.ring(g, CX, CY, POD_R, "#2b3646", 2);
        FACES.forEach((_, i) => {
          const a = ANG(i);
          g.beginPath(); g.moveTo(CX + Math.cos(a) * 56, CY + Math.sin(a) * 56); g.lineTo(CX + Math.cos(a) * (POD_R - 40), CY + Math.sin(a) * (POD_R - 40));
          g.strokeStyle = "#3a4656"; g.lineWidth = 10; g.stroke();
          g.strokeStyle = C.copper; g.lineWidth = 3; g.stroke();
        });
        D.disc(g, CX, CY, 56, "#141c28"); D.ring(g, CX, CY, 56, "#3a4656", 4);
        const lockedN = live ? FACES.filter((_, i) => state(i) === "ok").length : 5;
        const core = g.createRadialGradient(CX, CY, 0, CX, CY, 44);
        core.addColorStop(0, `rgba(160,220,255,${0.5 + 0.08 * lockedN})`); core.addColorStop(1, "rgba(79,195,247,0)");
        D.disc(g, CX, CY, 44, core);

        // The emitter pods: a block, a dish, a lamp whose shape says its state.
        FACES.forEach((name, i) => {
          const st = state(i), a = ANG(i);
          const x = CX + Math.cos(a) * POD_R, y = CY + Math.sin(a) * POD_R;
          if (st === "empty") {
            g.setLineDash([7, 6]); D.round(g, x - 40, y - 34, 80, 68, 12); g.lineWidth = 2; g.strokeStyle = C.amber; g.stroke(); g.setLineDash([]);
            D.text(g, name, CX + Math.cos(a) * (POD_R + 104), CY + Math.sin(a) * (POD_R + 104), 18, C.danger, "center", 700);
            return;
          }
          const isActive = live && i === active && !segs[i].locked;
          D.panel(g, x - 40, y - 34, 80, 68, 12, "#1b2433", isActive ? C.amber : "#3a4656");
          D.disc(g, x, y, 22, "#0e151f"); D.ring(g, x, y, 22, "#566273", 3);
          D.disc(g, x, y, 8, st === "ok" ? C.accent : "#33404f");
          const lx = x + 28, ly = y - 22;
          if (st === "ok") D.disc(g, lx, ly, 7, C.ok);
          else if (st === "pair") { g.beginPath(); g.moveTo(lx, ly - 8); g.lineTo(lx + 8, ly + 6); g.lineTo(lx - 8, ly + 6); g.closePath(); g.fillStyle = C.amber; g.fill(); }
          else { g.save(); g.translate(lx, ly); g.rotate(Math.PI / 4); g.fillStyle = C.danger; g.fillRect(-7, -2, 14, 4); g.fillRect(-2, -7, 4, 14); g.restore(); }
          if (isActive) {
            const h = segs[i].hold / HOLD;
            D.ring(g, x, y, 52, "#1d2636", 6);
            if (h > 0) { g.beginPath(); g.arc(x, y, 52, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * h); g.strokeStyle = C.ok; g.lineWidth = 6; g.stroke(); }
          }
          if (isActive && crackle > 0) {
            g.strokeStyle = `rgba(255,255,255,${crackle})`; g.lineWidth = 2;
            for (let k = 0; k < 6; k++) {
              const b = t * 40 + k;
              g.beginPath(); g.moveTo(x, y);
              for (let j = 1; j <= 4; j++) g.lineTo(x + Math.cos(b) * j * 16 + Math.sin(b * 3 + j) * 8, y + Math.sin(b) * j * 16 + Math.cos(b * 2 + j) * 8);
              g.stroke();
            }
          }
          const tx = CX + Math.cos(a) * (POD_R + 104), ty = CY + Math.sin(a) * (POD_R + 104);
          D.text(g, name, tx, ty, 18, st === "ok" ? C.dim : st === "pair" ? C.amber : C.danger, "center", 700);
        });

        // The scope: the master wave as a broad band, the segment's wave as a line over it.
        D.panel(g, SC.x, SC.y, SC.w, SC.h, 18, "#060c10", "#1f3a33");
        g.save(); D.round(g, SC.x, SC.y, SC.w, SC.h, 18); g.clip();
        g.strokeStyle = "rgba(61,220,132,0.08)"; g.lineWidth = 1;
        for (let x = SC.x; x < SC.x + SC.w; x += 40) { g.beginPath(); g.moveTo(x, SC.y); g.lineTo(x, SC.y + SC.h); g.stroke(); }
        for (let y = SC.y + 15; y < SC.y + SC.h; y += 40) { g.beginPath(); g.moveTo(SC.x, y); g.lineTo(SC.x + SC.w, y); g.stroke(); }
        g.fillStyle = "rgba(255,71,87,0.14)";
        const mid = SC.y + SC.h / 2, A = 72;
        g.fillRect(SC.x, SC.y, SC.w, mid - A * G_RED - SC.y); g.fillRect(SC.x, mid + A * G_RED, SC.w, SC.y + SC.h - mid - A * G_RED);
        const wave = (amp, ph, w) => { g.beginPath(); for (let x = 0; x <= SC.w; x += 4) { const y = mid - A * amp * Math.sin((x / SC.w) * Math.PI * 2 * 2 - t * w + ph); x ? g.lineTo(SC.x + x, y) : g.moveTo(SC.x + x, y); } };
        wave(1, 0, 3); g.strokeStyle = "rgba(79,195,247,0.35)"; g.lineWidth = 16; g.stroke();
        if (live) {
          const s = segs[active];
          wave(s.amp, s.ph, 3); g.strokeStyle = s.locked || matched(s) ? C.ok : C.amber; g.lineWidth = 4; g.stroke();
        }
        g.restore();

        // The dials: phase turns freely; gain sweeps to a hatched red zone at its top.
        const dial = (d, a, on) => {
          D.disc(g, d.x, d.y, d.r + 14, "#0c121a"); D.ring(g, d.x, d.y, d.r + 14, on ? C.amber : C.line, 3);
          for (let k = 0; k < 24; k++) {
            const b = (k / 24) * Math.PI * 2;
            g.beginPath(); g.moveTo(d.x + Math.cos(b) * (d.r + 4), d.y + Math.sin(b) * (d.r + 4)); g.lineTo(d.x + Math.cos(b) * (d.r + 10), d.y + Math.sin(b) * (d.r + 10));
            g.strokeStyle = "#3b4a5e"; g.lineWidth = 2; g.stroke();
          }
          D.disc(g, d.x, d.y, d.r - 10, "#1b2433"); D.ring(g, d.x, d.y, d.r - 10, "#566273", 3);
          for (let k = 0; k < 12; k++) { const b = (k / 12) * Math.PI * 2 + a; D.disc(g, d.x + Math.cos(b) * (d.r - 18), d.y + Math.sin(b) * (d.r - 18), 4, "#2b3646"); }
          g.beginPath(); g.moveTo(d.x, d.y); g.lineTo(d.x + Math.cos(a) * (d.r - 16), d.y + Math.sin(a) * (d.r - 16));
          g.strokeStyle = C.fg; g.lineWidth = 6; g.lineCap = "round"; g.stroke(); g.lineCap = "butt";
          D.disc(g, d.x, d.y, 12, "#3a4656");
        };
        const s = live ? segs[active] : { ph: 0, amp: 1 };
        // The gain's red zone, hatched, outside the dial.
        const redA = G_A0 + (G_RED / G_MAX) * G_SWEEP, endA = G_A0 + G_SWEEP;
        g.save(); g.beginPath(); g.arc(GA.x, GA.y, GA.r + 28, redA, endA); g.arc(GA.x, GA.y, GA.r + 16, endA, redA, true); g.closePath(); g.clip();
        g.fillStyle = "rgba(255,71,87,0.35)"; g.fillRect(GA.x - 130, GA.y - 130, 260, 260);
        g.strokeStyle = C.danger; g.lineWidth = 3;
        for (let x = -140; x < 140; x += 10) { g.beginPath(); g.moveTo(GA.x + x, GA.y - 130); g.lineTo(GA.x + x + 40, GA.y + 130); g.stroke(); }
        g.restore();
        g.beginPath(); g.arc(GA.x, GA.y, GA.r + 22, G_A0, redA); g.strokeStyle = "#2b3646"; g.lineWidth = 12; g.stroke();
        dial(PH, -Math.PI / 2 + s.ph, grab === PH);
        dial(GA, G_A0 + (s.amp / G_MAX) * G_SWEEP, grab === GA);
        D.text(g, "PHASE", PH.x, PH.y + PH.r + 42, 20, C.dim, "center", 700);
        D.text(g, "GAIN", GA.x, GA.y + GA.r + 42, 20, C.dim, "center", 700);

        // The part: the new emitter in its crate, or in the hand.
        if (part && !pod.set) {
          D.panel(g, 505, 595, 140, 106, 14, "#141b27");
          D.panel(g, pod.x - 40, pod.y - 34, 80, 68, 12, "#2b3646", "#7d8796");
          D.disc(g, pod.x, pod.y, 22, "#0e151f"); D.ring(g, pod.x, pod.y, 22, "#b8c2d0", 3);
          D.disc(g, pod.x, pod.y, 8, C.accent);
        }
      },
    };
  },
});
