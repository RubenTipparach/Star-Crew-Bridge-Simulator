# Tasks: nebula-skies

## 1. Write-up and mockup (this change, done before approval)

- [x] 1.1 Research Homeworld's backgrounds and FreeSpace's star streaks from sources (design 1)
- [x] 1.2 The generator `tools/space/nebula_skies.py`, the recipes `data/space/skies/recipes.json`, the streak tuning `data/space/starfield.json`
- [x] 1.3 Four skies generated (`bloom`, `quiet`, `ember`, `verdigris`)
- [x] 1.4 The mockup `docs/mockups/nebula-skies.html`: skies A-D, behind the Tern, behind Tamsin Station, through the bridge window, a fast turn each, streaks on and off, the mesh, sky ambient on and off, motes at speed, the Pi cost
- [ ] 1.5 The owner's answers to Q1-Q5 (design 10), with the shots

## 2. The sky mesh in the renderer (after approval)

- [ ] 2.1 `sc-core`: load `data/space/skies/<id>.json` (schema, base64 rows, counts, indices in range, finite directions); a bad file stops startup with its path and field
- [ ] 2.2 `sc-render`: the sky program drawn on the sky mesh (one indexed draw, depth test and writes off), its fragment the vertex colour plus `sky.glsl`'s faint stars times `1 - hide` times the turn fade, the sun, the planet and the dither; the full-screen triangle retired. Coordinate with the change that fixes stars drawing over ships (design 7)
- [ ] 2.3 A test that the decoded mesh is closed (every edge in two triangles) and wound toward the centre; a test that the program's colour at a vertex is the file's
- [ ] 2.4 A headless capture of each sky against the mockup's `<L>-sky` shot

## 3. The system's sky

- [ ] 3.1 `data/space/systems/shoals.json` gains `"sky"` (the owner's Q1 pick); `sc-core::system` loads it; a missing sky is an error
- [ ] 3.2 The client loads a system's sky with the system and changes it at a jump's settle (`patrol-mission` 11.5)

## 4. The vector stars

- [ ] 4.1 `sc-render`: the star program: `GL_LINES` (two vertices a star) and `GL_POINTS` heads, the streak of design 5.2 from this frame's and last frame's view rotation, `starfield.json`'s numbers, the planet-disc test
- [ ] 4.2 The client keeps each camera's last rotation and resets it on a cut (sit, stand, feed switch, jump end, a turn over `cut_deg_s`)
- [ ] 4.3 Tests: a star at rest draws a 1 px line; a tail never passes `max_deg`; `dim` never falls under `dim_floor`; a cut draws no tail; a star inside the planet's disc is not drawn
- [ ] 4.4 Motes (`patrol-mission` 11.4) and the jump's streaks (11.5) on the same rule (design 5.3, 5.4)

## 5. The sky's ambient

- [ ] 5.1 `sc-client/src/outside.rs`: the outside ambient cube takes the sky's `cube_linear`, turned into each object's frame; the hull relight's ambient term the same
- [ ] 5.2 A test that the cube's brightest face's luminance is the sky's strength

## 6. Measure on the Pi 5

- [ ] 6.1 The probe: the sky pass (mesh, lines, heads) at 1280 x 720 on the Pi 5, old sky against new, interleaved, percentiles and spread; `GL_ALIASED_LINE_WIDTH_RANGE` and the lines' look; 8,000 against 4,000 vertices
- [ ] 6.2 The budget table in `engine-stack` updated from the measurement
