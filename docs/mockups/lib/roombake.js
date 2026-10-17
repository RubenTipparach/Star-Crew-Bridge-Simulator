/*
 * roombake.js: the baked light of a ship's compartment, for every mockup that draws one.
 *
 * What it owns: turning one room's mesh (ShipKit's buildCompartment and geometryOf, with whatever props a page
 * places) and its lamps into baked light, by light-baking design 15's method, as the deck plan has baked the Tern
 * since 2026-10-07: the room's surfaces as patches (each texture layer's mean colour times the vertex tint and a
 * prop's tint), a leaf closed in every door, the kit's lamps (ceiling panels, high-bay lamps, each console's glow), a
 * light strip down every cove, the kick strips; then direct light with soft shadows, occlusion and one bounce at
 * every vertex, in the three lighting states (normal, red alert, emergency) from one set of rays.
 *
 * sceneDef, key and bake are the deck plan's own bake (moved here 2026-10-16 from deck-plan.html, unchanged, so its
 * cache stays good). page() is the same bake for any other page: page.relight(geo, lamps, opts) bakes a room's mesh
 * (from the page's inlined cache when its key matches, else live, one room at a time) and puts the light on
 * geo.userData.bakedLight, which ShipKit.bakeDirect then writes instead of its quick light; opts.onBaked() lets the
 * page re-run its own lighting step. tools/mockups/bake_pages.mjs bakes a page's rooms fresh and writes its cache
 * (docs/mockups/cache/<page>-bake.bin), as bake_ship.mjs does the deck plan's.
 *
 * Why it lives here: it is documentation tooling (CLAUDE.md section 4), like lightbake.js, which it drives; one copy
 * of the room bake for every page (CLAUDE.md 6.1). A classic script, inlined by tools/mockups/inline.py between
 * "INLINE lib:roombake" markers, after lib:lightbake. Edit it here, never in a page.
 *
 * Units: metres; light as lightbake.js's (lux, then display multipliers 0-2 stored as linear light, (d)^2.2).
 */
