# Tasks: console-parity

## 1. One look

- [x] 1.1 Proposal, design and spec delta from the owner's "stations are not appearing the same as their approved
  mockups" (2026-10-10), with the gap tables (design 1).
- [ ] 1.2 `data/ui/palette.json`, `console_style.json`, `icons.json` from `consoles.html`'s `C`, layout constants and
  `ICONS`; validated (unknown keys stop it).
- [ ] 1.3 `inline.py` `data:ui/*` blocks; `consoles.html` reads them; its shots pixel-identical before and after.
- [ ] 1.4 `tools/ui/icons.py`: the icon atlas and its manifest, reproducible.

## 2. The check

- [ ] 2.1 `data/consoles/states.json`: named console states (`engage` first) as fixed snapshots.
- [ ] 2.2 `consoles.html` `MOCKUP_LAYOUT(station, state)`; `shoot.mjs --layout` writes the mockup's manifests.
- [ ] 2.3 `tools/consoles/parity.py`; a row in `scripts/check.sh` and CLAUDE.md 12.

## 3. The engine

- [ ] 3.1 A console module in `sc-client` drawing the bands, panel style, icons and typeface from `data/ui`; the
  status strip and title band.
- [ ] 3.2 Helm to parity (design 1, 5), unavailable controls drawn as such; `--console-layout`; the parity check
  passes; captures beside the mockup's.
- [ ] 3.3 Tactical to parity, the same way.
- [ ] 3.4 The owner signs off both consoles' pictures (recorded here).

## 4. The core

- [ ] 4.1 Strafe set points (FN 3), with tests; the strafe pad live.
- [ ] 4.2 Attitude orders, GO and FLIP (FN 6a), with tests.
- [ ] 4.3 Autopilot Hold, Course, Chase, Match, Evade (FN 5), with tests.
- [ ] 4.4 Turret modes AUTO and HOLD per mount; the drill's bot re-measured.

## 5. The other stations

- [ ] 5.1 Engineering, Science and Captain built from their mockups under the same check when their simulations exist.
