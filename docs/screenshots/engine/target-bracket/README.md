# Target bracket and viewscreen zoom

`openspec/changes/target-bracket` and `openspec/changes/viewscreen-zoom`. Engine frames from the chase camera's film
(`docs/videos/chase-camera.mp4`, `sc-client --headless --solo --station captain --film`), and the mockup's Tactical
console (`node tools/mockups/shoot.mjs docs/mockups/consoles.html --only tactical`). Taken in a Claude Code cloud
session on lavapipe and SwiftShader, with no GPU: they show what is drawn, not how fast.

| File | What it shows | Requirement |
| --- | --- | --- |
| `0-mockup-tactical.png` | The mockup: the bracket's corners, HOSTILE and the hull bar above, `T1 8.1 km` below; the zoom column left of the camera widget | The target bracket frames the target; every viewscreen zooms in five steps |
| `1-target-chase-1x.png` | TARGET, chase on, 1x: the Hound far off, bracketed, past the Tern | The bracket follows the chase camera |
| `2-target-bow-1x.png` | TARGET from the bow at 1x | The target bracket frames the target |
| `3-target-bow-4x.png` | The same at 4x: the corners close round the Hound's hull | The bracket closes round the hull; zoom |
| `4-target-bow-20x.png` | 20x: the Hound fills the screen, the corners beyond its edges, the texts kept inside | Zoom; texts inside the screen |
| `5-target-chase-4x.png` | TARGET, chase on, 4x: the Tern its 1x size and low, the Hound magnified in its bracket beyond it | Zoomed chase |
| `sheet.png` | The engine frames on one page | |
