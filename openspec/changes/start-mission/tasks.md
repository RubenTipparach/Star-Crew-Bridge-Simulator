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

## 2a. No automatic start (owner, 2026-10-17: "I need a button to actually start the mission. Not just have the mission autostart")

- [x] 2a.1 Write-up: proposal, design 1, the spec delta.
- [ ] 2a.2 `sc-core`: Muster never ends by itself; tests (the round waits with everyone seated and ready; sitting on
  foot marks ready and does not start it; START MISSION starts it).
- [ ] 2a.3 Everything that relied on the automatic start presses START MISSION: headless tests, the server and net
  tests, the bots, the playtest and capture tools, `tools/web/smoke.mjs` (single player, `--touch`, `--fit`).
- [ ] 2a.4 The briefing's single-player READY no longer reads "Crew sitting down"; "Launch in" only in Countdown.
- [ ] 2a.5 Evidence: `docs/videos/2026-10-17-manual-start.mp4` (walk, sit, the round waiting, START MISSION, the
  countdown, Engage) and `docs/screenshots/engine/manual-start/`.

## 3. Gate

- [ ] 3.1 The owner looks at the mockup and engine captures (next survey).
