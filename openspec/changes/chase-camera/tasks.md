# Tasks: chase-camera

The owner asked for it and for a video to approve on 2026-10-14 (the proposal).

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta.

## 2. Build

- [x] 2.1 `console::feed_view` and `console::chase_camera`, and their tests; `viewscreen.chase` in `data/space/exterior.json`, validated.
- [x] 2.2 CHASE a toggle on the camera chosen (the owner's word on the first video's frames); the feed draws the hull.
- [x] 2.3 `Command::MainView { feed, chase }`, `Bridge::main_view` and `main_chase`, the snapshot fields (protocol
  13); the captain's camera and CHASE presses send it; the bridge's viewscreens show it.
- [x] 2.4 `sc-client --film` and `tools/video/film.sh`; the video is `docs/videos/chase-camera.mp4`, the stills
  `docs/screenshots/engine/chase-camera/`.
- [x] 2.5 The lift settled on the final aim, so a near target keeps the ship low (seen on the bridge's main screen).

## 3. Gate

- [ ] 3.1 The owner watches the video and approves it.
- [ ] 3.2 The hull in the feed on a Pi 5: frame time with CHASE on.
