# Exterior design fleet

The Tern uses broad white armor, a narrow grey equatorial split, and deeper cyan bands with fine white outlines and flat central joins that
connect across roofs, bevels and vertical walls. Recessed joints have light plate lips and localized wear. Open [the inspection page](../../../docs/mockups/ship-exteriors.html)
and select **Hull detail** for the equator, **Paint detail** for the flat band and finish, **Seam joins** for aligned linework, or a
window to look into the furnished decks.
Select **Drive detail** for the broad forward collector and dedicated nacelle paint,
or **Warp pylons** for the whole level assembly and its tapered rear end.
Cyan is the default; Copper and Rescue use the same projected layout. The 1 m equatorial band has a 0.04 m side recess and small pipes. White armor remains dominant.

The shoulder channel beside the registration now ends at panel joints instead of
pointed grey wedges. See the [before/after shoulder review](../../../docs/screenshots/exteriors-shoulder/README.md).
Its [editable 3D projection](shoulder-armor-projection.blend) produces an offline mask;
the existing Material Maker graphs reuse matching generated armor beneath it.

Open [fleet.blend](fleet.blend) for the editable Copper fleet, or the livery folders for their
packed Blender sources. All GLBs embed their textures. The engine's deck compiler now loads
the default Cyan full model; the HTML preview also exposes the other liveries and LODs.

| Model | Design | Triangles | Ceiling | Material draws |
| --- | --- | ---: | ---: | ---: |
| [tern.glb](tern.glb) | Player frigate, level swept pylons and 56 m nacelles | 4,440 | 12,000 | 7 |
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

The nacelles now use a dedicated neutral [1024 x 2048 atlas](../../textures/tern_nacelle.png),
shared across both sides and all liveries. Its [generated artwork](../../../tools/materials/sources/tern-nacelle-finish.png)
was made with built-in imagegen using the [saved prompt](../../../tools/materials/sources/tern-nacelle-prompt.txt).
The [native graph](../../../tools/materials/ptex/tern_nacelle.ptex) renders at 2048 px square;
packaging resamples the normalized UV domain to the rectangular texture and sets emission to zero.
The source painting is 887 x 1774 px, about 30.4 original longitudinal px/m; the delivered UVs
allocate 35.11 px/m, versus about 3.8 in the old small hull-fitting patch.

[`project_nacelle_paint.py`](../../../tools/blender/project_nacelle_paint.py) intersects station
planes with the actual mesh before exporting the [projected layout](../../../tools/materials/sources/tern-nacelle-projected-layout.png)
and [UV wireframe](../../../tools/materials/sources/tern-nacelle-uv-wireframe.png). The shell wraps
continuously around its eight facets; support, rail and cap islands have dedicated artwork.
Forward width is 4.14 m, aft width 2.208 m. The long 11.76 m taper faces aft; the front chamfer
spans 3.36 m. Support sweep, 56 m bodies and the Y = 1.2 m centerline stay fixed.
The [original painting template](../../../tools/materials/sources/tern-nacelle-painting-template.png)
is retained separately; final cap UV packing removes its overlap with the rail.
See the [review captures and measurements](../../../docs/screenshots/exteriors-nacelles/README.md).

| Livery | Full model | Medium model | Distant model |
| --- | --- | --- | --- |
| Copper | [GLB](tern.glb) | [GLB](tern_lod1.glb) | [GLB](tern_lod2.glb) |
| Cyan (preferred) | [GLB](liveries/cobalt/tern.glb) | [GLB](liveries/cobalt/tern_lod1.glb) | [GLB](liveries/cobalt/tern_lod2.glb) |
| Rescue | [GLB](liveries/rescue/tern.glb) | [GLB](liveries/rescue/tern_lod1.glb) | [GLB](liveries/rescue/tern_lod2.glb) |

All three use the shared [integrated armor joints](../../../tools/materials/sources/tern-integrated-joints.png)
([exact built-in imagegen prompt](../../../tools/materials/sources/tern-integrated-joints-prompt.txt))
and the new [cyan pigment finish](../../../tools/materials/sources/tern-flat-painted-cyan.png),
with its [exact built-in imagegen prompt](../../../tools/materials/sources/tern-flat-painted-cyan-prompt.txt).
Material Maker carries its pigment variation into each selected color through the baked mask.

