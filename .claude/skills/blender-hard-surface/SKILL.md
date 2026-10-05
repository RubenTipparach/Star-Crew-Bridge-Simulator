---
name: blender-hard-surface
description: Model Star Crew's low-poly hard-surface props in Blender the CSG way, scripted and headless. Block out from primitives (boxes, extruded profiles, convex hulls, a lathe), carve with named cutter objects through Boolean modifiers (Exact solver, materials transferred from the cutters), union the pieces, chamfer only the edges that catch light with a 1-segment Bevel, then clean (weld, degenerate and limited dissolve), triangulate, project UVs in metres the way shipkit's worldUv does, check (manifold, no coplanar overlaps, triangle budget) and export a byte-reproducible glb with a manifest. Use whenever making or changing a Blender prop or a console, desk, chair, panel or fixture model (tools/blender/build_bridge_props.py), or asking about hard surface modelling, booleans, CSG, cutters, bevels and chamfers, low poly props, triangle budgets, cleaning a boolean result, glb export or running Blender headless ("hard surface", "boolean", "CSG", "Blender prop", "console model", "chair model", "cutter", "bevel", "low poly prop", "glb export").
metadata:
  author: Star Crew (Claude Code)
  version: "1.0"
---

# Hard-surface props in Blender, the CSG way

The owner, 2026-10-05, sharing Star Trek bridge cutaways: "look at how to do hard surface
modeling in blender with booleans and CSG". This skill is how a Star Crew prop is made that way:
the bridge furniture in `assets/models/bridge/` is its worked example. CLAUDE.md section 9 has the
art rules it serves (low poly, textured, lit by the bake, meshes built by committed generators,
every asset under a triangle budget that the build enforces) and section 8 the no-z-fighting
rule. Rooms and levels are a different job: see "Props here, rooms in blender-csg-levels" below.

## The parts

