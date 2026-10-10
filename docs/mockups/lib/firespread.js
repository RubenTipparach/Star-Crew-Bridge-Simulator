/*
 * firespread.js: fire as burning floor cells inside a room, its spread, flashover and aimed suppression.
 *
 * What it owns: the cell model of openspec/changes/fire-spread (design sections 1, 2 and 5), in one place for the
 * mockups (CLAUDE.md 6.1). Every room's floor is cut into square cells on the deck's own x and z; a cell takes the fuel
 * class of the prop that covers its centre, burns toward its peak, preheats its neighbours until they ignite, and is
 * knocked down by an extinguisher's cone or by water mist. The room model (docs/mockups/lib/shipsystems.js) stays the
 * one authority for gases and heat: it hands this file each room's air temperature and oxygen factor every step, takes
 * the sum of the cells' heat release as the room's fire, and asks this file to scale every cell when it caps the room
 * (its oxygen, its ceiling). Nothing here knows about gases, crew or power.
 *
 * Every tuning number is data: atmosphere.json fire.cells (validated here: a missing or unknown key stops the page),
 * fire.extinguisher (the cone) and the room's fuel from fire.fuel_mj_per_m2 through shipsystems (CLAUDE.md 6.5).
 * Deterministic: no random numbers; cells are visited in a fixed order (CLAUDE.md 6.4).
 *
 * Geometry comes from the layout's brushes through shipkit.js (the one reading of the layout) and from the props the
 * rooms are furnished with (data/ships/<id>/crew_rooms.json and the prop sets' manifests, assets/models/<set>/props.json),
 * through placements(), which the mockup also draws its props from, so the fuel and the picture cannot disagree.
 *
 * It is a mockup's instrument, not engine code (CLAUDE.md 4): the engine's sc-core::fire takes the same model when
 * the change is built (fire-spread tasks 2.1). Classic script, no DOM: works in a page (window.FireSpread) and in
 * node (globalThis), after shipkit.js. Units: SI; heat release in W inside, kW at the edges.
 */
