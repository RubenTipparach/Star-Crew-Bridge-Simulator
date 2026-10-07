/*
 * lightbake.js: a small CPU light baker for the Star Crew mockups.
 *
 * What it owns: turning a compartment's planar patches, its light fixtures and
 * its emissive surfaces into baked light, by the method that
 * openspec/changes/light-baking designs for the engine's baker (`sc-tools
 * bake`, planned, not built):
 *   - direct light from fixtures, with soft shadows from walls, props and
 *     closed doors (shadow rays to sample points on each lamp);
 *   - emissive surfaces (screens, light strips, the viewscreen) as area lights;
 *   - ambient occlusion on a small ambient term;
 *   - zero, one or two diffuse bounces, gathered once on a coarse irradiance
 *     cache (a uniform 0.5 m grid on every surface, many rays per point) and
 *     interpolated to vertices and texels: bounce is smooth, so this is
 *     quieter than gathering at every output sample, at about the same cost
 *     in a big room (after Ward's irradiance caching);
 *   - all three lighting states (normal, red alert, emergency power) from one
 *     set of rays, because light is linear in its sources;
 *   - storage as 8-bit gamma-encoded vertex colours (three sets), or as a
 *     lightmap atlas (three textures), for comparison;
 *   - adaptive subdivision, so vertex lighting can carry shadows and pools;
 *   - ambient cubes (six colours) at probe points, for things that move.
 *
 * Surfaces are patches: parallelograms, or convex polygons (a hull-following
 * floor, a mitred cove) held as a polygon inside their bounding rectangle, so
 * every face of a brush-built room subdivides and takes texels the same way.
 * patchesFromTriangles() turns ShipKit's generated triangles (shell and detail)
 * into patches. bakedSurfaceMaterial() draws the result the engine's way: the
 * three baked sets, blended by a uniform, multiply a ShipKit surface material's
 * Material Maker texel.
 *
 * Why it lives here: it is documentation tooling (CLAUDE.md section 4), not
 * engine code. The lighting mockup uses it so the owner sees what each option
 * looks like and what it costs, measured by a real bake rather than painted.
 * It mirrors the planned engine baker's method so the pictures are honest; it
 * is not a reference the engine must match bit for bit.
 *
 * Like shipkit.js it is a classic script (pages work from disk), copied into a
 * page by tools/mockups/inline.py between "INLINE lib:lightbake" markers. Edit
 * it here, never in a page. Functions that build three.js objects take THREE as
 * their first argument because the page owns the three.js import.
 *
 * Deterministic: no Math.random. Every sample pattern is an R2 quasi-random
 * sequence rotated by a hash of (bake seed, receiver key, purpose), so the same
 * inputs and seed give byte-identical output, and digest() proves it.
 *
 * Units: metres; candela (cd) for a point light's intensity on its beam axis;
 * candela per square metre (cd/m^2) for emissive luminance; lux (lx) for
 * irradiance. State colours are linear RGB multipliers. Stored light is
 * gamma-encoded (2.2) with 2x overbright headroom: byte 128 shows a surface at
 * its palette colour, under the reference irradiance settings.reference_lux.
 */
