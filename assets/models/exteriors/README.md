# Exterior design fleet

Open [fleet.blend](fleet.blend) to edit the named assemblies in Blender, or [the inspection page](../../../docs/mockups/ship-exteriors.html) to orbit the delivered models. Textures are embedded in every GLB and packed in the Blender file.

| Model | Design | Triangles | Ceiling | Material draws |
| --- | --- | ---: | ---: | ---: |
| [tern.glb](tern.glb) | Player frigate, level swept pylons and 56 m nacelles | 4,714 | 12,000 | 6 |
| [tern_lod1.glb](tern_lod1.glb) | Medium detail, custom 1024 px atlas | 1,290 | 3,000 | 1 |
| [tern_lod2.glb](tern_lod2.glb) | Distant detail, custom 512 px atlas | 496 | 600 | 1 |
| [osprey.glb](osprey.glb) | Broad-bow courier, low drives, teal livery | 1,248 | 12,000 | 7 |
| [shrike.glb](shrike.glb) | Arrowhead raider, weapon shoulders, red livery | 1,164 | 12,000 | 7 |

The Tern's central hull is 79 m long after rounding the bow. Its overall envelope, including fittings, is recorded in [props.json](props.json). Metres, +X port, +Y up, +Z bow. The origin is the layout's deck B origin, not a floor or centre-of-bounds pivot. All models export as one mesh split into material primitives. Source assemblies and named cutters stay editable in the Blender file. Companion ships are exterior concepts with no approved interiors or simulation data.

The Tern has three liveries, each with its own 1024 px hull/fittings atlas and freshly baked 1024/512 px LOD atlases. White armor sits over continuous grey mechanical channels. Two 51 m side service bands run between the lower window rows, replacing the disconnected patches. Their modeled pipes, clamps and equipment modules join the painted couplings, valves and vents. The side channels and two retained dorsal breaks recess the full-detail skin by 0.08 m while retaining at least 0.14 m of closed pressure hull. The builder reads back the actual recess depth and remaining skin. The Tern registration plate is removed: SCS TERN and SC-084 are painted directly into the hull texture. The accepted silhouette, pale window frames, level pylons and 56 m nacelles are retained.

| Livery | Full model | Medium model | Distant model | Painted source | Prompt |
| --- | --- | --- | --- | --- | --- |
| Copper | [GLB](tern.glb) | [GLB](tern_lod1.glb) | [GLB](tern_lod2.glb) | [PNG](../../../tools/materials/sources/tern-service-copper.png) | [Prompt](../../../tools/materials/sources/tern-service-copper-prompt.txt) |
| Cyan (preferred) | [GLB](liveries/cobalt/tern.glb) | [GLB](liveries/cobalt/tern_lod1.glb) | [GLB](liveries/cobalt/tern_lod2.glb) | [PNG](../../../tools/materials/sources/tern-service-cyan.png) | [Prompt](../../../tools/materials/sources/tern-service-cyan-prompt.txt) |
| Rescue | [GLB](liveries/rescue/tern.glb) | [GLB](liveries/rescue/tern_lod1.glb) | [GLB](liveries/rescue/tern_lod2.glb) | [PNG](../../../tools/materials/sources/tern-service-rescue.png) | [Prompt](../../../tools/materials/sources/tern-service-rescue-prompt.txt) |

The liveries share identical triangle geometry, normals, UVs, window openings and triangle counts. Vertex packing can differ because the baked accent colors affect vertex sharing. Each directory retains its own manifest, editable Blender source and `atlases/` PNGs. The inspection page opens in Cyan and offers Copper, Cyan and Rescue buttons, plus **Subhull** for the service-channel close-up and **Lettering** for the painted name. LOD textures are shared only when their actual baked hashes match.

The lower bow follows the actual bridge polygon, scaled 1.5 across its width and 2.0 along its forward half around Z = 26.2 m. Its faceted curved shoulders end at Z = 37 m with a 0.35 m rim bevel. All interior coordinates stay fixed. Actual inner/outer hull containment checks include the torpedo room and magazine. The hull painting stays anchored to its original longitudinal UV range. Select **Bow detail** in the inspection page for a close view.

