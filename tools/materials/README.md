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
| `postprocess.py` | Recolour, relief bake, fit to the layer, palette reduction; writes the layers, the contact sheet (`docs/screenshots/materials/contact-sheet.png`) and a table of sizes, texel densities, bytes and seam ratios. Deterministic: the last line is a digest. |

```sh
python3 tools/materials/build_ptex.py                                   # graphs (only when one changed)
MATERIAL_MAKER_DIR=/path/to/material-maker tools/materials/export_materials.sh  # render + build
tools/materials/export_materials.sh --from-fps /path/to/fps-game-demo   # no Material Maker: fps's exports
python3 tools/materials/postprocess.py                                  # rebuild from raw/ after a knob change
```

Needs Python 3 with numpy and Pillow (`pip install numpy pillow`).

**Wall panels** (`openspec/changes/wall-panels`, a prototype) are not Material Maker graphs: they are
modelled and baked in Blender by `tools/blender/build_wall_panels.py` from
`data/materials/panels.json`, which renders into `raw/panels/` and reuses this directory's
`postprocess.py` for palette reduction and sizes. How panels are made from now on is the owner's
question V2.

## How the 2026-10-05 layers were made

Material Maker could not run in the Claude Code cloud session that built the first set (it needs
Godot 4.7 and a GPU, and release downloads are blocked there). The layers were built with
`--from-fps /home/user/fps-game-demo` at fps-game-demo revision `f6cd25c`: its committed
`game/textures/<graph>.png`, `_normal`, `_orm` and `_emission` maps, which are Material Maker
renders of the same `.ptex` files (graphs and exports landed together in fps commit `b832b84` and
have not changed since), rendered at 2048 px and downsampled to 1024 px by fps's own
`postprocess.py`. A fresh render through Material Maker starts from 2048 px and gives slightly
different layers (a different digest); that is expected, and the contact sheet is the check.

## Provenance

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
