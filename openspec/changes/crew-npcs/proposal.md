# Proposal: NPC crew with jobs

## Why

The owner, 2026-10-08: "maybe we can add NPCs that have jobs on the ship too."

`bridge-stations` 6.1 already has NPC bodies: spare bodies that sit at automated core stations and walk
a post list to an empty seat. They never do anything else, so a ship with four players is a ship of
eight empty rooms. The Tern sleeps twelve (`ship-props` 4d): the crew a player does not bring should
be aboard, working, and visibly so: an engineer at the switchboard, a cook in the galley, a deckhand in
the magazine, someone asleep behind a bunk curtain.

## What Changes

- **A ship's company** (design section 1): the Tern carries a complement of 12. Each player takes a
  berth; the rest are NPC crew, each with a name, a department and a job: engineer, medic, cook,
  deckhand, security, and the watch officers who sit at automated stations.
- **Watches and a day** (section 2): a ship's day of three watches; on watch an NPC works, off watch it
  eats, showers, sleeps. They use the ship's fixtures (`ship-interactables`): the same Use, the same rule.
- **Jobs that matter** (section 3): NPC work runs through the same code a player's does (CLAUDE.md 6.1):
  an engineer repairs damage at `damage-control`'s rates times a competence below a player's, a deckhand
  carries missiles to the tubes when no player does, a medic treats the downed, security patrols and
  answers boarders. Automation keeps running the consoles (`bridge-stations` 4); an NPC is its body.
- **Orders** (section 4): the captain, or anyone at the bridge, can send a department to a place
  ("Engineering to the drive section"), and red alert sends everyone to their action stations.
- **The crew panel** shows where everyone is on the plan (`ship-plan-view`).

## Impact

- `bridge-stations` 6.1 (NPC bodies grow jobs), `crew-on-deck` (bodies, the movement function they share),
  `damage-control`, `weapons-and-shields` (loading), `medical-officer`, `security-station`,
  `ship-interactables`, `ship-plan-view`, `netcode-and-sessions` (NPC avatars in the snapshot, as players'),
  the engine (`sc-core::crew`), the server's CPU budget.