(function () {
  "use strict";

  /** The three lighting states, in colour-set order (CLAUDE.md section 9). */
  const STATES = ["normal", "red_alert", "emergency"];
  const NS = 3;
  const GAMMA = 2.2;
  const OVERBRIGHT = 2.0;   // stored light 1.0 (byte 255) shows 2x the palette colour
  const SHOULDER = 1.5;     // display multiplier above which highlights roll off instead of clipping

  /** Defaults for bake(). Every key has a unit; a present zero means zero. */
  const DEFAULTS = {
    seed: 1,
    reference_lux: 100,      // irradiance that shows a surface at exactly its palette colour
    shadow_samples: 12,      // per lamp per receiver; adaptive: 4 first, the rest only in penumbra
    emitter_sample_area_m2: 0.06, // one sample per this much emissive area, 4 to 16 per emitter
    gather_rays: 48,         // cosine-weighted rays per sample, where bounce is gathered per sample (probes)
    cache_gather_rays: 128,  // cosine-weighted rays per irradiance-cache vertex; outputs interpolate the cache
    bounce_from_cache: true, // false: gather bounce at every output sample (noisier, slower; for comparison)
    cache_filter: 1,         // passes of a 3 x 3 tent filter over each patch's cached bounce (0: none); bounce is smooth
    emitter_sample_spacing_m: 0.25, // at least one sample per this much of an emitter's longest edge (long strips)
    ao_rays: 24,             // cosine-weighted rays per receiver for ambient occlusion
    ao_radius_m: 0.8,
    ao: true,
    bounces: 1,              // 0, 1 or 2
    cache_spacing_m: 0.5,    // vertex spacing of the irradiance cache that bounce rays read
    bias_m: 0.004,           // ray origin offset along the normal (no shadow acne)
    inset_m: 0.01,           // receivers on a patch edge are sampled this far inside it
    dither: true,            // +-0.5 LSB seeded dither on the 8-bit output
    yield_ms: 30,            // async bakes yield to the page this often
  };

  // ------------------------------------------------------------ hashing

  function mix32(x) {
    x = x >>> 0;
    x ^= x >>> 16; x = Math.imul(x, 0x7feb352d);
    x ^= x >>> 15; x = Math.imul(x, 0x846ca68b);
    x ^= x >>> 16;
    return x >>> 0;
  }
  function hash3(a, b, c) { return mix32((a >>> 0) ^ mix32((b >>> 0) ^ mix32((c >>> 0) + 0x9e3779b9))); }
  function u01(h) { return (h >>> 0) / 4294967296; }
  const R2A = 0.7548776662466927, R2B = 0.5698402909980532; // Roberts' R2 sequence
  function frac(x) { return x - Math.floor(x); }

  // ------------------------------------------------------------ colour

  function srgbToLinear(c) { return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4); }
  function hexToSrgb(hex) { return [((hex >> 16) & 255) / 255, ((hex >> 8) & 255) / 255, (hex & 255) / 255]; }
  function hexToLinear(hex) { return hexToSrgb(hex).map(srgbToLinear); }

  /** Irradiance ratio x = E / reference_lux to a display multiplier in [0, OVERBRIGHT]. */
  function encodeLevel(x) {
    if (!(x > 0)) return 0;
    let d = Math.pow(x, 1 / GAMMA);
    if (d > SHOULDER) d = SHOULDER + (OVERBRIGHT - SHOULDER) * Math.tanh((d - SHOULDER) / (OVERBRIGHT - SHOULDER));
    return d;
  }
  /** Display multiplier to a byte, with an optional dither offset in [0, 1). */
  function toByte(d, dither) {
    const v = Math.floor((d / OVERBRIGHT) * 255 + dither);
    return v < 0 ? 0 : v > 255 ? 255 : v;
  }
  /** Display luminance of one state of a 9-float lux record, in 8-bit levels (for error tests). */
  function level8(E, s, refLux) {
    const y = (0.2126 * E[s * 3] + 0.7152 * E[s * 3 + 1] + 0.0722 * E[s * 3 + 2]) / refLux;
    return (encodeLevel(y) / OVERBRIGHT) * 255;
  }

  // ------------------------------------------------------------ vectors (small, allocation-light)

  const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
  const add = (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
  const scale = (a, s) => [a[0] * s, a[1] * s, a[2] * s];
  const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
  const len = (a) => Math.sqrt(dot(a, a));
  const norm = (a) => { const l = len(a) || 1; return [a[0] / l, a[1] / l, a[2] / l]; };

  /** Orthonormal basis around a unit normal (Duff et al. 2017, branchless). */
  function basis(nx, ny, nz, out) {
    const sgn = nz >= 0 ? 1 : -1;
    const a = -1 / (sgn + nz), b = nx * ny * a;
    out[0] = 1 + sgn * nx * nx * a; out[1] = sgn * b; out[2] = -sgn * nx;
    out[3] = b; out[4] = sgn + ny * ny * a; out[5] = -ny;
  }

  // ------------------------------------------------------------ patches

  /**
   * A patch is a planar parallelogram P(s, t) = o + s u + t v, s, t in [0, 1],
   * facing n = normalize(u x v); or a triangle (tri: true, s + t <= 1); or a
   * convex polygon inside the rectangle (polyM, below). Patches are what the
   * baker subdivides and what a lightmap gives texels to.
   *
   * opts: albedoHex (palette colour) or albedo (linear RGB) and display (sRGB),
   * cast (in the ray scene, default true), receive (in the outputs, default
   * true), cheap (casts shadows but neither sends nor gathers bounce: a small
   * prop face whose bounce is not worth its cache points), emissive ({ normal, red_alert, emergency } display colours as
   * [r, g, b] in 0..1, drawn unlit), tag, material (a name the page uses to
   * texture the patch), polyM (a convex polygon, [[x, y], ...] in metres along
   * u and v from o, any winding; u and v must then be perpendicular).
   */
  function patch(o, u, v, opts) {
    opts = opts || {};
    const c = cross(u, v), area = len(c);
    const hex = opts.albedoHex !== undefined ? opts.albedoHex : 0x808080;
    const p = {
      o: o.slice(), u: u.slice(), v: v.slice(), n: norm(c),
      area: opts.tri ? area / 2 : area, lenU: len(u), lenV: len(v), tri: !!opts.tri,
      display: opts.display || hexToSrgb(hex), albedo: opts.albedo || hexToLinear(hex),
      cast: opts.cast !== false && !opts.emissive, receive: opts.receive !== false && !opts.emissive,
      emissive: opts.emissive || null, tag: opts.tag || "", material: opts.material || null, poly: null,
      cheap: !!opts.cheap,
    };
    if (opts.polyM) setPolygon(p, opts.polyM);
    return p;
  }

  /**
   * Give a patch a convex polygon: poly ([s, t] corners, counter-clockwise about
   * n), polyM (the same in metres), edges (each edge's start and inward unit
   * normal, in metres) and insetMax (a limit on the sample inset for a slim
   * polygon). Its area becomes the polygon's.
   */
  function setPolygon(p, polyM) {
    let pts = polyM.map((q) => [q[0], q[1]]);
    // Drop repeated corners.
    pts = pts.filter((q, i) => { const r = pts[(i + 1) % pts.length]; return Math.abs(q[0] - r[0]) + Math.abs(q[1] - r[1]) > 1e-9; });
    if (signedArea2(pts) < 0) pts.reverse();
    const area = signedArea2(pts);
    let perim = 0;
    const edges = pts.map((a, i) => {
      const b = pts[(i + 1) % pts.length], dx = b[0] - a[0], dy = b[1] - a[1], l = Math.hypot(dx, dy) || 1;
      perim += l;
      return { ax: a[0], ay: a[1], nx: -dy / l, ny: dx / l };
    });
    p.polyM = pts; p.edges = edges; p.area = area;
    p.poly = pts.map((q) => [q[0] / (p.lenU || 1), q[1] / (p.lenV || 1)]);
    p.insetMax = perim > 0 ? (0.45 * 2 * area) / perim : 0;
  }
  function signedArea2(pts) {
    let s = 0;
    for (let i = 0; i < pts.length; i++) { const a = pts[i], b = pts[(i + 1) % pts.length]; s += a[0] * b[1] - b[0] * a[1]; }
    return s / 2;
  }
  /** True when (x, y), in metres, is inside a polygon patch (its boundary counts). */
  function insidePolygon(p, x, y) {
    for (const e of p.edges) if ((x - e.ax) * e.nx + (y - e.ay) * e.ny < -1e-7) return false;
    return true;
  }
  /** A convex polygon [[x, y], ...] clipped to a polygon patch (Sutherland-Hodgman), in metres; may be empty. */
  function clipToPolygon(p, subject) {
    let out = subject;
    for (const e of p.edges) {
      const inp = out; out = [];
      if (!inp.length) break;
      for (let i = 0; i < inp.length; i++) {
        const P = inp[i], Q = inp[(i + 1) % inp.length];
        const dp = (P[0] - e.ax) * e.nx + (P[1] - e.ay) * e.ny, dq = (Q[0] - e.ax) * e.nx + (Q[1] - e.ay) * e.ny;
        const pin = dp >= -1e-9, qin = dq >= -1e-9;
        if (pin) out.push(P);
        if (pin !== qin) { const t = dp / (dp - dq); out.push([P[0] + (Q[0] - P[0]) * t, P[1] + (Q[1] - P[1]) * t]); }
      }
      // Remove repeated corners a tangent cut can leave.
      out = out.filter((q, i) => { const r = out[(i + 1) % out.length]; return Math.abs(q[0] - r[0]) + Math.abs(q[1] - r[1]) > 1e-9; });
    }
    return out.length >= 3 && signedArea2(out) > 1e-10 ? out : [];
  }

  /**
   * A convex planar polygon (3D corners in order, either winding) facing n, as a
   * patch: a parallelogram when it is one, else a polygon patch whose rectangle
   * runs along its longest edge (so a hull-following floor's grid lines up with
   * its walls). opts as for patch().
   */
  function polygonPatch(pts, n, opts) {
    n = norm(n);
    if (pts.length === 4) {
      const s0 = add(pts[0], pts[2]), s1 = add(pts[1], pts[3]);
      if (len(sub(s0, s1)) < 1e-6) {
        let u = sub(pts[1], pts[0]), v = sub(pts[3], pts[0]);
        if (dot(cross(u, v), n) < 0) { const t = u; u = v; v = t; }
        return patch(pts[0], u, v, opts);
      }
    }
    let best = -1, ud = null;
    for (let i = 0; i < pts.length; i++) {
      const e = sub(pts[(i + 1) % pts.length], pts[i]), l = len(e);
      if (l > best + 1e-9) { best = l; ud = e; }
    }
    ud = norm(sub(ud, scale(n, dot(ud, n))));
    const vd = cross(n, ud); // u x v = n
    const q = pts.map((p) => [dot(sub(p, pts[0]), ud), dot(sub(p, pts[0]), vd)]);
    const mnU = Math.min(...q.map((a) => a[0])), mxU = Math.max(...q.map((a) => a[0]));
    const mnV = Math.min(...q.map((a) => a[1])), mxV = Math.max(...q.map((a) => a[1]));
    const o = add(pts[0], add(scale(ud, mnU), scale(vd, mnV)));
    const polyM = q.map((a) => [a[0] - mnU, a[1] - mnV]);
    return patch(o, scale(ud, mxU - mnU), scale(vd, mxV - mnV), Object.assign({}, opts, { polyM }));
  }

  /** A patch's corners in 3D, counter-clockwise about its normal. */
  function patchCorners(p) {
    const at = (s, t) => [p.o[0] + s * p.u[0] + t * p.v[0], p.o[1] + s * p.u[1] + t * p.v[1], p.o[2] + s * p.u[2] + t * p.v[2]];
    if (p.poly) return p.poly.map((q) => at(q[0], q[1]));
    if (p.tri) return [at(0, 0), at(1, 0), at(0, 1)];
    return [at(0, 0), at(1, 0), at(1, 1), at(0, 1)];
  }

  /**
   * Patches from a flat triangle list (ShipKit's buildCompartment parts: position
   * and normal, 9 numbers per triangle). Coplanar triangles whose union is one
   * convex polygon (a floor, a ceiling, a wall cell, a mitred cove) become one
   * patch; otherwise consecutive pairs that make a parallelogram (ShipKit's
   * quads) become one, and a lone triangle is a polygon patch of three corners.
   * The normal array decides which way a patch faces. opts as for patch().
   */
  function patchesFromTriangles(P, N, opts) {
    const groups = new Map(), order = [];
    for (let t = 0; t + 8 < P.length; t += 9) {
      const a = [P[t], P[t + 1], P[t + 2]], b = [P[t + 3], P[t + 4], P[t + 5]], c = [P[t + 6], P[t + 7], P[t + 8]];
      const cr = cross(sub(b, a), sub(c, a)), area = len(cr) / 2;
      if (area < 1e-10) continue;
      let n = norm(cr);
      if (N && dot(n, [N[t], N[t + 1], N[t + 2]]) < 0) n = scale(n, -1);
      const key = [n[0], n[1], n[2], dot(n, a)].map((x) => Math.round(x * 1000)).join(",");
      let g = groups.get(key);
      if (!g) { g = { n, tris: [], area: 0 }; groups.set(key, g); order.push(g); }
      g.tris.push([a, b, c]); g.area += area;
    }
    const out = [];
    const same = (p, q) => Math.abs(p[0] - q[0]) + Math.abs(p[1] - q[1]) + Math.abs(p[2] - q[2]) < 1e-6;
    for (const g of order) {
      // The convex hull of the group's corners, in the plane (Andrew's monotone chain).
      const e1 = norm(Math.abs(g.n[1]) < 0.9 ? cross([0, 1, 0], g.n) : cross([1, 0, 0], g.n)), e2 = cross(g.n, e1);
      const pts = [];
      for (const tr of g.tris) for (const p of tr) if (!pts.some((q) => same(p, q))) pts.push(p);
      const xy = pts.map((p, i) => ({ x: dot(p, e1), y: dot(p, e2), i })).sort((a, b) => a.x - b.x || a.y - b.y);
      const turn = (o, a, b) => (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
      const lower = [], upper = [];
      for (const p of xy) { while (lower.length >= 2 && turn(lower[lower.length - 2], lower[lower.length - 1], p) <= 1e-12) lower.pop(); lower.push(p); }
      for (let k = xy.length - 1; k >= 0; k--) { const p = xy[k]; while (upper.length >= 2 && turn(upper[upper.length - 2], upper[upper.length - 1], p) <= 1e-12) upper.pop(); upper.push(p); }
      const hull = lower.slice(0, -1).concat(upper.slice(0, -1));
      let ha = 0;
      for (let k = 0; k < hull.length; k++) { const a = hull[k], b = hull[(k + 1) % hull.length]; ha += a.x * b.y - b.x * a.y; }
      ha /= 2;
      if (hull.length >= 3 && Math.abs(ha - g.area) <= 1e-6 + 1e-5 * g.area) {
        out.push(polygonPatch(hull.map((h) => pts[h.i]), g.n, opts));
        continue;
      }
      // Not one convex piece: pair ShipKit's quads, keep lone triangles.
      for (let k = 0; k < g.tris.length; k++) {
        const A = g.tris[k], B = g.tris[k + 1];
        if (B) {
          const shared = A.filter((p) => B.some((q) => same(p, q)));
          if (shared.length === 2) {
            const X = A.find((p) => !shared.some((q) => same(p, q))), Y = B.find((p) => !shared.some((q) => same(p, q)));
            if (len(sub(add(X, Y), add(shared[0], shared[1]))) < 1e-6) {
              out.push(polygonPatch([X, shared[0], Y, shared[1]], g.n, opts));
              k++; continue;
            }
          }
        }
        out.push(polygonPatch(A, g.n, opts));
      }
    }
    return out;
  }

  /**
   * The faces of an oriented box: centre c, unit axes u, v, w, half sizes hu, hv,
   * hw. faces lists the faces to make ("+u", "-u", "+v", "-v", "+w", "-w"; all
   * when omitted), as ShipKit's Builder.box names them. opts as for patch().
   */
  function orientedBoxPatches(c, u, v, w, hu, hv, hw, faces, opts) {
    const ax = { u: [u, hu], v: [v, hv], w: [w, hw] }, out = [];
    for (const f of faces || ["+u", "-u", "+v", "-v", "+w", "-w"]) {
      const sgn = f[0] === "+" ? 1 : -1, k = f[1], others = ["u", "v", "w"].filter((x) => x !== k);
      const [a, ha] = ax[others[0]], [b, hb] = ax[others[1]], [n, hn] = ax[k];
      const out3 = scale(n, sgn);
      const fc = add(c, scale(out3, hn));
      const o = sub(sub(fc, scale(a, ha)), scale(b, hb));
      let U = scale(a, 2 * ha), V = scale(b, 2 * hb), O = o;
      if (dot(cross(U, V), out3) < 0) { const t = U; U = V; V = t; }
      if (len(cross(U, V)) > 1e-12) out.push(patch(O, U, V, opts));
    }
    return out;
  }

  /** The six faces of an axis-aligned box, facing out. opts.skip: ["-y", "+x", ...]. */
  function boxPatches(min, max, opts) {
    opts = opts || {};
    const skip = opts.skip || [];
    const [x0, y0, z0] = min, [x1, y1, z1] = max;
    const dx = x1 - x0, dy = y1 - y0, dz = z1 - z0;
    const out = [];
    const f = (key, o, u, v) => { if (skip.indexOf(key) < 0 && len(cross(u, v)) > 1e-9) out.push(patch(o, u, v, opts)); };
    f("+x", [x1, y0, z0], [0, dy, 0], [0, 0, dz]);
    f("-x", [x0, y0, z0], [0, 0, dz], [0, dy, 0]);
    f("+y", [x0, y1, z0], [0, 0, dz], [dx, 0, 0]);
    f("-y", [x0, y0, z0], [dx, 0, 0], [0, 0, dz]);
    f("+z", [x0, y0, z1], [dx, 0, 0], [0, dy, 0]);
    f("-z", [x0, y0, z0], [0, dy, 0], [dx, 0, 0]);
    return out;
  }

  /**
   * Recover patches from a triangle BufferGeometry (world space): consecutive
   * coplanar triangle pairs that share an edge and form a parallelogram become
   * one quad patch; anything else stays a triangle patch. The geometry's normal
   * attribute, when present, decides which way a patch faces. ShipKit.roomShell
   * output comes back as exactly its rectangles. opts as for patch().
   */
  function patchesFromGeometry(geometry, opts) {
    const P = geometry.getAttribute("position"), N = geometry.getAttribute("normal");
    const idx = geometry.index ? geometry.index.array : null;
    const count = idx ? idx.length : P.count;
    const vi = (k) => (idx ? idx[k] : k);
    const pos = (i) => [P.getX(i), P.getY(i), P.getZ(i)];
    const out = [];
    const same = (a, b) => Math.abs(a[0] - b[0]) + Math.abs(a[1] - b[1]) + Math.abs(a[2] - b[2]) < 1e-6;
    const facing = (tri, i) => {
      const fn = cross(sub(tri[1], tri[0]), sub(tri[2], tri[0]));
      if (!N) return fn;
      const nn = [N.getX(i), N.getY(i), N.getZ(i)];
      return dot(fn, nn) < 0 ? scale(nn, 1) : fn;
    };
    for (let k = 0; k + 2 < count; k += 3) {
      const a = [pos(vi(k)), pos(vi(k + 1)), pos(vi(k + 2))];
      const nA = facing(a, vi(k));
      let used = false;
      if (k + 5 < count) {
        const b = [pos(vi(k + 3)), pos(vi(k + 4)), pos(vi(k + 5))];
        const shared = a.filter((p) => b.some((q) => same(p, q)));
        if (shared.length === 2) {
          const X = a.find((p) => !shared.some((q) => same(p, q)));
          const D = b.find((p) => !shared.some((q) => same(p, q)));
          const [S1, S2] = shared;
          const coplanar = Math.abs(dot(norm(nA), norm(sub(D, X)))) < 1e-5;
          const parallelogram = same(add(X, D), add(S1, S2));
          if (coplanar && parallelogram) {
            let u = sub(S1, X), v = sub(S2, X);
            if (dot(cross(u, v), nA) < 0) { const t = u; u = v; v = t; }
            out.push(patch(X, u, v, opts));
            k += 3; used = true;
          }
        }
      }
      if (!used) {
        let u = sub(a[1], a[0]), v = sub(a[2], a[0]);
        if (dot(cross(u, v), nA) < 0) { const t = u; u = v; v = t; }
        out.push(patch(a[0], u, v, Object.assign({}, opts, { tri: true })));
      }
    }
    return out;
  }

  /**
   * A point on a patch, clamped inset_m inside its edges so edge receivers do not sit on a
   * neighbouring wall. On a polygon patch, a point outside the polygon (a grid point of its
   * rectangle beyond an angled wall) is moved inside it, so it reads the light at the edge.
   */
  function pointOn(p, s, t, inset, out) {
    if (p.poly) {
      let x = s * p.lenU, y = t * p.lenV;
      const d = Math.min(inset, p.insetMax);
      for (let it = 0; it < 2; it++) {
        for (const e of p.edges) {
          const k = (x - e.ax) * e.nx + (y - e.ay) * e.ny;
          if (k < d) { x += e.nx * (d - k); y += e.ny * (d - k); }
        }
      }
      s = x / (p.lenU || 1); t = y / (p.lenV || 1);
      out[0] = p.o[0] + s * p.u[0] + t * p.v[0];
      out[1] = p.o[1] + s * p.u[1] + t * p.v[1];
      out[2] = p.o[2] + s * p.u[2] + t * p.v[2];
      return out;
    }
    const es = Math.min(0.45, inset / Math.max(p.lenU, 1e-6)), et = Math.min(0.45, inset / Math.max(p.lenV, 1e-6));
    s = Math.min(1 - es, Math.max(es, s)); t = Math.min(1 - et, Math.max(et, t));
    if (p.tri && s + t > 1 - es) { const k = (1 - es) / (s + t); s *= k; t *= k; }
    out[0] = p.o[0] + s * p.u[0] + t * p.v[0];
    out[1] = p.o[1] + s * p.u[1] + t * p.v[1];
    out[2] = p.o[2] + s * p.u[2] + t * p.v[2];
    return out;
  }

  // ------------------------------------------------------------ meshes

  /**
   * A mesh here is plain typed arrays: positions, normals, display (sRGB
   * albedo 0..1), indices, plus per vertex the patch it lies on and its (s, t),
   * and per triangle its patch. Vertices are shared inside a patch (one normal,
   * one colour) and never across patches: flat shading.
   */
  function meshBuilder() { return { pos: [], st: [], pv: [], idx: [], tp: [] }; }
  function finishMesh(b, patches) {
    const nv = b.pv.length, nt = b.idx.length / 3;
    const m = {
      positions: new Float32Array(b.pos), normals: new Float32Array(nv * 3), display: new Float32Array(nv * 3),
      st: new Float32Array(b.st), patchOfVertex: new Int32Array(b.pv), patchOfTriangle: new Int32Array(b.tp),
      indices: nv < 65536 ? new Uint16Array(b.idx) : new Uint32Array(b.idx),
      vertexCount: nv, triangleCount: nt, uv2: b.uv2 ? new Float32Array(b.uv2) : null, keys: new Uint32Array(b.keys || []),
      grids: b.grids || null, // per patch: { base, nu, nv } of its uniform grid (quad patches from tessellate), else null
    };
    for (let i = 0; i < nv; i++) {
      const p = patches[m.patchOfVertex[i]];
      m.normals.set(p.n, i * 3); m.display.set(p.display, i * 3);
    }
    return m;
  }

  /**
   * Tessellate patches into a uniform grid of about spacing_m (Infinity: one
   * quad, triangle or polygon fan per patch, the deck's own density). Only
   * patches passing filter(p) are included; patch indices refer to the full
   * list. A polygon patch keeps its rectangle's whole vertex grid (so bounce
   * can be read bilinearly anywhere on it) and draws only the cells inside its
   * polygon, cutting the cells its edges cross.
   */
  function tessellate(patches, spacing, filter) {
    const b = meshBuilder(); b.keys = []; b.grids = new Array(patches.length).fill(null);
    patches.forEach((p, pi) => {
      if (filter && !filter(p)) return;
      const nu = isFinite(spacing) ? Math.max(1, Math.round(p.lenU / spacing)) : 1;
      const nv = isFinite(spacing) ? Math.max(1, Math.round(p.lenV / spacing)) : 1;
      const base = b.pv.length;
      if (p.poly && !isFinite(spacing)) {
        // The deck's own density: the polygon itself, fanned.
        p.poly.forEach((q, k) => {
          b.pos.push(p.o[0] + q[0] * p.u[0] + q[1] * p.v[0], p.o[1] + q[0] * p.u[1] + q[1] * p.v[1], p.o[2] + q[0] * p.u[2] + q[1] * p.v[2]);
          b.st.push(q[0], q[1]); b.pv.push(pi); b.keys.push(hash3(pi, 0x9000 + k, 0));
        });
        for (let k = 1; k + 1 < p.poly.length; k++) { b.idx.push(base, base + k, base + k + 1); b.tp.push(pi); }
        return;
      }
      if (!p.tri) b.grids[pi] = { base, nu, nv };
      if (p.poly) {
        for (let j = 0; j <= nv; j++) for (let i = 0; i <= nu; i++) {
          const s = i / nu, t = j / nv;
          b.pos.push(p.o[0] + s * p.u[0] + t * p.v[0], p.o[1] + s * p.u[1] + t * p.v[1], p.o[2] + s * p.u[2] + t * p.v[2]);
          b.st.push(s, t); b.pv.push(pi); b.keys.push(hash3(pi, i, j));
        }
        const cw = p.lenU / nu, ch = p.lenV / nv, extra = new Map();
        const vtx = (x, y) => {
          const gi = Math.round(x / cw), gj = Math.round(y / ch);
          if (Math.abs(x - gi * cw) < 1e-7 && Math.abs(y - gj * ch) < 1e-7 && gi >= 0 && gi <= nu && gj >= 0 && gj <= nv) return base + gj * (nu + 1) + gi;
          const qx = Math.round(x * 1e5), qy = Math.round(y * 1e5), k = qx * 1e7 + qy;
          let id = extra.get(k);
          if (id === undefined) {
            const s = x / p.lenU, t = y / p.lenV;
            id = b.pv.length; extra.set(k, id);
            b.pos.push(p.o[0] + s * p.u[0] + t * p.v[0], p.o[1] + s * p.u[1] + t * p.v[1], p.o[2] + s * p.u[2] + t * p.v[2]);
            b.st.push(s, t); b.pv.push(pi); b.keys.push(hash3(pi, 0x40000000 + (qx & 0xfffffff), qy));
          }
          return id;
        };
        for (let j = 0; j < nv; j++) for (let i = 0; i < nu; i++) {
          const x0 = i * cw, x1 = (i + 1) * cw, y0 = j * ch, y1 = (j + 1) * ch;
          const a = base + j * (nu + 1) + i, c = a + nu + 2;
          if (insidePolygon(p, x0, y0) && insidePolygon(p, x1, y0) && insidePolygon(p, x1, y1) && insidePolygon(p, x0, y1)) {
            b.idx.push(a, a + 1, c, a, c, c - 1); b.tp.push(pi, pi);
            continue;
          }
          const cut = clipToPolygon(p, [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]);
          const ids = cut.map((q) => vtx(q[0], q[1]));
          for (let k = 1; k + 1 < ids.length; k++) { b.idx.push(ids[0], ids[k], ids[k + 1]); b.tp.push(pi); }
        }
        return;
      }
      if (!p.tri) {
        for (let j = 0; j <= nv; j++) for (let i = 0; i <= nu; i++) {
          const s = i / nu, t = j / nv;
          b.pos.push(p.o[0] + s * p.u[0] + t * p.v[0], p.o[1] + s * p.u[1] + t * p.v[1], p.o[2] + s * p.u[2] + t * p.v[2]);
          b.st.push(s, t); b.pv.push(pi); b.keys.push(hash3(pi, i, j));
        }
        for (let j = 0; j < nv; j++) for (let i = 0; i < nu; i++) {
          const a = base + j * (nu + 1) + i, c = a + nu + 2;
          b.idx.push(a, a + 1, c, a, c, c - 1); b.tp.push(pi, pi);
        }
      } else {
        const n = Math.max(nu, nv);
        const row = []; let k = base;
        for (let j = 0; j <= n; j++) { row.push(k); for (let i = 0; i <= n - j; i++) {
          const s = i / n, t = j / n;
          b.pos.push(p.o[0] + s * p.u[0] + t * p.v[0], p.o[1] + s * p.u[1] + t * p.v[1], p.o[2] + s * p.u[2] + t * p.v[2]);
          b.st.push(s, t); b.pv.push(pi); b.keys.push(hash3(pi, i, j)); k++;
        } }
        for (let j = 0; j < n; j++) for (let i = 0; i < n - j; i++) {
          const a = row[j] + i, c = row[j + 1] + i;
          b.idx.push(a, a + 1, c); b.tp.push(pi);
          if (i < n - j - 1) { b.idx.push(a + 1, c + 1, c); b.tp.push(pi); }
        }
      }
    });
    return finishMesh(b, patches);
  }

  // ------------------------------------------------------------ BVH ray caster

  /**
   * A bounding volume hierarchy over a mesh's triangles (median split on the
   * widest centroid axis, leaves of up to 4). Occlusion is two-sided: the back
   * of a wall blocks light as well as its front. Closest-hit reports whether
   * the ray met a back face, which the gather treats as black (a leak guard).
   */
  function buildBVH(mesh) {
    const nt = mesh.triangleCount, P = mesh.positions, I = mesh.indices;
    const cen = new Float32Array(nt * 3), tb = new Float32Array(nt * 6);
    for (let t = 0; t < nt; t++) {
      let mnx = Infinity, mny = Infinity, mnz = Infinity, mxx = -Infinity, mxy = -Infinity, mxz = -Infinity;
      for (let k = 0; k < 3; k++) {
        const i = I[t * 3 + k] * 3, x = P[i], y = P[i + 1], z = P[i + 2];
        if (x < mnx) mnx = x; if (y < mny) mny = y; if (z < mnz) mnz = z;
        if (x > mxx) mxx = x; if (y > mxy) mxy = y; if (z > mxz) mxz = z;
      }
      tb.set([mnx, mny, mnz, mxx, mxy, mxz], t * 6);
      cen[t * 3] = (mnx + mxx) / 2; cen[t * 3 + 1] = (mny + mxy) / 2; cen[t * 3 + 2] = (mnz + mxz) / 2;
    }
    const order = new Int32Array(nt); for (let t = 0; t < nt; t++) order[t] = t;
    const maxNodes = Math.max(1, 2 * nt);
    const bb = new Float32Array(maxNodes * 6), nc = new Int32Array(maxNodes * 2);
    let nodes = 1;
    const stack = [[0, 0, nt]];
    while (stack.length) {
      const [node, start, end] = stack.pop();
      let mnx = Infinity, mny = Infinity, mnz = Infinity, mxx = -Infinity, mxy = -Infinity, mxz = -Infinity;
      let cnx = Infinity, cny = Infinity, cnz = Infinity, cxx = -Infinity, cxy = -Infinity, cxz = -Infinity;
      for (let k = start; k < end; k++) {
        const t = order[k], o = t * 6;
        if (tb[o] < mnx) mnx = tb[o]; if (tb[o + 1] < mny) mny = tb[o + 1]; if (tb[o + 2] < mnz) mnz = tb[o + 2];
        if (tb[o + 3] > mxx) mxx = tb[o + 3]; if (tb[o + 4] > mxy) mxy = tb[o + 4]; if (tb[o + 5] > mxz) mxz = tb[o + 5];
        const c = t * 3;
        if (cen[c] < cnx) cnx = cen[c]; if (cen[c + 1] < cny) cny = cen[c + 1]; if (cen[c + 2] < cnz) cnz = cen[c + 2];
        if (cen[c] > cxx) cxx = cen[c]; if (cen[c + 1] > cxy) cxy = cen[c + 1]; if (cen[c + 2] > cxz) cxz = cen[c + 2];
      }
      bb.set([mnx - 1e-5, mny - 1e-5, mnz - 1e-5, mxx + 1e-5, mxy + 1e-5, mxz + 1e-5], node * 6);
      const count = end - start;
      const ex = cxx - cnx, ey = cxy - cny, ez = cxz - cnz;
      if (count <= 4 || Math.max(ex, ey, ez) < 1e-6) { nc[node * 2] = start; nc[node * 2 + 1] = count; continue; }
      const axis = ex >= ey && ex >= ez ? 0 : ey >= ez ? 1 : 2;
      const seg = Array.from(order.subarray(start, end)).sort((a, b) => cen[a * 3 + axis] - cen[b * 3 + axis] || a - b);
      order.set(seg, start);
      const mid = (start + end) >> 1, left = nodes; nodes += 2;
      nc[node * 2] = left; nc[node * 2 + 1] = -(axis + 1);
      stack.push([left + 1, mid, end], [left, start, mid]);
    }
    // Triangles in BVH order as v0, e1, e2 for Moller-Trumbore.
    const tri = new Float32Array(nt * 9), triId = new Int32Array(nt);
    for (let k = 0; k < nt; k++) {
      const t = order[k]; triId[k] = t;
      const a = I[t * 3] * 3, b = I[t * 3 + 1] * 3, c = I[t * 3 + 2] * 3;
      tri[k * 9] = P[a]; tri[k * 9 + 1] = P[a + 1]; tri[k * 9 + 2] = P[a + 2];
      tri[k * 9 + 3] = P[b] - P[a]; tri[k * 9 + 4] = P[b + 1] - P[a + 1]; tri[k * 9 + 5] = P[b + 2] - P[a + 2];
      tri[k * 9 + 6] = P[c] - P[a]; tri[k * 9 + 7] = P[c + 1] - P[a + 1]; tri[k * 9 + 8] = P[c + 2] - P[a + 2];
    }
    return { bb: bb.subarray(0, nodes * 6), nc: nc.subarray(0, nodes * 2), tri, triId, nodes, triangles: nt, stack: new Int32Array(128) };
  }

  const EPS_T = 1e-4;

  /** True when anything lies on the ray within (EPS_T, tmax). */
  function occluded(B, ox, oy, oz, dx, dy, dz, tmax) {
    if (B.triangles === 0) return false;
    const ix = 1 / (dx || 1e-12), iy = 1 / (dy || 1e-12), iz = 1 / (dz || 1e-12);
    const bb = B.bb, nc = B.nc, T = B.tri, st = B.stack;
    let sp = 0; st[sp++] = 0;
    while (sp > 0) {
      const n = st[--sp], b = n * 6;
      let t0 = (bb[b] - ox) * ix, t1 = (bb[b + 3] - ox) * ix;
      let tn = t0 < t1 ? t0 : t1, tf = t0 < t1 ? t1 : t0;
      t0 = (bb[b + 1] - oy) * iy; t1 = (bb[b + 4] - oy) * iy;
      if (t0 > t1) { const q = t0; t0 = t1; t1 = q; }
      if (t0 > tn) tn = t0; if (t1 < tf) tf = t1;
      t0 = (bb[b + 2] - oz) * iz; t1 = (bb[b + 5] - oz) * iz;
      if (t0 > t1) { const q = t0; t0 = t1; t1 = q; }
      if (t0 > tn) tn = t0; if (t1 < tf) tf = t1;
      if (tf < tn || tf < 0 || tn > tmax) continue;
      const first = nc[n * 2], cnt = nc[n * 2 + 1];
      if (cnt > 0) {
        for (let k = first; k < first + cnt; k++) {
          const o = k * 9;
          const e1x = T[o + 3], e1y = T[o + 4], e1z = T[o + 5], e2x = T[o + 6], e2y = T[o + 7], e2z = T[o + 8];
          const px = dy * e2z - dz * e2y, py = dz * e2x - dx * e2z, pz = dx * e2y - dy * e2x;
          const det = e1x * px + e1y * py + e1z * pz;
          if (det > -1e-12 && det < 1e-12) continue;
          const inv = 1 / det;
          const tx = ox - T[o], ty = oy - T[o + 1], tz = oz - T[o + 2];
          const u = (tx * px + ty * py + tz * pz) * inv;
          if (u < 0 || u > 1) continue;
          const qx = ty * e1z - tz * e1y, qy = tz * e1x - tx * e1z, qz = tx * e1y - ty * e1x;
          const v = (dx * qx + dy * qy + dz * qz) * inv;
          if (v < 0 || u + v > 1) continue;
          const t = (e2x * qx + e2y * qy + e2z * qz) * inv;
          if (t > EPS_T && t < tmax) return true;
        }
      } else { st[sp++] = first; st[sp++] = first + 1; }
    }
    return false;
  }

  /** Closest hit. Returns false or fills hit = { tri (mesh triangle), t, u, v, back }. */
  function closest(B, ox, oy, oz, dx, dy, dz, tmax, hit) {
    if (B.triangles === 0) return false;
    const ix = 1 / (dx || 1e-12), iy = 1 / (dy || 1e-12), iz = 1 / (dz || 1e-12);
    const bb = B.bb, nc = B.nc, T = B.tri, st = B.stack;
    let sp = 0; st[sp++] = 0;
    let best = tmax, bk = -1, bu = 0, bv = 0, bdet = 0;
    while (sp > 0) {
      const n = st[--sp], b = n * 6;
      let t0 = (bb[b] - ox) * ix, t1 = (bb[b + 3] - ox) * ix;
      let tn = t0 < t1 ? t0 : t1, tf = t0 < t1 ? t1 : t0;
      t0 = (bb[b + 1] - oy) * iy; t1 = (bb[b + 4] - oy) * iy;
      if (t0 > t1) { const q = t0; t0 = t1; t1 = q; }
      if (t0 > tn) tn = t0; if (t1 < tf) tf = t1;
      t0 = (bb[b + 2] - oz) * iz; t1 = (bb[b + 5] - oz) * iz;
      if (t0 > t1) { const q = t0; t0 = t1; t1 = q; }
      if (t0 > tn) tn = t0; if (t1 < tf) tf = t1;
      if (tf < tn || tf < 0 || tn > best) continue;
      const first = nc[n * 2], cnt = nc[n * 2 + 1];
      if (cnt > 0) {
        for (let k = first; k < first + cnt; k++) {
          const o = k * 9;
          const e1x = T[o + 3], e1y = T[o + 4], e1z = T[o + 5], e2x = T[o + 6], e2y = T[o + 7], e2z = T[o + 8];
          const px = dy * e2z - dz * e2y, py = dz * e2x - dx * e2z, pz = dx * e2y - dy * e2x;
          const det = e1x * px + e1y * py + e1z * pz;
          if (det > -1e-12 && det < 1e-12) continue;
          const inv = 1 / det;
          const tx = ox - T[o], ty = oy - T[o + 1], tz = oz - T[o + 2];
          const u = (tx * px + ty * py + tz * pz) * inv;
          if (u < 0 || u > 1) continue;
          const qx = ty * e1z - tz * e1y, qy = tz * e1x - tx * e1z, qz = tx * e1y - ty * e1x;
          const v = (dx * qx + dy * qy + dz * qz) * inv;
          if (v < 0 || u + v > 1) continue;
          const t = (e2x * qx + e2y * qy + e2z * qz) * inv;
          if (t > EPS_T && t < best) { best = t; bk = k; bu = u; bv = v; bdet = det; }
        }
      } else {
        // Near child first: the split axis is stored as -(axis + 1).
        const axis = -cnt - 1, dpos = axis === 0 ? dx > 0 : axis === 1 ? dy > 0 : dz > 0;
        if (dpos) { st[sp++] = first + 1; st[sp++] = first; } else { st[sp++] = first; st[sp++] = first + 1; }
      }
    }
    if (bk < 0) return false;
    hit.tri = B.triId[bk]; hit.t = best; hit.u = bu; hit.v = bv; hit.back = bdet < 0;
    return true;
  }

  // ------------------------------------------------------------ scene and lights

  /**
   * Prepare a bake. def:
   *   patches   all patches (receivers, occluders and emissive), world space
   *   occluders axis-aligned boxes [{ min, max, albedoHex }] that only cast
   *   lights    point lights: { id, position_m, radius_m, intensity_cd,
   *             beam: { axis, exponent } or null, range_m, states }
   *   emitters  area lights: { id, quad: { o, u, v } (emits along u x v),
   *             luminance_cd_m2, beam_exponent, range_m, states }
   *   ambient_lux { normal: [r, g, b], ... } lux of fill light, times AO
   *   settings  see DEFAULTS
   * states: { normal: [r, g, b], red_alert: [...], emergency: [...] } linear
   * multipliers of the light's intensity; [0, 0, 0] is off in that state.
   */
  function scene(def) {
    const settings = Object.assign({}, DEFAULTS, def.settings || {});
    const patches = def.patches.slice();
    for (const b of def.occluders || []) for (const p of boxPatches(b.min, b.max, { albedoHex: b.albedoHex !== undefined ? b.albedoHex : 0x606060, receive: false, tag: "occluder" })) patches.push(p);
    const stateVec = (st) => { const v = new Float32Array(9); STATES.forEach((s, i) => { const c = (st && st[s]) || [0, 0, 0]; v[i * 3] = c[0]; v[i * 3 + 1] = c[1]; v[i * 3 + 2] = c[2]; }); return v; };
    const lights = (def.lights || []).map((l) => ({
      id: l.id, p: l.position_m.slice(), r: l.radius_m || 0, I: l.intensity_cd,
      axis: l.beam ? norm(l.beam.axis) : null, exp: l.beam ? l.beam.exponent : 0,
      range: l.range_m, st: stateVec(l.states),
    }));
    const emitters = (def.emitters || []).map((e) => {
      const q = e.quad, c = cross(q.u, q.v), area = len(c), n = norm(c);
      const longest = Math.max(len(q.u), len(q.v));
      const ns = Math.max(4, Math.min(64, Math.max(Math.ceil(area / settings.emitter_sample_area_m2), Math.ceil(longest / settings.emitter_sample_spacing_m))));
      const centre = add(q.o, add(scale(q.u, 0.5), scale(q.v, 0.5)));
      return { id: e.id, o: q.o.slice(), u: q.u.slice(), v: q.v.slice(), n, area, L: e.luminance_cd_m2, exp: e.beam_exponent || 1,
        range: e.range_m, st: stateVec(e.states), ns, centre, halfDiag: 0.5 * len(add(q.u, q.v)) };
    });
    const ambient = stateVec(def.ambient_lux);
    const t0 = performance.now();
    const cache = tessellate(patches, settings.cache_spacing_m, (p) => p.cast);
    const bvh = buildBVH(cache);
    const triAlbedo = new Float32Array(cache.triangleCount * 3);
    for (let t = 0; t < cache.triangleCount; t++) triAlbedo.set(patches[cache.patchOfTriangle[t]].albedo, t * 3);
    return {
      settings, patches, lights, emitters, ambient, cache, bvh, triAlbedo,
      source: null, // per cache vertex, 9 floats: the light that bounce rays pick up
      field: null,  // per cache vertex, 9 floats: the bounce irradiance there (bounce_from_cache)
      rays: { shadow: 0, gather: 0, ao: 0 }, buildMs: performance.now() - t0,
    };
  }

  // ------------------------------------------------------------ the receiver function

  const DIRECT = 1, AMBIENT = 2, BOUNCE = 4, ALL = 7;
  const _b = new Float64Array(6), _q = [0, 0, 0], _hit = { tri: 0, t: 0, u: 0, v: 0, back: false };

  /**
   * The one irradiance function. Everything baked (cache vertices, output
   * vertices, lightmap texels, probe faces) is a call to this with a point, a
   * normal and a seed key, so every output form agrees by construction.
   * Adds into out[9] (lux per state, RGB) and returns the AO factor.
   */
  function irradiance(S, px, py, pz, nx, ny, nz, key, what, out, gatherRays) {
    const set = S.settings, B = S.bvh, seed = set.seed >>> 0;
    const ox = px + nx * set.bias_m, oy = py + ny * set.bias_m, oz = pz + nz * set.bias_m;
    let ao = 1;
    if (what & DIRECT) {
      // Point lights with a radius: sample a disc facing the receiver (soft shadows).
      for (let li = 0; li < S.lights.length; li++) {
        const L = S.lights[li];
        const cx = L.p[0] - px, cy = L.p[1] - py, cz = L.p[2] - pz;
        const cd = Math.sqrt(cx * cx + cy * cy + cz * cz);
        if (cd - L.r > L.range) continue;
        if (cx * nx + cy * ny + cz * nz < -L.r) continue;
        const wx = cx / cd, wy = cy / cd, wz = cz / cd;
        basis(wx, wy, wz, _b);
        const h = hash3(seed, key, 0x1000 + li), r1 = u01(h), r2 = u01(mix32(h));
        const nsMax = L.r > 0 ? set.shadow_samples : 1;
        let sum = 0, cnt = 0, vis = 0, tested = 0;
        for (let k = 0; k < nsMax; k++) {
          if (k === 4 && tested > 0 && (vis === 0 || vis === tested)) break; // umbra or fully lit: done
          const a = frac(r1 + k * R2A), bb = frac(r2 + k * R2B);
          const rr = L.r * Math.sqrt(a), ph = 6.283185307179586 * bb;
          const qx = L.p[0] + rr * (Math.cos(ph) * _b[0] + Math.sin(ph) * _b[3]);
          const qy = L.p[1] + rr * (Math.cos(ph) * _b[1] + Math.sin(ph) * _b[4]);
          const qz = L.p[2] + rr * (Math.cos(ph) * _b[2] + Math.sin(ph) * _b[5]);
          let dx = qx - ox, dy = qy - oy, dz = qz - oz;
          const d2 = dx * dx + dy * dy + dz * dz, d = Math.sqrt(d2);
          dx /= d; dy /= d; dz /= d;
          cnt++;
          const cosr = dx * nx + dy * ny + dz * nz;
          if (cosr <= 0) continue;
          let g = 1;
          if (L.axis) { const ce = -(dx * L.axis[0] + dy * L.axis[1] + dz * L.axis[2]); if (ce <= 0) continue; g = Math.pow(ce, L.exp); }
          const x = d / L.range, win = x >= 1 ? 0 : (1 - x * x * x * x) * (1 - x * x * x * x);
          if (win <= 0) continue;
          tested++; S.rays.shadow++;
          if (occluded(B, ox, oy, oz, dx, dy, dz, d - 0.01)) continue;
          vis++;
          sum += (L.I * g * cosr * win) / Math.max(d2, 0.04);
        }
        if (sum > 0 && cnt > 0) {
          const e = sum / cnt;
          for (let c = 0; c < 9; c++) out[c] += e * L.st[c];
        }
      }
      // Area emitters: sample points on the quad, cosine at both ends.
      for (let ei = 0; ei < S.emitters.length; ei++) {
        const E = S.emitters[ei];
        const cx = E.centre[0] - px, cy = E.centre[1] - py, cz = E.centre[2] - pz;
        const cd = Math.sqrt(cx * cx + cy * cy + cz * cz);
        if (cd - E.halfDiag > E.range) continue;
        if (cx * nx + cy * ny + cz * nz < -E.halfDiag) continue;
        if (-(cx * E.n[0] + cy * E.n[1] + cz * E.n[2]) < -E.halfDiag) continue; // receiver behind the emitter
        const h = hash3(seed, key, 0x2000 + ei), r1 = u01(h), r2 = u01(mix32(h));
        const dA = E.area / E.ns;
        let sum = 0;
        for (let k = 0; k < E.ns; k++) {
          const a = frac(r1 + k * R2A), bb = frac(r2 + k * R2B);
          const qx = E.o[0] + a * E.u[0] + bb * E.v[0] + E.n[0] * 0.01;
          const qy = E.o[1] + a * E.u[1] + bb * E.v[1] + E.n[1] * 0.01;
          const qz = E.o[2] + a * E.u[2] + bb * E.v[2] + E.n[2] * 0.01;
          let dx = qx - ox, dy = qy - oy, dz = qz - oz;
          const d2 = dx * dx + dy * dy + dz * dz, d = Math.sqrt(d2);
          dx /= d; dy /= d; dz /= d;
          const cosr = dx * nx + dy * ny + dz * nz;
          if (cosr <= 0) continue;
          const ce = -(dx * E.n[0] + dy * E.n[1] + dz * E.n[2]);
          if (ce <= 0) continue;
          const x = d / E.range, win = x >= 1 ? 0 : (1 - x * x * x * x) * (1 - x * x * x * x);
          if (win <= 0) continue;
          S.rays.shadow++;
          if (occluded(B, ox, oy, oz, dx, dy, dz, d - 0.012)) continue;
          sum += (E.L * (E.exp === 1 ? ce : Math.pow(ce, E.exp)) * cosr * dA * win) / Math.max(d2, dA);
        }
        if (sum > 0) for (let c = 0; c < 9; c++) out[c] += sum * E.st[c];
      }
    }
    if ((what & AMBIENT) && set.ao) {
      basis(nx, ny, nz, _b);
      const h = hash3(seed, key, 0x3000), r1 = u01(h), r2 = u01(mix32(h));
      let open = 0;
      for (let k = 0; k < set.ao_rays; k++) {
        const a = frac(r1 + k * R2A), bb = frac(r2 + k * R2B);
        const rr = Math.sqrt(a), ph = 6.283185307179586 * bb, lz = Math.sqrt(1 - a);
        const lx = rr * Math.cos(ph), ly = rr * Math.sin(ph);
        S.rays.ao++;
        if (!occluded(B, ox, oy, oz, lx * _b[0] + ly * _b[3] + lz * nx, lx * _b[1] + ly * _b[4] + lz * ny, lx * _b[2] + ly * _b[5] + lz * nz, set.ao_radius_m)) open++;
      }
      ao = open / set.ao_rays;
    }
    if (what & AMBIENT) for (let c = 0; c < 9; c++) out[c] += S.ambient[c] * ao;
    if ((what & BOUNCE) && S.source && set.bounces > 0) {
      basis(nx, ny, nz, _b);
      const h = hash3(seed, key, 0x4000), r1 = u01(h), r2 = u01(mix32(h));
      const src = S.source, I = S.cache.indices, alb = S.triAlbedo, n = gatherRays || set.gather_rays;
      let a0 = 0, a1 = 0, a2 = 0, a3 = 0, a4 = 0, a5 = 0, a6 = 0, a7 = 0, a8 = 0;
      for (let k = 0; k < n; k++) {
        const a = frac(r1 + k * R2A), bb = frac(r2 + k * R2B);
        const rr = Math.sqrt(a), ph = 6.283185307179586 * bb, lz = Math.sqrt(1 - a);
        const lx = rr * Math.cos(ph), ly = rr * Math.sin(ph);
        S.rays.gather++;
        if (!closest(B, ox, oy, oz, lx * _b[0] + ly * _b[3] + lz * nx, lx * _b[1] + ly * _b[4] + lz * ny, lx * _b[2] + ly * _b[5] + lz * nz, 80, _hit)) continue;
        if (_hit.back) continue; // the back of a surface is inside a wall or a prop: no light there
        const t = _hit.tri, w0 = 1 - _hit.u - _hit.v, w1 = _hit.u, w2 = _hit.v;
        const i0 = I[t * 3] * 9, i1 = I[t * 3 + 1] * 9, i2 = I[t * 3 + 2] * 9;
        const ar = alb[t * 3], ag = alb[t * 3 + 1], ab = alb[t * 3 + 2];
        a0 += ar * (w0 * src[i0] + w1 * src[i1] + w2 * src[i2]);
        a1 += ag * (w0 * src[i0 + 1] + w1 * src[i1 + 1] + w2 * src[i2 + 1]);
        a2 += ab * (w0 * src[i0 + 2] + w1 * src[i1 + 2] + w2 * src[i2 + 2]);
        a3 += ar * (w0 * src[i0 + 3] + w1 * src[i1 + 3] + w2 * src[i2 + 3]);
        a4 += ag * (w0 * src[i0 + 4] + w1 * src[i1 + 4] + w2 * src[i2 + 4]);
        a5 += ab * (w0 * src[i0 + 5] + w1 * src[i1 + 5] + w2 * src[i2 + 5]);
        a6 += ar * (w0 * src[i0 + 6] + w1 * src[i1 + 6] + w2 * src[i2 + 6]);
        a7 += ag * (w0 * src[i0 + 7] + w1 * src[i1 + 7] + w2 * src[i2 + 7]);
        a8 += ab * (w0 * src[i0 + 8] + w1 * src[i1 + 8] + w2 * src[i2 + 8]);
      }
      // Cosine-weighted sampling: E = (1/N) sum of albedo x E(hit). The pi of the
      // cosine pdf cancels the 1/pi of a diffuse surface's radiance.
      out[0] += a0 / n; out[1] += a1 / n; out[2] += a2 / n; out[3] += a3 / n; out[4] += a4 / n;
      out[5] += a5 / n; out[6] += a6 / n; out[7] += a7 / n; out[8] += a8 / n;
    }
    return ao;
  }

  // ------------------------------------------------------------ async plumbing

  function yielder(S, onProgress) {
    let last = performance.now();
    return async (frac) => {
      const now = performance.now();
      if (now - last >= S.settings.yield_ms) {
        if (onProgress) onProgress(frac);
        await new Promise((r) => setTimeout(r, 0));
        last = performance.now();
      }
    };
  }

  /**
   * Light the irradiance cache: direct light at every cache vertex (what
   * bounce rays pick up), then, for two bounces, one gather pass over it.
   * With bounce_from_cache, one more gather (cache_gather_rays per vertex)
   * stores the bounce irradiance itself at every cache vertex, and outputs
   * interpolate it instead of gathering per sample. Call once per scene (and
   * per change of bounce count) before baking outputs.
   */
  async function prepare(S, onProgress) {
    const t0 = performance.now(), set = S.settings;
    const C = S.cache, nv = C.vertexCount, src = new Float32Array(nv * 9), tmp = new Float32Array(9), p = [0, 0, 0];
    const y = yielder(S, onProgress);
    const passes = 1 + (set.bounces > 1 ? 1 : 0) + (set.bounce_from_cache ? 1 : 0);
    let pass = 0;
    const gatherPass = async (from, rays, salt) => {
      const out = new Float32Array(nv * 9);
      S.source = from;
      for (let i = 0; i < nv; i++) {
        const P = S.patches[C.patchOfVertex[i]];
        if (P.cheap) continue;
        pointOn(P, C.st[i * 2], C.st[i * 2 + 1], set.inset_m, p);
        tmp.fill(0);
        irradiance(S, p[0], p[1], p[2], P.n[0], P.n[1], P.n[2], mix32(C.keys[i] ^ salt), BOUNCE, tmp, rays);
        out.set(tmp, i * 9);
        if ((i & 31) === 0) await y((pass + i / nv) / passes);
      }
      pass++;
      return out;
    };
    S.source = null; S.field = null;
    if (set.bounces > 0) {
      for (let i = 0; i < nv; i++) {
        const P = S.patches[C.patchOfVertex[i]];
        if (P.cheap) continue;   // a cheap patch casts shadows but neither sends nor gathers bounce (its source stays 0)
        pointOn(P, C.st[i * 2], C.st[i * 2 + 1], set.inset_m, p);
        tmp.fill(0);
        irradiance(S, p[0], p[1], p[2], P.n[0], P.n[1], P.n[2], C.keys[i], DIRECT, tmp);
        src.set(tmp, i * 9);
        if ((i & 63) === 0) await y(i / nv / passes);
      }
      pass++;
      let from = src;
      if (set.bounces > 1) {
        const g = await gatherPass(src, set.gather_rays, 0x5bd1e995);
        const second = new Float32Array(nv * 9);
        for (let k = 0; k < second.length; k++) second[k] = src[k] + g[k];
        from = second;
      }
      if (set.bounce_from_cache) {
        S.field = await gatherPass(from, set.cache_gather_rays, 0x27d4eb2f);
        for (let k = 0; k < set.cache_filter; k++) filterField(C, S.field);
      }
      S.source = from;
    }
    S.prepareMs = performance.now() - t0;
    return S;
  }

  /**
   * One pass of a 3 x 3 tent filter (weights 1 2 1) over each quad patch's
   * cache grid, never across patches, so bounce does not leak round a corner.
   * A few bright pools seen by a few gather rays make a splotchy field (the
   * classic irradiance-cache artefact); this trades it for softness that
   * bounce light has anyway.
   */
  function filterField(C, F) {
    for (const g of C.grids || []) {
      if (!g) continue;
      const W = g.nu + 1, H = g.nv + 1, src = F.slice(g.base * 9, (g.base + W * H) * 9);
      for (let j = 0; j < H; j++) for (let i = 0; i < W; i++) {
        const acc = new Float64Array(9); let wsum = 0;
        for (let dj = -1; dj <= 1; dj++) for (let di = -1; di <= 1; di++) {
          const x = i + di, y = j + dj;
          if (x < 0 || y < 0 || x >= W || y >= H) continue;
          const w = (2 - Math.abs(di)) * (2 - Math.abs(dj)), o = (y * W + x) * 9;
          for (let k = 0; k < 9; k++) acc[k] += w * src[o + k];
          wsum += w;
        }
        const o = (g.base + j * W + i) * 9;
        for (let k = 0; k < 9; k++) F[o + k] = acc[k] / wsum;
      }
    }
  }

  /**
   * Add the cached bounce at (s, t) on patch pi into out[9], bilinear between
   * the four cache vertices around it. Returns false when there is no cached
   * value there (no field, or a triangle patch), and the caller gathers.
   */
  function bounceAt(S, pi, s, t, out) {
    const g = S.field && S.cache.grids ? S.cache.grids[pi] : null;
    if (!g) return false;
    const x = Math.min(g.nu, Math.max(0, s * g.nu)), yy = Math.min(g.nv, Math.max(0, t * g.nv));
    const i = Math.min(g.nu - 1, Math.floor(x)), j = Math.min(g.nv - 1, Math.floor(yy)), fx = x - i, fy = yy - j;
    const a = (g.base + j * (g.nu + 1) + i) * 9, b = a + 9, c = a + (g.nu + 1) * 9, d = c + 9, F = S.field;
    const wa = (1 - fx) * (1 - fy), wb = fx * (1 - fy), wc = (1 - fx) * fy, wd = fx * fy;
    for (let k = 0; k < 9; k++) out[k] += wa * F[a + k] + wb * F[b + k] + wc * F[c + k] + wd * F[d + k];
    return true;
  }

  /** All light at a point on patch pi: direct and ambient here, bounce from the cache where it has one. Returns AO. */
  function receive(S, pi, s, t, pt, key, out) {
    const P = S.patches[pi];
    if (S.field && S.cache.grids && S.cache.grids[pi]) {
      const ao = irradiance(S, pt[0], pt[1], pt[2], P.n[0], P.n[1], P.n[2], key, DIRECT | AMBIENT, out);
      bounceAt(S, pi, s, t, out);
      return ao;
    }
    return irradiance(S, pt[0], pt[1], pt[2], P.n[0], P.n[1], P.n[2], key, ALL, out);
  }

  // ------------------------------------------------------------ encoding

  /** Encode 9-float lux records (one per sample) into three RGBA8 sets; alpha holds AO. */
  function encodeSets(S, E, ao, keys) {
    const n = ao.length, ref = S.settings.reference_lux, sets = {};
    STATES.forEach((name, s) => {
      const out = new Uint8Array(n * 4);
      for (let i = 0; i < n; i++) {
        for (let c = 0; c < 3; c++) {
          const d = S.settings.dither ? u01(hash3(S.settings.seed, keys[i], 0x6000 + s * 3 + c)) : 0.5;
          out[i * 4 + c] = toByte(encodeLevel(E[i * 9 + s * 3 + c] / ref), d);
        }
        out[i * 4 + 3] = toByte(ao[i] * OVERBRIGHT, 0.5);
      }
      sets[name] = out;
    });
    return sets;
  }

  // ------------------------------------------------------------ outputs: vertices

  /** Bake every vertex of a mesh (from tessellate). Returns { sets, stats }. */
  async function bakeVertices(S, mesh, onProgress) {
    const t0 = performance.now(), r0 = Object.assign({}, S.rays);
    const nv = mesh.vertexCount, E = new Float32Array(nv * 9), ao = new Float32Array(nv), tmp = new Float32Array(9), p = [0, 0, 0];
    const y = yielder(S, onProgress);
    for (let i = 0; i < nv; i++) {
      const pi = mesh.patchOfVertex[i], P = S.patches[pi], s = mesh.st[i * 2], t = mesh.st[i * 2 + 1];
      pointOn(P, s, t, S.settings.inset_m, p);
      tmp.fill(0);
      ao[i] = receive(S, pi, s, t, p, mesh.keys[i], tmp);
      E.set(tmp, i * 9);
      if ((i & 15) === 0) await y(i / nv);
    }
    return { sets: encodeSets(S, E, ao, mesh.keys), E, ao, stats: stats(S, t0, r0, nv, "vertices") };
  }

  function stats(S, t0, r0, samples, what) {
    const r = S.rays;
    const rays = (r.shadow - r0.shadow) + (r.gather - r0.gather) + (r.ao - r0.ao);
    const ms = performance.now() - t0;
    return { ms, samples, what, rays, shadowRays: r.shadow - r0.shadow, gatherRays: r.gather - r0.gather, aoRays: r.ao - r0.ao,
      raysPerSecond: ms > 0 ? (rays * 1000) / ms : 0 };
  }

  // ------------------------------------------------------------ outputs: adaptive subdivision

  /**
   * Adaptive subdivision for vertex lighting. Each quad patch starts as a grid
   * of about base_m; a cell splits into four (down to min_m) while the light
   * at its centre or edge midpoints differs from what its corners would
   * interpolate by more than max_error_levels (8-bit display levels, worst
   * lighting state), highest error first, until max_added_triangles is spent.
   * The quadtree is then balanced (neighbours differ by at most one level) and
   * cells with a finer neighbour are fanned from their centre, so the mesh has
   * no T-junctions inside a patch. Refinement looks at direct light and AO;
   * bounce, which is smooth, is added to the final vertices only.
   *
   * opts: base_m, min_m, max_error_levels, max_added_triangles.
   * Returns { mesh, sets, stats: { ..., baseTriangles, triangles, splits, capped, maxResidual } }.
   */
  async function bakeAdaptive(S, opts, onProgress) {
    const t0 = performance.now(), r0 = Object.assign({}, S.rays);
    const set = S.settings, ref = set.reference_lux, y = yielder(S, onProgress);
    const LMAX = 4, F = 1 << LMAX;
    const pts = [], cells = [];
    const heap = [];
    const push = (c) => { heap.push(c); let i = heap.length - 1; while (i > 0) { const pa = (i - 1) >> 1; if (heap[pa].err >= heap[i].err) break; [heap[pa], heap[i]] = [heap[i], heap[pa]]; i = pa; } };
    const pop = () => { const top = heap[0], last = heap.pop(); if (heap.length) { heap[0] = last; let i = 0; for (;;) { const l = 2 * i + 1, r = l + 1; let m = i; if (l < heap.length && heap[l].err > heap[m].err) m = l; if (r < heap.length && heap[r].err > heap[m].err) m = r; if (m === i) break; [heap[m], heap[i]] = [heap[i], heap[m]]; i = m; } } return top; };
    const tmp = new Float32Array(9), pp = [0, 0, 0];
    let evals = 0;
    // Per patch: a vertex cache keyed by fine-grid coordinates, and the cell tree.
    const P = S.patches.map((p, pi) => {
      if (!p.receive) return null;
      const nu = p.tri ? 1 : Math.max(1, Math.round(p.lenU / opts.base_m)), nv = p.tri ? 1 : Math.max(1, Math.round(p.lenV / opts.base_m));
      const cell = Math.min(p.lenU / nu, p.lenV / nv);
      const lmax = p.tri ? 0 : Math.max(0, Math.min(LMAX, Math.floor(Math.log2(cell / opts.min_m) + 1e-9)));
      return { pi, p, nu, nv, lmax, W: nu * F, H: nv * F, verts: new Map(), split: new Set(), cls: new Map(), extra: new Map() };
    });
    function evalVertex(R, s, t, key, base) {
      pointOn(R.p, s, t, set.inset_m, pp);
      tmp.fill(0);
      const ao = irradiance(S, pp[0], pp[1], pp[2], R.p.n[0], R.p.n[1], R.p.n[2], key, DIRECT | AMBIENT, tmp);
      evals++;
      return Object.assign(base, { s, t, key, E: Float32Array.from(tmp), ao, lv: [level8(tmp, 0, ref), level8(tmp, 1, ref), level8(tmp, 2, ref)], out: -1 });
    }
    function vertex(R, gx, gy) {
      const k = gx + gy * (R.W + 1);
      let v = R.verts.get(k);
      if (v) return v;
      v = evalVertex(R, gx / R.W, gy / R.H, hash3(R.pi, gx, gy), { gx, gy });
      R.verts.set(k, v);
      return v;
    }
    // A polygon patch: where a cell lies against its polygon ("in", "out" or "part"), and the
    // vertices its edges cut into cells (metres along u and v).
    function classify(R, L, i, j) {
      if (!R.p.poly) return "in";
      const id = L + "," + i + "," + j;
      let c = R.cls.get(id);
      if (c) return c;
      const k = F >> L, fw = R.p.lenU / R.W, fh = R.p.lenV / R.H;
      const x0 = i * k * fw, x1 = (i + 1) * k * fw, y0 = j * k * fh, y1 = (j + 1) * k * fh;
      if (insidePolygon(R.p, x0, y0) && insidePolygon(R.p, x1, y0) && insidePolygon(R.p, x1, y1) && insidePolygon(R.p, x0, y1)) c = "in";
      else c = clipToPolygon(R.p, [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]).length ? "part" : "out";
      R.cls.set(id, c);
      return c;
    }
    function pointVertex(R, x, y) {
      const fw = R.p.lenU / R.W, fh = R.p.lenV / R.H, gx = Math.round(x / fw), gy = Math.round(y / fh);
      if (Math.abs(x - gx * fw) < 1e-6 && Math.abs(y - gy * fh) < 1e-6 && gx >= 0 && gx <= R.W && gy >= 0 && gy <= R.H) return vertex(R, gx, gy);
      const qx = Math.round(x * 1e5), qy = Math.round(y * 1e5), k = qx * 1e7 + qy;
      let v = R.extra.get(k);
      if (!v) { v = evalVertex(R, x / R.p.lenU, y / R.p.lenV, hash3(R.pi, 0x40000000 + (qx & 0xfffffff), qy), {}); R.extra.set(k, v); }
      return v;
    }
    function cellError(R, L, i, j) {
      if (L >= R.lmax) return 0;
      if (classify(R, L, i, j) === "out") return 0;
      const k = F >> L, x0 = i * k, y0 = j * k, x1 = x0 + k, y1 = y0 + k, xm = x0 + k / 2, ym = y0 + k / 2;
      const c00 = vertex(R, x0, y0), c10 = vertex(R, x1, y0), c01 = vertex(R, x0, y1), c11 = vertex(R, x1, y1);
      const tests = [[vertex(R, xm, ym), [c00, c10, c01, c11]], [vertex(R, xm, y0), [c00, c10]], [vertex(R, xm, y1), [c01, c11]],
        [vertex(R, x0, ym), [c00, c01]], [vertex(R, x1, ym), [c10, c11]]];
      let err = 0;
      for (const [m, cs] of tests) for (let s = 0; s < NS; s++) {
        let avg = 0; for (const c of cs) avg += c.lv[s]; avg /= cs.length;
        const e = Math.abs(m.lv[s] - avg); if (e > err) err = e;
      }
      return err;
    }
    // Base grids.
    let leaves = 0;
    for (const R of P) {
      if (!R) continue;
      for (let j = 0; j < R.nv; j++) for (let i = 0; i < R.nu; i++) {
        if (classify(R, 0, i, j) === "out") continue;
        push({ R, L: 0, i, j, err: cellError(R, 0, i, j) }); leaves++;
      }
      await y(0.1);
    }
    const baseTriangles = 2 * leaves;
    let splits = 0, capped = false, maxResidual = 0;
    const cap = opts.max_added_triangles;
    while (heap.length) {
      const c = pop();
      if (c.err <= opts.max_error_levels) { maxResidual = Math.max(maxResidual, c.err); break; }
      if (6 * (splits + 1) > cap) { capped = true; maxResidual = Math.max(maxResidual, c.err); break; }
      c.R.split.add(c.L + "," + c.i + "," + c.j);
      splits++;
      for (let dj = 0; dj < 2; dj++) for (let di = 0; di < 2; di++) {
        const L = c.L + 1, i = c.i * 2 + di, j = c.j * 2 + dj;
        push({ R: c.R, L, i, j, err: cellError(c.R, L, i, j) });
      }
      if ((splits & 7) === 0) await y(0.1 + 0.6 * Math.min(1, (6 * splits) / Math.max(1, cap)));
    }
    if (heap.length && !capped) for (const c of heap) maxResidual = Math.max(maxResidual, c.err);
    // Balance: no leaf may touch a neighbour split two levels finer.
    const isSplit = (R, L, i, j) => R.split.has(L + "," + i + "," + j);
    let changed = true, forced = 0;
    while (changed) {
      changed = false;
      for (const R of P) {
        if (!R) continue;
        const leafList = [];
        collectLeaves(R, leafList);
        for (const [L, i, j] of leafList) {
          const nb = [[i + 1, j, (a) => [[2 * (i + 1), 2 * j + a]]], [i - 1, j, (a) => [[2 * (i - 1) + 1, 2 * j + a]]],
            [i, j + 1, (a) => [[2 * i + a, 2 * (j + 1)]]], [i, j - 1, (a) => [[2 * i + a, 2 * (j - 1) + 1]]]];
          let must = false;
          for (const [ni, nj, kids] of nb) {
            if (!isSplit(R, L, ni, nj)) continue;
            for (let a = 0; a < 2 && !must; a++) { const [[ki, kj]] = kids(a); if (isSplit(R, L + 1, ki, kj)) must = true; }
            if (must) break;
          }
          if (must && L < LMAX) { R.split.add(L + "," + i + "," + j); forced++; changed = true; }
        }
      }
    }
    function collectLeaves(R, list) {
      const walk = (L, i, j) => { if (isSplit(R, L, i, j)) { for (let dj = 0; dj < 2; dj++) for (let di = 0; di < 2; di++) walk(L + 1, 2 * i + di, 2 * j + dj); } else list.push([L, i, j]); };
      for (let j = 0; j < R.nv; j++) for (let i = 0; i < R.nu; i++) walk(0, i, j);
    }
    // Triangulate.
    const b = meshBuilder(); b.keys = [];
    const outVerts = [];
    const vid = (R, v) => {
      if (v.out < 0) { v.out = b.pv.length; const p = R.p; b.pos.push(p.o[0] + v.s * p.u[0] + v.t * p.v[0], p.o[1] + v.s * p.u[1] + v.t * p.v[1], p.o[2] + v.s * p.u[2] + v.t * p.v[2]); b.st.push(v.s, v.t); b.pv.push(R.pi); b.keys.push(v.key); outVerts.push(v); }
      return v.out;
    };
    for (const R of P) {
      if (!R) continue;
      if (R.p.tri) {
        const a = vertex(R, 0, 0), c = vertex(R, R.W, 0), d = vertex(R, 0, R.H);
        b.idx.push(vid(R, a), vid(R, c), vid(R, d)); b.tp.push(R.pi); continue;
      }
      const list = []; collectLeaves(R, list);
      // A triangle of vertex objects into the mesh; on a polygon patch's edge cell, cut to the polygon first.
      const emit = (cls, a, c, d) => {
        if (cls === "in") { b.idx.push(vid(R, a), vid(R, c), vid(R, d)); b.tp.push(R.pi); return; }
        const cut = clipToPolygon(R.p, [a, c, d].map((v) => [v.s * R.p.lenU, v.t * R.p.lenV]));
        const vs = cut.map((q) => pointVertex(R, q[0], q[1]));
        for (let q = 1; q + 1 < vs.length; q++) {
          if (vs[0] === vs[q] || vs[q] === vs[q + 1] || vs[0] === vs[q + 1]) continue;
          b.idx.push(vid(R, vs[0]), vid(R, vs[q]), vid(R, vs[q + 1])); b.tp.push(R.pi);
        }
      };
      for (const [L, i, j] of list) {
        const cls = classify(R, L, i, j);
        if (cls === "out") continue;
        const k = F >> L, x0 = i * k, y0 = j * k, x1 = x0 + k, y1 = y0 + k, h = k / 2;
        const hang = [isSplit(R, L, i, j - 1), isSplit(R, L, i + 1, j), isSplit(R, L, i, j + 1), isSplit(R, L, i - 1, j)];
        const c00 = vertex(R, x0, y0), c10 = vertex(R, x1, y0), c11 = vertex(R, x1, y1), c01 = vertex(R, x0, y1);
        if (!hang.some(Boolean)) {
          // Split along the diagonal whose ends agree best (fewer zig-zags in gradients).
          const d1 = Math.abs(c00.lv[0] - c11.lv[0]), d2 = Math.abs(c10.lv[0] - c01.lv[0]);
          if (d1 <= d2) { emit(cls, c00, c10, c11); emit(cls, c00, c11, c01); } else { emit(cls, c00, c10, c01); emit(cls, c10, c11, c01); }
        } else {
          const ring = [c00]; if (hang[0]) ring.push(vertex(R, x0 + h, y0));
          ring.push(c10); if (hang[1]) ring.push(vertex(R, x1, y0 + h));
          ring.push(c11); if (hang[2]) ring.push(vertex(R, x0 + h, y1));
          ring.push(c01); if (hang[3]) ring.push(vertex(R, x0, y0 + h));
          const cc = vertex(R, x0 + h, y0 + h);
          for (let q = 0; q < ring.length; q++) emit(cls, cc, ring[q], ring[(q + 1) % ring.length]);
        }
      }
      await y(0.75);
    }
    const mesh = finishMesh(b, S.patches);
    // Final pass: add the bounce to the vertices that made it into the mesh.
    const E = new Float32Array(mesh.vertexCount * 9), ao = new Float32Array(mesh.vertexCount);
    for (let n = 0; n < outVerts.length; n++) {
      const v = outVerts[n], R = P[mesh.patchOfVertex[n]];
      E.set(v.E, n * 9); ao[n] = v.ao;
      if (S.source && set.bounces > 0) {
        tmp.fill(0);
        if (!bounceAt(S, R.pi, v.s, v.t, tmp)) {
          pointOn(R.p, v.s, v.t, set.inset_m, pp);
          irradiance(S, pp[0], pp[1], pp[2], R.p.n[0], R.p.n[1], R.p.n[2], v.key, BOUNCE, tmp);
        }
        for (let c = 0; c < 9; c++) E[n * 9 + c] += tmp[c];
      }
      if ((n & 15) === 0) await y(0.8 + 0.2 * n / outVerts.length);
    }
    const st = stats(S, t0, r0, evals, "adaptive");
    Object.assign(st, { baseTriangles, triangles: mesh.triangleCount, splits, forced, capped, maxResidual, evaluated: evals });
    return { mesh, sets: encodeSets(S, E, ao, mesh.keys), E, ao, stats: st };
  }

  // ------------------------------------------------------------ outputs: lightmap atlas

  /**
   * Bake a lightmap: every receiving patch gets a block of texels of at most
   * texel_m on a side plus a one-texel gutter (copied from the edge, so
   * bilinear filtering never reads a neighbour's light), shelf-packed into a
   * power-of-two-wide atlas. The mesh is the deck density (one quad per patch)
   * with a second UV set. Returns { mesh, atlas: { width, height, sets }, stats }.
   */
  async function bakeLightmap(S, opts, onProgress) {
    const t0 = performance.now(), r0 = Object.assign({}, S.rays);
    const texel = opts.texel_m, set = S.settings, y = yielder(S, onProgress);
    const rects = [];
    S.patches.forEach((p, pi) => {
      if (!p.receive) return;
      const nu = Math.max(1, Math.ceil(p.lenU / texel - 1e-6)), nv = Math.max(1, Math.ceil(p.lenV / texel - 1e-6));
      rects.push({ pi, p, nu, nv, w: nu + 2, h: nv + 2, x: 0, y: 0 });
    });
    let area = 0, widest = 0; for (const r of rects) { area += r.w * r.h; widest = Math.max(widest, r.w); }
    let W = 64; while (W * W < area * 1.25 || W < widest) W *= 2;
    const order = rects.slice().sort((a, b) => b.h - a.h || b.w - a.w || a.pi - b.pi);
    let x = 0, yy = 0, shelf = 0;
    for (const r of order) {
      if (x + r.w > W) { x = 0; yy += shelf; shelf = 0; }
      r.x = x; r.y = yy; x += r.w; shelf = Math.max(shelf, r.h);
    }
    const H = yy + shelf;
    const texels = rects.reduce((n, r) => n + r.nu * r.nv, 0);
    const E = new Float32Array(W * H * 9), AO = new Float32Array(W * H), keys = new Uint32Array(W * H);
    const tmp = new Float32Array(9), pp = [0, 0, 0];
    let done = 0;
    for (const r of rects) {
      const p = r.p;
      for (let j = 0; j < r.nv; j++) for (let i = 0; i < r.nu; i++) {
        const s = (i + 0.5) / r.nu, t = (j + 0.5) / r.nv;
        pointOn(p, s, t, set.inset_m, pp);
        tmp.fill(0);
        const key = hash3(r.pi, 0x10000 + i, j);
        const a = receive(S, r.pi, s, t, pp, key, tmp);
        const at = (r.y + 1 + j) * W + (r.x + 1 + i);
        E.set(tmp, at * 9); AO[at] = a; keys[at] = key;
        done++;
        if ((done & 15) === 0) await y(done / texels);
      }
      // Gutter: copy the nearest inner texel (corners included).
      for (let j = -1; j <= r.nv; j++) for (let i = -1; i <= r.nu; i++) {
        if (i >= 0 && i < r.nu && j >= 0 && j < r.nv) continue;
        const ci = Math.min(r.nu - 1, Math.max(0, i)), cj = Math.min(r.nv - 1, Math.max(0, j));
        const from = (r.y + 1 + cj) * W + (r.x + 1 + ci), to = (r.y + 1 + j) * W + (r.x + 1 + i);
        for (let c = 0; c < 9; c++) E[to * 9 + c] = E[from * 9 + c];
        AO[to] = AO[from]; keys[to] = keys[from];
      }
    }
    // Mesh: one quad per receiving patch, second UV set into its block.
    const b = meshBuilder(); b.keys = []; b.uv2 = [];
    for (const r of rects) {
      const p = r.p, base = b.pv.length;
      // A polygon patch draws its polygon (fanned) over its rectangle's block; texels outside
      // the polygon read the light at its edge (pointOn), so they serve as its gutter.
      const corners = p.poly ? p.poly : p.tri ? [[0, 0], [1, 0], [0, 1]] : [[0, 0], [1, 0], [1, 1], [0, 1]];
      corners.forEach(([s, t], k) => {
        b.pos.push(p.o[0] + s * p.u[0] + t * p.v[0], p.o[1] + s * p.u[1] + t * p.v[1], p.o[2] + s * p.u[2] + t * p.v[2]);
        b.st.push(s, t); b.pv.push(r.pi); b.keys.push(p.poly ? hash3(r.pi, 0x9000 + k, 7) : hash3(r.pi, s * 7, t * 7));
        b.uv2.push((r.x + 1 + s * r.nu) / W, (r.y + 1 + t * r.nv) / H);
      });
      for (let k = 1; k + 1 < corners.length; k++) { b.idx.push(base, base + k, base + k + 1); b.tp.push(r.pi); }
    }
    const mesh = finishMesh(b, S.patches);
    const st = stats(S, t0, r0, texels, "lightmap");
    const bytesRGBA8 = W * H * 4 * NS;
    Object.assign(st, { texels, width: W, height: H, bytes: bytesRGBA8, fill: texels / (W * H) });
    return { mesh, atlas: { width: W, height: H, sets: encodeSets(S, E, AO, keys), texel_m: texel }, stats: st };
  }

  // ------------------------------------------------------------ probes

  const AXES = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]];

  /**
   * An ambient cube at a point (Half-Life 2's six-colour probe): the
   * irradiance a surface facing +X, -X, +Y, -Y, +Z and -Z would receive there,
   * from the same irradiance function as the walls. Returns Float32Array(54):
   * axis-major, then state, then RGB, in lux.
   */
  function probeCube(S, point, key) {
    const out = new Float32Array(54), tmp = new Float32Array(9);
    AXES.forEach((a, i) => {
      tmp.fill(0);
      irradiance(S, point[0] - a[0] * S.settings.bias_m, point[1] - a[1] * S.settings.bias_m, point[2] - a[2] * S.settings.bias_m,
        a[0], a[1], a[2], hash3(key >>> 0, 0x7000, i), ALL, tmp);
      out.set(tmp, i * 9);
    });
    return out;
  }

  /** Encode an ambient cube for one state into six display colours (0..OVERBRIGHT). */
  function cubeDisplay(S, cube, state) {
    const s = STATES.indexOf(state), ref = S.settings.reference_lux, out = [];
    for (let i = 0; i < 6; i++) out.push([0, 1, 2].map((c) => encodeLevel(cube[i * 9 + s * 3 + c] / ref)));
    return out;
  }

  // ------------------------------------------------------------ digest

  /** FNV-1a over byte arrays, as 8 hex digits: two bakes with one seed must match. */
  function digest(arrays) {
    let h = 0x811c9dc5;
    for (const a of arrays) for (let i = 0; i < a.length; i++) { h ^= a[i]; h = Math.imul(h, 0x01000193); }
    return (h >>> 0).toString(16).padStart(8, "0");
  }

  // ------------------------------------------------------------ three.js adapters

  /** BufferGeometry with position, normal, albedo (sRGB) and, when given, colN/colR/colE (RGBA8, normalized) and lmuv. */
  function toGeometry(THREE, mesh, sets) {
    const g = new THREE.BufferGeometry();
    g.setAttribute("position", new THREE.BufferAttribute(mesh.positions, 3));
    g.setAttribute("normal", new THREE.BufferAttribute(mesh.normals, 3));
    g.setAttribute("albedo", new THREE.BufferAttribute(mesh.display, 3));
    const lin = new Float32Array(mesh.display.length);
    for (let i = 0; i < lin.length; i++) lin[i] = srgbToLinear(mesh.display[i]);
    g.setAttribute("color", new THREE.BufferAttribute(lin, 3)); // for three's own materials (linear)
    if (sets) {
      g.setAttribute("colN", new THREE.BufferAttribute(sets.normal, 4, true));
      g.setAttribute("colR", new THREE.BufferAttribute(sets.red_alert, 4, true));
      g.setAttribute("colE", new THREE.BufferAttribute(sets.emergency, 4, true));
    }
    if (mesh.uv2) g.setAttribute("lmuv", new THREE.BufferAttribute(mesh.uv2, 2));
    g.setIndex(new THREE.BufferAttribute(mesh.indices, 1));
    return g;
  }

  /**
   * The deck shader the engine plans (engine-stack section 7, program 1),
   * reduced to what a mockup needs: three baked colour sets blended by a
   * per-compartment weight uniform, times the palette albedo. One texture-free
   * path; the output is display sRGB already (light is stored gamma-encoded).
   */
  function vertexMaterial(THREE) {
    return new THREE.ShaderMaterial({
      uniforms: { stateW: { value: new THREE.Vector3(1, 0, 0) }, aoOnly: { value: 0 } },
      vertexShader: `
        attribute vec3 albedo; attribute vec4 colN; attribute vec4 colR; attribute vec4 colE;
        uniform vec3 stateW; uniform float aoOnly; varying vec3 vCol;
        void main() {
          vec4 l = colN * stateW.x + colR * stateW.y + colE * stateW.z;
          vCol = aoOnly > 0.5 ? vec3(l.a * ${OVERBRIGHT.toFixed(1)} * 0.5) : albedo * l.rgb * ${OVERBRIGHT.toFixed(1)};
          gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
        }`,
      fragmentShader: `varying vec3 vCol; void main() { gl_FragColor = vec4(vCol, 1.0); }`,
    });
  }

  /** Three RGBA8 DataTextures (one per state), bilinear, clamped, no mipmaps. */
  function lightmapTextures(THREE, atlas) {
    const out = {};
    for (const s of STATES) {
      const t = new THREE.DataTexture(atlas.sets[s], atlas.width, atlas.height, THREE.RGBAFormat, THREE.UnsignedByteType);
      t.magFilter = THREE.LinearFilter; t.minFilter = THREE.LinearFilter; t.generateMipmaps = false;
      t.wrapS = t.wrapT = THREE.ClampToEdgeWrapping; t.flipY = false; t.needsUpdate = true;
      out[s] = t;
    }
    return out;
  }

  /**
   * The lightmap variant of the deck shader: the same weights, three texture
   * fetches (a real engine binds one, or two while a state crossfades).
   * grid > 0 tints alternate texels, the debug view of texel density.
   */
  function lightmapMaterial(THREE, tex, atlas) {
    return new THREE.ShaderMaterial({
      uniforms: { stateW: { value: new THREE.Vector3(1, 0, 0) }, lmN: { value: tex.normal }, lmR: { value: tex.red_alert }, lmE: { value: tex.emergency },
        lmSize: { value: new THREE.Vector2(atlas.width, atlas.height) }, grid: { value: 0 }, aoOnly: { value: 0 } },
      vertexShader: `
        attribute vec3 albedo; attribute vec2 lmuv; varying vec3 vAlb; varying vec2 vUv;
        void main() { vAlb = albedo; vUv = lmuv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0); }`,
      fragmentShader: `
        uniform sampler2D lmN; uniform sampler2D lmR; uniform sampler2D lmE; uniform vec3 stateW; uniform vec2 lmSize; uniform float grid; uniform float aoOnly;
        varying vec3 vAlb; varying vec2 vUv;
        void main() {
          vec4 l = texture2D(lmN, vUv) * stateW.x + texture2D(lmR, vUv) * stateW.y + texture2D(lmE, vUv) * stateW.z;
          vec3 c = aoOnly > 0.5 ? vec3(l.a * ${OVERBRIGHT.toFixed(1)} * 0.5) : vAlb * l.rgb * ${OVERBRIGHT.toFixed(1)};
          if (grid > 0.5) { vec2 cell = floor(vUv * lmSize); float k = mod(cell.x + cell.y, 2.0); c = mix(c, c * vec3(1.35, 0.75, 1.35) + vec3(0.05, 0.0, 0.05), k * 0.6); }
          gl_FragColor = vec4(c, 1.0);
        }`,
    });
  }

  /**
   * The deck shader as the engine plans it, on a textured surface: extends a ShipKit
   * surface material (ShipKit.surfaceMaterial(THREE, mats, { lit: false }), the kit's
   * Material Maker texture array times the vertex colour) so the vertex colour is the
   * three baked sets blended by a per-compartment weight uniform (or, where a vertex's
   * lmw is 1, the light from a lightmap atlas). base is modified and returned.
   *
   * Geometry attributes: colN, colR, colE (RGBA8, normalized: light, occlusion in alpha),
   * lmuv (atlas UV), lmw (1 lightmapped, 0 vertex light), plus the kit's own. Stored
   * light is display-space (gamma 2.2, 2x overbright), so the shader decodes it to the
   * linear multiplier three.js works in: (2 c)^2.2.
   * U holds uniform objects shared by every material made with it: stateW (Vector3),
   * aoOnly (occlusion as grey), flat (0: textured; else a flat linear albedo instead of
   * the texel, for views where the light itself is the point), grid (lightmap texel
   * checker). lightmap: { textures, atlas } from lightmapTextures and bakeLightmap.
   */
  // TODO kit: ShipKit.surfaceMaterial could take the three baked sets and the weight uniform itself (it is
  // the engine's deck shader); until then this extends its shader text, and throws if that text moves.
  function bakedSurfaceMaterial(THREE, base, U, lightmap) {
    const kitHook = base.onBeforeCompile;
    const G = GAMMA.toFixed(1), OB = OVERBRIGHT.toFixed(1);
    const lm = lightmap ? { lmN: { value: lightmap.textures.normal }, lmR: { value: lightmap.textures.red_alert }, lmE: { value: lightmap.textures.emergency },
      lmSize: { value: new THREE.Vector2(lightmap.atlas.width, lightmap.atlas.height) } } : null;
    const need = (src, s) => { if (src.indexOf(s) < 0) throw new Error("lightbake: shader text not found (ShipKit surfaceMaterial changed?): " + s); };
    base.onBeforeCompile = (sh, renderer) => {
      kitHook(sh, renderer);
      Object.assign(sh.uniforms, { stateW: U.stateW, aoOnly: U.aoOnly, flatAlbedo: U.flat, lmGrid: U.grid }, lm || {});
      need(sh.vertexShader, "#include <color_vertex>");
      need(sh.fragmentShader, "diffuseColor.rgb *= surfTex.rgb;");
      need(sh.fragmentShader, "#include <color_fragment>");
      sh.vertexShader = sh.vertexShader
        .replace("#include <common>", "#include <common>\nattribute vec4 colN;\nattribute vec4 colR;\nattribute vec4 colE;\nattribute vec2 lmuv;\nattribute float lmw;\nuniform vec3 stateW;\nuniform float aoOnly;\nvarying vec2 vLm;\nvarying float vLmw;")
        .replace("#include <color_vertex>", `#include <color_vertex>
          {
            vec4 l = colN * stateW.x + colR * stateW.y + colE * stateW.z;
            vec3 c = aoOnly > 0.5 ? vec3(l.a) : ${OB} * l.rgb;
            vColor = mix(pow(c, vec3(${G})), vec3(1.0), lmw);
            vLm = lmuv; vLmw = lmw;
          }`);
      let frag = sh.fragmentShader
        .replace("#include <common>", "#include <common>\nuniform float flatAlbedo;\nuniform float aoOnly;\nuniform float lmGrid;\nvarying vec2 vLm;\nvarying float vLmw;" +
          (lm ? "\nuniform sampler2D lmN;\nuniform sampler2D lmR;\nuniform sampler2D lmE;\nuniform vec2 lmSize;\nuniform vec3 stateW;" : ""))
        .replace("diffuseColor.rgb *= surfTex.rgb;", "diffuseColor.rgb *= flatAlbedo > 0.0 ? vec3(flatAlbedo) : surfTex.rgb;");
      if (lm) {
        frag = frag.replace("#include <color_fragment>", `#include <color_fragment>
          {
            vec4 l = texture(lmN, vLm) * stateW.x + texture(lmR, vLm) * stateW.y + texture(lmE, vLm) * stateW.z;
            vec3 c = aoOnly > 0.5 ? vec3(l.a) : ${OB} * l.rgb;
            diffuseColor.rgb *= mix(vec3(1.0), pow(c, vec3(${G})), vLmw);
            if (lmGrid > 0.5 && vLmw > 0.5) {
              vec2 cell = floor(vLm * lmSize);
              float k = mod(cell.x + cell.y, 2.0);
              diffuseColor.rgb = mix(diffuseColor.rgb, diffuseColor.rgb * vec3(2.4, 0.3, 2.4) + vec3(0.03, 0.0, 0.03), k * 0.8);
            }
          }`);
      }
      sh.fragmentShader = frag;
    };
    base.customProgramCacheKey = () => "lightbake-" + (lightmap ? "lightmap" : "vertex");
    return base;
  }

  /** A canvas showing one state of an atlas (for a debug panel). */
  function atlasCanvas(atlas, state) {
    const cv = document.createElement("canvas"); cv.width = atlas.width; cv.height = atlas.height;
    const ctx = cv.getContext("2d"), img = ctx.createImageData(atlas.width, atlas.height), src = atlas.sets[state];
    for (let i = 0; i < atlas.width * atlas.height; i++) { img.data[i * 4] = src[i * 4]; img.data[i * 4 + 1] = src[i * 4 + 1]; img.data[i * 4 + 2] = src[i * 4 + 2]; img.data[i * 4 + 3] = 255; }
    ctx.putImageData(img, 0, 0);
    return cv;
  }

  window.LightBake = {
    version: 2, STATES, DEFAULTS, GAMMA, OVERBRIGHT,
    patch, boxPatches, orientedBoxPatches, polygonPatch, patchCorners, patchesFromGeometry, patchesFromTriangles,
    tessellate, buildBVH, occluded, closest,
    scene, prepare, irradiance, bounceAt, bakeVertices, bakeAdaptive, bakeLightmap, probeCube, cubeDisplay,
    encodeLevel, srgbToLinear, hexToSrgb, hexToLinear, hash3, digest,
    toGeometry, vertexMaterial, lightmapTextures, lightmapMaterial, bakedSurfaceMaterial, atlasCanvas,
    DIRECT, AMBIENT, BOUNCE, ALL,
  };
})();
