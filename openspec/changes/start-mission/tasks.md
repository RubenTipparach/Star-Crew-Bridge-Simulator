# Tasks: start-mission

The owner asked on 2026-10-16 (survey 2, Q3; the proposal).

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta.
- [x] 1.2 `consoles.html`: the chip, its key, the parity states `start-captain` and `solo-start`.

## 2. Build

- [x] 2.1 `sc-core`: `may_start`, `Drill::start_mission`, `Refusal::CaptainsCall`, with tests (design 7).
- [x] 2.2 `sc-net` and `sc-server`: `ClientMsg::StartMission`, protocol 18, the handler, with tests.
- [x] 2.3 `data/input/bindings.json` `start`, `bindings.rs`, tests.
- [x] 2.4 `sc-client`: the chip in the title band, its click and its key; parity passes.
- [x] 2.5 Single player: the player starts the round with the crew still walking; test and captures in
  `docs/screenshots/engine/start-mission/` (the captain's console in a two-person crew, then the countdown with
  both still reading; the parity pairs).

## 3. Gate

- [ ] 3.1 The owner looks at the mockup and engine captures (next survey).
