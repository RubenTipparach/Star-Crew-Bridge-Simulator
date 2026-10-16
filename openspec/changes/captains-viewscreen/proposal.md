# Proposal: the captain's picture on every screen

## Why

The owner, 2026-10-15, after playing single player in the browser: "and see the captains screen on the main view
screen".

What the player expects is the Star Trek one: the captain says "on screen", and everyone sees it, on the bridge's big
screen and on their own console's VIEWSCREEN panel; the captain sees it too.

What the game did:

- **The bridge's main viewscreen** showed the captain's camera, CHASE and zoom (`chase-camera` 2, `viewscreen-zoom` 3).
  But a seated captain could not see it: a seated player sees the console full screen (CLAUDE.md 10), and he could not
  get up (`leave-station`).
- **Every console's VIEWSCREEN panel**, while the captain held the viewscreen, showed his camera with its own CHASE and
  zoom. A captain on CHASE at 10x put the bow camera at 1x on everyone else's panel: not his picture.
- **Taking the viewscreen** sent his camera alone, so the big screen kept the last CHASE and zoom he had pressed, which
  need not be what his own panel showed.

## What Changes

- **One camera on screen** (design 1): while the captain holds the viewscreen, every console's VIEWSCREEN panel and the
  bridge's main viewscreen show his camera, CHASE and zoom; otherwise each panel shows its own and the big screen the
  captain's. One function decides it for both.
- **SCREEN sends the whole picture** (design 2): taking the viewscreen sends his camera, CHASE and zoom with it.
- **The captain sees it** (design 3): seated, on his own panel, which is what goes up; on foot, on the big screen.

No new control or panel: the existing SCREEN button, camera pad, CHASE and zoom do what they promise.

## Impact

- `sc-client`: `console::on_screen`, used by `Seat::view` and the bridge's feed in `drill.rs`; SCREEN's command.
- Pi 5 budget: none (the same passes, a different camera).
