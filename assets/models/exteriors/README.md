# Exterior design fleet

The Tern uses broad white side plates with fine seams and cyan bands that wrap across roofs,
bevels and vertical walls. Open [the inspection page](../../../docs/mockups/ship-exteriors.html)
and select **Hull detail** to inspect those joins, or a window to look into the furnished decks.
Cyan is the default; Copper and Rescue use the same projected layout. The side service gaps,
pipes and brackets are removed. Dorsal and underside mechanical detail remains.

Open [fleet.blend](fleet.blend) for the editable Copper fleet, or the livery folders for their
packed Blender sources. All GLBs embed their textures. These are delivered art assets and a
design mockup; the Rust renderer does not load them yet.

| Model | Design | Triangles | Ceiling | Material draws |
| --- | --- | ---: | ---: | ---: |
| [tern.glb](tern.glb) | Player frigate, level swept pylons and 56 m nacelles | 3,834 | 12,000 | 6 |
| [tern_lod1.glb](tern_lod1.glb) | Medium detail, custom 1024 px atlas | 1,290 | 3,000 | 1 |
| [tern_lod2.glb](tern_lod2.glb) | Distant detail, custom 512 px atlas | 496 | 600 | 1 |
| [osprey.glb](osprey.glb) | Broad-bow courier, low drives, teal livery | 1,248 | 12,000 | 7 |
| [shrike.glb](shrike.glb) | Arrowhead raider, weapon shoulders, red livery | 1,164 | 12,000 | 7 |

The central hull is 79 m long, 81.47 m including fittings. Metres, +X port, +Y up, +Z bow;
the origin matches the layout's deck B origin. Pylon roots, tips and nacelle centers sit at
Y = 1.2 m. Deck floors remain +3.5 m, 0 m and -3.5 m. The lower bow follows the enlarged bridge
outline, with faceted corners and a 0.35 m rim bevel. The torpedo room, magazine and command
rooms fit inside the actual inner hull. Companion ships remain exterior concepts.

## Paint sources and liveries

| Livery | Full model | Medium model | Distant model | Painted source | Prompt |
| --- | --- | --- | --- | --- | --- |
| Copper | [GLB](tern.glb) | [GLB](tern_lod1.glb) | [GLB](tern_lod2.glb) | [PNG](../../../tools/materials/sources/tern-projected-copper.png) | [Prompt](../../../tools/materials/sources/tern-projected-copper-prompt.txt) |
| Cyan (preferred) | [GLB](liveries/cobalt/tern.glb) | [GLB](liveries/cobalt/tern_lod1.glb) | [GLB](liveries/cobalt/tern_lod2.glb) | [PNG](../../../tools/materials/sources/tern-projected-cyan.png) | [Prompt](../../../tools/materials/sources/tern-projected-cyan-prompt.txt) |
| Rescue | [GLB](liveries/rescue/tern.glb) | [GLB](liveries/rescue/tern_lod1.glb) | [GLB](liveries/rescue/tern_lod2.glb) | [PNG](../../../tools/materials/sources/tern-projected-rescue.png) | [Prompt](../../../tools/materials/sources/tern-projected-rescue-prompt.txt) |

[`project_hull_paint.py`](../../../tools/blender/project_hull_paint.py) projects seams and livery
in ship-local metres on the actual model, then bakes them into UV space. The band field is
Z - 0.6 |X| + 0.3 Y; two authored intervals cross both hull sides without per-face offsets.
The [projection data](../../../data/ships/tern/hull_paint_projection.json) owns their positions,
the 0.10 m seams and paint colors. All walls share one height-based UV mapping.

Retained authoring artifacts:

- [Editable projection scene](hull-paint-projection.blend), with the actual hull and shader.
- [Unenhanced layout](../../../tools/materials/sources/tern-projected-layout.png) and
  [region masks](../../../tools/materials/sources/tern-projected-masks.png), baked at 2048 px.
  Mask R is livery, G is seams, B is projected hull coverage.
