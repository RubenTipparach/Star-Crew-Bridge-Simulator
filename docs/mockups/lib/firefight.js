/*
 * firefight.js: fighting a fire on foot, in a mockup's first-person walk.
 *
 * What it owns: the player's side of openspec/changes/fire-spread (design 3, 5 and 6) and safety-points (1, 3 and 3a),
 * in one place for every page that walks into a fire (CLAUDE.md 6.1): fire.html's deck B and deck-plan.html's whole
 * ship. That is the safety points (an extinguisher in its bracket, a first aid cabinet and a glowing sign inside a
 * room's main door) and their restock; taking an extinguisher; its nozzle, aim, spray and footprint ring; the flames on
 * burning cells; and the smoke layer and the fog it makes in a room. It moved here from fire.html (2026-10-10) when the
 * deck plan's walk gained fires, so the two pages cannot drift apart.
 *
 * It draws and handles input; it decides nothing about fire. The fire is lib/firespread.js inside lib/shipsystems.js,
 * which this file reads (sim().fs, sim().links, compartmentReadout) and drives only through a held extinguisher
 * (sim().newExtinguisher(): its room, aim and on). Every tuning number is data: atmosphere.json fire.cells and
 * fire.extinguisher, bracket_restock_s included (CLAUDE.md 6.5). The bracket's restock runs on the simulation's clock.
 *
 * The safety points are placed here by the change's rule for the mockups only: tools/safety_points.py will own them
 * (kit.json, safety-points tasks 1.2), and this placement goes. The models are low-poly stand-ins until
 * tools/blender/build_safety_props.py (safety-points 5).
 *
 * Classic script (window.FireFight), after shipkit.js and firespread.js; three.js and its mergeGeometries are passed
 * in. Units: SI, metres and seconds.
 */
