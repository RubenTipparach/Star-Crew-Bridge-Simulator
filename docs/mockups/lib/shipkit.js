/*
 * shipkit.js: the one place a mockup turns a ship layout into three.js objects.
 *
 * Every Star Crew mockup reads the same layout (data/ships/<id>/layout.json), so
 * the bridge, the deck plan, the systems view and the exterior cannot disagree
 * about where a room or a door is (CLAUDE.md sections 8 and 11). This file is
 * the shared interpretation of that layout: palette, lighting states, the Pi 5
 * budget meter, room shells with door openings cut, the hull loft and labels.
 *
 * It is a classic script, not a module, so a mockup still works when opened
 * straight from disk. tools/mockups/inline.py copies it into each page between
 * the "INLINE shipkit" markers; edit it here, never in a page. Functions that
 * build geometry take THREE as their first argument because the page owns the
 * three.js import.
 *
 * Axes (from the layout): +X port, +Y up, +Z bow, metres.
 */
(function () {
  "use strict";

  /** The layout inlined into the page as <script id="ship-layout" type="application/json">. */
  function layout() {
    const el = document.getElementById("ship-layout");
    if (!el) throw new Error("shipkit: no #ship-layout script in the page (run tools/mockups/inline.py)");
    return JSON.parse(el.textContent);
  }

  /**
   * The Raspberry Pi 5 (1 GB) budget a mockup is measured against. These are
   * the provisional numbers from openspec/changes/engine-stack/design.md,
   * "The Pi 5 budget"; that table is the source, and this copy is checked
   * against its marker by tools/mockups/inline.py --check.
   */
  const PI_BUDGET = {
    board: "Pi 5 (1 GB)",
    triangles: 200000,       // visible triangles per frame, all passes
    drawCalls: 300,          // draw calls per frame, all passes
    textureMB: 96,           // texture memory
    viewscreenPx: [1024, 512], // render-to-texture size for the main viewscreen
    frameMs: 16.7,           // 60 frames a second target (33.3 ms floor), 3D at 1280x720
  };

  /** Named colour roles. Mockups use roles, not hex values (CLAUDE.md section 10). */
  const PALETTE = {
    space: 0x05070d,
    hullLight: 0x8f9aa8,
    hullDark: 0x4a5462,
    hullAccent: 0xc9643b,
    floor: 0x3a404a,
    floorCorridor: 0x454b55,
    wall: 0x6b7480,
    wallDark: 0x525a66,
    ceiling: 0x2c3138,
    trim: 0x9aa4b0,
    console: 0x2a3038,
    screen: 0x48d6ff,
    screenWarm: 0xffb347,
    glass: 0x9fd8ff,
    door: 0x8a6d3b,
    doorFrame: 0xd0a040,
    hazard: 0xe8c547,
    alert: 0xff2a2a,
    emergency: 0xff8a1c,
    lampWarm: 0xffe2b0,
    lampCool: 0xbfe0ff,
    // station roles, used for seats, labels and console accents
    role: {
      command: 0xf2f2f2,
      helm: 0x4fc3f7,
      tactical: 0xef5350,
      engineering: 0xffb300,
      science: 0x66bb6a,
      comms: 0xab47bc,
      flight_ops: 0x26c6da,
      gunner: 0xff7043,
    },
    // portal kinds, used by plan views and door markers (one table for the mockups and
    // tools/deck_plans.py, which parses it from here)
    portal: {
      door: 0xd0a040,
      pressure_door: 0xff8a1c,
      hatch: 0x9aa4b0,
      ladder: 0xbfe0ff,
      hoist: 0xffb347,
      window: 0x9fd8ff,
      bay_door: 0xff2a2a,
    },
    // compartment kinds, used by plan views
    kind: {
      room: 0x5c6b7a,
      corridor: 0x7d8792,
      bay: 0x4a6a8a,
      crawlspace: 0x6a5a4a,
      airlock: 0xb08a3a,
      pod: 0xa05050,
    },
  };

  /**
   * Interior lighting states. Every interior mockup shows at least normal and
   * red alert (CLAUDE.md section 11). In the engine these are baked vertex
   * colour sets blended by a per-compartment uniform; here they are light
   * colours and intensities.
   */
  const LIGHTING = {
    normal: { label: "Normal", ambient: 0x2a3340, ambientI: 0.55, lamp: 0xffe9c8, lampI: 1.0, strip: 0x9fd8ff, stripI: 0.6, fog: 0x0b0e14 },
    red_alert: { label: "Red alert", ambient: 0x2a0d10, ambientI: 0.45, lamp: 0xff3030, lampI: 0.75, strip: 0xff2020, stripI: 1.0, fog: 0x120406 },
    emergency: { label: "Emergency power", ambient: 0x120c08, ambientI: 0.3, lamp: 0xff8a1c, lampI: 0.35, strip: 0xff8a1c, stripI: 0.8, fog: 0x060403 },
  };

  // ---------------------------------------------------------------- lookups

  function byId(list, id) {
    for (const x of list) if (x.id === id) return x;
    return null;
  }
  function compartment(L, id) { return byId(L.compartments, id); }
  function portalsOf(L, compId) { return L.portals.filter((p) => p.between.indexOf(compId) >= 0); }
  function stationsIn(L, compId) { return (L.stations || []).filter((s) => s.compartment === compId); }
  function systemsIn(L, compId) { return (L.systems || []).filter((s) => s.compartment === compId); }
  function deckById(L, id) { return byId(L.decks, id); }

  /** Volume in cubic metres and floor area in square metres of a compartment. */
  function measure(comp) {
    let v = 0, a = 0;
    for (const b of comp.boxes) {
      const dx = b.x[1] - b.x[0], dy = b.y[1] - b.y[0], dz = b.z[1] - b.z[0];
      v += dx * dy * dz; a += dx * dz;
    }
    return { volume_m3: v, floor_m2: a };
  }

  /** Union bounds of a compartment's boxes. */
  function bounds(comp) {
    const r = { x: [Infinity, -Infinity], y: [Infinity, -Infinity], z: [Infinity, -Infinity] };
    for (const b of comp.boxes) for (const k of "xyz") { r[k][0] = Math.min(r[k][0], b[k][0]); r[k][1] = Math.max(r[k][1], b[k][1]); }
    return r;
  }
  function center(comp) {
    const b = bounds(comp);
    return [(b.x[0] + b.x[1]) / 2, (b.y[0] + b.y[1]) / 2, (b.z[0] + b.z[1]) / 2];
  }

  /** Half extents of a portal's opening on its two in-plane axes. */
  function portalHalf(p) {
    const s = p.size_m;
    if (p.axis === "x") return { z: s[0] / 2, y: s[1] / 2 };
    if (p.axis === "z") return { x: s[0] / 2, y: s[1] / 2 };
    return { x: s[0] / 2, z: s[1] / 2 };
  }

  // ------------------------------------------------------------ room shells

  const EPS = 1e-4;

  // Each face of a box: the axis it is normal to, which end, and its two in-plane axes (u, v).
  const FACES = [
    { key: "floor", axis: "y", end: 0, u: "x", v: "z", inward: [0, 1, 0] },
    { key: "ceiling", axis: "y", end: 1, u: "x", v: "z", inward: [0, -1, 0] },
    { key: "wall", axis: "x", end: 0, u: "z", v: "y", inward: [1, 0, 0] },
    { key: "wall", axis: "x", end: 1, u: "z", v: "y", inward: [-1, 0, 0] },
    { key: "wall", axis: "z", end: 0, u: "x", v: "y", inward: [0, 0, 1] },
    { key: "wall", axis: "z", end: 1, u: "x", v: "y", inward: [0, 0, -1] },
  ];

  /**
   * Rectangle minus holes, as a list of cells. Holes are clipped to the
   * rectangle. Cells come from the grid of every hole edge, so the result is
   * exact and has no slivers, at the price of a few extra triangles.
   */
  function rectMinusHoles(r, holes) {
    const hs = holes
      .map((h) => ({ u0: Math.max(h.u0, r.u0), u1: Math.min(h.u1, r.u1), v0: Math.max(h.v0, r.v0), v1: Math.min(h.v1, r.v1) }))
      .filter((h) => h.u1 - h.u0 > EPS && h.v1 - h.v0 > EPS);
    if (!hs.length) return [r];
    const us = [r.u0, r.u1], vs = [r.v0, r.v1];
    for (const h of hs) { us.push(h.u0, h.u1); vs.push(h.v0, h.v1); }
    const su = [...new Set(us.map((x) => +x.toFixed(4)))].sort((a, b) => a - b);
    const sv = [...new Set(vs.map((x) => +x.toFixed(4)))].sort((a, b) => a - b);
    const cells = [];
    for (let i = 0; i + 1 < su.length; i++) {
      for (let j = 0; j + 1 < sv.length; j++) {
        const cu = (su[i] + su[i + 1]) / 2, cv = (sv[j] + sv[j + 1]) / 2;
        if (hs.some((h) => cu > h.u0 && cu < h.u1 && cv > h.v0 && cv < h.v1)) continue;
        cells.push({ u0: su[i], u1: su[i + 1], v0: sv[j], v1: sv[j + 1] });
      }
    }
    // Merge runs along u in each row to keep the triangle count down.
    cells.sort((a, b) => a.v0 - b.v0 || a.u0 - b.u0);
    const merged = [];
    for (const c of cells) {
      const last = merged[merged.length - 1];
      if (last && Math.abs(last.v0 - c.v0) < EPS && Math.abs(last.v1 - c.v1) < EPS && Math.abs(last.u1 - c.u0) < EPS) last.u1 = c.u1;
      else merged.push({ ...c });
    }
    return merged;
  }

  /** The openings on one face of one box: portals on that plane, and faces shared with the compartment's other boxes. */
  function faceHoles(L, comp, box, face, opts) {
    const plane = box[face.axis][face.end];
    const holes = [];
    for (const p of portalsOf(L, comp.id)) {
      if (p.axis !== face.axis) continue;
      if (opts.skipKinds && opts.skipKinds.indexOf(p.kind) >= 0) continue;
      const c = { x: p.center_m[0], y: p.center_m[1], z: p.center_m[2] };
      const slab = face.axis === "y" ? 0.55 : EPS * 10;
      if (Math.abs(c[face.axis] - plane) > slab) continue;
      const h = portalHalf(p);
      holes.push({ u0: c[face.u] - h[face.u], u1: c[face.u] + h[face.u], v0: c[face.v] - h[face.v], v1: c[face.v] + h[face.v], portal: p });
    }
    for (const other of comp.boxes) {
      if (other === box) continue;
      const oEnd = 1 - face.end;
      if (Math.abs(other[face.axis][oEnd] - plane) > EPS) continue;
      holes.push({ u0: other[face.u][0], u1: other[face.u][1], v0: other[face.v][0], v1: other[face.v][1] });
    }
    return holes;
  }

  function pushQuad(pos, nor, col, face, plane, c, color) {
    const p = (u, v) => {
      const o = { x: 0, y: 0, z: 0 };
      o[face.axis] = plane; o[face.u] = u; o[face.v] = v;
      return [o.x, o.y, o.z];
    };
    const a = p(c.u0, c.v0), b = p(c.u1, c.v0), d = p(c.u1, c.v1), e = p(c.u0, c.v1);
    // Wind so the quad faces inward (into the room).
    const n = face.inward;
    const ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]], ad = [d[0] - a[0], d[1] - a[1], d[2] - a[2]];
    const cross = [ab[1] * ad[2] - ab[2] * ad[1], ab[2] * ad[0] - ab[0] * ad[2], ab[0] * ad[1] - ab[1] * ad[0]];
    const flip = cross[0] * n[0] + cross[1] * n[1] + cross[2] * n[2] < 0;
    const tri = flip ? [a, d, b, a, e, d] : [a, b, d, a, d, e];
    for (const v of tri) { pos.push(v[0], v[1], v[2]); nor.push(n[0], n[1], n[2]); col.push(color.r, color.g, color.b); }
  }

  /**
   * Build a compartment's inside surfaces (floor, ceiling, walls) with every
   * portal opening cut out. Returns { floor, ceiling, walls } BufferGeometries
   * with vertex colours, plus the list of openings for door props.
   *
   * opts.colors: { floor, ceiling, wall } as hex; opts.skipKinds: portal kinds
   * not to cut (for example ["window"] to keep a blank wall).
   */
  function roomShell(THREE, L, comp, opts) {
    opts = opts || {};
    const isCorridor = comp.kind === "corridor";
    const colors = Object.assign({ floor: isCorridor ? PALETTE.floorCorridor : PALETTE.floor, ceiling: PALETTE.ceiling, wall: PALETTE.wall }, opts.colors || {});
    const out = { floor: [[], [], []], ceiling: [[], [], []], wall: [[], [], []] };
    const openings = [];
    for (const box of comp.boxes) {
      for (const face of FACES) {
        const plane = box[face.axis][face.end];
        const rect = { u0: box[face.u][0], u1: box[face.u][1], v0: box[face.v][0], v1: box[face.v][1] };
        const holes = faceHoles(L, comp, box, face, opts);
        for (const h of holes) if (h.portal) openings.push({ portal: h.portal, axis: face.axis, plane, u: face.u, v: face.v, rect: h });
        const color = new THREE.Color(colors[face.key]);
        const bucket = out[face.key];
        for (const c of rectMinusHoles(rect, holes)) pushQuad(bucket[0], bucket[1], bucket[2], face, plane, c, color);
      }
    }
    const make = (b) => {
      const g = new THREE.BufferGeometry();
      g.setAttribute("position", new THREE.Float32BufferAttribute(b[0], 3));
      g.setAttribute("normal", new THREE.Float32BufferAttribute(b[1], 3));
      g.setAttribute("color", new THREE.Float32BufferAttribute(b[2], 3));
      return g;
    };
    return { floor: make(out.floor), ceiling: make(out.ceiling), walls: make(out.wall), openings };
  }

  // ------------------------------------------------------------------- hull

  /**
   * Loft the hull's octagonal sections into one low-poly BufferGeometry
   * (outward normals, flat shaded). opts.inflate_m grows it, for a shell drawn
   * around the interior.
   */
  function hullGeometry(THREE, L, opts) {
    opts = opts || {};
    const inflate = opts.inflate_m || 0;
    const secs = L.hull.sections.slice().sort((a, b) => a.z_m - b.z_m);
    const ring = (s) => {
      const hw = s.half_beam_m + inflate, top = s.top_m + inflate, bot = s.bottom_m - inflate, c = s.chamfer_m;
      // Eight points, counter-clockwise seen from the bow (+Z), starting at the top port corner.
      return [
        [hw - c, top], [-(hw - c), top], [-hw, top - c], [-hw, bot + c],
        [-(hw - c), bot], [hw - c, bot], [hw, bot + c], [hw, top - c],
      ].map((p) => [p[0], p[1], s.z_m]);
    };
    const rings = secs.map(ring);
    const pos = [];
    const tri = (a, b, c) => pos.push(...a, ...b, ...c);
    for (let i = 0; i + 1 < rings.length; i++) {
      const r0 = rings[i], r1 = rings[i + 1];
      for (let k = 0; k < 8; k++) {
        const k2 = (k + 1) % 8;
        tri(r0[k], r0[k2], r1[k2]);
        tri(r0[k], r1[k2], r1[k]);
      }
    }
    // Caps: stern (first ring) and bow (last ring), fanned from the centre.
    const cap = (r, flip) => {
      const c = r.reduce((a, p) => [a[0] + p[0] / 8, a[1] + p[1] / 8, a[2] + p[2] / 8], [0, 0, 0]);
      for (let k = 0; k < 8; k++) {
        const k2 = (k + 1) % 8;
        if (flip) tri(c, r[k2], r[k]); else tri(c, r[k], r[k2]);
      }
    };
    cap(rings[0], true);
    cap(rings[rings.length - 1], false);
    const g = new THREE.BufferGeometry();
    g.setAttribute("position", new THREE.Float32BufferAttribute(pos, 3));
    g.computeVertexNormals();
    // Every face winds outward: the stern cap (first ring) is flipped, the bow cap is not
    // (checked by the exterior mockup, which counts any triangle facing the loft axis).
    return g;
  }

  // ----------------------------------------------------------------- labels

  /** A camera-facing text sprite. size_m is the label's height in metres. */
  function label(THREE, text, opts) {
    opts = opts || {};
    const px = 64, pad = 16;
    const cv = document.createElement("canvas");
    const ctx = cv.getContext("2d");
    ctx.font = `600 ${px}px system-ui, sans-serif`;
    const w = Math.ceil(ctx.measureText(text).width) + pad * 2;
    cv.width = w; cv.height = px + pad * 2;
    ctx.font = `600 ${px}px system-ui, sans-serif`;
    ctx.fillStyle = opts.bg || "rgba(8,12,18,0.78)";
    ctx.fillRect(0, 0, cv.width, cv.height);
    if (opts.accent !== undefined) {
      ctx.fillStyle = "#" + new THREE.Color(opts.accent).getHexString();
      ctx.fillRect(0, 0, 10, cv.height);
    }
    ctx.fillStyle = opts.color || "#e8eef5";
    ctx.textBaseline = "middle";
    ctx.fillText(text, pad + (opts.accent !== undefined ? 6 : 0), cv.height / 2);
    const tex = new THREE.CanvasTexture(cv);
    tex.colorSpace = THREE.SRGBColorSpace;
    const mat = new THREE.SpriteMaterial({ map: tex, depthTest: opts.depthTest !== false, transparent: true });
    const s = new THREE.Sprite(mat);
    const h = opts.size_m || 0.5;
    s.scale.set((h * cv.width) / cv.height, h, 1);
    s.renderOrder = 10;
    return s;
  }

  // ------------------------------------------------------------ budget HUD

  /**
   * The Pi 5 cost meter every mockup shows (CLAUDE.md section 11). Call
   * hud.beginFrame(renderer) before the frame's first render and
   * hud.endFrame(renderer) after its last; it sums every pass (a viewscreen's
   * render-to-texture included).
   */
  function budgetHud(opts) {
    opts = opts || {};
    const el = document.createElement("div");
    el.className = "pi-budget";
    el.style.cssText = [
      "position:fixed", "right:12px", "bottom:12px", "z-index:20", "font:12px/1.35 ui-monospace,Menlo,Consolas,monospace",
      "color:#d8e2ec", "background:rgba(6,9,14,0.82)", "border:1px solid #2b3540", "border-radius:6px", "padding:8px 10px",
      "min-width:210px", "max-width:260px", "pointer-events:none",
    ].join(";");
    document.body.appendChild(el);
    let tris = 0, calls = 0, frames = 0, acc = { t: 0, c: 0 }, last = performance.now(), shown = { t: 0, c: 0, fps: 0 };
    const bar = (v, max) => {
      const f = Math.min(1, v / max);
      const color = v > max ? "#ff5252" : v > 0.8 * max ? "#ffb300" : "#66bb6a";
      return `<div style="height:5px;background:#1c242d;border-radius:3px;margin:2px 0 5px"><div style="height:5px;width:${(f * 100).toFixed(1)}%;background:${color};border-radius:3px"></div></div>`;
    };
    function render() {
      const b = PI_BUDGET;
      el.innerHTML =
        `<div style="font-weight:700;margin-bottom:4px">${PI_BUDGET.board} budget (provisional)</div>` +
        `triangles ${shown.t.toLocaleString()} / ${b.triangles.toLocaleString()}${bar(shown.t, b.triangles)}` +
        `draw calls ${shown.c} / ${b.drawCalls}${bar(shown.c, b.drawCalls)}` +
        `<div style="color:#8a98a8">desktop ${shown.fps.toFixed(0)} fps: not a Pi measurement</div>` +
        (opts.note ? `<div style="color:#8a98a8">${opts.note}</div>` : "");
    }
    return {
      el,
      beginFrame(renderer) { renderer.info.autoReset = false; renderer.info.reset(); },
      endFrame(renderer) {
        tris = renderer.info.render.triangles; calls = renderer.info.render.calls;
        acc.t += tris; acc.c += calls; frames++;
        const now = performance.now();
        if (now - last > 500) {
          shown = { t: Math.round(acc.t / frames), c: Math.round(acc.c / frames), fps: (frames * 1000) / (now - last) };
          acc = { t: 0, c: 0 }; frames = 0; last = now;
          render();
        }
      },
      /** The latest averaged numbers, for screenshots and tests. */
      read() { return { triangles: shown.t, drawCalls: shown.c }; },
    };
  }

  // -------------------------------------------------------- mockup plumbing

  /**
   * Standard page chrome: title block naming the change the mockup presents,
   * and the "mockup, not a Pi measurement" disclaimer.
   */
  function titleBlock(opts) {
    const el = document.createElement("div");
    el.className = "mockup-title";
    el.style.cssText = "position:fixed;left:12px;top:12px;z-index:20;max-width:min(520px,calc(100vw - 24px));font:13px/1.4 system-ui,sans-serif;color:#e8eef5;background:rgba(6,9,14,0.82);border:1px solid #2b3540;border-radius:6px;padding:10px 12px";
    el.innerHTML =
      `<div style="font-weight:700;font-size:15px">${opts.title}</div>` +
      `<div style="color:#9fb0c0">${opts.subtitle || ""}</div>` +
      `<div style="color:#7f8fa0;margin-top:4px">Presents <code>${opts.change}</code>. A three.js mockup on a desktop GPU: it shows layout and look, not Pi 5 performance.</div>`;
    document.body.appendChild(el);
    return el;
  }

  /**
   * Screenshot hooks for tools/mockups/shoot.mjs. A mockup registers named
   * shots; the tool calls each one's setup, waits for frames, and captures.
   * window.MOCKUP_READY is set once the scene has rendered its first frame.
   */
  function registerShots(shots) {
    window.MOCKUP_SHOTS = shots;
  }
  function markReady() {
    window.MOCKUP_READY = true;
  }

  window.ShipKit = {
    version: 1,
    layout, PI_BUDGET, PALETTE, LIGHTING,
    /** Deprecated alias from before the floor moved to the Pi 5; removed once no page uses it. */
    PI3_BUDGET: PI_BUDGET,
    byId, compartment, portalsOf, stationsIn, systemsIn, deckById, measure, bounds, center, portalHalf,
    roomShell, hullGeometry, label, budgetHud, titleBlock, registerShots, markReady,
    rectMinusHoles,
  };
})();