[`project_hull_paint.py`](../../../tools/blender/project_hull_paint.py) projects seams and livery
in ship-local metres on the actual model, then bakes them into UV space. The band field is
Z - 0.6 max(|X|, 4.5) + 0.3 Y. Each band has a flat 9 m center with parallel edges, then
oblique shoulders continuing onto both sides without per-face offsets.
The [projection data](../../../data/ships/tern/hull_paint_projection.json) owns their positions,
the 0.14 m seams and paint colors. All walls share one height-based UV mapping.

Retained authoring artifacts:

- [Editable projection scene](hull-paint-projection.blend), with the actual hull and shader.
- [Unenhanced layout](../../../tools/materials/sources/tern-projected-layout.png) and
  [region masks](../../../tools/materials/sources/tern-projected-masks.png), baked at 2048 px.
  Mask R is livery, G is seams, B is projected hull coverage.
- [3D layout inspection](../../../docs/screenshots/exteriors-flat-band-layout/ship-exteriors-livery-cobalt-0-service.png),
  captured before enhancement, with windows and stepped wall transitions visible.
- Supporting [UV wireframe](../../../tools/materials/sources/tern-uv-wireframe.png),
  [island mask](../../../tools/materials/sources/tern-uv-mask.png) and
  [UV polygons](../../../tools/materials/sources/tern-uv-layout.json).
- [Projection check](../../../tools/materials/sources/tern-projection-validation.json) and
  [enhancement check](../../../tools/materials/sources/tern-enhancement-validation.json).

Built-in imagegen cleans the ivory material finish on the inspected projection. Material Maker
then applies generated pigment variation inside the exact mask, preserving dark machinery
and putting the projected seams over the painted panels. Both roof bands have a flat join. The source
is 1254 px; the graph renders at 1024 px. `tern-seamless-finish.png` is the retained seam-free
input for projection, and `tern-clean-finish.png` is the neutral armor finish. The cyan pigment image enhances the new
flat-band projection; it is never fed back into the projector. This prevents a
self-dependent bake. Registration is painted directly on the hull with no raised plate.
There are no illustrated windows. Earlier paintings and prompts remain as design history.

The [Material Maker graph](../../../tools/materials/ptex/tern_hull.ptex) and its Cyan/Rescue
variants use one parameterized builder, rendered at 1024 px in Material Maker 1.4. Packaging
preserves RGB and sets emission alpha to zero. The stable `cobalt` ID and filenames contain
Cyan. Warp strips use a separate emissive material. Full-detail fixed sunlight is baked into
vertex color; materials use `KHR_materials_unlit`, with no normal maps or runtime PBR lighting.

Each livery has distinct 1024 px medium and 512 px distant atlases, baked from its finished full
model at twice the final resolution and reduced in linear light. They preserve paint, windows,
registration and machinery at 8.94 / 4.33 px/m. Temporary dark window backings exist during the
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
returns measure 0.551-0.560 m. All ten Deck B windows share a center 1.50 m above the floor,
with 1.70 x 0.80 m openings, sills at 1.10 m and heads at 1.90 m. The quarters, mess, medbay
and damage-control rooms use the same authored row. Interior cameras target the real windows.
Compare the [port](../../../docs/screenshots/exteriors-window-row/port.png) and
[starboard](../../../docs/screenshots/exteriors-window-row/starboard.png) rows.

The [window design](../../../data/ships/tern/exterior_windows_design.json) owns shared deck rows, stations and
42 furnishings. Its [generator](../../../tools/exterior_windows.py) validates the
[preview patch](../../../data/ships/tern/exterior_windows.json) against layout and prop bounds.
Eighteen rooms and corridors are shown, with deck selectors, window views, cutaways and
normal/red-alert lighting. Actual portal corners join the inner skin through combined liners
and transparent glass batches. No opaque window pictures cover the openings.

## Budget and validation

