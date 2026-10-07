# Tasks

The write-up, the mockup baker, the mockup, its screenshots and the skill (section 1) are
documentation tooling and come with the write-up (CLAUDE.md section 4). Everything from section 2
on is engine work, taken on a separate request once the owner has answered G1 to G4.

## 1. Write-up, mockup and skill

- [x] 1.1 Proposal, design and spec deltas for `light-baking`.
- [x] 1.2 `docs/mockups/lib/lightbake.js`: a deterministic CPU baker in the method of design.md (BVH, soft shadows, area emitters, occlusion, bounce from a filtered irradiance cache, three states from one set of rays, adaptive subdivision, lightmap atlas, ambient cubes, digest), inlined by `tools/mockups/inline.py`.
- [x] 1.3 `docs/mockups/lighting.html`: the bridge and engineering from the layout, unlit, runtime lights, vertex on the deck mesh, vertex adaptive (capped and uncapped), lightmap at three texel sizes, split views, three states, power loss, cutaway, close and eye-level cameras, debug views, the Pi 5 meter.
- [x] 1.4 Screenshots in `docs/screenshots/mockups/lighting-*.png`, looked at.
- [x] 1.5 `.claude/skills/light-baking/SKILL.md`.
- [x] 1.6 Put G1 to G4 in the owner survey with their shots (the survey's Lighting section); fold the answers back into this change when they come.
- [ ] 1.7 Offer `deck-pipeline` the probe chunk (`PROB`), the portal spill record and the lamp spacing rule (G5); record its answer here.

## 1b. The ship bake in the mockups (design section 15, the owner 2026-10-07: "commence light baking")

- [x] 1.8 G1 to G4 taken by recommendation to start (2026-10-07), and the survey says so under its Lighting table.
- [x] 1.9 The deck plan bakes every compartment with `lightbake.js` in the three states (doors closed, the kit's lamps, cove strips, station screens, props), the shell split to `mockup_cell_m`, the room in view first; a digest per room.
- [x] 1.10 `tools/mockups/bake_ship.mjs`: the ship bake report in `docs/benchmarks/<date>-tern-bake/` and shots of named rooms in the three states, looked at.
- [ ] 1.11 Engineering's lower floor on red alert and emergency power (design section 15, point 4): its lamps under the mezzanine on the emergency bus and the reactor glow lighting its base, judged on `walk-eng-lower` in the three states.
- [x] 1.12 The bake cached (design section 15, point 5; the owner: "can we cache those results?"): `bake_ship.mjs --write-cache`, `docs/mockups/cache/deck-plan-bake.bin`, the page's keys, `inline.py --check` on the baker's digest.

## 2. Data

- [ ] 2.1 `data/lighting/fixtures.json` and `data/lighting/bake.json` (design.md section 11, as amended by section 15) with a validator, `tools/lighting_check.py`: unknown keys, missing states, negative or non-finite numbers, undefined types; a test per rule (`a_fixture_type_without_an_emergency_colour_stops_the_bake`).
- [ ] 2.2 Fixture records in the detail-file schema (`deck-pipeline` task 1.2) and in `deckgen`'s placement: `type`, `center_m` or `from_m`/`to_m`, `facing_yaw_deg`, `emergency_bus`.

## 3. The baker (`sc-tools bake`)

- [ ] 3.1 Scene: faces, occluders and door leaves from `deckc`; BVH (median split, leaves of 4); two-sided occlusion; closest hit with back-face flag; tests against hand-built boxes.
- [ ] 3.2 The irradiance function: point fixtures with radius and beam, area emitters, ambient times occlusion, bounce from the cache; R2 sampling keyed by stable ids; nine values per sample (three states); test `three_states_cost_the_rays_of_one`.
- [ ] 3.3 The irradiance cache: uniform grid per face, direct pass, gather pass, per-face tent filter, bilinear lookup; up to two bounces.
- [ ] 3.4 Adaptive subdivision: base grid, error in display levels over three states, priority order with stable ties, cap, balance, crack-free fans, diagonal choice, level-synchronous for threads; test `the_bridge_converges_inside_its_cap`; the cap counts every added triangle, balancing and fans included (design section 3, "The cap is not yet a ceiling"); test `balancing_never_carries_a_bake_past_its_cap`.
- [ ] 3.5 Encoding: gamma 2.2, 2x overbright with shoulder, seeded dither, occlusion in alpha; test `reference_irradiance_shows_the_palette_colour`.
- [ ] 3.6 Probes: grid in the air boxes, ambient cubes, invalid flag; spill records per portal side.
- [ ] 3.7 Lightmap output for comparison (atlas, gutters, three layers).
- [ ] 3.8 Hull occlusion bake.
- [ ] 3.9 Threads (compartment queue, batches inside a compartment), the input-hash cache, the report per compartment; test `eight_threads_bake_the_same_bytes_as_one`.
- [ ] 3.10 Digest test pinning the Tern's compartments; run on x86-64 and aarch64; `libm` for transcendental functions if they differ.

## 4. Into the deck (with `deck-pipeline`)

- [ ] 4.1 `deckc` step 4 calls the baker per compartment and takes back tessellated faces, colours, probes and spill records (`deck-pipeline` task 3.2).
- [ ] 4.2 T-junction pass after subdivision interpolates colours on inserted vertices.
- [ ] 4.3 Budget check counts triangles after subdivision; the cap rule of design.md section 3 per compartment.

## 5. Client (`sc-render`, `sc-client`)

- [ ] 5.1 The runtime-light term in the deck shader (decode, add with occlusion, encode) on `deck-pipeline`'s uniform block; selection of four per compartment with 0.1 s fades.
- [ ] 5.2 Door spill lights from the spill records.
- [ ] 5.3 Probe sampling for avatars, craft and moving parts; the ambient-cube term in their shader.
- [ ] 5.4 Power-loss flicker from `bake.json`, seeded from the session seed, compartment and tick.
- [ ] 5.5 Debug views (design.md section 12).

## 6. Verification

- [ ] 6.1 Headless captures of the bridge and engineering from the engine in three states, beside the mockup's shots, in `docs/screenshots/`.
- [ ] 6.2 Bake report for the Tern (time per compartment, triangles, residuals) checked in with the first engine bake; compare with design.md section 13.
- [ ] 6.3 If G1 chooses c or d: an `sc-probe` scene measuring one lightmap fetch per deck pixel on a Pi 5, against the vertex path (owner, on hardware).
