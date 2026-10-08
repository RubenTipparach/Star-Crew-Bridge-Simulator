# Tasks: normal maps

## 1. Write-up and comparison

- [x] 1.1 Proposal, design and spec delta, from the owner's "Do you have normals for stuff? Should probably add normal maps to give things that modern textured look" (2026-10-08).
- [ ] 1.2 Normal layers for the bridge's panels and props (Blender bakes from the same detail geometry), colour layers without the baked key light beside them.
- [ ] 1.3 The bake writes `L` and `k` per vertex per state (`lightbake.js`); the deck plan's `?normals=1` shades the bridge by design section 3.
- [ ] 1.4 Screenshots of the bridge, today and option C, in the three lighting states, in `docs/screenshots/mockups/normal-maps/`; into the survey for the owner.

## 2. On the owner's word and the Pi probe

- [ ] 2.1 The Pi 5 probe: the deck pass at 1280 x 720, today's shader against option C's, interleaved, repeats and spread (design section 5).
- [ ] 2.2 Every panel, material and prop bake writes its normal layer; the colour bakes drop their key light.
- [ ] 2.3 `light-baking`'s baker writes `L` and `k`; `deckc` writes the 36-byte vertex; the engine's deck shader shades by design section 3.
- [ ] 2.4 CLAUDE.md section 9 amended in the same commit; the budget table gets the measured memory.
