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

## Custom exterior paint, 2026-10-10

The original [paint bitmap](sources/exterior-paint.png) was generated with the built-in image_gen tool. Its [complete prompt](sources/exterior-paint-prompt.txt) is committed beside it. The [exterior paint graph](ptex/exterior_paint.ptex) imports that bitmap as its editable albedo input. It supplies pearl and graphite ramps without a panel grid. Material Maker 1.4 rendered the graph at 2048 px using the owner's NVIDIA GPU; the existing post-process produced the two 128 px layers at 32 px/m. Existing layer indices remain stable; the new layers are 11 and 12, costing 174,760 bytes with mipmaps.

```powershell
python tools/materials/build_ptex.py
python tools/materials/export_graph.py exterior_paint --material-maker C:/Users/santi/repos/material_maker_1_4_windows/material_maker.exe
python tools/materials/postprocess.py --only exterior_pearl exterior_graphite
```

`--only` rebuilds the selected layers and uses the other committed PNGs for the complete contact sheet. It does not require raw exports for unchanged graphs. Two builds of the custom paint produce digest `09732705c1517bd9443c10dc0680f04cea3bc983e69603f451c1cef034cf7809`. The Windows exporter emits legacy texture-format and UI-scale messages but produces a valid albedo and exits successfully. Fresh output staging prevents its overwrite-dialog crash; the wrapper refuses a missing export.

Lower-detail exterior atlases are separate Blender bakes of the full model, not repeated copies of the paint layer. Their source is [the exterior builder](../blender/build_ship_exteriors.py).

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

The Tern hull and fittings use three built-in imagegen paintings, `sources/tern-connected-copper.png`,
`tern-connected-cyan.png` and `tern-connected-rescue.png`. The initial connected Cyan edit is recorded
in `tern-connected-cyan-prompt.txt`, its finer details in `tern-connected-refinement-prompt.txt`, and
the restored broad armor panels in `tern-armor-coverage-prompt.txt`. Copper and Rescue are color edits
of that revised Cyan painting; their exact prompts use the matching `tern-connected-<livery>-prompt.txt`
paths. Cyan's stable graph and delivered texture filenames retain the `cobalt` suffix. All use angular
white armor over continuous grey channels containing pipes, couplings and machinery, plus flat painted registration, with no illustrated
windows. `sources/tern-uv-wireframe.png` and its mask come from actual mesh UV polygons in
`sources/tern-uv-layout.json`, rasterized by `exterior_uv_guide.py`. The hull's dorsal UV orientation
makes lettering readable with the bow at image top. `ptex/tern_hull.ptex`, `tern_hull_cobalt.ptex`
and `tern_hull_rescue.ptex` share one parameterized graph builder and render at 1024 px in Material
Maker. `exterior_decals.py` preserves their RGB and sets their emission alpha to zero, so blue
paint cannot glow. Warp strips have a separate emissive material. Actual apertures and pale frames
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
| `ptex/tech_panel.ptex` | `tools/material_maker/ptex/tech_panel.ptex` | Authored by its `build_ptex.py` | None (byte for byte) |
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

Connected secondary structure: `tern-connected-cyan-prompt.txt` adds aligned roof-to-wall returns and a dedicated bow-face strip against the actual UV guide. `tern-connected-refinement-prompt.txt` reduces the scale of the new conduits. The final `tern-armor-coverage-prompt.txt` restores broad ivory armor between slimmer grey connections. Copper and Rescue are color-only edits of that final Cyan atlas, each with its own saved prompt. The graph builder imports `tern-connected-{livery}.png`; all three 1024 px layers retain zero emission.
