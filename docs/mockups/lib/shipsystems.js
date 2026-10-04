/*
 * shipsystems.js: the systems mockup's simulation of power, atmosphere, heat, fire and hits.
 *
 * It implements, in one place, the formulas of three proposed OpenSpec changes so the
 * mockup shows what the designs say and nothing else:
 *   openspec/changes/power-grid      (the power solve, reactor, battery, heat and coolant)
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
  function measure(c) {
    let v = 0, a = 0;
    for (const b of c.boxes) { const dx = b.x[1] - b.x[0], dy = b.y[1] - b.y[0], dz = b.z[1] - b.z[0]; v += dx * dy * dz; a += dx * dz; }
    return { volume_m3: v, floor_m2: a };
  }
  function boxSurface(b) { const dx = b.x[1] - b.x[0], dy = b.y[1] - b.y[0], dz = b.z[1] - b.z[0]; return 2 * (dx * dy + dy * dz + dx * dz); }
  /** Area of faces of box a and box b that face each other across a gap of at most gap_m. */
  function facingArea(a, b, gap) {
    let area = 0;
    for (const k of ["x", "y", "z"]) {
      const o = ["x", "y", "z"].filter((q) => q !== k);
      const ov = (q) => Math.max(0, Math.min(a[q][1], b[q][1]) - Math.max(a[q][0], b[q][0]));
      const g1 = b[k][0] - a[k][1], g2 = a[k][0] - b[k][1];
      if ((g1 >= -1e-6 && g1 <= gap) || (g2 >= -1e-6 && g2 <= gap)) area += ov(o[0]) * ov(o[1]);
    }
    return area;
  }
  function inBox(b, p) { return p[0] >= b.x[0] && p[0] <= b.x[1] && p[1] >= b.y[0] && p[1] <= b.y[1] && p[2] >= b.z[0] && p[2] <= b.z[1]; }
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
   * opts.seed is the session seed for every random choice.
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

    // Adjacency (shared bulkhead area) and exterior area, derived from the layout boxes.
    // The deck compiler will bake these; here they are computed once at start.
    const adj = [];
    for (let i = 0; i < N; i++) {
      let surf = 0, inner = 0;
      for (const b of comps[i].boxes) surf += boxSurface(b);
      for (const b1 of comps[i].boxes) for (const b2 of comps[i].boxes) if (b1 !== b2) inner += facingArea(b1, b2, 1e-4);
      let shared = 0;
      for (let j = 0; j < N; j++) {
        if (j === i) continue;
        let a = 0;
        for (const b1 of comps[i].boxes) for (const b2 of comps[j].boxes) a += facingArea(b1, b2, TH.adjacency_gap_m);
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
      l.mix = ["door", "pressure_door", "hatch", "ladder", "hoist"].indexOf(l.kind) >= 0 && l.b !== SPACE;
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
      loop: { T: PW.coolant.initial_k, flow: 1, rad_mw: 0, in_mw: 0, radiatorHealth: 1, branchOpen: {} },
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
      trip: new Float64Array(N), prevRoomP: new Float64Array(N), repress: new Uint8Array(N),
    };
    for (const g of PW.generators) st.genClosed[g.id] = true;
    for (const t of PW.ties) st.tieClosed[t.id] = !!t.closed;
    for (const k of PW.conduits) st.conduit[k.id] = { health: 1, severed: false, breaker: true };
    for (const l of loads) if (l.heat && l.heat.to === "node") st.thermal[l.id] = { T: PW.coolant.initial_k + 5 };
    for (const l of loads) st.loop.branchOpen[l.id] = true;
    st.reactor.T = PW.coolant.initial_k + 250;
    const FI = AT.fire;
    for (let i = 0; i < N; i++) {
      const fm2 = FI.fuel_mj_per_m2[comps[i].id] != null ? FI.fuel_mj_per_m2[comps[i].id] : FI.fuel_mj_per_m2_default;
      st.fire.push({ hrr: 0, fuel: fm2 * floor[i] * 1e6, ext: 0, mist: 0, mistLeft: FI.suppression.water_mist.discharges, inert: 0, prevP: P[i], out: 0 });
    }
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
      add("battery_out", BAT, pIdx[B.node], 1e9, true, "battery");
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
      return id === "emergency_lighting" || (id.startsWith("reactor_aux") && st.reactor.state === "igniting") ||
        (PW.coolant.pumps.indexOf(id) >= 0 && st.reactor.state !== "running");
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
        else {
          const a = l.activity ? (st.activity[l.activity] || 0) : 1;
          x = l.standby_mw + Math.max(0, l.nominal_mw * sp - l.standby_mw) * a;
        }
        d[i] = x * cap;
      }
      return d;
    }
    /** What a system can do at its integrity (damage-control's states): 1, integrity / nominal, or 0. */
    function capability(i) {
      const g = st.integrity[i], SY = DM.systems;
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
        const aux = auxRatio();
        if (aux >= 0.95) rx.ignition += dt;
        if (rx.ignition >= RC.restart.ignition_s) { rx.state = "running"; rx.throttle = RC.restart.start_throttle; log("Reactor ignited; throttle ramps from " + (RC.restart.start_throttle * 100).toFixed(0) + "%"); }
      }
    }
    /** Supply of the reactor auxiliaries against what they need now (a dead switchboard counts as none). */
    function auxRatio() {
      let need = 0, a = 0;
      for (const id of ["reactor_aux_p", "reactor_aux_s"]) {
        const i = loadIdx[id], l = loads[i];
        need += st.reactor.state === "igniting" ? PW.reactor.restart.ignition_mw / 2 : st.reactor.state === "running" ? l.nominal_mw : l.standby_mw;
        a += st.alloc[i];
      }
      return need > 0 ? a / need : 1;
    }
    const roomHeat = new Float64Array(NN);
    function stepHeat() {
      roomHeat.fill(0);
      const loop = st.loop, C = PW.coolant, RD = PW.radiators;
      let flow = 0;
      for (const id of C.pumps) { const i = loadIdx[id]; flow += (loads[i].nominal_mw > 0 ? st.alloc[i] / loads[i].nominal_mw : 0) * C.flow_per_pump * (st.integrity[i] / 100); }
      loop.flow = clamp(flow, 0, 1);
      let toLoop = 0;
      for (let i = 0; i < loads.length; i++) {
        const l = loads[i], h = l.heat; if (!h) continue;
        const aW = st.alloc[i] * MWW;
        const over = l.nominal_mw > 0 ? Math.max(0, st.alloc[i] / l.nominal_mw - 1) : 0;
        const q = h.fraction * aW * (1 + PW.overdrive.heat_factor * over);
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
          if (over > 0) st.integrity[i] = Math.max(0, st.integrity[i] - dt * (PW.overdrive.wear_pct_per_min_at_150 / 60) * (over / 0.5));
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
      roomHeat[idx[L.systems.find((s) => s.id === PW.battery.system).compartment]] += st.batteryLossW || 0;
      // Radiators.
      const Tb4 = Math.pow(RD.background_k, 4);
      // Capacity by the fourth-power law; the bypass valve holds the loop near its nominal
      // temperature when the heat is low, so the loop does not run cold at cruise.
      loop.rad_cap_mw = (RD.rated_mw * loop.radiatorHealth * loop.flow * (Math.pow(loop.T, 4) - Tb4)) / (Math.pow(RD.rated_at_k, 4) - Tb4);
      const open = RD.bypass_band_k > 0 ? clamp((loop.T - (C.nominal_k - RD.bypass_band_k)) / RD.bypass_band_k, 0, 1) : 1;
      loop.rad_mw = Math.max(0, loop.rad_cap_mw) * open;
      loop.in_mw = toLoop / MWW;
      loop.T += (dt * (toLoop - loop.rad_mw * MWW)) / (C.capacity_mj_per_k * MWW);
    }
    function checkScram() {
      const rx = st.reactor, SC = PW.reactor.scram;
      if (rx.state !== "running") return;
      const tm = rx.timers;
      tm.loop = st.loop.T > SC.loop_over_k ? tm.loop + dt : 0;
      tm.flow = st.loop.flow < SC.coolant_flow_below && rx.throttle > SC.coolant_check_above_throttle ? tm.flow + dt : 0;
      tm.aux = auxRatio() < SC.aux_supply_below ? tm.aux + dt : 0;
      let cause = null;
      if (tm.loop >= SC.loop_over_hold_s) cause = "coolant loop over " + SC.loop_over_k + " K";
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
        const H = l.portal && l.portal.axis !== "y" ? l.portal.size_m[1] : Math.sqrt(l.area);
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
      if (st.makeupOn && P[D] / 1000 < mk.start_below_kpa && P[D] / 1000 > mk.stop_if_duct_below_kpa) {
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
          if (room < 1000 || pb / 1000 >= SA.pressure_kpa - 0.5) { bay.mode = "idle"; setLink(ventOf[i], 1); log(bay.target + " repressurized from the receiver"); bay.target = null; }
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
        if (f.hrr <= 0) {
          // Spread: hot air with fuel and oxygen ignites (deterministic threshold).
          if (T[i] > FI.autoignition_k && f.fuel > 0 && fO2(i) > 0) { f.hrr = FI.seed_kw * 1000; log("Fire spreads into " + comps[i].name); }
          continue;
        }
        const max = FI.hrr_max_kw_per_m2 * 1000 * floor[i] * fO2(i) * (f.fuel > 0 ? 1 : 0);
        let cut = 0;
        if (f.ext > 0) { cut += FI.extinguisher.hrr_cut_kw_per_s * 1000; f.ext -= dt; }
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
      derive();
    }
    function ignite(compId, kw) {
      const i = idx[compId], f = st.fire[i];
      if (f.fuel <= 0) return false;
      f.hrr = Math.max(f.hrr, (kw || FI.seed_kw) * 1000);
      log("Fire in " + comps[i].name + " (" + (f.hrr / 1000).toFixed(0) + " kW)");
      return true;
    }

    // ======================================================= automation
    function stepAutomation() {
      derive();
      // Vent dampers. Each trips shut on excess net flow, more than damper_trip_fraction_per_s
      // of the room's gas a second (a leak, or a fire's expansion; normal heating is a
      // hundred times less), and
      // stays shut until its room has held within damper_reset_kpa of the duct, without
      // falling, for damper_retry_s; it also shuts on low pressure, smoke, or no fan. A forced
      // state (venting, isolation, repressurizing) overrides; a forced-open repressurization
      // ends itself once the room is back to the duct's pressure.
      const fan = fanRatio();
      const ductLow = P[DUCT] / 1000 < VE.duct_low_kpa;
      for (let i = 0; i < N; i++) {
        const v = links[ventOf[i]];
        const dpk = (P[DUCT] - P[i]) / 1000;
        const forced = st.damperForced[comps[i].id];
        if (forced === true && st.repress[i] && P[i] / 1000 > AT.plant.makeup.target_kpa - VE.damper_reset_kpa) { delete st.damperForced[comps[i].id]; st.repress[i] = 0; st.trip[i] = 0; }
        else if (forced != null) { v.target = forced ? 1 : 0; st.prevRoomP[i] = P[i]; continue; }
        if (Math.abs(v.flow) > VE.damper_trip_fraction_per_s * ntot[i]) st.trip[i] = VE.damper_retry_s;
        else if (st.trip[i] > 0) {
          const falling = st.prevRoomP[i] - P[i] > VE.damper_falling_pa_per_s * dt;
          st.trip[i] = !falling && Math.abs(dpk) < VE.damper_reset_kpa ? st.trip[i] - dt : VE.damper_retry_s;
        }
        st.prevRoomP[i] = P[i];
        if (!st.damperAuto) continue;
        const ppm = (n[SMOKE][i] / Math.max(1e-9, ntot[i])) * 1e6;
        const shut = fan <= 0 || st.trip[i] > 0 || P[i] / 1000 < VE.damper_close_below_kpa || ppm > VE.damper_close_smoke_ppm || ductLow;
        v.target = shut ? 0 : 1;
      }
      // Doors between two compartments close themselves when either side falls below the
      // threshold, unless the damage control board holds them (a door to space is commanded).
      if (st.pdoorAuto) for (const l of links) {
        if (l.target <= 0 || l.b === SPACE || PK.auto_close_kinds.indexOf(l.kind) < 0 || l.manualHold) continue;
        if (Math.min(P[l.a], P[l.b]) / 1000 < PK.auto_close_below_kpa && Math.max(P[l.a], P[l.b]) / 1000 >= PK.auto_close_below_kpa) { l.target = 0; log(l.id + " closing itself (pressure alarm)"); }
      }
      // Water mist: automatic discharge on a fire above 1 MW in a protected room.
      for (const id of FI.suppression.water_mist.compartments) {
        const i = idx[id], f = st.fire[i];
        const WM = FI.suppression.water_mist;
        f.detect = f.hrr > WM.auto_above_kw * 1000 ? (f.detect || 0) + dt : 0;
        if (st.autoMist !== false && f.detect >= WM.confirm_s && f.mist <= 0 && f.mistLeft > 0) dischargeMist(comps[i].id);
      }
      // Portal motion.
      for (const l of links) {
        if (l.open === l.target) continue;
        const rate = l.move > 0 ? dt / l.move : 1;
        l.open = l.open < l.target ? Math.min(l.target, l.open + rate) : Math.max(l.target, l.open - rate);
      }
    }

    // ======================================================= crew
    const CE = AT.crew_effects, MET = AT.metabolism;
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
        if (T[i] < HT.cold_harm_below_k) c.hp -= HT.cold_harm_hp_per_s * dt;
        // Pressure.
        const PR = CE.pressure;
        if (pk < PR.armstrong_kpa) c.vac += dt / PR.vacuum_death_s; else c.vac = Math.max(0, c.vac - dt / PR.vacuum_death_s);
        if (pk < PR.impaired_below_kpa) impaired = true;
        if (c.lastP != null && (c.lastP - P[i]) / 1000 > PR.knockdown_drop_kpa_in_1s * dt) { c.hp -= PR.knockdown_hp * dt; }
        c.lastP = P[i];
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
    function compAt(p) { for (let i = 0; i < N; i++) for (const b of comps[i].boxes) if (inBox(b, p)) return i; return SPACE; }
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
      // March inward.
      let s = 0, cur = SPACE, enter = null, Ein = E;
      const segs = [];
      for (; s <= PG.march_max_m && E > 0.05; s += PG.march_step_m) {
        const p = [point[0] + dir[0] * s, point[1] + dir[1] * s, point[2] + dir[2] * s];
        const c = compAt(p);
        if (c !== cur) {
          if (cur !== SPACE) segs.push({ comp: cur, a: enter, b: p, e: Ein - E });
          if (c !== SPACE && segs.length > 0) E = Math.max(0, E - PG.bulkhead_mj);
          if (c !== SPACE && !out.breach && segs.length === 0) {
            const area = clamp(BR.m2_per_mj * E, BR.min_m2, BR.max_m2);
            out.breach = { compartment: comps[c].id, area_m2: area };
            breach(comps[c].id, area, p);
          }
          cur = c; enter = p; Ein = E;
        }
        E *= Math.exp(-PG.march_step_m / PG.decay_m);
      }
      if (cur !== SPACE) segs.push({ comp: cur, a: enter, b: [point[0] + dir[0] * s, point[1] + dir[1] * s, point[2] + dir[2] * s], e: Ein - E });
      for (const sg of segs) {
        if (sg.e <= 0.01) continue;
        const r = PG.radius_m + PG.radius_per_sqrt_mj * Math.sqrt(sg.e);
        out.compartments.push({ id: comps[sg.comp].id, mj: sg.e, radius_m: r });
        for (let li = 0; li < loads.length; li++) {
          const d = segSegDist(sg.a, sg.b, loads[li].center, loads[li].center);
          if (d > r) continue;
          const pts = DM.systems.points_per_mj * sg.e * (1 - d / r);
          st.integrity[li] = Math.max(0, st.integrity[li] - pts);
          out.systems.push({ id: loads[li].id, points: pts, integrity: st.integrity[li] });
        }
        for (const nd of PW.nodes) {
          if (nd.compartment !== comps[sg.comp].id) continue;
          const d = segSegDist(sg.a, sg.b, nd.center_m, nd.center_m);
          if (d > r) continue;
          const h = st.nodeHealth[nd.id] == null ? 1 : st.nodeHealth[nd.id];
          const left = Math.max(0, h - (DM.systems.points_per_mj * sg.e * (1 - d / r)) / 100);
          st.nodeHealth[nd.id] = left < DM.nodes.destroyed_below ? 0 : left;
          out.nodes = out.nodes || []; out.nodes.push({ id: nd.id, health: st.nodeHealth[nd.id] });
          if (st.nodeHealth[nd.id] === 0) log(nd.name + " destroyed");
        }
        for (const k of PW.conduits) {
          if (k.route.indexOf(comps[sg.comp].id) < 0) continue;
          let dmin = Infinity;
          for (let q = 0; q + 1 < k.path_m.length; q++) dmin = Math.min(dmin, segSegDist(sg.a, sg.b, k.path_m[q], k.path_m[q + 1]));
          if (dmin > r) continue;
          const cs = st.conduit[k.id];
          if (sg.e >= DM.conduits.sever_mj) { cs.severed = true; out.conduits.push({ id: k.id, severed: true }); log("Conduit " + k.id + " severed in " + comps[sg.comp].name); }
          else if (sg.e >= DM.conduits.damage_mj) { cs.health = Math.min(cs.health, DM.conduits.damaged_capacity); out.conduits.push({ id: k.id, severed: false, health: cs.health }); }
        }
        const chance = Math.min(DM.fire.chance_max, DM.fire.chance_per_mj * sg.e) * fO2(sg.comp);
        if (hash32(seed, id, comps[sg.comp].id, "fire") < chance) { ignite(comps[sg.comp].id, FI.seed_kw + DM.fire.seed_kw_per_mj * sg.e); out.fire.push(comps[sg.comp].id); }
        for (const c of st.crew) {
          if (c.comp !== sg.comp || c.dead) continue;
          const hp = DM.crew.hp_per_mj * sg.e * DM.crew.share; c.hp -= hp; out.crew.push({ id: c.id, hp });
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
      st.breaches.push(l); log("Breach " + area.toFixed(2) + " m^2 in " + comps[idx[compId]].name);
      return l;
    }
    function patch(linkId) { const l = linkById[linkId]; if (l) { l.target = 0; l.open = 0; l.area = 0; } }
    /** Refill a compartment from the duct through its vent (the damage control board's command). */
    function repressurize(compId) { const i = idx[compId]; st.damperForced[compId] = true; st.repress[i] = 1; st.trip[i] = 0; log("Repressurizing " + comps[i].name + " from the duct"); }
    function setLink(id, target) { const l = typeof id === "number" ? links[id] : linkById[id]; if (l) l.target = target ? 1 : 0; return l; }
    function closeAllDoors() { for (const l of links) if (["door", "hatch", "ladder", "hoist"].indexOf(l.kind) >= 0) l.target = 0; }
    function isolate(compId) { const i = idx[compId]; for (const l of links) if ((l.a === i || l.b === i) && l.kind !== "breach") { l.target = 0; l.open = 0; } st.damperForced[compId] = false; }
    function bayPumpdown(bayId) {
      const i = idx[bayId];
      for (const l of links) if ((l.a === i || l.b === i) && l.kind !== "breach" && l.kind !== "valve") l.target = 0;
      st.damperForced[bayId] = false;
      Object.assign(st.bay, { target: bayId, mode: "pumpdown", moved_mol: 0, energy_mj: 0, t0: st.t });
      log("Pump-down of " + comps[i].name + " started");
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
    function useExtinguisher(compId) { const f = st.fire[idx[compId]]; f.ext = FI.extinguisher.discharge_s; log("Extinguisher on the fire in " + comps[idx[compId]].name); }
    function ventCompartment(compId) {
      // Vent through the duct: shut the room's doors, isolate the duct, open this room's vent and the dump.
      const vi = idx[compId];
      for (const l of links) if ((l.a === vi || l.b === vi) && ["door", "pressure_door", "hatch", "ladder", "hoist"].indexOf(l.kind) >= 0) l.target = 0;
      for (let i = 0; i < N; i++) st.damperForced[comps[i].id] = comps[i].id === compId;
      setLink(GA.overboard_dump.id, 1);
      log("Venting " + comps[idx[compId]].name + " overboard through the duct");
    }
    function stopVent() { st.damperForced = {}; setLink(GA.overboard_dump.id, 0); }
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
      l.target = 1; l.manualHold = !!override || l.kind === "door" || l.kind === "hatch" || l.kind === "ladder";
      return "ok";
    }
    /** Engineering's priority for a load (1 to 3); the vital class 0 is fixed. */
    function setPriority(loadId, p) {
      const i = loadIdx[loadId]; if (i == null || loads[i].priority === 0) return false;
      st.priority[i] = clamp(Math.round(p), 1, 3); return true;
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
    function spliceConduit(id) { const c = st.conduit[id]; if (!c) return false; c.severed = false; c.health = Math.max(c.health, DM.repair.conduit_splice_capacity); return true; }
    function rebuildNode(id) { st.nodeHealth[id] = Math.max(st.nodeHealth[id] == null ? 1 : st.nodeHealth[id], DM.repair.node_rebuild_to); return true; }
    function log(msg) { st.log.push({ t: st.t, msg }); if (st.log.length > 200) st.log.shift(); }

    // ======================================================= the sub-step
    function step() {
      stepAutomation();
      stepPower();
      stepHeat();
      stepPlant();
      stepPumps();
      stepFire();
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
      step, derive, solveGrid, gridSnapshot, computeDemand, solveWithDropout, previewSetpoint, scram, scramReset,
      resolveHit, hullHalfBeam, breach, patch, setLink, closeAllDoors, isolate, ignite, bayPumpdown, bayRepress,
      dischargeMist, dischargeInert, useExtinguisher, gravityG, capability, damageState, auxRatio,
      operateDoor, setPriority, applyPreset, repressurize, repairSystem, spliceConduit, rebuildNode, fanRatio,
      airlockCycleOut, airlockCycleIn, ventCompartment, stopVent, addCrew, setActivity, compartmentReadout, lighting,
      receiverKpa, storeMol, fillStandard, log,
      get P() { return P; }, get T() { return T; }, get n() { return n; }, get ntot() { return ntot; },
    };
  }

  root.ShipSystems = { version: 1, create, measure, hash32 };
})(typeof window !== "undefined" ? window : globalThis);
