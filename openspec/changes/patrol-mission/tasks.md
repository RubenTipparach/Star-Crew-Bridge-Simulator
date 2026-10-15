# Tasks: patrol-mission

The owner asked for it on 2026-10-14 ("have the ship undock from star base, go patrol a star system and encounter
hostiles", "the view through the window should reflect whats going on around the ship").

## 1. Write-up and mockup (phase 1)

- [x] 1.1 Proposal, design and spec deltas (`patrol-mission`, `exterior-view`).
- [x] 1.2 `docs/mockups/patrol.html`: the Tern docked at the placeholder starbase, undocking, in the jump and in the
  encounter, from outside and through the bridge's port window, with its Pi 5 cost; shot into
  `docs/screenshots/mockups/patrol-*.png` and looked at.

## 2. The windows in the engine, the client's drawing only (phase 1)

- [x] 2.1 The deck program's `relight` (design 11.3): the hull's bake divided out and the frame's sun multiplied in,
  per vertex; `DeckParams::relight`; the shader modules regenerated (`tools/sokol_shaders.py`).
- [x] 2.2 `sc-client/src/outside.rs`: the sun in the ship's frame, the hull's relight from `data/ships/exteriors.json`,
  and one ambient cube from the sun for everything outside; the hull drawn with it in the walk and the drill.
- [x] 2.3 `ships3d::station`: the relay and starbase placeholders and their beacons, within their triangle budgets.
- [x] 2.4 The relay drawn at the mission's `waypoint_m`, in the windows and the feed; its beacons blinking from
  `data/space/exterior.json` `beacons`, never under `min_radius_deg` across. (Not caught on screen yet: see design 12.)
- [x] 2.5 Captures from the bridge's and the ready room's windows in `docs/screenshots/engine/windows/` (deck B's
  windows were seen through the medbay's door in a run; a capture standing at one is still to take).

## 3. The system and the mission's data (phase 2)

- [x] 3.1 `data/space/systems/shoals.json` and its loader and checks in `sc-core::system` (positions finite, radii
  positive, arrival points at least 5 km from every body, the station outside its planet's 3-radius lock).
- [x] 3.2 The berth's attitude checked against the dock's view; in a patrol the sun and planet come from the system
  file (the walk's dock still reads `exterior.json`).
- [x] 3.3 `data/missions/patrol-shoals.json` and the mission schema's patrol fields.

## 4. The mission's engine (phase 2, after `crew-chat`'s protocol work is committed)

- [x] 4.1 Mockups first (`docs/mockups/patrol-consoles.html`, awaiting approval; not built): engineering's SHORE and
  DRIVE; helm's UNDOCK and JUMP; science's SWEEP; the captain's UNDOCK and JUMP orders (HOME is the last JUMP).
- [x] 4.2 `sc-core::patrol`: the phases, shore power and the drive's start, the clamps, the berth's departure limits,
  the jump between waypoints (FN 7's spool, alignment, mass locks, arrival scatter, cooldown), the sweep, the
  encounter placed from the arrival, the return; tests for each phase's end.
- [x] 4.3 Combat far from the origin: a floating origin moved to each arrival point, so combat's positions stay small (design 7).
- [x] 4.4 `sc-net`: the snapshot's `patrol` block and the new commands; `sc-server`: the mission's phases.
- [x] 4.5 The bots and automation for every phase (design 9), their lines through `crew-chat`.
- [x] 4.6 The objectives `undock`, `sweep`, `return` on the OBJECTIVES tab.

## 5. The windows for the patrol (phase 2)

- [x] 5.1 The sky from the system file: the star's direction, size and light; the near planet's look (the banded
  look and three far-body slots still to do).
- [x] 5.2 The starbase and its cradle in the system frame; the cradle sliding past as the Tern backs out. (Replaced
  2026-10-15 by 5.5.)
- [ ] 5.3 Motes, the jump's streaks and flash, the belt's far rocks.
- [ ] 5.4 `starbase.glb` replacing the placeholder when the `starbase` change lands. (Superseded by 5.5: Tamsin Station is
  spacedock's version E, generated.)
- [x] 5.5 The stations' models (design 12b): Tamsin Station generated from the spacedock sizes, lit per vertex (spacedock
  5b's fallback), its bay's floods baked at load, two sister ships at their berths; the other four stations as flagged
  placeholders; the outside's depth slice widened; the girder cradle gone from the drill.

## 6. Verification

- [ ] 6.1 A playtest recording of the whole patrol on foot on the Pi 5: walking to the station, undocking, both jumps,
  the fight, home (CLAUDE.md 12).
- [ ] 6.2 The Pi measures the built change (design 13) in a release build.
- [ ] 6.3 `scripts/check.sh` passes.
