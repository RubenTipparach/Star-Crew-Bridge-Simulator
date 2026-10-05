# Proposal: the deck pipeline, brush-built decks compiled offline

## Why

The owner, 2026-10-04: "a full floor plan" with "a full 3D starship bridge", and "So definitely
bsp style level design would be on the table!". The floor is a Raspberry Pi 5 with 1 GB of RAM
as a client, and a 4 GB Pi 5 can host the dedicated server (owner, 2026-10-04: "we're running
on a pi5 1gb-4gb, 4 GB can be used as main server too"). Its VideoCore VII GPU runs OpenGL ES
3.1; the renderer's floor is OpenGL ES 3.0 (`engine-stack`).

A Pi 5 is not the machine that forced Quake to precompute visibility, and this change says so
plainly: the reference ship's whole interior, at the proposed detail, is about 59,000
triangles, under a third of the 200,000-triangle frame budget. The brush-and-portal approach is
chosen for three other reasons that hold on any hardware:

1. **One compartment graph** (CLAUDE.md section 7). Air, fire, sound, crew pathing and damage
   already need compartments and the openings between them. Visibility through the same
   portals costs microseconds and needs no second structure.
2. **Exact, cheap collision.** Convex brushes give planes to collide a crew capsule against on
   the server, with no mesh collider and no physics engine.
3. **Authoring clarity in a low-poly style** (owner: "this game doesn't need high end
   graphics"). Boxes and convex detail compiled from the one layout source are diffable,
   checkable (z-fighting, people standing clear) and regenerated when the plan moves.

It also keeps the budget honest at the edges a Pi 5 still has: fill rate at 1280 x 720 when
compartments stack behind each other, 1 GB shared between the CPU and the GPU, and bigger ships
or two ships docked later. CLAUDE.md section 8 fixes the shape; this change designs the
pipeline and puts a number on every compartment of the reference ship.

## What Changes

- **Authoring.** `data/ships/<id>/layout.json` stays the plan. A generator, `deckgen`, turns it
  into convex brushes per compartment from a kit of trims, ribs, beams, lamps, frames, ladders
  and stairs. Hero rooms (the bridge, engineering) may add hand-authored detail brushes, props
  and light fixtures, placed in Blender and exported as a detail file, never as boolean meshes.
  TrenchBroom and a browser editor were weighed and are not the backbone (design, section 1).
- **The offline compiler, `deckc`** (in `sc-tools`, per `engine-stack`). It reads the layout,
  the kit and the detail files, calls the `light-baking` baker, and writes one `.deck` file per
  ship: compartments and portals keyed to the layout's ids; convex collision brushes with a
  k-d tree where a compartment is large; render meshes merged per compartment into at most
  four draw calls; three baked vertex-colour sets (normal, red alert, emergency); light
  fixtures, seats, spawns, ladders, stairs and door movers as entities.
- **Checks that refuse a deck**: brush convexity, no z-fighting (5 mm coplanar tolerance, 1 cm
  separation for deliberate parallels), people stand clear (the crew capsule at every seat,
  spawn, ladder end, stair end and door threshold, and every seat reachable on foot), detail
  inside its compartment, per-compartment triangle and draw-call ceilings, the worst visible
  set against the interior pass ceiling, finite numbers, and the layout hash.
- **The `.deck` format**: little endian, chunked, versioned, a CRC-32 per chunk and for the
  header, and the SHA-256 of the layout it was compiled from. Core chunks (graph, brushes,
  entities) are read by the server; render chunks only by the client. About 2.5 MB for the Tern.
- **Run time**: portal culling through the compartment graph with screen-space rectangles that
  narrow at each portal and doors that cut visibility when closed; a scissor per compartment;
  windows and open bay doors handed to `ship-frames`' exterior layers as scissor rectangles;
  capsule collision against brush planes. No precomputed PVS; small BSP trees only where they
  pay (collision in the two big compartments, back-to-front glass).
- **Budgets per compartment for the Tern**, measured from the layout: on the v2 plan
  (2026-10-05), 35,180 triangles for the whole ship before the light baker's subdivision (the
  kit's generated shell and detail, measured, plus the first estimate's props), where the first
  estimate on the v1 boxes, with a lighting grid standing in for the subdivision, was 58,573.
  Every compartment is under the `engine-stack`
  ceilings (bridge 30,000, others 8,000); the interior pass is capped at 80,000 triangles and
  120 draw calls a frame. The ceilings are not targets.

## Capabilities

### New Capabilities

- `deck-geometry`: how a ship's decks are authored, compiled, validated, stored and drawn: the
  generator and its kit, `deckc` and its refusals, the `.deck` file, collision, portal culling
  and the per-compartment budgets.

### Modified Capabilities

None.

## Impact

- **New tools** (in `sc-tools`, not yet built): `deckgen` and `deckc`, plus `deckc --report`,
  which prints the per-compartment table this design estimates and replaces the estimate.
- **New data**: `data/ships/<id>/detailing.json` (the generated detail's rules, sizes and
  finishes; written 2026-10-05, replacing the proposed `data/decks/kit.json`), per-ship
  `data/ships/<id>/detail/<compartment>.json` for hero detail and hand-placed fixtures.
- **Generated detail** (2026-10-05, owner: "look at our fps thing to design better levels, add
  more geometry"): frames, beams, coves, baseboards, door and window frames, rims, ladders,
  railings, conduits, runners and lamps generated from the layout by rules adapted from
  Undercity's `detailing.py` and its UT99 checklist (design section 5a), drawn today by the
  mockups' `shipkit.js` and measured by `tools/mockups/kit_report.mjs`.
- **Engine**: a format module shared by `deckc`, `sc-server` and `sc-render` (one definition of
  the file); portal traversal and draw submission in `sc-render`; brush collision in `sc-core`
  (crew movement is simulation, server authoritative).
- **Other changes**: `engine-stack` (the budget table and the crates), `light-baking` (the baker
  this compile step calls; it owns how light is computed), `ship-frames` (the exterior layers
  take the window rectangles), `crew-on-deck` (the crew capsule and walking speeds the checks
  and route times use), `reference-ship-tern` (the floor plan these numbers are measured on).
- **Pi 5 budget**: spends the interior pass: at most 80,000 triangles and 120 draw calls per
  frame (the Tern's worst case is under 54,000 with every door open), 2.3 MB of the 64 MB of
  vertex and index buffers, and about 0.2 MB of collision and graph data on the server.
