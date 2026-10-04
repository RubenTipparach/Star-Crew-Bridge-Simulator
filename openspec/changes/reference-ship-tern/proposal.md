# Proposal: the reference ship, SCS Tern, and its floor plan

## Why

The owner, 2026-10-04: "a full 3D starship bridge is vital to making this game work", with "a
full floor plan", "an engineering bay", "a fully working shuttle bay" and "manable or automated
turrets and missile launchers". Every system change (power, life support, damage control,
weapons, the shuttle bay, crew on deck) needs one concrete ship to put numbers on: where the
reactor is, how far the engineer walks, which compartment vents when a fighter launches.

The Tern's layout already exists as data (`data/ships/tern/layout.json`, schema
`starcrew.ship-layout/1`) and passes `tools/layout_check.py`. What does not exist is the
design document that presents it: every compartment with its purpose, contents, numbers and
reason for being where it is; the routes the crew walk and how long they take; the reasoning
behind the big decisions; the single points of failure; the schema, field by field; and deck
plans drawn to the standard Undercity set for its hub maps (numbered points of interest and a
legend, after Deus Ex: Mankind Divided's Prague map).

Measuring the layout for this write-up also found real gaps: as laid out, the engineering
console cannot be reached on foot from the catwalk or the lower floor of engineering (the
layout has no stairs inside a compartment), the hangar's galleries do not meet the landing, and
the bridge has a single door. This change proposes the fixes as exact layout patches for the
coordinator to apply.

## What Changes

- **The Tern's floor plan as a design document** (`design.md`): the ship at a glance; per deck,
  every compartment by POI number with purpose, contents, volume, floor area, doors and why it
  is there; the hangar, launch bays and engineering across decks.
- **Routes and walk times** at assumed speeds (1.6 m/s walk, 4.0 m/s run, 0.8 m/s on ladders,
  for `crew-on-deck` to confirm): quarters to the bridge 20.2 s; the bridge to the engineering
  console 35.7 s by the aft passage and 36.9 s through the hangar; damage control to the
  forward switchboard 20.7 s; any bridge station to a launch bay 35-38 s; the magazine to the
  torpedo room 26.9 s on foot.
- **Design reasoning**: the double-height hangar with galleries, the always-pressurized aft
  passage, turrets manned from pods, ventral launch bays, and a single-point-of-failure table
  with mitigations.
- **Layout patches proposed** (applied by the coordinator, not here): extend the galleries to
  meet the landing; four stairs inside engineering and the hangar; an emergency scuttle from the
  bridge to damage control; a size for every system.
- **The layout schema documented field by field**, and the checker's rules written as
  requirements of a new `ship-layout` capability.
- **Deck plans**: `tools/deck_plans.py` (documentation tooling, standard library only) draws
  `docs/design/maps/tern-deck-A.svg`, `-B.svg` and `-C.svg` from the layout and shipkit's
  colour roles.
- **The mockup** `docs/mockups/deck-plan.html`: the whole interior as a three.js cutaway in an
  x-ray hull, with deck filters, an exploded view, plan views, info cards and a route tool.

## Capabilities

### New Capabilities

- `ship-layout`: the one layout source per ship (schema `starcrew.ship-layout/1`), the rules
  the layout checker enforces, the deck plans drawn from it, and the reference ship's
  properties that other changes depend on (routes, pressure boundaries, reachability).

### Modified Capabilities

None.

## Impact

- **Data**: `data/ships/tern/layout.json` gains the proposed patches when the coordinator
  applies them (design, section 9): the ship's air becomes 8,924.8 m^3 (from 8,858.8 m^3), its
  floor area 2,157.0 m^2, its portals 41.
- **Tools**: `tools/deck_plans.py` (new); `tools/layout_check.py` gains the walk checks of
  section 8 when the coordinator takes them (it is not edited here).
- **Docs**: `docs/design/maps/tern-deck-{A,B,C}.svg`; `docs/mockups/deck-plan.html` and its
  screenshots in `docs/screenshots/mockups/`.
- **Other changes**: every system change quotes this plan's volumes, positions and routes;
  `crew-on-deck` owns the walking speeds; `deck-pipeline` measures the per-compartment budgets
  on this plan; `life-support` the volumes; `shuttle-bay-and-fighters` the bays.
- **Pi 5 budget**: the floor plan itself spends nothing; its geometry is `deck-pipeline`'s
  table (58,573 triangles for the whole ship at the proposed kit density, every compartment
  under its ceiling).
