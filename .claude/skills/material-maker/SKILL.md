---
name: material-maker
description: Make, change and judge Star Crew's surface materials the repository's way. Material Maker graphs (.ptex) rendered and post-processed into 128 px RGBA layers of one texture array, with relief baked into the albedo under a fixed key light (no PBR on the Pi 5), a fixed texel density (64 px/m inside, 32 px/m on the hull), palette reduction, emission in alpha, and a contact sheet looked at before it is done. Data in data/materials/materials.json, tools in tools/materials. Use whenever making a texture, adding a new material, editing or exporting a Material Maker graph or ptex, recolouring a material, building or checking the texture atlas or texture array, setting texel density or tile size, or asking what a texture costs on the Pi ("make a texture", "new material", "Material Maker", "ptex", "texture atlas", "recolour a material", "texel density", "why does this wall look flat", "the floor texture has a seam").
metadata:
  author: Star Crew (Claude Code)
  version: "1.0"
---

# Material Maker materials

The owner, 2026-10-05: "apply textures from our material maker skills (make one if you dont
have any lol)". This is that skill. CLAUDE.md section 9 has the art rules (flat-shaded low poly,
baked vertex light, few small textures, nearest sampling); `data/materials/materials.json` is the
one source for the materials and its `_rules` define a layer; `tools/materials/README.md` has the
provenance. This is how to work with them.

## The parts

| File | What it is |
| --- | --- |
| `data/materials/materials.json` | The one source: each material's `layer` index, `role`, `source` graph and provenance, `tile_m`, `span_m`, and its knobs. Unknown keys and out-of-range values stop the build. |
| `tools/materials/ptex/<graph>.ptex` | The Material Maker graphs. A graph is the only implementation of how a material looks. |
| `tools/materials/build_ptex.py` | Writes the custom graphs from Python (the `Graph` helper: nodes, links, `material()`). |
| `tools/materials/export_materials.sh` | Clears `tools/materials/raw/` (gitignored), fills it from Material Maker or from fps-game-demo's exports, then runs the post-process. |
| `tools/materials/postprocess.py` | Builds `assets/textures/<name>.png` and `docs/screenshots/materials/contact-sheet.png`, prints the table and a digest. |

## The layer rules

- **One texture array, one size.** Every material is a 128 x 128 RGBA8 layer, whatever it is.
  `layer` is its stable index; add new materials at the end and never renumber (a compiled deck
  names materials by index).
- **Fixed texel density.** Interior layers span 2.0 m (64 px per metre); exterior layers span
  4.0 m (32 px per metre). A surface's UV is its world position over its material's `span_m`, so
  every wall, floor and frame of a space has the same texel size.
- **`tile_m` is one repeat of the graph.** Smaller than the span, the graph is tiled k x k inside
  the layer (hazard and crate: 1 m, so 2 x 2); larger, the layer takes the graph's top-left 1/k
  (trim: one 2 m panel of `tech_panel`, mostly plain scratched surface). The ratio must be whole.
- **The layer tiles.** The downsample wraps around, so the layer's edges are filtered with its far
  side; the table's seam ratios (wrap-around step over the mean step inside) sit near or below 1.
- **No per-pixel lighting.** The baked vertex light multiplies the texel. The normal map and
  occlusion are baked into the albedo once, under a key light from the top of the layer at 45
  degrees (OpenGL normal convention, green is up), so bevels read: top edges lit, bottom edges in
  shade.
- **Alpha is the emission mask.** 255 shows the texel at full brightness whatever the light, 0 is
  lit only by the vertex light. Every non-emissive layer is alpha 0 throughout, so an image viewer
  shows it as transparent; the colour is still there. Never premultiply it or run a PNG optimiser
  that clears colour under zero alpha.
- **Sampling**: nearest on magnification, mipmapped on minification (`layers.mag_filter`,
  `layers.min_filter`).

## Make or change a material

1. **Try a ramp first.** Most new materials are an existing graph recoloured: add an entry to
   `materials.json` with the next `layer`, a `role`, `source` (graph and provenance), `tile_m`,
   `span_m` and a `ramp`. No graph work, and it renders anywhere.
2. **A new look needs a graph**, never a Python image generator imitating one (CLAUDE.md 6.1: one
   implementation per material, the graph). Either:
   - write it in `build_ptex.py` (copy a neighbouring function; bricks, sdf shapes, fbm noise,
     scratches, colorize, blend, tonality, normal_map, occlusion2), add it to `GRAPHS`, run
     `python3 tools/materials/build_ptex.py`; or
   - make or tweak it by hand in Material Maker, save it to `tools/materials/ptex/<graph>.ptex`,
     and **remove it from `GRAPHS`**, or the next `build_ptex.py` run overwrites your edit; or
   - start from a Material Maker example (MIT): save the `.ptex`, add a patch to `PATCHES` if
     metallic or roughness is unwired, and add a provenance row to `tools/materials/README.md`.
   Wire albedo, normal and occlusion at least, and emission for a lamp or screen.
