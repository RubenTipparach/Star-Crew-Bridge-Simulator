# The Tern on a Pi 5, 2026-10-10

`sc-client --walk-test` (the scripted route: bridge, down both ladders to deck C, the lift A to C and back),
full screen, timed by Mesa's own counter (`GALLIUM_HUD=fps`, one sample about every 0.5 s, dumped with
`GALLIUM_HUD_DUMP_DIR`). The client has no frame timer of its own yet.

| Machine | RAM | OS | GL | Desktop | Cooler | Build |
| --- | ---: | --- | --- | --- | --- | --- |
| Raspberry Pi 5 Model B Rev 1.1 | 8 GB | Raspberry Pi OS (Debian 12 bookworm), desktop, labwc | OpenGL ES 3.1 Mesa 24.2.8 (V3D 7.1.10.2) | 2560 x 1080 ultrawide on HDMI | fan on the fan header (model not recorded) | release, `-C target-cpu=cortex-a76`, main @ 9ca3a83 |

The deck: `compiled/tern.deck` built on the Pi (`export_deck.mjs`, then `sc-tools deckc`): 45 compartments
loaded, 175,611 triangles, 167 texture layers, 9 bot crew. 3D at 1280 x 720 with MSAA 4x, scaled to the output.

| Run | Video | Output | Swap | FPS p5 | Median | Mean | Max | Samples |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| `fps-uncapped.txt` | x11 (XWayland, SDL's default here) | 2560 x 1080 | `vblank_mode=0` | 10.7 | 25.8 | 27.7 | 45.7 | 101 |
| `fps-vsync.txt` | x11 (XWayland) | 2560 x 1080 | vsync | 10.5 | 26.0 | 27.6 | 45.6 | 101 |
| `fps-wayland.txt` | wayland (`SDL_VIDEODRIVER=wayland`) | 2560 x 1080 | `vblank_mode=0` | 9.6 | 28.0 | 29.7 | 50.9 | 94 |
| `fps-wayland-1080p.txt` | wayland | 1920 x 1080 | `vblank_mode=0` | 16.4 | 28.3 | 30.7 | 50.2 | 90 |

The first two samples of each run (loading) are left out of the statistics; the files keep them.

What it says:

- **About 26-28 FPS on the decks (36-38 ms a frame), up to 40-50 in the lift's car.** The 60 Hz target is
  not met with every compartment drawn (doors stand open, no portal culling yet).
- **The GPU is the limit, not the CPU or vsync.** Vsync on and off give the same numbers; the whole machine's
  CPU use was 7% median (22% at most) during the uncapped run.
- **The output size does not matter**: 1920 x 1080 and 2560 x 1080 are within 1%, as they should be with the 3D
  target fixed at 1280 x 720.
- **Native Wayland is about 8% faster than SDL's default X11 path on the desktop.** On Raspberry Pi OS Lite
  (KMS/DRM, the documented setup) there is no compositor at all; that run is still to do.
- The single-sample drops (2-12 FPS) line up with the walk test's 10 captures (a read-back and a PNG each),
  not with the scene.
- Against `sc-probe` on the same Pi (`../2026-10-10-pi5-8gb-probe`): 200k triangles in 50 calls took 11.8 ms
  and MSAA 4x at 4x overdraw 16.8 ms, so the Tern's 36 ms is more than its triangle count alone predicts:
  the draw count, the three blended lighting states and the texture array are the places to look.

No thermal throttling (`vcgencmd get_throttled` 0x0; 54-56 C).

`shots/` holds three of the walk test's captures from the Wayland run, taken at 2560 x 1080 and scaled to 1280 x 540 JPEG to keep the repository small.
