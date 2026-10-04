# Star Crew: the game in one page

This is the brief every OpenSpec change in this repository is written against. It states what
the owner asked for (CLAUDE.md section 1), the shape of a session, the crew and the reference
ship. Where it gives a number, the change named beside it is the source and this page only
repeats it; when they disagree, the change wins and this page is fixed.

## What it is

A starship bridge simulator for a crew of friends playing online. Four players are the core
crew; up to eight can play, and one can play alone. They walk a full 3D ship in first person,
sit at stations, and keep a simulated ship alive: power, air, heat, damage, weapons and craft
are all real systems with real flows, not health bars. Some players launch as fighter pilots.

It runs on a custom engine whose floor is a Raspberry Pi 5 with 1 GB of RAM (a 4 GB Pi 5 can be
the main server), so it is low poly,
vertex lit, brush built and portal culled, in the tradition of Quake and Descent.

## Pillars

1. **The ship is the character.** Every system is simulated from cause to effect: a hit in the
   switchboard cuts a bus, the bus feeds the scrubbers, the CO2 climbs in the quarters. A
   player can follow any number on a console back to the thing that caused it.
2. **Every station matters, and none is required.** Each station has a job no other station
   can do well. An empty station is held by automation at lower competence, so four players is
   the core but one to eight all work.
3. **The bridge is a place.** Crew stand up, walk to engineering, carry an extinguisher, climb
   into a turret and drop out of the hangar in a fighter. The interior is a space you inhabit,
   decoupled from the ship's motion through space.
4. **Cooperation by design.** Systems are built so that one station's choice is another's
   problem to solve: helm turns the bow so tactical's tubes bear; engineering's power budget
   decides whether shields or turrets win; flight ops cannot launch until the bay is empty of
   air, and the bay cannot empty while someone stands in it unsuited.
5. **Runs on a 1 GB Pi 5.** Every design states its cost against the Pi 5 budget.

## A session

1. **Muster.** Crew spawn in the mess (the lobby) and pick stations at the console there or by
   walking to a seat. Unclaimed stations are automated.
2. **Briefing.** The captain (or whoever holds command) reads the mission on the viewscreen.
3. **Mission.** Typically 20-45 minutes: patrol, escort, rescue, strike, survive. The ship
   travels between points of interest in one star system.
4. **Debrief.** Results, damage carried into the next mission, repairs and resupply.

A campaign carries the ship between missions: hull damage, consumables (missiles, fuel,
spare parts, oxygen reserve) and crew injuries persist.

## The crew

| Station | Seat | Core four | What they do |
| --- | --- | --- | --- |
| Helm | Bridge, front left | yes | Flies the ship; navigation; docking; evasive manoeuvres. |
| Tactical | Bridge, front right | yes | Targets; assigns turrets to gunners or automation; loads and fires missile tubes; shield facing. |
| Engineering | Bridge, port side; also the engineering bay console | yes | Reactor, power allocation, breakers, coolant, life support setpoints, damage control teams. |
| Science | Bridge, starboard side | yes | Sensors and scans; shield frequency and face balance; the viewscreen feed; comms when unmanned. |
| Captain | Bridge, centre dais | no | Overview and orders; ship-wide alerts. Merged into any console when absent. |
| Comms | Bridge, port aft | no | Hails, clearances, intercepts. Merged into Science when absent. |
| Flight ops | Bridge, starboard aft; bay control in the hangar | no | Bay pressure, launch and recovery, fighter tasking. Merged into Tactical when absent. |
| Gunner (x4) | Turret pods | no | Mans a turret by hand: better accuracy than automation. |
| Pilot (x2) | Swift fighters | no | Flies a fighter outside the ship. |

The detail is in `openspec/changes/bridge-stations` and `openspec/changes/crew-on-deck`.

## The reference ship: SCS Tern

