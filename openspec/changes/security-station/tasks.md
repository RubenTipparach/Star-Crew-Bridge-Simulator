# Tasks: a security station, boarders and crew orders

## 1. Write-up and mockup

- [x] 1.1 Proposal, design and spec delta, from the owner's note of 2026-10-08.
- [ ] 1.2 The security console in `docs/mockups/consoles.html`, glance first.
- [ ] 1.3 The security station in the command suite's port status board slot (`command_suite.json`), and the bridge's eight seats in the deck plan.

## 2. Simulation

- [ ] 2.1 Internal sensors: counts per compartment by side, "?" when damaged or unpowered.
- [ ] 2.2 Entry points and the alarm; forcing a locked door (20 s) or a pressure door (60 s).
- [ ] 2.3 Crew orders (GO, HOLD, CLEAR) for watch bodies and teams, with the walk graph's times.
- [ ] 2.4 Boarders: targets, sabotage at 2 %/s, fighting with the armory's weapons.
- [ ] 2.5 `netcode`: a session with 18 bodies (8 crew, 4 team members, 6 boarders) within its budget.
