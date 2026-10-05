# Proposal: wall panels, not wall tiles

## Why

The owner, 2026-10-05, on the mockups: "whats going on with the walls textures?", with
references (Star Trek: Elite Force, Alien Resurrection, a sheet of wall textures; Quake 1-3,
Halo and Unreal Tournament named), and then "notice how they dont have square panels on the
walls? its all paneling with different shapes, vents, pipes etc to make the room feel
industrial, mechancial and lived in".

Today every interior wall takes one material, `bulkhead`. It is a single 1 m square panel with
rivets and curved scratches (Undercity's `tech_panel` graph), pale and cool, repeated across
every wall of the ship. The scratches read as marble, a 15 m wall shows the same square 45
times, and the ribs and frames wear the same squares, so nothing reads as machinery. The
references, described in `docs/analysis/texture-references.md`, all do the opposite:
- a wall is a sequence of different panels;
- each panel is banded from floor to ceiling;
- light is painted into the panels;
- doors are framed by matching panels;
- the palette is dark and warm, with saturated colour only in the lights.

## What Changes

- **A wall is dressed bay by bay.** The generated ribs (`deck-pipeline` 5a) already divide
  every wall into 2 m bays. Each bay takes one **panel module**: a designed 2 m x 2 m panel such
  as plate, vent, pipes, hatch, junction box, light column, ribbed sheet or screen. Modules are
  chosen by a stable rule: neighbours never match, doors get flanking panels, and windows and
  consoles get plain plate.
- **A wall is banded.** A 0.5 m base strip (kick plate and grille) runs along the floor, the
  module band (2.0 m) sits above it, and a top strip (pipe and conduit run) fills the rest up
  to the cove. A tall wall stacks another module band.
- **Two sets, one per finish:** crew spaces take a cleaner warm grey set and working spaces a
  darker set with rust and hazard. Each set has ten modules and one layer of strips.
- **Panels are layers of the same texture array**, with an emission mask for their light
  strips. They add 22 layers: 1.9 MB at 64 px per metre, or 7.7 MB at 128 px per metre (survey
  S1). The deck stays one draw per compartment.
- **A prototype to judge by eye.** Ten modules per set are modelled and baked in Blender with
  the hard-surface kit, because Material Maker cannot run in a cloud session. They are drawn in a
  comparison mockup (`docs/mockups/wall-panels.html`) beside today's walls, in all three
  lighting states. **How panels are made from then on is the owner's call** (question V2): the
  prototype's Blender bake, Material Maker graphs (today's rule, CLAUDE.md section 9), or Blender
  shapes with Material Maker wear on top.

## Capabilities

### New Capabilities

- `wall-panels`: how a wall is divided into bays and bands, which panel module each bay takes,
  and what the panels cost.

### Modified Capabilities

None in `openspec/specs/` (it holds nothing yet). The unbuilt `surface-materials` change keeps
its tiling materials for floors, ceilings, trims and the hull. Its "every layer tiles" holds for
materials, not for panel modules, and its design says so. `deck-pipeline` 5a gains the wall
dressing as a detail rule.

## Impact

- **Data (proposed):** `data/materials/panels.json`: the module catalogue, the band heights, the
  rule's weights and the glow per lighting state.
- **Tooling:** `tools/blender/build_wall_panels.py` (the prototype's modules), a `panels` step in
  the materials post-process, the kit's wall builder in `docs/mockups/lib/shipkit.js` (opt-in,
  so pages change only when the owner picks), and the comparison mockup.
- **Engine (later):** `deckc` dresses walls by the same rule. The deck vertex already carries a
  layer and a texture coordinate.
- **Pi 5 budget:** +22 texture layers (above). Bands add at most 4 triangles a bay (about 650
  bays on the Tern). No new draw calls.
- **Rules:** if the owner picks Blender (V2), CLAUDE.md section 9's "every material is a Material
  Maker graph" gains "panel modules are modelled and baked in Blender" in the same commit.
