# Proposal: engineering fitted out as a working fusion plant

## Why

The owner, 2026-10-07, on a screenshot of engineering from the deck plan's walk mode: "engine core
room is kinda empty, need consiles, lots of pipes, and heavy machinery in here". Then, with five
reference photos (an industrial pipe gallery in silver lagging, CERN's ALICE, ATLAS and CMS
detectors, a reactor pool with its steam generators): "this right here looks awesome, yea think
about how a spaceship engine works, its heavy hot, needs fuel, needs coolant", "needs constant
maintentance with tools and pipes to move resources around", and "tanks to store or buffer stuff,
pumps to force fluids to move".

**What engineering holds today** (the walk shots in `docs/screenshots/mockups/engineering-before/`):
a 19 m by 14 m hall, 10 m tall through three decks, with the reactor column in its middle and
almost nothing else.
- The reactor is a plain grey column, and its window bands do not read as lit.
- The two switchboards stand on the side walls.
- There is one free-standing desk for the engineer, and one small skid holding both coolant pumps
  on the lower floor.
- No pipe joins the reactor to anything, though `power-grid` simulates a coolant loop of 740 kg/s,
  a fuel load, two generators and five feeders through this room.
- The mezzanine's well round the reactor has no railing.
- The lower floor's ceiling and the mezzanine's floor are flat generic tiles.

## What Changes

- **The plant, drawn as it works** (design section 2):
  - *fuel:* two deuterium dewars and a helium-3 rack feed a fuel processor, whose pellet injector
    feeds the reactor;
  - *confinement:* a cryoplant cools the reactor's magnet coils;
  - *exhaust:* four vacuum pumps draw the reaction chamber's exhaust into an ash tank;
  - *heat:* two coolant loops each run reactor to heat exchanger to pump and back, with a
    pressurizer and a drain and makeup tank as buffers, and secondary loops up to the hull
    radiators;
  - *power:* two converters feed the switchboards, and the feeders' cable trays carry the power
    on.
  Every machine stands on the loop it serves, and each loop the simulation has is drawn where the
  simulation says it runs.
- **An engineering prop set** (`tools/blender/build_engineering_props.py`, `assets/models/engineering/`):
  - 21 machines, each within a triangle budget, with connection ports in its manifest;
  - the reactor dressed after the references: eight red magnet coils, two field rings, radial
    ports and cable looms;
  - consoles: the engineer's control desk, a plant mimic wall and local panels at the machines;
  - maintenance: a bench and tool board, tool chests, parts racks and an overhead crane.
- **Pipe runs as data** (`data/ships/tern/engineering.json`, written and checked by
  `tools/engineering_fitout.py`):
  - 25 runs between the machines' ports, with elbows, flanges, supports and valves;
  - each run names its loop;
  - routed by rules that keep heads, doors, stairs and catwalks clear.
  The kit sweeps them into the room's mesh.
- **The feeders' cable trays.** The five `power.json` conduits in engineering are re-routed from
  diagonals across the room onto trays along the walls. The path drawn is the path the simulation
  uses.
- **Structure:**
  - a ring catwalk round the reactor at deck A level, reached by a bridge from the existing
    catwalk (after the reactor pool's yellow gantry);
  - railings at the well and at every open edge;
  - hangers for the ring catwalk.
- **Engineering's triangle ceiling rises from 8,000 to 30,000,** the bridge's, as the ship's second
  showpiece room (design section 7). That is argued against the frame budget, not assumed.

## Capabilities

### New Capabilities

- `engineering-room`: the plant drawn as the simulation runs it, the pipe-routing rules, railed
  open edges, and the room's triangle ceiling.

### Modified Capabilities

None in `openspec/specs/`. When applied it changes the unbuilt:
- `power-grid`: five conduits' paths in engineering; machines that `power.json` does not simulate
  yet, listed in design section 2;
- `engine-stack` and `deck-pipeline`: engineering's ceiling;
- `ship-props`: engineering's coolant pumps and console come from the new set;
- `reference-ship-tern`: the ring catwalk and the coolant pumps' position.

## Impact

- Data: `data/ships/tern/engineering.json` (new, a layout patch with machines and runs),
  `data/ships/tern/power.json` (five paths).
- Tools: `tools/engineering_fitout.py` (new), `tools/blender/build_engineering_props.py` (new).
- Assets: `assets/models/engineering/` (new).
- Mockups: the deck plan places the machines, sweeps the runs and draws the railings and trays;
  the systems page draws the re-routed conduits.
- Pi 5: engineering's geometry rises to about 30,000 triangles; texture memory rises by the new
  props' atlases; no draw call is added (design section 7).
