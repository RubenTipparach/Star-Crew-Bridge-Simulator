# Materials: Material Maker graphs to texture layers

This directory turns Material Maker graphs into the 128 px texture layers Star Crew's surfaces
use (`assets/textures/<name>.png`). `data/materials/materials.json` is the one source for which
materials exist, which graph each comes from, its size in metres and its post-process knobs;
its `_rules` say what a layer is. The how-to is the `material-maker` skill
(`.claude/skills/material-maker/SKILL.md`).

| File | What it does |
| --- | --- |
| `ptex/*.ptex` | The Material Maker graphs: one per source material, the only implementation of how it looks. |
| `build_ptex.py` | Writes the custom graphs from code (reproducible, diffable). Remove a graph from its `GRAPHS` once you edit it by hand in Material Maker. |
| `export_materials.sh` | Renders the graphs into `raw/` (gitignored) with Material Maker's command line, or takes fps-game-demo's committed exports with `--from-fps <dir>`, then runs the post-process. |
| `export_graph.py`, `render_graph.gd` | Export individual graphs through Material Maker's own renderer. Resolve bitmap paths and stage outputs in a fresh directory so the Windows 1.4 exporter needs no overwrite dialog. |
| `postprocess.py` | Recolour, relief bake, fit to the layer, palette reduction; writes the layers, the contact sheet (`docs/screenshots/materials/contact-sheet.png`) and a table of sizes, texel densities, bytes and seam ratios. Deterministic: the last line is a digest. |

```sh
python3 tools/materials/build_ptex.py                                   # graphs (only when one changed)
MATERIAL_MAKER_DIR=/path/to/material-maker tools/materials/export_materials.sh  # render + build
# no GPU or screen (a cloud session): the material-maker-headless skill's setup.sh, then the line above
tools/materials/export_materials.sh --from-fps /path/to/fps-game-demo   # no Material Maker: fps's exports
python3 tools/materials/postprocess.py                                  # rebuild from raw/ after a knob change
```

Needs Python 3 with numpy and Pillow (`pip install numpy pillow`).

Shoulder refinement, 2026-10-11: `tools/blender/project_shoulder_armor.py` bakes
`sources/tern-shoulder-armor-mask.png` on the actual hull. The native hull graphs
sample clear armor from the approved `tern-seamless-finish.png` at its existing texel scale through that mask,
then apply the existing projected seams and livery. Both channel ends meet panel
joints, without a pointed grey sliver. The source painting is retained unchanged;
the mask is offline authoring data and adds no runtime allocation. See the
[shoulder review](../../docs/screenshots/exteriors-shoulder/README.md).

## Custom exterior paint, 2026-10-10

Nacelle refinement, 2026-10-11: [tern_nacelle.ptex](ptex/tern_nacelle.ptex) packages the
dedicated neutral [generated painting](sources/tern-nacelle-finish.png) from built-in
imagegen ([exact prompt](sources/tern-nacelle-prompt.txt)). The input is the
[original projected painting template](sources/tern-nacelle-painting-template.png), built by
`tools/blender/project_nacelle_paint.py` from the model's triangles and ship-local
station planes. The wireframe and JSON retain the receiving polygons. The approved
hull painting was a style reference. Final cap packing separates the collector from
the rail; `tern-nacelle-projected-layout.png` and its wireframe show the delivered
UVs. The original painting input remains alongside them. No Python-generated surface art is used.

The 887 x 1774 px painting passes through a native 2048 px Material Maker render,
then rectangular 1024 x 2048 packaging with zero emission. Its standalone RGBA8
allocation, including mipmaps, is 10.67 MiB. It is shared by both nacelles and all
liveries; livery accents remain separate geometry. This fitted exterior atlas uses
the documented exterior-atlas workflow, rather than adding an interior array layer.
Its shell receives 35.11 longitudinal texture px/m; original source detail is
30.41 px/m. Full models and the six custom LOD atlases contain the new artwork.

Ship linework follows the owner's [3D projection workflow](../../CLAUDE.md#9-art-low-poly).
[`project_hull_paint.py`](../blender/project_hull_paint.py) builds the actual hull, projects
ship-local seams and oblique livery bands, and bakes their color and region masks into its UVs.
[`hull_paint_projection.json`](../../data/ships/tern/hull_paint_projection.json) owns the metre
positions, widths and colors. The [editable Blender source](../../assets/models/exteriors/hull-paint-projection.blend),
[unenhanced layout](sources/tern-projected-layout.png), [masks](sources/tern-projected-masks.png)
and [3D inspection captures](../../docs/screenshots/exteriors-equator-layout/) are retained.
The mask uses R for livery, G for seams and B for projected hull coverage.
The separate `tern-projected-ribbon.png` uses R for the inset white outline, G for
the full ribbon and B for receiving hull coverage. It is an offline authoring input;
the outline is composited into each existing hull atlas. The bake checker
compares actual surface samples against the ship-local band field with zero mismatches.

