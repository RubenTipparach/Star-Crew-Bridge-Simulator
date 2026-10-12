# Tasks: game-menu

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta from the owner's "esc should bring up a menu to return to menu, options, or exit" (2026-10-10).
- [x] 1.2 Mockup states 3, 4 and 5 in `docs/mockups/game-menu.html`, over engine captures.
- [x] 1.3 The owner approves the mockup (2026-10-10: "Approve menu").

## 2. Build (on a request to build it)

- [x] 2.1 The menu panel in `sc-client` (`game_menu.rs`), opened by Esc in the walk, the drill and seated. Not yet: Start on a pad (the platform reads no pads yet), and Esc on the lobby (its own Options and Quit stand in).
- [x] 2.2 Return to menu (leaves the crew, back to the lobby) and Exit, each with its Yes or No.
- [x] 2.3 Options and `settings/options.json`, applied at once, repaired on a bad file; one list of rows for the front menu and the game menu.
- [x] 2.4 Esc steps back one layer at a time (OBJECTIVES over a console first).
- [x] 2.5 The headless tests of design 6 (`game_menu` and `options` tests) and the capture `docs/screenshots/engine/game-menu-captain.jpg` (the drill's scripted `7-menu` shot).
