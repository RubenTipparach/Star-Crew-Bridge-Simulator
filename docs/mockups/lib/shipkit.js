/*
 * shipkit.js: the one place a mockup turns a ship layout into three.js objects.
 *
 * Every Star Crew mockup reads the same layout (data/ships/<id>/layout.json), so
 * the bridge, the deck plan, the systems view and the exterior cannot disagree
 * about where a room or a door is (CLAUDE.md sections 8 and 11). This file is
 * the shared interpretation of that layout: palette, lighting states, the Pi 5
 * budget meter, room shells with door openings cut, the generated detail (frames,
 * coves, trims, door frames, conduits, lamps), the Material Maker texture array,
 * the hull loft and labels.
 *
 * Layout schema starcrew.ship-layout/2: a compartment's air is the union of
 * convex prisms ("brushes"), a footprint polygon in plan and a floor and ceiling
 * height, so rooms follow the hull (openspec/changes/reference-ship-tern). The
 * detail rules and their sizes are data/ships/<id>/detailing.json
 * (openspec/changes/deck-pipeline, section 5a); the materials are
 * data/materials/materials.json (openspec/changes/surface-materials). This kit is
 * the mockups' implementation of those rules; deckc will be the engine's.
 *
 * It is a classic script, not a module, so a mockup still works when opened
 * straight from disk. tools/mockups/inline.py copies it into each page between
 * the "INLINE shipkit" markers; edit it here, never in a page. Functions that
 * build geometry take THREE as their first argument because the page owns the
 * three.js import.
 *
 * Axes (from the layout): +X port, +Y up, +Z bow, metres. Plan points are [x, z].
 */
