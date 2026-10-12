# Detailed Grabon hull assets

Shrike, Vark and Gorath have purple armor, orange livery and a central impulse
exhaust above the hangar. Gorath's impulse is centered at x=0 m, y=5.2 m.
Shrike's projecting aft boxes are removed; Vark has its own visible central
exhaust. Geometry and UVs mirror about the keel. Open the
[interactive review](../../../docs/mockups/grabon-ships.html) for their textured
interiors, window close views and working elevator previews.

The hulls have real window openings aligned with the furnished decks: 16 on
Shrike, 14 on Vark and 20 on Gorath. Each row is centered 1.45 m above its floor,
with 0.66 m high glazing. The native finish pass joins the pressure skin, hollows
the rooms, reserves armored surrounds and cuts intersecting service hardware.
Frames follow the tapered hull; the manifest supplies measured corners for the
interior liners. Window rays check the delivered GLBs against the same portals.

Gorath's bottom deck is inset beneath the engineering tier: 20.16 x 28.24 m
against 25.6 x 32.1 m above. Its full-height rooms and two fighter berths fit
the smaller footprint; the centered impulse and deck access retain their positions.

Each ship has an exterior `.glb`, an editable `.blend` with the enhanced atlas
packed, and a `-projection.blend` retaining the original assemblies, UVs and
projected paint. The finished `.blend` also retains named CSG cutters and the
revised assemblies. The GLBs use four unlit materials, the original albedo atlas,
the shared 128 px graphite texture and fixed sun vertex lighting. Recessed
1.10 m belts expose pipes and junctions, joined to roof channels by continuous
elbows. They contain the exterior; the distinct Grabon interior geometry is generated
from [the detailed layouts](../../../data/ships/grabon-detailed/layouts.json)
by the shared ShipKit builders in the HTML review.

The interiors use [Grabon panel textures](../../textures/grabon-panels/README.md)
and [Grabon furniture](../grabon_furniture/README.md), modeled and Cycles-baked
through the repository's panel-textures and hard-surface workflows.

| Hull | Triangles | Enhanced atlas |
|---|---:|---:|
| Shrike | 10,870 | 1,254 x 1,254 px |
| Vark | 13,566 | 1,254 x 1,254 px |
| Gorath | 16,406 | 1,254 x 1,254 px |

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
clear of the bay-door trim. Gorath's lower armor and bay fittings use the same
footprint transform as its derived layout, retaining the original painted UVs.
Its enclosed lower-roof fittings are removed. The [texture sources and prompt](../../textures/grabon/GENERATION.md)
include projection PNGs, material masks, actual UV wireframes and enhanced atlases.
