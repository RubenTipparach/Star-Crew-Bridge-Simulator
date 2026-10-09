/*
 * shipsystems.js: the systems mockup's simulation of power, atmosphere, heat, fire and hits.
 *
 * It implements, in one place, the formulas of three proposed OpenSpec changes so the
 * mockup shows what the designs say and nothing else:
 *   openspec/changes/power-grid      (the power solve, reactor, battery, heat and coolant)
 *   openspec/changes/reactor-cooling (the Cooling group, pump speeds, the radiator pumps, the loop's
 *                                     inventory and makeup, the two legs, the cooling automation, and the
 *                                     coolant parts: eight pipe segments, two tanks and the exchanger with
 *                                     their integrity, leaks, isolation and the bypass; coolantView is the
 *                                     reactor system screen's read-only picture of them, design 6a)
 *   openspec/changes/life-support    (the atmosphere step, plant, pumps, crew effects)
 *   openspec/changes/damage-control  (fire, hit resolution, suppression)
 * Every tuning number comes from the proposed data files data/ships/<id>/power.json,
 * atmosphere.json and damage.json, which the page inlines; this file holds formulas, not
 * numbers (CLAUDE.md 6.5). The tables in the designs were produced by running this file
 * headless in node, so the design, the data and the mockup cannot disagree.
 *
 * It is a mockup's instrument, not engine code (CLAUDE.md section 4): the engine's core
 * crate implements the same formulas in Rust when the changes are built, and its tests
 * pin the same table values.
 *
 * Designed but not implemented here (the designs say so): the magazine's cook-off, door
 * jams, remote control lost with the computer core, crew positions (crew are per
 * compartment, and a hit hurts them by the expected share of the room), damage control
 * teams, the time-to-pressure preview, and of reactor-cooling's coolant parts: a hit's march reaching them (they are
 * damaged by damagePart), the leak's heat and steam into engineering and its scald. The repair-time preview is repairTime() (damage::repair_time's rates, without
 * the walk). The damage map (ship-plan-view 6) reads conduitView, nodeView, breakers and repairTime, which change nothing.
 *
 * Geometry comes from the layout's brushes (starcrew.ship-layout/2: a compartment's air is the
 * union of convex prisms) through shipkit.js, the mockups' one reading of the layout: volumes,
 * floor areas, which compartment a point is in, portal frames. Shipkit must be loaded first:
 * in a page it is inlined before this file; in node, set globalThis.window = globalThis and
 * run shipkit.js, which touches no DOM until a page function is called.
 *
 * Fire on the floor (openspec/changes/fire-spread): when firespread.js is loaded before this file (and opts.cells is
 * not false), a room's fire is its burning floor cells, and the room's heat release is their sum; the room model
 * below stays the one authority for what that heat release does (oxygen, smoke, heat, damage, spread through doors)
 * and caps it (oxygen, ceiling), scaling every cell together. Without firespread.js the room grows on its own
 * t-squared line, as damage-control's table was measured. Extinguishers are aimed cones (useExtinguisher with
 * "careful" or "careless", or a held one from newExtinguisher whose aim the page sets); venting is the captain's,
 * with a warning before the dump opens (fire-spread design 6a), and ventPreview runs the same flow solve on a copy of
 * the gas to say how long the room takes to empty.
 *
 * Classic script, no DOM: works in a page (window.ShipSystems) and in node (globalThis).
 * Units: SI. Pressure in Pa inside, kPa at the edges. Power in W inside, MW at the edges.
 */
