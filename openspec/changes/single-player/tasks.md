# Tasks: single-player

The owner asked for it on 2026-10-11 (the proposal's words).

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta.
- [x] 1.2 The crew tabs and the mode switch in `docs/mockups/consoles.html` (the `solo-helm`, `solo-tactical-manual`
  and `red-solo-engineering` states) and their parity shots.

## 2. Build

- [x] 2.1 `platform::hand_over` (with `front-menu`), so the menu starts a game and the game returns to the menu
  (`--hand-over-test`: the menu, Beam aboard, Esc back, a capture of each).
- [x] 2.2 `sc-client::solo`: the drill server and the NPC sessions in the frame loop, named and seated.
- [x] 2.3 `Bot::manual`: seated and silent.
- [x] 2.4 The drill shows any crewed station's console and sends a MANUAL station's commands as its NPC's; a press on
  an AUTO station takes it MANUAL; Tab, Shift+Tab and M; READY held until the crew sit.
- [x] 2.5 The crew tabs and the switch in the engine's title band, at parity with the mockup (title band 0.0 missing
  on all three states).
- [x] 2.6 The in-process test (design 9), the headless captures (`docs/screenshots/engine/single-player/`, a round won
  with the crew), and the client's resident memory with a crew of four (design 8).
- [ ] 2.7 The same game on a Pi 5: frame time and memory with the crew aboard.
- [x] 2.8 The owner signs off the captures beside the mockup: Q3, 2026-10-16, "approve, but need a start mission button when everyone is ready, or captain can start even when everyone is not ready. it doesnt matter."
- [ ] 2.9 START MISSION: the captain (in single player, the player) starts the round before every seat is ready (`start-mission`).

## 3. Later

- [x] 3.1 A loopback transport in `sc-net`, so the browser hosts single player too: the `browser-single-player`
  change's local link.
- [ ] 3.2 Seven NPCs with Combat 2's stations (no change here: the roster grows).
- [x] 3.3 A way to stop `sc-net`'s signalling listener, so leaving a single-player game frees its thread and port
  (single player moved to the local link, which has no listener, in `browser-single-player`; a dedicated server's
  listener still runs until it exits, which is its whole life).