(function (root) {
  "use strict";

  // OUT: put out and cooled, fuel left (owner, 2026-10-09: "The cells in firefighting never get fully extinguished"):
  // a knocked-down cell with no burning neighbour for smoulder_s stops smouldering, and only a flame beside it or a
  // flashover lights it again, never the warm air of a room whose fire is out.
  const UNBURNT = 0, BURNING = 1, KNOCKED = 2, BURNT = 3, OUT = 4;

  // ------------------------------------------------------------------ validation (CLAUDE.md 6.5)
  // The keys of fire.cells and their kinds. An unknown key is an error, a missing one too: a misspelt knob that
  // silently does nothing is the worst kind of bug.
  const CELL_KEYS = {
    owned_by: "string", note: "?string", tuned: "?string", size_m: "number", rise_s: "number", ignite_kw: "number", out_below_kw: "number",
    agent_s: "number", smoulder_s: "number", dose_cooling_per_s: "number", neighbour_weights: "object", layer_preheat_k: "pair",
    bare_min_mj_per_m2: "number", aim_height_m: "number", flame_height_m: "pair", flame_on_props_below_m: "number", smoke_layer_ppm: "number",
    classes: "object", props: "object", fixtures: "object",
  };
  const WEIGHT_KEYS = ["edge", "corner", "next_but_one"];
  const CLASS_KEYS = ["fuel_mj_per_m2", "dose_s", "peak_kw_per_m2"];
  const EXT_KEYS = ["agent_kg", "cone_deg", "reach_m", "discharge_s", "hrr_cut_kw_per_s"];
  function fail(path, what) { throw new Error(`atmosphere.json ${path}: ${what} (openspec/changes/fire-spread design 1)`); }
  const finite = (v) => typeof v === "number" && Number.isFinite(v);
  /** Check fire.cells and the extinguisher's cone; throws with the path and field of the first fault. */
  function validate(FI) {
    const C = FI && FI.cells;
    if (!C || typeof C !== "object") fail("fire.cells", "missing");
    for (const k of Object.keys(C)) if (!(k in CELL_KEYS)) fail("fire.cells." + k, "unknown key");
    for (const [k, kind] of Object.entries(CELL_KEYS)) {
      const v = C[k];
      if (kind[0] === "?") { if (v !== undefined && typeof v !== "string") fail("fire.cells." + k, "not text"); continue; }
      if (v === undefined) fail("fire.cells." + k, "missing");
      if (kind === "number" && !(finite(v) && v >= 0)) fail("fire.cells." + k, "not a number of zero or more");
      if (kind === "string" && typeof v !== "string") fail("fire.cells." + k, "not text");
      if (kind === "object" && (typeof v !== "object" || Array.isArray(v))) fail("fire.cells." + k, "not an object");
      if (kind === "pair" && !(Array.isArray(v) && v.length === 2 && v.every(finite) && v[1] > v[0])) fail("fire.cells." + k, "not [low, high]");
    }
    if (!(C.size_m > 0) || !(C.rise_s > 0)) fail("fire.cells", "size_m and rise_s must be above zero");
    for (const k of Object.keys(C.neighbour_weights)) if (WEIGHT_KEYS.indexOf(k) < 0) fail("fire.cells.neighbour_weights." + k, "unknown key");
    for (const k of WEIGHT_KEYS) if (!finite(C.neighbour_weights[k])) fail("fire.cells.neighbour_weights." + k, "missing");
    if (!C.classes.bare) fail("fire.cells.classes.bare", "missing (the class of a cell nothing stands on)");
    for (const [name, c] of Object.entries(C.classes)) {
      for (const k of Object.keys(c)) if (CLASS_KEYS.indexOf(k) < 0) fail(`fire.cells.classes.${name}.${k}`, "unknown key");
      for (const k of CLASS_KEYS) {
        if (name === "bare" && k === "fuel_mj_per_m2") { if (c[k] !== undefined) fail("fire.cells.classes.bare.fuel_mj_per_m2", "bare takes the remainder; give no figure"); continue; }
        if (!(finite(c[k]) && c[k] > 0)) fail(`fire.cells.classes.${name}.${k}`, "missing or not above zero");
      }
    }
    for (const [p, cls] of Object.entries(C.props)) if (!C.classes[cls]) fail("fire.cells.props." + p, `names no class (${cls})`);
    for (const [f, prop] of Object.entries(C.fixtures)) if (!C.props[prop]) fail("fire.cells.fixtures." + f, `names a prop with no class (${prop})`);
    for (const k of EXT_KEYS) if (!(finite(FI.extinguisher && FI.extinguisher[k]) && FI.extinguisher[k] > 0)) fail("fire.extinguisher." + k, "missing or not above zero");
    const V = FI.suppression && FI.suppression.venting;
    if (!V || !finite(V.warning_s) || !finite(V.arm_window_s)) fail("fire.suppression.venting", "warning_s and arm_window_s are needed (design 6a)");
    return true;
  }

  function kit() {
    const K = root.ShipKit;
    if (!K || !(K.version >= 2)) throw new Error("firespread: load shipkit.js (version 2) first");
    return K;
  }

  // ------------------------------------------------------------------ what stands where
  /** A prop's plan footprint, [x, z] corners: its bounds (prop space, +Z its front) turned by yaw and moved to back. */
  function footprint(bounds, back, yawDeg) {
    const th = (yawDeg * Math.PI) / 180, c = Math.cos(th), s = Math.sin(th), b0 = bounds.min, b1 = bounds.max;
    return [[b0[0], b0[2]], [b1[0], b0[2]], [b1[0], b1[2]], [b0[0], b1[2]]].map(([x, z]) => [back[0] + x * c + z * s, back[2] - x * s + z * c]);
  }
  /**
   * The props that stand in the rooms, with their fire class and footprint: [{ room, prop, back, yaw, wall, cls, poly }].
   * furnishings: crew_rooms.json's list; recs: { prop: its props.json record } for every prop placed (a missing one is an
   * error). Also the layout's medical systems, as two beds 1.4 m apart (the deck plan's rule, deck-plan.html
   * systemItems; TODO kit: the placement rules belong in propkit), and the layout's fixtures whose kind fire.cells
   * names (a locker bank by its back on the wall, facing_yaw_deg into the room).
   */
  function placements(L, FI, furnishings, recs) {
    const C = FI.cells, out = [];
    const add = (room, prop, back, yaw) => {
      const rec = recs[prop];
      if (!rec) throw new Error(`firespread: no props.json record for ${prop} (inline its prop set's manifest)`);
      const cls = C.props[prop];
      if (!cls) throw new Error(`firespread: atmosphere.json fire.cells.props has no class for ${prop}`);
      out.push({ room, prop, back, yaw, wall: /wall/.test(String(rec.anchor || "")), cls, poly: footprint(rec.bounds_m, back, yaw) });
    };
    for (const f of furnishings || []) add(f.room, f.prop, f.back_m, f.yaw_deg);
    for (const s of L.systems || []) if (s.kind === "medical") for (const o of [-0.7, 0.7]) add(s.compartment, "med_bed", [s.center_m[0] + o, s.center_m[1], s.center_m[2]], 0);
    for (const f of L.fixtures || []) {
      const prop = C.fixtures[f.kind];
      if (!prop) continue;
      const th = ((f.facing_yaw_deg || 0) * Math.PI) / 180, d = (f.size_m ? f.size_m[0] : 0.6) / 2;
      add(f.compartment, prop, [f.center_m[0] - Math.sin(th) * d, f.center_m[1], f.center_m[2] - Math.cos(th) * d], f.facing_yaw_deg || 0);
    }
    return out;
  }

  // ------------------------------------------------------------------ the cells
  /**
   * The cell model for a ship. L: the layout; FI: atmosphere.json fire; o.items: placements(); o.fuel_j: each
   * compartment's total fuel in J (shipsystems' fire[i].fuel, from fuel_mj_per_m2 x floor), in layout order.
   */
  function create(L, FI, o) {
    validate(FI);
    const K = kit(), C = FI.cells, S = C.size_m, AREA = S * S;
    const CLS = Object.keys(C.classes), clsIdx = {};
    CLS.forEach((c, i) => (clsIdx[c] = i));
    const BARE = clsIdx.bare;
    const comps = L.compartments;
    const items = o.items || [];
    const W = C.neighbour_weights;
    // The neighbours within two cells: (dx, dz, weight). Edge and corner neighbours, and the next but one straight on.
    const OFFS = [];
    for (let dz = -2; dz <= 2; dz++) for (let dx = -2; dx <= 2; dx++) {
      const a = Math.abs(dx), b = Math.abs(dz);
      if (a + b === 1) OFFS.push([dx, dz, W.edge]);
      else if (a === 1 && b === 1) OFFS.push([dx, dz, W.corner]);
      else if ((a === 2 && b === 0) || (a === 0 && b === 2)) OFFS.push([dx, dz, W.next_but_one]);
    }
    const rooms = comps.map((c, ri) => {
      // Cells: a cell belongs to the room when its centre is on one of the room's brush floors (deck x and z, so cells
      // line up across a doorway). A brush whose floor is above another of the room's is a floor of its own.
      const xs = [], zs = [], ys = [], key = new Map();
      for (const br of c.brushes) {
        const px = br.poly.map((p) => p[0]), pz = br.poly.map((p) => p[1]);
        const i0 = Math.floor(Math.min(...px) / S), i1 = Math.ceil(Math.max(...px) / S), j0 = Math.floor(Math.min(...pz) / S), j1 = Math.ceil(Math.max(...pz) / S);
        for (let j = j0; j < j1; j++) for (let i = i0; i < i1; i++) {
          const x = (i + 0.5) * S, z = (j + 0.5) * S, k = i + "," + j + "," + br.y[0].toFixed(2);
          if (key.has(k) || !K.insidePoly(br.poly, x, z)) continue;
          key.set(k, xs.length); xs.push(x); zs.push(z); ys.push(br.y[0]);
        }
      }
      const n = xs.length;
      const r = {
        id: c.id, ri, n, x: Float32Array.from(xs), z: Float32Array.from(zs), y: Float32Array.from(ys),
        cls: new Uint8Array(n), prop: new Int16Array(n).fill(-1), fuel0: new Float64Array(n), fuel: new Float64Array(n), q: new Float64Array(n),
        dose: new Float64Array(n), agent: new Float64Array(n), state: new Uint8Array(n), peak: new Float64Array(n), doseN: new Float64Array(n),
        sprayed: new Uint8Array(n), feed: new Float64Array(n), smoulder: new Float64Array(n), scorch: new Uint8Array(n), nb: [], nbw: [], active: false, hrr: 0, burning: 0,
      };
      // Fuel class: the first prop (in placement order) whose footprint covers the cell's centre.
      const mine = items.map((it, k) => [it, k]).filter(([it]) => it.room === c.id);
      for (let k = 0; k < n; k++) {
        r.cls[k] = BARE;
        for (const [it, ik] of mine) if (K.insidePoly(it.poly, r.x[k], r.z[k])) { r.cls[k] = clsIdx[it.cls]; r.prop[k] = ik; break; }
      }
      // The room's total fuel is kept (damage-control 3): bare cells take what the furnished ones leave, never below the floor.
      let furnished = 0, nBare = 0;
      for (let k = 0; k < n; k++) { if (r.cls[k] === BARE) nBare++; else furnished += C.classes[CLS[r.cls[k]]].fuel_mj_per_m2 * 1e6 * AREA; }
      const total = o.fuel_j ? o.fuel_j[ri] : 0;
      const bare = nBare ? Math.max(C.bare_min_mj_per_m2 * 1e6 * AREA, (total - furnished) / nBare) : 0;
      r.fuelTotal = 0;
      for (let k = 0; k < n; k++) {
        const cl = C.classes[CLS[r.cls[k]]];
        r.fuel0[k] = r.cls[k] === BARE ? bare : cl.fuel_mj_per_m2 * 1e6 * AREA;
        r.fuel[k] = r.fuel0[k]; r.fuelTotal += r.fuel0[k];
        r.peak[k] = cl.peak_kw_per_m2 * 1000 * AREA; r.doseN[k] = cl.dose_s;
      }
      // Neighbours on the same floor within two cells, with their weights.
      for (let k = 0; k < n; k++) {
        const i = Math.round(r.x[k] / S - 0.5), j = Math.round(r.z[k] / S - 0.5), yk = r.y[k].toFixed(2), a = [], w = [];
        for (const [dx, dz, ww] of OFFS) { const m = key.get(i + dx + "," + (j + dz) + "," + yk); if (m !== undefined) { a.push(m); w.push(ww); } }
        r.nb.push(Int32Array.from(a)); r.nbw.push(Float32Array.from(w));
      }
      return r;
    });
    const byId = {};
    rooms.forEach((r) => (byId[r.id] = r));
    const roomOf = (id) => { const r = typeof id === "number" ? rooms[id] : byId[id]; if (!r) throw new Error("firespread: no room " + id); return r; };

    /** The cell of room nearest (x, z) that has fuel left (or any, with anyCell), or -1. */
    function nearestCell(r, x, z, anyCell) {
      let best = -1, bd = Infinity;
      for (let k = 0; k < r.n; k++) {
        if (!anyCell && r.fuel[k] <= 0) continue;
        const d = (r.x[k] - x) ** 2 + (r.z[k] - z) ** 2;
        if (d < bd) { bd = d; best = k; }
      }
      return best;
    }
    /** Which room and cell a deck point is on: { room, k } or null. y (the feet) picks the floor where floors stack. */
    function cellAt(x, z, y) {
      const i = Math.floor(x / S), j = Math.floor(z / S), cx = (i + 0.5) * S, cz = (j + 0.5) * S;
      let hit = null, bd = Infinity;
      for (const r of rooms) for (let k = 0; k < r.n; k++) {
        if (Math.abs(r.x[k] - cx) > 1e-4 || Math.abs(r.z[k] - cz) > 1e-4) continue;
        const d = y === undefined ? 0 : Math.abs(r.y[k] - y);
        if (d < bd) { bd = d; hit = { room: r.id, k }; }
      }
      return hit;
    }
    function igniteCell(r, k, q) { r.scorch[k] = 1; r.state[k] = BURNING; r.q[k] = q; r.dose[k] = 0; r.agent[k] = 0; r.active = true; }
    /**
     * A seed of kw (kW) at (x, z) in a room: the cell there and, if one cell's peak cannot hold it, its nearest
     * neighbours, kw shared among them, so a seed is the same heat release it is in the room model (design 2).
     * Returns the cells lit.
     */
    function seed(roomId, x, z, kw) {
      const r = roomOf(roomId), k0 = nearestCell(r, x, z);
      if (k0 < 0) return [];
      const W_ = kw * 1000, need = Math.max(1, Math.ceil(W_ / r.peak[k0]));
      const order = [];
      for (let k = 0; k < r.n; k++) if (r.fuel[k] > 0) order.push(k);
      order.sort((a, b) => ((r.x[a] - r.x[k0]) ** 2 + (r.z[a] - r.z[k0]) ** 2) - ((r.x[b] - r.x[k0]) ** 2 + (r.z[b] - r.z[k0]) ** 2) || a - b);
      const lit = order.slice(0, need);
      for (const k of lit) igniteCell(r, k, Math.max(r.q[k], W_ / lit.length));
      return lit;
    }
    /** The cells of a room an extinguisher's cone covers: from the nozzle [x, y, z] along dir, within the cone and its reach. */
    function footprintCells(roomId, from, dir) {
      const r = roomOf(roomId), EX = FI.extinguisher, cosH = Math.cos(((EX.cone_deg / 2) * Math.PI) / 180), reach = EX.reach_m;
      const dl = Math.hypot(dir[0], dir[1], dir[2]) || 1, d = [dir[0] / dl, dir[1] / dl, dir[2] / dl], out = [];
      for (let k = 0; k < r.n; k++) {
        const vx = r.x[k] - from[0], vy = r.y[k] + C.aim_height_m - from[1], vz = r.z[k] - from[2], l = Math.hypot(vx, vy, vz);
        if (l > reach || l < 1e-6) continue;
        if ((vx * d[0] + vy * d[1] + vz * d[2]) / l >= cosH) out.push(k);
      }
      return out;
    }
    /** The hot layer's preheat for a room at air temperature t (K): 0 below layer_preheat_k[0], 1 at [1]. */
    const layer = (t) => Math.max(0, Math.min(1, (t - C.layer_preheat_k[0]) / (C.layer_preheat_k[1] - C.layer_preheat_k[0])));

    /**
     * One step of a room (dt s). env: { t_k (the room's air), f_o2 (its oxygen factor, damage-control 3), cuts:
     * [{ cells, w_per_s }] (extinguishers: a cut in W/s shared over the burning cells listed, by q), mist_w_per_s (W/s
     * over every burning cell, by q), wet (mist running: agent on every cell) }. Returns the room's heat release, W.
     */
    function stepRoom(roomId, dt, env) {
      const r = roomOf(roomId), n = r.n;
      if (!r.active && !(env.t_k > C.layer_preheat_k[0]) && !(env.cuts && env.cuts.length) && !env.wet) return (r.hrr = 0);
      const fO2 = env.f_o2;
      // Flashover: at autoignition every cell with fuel ignites at once (the room's caps then hold it).
      if (env.t_k > FI.autoignition_k && fO2 > 0) {
        for (let k = 0; k < n; k++) if (r.fuel[k] > 0 && r.state[k] !== BURNING) igniteCell(r, k, C.ignite_kw * 1000);
      }
      r.sprayed.fill(0);
      // Cuts: an extinguisher's share by q over the burning cells it covers (none: the agent is wasted).
      for (const cut of env.cuts || []) {
        let sum = 0;
        for (const k of cut.cells) { r.sprayed[k] = 1; if (r.fuel[k] > 0 && r.state[k] !== BURNT) r.agent[k] = C.agent_s; if (r.state[k] === BURNING) sum += r.q[k]; }
        if (sum > 0) for (const k of cut.cells) if (r.state[k] === BURNING) r.q[k] -= (cut.w_per_s * dt * r.q[k]) / sum;
      }
      if (env.mist_w_per_s > 0 || env.wet) {
        let sum = 0;
        for (let k = 0; k < n; k++) { if (r.state[k] === BURNING) sum += r.q[k]; if (env.wet && r.fuel[k] > 0 && r.state[k] !== BURNT) r.agent[k] = C.agent_s; }
        if (sum > 0 && env.mist_w_per_s > 0) for (let k = 0; k < n; k++) if (r.state[k] === BURNING) { r.q[k] -= (env.mist_w_per_s * dt * r.q[k]) / sum; r.sprayed[k] = 1; }
      }
      // Feeds from the cells burning at the start of the step (order does not matter). The hot layer preheats only
      // while something in the room burns, and never a cell that was put out (it is wet and cooled): a room whose
      // fire is out stays out unless it flashes over (above).
      let lit = false;
      for (let k = 0; k < n && !lit; k++) if (r.state[k] === BURNING) lit = true;
      const pre = lit ? layer(env.t_k) : 0;
      for (let k = 0; k < n; k++) {
        if (r.state[k] === BURNING || r.state[k] === BURNT) { r.feed[k] = 0; continue; }
        let f = r.state[k] === UNBURNT ? pre : 0;
        const nb = r.nb[k], w = r.nbw[k];
        for (let m = 0; m < nb.length; m++) { const j = nb[m]; if (r.state[j] === BURNING) f += (r.q[j] / r.peak[j]) * w[m]; }
        r.feed[k] = f;
      }
      let hrr = 0, burning = 0, active = false;
      for (let k = 0; k < n; k++) {
        if (r.agent[k] > 0) r.agent[k] = Math.max(0, r.agent[k] - dt);
        const s = r.state[k];
        if (s === BURNING) {
          if (!r.sprayed[k]) r.q[k] += ((r.peak[k] * fO2 - r.q[k]) * dt) / C.rise_s;
          const burn = Math.min(r.fuel[k], Math.max(0, r.q[k]) * dt);
          r.fuel[k] -= burn;
          if (r.fuel[k] <= 0) { r.state[k] = BURNT; r.q[k] = 0; continue; }
          if (r.q[k] < C.out_below_kw * 1000) { r.state[k] = KNOCKED; r.q[k] = 0; r.agent[k] = Math.max(r.agent[k], C.agent_s); active = true; continue; }
          hrr += r.q[k]; burning++; active = true;
        } else if (s !== BURNT && r.fuel[k] > 0) {
          if (r.agent[k] > 0) { active = true; continue; }   // the agent on it: no dose
          if (r.feed[k] > 0) { r.dose[k] += r.feed[k] * dt; r.smoulder[k] = 0; active = true; }
          else if (r.dose[k] > 0) { r.dose[k] *= Math.max(0, 1 - C.dose_cooling_per_s * dt); if (r.dose[k] < 1e-3) r.dose[k] = 0; else active = true; }
          // A knocked-down cell with nothing burning beside it smoulders out: then it is out, charred, and cold.
          if (s === KNOCKED && r.feed[k] <= 0) {
            r.smoulder[k] += dt;
            if (r.smoulder[k] >= C.smoulder_s) { r.state[k] = OUT; r.dose[k] = 0; r.smoulder[k] = 0; continue; }
            active = true;
          }
          if (r.dose[k] >= r.doseN[k] && fO2 > 0) { igniteCell(r, k, C.ignite_kw * 1000); hrr += r.q[k]; burning++; }
        }
      }
      r.active = active; r.hrr = hrr; r.burning = burning;
      return hrr;
    }
    /** Scale every burning cell of a room by factor (the room capped its fire: oxygen, the ceiling, decay). */
    function scale(roomId, factor) {
      const r = roomOf(roomId);
      let hrr = 0;
      for (let k = 0; k < r.n; k++) if (r.state[k] === BURNING) {
        r.q[k] *= factor;
        if (r.q[k] < C.out_below_kw * 1000) { r.state[k] = KNOCKED; r.q[k] = 0; r.agent[k] = Math.max(r.agent[k], C.agent_s); } else hrr += r.q[k];
      }
      r.hrr = hrr;
      return hrr;
    }
    /** Fuel left in a room, J. */
    function fuelLeft(roomId) { const r = roomOf(roomId); let f = 0; for (let k = 0; k < r.n; k++) f += r.fuel[k]; return f; }
    /**
     * What a fire left in a room (owner, 2026-10-09: charred spots "should represent that room has damaged there"):
     * { charred (cells that ever burned: scorched, however briefly), burnt (cells with no fuel left), area_m2 (charred
     * floor), share (of the room's floor) }. The damage control map reads it.
     */
    function damage(roomId) {
      const r = roomOf(roomId);
      let charred = 0, burnt = 0;
      for (let k = 0; k < r.n; k++) { if (r.scorch[k]) charred++; if (r.state[k] === BURNT) burnt++; }
      return { charred, burnt, area_m2: charred * S * S, share: r.n ? charred / r.n : 0 };
    }
    /** Put a room back to unburnt (a scenario's reset). */
    function reset(roomId) {
      const r = roomOf(roomId);
      r.q.fill(0); r.dose.fill(0); r.agent.fill(0); r.smoulder.fill(0); r.scorch.fill(0); r.state.fill(UNBURNT); r.fuel.set(r.fuel0); r.active = false; r.hrr = 0; r.burning = 0;
    }
    /** The burning cells' heat-release-weighted centre of a room, { x, y, z, hrr }, or null. */
    function centroid(roomId) {
      const r = roomOf(roomId);
      let sx = 0, sy = 0, sz = 0, s = 0;
      for (let k = 0; k < r.n; k++) if (r.state[k] === BURNING) { sx += r.x[k] * r.q[k]; sy += r.y[k] * r.q[k]; sz += r.z[k] * r.q[k]; s += r.q[k]; }
      return s > 0 ? { x: sx / s, y: sy / s, z: sz / s, hrr: s } : null;
    }

    /**
     * The aim of a scripted crew member with an extinguisher (fire-spread design 4): "careful" stands at the near edge
     * and sweeps the burning cell nearest the door it came in by (the competent player); "careless" stands 2 m short of
     * the room's centre and points at it. door: [x, z] of the way in. Returns { from, dir } or null when nothing burns.
     */
    function scriptedAim(roomId, kind, door, opts) {
      const r = roomOf(roomId), stand = (opts && opts.stand_m) || 1.5, hand = (opts && opts.hand_m) || 1.0;
      let tx, tz, ty;
      if (kind === "careless") {
        const c = kit().center(comps[r.ri]);
        tx = c[0]; tz = c[2]; ty = r.n ? r.y[0] : 0;
        const dx = door[0] - tx, dz = door[1] - tz, l = Math.hypot(dx, dz) || 1, from = [tx + (dx / l) * 2.0, ty + hand, tz + (dz / l) * 2.0];
        return { from, dir: [tx - from[0], ty + C.aim_height_m - from[1], tz - from[2]] };
      }
      let best = -1, bd = Infinity;
      for (let k = 0; k < r.n; k++) if (r.state[k] === BURNING) { const d = (r.x[k] - door[0]) ** 2 + (r.z[k] - door[1]) ** 2; if (d < bd) { bd = d; best = k; } }
      if (best < 0) return null;
      tx = r.x[best]; tz = r.z[best]; ty = r.y[best];
      const dx = door[0] - tx, dz = door[1] - tz, l = Math.hypot(dx, dz) || 1, s = Math.min(stand, l);
      const from = [tx + (dx / l) * s, ty + hand, tz + (dz / l) * s];
      return { from, dir: [tx - from[0], ty + C.aim_height_m - from[1], tz - from[2]] };
    }

    return {
      rooms, byId, items, size_m: S, classes: CLS, C, UNBURNT, BURNING, KNOCKED, BURNT, OUT, damage,
      roomOf, cellAt, nearestCell, seed, footprintCells, stepRoom, scale, fuelLeft, reset, centroid, scriptedAim, layer,
      /** A flame's height at heat release q (W) of a cell whose peak is peak (W): flame_height_m from ignition to peak. */
      flameHeight: (q, peak) => C.flame_height_m[0] + (C.flame_height_m[1] - C.flame_height_m[0]) * Math.max(0, Math.min(1, q / peak)),
    };
  }

  // ------------------------------------------------------------------ outbreaks (design 6b)
  const OB_KEYS = { owned_by: "string", note: "?string", first_s: "number", interval_s: "pair", max_burning: "number", points: "array" };
  const OB_POINT_KEYS = ["id", "room", "at_m", "prop", "requires_damaged", "cause"];
  /** Check fire.outbreaks against the layout (rooms) and the placements (a spot by prop needs one); throws on the first fault. */
  function validateOutbreaks(L, FI, items) {
    const O = FI.outbreaks, f = (k, what) => fail("fire.outbreaks" + (k ? "." + k : ""), what + " (design 6b)");
    if (!O || typeof O !== "object") f("", "missing");
    for (const k of Object.keys(O)) if (!(k in OB_KEYS)) f(k, "unknown key");
    for (const [k, kind] of Object.entries(OB_KEYS)) {
      const v = O[k];
      if (kind[0] === "?") { if (v !== undefined && typeof v !== "string") f(k, "not text"); continue; }
      if (v === undefined) f(k, "missing");
      if (kind === "number" && !(finite(v) && v >= 0)) f(k, "not a number of zero or more");
      if (kind === "pair" && !(Array.isArray(v) && v.length === 2 && v.every(finite) && v[0] > 0 && v[1] >= v[0])) f(k, "not [low, high] above zero");
      if (kind === "array" && !(Array.isArray(v) && v.length)) f(k, "not a list of spots");
    }
    const ids = new Set(), rooms = new Set(L.compartments.map((c) => c.id));
    O.points.forEach((p, i) => {
      const at = `points[${i}]`;
      for (const k of Object.keys(p)) if (OB_POINT_KEYS.indexOf(k) < 0) f(`${at}.${k}`, "unknown key");
      if (typeof p.id !== "string" || ids.has(p.id)) f(`${at}.id`, "missing or used twice");
      ids.add(p.id);
      if (!rooms.has(p.room)) f(`${at}.room`, `names no compartment (${p.room})`);
      if ((p.at_m === undefined) === (p.prop === undefined)) f(at, "give at_m or prop, one of them");
      if (p.at_m !== undefined && !(Array.isArray(p.at_m) && p.at_m.length === 2 && p.at_m.every(finite))) f(`${at}.at_m`, "not [x, z]");
      if (p.prop !== undefined && !(items || []).some((it) => it.room === p.room && it.prop === p.prop)) f(`${at}.prop`, `no ${p.prop} is placed in ${p.room}`);
      if (p.requires_damaged !== undefined && typeof p.requires_damaged !== "string") f(`${at}.requires_damaged`, "not a station id");
    });
    return true;
  }
  /** FNV-1a 32 of text: the seed of a purpose (CLAUDE.md 6.4). */
  function fnv(text) { let h = 0x811c9dc5; for (let i = 0; i < text.length; i++) { h ^= text.charCodeAt(i); h = Math.imul(h, 0x01000193) >>> 0; } return h >>> 0; }
  /** mulberry32: a small seeded generator, 0-1. */
  function rngOf(seed) { let a = seed >>> 0; return () => { a = (a + 0x6d2b79f5) >>> 0; let t = a; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; }
  /**
   * When and where fires break out on their own (design 6b). L: the layout; FI: atmosphere.json fire; items:
   * placements(); seed: the session seed. Returns { points, step(t_s, can), force(t_s, can) }: points are the spots
   * resolved to { id, room, x, z, requires_damaged, cause }; step returns the spot that catches at sim time t_s (seconds)
   * or null; force breaks the next one out now, whatever the timer says; start(t_s, id) breaks that one out now. can: { burning(roomId) (an outbreak fire there
   * still burns), damaged(stationId), fuel(point) (its cell can still catch) }. One draw from the generator per interval
   * and per spot, in a fixed order, so the same seed breaks out the same fires (no Math.random).
   */
  function outbreaks(L, FI, items, seed) {
    validateOutbreaks(L, FI, items);
    const O = FI.outbreaks;
    const points = O.points.map((p) => {
      let x, z;
      if (p.at_m) [x, z] = p.at_m;
      else {
        const it = items.find((q) => q.room === p.room && q.prop === p.prop);
        x = it.poly.reduce((s, c) => s + c[0], 0) / it.poly.length; z = it.poly.reduce((s, c) => s + c[1], 0) / it.poly.length;
      }
      return { id: p.id, room: p.room, x, z, requires_damaged: p.requires_damaged || null, cause: p.cause || "" };
    });
    const rand = rngOf(fnv(String(seed) + "|outbreak"));
    const draw = () => O.interval_s[0] + (O.interval_s[1] - O.interval_s[0]) * rand();
    let waitUntil = O.first_s, last = null;
    const active = [];   // spots whose fire still burns
    function eligible(can) {
      return points.filter((p) => p.id !== (last && last.id) && !active.includes(p) && !can.burning(p.room) &&
        (!p.requires_damaged || can.damaged(p.requires_damaged)) && (!can.fuel || can.fuel(p)));
    }
    function catchOne(t, can) {
      const list = eligible(can);
      if (!list.length) return null;
      const p = list[Math.min(list.length - 1, Math.floor(rand() * list.length))];
      last = p; active.push(p);
      waitUntil = active.length >= O.max_burning ? Infinity : t + draw();
      return p;
    }
    function step(t, can) {
      for (let i = active.length - 1; i >= 0; i--) if (!can.burning(active[i].room)) { active.splice(i, 1); waitUntil = Math.min(waitUntil, t + draw()); }
      if (active.length >= O.max_burning || t < waitUntil) return null;
      return catchOne(t, can);
    }
    /** Spot id catches now (a scenario, a shot), through the same bookkeeping as one the timer chose. */
    function start(t, id) {
      const p = points.find((q) => q.id === id);
      if (!p) throw new Error(`firespread: no outbreak spot ${id}`);
      last = p; if (!active.includes(p)) active.push(p);
      waitUntil = active.length >= O.max_burning ? Infinity : t + draw();
      return p;
    }
    return { points, step, start, force: (t, can) => catchOne(t, can), active: () => active.slice(), next_s: () => waitUntil };
  }

  root.FireSpread = { version: 1, create, placements, footprint, validate, validateOutbreaks, outbreaks, UNBURNT, BURNING, KNOCKED, BURNT, OUT };
})(typeof window !== "undefined" ? window : globalThis);
