# Tasks

## 1. Data

- [ ] 1.1 Accept `data/ships/tern/power.json` (proposed here) and its schema `starcrew.ship-power/1`; move the reference ship's numbers out of the proposal state.
- [ ] 1.2 Apply the layout patches of design section 16 through `reference-ship-tern` (starboard main switchboard section, reactor panel, coolant valves) and re-run `python3 tools/layout_check.py`.
- [ ] 1.3 A data check that loads `power.json` against the layout: every node's compartment and every conduit's route exist, every path point lies in or between its route's compartments, every load's system or mount exists, groups cover every adjustable load once.

## 2. Core (`sc-core::power`, `sc-core::heat`)

- [ ] 2.1 Validated loading (unknown keys, non-finite numbers, missing ids, negative capacities stop startup with path and field).
- [ ] 2.2 The topology: nodes, generators, ties, conduits with health, breakers and severed state, the battery link; fixed arrays allocated at load.
- [ ] 2.3 Demand (design section 7) with capability from `damage-control`.
- [ ] 2.4 `power::solve`: passes, phases, rounds, augmenting paths in data order, drop-out re-solve, charging (section 9). One function, used by the sub-step and every preview.
- [ ] 2.5 The reactor: load following by probe solve, manual throttle, ramp, fuel, overdrive wear, blanket heat.
- [ ] 2.6 Scram causes and holds, the hands-on reset with its refusals, ignition at the trains' pace (section 3).
- [ ] 2.7 The battery: energy, efficiencies, the restart reserve and its users, release, losses to the loop (section 4).
- [ ] 2.8 `heat`: thermal nodes, the loop, pumps, radiators with bypass, warning and damage temperatures, overdrive heat and wear (section 10); `heat::project` for the preview.
- [ ] 2.9 Lighting states per compartment from panel and emergency supply.
- [ ] 2.10 Presets, groups, setpoints and priorities as commands (intents) from the engineering seats.

## 3. Tests (they read as sentences, one behaviour each)

- [ ] 3.1 `cruise_needs_less_than_half_the_reactor`: 21.2 MW at 44% throttle (design section 12).
- [ ] 3.2 `combat_runs_the_battery_to_its_reserve_in_about_two_minutes_then_sheds_the_drive`.
- [ ] 3.3 `a_cut_port_trunk_loses_nothing`: every load keeps its supply through the starboard trunk and the ties.
- [ ] 3.4 `both_trunks_cut_starve_the_forward_ship_after_the_battery`.
- [ ] 3.5 `losing_one_main_switchboard_section_does_not_scram_the_reactor` (the best-train rule).
- [ ] 3.6 `a_scram_from_an_overheated_loop_can_always_be_reset` (the 350 MJ reserve, radiators at 60%).
- [ ] 3.7 `the_automatic_throttle_never_runs_the_reactor_into_a_grid_that_cannot_take_it`.
- [ ] 3.8 `a_preview_equals_the_next_sub_step` for every group and setpoint in a sweep.
- [ ] 3.9 `the_solve_is_deterministic`: the same state and inputs give bit-identical allocations twice.
- [ ] 3.10 `the_loop_settles_near_330_kelvin_at_cruise` and `no_thermal_state_oscillates_at_10_hertz`.
- [ ] 3.11 `every_delivered_megawatt_becomes_heat_somewhere`: an energy balance over 600 s within 0.1%.

## 4. Consoles (with `bridge-stations`)

- [ ] 4.1 E1 allocation rows with the ghost bar from `power::preview_group`; E2 bus diagram with breakers; E3 reactor with the heat projection; E4 coolant; the battery block.
- [ ] 4.2 The engineering bay's reactor panel: reset hold 3 s, refusal text, throttle, coolant branch valves.

## 5. Measurement and mockup

- [ ] 5.1 Measure the sub-step and the solve on a Pi 5 with the probe (`engine-stack` section 11) and replace the estimates of design section 14.
- [ ] 5.2 Keep `docs/mockups/systems.html` running the same formulas until the core exists; then pin the design's tables with the core's tests and retire the library's copy of the numbers.
- [ ] 5.3 Owner review of the `power-overview` and `power-severed` shots (questions P1-P3).

## 6. Cross-change follow-ups

- [ ] 6.1 `weapons-and-shields`: hoist and loader draws (P5).
- [ ] 6.2 `flight-and-navigation`: the drive's draw column (P6) and the jump spool as a load if the jump drive is accepted.
