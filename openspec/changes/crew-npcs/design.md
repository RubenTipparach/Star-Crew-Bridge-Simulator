# Design: NPC crew with jobs

## Context

The owner, 2026-10-08: "maybe we can add NPCs that have jobs on the ship too."

| Decided elsewhere | Where |
| --- | --- |
| NPC bodies: spare bodies at automated core stations, a post list, body swap, never holding a seat against a player | `bridge-stations` 6.1 |
| Automation runs an unmanned station at a stated, lower competence | CLAUDE.md 7, `bridge-stations` 4 |
| One movement function for every body, server-run; bodies collide with the deck | `crew-on-deck` 3, 3a (`sc-core::walk`) |
| Twelve berths on the Tern | `ship-props` 4d, `crew_rooms.json` |
| Repair, fire, breach | `damage-control` |
| Randomness is seeded from the session seed, the entity's stable id and the purpose; order is stable | CLAUDE.md 6.4 |

## 1. The ship's company

The Tern's complement is 12 (its berths). At mission start each player's body takes a berth; the other
berths are filled by NPC crew, generated from the session seed so a replay has the same crew.

| Department | NPCs (with four players) | Job on watch | Station when red alert |
| --- | ---: | --- | --- |
| Command (watch officers) | 2 | Sit at the automated core stations no player holds (`bridge-stations` 6.1) | The bridge |
| Engineering | 2 | Rounds: switchboard, reactor gallery, drive; repair what is damaged | Engineering |
| Deckhands | 2 | Magazine and hangar: stow, carry, load the tubes when no player does | The magazine |
| Medical | 1 | The medbay; treats any downed body | The medbay |
| Security | 1 | Patrols the spine corridors; the armory | The armory, then boarders |
| Galley | 0 or 1 | The mess: meals at watch change | Damage control (a spare pair of hands) |

With more players, NPCs leave in reverse order of this table (galley first, command last). Each has a
name, a department, a competence (0.5-0.8 of a player's speed at its job, data) and a home berth.

## 2. Watches and a day

A ship's day is 3 watches of 8 game hours; game time runs at 12x real time in a session (data), so a
watch is 40 real minutes. On watch an NPC works its job; off watch it spends the time in an order
drawn from its seeded schedule: eat (the mess, a tray from the dispenser), shower and the head
(`ship-interactables`), free time (the mess table, the radio), sleep (its bunk, curtain closed, room
light on Night). Needs are simple counters (fed, rested, clean) that pick the next errand; they change
nothing else in the game.

## 3. Jobs through the players' code

An NPC's job is a list of **tasks**, each a place and an action that a player could do there, issued
through the same intent a player's input produces (CLAUDE.md 6.1):

| Task | The same code as | NPC difference |
| --- | --- | --- |
| Walk to a place | `sc-core::walk` (the body controller) and a path (section 5) | None |
| Use a fixture, a door, a ladder, the lift | `ship-interactables`, `crew-on-deck` 4-5 | None |
| Repair, patch, extinguish | `damage-control`'s rules | Rate times competence |
| Carry a missile, load a tube | `weapons-and-shields`' loading | Rate times competence; only when no player is loading |
| Treat a downed body | `medical-officer` | Rate times competence |
| Sit at a station | `bridge-stations` 6.1 | Never operates it: automation does |

**Who does what when something breaks.** Damage control's queue (`damage-control`) assigns the nearest
idle engineering NPC to the worst damage it can reach, after any player who has taken it. A player
can always take over a job an NPC is doing: the NPC steps back and takes the next.

## 4. Orders

From any bridge console's crew tab (`bridge-stations`, a page of the captain's): pick a department and a
compartment on the plan; its NPCs go there and do their job there (an engineer repairs what is there,
security holds the door). Red alert sends every NPC to its red-alert station (section 1) at a run;
returning to normal sends them back to their schedule. An order is an intent like any other, so a
player at the captain's console and a script in a test give the same order the same way.

## 5. Paths

The walk world already holds the deck's walkable triangles (`deck-pipeline` 13a). A **nav graph** is
built offline by `deckc` from the compartment graph (rooms and portals, CLAUDE.md 7: "one compartment
graph") with a few waypoints a room: a path is an A* over portals, then straight lines inside a room,
the body controller doing the walking. Ladders, the stair towers and the lift are edges with their own
costs (`reference-ship-tern` walk times). A closed pressure door or a breach cuts its edge.

## 6. Network and the Pi 5 budget

- NPC avatars travel in the snapshot like players' (`netcode-and-sessions`: 10 bytes each at 20 Hz);
  eight NPCs add 1.6 KB/s a client.
- Server CPU: the body step measured in the engine walk (`deck-pipeline` 13a: 0.26 ms mean on a cloud
  CPU for one body, two substeps) times eight NPCs is about 2 ms a 30 Hz tick there; a path is computed
  on a task's start, not every tick. To measure on the 4 GB Pi server (CLAUDE.md 2).
- Client: each NPC is a crew avatar (`crew-on-deck` 17), skinned, drawn only in the camera's room and
  its neighbours.

## Risks / Trade-offs

- **NPCs in the way.** A body is solid; an NPC blocking a doorway in a fight is the worst kind of bug.
  NPCs yield: a moving NPC that meets a player steps aside within 0.5 s, and an NPC never stands in a
  doorway or on a ladder longer than it takes to pass.
- **Competence that makes players idle.** NPC work is slower than a player's and never touches a console;
  the players stay the ship's best crew.
