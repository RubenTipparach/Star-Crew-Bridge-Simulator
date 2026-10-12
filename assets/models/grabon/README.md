# Detailed Grabon hull assets

Shrike, Vark and Gorath retain the approved purple armor, orange livery and hull
proportions. Gorath retains its centered engines. Open the
[interactive review](../../../docs/mockups/grabon-ships.html) for their textured
interiors and working elevator previews.

Each ship has an exterior `.glb`, an editable `.blend` with the enhanced atlas
packed, and a `-projection.blend` retaining the original assemblies, UVs and
projected paint. The GLBs use one unlit material, an albedo atlas and fixed sun
vertex lighting. They contain the exterior; the interior geometry is generated
from [the detailed layouts](../../../data/ships/grabon-detailed/layouts.json)
by the shared ShipKit builders in the HTML review.

| Hull | Triangles | Enhanced atlas |
|---|---:|---:|
| Shrike | 7,232 | 1,254 x 1,254 px |
| Vark | 7,360 | 1,254 x 1,254 px |
| Gorath | 8,312 | 1,254 x 1,254 px |

The manifest records file hashes, triangle ceilings and texture memory including
mips. These are review assets, pending owner approval before engine adoption.

Rebuild from the repository root:

```powershell
python tools/design/build_grabon_detail.py
node tools/mockups/export_grabon_hulls.cjs
blender -b --factory-startup -P tools/blender/build_grabon_detail.py -- --guide --review --maps
# Inspect the projection captures, then enhance each locked atlas.
# The existing enhanced images and exact generation prompt are committed.
blender -b --factory-startup -P tools/blender/build_grabon_detail.py -- --finish
python tools/mockups/inline.py docs/mockups/grabon-ships.html
```

The finish pass moves aft engine cleats 0.10 m clear of the bay-door trim while
retaining their UVs. The [texture sources and prompt](../../textures/grabon/GENERATION.md)
include projection PNGs, material masks, actual UV wireframes and enhanced atlases.