(function () {
  "use strict";

  /** The layout inlined into the page as <script id="ship-layout" type="application/json">. */
  function layout() {
    const el = document.getElementById("ship-layout");
    if (!el) throw new Error("shipkit: no #ship-layout script in the page (run tools/mockups/inline.py)");
    const L = JSON.parse(el.textContent);
    if (L.schema !== "starcrew.ship-layout/2") throw new Error("shipkit: layout schema " + L.schema + ", expected starcrew.ship-layout/2");
    return L;
  }

  /** A ship data file inlined as <script id="ship-data-<name>">, e.g. "detailing", "power". */
  function shipData(name) {
    const el = document.getElementById("ship-data-" + name);
    if (!el) throw new Error(`shipkit: no #ship-data-${name} script in the page (add <!-- INLINE data:<ship>/${name} --> markers and run tools/mockups/inline.py)`);
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
    statusOk: 0x5ee08a,
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
   * colours and intensities. status is the door status strips' colour.
   */
  const LIGHTING = {
    normal: { label: "Normal", ambient: 0x2a3340, ambientI: 0.55, lamp: 0xffe9c8, lampI: 1.0, strip: 0x9fd8ff, stripI: 0.6, fog: 0x0b0e14, status: 0x5ee08a },
    red_alert: { label: "Red alert", ambient: 0x2a0d10, ambientI: 0.45, lamp: 0xff3030, lampI: 0.75, strip: 0xff2020, stripI: 1.0, fog: 0x120406, status: 0xff2a2a },
    emergency: { label: "Emergency power", ambient: 0x120c08, ambientI: 0.3, lamp: 0xff8a1c, lampI: 0.35, strip: 0xff8a1c, stripI: 0.8, fog: 0x060403, status: 0xff8a1c },
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

  // ------------------------------------------------------- plan geometry [x, z]

  const EPS = 1e-4;

  /** Signed area in square metres; the layout winds every brush positive. */
  function signedArea(poly) {
    let s = 0;
    for (let i = 0; i < poly.length; i++) {
      const a = poly[i], b = poly[(i + 1) % poly.length];
      s += a[0] * b[1] - b[0] * a[1];
    }
    return s / 2;
  }
  /** Unit outward normal [x, z] of edge a->b of a positively wound polygon. */
  function outwardNormal(a, b) {
    const dx = b[0] - a[0], dz = b[1] - a[1], l = Math.hypot(dx, dz);
    return [dz / l, -dx / l];
  }
  /** Point inside a convex, positively wound polygon (the boundary counts). */
  function insidePoly(poly, x, z, tol) {
    tol = tol === undefined ? 1e-3 : tol;
    for (let i = 0; i < poly.length; i++) {
      const a = poly[i], n = outwardNormal(a, poly[(i + 1) % poly.length]);
      if ((x - a[0]) * n[0] + (z - a[1]) * n[1] > tol) return false;
    }
    return true;
  }
  /** Where the line x = v (axis 0) or z = v (axis 1) crosses a convex polygon: [lo, hi] of the other coordinate, or null. */
  function chord(poly, axis, v) {
    const o = 1 - axis, hits = [];
    for (let i = 0; i < poly.length; i++) {
      const a = poly[i], b = poly[(i + 1) % poly.length];
      const da = a[axis] - v, db = b[axis] - v;
      if (Math.abs(da) < EPS) hits.push(a[o]);
      if ((da < -EPS && db > EPS) || (da > EPS && db < -EPS)) hits.push(a[o] + (b[o] - a[o]) * (da / (da - db)));
    }
    if (hits.length < 2) return null;
    return [Math.min(...hits), Math.max(...hits)];
  }
  /** A convex polygon with edge i moved inward by d[i] metres (d may be one number for every edge). */
  function insetPoly(poly, d) {
    const n = poly.length, lines = [];
    for (let i = 0; i < n; i++) {
      const a = poly[i], b = poly[(i + 1) % n], on = outwardNormal(a, b), di = Array.isArray(d) ? d[i] : d;
      lines.push({ p: [a[0] - on[0] * di, a[1] - on[1] * di], t: [b[0] - a[0], b[1] - a[1]] });
    }
    const out = [];
    for (let i = 0; i < n; i++) {
      const l1 = lines[(i + n - 1) % n], l2 = lines[i];
      const den = l1.t[0] * l2.t[1] - l1.t[1] * l2.t[0];
      if (Math.abs(den) < 1e-9) { out.push(l2.p.slice()); continue; }
      const s = ((l2.p[0] - l1.p[0]) * l2.t[1] - (l2.p[1] - l1.p[1]) * l2.t[0]) / den;
      out.push([l1.p[0] + l1.t[0] * s, l1.p[1] + l1.t[1] * s]);
    }
    return out;
  }
  /** Area-weighted centroid of a polygon. */
  function polyCentroid(poly) {
    let cx = 0, cz = 0, a = 0;
    for (let i = 0; i < poly.length; i++) {
      const p = poly[i], q = poly[(i + 1) % poly.length], k = p[0] * q[1] - q[0] * p[1];
      a += k; cx += (p[0] + q[0]) * k; cz += (p[1] + q[1]) * k;
    }
    return [cx / (3 * a), cz / (3 * a)];
  }

  // ---------------------------------------------------- compartment geometry

  /** Volume in cubic metres and floor area in square metres of a compartment (brush air; detail is not air). */
  function measure(comp) {
    let v = 0, a = 0;
    for (const b of comp.brushes) {
      const ar = signedArea(b.poly);
      v += ar * (b.y[1] - b.y[0]); a += ar;
    }
    return { volume_m3: v, floor_m2: a };
  }
  /** Union bounds of a compartment's brushes: { x: [lo, hi], y: [lo, hi], z: [lo, hi] }. */
  function bounds(comp) {
    const r = { x: [Infinity, -Infinity], y: [Infinity, -Infinity], z: [Infinity, -Infinity] };
    for (const b of comp.brushes) {
      for (const p of b.poly) {
        r.x[0] = Math.min(r.x[0], p[0]); r.x[1] = Math.max(r.x[1], p[0]);
        r.z[0] = Math.min(r.z[0], p[1]); r.z[1] = Math.max(r.z[1], p[1]);
      }
      r.y[0] = Math.min(r.y[0], b.y[0]); r.y[1] = Math.max(r.y[1], b.y[1]);
    }
    return r;
  }
  /** A point well inside the compartment: the centroid of its largest brush, at mid height. */
  function center(comp) {
    let best = null;
    for (const b of comp.brushes) { const a = signedArea(b.poly); if (!best || a > best.a) best = { a, b }; }
    const c = polyCentroid(best.b.poly);
    return [c[0], (best.b.y[0] + best.b.y[1]) / 2, c[1]];
  }
  /** The brush of comp containing (x, y, z), with yTol of slack below its floor, or null. */
  function brushAt(comp, x, y, z, yTol) {
    yTol = yTol || 0;
    for (const b of comp.brushes) if (insidePoly(b.poly, x, z) && y >= b.y[0] - yTol - EPS && y <= b.y[1] + EPS) return b;
    return null;
  }
  /** The floor height under (x, z) in comp nearest to y (a gallery or a ladder well picks the right level). */
  function floorAt(comp, x, z, y) {
    let best = null;
    for (const b of comp.brushes) {
      if (!insidePoly(b.poly, x, z, 0.01)) continue;
      const d = Math.min(Math.abs(b.y[0] - y), Math.abs(b.y[1] - y));
      if (!best || d < best.d) best = { d, y0: b.y[0] };
    }
    return best ? best.y0 : y;
  }

  // ------------------------------------------------------------------ portals

  /**
   * A portal's opening in a form that is easy to draw. A wall portal: { floor: false,
   * c, n, t (unit along the wall), w, h }; a floor portal: { floor: true, c, n, sx, sz }.
   * n points from between[0] into between[1].
   */
  function portalFrame(p) {
    const n = p.normal, c = p.center_m;
    if (Math.abs(n[1]) > 0.5) return { floor: true, c, n, sx: p.size_m[0], sz: p.size_m[1] };
    return { floor: false, c, n, t: [-n[2], 0, n[0]], w: p.size_m[0], h: p.size_m[1] };
  }
  /** The portal's normal pointing out of compartment cid. */
  function portalOut(p, cid) { return p.between[0] === cid ? p.normal : p.normal.map((v) => -v); }
  /** The point a walker stands on at a portal, on the side of compartment cid (a floor portal: that side's floor). */
  function portalPoint(L, p, cid) {
    const f = portalFrame(p), x = f.c[0], y = f.c[1], z = f.c[2];
    if (!f.floor) return [x, y - f.h / 2, z];
    const comp = compartment(L, cid);
    return comp ? [x, floorAt(comp, x, z, y), z] : [x, y, z];
  }

  /**
   * Every wall of a compartment, with what is cut out of it. One entry per brush edge:
   * { brush, bi, i, a, b (plan ends), t, n (outward, plan), len, y0, y1, holes } where a hole
   * is { u0, u1, v0, v1, portal?, sibling?, siblingFloor? } in metres along the wall from a (u) and
   * height (v); siblingFloor is the floor height of the brush a sibling opening leads into.
   * opts.skipKinds: portal kinds not to cut (for example ["window"]).
   */
  function wallsOf(L, comp, opts) {
    opts = opts || {};
    const skip = opts.skipKinds || [];
    const ports = portalsOf(L, comp.id).filter((p) => Math.abs(p.normal[1]) < 0.5 && skip.indexOf(p.kind) < 0);
    const out = [];
    comp.brushes.forEach((br, bi) => {
      const poly = br.poly;
      for (let i = 0; i < poly.length; i++) {
        const a = poly[i], b = poly[(i + 1) % poly.length];
        const len = Math.hypot(b[0] - a[0], b[1] - a[1]);
        const t = [(b[0] - a[0]) / len, (b[1] - a[1]) / len], n = outwardNormal(a, b);
        const w = { brush: br, bi, i, a, b, t, n, len, y0: br.y[0], y1: br.y[1], holes: [] };
        for (const p of ports) {
          const o = portalOut(p, comp.id), c = p.center_m;
          if (o[0] * n[0] + o[2] * n[1] < 0.999) continue;
          if (Math.abs((c[0] - a[0]) * n[0] + (c[2] - a[1]) * n[1]) > 0.01) continue;
          const u = (c[0] - a[0]) * t[0] + (c[2] - a[1]) * t[1];
          if (u < -0.01 || u > len + 0.01) continue;
          const v0 = c[1] - p.size_m[1] / 2, v1 = c[1] + p.size_m[1] / 2;
          if (v1 < br.y[0] + 0.01 || v0 > br.y[1] - 0.01) continue;
          w.holes.push({ u0: u - p.size_m[0] / 2, u1: u + p.size_m[0] / 2, v0, v1, portal: p });
        }
        comp.brushes.forEach((ob, oi) => {
          if (oi === bi) return;
          for (let j = 0; j < ob.poly.length; j++) {
            const oa = ob.poly[j], oe = ob.poly[(j + 1) % ob.poly.length], on = outwardNormal(oa, oe);
            if (on[0] * n[0] + on[1] * n[1] > -0.999) continue;
            if (Math.abs((oa[0] - a[0]) * n[0] + (oa[1] - a[1]) * n[1]) > 0.01) continue;
            const ua = (oa[0] - a[0]) * t[0] + (oa[1] - a[1]) * t[1], ue = (oe[0] - a[0]) * t[0] + (oe[1] - a[1]) * t[1];
            const u0 = Math.max(0, Math.min(ua, ue)), u1 = Math.min(len, Math.max(ua, ue));
            const v0 = Math.max(br.y[0], ob.y[0]), v1 = Math.min(br.y[1], ob.y[1]);
            if (u1 - u0 > EPS && v1 - v0 > EPS) w.holes.push({ u0, u1, v0, v1, sibling: true, siblingFloor: ob.y[0] });
          }
        });
        out.push(w);
      }
    });
    return out;
  }

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
    cells.sort((a, b) => a.v0 - b.v0 || a.u0 - b.u0);
    const merged = [];
    for (const c of cells) {
      const last = merged[merged.length - 1];
      if (last && Math.abs(last.v0 - c.v0) < EPS && Math.abs(last.v1 - c.v1) < EPS && Math.abs(last.u1 - c.u0) < EPS) last.u1 = c.u1;
      else merged.push(Object.assign({}, c));
    }
    return merged;
  }
  /** An interval [lo, hi] minus intervals, as the pieces left. */
  function intervalMinus(lo, hi, cuts) {
    let pieces = [[lo, hi]];
    for (const c of cuts) {
      const next = [];
      for (const [a, b] of pieces) {
        if (c[1] <= a || c[0] >= b) { next.push([a, b]); continue; }
        if (c[0] > a) next.push([a, c[0]]);
        if (c[1] < b) next.push([c[1], b]);
      }
      pieces = next;
    }
    return pieces;
  }

  // -------------------------------------------------------- surface builder

  /** World-anchored planar UV in metres: x, z on floors and ceilings; along the face's horizontal tangent and y otherwise. */
  function worldUv(p, n) {
    if (Math.abs(n[1]) > 0.75) return [p[0], p[2]];
    const l = Math.hypot(n[0], n[2]) || 1, tx = -n[2] / l, tz = n[0] / l;
    return [p[0] * tx + p[2] * tz, p[1]];
  }

  /**
   * Accumulates triangles by role. Every vertex gets a world-anchored planar UV in metres
   * (deck-pipeline section 5a), divided later by its material's span.
   */
  function Builder() {
    this.parts = {};
  }
  Builder.prototype.part = function (role) {
    return this.parts[role] || (this.parts[role] = { position: [], normal: [], uvm: [] });
  };
  Builder.prototype.tri = function (role, a, b, c, n) {
    const P = this.part(role);
    // Wind so the face looks along n.
    const ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]], ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    const cr = [ab[1] * ac[2] - ab[2] * ac[1], ab[2] * ac[0] - ab[0] * ac[2], ab[0] * ac[1] - ab[1] * ac[0]];
    const pts = cr[0] * n[0] + cr[1] * n[1] + cr[2] * n[2] < 0 ? [a, c, b] : [a, b, c];
    for (const p of pts) {
      P.position.push(p[0], p[1], p[2]);
      P.normal.push(n[0], n[1], n[2]);
      const uv = worldUv(p, n);
      P.uvm.push(uv[0], uv[1]);
    }
  };
  Builder.prototype.quad = function (role, a, b, c, d, n) { this.tri(role, a, b, c, n); this.tri(role, a, c, d, n); };
  /**
   * A triangle with its own texture coordinates (in layer units, not metres) and a layer name per
   * vertex (vlayer), for surfaces whose UVs are not world-projected: wall panels (wall-panels design
   * section 5). Wound to face n like tri.
   */
  Builder.prototype.triUv = function (role, a, b, c, ua, ub, uc, n, layer) {
    const P = this.part(role);
    if (!P.vlayer) P.vlayer = [];
    const ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]], ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    const cr = [ab[1] * ac[2] - ab[2] * ac[1], ab[2] * ac[0] - ab[0] * ac[2], ab[0] * ac[1] - ab[1] * ac[0]];
    const pts = cr[0] * n[0] + cr[1] * n[1] + cr[2] * n[2] < 0 ? [[a, ua], [c, uc], [b, ub]] : [[a, ua], [b, ub], [c, uc]];
    for (const [p, uv] of pts) {
      P.position.push(p[0], p[1], p[2]);
      P.normal.push(n[0], n[1], n[2]);
      P.uvm.push(uv[0], uv[1]);
      P.vlayer.push(layer);
    }
  };
  /** A polygon with holes on the horizontal plane y, facing up (dir 1) or down (-1). */
  Builder.prototype.flat = function (THREE, role, contour, holes, y, dir) {
    const v2 = (p) => new THREE.Vector2(p[0], p[1]);
    const faces = THREE.ShapeUtils.triangulateShape(contour.map(v2), holes.map((h) => h.map(v2)));
    const all = contour.concat(...holes);
    for (const f of faces) {
      const p = f.map((k) => [all[k][0], y, all[k][1]]);
      this.tri(role, p[0], p[1], p[2], [0, dir, 0]);
    }
  };
  /**
   * An oriented box: centre c, unit axes u, v, w with half sizes hu, hv, hw. faces lists the
   * faces to draw ("+u", "-u", "+v", "-v", "+w", "-w"); faces pressed against a wall, floor
   * or ceiling are left out (CLAUDE.md section 8: no coplanar faces facing the same way).
   * roles maps a face to a role other than the box's own (a lamp's lens is its "-v" face).
   */
  Builder.prototype.box = function (role, c, u, v, w, hu, hv, hw, faces, roles) {
    const P = (su, sv, sw) => [0, 1, 2].map((k) => c[k] + u[k] * su * hu + v[k] * sv * hv + w[k] * sw * hw);
    const neg = (x) => x.map((y) => -y);
    const F = {
      "+u": [[1, -1, -1], [1, 1, -1], [1, 1, 1], [1, -1, 1], u],
      "-u": [[-1, -1, -1], [-1, -1, 1], [-1, 1, 1], [-1, 1, -1], neg(u)],
      "+v": [[-1, 1, -1], [-1, 1, 1], [1, 1, 1], [1, 1, -1], v],
      "-v": [[-1, -1, -1], [1, -1, -1], [1, -1, 1], [-1, -1, 1], neg(v)],
      "+w": [[-1, -1, 1], [1, -1, 1], [1, 1, 1], [-1, 1, 1], w],
      "-w": [[-1, -1, -1], [-1, 1, -1], [1, 1, -1], [1, -1, -1], neg(w)],
    };
    for (const f of faces) {
      const q = F[f], r = (roles && roles[f]) || role;
      this.quad(r, P(...q[0]), P(...q[1]), P(...q[2]), P(...q[3]), q[4]);
    }
  };
  /** An n-sided prism (a pipe) from a to b with radius r; no end caps. */
  Builder.prototype.pipe = function (role, a, b, r, sides) {
    const d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]], l = Math.hypot(d[0], d[1], d[2]), ax = d.map((x) => x / l);
    const ref = Math.abs(ax[1]) < 0.9 ? [0, 1, 0] : [1, 0, 0];
    let e1 = [ax[1] * ref[2] - ax[2] * ref[1], ax[2] * ref[0] - ax[0] * ref[2], ax[0] * ref[1] - ax[1] * ref[0]];
    const l1 = Math.hypot(e1[0], e1[1], e1[2]); e1 = e1.map((x) => x / l1);
    const e2 = [ax[1] * e1[2] - ax[2] * e1[1], ax[2] * e1[0] - ax[0] * e1[2], ax[0] * e1[1] - ax[1] * e1[0]];
    const o = (t) => [0, 1, 2].map((k) => (e1[k] * Math.cos(t) + e2[k] * Math.sin(t)) * r);
    for (let k = 0; k < sides; k++) {
      const t0 = (2 * Math.PI * k) / sides, t1 = (2 * Math.PI * (k + 1)) / sides;
      const o0 = o(t0), o1 = o(t1), n = o((t0 + t1) / 2).map((x) => x / r);
      const add = (p, q) => [p[0] + q[0], p[1] + q[1], p[2] + q[2]];
      this.quad(role, add(a, o0), add(b, o0), add(b, o1), add(a, o1), n);
    }
  };

  // ------------------------------------------------------------ the detail rules

  /** Frame stations (z, metres) between z0 and z1, at least clear from either end. */
  function frameStations(D, z0, z1, clear) {
    const s = D.frames.spacing_m, o = D.frames.origin_z_m, out = [];
    for (let k = Math.ceil((z0 + clear - o) / s - EPS); o + k * s <= z1 - clear + EPS; k++) out.push(o + k * s);
    return out;
  }

  /** Floor portals (hatch, ladder, hoist, bay door) in this compartment's ceilings, as frames. */
  function ceilingPortals(L, comp) {
    const out = [];
    for (const p of portalsOf(L, comp.id)) {
      const f = portalFrame(p);
      if (f.floor && portalOut(p, comp.id)[1] > 0) out.push(f);
    }
    return out;
  }

  /**
   * Lamps by the kit rule (detailing.json lamps; deck-pipeline section 5): in the bays
   * between frames, round(bay area / area_per_lamp_m2) across the bay's width (a
   * corridor bay at least one, a pod one), every emergency_every-th lamp on the emergency
   * bus. Returns [{ p: [x, y, z] (the lens), R, I, emergency, size: [x, z], tall }].
   */
  function lampsFor(L, comp, D) {
    D = D || shipData("detailing");
    const R = D.lamps, out = [];
    const holes = ceilingPortals(L, comp);
    const keep = systemsIn(L, comp.id).filter((s) => s.radius_m).map((s) => ({ x: s.center_m[0], z: s.center_m[2], r: s.radius_m + R.keep_out_m }));
    for (const br of comp.brushes) {
      const h = br.y[1] - br.y[0], tall = h > R.tall_room_m + EPS, scale = tall ? h / 3.0 : 1.0;
      const size = (comp.kind === "corridor" ? R.corridor_panel_m : R.panel_m).map((v) => v * (tall ? Math.min(scale, 1.6) : 1));
      const y = br.y[1] - (tall ? R.hang_m : 0) - R.housing_m;
      const pts = [];
      if (comp.kind === "pod") {
        // One lamp, at the centre or, when a hatch is there, beside it.
        const c = polyCentroid(br.poly);
        const clash = (x, z) => holes.some((f) => Math.abs(x - f.c[0]) < f.sx / 2 + size[0] / 2 && Math.abs(z - f.c[2]) < f.sz / 2 + size[1] / 2);
        const tries = [[0, 0]].concat(holes.map((f) => [0, f.sz / 2 + size[1] / 2 + 0.05]), holes.map((f) => [0, -(f.sz / 2 + size[1] / 2 + 0.05)]));
        const ok = tries.map(([dx, dz]) => [c[0] + dx, c[1] + dz]).find(([x, z]) => !clash(x, z) && insidePoly(br.poly, x - size[0] / 2, z - size[1] / 2) && insidePoly(br.poly, x + size[0] / 2, z + size[1] / 2));
        if (ok) pts.push(ok);
      } else {
        const zs = br.poly.map((p) => p[1]), z0 = Math.min(...zs), z1 = Math.max(...zs);
        const cuts = [z0].concat(frameStations(D, z0, z1, 0.01)).concat([z1]);
        for (let i = 0; i + 1 < cuts.length; i++) {
          const za = cuts[i], zb = cuts[i + 1];
          if (zb - za < R.min_bay_m) continue;
          const zm = (za + zb) / 2, ch = chord(br.poly, 1, zm);
          if (!ch) continue;
          const x0 = ch[0] + R.wall_clear_m, x1 = ch[1] - R.wall_clear_m;
          if (x1 <= x0) continue;
          let n = Math.round(((ch[1] - ch[0]) * (zb - za)) / R.area_per_lamp_m2);
          if (comp.kind === "corridor") n = Math.max(1, n);
          for (let k = 0; k < n; k++) pts.push([x0 + ((k + 0.5) * (x1 - x0)) / n, zm]);
        }
      }
      for (const [x, z] of pts) {
        if (keep.some((k) => Math.hypot(x - k.x, z - k.z) < k.r)) continue;
        if (holes.some((f) => Math.abs(x - f.c[0]) < f.sx / 2 + size[0] / 2 && Math.abs(z - f.c[2]) < f.sz / 2 + size[1] / 2)) continue;
        out.push({ p: [x, y, z], R: R.radius_m * scale, I: 1 / (scale * scale), emergency: false, size, tall });
      }
    }
    out.forEach((l, i) => { l.emergency = i % R.emergency_every === 0; });
    return out;
  }

  // ------------------------------------------------------------ wall panels

  /** data/materials/panels.json inlined as <script id="ship-panels">: { manifest, layers, ui }. */
  function panelsData() {
    const el = document.getElementById("ship-panels");
    if (!el) throw new Error("shipkit: no #ship-panels script in the page (add <!-- INLINE panels --> markers and run tools/mockups/inline.py)");
    return JSON.parse(el.textContent);
  }

  /**
   * The layer name a panel module or a finish's strips take in the texture array (loadPanels). A
   * ceiling or floor module (kind "ceiling", "floor") is panel:<finish>:<kind>:<module>; a finish's
   * trim layer is panel:<finish>:trims.
   */
  function panelLayerName(finish, module, kind) { return kind ? `panel:${finish}:${kind}:${module}` : `panel:${finish}:${module}`; }

  /**
   * Every panel layer panels.json numbers, as [{ name (panelLayerName), stem (the file's name in
   * assets/textures/panels/<px>/, without .png), layer }]: wall modules and strips (wall-panels),
   * ceiling and floor modules and the trim layer (ceilings-and-trims, floor-panels).
   */
  function panelLayerList(S) {
    const out = [];
    for (const fn of Object.keys(S.finishes)) {
      const F = S.finishes[fn];
      for (const m of Object.keys(F.modules)) if (m[0] !== "_") out.push({ name: panelLayerName(fn, m), stem: `${fn}_${m}`, layer: F.modules[m].layer });
      out.push({ name: panelLayerName(fn, "strips"), stem: `${fn}_strips`, layer: F.strips.layer });
      for (const kind of ["ceiling", "floor"]) {
        if (!F[kind]) continue;
        for (const m of Object.keys(F[kind].modules)) if (m[0] !== "_") out.push({ name: panelLayerName(fn, m, kind), stem: `${fn}_${kind}_${m}`, layer: F[kind].modules[m].layer });
      }
      if (F.trims) out.push({ name: panelLayerName(fn, "trims"), stem: `${fn}_trims`, layer: F.trims.layer });
    }
    return out;
  }

  /** FNV-1a, 32 bits, over a string's UTF-16 code units: the wall rule's draw (wall-panels design section 4). */
  function fnv1a(str) {
    let h = 0x811c9dc5;
    for (let i = 0; i < str.length; i++) { h ^= str.charCodeAt(i); h = Math.imul(h, 0x01000193) >>> 0; }
    return h >>> 0;
  }

  /** The panels' settings for one build: opts.panels true reads the page's #ship-panels; an object is panels.json itself. */
  function panelSetup(src) {
    const S = src === true ? panelsData().manifest : src.schema ? src : src.manifest;
    if (S.schema !== "starcrew.panels/1") throw new Error("shipkit: panels schema " + S.schema + ", expected starcrew.panels/1");
    return {
      S,
      stats: { bays: 0, cells: 0, modules: {}, ceiling: { cells: 0, modules: {}, rules: {} }, floor: { cells: 0, modules: {}, walkway: 0, rules: {} }, trims: { faces: 0, pillars: 0 } },
    };
  }

  // ------------------------------------------------- trims (ceilings-and-trims)

  /**
   * One trim face as quads on a row of its finish's trim layer (ceilings-and-trims design sections
   * 3-4). The face is o + A * a + C * c for a in [a0, a1] (along the member, metres) and c in
   * [-hc, hc] (across it); n is its normal. u is world-projected along A (dot(p, A) / span, so a
   * long member tiles with the layer's 2 m period without a seam); v runs across the face within
   * the row, which is repeated whole when the face is deeper than stretch_max rows. segs splits
   * the length: [{ a0, a1, piece? }], a piece being a non-tiling part of a row (a pillar's base or
   * capital) mapped onto [a0, a1].
   */
  function stripFace(B, role, T, o, A, C, hc, n, rowName, segs) {
    const row = T.rows[rowName], span = T.span;
    if (!row) throw new Error(`shipkit: no trim row ${rowName} (panels.json trims.rows)`);
    const reps = Math.max(1, Math.ceil((2 * hc) / (T.stretch * row.h_m) - 1e-6));
    const oA = o[0] * A[0] + o[1] * A[1] + o[2] * A[2];
    const P = (a, c) => [o[0] + A[0] * a + C[0] * c, o[1] + A[1] * a + C[1] * c, o[2] + A[2] * a + C[2] * c];
    for (const s of segs) {
      const pc = s.piece;
      const r = pc ? T.rows[pc.row] : row;
      const uOf = pc
        ? (a) => (span / 2 + pc.centre_m + ((a - s.a0) / (s.a1 - s.a0) - 0.5) * pc.length_m) / span
        : (a) => (oA + a) / span;
      const nr = pc ? Math.max(1, Math.ceil((2 * hc) / (T.stretch * r.h_m) - 1e-6)) : reps;
      for (let k = 0; k < nr; k++) {
        const c0 = -hc + (2 * hc * k) / nr, c1 = -hc + (2 * hc * (k + 1)) / nr;
        const v0 = r.v0_m / span, v1 = (r.v0_m + r.h_m) / span;
        const q = [[s.a0, c0, v0], [s.a1, c0, v0], [s.a1, c1, v1], [s.a0, c1, v1]];
        const pts = q.map(([a, c]) => P(a, c)), uvs = q.map(([a, , v]) => [uOf(a), v]);
        B.triUv(role, pts[0], pts[1], pts[2], uvs[0], uvs[1], uvs[2], n, T.layer);
        B.triUv(role, pts[0], pts[2], pts[3], uvs[0], uvs[2], uvs[3], n, T.layer);
      }
    }
    T.stats.faces++;
  }

  /** The trim mapping of a compartment: its finish's trim layer, the rows, members and pillar pieces. */
  function trimContext(ctx, comp) {
    const S = ctx.S, fin = S.finishes[comp.finish];
    if (!fin || !fin.trims) throw new Error(`shipkit: compartment ${comp.id} has finish ${comp.finish}, with no trims in panels.json`);
    const Tr = S.trims;
    return { S, layer: panelLayerName(comp.finish, "trims"), rows: Tr.rows, members: Tr.members, pieces: Tr.pieces, pillar: Tr.pillar, span: S.layers.span_m, stretch: Tr.stretch_max, stats: ctx.stats.trims };
  }

  /**
   * A box like Builder.box, its faces on trim strips: the front face takes its member's row, the long
   * side faces its "sides" row, the end faces "side". A face whose role is not a trim member (a
   * pressure door's hazard jamb) is drawn as box would. pillar { base_m, capital_m } splits the front
   * face into base, shaft and capital (a rib, ceilings-and-trims design section 3).
   */
  function stripBox(B, T, role, c, u, v, w, hu, hv, hw, faces, roles, front, pillar) {
    const ax = [u, v, w], hs = [hu, hv, hw], names = ["u", "v", "w"];
    const long = hs[0] >= hs[1] && hs[0] >= hs[2] ? 0 : hs[1] >= hs[2] ? 1 : 2;
    for (const f of faces) {
      const r = (roles && roles[f]) || role, M = T.members[r];
      const a = names.indexOf(f[1]), s = f[0] === "+" ? 1 : -1;
      if (!M) {
        const sub = {};
        sub[f] = r;
        B.box(r, c, u, v, w, hu, hv, hw, [f], sub);
        continue;
      }
      const others = [0, 1, 2].filter((k) => k !== a);
      const al = others.indexOf(long) >= 0 ? long : (hs[others[0]] >= hs[others[1]] ? others[0] : others[1]);
      const ac = others.find((k) => k !== al);
      const isEnd = others.indexOf(long) < 0;
      const rowName = isEnd ? "side" : f === front ? M.face : M.sides;
      const o = [0, 1, 2].map((k) => c[k] + ax[a][k] * s * hs[a]);
      const n = ax[a].map((x) => x * s);
      const L = hs[al];
      let segs = [{ a0: -L, a1: L }];
      if (pillar && f === front && !isEnd) {
        const b = pillar.base_m, t = pillar.capital_m;
        if (2 * L - b - t >= T.pillar.min_shaft_m) {
          segs = [{ a0: -L, a1: -L + b, piece: T.pieces.rib_base }, { a0: -L + b, a1: L - t }, { a0: L - t, a1: L, piece: T.pieces.rib_capital }];
          T.stats.pillars++;
        }
      }
      stripFace(B, r, T, o, ax[al], ax[ac], hs[ac], n, rowName, segs);
    }
  }

  /**
   * The bands of a wall from its floor y0 to the top of its flat part (wall-panels design section 2):
   * a base strip, a module band, then for each further whole strip + module of height the "between"
   * strip and another module band (level 1, 2, ...), and the top strip in strip_m quads to the top,
   * the last cropped. A module band cut short by a low wall is marked cropped.
   */
  function panelBands(y0, top, Sb) {
    const out = [];
    let y = y0;
    if (top - y0 <= EPS) return out;
    out.push({ strip: Sb.base, v0: y, v1: Math.min(top, y + Sb.base_m), q0: y });
    y += Sb.base_m;
    if (y >= top - EPS) return out;
    let level = 0;
    out.push({ module: true, level, v0: y, v1: Math.min(top, y + Sb.module_m), cropped: top - y < Sb.module_m - EPS });
    y += Sb.module_m;
    while (top - y >= Sb.strip_m + Sb.module_m - EPS) {
      out.push({ strip: Sb.between, v0: y, v1: y + Sb.strip_m, q0: y });
      y += Sb.strip_m;
      level++;
      out.push({ module: true, level, v0: y, v1: y + Sb.module_m, cropped: false });
      y += Sb.module_m;
    }
    while (y < top - EPS) {
      const v1 = Math.min(top, y + Sb.strip_m);
      out.push({ strip: Sb.top, v0: y, v1, q0: y });
      y = v1;
    }
    return out;
  }

  /**
   * The bays of a wall (wall-panels design section 1): cut at its ends, its ribs and stiffeners, and
   * the edges of every opening and wall fixture; a stretch over full_max_m is split evenly. A bay
   * inside an opening's or a fixture's span along the wall is marked zone; a bay whose end meets a
   * door's frame records the door's side ("left" or "right", as seen facing the wall) and the
   * door's height range.
   */
  function panelBays(len, clear, ribs, S) {
    const zones = clear.map((h) => ({ u0: Math.max(0, h.u0), u1: Math.min(len, h.u1), h })).filter((z) => z.u1 - z.u0 > 1e-3);
    const cuts = [0, len].concat(ribs);
    for (const z of zones) cuts.push(z.u0, z.u1);
    const cs = [...new Set(cuts.map((x) => +x.toFixed(4)))].sort((a, b) => a - b);
    const bays = [];
    for (let i = 0; i + 1 < cs.length; i++) {
      const u0 = cs[i], u1 = cs[i + 1], mid = (u0 + u1) / 2;
      if (u1 - u0 < 1e-3) continue;
      // Split every long stretch, an opening's too: under a gallery's opening no ribs stand, and the
      // wall below it still needs bays.
      const zone = zones.some((z) => mid > z.u0 && mid < z.u1);
      const wd = u1 - u0, n = wd > S.bays.full_max_m + 1e-3 ? Math.ceil(wd / S.bays.full_max_m - 1e-6) : 1;
      for (let k = 0; k < n; k++) bays.push({ u0: u0 + (wd * k) / n, u1: u0 + (wd * (k + 1)) / n, zone });
    }
    const kinds = S.rule.door_kinds;
    for (const b of bays) {
      if (b.zone) continue;
      for (const z of zones) {
        const p = z.h.portal;
        if (!p || kinds.indexOf(p.kind) < 0) continue;
        if (Math.abs(z.u0 - b.u1) < 0.02) b.door = { side: "right", v0: z.h.v0, v1: z.h.v1 };
        else if (Math.abs(z.u1 - b.u0) < 0.02 && !b.door) b.door = { side: "left", v0: z.h.v0, v1: z.h.v1 };
      }
    }
    return bays;
  }

  /** Draw a module by weight with the rule's hash, leaving out the excluded ones (wall-panels design section 4). */
  function drawPanelModule(fin, key, exclude) {
    const ids = Object.keys(fin.modules)
      .filter((m) => fin.modules[m].placed === "draw" && fin.modules[m].weight > 0 && exclude.indexOf(m) < 0)
      .sort((a, b) => fin.modules[a].layer - fin.modules[b].layer);
    const total = ids.reduce((a, m) => a + fin.modules[m].weight, 0);
    let t = (fnv1a(key) / 4294967296) * total;
    for (const m of ids) { t -= fin.modules[m].weight; if (t < 0) return m; }
    return ids[ids.length - 1];
  }

  /**
   * Dress one wall with panels: bays by bands, each cell a quad (minus the openings) on its module's
   * layer with bay-local texture coordinates, or a strip's quarter of the strip layer with
   * world-anchored u along the wall (wall-panels design sections 1-5). Adds role "panel" to the
   * builder, with a layer name per vertex (vlayer) that geometryOf resolves.
   */
  function dressWall(B, comp, w, top, holes, clear, ribs, ctx, P3, nIn) {
    const S = ctx.S, fin = S.finishes[comp.finish];
    if (!fin) throw new Error(`shipkit: compartment ${comp.id} has finish ${comp.finish}, not in panels.json finishes`);
    const span = S.layers.span_m, half = S.layers.margin_m / 2, Sb = S.bands;
    const bays = panelBays(w.len, clear, ribs, S), bands = panelBands(w.y0, top, Sb);
    const nRows = Object.keys(fin.strips.rows).length;
    // World-anchored strip u: along the wall to the viewer's right (w.t), in metres, over the 2 m period.
    const along = (u) => (w.a[0] + w.t[0] * u) * w.t[0] + (w.a[1] + w.t[1] * u) * w.t[1];
    // Rule 1, cell by cell: a bay's cell that an opening or a wall fixture reaches into is plain
    // plate (a door's bay above the door in a tall wall, or a wall under a gallery's opening, is not).
    const hits = (c) => clear.some((h) => h.u0 < c.u1 - EPS && h.u1 > c.u0 + EPS && h.v0 < c.v1 - EPS && h.v1 > c.v0 + EPS);
    const quad = (c, uvOf, layer) => {
      const p = [[c.u0, c.v0], [c.u1, c.v0], [c.u1, c.v1], [c.u0, c.v1]];
      const pts = p.map(([u, v]) => P3(u, v)), uvs = p.map(([u, v]) => uvOf(u, v));
      B.triUv("panel", pts[0], pts[1], pts[2], uvs[0], uvs[1], uvs[2], nIn, layer);
      B.triUv("panel", pts[0], pts[2], pts[3], uvs[0], uvs[2], uvs[3], nIn, layer);
    };
    const below = new Map();
    for (const band of bands) {
      if (band.strip) {
        const row = fin.strips.rows[band.strip], layer = panelLayerName(comp.finish, "strips");
        for (const c of rectMinusHoles({ u0: 0, u1: w.len, v0: band.v0, v1: band.v1 }, holes)) {
          quad(c, (u, v) => [along(u) / span, (row + (v - band.q0) / Sb.strip_m) / nRows], layer);
        }
        continue;
      }
      let prev = null;
      bays.forEach((bay, bi) => {
        const wd = bay.u1 - bay.u0, c = (bay.u0 + bay.u1) / 2;
        const door = bay.door && bay.door.v0 < band.v1 - EPS && bay.door.v1 > band.v0 + EPS ? bay.door : null;
        const cell = { u0: bay.u0, u1: bay.u1, v0: band.v0, v1: band.v1 };
        let m;
        if (band.cropped || hits(cell)) m = "plate";
        else if (door) m = "flank";
        else if (wd < S.bays.narrow_min_m - EPS) m = "plate";
        else if (wd < S.bays.full_min_m - EPS) m = "narrow";
        else {
          const key = `${S.rule.seed}|${comp.id}|${w.bi}|${w.i}|${bi}|${band.level}`;
          m = drawPanelModule(fin, key, [prev, below.get(bi)].filter(Boolean));
        }
        const shown = m;
        prev = m;
        below.set(bi, m);
        ctx.stats.bays++;
        ctx.stats.modules[shown] = (ctx.stats.modules[shown] || 0) + 1;
        // Bay-local u: centred, so a bay of width wd shows u 0.5 -+ wd / 2 / span. The flank's light
        // strip is on the module's right: with the door on the bay's left, u runs right to left
        // (mirrored). A flank narrower than a full bay is anchored at its door side, so the strip
        // stays in view.
        let uvOf;
        if (shown === "flank") {
          const edge = 1 - half / span, right = door.side === "right";
          const s = (u) => wd >= S.bays.full_min_m - EPS ? 0.5 + (right ? u - c : c - u) / span : edge - (right ? bay.u1 - u : u - bay.u0) / span;
          uvOf = (u, v) => [s(u), (v - band.v0) / Sb.module_m];
        } else {
          uvOf = (u, v) => [0.5 + (u - c) / span, (v - band.v0) / Sb.module_m];
        }
        const layer = panelLayerName(comp.finish, shown);
        for (const pc of rectMinusHoles(cell, holes)) { quad(pc, uvOf, layer); ctx.stats.cells++; }
      });
    }
  }

  /**
   * A cove (the 45 degree panel between a wall and the ceiling) on its strip: corners a, b along the
   * wall's top and ib, ia along the ceiling's edge. u runs along the wall to the viewer's right, as
   * the wall's strips do; v from the wall (0) to the ceiling (1), the row repeated whole when the
   * slope is deeper than stretch_max rows (a tall room's 0.6 m cove).
   */
  function coveStrip(B, T, a, b, ib, ia, n) {
    const M = T.members.cove, row = T.rows[M.face], span = T.span;
    const len = Math.hypot(b[0] - a[0], b[2] - a[2]), t = [(b[0] - a[0]) / len, 0, (b[2] - a[2]) / len];
    const slope = Math.hypot(ia[0] - a[0], ia[1] - a[1], ia[2] - a[2]);
    const reps = Math.max(1, Math.ceil(slope / (T.stretch * row.h_m) - 1e-6));
    const lerp = (p, q, s) => [p[0] + (q[0] - p[0]) * s, p[1] + (q[1] - p[1]) * s, p[2] + (q[2] - p[2]) * s];
    const v0 = row.v0_m / span, v1 = (row.v0_m + row.h_m) / span;
    for (let k = 0; k < reps; k++) {
      const s0 = k / reps, s1 = (k + 1) / reps;
      const pts = [lerp(a, ia, s0), lerp(b, ib, s0), lerp(b, ib, s1), lerp(a, ia, s1)];
      const uvs = pts.map((p, i) => [(p[0] * t[0] + p[2] * t[2]) / span, i < 2 ? v0 : v1]);
      B.triUv("cove", pts[0], pts[1], pts[2], uvs[0], uvs[1], uvs[2], n, T.layer);
      B.triUv("cove", pts[0], pts[2], pts[3], uvs[0], uvs[2], uvs[3], n, T.layer);
    }
    T.stats.faces++;
  }

  // ------------------------------------- ceilings and floors (ceilings-and-trims, floor-panels)

  /** A convex polygon [[x, z], ...] clipped to the rectangle x0..x1, z0..z1 (Sutherland-Hodgman). */
  function clipPolyRect(poly, x0, x1, z0, z1) {
    let out = poly;
    const edges = [[0, x0, 1], [0, x1, -1], [1, z0, 1], [1, z1, -1]];
    for (const [k, v, sgn] of edges) {
      const inp = out;
      out = [];
      for (let i = 0; i < inp.length; i++) {
        const a = inp[i], b = inp[(i + 1) % inp.length];
        const da = (a[k] - v) * sgn, db = (b[k] - v) * sgn;
        if (da >= 0) out.push(a);
        if ((da >= 0) !== (db >= 0)) {
          const t = da / (da - db);
          out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
      }
      if (out.length < 3) return [];
    }
    return Math.abs(signedArea(out)) > 1e-6 ? out : [];
  }

  /**
   * The walkway's bands in a compartment (floor-panels design section 1): [{ a, b ([x, z]), y (the
   * floor they lie on) }]. A room: from each door (a wall portal of rule.door_kinds) to the centroid
   * of the brushes on that door's floor. A corridor: down each brush's long axis through its
   * centroid, and from each door square onto that axis.
   */
  function walkwayPaths(L, comp, S) {
    const out = [];
    const doors = portalsOf(L, comp.id).filter((p) => Math.abs(p.normal[1]) < 0.5 && S.rule.door_kinds.indexOf(p.kind) >= 0)
      .map((p) => ({ x: p.center_m[0], z: p.center_m[2], y: p.center_m[1] - p.size_m[1] / 2 }));
    const onFloor = (y) => comp.brushes.filter((br) => Math.abs(br.y[0] - y) < 0.3);
    if (comp.kind === "corridor") {
      for (const br of comp.brushes) {
        const xs = br.poly.map((p) => p[0]), zs = br.poly.map((p) => p[1]), c = polyCentroid(br.poly);
        const alongZ = Math.max(...zs) - Math.min(...zs) >= Math.max(...xs) - Math.min(...xs);
        const a = alongZ ? [c[0], Math.min(...zs)] : [Math.min(...xs), c[1]], b = alongZ ? [c[0], Math.max(...zs)] : [Math.max(...xs), c[1]];
        out.push({ a, b, y: br.y[0] });
        for (const d of doors) {
          if (Math.abs(d.y - br.y[0]) > 0.3 || !insidePoly(br.poly, d.x, d.z, 0.05)) continue;
          out.push({ a: [d.x, d.z], b: alongZ ? [c[0], d.z] : [d.x, c[1]], y: br.y[0] });
        }
      }
      return out;
    }
    for (const d of doors) {
      const brs = onFloor(d.y);
      if (!brs.length) continue;
      let ax = 0, az = 0, aa = 0;
      for (const br of brs) { const ar = signedArea(br.poly), c = polyCentroid(br.poly); ax += c[0] * ar; az += c[1] * ar; aa += ar; }
      out.push({ a: [d.x, d.z], b: [ax / aa, az / aa], y: brs[0].y[0] });
    }
    return out;
  }

  /** The shortest distance from the segment a-b to the rectangle x0..x1, z0..z1 (0 when they meet). */
  function segRectDistance(a, b, x0, x1, z0, z1) {
    const len = Math.hypot(b[0] - a[0], b[1] - a[1]), n = Math.max(1, Math.ceil(len / 0.02));
    let best = Infinity;
    for (let i = 0; i <= n; i++) {
      const x = a[0] + ((b[0] - a[0]) * i) / n, z = a[1] + ((b[1] - a[1]) * i) / n;
      const dx = Math.max(0, x0 - x, x - x1), dz = Math.max(0, z0 - z, z - z1);
      best = Math.min(best, Math.hypot(dx, dz));
      if (best === 0) break;
    }
    return best;
  }

  /**
   * Dress one brush's ceiling (kind "ceiling", dir -1) or floor ("floor", dir 1) cell by cell
   * (ceilings-and-trims design sections 1 and 4, floor-panels design section 1): bays at the frames,
   * cells.size_m cells centred on x = origin_x_m + k * size_m, each cell a module of its finish's
   * set with cell-local texture coordinates, drawn into role kind with a layer per vertex. poly is
   * the surface's outline (a ceiling's is inset by its coves); holes are the floor portals cut in
   * it ({ rect, f }), zone their collar or rim width; lamps (a ceiling) are the lamps under it,
   * whose housings make the cells they span lamp_surround; paths (a floor) the walkway bands. A cell
   * is cropped (and takes plate) when its outline or a hole cuts its central core (cells.core_m
   * either side of its centre).
   */
  function dressFlat(B, comp, br, bi, kind, poly, holes, zone, y, dir, lamps, paths, ctx, D) {
    const S = ctx.S, set = S.finishes[comp.finish][kind], st = ctx.stats[kind];
    if (!set) throw new Error(`shipkit: finish ${comp.finish} has no ${kind} in panels.json`);
    const span = S.layers.span_m, cs = S.cells.size_m, ox = S.cells.origin_x_m, mg = S.cells.margin_m;
    const fs = D.frames.spacing_m, fo = D.frames.origin_z_m, n = [0, dir, 0], W = S.walkway;
    const xs = poly.map((p) => p[0]), zs = poly.map((p) => p[1]);
    const X0 = Math.min(...xs), X1 = Math.max(...xs), Z0 = Math.min(...zs), Z1 = Math.max(...zs);
    const cut = holes.map((h) => { const r = h.rect; return { u0: r[0][0], u1: r[2][0], v0: r[0][1], v1: r[2][1] }; });
    const zones = cut.map((h) => ({ u0: h.u0 - zone, u1: h.u1 + zone, v0: h.v0 - zone, v1: h.v1 + zone }));
    const meets = (h, x0, x1, z0, z1) => h.u0 < x1 - EPS && h.u1 > x0 + EPS && h.v0 < z1 - EPS && h.v1 > z0 + EPS;
    const myPaths = (paths || []).filter((p) => Math.abs(p.y - br.y[0]) < 0.3);
    const emit = (x0, x1, z0, z1, cu, cz, layer) => {
      for (const r of rectMinusHoles({ u0: x0, u1: x1, v0: z0, v1: z1 }, cut)) {
        const pg = clipPolyRect(poly, r.u0, r.u1, r.v0, r.v1);
        for (let i = 1; i + 1 < pg.length; i++) {
          const p = [pg[0], pg[i], pg[i + 1]].map((q) => [q[0], y, q[1]]);
          const uv = p.map((q) => [0.5 + (q[0] - cu) / span, 0.5 + (q[2] - cz) / span]);
          B.triUv(kind, p[0], p[1], p[2], uv[0], uv[1], uv[2], n, layer);
        }
      }
    };
    const count = (m) => { st.cells++; st.modules[m] = (st.modules[m] || 0) + 1; };
    const shown = new Map();
    const j0 = Math.floor((Z0 - fo) / fs + EPS), j1 = Math.ceil((Z1 - fo) / fs - EPS) - 1;
    const k0 = Math.floor((X0 - ox) / cs + 0.5 + EPS), k1 = Math.ceil((X1 - ox) / cs - 0.5 - EPS);
    const core = S.cells.core_m;
    for (let j = j0; j <= j1; j++) {
      const z0 = fo + j * fs, z1 = z0 + fs, zc = (z0 + z1) / 2;
      // A lamp's cells: every cell its housing spans takes lamp_surround, a light channel that runs
      // the cell's width, with v centred on the lamp so the housing sits in the channel.
      const lit = new Map();
      for (const l of lamps || []) {
        const lz = l.p[2], hx = l.size[0] / 2 + 0.05;
        if (lz < z0 - EPS || lz >= z1 - EPS) continue;
        for (let k = k0; k <= k1; k++) {
          const cx = ox + k * cs;
          if (cx + cs / 2 > l.p[0] - hx && cx - cs / 2 < l.p[0] + hx && !lit.has(k)) lit.set(k, lz);
        }
      }
      for (let k = k0; k <= k1; k++) {
        const cx = ox + k * cs, x0 = cx - cs / 2, x1 = cx + cs / 2;
        if (!clipPolyRect(poly, x0, x1, z0, z1).length) continue;
        const fx0 = cx - core, fx1 = cx + core, fz0 = zc - core, fz1 = zc + core;
        const whole = [[fx0, fz0], [fx1, fz0], [fx1, fz1], [fx0, fz1]].every(([x, z]) => insidePoly(poly, x, z)) && !cut.some((h) => meets(h, fx0, fx1, fz0, fz1));
        let m, vz = zc, why = "draw";
        if (lit.has(k)) { m = "lamp_surround"; vz = lit.get(k); why = "lamp"; }
        else if (zones.some((h) => meets(h, x0, x1, z0, z1))) { m = "plate"; why = "portal"; }
        else if (kind === "floor" && myPaths.some((p) => segRectDistance(p.a, p.b, x0, x1, z0, z1) <= W.width_m / 2 - W.min_overlap_m + EPS)) { m = "walkway"; st.walkway++; why = "walkway"; }
        else if (!whole) { m = "plate"; why = "cropped"; }
        else {
          const key = `${S.rule.seed}|${comp.id}|${bi}|${kind}|${j}|${k}`;
          m = drawPanelModule(set, key, [shown.get(`${j}|${k - 1}`), shown.get(`${j - 1}|${k}`)].filter(Boolean));
        }
        shown.set(`${j}|${k}`, m);
        count(m);
        st.rules[why] = (st.rules[why] || 0) + 1;
        emit(x0, x1, z0, z1, cx, vz, panelLayerName(comp.finish, m, kind));
      }
    }
  }

  /**
   * Build one compartment's surfaces: the shell (floors, walls, coves, ceilings with every
   * portal opening cut) and, unless opts.detail === false, the generated detail
   * (deck-pipeline section 5a). Returns { parts: { role: { position, normal, uvm } }, lamps,
   * walls, openings }. Roles: floor, wall, ceiling, cove, rib, beam, baseboard, frame,
   * frame_hazard, status, window_frame, rim, collar, ladder, runner, pipe, railing, kickplate,
   * lamp_housing, lamp.
   * opts.skipKinds: portal kinds not to cut (["bay_door"] keeps a closed bay door's floor);
   * opts.wallFixtures: things on a wall that ribs and baseboards must clear (see below).
   * opts.panels (true, or panels.json): the panel dressing (wall-panels, ceilings-and-trims,
   * floor-panels). Flat walls become role "panel"; floor, ceiling, rib, beam, cove, baseboard,
   * frame and window_frame carry a layer name per vertex (vlayer) and coordinates in layer units;
   * no runner is drawn; the result gains panels: the counts of bays, cells, modules and pillars.
   */
  function buildCompartment(THREE, L, comp, opts) {
    opts = opts || {};
    const D = opts.detailing || shipData("detailing");
    const detail = opts.detail !== false;
    const B = new Builder();
    const walls = wallsOf(L, comp, opts);
    const skip = opts.skipKinds || [];
    const openings = [];
    const FR = D.door_frame, WF = D.window_frame, FP = D.floor_portal, F = D.frames;
    const tallBrush = (br) => br.y[1] - br.y[0] > D.cove.tall_room_m + EPS;
    // Wall panels (openspec/changes/wall-panels), opt-in: opts.panels true (or the panels.json
    // data) dresses every flat wall bay by bay instead of one tiled "wall" surface, and with it the
    // ceilings and floors cell by cell and the trims on their strips (ceilings-and-trims,
    // floor-panels). A page without the option gets exactly the geometry it always had.
    const panelsOn = opts.panels && detail ? panelSetup(opts.panels) : null;
    const T = panelsOn ? trimContext(panelsOn, comp) : null;
    // A trim member's box: on its strips with panels, the plain box without.
    const tbox = (role, c, u, v, w, hu, hv, hw, faces, roles, front, pillar) =>
      T ? stripBox(B, T, role, c, u, v, w, hu, hv, hw, faces, roles, front, pillar) : B.box(role, c, u, v, w, hu, hv, hw, faces, roles);
    const lampList = lampsFor(L, comp, D);

    // Per brush: cove size by edge (0 where the edge opens to a sibling, a portal reaches the cove, or in a pod).
    const coveOf = new Map();
    for (const br of comp.brushes) coveOf.set(br, br.poly.map(() => 0));
    for (const w of walls) {
      const c = tallBrush(w.brush) ? D.cove.tall_size_m : D.cove.size_m;
      const blocked = w.holes.some((h) => h.v1 > w.y1 - c - EPS);
      coveOf.get(w.brush)[w.i] = comp.kind === "pod" || blocked ? 0 : c;
    }

    // A framed portal's wall hole is wider than its clear opening by the frame, which fills it.
    const frameOf = (p) => {
      if (p.kind === "window") return { side: WF.frame_m, top: WF.frame_m, bottom: WF.frame_m, proud: WF.proud_m, role: "window_frame" };
      if (p.kind === "pressure_door") return { side: FR.pressure_jamb_m, top: FR.lintel_m, bottom: 0, proud: FR.pressure_proud_m, role: "frame_hazard" };
      if (p.kind === "door" || p.kind === "hatch") return { side: FR.jamb_m, top: FR.lintel_m, bottom: 0, proud: FR.proud_m, role: "frame" };
      return null;
    };

    // ---- walls, and the detail that stands on them
    for (const w of walls) {
      const cv = coveOf.get(w.brush)[w.i];
      const top = w.y1 - cv;
      const P3 = (u, y, inward) => [w.a[0] + w.t[0] * u - w.n[0] * (inward || 0), y, w.a[1] + w.t[1] * u - w.n[1] * (inward || 0)];
      const nIn = [-w.n[0], 0, -w.n[1]], ax = [w.t[0], 0, w.t[1]], up = [0, 1, 0];
      const holes = w.holes.map((h) => {
        const f = h.portal && frameOf(h.portal);
        if (!f) return h;
        const v0 = h.v0 - f.bottom > w.y0 + 0.01 ? h.v0 - f.bottom : h.v0;
        return Object.assign({}, h, { u0: h.u0 - f.side, u1: h.u1 + f.side, v0, v1: Math.min(top, h.v1 + f.top), framed: f });
      });
      // With opts.panels the flat wall is drawn later, bay by bay (dressWall), once the ribs are known.
      if (!panelsOn) {
        for (const c of rectMinusHoles({ u0: 0, u1: w.len, v0: w.y0, v1: top }, holes)) {
          B.quad("wall", P3(c.u0, c.v0), P3(c.u1, c.v0), P3(c.u1, c.v1), P3(c.u0, c.v1), nIn);
        }
      }
      for (const h of holes) if (h.portal) openings.push({ portal: h.portal, wall: w, hole: h });
      if (!detail) continue;
      // Fixtures on this wall (a viewscreen, a console built into it) keep ribs and trims clear
      // without cutting the wall: opts.wallFixtures [{ center: [x, y, z], normal (into the room), width }].
      const clear = holes.slice();
      for (const f of opts.wallFixtures || []) {
        if (f.normal[0] * nIn[0] + f.normal[2] * nIn[2] < 0.99) continue;
        if (Math.abs((f.center[0] - w.a[0]) * w.n[0] + (f.center[2] - w.a[1]) * w.n[1]) > 0.05) continue;
        const u = (f.center[0] - w.a[0]) * w.t[0] + (f.center[2] - w.a[1]) * w.t[1];
        if (u + f.width / 2 < 0 || u - f.width / 2 > w.len) continue;
        clear.push({ u0: u - f.width / 2, u1: u + f.width / 2, v0: w.y0, v1: top });
      }

      // Ribs on fore-and-aft walls at the frames; stiffeners on athwartship walls between them.
      const ribs = [];
      if (comp.kind !== "pod") {
        const along = Math.abs(w.t[1]) >= Math.SQRT1_2;
        let stations;
        if (along) {
          stations = frameStations(D, Math.min(w.a[1], w.b[1]), Math.max(w.a[1], w.b[1]), 0).map((z) => (z - w.a[1]) / w.t[1]);
        } else {
          const s = F.spacing_m, x0 = Math.min(w.a[0], w.b[0]), x1 = Math.max(w.a[0], w.b[0]);
          stations = [];
          for (let k = Math.ceil((x0 - s / 2) / s); k * s + s / 2 <= x1; k++) stations.push((k * s + s / 2 - w.a[0]) / w.t[0]);
        }
        const depth = tallBrush(w.brush) ? F.tall_rib_depth_m : F.rib_depth_m;
        for (const u of stations) {
          if (u < F.end_clear_m || u > w.len - F.end_clear_m) continue;
          const lo = u - F.rib_width_m / 2 - F.portal_clear_m, hi = u + F.rib_width_m / 2 + F.portal_clear_m;
          if (clear.some((h) => h.u1 > lo && h.u0 < hi)) continue;
          ribs.push(u);
          const pl = T && (tallBrush(w.brush) ? { base_m: T.pillar.tall_base_m, capital_m: T.pillar.tall_capital_m } : { base_m: T.pillar.base_m, capital_m: T.pillar.capital_m });
          tbox("rib", P3(u, (w.y0 + top) / 2, depth / 2), ax, up, nIn, F.rib_width_m / 2, (top - w.y0) / 2, depth / 2, ["+u", "-u", "+v", "+w"], null, "+w", pl);
        }
      }
      // Baseboard between ribs and doors.
      const BB = D.baseboard;
      const cuts = clear.filter((h) => h.v0 < w.y0 + BB.height_m).map((h) => [h.u0, h.u1]).concat(ribs.map((u) => [u - F.rib_width_m / 2, u + F.rib_width_m / 2]));
      for (const [u0, u1] of intervalMinus(0, w.len, cuts)) {
        if (u1 - u0 < 0.05) continue;
        tbox("baseboard", P3((u0 + u1) / 2, w.y0 + BB.height_m / 2, BB.depth_m / 2), ax, up, nIn, (u1 - u0) / 2, BB.height_m / 2, BB.depth_m / 2, ["+v", "+w", "+u", "-u"], null, "+w");
      }
      // Door and window frames on this side: jambs, lintel, sill, and a status strip.
      for (const h of holes) {
        if (!h.framed) continue;
        const f = h.framed, p = h.portal;
        const pv0 = p.center_m[1] - p.size_m[1] / 2, pv1 = Math.min(p.center_m[1] + p.size_m[1] / 2, h.v1);
        const pu0 = h.u0 + f.side, pu1 = h.u1 - f.side, jv0 = h.v0, jv1 = h.v1;
        const inner = { "+u": "frame", "-u": "frame" };
        tbox(f.role, P3(pu0 - f.side / 2, (jv0 + jv1) / 2, f.proud / 2), ax, up, nIn, f.side / 2, (jv1 - jv0) / 2, f.proud / 2, ["+u", "-u", "+w", "+v"], inner, "+w");
        tbox(f.role, P3(pu1 + f.side / 2, (jv0 + jv1) / 2, f.proud / 2), ax, up, nIn, f.side / 2, (jv1 - jv0) / 2, f.proud / 2, ["+u", "-u", "+w", "+v"], inner, "+w");
        const lintelRole = f.role === "frame_hazard" ? "frame" : f.role;
        if (jv1 - pv1 > EPS) tbox(lintelRole, P3((pu0 + pu1) / 2, (pv1 + jv1) / 2, f.proud / 2), ax, up, nIn, (pu1 - pu0) / 2, (jv1 - pv1) / 2, f.proud / 2, ["-v", "+w", "+v"], null, "+w");
        if (pv0 - jv0 > EPS) tbox(f.role, P3((pu0 + pu1) / 2, (jv0 + pv0) / 2, WF.sill_depth_m / 2), ax, up, nIn, (pu1 - pu0) / 2, (pv0 - jv0) / 2, WF.sill_depth_m / 2, ["+v", "+w", "-v"], null, "+w");
        if (p.kind !== "window" && jv1 - pv1 > FR.status_strip_m[1]) {
          const sw = Math.min(FR.status_strip_m[0], pu1 - pu0) / 2;
          B.box("status", P3((pu0 + pu1) / 2, (pv1 + jv1) / 2, f.proud + 0.01), ax, up, nIn, sw, FR.status_strip_m[1] / 2, 0.01, ["+w", "-v", "+v", "+u", "-u"]);
        }
      }
      // A railing along an edge that opens onto a lower floor of the same compartment (a gallery's edge).
      const RL = D.railing;
      for (const h of w.holes) {
        if (!h.sibling || h.siblingFloor > w.y0 - RL.min_drop_m) continue;
        const u0 = h.u0 + RL.post_m, u1 = h.u1 - RL.post_m, inset = RL.inset_m + RL.post_m / 2, hp = RL.post_m / 2;
        if (u1 - u0 < 0.3) continue;
        for (const y of [w.y0 + RL.height_m, w.y0 + RL.height_m / 2]) {
          B.box("railing", P3((u0 + u1) / 2, y, inset), ax, up, nIn, (u1 - u0) / 2, hp, hp, ["+v", "-v", "+w", "-w", "+u", "-u"]);
        }
        B.box("kickplate", P3((u0 + u1) / 2, w.y0 + RL.kick_m / 2, inset), ax, up, nIn, (u1 - u0) / 2, RL.kick_m / 2, 0.01, ["+v", "+w", "-w", "+u", "-u"]);
        const n = Math.max(1, Math.round((u1 - u0) / RL.post_spacing_m));
        for (let k = 0; k <= n; k++) {
          const u = u0 + ((u1 - u0) * k) / n;
          B.box("railing", P3(u, w.y0 + RL.height_m / 2, inset), ax, up, nIn, hp, RL.height_m / 2, hp, ["+u", "-u", "+w", "-w"]);
        }
      }
      // Conduits along fore-and-aft corridor walls.
      if (comp.kind === "corridor" && Math.abs(w.t[1]) > 0.9) {
        const C = D.conduits;
        let y = top - C.drop_m;
        for (const r of C.radii_m) {
          y -= r;
          const inset = F.rib_depth_m + C.gap_m + r;
          B.pipe("pipe", P3(0.02, y, inset), P3(w.len - 0.02, y, inset), r, C.sides);
          y -= r + C.gap_m;
        }
      }
      if (panelsOn) dressWall(B, comp, w, top, holes, clear, ribs, panelsOn, P3, nIn);
    }

    // ---- per brush: coves, floor, ceiling, and the detail on them
    const walkPaths = panelsOn ? walkwayPaths(L, comp, panelsOn.S) : null;
    for (const br of comp.brushes) {
      const cvs = coveOf.get(br), inner = insetPoly(br.poly, cvs), npts = br.poly.length;
      for (let i = 0; i < npts; i++) {
        if (!cvs[i]) continue;
        const a = br.poly[i], b = br.poly[(i + 1) % npts], ia = inner[i], ib = inner[(i + 1) % npts];
        const on = outwardNormal(a, b), n = [-on[0] * Math.SQRT1_2, -Math.SQRT1_2, -on[1] * Math.SQRT1_2], y = br.y[1];
        if (T) coveStrip(B, T, [a[0], y - cvs[i], a[1]], [b[0], y - cvs[i], b[1]], [ib[0], y, ib[1]], [ia[0], y, ia[1]], n);
        else B.quad("cove", [a[0], y - cvs[i], a[1]], [b[0], y - cvs[i], b[1]], [ib[0], y, ib[1]], [ia[0], y, ia[1]], n);
      }
      // Floor portals that open in this brush's floor or ceiling.
      const fHoles = [], cHoles = [];
      for (const p of portalsOf(L, comp.id)) {
        const f = portalFrame(p);
        if (!f.floor || skip.indexOf(p.kind) >= 0 || !insidePoly(br.poly, f.c[0], f.c[2])) continue;
        const o = portalOut(p, comp.id);
        const rect = [[f.c[0] - f.sx / 2, f.c[2] - f.sz / 2], [f.c[0] + f.sx / 2, f.c[2] - f.sz / 2], [f.c[0] + f.sx / 2, f.c[2] + f.sz / 2], [f.c[0] - f.sx / 2, f.c[2] + f.sz / 2]];
        if (o[1] < 0 && Math.abs(br.y[0] - f.c[1]) < 0.6) fHoles.push({ rect, f, p });
        if (o[1] > 0 && Math.abs(br.y[1] - f.c[1]) < 0.6) cHoles.push({ rect, f, p });
      }
      if (panelsOn) {
        const bi = comp.brushes.indexOf(br);
        const under = lampList.filter((l) => insidePoly(br.poly, l.p[0], l.p[2]) && l.p[1] > br.y[0] && l.p[1] < br.y[1] + EPS);
        dressFlat(B, comp, br, bi, "floor", br.poly, fHoles, FP.rim_width_m, br.y[0], 1, null, walkPaths, panelsOn, D);
        dressFlat(B, comp, br, bi, "ceiling", inner, cHoles, FP.collar_width_m, br.y[1], -1, under, null, panelsOn, D);
      } else {
        B.flat(THREE, "floor", br.poly, fHoles.map((h) => h.rect), br.y[0], 1);
        B.flat(THREE, "ceiling", inner, cHoles.map((h) => h.rect), br.y[1], -1);
      }
      for (const h of fHoles.concat(cHoles)) openings.push({ portal: h.p, floor: true, frame: h.f, brush: br });
      if (!detail) continue;

      const zs = br.poly.map((p) => p[1]), z0 = Math.min(...zs), z1 = Math.max(...zs);
      const SIDES = [[0, -1], [0, 1], [-1, 0], [1, 0]];
      // Hazard rims round floor holes.
      for (const h of fHoles) {
        const f = h.f, W = FP.rim_width_m, y = br.y[0] + FP.raise_m / 2;
        for (const [sx, sz] of SIDES) {
          const along = sx === 0, hu = along ? f.sx / 2 + W : W / 2, hw = along ? W / 2 : f.sz / 2;
          B.box("rim", [f.c[0] + sx * (f.sx / 2 + W / 2), y, f.c[2] + sz * (f.sz / 2 + W / 2)], [1, 0, 0], [0, 1, 0], [0, 0, 1], hu, FP.raise_m / 2, hw, ["+v", "+u", "-u", "+w", "-w"]);
        }
      }
      // Collars round ceiling holes, and ladders up through them.
      for (const h of cHoles) {
        const f = h.f, W = FP.collar_width_m, y = br.y[1] - FP.collar_depth_m / 2;
        for (const [sx, sz] of SIDES) {
          const along = sx === 0, hu = along ? f.sx / 2 + W : W / 2, hw = along ? W / 2 : f.sz / 2;
          B.box("collar", [f.c[0] + sx * (f.sx / 2 + W / 2), y, f.c[2] + sz * (f.sz / 2 + W / 2)], [1, 0, 0], [0, 1, 0], [0, 0, 1], hu, FP.collar_depth_m / 2, hw, ["-v", "+u", "-u", "+w", "-w"]);
        }
        if (h.p.kind === "ladder") {
          const topY = br.y[1] + 0.5 + FP.ladder_above_m, bot = br.y[0], zl = f.c[2] - f.sz / 2 + FP.ladder_rail_m, hr = FP.ladder_rail_m / 2;
          for (const s of [-1, 1]) {
            B.box("ladder", [f.c[0] + (s * FP.ladder_width_m) / 2, (topY + bot) / 2, zl], [1, 0, 0], [0, 1, 0], [0, 0, 1], hr, (topY - bot) / 2, hr, ["+u", "-u", "+w", "-w", "+v"]);
          }
          for (let y = bot + FP.rung_spacing_m; y < topY - 0.05; y += FP.rung_spacing_m) {
            B.box("ladder", [f.c[0], y, zl], [1, 0, 0], [0, 1, 0], [0, 0, 1], FP.ladder_width_m / 2, hr * 0.7, hr * 0.7, ["+v", "-v", "+w", "-w"]);
          }
        }
      }
      // A runner down a corridor, unless the floor is dressed: its walkway does the runner's job (floor-panels).
      if (comp.kind === "corridor" && !panelsOn) {
        const R = D.runner, xs = br.poly.map((p) => p[0]), x0 = Math.min(...xs), x1 = Math.max(...xs);
        const alongZ = z1 - z0 >= x1 - x0;
        const lo = (alongZ ? z0 : x0) + R.end_clear_m, hi = (alongZ ? z1 : x1) - R.end_clear_m, mid = alongZ ? (x0 + x1) / 2 : (z0 + z1) / 2;
        const rc = fHoles.map((h) => alongZ ? [h.f.c[2] - h.f.sz / 2 - FP.rim_width_m, h.f.c[2] + h.f.sz / 2 + FP.rim_width_m] : [h.f.c[0] - h.f.sx / 2 - FP.rim_width_m, h.f.c[0] + h.f.sx / 2 + FP.rim_width_m]);
        for (const [a, b] of intervalMinus(lo, hi, rc)) {
          if (b - a < 0.1) continue;
          const c = alongZ ? [mid, br.y[0] + R.raise_m / 2, (a + b) / 2] : [(a + b) / 2, br.y[0] + R.raise_m / 2, mid];
          B.box("runner", c, alongZ ? [1, 0, 0] : [0, 0, 1], [0, 1, 0], alongZ ? [0, 0, 1] : [1, 0, 0], R.width_m / 2, R.raise_m / 2, (b - a) / 2, ["+v", "+u", "-u", "+w", "-w"]);
        }
      }
      // Beams under the ceiling at the frames, butting the coves, split round tall systems and ceiling holes.
      if (comp.kind !== "pod") {
        const keep = systemsIn(L, comp.id).filter((s) => s.radius_m);
        for (const z of frameStations(D, z0, z1, F.end_clear_m + 0.05)) {
          const ch = chord(inner, 1, z);
          if (!ch) continue;
          const d = tallBrush(br) ? F.deep_beam_depth_m : F.beam_depth_m;
          const bc = [];
          for (const s of keep) {
            const dz = Math.abs(z - s.center_m[2]), r = s.radius_m + 0.1;
            if (dz < r + F.beam_width_m) { const hx = Math.sqrt(Math.max(0, r * r - Math.min(dz, r) ** 2)) + F.beam_width_m; bc.push([s.center_m[0] - hx, s.center_m[0] + hx]); }
          }
          for (const h of cHoles) if (Math.abs(z - h.f.c[2]) < h.f.sz / 2 + F.beam_width_m) bc.push([h.f.c[0] - h.f.sx / 2 - FP.collar_width_m, h.f.c[0] + h.f.sx / 2 + FP.collar_width_m]);
          for (const [a, b] of intervalMinus(ch[0], ch[1], bc)) {
            if (b - a < 0.2) continue;
            tbox("beam", [(a + b) / 2, br.y[1] - d / 2, z], [1, 0, 0], [0, 1, 0], [0, 0, 1], (b - a) / 2, d / 2, F.beam_width_m / 2, ["-v", "+w", "-w", "+u", "-u"], null, "-v");
          }
        }
      }
    }

    // ---- lamp fixtures: a housing at the ceiling whose underside is the lens
    const lamps = lampList;
    if (detail) {
      const hh = D.lamps.housing_m / 2;
      for (const l of lamps) {
        B.box("lamp_housing", [l.p[0], l.p[1] + hh, l.p[2]], [1, 0, 0], [0, 1, 0], [0, 0, 1], l.size[0] / 2, hh, l.size[1] / 2, ["-v", "+u", "-u", "+w", "-w"], { "-v": "lamp" });
      }
    }
    return panelsOn ? { parts: B.parts, lamps, walls, openings, panels: panelsOn.stats } : { parts: B.parts, lamps, walls, openings };
  }

  // ------------------------------------------------------------- materials

  /** The materials inlined as <script id="ship-materials">: { manifest, layers: { name: "data:image/png;base64,..." } }. */
  function materialsData() {
    const el = document.getElementById("ship-materials");
    if (!el) throw new Error("shipkit: no #ship-materials script in the page (add <!-- INLINE materials --> markers and run tools/mockups/inline.py)");
    return JSON.parse(el.textContent);
  }

  /**
   * Decode a PNG data URI to straight (not premultiplied) RGBA bytes. A canvas would
   * premultiply, and Star Crew layers carry the emission mask in alpha, so a texel with
   * alpha 0 would lose its colour (surface-materials design).
   */
  async function decodePng(uri) {
    const bin = Uint8Array.from(atob(uri.slice(uri.indexOf(",") + 1)), (ch) => ch.charCodeAt(0));
    const dv = new DataView(bin.buffer);
    let pos = 8, w = 0, h = 0, depth = 0, ctype = 0, plte = null, trns = null;
    const idat = [];
    while (pos < bin.length) {
      const len = dv.getUint32(pos), type = String.fromCharCode(bin[pos + 4], bin[pos + 5], bin[pos + 6], bin[pos + 7]);
      const data = bin.subarray(pos + 8, pos + 8 + len);
      if (type === "IHDR") { w = dv.getUint32(pos + 8); h = dv.getUint32(pos + 12); depth = bin[pos + 16]; ctype = bin[pos + 17]; if (bin[pos + 20]) throw new Error("shipkit: interlaced PNG"); }
      else if (type === "PLTE") plte = data;
      else if (type === "tRNS") trns = data;
      else if (type === "IDAT") idat.push(data);
      else if (type === "IEND") break;
      pos += 12 + len;
    }
    if (depth !== 8) throw new Error("shipkit: PNG bit depth " + depth + " (8 expected)");
    const bpp = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 }[ctype];
    const raw = new Uint8Array(await new Response(new Blob(idat).stream().pipeThrough(new DecompressionStream("deflate"))).arrayBuffer());
    const stride = w * bpp, out = new Uint8Array(w * h * 4);
    let prev = new Uint8Array(stride), cur = new Uint8Array(stride), p = 0;
    for (let y = 0; y < h; y++) {
      const f = raw[p++];
      for (let x = 0; x < stride; x++) {
        const a = x >= bpp ? cur[x - bpp] : 0, b = prev[x], c = x >= bpp ? prev[x - bpp] : 0;
        let v = raw[p++];
        if (f === 1) v += a; else if (f === 2) v += b; else if (f === 3) v += (a + b) >> 1;
        else if (f === 4) { const pa = Math.abs(b - c), pb = Math.abs(a - c), pc = Math.abs(a + b - 2 * c); v += pa <= pb && pa <= pc ? a : pb <= pc ? b : c; }
        cur[x] = v & 255;
      }
      for (let x = 0; x < w; x++) {
        const o = (y * w + x) * 4;
        if (ctype === 6) { out[o] = cur[x * 4]; out[o + 1] = cur[x * 4 + 1]; out[o + 2] = cur[x * 4 + 2]; out[o + 3] = cur[x * 4 + 3]; }
        else if (ctype === 2) { out[o] = cur[x * 3]; out[o + 1] = cur[x * 3 + 1]; out[o + 2] = cur[x * 3 + 2]; out[o + 3] = 255; }
        else if (ctype === 3) { const i = cur[x]; out[o] = plte[i * 3]; out[o + 1] = plte[i * 3 + 1]; out[o + 2] = plte[i * 3 + 2]; out[o + 3] = trns && i < trns.length ? trns[i] : 255; }
        else if (ctype === 0) { out[o] = out[o + 1] = out[o + 2] = cur[x]; out[o + 3] = 255; }
        else { out[o] = out[o + 1] = out[o + 2] = cur[x * 2]; out[o + 3] = cur[x * 2 + 1]; }
      }
      const t = prev; prev = cur; cur = t;
    }
    return { w, h, rgba: out };
  }

  /**
   * Load the Material Maker layers into one texture array, the way the engine will
   * (surface-materials): nearest on magnification, mipmapped on minification, sRGB.
   * Resolves to { texture, names, layer: { name: index }, span: { name: metres }, manifest, bytes }.
   */
  async function loadMaterials(THREE) {
    const d = materialsData(), man = d.manifest;
    const names = Object.keys(man.materials).sort((a, b) => man.materials[a].layer - man.materials[b].layer);
    names.forEach((n, i) => { if (man.materials[n].layer !== i) throw new Error("shipkit: material layers are not numbered 0.." + (names.length - 1)); });
    const px = man.layers.px, data = new Uint8Array(px * px * 4 * names.length);
    for (let i = 0; i < names.length; i++) {
      const img = await decodePng(d.layers[names[i]]);
      if (img.w !== px || img.h !== px) throw new Error(`shipkit: layer ${names[i]} is ${img.w}x${img.h}, expected ${px}`);
      putLayer(data, i, px, img);
    }
    const tex = layerTexture(THREE, data, px, names.length);
    const layer = {}, span = {};
    names.forEach((n, i) => { layer[n] = i; span[n] = man.materials[n].span_m; });
    return { texture: tex, names, layer, span, manifest: man, bytes: Math.round((data.length * 4) / 3) };
  }

  /** Copy a decoded layer into the array's data as layer i, rows flipped so v = 0 is the image's bottom (a wall's texture stands the right way up). */
  function putLayer(data, i, px, img) {
    for (let y = 0; y < px; y++) data.set(img.rgba.subarray((px - 1 - y) * px * 4, (px - y) * px * 4), (i * px + y) * px * 4);
  }

  /** The texture array the engine will use (surface-materials): sRGB, nearest on magnification, mipmapped on minification. */
  function layerTexture(THREE, data, px, n) {
    const tex = new THREE.DataArrayTexture(data, px, px, n);
    tex.format = THREE.RGBAFormat; tex.type = THREE.UnsignedByteType; tex.colorSpace = THREE.SRGBColorSpace;
    tex.magFilter = THREE.NearestFilter; tex.minFilter = THREE.LinearMipmapLinearFilter; tex.generateMipmaps = true;
    tex.wrapS = tex.wrapT = THREE.RepeatWrapping; tex.needsUpdate = true;
    return tex;
  }

  /** A decoded image scaled up to px by nearest neighbour (a whole factor), as the engine would fill a larger array. */
  function scaleNearest(img, px) {
    if (img.w === px) return img;
    const f = px / img.w;
    if (f !== Math.round(f) || img.h !== img.w) throw new Error(`shipkit: cannot scale a ${img.w} px layer to ${px} px`);
    const out = new Uint8Array(px * px * 4);
    for (let y = 0; y < px; y++) for (let x = 0; x < px; x++) {
      const s = (Math.floor(y / f) * img.w + Math.floor(x / f)) * 4, o = (y * px + x) * 4;
      out[o] = img.rgba[s]; out[o + 1] = img.rgba[s + 1]; out[o + 2] = img.rgba[s + 2]; out[o + 3] = img.rgba[s + 3];
    }
    return { w: px, h: px, rgba: out };
  }

  /**
   * The materials and the panels in one texture array (wall-panels design section 5): the
   * materials' layers first, then every panel layer in panels.json's numbering (walls, strips,
   * ceilings, floors and trims; panelLayerList), all opts.px wide
   * (128: 64 px per metre; 256: 128 px per metre, with the materials scaled up by nearest
   * neighbour). Resolves to loadMaterials' shape plus panels: { manifest, first, px }. Panel layers
   * have span 1 (their texture coordinates are already in layer units), and surfaceMaterial makes
   * their alpha glow.
   */
  async function loadPanels(THREE, mats, opts) {
    opts = opts || {};
    const d = panelsData(), S = d.manifest, px = opts.px || 128, md = materialsData(), first = mats.names.length;
    if (S.layers.first_layer !== first) throw new Error(`shipkit: panels.json first_layer ${S.layers.first_layer}, but the page has ${first} materials`);
    const files = d.layers[String(px)];
    if (!files) throw new Error(`shipkit: no ${px} px panel layers in the page (layers.sizes_px)`);
    const pnames = [];
    for (const r of panelLayerList(S)) pnames[r.layer - first] = [r.name, r.stem];
    const n = first + pnames.length, data = new Uint8Array(px * px * 4 * n);
    for (let i = 0; i < first; i++) putLayer(data, i, px, scaleNearest(await decodePng(md.layers[mats.names[i]]), px));
    for (let j = 0; j < pnames.length; j++) {
      if (!pnames[j] || !files[pnames[j][1]]) throw new Error(`shipkit: panel layer ${first + j} is missing (panels.json numbering)`);
      const img = await decodePng(files[pnames[j][1]]);
      if (img.w !== px || img.h !== px) throw new Error(`shipkit: panel layer ${pnames[j][1]} is ${img.w}x${img.h}, expected ${px}`);
      putLayer(data, first + j, px, img);
    }
    const layer = Object.assign({}, mats.layer), span = Object.assign({}, mats.span);
    pnames.forEach(([name], j) => { layer[name] = first + j; span[name] = 1; });
    return {
      texture: layerTexture(THREE, data, px, n), names: mats.names.concat(pnames.map((q) => q[0])), layer, span,
      manifest: mats.manifest, bytes: Math.round((data.length * 4) / 3), panels: { manifest: S, first, px },
    };
  }

  /** A per-vertex layer name (wall panels) as an index into mats' array; 0 with no mats. */
  function vlayerIndex(mats, name) {
    if (!mats) return 0;
    const i = mats.layer[name];
    if (i === undefined) throw new Error(`shipkit: no texture layer ${name} (wall panels need ShipKit.loadPanels)`);
    return i;
  }

  /**
   * The one surface material every textured mesh uses: the vertex's layer, times the vertex
   * colour. opts.lit true (default): Lambert, lit by the scene's lights; texels glow where the
   * vertex's surfGlow is set (lamp lenses, status strips), tinted by material.userData.emissive
   * (a THREE.Color the page sets per lighting state). opts.lit false: unlit, the vertex colour
   * is the baked light (the engine's way).
   */
  function surfaceMaterial(THREE, mats, opts) {
    opts = opts || {};
    const lit = opts.lit !== false;
    const m = lit ? new THREE.MeshLambertMaterial({ vertexColors: true }) : new THREE.MeshBasicMaterial({ vertexColors: true });
    if (opts.side !== undefined) m.side = opts.side;
    if (opts.transparent) { m.transparent = true; m.opacity = opts.opacity === undefined ? 1 : opts.opacity; m.depthWrite = opts.depthWrite !== false; }
    m.userData.emissive = new THREE.Color(opts.emissive === undefined ? 0xffe9c8 : opts.emissive);
    m.userData.glowGain = { value: opts.glowGain === undefined ? 1.0 : opts.glowGain };
    // Wall panels (loadPanels): a panel layer's alpha is its emission mask, scaled per lighting
    // state by material.userData.panelGlow.value (panels.json glow). Pages without panels compile
    // exactly the shader they always did.
    const panels = mats.panels || null;
    if (panels) m.userData.panelGlow = { value: 1.0 };
    m.onBeforeCompile = (sh) => {
      sh.uniforms.uLayers = { value: mats.texture };
      sh.uniforms.uGlowTint = { value: m.userData.emissive };
      sh.uniforms.uGlowGain = m.userData.glowGain;
      sh.vertexShader = sh.vertexShader
        .replace("#include <common>", "#include <common>\nattribute vec2 surfUv;\nattribute float surfLayer;\nattribute float surfGlow;\nvarying vec3 vSurf;\nvarying float vGlow;")
        .replace("#include <begin_vertex>", "#include <begin_vertex>\nvSurf = vec3(surfUv, surfLayer);\nvGlow = surfGlow;");
      sh.fragmentShader = sh.fragmentShader
        .replace("#include <common>", "#include <common>\nprecision highp sampler2DArray;\nuniform sampler2DArray uLayers;\nuniform vec3 uGlowTint;\nuniform float uGlowGain;\nvarying vec3 vSurf;\nvarying float vGlow;")
        .replace("#include <map_fragment>", "vec4 surfTex = texture(uLayers, vec3(vSurf.xy, floor(vSurf.z + 0.5)));\ndiffuseColor.rgb *= surfTex.rgb;");
      if (lit) sh.fragmentShader = sh.fragmentShader.replace("#include <emissivemap_fragment>", "#include <emissivemap_fragment>\ntotalEmissiveRadiance += surfTex.rgb * vGlow * uGlowTint * uGlowGain;");
      if (panels) {
        sh.uniforms.uPanelFirst = { value: panels.first };
        sh.uniforms.uPanelGlow = m.userData.panelGlow;
        sh.fragmentShader = sh.fragmentShader
          .replace("uniform float uGlowGain;", "uniform float uGlowGain;\nuniform float uPanelFirst;\nuniform float uPanelGlow;")
          .replace("diffuseColor.rgb *= surfTex.rgb;", "diffuseColor.rgb *= surfTex.rgb;\nfloat panelGlow = step(uPanelFirst - 0.5, vSurf.z) * surfTex.a * uPanelGlow;");
        if (lit) sh.fragmentShader = sh.fragmentShader.replace("totalEmissiveRadiance += surfTex.rgb * vGlow", "totalEmissiveRadiance += surfTex.rgb * panelGlow;\ntotalEmissiveRadiance += surfTex.rgb * vGlow");
        else sh.fragmentShader = sh.fragmentShader.replace("#include <color_fragment>", "#include <color_fragment>\ndiffuseColor.rgb = mix(diffuseColor.rgb, surfTex.rgb, panelGlow);");
      }
    };
    m.customProgramCacheKey = () => "shipkit-surface-" + (lit ? "lit" : "baked") + (panels ? "-panels" : "");
    return m;
  }

  /**
   * Merge parts into one BufferGeometry: position, normal, color (white), surfUv (metres
   * over the material's span), surfLayer, surfGlow. finish maps role to material name
   * (detailing.json finishes[comp.finish]); with no mats every vertex gets layer 0.
   * geometry.userData.roles = { role: [firstVertex, count] } for recolouring. A part may
   * carry tint (an r, g, b per vertex: an albedo tint the bake keeps) and glow (one per
   * vertex). Roles lamp, status and screen glow by default. finish may also name a role's
   * material through parts[role].material.
   */
  function geometryOf(THREE, parts, mats, finish) {
    let n = 0;
    for (const r in parts) n += parts[r].position.length / 3;
    const pos = new Float32Array(n * 3), nor = new Float32Array(n * 3), col = new Float32Array(n * 3).fill(1);
    const uv = new Float32Array(n * 2), lay = new Float32Array(n), glow = new Float32Array(n);
    const anyTint = Object.values(parts).some((P) => P.tint);
    const tint = anyTint ? new Float32Array(n * 3).fill(1) : null;
    const roles = {};
    let k = 0;
    for (const r of Object.keys(parts).sort()) {
      const P = parts[r], cnt = P.position.length / 3;
      let li = 0, span = 1;
      if (mats && !P.vlayer) {
        const name = P.material || (finish && finish[r]);
        if (!name || !(name in mats.layer)) throw new Error(`shipkit: no material for role ${r} (detailing.json finishes)`);
        li = mats.layer[name]; span = mats.span[name];
      }
      const g = r === "lamp" || r === "status" || r === "screen" ? 1 : 0;
      pos.set(P.position, k * 3); nor.set(P.normal, k * 3);
      for (let i = 0; i < cnt; i++) { uv[(k + i) * 2] = P.uvm[i * 2] / span; uv[(k + i) * 2 + 1] = P.uvm[i * 2 + 1] / span; lay[k + i] = P.vlayer ? vlayerIndex(mats, P.vlayer[i]) : li; glow[k + i] = P.glow ? P.glow[i] : g; }
      if (P.tint) { tint.set(P.tint, k * 3); col.set(P.tint, k * 3); }
      roles[r] = [k, cnt];
      k += cnt;
    }
    const geo = new THREE.BufferGeometry();
    geo.setAttribute("position", new THREE.BufferAttribute(pos, 3));
    geo.setAttribute("normal", new THREE.BufferAttribute(nor, 3));
    geo.setAttribute("color", new THREE.BufferAttribute(col, 3));
    geo.setAttribute("surfUv", new THREE.BufferAttribute(uv, 2));
    geo.setAttribute("surfLayer", new THREE.BufferAttribute(lay, 1));
    geo.setAttribute("surfGlow", new THREE.BufferAttribute(glow, 1));
    if (tint) geo.setAttribute("tint", new THREE.BufferAttribute(tint, 3));
    geo.userData.roles = roles;
    return geo;
  }

  /** The finish table for a compartment: { role: material name }. */
  function finishOf(comp, D) {
    D = D || shipData("detailing");
    const f = D.finishes[comp.finish];
    if (!f) throw new Error(`shipkit: compartment ${comp.id} has finish ${comp.finish}, not in detailing.json finishes`);
    return f;
  }

  /**
   * The shell only (no detail): { floor, ceiling, walls, coves } as BufferGeometries with
   * a white colour attribute and no material layers, plus the openings and the lamps.
   */
  function roomShell(THREE, L, comp, opts) {
    const r = buildCompartment(THREE, L, comp, Object.assign({}, opts, { detail: false }));
    const g = (role) => geometryOf(THREE, r.parts[role] ? { [role]: r.parts[role] } : {}, null);
    return { floor: g("floor"), ceiling: g("ceiling"), walls: g("wall"), coves: g("cove"), openings: r.openings, lamps: r.lamps };
  }

  /**
   * One compartment as one textured mesh (one draw call, as the engine draws it with a
   * texture array): shell plus detail. Returns { mesh, built }. opts as buildCompartment,
   * plus opts.material (default: one lit surfaceMaterial shared by every call with these mats).
   */
  function compartmentMesh(THREE, L, comp, mats, opts) {
    opts = opts || {};
    const D = opts.detailing || shipData("detailing");
    const built = buildCompartment(THREE, L, comp, Object.assign({ detailing: D }, opts));
    const geo = geometryOf(THREE, built.parts, mats, finishOf(comp, D));
    const mat = opts.material || mats._lit || (mats._lit = surfaceMaterial(THREE, mats, { lit: true }));
    const mesh = new THREE.Mesh(geo, mat);
    mesh.name = comp.id;
    mesh.userData.compartment = comp.id;
    return { mesh, built };
  }

  /** Paint every vertex of a role in a merged geometry (lamp lenses per lighting state, for example). */
  function paintRole(geo, role, color) {
    const r = geo.userData.roles[role];
    if (!r) return;
    const c = geo.attributes.color;
    for (let i = r[0]; i < r[0] + r[1]; i++) c.setXYZ(i, color.r, color.g, color.b);
    c.needsUpdate = true;
  }

  /**
   * The stand-in bake (deck-pipeline section 7 until sc-tools bake exists): direct light
   * from the compartment's lamps (lampsFor), no shadows or bounce, into the vertex colours
   * of a merged geometry, for one lighting state; the emergency state lights only the
   * emergency-bus lamps. A lamp may carry color ([r, g, b], times the state's lamp colour)
   * or, with fixed: true, a colour of its own in every state (a console's glow, a strip).
   * A vertex's tint (geometryOf) multiplies its light. Lamp lenses are painted lit or dark
   * lamp by lamp (the lamps that are not fixed, in order, one lens each); status strips the
   * state's colour; screens keep their tint, brightened. Use with surfaceMaterial({ lit: false }).
   * The light-baking mockup has the real baker (lightbake.js); this is for pages that only
   * need rooms to read as lit. opts.gain (default 0.55) scales lamp light; opts.cap (1.6)
   * clamps a vertex.
   */
  function bakeDirect(THREE, geo, lamps, key, opts) {
    opts = opts || {};
    const s = LIGHTING[key], gain = opts.gain === undefined ? 0.55 : opts.gain, cap = opts.cap || 1.6;
    const amb = new THREE.Color(s.ambient).multiplyScalar(s.ambientI * 2.2), lampC = new THREE.Color(s.lamp).multiplyScalar(s.lampI);
    const P = geo.attributes.position.array, N = geo.attributes.normal.array, C = geo.attributes.color;
    const T = geo.attributes.tint ? geo.attributes.tint.array : null;
    const on = lamps.filter((l) => l.fixed || key !== "emergency" || l.emergency).map((l) => {
      const c = l.fixed ? (l.color || [1, 1, 1]) : [lampC.r * (l.color ? l.color[0] : 1), lampC.g * (l.color ? l.color[1] : 1), lampC.b * (l.color ? l.color[2] : 1)];
      return { l, c };
    });
    for (let i = 0; i < P.length / 3; i++) {
      let er = 0, eg = 0, eb = 0;
      for (const { l, c } of on) {
        const dx = l.p[0] - P[i * 3], dy = l.p[1] - P[i * 3 + 1], dz = l.p[2] - P[i * 3 + 2], d2 = dx * dx + dy * dy + dz * dz, d = Math.sqrt(d2) || 1e-6;
        const cos = (N[i * 3] * dx + N[i * 3 + 1] * dy + N[i * 3 + 2] * dz) / d;
        if (cos > 0) { const f = (l.I * cos) / (1 + d2 / (l.R * l.R)); er += f * c[0]; eg += f * c[1]; eb += f * c[2]; }
      }
      const t = T ? [T[i * 3], T[i * 3 + 1], T[i * 3 + 2]] : [1, 1, 1];
      C.setXYZ(i, Math.min(cap, (amb.r + er * gain) * t[0]), Math.min(cap, (amb.g + eg * gain) * t[1]), Math.min(cap, (amb.b + eb * gain) * t[2]));
    }
    // Lenses lamp by lamp: lit when the lamp is on in this state, dark otherwise.
    const lens = new THREE.Color(key === "normal" ? PALETTE.lampWarm : key === "red_alert" ? PALETTE.alert : PALETTE.emergency).multiplyScalar(1.4);
    const dark = new THREE.Color(0x1a1d22);
    const lr = geo.userData.roles.lamp;
    if (lr) {
      const real = lamps.filter((l) => !l.fixed);
      for (let j = 0; j < real.length && (j + 1) * 6 <= lr[1]; j++) {
        const c = key !== "emergency" || real[j].emergency ? lens : dark;
        for (let v = 0; v < 6; v++) C.setXYZ(lr[0] + j * 6 + v, c.r, c.g, c.b);
      }
    }
    paintRole(geo, "status", new THREE.Color(s.status).multiplyScalar(1.4));
    const sr = geo.userData.roles.screen;
    if (sr) for (let i = sr[0]; i < sr[0] + sr[1]; i++) C.setXYZ(i, (T ? T[i * 3] : 1) * 1.25, (T ? T[i * 3 + 1] : 1) * 1.25, (T ? T[i * 3 + 2] : 1) * 1.25);
    C.needsUpdate = true;
  }

  /**
   * Split the triangles of the given roles until no edge is longer than maxEdge metres
   * (longest-edge bisection), so a vertex bake shows pools of light on a floor or a wall.
   * Works on buildCompartment's parts in place; returns them.
   */
  function subdivideParts(parts, maxEdge, roles) {
    const m2 = maxEdge * maxEdge;
    for (const r of roles) {
      const P = parts[r];
      if (!P) continue;
      const out = { position: [], normal: [], uvm: [] };
      if (P.vlayer) out.vlayer = [];
      for (let t = 0; t < P.position.length / 9; t++) {
        const stack = [[0, 1, 2].map((k) => ({ p: P.position.slice((t * 3 + k) * 3, (t * 3 + k) * 3 + 3), n: P.normal.slice((t * 3 + k) * 3, (t * 3 + k) * 3 + 3), u: P.uvm.slice((t * 3 + k) * 2, (t * 3 + k) * 2 + 2), l: P.vlayer ? P.vlayer[t * 3 + k] : null }))];
        while (stack.length) {
          const tri = stack.pop();
          const e = [0, 1, 2].map((k) => { const a = tri[k].p, b = tri[(k + 1) % 3].p; return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2 + (a[2] - b[2]) ** 2; });
          const k = e[0] >= e[1] && e[0] >= e[2] ? 0 : e[1] >= e[2] ? 1 : 2;
          if (e[k] <= m2) {
            for (const v of tri) { out.position.push(...v.p); out.normal.push(...v.n); out.uvm.push(...v.u); if (out.vlayer) out.vlayer.push(v.l); }
            continue;
          }
          const a = tri[k], b = tri[(k + 1) % 3], c = tri[(k + 2) % 3];
          const mid = { p: a.p.map((x, i) => (x + b.p[i]) / 2), n: a.n, u: a.u.map((x, i) => (x + b.u[i]) / 2), l: a.l };
          stack.push([a, mid, c], [mid, b, c]);
        }
      }
      parts[r] = Object.assign({}, P, out);
    }
    return parts;
  }

  /**
   * Platforms and stairs (bridge-stations section 11a): raised floors that are solid, like a
   * dais, not air. A platform { poly, top_m, edges[], rail_gaps_m? } stands on floorY; edges[i]
   * is the kind of the edge from poly[i] to poly[i + 1]: "wall" (against the room's wall:
   * nothing drawn), "riser" (a step face and a hazard nosing), "rail" (the same and a
   * railing), "step" (a riser where a stair meets it). Rails leave gaps at rail_gaps_m (x or z
   * intervals, along the edge's main axis) and wherever a stair's top edge lands. A stair
   * { top_m, foot_m, width_m } (the layout's stair convention) gets treads of
   * detailing.platforms.tread_rise_m with nosings. Adds roles platform, riser, nosing,
   * railing and stair to the builder.
   */
  function buildPlatforms(THREE, B, platforms, stairs, floorY, D) {
    D = D || shipData("detailing");
    const Q = D.platforms;
    stairs = stairs || [];
    for (const pf of platforms) {
      let poly = pf.poly.map((p) => p.slice()), edges = pf.edges.slice();
      if (signedArea(poly) < 0) { poly = poly.reverse(); edges = edges.slice(0, -1).reverse().concat(edges.slice(-1)); }
      const top = pf.top_m, h = top - floorY, n = poly.length;
      B.flat(THREE, "platform", poly, [], top, 1);
      for (let i = 0; i < n; i++) {
        const kind = edges[i];
        if (kind === "wall") continue;
        const a = poly[i], b = poly[(i + 1) % n], len = Math.hypot(b[0] - a[0], b[1] - a[1]);
        const t = [(b[0] - a[0]) / len, 0, (b[1] - a[1]) / len], on = outwardNormal(a, b), out = [on[0], 0, on[1]];
        B.quad("riser", [a[0], floorY, a[1]], [b[0], floorY, b[1]], [b[0], top, b[1]], [a[0], top, a[1]], out);
        // The nosing: a hazard strip on the top's edge, raised clear of the platform's face.
        const nm = Q.nosing_m, nr = Q.nosing_raise_m;
        const c = [(a[0] + b[0]) / 2 - on[0] * nm / 2, top + nr / 2, (a[1] + b[1]) / 2 - on[1] * nm / 2];
        B.box("nosing", c, t, [0, 1, 0], out, len / 2, nr / 2, nm / 2, ["+v", "+w", "+u", "-u"]);
        if (kind !== "rail") continue;
        // The railing, inset from the edge, with gaps.
        const along = Math.abs(t[0]) >= Math.abs(t[2]) ? 0 : 2;
        const proj = (u) => along === 0 ? a[0] + t[0] * u : a[1] + t[2] * u;
        const gaps = [];
        for (const g of pf.rail_gaps_m || []) {
          const u0 = along === 0 ? (g[0] - a[0]) / t[0] : (g[0] - a[1]) / t[2], u1 = along === 0 ? (g[1] - a[0]) / t[0] : (g[1] - a[1]) / t[2];
          gaps.push([Math.min(u0, u1), Math.max(u0, u1)]);
        }
        for (const st of stairs) {
          const tp = st.top_m;
          if (Math.abs((tp[0] - a[0]) * on[0] + (tp[2] - a[1]) * on[1]) > 0.15) continue;
          const u = (tp[0] - a[0]) * t[0] + (tp[2] - a[1]) * t[2];
          if (u > -st.width_m / 2 && u < len + st.width_m / 2) gaps.push([u - st.width_m / 2 - 0.1, u + st.width_m / 2 + 0.1]);
        }
        void proj;
        const inset = Q.rail_inset_m, rm = Q.rail_m / 2;
        const P3 = (u, y) => [a[0] + t[0] * u - on[0] * inset, y, a[1] + t[2] * u - on[1] * inset];
        for (const [u0, u1] of intervalMinus(Q.rail_end_m, len - Q.rail_end_m, gaps)) {
          if (u1 - u0 < 0.3) continue;
          for (const y of [top + Q.rail_height_m, top + Q.rail_mid_m]) {
            B.box("railing", P3((u0 + u1) / 2, y), t, [0, 1, 0], out, (u1 - u0) / 2, rm, rm, ["+v", "-v", "+w", "-w", "+u", "-u"]);
          }
          const k = Math.max(1, Math.round((u1 - u0) / Q.rail_post_spacing_m));
          for (let j = 0; j <= k; j++) {
            B.box("railing", P3(u0 + ((u1 - u0) * j) / k, top + Q.rail_height_m / 2), t, [0, 1, 0], out, rm, Q.rail_height_m / 2, rm, ["+u", "-u", "+w", "-w"]);
          }
        }
      }
    }
    for (const st of stairs) {
      const tp = st.top_m, ft = st.foot_m, rise = tp[1] - ft[1];
      const n = Math.max(1, Math.round(rise / Q.tread_rise_m)), r = rise / n;
      if (n < 2) continue;
      const dx = tp[0] - ft[0], dz = tp[2] - ft[2], run = Math.hypot(dx, dz), w = [dx / run, 0, dz / run], u = [w[2], 0, -w[0]];
      const depth = run / (n - 1);
      for (let k = 0; k < n - 1; k++) {
        const hh = (k + 1) * r, s0 = k * depth, cz = s0 + depth / 2;
        const c = [ft[0] + w[0] * cz, ft[1] + hh / 2, ft[2] + w[2] * cz];
        // Each tread runs from its own front edge back to the top, so the steps are solid.
        const back = run - s0;
        const cb = [ft[0] + w[0] * (s0 + back / 2), ft[1] + hh / 2, ft[2] + w[2] * (s0 + back / 2)];
        void c;
        B.box("stair", cb, u, [0, 1, 0], w, st.width_m / 2, hh / 2, back / 2, ["+v", "-w", "+u", "-u"]);
        const nm = Q.nosing_m;
        B.box("nosing", [ft[0] + w[0] * (s0 + nm / 2), ft[1] + hh + Q.nosing_raise_m / 2, ft[2] + w[2] * (s0 + nm / 2)], u, [0, 1, 0], w, st.width_m / 2, Q.nosing_raise_m / 2, nm / 2, ["+v", "-w", "+u", "-u"]);
      }
    }
    return B;
  }

  // ------------------------------------------------------------------- hull

  /**
   * Loft the hull's octagonal sections into one low-poly BufferGeometry
   * (outward normals, flat shaded). opts.inflate_m grows it, for a shell drawn
   * around the interior. With opts.mats it also carries color, surfUv, surfLayer and
   * surfGlow for a surfaceMaterial, in the "hull" material (opts.material names another).
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
    if (opts.mats) {
      const name = opts.material || "hull", li = opts.mats.layer[name], span = opts.mats.span[name];
      const n = pos.length / 3, uv = new Float32Array(n * 2);
      for (let i = 0; i < n; i += 3) {
        // One projection per face, from the face normal, so a face's texels do not shear.
        const a = pos.slice(i * 3, i * 3 + 3), b = pos.slice(i * 3 + 3, i * 3 + 6), c = pos.slice(i * 3 + 6, i * 3 + 9);
        const ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]], ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        const fn = [ab[1] * ac[2] - ab[2] * ac[1], ab[2] * ac[0] - ab[0] * ac[2], ab[0] * ac[1] - ab[1] * ac[0]];
        const ax = Math.abs(fn[0]), ay = Math.abs(fn[1]), az = Math.abs(fn[2]);
        for (let k = 0; k < 3; k++) {
          const p = pos.slice((i + k) * 3, (i + k) * 3 + 3);
          const q = ax >= ay && ax >= az ? [p[2], p[1]] : ay >= az ? [p[0], p[2]] : [p[0], p[1]];
          uv[(i + k) * 2] = q[0] / span; uv[(i + k) * 2 + 1] = q[1] / span;
        }
      }
      g.setAttribute("color", new THREE.BufferAttribute(new Float32Array(n * 3).fill(1), 3));
      g.setAttribute("surfUv", new THREE.BufferAttribute(uv, 2));
      g.setAttribute("surfLayer", new THREE.BufferAttribute(new Float32Array(n).fill(li), 1));
      g.setAttribute("surfGlow", new THREE.BufferAttribute(new Float32Array(n), 1));
    }
    return g;
  }

  /** Half-width of the hull at height y and station z (the octagon's x extent there), or 0 outside it. */
  function hullHalfWidth(L, y, z) {
    const secs = L.hull.sections.slice().sort((a, b) => a.z_m - b.z_m);
    if (z < secs[0].z_m || z > secs[secs.length - 1].z_m) return 0;
    let s = secs[0];
    for (let i = 0; i + 1 < secs.length; i++) {
      const a = secs[i], b = secs[i + 1];
      if (z >= a.z_m && z <= b.z_m) {
        const t = (z - a.z_m) / (b.z_m - a.z_m || 1);
        s = {};
        for (const k of ["half_beam_m", "top_m", "bottom_m", "chamfer_m"]) s[k] = a[k] + (b[k] - a[k]) * t;
        break;
      }
    }
    if (y > s.top_m || y < s.bottom_m) return 0;
    return Math.max(0, Math.min(s.half_beam_m, s.half_beam_m - s.chamfer_m + (s.top_m - y), s.half_beam_m - s.chamfer_m + (y - s.bottom_m)));
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
   * render-to-texture included). Set hud.textureBytes (loadMaterials' bytes) to
   * add the texture memory line.
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
    let frames = 0, acc = { t: 0, c: 0 }, last = performance.now(), shown = { t: 0, c: 0, fps: 0 };
    const bar = (v, max) => {
      const f = Math.min(1, v / max);
      const color = v > max ? "#ff5252" : v > 0.8 * max ? "#ffb300" : "#66bb6a";
      return `<div style="height:5px;background:#1c242d;border-radius:3px;margin:2px 0 5px"><div style="height:5px;width:${(f * 100).toFixed(1)}%;background:${color};border-radius:3px"></div></div>`;
    };
    const hud = {
      el, textureBytes: opts.textureBytes || 0,
      beginFrame(renderer) { renderer.info.autoReset = false; renderer.info.reset(); },
      endFrame(renderer) {
        acc.t += renderer.info.render.triangles; acc.c += renderer.info.render.calls; frames++;
        const now = performance.now();
        if (now - last > 500) {
          shown = { t: Math.round(acc.t / frames), c: Math.round(acc.c / frames), fps: (frames * 1000) / (now - last) };
          acc = { t: 0, c: 0 }; frames = 0; last = now;
          render();
        }
      },
      /** The latest averaged numbers, for screenshots and tests. */
      read() { return { triangles: shown.t, drawCalls: shown.c, textureMB: hud.textureBytes / 1048576 }; },
    };
    function render() {
      const b = PI_BUDGET, mb = hud.textureBytes / 1048576;
      el.innerHTML =
        `<div style="font-weight:700;margin-bottom:4px">${PI_BUDGET.board} budget (provisional)</div>` +
        `triangles ${shown.t.toLocaleString()} / ${b.triangles.toLocaleString()}${bar(shown.t, b.triangles)}` +
        `draw calls ${shown.c} / ${b.drawCalls}${bar(shown.c, b.drawCalls)}` +
        (mb ? `textures ${mb.toFixed(2)} / ${b.textureMB} MB${bar(mb, b.textureMB)}` : "") +
        `<div style="color:#8a98a8">desktop ${shown.fps.toFixed(0)} fps: not a Pi measurement</div>` +
        (opts.note ? `<div style="color:#8a98a8">${opts.note}</div>` : "");
    }
    return hud;
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
    version: 2,
    layout, shipData, PI_BUDGET, PALETTE, LIGHTING,
    byId, compartment, portalsOf, stationsIn, systemsIn, deckById,
    // plan geometry
    signedArea, outwardNormal, insidePoly, chord, insetPoly, polyCentroid,
    // compartments and portals
    measure, bounds, center, brushAt, floorAt, portalFrame, portalOut, portalPoint, wallsOf,
    // shells, detail, lamps, materials
    buildCompartment, roomShell, lampsFor, frameStations, loadMaterials, surfaceMaterial, geometryOf, finishOf, compartmentMesh, paintRole, bakeDirect,
    // wall panels (openspec/changes/wall-panels)
    loadPanels, panelsData, panelBands, panelBays, fnv1a, decodePng, panelLayerList, panelLayerName, walkwayPaths,
    Builder, subdivideParts, buildPlatforms,
    // hull, labels, chrome
    hullGeometry, hullHalfWidth, label, budgetHud, titleBlock, registerShots, markReady,
    rectMinusHoles, intervalMinus, worldUv,
  };
})();
