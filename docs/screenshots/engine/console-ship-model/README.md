# The consoles draw the Tern's low poly model

`openspec/changes/console-ship-model`. Science's console, before (the mockup's committed state, the hull loft) and
after (the engine drawing the regenerated mockup state with `sc-client --console-fixture`): the camera widget beside the
viewscreen and the shield view now draw `tern_lod2`'s 496 triangles in its own colours. Rendered headless (lavapipe,
SwiftShader), with no GPU.

| File | What it shows | Requirement |
| --- | --- | --- |
| `before-after.png` | Top: before, the pressure hull's capsule in the old bubble. Bottom: after, the low poly Tern in the camera widget and wholly inside the grown shield bubble | The consoles draw the Tern's low poly model; the shield bubble envelops the ship |
| `science-mockup-before.png` | The mockup before | |
| `science-mockup-after.png` | The mockup after | |
| `science-engine-after.png` | The engine after, drawing the mockup's state | |

The shield bubble grew to envelop the whole ship (design 3; owner, 2026-10-14): 27 x 10.5 x 59.5 m, the smallest that holds the exterior model with 1 m to spare.
