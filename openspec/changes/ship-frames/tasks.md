# Tasks

## 1. Data

- [ ] 1.1 `data/ships/tern/dampers.json`: `rated_capacity_m_s2` 25, `lag_s` 0.05, `felt_threshold_m_s2` 0.3, `standby_draw_mw` 2, `active_draw_mw` 6, head spring `frequency_hz` 1.5, `damping_ratio` 0.7, `max_offset_m` 0.12, `max_tilt_deg` 10; shake `reference_energy_mj` 20, `falloff_m` 15, `decay_per_s` 1.5, `max_angle_deg` 2.5, `max_offset_m` 0.04, `noise_hz` 15. Validated on load (CLAUDE.md 6.5).
- [ ] 1.2 Viewscreen and feed settings in data: size, refresh, slew rate, field-of-view range, default mount.

## 2. Core (`sc-core::frames`)

- [ ] 2.1 Frame-tagged types (`SysPos`, `IntPos`, `NearPos`, `Pose { frame, .. }`) and the frame id enum shared with `sc-net`.
- [ ] 2.2 `interior_to_system`, `system_to_interior`, `point_velocity`, `to_near`, `to_camera_relative`: the one implementation of each transform.
- [ ] 2.3 `damper_step` per compartment (demand, cancel with lag and capacity, residual, threshold) and its draw report to `power-grid`.
- [ ] 2.4 `shake_add` and `shake_step` with seeded noise.
- [ ] 2.5 The hand-off phase in the server tick (`release`, `capture`), stable id order, mass and centre-of-mass update.
- [ ] 2.6 Docking: attachment of the smaller vessel, +Y alignment check, portal crossing frame change.

## 3. Tests (`sc-core`, headless)

- [ ] 3.1 `a_point_survives_interior_to_system_and_back_at_one_au` (0.1 mm).
- [ ] 3.2 `crew_do_not_move_when_the_ship_turns_with_full_dampers`.
- [ ] 3.3 `half_dampers_leave_two_and_a_half_m_s2_on_a_full_burn` and `unpowered_dampers_knock_standing_crew_down`.
- [ ] 3.4 `planet_gravity_is_never_felt`.
- [ ] 3.5 `a_released_fighter_carries_the_ships_point_velocity` (with yaw) and `a_snapshot_never_holds_a_craft_twice`.
- [ ] 3.6 `capture_absorbs_relative_velocity_as_an_impulse`.
- [ ] 3.7 `a_frame_id_that_does_not_resolve_is_dropped`.
- [ ] 3.8 `the_shake_is_identical_on_two_clients` (seeded).
- [ ] 3.9 The worked damper table in design section 4 reproduced from the layout and the mass table (validate the real artifact, CLAUDE.md 6.6).

## 4. Renderer (`sc-render`)

- [ ] 4.1 Pass order with the window scissor and stencil mask; skip the exterior with no space portal visible.
- [ ] 4.2 Far layer (stars as points, sun billboard, planet proxies, far bodies as point sprites) and near layer (1 m to 20 km).
- [ ] 4.3 Viewscreen target 1024 x 512 at 30 Hz and two 512 x 256 feeds at 15 Hz on alternate frames.
- [ ] 4.4 Instanced attachments drawn with the ship pose; one ship pose per rendered frame.
- [ ] 4.5 Turret sight, fighter cockpit (cockpit pass in the craft frame) and chase views.
- [ ] 4.6 Exterior mesh check in `meshc`: closed, every face outward (the hull loft's cap winding bug, design "Proposed kit fix", must fail it).

## 5. Verification

- [ ] 5.1 Capture: the decoupling split (exterior rolling, interior level) from the engine, beside the mockup's `decoupling-split` shot.
- [ ] 5.2 Pi 5 probe: the bridge view in a full engagement against the 90,000 triangle and 50 call share, and the viewscreen's fill cost (engine-stack's probe).
- [ ] 5.3 Move each requirement into `openspec/specs/ship-frames/spec.md` in the commit that makes it true, with its test.

## 6. Mockup (documentation, done with this write-up)

- [x] 6.1 `docs/mockups/exterior.html` shots `decoupling-split`, `decoupling-normal`, `gunner-view`, `fighter-cockpit`, `fighter-drop`.
