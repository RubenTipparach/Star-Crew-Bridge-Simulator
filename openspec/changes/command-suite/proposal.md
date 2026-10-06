# Proposal: the command suite, a round bridge with side rooms

## Why

The owner, 2026-10-06, on the three bridge variants (`bridge-stations` design 11a, survey B11):
"can we do the more circular bridge? but have like side rooms for meetings, captains quarters and
stuff?"

That picks **B, the round bridge**, and asks for rooms around it. The owner's own reference R2
(`docs/analysis/star-trek-bridges.md`) has this shape: a round main room with a briefing room on
one side, a door to the ready room and an elevator between them.

The round room also leaves space to use. B gives up 192.7 m^3 of the wedge's air, and 11a said
the corners between the round wall and the hull "become service space, which the layout does not
use yet". Deck A is also empty either side of the command passage between z 4 and z 14: about
45 m^2 a side that no room holds today.

## What Changes

- **The bridge becomes variant B**, with two changes:
  - **Doors at walkway level.** The ring starts one wall segment further forward, so a door in each
    aft diagonal wall opens from the walkway: the ready room to port, the briefing room to
    starboard. Each side still has four wall banks.
  - **Helm and tactical clear the screen.** They sit at two desks 1.5 m either side of the
    centreline. This is 11a's fix: from the captain's eye their heads now hide none of the
    screen, where B's single curved console hid 6.3 % of it.
- **The captain's ready room** keeps its room (z 14-20, port) and grows forward into the corner
  between the round bridge and the hull. Its new door opens onto the bridge. It has a desk with
  the captain's private console, a sofa and a window.
- **A briefing room** for meetings and mission briefings mirrors the ready room to starboard,
  with its own door onto the bridge. It holds a table for eight facing a wall screen, and a window.
- **The captain's quarters** (bed, wardrobe, desk, en-suite) take z 9-14 to port, with a door to
  the command passage and one into the ready room.
- **The computer core moves aft**, to z 9-14 starboard, so the briefing room can stand beside
  the bridge.
- **A head** (two toilets, two showers, a wash counter) and **a bridge locker** take z 4-9. The
  locker holds the bridge crew's own EVA suits and emergency kit, spare console boards and a
  workbench.
- **The rooms are furnished** with a new set of props, modelled in Blender the hard-surface way
  (`tools/blender/build_suite_props.py`, `assets/models/suite`).
- **A mockup of the whole command deck** (`docs/mockups/command-deck.html`) draws the bridge and
  every side room with their furniture, lit in the three lighting states.

The layout does not change yet. The suite is a patch, `data/ships/tern/command_suite.json`,
written and checked by `tools/command_suite.py`. Applying it to `layout.json`, and rerunning every
number that depends on the plan, is this change's task 2.

## Capabilities

### New Capabilities

- `command-deck`: the rooms of deck A around the bridge, how they connect, and how their
  furniture leaves the doors and walkways clear.

### Modified Capabilities

None in `openspec/specs/`. When applied it changes the unbuilt `reference-ship-tern` (deck A's
rooms), `bridge-stations` (11.1 rewritten from B, 11a closed) and the numbers other changes quote
from the plan (design section 8).

## Impact

- **Data:** the patch file now. Once applied: `layout.json`, which gains platforms (the dais
  fixture retires), four compartments, ten portals and a locker. The furniture moves into a deck
  detail file. `power.json`, `atmosphere.json` and `damage.json` gain the new rooms.
- **Tooling:** `tools/command_suite.py`, new. `tools/bridge_variants.py`'s variant B takes
  parameters (its output is unchanged). A suite prop builder shares one Blender module with the
  bridge props.
- **Mockups:** a new command deck page. The other pages move to the suite when it is applied.
- **Pi 5 budget:** design section 9. The furniture uses existing materials (no new texture
  layers), and each room stays one draw call.
