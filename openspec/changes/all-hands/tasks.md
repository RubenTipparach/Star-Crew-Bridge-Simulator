# Tasks: all-hands

The owner asked for it on 2026-10-12 (the proposal's words).

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta.
- [x] 1.2 The panel-box games mocked up for approval (`repair-minigames` 2a, survey RM1-RM3).
- [ ] 1.3 Mockups: the title band's STAND (`consoles.html`), the All hands toggle (the Single Player page and the
  briefing), the on-foot job marker (`repairs-on-deck.html`); the owner's approval.

## 2. Build (design 9)

- [ ] 2.1 `missions.json` `damage_control`; the server's damage state with no teams in `"crew"`; the core test.
- [ ] 2.2 Fires counted in hands; the core tests (a team of two 20 s, one player 40 s, two players 20 s).
- [x] 2.3 Standing up and sitting down; the station to automation while away (`coop-drill` 9.2, 2026-10-12; the title band's STAND waits for its mockup: hold E or Backspace stands).
- [x] 2.4 Bodies on every deck, ladders included: the player walks the whole ship in the game (`coop-drill` 9.2); the walk grid agrees with the body (`every_bridge_station_is_reached_on_foot_from_the_corridor`).
- [ ] 2.5 The damage-control locker, the extinguisher, spraying; a server test that puts a fire out.
- [ ] 2.6 Docking at a repair point and the rounds checked by the server (`repairs-on-deck` 6); its server test.
- [ ] 2.7 The job marker, at parity with its mockup.
- [ ] 2.8 The panel-box games in `sc-repairs` with their `data/repairs/<id>.json`, once approved.

## 3. The take

- [ ] 3.1 A whole fight in All hands recorded on the Pi (design 8), its notes in `docs/videos/README.md`.
- [ ] 3.2 The owner signs it off.
