# Tasks: spacedock

The owner asked for it on 2026-10-14 ("can you mockup some versions of spacedock? I want a big structure where the ship
can actually enter rather than something flimsy. I want there to be multiple ships docket there too").

## 1. Write-up and mockup

- [x] 1.1 Proposal, design and the `spacedock` spec delta; `openspec validate --all`.
- [x] 1.2 `data/space/spacedock.json`: the four versions' sizes, doors and berths, the sky at the station, the traffic
  limits, the fleet.
- [x] 1.3 `docs/mockups/spacedock.html`: the four versions, each from outside with the Tern on approach, the doors open
  and the Tern entering, inside the bay with the Tern at its gangway, and the view out of a bridge window while
  berthed; its Pi 5 cost; shot into `docs/screenshots/mockups/spacedock-*.png` and looked at.
- [ ] 1.4 The owner chooses a version (S1), with the shots beside the question in the survey.

## 2. After the choice

- [ ] 2.1 Fold the choice into this change: the chosen version's data as `starcrew.spacedock/1`, the others dropped;
  `patrol-mission` design 3, 4.1-4.3 and its spec follow (design 11).
- [ ] 2.2 The station generator (a committed generator, CLAUDE.md 9): outside, inside and doors in three draws, within
  design 9's budget, and its far model; the data checks of design 10.
- [ ] 2.3 The bay's bake: its floods and window tiers as fixtures in data, baked into the inside's vertex colours.
- [ ] 2.4 The docked ships placed from the berths at their middle level of detail; the berth probes lighting them.
- [ ] 2.5 The doors (traffic control's clearance, 20 s to open and close) and the bay's speed limit on the server;
  FN 10's dock autopilot with a berth as its port.
- [ ] 2.6 Captures from the bridge's windows and the viewscreen while entering and while berthed, walking the ship
  (CLAUDE.md 12), in `docs/screenshots/engine/`.
