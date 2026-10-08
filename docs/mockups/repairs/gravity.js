/*
 * repairs/gravity.js: the gravity generator's repair (openspec/changes/repair-minigames, design 2).
 *
 * Three field rings turn at their own speeds, each with a bright arc. Stop each ring (click, or Space) when its arc
 * lies under the marker at the top, outer ring first. A step is the three rings set; the next step's rings turn
 * faster. Stopped off the mark: a fumble, the field lurches. A disabled generator's first step fits the new field coil:
 * drag it from the crate into its socket at the hub.
 */
RepairKit.register({
  id: "gravity",
  title: "Gravity generator",
  place: "Engineering, deck C",
  group: "Engineering",
  hazard: "The field lurches: everyone near floats for 2 s",
  down: "Everyone aboard floats, slowly (design 4)",
  create(api) {
    const { C, KIT } = api, D = KIT.draw;
    const CX = 640, CY = 410, RADII = [250, 185, 120];
    const ARC = [0.42, 0.36, 0.3];          // the bright arc's half width, radians: inner rings are tighter
    const MARK = -Math.PI / 2;              // the marker: straight up
    let rings, active, part, coil, lurch;
    const wrap = (a) => Math.atan2(Math.sin(a), Math.cos(a));
    return {
      step(index, isPart) {
        const r = api.rand();
        part = isPart;
        coil = { x: 1080, y: 560, held: false, set: false };
        rings = RADII.map((rad, i) => ({
          rad, angle: r() * Math.PI * 2,
          speed: (0.9 + 0.35 * index + 0.25 * i) * (r() < 0.5 ? -1 : 1),
          set: false,
        }));
        active = 0;
        lurch = 0;
      },
      update(dt, input) {
        lurch = Math.max(0, lurch - dt);
        if (part && !coil.set) {
          // The part step: carry the coil to the hub.
          if (input.pressed && Math.hypot(input.x - coil.x, input.y - coil.y) < 50) coil.held = true;
          if (coil.held && input.down) { coil.x = input.x; coil.y = input.y; }
          if (coil.held && input.released) {
            coil.held = false;
            if (Math.hypot(coil.x - CX, coil.y - CY) < 40) { coil.set = true; coil.x = CX; coil.y = CY; api.stepDone(); }
          }
          return;
        }
        for (const g of rings) if (!g.set) g.angle += g.speed * dt;
        if (active < rings.length && (input.pressed || input.actionPressed)) {
          const g = rings[active];
          const off = Math.abs(wrap(g.angle - MARK));
          if (off <= ARC[active]) {
            g.set = true; g.angle = MARK; active++;
            if (active === rings.length) api.stepDone();
          } else {
            lurch = 0.8;
            api.fumble("Off the mark: the field lurches");
          }
        }
      },
      draw(g, t) {
        // The generator from above: a dark well, the rings, the hub, the marker.
        g.fillStyle = "#070b12"; g.fillRect(0, api.BAR_H, api.W, api.H);
        const bob = lurch > 0 ? Math.sin(t * 30) * 6 * lurch : 0;
        g.save(); g.translate(0, bob);
        D.disc(g, CX, CY, 290, "#0b111b");
        D.ring(g, CX, CY, 290, C.line, 3);
        rings.forEach((r, i) => {
          D.ring(g, CX, CY, r.rad, i === active && !part ? "#2c3a52" : "#1b2433", 26);
          g.beginPath();
          g.arc(CX, CY, r.rad, r.angle - ARC[i], r.angle + ARC[i]);
          g.strokeStyle = r.set ? C.ok : i === active && !part ? C.accent : "#3d6f8f";
          g.lineWidth = 22; g.lineCap = "round"; g.stroke(); g.lineCap = "butt";
        });
        // The hub and, on the part step, its empty socket.
        D.disc(g, CX, CY, 46, part && !coil.set ? "#120d08" : "#1d2738");
        D.ring(g, CX, CY, 46, part && !coil.set ? C.amber : C.steel, 4);
        const glow = rings.filter((r) => r.set).length / 3;
        D.disc(g, CX, CY, 26, `rgba(79,195,247,${0.25 + 0.6 * glow})`);
        // The marker at the top.
        g.beginPath(); g.moveTo(CX, CY - 300); g.lineTo(CX - 16, CY - 330); g.lineTo(CX + 16, CY - 330); g.closePath();
        g.fillStyle = C.amber; g.fill();
        g.fillStyle = "rgba(242,160,70,0.18)"; g.fillRect(CX - 3, CY - 300, 6, 210);
        if (part && !coil.set) {
          D.panel(g, 1010, 500, 140, 120, 14, "#141b27");
          D.disc(g, coil.x, coil.y, 34, C.copper);
          D.ring(g, coil.x, coil.y, 34, "#f0c08a", 3);
          D.ring(g, coil.x, coil.y, 20, "#7a4a22", 6);
        }
        g.restore();
      },
    };
  },
});
