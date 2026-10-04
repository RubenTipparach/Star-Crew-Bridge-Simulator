# Tasks

## 1. Data and layout

- [ ] 1.1 Accept `data/ships/tern/atmosphere.json` (proposed here) and its schema `starcrew.ship-atmosphere/1`.
- [ ] 1.2 Move the duct node and the thirty vent portals, the overboard dump, the bay vent valves and the airlock equalizing valve from `graph_additions` into the one compartment graph in `layout.json` (with `reference-ship-tern`), and add the reserve bottles fixture (design section 19).
- [ ] 1.3 A data check: every compartment has a vent; every store and pump names a compartment that exists; crew effect tables are sorted and finite.

## 2. Core (`sc-core::atmosphere`, `sc-core::plant`, `sc-core::crew_effects`)

- [ ] 2.1 The gas state per node: four species, internal energy, derived temperature and pressure with the fittings' heat capacity (section 1).
- [ ] 2.2 Link conductances by the compressible orifice equation with choked flow and the linear band (section 4).
- [ ] 2.3 The implicit pressure solve (dense Cholesky at 31 nodes, clamped), fluxes, and the ordered upwind transport of species and enthalpy.
- [ ] 2.4 Door mixing with buoyancy scaled by gravity; ventilation at design air changes times the fans' supply (section 5).
- [ ] 2.5 Dampers: excess-flow trip, latch, retry, automatic refill above 10 kPa, the board's refill and its abandonment on a leak (section 6).
- [ ] 2.6 Doors: closed by default, open on approach, interlock at 20 kPa with override, self-closing on a falling pressure alarm, travel times (section 3).
- [ ] 2.7 The plant and make-up with their power draws through `power-grid`'s demand (section 7).
- [ ] 2.8 Metabolism and crew effects (sections 8 and 12), handed to `crew-on-deck`.
- [ ] 2.9 Room heat: loads, lights, crew, bulkheads, hull (section 9).
- [ ] 2.10 Bay pump-down, the receiver, launch permission at the stop, vent valve, repressurization, the unsuited-crew interlock; emergency vent (section 13).
- [ ] 2.11 The airlock cycle (section 14).
- [ ] 2.12 `life_support::time_to_pressure` and the refill cost preview (section 16), sharing the sub-step's functions.

## 3. Tests

- [ ] 3.1 `a_launch_bay_pumps_down_to_five_kilopascals_in_under_thirty_seconds` (28.6 s) and `the_hangar_takes_about_three_and_a_half_minutes` (208 s).
- [ ] 3.2 `a_one_square_metre_breach_takes_the_bridge_to_armstrongs_limit_in_eleven_seconds`, and the whole decompression table of section 11 within 5%.
- [ ] 3.3 `decompression_at_ten_hertz_matches_one_kilohertz_within_a_third_of_a_second`.
- [ ] 3.4 `no_node_ever_holds_negative_gas` and `a_draining_room_never_gains_pressure_above_one_pascal` over every compartment and breach size.
- [ ] 3.5 `gas_is_conserved`: with the oxygen generator, scrubbers, metabolism and fire off, moles in the ship, the stores, the receiver and those lost overboard sum to the start within 0.01% over an hour of breaches, pumps and refills.
- [ ] 3.6 `a_breached_rooms_damper_shuts_within_two_seconds` for 0.01 m^2 in engineering and 2.2 m^2 in a pod; `no_damper_trips_in_twenty_minutes_of_combat`.
- [ ] 3.7 `an_open_door_closes_itself_when_its_room_is_breached`, and `a_held_door_does_not`.
- [ ] 3.8 `a_refill_stops_if_the_room_is_still_leaking`.
- [ ] 3.9 `the_pumps_will_not_start_with_an_unsuited_crew_member_in_the_bay`.
- [ ] 3.10 `a_preview_of_the_pump_down_equals_the_pump_down` within one sub-step.

## 4. Consoles (with `bridge-stations`)

- [ ] 4.1 E5 life support panel, damage control D1 compartment values and previews, flight ops and bay control bay panels.

## 5. Measurement and mockup

- [ ] 5.1 Measure the atmosphere step on a Pi 5 within the systems sub-step (with `power-grid` task 5.1).
- [ ] 5.2 Owner review of `atmosphere-normal`, `breach-launch-bay` and `hangar-pumpdown` (questions L1, L2, L3, L6).

## 6. Cross-change follow-ups

- [ ] 6.1 `shuttle-bay-and-fighters`: replace its assumed times with section 13's, the launch permission at 5 kPa (L2), the Petrel's longer pump-down (L3).
- [ ] 6.2 `bridge-stations`: F3's pump-down time (29 s).
- [ ] 6.3 `docs/references.md`: the time-of-useful-consciousness, hypercapnia and Purser sources cited in section 12.
