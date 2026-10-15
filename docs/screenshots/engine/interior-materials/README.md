# Interior materials without the grid, 2026-10-14

The owner had the grey square grid of `tech_panel` removed ("can you remove that grid texture?"),
and for a few hours its four layers were the magenta [PLACEHOLDER] marker (0f8a745). The owner then
clarified: "sorry some textures can use the generic base textures, I just wanted you to remove the
grid pattern that I didnt like that you were using for ship interiors". So now:

| Layer | Material | Graph | Where it shows here |
| --- | --- | --- | --- |
| 0 | `bulkhead` | `plain_steel`, slate blue-grey | The finishes' `wall`, where no wall panel covers it |
| 4 | `trim` | `plain_steel`, lighter grey | The bridge's main viewscreen frame (`bridge-forward.png`, `bridge-viewscreen.png`), railings, risers, coves, ribs, window frames, ladders (the `trim` of the ship's finishes in `data/ships/tern/detailing.json`) |
| 9, 10 | `hull`, `hull_dark` | `tech_panel`, as before | Outside only: the placeholder starbase's docking cradle (`outside-port-bow.png`, and through the windows) |

`plain_steel` is a new Material Maker graph (`tools/materials/build_ptex.py`): painted steel with a
horizontal brushed grain and a few light scratches, no panels, seams or grid. It was rendered by
Material Maker on the Pi 5 itself and post-processed into the two layers; see
`docs/screenshots/materials/contact-sheet.png` and `openspec/changes/surface-materials` design.

`compiled/tern.deck` was exported and compiled again on Pi 1 and these eight poses shot headless,
normal lighting (`sc-client --headless --shots --poses`). No interior surface shows a grid and none
shows magenta: the viewscreen frame that showed [PLACEHOLDER] is plain grey steel. The walls are
the wall panels; the floor tiles and ceiling tiles are their own materials (`deck_tiles`,
`ceiling`), unchanged. The cradle outside is grey `tech_panel` again, which is expected: the grid
is on the outside of the ship only, and the cradle changes when the starbase model arrives.
