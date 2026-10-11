# Tasks: crew-nameplates

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta from the owner's "I want to always see peoples name in the game" and "show if they are (npc)" (2026-10-10).
- [x] 1.2 Mockup states 1 and 2 in `docs/mockups/game-menu.html`, over engine captures.
- [x] 1.3 The owner approves the mockup (2026-10-10: "Approve menu", the names and Esc-menu mockup).

## 2. Build (on a request to build it)

- [x] 2.1 The line of sight in `sc-core` (`WalkWorld::sees`, a ray cast; design 2 says why not the grid walk), with its tests.
- [x] 2.2 One nameplate routine in `sc-client` (`nameplate.rs`), drawn by the walk and the drill, nearest on top.
- [x] 2.3 (NPC) for bots and a single player's crew; the Names option read from `settings/options.json`.
- [x] 2.4 Captures of the walk and the drill: `docs/screenshots/engine/nameplates-walk.jpg`, `nameplates-drill.jpg`.
