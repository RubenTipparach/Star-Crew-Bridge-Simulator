# Tasks

Nothing here is started. Section 1 is the write-up and its measurement instrument (CLAUDE.md 4);
everything from section 2 on is engine work, taken on a separate request once the owner has seen
the write-up.

## 1. Write-up

- [x] 1.1 Proposal, design and spec deltas for `crew-on-deck`.
- [x] 1.2 The walk-time table (design section 16) and the portal clearance table (section 2), computed from `data/ships/tern/layout.json`.
- [ ] 1.3 Commit the walk-time script as `tools/walk_times.py` (a measurement instrument: it reads the layout and prints section 16's tables; no game code) and add the portal clearance check to `tools/layout_check.py`.
- [ ] 1.4 Put question C1 in the owner survey with `bridge-walk-aft.png` and `bridge-captain-view.png`; record C2-C9 as recommendations taken; fold the answers back into this change.
- [ ] 1.5 Hand the proposed patches to their changes: the capsule and handhold rails and ladder deck hatches to `deck-pipeline`; the crew status group and felt residual to `netcode-and-sessions`; the speeds to `reference-ship-tern` (T5); kit and bed positions to `damage-control` and the layout.

## 2. Data

- [ ] 2.1 `data/crew.json` with every value of design sections 2-12, units in the keys; unknown keys and non-finite values stop startup.
- [ ] 2.2 `data/ships/tern/kit.json`: extinguisher brackets, kits, suit lockers, beds, trolleys; a validator checks each inside its compartment and clear of door openings.
- [ ] 2.3 On-foot bindings in `data/input/bindings.json` beside the seated ones.

## 3. Core (`sc-core::crew`)

- [ ] 3.1 `crew::step`: capsule movement against `deck-pipeline`'s query, slide and step-up, speeds, grip-limited acceleration, stairs, crouch, falling; test `a_body_reaches_walking_speed_in_a_third_of_a_second`.
- [ ] 3.2 Ladders: volumes, getting on and off, speeds, one body per 1.8 m, hands rules; test `a_trolley_cannot_be_taken_onto_a_ladder`.
- [ ] 3.3 Doors, hatches and pressure doors: approach sensing, times, never closing on a body, locks, `life-support`'s interlock (20 kPa) and the override hold; tests `a_door_will_not_open_into_vacuum` and `a_closing_door_waits_for_the_body_in_it`.
- [ ] 3.4 Use: reach, line of sight, posture and hands checks; items as interior objects; pick up and drop; the trolley's push, brake and room limits.
- [ ] 3.5 Health: `crew::injure`, wounded, downed, the stabilize window, critical, revive by hand, field recovery, every body down; tests `a_revived_body_gets_up_at_twenty_five_hp` and `damage_while_down_shortens_the_window`.
- [ ] 3.6 Medbay beds; air effects from `life-support`'s values; suits, oxygen and punctures.
- [ ] 3.7 Zero gravity: floating, kick off, handholds, flail, impacts, the return of gravity.
- [ ] 3.8 The residual response and bracing (ship-frames' table); hit shake stumbles; tests per row of design section 12.

## 4. Client (`sc-client`, `sc-render`)

- [ ] 4.1 First-person camera: eye per posture, raw look, the lurch offset and tilt after the look rotation.
- [ ] 4.2 Prediction of the own body through `crew::step`, door prediction, correction per `netcode-and-sessions`.
- [ ] 4.3 The avatar: skinned mesh with LODs, role colour uniform, light probe lighting, first-person arms.
- [ ] 4.4 Clips: the baker in `sc-tools`, playback scaled by speed, the layers and the head turn.
- [ ] 4.5 Interaction prompts, the status line (HP, effects by name, suit oxygen), the brace countdown.

## 5. EVA (after the interior)

- [ ] 5.1 The airlock cycle with `life-support`; the hull walk surface with `deck-pipeline`; magnetic boots, the tether, the full-demand rule; the EVA view with `ship-frames`.

## 6. Verification

- [ ] 6.1 The layout check passes every crew portal for the standing, suited and casualty-carrying body; `deck-pipeline`'s walkable check passes (T2 was applied on 2026-10-04).
- [ ] 6.2 `tools/walk_times.py` reproduces design section 16 and is rerun whenever the layout or the speeds change (the snappier ladders of section 4, 2026-10-07, are not in it yet).
- [ ] 6.3 Headless captures in `docs/screenshots/`: walking, a ladder, a door refusing across vacuum, a revive, the medbay, zero gravity, a knockdown.
- [ ] 6.4 Pi 5 probe run with eight skinned bodies on the bridge: triangles, draw calls, posing time (owner, on hardware).
- [ ] 6.5 Move each requirement into `openspec/specs/crew-on-deck/` with the test that proves it.
