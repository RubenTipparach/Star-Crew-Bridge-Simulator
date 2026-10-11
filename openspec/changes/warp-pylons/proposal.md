# Proposal: warp pylons that break in combat and are mended outside

## Why

The owner, 2026-10-11, on the new exterior model: "or go outside to fix pylons (which needs fixing if they are damaged
in combat)".

The Tern's exterior model (`ship-exteriors`) carries two warp pylons, swept out level from the hull's flanks, each
holding a 56 m nacelle. `repair-minigames` 5 proposed pylons, an EVA to reach them and a "no warp jump until repaired"
rule, and left them to "its own change (`warp-pylons`)" once the hull had them; it has them now. Combat 2's repair
jobs already name `warp_pylon_port` and `warp_pylon_starboard`, played outside with the pylons game. But no shot can
reach a pylon (`ship-damage` only knows the lofted hull), nothing walks outside, and the model has no outer airlock
door.

This change makes the pylons parts of the ship that combat damages, and makes mending them a walk outside: out
through the airlock, along the hull on magnetic boots, tethered, to the damaged coil, and the pylons game there.

## What Changes

- **The pylons take hits** (design 2): each pylon and its nacelle is a hit volume outside the hull, taken from the
  model; a shot's path meets it before the hull, and its integrity falls as `ship-damage` 2 rules for any system.
- **Warp needs both pylons** (design 3): a jump is refused while either pylon is below 25 integrity, and spools slower
  between 25 and 75; Helm and the Captain see which pylon is down.
- **The airlock opens onto the hull** (design 4): the exterior model gains the airlock's outer door where the layout
  has it (deck C, starboard, z 12 m); its cycle is `life-support` 14's.
- **Walking the hull** (design 5): a suited body walks the hull's outer surface on magnetic boots at 1.0 m/s, its "up"
  the surface under its feet, over the edges onto the pylon; a 30 m tether from the airlock, clipped on again at
  handholds along the way to reach the port pylon.
- **The repair** (design 6): docked at the damaged coil segment, the pylons game from its bolts on (the walk was its
  first move); the coil glows again when it is done.
- **Not in combat** (design 7): the outer door will not open while a hostile is within weapons range.

## Capabilities

### New Capabilities

- `warp-pylons`: the pylons as damageable parts, their effect on warp, the walk outside and their repair.

### Modified Capabilities

(none in `openspec/specs/`; it builds on `tern-exterior`, Combat 2's `ship-damage` and repair jobs, `crew-on-deck` 10
and `life-support` 14, all changes)

## Impact

- `tools/blender/build_ship_exteriors.py`, `assets/models/exteriors/`: the airlock's outer door; the pylons' coil
  segments as their own index ranges, so a damaged one can be drawn dark.
- `data/ships/tern/damage.json` (Combat 2): the pylon volumes and parts; `sc-core::combat::damage`: the volume test.
- `sc-core::walk`: the boots mode; the deck file: the hull's walk triangles.
- `sc-repairs` `pylons`: started docked, at the bolts.
- Pi 5 budget (design 9): about 4,440 collision triangles and one tether line; the suited figure a crew figure's.
