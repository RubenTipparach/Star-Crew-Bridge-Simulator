# The consoles draw the Tern's low poly model

`openspec/changes/console-ship-model`. Science's console, before (the mockup's committed state, the hull loft) and
after (the engine drawing the regenerated mockup state with `sc-client --console-fixture`): the camera widget beside the
viewscreen and the shield view now draw `tern_lod2`'s 496 triangles in its own colours. Rendered headless (lavapipe,
SwiftShader), with no GPU.

| File | What it shows | Requirement |
| --- | --- | --- |
| `before-after.png` | Top: before, the pressure hull's capsule. Bottom: after, the low poly Tern in the camera widget and inside the shield bubble | The consoles draw the Tern's low poly model |
| `science-mockup-before.png` | The mockup before | |
| `science-mockup-after.png` | The mockup after | |
| `science-engine-after.png` | The engine after, drawing the mockup's state | |

The model's sides reach past the shield bubble, which is sized to the hull loft (design 3): left for `weapons-and-shields`.
