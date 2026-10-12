# Combat 2: hand-off for Pi 1 (2026-10-10)

Pi 1's coordinating session lost its shell (auto mode's safety check blocked every command). This is where it stopped,
so the next session on Pi 1 can go on. The plan is `combat-2-work-plan.md`; the designs are the five changes it names.

The owner, 2026-10-10: "Approved all of it. Start the work..merge in changes from other sessions". That approves the
names and Esc-menu mockup (`crew-nameplates`, `game-menu`: tasks ticked in the working tree) and the Combat 2 console
mockups on `feat/combat-consoles` (0f04088).

## 1. Pi 1's working tree on `feat/combat-2`: not committed

Stream A's core weapons, written and compiling, 106 of 108 core tests passing (the two failures are the fight's
balance tests, waiting on step 2):

- `sc-core/src/combat/mod.rs`: stations LaserPort, LaserStbd, Gunner; commands LaserFire, Decoy, GunnerAim,
  GunnerTrigger; LaserBank state; lasers_tick, fire_laser, laser_wanted; throw_decoy, decoy_wanted; load_tube and
  fire_tube shared by the Tern and the Hound; enemy_weapons; point defence against missiles in turrets; the gunner's
  manual turret; hit_ship (shield by weapon) and after_shield (stand-in armour, one plate a face); missiles for both
  ships; events Beam, DecoyOut, Fooled; new stats; `tuning_report` and `debug_enemy_missiles` (ignored tests).
- `sc-core/src/combat/data.rs`: Gun.capital_factor; LaserData, DecoyData, DecoyWindow; LaserMount, LasersBlock,
  DecoysBlock, ArmourBlock (the stand-in); CombatBlock.pd_range_m; MissileData.hit_radius_m, shield_share;
  LaserProfile, GunnerProfile; Orders for the new stations; mission stations up to 8; cross-checks.
- Data: `data/weapons.json`, `data/ships/tern/combat.json`, `data/enemies.json`, `data/stations.json`,
  `data/missions/drill-hound.json`. Tuning so far: Tern hull 400, Hound hull 250, point defence 900 m, missile hit
  radius 1.5 m.
- `openspec/changes/game-menu/tasks.md`, `crew-nameplates/tasks.md`: the owner's approval.

## 2. Next, in order

1. **The missiles' heading** (the fix that was blocked): `Missile.heading`, starting as the launcher's forward; the
   seeker looks along it; while it sees, it turns toward the target at `MissileData.turn_deg_s` (data: 60); the motor
   pushes along it. Without it the Hound's missiles fly along its orbit and never come within 1.4 km (from
   `debug_enemy_missiles`). Then rerun `cargo test -p sc-core tuning_report -- --ignored --nocapture`, tune until the
   Hound damages the Tern and the fight runs 5-10 minutes, update the two balance tests, fmt, clippy, commit, push.
2. **Merge stream C**: `origin/feat/fighters` (9f0bf9c, Pi 2). Wire it in: launch at `launch.after_engage_s`, the
   fighters' shots become bolts with the drill's `scatter`, bolts hit fighters (full damage, not the capital factor),
   turrets take fighters after missiles (enemy-fighters 2), the gunner's sight can choose them.
3. **Merge stream B**: `origin/claude/starcrew-bridge-sim-docs-85s176` (2d46b5d, draft PR #17, the cloud session).
   Its seam: `ShipLayout::tern(texts)` and `ShipLayout::hound(text)` take file texts (`TERN_FILES`, `HOUND_FILE`,
   `DAMAGE_FILE`); `capability(l, s, cfg, system)` takes the rules. Wire it into `after_shield` (hull entry, armour
   section, march, cascade), delete the stand-in armour (`Ship.armour`, `ArmourBlock`, the `armour` blocks in the two
   combat files), and do the cloud's two tasks left to Pi 1: ship-damage 2.2a (capability into the fight, design 4's
   table) and 2.3a (the captain's REPAIR order). The cook-off waits for the magazine count.
4. **The cloud's flags**: run `python3 tools/mockups/inline.py` (six pages fail its check on `feat/combat-2`); make the
   Hound's shield ellipsoid in `data/enemies.json` fit its 30 m hull (it is a 9 m sphere); ship-damage design 7 names
   `data/ships/hound/layout.json`, not `enemies.json`.
5. **The objectives' kinds**: `MissionFile.objectives` becomes objects `{ "text", "kind": "destroy" | "survive" |
   "destroy_fighters", "ship"?, "count"? }` (objectives-tab design 2); tell the Mac when it lands.
6. **The protocol**: lasers, decoys, missile owner and target, fighters, systems, fires and teams in the snapshot;
   Beam, DecoyOut, Fooled, HullHit, Burst as events; the new stations and commands on the wire; bump `PROTOCOL`.
   Then `sc-bot` and the server's seats (laser_port to the layout's `comms`, laser_stbd to `flight_ops`, gunner to
   `gunner_dorsal`).
7. **The client**: beams, decoy flares, fighters, sparks and debris drawn; `seat.rs` fills the Mac's view structs.
8. **The recording**: the shot list on https://claude.ai/artifact/8GJHvMRVHzu8G2GZh3q4q7 ("The next recording").

## 3. Messages to send (Pi 1 could not)

- **Mac** (`[Mac] Star-crew-bridge-simulator clone`): the owner approved the console mockups; build the engine consoles
  (stream D step 3). Objectives become objects with a kind (above); the laser colour stays in `consoles.html` for now;
  63 s for a tube is right.
- **Pi 2** (`[PI-2] Star Crew b`): stream C is merged (once it is); the owner approved Combat 2. Ask the owner whether
  Pi 2's session may run its bot in the background, for the recording.
- **Cloud** (session_01XPxx9YMuxAXorVnQzDeXcp, through the Remote MCP `send_message`): B merged; its flags handled.

## 4. Pages for the owner

- First Mission Gate: https://claude.ai/artifact/8GJHvMRVHzu8G2GZh3q4q7
- Combat 2 Work Plan: https://claude.ai/artifact/4RzwE7d13hC9Fv1nsEsEcZ
- Combat 2 Designs: https://claude.ai/artifact/438vT8bGsUmT3qSCGGPCgy
