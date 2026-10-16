# Nebula skies on a Pi 5, 2026-10-16

`openspec/changes/nebula-skies` task 6.1: what the sky costs on the Pi, measured two ways on Pi 1 (Raspberry Pi 5 Model
B Rev 1.1, 8 GB, Raspberry Pi OS bookworm with the desktop, labwc, OpenGL ES 3.1 Mesa 24.2.8, V3D 7.1.10.2), release
builds, nothing else running, no throttling (`vcgencmd get_throttled` 0x0, 54-55 C).

## 1. The sky pass alone: `sc-probe --scenes 10`

`target/release/sc-probe --scenes 10 --repeats 5 --frames 240 --warmup 30`, full screen (the window was the
display's 2560 x 1080; the report's "Output" column prints the probe's default 1920 x 1080). Each frame draws one sky
into a 1280 x 720 target with MSAA 4x (the bridge's and the feed's), blits it and waits for the GPU (`glFinish`), so a
frame is the whole cost of one view's sky plus the blit. The steps run in turn inside each repeat, so old and new are
interleaved. `probe/report.md` and `probe/report.json`.

| Step | Frame p50 ms | p95 | Spread of p50 over 5 repeats | Draws (with the blit) |
| --- | ---: | ---: | ---: | ---: |
| The old sky (the black octahedron: faint stars, sun, planet) | 20.45 | 21.67 | 1.07 | 3 |
| Bloom's mesh, no vector stars | 21.32 | 22.16 | 0.66 | 3 |
| Bloom with its 1,600 stars, at rest | 21.45 | 22.20 | 0.63 | 5 |
| Bloom turning at 70 degrees a second (every star at the 1.7 degree cap) | 21.23 | 22.78 | 0.27 | 5 |

So the nebula costs about 1 ms a view at 1280 x 720 with MSAA 4x, the mesh nearly all of it; the 1,600 lines and 226
round heads are within the spread, at rest or streaking.

**The first build cost twice that** (`probe-first-build/`): with the faint stars computed in the mesh's own fragment
shader, Bloom's mesh took 22.25 ms against the old full-screen sky's 19.99 ms, +2.3 ms. The faint stars are the sky's
dearest per-pixel work (three hashed grids), and on a mesh of small triangles a pixel on a triangle's edge is shaded
once for each triangle that touches it, so that work was paid again along every edge. The build that shipped moves them
into their own full-screen triangle, added in proportion to the alpha the mesh leaves (`shaders/sky.glsl`, `faint`):
the mesh's fragment work is now the vertex colour, the sun, the planet and the dither.

## 2. The game: the captain's film in real time

`sc-client --solo --station captain --film` (the chase-camera film's script played on the clock: the walk to the
captain's chair, sitting, the console with the viewscreen's cameras, chase on, the bridge's viewscreen), full screen
at 2560 x 1080, `vblank_mode=0`, timed by Mesa's own counter (`GALLIUM_HUD=frametime`, dumped one line a frame with
`GALLIUM_HUD_DUMP_DIR`, `GALLIUM_HUD_VISIBLE=false`). 80 s a run, the first 15 s (loading, muster) dropped. Before and
after alternated three times each in one sitting. Before is the release build of the working tree at 09:13 that day
(e4c43b3 with the chase-camera smoothing then in hand), run against `HEAD`'s data without the sky; after is this
change's build. `game/<before|after>-<n>-frametime-ms.txt` are the raw frame times.

| Build | Frames kept | Frame p50 ms | p95 | p99 | Mean | Spread of p50 over 3 runs |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Before (no nebula) | 12,548 | 14.25 | 23.89 | 34.34 | 15.32 | 0.05 |
| After (Bloom, its stars, the hull tint) | 12,098 | 14.83 | 24.47 | 34.38 | 15.89 | 0.01 |

The game's median frame is 0.58 ms longer (4 %), far above the 0.05 ms spread: a real cost, within the probe's 1 ms a
view (the film shows one or two skies a frame, the bridge's windows and the feed). p99 is unchanged. The frame stays
under the 16.7 ms target at the median.

## What it does not say

- A 1 GB Pi 5 was not measured; this Pi has 8 GB. The loaded sky's buffers are about 305 KB (224 KB of mesh, 82 KB of stars).
- 8,000 against 4,000 vertices (task 6.1) was not measured: there is no 4,000-vertex sky to load. The mesh's cost is
  mostly its edges, so half the vertices would be expected to save roughly half of the 1 ms; not a result until run.
- `GL_ALIASED_LINE_WIDTH_RANGE` was not queried. The lines are 1 px, which OpenGL ES 3.0 always allows.
