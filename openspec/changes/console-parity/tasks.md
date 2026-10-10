# Tasks: console-parity

## 1. One look

- [x] 1.1 Proposal, design and spec delta from the owner's "stations are not appearing the same as their approved
  mockups" (2026-10-10), with the gap tables (design 1).
- [x] 1.2 `data/ui/console_style.json` read out of `consoles.html` (`C`, `ICONS`) and `shipkit.js` (`PALETTE.role`)
  by `tools/ui/console_style.py`, with `--check` (design 3; replaces the three hand-made files first planned).
- [x] 1.3 The mockup is the source, so it reads nothing new and its pictures are unchanged; it gains
  `window.consoleState()` only.
- [x] 1.4 Icons drawn as vectors from their SVG in `vg.rs` (no atlas; design 3); the typeface by `tools/ui/fonts.py`,
  with `--check`.

## 2. The check

- [x] 2.1 Saved states: `consoles.html` `window.consoleState()`; `tools/consoles/parity.mjs` writes each named shot's
  picture and state to `docs/screenshots/parity/`.
- [x] 2.2 `sc-client --console-fixture` draws a saved state with the drill's console code.
- [x] 2.3 `tools/consoles/compare.py` (pixels and missing ink by region); steps 11-13 in `scripts/check.sh` and
  CLAUDE.md 12.

## 3. The engine

- [x] 3.1 A console module in `sc-client` (`console/`, `vg.rs`): the bands, panel style, icons and typeface; the
  status strip and title band.
- [x] 3.2 Helm to parity (design 1, 5); captures beside the mockup's.
- [x] 3.3 Tactical to parity, the same way.
- [ ] 3.4 The owner signs off both consoles' pictures (recorded here). Then the requirements move to
  `openspec/specs/console-parity/` and the change is archived.
- [ ] 3.5 The Mac runs `scripts/check.sh` when next connected; the Pi 5 measures the console's UI cost (design 7).

## 4. The core

- [x] 4.1 Strafe set points, with tests; the strafe pad live.
- [x] 4.2 Attitude orders, GO, FLIP and LEVEL (FN 6a), with tests.
- [x] 4.3 Autopilot HOLD, COURSE, CHASE, MATCH, EVADE (FN 5), with tests.
- [x] 4.4 Turret modes AUTO, TARGET, PD, HOLD per mount, turret heat, the layout's four mounts and the shield
  ellipsoid's faces, with tests; the drill's bots re-measured (8 of 8 seeds won).

## 5. The other stations

- [ ] 5.1 Engineering, Science and Captain built from their mockups under the same check when their simulations exist.
