# Proposal: deck access, many ways up and down

## Why

The owner, 2026-10-06: "how do the levels connect? can we get starways on the sides? elevators?",
then "we need to install multiple ways to go up and down since various parts of the ship can be
damaged".

**How the decks connect today** (measured by `tools/deck_access.py` on `layout.json`):
- Forward, one ladder joins decks A, B and C, on the centreline at z 11, through the three spine
  corridors.
- Aft, engineering's own stairs join its catwalk (deck A), mezzanine (B) and lower floor (C).
- The hangar's two gallery stairs join decks B and C.
- The missile hoist and the turret pods' hatches are not crew routes between decks.

**One damaged compartment cuts off far too much.**

| Lost compartment | Rooms the bridge can no longer reach, today |
| --- | ---: |
| Command passage (deck A) | 28: every other room in the ship |
| Main corridor (deck B) | 9 |
| Lower corridor (deck C) | 7 |

The command suite does not change that: every door from the bridge and its side rooms opens onto
the passage.

## What Changes

- **Two stair towers, one each side of the spine,** beside the old ladder. Each is a spiral stair
  in a 2.6 x 2.6 m trunk from deck C to deck A. On every deck it has a landing with a door onto the
  corridor and a door into the room beside it. They need no power.
- **A lift (elevator) beside the bridge door,** on the starboard side, from deck C to deck A. Its car
  takes a stretcher, so a casualty goes from the bridge to the medbay in 20.3 s instead of 34.8 s
  down the ladder. It needs power; the towers and the ladder do not.
- **Two emergency scuttles,** a hatch and ladder down from each side room by the bridge: the ready
  room to the medbay and the briefing room to damage control. The second is `reference-ship-tern`'s
  T3, taken by the owner's direction.
- **Second ways out:** six doors through walls two rooms already share. They are for the rooms whose
  only door is on a spine corridor: the turret rooms, the torpedo room, the shield room, the
  switchboard and the magazine.
- **The ladder stays,** as the route that needs neither power nor a tower.

With all of it, no single lost compartment cuts the bridge off from any room except the true dead
ends. Those are a turret pod behind its access room, the launch bays behind the hangar, the drive
section behind engineering, and the airlock behind cargo. The result holds with the lift unpowered too.

## Capabilities

### New Capabilities

- `deck-access`: the routes between decks, and the rule that one lost compartment never cuts off a
  room that is not a dead end.

### Modified Capabilities

None in `openspec/specs/`. When applied it changes the unbuilt `reference-ship-tern` (rooms reshaped
around the trunks, T3 answered), `crew-on-deck` (spiral stairs and the lift as ways to move),
`damage-control` and `life-support` (three new compartments, ten more doors), and `power-grid` (the
lift's load).

## Impact

- **Data:** a patch, `data/ships/tern/deck_access.json`, applied after `command_suite.json`. The
  layout gains two fixture kinds, `spiral_stair` and `lift`, and the trunk compartment kind.
- **Tooling:** `tools/deck_access.py` writes and checks it, and measures single losses and route
  times. The mockup kit draws spiral stairs and a lift car. The deck plan shows the proposal by
  default.
- **Pi 5 budget:** design section 6.
