# Design: surface materials from Material Maker

## Context

- **The owner's direction** (2026-10-05): "apply textures from our material maker skills (make
  one if you dont have any lol)". There was no Material Maker skill in any of the owner's
  repositories; Undercity (`fps-game-demo`, revision `f6cd25c`) has the pipeline:
  `tools/material_maker/build_ptex.py` authors `.ptex` graphs as code, `export_materials.sh`
  drives Material Maker's command-line export, `postprocess.py` writes Godot materials, and
  `game/materials/materials.json` sets each material's real-world tile size so every level
  agrees on texel density.
- **The floor is a Raspberry Pi 5** (CLAUDE.md 2): OpenGL ES 3.0, no per-pixel lighting budget
  beyond an emissive term, memory as the hard limit, 96 MB of texture memory in the provisional
  table (`engine-stack` section 5). Texture arrays are core in ES 3.0.
- **Light is baked into vertex colours** (`light-baking`), one set per lighting state.
- **Material Maker cannot run in a Claude Code cloud session**: it needs Godot 4.7 (Material
  Maker's `project.godot` declares 4.7) and a GPU, and GitHub release downloads are blocked by
  the session's egress policy. Undercity's committed exports are real Material Maker renders of
  its graphs (graphs and exports landed together in its commit `b832b84`).

## Goals / Non-Goals

**Goals:** a material is defined once, as a graph; any machine with Material Maker rebuilds the
layers; the layers cost almost nothing on a Pi 5; one draw call per compartment survives
texturing; the look is judged on a contact sheet and in the mockups before it is called done.

**Non-Goals:** PBR, normal maps or specular at run time; decals and screen textures (a small
further set, later); per-ship palettes (one material set for now; a ship may recolour by ramps
later).

## Decisions

### 1. A layer, not a material: one texture array

Every material is one layer of one `GL_TEXTURE_2D_ARRAY`, 128 x 128 px, RGBA8 (sRGB colour,
linear alpha). The deck vertex carries the layer index (a `u8`) and a texture coordinate
(`deck-pipeline` section 5), so one draw covers a compartment's floor, walls, trims and lamps
whatever their materials.

| Option | Draw calls per compartment | Why not |
| --- | --- | --- |
| **Texture array (chosen)** | 1 | |
| One texture per material | One per material in the room, 6-9 | Triples the deck's draw calls against a 300-call frame |
| One atlas with sub-rectangles | 1 | Wrapping a tiled texture inside an atlas needs `fract()` and padding in the shader, and mipmaps bleed across sub-rectangles |

**Why 128 px.** At 64 px per metre a 128 px layer spans 2 m, the frame spacing
(`deck-pipeline` section 5a), so a wall panel's seams line up with the ribs. Seen from 1 m at
1280 x 720 with a 75 degree field of view a texel is about 6 screen pixels: the chunky, crisp
look of the low-poly style. Eleven layers with mipmaps are 961,180 bytes.

### 2. Texel density is fixed per space

A surface's texture coordinate is its world position, projected on the plane it faces (x and z
for a floor or ceiling; the face's horizontal tangent and y otherwise), divided by its
material's `span_m`: 2.0 m inside (64 px per metre), 4.0 m on the hull (32 px per metre). So
every wall, floor and frame of a space has the same texel size, a texture runs continuously
across the segments of one wall, and nobody unwraps UVs. `tile_m` is one repeat of the graph: a
graph smaller than the span is tiled k x k inside the layer (hazard stripes, the light panel and
the crate are 1 m, so 2 x 2); a larger one gives the layer its top-left 1/k (trim takes one 2 m
panel of the 4 m `tech_panel`). The ratio must be whole, so the layer tiles.

### 3. Relief baked into the colour

The Pi draws no per-pixel lighting, so a graph's normal map and occlusion are spent once, in the
post-process:

```
shade  = ao * (ambient + (1 - ambient) * max(0, n . L) / max(1e-3, n0 . L))
albedo = albedo * lerp(1, shade, relief)
```

with `n` the graph's normal (OpenGL convention, green up), `n0 = (0, 0, 1)` a flat face, `L` the
key light from the top of the layer at 45 degrees (`bake.key_light_tangent` [0, 1, 1]),
`ambient` the shade of a face turned away (`relief_ambient`, 0.45), `relief` 0-1 per material.
A flat face keeps its colour; a top bevel brightens, a bottom bevel darkens, so panels read as
panels under any baked light. The light is fixed in texture space: on a wall it comes from the
ceiling, which is where the lamps are.

### 4. Recolour, reduce, emit

- **Ramp**: luminance (Rec. 709 weights), stretched between the material's 1st and 99th
  percentiles, mapped onto a dark-to-light colour pair and blended over the original by
  `ramp_mix`. This is how one Undercity graph becomes a Star Crew bulkhead, a trim and two hull
  platings. Undercity's own `albedo_ramp` maps raw luminance; the stretch makes the two ramp
  colours mean the material's darkest and lightest texels.
- **Palette reduction**: median cut to `colours` (32) per layer, no dither, for the retro look
  and a smaller PNG.
- **Emission**: alpha is the emission mask (255 full glow whatever the light, 0 lit only by the
  bake), taken from the graph's emission map for an emissive material (the light panel: 56 % of
  its texels glow). The emissive pass multiplies the texel by the lighting state's tint. Because
  non-emissive layers are alpha 0 throughout, no tool may premultiply alpha or strip colour
  under zero alpha; the mockups decode the PNGs themselves for that reason (`shipkit.decodePng`).

### 5. Sampling

