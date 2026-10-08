/*
 * shipwalk.js: walking a whole ship in first person, deck to deck, for the Star Crew mockups.
 *
 * What it owns: a crew member's body on the decks the way openspec/changes/crew-on-deck designs it, so the
 * owner can run round a ship (owner, 2026-10-06: "do you have fps mode in deck plan for me to run arond?").
 *   - The body is crew-on-deck's standing capsule (section 2): 0.25 m radius, 1.80 m tall, the eye at 1.65 m.
 *   - It moves at crew-on-deck's speeds (section 3): walk 1.8 m/s, run 4.0 m/s, 70 % of that backwards and
 *     on stairs; it speeds up at 20 m/s2 and brakes at 30 (the owner: "walking is to much like ice skating"). It
 *     steps up and down 0.35 m, so stairs and spiral stairs are walked, not animated; it jumps 0.46 m (Space; C5,
 *     the owner: "bring back jumping") and falls where there is no floor.
 *   - Doors (section 5): a door slides open as the body comes near and closes 2 s after it leaves; a pressure
 *     door opens and closes on E; a closed door is a wall; a lift's doors open with its car. The page draws the
 *     leaves from each door's set(open).
 *   - Ladders and floor hatches are climbed at 2.0 m/s up and 2.5 m/s down, 0.15 s on and off (section 4,
 *     snappy at the owner's word, 2026-10-07); a hatch in a wall with a sill too high to step over takes
 *     0.6 s; a lift rides at its fixture's speed and door time, and comes when the body walks up to one of
 *     its doors.
 *   - It moves by Rapier's kinematic character controller (section 3a: autostep, snap to ground, slopes,
 *     sliding along walls), over the rooms as they are drawn: every room's mesh as one static triangle mesh,
 *     with covers over the openings a body must not fall or walk through, and every stair as a ramp (the
 *     page leaves the steps out and passes the ramps as covers). The eye follows the feet smoothed. Without
 *     Rapier (o.rapier null: the CDN did not load) it falls back to the first controller over an Octree.
 * Input: W A S D or the arrows, the mouse (click to capture it, or drag), Shift to run, E to use (climb, go
 * through a hatch, take the lift up), Q to take the lift or a ladder down, Esc to stop. On a touch screen: a stick on the
 * left, drag on the right to look, and buttons.
 *
 * Why a lib of its own: every interior page can walk with it (CLAUDE.md 6.1). freecam.js's walk keeps the
 * eye over one floor and stays for the one-floor pages until they move onto this. Inlined by
 * tools/mockups/inline.py between the "INLINE lib:shipwalk" markers. Edit it here, never inside a page.
 */
