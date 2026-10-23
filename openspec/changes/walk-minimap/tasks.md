# Tasks: walk-minimap

The owner asked on 2026-10-17 (the proposal).

## 1. Write-up and mockup

- [x] 1.1 Proposal, design and spec delta.
- [x] 1.2 `docs/mockups/lib/minimap.js`, the one drawing for the mockups, from the layout and the deck build's patches.
- [x] 1.3 `ui-art-direction.html`: the "Minimap (proposed)" page, deck A with the first-time card, deck B, red alert, the
  smaller sizes; shots.
- [x] 1.4 `touch-controls.html`: the minimap in the MAP corner (`?mini=1`, the menu's button), the corridor, the bridge,
  red alert; shots.
- [x] 1.5 The contact sheet `docs/screenshots/mockups/walk-minimap/contact-sheet.png`.

## 2. Gate

- [ ] 2.1 The owner looks at the contact sheet and the open questions M1 to M5 (next survey).

## 3. Build (after 2.1)

- [ ] 3.1 `sc-core::nav`: one public `deck_of` (the map and `sc-tools` move onto it, finding F1); a deck's cells; the
  outlines per compartment from the walk grid and the deck's floor triangles, with tests (design 10).
- [ ] 3.2 `sc-client`: the ship map's bodies list in one function both maps call.
- [ ] 3.3 `sc-client`: the minimap on foot (`hud.rs`), its sizes in `data/ui/ui_style.json` and `data/input/touch.json`,
  the click and the tap to the full map on the current deck, the Options switch.
- [ ] 3.4 The phone: the minimap in MAP's corner, the objective card beside it (finding F3); `smoke.mjs --touch` taps it
  and sees the map.
- [ ] 3.5 Release check and parity against the mockup; captures in `docs/screenshots/engine/walk-minimap/` from a walk
  over all three decks and up a ladder.
