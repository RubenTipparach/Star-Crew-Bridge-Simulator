# Tasks: front-menu

The owner asked for it on 2026-10-11 ("make sure first buttons on menu are single player, multiplayer, options, quit").

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta.

## 2. Build

- [x] 2.1 `platform::hand_over`: the window passes from one app to the next (`platform.rs`).
- [x] 2.2 The four buttons and their pages in `lobby.rs`; Single Player's Beam aboard and Tour; Multiplayer's server
  and Join; Quit (off in the browser).
- [x] 2.3 Options: Display, applied at once, kept in `settings/options.json`, repaired on a bad file, with its test
  (`options.rs`).
- [x] 2.4 `--lobby-test` walks the pages; its captures in `docs/screenshots/engine/` (`menu-1-main.png`,
  `menu-2-multi.png`, `menu-3-options.png`, `lobby-1.png`, `lobby-2-rolled.png`, `lobby-3-aboard.png`).
- [x] 2.5 Beam aboard into a single-player game and back to the menu in one window: `--hand-over-test` and its three
  captures.
- [x] 2.6 The owner signs off the captures: X1, 2026-10-16, "approve, ".