The restored narrow service recess and pipes add 608 full-detail triangles. Tern now uses
4,440 / 1,290 / 496 triangles and 7 / 1 / 1 material draws. The fleet with furnished interiors
uses 82,992 triangles in 51 draws. Decoded textures with mipmaps use 88.83 MiB against the
96 MiB scene ceiling; the exterior library alone is 68.50 MiB. These are asset counts, not
Pi frame-time measurements. Runtime rotation will need an adaptation of the fixed sun bake.
The engine's existing square-array format expands the nacelle texture to 2048 x 2048 with
exact pixel replication during compilation. The full Cyan hull's runtime maps therefore
cost 26.83 MiB, separate from the preview's all-livery allocation. Normalized UVs stay intact.
Warp animation, damage meshes, turret articulation, attachment collision and weapon/shield
occlusion updates remain separate engine work.

## Rebuild

Use Blender 4.4.3 with the dependencies described by the exterior builder, plus Material Maker
1.4, Python with Pillow/NumPy/fontTools/Brotli, and Node with Playwright/Chromium. Rebuild the
projection when editing its authored layout or receiving hull geometry, and inspect it before enhancing its texture:

```powershell
blender -b --factory-startup --python-use-system-env -P tools/blender/project_hull_paint.py
```

The committed generated paintings are the final art inputs. For existing artwork:

```powershell
python tools/materials/build_ptex.py
python tools/materials/export_graph.py tern_hull tern_hull_cobalt tern_hull_rescue --material-maker C:/Users/santi/repos/material_maker_1_4_windows/material_maker.exe --size 1024
python tools/materials/export_graph.py tern_nacelle --material-maker C:/Users/santi/repos/material_maker_1_4_windows/material_maker.exe --size 2048
python -c "from tools.materials.exterior_decals import pack_surfaces; pack_surfaces('tern_nacelle',2048,False,(1024,2048))"
python -c "from tools.materials.exterior_decals import pack_surfaces; [pack_surfaces(n,1024,False) for n in ['tern_hull','tern_hull_cobalt','tern_hull_rescue']]"
python tools/materials/check_hull_projection.py
python tools/exterior_windows.py --check
foreach ($livery in @("copper", "cobalt", "rescue")) {
    blender -b --factory-startup --python-use-system-env -P tools/blender/build_ship_exteriors.py -- --livery $livery
    blender -b --factory-startup --python-use-system-env -P tools/blender/build_ship_exteriors.py -- --livery $livery --check
}
python tools/materials/exterior_uv_guide.py
blender -b --factory-startup --python-use-system-env -P tools/blender/check_exterior_nacelles.py --
python tools/mockups/inline.py docs/mockups/ship-exteriors.html
node tools/mockups/shoot.mjs docs/mockups/ship-exteriors.html
node tools/mockups/zfight.mjs docs/mockups/ship-exteriors.html --setup "window.EXTERIOR_VIEW.show('fleet')"
```

LOD chart packing is cached under ignored `build/exterior-tools/uv-cache/`, keyed by exact
geometry, Blender version, packer source and settings. Every build rebakes artwork and emission.
The manifests record hashes, bounds, source assemblies, atlas density and actual costs. Another
Blender version can change export bytes and requires review. The repository-wide inliner still
reports the pre-existing deck-plan light-bake cache hash mismatch; this page does not use it.


## Connected paint validation, 2026-10-10

The final 3D projection agrees with 440 actual surface samples. Each packed livery has
25,681 sampled stripe-interior texels with 100% paint coverage and no hull emission.
The seam check reads 64 receiving-face profiles at 32 shared edges; the largest paired
center offset is 0.045 m, within the documented texel tolerance. The old committed texture
fails this guard. Material Maker preserves the generated finish outside the authored masks.

All eleven GLBs, six LOD atlases and three manifests reproduce byte for byte. Tern uses
4,442 / 1,290 / 496 triangles and 6 / 1 / 1 material draws. The narrow equator adds 608
full-detail triangles. The fleet with furnished interiors uses 83,092 triangles in 50 draws,
with unchanged 78.17 MiB decoded textures. No Pi performance measurement is claimed.

