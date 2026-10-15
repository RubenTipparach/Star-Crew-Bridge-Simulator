# Tasks: spacedock

The owner asked for it on 2026-10-14 ("can you mockup some versions of spacedock? I want a big structure where the ship
can actually enter rather than something flimsy. I want there to be multiple ships docket there too").

## 1. Write-up and mockup

- [x] 1.1 Proposal, design and the `spacedock` spec delta; `openspec validate --all`.
- [x] 1.2 `data/space/spacedock.json`: the five versions' sizes, doors and berths, the sky at the station, the traffic
  limits, the fleet.
- [x] 1.3 `docs/mockups/spacedock.html`: the versions, each from outside with the Tern on approach, the doors open
  and the Tern entering, inside the bay with the Tern at its gangway, and the view out of a bridge window while
  berthed; its Pi 5 cost; shot into `docs/screenshots/mockups/spacedock-*.png` and looked at.
- [x] 1.4 Version E, the O'Neill pair, rebuilt hollow from the cited sources: its data, its Material Maker graphs
  (`oneill_macro`, `oneill_cap`, `oneill_detail`, `oneill_mirror`, `oneill_land`, `oneill_land_detail`,
  `oneill_clouds`) rendered on the Pi and packaged by `tools/materials/oneill_atlases.py`, its contact sheet, the
  per-pixel shaders and their light, the axial port and hall, its nine shots looked at.
- [x] 1.5 The owner chose (S1, 2026-10-14): "I want to keep all of them as various stations you can visit across the
  system, starting at oniel pairs". Design 11a places them; `spacedock.json` `stations` lists them.
- [x] 1.6 E's port as a pressure boundary: the hub sealed into the dome, one round throat, iris and bay, the seal
  check (`SPACEDOCK_SEAL_CHECK`, 0 of 1,600 rays leak); E's own bay and bridge-window views.

## 2. The stations in the engine

- [ ] 2.1 The system file names its stations: `data/space/systems/shoals.json` `stations` gains the five (design 11a),
  each with its version, body or point, orbit, berths and departure legs; `sc-core::system` takes the new field
  (a schema change: the file is read with `deny_unknown_fields`) and checks every station's body or point exists, its
  orbit is outside its body's 3-radius mass lock, and its berths sit inside its bay. `spacedock.json` `stations` then
  goes, the system file being the one source.
- [ ] 2.2 `patrol-mission` follows: its starbase becomes E, Tamsin Station; the patrol starts with the Tern berthed in
  E's round bay (a berth pose on the bay's wall, keel to the wall), and leaves by a departure leg up off the wall to
  the axis, along the axis out through the iris and the throat, then clear of the dome and the mirrors to the 2 km mass
  lock (design 5a, why the sunward end). Its design 3, 4.1-4.3 and spec change with it.
- [ ] 2.3 The station generators (committed generators, CLAUDE.md 9), one per version, E first: outside, inside and
  doors in three draws within design 9's budget, and a far model each; the data checks of design 10.
- [ ] 2.4 The bays' bakes: their floods and window tiers as fixtures in data, baked into the insides' vertex colours.
- [ ] 2.5 The docked ships placed from the berths at their middle level of detail; the berth probes lighting them.
- [ ] 2.6 The doors and iris (traffic control's clearance, 20 s to open and close) and the bay's speed limit on the
  server; FN 10's dock autopilot with a berth as its port.
- [ ] 2.7 Measure E's per-pixel shaders on the Pi (S7) and write the result into `engine-stack`'s budget table before
  the engine takes them; otherwise their fallback.
- [ ] 2.8 E's seal in the engine: the hub, seal collars and bay as closed geometry, and the seal check as a test on the
  generated mesh.
- [ ] 2.9 Captures from the bridge's windows and the viewscreen while entering and while berthed at Tamsin Station,
  walking the ship (CLAUDE.md 12), in `docs/screenshots/engine/`.