(function (root) {
  "use strict";

  const SPECIES = ["o2", "n2", "co2", "smoke"];
  const O2 = 0, N2 = 1, CO2 = 2, SMOKE = 3;
  const SPACE = -1;

  // ------------------------------------------------------------- small helpers

  const clamp = (x, a, b) => (x < a ? a : x > b ? b : x);
  const lerpTable = (table, x) => {
    // table: [[x, y], ...] sorted by x descending (as the data files write them).
    if (x >= table[0][0]) return table[0][1];
    for (let i = 0; i + 1 < table.length; i++) {
      const [x0, y0] = table[i], [x1, y1] = table[i + 1];
      if (x <= x0 && x >= x1) return y0 + ((y1 - y0) * (x - x0)) / (x1 - x0);
    }
    return table[table.length - 1][1];
  };
  /** Seeded, stable random numbers (CLAUDE.md 6.4): splitmix32 over a mixed key. */
  function hash32(...parts) {
    let h = 0x9e3779b9;
    for (const p of parts) {
      const s = String(p);
      for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 0x85ebca6b); h ^= h >>> 13; }
      h = Math.imul(h ^ (h >>> 16), 0xc2b2ae35);
    }
    h ^= h >>> 16;
    return (h >>> 0) / 4294967296;
  }
  /** The ship kit (shipkit.js, version 2): the one reading of the layout's brushes. */
  function kit() {
    const K = root.ShipKit;
    if (!K || !(K.version >= 2)) throw new Error("shipsystems: load shipkit.js (version 2) first; compartments are brushes (starcrew.ship-layout/2)");
    return K;
  }
  /** Volume (m^3) and floor area (m^2) of a compartment: its brushes' air, as the kit measures it. */
  function measure(c) { return kit().measure(c); }
  /** Surface of one brush (a convex prism), m^2: floor and ceiling plus its walls. */
  function brushSurface(b) {
    const K = kit();
    let per = 0;
    for (let i = 0; i < b.poly.length; i++) { const p = b.poly[i], q = b.poly[(i + 1) % b.poly.length]; per += Math.hypot(q[0] - p[0], q[1] - p[1]); }
    return 2 * K.signedArea(b.poly) + per * (b.y[1] - b.y[0]);
  }
  /**
   * The intersection of two convex plan polygons wound positive (Sutherland-Hodgman: a clipped
   * by each edge of b). TODO kit: a plan-polygon intersection belongs in shipkit beside
   * insidePoly and insetPoly.
   */
  function clipConvex(a, b) {
    const K = kit();
    let out = a;
    for (let i = 0; i < b.length && out.length; i++) {
      const p = b[i], n = K.outwardNormal(p, b[(i + 1) % b.length]);
      const side = (v) => (v[0] - p[0]) * n[0] + (v[1] - p[1]) * n[1]; // above zero: outside this edge
      const next = [];
      for (let j = 0; j < out.length; j++) {
        const u = out[j], v = out[(j + 1) % out.length], su = side(u), sv = side(v);
        if (su <= 0) next.push(u);
        if ((su < 0 && sv > 0) || (su > 0 && sv < 0)) { const t = su / (su - sv); next.push([u[0] + (v[0] - u[0]) * t, u[1] + (v[1] - u[1]) * t]); }
      }
      out = next;
    }
    return out;
  }
  /**
   * Area of the faces of brush a and brush b that face each other across a gap of at most gap_m:
   * a floor under a ceiling (over the overlap of their footprints), and two walls whose outward
   * normals are opposite (over the overlap of their lengths and heights). Walls may be angled.
   */
  function facingArea(a, b, gap) {
    const K = kit();
    let area = 0;
    const g1 = b.y[0] - a.y[1], g2 = a.y[0] - b.y[1];
    if ((g1 >= -1e-6 && g1 <= gap) || (g2 >= -1e-6 && g2 <= gap)) {
      const p = clipConvex(a.poly, b.poly);
      if (p.length >= 3) area += Math.abs(K.signedArea(p));
    }
    const yov = Math.min(a.y[1], b.y[1]) - Math.max(a.y[0], b.y[0]);
    if (yov <= 0) return area;
    for (let i = 0; i < a.poly.length; i++) {
      const pa = a.poly[i], qa = a.poly[(i + 1) % a.poly.length], na = K.outwardNormal(pa, qa);
      const len = Math.hypot(qa[0] - pa[0], qa[1] - pa[1]), t = [(qa[0] - pa[0]) / len, (qa[1] - pa[1]) / len];
      for (let j = 0; j < b.poly.length; j++) {
        const pb = b.poly[j], qb = b.poly[(j + 1) % b.poly.length], nb = K.outwardNormal(pb, qb);
        if (na[0] * nb[0] + na[1] * nb[1] > -0.999) continue;
        const sep = (pb[0] - pa[0]) * na[0] + (pb[1] - pa[1]) * na[1];
        if (sep < -1e-6 || sep > gap) continue;
        const u0 = (pb[0] - pa[0]) * t[0] + (pb[1] - pa[1]) * t[1], u1 = (qb[0] - pa[0]) * t[0] + (qb[1] - pa[1]) * t[1];
        area += Math.max(0, Math.min(len, Math.max(u0, u1)) - Math.max(0, Math.min(u0, u1))) * yov;
      }
    }
    return area;
  }
  /** Whether point p (ship metres) is in compartment c's air: inside one of its brushes. */
  function inComp(c, p) { return kit().brushAt(c, p[0], p[1], p[2]) !== null; }
  /** The point of segment ab nearest p. */
  function closestOnSeg(p, a, b) {
    const d = [b[0] - a[0], b[1] - a[1], b[2] - a[2]], l2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
    const t = l2 > 1e-12 ? clamp(((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1] + (p[2] - a[2]) * d[2]) / l2, 0, 1) : 0;
    return [a[0] + d[0] * t, a[1] + d[1] * t, a[2] + d[2] * t];
  }
  function segSegDist(p1, q1, p2, q2) {
    // Closest distance between segments p1q1 and p2q2 (Ericson, Real-Time Collision Detection 5.1.9).
    const d1 = [q1[0] - p1[0], q1[1] - p1[1], q1[2] - p1[2]], d2 = [q2[0] - p2[0], q2[1] - p2[1], q2[2] - p2[2]];
    const r = [p1[0] - p2[0], p1[1] - p2[1], p1[2] - p2[2]];
    const dot = (u, v) => u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
    const a = dot(d1, d1), e = dot(d2, d2), f = dot(d2, r);
    let s, t;
    if (a <= 1e-12 && e <= 1e-12) { s = t = 0; }
    else if (a <= 1e-12) { s = 0; t = clamp(f / e, 0, 1); }
    else {
      const c = dot(d1, r);
      if (e <= 1e-12) { t = 0; s = clamp(-c / a, 0, 1); }
      else {
        const b = dot(d1, d2), den = a * e - b * b;
        s = den !== 0 ? clamp((b * f - c * e) / den, 0, 1) : 0;
        t = (b * s + f) / e;
        if (t < 0) { t = 0; s = clamp(-c / a, 0, 1); } else if (t > 1) { t = 1; s = clamp((b - c) / a, 0, 1); }
      }
    }
    const c1 = [p1[0] + d1[0] * s, p1[1] + d1[1] * s, p1[2] + d1[2] * s], c2 = [p2[0] + d2[0] * t, p2[1] + d2[1] * t, p2[2] + d2[2] * t];
    return Math.hypot(c1[0] - c2[0], c1[1] - c2[1], c1[2] - c2[2]);
  }

  // ======================================================================= create

  /**
   * Build a ship's systems simulation from its layout and the three proposed data files.
   * opts.dt_s overrides the sub-step (default 1 / power.solve.substep_hz = 0.1 s);
   * opts.seed is the session seed for every random choice; opts.health is data/crew/health.json (crew-on-deck 7: a body
 * below its incapacitated threshold loses vitals), opts.fireItems FireSpread.placements, opts.cells false the cell model off.
   */
  function create(L, PW, AT, DM, opts) {
    opts = opts || {};
    const dt = opts.dt_s || 1 / PW.solve.substep_hz;
    const seed = opts.seed == null ? 1 : opts.seed;
    const GAS = AT.gas, R = GAS.gas_constant_j_per_mol_k, CV = GAS.cv_j_per_mol_k, CP = GAS.cp_j_per_mol_k, GAMMA = GAS.gamma;
    const MM = SPECIES.map((s) => GAS.molar_mass_kg_per_mol[s]);
    const R_CRIT = Math.pow(2 / (GAMMA + 1), GAMMA / (GAMMA - 1));
    const PSI_CRIT = Math.sqrt(GAMMA) * Math.pow(2 / (GAMMA + 1), (GAMMA + 1) / (2 * (GAMMA - 1)));
    /** Orifice function: mol/s = Cd A P_up psi(r) / sqrt(M R T_up), r = P_down / P_up. */
    const psi = (r) => (r <= R_CRIT ? PSI_CRIT : Math.sqrt(((2 * GAMMA) / (GAMMA - 1)) * (Math.pow(r, 2 / GAMMA) - Math.pow(r, (GAMMA + 1) / GAMMA))));

    // ------------------------------------------------------------ the graph
    const comps = L.compartments;
    const N = comps.length, DUCT = N, NN = N + 1;
    const idx = { space: SPACE, duct: DUCT };
    comps.forEach((c, i) => (idx[c.id] = i));
    const vol = new Float64Array(NN), floor = new Float64Array(NN), cfit = new Float64Array(NN), extArea = new Float64Array(NN);
    const TH = AT.thermal;
    comps.forEach((c, i) => { const m = measure(c); vol[i] = m.volume_m3; floor[i] = m.floor_m2; });
    vol[DUCT] = AT.graph_additions.duct.volume_m3;
    for (let i = 0; i < NN; i++) cfit[i] = TH.fittings_j_per_k_m3 * vol[i];

    // Adjacency (shared bulkhead area) and exterior area, derived from the layout brushes.
    // The deck compiler will bake these; here they are computed once at start.
    const adj = [];
    for (let i = 0; i < N; i++) {
      let surf = 0, inner = 0;
      for (const b of comps[i].brushes) surf += brushSurface(b);
      for (const b1 of comps[i].brushes) for (const b2 of comps[i].brushes) if (b1 !== b2) inner += facingArea(b1, b2, 1e-4);
      let shared = 0;
      for (let j = 0; j < N; j++) {
        if (j === i) continue;
        let a = 0;
        for (const b1 of comps[i].brushes) for (const b2 of comps[j].brushes) a += facingArea(b1, b2, TH.adjacency_gap_m);
        if (a > 0) { shared += a; if (j > i) adj.push({ a: i, b: j, area: a }); }
      }
      extArea[i] = Math.max(0, surf - inner - shared);
    }

    // Links: layout portals, proposed vents and dump, bay and airlock valves, breaches.
    const PK = AT.portals, GA = AT.graph_additions, BP = AT.bay_pumps, AL = AT.airlock;
    const links = [];
    const linkById = {};
    function addLink(l) {
      l.open = l.open == null ? 0 : l.open; l.target = l.target == null ? l.open : l.target;
      l.cd = PK.discharge_coefficient[l.kind];
      l.move = (PK.move_time_override_s && PK.move_time_override_s[l.id] != null) ? PK.move_time_override_s[l.id] : (PK.move_time_s[l.kind] || 0); l.G = 0; l.F = 0; l.flow = 0;
      l.moveClose = PK.close_time_s && PK.close_time_s[l.kind] != null ? PK.close_time_s[l.kind] : l.move; // closing time where the data gives one apart from opening
      l.mix = ["door", "pressure_door", "hatch", "ladder", "hoist"].indexOf(l.kind) >= 0 && l.b !== SPACE;
      // The height a buoyant exchange stands on: a wall opening's height; a floor opening's square root of area.
      if (l.mix) { const f = l.portal ? kit().portalFrame(l.portal) : null; l.mixH = f && !f.floor ? f.h : Math.sqrt(l.area); }
      links.push(l); linkById[l.id] = l; return l;
    }
    for (const p of L.portals) {
      const a = idx[p.between[0]], b = idx[p.between[1]];
      const o = PK.default_open[p.kind] ? 1 : 0;
      addLink({ id: p.id, kind: p.kind, a, b, area: p.size_m[0] * p.size_m[1], open: o, center: p.center_m, portal: p });
    }
    const ventOf = new Int32Array(N);
    for (let i = 0; i < N; i++) {
      const area = clamp(vol[i] * GA.vent_area_m2_per_m3, GA.vent_area_min_m2, GA.vent_area_max_m2);
      ventOf[i] = links.length;
      addLink({ id: "vent_" + comps[i].id, kind: "vent", a: i, b: DUCT, area, open: 1 });
    }
    addLink({ id: GA.overboard_dump.id, kind: "dump", a: DUCT, b: SPACE, area: GA.overboard_dump.area_m2, open: GA.overboard_dump.open ? 1 : 0 });
    for (const bay of BP.serves) addLink({ id: "valve_" + bay, kind: "valve", a: idx[bay], b: SPACE, area: BP.vent_valve_m2, open: 0 });
    addLink({ id: "valve_airlock_eq", kind: "valve", a: idx[AL.compartment], b: idx[AL.pump_into], area: AL.equalize_valve_m2, open: 0 });

    // Ventilation design flow per compartment (m^3/s).
    const VE = AT.ventilation;
    const achFlow = new Float64Array(N);
    comps.forEach((c, i) => {
      const ach = VE.air_changes_override[c.id] != null ? VE.air_changes_override[c.id] : VE.air_changes_per_hour[c.kind];
      achFlow[i] = (ach * vol[i]) / 3600;
    });

    // ------------------------------------------------------------ gas state
    const n = SPECIES.map(() => new Float64Array(NN));
    const U = new Float64Array(NN);
    const ntot = new Float64Array(NN), T = new Float64Array(NN), P = new Float64Array(NN), MW = new Float64Array(NN);
    const SA = AT.standard_air;
    function fillStandard(i, kpa, tk) {
      const nt = (kpa * 1000 * vol[i]) / (R * tk);
      SPECIES.forEach((s, k) => (n[k][i] = nt * SA.mole_fraction[s]));
      U[i] = (nt * CV + cfit[i]) * tk;
    }
    for (let i = 0; i < NN; i++) fillStandard(i, SA.pressure_kpa, TH.initial_k);
    function derive() {
      for (let i = 0; i < NN; i++) {
        let nt = 0, m = 0;
        for (let k = 0; k < 4; k++) { nt += n[k][i]; m += n[k][i] * MM[k]; }
        ntot[i] = nt; T[i] = U[i] / (nt * CV + cfit[i]);
        P[i] = (nt * R * T[i]) / vol[i];
        MW[i] = nt > 1e-9 ? m / nt : 0.029;
      }
    }
    derive();

    // Stores (reserve bottles and the bay receiver), as moles by species.
    const stores = {};
    for (const s of AT.stores) {
      const st = { def: s, mol: [0, 0, 0, 0] };
      if (s.species) st.mol[SPECIES.indexOf(s.species)] = s.mol;
      else {
        const nt = (s.initial_kpa * 1000 * s.volume_m3) / (R * s.temperature_k);
        SPECIES.forEach((sp, k) => (st.mol[k] = nt * SA.mole_fraction[sp]));
      }
      stores[s.id] = st;
    }
    const storeMol = (st) => st.mol[0] + st.mol[1] + st.mol[2] + st.mol[3];
    const receiverKpa = () => { const st = stores.bay_receiver; return (storeMol(st) * R * st.def.temperature_k) / st.def.volume_m3 / 1000; };

    /** Add gas to node i as a flow from a source at temperature tk (brings cp T per mole). */
    function addGas(i, k, mol, tk) { if (mol <= 0) return; n[k][i] += mol; U[i] += mol * CP * tk; }
    /** Remove mol moles of node i's mixture as a flow (takes cp T per mole); returns per-species moles taken. */
    function takeMix(i, mol) {
      const nt = n[0][i] + n[1][i] + n[2][i] + n[3][i];
      if (nt <= 0 || mol <= 0) return [0, 0, 0, 0];
      mol = Math.min(mol, nt);
      const tk = U[i] / (nt * CV + cfit[i]);
      const out = [0, 0, 0, 0];
      for (let k = 0; k < 4; k++) { out[k] = (n[k][i] * mol) / nt; n[k][i] -= out[k]; }
      U[i] -= mol * CP * tk;
      return out;
    }
    function exchange(a, b, mol) {
      // Equal moles both ways: composition and heat mix, pressure does not change.
      if (mol <= 0) return;
      const ga = takeMix(a, mol), gb = takeMix(b, mol);
      const ta = T[a], tb = T[b];
      for (let k = 0; k < 4; k++) { n[k][b] += ga[k]; n[k][a] += gb[k]; }
      U[b] += mol * CP * ta; U[a] += mol * CP * tb;
    }

    // ------------------------------------------------------------ power data
    const nodes = PW.nodes.map((x) => x.id);
    const loads = PW.loads.map((d) => Object.assign({}, d));
    // Lighting loads, one per panel, from floor area (normal lighting) and one emergency load.
    const LG = PW.lighting;
    for (const panel of Object.keys(LG.panels)) {
      const area = LG.panels[panel].reduce((s, id) => s + floor[idx[id]], 0);
      loads.push({ id: "lights_" + panel, name: "Lighting, " + panel, node: panel, priority: LG.priority, nominal_mw: (area * LG.normal_w_per_m2) / 1e6,
        standby_mw: (area * LG.normal_w_per_m2) / 1e6, setpoint_max: 1.0, min_ratio: 0, lighting: LG.panels[panel], heat: { to: "lights", fraction: 1 } });
    }
    const loadIdx = {}; loads.forEach((l, i) => (loadIdx[l.id] = i));
    const sysPos = {};
    for (const s of L.systems) sysPos[s.id] = s.center_m;
    for (const m of L.mounts) sysPos["mount:" + m.id] = m.center_m;
    for (const l of loads) {
      l.center = l.center_m || (l.mount ? sysPos["mount:" + l.mount] : l.system ? sysPos[l.system] : PW.nodes.find((x) => x.id === l.node).center_m);
      const at = l.mount ? L.mounts.find((m) => m.id === l.mount) : null;
      const sys = l.system ? L.systems.find((s) => s.id === l.system) : null;
      l.compartment = l.heat && l.heat.room ? l.heat.room : sys ? sys.compartment : at ? (L.stations.find((s) => s.mount === at.id) || {}).compartment : null;
    }

    // ------------------------------------------------------------ state
    const st = {
      t: 0, seed,
      alert: "normal",
      reactor: { mode: "auto", state: "running", throttle: PW.reactor.throttle_default * 0.6, target: PW.reactor.throttle_default, P_th: 0, P_e: 0,
        T: 600, integrity: 100, fuel_kg: PW.reactor.fuel_load_kg, timers: { loop: 0, flow: 0, aux: 0 }, scramCause: null, ignition: 0, releaseReserve: false },
      battery: { soc_mj: PW.battery.capacity_mj * PW.battery.initial_soc, out_mw: 0, in_mw: 0, health: 1 },
      loop: { T: PW.coolant.initial_k, flow: 1, rad_mw: 0, in_mw: 0, radiatorHealth: 1, branchOpen: {},
        // reactor-cooling: inventory (kg), the tanks' reserve, the hot leg, the radiator pumps' flow, the
        // exchanger's capability (1 until its damage is modelled) and a leak hook (kg/s) for the pipe segments to come.
        m_kg: PW.coolant.inventory_kg, tanks_kg: PW.coolant.tanks.map((t) => t.capacity_kg), hot_k: PW.coolant.initial_k,
        rad_flow: 1, exchanger: 1, chiller: 1, leak_kg_s: 0, makeup_kg_s: 0,
        // The coolant parts (reactor-cooling design 1-2): leak_kg_s above stays an outside hook (a page's own leak);
        // the segments' and tanks' leaks are computed each step into seg_leak_kg_s and tank_leak_kg_s.
        seg_leak_kg_s: 0, tank_leak_kg_s: 0, legs: { hot: 1, cold: 1 } },
      // Each coolant part's integrity (percent) and, for a pipe segment, whether its valves isolate it.
      coolantParts: {},
      // The engineer's hand on the loop (reactor-cooling design 5): AUTO, or MANUAL with the chiller's share and the
      // makeup valve set by hand (the pumps' speeds are their loads' setpoints either way).
      cooling: { mode: "auto", chiller: 1, makeup: 0, timer: 0 },
      groupBreaker: {}, // group id -> "closed" | "open" | "locked" (a group with feed_breaker, power-grid 5)
      setpoint: new Float64Array(loads.length).fill(1),
      priority: Int8Array.from(loads.map((l) => l.priority)),
      activity: {},
      integrity: new Float64Array(loads.length).fill(100),
      breakerOpen: new Uint8Array(loads.length),
      tieClosed: {}, genClosed: {}, conduit: {}, nodeHealth: {},
      demand: new Float64Array(loads.length), want: new Float64Array(loads.length), alloc: new Float64Array(loads.length), dropped: new Uint8Array(loads.length),
      flows: {}, rxOut: 0, batOut: 0, charge: 0,
      thermal: {}, // per load id: { T }
      fire: [], hull: {}, breaches: [], crew: [], log: [],
      plant: { o2_mol_s: 0, o2_want_mw: 0, scrub_mol_s: 0, tc_mw: 0, heater_mw: 0, makeup_mol_s: 0, water_kg: 0 },
      bay: { target: null, mode: "idle", pump_mw_want: 0, moved_mol: 0, energy_mj: 0, lost_mol: 0, t0: 0 },
      airlock: { mode: "idle", pump_mw_want: 0 },
      lostOverboard: 0,
      makeupOn: true,
      damperAuto: true,
      damperForced: {},
      pdoorAuto: true,
      batteryBreaker: true,
      trip: new Float64Array(N), repress: new Uint8Array(N), hold: new Uint8Array(N), fall: new Float64Array(NN), pHist: new Float64Array(NN),
    };
    for (const g of PW.generators) st.genClosed[g.id] = true;
    for (const t of PW.ties) st.tieClosed[t.id] = !!t.closed;
    // cut: where a hit severed the conduit, [x, y, z] on its path (null while whole), for the damage map's break marker.
    for (const k of PW.conduits) st.conduit[k.id] = { health: 1, severed: false, breaker: true, cut: null };
    for (const l of loads) if (l.heat && l.heat.to === "node") st.thermal[l.id] = { T: PW.coolant.initial_k + 5 };
    for (const l of loads) st.loop.branchOpen[l.id] = true;
    // The coolant parts (reactor-cooling design 1): a missing block is an error, not a silent zero (CLAUDE.md 6.5).
    if (!Array.isArray(PW.coolant.segments) || !PW.coolant.exchanger || !Number.isFinite(PW.coolant.isolated_leg_flow))
      throw new Error("power.json coolant.segments, exchanger and isolated_leg_flow: missing (reactor-cooling design 1-2)");
    if (!DM.coolant || !Number.isFinite(DM.coolant.leak_below_pct) || !Number.isFinite(DM.coolant.leak_kg_s_at_zero))
      throw new Error("damage.json coolant.leak_below_pct and leak_kg_s_at_zero: missing (reactor-cooling design 2)");
    for (const sg of PW.coolant.segments) st.coolantParts[sg.id] = { integrity: 100, isolated: false };
    for (const t of PW.coolant.tanks) st.coolantParts[t.id] = { integrity: 100, isolated: false };
    st.coolantParts[PW.coolant.exchanger.id] = { integrity: 100, isolated: false };
    st.reactor.T = PW.coolant.initial_k + 250;
    const FI = AT.fire;
    // The rates fire-spread design 6a adds; a missing one is an error, not a silent zero (CLAUDE.md 6.5).
    for (const [blk, keys] of [["hypoxia", ["harm_below_po2_kpa", "harm_full_po2_kpa", "harm_full_hp_per_s"]], ["heat", ["cold_harm_below_k", "cold_harm_hp_per_s_per_k"]]])
      for (const k of keys) if (!Number.isFinite(AT.crew_effects[blk][k])) throw new Error(`atmosphere.json crew_effects.${blk}.${k}: missing (openspec/changes/fire-spread design 6a)`);
    if (!FI.suppression.venting || !Number.isFinite(FI.suppression.venting.warning_s)) throw new Error("atmosphere.json fire.suppression.venting.warning_s: missing (fire-spread design 6a)");
    for (let i = 0; i < N; i++) {
      const fm2 = FI.fuel_mj_per_m2[comps[i].id] != null ? FI.fuel_mj_per_m2[comps[i].id] : FI.fuel_mj_per_m2_default;
      st.fire.push({ hrr: 0, fuel: fm2 * floor[i] * 1e6, ext: 0, mist: 0, mistLeft: FI.suppression.water_mist.discharges, inert: 0, prevP: P[i], out: 0 });
    }
    // The cell model (fire-spread design 1-2), when firespread.js is loaded: opts.fireItems are FireSpread.placements
    // (what stands where, and so where the fuel is); without them every cell is bare deck.
    const FS = root.FireSpread && opts.cells !== false && FI.cells ? root.FireSpread.create(L, FI, { items: opts.fireItems || [], fuel_j: st.fire.map((f) => f.fuel) }) : null;
    const EX = FI.extinguisher, VENT = FI.suppression.venting;
    st.extinguishers = [];   // { comp, aim ("careful" | "careless" | { from, dir }), on, agent_kg, n, door, held }
    st.vent = null;          // { comp, phase: "warning" | "venting", warn }
    function setActivity(a) { Object.assign(st.activity, a); }
    setActivity({ drive: 0.3, dampers: 0.2, rcs: 0.1, shield_regen: 0, turret_dorsal: 0, turret_ventral: 0, turret_port: 0, turret_stbd: 0,
      sensors_active: 0, comms: 0.2, hoist: 0, cradle_p: 0, cradle_s: 0, pad: 0 });

    // ======================================================= POWER SOLVE
    // The one power-flow solve (CLAUDE.md 6.1): every delivered MW and every console
    // preview comes from solveGrid(). Nodes: S (source), RX (reactor), BAT, then power nodes.
    const S = 0, RX = 1, BAT = 2, NOFF = 3;
    const pIdx = {}; nodes.forEach((id, i) => (pIdx[id] = NOFF + i));
    const GN = NOFF + nodes.length;
    const MWW = 1e6, EPS = PW.solve.epsilon_mw;

    /** Snapshot of edge capacities from the current state (MW). */
    function gridSnapshot() {
      const edges = [];
      const adjL = Array.from({ length: GN }, () => []);
      const add = (id, u, v, c, directed, kind) => { const e = { id, u, v, c: Math.max(0, c), f: 0, directed, kind }; adjL[u].push([edges.length, 1]); adjL[v].push([edges.length, -1]); edges.push(e); return e; };
      const rx = st.reactor;
      const rxAvail = rx.state === "running" ? rx.P_th * PW.reactor.conversion_efficiency / MWW : 0;
      const B = PW.battery;
      // The restart reserve is spent only by priority 0 (reactor auxiliaries, coolant pumps,
      // emergency lighting) unless engineering releases it.
      const reserve = rx.releaseReserve ? 0 : B.restart_reserve_mj;
      const lim = (e) => Math.max(0, Math.min(B.discharge_max_mw * st.battery.health, (e * B.discharge_efficiency) / dt));
      const batAvail = lim(st.battery.soc_mj - reserve), batAvailAll = lim(st.battery.soc_mj);
      const nh = (id) => (st.nodeHealth[id] == null ? 1 : st.nodeHealth[id]);
      const eRx = add("reactor", S, RX, rxAvail, true, "source");
      const eBat = add("battery", S, BAT, 0, true, "source");
      add("battery_out", BAT, pIdx[B.node], st.batteryBreaker ? 1e9 : 0, true, "battery");
      for (const g of PW.generators) add(g.id, RX, pIdx[g.node], st.genClosed[g.id] && nh(g.node) > 0 ? g.capacity_mw : 0, true, "generator");
      for (const t of PW.ties) add(t.id, pIdx[t.between[0]], pIdx[t.between[1]], st.tieClosed[t.id] && nh(t.between[0]) > 0 && nh(t.between[1]) > 0 ? t.capacity_mw : 0, false, "tie");
      for (const k of PW.conduits) {
        const s = st.conduit[k.id];
        const ok = !s.severed && s.breaker && nh(k.between[0]) > 0 && nh(k.between[1]) > 0;
        add(k.id, pIdx[k.between[0]], pIdx[k.between[1]], ok ? k.capacity_mw * s.health : 0, false, "conduit");
      }
      if (nh(B.node) <= 0) edges[2].c = 0;
      const chargeWant = Math.max(0, Math.min(B.charge_max_mw * st.battery.health, (B.capacity_mj - st.battery.soc_mj) / (dt * B.charge_efficiency)));
      return { edges, adjL, eRx, eBat, batAvail, batAvailAll, chargeWant };
    }

    function residual(e, dir) { return dir > 0 ? e.c - e.f : e.directed ? e.f : e.c + e.f; }
    const prevE = new Int32Array(GN), prevD = new Int8Array(GN), seen = new Uint8Array(GN), queue = new Int32Array(GN);
    /** Push up to amount MW from S to node t along shortest residual paths (BFS, stable order). */
    function pushTo(grid, t, amount) {
      let got = 0;
      for (let iter = 0; iter < 16 && amount - got > EPS; iter++) {
        seen.fill(0); let qh = 0, qt = 0; queue[qt++] = S; seen[S] = 1;
        while (qh < qt && !seen[t]) {
          const u = queue[qh++];
          for (const [ei, dir] of grid.adjL[u]) {
            const e = grid.edges[ei]; const v = dir > 0 ? e.v : e.u;
            if (seen[v] || residual(e, dir) <= EPS) continue;
            seen[v] = 1; prevE[v] = ei; prevD[v] = dir; queue[qt++] = v;
          }
        }
        if (!seen[t]) break;
        let b = amount - got;
        for (let v = t; v !== S;) { const e = grid.edges[prevE[v]]; b = Math.min(b, residual(e, prevD[v])); v = prevD[v] > 0 ? e.u : e.v; }
        for (let v = t; v !== S;) { const e = grid.edges[prevE[v]]; e.f += prevD[v] * b; v = prevD[v] > 0 ? e.u : e.v; }
        got += b;
      }
      return got;
    }

    /**
     * Allocate demand (MW per load) over a grid snapshot: priority classes in order; within a
     * class the reactor first, then the battery; within a phase a proportional share of what the
     * sources can still give, routed by augmenting paths; loads cut off by their path keep what
     * they got and the rest is offered again (PW.solve.quanta_rounds rounds).
     */
    function solveGrid(grid, demand) {
      const alloc = new Float64Array(loads.length);
      // Passes: the loads that may spend the restart reserve (emergency lighting always, the
      // reactor auxiliaries while igniting) come first, then priority classes 0 to 3.
      const passes = [{ p: 0, reserve: true }, { p: 0, reserve: false }, { p: 1 }, { p: 2 }, { p: 3 }];
      for (const pass of passes) {
        const p = pass.p;
        const members = [];
        for (let i = 0; i < loads.length; i++) if (st.priority[i] === p && demand[i] > EPS && !!pass.reserve === mayUseReserve(i)) members.push(i);
        if (!members.length) continue;
        for (const phase of [0, 1]) {
          grid.eBat.c = phase ? Math.max(grid.eBat.f, pass.reserve ? grid.batAvailAll : grid.batAvail) : Math.min(grid.eBat.c, grid.eBat.f);
          const blocked = new Uint8Array(loads.length);
          for (let round = 0; round < PW.solve.quanta_rounds; round++) {
            let totalRem = 0;
            for (const i of members) if (!blocked[i]) totalRem += demand[i] - alloc[i];
            const srcRem = (grid.eRx.c - grid.eRx.f) + (grid.eBat.c - grid.eBat.f);
            if (totalRem <= EPS || srcRem <= EPS) break;
            const lam = Math.min(1, srcRem / totalRem);
            let short = false;
            for (const i of members) {
              if (blocked[i]) continue;
              const want = lam * (demand[i] - alloc[i]);
              if (want <= EPS) continue;
              const got = pushTo(grid, pIdx[loads[i].node], want);
              alloc[i] += got;
              if (got < want - EPS) { blocked[i] = 1; short = true; }
            }
            if (!short && lam >= 1) break;
          }
        }
      }
      // Battery charge: reactor surplus only, never while the battery is discharging.
      grid.eBat.c = grid.eBat.f;
      let charge = 0;
      if (grid.eBat.f <= EPS && grid.chargeWant > EPS) charge = pushTo(grid, pIdx[PW.battery.node], grid.chargeWant);
      return { alloc, rxOut: grid.eRx.f, batOut: grid.eBat.f, charge, edges: grid.edges };
    }

    function mayUseReserve(i) {
      const id = loads[i].id;
      // The loop's pumps (core and radiator) may spend it while the reactor is down, as long as they stay vital.
      return id === "emergency_lighting" || (id.startsWith("reactor_aux") && st.reactor.state === "igniting") ||
        ((PW.coolant.pumps.indexOf(id) >= 0 || PW.coolant.radiator_pumps.indexOf(id) >= 0) && st.reactor.state !== "running" && st.priority[i] === 0);
    }
    /** Demand per load from setpoints, activity, damage and breakers (MW). */
    function computeDemand(setpoints) {
      const d = new Float64Array(loads.length);
      for (let i = 0; i < loads.length; i++) {
        const l = loads[i];
        const sp = Math.min(setpoints[i], l.setpoint_max);
        if (sp <= 0 || st.breakerOpen[i] || st.nodeHealth[l.node] === 0) continue;
        const cap = capability(i);
        if (cap <= 0) continue;
        let x;
        if (l.activity === "o2_output") x = Math.min(st.plant.o2_want_mw, l.nominal_mw * sp);
        else if (l.activity === "heater") x = Math.min(l.standby_mw + st.plant.heater_mw, l.nominal_mw * sp);
        else if (l.activity === "bay_pumps") x = Math.min(st.bay.pump_mw_want, l.nominal_mw * sp);
        else if (l.activity === "airlock_pump") x = Math.min(st.airlock.pump_mw_want, l.nominal_mw * sp);
        else if (l.id.startsWith("reactor_aux") && st.reactor.state === "igniting") x = PW.reactor.restart.ignition_mw / 2;
        else if (l.id.startsWith("reactor_aux") && st.reactor.state === "scrammed") x = l.standby_mw;
        else if (l.power_exponent) x = l.nominal_mw * Math.pow(sp, l.power_exponent); // a pump: its setpoint is its speed
        else {
          const a = l.activity ? (st.activity[l.activity] || 0) : 1;
          x = l.standby_mw + Math.max(0, l.nominal_mw * sp - l.standby_mw) * a;
        }
        d[i] = x * cap;
      }
      return d;
    }
    /** What a system can do at its integrity (damage-control's states): 1, integrity / nominal, or 0. */
    function capability(i) { return capOf(st.integrity[i]); }
    /** The capability at integrity g (percent): the one rule for loads, coolant parts and anything else with integrity. */
    function capOf(g) {
      const SY = DM.systems;
      if (g < SY.disabled_below_pct) return 0;
      if (g < SY.nominal_from_pct) return g / SY.nominal_from_pct;
      return 1;
    }
    function damageState(g) {
      const SY = DM.systems;
      return g <= 0 ? "destroyed" : g < SY.disabled_below_pct ? "disabled" : g < SY.nominal_from_pct ? "damaged" : "nominal";
    }
    /** Full solve with drop-out: a load below its minimum ratio switches off and the rest re-solve. */
    function solveWithDropout(setpoints) {
      const demand = computeDemand(setpoints);
      const want = Float64Array.from(demand);
      let res = solveGrid(gridSnapshot(), demand);
      const dropped = new Uint8Array(loads.length);
      for (let pass = 0; pass < PW.solve.dropout_resolves; pass++) {
        let any = false;
        for (let i = 0; i < loads.length; i++) {
          if (demand[i] > EPS && loads[i].min_ratio > 0 && res.alloc[i] < loads[i].min_ratio * demand[i] - EPS) { dropped[i] = 1; demand[i] = 0; any = true; }
        }
        if (!any) break;
        res = solveGrid(gridSnapshot(), demand);
      }
      res.demand = demand; res.want = want; res.dropped = dropped;
      return res;
    }
    /** Console preview (preview = resolver): what a load would get at a hypothetical setpoint. */
    /**
     * Console preview for a load group (engineering's allocation row): delivered MW of every
     * load in the group, and the whole ship's totals, if the group's setpoint were sp.
     */
    function previewGroup(groupId, sp) {
      const g = PW.groups.find((x) => x.id === groupId);
      const s = Float64Array.from(st.setpoint);
      for (const id of g.loads) s[loadIdx[id]] = sp;
      const r = solveWithDropout(s);
      let want = 0, got = 0;
      for (const id of g.loads) { want += r.want[loadIdx[id]]; got += r.alloc[loadIdx[id]]; }
      return { want_mw: want, alloc_mw: got, battery_mw: r.batOut, reactor_mw: r.rxOut };
    }
    function previewSetpoint(loadId, sp) {
      const i = loadIdx[loadId];
      const s = Float64Array.from(st.setpoint); s[i] = sp;
      const r = solveWithDropout(s);
      return { demand_mw: r.want[i], alloc_mw: r.alloc[i], dropped: !!r.dropped[i], battery_mw: r.batOut, reactor_mw: r.rxOut };
    }

    // ======================================================= STEP: power and heat
    function stepPower() {
      const rx = st.reactor, RC = PW.reactor;
      // Reactor control: automation follows the load the grid can take from the reactor (a
      // probe solve with the reactor at its rated output and the battery held back); a player
      // sets the throttle by hand, and power the grid cannot take heats the blanket.
      const ratedE = RC.thermal_rated_mw * RC.conversion_efficiency * (rx.integrity / 100);
      if (rx.mode === "auto" && rx.state === "running") {
        const g0 = gridSnapshot();
        g0.eRx.c = ratedE; g0.batAvail = 0; g0.batAvailAll = 0;
        const probe = solveGrid(g0, computeDemand(st.setpoint));
        rx.target = clamp((probe.rxOut + probe.charge) / Math.max(1e-6, ratedE), RC.throttle_min, 1.0);
      }
      if (rx.state === "running") {
        const d = rx.target - rx.throttle;
        rx.throttle += d > 0 ? Math.min(d, RC.ramp_up_per_s * dt) : Math.max(d, -RC.ramp_down_per_s * dt);
        rx.throttle = clamp(rx.throttle, RC.throttle_min, RC.throttle_max);
        rx.P_th = rx.throttle * RC.thermal_rated_mw * MWW * (rx.integrity / 100);
      } else rx.P_th = 0;
      const res = solveWithDropout(st.setpoint);
      st.demand = res.demand; st.want = res.want; st.alloc = res.alloc; st.dropped = res.dropped;
      st.rxOut = res.rxOut; st.batOut = res.batOut; st.charge = res.charge;
      st.flows = {}; for (const e of res.edges) st.flows[e.id] = e.f;
      rx.P_e = res.rxOut * MWW;
      // Battery energy.
      const B = PW.battery, b = st.battery;
      b.soc_mj = clamp(b.soc_mj - (res.batOut * dt) / B.discharge_efficiency + res.charge * dt * B.charge_efficiency, 0, B.capacity_mj);
      b.out_mw = res.batOut; b.in_mw = res.charge;
      st.batteryLossW = (res.batOut * (1 / B.discharge_efficiency - 1) + res.charge * (1 - B.charge_efficiency)) * MWW;
      // Fuel.
      rx.fuel_kg = Math.max(0, rx.fuel_kg - (rx.P_th * dt) / (RC.fuel_energy_mj_per_kg * MWW));
      if (rx.state === "running" && rx.throttle > 1) rx.integrity = Math.max(0, rx.integrity - (RC.overdrive_wear_pct_per_min_at_max / 60) * dt * ((rx.throttle - 1) / (RC.throttle_max - 1)));
      // Ignition sequence after a reset.
      if (rx.state === "igniting") {
        rx.ignition += dt * ignitionRate();
        if (rx.ignition >= RC.restart.ignition_s) { rx.state = "running"; rx.throttle = RC.restart.start_throttle; log("Reactor ignited; throttle ramps from " + (RC.restart.start_throttle * 100).toFixed(0) + "%"); }
      }
    }
    /**
     * The reactor auxiliaries are two full trains, port and starboard: the reactor keeps running
     * while either train has its supply. Returns the better train's supply / need (a dead
     * switchboard counts as none); ignitionRate() is the pace both trains together allow.
     */
    function auxNeed(l) { return st.reactor.state === "igniting" ? PW.reactor.restart.ignition_mw / 2 : st.reactor.state === "running" ? l.nominal_mw : l.standby_mw; }
    function auxRatio() {
      let best = 0;
      for (const id of ["reactor_aux_p", "reactor_aux_s"]) { const i = loadIdx[id]; best = Math.max(best, st.alloc[i] / Math.max(1e-9, auxNeed(loads[i]))); }
      return Math.min(1, best);
    }
    function ignitionRate() {
      let a = 0; for (const id of ["reactor_aux_p", "reactor_aux_s"]) a += st.alloc[loadIdx[id]];
      return clamp(a / PW.reactor.restart.ignition_mw, 0, 1);
    }
    // ======================================================= the cooling loop (reactor-cooling)
    /**
     * A speed-law load's speed (0 to its setpoint_max): the speed its delivered power allows, power = nominal x
     * capability x speed^exponent (computeDemand scales a damaged load's demand by its capability, so a damaged pump fed
     * in full turns at its setpoint and moves capability x its flow; the capability is not counted twice).
     */
    function pumpSpeed(i) {
      const l = loads[i], cap = capability(i);
      if (!(l.nominal_mw > 0) || cap <= 0) return 0;
      return Math.pow(Math.max(0, st.alloc[i] / (l.nominal_mw * cap)), 1 / (l.power_exponent || 1));
    }
    /** A set of pumps' flow: speed x capability x each one's share, summed (reactor-cooling design 2). */
    function pumpsFlow(ids, share) { let f = 0; for (const id of ids) { const i = loadIdx[id]; f += pumpSpeed(i) * capability(i) * share; } return f; }
    /** The loop's inventory as a fraction of full, and the cavitation it allows (1 from 80%, 0 at 60%). */
    function cavitation() {
      const C = PW.coolant, CV = C.cavitation;
      return clamp((st.loop.m_kg / C.inventory_kg - CV.no_flow_below) / (CV.full_flow_from - CV.no_flow_below), 0, 1);
    }
    /**
     * A leg's factor (reactor-cooling design 2): the worst capability of its segments; with one isolated, the leg runs
     * through the bypass jumper round it at isolated_leg_flow, times the worst of the rest.
     */
    function legFactor(leg) {
      const C = PW.coolant;
      let worst = 1, isolated = false;
      for (const sg of C.segments) {
        if (sg.leg !== leg) continue;
        const p = st.coolantParts[sg.id];
        if (p.isolated) isolated = true; else worst = Math.min(worst, capOf(p.integrity));
      }
      return isolated ? C.isolated_leg_flow * worst : worst;
    }
    /** The loop's flow fraction: the core pumps, cavitation, the worse leg, and natural circulation when they stop. */
    function loopFlow() {
      const C = PW.coolant, loop = st.loop;
      loop.legs.hot = legFactor("hot"); loop.legs.cold = legFactor("cold");
      return Math.max(C.natural_circulation_flow, pumpsFlow(C.pumps, C.flow_per_pump) * cavitation() * Math.min(loop.legs.hot, loop.legs.cold));
    }
    /** A pipe segment's or tank's leak, kg/s (damage.json coolant): none at or above leak_below_pct, or isolated. */
    function partLeak(id) {
      const p = st.coolantParts[id], CL = DM.coolant;
      if (!p || p.isolated || p.integrity >= CL.leak_below_pct || id === PW.coolant.exchanger.id) return 0;
      return CL.leak_kg_s_at_zero * (CL.leak_below_pct - p.integrity) / CL.leak_below_pct;
    }
    /** The heat the loop's flow carries at the design rise, MW (40 MW at full flow): the second needle's scale. */
    const carryMw = (flow) => flow * PW.coolant.design_flow_kg_s * PW.coolant.specific_heat_kj_per_kg_k * PW.coolant.automation.design_rise_k / 1000;
    /**
     * The loop's controls before the power solve: the makeup valve (AUTO opens it below auto_open_below and shuts it at
     * full; MANUAL is the engineer's), the makeup pump's activity, and in AUTO every period_s the pumps' speeds
     * (reactor-cooling design 5): the core pumps at the flow the heat needs at the design rise, never below
     * pump_speed_min, up to their setpoint_max, and the radiator pumps at full. It reckons both core pumps whole, so a
     * lost pump or a low loop is not made up for: a hurt loop under load drifts hot, and the engineer does better.
     */
    function stepCooling() {
      const C = PW.coolant, A = C.automation, cl = st.cooling, loop = st.loop;
      const frac = loop.m_kg / C.inventory_kg, tanks = loop.tanks_kg.reduce((a, b) => a + b, 0);
      if (cl.mode === "auto") {
        if (frac < C.makeup.auto_open_below) cl.autoMakeup = 1; else if (frac >= 1) cl.autoMakeup = 0;
        loop.makeupValve = cl.autoMakeup || 0;
      } else loop.makeupValve = cl.makeup;
      st.activity.makeup = loop.makeupValve > 0 && frac < 1 && tanks > 0 ? 1 : 0;
      if (cl.mode !== "auto") return;
      cl.timer -= dt;
      if (cl.timer > 0) return;
      cl.timer = A.period_s;
      const need = loop.in_mw / carryMw(1);
      for (const id of C.pumps) { const i = loadIdx[id]; st.setpoint[i] = clamp(Math.max(A.pump_speed_min, need), 0, loads[i].setpoint_max); }
      for (const id of C.radiator_pumps) st.setpoint[loadIdx[id]] = 1;
    }
    /** The engineer's hand on the loop: { mode, pumps, radiators (speeds), chiller (share through the exchanger), makeup (open) }. */
    function setCooling(o) {
      const C = PW.coolant, cl = st.cooling;
      if (o.mode === "manual" && cl.mode !== "manual") { cl.chiller = st.loop.chiller; cl.makeup = st.loop.makeupValve || 0; }
      if (o.mode === "auto" && cl.mode !== "auto") cl.timer = 0;
      if (o.mode) cl.mode = o.mode;
      const speed = (ids, v) => { for (const id of ids) { const i = loadIdx[id]; st.setpoint[i] = clamp(v, 0, loads[i].setpoint_max); } };
      if (o.pumps != null) speed(C.pumps, o.pumps);
      if (o.radiators != null) speed(C.radiator_pumps, o.radiators);
      if (o.chiller != null) cl.chiller = clamp(o.chiller, 0, 1);
      if (o.makeup != null) cl.makeup = o.makeup ? 1 : 0;
    }
    /** The loop as the engineering console shows it (preview = resolver: these are the step's own numbers). */
    function coolantReadout() {
      const C = PW.coolant, loop = st.loop;
      const pumps = (ids) => ids.map((id) => { const i = loadIdx[id]; return { id, name: loads[i].name, setpoint: st.setpoint[i], speed: pumpSpeed(i), capability: capability(i), alloc_mw: st.alloc[i] }; });
      return {
        mode: st.cooling.mode, cold_k: loop.T, hot_k: loop.hot_k, flow: loop.flow, flow_kg_s: loop.flow * C.design_flow_kg_s,
        heat_mw: loop.in_mw, carry_mw: carryMw(loop.flow), rad_mw: loop.rad_mw, rad_cap_mw: loop.rad_cap_mw, chiller: loop.chiller,
        rad_flow: loop.rad_flow, exchanger: loop.exchanger, inventory: loop.m_kg / C.inventory_kg, m_kg: loop.m_kg,
        tanks: C.tanks.map((t, k) => ({ id: t.id, name: t.name, kg: loop.tanks_kg[k], frac: loop.tanks_kg[k] / t.capacity_kg })),
        makeup_valve: loop.makeupValve || 0, makeup_kg_s: loop.makeup_kg_s, pumps: pumps(C.pumps), radiator_pumps: pumps(C.radiator_pumps),
        bands: C.bands, limit_k: PW.reactor.scram.loop_over_k,
      };
    }
    /**
     * The whole reactor system as its screen draws it (reactor-cooling design 6a), read from the state the step leaves:
     * nothing here changes the ship, and the screen decides nothing for itself (CLAUDE.md 6.1). It is coolantReadout
     * plus every part: { ...coolantReadout(), cavitation, legs: { hot, cold } (each leg's factor), leak_kg_s (the
     * segments' and the outside hook's), tank_leak_kg_s, breaker ("closed" | "open" | "locked", the Cooling feed),
     * cooling_mw and cooling_want_mw (the group's delivered and wanted power), core: { integrity, state, capability,
     * throttle, p_th_mw, p_e_mw, blanket_k, scram_cause }, parts: [{ id, name, kind ("core" | "segment" | "tank" |
     * "pump" | "radiator_pump" | "makeup" | "exchanger"), leg, index, integrity, state (damageState), capability, leak_kg_s,
     * isolated, flow (the share of design flow through it now), temp_k, speed, setpoint, alloc_mw, want_mw, kg, frac,
     * job (the repairTime job that mends it) }] }.
     */
    function coolantView() {
      const C = PW.coolant, loop = st.loop, rx = st.reactor, base = coolantReadout();
      const part = (o) => Object.assign({ leg: null, index: 0, leak_kg_s: 0, isolated: false, speed: null, setpoint: null, alloc_mw: 0, want_mw: 0, kg: null, frac: null }, o,
        { state: damageState(o.integrity), capability: capOf(o.integrity) });
      const parts = [];
      parts.push(part({ id: PW.reactor.system, name: "Magnetic core", kind: "core", integrity: rx.integrity, flow: loop.flow, temp_k: rx.T, job: { kind: "reactor" } }));
      const legIdx = { hot: 0, cold: 0 };
      for (const sg of C.segments) {
        const p = st.coolantParts[sg.id];
        parts.push(part({ id: sg.id, name: sg.name, kind: "segment", leg: sg.leg, index: ++legIdx[sg.leg], integrity: p.integrity, isolated: p.isolated, leak_kg_s: partLeak(sg.id),
          flow: p.isolated ? 0 : loop.flow, temp_k: sg.leg === "hot" ? loop.hot_k : loop.T, job: { kind: "coolant", id: sg.id } }));
      }
      C.tanks.forEach((t, k) => {
        const p = st.coolantParts[t.id];
        parts.push(part({ id: t.id, name: t.name, kind: "tank", index: k + 1, integrity: p.integrity, leak_kg_s: partLeak(t.id), kg: loop.tanks_kg[k], frac: loop.tanks_kg[k] / t.capacity_kg,
          flow: loop.makeup_kg_s / C.design_flow_kg_s, temp_k: loop.T, job: { kind: "coolant", id: t.id } }));
      });
      const pumpPart = (id, kind, share, k) => {
        const i = loadIdx[id];
        return part({ id, name: loads[i].name, kind, index: k + 1, integrity: st.integrity[i], speed: pumpSpeed(i), setpoint: st.setpoint[i], alloc_mw: st.alloc[i], want_mw: st.want[i],
          flow: pumpSpeed(i) * capOf(st.integrity[i]) * share, temp_k: kind === "pump" ? loop.T : null, job: { kind: "system", id } });
      };
      C.pumps.forEach((id, k) => parts.push(pumpPart(id, "pump", C.flow_per_pump, k)));
      C.radiator_pumps.forEach((id, k) => parts.push(pumpPart(id, "radiator_pump", C.flow_per_radiator_pump, k)));
      const mi = loadIdx[C.makeup.pump];
      parts.push(part({ id: C.makeup.pump, name: loads[mi].name, kind: "makeup", integrity: st.integrity[mi], alloc_mw: st.alloc[mi], want_mw: st.want[mi],
        flow: loop.makeup_kg_s / C.design_flow_kg_s, temp_k: loop.T, job: { kind: "system", id: C.makeup.pump } }));
      const ex = st.coolantParts[C.exchanger.id];
      parts.push(part({ id: C.exchanger.id, name: C.exchanger.name, kind: "exchanger", integrity: ex.integrity, flow: loop.flow * loop.chiller, temp_k: loop.hot_k,
        job: { kind: "coolant", id: C.exchanger.id } }));
      const g = PW.groups.find((x) => x.loads.indexOf(C.pumps[0]) >= 0);
      let mw = 0, want = 0;
      if (g) for (const id of g.loads) { mw += st.alloc[loadIdx[id]]; want += st.want[loadIdx[id]]; }
      return Object.assign(base, {
        cavitation: cavitation(), legs: { hot: loop.legs.hot, cold: loop.legs.cold }, leak_kg_s: loop.leak_kg_s + loop.seg_leak_kg_s, tank_leak_kg_s: loop.tank_leak_kg_s,
        radiator_health: loop.radiatorHealth, breaker: g ? st.groupBreaker[g.id] || "closed" : "closed", group: g ? g.id : null, cooling_mw: mw, cooling_want_mw: want,
        core: { integrity: rx.integrity, state: rx.state, capability: capOf(rx.integrity), throttle: rx.throttle, target: rx.target, p_th_mw: rx.P_th / MWW, p_e_mw: rx.P_e / MWW,
          blanket_k: rx.T, blanket_scram_k: PW.reactor.heat.scram_k, scram_cause: rx.scramCause },
        parts,
      });
    }
    /** The integrity of any part the reactor system screen names: a coolant part, a load (a pump), or the core. */
    function partIntegrity(id) {
      if (st.coolantParts[id]) return st.coolantParts[id].integrity;
      if (id === PW.reactor.system) return st.reactor.integrity;
      if (loadIdx[id] != null) return st.integrity[loadIdx[id]];
      return null;
    }
    function setPartIntegrity(id, g) {
      g = clamp(g, 0, 100);
      if (st.coolantParts[id]) st.coolantParts[id].integrity = g;
      else if (id === PW.reactor.system) st.reactor.integrity = g;
      else if (loadIdx[id] != null) st.integrity[loadIdx[id]] = g;
      else return null;
      return g;
    }
    /** Damage a part to at most pct integrity (a hit, or a scenario button). Returns its integrity, or null for no such part. */
    function damagePart(id, pct) {
      const g = partIntegrity(id);
      if (g == null || !Number.isFinite(pct)) return null;
      const out = setPartIntegrity(id, Math.min(g, pct));
      log("Damage: " + id + " at " + out.toFixed(0) + "%");
      return out;
    }
    /** Repair a part by pts integrity points, or to 100% with no pts (its repair job completed). Returns its integrity. */
    function repairPart(id, pts) {
      const g = partIntegrity(id);
      if (g == null) return null;
      const out = setPartIntegrity(id, pts == null ? 100 : g + pts);
      log("Repaired: " + id + " at " + out.toFixed(0) + "%");
      return out;
    }
    /** Close (on) or open the valves either side of a pipe segment (the pipe game's first step): isolated, it neither leaks nor carries. */
    function isolateSegment(id, on) {
      const p = st.coolantParts[id];
      if (!p || !PW.coolant.segments.some((sg) => sg.id === id)) return false;
      p.isolated = !!on; log((on ? "Isolated " : "Opened ") + id);
      return true;
    }
    /** Open, close or lock open a group's feed breaker (power-grid 5): a locked breaker refuses to close until it is unlocked (opened). */
    function setGroupBreaker(gid, state) {
      const g = PW.groups.find((x) => x.id === gid);
      if (!g || !g.feed_breaker) return "no breaker";
      if (state === "closed" && st.groupBreaker[gid] === "locked") return "locked";
      st.groupBreaker[gid] = state;
      for (const id of g.loads) st.breakerOpen[loadIdx[id]] = state === "closed" ? 0 : 1;
      log(g.name + " breaker " + state);
      return "ok";
    }

    const roomHeat = new Float64Array(NN);
    function stepHeat() {
      roomHeat.fill(0);
      const loop = st.loop, C = PW.coolant, RD = PW.radiators;
      loop.flow = loopFlow();
      loop.rad_flow = pumpsFlow(C.radiator_pumps, C.flow_per_radiator_pump);
      loop.exchanger = capOf(st.coolantParts[C.exchanger.id].integrity);
      let toLoop = 0;
      for (let i = 0; i < loads.length; i++) {
        const l = loads[i], h = l.heat; if (!h) continue;
        const aW = st.alloc[i] * MWW;
        const over = l.nominal_mw > 0 ? Math.max(0, st.alloc[i] / l.nominal_mw - 1) : 0;
        const q = h.fraction * aW * (1 + PW.overdrive.heat_factor * over);
        // Overdrive wears any load (power-grid 8): 2% of integrity a minute at 150%, in proportion to over / 0.5.
        if (over > 0) st.integrity[i] = Math.max(0, st.integrity[i] - dt * (PW.overdrive.wear_pct_per_min_at_150 / 60) * (over / 0.5));
        if (h.to === "loop") toLoop += q;
        else if (h.to === "room") roomHeat[idx[h.room]] += q;
        else if (h.to === "duct") roomHeat[DUCT] += q;
        else if (h.to === "lights") { const tot = l.lighting.reduce((s, id) => s + floor[idx[id]], 0); for (const id of l.lighting) roomHeat[idx[id]] += (aW * floor[idx[id]]) / tot; }
        else if (h.to === "node") {
          const th = st.thermal[l.id];
          const branch = loop.branchOpen[l.id] ? 1 : 0;
          const qLoop = h.loop_kw_per_k * 1000 * loop.flow * branch * (th.T - loop.T);
          const room = l.compartment != null && idx[l.compartment] >= 0 ? idx[l.compartment] : null;
          const qRoom = room != null ? h.room_w_per_k * (th.T - T[room]) : 0;
          th.T += (dt * (q - qLoop - qRoom)) / (h.capacity_mj_per_k * MWW);
          toLoop += qLoop; if (room != null) roomHeat[room] += qRoom;
          // Over-temperature wears the system (damage-control applies the state).
          if (th.T > h.damage_k) st.integrity[i] = Math.max(0, st.integrity[i] - dt * (th.T - h.damage_k) * 0.01);
        }
      }
      // Reactor blanket.
      const rx = st.reactor, RH = PW.reactor.heat;
      const waste = Math.max(0, rx.P_th - rx.P_e);
      const qRxLoop = RH.loop_kw_per_k * 1000 * loop.flow * (rx.T - loop.T);
      const eng = idx[L.systems.find((s) => s.id === PW.reactor.system).compartment];
      const qRxRoom = RH.room_w_per_k * (rx.T - T[eng]);
      rx.T += (dt * (waste - qRxLoop - qRxRoom)) / (RH.capacity_mj_per_k * MWW);
      toLoop += qRxLoop; roomHeat[eng] += qRxRoom;
      // Battery losses: liquid-cooled cells reject to the loop (power.json battery.heat_to).
      if (PW.battery.heat_to === "loop") toLoop += st.batteryLossW || 0;
      else roomHeat[idx[L.systems.find((s) => s.id === PW.battery.system).compartment]] += st.batteryLossW || 0;
      // Radiators.
      const Tb4 = Math.pow(RD.background_k, 4);
      // Capacity by the fourth-power law; the bypass valve holds the loop near its nominal
      // temperature when the heat is low, so the loop does not run cold at cruise. The chiller (reactor-cooling design 2)
      // passes it on through the exchanger's capability and the radiator pumps' flow; in MANUAL the engineer sets the
      // share of the hot leg that goes through the exchanger instead of the bypass valve.
      loop.rad_cap_mw = (RD.rated_mw * loop.radiatorHealth * loop.flow * loop.exchanger * loop.rad_flow * (Math.pow(loop.T, 4) - Tb4)) / (Math.pow(RD.rated_at_k, 4) - Tb4);
      const open = st.cooling.mode === "manual" ? st.cooling.chiller : RD.bypass_band_k > 0 ? clamp((loop.T - (C.nominal_k - RD.bypass_band_k)) / RD.bypass_band_k, 0, 1) : 1;
      loop.chiller = open;
      loop.rad_mw = Math.max(0, loop.rad_cap_mw) * open;
      loop.in_mw = toLoop / MWW;
      // Inventory: the makeup pump feeds the loop from the tanks while its valve is open and the loop is under full
      // (at the share of its demand it gets); the leak hook takes it away (reactor-cooling design 2).
      const mi = loadIdx[C.makeup.pump], tanks = loop.tanks_kg.reduce((a, b) => a + b, 0);
      const mkSupply = st.demand[mi] > 1e-9 ? clamp(st.alloc[mi] / st.demand[mi], 0, 1) : 0;
      const mk = Math.max(0, Math.min(C.makeup.max_kg_s * (loop.makeupValve || 0) * mkSupply * dt, tanks, C.inventory_kg - loop.m_kg));
      if (mk > 0) for (let k = 0; k < loop.tanks_kg.length; k++) loop.tanks_kg[k] -= mk * (loop.tanks_kg[k] / tanks);
      loop.makeup_kg_s = mk / dt;
      // The parts' leaks (reactor-cooling design 2): a segment's from the loop, a tank's from its own reserve.
      let segLeak = 0, tankLeak = 0;
      for (const sg of C.segments) segLeak += partLeak(sg.id);
      C.tanks.forEach((t, k) => { const q = Math.min(loop.tanks_kg[k], partLeak(t.id) * dt); loop.tanks_kg[k] -= q; tankLeak += q / dt; });
      loop.seg_leak_kg_s = Math.min(segLeak, (loop.m_kg + mk) / dt); loop.tank_leak_kg_s = tankLeak;
      loop.m_kg = clamp(loop.m_kg + mk - (loop.leak_kg_s + segLeak) * dt, 0, C.inventory_kg);
      // The loop's heat capacity follows its inventory; the hot leg is the cold leg plus the heat the flow carries.
      const cLoop = C.capacity_mj_per_k * MWW * Math.max(0.05, loop.m_kg / C.inventory_kg);
      loop.T += (dt * (toLoop - loop.rad_mw * MWW)) / cLoop;
      loop.hot_k = loop.T + toLoop / (loop.flow * C.design_flow_kg_s * C.specific_heat_kj_per_kg_k * 1000);
    }
    function checkScram() {
      const rx = st.reactor, SC = PW.reactor.scram;
      if (rx.state !== "running") return;
      const tm = rx.timers;
      tm.loop = st.loop.hot_k > SC.loop_over_k ? tm.loop + dt : 0; // the hot leg (reactor-cooling design 3)
      tm.flow = st.loop.flow < SC.coolant_flow_below && rx.throttle > SC.coolant_check_above_throttle ? tm.flow + dt : 0;
      tm.aux = auxRatio() < SC.aux_supply_below ? tm.aux + dt : 0;
      let cause = null;
      if (tm.loop >= SC.loop_over_hold_s) cause = "coolant hot leg over " + SC.loop_over_k + " K";
      else if (rx.T > PW.reactor.heat.scram_k) cause = "blanket over " + PW.reactor.heat.scram_k + " K";
      else if (tm.flow >= SC.coolant_flow_hold_s) cause = "coolant flow below " + SC.coolant_flow_below * 100 + "%";
      else if (tm.aux >= SC.aux_supply_hold_s) cause = "auxiliaries below " + SC.aux_supply_below * 100 + "% supply";
      else if (rx.integrity < SC.integrity_below_pct) cause = "reactor integrity below " + SC.integrity_below_pct + "%";
      if (cause) scram(cause);
    }
    function scram(cause) {
      const rx = st.reactor; rx.state = "scrammed"; rx.scramCause = cause; rx.throttle = 0; rx.P_th = 0; rx.timers = { loop: 0, flow: 0, aux: 0 };
      log("SCRAM: " + cause);
    }
    /** The hands-on reset at the reactor panel: refused while a cause persists. */
    function scramReset() {
      const rx = st.reactor, RS = PW.reactor.reset;
      if (rx.state !== "scrammed") return "not scrammed";
      if (st.loop.T >= RS.loop_below_k) return "loop too hot (" + st.loop.T.toFixed(0) + " K)";
      if (rx.T >= RS.blanket_below_k) return "blanket too hot (" + rx.T.toFixed(0) + " K)";
      if (rx.integrity < PW.reactor.scram.integrity_below_pct) return "reactor damaged";
      rx.state = "igniting"; rx.ignition = 0; rx.scramCause = null; log("Scram reset; ignition draws " + PW.reactor.restart.ignition_mw + " MW for " + PW.reactor.restart.ignition_s + " s");
      return "ok";
    }

    // ======================================================= STEP: atmosphere
    const order = new Int32Array(NN);
    const K = new Float64Array(NN * NN), rhs = new Float64Array(NN), Pn = new Float64Array(NN);
    /** Solve K x = rhs in place (K symmetric positive definite): dense Cholesky, NN is ~31. */
    function cholSolve() {
      const m = NN;
      for (let j = 0; j < m; j++) {
        let s = K[j * m + j];
        for (let k = 0; k < j; k++) s -= K[j * m + k] * K[j * m + k];
        const d = Math.sqrt(Math.max(s, 1e-300)); K[j * m + j] = d;
        for (let i = j + 1; i < m; i++) {
          let t = K[i * m + j];
          for (let k = 0; k < j; k++) t -= K[i * m + k] * K[j * m + k];
          K[i * m + j] = t / d;
        }
      }
      for (let i = 0; i < m; i++) { let t = rhs[i]; for (let k = 0; k < i; k++) t -= K[i * m + k] * Pn[k]; Pn[i] = t / K[i * m + i]; }
      for (let i = m - 1; i >= 0; i--) { let t = Pn[i]; for (let k = i + 1; k < m; k++) t -= K[k * m + i] * Pn[k]; Pn[i] = t / K[i * m + i]; }
    }
    const inS = SPECIES.map(() => new Float64Array(NN)), inH = new Float64Array(NN);
    function flowSolve() {
      derive();
      const FL = AT.flow;
      // 1. Secant conductance of every open link, from the start-of-step pressures.
      for (const l of links) {
        l.G = 0; l.F = 0;
        const A = l.area * l.open; if (A <= 1e-9) continue;
        const pa = P[l.a], pb = l.b === SPACE ? 0 : P[l.b];
        const up = pa >= pb ? l.a : l.b, pUp = Math.max(pa, pb);
        if (pUp < FL.min_pressure_pa) continue;
        const dpe = Math.max(Math.abs(pa - pb), FL.linear_below_pa);
        const r = Math.max(0, (pUp - dpe) / pUp);
        const ndot = (l.cd * A * pUp * psi(r)) / Math.sqrt(MW[up] * R * T[up]);
        l.G = ndot / dpe; // mol/(s Pa)
      }
      // 2. Implicit pressure solve: (V/RT) P' + dt L P' = n, space held at 0 Pa.
      K.fill(0);
      for (let i = 0; i < NN; i++) { K[i * NN + i] = vol[i] / (R * T[i]); rhs[i] = ntot[i]; }
      for (const l of links) {
        if (l.G <= 0) continue;
        const g = dt * l.G;
        K[l.a * NN + l.a] += g;
        if (l.b !== SPACE) { K[l.b * NN + l.b] += g; K[l.a * NN + l.b] -= g; K[l.b * NN + l.a] -= g; }
      }
      cholSolve();
      for (let i = 0; i < NN; i++) if (Pn[i] < 0) Pn[i] = 0;
      // 3. Fluxes, then upwind transport in order of falling new pressure (exact, positive).
      for (let i = 0; i < NN; i++) { order[i] = i; inH[i] = 0; for (let k = 0; k < 4; k++) inS[k][i] = 0; }
      const ord = Array.from(order).sort((x, y) => Pn[y] - Pn[x] || x - y);
      const outs = Array.from({ length: NN }, () => []);
      for (const l of links) {
        if (l.G <= 0) continue;
        l.F = dt * l.G * (Pn[l.a] - (l.b === SPACE ? 0 : Pn[l.b])); // mol over the step, a -> b positive
        l.flow = l.F / dt;
        if (l.F > 0) outs[l.a].push([l, l.b, l.F]); else if (l.F < 0) outs[l.b].push([l, l.a, -l.F]);
      }
      for (const i of ord) {
        let avail = 0; const av = [0, 0, 0, 0];
        for (let k = 0; k < 4; k++) { av[k] = n[k][i] + inS[k][i]; avail += av[k]; }
        const Uav = U[i] + inH[i];
        let O = 0; for (const o of outs[i]) O += o[2];
        if (avail <= 0) { for (let k = 0; k < 4; k++) n[k][i] = av[k]; U[i] = Uav; continue; }
        const scale = O > avail ? avail / O : 1;
        const Tm = Uav / (avail * CV + cfit[i]);
        for (const [, j, amt0] of outs[i]) {
          const amt = amt0 * scale;
          if (j === SPACE) { st.lostOverboard += amt; continue; }
          for (let k = 0; k < 4; k++) inS[k][j] += (amt * av[k]) / avail;
          inH[j] += amt * CP * Tm;
        }
        const keep = 1 - (O * scale) / avail;
        for (let k = 0; k < 4; k++) n[k][i] = av[k] * keep;
        U[i] = Uav - O * scale * CP * Tm;
      }
      derive();
    }
    function mixing() {
      const FL = AT.flow;
      // Door exchange: a small base rate, plus the buoyant two-way flow a hot room drives through
      // an opening (q = c W H sqrt(g H dT / T)), scaled by the gravity generator's field.
      const g = FL.gravity_m_s2 * gravityG();
      for (const l of links) {
        if (!l.mix || l.open <= 0) continue;
        const A = l.area * l.open;
        const H = l.mixH;
        const dT = Math.abs(T[l.a] - T[l.b]), Tm = 0.5 * (T[l.a] + T[l.b]);
        const q = (FL.mixing_m3_s_per_m2 * A + FL.buoyant_exchange_coefficient * A * Math.sqrt((g * H * dT) / Tm)) * dt;
        const mol = Math.min(q * Math.min(ntot[l.a] / vol[l.a], ntot[l.b] / vol[l.b]), FL.mixing_max_fraction * Math.min(ntot[l.a], ntot[l.b]));
        exchange(l.a, l.b, mol);
      }
      // Fan circulation through open vents: rooms and the duct trade equal moles.
      const fan = fanRatio();
      for (let i = 0; i < N; i++) {
        const v = links[ventOf[i]]; if (v.open <= 0 || fan <= 0) continue;
        const q = achFlow[i] * fan * v.open * dt;
        const mol = Math.min(q * Math.min(ntot[i] / vol[i], ntot[DUCT] / vol[DUCT]), FL.mixing_max_fraction * Math.min(ntot[i], ntot[DUCT]));
        exchange(i, DUCT, mol);
      }
      derive();
    }
    function fanRatio() {
      const i = loadIdx.air_handler, l = loads[i];
      if (st.t === 0) return 1; // before the first power solve the fans are taken as running
      if (st.demand[i] <= 0) return 0;
      const r = st.alloc[i] / st.demand[i];
      if (r < l.min_ratio) return 0;
      return r * Math.min(st.setpoint[i], l.setpoint_max);
    }
    /** The artificial gravity field in g: the generator's delivered fraction of nominal, zero below its minimum. */
    function gravityG() {
      const i = loadIdx.gravity_generator;
      if (st.demand[i] <= 0) return st.t === 0 ? 1 : 0;
      return st.alloc[i] / loads[i].nominal_mw;
    }
    function supplyRatio(id) { const i = loadIdx[id]; return st.demand[i] > 0 ? st.alloc[i] / st.demand[i] : 0; }

    function stepPlant() {
      const PL = AT.plant, D = DUCT;
      derive();
      // O2 generator: holds the supply air's oxygen partial pressure.
      const og = PL.o2_generator, oi = loadIdx.o2_generator;
      const po2 = (P[D] * n[O2][D]) / Math.max(1e-9, ntot[D]) / 1000;
      const wantRate = og.max_mol_s * clamp((og.target_po2_kpa - po2) / og.band_kpa, 0, 1) * Math.min(st.setpoint[oi], loads[oi].setpoint_max);
      st.plant.o2_want_mw = wantRate * og.energy_mj_per_mol;
      const o2Rate = st.demand[oi] > 0 ? (st.alloc[oi] / og.energy_mj_per_mol) : 0;
      addGas(D, O2, o2Rate * dt, T[D]); st.plant.o2_mol_s = o2Rate; st.plant.water_kg += o2Rate * dt * og.water_kg_per_mol;
      // Scrubbers: CO2 and smoke out of the supply air.
      const sc = PL.co2_scrubbers, si = loadIdx.co2_scrubbers;
      const sr = (st.demand[si] > 0 ? st.alloc[si] / loads[si].nominal_mw : 0);
      const thr = sc.throughput_m3_s * sr;
      const co2Take = Math.min(sc.max_mol_s * sr, (sc.co2_efficiency * thr * n[CO2][D]) / vol[D]) * dt;
      const smTake = Math.min(n[SMOKE][D], (sc.smoke_efficiency * thr * n[SMOKE][D] * dt) / vol[D]);
      const tD = T[D];
      n[CO2][D] -= Math.min(co2Take, n[CO2][D]); n[SMOKE][D] -= smTake; U[D] -= (co2Take + smTake) * CP * tD;
      st.plant.scrub_mol_s = co2Take / dt;
      // Make-up from the reserve bottles keeps the supply pressure.
      const mk = PL.makeup; derive();
      let mkRate = 0;
      // Make-up stops if the duct itself is falling below stop_if_duct_below_kpa (a leak the
      // reserves would only feed), except while the board is refilling a sealed compartment.
      const refilling = st.repress.some((x) => x > 0);
      if (st.makeupOn && P[D] / 1000 < mk.start_below_kpa && (P[D] / 1000 > mk.stop_if_duct_below_kpa || refilling)) {
        mkRate = mk.max_mol_s * clamp((mk.target_kpa - P[D] / 1000) / 1.0, 0, 1);
        const rn = stores.reserve_n2, ro = stores.reserve_o2;
        const qn = Math.min(mkRate * dt * SA.mole_fraction.n2, rn.mol[N2]), qo = Math.min(mkRate * dt * SA.mole_fraction.o2, ro.mol[O2]);
        rn.mol[N2] -= qn; ro.mol[O2] -= qo; addGas(D, N2, qn, TH.initial_k); addGas(D, O2, qo, TH.initial_k);
      }
      st.plant.makeup_mol_s = mkRate;
      // Thermal control conditions the supply air (cooling to the hull panels, electric heat).
      derive();
      const tc = PL.thermal_control, ti = loadIdx.thermal_control;
      const tr = st.demand[ti] > 0 ? st.alloc[ti] / st.demand[ti] : 0;
      const Cd = ntot[D] * CV + cfit[D];
      let Q = ((T[D] - tc.supply_setpoint_k) * Cd) / dt; // W to remove (negative: to add)
      Q = clamp(Q, -tc.heating_max_mw * MWW * tr, tc.cooling_max_mw * MWW * tr);
      U[D] -= Q * dt; st.plant.tc_mw = Q / MWW;
      // Heater demand for the next solve: what it would have liked to add this step.
      const Qwant = ((tc.supply_setpoint_k - T[D]) * Cd) / dt;
      st.plant.heater_mw = clamp(Qwant / MWW, 0, tc.heating_max_mw);
    }

    function stepRoomHeat() {
      // Heat from loads, lights, crew; bulkhead conduction; hull loss.
      for (const c of st.crew) if (!c.dead && c.comp != null) roomHeat[c.comp] += c.working ? AT.metabolism.heat_w_work : AT.metabolism.heat_w_rest;
      for (let i = 0; i < NN; i++) U[i] += roomHeat[i] * dt;
      derive();
      for (const e of adj) { const q = TH.bulkhead_u_w_per_m2_k * e.area * (T[e.a] - T[e.b]) * dt; U[e.a] -= q; U[e.b] += q; }
      for (let i = 0; i < N; i++) U[i] -= TH.hull_u_w_per_m2_k * extArea[i] * (T[i] - TH.hull_skin_k) * dt;
      derive();
    }

    function stepPumps() {
      const BP_ = BP, bay = st.bay;
      st.bay.pump_mw_want = 0;
      if (bay.target != null) {
        const i = idx[bay.target];
        const pb = P[i], prc = receiverKpa() * 1000;
        if (bay.mode === "pumpdown") {
          if (pb / 1000 <= BP_.stop_kpa || prc / 1000 >= stores.bay_receiver.def.max_kpa) {
            bay.mode = "ready"; bay.t_ready = st.t - bay.t0; setLink("valve_" + bay.target, 1);
            log("Bay pumps stop at " + (pb / 1000).toFixed(1) + " kPa after " + bay.t_ready.toFixed(0) + " s: launch permitted; the vent valve takes the rest");
          }
          else {
            const S_ = BP_.displacement_m3_s * Math.min(1, st.setpoint[loadIdx.hangar_pumps]);
            const lnr = Math.log(Math.max(1, prc / Math.max(pb, 1)));
            bay.pump_mw_want = BP_.base_mw + (S_ * pb * lnr) / BP_.efficiency / MWW;
            const ratio = supplyRatio("hangar_pumps");
            const molRate = (S_ * ratio * pb) / (R * T[i]);
            const g = takeMix(i, Math.min(molRate * dt, 0.5 * ntot[i]));
            for (let k = 0; k < 4; k++) stores.bay_receiver.mol[k] += g[k];
            bay.moved_mol += g[0] + g[1] + g[2] + g[3];
            bay.energy_mj += (st.alloc[loadIdx.hangar_pumps] * dt);
          }
        } else if (bay.mode === "ready") {
          if (pb < 100) setLink("valve_" + bay.target, 0);
        } else if (bay.mode === "repress") {
          const rc = stores.bay_receiver; const nt = storeMol(rc);
          const room = prc - pb;
          if (room < 1000 || pb / 1000 >= SA.pressure_kpa - 0.5) { bay.mode = "idle"; log(comps[i].name + " repressurized from the receiver to " + (pb / 1000).toFixed(1) + " kPa"); repressurize(bay.target); bay.target = null; }
          else {
            const mol = Math.min(BP_.repress_max_mol_s * dt, nt * 0.5, (room * vol[i]) / (R * T[i]) * 0.5);
            for (let k = 0; k < 4; k++) addGas(i, k, (rc.mol[k] * mol) / nt, rc.def.temperature_k);
            for (let k = 0; k < 4; k++) rc.mol[k] -= (rc.mol[k] * mol) / nt;
          }
        }
      }
      // Airlock pump: airlock air back into the cabin.
      const al = st.airlock; al.pump_mw_want = 0;
      if (al.mode === "out_pump") {
        const i = idx[AL.compartment], j = idx[AL.pump_into];
        if (P[i] / 1000 <= AL.stop_kpa) { al.mode = "out_ready"; setLink(AL.outer, 1); log("Airlock pumped down at t=" + (st.t - al.t0).toFixed(1) + " s; outer door opening"); }
        else {
          const lnr = Math.log(Math.max(1, P[j] / Math.max(P[i], 1)));
          al.pump_mw_want = (AL.pump_m3_s * P[i] * lnr) / AL.efficiency / MWW + 0.005;
          const ratio = supplyRatio("airlock_pump");
          const molRate = (AL.pump_m3_s * ratio * P[i]) / (R * T[i]);
          const tk = T[i];
          const g = takeMix(i, Math.min(molRate * dt, 0.5 * ntot[i]));
          for (let k = 0; k < 4; k++) addGas(j, k, g[k], tk);
        }
      } else if (al.mode === "in_eq") {
        const i = idx[AL.compartment], j = idx[AL.pump_into];
        if (Math.abs(P[j] - P[i]) < 1000) { al.mode = "idle"; setLink("valve_airlock_eq", 0); setLink(AL.inner, 1); log("Airlock equalized at t=" + (st.t - al.t0).toFixed(1) + " s; inner door opening"); }
      }
      derive();
    }

    // ======================================================= FIRE
    function fO2(i) {
      const x = ntot[i] > 0 ? n[O2][i] / ntot[i] : 0;
      return clamp((x - FI.extinct_o2_fraction) / (FI.full_o2_fraction - FI.extinct_o2_fraction), 0, 1) *
        clamp((P[i] / 1000 - FI.extinct_kpa) / (FI.full_kpa - FI.extinct_kpa), 0, 1);
    }
    function stepFire() {
      const alpha = FI.growth_alpha_kw_per_s2 * 1000;
      const MIST = FI.suppression.water_mist, INERT = FI.suppression.inert_gas;
      for (let i = 0; i < N; i++) {
        const f = st.fire[i];
        // Inert gas flooding: replace room gas with nitrogen until the design O2 fraction.
        if (f.inert > 0) {
          // Flood and hold: replace room gas with nitrogen (relief overboard) while O2 is above design.
          const x = n[O2][i] / Math.max(1e-9, ntot[i]);
          if (x > INERT.design_o2_fraction) {
            const k = Math.log(SA.mole_fraction.o2 / INERT.design_o2_fraction) / INERT.time_s;
            const mol = Math.min(ntot[i] * k * dt, stores[INERT.store].mol[N2]);
            const tk = T[i];
            const g = takeMix(i, mol); st.lostOverboard += g[0] + g[1] + g[2] + g[3];
            stores[INERT.store].mol[N2] -= mol; addGas(i, N2, mol, tk);
            st.n2Used = (st.n2Used || 0) + mol;
          }
          f.inert = Math.max(0, f.inert - dt);
          if (f.inert <= 0) { delete st.damperForced[comps[i].id]; log("Inert gas soak ends in " + comps[i].name); }
        }
        if (f.mist > 0) { f.mist -= dt; U[i] -= MIST.cooling_mw * MWW * dt; }
        derive();
        if (FS) { stepFireCells(i); continue; }
        if (f.hrr <= 0) {
          // Spread: hot air with fuel and oxygen ignites (deterministic threshold).
          if (T[i] > FI.autoignition_k && f.fuel > 0 && fO2(i) > 0) { f.hrr = FI.seed_kw * 1000; log("Fire spreads into " + comps[i].name); }
          continue;
        }
        const max = FI.hrr_max_kw_per_m2 * 1000 * floor[i] * fO2(i) * (f.fuel > 0 ? 1 : 0);
        let cut = 0;
        for (const e of st.extinguishers) if (e.comp === i && e.on && e.agent_kg > 1e-6) { cut += EX.hrr_cut_kw_per_s * 1000 * e.n; spend(e); }
        if (f.ext > 0) f.ext -= dt;
        if (f.mist > 0) cut += MIST.hrr_cut_kw_per_s * 1000;
        if (cut > 0) f.hrr -= cut * dt;
        else if (f.hrr < max) f.hrr = Math.min(max, f.hrr + dt * 2 * Math.sqrt(alpha * f.hrr));
        if (f.hrr > max) f.hrr += ((max - f.hrr) * dt) / FI.decay_time_s;
        if (f.hrr < FI.out_below_kw * 1000) { f.hrr = 0; log("Fire out in " + comps[i].name); continue; }
        let o2 = (f.hrr * dt) / (FI.mj_per_mol_o2 * MWW);
        if (o2 > 0.5 * n[O2][i]) { o2 = 0.5 * n[O2][i]; f.hrr = (o2 * FI.mj_per_mol_o2 * MWW) / dt; }
        const x = n[O2][i] / Math.max(1e-9, ntot[i]);
        const y = x < FI.starved_below_o2_fraction ? FI.smoke_mol_per_mol_o2_starved : FI.smoke_mol_per_mol_o2_ventilated;
        n[O2][i] -= o2; n[CO2][i] += FI.co2_mol_per_mol_o2 * o2; n[SMOKE][i] += y * o2;
        U[i] += f.hrr * dt; f.fuel -= f.hrr * dt;
      }
      st.extinguishers = st.extinguishers.filter((e) => e.held || e.agent_kg > 1e-6);
      derive();
    }
    /** An extinguisher's agent for one step (0.4 kg a second: agent_kg over discharge_s); empty, it stops. */
    function spend(e) { e.agent_kg -= (EX.agent_kg / EX.discharge_s) * dt; if (e.agent_kg <= 1e-6) { e.agent_kg = 0; e.on = false; } }
    /** The way into a room a crew member comes by: its first door to a corridor (else its first door), [x, z]. */
    function doorOf(i) {
      const ps = kit().portalsOf(L, comps[i].id).filter((p) => Math.abs(p.normal[1]) < 0.5 && p.between.indexOf("space") < 0);
      const other = (p) => comps[idx[p.between[0] === comps[i].id ? p.between[1] : p.between[0]]];
      const p = ps.find((q) => (other(q) || {}).kind === "corridor") || ps[0];
      if (p) return [p.center_m[0], p.center_m[2]];
      const c = kit().center(comps[i]); return [c[0], c[2]];
    }
    /**
     * One room's fire as its cells (fire-spread design 2 and 5): spread in from a neighbour at autoignition, the cells'
     * step with the extinguishers' cones and the mist, then the room's caps (ceiling and decay, half its oxygen in a step)
     * scaling every cell, then the chemistry exactly as the room model does it.
     */
    function stepFireCells(i) {
      const f = st.fire[i], r = FS.rooms[i], MIST = FI.suppression.water_mist, fo = fO2(i), was = f.hrr;
      if (f.hrr <= 0 && T[i] > FI.autoignition_k && f.fuel > 0 && fo > 0) {
        // Spread: the new fire starts at the cell nearest the portal whose flow brought the heat in (the open one to
        // the hottest neighbour).
        let best = null;
        for (const l of links) {
          if (!l.mix || l.open <= 0 || (l.a !== i && l.b !== i)) continue;
          const j = l.a === i ? l.b : l.a;
          if (j !== SPACE && (!best || T[j] > T[best.j])) best = { l, j };
        }
        const at = best && best.l.center ? best.l.center : kit().center(comps[i]);
        FS.seed(i, at[0], at[2], FI.seed_kw);
        log("Fire spreads into " + comps[i].name);
      }
      const cuts = [];
      for (const e of st.extinguishers) {
        if (e.comp !== i || !e.on || e.agent_kg <= 1e-6) continue;
        const aim = typeof e.aim === "string" ? FS.scriptedAim(i, e.aim, e.door || doorOf(i)) : e.aim;
        cuts.push({ cells: aim ? FS.footprintCells(i, aim.from, aim.dir) : [], w_per_s: EX.hrr_cut_kw_per_s * 1000 * e.n });
        e.lastAim = aim;
        spend(e);
      }
      if (f.ext > 0) f.ext -= dt;
      let hrr = FS.stepRoom(i, dt, { t_k: T[i], f_o2: fo, cuts, mist_w_per_s: f.mist > 0 ? MIST.hrr_cut_kw_per_s * 1000 : 0, wet: f.mist > 0 });
      const max = FI.hrr_max_kw_per_m2 * 1000 * floor[i] * fo;
      if (hrr > max) hrr = FS.scale(i, (hrr + ((max - hrr) * dt) / FI.decay_time_s) / hrr);
      let o2 = (hrr * dt) / (FI.mj_per_mol_o2 * MWW);
      if (o2 > 0.5 * n[O2][i]) { const k = (0.5 * n[O2][i]) / o2; hrr = FS.scale(i, k); o2 = (hrr * dt) / (FI.mj_per_mol_o2 * MWW); }
      f.hrr = hrr;
      if (was >= FI.out_below_kw * 1000 && hrr < FI.out_below_kw * 1000) log("Fire out in " + comps[i].name);
      if (hrr > 0 || r.active) f.fuel = FS.fuelLeft(i);
      if (hrr <= 0) return;
      const x = n[O2][i] / Math.max(1e-9, ntot[i]);
      const y = x < FI.starved_below_o2_fraction ? FI.smoke_mol_per_mol_o2_starved : FI.smoke_mol_per_mol_o2_ventilated;
      n[O2][i] -= o2; n[CO2][i] += FI.co2_mol_per_mol_o2 * o2; n[SMOKE][i] += y * o2;
      U[i] += hrr * dt;
    }
    /** Hot air damages what is in the room: systems, switchboards and (more slowly) the reactor. */
    function stepFireDamage() {
      for (let i = 0; i < N; i++) {
        const over = T[i] - FI.system_damage_above_k;
        if (over <= 0) continue;
        const pts = FI.system_damage_pct_per_s_per_k * over * dt;
        for (let li = 0; li < loads.length; li++) if (loads[li].compartment === comps[i].id) st.integrity[li] = Math.max(0, st.integrity[li] - pts);
        for (const nd of PW.nodes) if (nd.compartment === comps[i].id) { const h = st.nodeHealth[nd.id] == null ? 1 : st.nodeHealth[nd.id]; const left = h - pts / 100; st.nodeHealth[nd.id] = left < DM.nodes.destroyed_below ? 0 : left; }
        if (comps[i].id === L.systems.find((x) => x.id === PW.reactor.system).compartment) st.reactor.integrity = Math.max(0, st.reactor.integrity - pts * FI.reactor_damage_factor);
      }
    }
    /** A fire of kw (kW) in a room; at ([x, z]) is where, for the cell model (a hit's entry, a click), else the room's centre. */
    function ignite(compId, kw, at) {
      const i = idx[compId], f = st.fire[i];
      if (f.fuel <= 0) return false;
      if (FS) {
        const c = at || (() => { const m = kit().center(comps[i]); return [m[0], m[2]]; })();
        FS.seed(i, c[0], c[1], kw || FI.seed_kw);
        let q = 0; const r = FS.rooms[i]; for (let k = 0; k < r.n; k++) if (r.state[k] === FS.BURNING) q += r.q[k];
        f.hrr = q;
      } else f.hrr = Math.max(f.hrr, (kw || FI.seed_kw) * 1000);
      log("Fire in " + comps[i].name + " (" + (f.hrr / 1000).toFixed(0) + " kW)");
      return true;
    }

    // ======================================================= automation
    function stepAutomation() {
      derive();
      // Rate of pressure fall of every node since the last sub-step (Pa/s, positive falling).
      for (let i = 0; i < NN; i++) { st.fall[i] = st.t > 0 ? (st.pHist[i] - P[i]) / dt : 0; st.pHist[i] = P[i]; }
      // Vent dampers (life-support, "Ventilation"). A damper trips shut on excess net flow
      // (more than damper_trip_fraction_per_s of its room's gas a second: a leak or a fire's
      // expansion; ordinary heating is about a hundred times less), unless the flow is a refill
      // of a rising room. A tripped damper waits until its room has stopped falling for
      // damper_retry_s, then reopens if the room is near the duct's pressure, or refills it
      // (repressurize) if the room is above auto_repress_above_kpa; a room below that stays
      // isolated for the damage control board. Dampers also shut on low pressure, smoke, a
      // low duct, or no fan. A forced state (venting, isolation) overrides; a refill ends at
      // the make-up threshold, and is abandoned if the room falls while it runs (still leaking).
      const fan = fanRatio();
      const ductLow = P[DUCT] / 1000 < VE.duct_low_kpa;
      for (let i = 0; i < N; i++) {
        const v = links[ventOf[i]], id = comps[i].id;
        if (st.repress[i]) {
          if (P[i] / 1000 >= AT.plant.makeup.start_below_kpa) { delete st.damperForced[id]; st.repress[i] = 0; st.trip[i] = 0; log(comps[i].name + " back to " + (P[i] / 1000).toFixed(1) + " kPa"); }
          else if (st.fall[i] > VE.damper_falling_pa_per_s && v.open >= 1) { delete st.damperForced[id]; st.repress[i] = 0; st.trip[i] = VE.damper_retry_s; st.hold[i] = 1; log("Refill of " + comps[i].name + " stopped: it is still leaking"); }
          else { v.target = 1; continue; }
        }
        const forced = st.damperForced[id];
        if (forced != null) { v.target = forced ? 1 : 0; continue; }
        const refill = v.flow < 0 && st.fall[i] < 0;
        if (Math.abs(v.flow) > VE.damper_trip_fraction_per_s * ntot[i] && !refill) st.trip[i] = VE.damper_retry_s;
        else if (st.trip[i] > 0) {
          if (st.fall[i] > VE.damper_falling_pa_per_s) st.trip[i] = VE.damper_retry_s;
          else if ((st.trip[i] -= dt) <= 0) {
            const dpk = (P[DUCT] - P[i]) / 1000;
            if (Math.abs(dpk) < VE.damper_reset_kpa) st.trip[i] = 0;
            else if (P[i] / 1000 > VE.auto_repress_above_kpa && !st.hold[i]) { st.trip[i] = 0; repressurize(id); continue; }
            else st.trip[i] = dt; // isolated until the board acts
          }
        }
        if (!st.damperAuto) continue;
        const ppm = (n[SMOKE][i] / Math.max(1e-9, ntot[i])) * 1e6;
        const shut = fan <= 0 || st.trip[i] > 0 || P[i] / 1000 < VE.damper_close_below_kpa || ppm > VE.damper_close_smoke_ppm || ductLow;
        v.target = shut ? 0 : 1;
      }
      // Doors between two compartments close themselves on a pressure alarm: either side below
      // auto_close_below_kpa and falling faster than auto_close_fall_kpa_per_s, unless the
      // damage control board holds them. Doors to space are commanded, never automatic.
      if (st.pdoorAuto) for (const l of links) {
        if (l.target <= 0 || l.b === SPACE || PK.auto_close_kinds.indexOf(l.kind) < 0 || l.manualHold) continue;
        const alarm = (k) => P[k] / 1000 < PK.auto_close_below_kpa && st.fall[k] / 1000 > PK.auto_close_fall_kpa_per_s;
        if (alarm(l.a) || alarm(l.b)) { l.target = 0; log(l.id + " closing itself (pressure alarm)"); }
      }
      // Water mist: automatic discharge on a fire above 1 MW in a protected room.
      for (const id of FI.suppression.water_mist.compartments) {
        const i = idx[id], f = st.fire[i];
        const WM = FI.suppression.water_mist;
        f.detect = f.hrr > WM.auto_above_kw * 1000 ? (f.detect || 0) + dt : 0;
        if (st.autoMist !== false && f.detect >= WM.confirm_s && f.mist <= 0 && f.mistLeft > 0) dischargeMist(comps[i].id);
      }
      movePortals();
    }
    /** Portal motion: every door, vent, dump and valve toward its target at its own speed. */
    function movePortals() {
      for (const l of links) {
        if (l.open === l.target) continue;
        const opening = l.open < l.target, time = opening ? l.move : l.moveClose;
        const rate = time > 0 ? dt / time : 1;
        l.open = opening ? Math.min(l.target, l.open + rate) : Math.max(l.target, l.open - rate);
      }
    }

    // ======================================================= crew
    const CE = AT.crew_effects, MET = AT.metabolism, HE = opts.health || null;
    function addCrew(id, name, compId, o) {
      st.crew.push(Object.assign({ id, name, comp: idx[compId], working: false, suited: false, hp: 100, hyp: 0, hyc: 0, fed: 0, vac: 0, uncon: false, uncon_s: 0, dead: false, status: "ok", lastP: null }, o || {}));
    }
    function stepCrew() {
      derive();
      for (const c of st.crew) {
        if (c.dead || c.comp == null) continue;
        const i = c.comp;
        // Metabolism.
        const m = c.working ? MET.work_multiplier : 1;
        const o2 = Math.min(n[O2][i], MET.o2_mol_s_rest * m * dt);
        n[O2][i] -= o2; n[CO2][i] += MET.co2_mol_s_rest * m * dt;
        if (c.suited) { c.status = "suited"; c.lastP = P[i]; continue; }
        const pk = P[i] / 1000, po2 = pk * n[O2][i] / Math.max(1e-9, ntot[i]), pco2 = pk * n[CO2][i] / Math.max(1e-9, ntot[i]);
        const ppm = (n[SMOKE][i] / Math.max(1e-9, ntot[i])) * 1e6;
        let impaired = false;
        // Hypoxia: time of useful consciousness by oxygen partial pressure.
        const H = CE.hypoxia;
        if (po2 < H.tuc_table_po2_kpa_s[0][0]) { c.hyp += dt / lerpTable(H.tuc_table_po2_kpa_s, po2); }
        else if (po2 >= H.impaired_below_po2_kpa) c.hyp = Math.max(0, c.hyp - H.recover_per_s * dt);
        if (po2 < H.impaired_below_po2_kpa) impaired = true;
        // Health lost as oxygen falls (fire-spread design 6a): 0 at harm_below_po2_kpa, harm_full_hp_per_s at harm_full_po2_kpa.
        if (po2 < H.harm_below_po2_kpa) c.hp -= H.harm_full_hp_per_s * clamp((H.harm_below_po2_kpa - po2) / (H.harm_below_po2_kpa - H.harm_full_po2_kpa), 0, 1) * dt;
        // Hypercapnia.
        const HC = CE.hypercapnia;
        if (pco2 >= HC.tuc_table_pco2_kpa_s[0][0]) {
          const tb = HC.tuc_table_pco2_kpa_s; let t = tb[tb.length - 1][1];
          for (let k = 0; k + 1 < tb.length; k++) if (pco2 >= tb[k][0] && pco2 < tb[k + 1][0]) t = tb[k][1] + ((tb[k + 1][1] - tb[k][1]) * (pco2 - tb[k][0])) / (tb[k + 1][0] - tb[k][0]);
          c.hyc += dt / t;
        } else c.hyc = Math.max(0, c.hyc - HC.recover_per_s * dt);
        if (pco2 > HC.impaired_above_pco2_kpa) impaired = true;
        // Smoke: fractional effective dose of CO (Purser).
        c.fed += (ppm * dt) / 60 / CE.smoke.incapacitating_ppm_min;
        // Heat and cold.
        const HT = CE.heat;
        if (T[i] > HT.harm_above_k) c.hp -= HT.harm_hp_per_s_per_k * (T[i] - HT.harm_above_k) * dt;
        if (T[i] > HT.impaired_above_k || T[i] < HT.cold_impaired_below_k) impaired = true;
        if (T[i] < HT.cold_harm_below_k) c.hp -= HT.cold_harm_hp_per_s_per_k * (HT.cold_harm_below_k - T[i]) * dt;   // fire-spread 6a
        // Pressure.
        const PR = CE.pressure;
        if (pk < PR.armstrong_kpa) c.vac += dt / PR.vacuum_death_s; else c.vac = Math.max(0, c.vac - dt / PR.vacuum_death_s);
        if (pk < PR.impaired_below_kpa) impaired = true;
        if (c.lastP != null && (c.lastP - P[i]) / 1000 > PR.knockdown_drop_kpa_in_1s * dt) { c.hp -= PR.knockdown_hp * dt; }
        c.lastP = P[i];
        // crew-on-deck 7 (opts.health, data/crew/health.json): below incapacitated_below_hp a body is down and its vitals
        // fall on top of whatever still hurts it; nobody revives in the field.
        if (HE) {
          c.wounded = c.hp <= HE.wounded_at_or_below_hp;
          c.incapacitated = c.hp < HE.incapacitated_below_hp;
          if (c.incapacitated) c.hp -= HE.vitals_fall_hp_per_s * dt;
        }
        const unc = c.hyp >= 1 || c.hyc >= 1 || c.fed >= 1 || c.hp <= 0;
        c.uncon = unc;
        if (unc) {
          c.uncon_s += dt;
          if ((c.hyp >= 1 && po2 < H.death_below_po2_kpa && c.uncon_s > H.death_after_unconscious_s) ||
              (c.hyc >= 1 && pco2 > HC.death_above_pco2_kpa && c.uncon_s > HC.death_after_unconscious_s) ||
              c.fed >= CE.smoke.lethal_fed || c.vac >= 1 || c.hp <= -50) { c.dead = true; log(c.name + " died in " + comps[i].name); }
        } else c.uncon_s = 0;
        c.status = c.dead ? "dead" : unc ? "unconscious" : impaired ? "impaired" : "ok";
      }
      derive();
    }

    // ======================================================= damage: hits
    const HU = DM.hull, BR = DM.breach, PG = DM.propagation;
    let hitCount = 0;
    function compAt(p) { for (let i = 0; i < N; i++) if (inComp(comps[i], p)) return i; return SPACE; }
    function hullSection(p, dir) {
      const secs = L.hull.sections.map((s) => s.z_m).sort((a, b) => a - b);
      let span = 0; while (span + 1 < secs.length - 1 && p[2] > secs[span + 1]) span++;
      const ax = [Math.abs(dir[0]), Math.abs(dir[1]), Math.abs(dir[2])];
      const face = ax[1] >= ax[0] && ax[1] >= ax[2] ? (dir[1] < 0 ? "dorsal" : "ventral") : ax[0] >= ax[2] ? (dir[0] < 0 ? "port" : "starboard") : dir[2] < 0 ? "bow" : "stern";
      return span + ":" + face;
    }
    /**
     * Resolve one hit (damage-control): hull armour, breach, energy along the ray through the
     * compartments, systems and conduits within the blast radius, fire, crew.
     */
    function resolveHit(point, dir, energy_mj) {
      hitCount++;
      const id = "hit" + hitCount;
      const out = { id, breach: null, systems: [], conduits: [], fire: [], crew: [], compartments: [] };
      const key = hullSection(point, dir);
      const hs = st.hull[key] || (st.hull[key] = { integrity: 100 });
      const absorb = Math.min(energy_mj, HU.armour_mj * hs.integrity / 100);
      hs.integrity = Math.max(0, hs.integrity - (Math.min(energy_mj, HU.armour_mj) * 100) / HU.section_mj);
      let E = energy_mj - absorb;
      out.hull = { section: key, integrity: hs.integrity, absorbed_mj: absorb };
      if (E <= 0) return out;
      // March inward in steps of march_step_m. Each step in a compartment deposits the energy
      // the step takes, dE = E (1 - exp(-step / decay_m)), at its point; a target (a load, a
      // power node, a conduit segment) within r = radius_m + radius_per_sqrt_mj sqrt(E) of
      // that point takes points_per_mj dE (1 - d / r). A bulkhead crossed costs bulkhead_mj;
      // the first compartment entered from outside is breached, sized by the energy left.
      const fall = 1 - Math.exp(-PG.march_step_m / PG.decay_m);
      // Pre-filter: only targets within the largest radius of the whole ray, and only the
      // compartments whose brushes' bounds the ray's bounding box touches, are tested per step.
      const end = [point[0] + dir[0] * PG.march_max_m, point[1] + dir[1] * PG.march_max_m, point[2] + dir[2] * PG.march_max_m];
      const rMax = PG.radius_m + PG.radius_per_sqrt_mj * Math.sqrt(E);
      const near = (q) => segSegDist(point, end, q, q) < rMax;
      const candLoads = []; for (let li = 0; li < loads.length; li++) if (near(loads[li].center)) candLoads.push(li);
      const candNodes = PW.nodes.filter((nd) => near(nd.center_m));
      const candConduits = PW.conduits.filter((k) => { for (let q = 0; q + 1 < k.path_m.length; q++) if (segSegDist(point, end, k.path_m[q], k.path_m[q + 1]) < rMax) return true; return false; });
      const lo = [0, 1, 2].map((a) => Math.min(point[a], end[a])), hi = [0, 1, 2].map((a) => Math.max(point[a], end[a]));
      const candComps = []; for (let c = 0; c < N; c++) { const b = kit().bounds(comps[c]); if (b.x[0] <= hi[0] && b.x[1] >= lo[0] && b.y[0] <= hi[1] && b.y[1] >= lo[1] && b.z[0] <= hi[2] && b.z[1] >= lo[2]) candComps.push(c); }
      const compOn = (p) => { for (const c of candComps) if (inComp(comps[c], p)) return c; return SPACE; };
      const sysPts = new Float64Array(loads.length), nodePts = {}, condE = {}, condAt = {}, roomE = new Float64Array(N), roomLen = new Float64Array(N), roomR = new Float64Array(N), roomAt = [];
      let cur = SPACE, entered = false, s = 0;
      for (; s <= PG.march_max_m && E > 0.05; s += PG.march_step_m) {
        const p = [point[0] + dir[0] * s, point[1] + dir[1] * s, point[2] + dir[2] * s];
        const c = compOn(p);
        if (c !== cur && c !== SPACE) {
          if (entered) E = Math.max(0, E - PG.bulkhead_mj);
          else {
            entered = true;
            const area = clamp(BR.m2_per_mj * E, BR.min_m2, BR.max_m2);
            out.breach = { compartment: comps[c].id, area_m2: area };
            breach(comps[c].id, area, p);
          }
        }
        cur = c;
        const dE = E * fall;
        E -= dE;
        if (c === SPACE || dE <= 0) continue;
        const r = PG.radius_m + PG.radius_per_sqrt_mj * Math.sqrt(E + dE);
        roomE[c] += dE; roomLen[c] += PG.march_step_m; roomR[c] = Math.max(roomR[c], r);
        if (!roomAt[c]) roomAt[c] = p;   // where the march entered the room: a fire there starts under it (fire-spread 2)
        for (const li of candLoads) {
          const q = loads[li].center, d = Math.hypot(q[0] - p[0], q[1] - p[1], q[2] - p[2]);
          if (d < r) sysPts[li] += DM.systems.points_per_mj * dE * (1 - d / r);
        }
        for (const nd of candNodes) {
          if (nd.compartment !== comps[c].id) continue;
          const q = nd.center_m, d = Math.hypot(q[0] - p[0], q[1] - p[1], q[2] - p[2]);
          if (d < r) nodePts[nd.id] = (nodePts[nd.id] || 0) + DM.systems.points_per_mj * dE * (1 - d / r);
        }
        for (const k of candConduits) {
          if (k.route.indexOf(comps[c].id) < 0) continue;
          let dmin = Infinity, qmin = 0;
          for (let q = 0; q + 1 < k.path_m.length; q++) { const d = segSegDist(p, p, k.path_m[q], k.path_m[q + 1]); if (d < dmin) { dmin = d; qmin = q; } }
          if (dmin < r) {
            const e = dE * (1 - dmin / r);
            condE[k.id] = (condE[k.id] || 0) + e;
            // Where the conduit took most of it: the cut, should it sever (the damage map draws the break there).
            if (!condAt[k.id] || e > condAt[k.id].e) condAt[k.id] = { e, at: closestOnSeg(p, k.path_m[qmin], k.path_m[qmin + 1]) };
          }
        }
      }
      for (let li = 0; li < loads.length; li++) if (sysPts[li] > 0.05) {
        st.integrity[li] = Math.max(0, st.integrity[li] - sysPts[li]);
        out.systems.push({ id: loads[li].id, points: sysPts[li], integrity: st.integrity[li] });
      }
      for (const nd of PW.nodes) if (nodePts[nd.id] > 0.05) {
        const h = st.nodeHealth[nd.id] == null ? 1 : st.nodeHealth[nd.id];
        const left = Math.max(0, h - nodePts[nd.id] / 100);
        st.nodeHealth[nd.id] = left < DM.nodes.destroyed_below ? 0 : left;
        out.nodes = out.nodes || []; out.nodes.push({ id: nd.id, health: st.nodeHealth[nd.id] });
        if (st.nodeHealth[nd.id] === 0) log(nd.name + " destroyed");
      }
      for (const k of PW.conduits) {
        const e = condE[k.id]; if (!e) continue;
        const cs = st.conduit[k.id];
        if (e >= DM.conduits.sever_mj) { cs.severed = true; cs.cut = condAt[k.id].at; out.conduits.push({ id: k.id, severed: true, at: cs.cut }); log("Conduit " + k.id + " severed"); }
        else if (e >= DM.conduits.damage_mj) { cs.health = Math.min(cs.health, DM.conduits.damaged_capacity); out.conduits.push({ id: k.id, severed: false, health: cs.health }); }
      }
      for (let c = 0; c < N; c++) {
        const e = roomE[c]; if (e <= 0.01) continue;
        out.compartments.push({ id: comps[c].id, mj: e, radius_m: roomR[c] });
        const chance = Math.min(DM.fire.chance_max, DM.fire.chance_per_mj * e) * fO2(c);
        if (hash32(seed, id, comps[c].id, "fire") < chance) { ignite(comps[c].id, FI.seed_kw + DM.fire.seed_kw_per_mj * e, roomAt[c] ? [roomAt[c][0], roomAt[c][2]] : null); out.fire.push(comps[c].id); }
        // Crew: the engine knows where each crew member stands (crew-on-deck) and applies the
        // same falloff as systems; this mockup has no positions, so every crew member in the
        // room takes the expected share: the fraction of the floor within r of the path.
        const cover = Math.min(1, (2 * roomR[c] * roomLen[c]) / Math.max(1, floor[c]));
        for (const cr of st.crew) {
          if (cr.comp !== c || cr.dead) continue;
          const hp = DM.crew.hp_per_mj * e * DM.crew.share * cover; cr.hp -= hp; out.crew.push({ id: cr.id, hp });
        }
      }
      log("Hit " + energy_mj + " MJ at " + key + ": " + out.compartments.map((c) => c.id).join(", "));
      return out;
    }
    function hullHalfBeam(z) {
      const secs = L.hull.sections.slice().sort((a, b) => a.z_m - b.z_m);
      for (let i = 0; i + 1 < secs.length; i++) if (z >= secs[i].z_m && z <= secs[i + 1].z_m) {
        const f = (z - secs[i].z_m) / (secs[i + 1].z_m - secs[i].z_m);
        return { hw: secs[i].half_beam_m + f * (secs[i + 1].half_beam_m - secs[i].half_beam_m), top: secs[i].top_m + f * (secs[i + 1].top_m - secs[i].top_m), bottom: secs[i].bottom_m + f * (secs[i + 1].bottom_m - secs[i].bottom_m) };
      }
      return { hw: 8, top: 6, bottom: -4 };
    }

    // ======================================================= commands (intents)
    let breachCount = 0;
    function breach(compId, area, at) {
      breachCount++;
      const l = addLink({ id: "breach_" + breachCount, kind: "breach", a: idx[compId], b: SPACE, area, open: 1, center: at || null });
      st.breaches.push(l); log("Breach " + (area < 0.1 ? area.toFixed(3) : area.toFixed(2)) + " m^2 in " + comps[idx[compId]].name);
      return l;
    }
    function patch(linkId) { const l = linkById[linkId]; if (l) { l.target = 0; l.open = 0; l.area = 0; if (l.a >= 0 && l.a < N) st.hold[l.a] = 0; log("Breach " + linkId + " patched"); } }
    /** Refill a compartment from the duct through its vent (the damage control board's command). */
    function repressurize(compId) { const i = idx[compId]; st.damperForced[compId] = true; st.repress[i] = 1; st.trip[i] = 0; st.hold[i] = 0; log("Refilling " + comps[i].name + " from the duct"); }
    function setLink(id, target) { const l = typeof id === "number" ? links[id] : linkById[id]; if (l) l.target = target ? 1 : 0; return l; }
    function closeAllDoors() { for (const l of links) if (["door", "hatch", "ladder", "hoist"].indexOf(l.kind) >= 0) l.target = 0; }
    function isolate(compId) { const i = idx[compId]; for (const l of links) if ((l.a === i || l.b === i) && l.kind !== "breach") { l.target = 0; l.open = 0; } st.damperForced[compId] = false; }
    function bayPumpdown(bayId) {
      const i = idx[bayId];
      if (BP.interlock_unsuited_crew && st.crew.some((c) => c.comp === i && !c.suited && !c.dead)) { log("Pump-down of " + comps[i].name + " refused: unsuited crew inside"); return false; }
      for (const l of links) if ((l.a === i || l.b === i) && l.kind !== "breach" && l.kind !== "valve") l.target = 0;
      st.damperForced[bayId] = false;
      Object.assign(st.bay, { target: bayId, mode: "pumpdown", moved_mol: 0, energy_mj: 0, t0: st.t });
      log("Pump-down of " + comps[i].name + " started");
      return true;
    }
    function bayRepress(bayId) {
      const i = idx[bayId];
      for (const l of links) if ((l.a === i || l.b === i) && (l.kind === "bay_door" || l.kind === "valve")) l.target = 0;
      Object.assign(st.bay, { target: bayId, mode: "repress", t0: st.t });
      delete st.damperForced[bayId];
    }
    function airlockCycleOut() { setLink(AL.inner, 0); setLink(AL.outer, 0); Object.assign(st.airlock, { mode: "out_pump", t0: st.t }); st.damperForced[AL.compartment] = false; }
    function airlockCycleIn() { setLink(AL.outer, 0); setLink("valve_airlock_eq", 1); Object.assign(st.airlock, { mode: "in_eq", t0: st.t }); }
    function dischargeMist(compId) {
      const f = st.fire[idx[compId]], WM = FI.suppression.water_mist;
      if (WM.compartments.indexOf(compId) < 0 || f.mistLeft <= 0) return false;
      f.mist = WM.duration_s; f.mistLeft--; log("Water mist discharging in " + comps[idx[compId]].name); return true;
    }
    function dischargeInert(compId) {
      const IG = FI.suppression.inert_gas;
      if (IG.compartments.indexOf(compId) < 0) return false;
      // The discharge closes the room's doors and its vent damper first (hold the concentration).
      const i = idx[compId];
      for (const l of links) if ((l.a === i || l.b === i) && l.kind !== "breach" && l.kind !== "valve") l.target = 0;
      st.damperForced[compId] = false;
      st.fire[i].inert = IG.time_s + IG.soak_s; log("Inert gas flooding " + comps[i].name); return true;
    }
    /**
     * count crew discharging extinguishers at the same fire together (their cuts add). aim (the cell model): "careful"
     * (the default: they sweep the burning cells nearest the door they came in by) or "careless" (at the room's centre).
     */
    function useExtinguisher(compId, count, aim) {
      const i = idx[compId], f = st.fire[i];
      f.ext = EX.discharge_s; f.extN = count || 1;
      st.extinguishers.push({ comp: i, aim: aim || "careful", on: true, agent_kg: EX.agent_kg, n: count || 1, door: doorOf(i), held: false });
      log((count > 1 ? count + " extinguishers" : "Extinguisher") + " on the fire in " + comps[i].name);
    }
    /** An extinguisher a crew member carries (the walking player): the page sets comp (a compartment index), aim
     * ({ from, dir }) and on (the trigger held) each frame; agent_kg is what is left (refill: set it to EX.agent_kg). */
    function newExtinguisher() {
      const e = { comp: -1, aim: null, on: false, agent_kg: EX.agent_kg, n: 1, door: null, held: true };
      st.extinguishers.push(e); return e;
    }
    const VENT_DOORS = ["door", "pressure_door", "hatch", "ladder", "hoist"];
    /**
     * The captain's vent (fire-spread design 6a; damage-control 4): the room's doors and its damper shut at once and the
     * room is warned for VENT.warning_s (klaxon, red strobe; its doors still open from inside on a press), then the
     * duct is isolated, this room's vent opens and the dump opens. The board can only request it.
     */
    function ventCompartment(compId) {
      const vi = idx[compId];
      for (const l of links) if ((l.a === vi || l.b === vi) && VENT_DOORS.indexOf(l.kind) >= 0) { l.target = 0; l.manualHold = false; }
      st.damperForced[compId] = false;
      st.vent = { comp: vi, phase: "warning", warn: VENT.warning_s, t0: st.t };
      log("Venting " + comps[vi].name + " in " + VENT.warning_s.toFixed(0) + " s: doors shut, klaxon");
    }
    function openDump(vi) {
      for (const l of links) if ((l.a === vi || l.b === vi) && VENT_DOORS.indexOf(l.kind) >= 0) { l.target = 0; l.manualHold = false; }
      for (let i = 0; i < N; i++) st.damperForced[comps[i].id] = i === vi;
      setLink(GA.overboard_dump.id, 1);
      log("Venting " + comps[vi].name + " overboard through the duct");
    }
    function stepVent() {
      const v = st.vent;
      if (!v || v.phase !== "warning") return;
      v.warn -= dt;
      if (v.warn <= 1e-9) { v.warn = 0; v.phase = "venting"; openDump(v.comp); }
    }
    function stopVent() { st.vent = null; st.damperForced = {}; setLink(GA.overboard_dump.id, 0); }
    /**
     * What the captain sees on arming a vent (fire-spread design 6a): who is in the room, the warning, and the seconds the
     * room takes to fall to extinct_kpa once the dump opens, from the same flow solve run on a copy of the gas (doors
     * shut, the duct isolated, this room's vent and the dump opening at their own speeds), which is then put back.
     */
    function ventPreview(compId) {
      const i = idx[compId];
      derive();
      const save = { n: n.map((a) => a.slice()), U: U.slice(), links: links.map((l) => [l.open, l.target, l.G, l.F, l.flow]), lost: st.lostOverboard };
      for (const l of links) if ((l.a === i || l.b === i) && VENT_DOORS.indexOf(l.kind) >= 0) { l.open = 0; l.target = 0; }
      for (let k = 0; k < N; k++) links[ventOf[k]].target = k === i ? 1 : 0;
      linkById[GA.overboard_dump.id].target = 1;
      const end = FI.extinct_kpa * 1000;
      let t = 0;
      while (t < 900 && P[i] > end) { movePortals(); flowSolve(); t += dt; }
      for (let k = 0; k < 4; k++) n[k].set(save.n[k]);
      U.set(save.U); st.lostOverboard = save.lost;
      links.forEach((l, k) => { [l.open, l.target, l.G, l.F, l.flow] = save.links[k]; });
      derive();
      return { compartment: compId, warning_s: VENT.warning_s, empty_s: t, total_s: VENT.warning_s + t, reached: P[i] > end ? t < 900 : true,
        crew: st.crew.filter((c) => c.comp === i && !c.dead).map((c) => c.name) };
    }
    /**
     * Open or close a door the way a crew member or the damage control board does: an opening
     * across more than the interlock's pressure difference is refused unless overridden.
     */
    function operateDoor(id, open, override) {
      const l = linkById[id]; if (!l) return "no such portal";
      if (!open) { l.target = 0; l.manualHold = false; return "ok"; }
      const pb = l.b === SPACE ? 0 : P[l.b];
      const dp = Math.abs(P[l.a] - pb) / 1000;
      if (l.b !== SPACE && dp > PK.interlock_max_dp_kpa && !override) return "interlock: " + dp.toFixed(0) + " kPa across";
      l.target = 1; l.manualHold = !!override; // a crew member passing does not hold it; the board's override does
      return "ok";
    }
    /**
     * Engineering's priority for a load (1 to 3); the vital class 0 is fixed, except in a group whose priority_min is 0
     * (Cooling, reactor-cooling design 6), whose loads go from 0 to 3 and back.
     */
    function setPriority(loadId, p) {
      const i = loadIdx[loadId]; if (i == null) return false;
      const g = PW.groups.find((x) => x.loads.indexOf(loadId) >= 0), lo = g && g.priority_min != null ? g.priority_min : 1;
      if (loads[i].priority < lo) return false;
      st.priority[i] = clamp(Math.round(p), lo, 3); return true;
    }
    /** Apply a named preset from the data: setpoints by group. */
    function applyPreset(name) {
      const pr = PW.presets[name]; if (!pr) return false;
      for (const g of PW.groups) {
        const v = pr[g.id] != null ? pr[g.id] : 1.0;
        for (const id of g.loads) st.setpoint[loadIdx[id]] = v;
      }
      st.preset = name; log("Preset " + name.toUpperCase());
      return true;
    }
    /** Repairs (damage-control): integrity points to a system, a conduit splice, a node rebuild. */
    function repairSystem(loadId, pts) { const i = loadIdx[loadId]; st.integrity[i] = clamp(st.integrity[i] + pts, 0, 100); return st.integrity[i]; }
    function spliceConduit(id) { const c = st.conduit[id]; if (!c) return false; c.severed = false; c.cut = null; c.health = Math.max(c.health, DM.repair.conduit_splice_capacity); return true; }
    function rebuildNode(id) { st.nodeHealth[id] = Math.max(st.nodeHealth[id] == null ? 1 : st.nodeHealth[id], DM.repair.node_rebuild_to); return true; }
    function log(msg) { st.log.push({ t: st.t, msg }); if (st.log.length > 200) st.log.shift(); }

    // ======================================================= read-only views (ship-plan-view 6: the damage map)
    // What the map draws, read from the state the solve and the hits leave: nothing here changes the ship, and the map
    // never decides for itself whether a conduit is live or a box needs fixing (CLAUDE.md 6.1).
    const nodeHp = (id) => (st.nodeHealth[id] == null ? 1 : st.nodeHealth[id]);
    /** Whether a power node carries power now: an edge touching it carries flow, or a load on it is supplied. */
    function nodeLive(id) {
      if (nodeHp(id) <= 0) return false;
      for (const k of PW.conduits) if ((k.between[0] === id || k.between[1] === id) && Math.abs(st.flows[k.id] || 0) > EPS) return true;
      for (const t of PW.ties) if ((t.between[0] === id || t.between[1] === id) && Math.abs(st.flows[t.id] || 0) > EPS) return true;
      for (const g of PW.generators) if (g.node === id && (st.flows[g.id] || 0) > EPS) return true;
      if (PW.battery.node === id && (st.flows.battery_out || 0) > EPS) return true;
      for (let i = 0; i < loads.length; i++) if (loads[i].node === id && st.alloc[i] > EPS) return true;
      return false;
    }
    /**
     * A conduit as the map draws it (power-grid 9): mw signed, positive from between[0] to between[1] (the solve's own
     * edge flow); state "severed" (cut by a hit), "dead" (its breaker open, an end destroyed, or no power at either
     * end) or "live" (carrying power, or energized and idle); health (its capacity's share left); breaker "closed" or
     * "open"; cut, where a hit severed it ([x, y, z] on its path) or null.
     */
    function conduitView(id) {
      const k = PW.conduits.find((x) => x.id === id), c = st.conduit[id];
      if (!k || !c) return null;
      const mw = st.flows[id] || 0;
      let state = "live";
      if (c.severed) state = "severed";
      else if (!c.breaker || nodeHp(k.between[0]) <= 0 || nodeHp(k.between[1]) <= 0) state = "dead";
      else if (Math.abs(mw) <= EPS && !(nodeLive(k.between[0]) && nodeLive(k.between[1]))) state = "dead";
      return { id, name: k.name, mw, state, health: c.health, breaker: c.breaker ? "closed" : "open", cut: c.cut || null, capacity_mw: k.capacity_mw };
    }
    /** A switchboard section, panel or bus as the map draws it (power-grid 5): health 0-1 and state "sound", "damaged" or "destroyed". */
    function nodeView(id) {
      const h = nodeHp(id);
      return { id, health: h, state: h <= 0 ? "destroyed" : h < 1 ? "damaged" : "sound", live: nodeLive(id) };
    }
    /**
     * Every breaker, by the node whose box it stands in, and its state "closed", "open" (opened or tripped) or "locked"
     * (locked open): a conduit's at its first end, a tie's at its first end, a generator's at its switchboard section,
     * the battery's at its bus, a load's at its panel, a group's feed breaker (power-grid 5) at each of its loads'
     * panels. [{ kind, id, name, node, state }]
     */
    function breakers() {
      const out = [];
      for (const k of PW.conduits) out.push({ kind: "conduit", id: k.id, name: k.name, node: k.between[0], state: st.conduit[k.id].breaker ? "closed" : "open" });
      for (const t of PW.ties) out.push({ kind: "tie", id: t.id, name: t.name, node: t.between[0], state: st.tieClosed[t.id] ? "closed" : "open" });
      for (const g of PW.generators) out.push({ kind: "generator", id: g.id, name: g.name, node: g.node, state: st.genClosed[g.id] ? "closed" : "open" });
      out.push({ kind: "battery", id: "battery_out", name: "Battery", node: PW.battery.node, state: st.batteryBreaker ? "closed" : "open" });
      const grouped = {};
      for (const g of PW.groups) if (g.feed_breaker) {
        const s = st.groupBreaker[g.id] || "closed", at = new Set(g.loads.map((id) => loads[loadIdx[id]].node));
        for (const nd of at) out.push({ kind: "group", id: g.id, name: g.name, node: nd, state: s });
        for (const id of g.loads) grouped[id] = true;
      }
      for (let i = 0; i < loads.length; i++) if (!grouped[loads[i].id] && st.breakerOpen[i]) out.push({ kind: "load", id: loads[i].id, name: loads[i].name, node: loads[i].node, state: "open" });
      return out;
    }
    /**
     * damage::repair_time (damage-control 6 and 6a): the time and the parts a job takes, the one rate the damage board
     * previews and the repair spends. job: { kind: "system", id: a load id } | { kind: "reactor" }
     * | { kind: "coolant", id: a pipe segment, tank or the exchanger (reactor-cooling design 1) } | { kind: "node", id }
     * | { kind: "conduit", id } | { kind: "breach", id: a breach link id } | { kind: "hull", key: "span:face" }.
     * who: "officer" (every player) or "rating" (the teams); hands: 1, or 2 working together (two_hands_factor times the
     * faster). Returns { s, parts, kit, plates, eva, steps: [{ what, s }] }, or null when there is nothing to do, or
     * { dock: true } for a damaged conduit (half its capacity until the dock). The walk to the job and suiting are not
     * in it: this library has no crew-portal graph (crew-on-deck's walk times are the engine's).
     */
    function repairTime(job, who, hands) {
      const RP = DM.repair, rate = RP.kit_pct_per_s[who || "officer"];
      if (!Number.isFinite(rate)) throw new Error("damage.json repair.kit_pct_per_s." + who + ": missing");
      const speed = (hands === 2 ? RP.two_hands_factor : 1), slow = RP.kit_pct_per_s.officer / rate; // fixed times are the officer's
      const fixed = (s) => (s * slow) / speed, kit = (from, to) => (Math.max(0, to - from) / rate) / speed;
      const res = (steps, parts, o) => Object.assign({ s: steps.reduce((a, x) => a + x.s, 0), parts, kit: false, plates: 0, eva: false, steps }, o || {});
      const system = (g) => {
        if (g >= 100) return null;
        if (g <= 0) return res([{ what: "rebuild", s: fixed(RP.destroyed_rebuild_s) }, { what: "kit", s: kit(RP.destroyed_rebuild_to_pct, 100) }], RP.parts.destroyed_system, { kit: true });
        return res([{ what: "kit", s: kit(g, 100) }], g < DM.systems.disabled_below_pct ? RP.parts.disabled_system : 0, { kit: true });
      };
      if (job.kind === "system") return system(st.integrity[loadIdx[job.id]]);
      if (job.kind === "reactor") return system(st.reactor.integrity);
      if (job.kind === "coolant") return st.coolantParts[job.id] ? system(st.coolantParts[job.id].integrity) : null;
      if (job.kind === "node") {
        const h = nodeHp(job.id);
        if (h >= 1) return null;
        if (h <= 0) return res([{ what: "rebuild", s: fixed(RP.node_rebuild_s) }, { what: "kit", s: kit(RP.node_rebuild_to * 100, 100) }], RP.parts.node_rebuild, { kit: true });
        return res([{ what: "kit", s: kit(h * 100, 100) }], 0, { kit: true });
      }
      if (job.kind === "conduit") {
        const c = st.conduit[job.id];
        if (!c) return null;
        if (c.severed) return res([{ what: "splice", s: fixed(RP.conduit_splice_s) }], RP.parts.conduit_splice);
        return c.health < 1 ? { dock: true } : null;
      }
      if (job.kind === "breach") {
        const l = linkById[job.id];
        if (!l || !(l.area > 0)) return null;
        const PT = DM.patch;
        if (l.area <= PT.inside_max_m2) { const n = Math.ceil(l.area / PT.plate_m2 - 1e-9); return res([{ what: "plates", s: fixed(n * PT.inside_s_per_plate) }], 0, { plates: n }); }
        return res([{ what: "eva", s: fixed(l.area * PT.eva_s_per_m2) }], 0, { eva: true });
      }
      if (job.kind === "hull") {
        const g = st.hull[job.key] ? st.hull[job.key].integrity : 100;
        if (g >= 100) return null;
        return res([{ what: "eva", s: fixed((100 - g) / RP.hull_section_pct_per_s_eva) }], 0, { eva: true });
      }
      return null;
    }

    // ======================================================= the sub-step
    function step() {
      stepVent();
      stepAutomation();
      stepCooling();
      stepPower();
      stepHeat();
      stepPlant();
      stepPumps();
      stepFire();
      stepFireDamage();
      stepCrew();
      stepRoomHeat();
      flowSolve();
      mixing();
      checkScram();
      st.t += dt;
    }

    /** Read-only numbers for consoles and tables (kPa, K, percent). */
    function compartmentReadout(i) {
      const nt = ntot[i] > 0 ? ntot[i] : 1e-30;
      return {
        id: i === DUCT ? "duct" : comps[i].id, p_kpa: P[i] / 1000, po2_kpa: (P[i] / 1000) * n[O2][i] / nt, co2_pct: (100 * n[CO2][i]) / nt,
        pco2_kpa: (P[i] / 1000) * n[CO2][i] / nt, smoke_ppm: (1e6 * n[SMOKE][i]) / nt, t_k: T[i], mol: ntot[i],
        fire_kw: i < N ? st.fire[i].hrr / 1000 : 0,
      };
    }
    function lighting(i) {
      // Normal lighting from the compartment's panel; emergency lighting from the emergency bus.
      const id = comps[i].id;
      for (const l of loads) if (l.lighting && l.lighting.indexOf(id) >= 0) {
        const li = loadIdx[l.id];
        if (st.demand[li] > 0 && st.alloc[li] / st.demand[li] >= LG.normal_below_ratio) return st.alert === "red_alert" ? "red_alert" : "normal";
      }
      const ei = loadIdx.emergency_lighting;
      return st.demand[ei] > 0 && st.alloc[ei] / st.demand[ei] >= 0.5 ? "emergency" : "dark";
    }

    return {
      dt, L, PW, AT, DM, st, comps, N, DUCT, idx, vol, floor, extArea, adj, links, linkById, loads, loadIdx, stores, nodes, SPECIES,
      step, derive, solveGrid, gridSnapshot, computeDemand, solveWithDropout, previewSetpoint, previewGroup, scram, scramReset,
      resolveHit, hullHalfBeam, breach, patch, setLink, closeAllDoors, isolate, ignite, bayPumpdown, bayRepress,
      dischargeMist, dischargeInert, useExtinguisher, newExtinguisher, ventPreview, gravityG, capability, damageState, auxRatio, ignitionRate,
      operateDoor, setPriority, applyPreset, setCooling, coolantReadout, coolantView, damagePart, repairPart, isolateSegment, partIntegrity, setGroupBreaker, pumpSpeed, repressurize, repairSystem, spliceConduit, rebuildNode, fanRatio,
      airlockCycleOut, airlockCycleIn, ventCompartment, stopVent, addCrew, setActivity, compartmentReadout, lighting,
      receiverKpa, storeMol, fillStandard, log, fs: FS, fO2, conduitView, nodeView, nodeLive, breakers, repairTime, hullSection,
      get P() { return P; }, get T() { return T; }, get n() { return n; }, get ntot() { return ntot; },
    };
  }

  root.ShipSystems = { version: 2, create, measure, hash32 };
})(typeof window !== "undefined" ? window : globalThis);
