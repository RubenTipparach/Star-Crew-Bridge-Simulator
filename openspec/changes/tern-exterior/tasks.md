# Tasks: tern-exterior

The owner asked for it on 2026-10-11 ("can you start integrateing the TERN ship model in to the game?", "so players
can look out the window na dsee their ship").

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta; `warp-pylons` written beside it for going outside.

## 2. The hull in the engine

- [x] 2.1 `sc-tools/src/glb.rs`: the glTF binary reader and its tests on the committed model.
- [x] 2.2 The colour byte and the mip chain shared by the rooms and the hull.
- [x] 2.3 Deck file version 6: the optional hull in the index, its checks in `read`, a round-trip test.
- [x] 2.4 `deckc` compiles the hull from `props.json`'s default livery; manifest and budget checks; packing test.
- [x] 2.5 The client loads the hull and draws its seven parts after the rooms, in the walk and on the drill's bridge.

## 3. The windows in the decks

- [x] 3.1 `tools/exterior_windows.py` generates through the runtime chain (command suite, deck access, engineering,
  armory) and its `--check` holds the windows to the model's apertures; in `scripts/check.sh`.
- [x] 3.2 The deck plan applies the patch last; `tools/mockups/inline.py` inlines it; the layout check passes with it.
- [x] 3.3 The eight rooms baked again (rooms and light probes), the caches rewritten, the deck exported and compiled.
- [x] 3.4 `tools/crew_rooms.py` checks its furniture with the windows applied (nothing taller than a sill in the metre
  in front of a window); the quarters' two locker banks move off the hull side to the bunk island's ends.

## 4. Verification

- [x] 4.1 Headless captures in `docs/screenshots/exterior/` (design 8), looked at before they are shown.
- [x] 4.2 `scripts/check.sh` passes.
- [ ] 4.3 The Pi 5 measures the walk's frame time and memory with the hull (owner, on hardware).
- [ ] 4.4 The owner signs off the captures (O1, survey 2).
