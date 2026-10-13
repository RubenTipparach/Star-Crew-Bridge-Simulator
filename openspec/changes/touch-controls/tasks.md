# Tasks: touch-controls

The owner, 2026-10-13: "does the itch io game work with mobile? can you make it work with mobile?" (the proposal).

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta.
- [x] 1.2 The mockup, `docs/mockups/touch-controls.html`: walking, at a station, seated, the map, held upright.
- [ ] 1.3 The owner approves the mockup (CLAUDE.md section 10), and the STAND button in `consoles.html`.

## 2. Build (after 1.3)

- [ ] 2.1 `platform.rs`: `Event::Touch` from SDL's finger events; the touch-mouse hint off while touch is in use.
- [ ] 2.2 `data/input/touch.json` and its validation (units in the keys, unknown keys refused).
- [ ] 2.3 `touch.rs`: the stick, the look zone, USE, DOWN, JUMP, MAP, MENU, drawn with the UI pen; feeding the
  keyboard's `held` and `pressed` sets and the look deltas. The tests of design 7.
- [ ] 2.4 `seat.rs`: taps through `Hit`, hit areas inflated to 44 CSS px, long press for the tooltip, STAND.
- [ ] 2.5 `web/index.html`: viewport, `touch-action`, the strip hidden on touch, full screen on the first tap, the
  turn-sideways card.

## 3. Proof

- [ ] 3.1 `tools/web/smoke.mjs`: the touch-only run at 844 x 390; captures in `docs/screenshots/web/touch/`.
- [ ] 3.2 Side-by-side captures of the mockup and the engine, signed off by the owner.

## 4. Owner and hardware

- [ ] 4.1 itch.io: Mobile friendly, Landscape, Fullscreen button (design 6). An owner setting.
- [ ] 4.2 Played on a real Android phone and a real iPhone.
- [ ] 4.3 The phone's keyboard comes up for crew chat and the lobby name.
