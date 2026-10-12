# Tasks

## 1. Data and generator

- [x] 1.1 `tools/power/boxes.py`: boxes, doorway runs with their factory state and rating, every load's node and
  feeder, the lighting by box, from `layout.json` and the hand-authored `power.json`; `--check`.
- [x] 1.1a Runs through walls and decks until every panel joins at least two others.
- [x] 1.1b Every system on two feeds, the nearest panel first; the reactor on five panels.
- [x] 1.2 `power.json` `distribution` (the generator's rules) and `routing` (`max_live_runs` 42, `trip_dark_s` 8,
  `kept_on_trip`).

## 2. Core (`sc-core::power`)

- [x] 2.1 Read the new fields; every run starts at its factory state; a factory configuration over the limit stops
  startup.
- [x] 2.2 Box switches, run switches, the live count, the trip and the factory reset; nothing switched while dark.
- [x] 2.3 The damage cascade stops at an open run; the drill mirrors the switches into the damage state.
- [x] 2.4 `reroute`, and Engineering's automation closing a spare after its reaction time.
- [x] 2.5 Commands `PowerCmd::Box` and `PowerCmd::Run` (protocol 8).

## 3. Tests

- [x] 3.1 `every_room_has_a_box_and_every_load_a_feeder_from_it`.
- [x] 3.2 `the_factory_configuration_is_under_the_protections_limit`.
- [x] 3.3 `a_box_switched_off_darkens_what_it_feeds`.
- [x] 3.4 `a_spare_run_closed_routes_power_round_a_cut_feed`.
- [x] 3.5 `too_many_live_runs_trip_the_grid_dark_then_it_comes_back_at_its_factory_settings`.
- [x] 3.6 `the_reroute_closes_the_spare_that_reaches_a_cut_off_box`.
- [x] 3.7 `every_panel_connects_to_at_least_two_others`.
- [x] 3.8 `every_system_has_two_feeds_the_nearest_first`, `the_reactor_feeds_five_panels`,
  `a_system_swapped_to_its_second_feed_draws_from_that_panel`, `engineering_automation_swaps_a_dark_system_to_its_other_feed`.

## 4. Mockup and consoles

- [ ] 4.1 The power grid mockup as the expanded schematic: 2D / 3D toggle, boxes, feeders, spares, switching by tap,
  the live count, a trip, portrait phones, COLLAPSE.
- [x] 4.1b The rooms overlay in the schematic (design 3b), swapping a system's feed by tap.
- [x] 4.1c The 2D schematic as a circuit board placed by hand in `power_board.json` (design 3c).
- [x] 4.1d Damage on the schematic and HIT ROOM (design 3d); every flowing trace at one speed (design 3a).
- [x] 4.1e Subsystems colour-coded on the schematic, families in `subsystems.json` (design 3e).
- [ ] 4.1a Engineering's side panel in `consoles.html`: BUSES becomes SCHEMATIC with EXPAND (design 3a).
- [ ] 4.2 Owner review of the mockup's shots; question R1.
- [ ] 4.3 After approval: Engineering's routing view in `consoles.html`, then in the engine, with the switches in the
  grid snapshot.

## 5. In the game (design 6)

- [x] 5.1 `RouteSnap` in the snapshot (protocol 9), sent between the grid's; round trip and size tests.
- [ ] 5.2 Engineering's SCHEMATIC side panel with EXPAND and the live runs; parity fixtures redrawn.
- [ ] 5.3 The full-screen schematic: open and close, the 2D / 3D switch, the status strip, taps to `Box`, `Run`, `Feed`.
- [ ] 5.4 2D: the board from `power_board.json`, families from `subsystems.json` (client data, `check-data`).
- [ ] 5.5 3D: the decks, lanes, chamfered runs, markers, orbit and zoom.
- [ ] 5.6 Hover lighting the run under the pointer; damage marks; the trip banner.
- [ ] 5.7 Engine shots beside the mockup's; the owner's sign-off.

