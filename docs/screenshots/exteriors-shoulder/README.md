# Blunt shoulder channel ends

The dark exposure beside the registration now ends at the existing 2 m and 11 m
station joints. Matching white armor replaces both pointed slivers. The pipes
between them, cyan ribbon, equator and nacelle finish remain in place.

| View | Before | Finished |
| --- | --- | --- |
| Port above | [Pointed ends](before-cobalt-0-port-above.png) | [Blunt ends](cobalt-0-port-above.png) |
| Starboard above | [Pointed ends](before-cobalt-0-starboard-above.png) | [Blunt ends](cobalt-0-starboard-above.png) |
| Port side | | [Finished](cobalt-0-port-side.png) |
| Starboard side | | [Finished](cobalt-0-starboard-side.png) |

The remaining `copper-*`, `cobalt-*` and `rescue-*` images show both sides at full,
medium and distant detail. Twenty-two captures include the two controlled before
views, using the old texture on identical geometry. Open the
[interactive preview](../../mockups/ship-exteriors.html) to orbit the ship.

The correction is projected on the actual 3D hull before baking the
[UV mask](../../../tools/materials/sources/tern-shoulder-armor-mask.png).
The [editable Blender source](../../../assets/models/exteriors/shoulder-armor-projection.blend)
retains that projection. Native Material Maker graphs reuse approved generated
armor, then apply the existing projected joints and ribbon.

Validation loads all nine exported GLBs and their embedded textures. Its 360
surface samples check white armor at both ends and retained mechanics between
them. The previous pointed model is rejected. Every livery and LOD reproduces
byte for byte. The separate hull projection check retains aligned seams, flat
central ribbons, deeper cyan and the white outline.

All 460 window/frame rays pass per livery. The full ship remains 4,440 triangles
and seven material draws; medium and distant remain 1,290 / 496 triangles with
one draw each. The complete preview remains 82,992 triangles, 51 draws and
88.83 MiB of textures. No runtime textures or geometry were added. These are
asset counts, not Pi timing measurements. Eight `sc-tools` tests pass, including
loading and compiling the delivered default hull. See [validation.json](validation.json).
