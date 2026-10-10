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

- [ ] 3.1 Each game in the UI layer, at the repair points the layout names (the points: `repairs-on-deck` 3.2).
- [x] 3.2 Design 8: `sc-core::repair`, the job and the server's checks, with tests (with `repairs-on-deck` 3.1).
- [x] 3.3 Design 8: the data files (`data/repairs/`, `repairs.json`, `fumble_share_pct`), validated by `check-data`.
- [x] 3.4 Design 8: `sc-repairs`, the kit: frame, bar, cover screws, guide card and icons, input, shake.
- [x] 3.5 Design 8: every game in `sc-repairs::games`, played to the end by its hand in a test, with a fumble checked.
- [ ] 3.6 Design 8: `sc-client --repairs`, the menu and the games; captures beside the mockups' shots.

## 4. Guides (owner, 2026-10-09)

- [x] 4.1 The kit's ? button and card (design 6g); a guide in every game's registration; shown once on a game's first open.

## 5. Pump and chiller (owner, 2026-10-09)

- [x] 5.1 `repairs/pump.js` and `repairs/chiller.js` (design 6h), in the menu and opened from the reactor system screen;
  played by mouse and touch, damaged, disabled and a fumble; shots.

## 6. Rounds land at once and get harder (owner, 2026-10-09)

- [x] 6.1 Design 1 and 1a, the proposal and the spec delta: a round lands its share at once; every round is the same
  game at the next level; the heads become two jobs.
- [x] 6.2 `kit.js`: rounds land at once, the level passed to every game; the part move opens round 1.
- [x] 6.3 Every game's knobs by level (design 1a's table); the alternating games (heads, chiller, conduits, turret,
  pipes, fighter, shuttle, biobed) play their whole game every round.
- [x] 6.4 The levels as data (design 8 put them in `data/repairs/<game>.json`, the jobs in `data/ships/tern/repairs.json`), validated; shots of a level 1 and a level 3 round per game in `docs/screenshots/engine/repairs/`.

## 7. In the engine, after the port (2026-10-10)

- [ ] 7.1 The kit takes the helpers the ports wrote for themselves (design 8a): ellipse, dashed ring, Bezier curves,
  multi-stop gradients, a shape clip, the steady hand's pointer (`Hand` in `breakers.rs`), an offset dash.
- [ ] 7.2 `min_round_s` measured from people's play (design 8: provisional now).
- [ ] 7.3 The text's weight: egui's face has one; load the mockups' Barlow Semi Condensed as a TTF.
