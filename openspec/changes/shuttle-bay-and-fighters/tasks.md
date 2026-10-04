# Tasks

## 1. Data

- [ ] 1.1 `data/ships/tern/bays.json`: cradle travel and speed, door times, release and capture points, corridor box, speed limits, interlock thresholds, override clearance (design sections 1-5).
- [ ] 1.2 `data/craft/swift.json`, `data/craft/petrel.json`, `data/craft/drone.json` (sections 7-9).
- [ ] 1.3 Propose the `launch_trunk` mounts to `reference-ship-tern` (design, "Layout notes") and teach `tools/layout_check.py` to check a trunk's top lies on its bay's door and its bottom on the keel.

## 2. Core

- [ ] 2.1 `hangar`: bay postures, the launch and recovery sequences as step machines with interlocks and reasons, emergency vent, overrides with the predicted clearance.
- [ ] 2.2 Doors, cradles and the pad as attachments (`ship-frames` section 9), with cranks for crew.
- [ ] 2.3 Release and capture through `ship-frames`' hand-off; the recovery hold request to `flight-and-navigation`.
- [ ] 2.4 Craft: the Swift and the Petrel on the one flight model; propellant and mass; guns, Darts and shield through `weapons-and-shields`.
- [ ] 2.5 Drones: orders, escort, engage (the Jackal state machine with drone data), bingo and auto recovery.
- [ ] 2.6 Rearm and refuel on the cradle from the Tern's stores.
- [ ] 2.7 Ejection pods: spawn, beacon, air timer, recovery by the Petrel or a bay.

## 3. Tests

- [ ] 3.1 `a_cold_launch_takes_forty_four_seconds_at_a_thirty_second_pump_down`.
- [ ] 3.2 `the_bay_will_not_pump_down_around_an_unsuited_crewman`.
- [ ] 3.3 `release_holds_while_the_ship_turns_over_ten_degrees_a_second`.
- [ ] 3.4 `capture_needs_point_six_metres_and_one_metre_a_second`.
- [ ] 3.5 `a_door_jammed_at_forty_percent_blocks_a_swift`.
- [ ] 3.6 `the_combat_drop_preview_matches_the_release` (preview and outcome from one function).
- [ ] 3.7 `drones_never_launch_without_an_order` and `a_drone_comes_home_at_bingo`.
- [ ] 3.8 `the_petrel_needs_the_whole_hangar_empty`.
- [ ] 3.9 `a_swift_turnaround_takes_forty_seconds`.

## 4. Client

- [ ] 4.1 Flight ops and bay control panels (with `bridge-stations`): bays, sequences, interlocks, approach panel, tasking, the nose-camera feed.
- [ ] 4.2 The cockpit pass and HUD (with `ship-frames`), including the recovery corridor marker.

## 5. Verification

- [ ] 5.1 Capture a cold launch and a hand-flown recovery from the cockpit and from outside, beside the mockup's `fighter-drop` and `fighter-recovery`.
- [ ] 5.2 Move each requirement into `openspec/specs/hangar-and-craft/spec.md` with its test.

## 6. Mockup (documentation, done with this write-up)

- [x] 6.1 `docs/mockups/exterior.html` shots `fighter-drop`, `fighter-recovery`, `fighter-cockpit`.
- [ ] 6.2 Screenshot those shots (`node tools/mockups/shoot.mjs docs/mockups/exterior.html`) and look at each; the authoring session could not run the tool.
