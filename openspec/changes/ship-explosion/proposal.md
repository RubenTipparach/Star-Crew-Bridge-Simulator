# Proposal: a ship's death that lingers

## Why

The owner, 2026-10-14, with a frame of Starfleet Command: "also check out that cool explosion that persists for like
15 seconds! when the ship explodes."

When the Hound dies today the client draws one burst, 70 m across, for 2.5 s, and the Hound is gone the frame its
`alive` flag drops: one blink and an empty patch of stars. A fight's last moment should be the thing the crew watch on
the viewscreen.

## What Changes

- **A death in five layers, over 15 s** (design 1): a white flash, a fireball, a shock ring in the ship's plane, a
  glowing cloud that drifts and cools for 15 s, and the wreck: the hull, charred, breaking into tumbling pieces with
  embers trailing.
- **Additive glow sprites** (design 2): a new small render program, camera-facing quads with a soft round falloff
  added to what is behind them, so fire glows and fades instead of popping as the opaque burst mesh does.
- **Tuning in data** (design 3): `data/space/explosion.json`, every time, size and colour with its unit.
- **Client only** (design 4): the server's outcome is unchanged; every client draws the same death from the same seed.

## Impact

- `sc-render`: a glow program (`shaders/glow.glsl`) and its pipeline (blend one, one; depth test on, no depth write).
- `sc-client::drill`: a `Wreck` beside the booms, started by `Visual::Destroyed`; drawn in the feed, through the
  bridge's windows and on its viewscreens.
- `sc-core`: `ExplosionData`, loaded and validated with the drill's other data.
- Pi 5 budget: design 5.