- [3D layout inspection](../../../docs/screenshots/exteriors-projection-layout/ship-exteriors-livery-cobalt-0-service.png),
  captured before enhancement, with windows and stepped wall transitions visible.
- Supporting [UV wireframe](../../../tools/materials/sources/tern-uv-wireframe.png),
  [island mask](../../../tools/materials/sources/tern-uv-mask.png) and
  [UV polygons](../../../tools/materials/sources/tern-uv-layout.json).
- [Projection check](../../../tools/materials/sources/tern-projection-validation.json) and
  [enhancement check](../../../tools/materials/sources/tern-enhancement-validation.json).

Built-in imagegen adds the worn white paint finish while preserving the projected stripe and
seam layout. Copper and Rescue are color edits of Cyan. Each generated bitmap is 1254 px with
the same normalized layout. The old `tern-connected-cyan.png` is the retained source for roof,
underside and fittings before projection; do not overwrite it with the new finished painting.
Registration is painted directly on the hull, with no backing panel or raised plate. There are
no illustrated windows. Earlier paintings and prompts remain as design history.

The [Material Maker graph](../../../tools/materials/ptex/tern_hull.ptex) and its Cyan/Rescue
variants use one parameterized builder, rendered at 1024 px in Material Maker 1.4. Packaging
preserves RGB and sets emission alpha to zero. The stable `cobalt` ID and filenames contain
Cyan. Warp strips use a separate emissive material. Full-detail fixed sunlight is baked into
vertex color; materials use `KHR_materials_unlit`, with no normal maps or runtime PBR lighting.

Each livery has distinct 1024 px medium and 512 px distant atlases, baked from its finished full
model at twice the final resolution and reduced in linear light. They preserve paint, windows,
registration and machinery at 8.92 / 4.25 px/m. Temporary dark window backings exist during the
bake only. Embedded atlas pixels match the standalone PNGs. The liveries share triangle geometry,
normals, UVs and openings; vertex packing may differ with accent colors. Each has its own manifest,
packed Blender scene and atlases. The preview shares textures only when their actual hashes match.

Companions retain the original [surface atlas](../../textures/exterior_surfaces.png),
[source](../../../tools/materials/sources/exterior-surfaces.png),
[prompt](../../../tools/materials/sources/exterior-surfaces-prompt.txt) and
[graph](../../../tools/materials/ptex/exterior_surfaces.ptex). The original registration sheet
uses Barlow Semi Condensed under its [OFL license](../../fonts/barlow-semi-condensed/OFL.txt).
PNG alpha encodes emission, so ordinary image viewers can show transparent pixels; GLBs are opaque.

## Windows and preview

Twenty genuine apertures connect with the same metre-scale interiors shown in the preview:
four on A, ten on B and six on C. Pale 0.12 m frames surround the openings. The command shell is
0.22 m thick, with 0.10 m clearance to the room wall, giving 0.32 m window returns. Lower-deck
returns measure 0.551-0.560 m. Mess window centers are 1.50 m above the floor, with unchanged
1.70 x 0.80 m openings. The interior camera targets the authored middle window.

The [window design](../../../data/ships/tern/exterior_windows_design.json) owns stations and
42 furnishings. Its [generator](../../../tools/exterior_windows.py) validates the
[preview patch](../../../data/ships/tern/exterior_windows.json) against layout and prop bounds.
Eighteen rooms and corridors are shown, with deck selectors, window views, cutaways and
normal/red-alert lighting. Actual portal corners join the inner skin through combined liners
and transparent glass batches. No opaque window pictures cover the openings.

## Budget and validation

