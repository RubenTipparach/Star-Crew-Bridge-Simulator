# sc-probe, 2026-10-16

| Machine | RAM | OS | Video | GL | Output | 3D | Build | Frames a step | Warm-up | Repeats |
| --- | ---: | --- | --- | --- | --- | --- | --- | ---: | ---: | ---: |
| Raspberry Pi 5 Model B Rev 1.1 | 8059 MB | Debian GNU/Linux 12 (bookworm) | x11 | OpenGL ES 3.1 Mesa 24.2.8-1~bpo12+rpt5 (V3D 7.1.10.2) | 1920 x 1080 | 1280 x 720 | release | 240 | 30 | 5 |

| Scene | Step | Triangles | Draws | Frame p50 ms | p95 | p99 | Spread of p50 over repeats | CPU submit p50 ms | CPU a draw us |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 10 | sky-old | 8 | 2 | 19.99 | 21.39 | 23.32 | 1.20 | 0.180 | 90.0 |
| 10 | sky-nebula-no-stars | 15982 | 2 | 22.25 | 22.94 | 24.44 | 0.50 | 0.195 | 97.5 |
| 10 | sky-nebula | 15982 | 4 | 22.41 | 23.10 | 25.32 | 0.56 | 0.242 | 60.4 |
| 10 | sky-nebula-turning | 15982 | 4 | 22.05 | 23.63 | 24.74 | 0.39 | 0.204 | 51.0 |

Scene 8, sokol_gfx's memory: resident 15.1 MB before setup, 20.0 MB after (pools at `data/engine/render.json`'s sizes, so 4.90 MB for sokol_gfx and the context's first allocations), 20.1 MB with every scene's meshes, textures and targets made.

Not yet measured: scene 2 with instancing (the instanced program comes with the projectiles), scene 3 with a lightmap fetch (none planned, `light-baking`), scene 5 (UI: the egui painter), scene 6 (memory with the Tern's decks: `deckc`), scene 7 (the ship systems), scene 9 (the browser build). Scene 10 (the sky pass) counts the sky mesh's triangles; the stars' lines and heads are two more draws.
