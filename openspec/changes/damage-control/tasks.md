# Tasks

## 1. Data and layout

- [ ] 1.1 Accept `data/ships/tern/damage.json` (proposed here, schema `starcrew.ship-damage/1`) and the `fire` block of `atmosphere.json`.
- [ ] 1.2 Apply the layout patches of design section 15 through `reference-ship-tern` (damage control lockers, EVA suits, spare parts).
- [ ] 1.3 A data check: hull sections derive from the layout's hull; every room named in fuel loads and suppression exists; suppression stores exist.

## 2. Core (`sc-core::damage`, `sc-core::fire`, `sc-core::repair`)

- [ ] 2.1 `damage::resolve(ship, HullHit)`: hull section and face, armour and wear, march with decay and bulkheads, breach, deposits with falloff on loads, nodes, conduits and doors, seeded fire roll, crew by position (design section 1). One function for every ship and craft.
- [ ] 2.2 Per-compartment target lists precomputed at load; at most four penetrating hits resolved per tick, the rest queued in order.
- [ ] 2.3 System states and capability (section 2), read by `power-grid`.
- [ ] 2.4 Fire: ignition, t-squared growth, ceiling by floor and oxygen, decay, chemistry into `life-support`, damage to what is in the room, spread by hot air (section 3).
- [ ] 2.5 Suppression: extinguishers, water mist (automatic and manual), inert gas, venting (section 4).
- [ ] 2.6 The magazine's cook-off (section 5).
- [ ] 2.7 Repair jobs and `damage::repair_time` (section 6); parts and kits as counted stores.
- [ ] 2.8 Door jams; remote control lost below half the computer core's supply (sections 1 and 8).
- [ ] 2.9 Damage control teams as crew bodies with `crew-on-deck`: dispatch, suits, walking, jobs, risk; the board's automation order (section 7).

## 3. Tests

- [ ] 3.1 `a_gannet_on_the_port_switchboard_leaves_the_reactor_running` (W1: port section destroyed, starboard at 98%, no scram).
- [ ] 3.2 `twenty_five_pulse_bolts_strip_a_hull_section_and_the_next_ones_hole_it` (W3).
- [ ] 3.3 `the_same_hit_with_the_same_seed_does_the_same_thing`, fire roll included.
- [ ] 3.4 `one_extinguisher_stops_a_fire_caught_in_ninety_seconds_but_not_at_two_minutes`.
- [ ] 3.5 `a_sealed_room_starves_its_fire_in_about_five_minutes`.
- [ ] 3.6 `water_mist_ends_a_one_megawatt_engineering_fire_in_under_half_a_minute`.
- [ ] 3.7 `inert_gas_keeps_the_magazine_below_cook_off`.
- [ ] 3.8 `a_preview_of_a_repair_equals_the_repair` for a kit job, a rebuild and a splice.
- [ ] 3.9 `automation_never_vents_a_room_with_crew_inside`.

## 4. Consoles (with `bridge-stations`)

- [ ] 4.1 The damage control board D1-D4 with hull sections, the queue's previews, teams and doors.
- [ ] 4.2 The engineering bay's reactor panel and coolant valve manifold (with `power-grid` task 4.2); local breakers at the switchboard sections.

## 5. Measurement and mockup

- [ ] 5.1 Measure a 60 MJ hit's resolution in the Rust core on a Pi 5 and confirm the four-per-tick cap.
- [ ] 5.2 Owner review of `fire-in-engineering` and `breach-launch-bay` (questions D2, D6).

## 6. Cross-change follow-ups

- [ ] 6.1 `weapons-and-shields`: the `HullHit` contract (section 1), the magazine cook-off rule, turret and tube states from integrity.
- [ ] 6.2 `shuttle-bay-and-fighters`: cradle capability and drop door jams from section 1.
- [ ] 6.3 `crew-on-deck`: carrying kits, extinguishers and plates; suits; the teams' bodies.
- [ ] 6.4 Officers and ratings (design 6a): the rank from `crew-on-deck`'s roster in `damage::repair_time`, with the test that an officer takes 42 s and a rating 125 s from 25% (owner, 2026-10-08).
