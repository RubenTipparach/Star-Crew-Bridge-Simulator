# Tasks: reactor-cooling

## 1. Write-up and mockups

- [x] 1.1 Proposal, design and spec delta from the owner's reactor, coolant and engineering bench request (2026-10-09).
- [x] 1.2 `docs/mockups/repairs/pipes.js` (design 4) and `coolant.js` (design 5); the reactor game titled "Reactor: magnetic core"; shots.
- [x] 1.3 `docs/mockups/consoles.html`: the Cooling group on E1 and E4 as the balance panel (design 6); shots.

## 2. Rules

- [ ] 2.1 `power.json` coolant block (inventory, tanks, leak, makeup, cavitation, segments, radiator pumps) and the Cooling group; `power-grid` 10 updated.
- [ ] 2.2 The heat step with inventory, leaks, makeup and the chiller term; the spec's scenarios as tests.
- [ ] 2.3 Engineering automation's cooling (design 5) in `bridge-stations` 9.

## 3. The reactor system screen and the pipes' flow (owner, 2026-10-09)

- [x] 3.1 The coolant parts' damage in `shipsystems.js` (segments, tanks, exchanger, leaks), read-only views for the screen.
- [x] 3.2 `docs/mockups/reactor-system.html` (design 6a): the whole system, diagnosis by looking, click a part to its repair game, the repair restoring it.
- [ ] 3.3 The coolant pipes game's rebuild with coolant flowing through the tiles (design 4).
