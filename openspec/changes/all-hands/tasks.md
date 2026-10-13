# Tasks: all-hands

The owner asked for it on 2026-10-12 (the proposal's words).

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta.
- [x] 1.2 The panel-box games mocked up for approval (`repair-minigames` 2a, survey RM1-RM3).
- [ ] 1.3 Mockups: the title band's STAND (`consoles.html`), the All hands toggle (the Single Player page and the
  briefing), the on-foot job marker (`repairs-on-deck.html`); the owner's approval.

## 2. Build (design 9)

- [x] 2.1 The mission's `damage_control` (`teams` by default, `crew`: no teams aboard); `--all-hands` for single player (2026-10-12).
- [x] 2.2 Fires counted in hands: `a_fire_is_put_out_by_hands_a_team_its_members_a_player_one` (one player 40 s, two 20 s, nobody: it spreads).
- [x] 2.3 Standing up and sitting down; the station to automation while away (`coop-drill` 9.2, 2026-10-12; the title band's STAND waits for its mockup: hold E or Backspace stands).
- [x] 2.4 Bodies on every deck, ladders included: the player walks the whole ship in the game (`coop-drill` 9.2); the walk grid agrees with the body (`every_bridge_station_is_reached_on_foot_from_the_corridor`).
- [x] 2.5 The damage-control locker (`fire.locker`, `dc_lockers`), the extinguisher (`fire.extinguisher_s`, 15 s), spraying in the burning room (protocol 11: `TakeExtinguisher`, `Spray`; each body's agent and spraying in the snapshot); flames on burning rooms' floors, the extinguisher in hand and its spray in first person; E at the locker, F (or the left button) to spray: `a_player_takes_extinguishers_at_the_locker_and_sprays_a_fire_out`. Facing the fire is not checked yet (in the room is enough), and fire-spread's cells wait.
- [ ] 2.6 Docking at a repair point and the rounds checked by the server (`repairs-on-deck` 6); its server test.
- [ ] 2.7 The job marker, at parity with its mockup.
- [ ] 2.8 The panel-box games in `sc-repairs` with their `data/repairs/<id>.json`, once approved.

## 3. The take

- [ ] 3.1 A whole fight in All hands recorded on the Pi (design 8), its notes in `docs/videos/README.md`.
- [ ] 3.2 The owner signs it off.