(function () {
  "use strict";
  // The walk's numbers are data/crew/walk.json (crew-on-deck sections 2-5 and 3a), inlined into the page between the
  // "INLINE data:crew/walk" markers: the engine's sc-core::walk reads the same file, so the two walk alike.
  const WD = (() => {
    const el = document.getElementById("ship-data-walk");
    if (!el) throw new Error("shipwalk: no #ship-data-walk in the page (add <!-- INLINE data:crew/walk --> markers and run tools/mockups/inline.py)");
    return JSON.parse(el.textContent);
  })();
  const BODY = WD.body, MOVE = WD.move;   // the capsule; walk, run and back speeds, and the starting and stopping rates
  const LADDER = { up_m_s: WD.ladder.up_m_s, down_m_s: WD.ladder.down_m_s, mount_s: WD.ladder.mount_s, dismount_s: WD.ladder.dismount_s };
  const SIDE_HATCH_S = WD.side_hatch_s;   // a hatch in a wall
  // Doors (crew-on-deck section 5): a door opens when a body comes within zone_m of its plane, inside its width plus
  // zone_side_m, and closes close_after_s after the zone is empty; a pressure door opens and closes on E only.
  // Neither closes on a body in the doorway. Times are opening and closing, seconds.
  const DOOR = { zone_m: WD.door.zone_m, zone_side_m: WD.door.zone_side_m, close_after_s: WD.door.close_after_s,
    door_s: [WD.door.open_s, WD.door.close_s], pressure_s: [WD.door.pressure_open_s, WD.door.pressure_close_s],
    passable: WD.door.passable, thick_m: WD.door.thick_m };
  // Rapier's controller (crew-on-deck section 3a): the gap it keeps, the step it climbs over a ledge at least
  // step_depth_m deep, how far below it keeps the feet on the floor, and the slopes it climbs and slides on (the
  // Tern's straight stairs are 41-43 degrees and a spiral's walk line 44, steeper toward its column).
  const KCC = WD.controller;
  const EYE_TAU_S = WD.eye.tau_s, EYE_LAG_M = WD.eye.lag_m;   // the eye follows the feet's height smoothed, never further behind
  const GRAVITY_M_S2 = WD.gravity_m_s2, FALL_MAX_M_S = WD.fall_max_m_s;
  // A jump (owner, 2026-10-08: "bring back jumping"; crew-on-deck section 3): about 0.46 m at full gravity; for lift_s
  // after take-off the floor under the feet does not count as landing.
  const JUMP_M_S = WD.jump.speed_m_s, JUMP_LIFT_S = WD.jump.lift_s;
  const DROP_M = WD.drop_m;               // how far below the feet a walking body still finds its floor (a stair down)
  const REACH_M = WD.ladder.reach_m;      // how near a ladder's or hatch's centre the body stands to use it
  const CALL_M = WD.lift_call_m;          // how near a lift door calls the car
  const SUBSTEP_S = WD.substep_s;
  const LOOK_RAD_PX = 0.0022, TOUCH_LOOK_RAD_PX = 0.005, PITCH_MAX = 1.45, STICK_PX = 48;

  /** True when (x, z) is inside a polygon of [x, z] corners. */
  function inPoly(poly, x, z) {
    let inside = false;
    for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
      const [xi, zi] = poly[i], [xj, zj] = poly[j];
      if ((zi > z) !== (zj > z) && x < ((xj - xi) * (z - zi)) / (zj - zi) + xi) inside = !inside;
    }
    return inside;
  }
  /** Distance from (x, z) to a polygon's edges. */
  function polyDist(poly, x, z) {
    let d = Infinity;
    for (let i = 0; i < poly.length; i++) {
      const [ax, az] = poly[i], [bx, bz] = poly[(i + 1) % poly.length], ex = bx - ax, ez = bz - az;
      const t = Math.max(0, Math.min(1, ((x - ax) * ex + (z - az) * ez) / (ex * ex + ez * ez || 1)));
      d = Math.min(d, Math.hypot(x - ax - ex * t, z - az - ez * t));
    }
    return d;
  }

  /**
   * An Octree of triangles for the body to collide with: each entry of soups is a flat array of triangle
   * corners (x, y, z, nine numbers a triangle, world coordinates), and extra holds triangles [[x, y, z] x 3]
   * (covers). A cover meant to be stood on faces up: the body finds its floor with a ray that sees only faces
   * turned towards it.
   */
  function octreeOf(THREE, Octree, soups, extra) {
    const oc = new Octree(), V = (a, i) => new THREE.Vector3(a[i], a[i + 1], a[i + 2]);
    const add = (a, b, c) => {
      const t = new THREE.Triangle(a, b, c);
      if (t.getArea() > 1e-8) oc.addTriangle(t);
    };
    for (const a of soups) for (let i = 0; i + 8 < a.length; i += 9) add(V(a, i), V(a, i + 3), V(a, i + 6));
    for (const t of extra || []) add(...t.map((p) => new THREE.Vector3(p[0], p[1], p[2])));
    return oc.build();
  }

  /**
   * Rapier's world for the body to collide with: the same triangles as octreeOf, as one static triangle mesh.
   * R is the initialised Rapier module (@dimforge/rapier3d-compat).
   */
  function rapierWorld(R, soups, extra) {
    let n = 0;
    for (const a of soups) n += Math.floor(a.length / 9) * 9;
    n += (extra || []).length * 9;
    const v = new Float32Array(n);
    let k = 0;
    for (const a of soups) { const m = Math.floor(a.length / 9) * 9; v.set(a.subarray ? a.subarray(0, m) : a.slice(0, m), k); k += m; }
    for (const t of extra || []) for (const p of t) { v[k++] = p[0]; v[k++] = p[1]; v[k++] = p[2]; }
    const idx = new Uint32Array(n / 3);
    for (let i = 0; i < idx.length; i++) idx[i] = i;
    const world = new R.World({ x: 0, y: 0, z: 0 });
    world.createCollider(R.ColliderDesc.trimesh(v, idx));
    return world;
  }

  /**
   * A walking body for camera on canvas dom. o:
   *   octree: from octreeOf (floors under a point, standing room);
   *   rapier, world: the Rapier module and rapierWorld's world over the same triangles (null: the Octree moves the body);
   *   ladders: [{ x, z, lo, hi, name }]: floor openings climbed from floor lo to floor hi (metres);
   *   hatches: [{ x, z, n: [x, z], sill, name }]: wall hatches (n the wall's normal); those whose sill is
   *     more than a step above the floor on a side are gone through with E, the rest are walked through;
   *   lifts: [{ poly, stops, carY, speed_m_s, door_s, doors: [{ x, z, y }], onMove(y), name }]: a shaft's
   *     footprint, its stop heights, the car's floor now, and its doors (y their floor);
   *   where(x, feet, z) -> text: where the body is, for the HUD;
   *   buttons: [{ label() -> text, onClick() }]: the page's own buttons beside Stop;
   *   onExit(): the body was put away (Stop or Esc).
   * Returns { enter(pose), exit(), update(dt_s), active(), pose(), keys, use(dir), lifts }.
   */
  function create(THREE, lib, o) {
    const { Capsule } = lib;
    const camera = o.camera, dom = o.dom, octree = o.octree;
    const ladders = o.ladders || [], lifts = (o.lifts || []).map((l) => Object.assign({ phase: "idle", t: 0, target: l.carY }, l));
    // Doors: { kind "door" | "pressure_door" | "lift", c [x, y, z] (the opening's centre), n [x, z] (its normal), w, h,
    // name, set(open 0..1) }; a lift door also names its lift (an index into o.lifts) and stop (its floor's y).
    const doors = (o.doors || []).map((d) => Object.assign({ open: 0, target: 0, emptyT: Infinity, shown: -1 }, d));
    const pos = new THREE.Vector3(), cap = new Capsule(new THREE.Vector3(), new THREE.Vector3(), BODY.radius_m);
    const ray = new THREE.Ray(new THREE.Vector3(), new THREE.Vector3(0, -1, 0));
    const keys = {}, stick = { x: 0, y: 0 };
    let yaw = 0, pitch = 0, vx = 0, vz = 0, vy = 0, grounded = true, jumpT = 0, stairT = 0, action = null, on = false, runToggle = false, eyeY = 0;
    // Rapier: the body is a kinematic capsule its character controller moves. A lift's car floor is no collider: inside
    // the shaft the feet are held on it (kccMove), because a separate box flush with the deck jammed the controller at
    // the seam (owner, 2026-10-08: "I cant walk out when the doors are open").
    const R = o.rapier && o.world ? o.rapier : null, world = R ? o.world : null;
    const HALF = BODY.height_m / 2 - BODY.radius_m;
    let kBody = null, kCol = null, kcc = null;
    if (R) {
      kBody = world.createRigidBody(R.RigidBodyDesc.kinematicPositionBased());
      kCol = world.createCollider(R.ColliderDesc.capsule(HALF, BODY.radius_m), kBody);
      kcc = world.createCharacterController(KCC.offset_m);
      kcc.setUp({ x: 0, y: 1, z: 0 });
      kcc.enableAutostep(BODY.step_m, KCC.step_depth_m, false);
      kcc.enableSnapToGround(KCC.snap_m);
      kcc.setMaxSlopeClimbAngle((KCC.climb_deg * Math.PI) / 180);
      kcc.setMinSlopeSlideAngle((KCC.slide_deg * Math.PI) / 180);
      kcc.setSlideEnabled(true);
      // A closed door is a wall: a thin box in its opening, off while it stands open. A lift's landing door too, so the
      // shaft is shut wherever the car is not standing open (owner: "im stuck in the elevator well. this should be impossible").
      for (const d of doors) {
        const th = Math.atan2(d.n[0], d.n[1]);
        d.col = world.createCollider(R.ColliderDesc.cuboid(d.w / 2, d.h / 2, DOOR.thick_m / 2).setTranslation(d.c[0], d.c[1], d.c[2])
          .setRotation({ x: 0, y: Math.sin(th / 2), z: 0, w: Math.cos(th / 2) }));
      }
      world.step();
    }
    /** Put the capsule where pos (the feet) is, at once (a teleport: entering, a ladder, the lift). */
    function syncBody() {
      if (!R) return;
      kBody.setTranslation({ x: pos.x, y: pos.y + BODY.height_m / 2 + KCC.offset_m, z: pos.z }, true);
      world.step();
    }

    // ---------------------------------------------------------------- ground and walls
    /** The floor under (x, z) seen from `from` downwards, or null: a lift's car inside its shaft, else the octree. */
    function floorBelow(x, z, from) {
      for (const l of lifts) if (inPoly(l.poly, x, z)) return l.carY <= from + 1e-6 ? l.carY : null;
      ray.origin.set(x, from, z);
      const hit = octree.rayIntersect(ray);
      return hit ? hit.position.y : null;
    }
    /** The highest floor the body can stand on round (x, z): in [feet - down, feet + step], sampled under its centre and four points of its rim. */
    function groundAt(x, z, feet, down) {
      const r = BODY.radius_m * 0.8, top = feet + BODY.step_m + 0.02;
      let best = null;
      for (const [dx, dz] of [[0, 0], [r, 0], [-r, 0], [0, r], [0, -r]]) {
        const y = floorBelow(x + dx, z + dz, top);
        if (y !== null && y >= feet - down && (best === null || y > best)) best = y;
      }
      return best;
    }
    /** Push (x, z) out of the walls at the body's height above the step it may climb; returns the push. */
    function pushOut(x, z, feet) {
      let px = 0, pz = 0;
      for (let k = 0; k < 4; k++) {
        cap.start.set(x + px, feet + BODY.step_m + BODY.radius_m, z + pz);
        cap.end.set(x + px, feet + BODY.height_m - BODY.radius_m, z + pz);
        const r = octree.capsuleIntersect(cap);
        if (!r) break;
        const dx = r.normal.x * r.depth, dz = r.normal.z * r.depth;
        if (Math.abs(dx) + Math.abs(dz) < 1e-7) break;
        px += dx; pz += dz;
      }
      return [px, pz];
    }

    // ---------------------------------------------------------------- lifts
    const liftIn = (x, z) => lifts.find((l) => inPoly(l.poly, x, z)) || null;
    const boardable = (l) => l.phase === "idle" && Math.abs(l.carY - pos.y) < 0.05;
    function request(l, y) {
      if (l.phase !== "idle" || Math.abs(l.carY - y) < 1e-3) return;
      l.target = y; l.phase = "closing"; l.t = 0;
    }
    function liftTick(l, h) {
      if (l.phase === "idle") return;
      l.t += h;
      if (l.phase === "closing" && l.t >= l.door_s) { l.phase = "moving"; l.t = 0; }
      else if (l.phase === "moving") {
        const d = l.target - l.carY, s = l.speed_m_s * h;
        l.carY = Math.abs(d) <= s ? l.target : l.carY + Math.sign(d) * s;
        if (l.carY === l.target) { l.phase = "opening"; l.t = 0; }
        if (l.onMove) l.onMove(l.carY);
      } else if (l.phase === "opening" && l.t >= l.door_s) { l.phase = "idle"; l.t = 0; }
    }
    function liftCalls() {
      if (liftIn(pos.x, pos.z)) return;
      for (const l of lifts) for (const d of l.doors) {
        if (Math.abs(d.y - pos.y) < 0.5 && Math.hypot(d.x - pos.x, d.z - pos.z) < CALL_M) request(l, d.y);
      }
    }
    /** Inside a car, keep the body clear of the open side before it moves. */
    function keepInside(l) {
      const xs = l.poly.map((p) => p[0]), zs = l.poly.map((p) => p[1]), m = BODY.radius_m + 0.02;
      pos.x = Math.min(Math.max(pos.x, Math.min(...xs) + m), Math.max(...xs) - m);
      pos.z = Math.min(Math.max(pos.z, Math.min(...zs) + m), Math.max(...zs) - m);
      vx = vz = 0; syncBody();
    }

    // ---------------------------------------------------------------- doors
    /** Where the body stands against a door: along its width, across its plane, and whether on its floor. */
    function doorRel(d) {
      const rx = pos.x - d.c[0], rz = pos.z - d.c[2];
      return { along: rx * -d.n[1] + rz * d.n[0], across: rx * d.n[0] + rz * d.n[1], floor: Math.abs(pos.y - (d.c[1] - d.h / 2)) < 1.0 };
    }
    function inDoorway(d) { const r = doorRel(d); return r.floor && Math.abs(r.across) < BODY.radius_m + 0.15 && Math.abs(r.along) < d.w / 2 + BODY.radius_m; }
    function doorTick(d, h) {
      if (d.kind === "lift") {
        const l = lifts[d.lift], here = Math.abs(l.carY - d.stop) < 0.05;
        d.open = !here ? 0 : l.phase === "idle" ? 1 : l.phase === "opening" ? Math.min(1, l.t / l.door_s) : l.phase === "closing" ? Math.max(0, 1 - l.t / l.door_s) : 0;
        if (d.col) d.col.setEnabled(d.open < DOOR.passable);
      } else {
        const r = doorRel(d), busy = on && inDoorway(d);
        if (d.kind === "door") {
          const near = on && r.floor && Math.abs(r.across) < DOOR.zone_m && Math.abs(r.along) < d.w / 2 + DOOR.zone_side_m;
          if (near) { d.emptyT = 0; d.target = 1; } else { d.emptyT += h; if (d.emptyT >= DOOR.close_after_s) d.target = 0; }
        }
        if (busy && d.open > 0) d.target = 1;   // a closing leaf stops and reopens for a body in the doorway; a shut one stays shut
        const [up, down] = d.kind === "pressure_door" ? DOOR.pressure_s : DOOR.door_s;
        d.open = d.target > d.open ? Math.min(d.target, d.open + h / up) : Math.max(d.target, d.open - h / down);
        if (d.col) d.col.setEnabled(d.open < DOOR.passable);
      }
      if (Math.abs(d.open - d.shown) > 1e-4) { d.shown = d.open; if (d.set) d.set(d.open); }
    }

    // ---------------------------------------------------------------- climbing and hatches (moves the body along set points)
    function run(segs) { action = { segs, i: 0, t: 0, from: pos.clone() }; vx = vz = vy = 0; }
    function actionTick(h) {
      const a = action, s = a.segs[a.i];
      a.t += h;
      const k = Math.min(1, a.t / s.dur);
      pos.lerpVectors(a.from, s.to, k);
      if (k < 1) return;
      a.i++; a.t = 0; a.from = pos.clone();
      if (a.i < a.segs.length) return;
      action = null; grounded = true; vy = 0;
      const [px, pz] = pushOut(pos.x, pos.z, pos.y);
      pos.x += px; pos.z += pz;
      syncBody();
    }
    const P = (x, y, z) => new THREE.Vector3(x, y, z);
    function climb(l, up) {
      const a = up ? l.lo : l.hi, b = up ? l.hi : l.lo, speed = up ? LADDER.up_m_s : LADDER.down_m_s;
      run([
        { dur: LADDER.mount_s, to: P(l.x, a, l.z) },
        { dur: Math.abs(b - a) / speed, to: P(l.x, b, l.z) },
        { dur: LADDER.dismount_s, to: P(l.x, b, l.z) },
      ]);
    }
    // A wall hatch's two sides: 0.7 m out from its centre each way, and the floor there (found once, from the octree).
    const hatches = (o.hatches || []).map((h) => {
      const side = (s) => {
        const x = h.x + h.n[0] * 0.7 * s, z = h.z + h.n[1] * 0.7 * s;
        ray.origin.set(x, h.sill + 0.3, z);
        const hit = octree.rayIntersect(ray);
        return { x, z, y: hit ? hit.position.y : null };
      };
      const sides = [side(-1), side(1)];
      return Object.assign({ sides, needsUse: sides.some((s) => s.y === null || h.sill - s.y > BODY.step_m) }, h);
    }).filter((h) => h.needsUse && h.sides.every((s) => s.y !== null));
    function through(h, from) {
      const to = h.sides[1 - from];
      run([{ dur: SIDE_HATCH_S, to: P(to.x, to.y, to.z) }]);
    }
    /** What E does here, or null: { label, go }; with down, a ladder down before one up (Q, where a trunk goes both ways:
     * the engine's sc-core::walk has the same rule). */
    function choice(down) {
      if (action) return null;
      const l = liftIn(pos.x, pos.z);
      if (l) return null;   // a car has its own buttons
      const near = ladders.filter((d) => Math.hypot(d.x - pos.x, d.z - pos.z) <= REACH_M);
      for (const d of near.slice().sort((a, b) => (Math.abs(pos.y - a.hi) < 0.4) === down ? -1 : (Math.abs(pos.y - b.hi) < 0.4) === down ? 1 : 0)) {
        if (Math.abs(pos.y - d.lo) < 0.4) return { label: "Climb up" + (d.name ? " to " + d.name.up : ""), go: () => climb(d, true) };
        if (Math.abs(pos.y - d.hi) < 0.4) return { label: "Climb down" + (d.name ? " to " + d.name.down : ""), go: () => climb(d, false) };
      }
      for (const d of doors) {
        if (d.kind !== "pressure_door") continue;
        const r = doorRel(d);
        if (!r.floor || Math.abs(r.across) > 1.5 || Math.abs(r.along) > d.w / 2 + 0.3) continue;
        const opening = d.target < 0.5;
        if (!opening && inDoorway(d)) continue;
        return { label: (opening ? "Open the pressure door" : "Close the pressure door") + (d.name ? " to " + d.name : ""), go: () => { d.target = opening ? 1 : 0; } };
      }
      for (const h of hatches) {
        if (Math.hypot(h.x - pos.x, h.z - pos.z) > REACH_M + 0.2) continue;
        const s = (pos.x - h.x) * h.n[0] + (pos.z - h.z) * h.n[1] < 0 ? 0 : 1;
        if (Math.abs(pos.y - h.sides[s].y) < 0.5) return { label: "Through the hatch" + (h.name ? " to " + h.name[1 - s] : ""), go: () => through(h, s) };
      }
      return null;
    }
    /** E (dir 1) or Q (dir -1): in a car, up or down a stop; elsewhere E climbs or goes through. */
    function use(dir) {
      if (!on || action) return;
      const l = liftIn(pos.x, pos.z);
      if (l) {
        if (l.phase !== "idle") return;
        const i = l.stops.reduce((b, y, k) => (Math.abs(y - l.carY) < Math.abs(l.stops[b] - l.carY) ? k : b), 0);
        const j = i + dir;
        if (j < 0 || j >= l.stops.length) return;
        keepInside(l); request(l, l.stops[j]);
        return;
      }
      const c = choice(dir < 0);
      if (c && (dir > 0 || /^Climb down/.test(c.label))) c.go();
    }

    // ---------------------------------------------------------------- the step
    function step(h) {
      jumpT = Math.max(0, jumpT - h);
      for (const l of lifts) liftTick(l, h);
      for (const d of doors) doorTick(d, h);
      if (action) { actionTick(h); return; }
      liftCalls();
      let f = 0, s = 0;
      if (keys.KeyW || keys.ArrowUp) f += 1;
      if (keys.KeyS || keys.ArrowDown) f -= 1;
      if (keys.KeyD || keys.ArrowRight) s += 1;
      if (keys.KeyA || keys.ArrowLeft) s -= 1;
      f -= stick.y; s += stick.x;
      const mag = Math.hypot(f, s);
      if (mag > 1) { f /= mag; s /= mag; }
      let speed = keys.ShiftLeft || keys.ShiftRight || runToggle ? MOVE.run_m_s : MOVE.walk_m_s;
      if (f < -0.1) speed *= MOVE.back_scale;
      if (stairT > 0 && !R) speed *= MOVE.stair_scale;   // Rapier's controller already slows the body on a slope
      const fx = -Math.sin(yaw), fz = -Math.cos(yaw), rx = Math.cos(yaw), rz = -Math.sin(yaw);
      const tx = (fx * f + rx * s) * speed, tz = (fz * f + rz * s) * speed;
      if (grounded) {
        let dx = tx - vx, dz = tz - vz;
        // Braking (slowing, or turning against the way the body moves) takes the stopping rate; speeding up the other.
        const braking = tx * tx + tz * tz < vx * vx + vz * vz || tx * vx + tz * vz < 0;
        const dl = Math.hypot(dx, dz), capV = (braking ? MOVE.stop_m_s2 : MOVE.accel_m_s2) * h;
        if (dl > capV) { dx *= capV / dl; dz *= capV / dl; }
        vx += dx; vz += dz;
      }
      let nx = pos.x + vx * h, nz = pos.z + vz * h;
      // A lift's doors: closed unless its car stands here, open. A step toward a shut shaft stops; a step away from it
      // never does, so a body left at the door when the car goes can walk off.
      const inCar = liftIn(pos.x, pos.z);
      for (const l of lifts) {
        const dn = inPoly(l.poly, nx, nz) ? -1 : polyDist(l.poly, nx, nz), d0 = inPoly(l.poly, pos.x, pos.z) ? -1 : polyDist(l.poly, pos.x, pos.z);
        const near = dn < BODY.radius_m * 0.6 && dn < d0;
        if (l !== inCar && near && !boardable(l)) { nx = pos.x; nz = pos.z; vx = vz = 0; }
        if (l === inCar && !boardable(l) && !inPoly(l.poly, nx, nz)) { nx = pos.x; nz = pos.z; vx = vz = 0; }
      }
      if (R) { kccMove(nx - pos.x, nz - pos.z, h, inCar); return; }
      const [px, pz] = pushOut(nx, nz, pos.y);
      if (px || pz) {
        nx += px; nz += pz;
        const pl = Math.hypot(px, pz), ux = px / pl, uz = pz / pl, vn = vx * ux + vz * uz;
        if (vn < 0) { vx -= vn * ux; vz -= vn * uz; }   // slide along the wall
      }
      if (grounded) {
        const g = groundAt(nx, nz, pos.y, DROP_M);
        if (g !== null) {
          if (Math.abs(g - pos.y) > 0.02 && !liftIn(nx, nz)) stairT = 0.35;
          pos.y = g; vy = 0;
        } else grounded = false;
      }
      if (!grounded) {
        vy = Math.max(vy - GRAVITY_M_S2 * h, -FALL_MAX_M_S);
        const ny = pos.y + vy * h, g = groundAt(nx, nz, pos.y, pos.y - ny + 0.02);
        if (g !== null && g >= ny) { pos.y = g; vy = 0; grounded = true; } else pos.y = ny;
      }
      stairT = Math.max(0, stairT - h);
      pos.x = nx; pos.z = nz;
    }

    /** One step by Rapier's character controller: the wanted move (dx, dz) plus gravity, corrected against the world. */
    function kccMove(dx, dz, h, inCar) {
      if (inCar && inCar.phase === "moving") {   // riding: the car carries the feet
        pos.y = inCar.carY; vy = 0; grounded = true; syncBody(); return;
      }
      // In a car's shaft nothing is under the feet but the car: no pull down while standing, and the car's floor stops a fall.
      vy = grounded ? (inCar ? 0 : -0.5) : Math.max(vy - GRAVITY_M_S2 * h, -FALL_MAX_M_S);
      kcc.computeColliderMovement(kCol, { x: dx, y: vy * h, z: dz });
      const m = kcc.computedMovement(), t = kBody.translation();
      const nt = { x: t.x + m.x, y: t.y + m.y, z: t.z + m.z }, up = BODY.height_m / 2 + KCC.offset_m;
      const onCar = inCar && nt.y - up <= inCar.carY + 1e-4;
      if (onCar) nt.y = inCar.carY + up;
      kBody.setNextKinematicTranslation(nt);
      world.step();
      const was = pos.y;
      pos.set(nt.x, nt.y - up, nt.z);
      grounded = (onCar || kcc.computedGrounded()) && jumpT <= 0;
      if (grounded) vy = 0;
      else if (vy > 0 && m.y < vy * h * 0.5) vy = 0;   // the head met a ceiling
      // A wall (a near-vertical contact): drop the part of the velocity into it, keep the part along it. A floor, a
      // ramp or a stair's slope is not a wall, and slows nothing.
      for (let i = 0; i < kcc.numComputedCollisions(); i++) {
        const n = kcc.computedCollision(i).normal1, hl = Math.hypot(n.x, n.z);
        if (Math.abs(n.y) > 0.3 || hl < 1e-6) continue;
        const ux = n.x / hl, uz = n.z / hl, into = vx * ux + vz * uz;
        if (into < 0) { vx -= into * ux; vz -= into * uz; }
      }
      if (grounded && h > 0 && Math.abs(pos.y - was) / h > 0.3) stairT = 0.35;   // climbing or descending: a stair's pace
    }

    // ---------------------------------------------------------------- HUD and input
    let hud = null, hudT = 0;
    const touch = matchMedia("(pointer: coarse)").matches || "ontouchstart" in window;
    function el(tag, cls, parent, text) { const e = document.createElement(tag); if (cls) e.className = cls; if (text) e.textContent = text; (parent || document.body).appendChild(e); return e; }
    function buildHud() {
      if (!document.getElementById("shipwalk-css")) {
        const css = el("style"); css.id = "shipwalk-css";
        css.textContent =
          ".sw-hud{position:fixed;inset:0;pointer-events:none;z-index:60;font:13px/1.35 system-ui,sans-serif;color:#e8eef5}" +
          ".sw-where{position:absolute;left:12px;top:10px;background:rgba(8,11,16,.72);border:1px solid #2a3542;border-radius:6px;padding:5px 10px}" +
          ".sw-where b{color:#48d6ff;font-weight:600}" +
          ".sw-btns{position:absolute;right:12px;top:10px;display:flex;gap:6px;pointer-events:auto}" +
          ".sw-btns button,.sw-act button{background:#141a22;color:#e8eef5;border:1px solid #3a4654;border-radius:6px;padding:7px 12px;font:600 13px system-ui,sans-serif;cursor:pointer}" +
          ".sw-btns button:hover,.sw-act button:hover{border-color:#48d6ff;color:#48d6ff}" +
          ".sw-btns button.on,.sw-act button.on{border-color:#48d6ff;color:#48d6ff}" +
          ".sw-cross{position:absolute;left:50%;top:50%;width:6px;height:6px;margin:-3px 0 0 -3px;border-radius:50%;background:rgba(232,238,245,.8);box-shadow:0 0 0 1px rgba(0,0,0,.6)}" +
          ".sw-prompt{position:absolute;left:50%;bottom:26%;transform:translateX(-50%);background:rgba(8,11,16,.8);border:1px solid #48d6ff;border-radius:6px;padding:6px 12px;color:#48d6ff;white-space:nowrap}" +
          ".sw-keys{position:absolute;left:12px;bottom:10px;color:#8a98a8;font-size:12px;background:rgba(8,11,16,.6);border-radius:6px;padding:4px 8px}" +
          ".sw-act{position:absolute;right:14px;bottom:18px;display:flex;flex-direction:column;align-items:flex-end;gap:8px;pointer-events:auto}" +
          ".sw-act button{min-width:84px;padding:12px 14px;font-size:14px;border-radius:10px;touch-action:none}" +
          ".sw-stick{position:absolute;left:28px;bottom:28px;width:112px;height:112px;border-radius:50%;border:2px solid rgba(232,238,245,.35);background:rgba(8,11,16,.35)}" +
          ".sw-knob{position:absolute;left:50%;top:50%;width:46px;height:46px;margin:-23px 0 0 -23px;border-radius:50%;background:rgba(232,238,245,.55)}";
      }
      const root = el("div", "sw-hud");
      const where = el("div", "sw-where", root);
      const btns = el("div", "sw-btns", root);
      const pageBtns = (o.buttons || []).map((b) => { const e = el("button", "", btns); e.type = "button"; e.onclick = () => { b.onClick(); refresh(); }; return { b, e }; });
      const stop = el("button", "", btns, "Stop walking"); stop.type = "button"; stop.onclick = () => exit();
      el("div", "sw-cross", root);
      const prompt = el("div", "sw-prompt", root); prompt.hidden = true;
      let act = null, stickEl = null, knob = null, runB = null, useB = null, upB = null, downB = null;
      if (touch) {
        stickEl = el("div", "sw-stick", root); knob = el("div", "sw-knob", stickEl);
        act = el("div", "sw-act", root);
        upB = el("button", "", act, "Lift up"); downB = el("button", "", act, "Lift down");
        useB = el("button", "", act, "Use"); runB = el("button", "", act, "Run");
        for (const b of [upB, downB, useB, runB]) b.type = "button";
        upB.onclick = () => use(1); downB.onclick = () => use(-1); useB.onclick = () => use(1);
        runB.onclick = () => { runToggle = !runToggle; runB.classList.toggle("on", runToggle); };
      } else el("div", "sw-keys", root, "W A S D move, mouse looks (click to capture), Shift run, Space jump, E use, Q down (a lift or a ladder), Esc stop");
      function refresh() { for (const { b, e } of pageBtns) e.textContent = b.label(); }
      refresh();
      return { root, where, prompt, upB, downB, useB, knob, refresh };
    }
    function hudTick(dt) {
      if (!hud) return;
      hudT -= dt;
      if (hudT <= 0) {
        hudT = 0.2;
        const w = o.where ? o.where(pos.x, pos.y, pos.z) : "";
        hud.where.innerHTML = w;
      }
      const l = liftIn(pos.x, pos.z), c = choice();
      let text = c ? (touch ? c.label : "E: " + c.label) : "";
      const busy = lifts.find((x) => x.phase !== "idle");
      if (busy) {
        const what = { closing: "doors closing", moving: "moving", opening: "doors opening" }[busy.phase];
        text = `${busy.name || "Lift"}: ${what}`;
      } else if (l) text = touch ? "" : "E: lift up, Q: lift down";
      hud.prompt.hidden = !text; hud.prompt.textContent = text;
      if (hud.useB) {
        hud.useB.hidden = !c; if (c) hud.useB.textContent = c.label;
        hud.upB.hidden = hud.downB.hidden = !(l && l.phase === "idle");
      }
    }
    /** Leave the floor, if standing on one and not climbing, riding or busy. */
    function jump() {
      if (!grounded || action || liftIn(pos.x, pos.z)) return;
      vy = JUMP_M_S; grounded = false; jumpT = JUMP_LIFT_S;
    }
    function onKey(e, down) {
      if (!on) return;
      if (/^(Arrow|Space)/.test(e.code)) e.preventDefault();
      keys[e.code] = down;
      if (!down) return;
      if (e.code === "Space") jump();
      if (e.code === "KeyE") use(1);
      if (e.code === "KeyQ") use(-1);
      if (e.code === "Escape" && document.pointerLockElement !== dom) exit();
    }
    const kd = (e) => onKey(e, true), ku = (e) => onKey(e, false), blur = () => { for (const k of Object.keys(keys)) keys[k] = false; };
    const look = (dx, dy, k) => { yaw -= dx * k; pitch = Math.max(-PITCH_MAX, Math.min(PITCH_MAX, pitch - dy * k)); };
    const mm = (e) => { if (on && document.pointerLockElement === dom) look(e.movementX, e.movementY, LOOK_RAD_PX); };
    const ptr = new Map();   // pointerId -> { kind: "look" | "stick", x, y, x0, y0 }
    function pd(e) {
      if (!on) return;
      if (e.pointerType === "mouse") {
        if (document.pointerLockElement !== dom && dom.requestPointerLock) { try { const r = dom.requestPointerLock(); if (r && r.catch) r.catch(() => {}); } catch (_) { /* drag to look instead */ } }
        ptr.set(e.pointerId, { kind: "look", x: e.clientX, y: e.clientY });
        return;
      }
      const isStick = e.clientX < innerWidth * 0.45 && e.clientY > innerHeight * 0.35 && ![...ptr.values()].some((p) => p.kind === "stick");
      ptr.set(e.pointerId, { kind: isStick ? "stick" : "look", x: e.clientX, y: e.clientY, x0: e.clientX, y0: e.clientY });
      try { dom.setPointerCapture(e.pointerId); } catch (_) { /* not every pointer can be captured */ }
    }
    function pm(e) {
      const p = ptr.get(e.pointerId);
      if (!on || !p) return;
      if (p.kind === "look") {
        if (document.pointerLockElement !== dom) look(e.clientX - p.x, e.clientY - p.y, e.pointerType === "mouse" ? LOOK_RAD_PX * 1.6 : TOUCH_LOOK_RAD_PX);
        p.x = e.clientX; p.y = e.clientY;
      } else {
        let dx = (e.clientX - p.x0) / STICK_PX, dy = (e.clientY - p.y0) / STICK_PX;
        const m = Math.hypot(dx, dy);
        if (m > 1) { dx /= m; dy /= m; }
        stick.x = dx; stick.y = dy;
        if (hud && hud.knob) hud.knob.style.transform = `translate(${dx * 34}px, ${dy * 34}px)`;
      }
    }
    function pu(e) {
      const p = ptr.get(e.pointerId);
      ptr.delete(e.pointerId);
      if (p && p.kind === "stick") { stick.x = stick.y = 0; if (hud && hud.knob) hud.knob.style.transform = ""; }
    }

    // ---------------------------------------------------------------- the public face
    /** Put the body at { x, y (its floor, roughly), z } facing face [x, z] (or yaw), pitch_deg up. */
    function enter(p) {
      on = true; action = null; vx = vz = vy = 0; grounded = true; stairT = 0;
      for (const k of Object.keys(keys)) keys[k] = false;
      pos.set(p.x, p.y, p.z);
      const g = groundAt(p.x, p.z, p.y, 1.0);   // the floor at about p.y, not the top of a chair beside it
      if (g !== null) pos.y = g;
      eyeY = pos.y;
      syncBody();
      yaw = p.face ? Math.atan2(-p.face[0], -p.face[1]) : p.yaw || 0;
      pitch = ((p.pitch_deg || 0) * Math.PI) / 180;
      if (!hud) hud = buildHud();
      addEventListener("keydown", kd); addEventListener("keyup", ku); addEventListener("blur", blur);
      document.addEventListener("mousemove", mm);
      dom.addEventListener("pointerdown", pd); dom.addEventListener("pointermove", pm);
      dom.addEventListener("pointerup", pu); dom.addEventListener("pointercancel", pu);
      dom.style.touchAction = "none";
      place();
    }
    function exit() {
      if (!on) return;
      on = false;
      for (const d of doors) { d.open = d.target = 0; d.emptyT = Infinity; d.shown = 0; if (d.col) d.col.setEnabled(true); if (d.set) d.set(0); }
      removeEventListener("keydown", kd); removeEventListener("keyup", ku); removeEventListener("blur", blur);
      document.removeEventListener("mousemove", mm);
      dom.removeEventListener("pointerdown", pd); dom.removeEventListener("pointermove", pm);
      dom.removeEventListener("pointerup", pu); dom.removeEventListener("pointercancel", pu);
      ptr.clear(); stick.x = stick.y = 0;
      if (document.pointerLockElement === dom && document.exitPointerLock) document.exitPointerLock();
      if (hud) { hud.root.remove(); hud = null; }
      if (o.onExit) o.onExit();
    }
    function place() {
      camera.position.set(pos.x, eyeY + BODY.eye_m, pos.z);
      camera.rotation.set(pitch, yaw, 0, "YXZ");
    }
    /** Advance dt_s seconds (in steps of 1/120 s) and place the camera at the eye. */
    function update(dt) {
      if (!on) return;
      dt = Math.min(dt, 0.1);
      const n = Math.max(1, Math.ceil(dt / SUBSTEP_S));
      for (let i = 0; i < n; i++) step(dt / n);
      // The eye: straight with the feet on a ladder or in a lift, else smoothed over a step or a ledge.
      if (action || lifts.some((l) => l.phase === "moving" && liftIn(pos.x, pos.z) === l)) eyeY = pos.y;
      else {
        eyeY += (pos.y - eyeY) * (1 - Math.exp(-dt / EYE_TAU_S));
        eyeY = Math.max(pos.y - EYE_LAG_M, Math.min(pos.y + EYE_LAG_M, eyeY));
      }
      place();
      hudTick(dt);
    }
    return {
      enter, exit, update, use, keys, lifts, doors,
      active: () => on,
      /** Where the body stands and looks: { x, y (feet), z, yaw, pitch, grounded, climbing, eye (the camera's height) }. */
      pose: () => ({ x: pos.x, y: pos.y, z: pos.z, yaw, pitch, grounded, climbing: !!action, eye: eyeY + BODY.eye_m }),
      /** True when a body can stand at (x, z) on a floor within a step of feet: nothing on top of the floor there
       * (a table, a machine) and no wall in its way. */
      canStand: (x, z, feet) => {
        ray.origin.set(x, feet + 2.9, z);   // above a machine's top (2.7 m at most), under the ceiling (3.0 m)
        const hit = octree.rayIntersect(ray);
        if (!hit || Math.abs(hit.position.y - feet) > BODY.step_m) return false;
        const [px, pz] = pushOut(x, z, hit.position.y);
        return Math.hypot(px, pz) < 0.02;
      },
      /** Turn the body to face [x, z]. */
      face: (f) => { yaw = Math.atan2(-f[0], -f[1]); },
      /** Tilt the view: radians, up positive, within the pitch limits. */
      look: (p) => { pitch = Math.max(-PITCH_MAX, Math.min(PITCH_MAX, p)); },
      refreshButtons: () => hud && hud.refresh(),
    };
  }

  window.ShipWalk = { create, octreeOf, rapierWorld, BODY, MOVE, LADDER, KCC, inPoly };
})();