Removing the side service structures saves 990 full-detail triangles. Tern now uses
3,834 / 1,290 / 496 triangles and 6 / 1 / 1 material draws. The fleet with furnished interiors
uses 82,484 triangles in 50 draws. Decoded textures with mipmaps remain 78.17 MiB against the
96 MiB scene ceiling; the exterior library alone is 57.83 MiB. These are asset counts, not
Pi frame-time measurements. Runtime rotation will need an adaptation of the fixed sun bake.
Warp animation, damage meshes, turret articulation, attachment collision and weapon/shield
occlusion updates remain separate engine work.

## Rebuild

Use Blender 4.4.3 with the dependencies described by the exterior builder, plus Material Maker
1.4, Python with Pillow/NumPy/fontTools/Brotli, and Node with Playwright/Chromium. Rebuild the
projection only when editing its authored layout, and inspect it before enhancing its texture:

```powershell
blender -b --factory-startup --python-use-system-env -P tools/blender/project_hull_paint.py
```

The committed generated paintings are the final art inputs. For existing artwork:

```powershell
python tools/materials/build_ptex.py
python tools/materials/export_graph.py tern_hull tern_hull_cobalt tern_hull_rescue --material-maker C:/Users/santi/repos/material_maker_1_4_windows/material_maker.exe --size 1024
python -c "from tools.materials.exterior_decals import pack_surfaces; [pack_surfaces(n,1024,False) for n in ['tern_hull','tern_hull_cobalt','tern_hull_rescue']]"
python tools/materials/check_hull_projection.py
python tools/exterior_windows.py --check
foreach ($livery in @("copper", "cobalt", "rescue")) {
    blender -b --factory-startup --python-use-system-env -P tools/blender/build_ship_exteriors.py -- --livery $livery
    blender -b --factory-startup --python-use-system-env -P tools/blender/build_ship_exteriors.py -- --livery $livery --check
}
python tools/materials/exterior_uv_guide.py
python tools/mockups/inline.py docs/mockups/ship-exteriors.html
node tools/mockups/shoot.mjs docs/mockups/ship-exteriors.html
node tools/mockups/zfight.mjs docs/mockups/ship-exteriors.html --setup "window.EXTERIOR_VIEW.show('fleet')"
```

LOD chart packing is cached under ignored `build/exterior-tools/uv-cache/`, keyed by exact
geometry, Blender version, packer source and settings. Every build rebakes artwork and emission.
The manifests record hashes, bounds, source assemblies, atlas density and actual costs. Another
Blender version can change export bytes and requires review. The repository-wide inliner still
reports the pre-existing deck-plan light-bake cache hash mismatch; this page does not use it.


Validation of projected plating, 2026-10-10: the new hull uses 3,834 / 1,290 / 496 triangles
and 6 / 1 / 1 material draws. All eleven GLBs, six LOD atlases and three manifests reproduce
byte for byte. The projection bake agrees with 385 sampled hull positions, with zero mismatches.
All 27,718 stripe-interior pixels retain their livery color in each enhanced painting; boundary
joins were inspected in the 3D captures. Material Maker RGB is preserved with zero hull emission.
The original editable projection, its unenhanced bake and four pre-enhancement captures are retained.

The preview passes 460 opening/frame rays per livery, all twenty window menu entries, three
cutaways, nine livery/LOD selections, distinct LOD textures and separate control rows. Reviewed
47 desktop captures and the phone default: 36 livery/LOD views, six mess-window views, both
mess lighting states, side, pylons and default Cyan. The fleet with furnished interiors uses
82,484 triangles in 50 draws, with unchanged 78.17 MiB decoded textures. The overlap check
reports no exterior overlap; the existing shared room railing/baseboard overlap is 0.046 m2.
The window patch, all 16 preview inline blocks and its budget marker match their sources.
OpenSpec passes all 42 items. No Pi performance measurement is claimed.

Full / medium / distant GLB bytes: Copper 1,797,816 / 1,515,260 / 400,304;
Cyan 1,878,028 / 1,563,048 / 407,632; Rescue 1,789,944 / 1,508,900 / 399,652.
