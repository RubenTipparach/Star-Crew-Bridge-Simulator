# Chase camera

`openspec/changes/chase-camera`. Stills from the film, `docs/videos/chase-camera.mp4`, rendered with
`sc-client --headless --solo --station captain --film --shots DIR` and encoded by `tools/video/film.sh`. Taken in a
Claude Code cloud session on lavapipe, with no GPU: they show what the screens draw, not how fast (frame time not
measured). The player walks to the captain's chair and sits before any of these.

| File | What it shows | Requirement |
| --- | --- | --- |
| `2-fwd-chase-off.png` | FWD, chase off: the captain's viewscreen from the bow, as before | Chase off is the bow camera |
| `3-fwd-chase-on.png` | FWD, chase on: from behind the Tern, the ship low and centred | CHASE toggles the chase camera |
| `4-target-chase-on.png` | TARGET, chase on: past the Tern at the Hound, the Hound in the middle, the ship low | CHASE toggles the chase camera; the target in the middle |
| `5-port-chase-on.png` | PORT, chase on | CHASE toggles the chase camera, on every camera |
| `6-aft-chase-on.png` | AFT, chase on | The same |
| `7-stbd-chase-on.png` | STBD, chase on | The same |
| `8-target-chase-off.png` | TARGET, chase off: first person from the bow again | Chase off is the bow camera |
| `9-main-viewscreen-target-chase.png` | The bridge's main viewscreen, seen from behind the captain's chair, showing his camera: TARGET, chase on | The main viewscreen shows the captain's camera |
| `sheet.png` | All of the above on one page | |
