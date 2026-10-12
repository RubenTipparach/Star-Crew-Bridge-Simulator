# Detailed Grabon hull assets

Shrike, Vark and Gorath have purple armor, orange livery and a central impulse
exhaust above the hangar. Gorath's impulse is centered at x=0 m, y=5.2 m.
Shrike's projecting aft boxes are removed; Vark has its own visible central
exhaust. Geometry and UVs mirror about the keel. Open the
[interactive review](../../../docs/mockups/grabon-ships.html) for their textured
interiors and working elevator previews.

Each ship has an exterior `.glb`, an editable `.blend` with the enhanced atlas
packed, and a `-projection.blend` retaining the original assemblies, UVs and
projected paint. The finished `.blend` also retains named CSG cutters and the
revised assemblies. The GLBs use four unlit materials, the original albedo atlas,
the shared 128 px graphite texture and fixed sun vertex lighting. Recessed
1.10 m belts expose pipes and junctions, joined to roof channels by continuous
elbows. They contain the exterior; the interior geometry is generated
from [the detailed layouts](../../../data/ships/grabon-detailed/layouts.json)
by the shared ShipKit builders in the HTML review.

| Hull | Triangles | Enhanced atlas |
|---|---:|---:|
| Shrike | 9,728 | 1,254 x 1,254 px |
| Vark | 12,328 | 1,254 x 1,254 px |
| Gorath | 14,564 | 1,254 x 1,254 px |

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

For an exterior revision using the committed projection and enhanced paint,
run only `--finish`, then inline the preview. The finish pass reads
`data/ships/grabon-detailed/exterior.json`, restores the atlas UVs onto editable
assemblies, cuts the machinery openings, fits central impulse units and mirrors
the port geometry and UVs onto starboard. Retained aft engine cleats move 0.10 m
clear of the bay-door trim. The [texture sources and prompt](../../textures/grabon/GENERATION.md)
include projection PNGs, material masks, actual UV wireframes and enhanced atlases.
