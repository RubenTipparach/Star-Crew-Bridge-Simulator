# Design: a ship's death that lingers

## Context

| Decided elsewhere | Where |
| --- | --- |
| The Hound, its death (`Visual::Destroyed`) and the bursts the client draws | `coop-drill` 5, `ship-damage` |
| The debrief lasts at least 20 s after a kill, so a 15 s death plays out before the next muster | `data/missions/drill-hound.json` |
| Low poly; the Pi 5 renders on OpenGL ES 3.0 | CLAUDE.md 2, 9 |
| Effects are client state; the server decides outcomes | CLAUDE.md 6.3 |

The owner's reference is Starfleet Command (2000): a ship's death is a bright burst that leaves a glowing cloud hanging
where it was for about fifteen seconds. The shape is taken, not the art (CLAUDE.md 15).

## 1. Five layers

Times from the moment of death; sizes in metres for the Hound (38 m long), scaled by a class's length.

| Layer | When | What |
| --- | --- | --- |
| Flash | 0-0.4 s | A white glow sprite growing to 1.5 lengths and gone; the viewscreen's whole picture brightens by its light |
| Fireball | 0-4 s | Three overlapping glow sprites, white to orange to deep red, growing to 2 lengths, fading out |
| Shock ring | 0-2 s | A flat ring in the dead ship's own plane, out to 6 lengths at a constant speed, thinning and fading |
| Cloud | 0-15 s | 16 glow puffs placed by a seeded draw round the hull, drifting out at 2-6 m/s and growing to 1-2 lengths, orange to dull red to a faint violet grey, fading to nothing at 15 s, each flickering a little |
| Wreck | 0-15 s | The hull drawn charred (its albedo times 0.25), split into 3 pieces at two seeded cuts along its length, drifting apart at 3-8 m/s and tumbling; 30 embers (the debris mesh, emissive) flung out on long lives of 6-15 s; the pieces fade into the dark over the last 3 s |

The seed is the dead ship's id and the round, so every client draws the same death.

## 2. Glow sprites

A new program, `glow`: a quad turned to face the camera, its colour times a soft round falloff (`(1 - r^2)^2`), added
to the target (blend one, one). Depth tested so the Tern's hull or the bridge's walls hide it, never written, so the
sprites never hide each other. One draw call a sprite: a death is at most 20 at once (flash, three fireball, sixteen
puffs, the ring is the existing flat program).

## 3. Data

`data/space/explosion.json`, read into `sc_core::explosion::ExplosionData`, `deny_unknown_fields`, every number
validated (finite, in range) and named with its unit: `flash_s`, `fireball_s`, `ring_s`, `ring_reach_lengths`,
`cloud_s`, `cloud_puffs`, `puff_drift_m_s`, `puff_size_lengths`, colours as sRGB, `wreck_pieces`, `embers`,
`ember_life_s`, `char`.

## 4. Client only

`Visual::Destroyed` already reaches every client. The wreck lives in the client beside the booms and debris, cleared at
the next muster as they are. A late joiner sees no death it did not see happen.

## 5. Pi 5 budget

| Cost | Amount |
| --- | --- |
| Draw calls | Up to 20 glow sprites, 3 wreck pieces and 30 embers for 15 s after a death: about 50 draws in each view that sees it (the feed, and the bridge through its windows) |
| Fill | The cloud covers at most about a third of a 1228 x 388 feed, up to four sprites deep: about 0.6 million blended pixels a frame at its largest, for the first few seconds |
| Memory | One quad's vertices for the glow program, and the wreck's state: under 10 kB |
| CPU | 53 small transforms a frame for 15 s |

The fill is the cost to measure on the Pi; the puff count is the lever (`cloud_puffs`).

## 6. Checks

- `sc-core`: the data loads, and a bad number or an unknown key stops startup with its path.
- `sc-client`: the same seed draws the same death; every layer is gone by 15 s; the wreck is cleared at the muster.
- A film of the Hound's death from the captain's chair, TARGET at 1x and 4x, and from a window on the bridge.
- The Pi 5 measures the frame through the death (owner's hardware).

## Open questions

| Id | Question | Recommendation | Status |
| --- | --- | --- | --- |
| E1 | The Tern's own death | The same, when the Tern can die in view (today the round ends at once) | Recommendation taken (ask only with screenshots) |
