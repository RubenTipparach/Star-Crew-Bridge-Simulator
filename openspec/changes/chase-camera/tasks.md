# Tasks: chase-camera

The owner asked for it and for a video to approve on 2026-10-14 (the proposal).

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta.

## 2. Build

- [x] 2.1 `console::chase_camera` and its tests; `viewscreen.chase` in `data/space/exterior.json`, validated.
- [x] 2.2 The seat's chase angles, set by dragging the camera widget while CHASE is on; the feed draws the hull.
- [x] 2.3 `Command::MainView`, `Bridge::main_view`, the snapshot field (protocol 13); the captain's camera presses
  send it; the bridge's viewscreens show it.
- [x] 2.4 `sc-client --film` and `tools/video/film.sh`.

## 3. Gate

- [ ] 3.1 The owner watches the video and approves it.
- [ ] 3.2 The hull in the feed on a Pi 5: frame time with CHASE on.