(function () {
  "use strict";

  const srgb2lin = (c) => (c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4));
  const v3 = { add: (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2]], sub: (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]], mul: (a, k) => [a[0] * k, a[1] * k, a[2] * k],
    dot: (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2], cross: (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]] };
  const unit3 = (a) => v3.mul(a, 1 / (Math.hypot(a[0], a[1], a[2]) || 1));
  /** Roles that are drawn lights, not surfaces: they neither take nor give baked light. */
  const DRAWN = ["lamp", "status", "screen", "kick_strip"];

  /** Each texture layer's mean colour, linear, for the bounce: the main array's layers and every bigger array's. */
  function layerAlbedo(pmats) {
    const out = new Map();
    const mean = (data, px, i) => {
      let r = 0, g = 0, b = 0, n = 0;
      const off = i * px * px * 4;
      for (let k = 0; k < px * px; k += 7) { r += srgb2lin(data[off + k * 4] / 255); g += srgb2lin(data[off + k * 4 + 1] / 255); b += srgb2lin(data[off + k * 4 + 2] / 255); n++; }
      return [r / n, g / n, b / n];
    };
    const T = pmats.texture.image;
    for (let i = 0; i < T.depth; i++) out.set(i, mean(T.data, T.width, i));
    for (const B of pmats.big || []) { const I = B.texture.image; for (let i = 0; i < I.depth; i++) out.set(B.base + i, mean(I.data, I.width, i)); }
    return out;
  }

  /** A rectangle centred at c spanning +-ha along a and +-hb along b, facing facing (as lighting.html's quadPatch). */
  function quadPatch(LBK, c, a, b, ha, hb, facing, opts) {
    let u = v3.mul(a, 2 * ha), v = v3.mul(b, 2 * hb);
    if (v3.dot(v3.cross(u, v), facing) < 0) [u, v] = [v, u];
    return LBK.patch(v3.sub(c, v3.add(v3.mul(u, 0.5), v3.mul(v, 0.5))), u, v, opts);
  }

  /**
   * One room's bake scene definition: its surfaces as patches (each texture layer's mean colour, times its tint and a
   * prop's tint), door leaves in its doors, its lights and strips. o: { K, LBK, comp, geo, lamps (every lamp: the
   * kit's, a page's, each console's glow { fixed, axis, roleHex }), openings (buildCompartment's), kicks, coveParts
   * (the plain cove parts, for the cove strips), props (role -> { tint, glow }), tint (a tint per vertex when the page
   * keeps it off the geometry), albedo (layerAlbedo), fix (fixtures.json's types), ambient_lux, settings }.
   */
  function sceneDef(o) {
    const { K, LBK } = o, c = o.comp, geo = o.geo, PROPS = o.props || {}, FIX = o.fix;
    const P = geo.attributes.position.array, N = geo.attributes.normal.array, Ly = geo.attributes.surfLayer.array;
    const T = o.tint || (geo.attributes.tint ? geo.attributes.tint.array : null), groups = new Map();
    for (const [role, [v0, n]] of Object.entries(geo.userData.roles)) {
      if (DRAWN.includes(role) || (PROPS[role] && PROPS[role].glow)) continue;   // drawn lights, not surfaces
      const pt = PROPS[role] ? PROPS[role].tint : null;
      for (let v = v0; v < v0 + n; v += 3) {
        const a = o.albedo.get(Math.round(Ly[v])) || [0.18, 0.18, 0.18];
        const alb = [0, 1, 2].map((k) => Math.min(0.95, a[k] * (T ? T[v * 3 + k] : 1) * (pt ? [pt.r, pt.g, pt.b][k] : 1)));
        const key = alb.map((x) => Math.round(x * 50)).join(",");
        let g = groups.get(key);
        if (!g) { g = { alb, pos: [], nor: [] }; groups.set(key, g); }
        for (let k = 0; k < 9; k++) { g.pos.push(P[v * 3 + k]); g.nor.push(N[v * 3 + k]); }
      }
    }
    const patches = [];
    // A face under 0.1 m^2 (a prop's bevel, a bolt, a rail) casts shadows but its bounce is not worth its cache points.
    for (const g of groups.values()) for (const p of LBK.patchesFromTriangles(g.pos, g.nor, { albedo: g.alb, receive: false })) { if (p.area < 0.1) p.cheap = true; patches.push(p); }
    // Doors closed (design section 9): a leaf in every door, hatch and pressure door, as lighting.html draws them.
    const door = { albedo: [0.12, 0.12, 0.13], receive: false };
    for (const op of o.openings || []) {
      const p = op.portal;
      if (op.floor || !["door", "hatch", "pressure_door"].includes(p.kind)) continue;
      const f = K.portalFrame(p), out = K.portalOut(p, c.id), cc = f.c, hw = f.w / 2, hh = f.h / 2;
      patches.push(quadPatch(LBK, v3.add(cc, v3.mul(out, 0.05)), f.t, [0, 1, 0], hw, hh, v3.mul(out, -1), door));
    }
    // The lamps: the kit's (ceiling panels, high-bay lamps), the lower floor's, each console's glow.
    const lights = o.lamps.map((l, i) => {
      if (l.fixed) {
        const t = FIX.console_glow;
        return { id: `${c.id}_glow_${i}`, position_m: l.p.slice(), radius_m: t.radius_m, intensity_cd: t.intensity_cd, range_m: t.range_m,
          beam: l.axis ? { axis: unit3(l.axis), exponent: t.beam_exponent } : null, states: K.fixtureStates(t, true, l.roleHex || 0x48d6ff) };
      }
      const t = FIX[l.tall ? "high_bay" : "ceiling_panel"];
      return { id: `${c.id}_lamp_${i}`, position_m: [l.p[0], l.p[1] - t.drop_m, l.p[2]], radius_m: t.radius_m, intensity_cd: t.intensity_cd,
        range_m: t.range_m, beam: { axis: [0, -1, 0], exponent: t.beam_exponent }, states: K.fixtureStates(t, !!l.emergency) };
    });
    // A light strip along the middle of every cove (the kit's plain coves), facing into the room, as lighting.html's.
    const emitters = [], st = FIX.cove_strip;
    const plain = o.coveParts;
    if (plain) for (const cove of LBK.patchesFromTriangles(plain.position, plain.normal, {})) {
      const cs = LBK.patchCorners(cove).slice().sort((a, b) => a[1] - b[1]), A = cs[0], B = cs[1], hi = cs.slice(2);
      if (cs.length !== 4) continue;   // a cove's mitred end (a triangle) carries no strip
      const plan = (p, q) => Math.hypot(p[0] - q[0], p[2] - q[2]);
      const A2 = plan(A, hi[0]) <= plan(A, hi[1]) ? hi[0] : hi[1], B2 = A2 === hi[0] ? hi[1] : hi[0];
      const edge = Math.hypot(...v3.sub(B, A)), slant = Math.hypot(...v3.sub(A2, A));
      if (edge < 2 * st.end_clear_m + 0.3) continue;
      const mix = (a, b, f) => v3.add(a, v3.mul(v3.sub(b, a), f));
      const Q = (f, g) => v3.add(mix(mix(A, B, g), mix(A2, B2, g), f), v3.mul(cove.n, 0.01));
      const g0 = st.end_clear_m / edge, g1 = 1 - g0, f0 = 0.5 - st.width_m / 2 / slant, f1 = 0.5 + st.width_m / 2 / slant;
      const q0 = Q(f0, g0);
      let U = v3.sub(Q(f0, g1), q0), V = v3.sub(Q(f1, g0), q0);
      if (v3.dot(v3.cross(U, V), cove.n) < 0) [U, V] = [V, U];
      emitters.push({ id: `${c.id}_cove_${emitters.length}`, quad: { o: q0, u: U, v: V }, luminance_cd_m2: st.luminance_cd_m2, beam_exponent: 1,
        range_m: st.range_m, states: K.fixtureStates(st) });
    }
    // The kick strips (kick-lights design 4): every platform toe kick's and console's, facing out and down, and the
    // light columns beside the wall banks.
    for (const k of o.kicks || []) {
      const ks = FIX[k.type || "kick_strip"];   // a light column behind a terminal is a column_strip (design 2a)
      if (k.len < 0.05) continue;
      emitters.push({ id: `${c.id}_kick_${emitters.length}`, quad: { o: v3.add(k.o, v3.mul(k.n, 0.005)), u: k.u, v: k.v }, luminance_cd_m2: ks.luminance_cd_m2,
        beam_exponent: 1, range_m: ks.range_m, states: K.fixtureStates(ks) });
    }
    return { patches, lights, emitters, ambient_lux: o.ambient_lux, settings: o.settings };
  }

  /** A room's cache key: FNV-1a over its vertex positions and normals (as bits) and its bake scene's inputs. */
  function key(LBK, def, geo) {
    const g = geo.attributes;
    const pr = (p) => [p.o, p.u, p.v, p.albedo, p.cheap ? 1 : 0, p.cast ? 1 : 0, p.receive ? 1 : 0, p.tri ? 1 : 0, p.area];
    const text = JSON.stringify({ v: 1, patches: def.patches.map(pr), lights: def.lights, emitters: def.emitters, amb: def.ambient_lux, set: def.settings });
    const T = new Uint32Array(text.length);
    for (let i = 0; i < text.length; i++) T[i] = text.charCodeAt(i);
    return LBK.digest([new Uint32Array(g.position.array.buffer.slice(0)), new Uint32Array(g.normal.array.buffer.slice(0)), T]);
  }

  /** A room's bake: its lights at every vertex (direct and ambient traced, bounce from the cache), as display
   *  multipliers in three states (9 floats a vertex, linear), and its stats. roomId seeds the samples. */
  async function bake(LBK, def, geo, roomId, onProgress) {
    const BAKE_SET = def.settings;
    const t0 = performance.now(), S = LBK.scene(def), tScene = performance.now() - t0;
    await LBK.prepare(S, (f) => onProgress && onProgress(0.5 * f));
    const P = geo.attributes.position.array, N = geo.attributes.normal.array, nv = P.length / 3;
    // The cache's points in a 0.5 m grid, for the bounce at a vertex: the nearest ones on its plane, by distance.
    const C = S.cache, cell = 0.5, grid = new Map(), gk = (x, y, z) => `${Math.floor(x / cell)},${Math.floor(y / cell)},${Math.floor(z / cell)}`;
    if (S.field) for (let i = 0; i < C.vertexCount; i++) {
      const k = gk(C.positions[i * 3], C.positions[i * 3 + 1], C.positions[i * 3 + 2]);
      let l = grid.get(k); if (!l) grid.set(k, (l = [])); l.push(i);
    }
    const ref = BAKE_SET.reference_lux, out = new Float32Array(nv * 9), memo = new Map(), tmp = new Float32Array(9);
    const roomKey = Array.from(roomId).reduce((h, ch) => Math.imul(h ^ ch.charCodeAt(0), 16777619) >>> 0, 2166136261);
    const skip = new Uint8Array(nv);
    for (const role of DRAWN) { const g = geo.userData.roles[role]; if (g) skip.fill(1, g[0], g[0] + g[1]); }
    const tB = performance.now();
    let baked = 0;
    for (let v = 0; v < nv; v++) {
      if (skip[v]) continue;
      const px = P[v * 3], py = P[v * 3 + 1], pz = P[v * 3 + 2], nx = N[v * 3], ny = N[v * 3 + 1], nz = N[v * 3 + 2];
      const mk = `${Math.round(px * 1e3)},${Math.round(py * 1e3)},${Math.round(pz * 1e3)},${Math.round(nx * 100)},${Math.round(ny * 100)},${Math.round(nz * 100)}`;
      let E = memo.get(mk);
      if (!E) {
        tmp.fill(0);
        const ins = BAKE_SET.inset_m;
        LBK.irradiance(S, px + nx * ins, py + ny * ins, pz + nz * ins, nx, ny, nz, LBK.hash3(BAKE_SET.seed, roomKey, memo.size), LBK.DIRECT | LBK.AMBIENT, tmp);
        if (S.field) {   // bounce: inverse-distance weights over the cache points within 0.75 m on this plane
          let w = 0;
          const acc = new Float32Array(9), x0 = Math.floor((px - 0.75) / cell), x1 = Math.floor((px + 0.75) / cell);
          for (let gx = x0; gx <= x1; gx++) for (let gy = Math.floor((py - 0.75) / cell); gy <= Math.floor((py + 0.75) / cell); gy++)
            for (let gz = Math.floor((pz - 0.75) / cell); gz <= Math.floor((pz + 0.75) / cell); gz++) {
              for (const i of grid.get(`${gx},${gy},${gz}`) || []) {
                if (C.normals[i * 3] * nx + C.normals[i * 3 + 1] * ny + C.normals[i * 3 + 2] * nz < 0.9) continue;
                const dx = C.positions[i * 3] - px, dy = C.positions[i * 3 + 1] - py, dz = C.positions[i * 3 + 2] - pz;
                if (Math.abs(dx * nx + dy * ny + dz * nz) > 0.05) continue;
                const d = Math.hypot(dx, dy, dz);
                if (d > 0.75) continue;
                const wi = 1 / (d + 0.1);
                for (let k = 0; k < 9; k++) acc[k] += wi * S.field[i * 9 + k];
                w += wi;
              }
            }
          if (w > 0) for (let k = 0; k < 9; k++) tmp[k] += acc[k] / w;
        }
        E = new Float32Array(9);
        for (let k = 0; k < 9; k++) E[k] = Math.pow(LBK.encodeLevel(tmp[k] / ref), LBK.GAMMA);
        memo.set(mk, E);
      }
      out.set(E, v * 9);
      if ((++baked & 255) === 0) { if (onProgress) onProgress(0.5 + 0.5 * v / nv); await new Promise((res) => setTimeout(res, 0)); }
    }
    const bytes = new Uint8Array(out.buffer.slice(0));
    return { light: out, stats: {
      vertices: memo.size, triangles: nv / 3, patches: S.patches.length, lamps: S.lights.length, strips: S.emitters.length,
      cachePoints: C.vertexCount, rays: Object.assign({}, S.rays), sceneMs: Math.round(tScene), cacheMs: Math.round(S.prepareMs || 0),
      verticesMs: Math.round(performance.now() - tB), ms: Math.round(performance.now() - t0),
      digest: LBK.digest([bytes]),
    } };
  }

  /** Light (9 floats a vertex, linear display multipliers) to cache bytes and back, as the deck plan's cache keeps them. */
  const toBytes = (light) => { const b = new Uint8Array(light.length); for (let i = 0; i < b.length; i++) b[i] = Math.max(0, Math.min(255, Math.round((Math.pow(light[i], 1 / 2.2) / 2) * 255))); return b; };
  const fromBytes = (d, off, n) => { const out = new Float32Array(n); for (let i = 0; i < n; i++) out[i] = Math.pow((d[off + i] / 255) * 2, 2.2); return out; };

  /**
   * One page's room bakes. ctx: { name (the page; its cache is "<name>-bake"), THREE, K, LBK, L (the layout the rooms
   * were built from), DET (detailing), pmats }. relight(geo, lamps, opts) queues a room: opts { id (stable within the
   * page), comp (its compartment, for the door leaves' side and the plain coves), openings, kicks, props, tint,
   * pmats (the texture array it is drawn in), L and DET (its layout and detailing), each when not the page's;
   * coveParts (its plain coves, built from L and DET when not given), onBaked }. It returns a promise of the light. The page's whole bake: window.MOCKUP_BAKING() (a promise shoot.mjs
   * waits for before its shots), window.MOCKUP_ROOMBAKE (done, stats, export for tools/mockups/bake_pages.mjs).
   * ?bake=0 keeps the quick light; ?bake=fresh or ?bake=rooms ignores the cache.
   */
  function page(ctx) {
    const { THREE, K, LBK } = ctx, q = new URLSearchParams(location.search);
    // ?bake=rooms: the rooms fresh, whatever else the page bakes from its own caches (the spacedock's bays).
    const OFF = q.get("bake") === "0", FRESH = q.get("bake") === "fresh" || q.get("bake") === "rooms";
    const LD = K.lightingData(), FIX = LD.fixtures.types, SET = K.bakeSettings(), AMB = K.ambientLux(), ALB = new Map();
    /** Each texture array's layer means, once (a page may draw rooms in more than one array). */
    const albedoOf = (pm) => { if (!ALB.has(pm)) ALB.set(pm, layerAlbedo(pm)); return ALB.get(pm); };
    const cacheP = FRESH || OFF ? Promise.resolve(null) : LBK.readBakeCache(ctx.name + "-bake");
    const OUT = new Map(), STATS = new Map(), all = [];
    let chain = Promise.resolve(), pending = 0;
    const badge = document.createElement("div");
    badge.style.cssText = "position:fixed;left:50%;transform:translateX(-50%);top:8px;z-index:40;padding:4px 10px;border-radius:6px;background:rgba(8,11,16,.8);" +
      "border:1px solid #2a3340;color:#9fb0c4;font:600 12px system-ui,sans-serif;pointer-events:none;display:none";
    document.body.appendChild(badge);
    const label = (t) => { badge.textContent = t; badge.style.display = t ? "" : "none"; };
    function relight(geo, lamps, opts) {
      opts = opts || {};
      if (OFF) return Promise.resolve(null);
      const id = opts.id || "room" + all.length;
      pending++;
      const job = (chain = chain.then(async () => {
        const LL = opts.L || ctx.L, DD = opts.DET || ctx.DET;
        const coveParts = opts.coveParts || (opts.comp && LL ? (K.buildCompartment(THREE, LL, opts.comp, { detailing: DD, skipKinds: ["bay_door"] }).parts.cove || null) : null);
        const def = sceneDef({ K, LBK, comp: opts.comp || { id }, geo, lamps, openings: opts.openings, kicks: opts.kicks, coveParts, props: opts.props,
          tint: opts.tint, albedo: albedoOf(opts.pmats || ctx.pmats), fix: FIX, ambient_lux: AMB, settings: SET });
        const k = key(LBK, def, geo), cache = await cacheP, e = cache && cache.index.rooms && cache.index.rooms[id];
        let light;
        const again = OUT.get(id);   // a room the page built again, unchanged, takes the light it baked the first time
        if (e && e.key === k && e.count === geo.attributes.position.count * 9) { light = fromBytes(cache.data, e.offset, e.count); STATS.set(id, Object.assign({ cached: true }, e.stats)); }
        else if (again && again.key === k && again.light.length === geo.attributes.position.count * 9) light = again.light;
        else {
          const r = await bake(LBK, def, geo, opts.comp ? opts.comp.id : id, (f) => label(`Baking ${id}: ${Math.round(f * 100)}% (${pending} to go)`));
          light = r.light; STATS.set(id, r.stats); OUT.set(id, { key: k, light, stats: r.stats });
        }
        geo.userData.bakedLight = light;
        pending--;
        label(pending ? `Baking: ${pending} rooms to go` : "");
        if (opts.onBaked) opts.onBaked(light);
        return light;
      }));
      all.push(job);
      return job;
    }
    window.MOCKUP_BAKING = () => Promise.all(all);
    window.MOCKUP_ROOMBAKE = {
      /** The cache's name: docs/mockups/cache/<name>-bake.bin. */
      name: ctx.name,
      done: () => pending === 0,
      stats: () => Object.fromEntries(STATS),
      /** The rooms baked live here: { rooms: { id: { key, offset, count, stats } }, b64 }. */
      export: () => {
        const rooms = {}, chunks = [];
        let off = 0;
        for (const id of [...OUT.keys()].sort()) { const o = OUT.get(id), b = toBytes(o.light); rooms[id] = { key: o.key, offset: off, count: b.length, stats: o.stats }; chunks.push(b); off += b.length; }
        const allB = new Uint8Array(off);
        let p = 0;
        for (const c of chunks) { allB.set(c, p); p += c.length; }
        let s = "";
        for (let i = 0; i < allB.length; i += 0x8000) s += String.fromCharCode(...allB.subarray(i, i + 0x8000));
        return { rooms, b64: btoa(s) };
      },
    };
    return { relight, off: OFF };
  }

  window.RoomBake = { DRAWN, layerAlbedo, sceneDef, key, bake, page, toBytes, fromBytes };
})();
