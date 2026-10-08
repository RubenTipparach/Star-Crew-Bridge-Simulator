# Tasks: the lobby

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta from the owner's "a lobby menu before starting a game, ability to name your player, maybe autogenerate from some name list, make it star trek themed" (2026-10-08).

## 2. Built

- [ ] 2.1 `data/crew/names.json` and `sc-core::names` (seeded, the deny list), with tests.
- [ ] 2.2 The UI layer: egui on a sokol painter, SDL input to egui.
- [ ] 2.3 The lobby screen in `sc-client`: officer, station, roll, Beam aboard; Join a crew shown, off.
- [ ] 2.4 Shots of the lobby in `docs/screenshots/engine/`.
- [ ] 2.5 The officer saved to `settings/officer.json`; the browser's storage later.
