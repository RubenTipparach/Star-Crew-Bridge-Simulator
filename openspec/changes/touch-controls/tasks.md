# Tasks: touch-controls

The owner, 2026-10-13: "does the itch io game work with mobile? can you make it work with mobile?" (the proposal).

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta.
- [x] 1.2 The mockup, `docs/mockups/touch-controls.html`, played by finger or mouse: deck A walked with the stick and
  the look drag, USE at a seat, the embedded console with taps and long presses, STAND, the map, the menu, held upright.
- [x] 1.3 The owner approves the mockup (CLAUDE.md section 10), and the STAND button with it (2026-10-14: "Looks good.
  Implement that").

## 2. Build (after 1.3)

- [x] 2.1 `platform.rs`: `Event::Touch` from SDL's finger events; SDL's mouse from touch off for good (design 2), and
  `TouchMouse`, the first finger as the pointer, for the screens that take one (the UI's menus, a repair game).
- [x] 2.2 `data/input/touch.json` and its validation (units in the keys, unknown keys refused): read at the client's
  start, which stops on a bad file, and by the touch tests.
- [x] 2.3 `touch.rs`: the stick, the look, USE, DOWN, JUMP, MAP, MENU and STAND, drawn with egui's painter; feeding
  the drill's `held` and `pressed` keys, the stick's walk and the look's turn. The tests of design 7.
- [x] 2.4 Seated: `touch::ConsoleFinger` turns a finger into the console's pointer events through its `Hits` (the
  nearest control within 22 px, a long press for the tip); STAND holds the stand key.
- [x] 2.5 `web/index.html`: viewport, `touch-action`, the strip hidden on touch, full screen on the first tap, the
  turn-sideways card.

## 3. Proof

- [x] 3.1 `tools/web/smoke.mjs --touch`: the touch-only run at 844 x 390, run by `tools/web/release.sh` before every
  push; captures in `docs/screenshots/web/touch/`.
- [ ] 3.2 Side-by-side captures of the mockup and the engine, signed off by the owner.

## 4. Owner and hardware

- [ ] 4.1 itch.io: Mobile friendly, Landscape, Fullscreen button (design 6). An owner setting.
- [ ] 4.2 Played on a real Android phone and a real iPhone.
- [ ] 4.3 The phone's keyboard comes up for crew chat and the lobby name.
