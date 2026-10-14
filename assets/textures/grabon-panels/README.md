# Grabon interior panels

These are separate Grabon designs, modeled as relief in Blender and rendered
with the repository's `panel-textures` workflow. Interlocking plum armor,
diagonal bronze braces, guarded amber lamps, pipe wells and angular hatches
replace the Federation panel designs. Floors use broad tread ribs and small
route inlays; ceiling and trim layers use the same structural language.

[The catalogue](../../../data/materials/grabon-panels.json) inherits placement,
layer numbers, render settings and material wear from the shared panel rules.
[The designs](../../../tools/blender/grabon_panel_designs.py) use the existing
hard-surface kit. The Federation textures are unchanged.

Cycles renders beauty and emission at 512 px/m. The shared postprocessor
averages in linear light, reduces the palette to 48 colors and writes 256 px
and 128 px layers. Alpha stores emission, not transparency. The manifest
records pixel hashes, emission coverage, tiling seams and GPU memory.

Rebuild with Blender's Python dependencies from the shared panel workflow:

```powershell
blender -b --factory-startup --python-use-system-env --python-exit-code 1 -P tools/blender/build_wall_panels.py -- --catalog data/materials/grabon-panels.json
python tools/mockups/inline.py docs/mockups/grabon-ships.html
```

`--post-only` processes the cached EXRs. `--raw-dir <path>` gives a repeat build
an independent render cache. No normal maps or runtime PBR materials are used.

[Rendered contact sheet](../../../docs/screenshots/materials/grabon-panels-contact-sheet.png)
and [interactive ship review](../../../docs/mockups/grabon-ships.html).