(function (root) {
  "use strict";

  const SP_FROM_FRAME_M = 0.4, SP_GAP_M = 0.5, SP_OFF_WALL_M = 0.015;   // safety-points 1: beside the door, inside
  const REACH_M = 1.3;           // how near a bracket a hand takes its extinguisher
  const MAX_FLAMES = 256;        // fire-spread design 7
  const NP = 220;                // spray particles
  const RED = 0xb8231b, DARK = 0x1b1f24, WHITE = 0xd8dde2, GREEN = 0x18b05a, SIGN = 0xa8ffd0;

  /**
   * o: { K, L, DET, FI, sim: () => the ShipSystems, scene, camera, walk, mergeGeometries, items (FireSpread.placements),
   * pointRooms (room ids that take a safety point at their main door), extraPoints ([{ id, room, at(d), into, t, d0,
   * y0 }]), clip (clipping planes for the drawn parts, optional), blocked(room, x, z) (optional, a floor point a point's
   * wall must keep clear; default: the items' footprints), floorAt(x, z, y) (optional: where a flame stands, for floors
   * raised over the cells), covers ([{ room, poly, top_y }]: what stands on the floor, its plan footprint and its top's
   * height, metres; a flame on a cell under one no taller than fire.cells.flame_on_props_below_m stands on its top),
   * extraKit ({ near(pose), take() }, optional: another place to take one, fire.html's locker) }.
   */
  function create(THREE, o) {
    const K = o.K, L = o.L, DET = o.DET, FI = o.FI, CELLS = FI.cells, EX = FI.extinguisher;
    if (!(Number.isFinite(EX.bracket_restock_s) && EX.bracket_restock_s >= 0))
      throw new Error("atmosphere.json fire.extinguisher.bracket_restock_s: missing or below zero (openspec/changes/safety-points design 3a)");
    const scene = o.scene, camera = o.camera, sim = o.sim, clip = o.clip || [];
    const walk = { pose: () => (typeof o.walk === "function" ? o.walk() : o.walk).pose() };   // the walk may be made after this
    const items = o.items || [];
    const blocked = o.blocked || ((room, x, z) => items.some((it) => it.room === room && K.insidePoly(it.poly, x, z)));
    const F = { ext: null, carrying: false, sprayHeld: false, points: [] };
    const own = [];   // every object this file adds to the scene
    const add = (obj) => { scene.add(obj); own.push(obj); return obj; };

    // ---------------------------------------------------------------- models
    const VC = new THREE.MeshBasicMaterial({ vertexColors: true, clippingPlanes: clip });
    /** A geometry painted one colour, darker on faces that look away from a light above and in front. */
    function paint(g, hex, glow) {
      g = g.index ? g.toNonIndexed() : g;
      const c = new THREE.Color(hex), n = g.attributes.normal, a = new Float32Array(n.count * 3);
      for (let i = 0; i < n.count; i++) { const k = glow ? 1 : 0.62 + 0.3 * Math.max(0, n.getY(i)) + 0.12 * Math.max(0, n.getZ(i)); a.set([c.r * k, c.g * k, c.b * k], i * 3); }
      g.setAttribute("color", new THREE.BufferAttribute(a, 3));
      for (const k of Object.keys(g.attributes)) if (!["position", "normal", "color"].includes(k)) g.deleteAttribute(k);
      return g;
    }
    function extinguisherGeo(scale) {
      const parts = [];
      parts.push(paint(new THREE.CylinderGeometry(0.075 * scale, 0.075 * scale, 0.42 * scale, 10).translate(0, 0.21 * scale, 0), RED));
      parts.push(paint(new THREE.SphereGeometry(0.075 * scale, 10, 4, 0, Math.PI * 2, 0, Math.PI / 2).translate(0, 0.42 * scale, 0), RED));
      parts.push(paint(new THREE.BoxGeometry(0.04 * scale, 0.07 * scale, 0.1 * scale).translate(0, 0.5 * scale, 0.02 * scale), DARK));
      parts.push(paint(new THREE.ConeGeometry(0.035 * scale, 0.16 * scale, 8).rotateX(Math.PI / 2).translate(0, 0.48 * scale, 0.14 * scale), DARK));
      return o.mergeGeometries(parts);
    }
    function extinguisherModel(scale) { return new THREE.Mesh(extinguisherGeo(scale), VC); }

    // ---------------------------------------------------------------- safety points (safety-points 1-2, 3a)
    /** A room's main door: its widest door to a corridor, else (a room off another, the bridge) its widest door. */
    function mainDoor(id) {
      const doors = L.portals.filter((p) => p.kind === "door" && Math.abs(p.normal[1]) < 0.5 && p.between.includes(id) && !p.between.includes("space"));
      const wide = (ps) => ps.sort((a, b) => b.size_m[0] - a.size_m[0])[0] || null;
      return wide(doors.filter((p) => p.between.some((s) => s !== id && (K.compartment(L, s) || {}).kind === "corridor"))) || wide(doors);
    }
    for (const id of o.pointRooms || []) {
      const p = mainDoor(id);
      if (!p) continue;
      const into = p.between[0] === id ? [-p.normal[0], -p.normal[2]] : [p.normal[0], p.normal[2]];   // the room's side of the wall
      const along = p.size_m[0] / 2 + DET.door_frame.jamb_m + SP_FROM_FRAME_M;
      const base = [p.center_m[0] + into[0] * SP_OFF_WALL_M, 0, p.center_m[2] + into[1] * SP_OFF_WALL_M];
      // Along the wall, on the side whose 1 m beside the door is clear of the room's props (the latch side, the change
      // says; the doors slide, so the clear side stands in for it).
      const clear = (t) => [0.3, 0.6, 0.9, 1.2].every((e) => !blocked(id, base[0] + t[0] * (along + e) + into[0] * 0.35, base[2] + t[1] * (along + e) + into[1] * 0.35));
      const t = [[-into[1], into[0]], [into[1], -into[0]]].find(clear) || [-into[1], into[0]];
      F.points.push({ id: "sp_" + id, room: id, at: (d) => [base[0] + t[0] * d, base[2] + t[1] * d], into, t, d0: along + 0.15, y0: p.center_m[1] - p.size_m[1] / 2 });
    }
    for (const e of o.extraPoints || []) F.points.push(Object.assign({ y0: 0 }, e));
    const FIXED = [];   // the points' brackets, cabinets, crosses and signs, merged into one mesh below
    function onWall(g, sp, d, y, out) {
      const [x, z] = sp.at(d);
      g.rotateY(Math.atan2(sp.into[0], sp.into[1])); g.translate(x + sp.into[0] * out, sp.y0 + y, z + sp.into[1] * out);
      return g;
    }
    // The restock's ghost: the extinguisher in its red at a quarter of its brightness, filling from the bottom up (3a).
    const GHOST = new THREE.MeshBasicMaterial({ color: RED, transparent: true, opacity: 0.35, depthWrite: false, clippingPlanes: clip });
    for (const sp of F.points) {
      // The bracket's back plate and strap, the extinguisher (its handle at 1.1 m), the cabinet with its cross, the sign at 1.9 m.
      FIXED.push(onWall(paint(new THREE.BoxGeometry(0.2, 0.5, 0.02), DARK), sp, sp.d0, 0.8, 0.01));
      FIXED.push(onWall(paint(new THREE.BoxGeometry(0.19, 0.04, 0.17), DARK), sp, sp.d0, 0.78, 0.105));
      const g = extinguisherGeo(1); g.rotateY(Math.atan2(sp.into[0], sp.into[1]));
      const [ex, ez] = sp.at(sp.d0);
      sp.ext = new THREE.Mesh(g, VC); sp.ext.position.set(ex + sp.into[0] * 0.1, sp.y0 + 0.58, ez + sp.into[1] * 0.1); add(sp.ext);
      sp.ghost = new THREE.Mesh(g, GHOST); sp.ghost.position.copy(sp.ext.position); sp.ghost.visible = false; sp.ghost.renderOrder = 3; add(sp.ghost);
      FIXED.push(onWall(paint(new THREE.BoxGeometry(0.45, 0.35, 0.14), WHITE), sp, sp.d0 + SP_GAP_M + 0.1, 1.25, 0.07));
      for (const [w, h] of [[0.16, 0.05], [0.05, 0.16]]) FIXED.push(onWall(paint(new THREE.BoxGeometry(w, h, 0.012), GREEN, true), sp, sp.d0 + SP_GAP_M + 0.1, 1.25, 0.146));
      FIXED.push(onWall(paint(new THREE.BoxGeometry(0.30, 0.15, 0.012), SIGN, true), sp, sp.d0 + SP_GAP_M / 2 + 0.05, 1.9, 0.006));
      sp.pos = [ex + sp.into[0] * 0.4, ez + sp.into[1] * 0.4];
      sp.has = true; sp.restock = 0;
    }
    if (FIXED.length) add(new THREE.Mesh(o.mergeGeometries(FIXED), VC));
    /** Show a point as it is: its extinguisher, or the ghost filling toward the restock. */
    function showPoint(sp) {
      sp.ext.visible = sp.has;
      sp.ghost.visible = !sp.has;
      if (!sp.has) sp.ghost.scale.set(1, Math.max(0.02, 1 - sp.restock / Math.max(1e-6, EX.bracket_restock_s)), 1);
    }
    /** The restock (3a), on the simulation's clock: dt is simulated seconds. */
    function tick(dt) {
      for (const sp of F.points) {
        if (sp.has) continue;
        sp.restock = Math.max(0, sp.restock - dt);
        if (sp.restock <= 0) sp.has = true;
        showPoint(sp);
      }
    }

    // ---------------------------------------------------------------- taking one
    /** The kit within reach of the body: a point's extinguisher, or the page's extra place. */
    function near() {
      const p = walk.pose();
      for (const sp of F.points) if (sp.has && Math.abs(p.y - sp.y0) < 1.2 && Math.hypot(sp.pos[0] - p.x, sp.pos[1] - p.z) < REACH_M) return { sp };
      if (o.extraKit && o.extraKit.near(p)) return { extra: true };
      return null;
    }
    /** Take the extinguisher in reach (E). A full one in hand is kept; a part-used one is put down for the full one. */
    function take() {
      const k = near();
      if (!k || (F.carrying && F.ext.agent_kg >= EX.agent_kg - 1e-6)) return false;
      if (k.sp) { k.sp.has = false; k.sp.restock = EX.bracket_restock_s; showPoint(k.sp); }
      else o.extraKit.take();
      F.carrying = true; F.ext.agent_kg = EX.agent_kg; handExt.visible = true;
      return true;
    }
    /** True when E would take one now: one in reach, and the hand not already holding a full one. */
    function canTake() { return !!near() && !(F.carrying && F.ext.agent_kg >= EX.agent_kg - 1e-6); }
    /** A new simulation: a held extinguisher with nothing in it, every bracket full. */
    function reset() {
      F.ext = sim().newExtinguisher(); F.ext.agent_kg = 0; F.carrying = false; F.sprayHeld = false;
      handExt.visible = false;
      for (const sp of F.points) { sp.has = true; sp.restock = 0; showPoint(sp); }
    }

    // ---------------------------------------------------------------- flames: billboards on burning cells (one instanced draw)
    const fGeo = new THREE.InstancedBufferGeometry();
    fGeo.setAttribute("position", new THREE.Float32BufferAttribute([-0.5, 0, -1000, 0.5, 0, -1000, 0.5, 1, -1000, -0.5, 0, -1000, 0.5, 1, -1000, -0.5, 1, -1000], 3));
    const fPos = new Float32Array(MAX_FLAMES * 3), fP = new Float32Array(MAX_FLAMES * 4), fL = new Float32Array(MAX_FLAMES * 3);
    const fPosA = new THREE.InstancedBufferAttribute(fPos, 3), fPA = new THREE.InstancedBufferAttribute(fP, 4), fLA = new THREE.InstancedBufferAttribute(fL, 3);
    for (const a of [fPosA, fPA, fLA]) a.setUsage(THREE.DynamicDrawUsage);
    fGeo.setAttribute("iPos", fPosA); fGeo.setAttribute("iP", fPA); fGeo.setAttribute("iL", fLA);
    fGeo.instanceCount = 0;
    const flameU = { uTime: { value: 0 } };
    const flameMat = new THREE.ShaderMaterial({
      uniforms: flameU, transparent: true, depthWrite: false, blending: THREE.AdditiveBlending,
      vertexShader: `attribute vec3 iPos; attribute vec4 iP; attribute vec3 iL; uniform float uTime;
        varying vec2 vUv; varying float vHeat; varying float vPhase; varying float vKind;
        void main() {
          vUv = vec2(position.x + 0.5, position.y);
          vec3 toCam = cameraPosition - iPos; toCam.y = 0.0;
          float l = length(toCam);
          vec3 right = l > 1e-4 ? vec3(toCam.z, 0.0, -toCam.x) / l : vec3(1.0, 0.0, 0.0);
          float y = position.y;
          float sway = sin(uTime * 3.3 + iP.z * 6.28) * 0.07 * y * y;
          vec3 w = iPos + right * (position.x * iP.x + sway) + vec3(0.0, y * iP.y, 0.0) + vec3(iL.x, 0.0, iL.y) * y * y * iP.y;
          gl_Position = projectionMatrix * viewMatrix * vec4(w, 1.0);
          vHeat = iP.w; vPhase = iP.z; vKind = iL.z;
        }`,
      fragmentShader: `uniform float uTime; varying vec2 vUv; varying float vHeat; varying float vPhase; varying float vKind;
        float hash(vec2 p) { return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }
        float noise(vec2 p) { vec2 i = floor(p), f = fract(p); f = f * f * (3.0 - 2.0 * f);
          return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), f.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), f.x), f.y); }
        float fbm(vec2 p) { float v = 0.0, a = 0.5; for (int i = 0; i < 4; i++) { v += a * noise(p); p *= 2.03; a *= 0.5; } return v; }
        void main() {
          vec2 uv = vUv;
          if (vKind > 0.5) {
            float n = fbm(vec2(uv.x * 5.0 + vPhase * 13.0, uv.y * 2.0 - uTime * 0.5));
            float m = smoothstep(0.52, 0.8, n) * (1.0 - uv.y) * smoothstep(0.0, 0.2, uv.x) * smoothstep(1.0, 0.8, uv.x);
            float fl = 0.55 + 0.45 * sin(uTime * 3.7 + vPhase * 21.0);
            gl_FragColor = vec4(vec3(1.0, 0.22, 0.04) * m * fl * 1.4, 1.0);
            return;
          }
          float t = uTime * 1.7 + vPhase * 7.0;
          float n = fbm(vec2(uv.x * 3.0 + vPhase * 5.0, uv.y * 2.3 - t));
          float n2 = fbm(vec2(uv.x * 6.5 - vPhase, uv.y * 4.5 - t * 1.6));
          float x = (uv.x - 0.5) * 2.0;
          float wy = mix(0.85, 0.06, pow(uv.y, 0.75));
          float d = abs(x + (n - 0.5) * 0.8 * uv.y) / max(wy, 0.03);
          float body = 1.0 - smoothstep(0.3, 1.0, d + (n2 - 0.5) * 0.7);
          body *= smoothstep(1.0, 0.5, uv.y + (n - 0.5) * 0.5);
          body *= smoothstep(0.0, 0.05, uv.y);
          float core = body * (1.0 - smoothstep(0.0, 0.6, uv.y * 1.2 + d * 0.6));
          vec3 col = mix(vec3(0.85, 0.1, 0.02), vec3(1.0, 0.5, 0.07), smoothstep(0.0, 0.7, body));
          col = mix(col, vec3(1.0, 0.9, 0.55), core);
          gl_FragColor = vec4(col * body * (0.6 + 0.5 * vHeat) * 1.25, 1.0);
        }`,
    });
    const flameMesh = new THREE.Mesh(fGeo, flameMat);
    flameMesh.frustumCulled = false; flameMesh.renderOrder = 4;
    add(flameMesh);
    // What stands on the floor, by room: a flame on a low prop's cell stands on its top (the cells do not see height).
    const coversOf = {};
    for (const c of o.covers || []) (coversOf[c.room] = coversOf[c.room] || []).push(c);
    /** Where a room's flames lean: toward its open door with the most flow, [x, z] in metres per metre of flame. */
    function leanOf(rid) {
      const sys = sim(), i = sys.idx[rid];
      let best = null;
      for (const l of sys.links) {
        if (!l.mix || l.open <= 0.05 || (l.a !== i && l.b !== i) || !l.center) continue;
        if (!best || l.open > best.open) best = l;
      }
      if (!best) return [0, 0];
      const c = sys.fs.centroid(rid);
      if (!c) return [0, 0];
      const dx = best.center[0] - c.x, dz = best.center[2] - c.z, d = Math.hypot(dx, dz) || 1;
      return [(dx / d) * 0.35 * best.open, (dz / d) * 0.35 * best.open];
    }
    /** Flames on burning cells and embers on knocked-down ones, hottest first, in rooms (ids; every room when absent). */
    function updateFlames(roomIds) {
      const FS = sim().fs, list = [], embers = [];
      const rooms = roomIds ? roomIds.map((id) => FS.byId[id]) : FS.rooms;
      for (const fr of rooms) {
        if (!fr || (!fr.active && fr.burning === 0)) continue;
        const lean = leanOf(fr.id);
        for (let k = 0; k < fr.n; k++) {
          if (fr.state[k] === FS.BURNING) list.push([fr, k, lean]);
          else if (fr.state[k] === FS.KNOCKED && fr.fuel[k] < fr.fuel0[k] * 0.999) embers.push([fr, k]);
        }
      }
      list.sort((a, b) => b[0].q[b[1]] - a[0].q[a[1]]);
      const base = (fr, k) => {
        const y = o.floorAt ? o.floorAt(fr.x[k], fr.z[k], fr.y[k]) : fr.y[k];
        let top = y;
        for (const c of coversOf[fr.id] || []) if (c.top_y > top && c.top_y - y <= CELLS.flame_on_props_below_m && K.insidePoly(c.poly, fr.x[k], fr.z[k])) top = c.top_y;
        return top;
      };
      let n = 0;
      for (const [fr, k, lean] of list) {
        if (n >= MAX_FLAMES) break;
        const h = FS.flameHeight(fr.q[k], fr.peak[k]), heat = Math.min(1, fr.q[k] / fr.peak[k]);
        const ph = ((fr.ri * 7919 + k * 104729) % 1000) / 1000;
        fPos.set([fr.x[k] + (ph - 0.5) * 0.12, base(fr, k) + 0.01, fr.z[k] + (((ph * 7) % 1) - 0.5) * 0.12], n * 3);
        fP.set([0.55 + 0.35 * heat, h, ph, heat], n * 4); fL.set([lean[0], lean[1], 0], n * 3); n++;
      }
      for (const [fr, k] of embers) {
        if (n >= MAX_FLAMES) break;
        const ph = ((fr.ri * 7919 + k * 104729) % 1000) / 1000;
        fPos.set([fr.x[k], base(fr, k) + 0.02, fr.z[k]], n * 3); fP.set([0.5, 0.16, ph, 0.3], n * 4); fL.set([0, 0, 1], n * 3); n++;
      }
      fGeo.instanceCount = n;
      fPosA.needsUpdate = fPA.needsUpdate = fLA.needsUpdate = true;
      return n;
    }

    // ---------------------------------------------------------------- smoke: the layer under the ceiling, and the fog in it
    const readout = (id) => sim().compartmentReadout(sim().idx[id]);
    /** The smoke layer's depth in room r ({ id, floorY, top }), metres from the ceiling: the room's smoke gathered at smoke_layer_ppm (fire-spread 3). */
    function layerDepth(r) {
      const ppm = readout(r.id).smoke_ppm, H = r.top - r.floorY;
      return Math.min(H, (H * ppm) / CELLS.smoke_layer_ppm);
    }
    /** Give room r ({ comp }) its smoke layer: one translucent quad of its tallest, largest brush (r.smoke, r.smokeTop). */
    function addSmoke(r) {
      const br = r.comp.brushes.reduce((m, b) => (b.y[1] >= m.y[1] && K.signedArea(b.poly) > K.signedArea(m.poly) ? b : m));
      const shape = new THREE.Shape(br.poly.map((p) => new THREE.Vector2(p[0], p[1])));
      const g = new THREE.ShapeGeometry(shape); g.rotateX(Math.PI / 2);
      const m = new THREE.MeshBasicMaterial({ color: 0x1b1a19, transparent: true, opacity: 0, depthWrite: false, side: THREE.DoubleSide });
      const mesh = new THREE.Mesh(g, m); mesh.renderOrder = 6; mesh.visible = false;
      add(mesh);
      r.smoke = mesh; r.smokeTop = br.y[1];
      return mesh;
    }
    /** Set room r's smoke layer: from the ceiling down by its smoke; on a board (seen from above) a tint of the layer's heat. */
    function updateSmoke(r, board) {
      const sys = sim(), i = sys.idx[r.id], depth = layerDepth(r), rd = readout(r.id), own = sys.fs.centroid(r.id);
      const sm = r.smoke, m = sm.material, hot = Math.max(0, Math.min(1, (rd.t_k - 323.15) / 250));
      const mist = sys.st.fire[i].mist > 0;
      sm.visible = depth > 0.04 || (board && hot > 0.02) || mist;
      sm.position.y = board ? Math.min(r.smokeTop - depth, 2.2) : r.smokeTop - depth;
      if (mist) { m.color.setRGB(0.6, 0.85, 1.0); m.opacity = board ? 0.12 : 0.3; }
      else if (board) { m.color.setRGB(0.25 + 0.75 * hot, 0.16 + 0.2 * hot, 0.1); m.opacity = Math.min(0.22, 0.05 + 0.17 * hot); }
      else {
        const glow = own ? Math.min(0.5, own.hrr / 2e6) : 0;
        m.color.setRGB(0.012 + glow * 0.07, 0.011 + glow * 0.025, 0.01 + glow * 0.006);
        m.opacity = Math.min(0.94, 0.68 + 0.3 * depth / 3);
      }
    }
    /** The walk's fog in room r for an eye at height eye: in the layer the sight is 3 / K metres (crew-on-deck 9), below it clearer. */
    function walkFog(r, eye, smokeK) {
      const depth = layerDepth(r), bottom = r.smokeTop - depth, rd = readout(r.id);
      const inLayer = eye > bottom;
      const k = smokeK * (inLayer ? Math.max(rd.smoke_ppm, depth > 0.04 ? CELLS.smoke_layer_ppm : 0) : rd.smoke_ppm * 0.25) * 1e-6;
      const far = k > 1e-6 ? Math.max(0.9, Math.min(80, 3 / k)) : 80;
      if (!scene.fog) scene.fog = new THREE.Fog(0x0e0d0c, 0, 80);
      scene.fog.far = far; scene.fog.near = Math.min(far * 0.15, 2);
      scene.fog.color.setRGB(inLayer ? 0.07 : 0.04, inLayer ? 0.065 : 0.04, inLayer ? 0.06 : 0.045);
      return far;
    }

    // ---------------------------------------------------------------- the spray, the footprint ring, the extinguisher in hand
    const pGeo = new THREE.BufferGeometry(), pPos = new Float32Array(NP * 3), pAge = new Float32Array(NP).fill(1), pVel = new Float32Array(NP * 3);
    pGeo.setAttribute("position", new THREE.BufferAttribute(pPos, 3)); pGeo.setAttribute("aAge", new THREE.BufferAttribute(pAge, 1));
    const spray = new THREE.Points(pGeo, new THREE.ShaderMaterial({
      transparent: true, depthWrite: false,
      vertexShader: `attribute float aAge; varying float vA; void main() { vA = aAge; vec4 mv = modelViewMatrix * vec4(position, 1.0); gl_PointSize = (14.0 + 90.0 * aAge) * (1.0 / max(0.2, -mv.z)); gl_Position = projectionMatrix * mv; }`,
      fragmentShader: `varying float vA; void main() { vec2 d = gl_PointCoord - 0.5; float r = dot(d, d); if (r > 0.25 || vA >= 1.0) discard; gl_FragColor = vec4(vec3(0.92, 0.95, 0.98), (1.0 - vA) * 0.42 * (1.0 - r * 4.0)); }`,
    }));
    spray.frustumCulled = false; spray.renderOrder = 7; add(spray);
    let pNext = 0;
    const ring = new THREE.Mesh(new THREE.RingGeometry(0.88, 1, 48), new THREE.MeshBasicMaterial({ color: 0xe8f4ff, transparent: true, opacity: 0.75, depthWrite: false, side: THREE.DoubleSide }));
    ring.geometry.rotateX(-Math.PI / 2); ring.renderOrder = 5; ring.visible = false; add(ring);
    const handExt = extinguisherModel(1.0);
    handExt.scale.setScalar(0.75); handExt.position.set(0.34, -0.6, -0.78); handExt.rotation.set(0.25, -0.35, 0);
    handExt.material = new THREE.MeshBasicMaterial({ vertexColors: true });
    handExt.visible = false; camera.add(handExt);

    /** The nozzle and the way it points: from the hand toward where the crosshair meets the floor (the flames' base). */
    const _f = new THREE.Vector3(), _r = new THREE.Vector3();
    function nozzleAim() {
      camera.getWorldDirection(_f);
      _r.set(1, 0, 0).applyQuaternion(camera.quaternion);
      const from = [camera.position.x + _r.x * 0.22 + _f.x * 0.45, camera.position.y - 0.42, camera.position.z + _r.z * 0.22 + _f.z * 0.45];
      const p = walk.pose(), base = p.y + CELLS.aim_height_m;
      let to;
      if (_f.y < -0.02) { const s = (base - camera.position.y) / _f.y; to = [camera.position.x + _f.x * s, base, camera.position.z + _f.z * s]; }
      else to = [camera.position.x + _f.x * 4, camera.position.y + _f.y * 4, camera.position.z + _f.z * 4];
      return { from, dir: [to[0] - from[0], to[1] - from[1], to[2] - from[2]], to };
    }
    function emitSpray(aim, dt) {
      const d = Math.hypot(...aim.dir), u = aim.dir.map((x) => x / d);
      const per = Math.round(dt * 260);
      for (let k = 0; k < per; k++) {
        const i = pNext; pNext = (pNext + 1) % NP;
        const half = ((EX.cone_deg / 2) * Math.PI) / 180, a = Math.random() * Math.PI * 2, s = Math.sqrt(Math.random()) * Math.tan(half);   // visual only
        const o1 = Math.abs(u[1]) < 0.9 ? [0, 1, 0] : [1, 0, 0];
        const b1 = [u[1] * o1[2] - u[2] * o1[1], u[2] * o1[0] - u[0] * o1[2], u[0] * o1[1] - u[1] * o1[0]], l1 = Math.hypot(...b1);
        const e1 = b1.map((x) => x / l1), e2 = [u[1] * e1[2] - u[2] * e1[1], u[2] * e1[0] - u[0] * e1[2], u[0] * e1[1] - u[1] * e1[0]];
        const v = [0, 1, 2].map((j) => (u[j] + (e1[j] * Math.cos(a) + e2[j] * Math.sin(a)) * s) * 6.5);
        pPos.set(aim.from, i * 3); pVel.set(v, i * 3); pAge[i] = 0;
      }
    }
    function stepSpray(dt, floorY) {
      for (let i = 0; i < NP; i++) {
        if (pAge[i] >= 1) continue;
        pAge[i] = Math.min(1, pAge[i] + dt / 0.55);
        for (let j = 0; j < 3; j++) pPos[i * 3 + j] += pVel[i * 3 + j] * dt;
        pVel[i * 3] *= 0.93; pVel[i * 3 + 1] = pVel[i * 3 + 1] * 0.93 - 0.6 * dt; pVel[i * 3 + 2] *= 0.93;
        if (pPos[i * 3 + 1] < floorY + 0.03) { pPos[i * 3 + 1] = floorY + 0.03; pVel[i * 3 + 1] = Math.abs(pVel[i * 3 + 1]) * 0.2; }
      }
      pGeo.attributes.position.needsUpdate = pGeo.attributes.aAge.needsUpdate = true;
    }
    /**
     * The held extinguisher for this frame: its room (a compartment index, -1 for none), its aim from the camera, on
     * while the trigger is held and agent is left; the spray and the footprint ring drawn. eye: the camera's height.
     */
    function aimFrame(dt, roomIdx, floorY, eye) {
      const aim = F.carrying ? nozzleAim() : null, ext = F.ext;
      ext.comp = roomIdx;
      ext.aim = aim ? { from: aim.from, dir: aim.dir } : null;
      ext.on = F.carrying && F.sprayHeld && ext.agent_kg > 0 && roomIdx >= 0;
      if (ext.on) emitSpray(aim, dt);
      stepSpray(dt, floorY);
      ring.visible = !!(ext.on && aim && aim.to[1] < eye);
      if (ring.visible) {
        const dist = Math.hypot(...aim.dir), rad = dist * Math.tan(((EX.cone_deg / 2) * Math.PI) / 180);
        ring.position.set(aim.to[0], aim.to[1] - CELLS.aim_height_m + 0.03, aim.to[2]);
        ring.scale.set(rad, 1, rad);
        ring.material.opacity = dist <= EX.reach_m ? 0.8 : 0.25;
      }
      return aim;
    }
    // ---------------------------------------------------------------- a small HUD, for a page with none of its own
    // Glance first (CLAUDE.md 10): the take prompt (E and the extinguisher), the agent left as a ring round the
    // extinguisher while one is held, and the alarm's banner (a flame and the room's name) when a fire breaks out.
    let hudEl = null;
    function hud() {
      if (hudEl) return hudEl;
      const css = document.createElement("style");
      css.textContent = `
.ff-take{position:fixed;left:50%;top:calc(50% + 40px);transform:translateX(-50%);z-index:60;pointer-events:none;padding:6px 12px;display:none;
  background:rgba(6,9,14,.84);border:1px solid #2b3540;border-radius:6px;font:600 14px system-ui,sans-serif;color:#e8eef6;white-space:nowrap}
.ff-take .k{display:inline-block;min-width:18px;padding:0 5px;margin-right:6px;border:1px solid #ff5a3a;border-radius:4px;color:#ff5a3a;text-align:center}
.ff-agent{position:fixed;right:18px;bottom:64px;width:64px;height:64px;z-index:60;pointer-events:none;display:none}
.ff-alarm{position:fixed;left:50%;top:12%;transform:translateX(-50%);z-index:66;pointer-events:none;font:700 22px system-ui,sans-serif;color:#ff6a3a;
  text-shadow:0 2px 10px #000;opacity:0;transition:opacity .25s;white-space:nowrap}`;
      document.head.appendChild(css);
      const take = document.createElement("div"); take.className = "ff-take"; take.innerHTML = '<span class="k">E</span>Extinguisher';
      const agent = document.createElement("div"); agent.className = "ff-agent";
      const RL = 2 * Math.PI * 27;
      agent.innerHTML = `<svg viewBox="0 0 64 64"><circle cx="32" cy="32" r="27" fill="rgba(6,9,14,.7)" stroke="#2a323c" stroke-width="5"/>` +
        `<circle class="r" cx="32" cy="32" r="27" fill="none" stroke="#ff5a3a" stroke-width="5" stroke-linecap="round" transform="rotate(-90 32 32)" stroke-dasharray="0 ${RL.toFixed(1)}"/>` +
        `<rect x="25" y="22" width="14" height="28" rx="5" fill="#ff5a3a"/><rect x="28" y="16" width="8" height="7" rx="2" fill="#ff5a3a"/><path d="M36 18 h7 l3 5" stroke="#ff5a3a" stroke-width="2.5" fill="none"/></svg>`;
      const alarm = document.createElement("div"); alarm.className = "ff-alarm";
      document.body.append(take, agent, alarm);
      let alarmT = 0;
      hudEl = {
        update(dt, shown) {
          take.style.display = shown && canTake() ? "block" : "none";
          agent.style.display = shown && F.carrying ? "block" : "none";
          if (F.carrying) {
            const f = Math.max(0, Math.min(1, F.ext.agent_kg / EX.agent_kg));
            agent.querySelector(".r").setAttribute("stroke-dasharray", `${(f * RL).toFixed(1)} ${RL.toFixed(1)}`);
            agent.title = `agent ${F.ext.agent_kg.toFixed(1)} kg of ${EX.agent_kg} kg`;
          }
          alarmT = Math.max(0, alarmT - dt);
          alarm.style.opacity = shown && alarmT > 0 ? "1" : "0";
        },
        /** The alarm's banner for seconds s: a flame and text (the room's name). */
        alarm(text, sec) { alarm.innerHTML = `<svg viewBox="0 0 24 24" width="22" height="22" style="vertical-align:-3px;margin-right:8px"><path d="M12 2c1 4 6 6 6 12a6 6 0 0 1-12 0c0-3 2-5 3-7 0 2 1 3 2 3 0-3-1-5 1-8z" fill="#ff6a3a"/></svg>${text}`; alarmT = sec || 4; },
      };
      return hudEl;
    }

    /** Show or hide everything this file draws (a page's plan views, which clip by deck, hide it). */
    function setVisible(v) { for (const ob of own) ob.layers.set(v ? 0 : 31); handExt.layers.set(v ? 0 : 31); }
    /** Stop drawing the hand's part (leaving the walk). */
    function stow() { ring.visible = false; handExt.visible = false; F.sprayHeld = false; }

    return Object.assign(F, {
      VC, paint, extinguisherGeo, extinguisherModel, handExt, ring, spray, flameMesh, flameU, flames: () => fGeo.instanceCount,
      mainDoor, near, canTake, take, reset, hud, setVisible, tick, showPoint, nozzleAim, emitSpray, stepSpray, aimFrame, stow, updateFlames,
      layerDepth, addSmoke, updateSmoke, walkFog,
    });
  }

  root.FireFight = { version: 1, create };
})(typeof window !== "undefined" ? window : globalThis);
