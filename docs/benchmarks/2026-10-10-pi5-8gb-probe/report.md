# sc-probe, 2026-10-10

| Machine | RAM | OS | Video | GL | Output | 3D | Build | Frames a step | Warm-up | Repeats |
| --- | ---: | --- | --- | --- | --- | --- | --- | ---: | ---: | ---: |
| Raspberry Pi 5 Model B Rev 1.1 | 8059 MB | Debian GNU/Linux 12 (bookworm) | x11 | OpenGL ES 3.1 Mesa 24.2.8-1~bpo12+rpt5 (V3D 7.1.10.2) | 1920 x 1080 | 1280 x 720 | release | 120 | 30 | 3 |

| Scene | Step | Triangles | Draws | Frame p50 ms | p95 | p99 | Spread of p50 over repeats | CPU submit p50 ms | CPU a draw us |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | triangles-50000 | 48400 | 51 | 10.32 | 11.06 | 21.32 | 0.11 | 0.296 | 5.8 |
| 1 | triangles-100000 | 102400 | 51 | 10.55 | 11.44 | 12.19 | 0.02 | 0.277 | 5.4 |
| 1 | triangles-200000 | 202500 | 51 | 11.78 | 13.11 | 15.08 | 0.23 | 0.298 | 5.9 |
| 1 | triangles-300000 | 302500 | 51 | 13.32 | 14.61 | 15.87 | 0.38 | 0.396 | 7.8 |
| 1 | triangles-400000 | 396900 | 51 | 14.49 | 15.49 | 17.10 | 0.27 | 0.422 | 8.3 |
| 1 | triangles-500000 | 504100 | 51 | 15.90 | 16.82 | 18.22 | 0.54 | 0.428 | 8.4 |
| 1 | triangles-600000 | 592900 | 51 | 17.12 | 18.53 | 20.02 | 0.75 | 0.450 | 8.8 |
| 1 | triangles-800000 | 792100 | 51 | 18.94 | 20.32 | 21.62 | 0.90 | 0.387 | 7.6 |
| 1 | triangles-1000000 | 1000000 | 51 | 22.28 | 23.93 | 25.09 | 0.20 | 0.498 | 9.8 |
| 2 | calls-50 | 102400 | 51 | 10.70 | 11.53 | 12.23 | 0.21 | 0.384 | 7.5 |
| 2 | calls-100 | 102400 | 101 | 10.76 | 11.32 | 12.34 | 0.24 | 0.444 | 4.4 |
| 2 | calls-200 | 102400 | 201 | 10.88 | 11.35 | 12.33 | 0.24 | 0.576 | 2.9 |
| 2 | calls-500 | 102400 | 501 | 11.30 | 11.64 | 12.50 | 0.35 | 0.989 | 2.0 |
| 2 | calls-1000 | 102400 | 1001 | 11.97 | 12.30 | 13.32 | 0.59 | 1.666 | 1.7 |
| 2 | calls-2000 | 102400 | 2001 | 13.29 | 13.62 | 14.74 | 0.44 | 2.874 | 1.4 |
| 3 | fill-1x-flat | 2 | 2 | 9.71 | 10.05 | 11.01 | 0.14 | 0.157 | 78.4 |
| 3 | fill-1x-textured | 2 | 2 | 10.02 | 10.44 | 11.20 | 0.13 | 0.147 | 73.3 |
| 3 | fill-2x-flat | 4 | 2 | 10.00 | 10.40 | 11.11 | 0.12 | 0.139 | 69.4 |
| 3 | fill-2x-textured | 4 | 2 | 10.70 | 10.97 | 11.56 | 0.12 | 0.132 | 65.9 |
| 3 | fill-4x-flat | 8 | 2 | 10.61 | 10.90 | 11.43 | 0.08 | 0.142 | 71.1 |
| 3 | fill-4x-textured | 8 | 2 | 12.19 | 12.47 | 13.21 | 0.11 | 0.155 | 77.5 |
| 3 | fill-8x-flat | 16 | 2 | 11.92 | 12.24 | 12.91 | 0.13 | 0.150 | 75.1 |
| 3 | fill-8x-textured | 16 | 2 | 15.17 | 15.45 | 16.04 | 0.12 | 0.161 | 80.4 |
| 3 | fill-4x-textured-msaa4 | 8 | 2 | 16.83 | 18.23 | 19.68 | 0.11 | 0.217 | 108.6 |
| 4 | rtt-512x256 | 29400 | 4 | 8.82 | 9.30 | 10.08 | 0.10 | 0.138 | 34.6 |
| 4 | rtt-1024x512 | 29400 | 4 | 9.23 | 9.58 | 10.46 | 0.11 | 0.136 | 34.0 |
| 4 | rtt-2048x1024 | 29400 | 4 | 11.18 | 11.58 | 12.47 | 0.12 | 0.149 | 37.3 |

Scene 8, sokol_gfx's memory: resident 15.6 MB before setup, 20.0 MB after (pools at `data/engine/render.json`'s sizes, so 4.41 MB for sokol_gfx and the context's first allocations), 20.1 MB with every scene's meshes, textures and targets made.

Not yet measured: scene 2 with instancing (the instanced program comes with the projectiles), scene 3 with a lightmap fetch (none planned, `light-baking`), scene 5 (UI: the egui painter), scene 6 (memory with the Tern's decks: `deckc`), scene 7 (the ship systems), scene 9 (the browser build).