Editable sources: [UV wireframe](../../../tools/materials/sources/tern-uv-wireframe.png), [island mask](../../../tools/materials/sources/tern-uv-mask.png), [UV polygons](../../../tools/materials/sources/tern-uv-layout.json), and [Material Maker graph](../../../tools/materials/ptex/tern_hull.ptex). The built-in imagegen tool created the three paintings above against the actual 2048 px guide; each 1254 px bitmap uses normalized island placement. Material Maker renders the delivered hull texture at 1024 px. The cyan and rescue graphs use the same graph builder with their own source image. The shared [UV coverage repair prompt](../../../tools/materials/sources/tern-service-coverage-prompt.txt) extends ivory into the side island margins. UV anchors align the painted channel to both physical recess edges. Bow faces sample the solid ivory part of their island. A targeted built-in imagegen edit changes the cobalt markings to cyan blue. The source and exact edit prompt are linked in the livery table. The stable `cobalt` ID and filenames now contain Cyan, selected by `default_livery` in the root manifest. Earlier paintings remain as design history.

Companion ships retain the shared 2048 px [surface atlas](../../textures/exterior_surfaces.png), [source](../../../tools/materials/sources/exterior-surfaces.png), [prompt](../../../tools/materials/sources/exterior-surfaces-prompt.txt), and [graph](../../../tools/materials/ptex/exterior_surfaces.ptex). Small accents and hazards use the [pearl](../../textures/exterior_pearl.png) and [graphite](../../textures/exterior_graphite.png) layers. Exterior materials contain no tiled square grid.

PNG alpha is the engine's emission mask, including cyan window detail, so an ordinary image viewer may display textures as transparent. GLB materials are opaque and preserve RGB. The [registration sheet](../../textures/exterior_decals.png) uses the repository's Barlow Semi Condensed font, under its [OFL licence](../../fonts/barlow-semi-condensed/OFL.txt). Full-detail colours and fixed sun shading are baked into `COLOR_0`; `KHR_materials_unlit` avoids double lighting. No normal maps or PBR lighting are needed. Twenty genuine openings match the portals from the command-suite and lower-deck window patches. Their dimensions and hull intersections are recorded in the manifest. The HTML preview shows furnished rooms on all three decks, bridge crew, transparent glass and passages between each interior window and the hull opening. Its window, interior and cutaway controls let you inspect the same rooms from both sides. Both normal and red-alert lighting are available.

The [medium atlas](atlases/tern_lod1.png) and [distant atlas](atlases/tern_lod2.png) bake the full-detail ship onto each lower mesh. Armour panels, service recesses, windows, radiators, bay doors, registration, copper markings and warp strips survive reduced geometry. Both are baked at twice their final width and height and reduced in linear light to limit grain. Their final densities are 8.92 and 4.25 px/m, intended for smaller distant views. Each has its own emission mask in alpha. Fixed shading is in the atlas, with neutral vertex colours to avoid double shading. Read-back confirms the GLB's embedded RGBA pixels exactly match the standalone atlas.

The exterior library with all three liveries uses 60,642,636 bytes of RGBA8 textures with mipmaps (57.83 MiB). With the furnished rooms and corridors across all decks, the preview loads about 78.17 MiB. The visible fleet plus interior draws 83,072 triangles in 50 calls, below the provisional scene ceilings. GLBs carry standalone image copies; the preview shares identical textures. Pylon roots, tips and nacelle centres are at Y = 1.2 m. The three deck floor heights remain +3.5 m, 0 m and -3.5 m; the preview uses those same deck floors and existing furniture dimensions.

To regenerate with Blender 4.4.3:

```powershell
python tools/materials/build_ptex.py
python tools/materials/export_graph.py exterior_paint exterior_surfaces --material-maker C:/Users/santi/repos/material_maker_1_4_windows/material_maker.exe
python tools/materials/export_graph.py tern_hull tern_hull_cobalt tern_hull_rescue --material-maker C:/Users/santi/repos/material_maker_1_4_windows/material_maker.exe --size 1024
python tools/materials/postprocess.py --only exterior_pearl exterior_graphite
python tools/materials/exterior_decals.py
python tools/exterior_windows.py
python tools/exterior_windows.py --check
foreach ($livery in @("copper", "cobalt", "rescue")) {
    blender -b --factory-startup -P tools/blender/build_ship_exteriors.py -- --livery $livery
    blender -b --factory-startup -P tools/blender/build_ship_exteriors.py -- --livery $livery --check
}
python tools/materials/exterior_uv_guide.py
python tools/mockups/inline.py docs/mockups/ship-exteriors.html
node tools/mockups/shoot.mjs docs/mockups/ship-exteriors.html
node tools/mockups/zfight.mjs docs/mockups/ship-exteriors.html --setup "window.EXTERIOR_VIEW.show('fleet')"
```