| File | What it is |
| --- | --- |
| `tools/blender/build_bridge_props.py` | The generator and the kit: primitives, the `Prop` class (`cut`, `union`, `chamfer`, `recess`), `clean`, `check`, the glb export and its read-back. The script is the source; nothing else is. |
| `tools/blender/render_bridge_props.py` | Stills of the exported glbs (not the build's scene), one per prop plus a contact sheet. |
| `assets/models/bridge/<name>.glb` | One prop per file. |
| `assets/models/bridge/props.json` | The manifest (`starcrew.props/1`): per prop its file, dimensions, bounds, anchor, operator or seat points, screens, triangles against budget, materials, its CSG steps and the glb's sha256; the generator and Blender versions. Written by the build, never by hand. |
| `docs/screenshots/props/` | `<name>.png` and `contact-sheet.png`. Look at them before anyone else does. |

## Running it headless

Two ways, the same script:

```sh
# Blender as a Python module (pip install bpy; matches Blender 4.5 LTS):
python tools/blender/build_bridge_props.py [--check] [--only a,b] [--blend out.blend]
python tools/blender/render_bridge_props.py [--only a,b] [--samples 24] [--no-sheet]
# The Blender binary:
blender -b --factory-startup -P tools/blender/build_bridge_props.py -- [same options]
```

- **`import bpy` first.** With the pip module, `bmesh` and `mathutils` do not exist until `bpy`
  is imported; `import bmesh` on line one fails with ModuleNotFoundError.
- **Arguments.** Under the binary, the script's options follow `--`; as a module they are plain
  `sys.argv`. Both scripts accept both.
- **Start empty.** `bpy.ops.wm.read_factory_settings(use_empty=True)`, so no user preference or
  startup file changes the result.
- **Rendering needs no GPU, but only Cycles works.** Workbench and EEVEE need an OpenGL context
  (`libEGL.so.1`); in a cloud session the process dies without one. Cycles on the CPU renders,
  denoising works, and Freestyle draws the edge lines (a new line set in an empty scene has no
  line style: create one). About 40 s a prop at 1024 x 768 and 24 samples on four cores.
- **`--blend` saves the scene** with every cutter collection, to look at the CSG in Blender. It
  is a debugging view: the steps are already applied and the file is not reproducible.

## Booleans or extrusion?

| The shape | Make it with | Why |
| --- | --- | --- |
| A body whose side view says it all (a console cabinet, a desk, an arm, a fin) | One extruded profile (`prism`), or a box carved by one convex profile cutter | One profile is one face loop: the cheapest correct mesh, and every corner is placed by a number |
| A recess (a screen, an access panel, a button band) or a hole (a fin's truss) | A cutter box or prism, Boolean Difference | The recess walls and floor come with their materials, and the panel around it is split exactly once |
| Pieces that meet (a pedestal under a desk, a back on a seat, fins on a cabinet) | Boolean Union of overlapping solids | Removes the hidden faces and leaves one closed solid; a plain join keeps every buried face and its triangles |
| A curve (the helm arc) | `lathe`: the profile swept in N steps | The profile carries the toe kicks, slope and rim in one go, and N sets the facet count directly. Measured: an Exact cylinder minus cylinder cut to a wedge gives the same 68 triangles and no slivers, but needs a 54-sided cylinder and one cutter per detail |
| A tapered or leaning block (a pedestal, a column, a hood) | `hull` of its corner points | Planar faces by construction |

Booleans earn their place where a shape is cut into another or two shapes meet. Do not use them
to make what one profile describes: every Boolean splits the faces it touches.

## The cutter workflow

Every prop follows the same order. Each step is applied at once (the evaluated mesh replaces the
object's mesh, `apply_modifiers`), because the next step selects real edges and faces.

1. **Block out** in prop space (below) with `prism`, `obox`, `hull`, `ngon`, `lathe`. Every
   primitive carries all the material slots in one fixed order (`ROLES`), and each face its role.
2. **Carve** with named cutters: `p.cut(body, "carve", [cutter, ...])` puts them in a collection
   `<prop>.carve` under `<prop>.cutters` (wire display, hidden in renders) and adds one Boolean
   modifier: operation Difference, operand type Collection, solver Exact, materials Transfer. The
   faces a cutter leaves behind take the cutter's own face roles, so a cutter's profile edges say
   what each new surface is (desk top `trim`, button band `accent`, display wall `machinery`).
3. **Chamfer** the silhouette edges that should catch light (next section), before anything
   else meets them.
4. **Make the other pieces the same way** (a fin is blocked out, chamfered, then pierced), and
   **union** them in: `p.union(body, "fins", [fin_l, fin_r])`.
5. **Cut the screens last** with `p.recess(...)`: a box whose floor, 12-20 mm behind the panel,
   becomes the `screen` face and whose sides become the recess walls. Last, so no bevel and no
   union ever touches a recess edge.
6. **Clean, check, finish, export, read back, render, look** (sections below).

Rules that keep the Exact solver's result clean (they are the level kit's, at prop scale):

- **No cutter face lies on a face it cuts.** A recess cutter reaches 5 cm out of its panel; a
  carve cutter runs 5 cm past the body on every side it crosses; a union piece overlaps 1 cm into
  what it joins (the fins start 1 cm inside the cabinet's ends).
- **No coplanar faces between union inputs either.** The headrest band's top sits 1 cm above the
  back's top; the fins stand 6 cm proud of the desk front rather than flush with it.
- **A hole must stay clear of the body behind it.** The fins' truss holes are confined to the
  region in front of the screen panel and above the desk, computed by clipping (`clip_polygon`),
  then inset by half a web (`triangle_inset`); a hole that overlapped the cabinet would show the
  cabinet's end face through it.
- **Name every cutter** `<prop>.<what>` and every collection `<prop>.<step>`: the `--blend` view
  and the manifest's `csg` list then read as the history of the prop.

## Chamfers that cost few triangles

- **Chamfer only the edges that catch light**: the desk's front edge, a fin's front and top edges,
  a plinth's or a foot's top edges, a seat cushion's front edge. `p.chamfer(ob, label, width,
  where)` marks the convex edges whose prop-space midpoint, direction and face normals `where`
  accepts with the `bevel_weight_edge` attribute, then applies a Bevel modifier: segments 1,
  limit method Weight, clamp overlap on, outer miter Sharp. A chamfer that matches no edge stops
  the build (a predicate that silently matches nothing is a chamfer you think you have).
- **Widths** 12-25 mm: 20 mm on a desk edge, 15 mm on fins and housings. Narrower vanishes at
  64 px per metre; wider reads as a rounded toy.
- **What one costs** (measured). A chamfer along an edge adds a 2-triangle strip; where it ends
  against an unchamfered face, that face gains a vertex (one more triangle). A closed loop has no
  ends: a hexagonal foot's six top edges cost exactly 12 (20 to 32). A box chamfered on all
  twelve edges is 44 triangles instead of 12.
- **A blanket Bevel explodes the count.** Measured on the finished props, one Angle-limited
  (30 degree) Bevel over everything at the end: wall_bank_core 304 to 792 triangles,
  crew_chair 154 to 470, standup_console 80 to 248. It also bevels every recess and hole edge.
- **A chamfer drawn into a profile** (an extra vertex in a `prism` profile) costs the same strip
  with no corner patches: use it where the edge is part of the profile anyway (the light strip's
  45 degree face is a profile cut, not a bevel).

## Cleaning after the booleans

`clean(ob)` runs, in this order:

1. Merge by distance, 0.1 mm (`deckc` welds at the same distance).
2. Dissolve degenerate (zero-length edges, zero-area faces), 0.1 mm.
3. Limited Dissolve, 1 degree, delimit Material: the Exact solver leaves coplanar faces split
   along its cut lines; this merges them back into one face per plane and material, and drops
   vertices left in the middle of straight edges. Delimit Material keeps a screen a separate face.
4. Delete loose vertices; triangulate (Beauty for quads and n-gons); dissolve degenerate again
   and re-triangulate anything it merged; recalculate normals outward.

Then `check(prop)` refuses the prop when it finds: an edge without exactly two faces or a
non-manifold vertex, a degenerate triangle, two triangles facing the same way within 1 cm of each
other's plane that overlap (coplanar z-fighting, CLAUDE.md 8), an upward face within 1 cm of the
floor, or (a wall bank) a face toward the room within 1 cm of the wall plane. The check is pinned:
a join of two overlapping boxes is caught (coplanar overlaps), an open box is caught (4
non-manifold edges), and the same boxes through an Exact union pass with 12 triangles.

## Triangle budgets

The budgets came with the props' brief (2026-10-05), inside bridge-stations section 12's
allowance (700 for a desk with its screen, 300 for a seat). The build refuses a prop over its
budget, naming it and its count, and writes nothing at all.

| Prop | Size (x, y, z) m | Triangles | Budget |
| --- | --- | ---: | ---: |
| `wall_bank_core` | 1.64 x 2.22 x 0.66 (desk 1.40) | 304 | 420 |
| `wall_bank` | 1.34 x 2.22 x 0.66 (desk 1.10) | 304 | 420 |
| `wall_bank_double` | 2.64 x 2.22 x 0.66 (desk 2.40, two seats) | 368 | 700 |
| `free_console` | 1.40 x 1.23 x 0.64 | 184 | 420 |
| `helm_arc` | 2.57 x 1.02 x 0.82 (2.4 m arc at mid depth) | 296 | 600 |
| `captain_chair` | 0.86 x 1.18 x 0.78 | 258 | 300 |
| `crew_chair` | 0.55 x 1.04 x 0.67 | 154 | 160 |
| `standup_console` | 0.64 x 1.16 x 0.52 | 80 | 160 |

Where the triangles go, and what to do about it:

- **Holes, not pieces.** A union or a recess punches a hole in the face it passes through, and a
  face with a hole triangulates to (outer + hole) triangles instead of (outer - 2). On the crew
  chair the back shell's front face, holed by its cushion and crossed by the headrest band, is 15
  triangles; the foot's top and the seat's underside, holed by the column, are 12 and 10.
- **Separating parts can cost more, not less.** Stopping the seat cushion short of the backrest
  (so the back no longer notched it) took the crew chair from 158 to 162: the strip of seat shell
  now visible between them was a new face with two holes. Count, don't guess: `read_glb` and a
  per-plane tally show which faces are expensive.
- **Cut the cheapest detail first**: side chamfers before front chamfers, a chamfer before a
  cushion. The crew chair kept its cushions and lost the cushion's side chamfers (158 to 154).

## The export conventions

| Convention | What the build does |
| --- | --- |
| Units | Metres |
| Prop space | The exported glTF frame: +Y up, +Z toward the operator (out of a console's working face; for a chair, the way the sitter faces), +X the operator's right. Every primitive is made in prop space and turned into Blender's Z-up frame as it is made (`PROP_TO_BLENDER`: x, y, z to x, -z, y); the glTF exporter's +Y Up turns it back. The read-back proves it: the floor is at y = 0, a wall bank's back at z = 0, and every screen faces +Z or up. |
| Origin | On the floor at the centre of the prop's back: the wall plane for a wall bank, the back of the pedestal for everything else |
| Materials | One per role: `machinery`, `trim`, `bulkhead`, `hazard`, `light_panel` (keys of `data/materials/materials.json`, checked at build time), `screen` (emissive, coloured by the page) and `accent` (the station's role colour). The glb's base colours are each layer's mean texel colour (and shipkit's `screen` and `engineering` for the page-coloured two), so a viewer shows something that reads |
| UV0 | Metres, per face, exactly shipkit.js `worldUv`: (x, z) where the face normal's \|y\| > 0.75, else (the horizontal tangent (-n.z, n.x) dotted with (x, z), y). The page divides by the material's `span_m`. The exporter stores 1 - v, so the build writes (u, 1 - v) and the read-back checks the file's UVs against the formula |
| Shading | Flat: every face its own normal; the exporter splits the vertices |
| Topology | Triangulated, one closed manifold solid per prop. Faces against the floor or a wall are kept: `deckc` drops faces pressed against another surface (deck-pipeline section 5) |
| Back faces | `use_backface_culling` on every material, so the glb says `doubleSided: false`. Blender's default exports `true`, which forbids culling on a solid that never shows its inside |
| Export options | GLB, selection only, +Y up, UVs and normals on, tangents, vertex colours, attributes, extras, cameras, lights, animation, skins and morphs off, images none |

**Determinism.** Two builds in separate processes write identical bytes (sha256 compared on all
eight glbs and props.json). Nothing in the file varies: the exporter writes no images, no extras
and no timestamps, and its one free-text field is `asset.generator`, "Khronos glTF Blender I/O
v4.5.51", fixed per version. Everything that orders output is stable: prop order is the `PROPS`
table's, material slots are `ROLES` order, bmesh operators are deterministic. The manifest records
each glb's sha256 with the Blender and exporter versions; `--check` rebuilds in memory and fails
on any difference (so run it with the version the manifest names).

## How the kit and the mockups consume a prop

This is the contract the files are written to. `docs/mockups/bridge-variants.html` reads them
today: `python3 tools/mockups/inline.py` (its `models:bridge` block) copies props.json and every
glb, base64, into the page, so **re-run the inliner after a rebuild**, or `inline.py --check`
fails on the stale copy. No engine code loads them yet.

1. **Place it.** A layout station has `seat_m` (on the floor) and `yaw_deg`, its seat facing +Z
   at yaw 0. Every row's `operators_m` are floor points, so `seat_m` lands on one: a chair turns
   by `yaw_deg` (its `operators_m` is the floor under its seat), a console faces the seat and
   turns by `yaw_deg + 180`. A wall bank goes on the wall plane with its +Z into the room. The
   variants page reads `operators_m[0][2]` for consoles and chairs alike: keep the field on
   every row.
2. **Texture it** the way `shipkit.js` `geometryOf` does the room's own geometry: a primitive's
   material name gives the layer (`materials.json` `layer`) and the span (`span_m`), the surface
   UV is UV0 over the span, and the vertex colour is the bake. `screen` and `accent` are not
   layers: the page colours them. The variants page (`loadProps`, `placeProp`) takes UV0 as
   metres as it is, draws `screen` faces on the `light_panel` layer tinted by the station's role
   colour, and tints `accent` faces with it.
3. **Merge it** into its compartment's one draw, as the bridge mockup's `Props` do (one draw per
   compartment, deck-pipeline section 5); its triangles count against the compartment.
4. **Light it**: `deckc` and the mockup baker light its vertices like any other surface; the
   screens and light strips are emissive.

The main bridge mockup still builds its own box furniture (`stationProps` in
`docs/mockups/bridge.html`). Moving it onto these glbs is a mockup change of its own (the
`threejs-mockups` skill).

## Common failures

| You see | Cause | Fix |
| --- | --- | --- |
| `ModuleNotFoundError: No module named 'bmesh'` | Imported before `bpy` (pip module) | `import bpy` first |
| The render process dies, "Couldn't open libEGL.so.1" | Workbench or EEVEE in a session without OpenGL | Cycles on the CPU |
| `'NoneType' object has no attribute 'color'` setting Freestyle | An empty scene's line set has no line style | `ls.linestyle = bpy.data.linestyles.new(...)` |
| Slivers or a missing face after a Boolean | A cutter face on the face it cuts | Overlap 5 cm (cutters) or 1 cm (union pieces) |
| Flicker where two pieces meet | Coplanar faces facing the same way survived (a plain join, or a union of coplanar inputs) | Union, and move one input 1 cm; `check` names the pair |
| An end face showing through a hole | The hole overlaps the body behind it | Clip the hole region against the body first |
| The triangle count triples | A Bevel over every sharp edge, or after the recesses were cut | Chamfer by weight, before the screens |
| A screen floor split into many triangles | The recess crosses facets or other cuts | One recess per flat panel; on a facet the cutter floor must sit below the lowest facet (the helm arc's 2 cm clears its 6 mm) |
| `non-manifold edges` | An open primitive, or a Boolean on one | Build closed solids; never delete hidden faces by hand |
| A shading seam or smeared light across a hard edge | Smooth shading or shared normals | `use_smooth = False` on every face; the read-back checks every normal is its face's |
| Texture scale wrong in the page | UVs not in metres, or V unflipped | Write (u, 1 - v); the read-back compares the file with `worldUv` |
| `doubleSided: true` in the glb | Blender's material default | `use_backface_culling = True` |
| A different glb from the same script | Another Blender or exporter version | Compare against the versions in `props.json` |

## Checks before calling a prop done

| Check | How |
| --- | --- |
| Builds, inside its budget, manifold, no overlaps | `build_bridge_props.py` (it refuses otherwise, and writes nothing) |
| The file is what it says | The build's read-back: floor at y 0, back at z 0, flat normals, UV0 equals `worldUv`, screens face +Z, every manifest screen on a screen face |
| Reproducible | `build_bridge_props.py --check` |
| It reads | `render_bridge_props.py`, then look at `<name>.png` and the contact sheet yourself: crisp, chamfered, purposeful, in the cutaway references' style |
| Dashes | CLAUDE.md 5 grep |
| On the Pi | Not from here: a cloud render is not a Pi measurement (CLAUDE.md 2) |

## Props here, rooms in blender-csg-levels

`blender-csg-levels` is Undercity's level kit: a solid shell carved by room-shaped air cutters
(the boolean leaves the room), trims generated from the room list, `ENT_` empties, one live
Boolean evaluated at export, a Godot import. This skill builds things that stand in a room: solids
built up from blocks, carved and joined, every step applied, one glb per prop in Star Crew's frame
and materials. What they share: cutters carry their faces' materials (Transfer), nothing
coincides (overlap or shorten), the build refuses z-fighting rather than hoping, and you render
the exported file and look at it. A deck's rooms come from `deck-pipeline` and `detailing.json`,
not from either skill.

## References

- Blender 4.5 manual, Boolean modifier:
  https://docs.blender.org/manual/en/4.5/modeling/modifiers/generate/booleans.html (operations,
  Collection operands, solvers Float, Exact and Manifold, Materials Index Based and Transfer, Self
  Intersection and Hole Tolerant; "only manifold meshes are guaranteed to give proper results").
  The Python names are `solver` `FAST`, `EXACT`, `MANIFOLD`.
- Bevel modifier: https://docs.blender.org/manual/en/4.5/modeling/modifiers/generate/bevel.html
  (Limit Method Weight reads `bevel_weight_edge`; Segments; Clamp Overlap; Miter Outer).
- Limited Dissolve:
  https://docs.blender.org/manual/en/4.5/modeling/meshes/editing/mesh/delete.html (Max Angle,
  All Boundaries, Delimit).
- glTF 2.0 add-on: https://docs.blender.org/manual/en/4.5/addons/import_export/scene_gltf2.html
  (+Y Up; meshes are triangulated and flat-shaded edges split vertices; Apply Modifiers; custom
  properties go to `extras` only when included).
- `docs/mockups/lib/shipkit.js` `worldUv`, `geometryOf`; `data/materials/materials.json`;
  `openspec/changes/deck-pipeline/design.md` sections 5 and 5a; `openspec/changes/bridge-stations/design.md`
  section 11.1 (desk and screen sizes).
