# Design: missions

## 1. The list

The server reads every `data/missions/*.json` at start (each validated as today) and offers them in Muster. The
room's briefing panel shows the mission chosen with a MISSION row; the captain's row is a button that opens the list
over the panel: each mission's title and its `summary` (one line), the chosen one marked. A pick sends
`ClientMsg::Mission(id)`; the server accepts it only from the captain's seat (in single player, the player), only in
Muster, and only while nobody is READY; it reloads the drill's data for that mission, keeps everyone's slot and
station, and says so to all (`ServerMsg::Mission(id)`). The briefing then reads the new mission.

## 2. The Grabon demo mission

| | |
| --- | --- |
| Title | Demo: The Grabon Raider |
| Situation | A Grabon raider is stripping the derelict relay at Kessan's Reach. Drive it off or destroy it, then jump home. |
| Enemy | The Grabon raider: faster than the Hound, two heavy turrets and four tubes, no fighters, a heavier hull, weaker shields; its own low-poly shape (a crescent) |
| Objectives | Destroy the raider; keep the hull above zero; jump out |

"Grabon" is the owner's name for the species; their look is a recommendation taken (ask only with screenshots): a
crescent hull in rust and bone, set apart from the Hound's wedge at a glance. The raider uses the Hound's damage layout
until it has its own (`data/ships/hound/layout.json`, named by its enemy class).

## 3. Warp out

- The captain's orders to Helm gain JUMP. Helm's THRUST panel gains a JUMP control under STOP, held 0.6 s (a guarded
  control, bridge-stations 8.5); it works with the drive at capability above a quarter, and not within 2 km of a
  hostile ship (the jump needs a clear field).
- Held, the drive spins up for `jump_spool_s` (10 s, data): a ring fills on JUMP and on every console's title band
  ("JUMP 7 s"); STOP or a hostile closing inside 2 km cancels it.
- At the end of the spin-up the Tern jumps: the viewscreens and windows streak white for a second, and the round ends:
  MISSION COMPLETE if the mission's destroy objective is done, ESCAPED otherwise.
- `Outcome` gains `Jumped`.

## 4. The end screen

Over every console and the bridge: the outcome (MISSION COMPLETE in green, ESCAPED in amber, THE TERN IS LOST in red),
the mission's title, the objectives with their marks, the fight's numbers as the debrief has them, the ship's state
(hull, systems below nominal), and two buttons: NEXT MISSION (back to the room) and RETURN TO MENU.

## 5. Cost

One message a pick, one byte a snapshot for the jump's state. The streak is a full-screen quad for a second.
