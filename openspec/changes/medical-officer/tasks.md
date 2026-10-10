# Tasks: a playable medical officer

## 1. Write-up and mockup

- [x] 1.1 Proposal, design and spec delta, from the owner's "a player can play as the medical officer to heal players" (2026-10-08).
- [ ] 1.2 The medical console in `docs/mockups/consoles.html`, glance first, beside the other five.
- [ ] 1.3 A `medical` station in the layout's medbay, its seat beside the beds; the medkit and its cabinet in `crew_rooms.json`.

## 2. Simulation (with `crew-on-deck`)

- [ ] 2.1 `crew::heal_rate` with the tender, and tests: a tended bed heals 25 to 100 HP in 12.5 s; untended 37.5 s.
- [ ] 2.2 The medkit: stabilize 2.0 s, field healing 4.0 HP/s to 75 HP for wounded bodies; doses and refill (owner's health rule, 2026-10-09).
- [ ] 2.3 MEDIC calls on the crew panel, the console and the HUD.