A Heron-class frigate, 84 m long, 24 m in the beam, on three decks. The layout is
`data/ships/tern/layout.json` (the one source), checked by `tools/layout_check.py`, presented
by `openspec/changes/reference-ship-tern` and the mockups.

| Deck | Floor | Compartments |
| --- | --- | --- |
| A, Command | +3.5 m | Bridge, ready room, computer core, command passage, dorsal turret access, aft passage |
| B, Main | 0.0 m | Torpedo room, medbay, damage control, quarters, mess, port and starboard turret access, main corridor |
| C, Lower | -3.5 m | Magazine, life support, cargo, airlock, shield generator, forward switchboard, lower corridor |
| B and C | | Hangar (double height, with galleries), port and starboard launch bays |
| A, B and C | | Engineering (the reactor through three deck heights) |

30 compartments (4 of them turret pods), 40 portals, about 8,860 cubic metres of air. Four
twin pulse cannon turrets (dorsal, ventral, port, starboard), two forward missile tubes fed
from a magazine, two Swift fighters that drop through ventral doors, and a Petrel shuttle on
a lift pad in the hangar.

## Names used across the changes

| Thing | Name | Source change |
| --- | --- | --- |
| Power source | Fusion reactor | `power-grid` |
| Distribution | Port main bus, starboard main bus, emergency bus; main switchboard (engineering), forward switchboard (deck C) | `power-grid` |
| Storage | Battery bank (forward switchboard) | `power-grid` |
| Air | Oxygen generator, CO2 scrubbers, air handler, thermal control, bay pumps | `life-support` |
| Shields | Six faces: bow, stern, port, starboard, dorsal, ventral | `weapons-and-shields` |
| Turrets | Twin pulse cannon, manned or automated | `weapons-and-shields` |
| Missiles | Gannet anti-ship missile, two tubes, magazine and hoist | `weapons-and-shields` |
| Craft | Swift fighter (2), Petrel shuttle (1) | `shuttle-bay-and-fighters` |
| Frames | System frame, ship frame, interior frame | `ship-frames` |

## The changes

| Change | What it designs |
| --- | --- |
| `engine-stack` | The custom engine: language, platform layer, renderer floor, crate layout, the Pi 5 budget table. |
| `ship-frames` | Decoupling the interior from the exterior: frames, rendering composition, hand-off of craft. |
| `deck-pipeline` | Brush-style decks compiled to compartments, portals, collision and baked vertex light. |
| `light-baking` | The static light baker: shadows, emissive surfaces, bounce, vertex lighting or lightmaps, light probes, the three lighting states. |
| `reference-ship-tern` | The Tern's floor plan, compartment by compartment. |
| `bridge-stations` | The station roster, automation, console UI and the bridge as a room. |
| `crew-on-deck` | Walking, ladders, seats, carrying, injury and revival in the interior. |
| `power-grid` | The energy simulation: reactor, buses, breakers, batteries, allocation, heat and coolant. |
| `life-support` | The atmosphere simulation: gases, pressure, temperature, flow, breaches and their effect on crew. |
| `damage-control` | Hits to compartments and systems, fire, repair, damage control teams, the engineering bay as a place. |
| `weapons-and-shields` | Turrets manned and automated, missiles, magazine and tubes, point defence, six shield faces. |
| `shuttle-bay-and-fighters` | Hangar and launch bay operations, launch and recovery, fighter flight. |
| `flight-and-navigation` | The ship's flight model, helm controls, navigation in a star system. |
| `netcode-and-sessions` | The authoritative server, the client, sessions, joining, and the network budget. |

## Starting from star-crew-64

`docs/analysis/star-crew-64.md` lists what the N64 prototype proved and what it got wrong. In
short: the four stations reached on foot, NPC bodies filling empty seats, body swap, six shield
faces, the zero-sum power split, the forward firing arc and fire that needs crew to put it out
all carry over as design. The single 1,600-line `main.c`, frame-counted timings, a renderer
mixed into the simulation and a picture-in-picture as the only view of space do not.
