# Tasks: browser-page-fit

The owner, 2026-10-15: "I'm stuck, I cant press the captain button, going full screen in browser is broken, mouse is
stuck offset, its basically unlpayable on itch" (the proposal).

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta, with the owner's words; the diagnosis of design 1 read in SDL 3.4.18's
  emscripten driver.

## 2. Build

- [x] 2.1 `platform::open`: the browser's window `RESIZABLE | HIGH_PIXEL_DENSITY`; the window's size said on each
  change (design 1).
- [x] 2.2 `web/index.html`: no `fit()`, no key strip, no note, the canvas edge to edge; the pixel ratio capped at 2
  (designs 1 and 2).
- [x] 2.3 `data/ui/reference.json` and `ui::frame_in` / `ui::Fit`, the one mapping; the front menu, the game menu,
  the briefing, the bridge band, the consoles, the repairs menu, deck repair and the touch layer use it; the copies in
  `console::fit`, `repairs.rs`, `deckrepair.rs` and the drill's `CANVAS` are gone (design 5).
- [x] 2.4 Tests: the frame fits every output of design 5 centred, a click lands where it was drawn at pixel ratios 1 to
  3, no screen reads the output's size, the briefing and the band fit every output.
- [x] 2.5 `tools/web/smoke.mjs --fit`, run by `tools/web/release.sh` (design 3).
- [ ] 2.6 The keys' square panel in Options, once its mockup is approved (design 2; the mockup is separate work).

## 3. Single player as a released game (design 4)

- [x] 3.1 The pass at 1280 x 720 and 1920 x 1080, at pixel ratios 1 and 2, and in full screen; the findings in
  design 6, each blocker fixed with its test.
- [x] 3.2 Captures in `docs/screenshots/web/fit/` (each size, and `play/` the walk) and the walking playtest clip
  `docs/videos/2026-10-16-browser-single-player-walk.mp4`.

## 4. Gate

- [ ] 4.1 The owner plays single player on itch.io after the merge and signs off.
