# Tasks

## 1. Data

- [ ] 1.1 `data/weapons.json`: twin pulse cannon, Swift guns, Jackal guns, Hound turrets and point defence; Gannet and Lance (design sections 2, 10, 13), units in keys.
- [ ] 1.2 `data/ships/tern/shields.json` (section 11) and `data/ships/tern/magazine.json` (section 10).
- [ ] 1.3 `data/enemies.json`: Jackal, Hound, target drone (section 13).
- [ ] 1.4 Mission rosters for missions 1-4 in the format the missions change defines.

## 2. Tools

- [ ] 2.1 `sc-tools` arc baker: from the layout's hull loft and mounts, write each turret's 72 x 20 mask, report the coverage table of section 3, and fail when any direction has fewer than two turrets (a measurement instrument; the scratch figures in the design come from the same ray test).

## 3. Core

- [ ] 3.1 `weapons::fire` (the one fire rule), the projectile pool of 1,024 and its 100 m grid sweep.
- [ ] 3.2 `weapons::lead` (the solver) and `weapons::hit_chance` (the estimate), and nothing else that computes either.
- [ ] 3.3 Manned turret control (aim point, slew limits, gunner's measured aim error) and automated turret control (priorities, modes, reaction, aim error, fire discipline, core load).
- [ ] 3.4 Point-defence bolts (proximity fuse) and missile fragility.
- [ ] 3.5 Magazine, hoist, ready racks, loader, tube state machine, jam rolls, crew actions at the breech.
- [ ] 3.6 Missiles: launch hand-off via `ship-frames`, motor, proportional navigation, seeker, fuse, warhead falloff, self-destruct.
- [ ] 3.7 `shields`: ellipsoid, face selection, capacity by weights and generator health, regeneration, shunting, holding decay, bands and retuning.
- [ ] 3.8 `damage::resolve` (the one damage resolution) and the `HullHit` report to `damage-control`; the shake input to `ship-frames`.
- [ ] 3.9 Enemy behaviour: Jackal state machine, Hound orbit and facing, retreat rules, seeded timers.

## 4. Tests

- [ ] 4.1 `a_full_capacitor_gives_twelve_seconds_of_full_rate_fire` and `heat_locks_a_turret_after_fifteen_seconds_from_cold`.
- [ ] 4.2 `the_hit_chance_estimate_is_calibrated` (seeded engagements, bands within 5 points).
- [ ] 4.3 `two_turrets_stop_about_four_missiles_in_five`.
- [ ] 4.4 `a_side_hit_near_the_bow_is_a_port_hit`.
- [ ] 4.5 `only_overflow_reaches_the_hull` and `collisions_bypass_shields`.
- [ ] 4.6 `only_crew_clear_a_jammed_tube` and the tube state transitions table, every edge.
- [ ] 4.7 `a_gannet_reaches_five_km_in_about_ten_seconds`.
- [ ] 4.8 `jackal_groups_do_not_move_in_lockstep` (seeded replay identical).
- [ ] 4.9 `every_direction_is_covered_by_two_turrets` (from the arc baker's output on the real layout).

## 5. Client and render

- [ ] 5.1 Turret sight UI (reticle, lead pip, hit chance, capacitor, heat, arc edge, threat arrows).
- [ ] 5.2 Instanced projectiles and exhaust billboards (one call), missile instances, the shield flash ellipsoid with six face uniforms.
- [ ] 5.3 Fired and hit events with `netcode-and-sessions` (about 4.5 kbit/s in a full engagement, added to its table).

## 6. Verification

- [ ] 6.1 Capture of a mission 3 engagement from the bridge and from the dorsal sight, beside the mockup's `broadside` and `gunner-view`.
- [ ] 6.2 Move each requirement into `openspec/specs/weapons-and-shields/spec.md` with the test that proves it.

## 7. Mockup (documentation, done with this write-up)

- [x] 7.1 `docs/mockups/exterior.html` shots `broadside`, `missile-launch`, `gunner-view`, `shields-hit`, `chase`.
