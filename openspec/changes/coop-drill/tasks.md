# Tasks: the first co-op drill

The owner asked for implementation on 2026-10-10 (proposal). Progress and findings: `log.md`.

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta.

## 2. Core

- [ ] 2.1 `data/ships/tern/flight.json`, `shields.json`, `combat.json`, `data/weapons.json`, `data/enemies.json`, `data/stations.json`, `data/missions/drill-hound.json`, parsed and validated by `sc-core`.
- [ ] 2.2 `sc-core::combat`: flight, shields and hull, turrets with the shared hit chance, bolts, Gannet missiles, the Hound, the mission's phases; a replay hash; tests (the hit chance predicts the hit rate; a drill with nobody aboard stays in Muster; bots win).
- [ ] 2.3 `sc-core::automation`: helm and tactical at the `automation` and `bot` profiles, through the same commands.

## 3. Network

- [ ] 3.1 `sc-net` messages: codec, validation, the unreliable header; round-trip and rejection tests.
- [ ] 3.2 `sc-net` transport: `str0m` on a network thread, server and client; LAN signalling (`POST /rtc`) and `GET /status`.
- [ ] 3.3 `sc-server --drill drill-hound`: seats, automation, snapshots at 20 Hz, the drill's loop; the loss and delay test option.
- [ ] 3.4 An in-process test: a server and two bot clients over loopback finish a drill.

## 4. Client

- [ ] 4.1 `--connect`, `--station`, `--bot`; the lobby's Join a crew takes an address.
- [ ] 4.2 The briefing and debrief screens.
- [ ] 4.3 The Helm and Tactical consoles over the 3D bow view; the Hound's placeholder model; bolts and missiles; interpolation.
- [ ] 4.4 Captures of each screen in `docs/screenshots/engine/coop-drill/`.

## 5. Two machines

- [ ] 5.1 The Pi 5 8 GB hosts the server and the Helm bot; the Mac runs the Tactical bot; drills complete; numbers in `docs/benchmarks/2026-10-10-coop-drill-lan/`.
- [ ] 5.2 The same with `--loss 0.1 --delay-ms 50`.
- [ ] 5.3 A demo video, each shot named.
