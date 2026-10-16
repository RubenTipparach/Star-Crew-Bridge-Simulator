# Tasks: chase-orbit

The owner asked on 2026-10-16 (the proposal).

## 1. Write-up

- [x] 1.1 The rough turn measured on the Pi (design 1); proposal, design and spec delta.

## 2. Build

- [x] 2.1 The playback clock and interpolation by tick (`sc-net::session`); tests.
- [x] 2.2 `viewscreen.chase`'s `follow_tau_s`, `blend_s` and `orbit` in `data/space/exterior.json`, validated.
- [x] 2.3 `console::rig`: the follow, the blend and the orbit; tests (converges, framerate independent, no overshoot,
  orbit clamps, the bracket agrees with the smoothed camera).
- [x] 2.4 The feed and the bracket from one camera (`ConsoleView::cam`); the bridge's main viewscreen's rig.
- [x] 2.5 ORBIT in the mockup first, then the engine's button, drag, wheel, pinch and the O key.
- [x] 2.6 `Command::MainView`'s orbit, `Bridge::main_orbit`, the snapshot (protocol 17), `on_screen`; round trip test.
- [ ] 2.7 The video and the shots: CHASE turning, before and after, and ORBIT round the ship.

## 3. Gate

- [ ] 3.1 The owner watches the video and approves it.
