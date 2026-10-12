# Proposal: hits that damage parts of the ship

## Why

The owner, 2026-10-10: "the enemy ship should be damaging your ship, this battle show go on much longer with peope
fixing the ship along the way to bring damaged systems back online", "both missils and lasers can cause fire", then
"whats important is I want the enemy to actually damage parts of the ship", "hit detection should be precise enough
to check were the ship is hit, explosions spun off", and "if a system is already damaged when hit, the electrical
conduits will cascade damage to adjacent components and so on so forth".

Today a hit is a point inside a 12 m sphere, and whatever the shield does not hold comes off one hull number. Nothing
aboard breaks. `damage-control` designed the real model in 2026-10-04 (armour by hull section, energy marching into
the ship, systems, power nodes and conduits taking points, fires, breaches, repair teams), with its data in
`data/ships/tern/damage.json` and a working version in the systems mockup (`docs/mockups/lib/shipsystems.js`,
`resolveHit`); no engine code reads it. This change brings that model into the drill's simulation, for both ships,
and adds the owner's cascade.

## What Changes

- **Where a shot lands** (design 1): a bolt, a beam or a blast meets the shield's ellipsoid, then the hull's lofted
  octagons (`layout.json` `hull.sections`), at a point; the point picks the armour section (9 spans x 6 faces).
- **What it does inside** (design 2): `damage-control`'s march, ported: the energy past the armour deposits along the
  shot's line into the compartments it crosses; every system (a power load), node and conduit within reach takes
  points; integrity sets capability (`damage.json` `systems`); fires start by its ignition rule; the first room is
  breached.
- **The cascade** (design 3, the owner's): a hit on a system already below nominal surges through its node and
  conduits into the components wired to it, each taking a share; any of those already damaged passes it on, and so on.
- **Damage that matters** (design 4): broken systems weaken what the drill does (the drive slows, the shield
  generator stops regenerating, a tube or a laser bank goes dark, the sensors slow the lock, the reactor slows every
  charge).
- **Repairs and fires** (design 5): two damage-control teams (NPC crew) go to fires and broken systems, by priority,
  and bring them back; the captain can set the priority. On foot later (`crew-on-deck`); timed by distance for now.
- **Explosions** (design 6): every hull hit, burst system and cascade sends its point to the clients, which throw off
  sparks and debris there.
- **The Hound gets a layout** (design 7): hull, rooms, systems, nodes and conduits of its own, so the crew can break
  its drive, its banks and its hangar.

## Impact

- `sc-core::combat`: a new `damage` module (the hit, the march, the cascade, capability, fires, teams), read from
  `layout.json`, `power.json` and `damage.json` (status moves from PROPOSED to used by the drill); the Hound's data.
- `sc-net`: systems, fires and teams in the snapshot; hit and burst events. `sc-client`: debris and sparks; the
  captain's SHIP plan coloured by the real state; damage on every console's status strip.
- `damage-control` and `fire-spread` stay the full design; this is the drill's first part of them.
- Budget: the owner, "dont worry about overbudget"; the march costs at most 120 steps a hit.