The preview passes 460 opening/frame rays per livery, twenty window menu entries, three
cutaways and all nine livery/LOD combinations with distinct reduced textures. Reviewed
57 desktop captures and the phone default: 45 livery/LOD views, six mess-window angles,
two mess lighting states, side, pylons, the dedicated seam view and initial Cyan.
The overlap check finds no exterior overlap; the existing shared room railing/baseboard
pairs total 0.046 m2. The window patch, sixteen inline blocks and budget marker match their
sources. OpenSpec passes all 42 items.

Full / medium / distant GLB bytes: Copper 1,642,780 / 1,357,520 / 362,220;
Cyan 1,641,148 / 1,356,536 / 361,880; Rescue 1,642,024 / 1,357,628 / 362,372.

## Flat band and paint finish correction, 2026-10-10

The previous pointed join and uniform RGB coat were rejected. The revised projection has
a 9 m flat central bridge with parallel edges and oblique outer shoulders. Imagegen enhanced
the inspected layout into `tern-flat-painted-cyan.png`. Material Maker retains its pigment
variation, keeps dark mechanical relief, and restores projected panel seams through the coat.
The new Paint detail view frames the forward join; desktop controls still occupy separate rows.

Ten traces on the actual upper hull verify the two baked flat joins. The previous arrow mask
fails the same check. The projection agrees with 440 surface samples. All 17,278 sampled armor
paint texels retain their livery color; seams and dark machinery are checked separately.
The 5th-to-95th percentile channel ranges are 24 / 22 / 20 levels of 255 for Cyan / Copper /
Rescue, rejecting the old uniform fill. All eighty seam profiles at forty shared edges pass,
including seams within the painted regions; the largest paired offset remains 0.045 m.
Material Maker RGB is preserved and hull emission is zero.

All three livery sets, their LOD atlases and manifests reproduce byte for byte. Geometry and
budget remain 4,442 / 1,290 / 496 triangles, 6 / 1 / 1 material draws and 78.17 MiB preview
textures. The fleet with furnished interiors remains 83,092 triangles in 50 draws. Geometry
is unchanged, so the preceding overlap result still applies. No Pi measurement is claimed.
The window patch and 460 opening/frame rays pass, as do all twenty window menu entries,
three cutaways, nine livery/LOD selections, distinct LOD textures and separate control rows.
OpenSpec passes all 42 items and all sixteen exterior inline blocks match their sources.

Full / medium / distant GLB bytes: Copper 1,703,472 / 1,392,612 / 370,412;
Cyan 1,706,380 / 1,395,084 / 370,656; Rescue 1,703,972 / 1,394,100 / 370,996.

Reviewed fifty desktop captures and the phone default, including forty-five livery/LOD views,
the new Paint detail close-up, seams, side, pylons and initial Cyan. Paint detail and the Cyan
hull close-up retain true-color PNGs because palette reduction obscured their pigment variation;
the remaining captures use the normal screenshot palette compression.

## Integrated armor joints, 2026-10-10

The latest joint finish follows the owner reference: narrow recessed cores, ivory edge
lips and localized wear. Material Maker samples core variation from the generated
finish through the existing 3D mask and restores machinery from the seam-free source.
All liveries and both LOD atlases are rebuilt. The painted band shape, hull geometry,
windows and 78.17 MiB preview texture allocation remain unchanged.

Forty shared-edge crossings and twenty-eight finished armor profiles per livery pass.
The flat-seam regression is rejected; 1,584 core texels now carry varied material.
All model and atlas bytes reproduce. See the [design validation](../../../openspec/changes/ship-exteriors/design.md)
for the current results and delivered sizes.

## White ribbon outline and deeper cyan, 2026-10-10

The ribbon has a fine white painted line just inside both edges, projected on the
hull before applying the existing textured finish. The deeper Cyan matches the
earlier atlas median RGB 2 / 167 / 217. Roof, bevel and wall edges share the same
projection; integrated panel joints remain visible through the paint.

- [Projected outline before enhancement](../../../docs/screenshots/exteriors-ribbon-layout/README.md)
- [Offline outline mask](../../../tools/materials/sources/tern-projected-ribbon.png)
- [Finished paint close-up](../../../docs/screenshots/mockups/ship-exteriors-tern-paint.png)
