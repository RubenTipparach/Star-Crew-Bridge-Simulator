# Proposal: extinguishers and first aid kits round the ship

## Why

The owner, 2026-10-09: "We should probably add fire extinguishers and med kits across the ship. Medkits can help
stabilize a player's vitals." With the health rule of the same day (`crew-on-deck` 7: below 20 HP a body is
incapacitated and its vitals fall until someone stabilizes it and carries it to the medbay), a medkit is what keeps a
friend alive on the way, and a fire caught in its first 90 s is one extinguisher's work (`damage-control` 3). Today
the only extinguishers are six in the damage control locker and the only medkit is the medic's bag: from the far end
of engineering, either is a long walk while the fire grows and a friend's vitals fall.

## What Changes

- **Safety points** (design 1-2): an extinguisher in a bracket and a first aid cabinet side by side on a wall, under a
  glowing sign, placed by one rule from the layout: inside every room's main door, and along the passages so no
  floor on any deck is more than 10 m's walk from one.
- **The first aid kit** (design 3): 3 kg, one hand, 3 doses, stabilizes only. The extinguisher is `crew-on-deck` 6's.
- **Data and checks** (design 4): `data/ships/tern/kit.json` written by `tools/safety_points.py`, validated with the
  layout (clear of doors, on a wall, within the room).
- **Props** (design 5): the extinguisher, its bracket and the cabinet modelled in Blender with the hard-surface kit,
  drawn by the mockups and the engine; an empty bracket shows its kit was taken.

## Capabilities

### New Capabilities

- `safety-points`: where extinguishers and first aid kits are kept aboard, the first aid kit, and taking and
  refilling them.

### Modified Capabilities

- None in `openspec/specs/`. `crew-on-deck` 15's proposed `kit.json` list of 24 extinguisher brackets is replaced by
  this change's rule.

## Impact

- `tools/safety_points.py`, `data/ships/tern/kit.json`, `tools/layout_check.py` (validation).
- `tools/blender/build_safety_props.py` and its glb, `docs/mockups/lib/shipkit.js` or `propkit.js` (drawing them),
  `docs/mockups/deck-plan.html` and `fire.html`; the engine's deck build (`deckc`) and later the item entities.