Nearest on magnification (the texel look), trilinear mipmaps on minification (no shimmer on a
far wall), repeat wrapping. Mipmaps of an RGBA8 array cost a third more than the base level and
are generated at load.

### 6. The first eleven materials

Built 2026-10-05 from Undercity's graphs (provenance per file in `tools/materials/README.md`),
ramps chosen by eye on the contact sheet (`docs/screenshots/materials/contact-sheet.png`) as a
cool, slightly worn interior: steel greys and slate blues, hazard yellow kept, rust muted.

| Layer | Material | Graph | Span | Role |
| ---: | --- | --- | --- | --- |
| 0 | bulkhead | tech_panel | 2 m | Walls |
| 1 | deck_tiles | floor_tiles | 2 m | Floors in crew spaces |
| 2 | deck_plate | diamond_plate (Material Maker's `metal_pattern` example) | 2 m | Floors in working spaces |
| 3 | ceiling | ceiling_tiles | 2 m | Ceilings in crew spaces |
| 4 | trim | tech_panel, one panel | 2 m | Frames, ribs, beams, door frames, baseboards |
| 5 | hazard | hazard_stripes | 2 m (1 m tiles) | Rims, kick plates, pressure doors, working-space baseboards |
| 6 | light_panel | light_panel, emissive | 2 m (1 m tiles) | Lamp lenses, status strips |
| 7 | machinery | rust_metal, muted | 2 m | Working-space ceilings and frames, pipes, housings |
| 8 | crate | crate, olive | 2 m (1 m tiles) | Cargo |
| 9 | hull | tech_panel | 4 m | Exterior plating |
| 10 | hull_dark | tech_panel, 1 m panels | 4 m | Exterior accents |

Which material each generated deck surface takes is a ship's **finish** table
(`data/ships/<id>/detailing.json`, `finishes`), chosen per compartment by its `finish` in the
layout (`crew` or `working`, `deck-pipeline` section 5a).

### 7. Making them: tools and the skill

```
tools/materials/build_ptex.py      graphs as code (optional; a hand-edited graph leaves GRAPHS)
        |  .ptex
Material Maker 1.7 on Godot 4.7    --export-material, 2048 px albedo, normal, ORM, emission
        |  tools/materials/raw/ (gitignored)        or  --from-fps: Undercity's committed exports
tools/materials/postprocess.py     ramp, relief bake, fit to the layer, palette, emission alpha
        |
assets/textures/<name>.png  +  docs/screenshots/materials/contact-sheet.png  +  a digest
```

The post-process validates `materials.json` strictly (an unknown key, an out-of-range knob, a
non-whole span ratio or a gap in the layer numbering stops it, naming the field) and is
deterministic: two runs give the same digest on one machine (Pillow 12.3.0, numpy 2.4.6; other
versions may differ in the last bit). The `material-maker` skill is the how-to: try a ramp
before a new graph, never imitate a graph in Python, look at the contact sheet and the tiled
3 x 3 before calling a layer done.

### 8. What the mockups do

`shipkit.loadMaterials` decodes the inlined PNGs (its own PNG decoder, because a canvas would
premultiply), flips rows so a wall stands the right way up, and builds a `DataArrayTexture` with
the same filters; `surfaceMaterial` samples it by the vertex's layer, lit by scene lights or
(`lit: false`) by baked vertex colours, the engine's way. Every compartment is one mesh, and
the budget meter shows the texture memory (0.92 MB). `tools/mockups/inline.py` inlines the
manifest and layers between `<!-- INLINE materials -->` markers (about 290 KB per page).

### 9. The Pi 5 budget this change spends

| Item | Cost | Budget |
| --- | --- | --- |
| Texture memory | 961,180 bytes (11 layers, RGBA8, mipmaps) | 96 MB (1 %) |
| Bound textures per deck draw | 1 array | |
| Fragment work | One array fetch, one multiply by the blended vertex colour, one emissive multiply-add | Within the deck shader `engine-stack` plans |
| Disk | 216,715 bytes of PNG | |
| Vertex | Layer and texture coordinate replace the palette coordinate and the flags byte: still 28 bytes | `deck-pipeline` section 5 |

Unmeasured on a Pi: whether nearest magnification with trilinear minification on a
`TEXTURE_2D_ARRAY` costs the same as a 2D texture on the VideoCore VII. `sc-probe` scene 3
(`engine-stack` section 11) draws the deck with and without the array.

## Risks / Trade-offs

- **The first layers are not fresh Material Maker renders.** They come from Undercity's 1024 px
  exports, themselves downsampled from 2048 px by Undercity's post-process; a real render gives
  slightly different layers. Mitigation: the exporter's normal path renders the graphs, the
  contact sheet is the check, and the digest shows what changed.
- **Alpha 0 means "not emissive"**, so the PNGs look empty in an image viewer and a careless tool
  can wipe their colour. Mitigation: the rule is in `materials.json`, the skill and this design,
  and the mockups decode the PNGs without premultiplying.
- **Relief from one fixed light** reads wrong on a floor seen from the side the light "comes
  from"; accepted for the low-poly style, where the bake's light dominates.
- **Machinery keeps rust_metal's mottling**, which reads slightly like camouflage close up; a
  later graph or ramp change, judged on the contact sheet.

## Open questions

Questions go to the owner only with something to look at (CLAUDE.md 13).

| Id | Question | Options | Recommendation | Shots |
| --- | --- | --- | --- | --- |
| S1 | The texture density and look | a. 128 px at 64 px per metre (crisp texels); b. 256 px at 128 px per metre (finer, 4 MB) | a: the low-poly look, 1 % of the texture budget | Survey S1: the contact sheet, and the bridge and main corridor shots from the bridge and deck plan mockups |
