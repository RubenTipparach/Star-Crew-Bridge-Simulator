# Tasks

## 1. Data and tools

- [ ] 1.1 `data/ships/tern/flight.json` (mass table, thrusts, torques, draws, slews, limits, assist constants, damper-safe margin) and `data/ships/tern/jump.json` (design sections 1, 2, 4, 5, 7).
- [ ] 1.2 `sc-tools` inertia tool: centre of mass and inertia tensor from the layout's hull loft and the mass table, written into the compiled ship (a measurement instrument; the design's figures come from the same 0.5 m grid integration).
- [ ] 1.3 A first system file, `data/systems/<id>.json`, with bodies on rails and the points of interest for missions 1-4.
- [ ] 1.4 Propose the `jump_drive` system to `reference-ship-tern` (design, "Layout note").

## 2. Core

- [ ] 2.1 `flight::step`: the one integrator for ships and craft, with mass changes at hand-offs.
- [ ] 2.2 `flight::assist`: full, rotation-only and off; reference frames; speed caps; flip and burn.
- [ ] 2.3 `flight::damper_safe`: the lambda solve over compartments using `ship-frames`' demand function.
- [ ] 2.4 Thrust from power: supply ratios in, draws out, slews.
- [ ] 2.5 `nav::rails`: Kepler evaluation, local gravity with smoothstep falloff, influence and mass-lock radii.
- [ ] 2.6 `nav::jump`: spool, alignment, mass lock, field, arrival scatter, cooldown, the rebase through `ship-frames`.
- [ ] 2.7 `nav::autopilot`: every mode on top of the assist; docking corridors and latching; helm automation.
- [ ] 2.8 `collide`: near-frame broad phase, GJK and EPA, swept tests, impulse response, kinetic hull hits.

## 3. Tests

- [ ] 3.1 `the_terns_inertia_comes_from_its_layout` (validate the real artifact).
- [ ] 3.2 `half_drive_power_halves_the_burn`.
- [ ] 3.3 `assist_off_keeps_momentum` and `full_assist_turns_about_to_brake`.
- [ ] 3.4 `the_damper_safe_limit_keeps_every_compartment_under_capacity` (property test over random commands and rates).
- [ ] 3.5 `planet_gravity_fades_smoothly_at_the_influence_radius`.
- [ ] 3.6 `a_mass_lock_refuses_the_spool` and `craft_outside_the_field_stay_behind`.
- [ ] 3.7 `docking_faster_than_point_two_metres_a_second_bounces`.
- [ ] 3.8 `ramming_a_jackal_splits_the_energy_and_jolts_the_crew`.

## 4. Client

- [ ] 4.1 Input mapping for keyboard and mouse, pads and HOTAS to helm commands (every device drives the station).
- [ ] 4.2 Helm console with `bridge-stations`: throttle and assist state, damper limit, speed and its reference, the local plot, the system map, the jump panel, autopilot.

## 5. Verification

- [ ] 5.1 Capture: a hard turn with the damper-safe limit on and off, from the bridge, beside the mockup's `decoupling-split`.
- [ ] 5.2 Move each requirement into `openspec/specs/flight-and-navigation/spec.md` with its test.

## 6. Mockup (documentation, done with this write-up)

- [x] 6.1 `docs/mockups/exterior.html` shots `chase` and `decoupling-split`.
- [ ] 6.2 Screenshot those shots (`node tools/mockups/shoot.mjs docs/mockups/exterior.html`) and look at each; the authoring session could not run the tool.
