# Proposal: ship props, the machinery and the crew rooms modelled

## Why

The owner, 2026-10-06, after walking the deck plan in first person: "You had like really dope looking
bridge chairs and consoles in one of the mockups, bring that into the deck plan please. Also fix the
lights. UV them correctly. They look off centered. I recommended you go in fps mode and take screenshots
and analyze problems that you see. The new walls look great. But anything left over from initial design
should be redone".

A first-person tour of every compartment (`docs/screenshots/mockups/walk-tour-before/`) found what the
first designs left behind:
- **Systems were grey boxes** with a glowing band: the switchboards, the battery bank, the coolant pumps,
  the drive, the dampers, the shield generator, life support's five machines, the gravity generator,
  the medbay beds, the magazine, the missile tubes, and the reactor as a plain cylinder.
- **Craft were boxes and cones:** the two Swift fighters and the Petrel shuttle.
- **Stations away from the bridge were blocks:** engineering's console, the damage control board, bay
  control, and the four gunners' seats.
- **The bridge and the suite's furniture were blocks in the deck plan,** where the command deck page
  draws the Blender consoles, chairs and furniture.
- **The crew quarters and the mess were empty,** though their purposes name bunks, lockers, a galley
  and tables.
- **Every lamp's lens showed an arbitrary window onto the light-panel texture,** off centre.
- **Floor hatches had no ladders** (the deck plan drew flat coloured bars), and the lift car's walls
  took the machinery texture.
- **The lift stood 0.6 m behind the briefing room's door from the bridge.** That one is a `deck-access`
  bug, fixed there (its design section 3), with a check that no trunk stands in a door's zone.

## What Changes

- **A machinery prop set** (`assets/models/machinery`, built by `tools/blender/build_machinery_props.py`
  with the hard-surface kit): twenty props for the systems, the craft and the crew rooms, each within a
  triangle budget.
- **The crew rooms furnished:** four two-tier bunks and lockers in the crew quarters, a galley and two
  tables of four in the mess, a workbench and lockers in damage control
  (`data/ships/tern/crew_rooms.json`, written and checked by `tools/crew_rooms.py`).
- **The deck plan places props, not blocks:** the bridge's consoles and chairs and the suite's
  furniture as the command deck does; a console and a chair at every other station by one rule; each
  system as its machinery prop; the craft. A block stays only for what no set models.
- **The kit:** a lamp's lens shows two whole light panels, centred; a ladder runs up through every
  floor hatch, to the floor above; the lift car takes a deck plate floor, bulkhead walls and a trim
  rail; the trunk lighting of `deck-access`.
- **propkit:** a prop's light-panel strips glow; the starfield on the viewscreens and in the windows is
  shared, so the deck plan's viewscreen shows space as the command deck's does.

## Capabilities

### New Capabilities

- `ship-props`: every system, craft, station and crew room drawn as a modelled prop within its budget.

### Modified Capabilities

None in `openspec/specs/`. It draws the unbuilt `reference-ship-tern`'s rooms, `bridge-stations`'
consoles and `command-suite`'s furniture.

## Impact

- New: `tools/blender/build_machinery_props.py`, `assets/models/machinery/`, `tools/crew_rooms.py`,
  `data/ships/tern/crew_rooms.json`, `docs/screenshots/props/machinery-props.png`.
- Changed: `docs/mockups/lib/shipkit.js` (lens, hatch ladders, lift car), `docs/mockups/lib/propkit.js`
  (glowing strips, space views), `docs/mockups/deck-plan.html`, `docs/mockups/command-deck.html` (uses
  the shared space views), `tools/blender/render_props.py`.
- The deck plan's rooms gain the props' triangles and a console-face draw where they have screens
  (design section 5).