The projection starts from the seam-free `tern-seamless-finish.png`, retaining machinery and
registration while removing the obsolete roof seams. All roofs, bevels and walls receive the
same station planes. The main hull paint has a flat 9 m central join, with a narrow equatorial
service belt excluded from the livery. `exterior_uv_guide.py` exports supporting mesh edges.
After inspecting the layout in 3D, built-in imagegen produced `tern-clean-finish.png` with
quieter ivory paint. The current `tern-integrated-joints.png` and its retained built-in
imagegen prompt enhance those same projected boundaries with recessed cores, ivory lips
and localized wear, matching the owner reference. `tern-flat-painted-cyan.png` adds the pigment finish after inspection of
the new flat-band layout. The three graphs preserve its tonal variation under the colors in
`data/ships/exteriors.json`, apply the fine white border, retain dark mechanical relief, and sample varied joint-core tones from the enhanced finish
through the 3D seam mask. Seam-free source texels remove guide strokes over pipes.
Geometry and materials own the
narrow physical service recess. Blender bakes each reduced atlas from the finished full model.

`check_hull_projection.py` checks actual packed stripe color coverage, samples both receiving
UV faces at shared seam crossings, measures ten real roof traces for the flat center edges,
rejects uniform-color paint, confirms Material Maker RGB is preserved, and rejects emissive
hull paint. All eighty projected profiles are checked; twenty-eight broad armor profiles
also verify the finished varied recess. Mechanical boundaries are classified using the
retained seam-free source, since pipes no longer receive plate-seam strokes. Run it after rendering and packaging the graphs. Sources, exact imagegen prompts,
graphs and models are linked in the [asset README](../../assets/models/exteriors/README.md).

The original [paint bitmap](sources/exterior-paint.png) was generated with the built-in image_gen tool. Its [complete prompt](sources/exterior-paint-prompt.txt) is committed beside it. The [exterior paint graph](ptex/exterior_paint.ptex) imports that bitmap as its editable albedo input. It supplies pearl and graphite ramps without a panel grid. Material Maker 1.4 rendered the graph at 2048 px using the owner's NVIDIA GPU; the existing post-process produced the two 128 px layers at 32 px/m. Existing layer indices remain stable; the new layers are 11 and 12, costing 174,760 bytes with mipmaps.

```powershell
python tools/materials/build_ptex.py
python tools/materials/export_graph.py exterior_paint --material-maker C:/Users/santi/repos/material_maker_1_4_windows/material_maker.exe
python tools/materials/postprocess.py --only exterior_pearl exterior_graphite
```

`--only` rebuilds the selected layers and uses the other committed PNGs for the complete contact sheet. It does not require raw exports for unchanged graphs. Two builds of the custom paint produce digest `09732705c1517bd9443c10dc0680f04cea3bc983e69603f451c1cef034cf7809`. The Windows exporter emits legacy texture-format and UI-scale messages but produces a valid albedo and exits successfully. Fresh output staging prevents its overwrite-dialog crash; the wrapper refuses a missing export.

Lower-detail exterior atlases are separate Blender bakes of the full model, not repeated copies of the paint layer. Their source is [the exterior builder](../blender/build_ship_exteriors.py).

