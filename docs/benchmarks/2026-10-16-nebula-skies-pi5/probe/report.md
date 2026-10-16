# sc-probe, 2026-10-16

| Machine | RAM | OS | Video | GL | Output | 3D | Build | Frames a step | Warm-up | Repeats |
| --- | ---: | --- | --- | --- | --- | --- | --- | ---: | ---: | ---: |
| Raspberry Pi 5 Model B Rev 1.1 | 8059 MB | Debian GNU/Linux 12 (bookworm) | x11 | OpenGL ES 3.1 Mesa 24.2.8-1~bpo12+rpt5 (V3D 7.1.10.2) | 1920 x 1080 | 1280 x 720 | release | 240 | 30 | 5 |

| Scene | Step | Triangles | Draws | Frame p50 ms | p95 | p99 | Spread of p50 over repeats | CPU submit p50 ms | CPU a draw us |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 10 | sky-old | 8 | 3 | 20.45 | 21.67 | 24.48 | 1.07 | 0.202 | 67.2 |
| 10 | sky-nebula-no-stars | 15982 | 3 | 21.32 | 22.16 | 24.74 | 0.66 | 0.198 | 65.9 |
| 10 | sky-nebula | 15982 | 5 | 21.45 | 22.20 | 24.04 | 0.63 | 0.244 | 48.8 |
| 10 | sky-nebula-turning | 15982 | 5 | 21.23 | 22.78 | 23.61 | 0.27 | 0.241 | 48.2 |

Scene 8, sokol_gfx's memory: resident 15.1 MB before setup, 19.7 MB after (pools at `data/engine/render.json`'s sizes, so 4.64 MB for sokol_gfx and the context's first allocations), 19.9 MB with every scene's meshes, textures and targets made.

Not yet measured: scene 2 with instancing (the instanced program comes with the projectiles), scene 3 with a lightmap fetch (none planned, `light-baking`), scene 5 (UI: the egui painter), scene 6 (memory with the Tern's decks: `deckc`), scene 7 (the ship systems), scene 9 (the browser build). Scene 10 (the sky pass) counts the sky mesh's triangles; the stars' lines and heads are two more draws.
