# Proposal: normal maps, relief that answers the room's own lamps

## Why

The owner, 2026-10-08: "Do you have normals for stuff? Should probably add normal maps to give things
that modern textured look".

Today nothing has a normal map. Every surface's relief (a panel's bolts and grooves, a prop's vents
and seams, a Material Maker graph's normal) is baked into its colour once, under one fixed key
light from the upper left of each face (`surface-materials` design, `wall-panels` design 4,
`data/materials/prop_atlas.json` `bake.key_tangent`). The room's light is baked into the vertices
(`light-baking`). That was a rule, CLAUDE.md section 9: "No normal maps, no PBR, no per-pixel
lighting beyond an emissive term", written for the Pi 5's GPU.

The cost of that rule shows up close: a bolt is lit from the upper left whether the lamp is over
it or beside it, a recess under a ceiling lamp shades the same as one under a floor strip, and
nothing shines. That is the "flat" look the owner means. A normal map only helps if the light that
falls on the pixel can change with it, so this change is about where light is computed as much as
about a texture.

## What Changes

- **Recommended: directional baked light with normal maps (design section 3, option C).** The bake
  already knows where every lamp is. It stores, beside each vertex's colour, the direction the light
  mostly comes from, per lighting state. The fragment shader reads the surface's normal map and
  shades the baked colour by how much that pixel's normal faces the light: a bolt lit on its lamp's
  side, a groove shadowed on the other, red alert's strips lighting relief from below. A small
  specular term on metal from the same direction gives the sheen. One extra texture fetch and a few
  multiplies a pixel, no loop over lights.
- **Normal layers beside the colour layers.** Every texture array layer (the 11 materials, the 55
  panel layers, every prop atlas) gets a normal layer, baked from the same Blender geometry that
  already bakes its colour (the panels and props are modelled in 3D: their normals are free) and
  from the Material Maker graphs' own normal outputs. The colour layers stop baking a key light into
  their relief and keep only occlusion and wear, so the light is never counted twice.
- **The dynamic lights** the deck uniform block already allows (up to four, `deck-pipeline`: muzzle
  flashes, sparks, a hand lamp) light the normal maps per pixel too.
- **A Pi 5 probe before any of it ships**: the deck shader with and without the normal fetch, on the
  Pi, at 1280 x 720, the repeatable way (CLAUDE.md 12). The rule in CLAUDE.md section 9 changes in
  the same commit that lands the feature, and only if the probe fits the frame budget.
- **A comparison mockup first**: one room in the deck plan, today's look beside option C, in the
  three lighting states, so the owner judges it on screenshots before the pipelines change.

## Impact

- `light-baking`: the bake writes a dominant light direction per vertex per state (the alpha of the
  three RGBA8 sets that design reserved is not enough; design section 4).
- `surface-materials`, `wall-panels`, `ceilings-and-trims`, `floor-panels`, `ship-props`: each bake
  writes a normal layer and drops the baked key light from its colour.
- `engine-stack`: the deck shader gains a normal fetch and a tangent per vertex; the Pi 5 budget
  table gains the normal layers' memory.
- CLAUDE.md section 9, once measured.
- Nothing about collision, simulation or saves.