The decal author needs Pillow, NumPy, fontTools and Brotli support. Material export needs Material Maker 1.4 or a compatible installed application. LOD chart packing is cached under ignored `build/exterior-tools/uv-cache/`; the key includes exact geometry, Blender version, packer source and packing settings. Every build still bakes the current artwork and emission into fresh LOD atlases. A clean build reconstructs the chart placement through the shared packer. Cached rebuilds skip packing; a clean pack takes several minutes on this PC. The mockup tools need Playwright and Chromium. `props.json` records file hashes, bounds, source assemblies, atlas density, emission coverage and actual costs. Rebuilding with another Blender version may change export bytes and must be reviewed.

These are delivered art assets and a design mockup. The Rust runtime does not load them yet. Warp animation, damage states, turret articulation, attachment collisions and shield/weapon occlusion updates belong to the separate engine implementation. The vertex sun bake is a fixed presentation light and must be replaced or adapted when ships rotate at runtime. No Pi performance measurement is claimed.

Window correction: the side UV island contains solid plating, with no illustrated window slots. Each current livery prompt explicitly excludes illustrated windows; the full-detail openings remain geometry. Twenty 0.12 m pale-silver rims follow the actual portal/hull intersections. The preview checks their dedicated rim material along every frame edge and includes angled window captures.

Shallow-window correction: the forward upper hull now follows the existing bridge, ready-room and briefing-room outlines, joined to the rear by a 3 m sloped transition and finished with a 0.16 m roof chamfer. Every command window is 0.32 m from its room wall, replacing the 4.178 m bridge and 1.841 m side-room recesses. The shell remains 0.22 m thick, with 0.10 m of interior clearance. Cut walls use graphite, and the preview liners join the exact portal and inner-skin corners. The builder checks room containment and continuous UVs across coplanar painted faces; the preview checks aperture corners as well as the 460 opening/frame rays. Both LODs and their textures follow the revised hull.

Windows on all decks: the Tern has 20 real openings, four on A, ten on B and six on C. The additional B/C room walls follow the existing hull with at least 0.50 m clearance; measured window returns are 0.551 to 0.560 m. The [design recipe](../../../data/ships/tern/exterior_windows_design.json) owns window stations and 42 furnishings. [The generator](../../../tools/exterior_windows.py) derives and validates the [preview patch](../../../data/ships/tern/exterior_windows.json) against the source layout and actual prop bounds. The preview shows 18 rooms and corridors, with deck selectors, a window menu, cutaways and inside views. Shared materials and two combined liner/glass batches keep the scene at 78.17 MiB for all liveries and 50 fleet-plus-interior draws. The side navigation lamps move to Z = 19 m to clear the new deck B apertures. Extra windows are baked into the existing LOD meshes and atlas sizes.

The global inliner still reports the pre-existing deck-plan light-bake cache hash mismatch; this page does not use that cache.

During LOD baking only, temporary graphite surfaces close the openings behind the window rims. This gives bake rays a dark window surface and prevents frame-coloured fill. These surfaces are removed before export; full-detail windows remain open into the preview interiors. The builder checks all 20 window centres against the actual packed UVs in each reduced atlas.

Validation after the continuous service-channel revision: all eleven GLBs, six LOD atlases and three manifests reproduce byte for byte. The preview passes 460 opening/frame rays per livery, all deck/window controls, all nine livery/LOD selections and distinct reduced textures. Triangle geometry, normals and UVs match across all three liveries. Control rows do not overlap at desktop width. The overlap check reports no new exterior overlap; shared room railing/baseboard overlap remains 0.046 m2. The actual room corners fit the hull; OpenSpec passes all 42 items. The page's own inline blocks and budget marker match their sources. Desktop and phone captures cover all window close-ups, all decks, both lighting states, three liveries at all three LODs, lettering, the revised bow and Subhull close-ups.

Cyan revision: the preferred livery is cyan blue, generated with built-in imagegen from the retained cobalt source. Matching cyan fittings and both LOD atlases were rebuilt. Cyan GLBs are 1,667,280 / 1,413,852 / 380,276 bytes, with unchanged 4,714 / 1,290 / 496 triangles and 6 / 1 / 1 draws. Cyan and the updated root catalog pass byte-for-byte rebuild checks. Browser checks pass all nine livery/LOD selections, 460 window rays, deck controls and the default Cyan selection. Sixteen desktop captures and one phone capture were inspected. Material Maker RGB is preserved exactly and painted cyan has zero emission. Texture allocation remains 78.17 MiB including interiors.