**Plain steel, 2026-10-14.** `ptex/plain_steel.ptex` is Star Crew's own graph (`plain_steel()` in
`build_ptex.py`): painted steel with a horizontal brushed grain and a few light scratches, with no
panels, seams or grid. `bulkhead` and `trim` take it with two ramps (slate blue-grey walls, lighter
grey trims). The owner had the `tech_panel` grid taken off the interiors ("I just wanted you to
remove the grid pattern that I didnt like that you were using for ship interiors") and allowed
generic base textures there. It was rendered by Material Maker `174f15e` on the Raspberry Pi 5 itself:
Godot 4.7's linux.arm64 build, the `material-maker-headless` checkout and patches, Mesa's lavapipe
(`VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.aarch64.json`) on the Pi's desktop display, about
45 s for one 2048 px graph. Only `plain_steel` was rendered there; the two layers were built with
`postprocess.py --only bulkhead trim`, the others are the committed layers.

**The O'Neill pair's two layers, 2026-10-14** (`openspec/changes/spacedock` design 5b, version E). The owner: "we
should have 2 layers of texture, detail layer, and the big albiedo color map, and maybe normal maps on the details".
These are exterior atlases and tiled detail maps, not layers of the texture array (no layer is added or renumbered):

| Texture (`assets/textures/oneill/`) | Graph (`build_ptex.py`) | Size | Use |
| --- | --- | ---: | --- |
| `oneill_macro` | `oneill_macro()` | 1024 px | Wrapped once round a cylinder: three hull strips, three window strips showing the land inside, orange bands, frames, km-scale grime; alpha is the glazing (the graph's emission) |
| `oneill_cap` | `oneill_cap()` | 512 px | Once over an end cap |
| `oneill_detail`, `oneill_detail_normal` | `oneill_detail()` | 512 px | 16 m of uneven hull plating, tiled, multiplying the macro; its tangent-space normal map (OpenGL, green up) |
| `oneill_mirror`, `oneill_mirror_normal` | `oneill_mirror()` | 256 px | Mirror facets, tiled, and their normal map |

10,485,752 bytes with mips (10.0 MiB). Rendered by Material Maker `174f15e` on the Raspberry Pi 5 (as plain steel
above), packaged by `oneill_atlases.py` (resizing and channel packing only; deterministic, prints a digest) with its own
contact sheet, `docs/screenshots/materials/oneill-contact-sheet.png`. The detail normal map is for the per-pixel
lighting the spacedock design proposes for station exteriors, to be measured on the Pi before the engine takes it.

```sh
python3 tools/materials/build_ptex.py      # writes ptex/oneill_*.ptex (restore tern_hull*.ptex if it rewrites them)
MM_DIR=/tmp/material-maker GODOT=/path/to/Godot_v4.7-stable_linux.arm64 \
  bash .claude/skills/material-maker-headless/scripts/render.sh -o tools/materials/raw/oneill --size 2048 tools/materials/ptex/oneill_*.ptex
python3 tools/materials/oneill_atlases.py
```

**Wall panels** (`openspec/changes/wall-panels`, a prototype) are not Material Maker graphs: they are
modelled and baked in Blender by `tools/blender/build_wall_panels.py` from
`data/materials/panels.json`, which renders into `raw/panels/` and reuses this directory's
`postprocess.py` for palette reduction and sizes. How panels are made from now on is the owner's
question V2.

## How the 2026-10-05 layers were made

Material Maker was not run in the Claude Code cloud session that built the first set (it was
thought to need a GPU, and release downloads were blocked then). The layers were built with
`--from-fps /home/user/fps-game-demo` at fps-game-demo revision `f6cd25c`: its committed
`game/textures/<graph>.png`, `_normal`, `_orm` and `_emission` maps, which are Material Maker
renders of the same `.ptex` files (graphs and exports landed together in fps commit `b832b84` and
have not changed since), rendered at 2048 px and downsampled to 1024 px by fps's own
`postprocess.py`. A fresh render through Material Maker starts from 2048 px and gives slightly
different layers (a different digest); that is expected, and the contact sheet is the check.

On 2026-10-09 Material Maker ran headless in a cloud session (`material-maker-headless` skill:
Godot 4.7 with Mesa's software Vulkan under Xvfb). Its 2048 px renders of `tech_panel` and
`light_panel`, downsampled to 1024, match fps-game-demo's committed exports exactly. Layers rebuilt
straight from those 2048 px renders differ from the committed ones by 0.2 to 1.9 of 255 on
average; that rebuild was measured and not committed. Moving the layers onto Material Maker's own
render is a commit of its own, with the contact sheet.

## Provenance

The Tern hull and fittings use the built-in imagegen finish `sources/tern-clean-finish.png`,
with its exact edit prompt beside it. The later `sources/tern-flat-painted-cyan.png` and its
prompt provide pigment grain and scuffs for the flat central band. The three Material Maker
graphs apply Copper, Cyan and Rescue paint to the same projected mask while retaining variation. The source keeps only a cyan fitting guide outside
the main hull islands, recolored by the graph. The calm white armor surrounds a narrow grey
equator with small pipes. No illustrated windows cover the genuine apertures. Cyan's stable
graph and delivered texture filenames retain the `cobalt` suffix.

`sources/tern-uv-wireframe.png` and its mask come from actual mesh UV polygons in
`sources/tern-uv-layout.json`, rasterized by `exterior_uv_guide.py`. The hull's dorsal UV orientation
makes lettering readable with the bow at image top. `ptex/tern_hull.ptex`, `tern_hull_cobalt.ptex`
and `tern_hull_rescue.ptex` share one parameterized graph builder and render at 1024 px in Material
Maker. `exterior_decals.py` preserves their RGB and sets emission alpha to zero, so blue paint
cannot glow. Warp strips have a separate emissive material. Actual apertures and pale frames
are geometry. Earlier paintings and prompts remain as design history; the owner's Fallen Tribes
reference textures were never changed.

The exterior design also uses original generated assembly artwork: `sources/exterior-surfaces.png`,
with its complete built-in image_gen prompt in `sources/exterior-surfaces-prompt.txt`. The owner's
spacecraft reference supplied the surface-detail direction. `ptex/exterior_surfaces.ptex` imports
that bitmap into Material Maker; `export_graph.py exterior_surfaces --material-maker PATH` renders
the actual graph. `exterior_decals.py` preserves the rendered RGB and packages cyan window emission
in alpha as `assets/textures/exterior_surfaces.png`. This fitted 2048 px decal/assembly atlas has
four longitudinal regions: dorsal armour, ventral armour, side windows and mechanical trim. It is
not a tiling texture-array layer. The ship builder fits its UVs and transfers the design into each
LOD atlas. No procedural Python substitute paints its artwork. The existing 128 px pearl and
graphite layers still supply small accent and hazard surfaces.

Copied from **fps-game-demo** (Undercity/Brushfire), revision **`f6cd25c`**, on 2026-10-05.

| Here | Source in fps-game-demo | How the graph was made | Changes here |
| --- | --- | --- | --- |
| `ptex/tech_panel.ptex` | `tools/material_maker/ptex/tech_panel.ptex` | Authored by its `build_ptex.py` | None (byte for byte). Since 2026-10-14 it makes only the exterior `hull` and `hull_dark`; no interior surface takes its grid |
| `ptex/floor_tiles.ptex` | `tools/material_maker/ptex/floor_tiles.ptex` | Authored by its `build_ptex.py` | None |
| `ptex/ceiling_tiles.ptex` | `tools/material_maker/ptex/ceiling_tiles.ptex` | Authored by its `build_ptex.py` | None |
| `ptex/hazard_stripes.ptex` | `tools/material_maker/ptex/hazard_stripes.ptex` | Authored by its `build_ptex.py` | None |
| `ptex/light_panel.ptex` | `tools/material_maker/ptex/light_panel.ptex` | Authored by its `build_ptex.py` | None |
| `ptex/rust_metal.ptex` | `tools/material_maker/ptex/rust_metal.ptex` | Authored by its `build_ptex.py` | None |
| `ptex/crate.ptex` | `tools/material_maker/ptex/crate.ptex` | Authored by its `build_ptex.py` | None |
| `ptex/diamond_plate.ptex` | `tools/material_maker/ptex/diamond_plate.ptex` | Material Maker's example `metal_pattern` ([Material Maker](https://github.com/RodZill4/material-maker), MIT, Rodolphe Suescun and contributors), with fps's patch wiring metallic to the rust mask (`bf_metallic`) | None |
| `build_ptex.py` | `tools/material_maker/build_ptex.py` | | Restricted to the graphs above: drops `concrete` and the `lava` and `brick_wall` patches; writes into `tools/materials/ptex/`. Running it reproduces the fps graphs byte for byte. |
| `export_materials.sh` | `tools/material_maker/export_materials.sh` | | Star Crew paths, graphs listed from `materials.json`, `--from-fps`, `raw/` cleared first. |
| `postprocess.py` | `tools/material_maker/postprocess.py` | | Rewritten for Star Crew: the raw naming, Lanczos downsample, normal renormalisation and luminance ramp come from fps; the relief bake, layer fitting, palette reduction, emission alpha, validation and contact sheet are new. No Godot output. |

Every source material has its `.ptex` here; none exists only as an export. fps-game-demo's
other graphs (concrete, brick wall, stone blocks, lava) and its procedural city materials are not
used by Star Crew and were not copied.

Historical connected secondary structure: `tern-connected-cyan-prompt.txt` adds aligned roof-to-wall returns and a dedicated bow-face strip against the actual UV guide. `tern-connected-refinement-prompt.txt` reduces the scale of the new conduits. The final `tern-armor-coverage-prompt.txt` restores broad ivory armor between slimmer grey connections. Copper and Rescue are color-only edits of that final Cyan atlas, each with its own saved prompt. That revision imported `tern-connected-{livery}.png`; all three 1024 px layers retain zero emission.

The Cyan ribbon returns to the earlier `tern-service-cyan.png` shade, with median
paint RGB 2 / 167 / 217. `pigment_reference_srgb` records the source painting color
separately from the chosen livery, so normalization cannot cancel a color change.
The ribbon border follows the same 3D field as the flat band and wraps onto walls.
