# Tasks: repairs on deck

## 1. Write-up and mockup

- [x] 1.1 Proposal, design and spec delta from the owner's "how we would perform these mechanics in the games 3d
  environment" (2026-10-09).
- [x] 1.2 `docs/mockups/repairs-on-deck.html`: engineering in 3D, the pump and the valve board docked, the covers unscrewed in 3D, the real games over
  the live room, rounds and fumbles shown in the room; shots in `docs/screenshots/repairs-on-deck/`.

## 2. Data

- [ ] 2.1 The `repair` use point kind and service faces on the engineering props' Blender builds.
- [ ] 2.2 `data/ships/tern/repairs.json` with jobs, rounds, levels and a measured `min_round_s`; validated.

## 3. Engine

- [ ] 3.1 `sc-core`: the job's rounds, the intents of section 6 and their checks; tests that a round lands at once and
  a round under `min_round_s` is refused.
- [ ] 3.2 `sc-client`: docking, the camera cut-in, the panel over the darkened live view, look up, leaving.
- [ ] 3.3 The machine's state cues (lamp, sound, effects) by band, and the round and fumble events.
