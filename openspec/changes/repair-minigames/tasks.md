# Tasks: repair mini-games

## 1. Write-up and mockups

- [x] 1.1 Proposal, design and spec delta from the owner's repair list and "a medbay minigame for healing crew" (2026-10-08).
- [x] 1.2 `docs/mockups/repairs.html` and `docs/mockups/repairs/kit.js`: the menu, the job's bar, the step rule, the combat shake.
- [x] 1.3 The seventeen system games and the medic's treatment, one script each, playable with mouse and keys.
- [x] 1.4 Shots of every game in `docs/screenshots/repairs/`; published for the owner.
- [ ] 1.5 Move the repeated part-step drag, the star order and the drag helper into `kit.js` (CLAUDE.md 6.1).

## 2. Rules (after the owner has played them)

- [ ] 2.1 `damage::repair_time` gains steps and `repair.fumble_share`; tests that a clean run matches the preview.
- [ ] 2.2 Zero gravity in `crew-on-deck` (section 4), the galley's supply, the heads' comfort: their own changes.
- [ ] 2.3 `warp-pylons`: the hull addition, EVA only, out of combat.

## 3. In the engine

- [ ] 3.1 Each game in the UI layer, at the repair points the layout names.
