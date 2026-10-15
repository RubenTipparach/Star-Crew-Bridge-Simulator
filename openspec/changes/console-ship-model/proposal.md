# Proposal: the consoles draw the Tern's low poly model

## Why

The owner, 2026-10-14, with two console frames (Science's shield view and the camera widget beside the viewscreen):
"this ship model, and the one on the camera view needs to be updated too. to use the low poly version of the ship".

Both draw the layout's hull loft, the pressure hull's eight-sided sections: a capsule that is the inside of the ship,
not the Tern anyone sees on the viewscreen. The Tern's exterior has a low poly model already,
`assets/models/exteriors/tern_lod2.glb` (496 triangles, `tern-exterior`).

## What Changes

- **The console's ship pictures draw `tern_lod2`** (design 1): the camera widget beside every viewscreen, the shield
  view (Science, the Captain's shield picture) and wherever else the console draws the Tern in 3D.
- **One data file from the model** (design 2): `tools/ships/console_model.py` reads the glb and writes
  `data/ships/tern/console_model.json`, its triangles with one colour each from the model's atlas; the engine includes
  it and the mockup inlines it, and the tool's `--check` fails when it is stale.
- **The shield bubble envelops the whole ship** (design 3; owner, 2026-10-14: "shield should envolop the ship
  entirely"): the ellipsoid grows from 17 x 13 x 54 m to 27 x 10.5 x 59.5 m, the smallest that holds the exterior
  model with 1 m to spare.

## Impact

- `tools/ships/console_model.py`, `data/ships/tern/console_model.json`.
- `sc-client::console::kit`: the hull polygons from the file.
- `docs/mockups/consoles.html`: `HULL_POLYS` from the file (an `INLINE data:tern/console_model` block).
- Pi 5 budget: design 4.
