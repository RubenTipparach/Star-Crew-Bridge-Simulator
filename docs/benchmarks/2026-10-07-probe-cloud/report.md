# sc-probe, 2026-10-07

**Not a Raspberry Pi 5.** This run is on Intel(R) Xeon(R) Processor @ 2.10GHz through llvmpipe (LLVM 20.1.2, 256 bits): it proves every scene draws, and its times say nothing about the Pi (CLAUDE.md 2, 12).

| Machine | RAM | OS | Video | GL | Output | 3D | Build | Frames a step | Warm-up | Repeats |
| --- | ---: | --- | --- | --- | --- | --- | --- | ---: | ---: | ---: |
| Intel(R) Xeon(R) Processor @ 2.10GHz | 16094 MB | Ubuntu 24.04.4 LTS | offscreen | OpenGL ES 3.2 Mesa 25.2.8-0ubuntu0.24.04.4 (llvmpipe (LLVM 20.1.2, 256 bits)) | 1920 x 1080 | 1280 x 720 | release | 60 | 15 | 2 |

| Scene | Step | Triangles | Draws | Frame p50 ms | p95 | p99 | Spread of p50 over repeats | CPU submit p50 ms | CPU a draw us |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | triangles-50000 | 48400 | 51 | 14.47 | 17.88 | 19.32 | 0.80 | 10.239 | 200.8 |
| 1 | triangles-100000 | 102400 | 51 | 19.72 | 25.65 | 28.28 | 0.70 | 15.441 | 302.8 |
| 1 | triangles-200000 | 202500 | 51 | 29.63 | 37.45 | 39.41 | 2.92 | 25.205 | 494.2 |
| 1 | triangles-300000 | 302500 | 51 | 36.23 | 47.63 | 56.58 | 6.06 | 32.575 | 638.7 |
| 1 | triangles-400000 | 396900 | 51 | 40.70 | 47.97 | 54.40 | 0.99 | 36.606 | 717.8 |
| 1 | triangles-500000 | 504100 | 51 | 47.06 | 56.02 | 61.69 | 2.07 | 42.840 | 840.0 |
| 1 | triangles-600000 | 592900 | 51 | 59.55 | 74.64 | 91.84 | 9.77 | 55.251 | 1083.3 |
| 1 | triangles-800000 | 792100 | 51 | 72.05 | 114.39 | 131.64 | 35.65 | 67.502 | 1323.6 |
| 1 | triangles-1000000 | 1000000 | 51 | 78.88 | 105.47 | 118.46 | 6.20 | 74.142 | 1453.8 |
| 2 | calls-50 | 102400 | 51 | 18.99 | 24.91 | 27.28 | 3.38 | 14.873 | 291.6 |
| 2 | calls-100 | 102400 | 101 | 17.52 | 20.49 | 22.04 | 1.00 | 13.549 | 134.1 |
| 2 | calls-200 | 102400 | 201 | 17.95 | 21.57 | 24.82 | 0.40 | 13.889 | 69.1 |
| 2 | calls-500 | 102400 | 501 | 18.68 | 23.36 | 26.42 | 0.83 | 14.547 | 29.0 |
| 2 | calls-1000 | 102400 | 1001 | 18.28 | 22.37 | 24.96 | 1.72 | 14.347 | 14.3 |
| 2 | calls-2000 | 102400 | 2001 | 19.98 | 26.07 | 27.20 | 0.59 | 15.881 | 7.9 |
| 3 | fill-1x-flat | 2 | 2 | 6.48 | 8.77 | 9.62 | 0.36 | 2.833 | 1416.7 |
| 3 | fill-1x-textured | 2 | 2 | 6.73 | 8.24 | 9.57 | 0.18 | 3.261 | 1630.3 |
| 3 | fill-2x-flat | 4 | 2 | 6.98 | 8.12 | 9.27 | 0.08 | 3.629 | 1814.6 |
| 3 | fill-2x-textured | 4 | 2 | 8.11 | 9.94 | 10.51 | 0.95 | 4.720 | 2360.0 |
| 3 | fill-4x-flat | 8 | 2 | 9.25 | 10.54 | 11.58 | 0.56 | 5.674 | 2836.8 |
| 3 | fill-4x-textured | 8 | 2 | 11.34 | 13.33 | 14.89 | 0.03 | 7.888 | 3944.2 |
| 3 | fill-8x-flat | 16 | 2 | 12.67 | 14.89 | 17.31 | 0.60 | 9.165 | 4582.5 |
| 3 | fill-8x-textured | 16 | 2 | 16.34 | 18.66 | 21.69 | 0.35 | 12.978 | 6489.1 |
| 3 | fill-4x-textured-msaa4 | 8 | 2 | 20.58 | 25.54 | 27.45 | 2.87 | 17.156 | 8577.8 |
| 4 | rtt-512x256 | 29400 | 4 | 7.61 | 9.35 | 10.59 | 0.73 | 4.293 | 1073.2 |
| 4 | rtt-1024x512 | 29400 | 4 | 8.42 | 10.27 | 11.81 | 0.26 | 4.920 | 1229.9 |
| 4 | rtt-2048x1024 | 29400 | 4 | 13.18 | 15.68 | 17.53 | 0.43 | 8.941 | 2235.3 |

Scene 8, sokol_gfx's memory: resident 84.4 MB before setup, 94.9 MB after (pools at `data/engine/render.json`'s sizes, so 10.56 MB for sokol_gfx and the context's first allocations), 184.0 MB with every scene's meshes, textures and targets made.

Not yet measured: scene 2 with instancing (the instanced program comes with the projectiles), scene 3 with a lightmap fetch (none planned, `light-baking`), scene 5 (UI: the egui painter), scene 6 (memory with the Tern's decks: `deckc`), scene 7 (the ship systems), scene 9 (the browser build).