3. **Export.**
   - With Material Maker: `MATERIAL_MAKER_DIR=/path/to/material-maker tools/materials/export_materials.sh [graph ...]`.
     The graphs are Material Maker 1.7 graphs (fps-game-demo's README); a release folder holds
     `material_maker.x86_64`, and a source checkout is run by a Godot 4.7 binary (`GODOT=...`).
     It writes `raw/<graph>_albedo.png`, `_normal`, `_orm`, `_emission` at 2048 px.
   - In a cloud session (no Godot, no GPU, release downloads blocked):
     `tools/materials/export_materials.sh --from-fps /home/user/fps-game-demo` takes fps-game-demo's
     committed exports of the same graphs. It only covers graphs fps has exported. A new or
     changed graph cannot be rendered there: say so in the reply and leave its layer to a machine
     with Material Maker. Do not fake it.
4. **Tune the knobs** and rebuild from the same raw maps with `python3 tools/materials/postprocess.py`
   (seconds; `raw/` stays until the next export).
5. **Look at the contact sheet** (Read `docs/screenshots/materials/contact-sheet.png`) before
   calling it done: each layer at 3 x with its name, span and px/m, and under it the layer tiled
   3 x 3. Judge it with the table below and against its neighbours: the set should read as one
   cool, slightly worn starship (steel greys and slate blues, hazard yellow, low-chroma wear).
   For detail, scale one layer up with nearest neighbour into the scratchpad.
6. **Check it**: run the post-process twice and compare the `digest:` lines (they must match),
   run the dash check (CLAUDE.md 5), and commit the `.ptex`, `materials.json`, the layers and the
   contact sheet together. `raw/` is never committed.

## The knobs

| Knob | Default | Does | Reach for it when |
| --- | --- | --- | --- |
| `ramp` | none | `[[r, g, b] dark, [r, g, b] light]` in 0-1 sRGB. Luminance, stretched so the material's 1st percentile is `dark` and its 99th `light`, picks a colour on the ramp | The graph's colour is wrong for the ship. `light` is roughly the colour of the bulk of a bright material; `dark` is its seams and wear |
| `ramp_mix` | 1 | Blends the ramp over the original colour | Keep a trace of the original hue (0.85-0.95 keeps a little) |
| `relief` | 1 | Blends albedo toward albedo x shade | Lower it if bevels look embossed or noisy |
| `relief_ambient` | 0.45 | Shade of a face turned fully away from the key light | Raise it if bottom bevels go too dark |
| `colours` | 32 | Median-cut palette per layer, no dithering; `null` for none | Lower for a harder retro look; never below what keeps the ramp smooth |
| `emissive` | false | Takes alpha from the graph's emission map | Lamps, screens, indicator panels |
| `tile_m` | required | World size of one graph repeat | Detail too fine at 128 px (raise it) or too coarse (lower it) |

## Judge a layer

| Artefact | Looks like | Usual cause | Fix |
| --- | --- | --- | --- |
| Seam | A line or a jump through the middle tile of the 3 x 3 | `tile_m` larger than the span on a graph whose noise does not repeat there | Pick a `tile_m` where the graph's pattern repeats (a panel joint at the edge hides it), or keep the whole graph; seam ratio over 2 is reported |
| Mush | Fine detail (perforations, treads) turned to noise or moire | Pattern under about 4 px at 128 px | Raise `tile_m` so each feature gets more texels |
| Flat | Panels without edges, one colour | `relief` low, or a graph with no normal wired | `relief` 1; wire the normal in the graph |
| Black blotches | Wear or rust reads as holes | `ramp` dark end too dark for a material whose wear is its darkest part | Lift `dark` |
| Camouflage | Big mottled blobs | High-contrast noise mask in the graph (rust_metal) | Narrow the ramp (dark and light closer); or change the graph |
| Outlier | One layer warmer, brighter or more saturated than the set | Ramp chosen alone | Compare on the sheet; adjust toward the set |
| Banding | Steps in a smooth gradient | `colours` too low | Raise `colours` |
| Invisible PNG | A layer looks empty in an image viewer | Alpha 0 is "not emissive", by design | Look at the contact sheet |

## The Pi 5 cost

A layer is 128 x 128 RGBA8 with its mip chain: **87,380 bytes**. The eleven layers of 2026-10-05
are 961,180 bytes (0.92 MiB), about 1% of the 96 MB texture memory in
`openspec/changes/engine-stack` design section 5, which stays the source for that number: quote
it from there, never from here. Every new material adds 87,380 bytes; say so in the change that
adds it (CLAUDE.md 2). ETC2 would cut a layer to a quarter, but block compression is worst on
small palettised pixel art; it is not used, and would be measured on a Pi before it was.

## In Star Crew, unlike Undercity

The pipeline is adapted from fps-game-demo (Undercity) `f6cd25c` `tools/material_maker`. What
differs:

- **No PBR at run time.** Undercity ships albedo, normal, ORM and emission maps per material into
  Godot's `ORMMaterial3D`. Star Crew ships one RGBA layer: normal and occlusion are baked into the
  albedo, emission is the alpha, roughness and metallic are dropped.
- **Small layers.** 128 px, not 1024 px; one texture array, not a material file per texture.
- **Texel density is per space, not per material.** Undercity's `tile_m` is the world size of a
  texture; here every layer spans 2 m (inside) or 4 m (hull), and `tile_m` only says how the graph
  fills it.
- **Nearest magnification, mipmapped minification**, palette reduction, no anisotropic filtering.
- **Lighting is baked into vertices** (`light-baking` skill); the texel only modulates it.

## What is built and what is not

Built: the graphs, the tools, the eleven layers and the contact sheet. Not built: no engine code
reads the layers yet. The renderer's design (`engine-stack` section 7, the deck shader) and
CLAUDE.md section 9 still speak of one palette atlas; the texture array, the per-draw layer index
(geometry is merged per compartment per material, so a uniform can name the layer) and the
emission blend are to be written into `engine-stack` or `deck-pipeline` before engine code relies
on them (CLAUDE.md 4).

## Do not

- Do not write a Python texture generator that imitates a graph; the graph is the one
  implementation. A cloud session that cannot render a new graph says so.
- Do not hand-edit `assets/textures/*.png`; change the graph or the knobs and rebuild.
- Do not renumber `layer`, change `layers.px` or a span without a change that says why and what
  it costs.
- Do not commit `tools/materials/raw/`.
- Do not call a material done without reading the contact sheet.
- No em or en dashes in data, docs or this skill (CLAUDE.md 5).
