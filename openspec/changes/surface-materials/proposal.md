# Proposal: surface materials from Material Maker

## Why

The owner, 2026-10-05: "add more geometry to make things more interesting looking and apply
textures from our material maker skills (make one if you dont have any lol)". Until now the art
rule was flat-shaded, vertex-coloured low poly with one palette atlas: every room in every
mockup was a flat grey box, and nothing said what a wall, a deck plate or a lamp is made of.

The owner's fps project (Undercity, `/home/user/fps-game-demo`) makes every level material in
Material Maker: graphs authored as code, rendered by Material Maker's command line and
post-processed into Godot PBR materials. It has no skill for it, and its output (1024 px albedo,
normal and ORM maps for per-pixel lighting) is a desktop's, not a Raspberry Pi 5's.

## What Changes

- **Materials are Material Maker graphs**, the only implementation of how a material looks
  (CLAUDE.md 6.1), rendered and post-processed into **small layers of one texture array**:
  128 x 128 px RGBA8, 64 px per metre inside and 32 on the hull, nearest sampling up close and
  mipmaps at a distance.
- **No per-pixel lighting on the Pi**: a graph's normal map and occlusion are **baked into the
  colour** once, under a fixed key light, so bevels read; the baked vertex light (`light-baking`)
  multiplies the texel. Emission is the layer's alpha.
- **The first eleven materials** (bulkhead, two floors, ceiling, trim, hazard, light panel,
  machinery, crate, two hull platings) come from Undercity's graphs, recoloured into one cool,
  worn starship palette. They were built from Undercity's committed Material Maker exports
  because Material Maker cannot run in a cloud session (it needs Godot 4.7 and a GPU).
- **One texture array means one draw call per compartment** whatever its materials: the deck
  vertex carries a layer index and a world-projected texture coordinate (`deck-pipeline`
  section 5 revises the vertex; `engine-stack` the deck shader).
- **Tooling and a skill**: `tools/materials/` (graph builder, exporter, post-process with a
  contact sheet and a digest), `data/materials/materials.json` (the one source), and the
  `material-maker` skill. The mockups load the same layers into the same kind of array
  (`shipkit.loadMaterials`).

## Capabilities

### New Capabilities

- `surface-materials`: what a material is, how a Material Maker graph becomes a Pi 5 texture
  layer, the texel density and sampling rules, and the one texture array a deck draws from.

### Modified Capabilities

None (`deck-pipeline` and `engine-stack` are unbuilt changes, revised in place).

## Impact

- **Data**: `data/materials/materials.json` (schema `starcrew.materials/1`), `assets/textures/*.png`
  (11 layers, 216,715 bytes on disk).
- **Tools**: `tools/materials/` (`build_ptex.py`, `export_materials.sh`, `postprocess.py`, the
  copied `.ptex` graphs, provenance in its README).
- **Mockups**: every interior and the hull are textured through `shipkit.js` (`loadMaterials`,
  `surfaceMaterial`, world-projected UVs), and the budget meter shows texture memory.
- **Rules**: CLAUDE.md section 9 (owner direction 2026-10-05) and section 14 (the skill).
- **Other changes**: `deck-pipeline` (vertex format, the emissive pass, finishes per compartment
  in `detailing.json`), `engine-stack` (the deck shader samples the array; the texture memory
  row), `light-baking` (the bake multiplies the texel; lightmaps, if adopted, multiply it too).
- **Pi 5 budget**: 0.92 MB of the 96 MB texture memory (11 layers with mipmaps), one texture
  bound for every deck draw, one array fetch per fragment.
