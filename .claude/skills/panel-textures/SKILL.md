---
name: panel-textures
description: Make, place and judge Star Crew's panel textures, the owner's "textures skill" - every wall, ceiling, floor and trim texture of a ship's interior. Designed panels (vents, pipes, hatches, screens, keypads, light columns, grates, light channels, steel members) modelled as low relief with the blender-hard-surface kit, baked in Blender under a fixed light into layers of the one texture array (128 px per metre or 64), the emission mask in alpha, and placed by stable rules - wall bays and bands, ceiling and floor cells with a walkway, trim strips along members, ribs as pillars with a base and a capital - so neighbours differ and nothing is a grid of one tile. Data in data/materials/panels.json, the build in tools/blender/build_wall_panels.py, the placement in docs/mockups/lib/shipkit.js (opts.panels). Use whenever making or changing a wall, ceiling, floor or trim texture, adding a panel module, a ceiling or floor module or a trim strip, tuning panel colours, wear, weights or light, judging the panel contact sheet, turning panels on in a mockup, or asking why a wall, ceiling, floor or trim looks flat, tiled, repetitive or wrong ("wall texture", "ceiling texture", "floor texture", "trim", "panel module", "new panel", "walkway", "pillar", "the walls look tiled", "the ceiling looks awkward", "contact sheet", "panels.json").
metadata:
  author: Star Crew (Claude Code)
  version: "1.0"
---

# Panel textures: walls, ceilings, floors and trims

The owner, 2026-10-05, on the wall panel prototype: "Ooh now that is good. Walls and ceilings still
look awkward update those as well using your new textures skill", then "Floors and ceilings in mean.
Wall panels look great..some wall pillars might need retouching up too". This is that skill. The
rules it serves are CLAUDE.md section 9 ("Walls, ceilings and trims are panels, modelled and baked in
Blender"; Material Maker stays the source of tiling materials such as the hull) and section 8 (no
z-fighting, rooms follow the hull). The designs are three OpenSpec changes:

| Change | What it decides |
| --- | --- |
| `openspec/changes/wall-panels` | Bays at the ribs, bands from the floor, ten wall modules per finish, the FNV-1a rule, bay-local coordinates, the four strips |
| `openspec/changes/ceilings-and-trims` | Ceiling cells and their eight modules, the lamp cells, trim strips along members, ribs as pillars with a base and a capital |
| `openspec/changes/floor-panels` | Floor cells, the walkway, the floor modules, the runner retired |

Read the change before changing what it decides. This skill is the how-to: it never overrides them.

## The parts

| File | What it is |
| --- | --- |
| `data/materials/panels.json` | The one source: sizes, rules, colours, wear, render light, every module's layer, weight and placement, the trim rows and members. Its `_rules` say what each field means; units are in the keys |
| `tools/blender/build_wall_panels.py` | The build: models every module, strip and trim row with the hard-surface kit, bakes them in Cycles, post-processes them into layers, writes the manifest and the contact sheet. Validates `panels.json` first and stops on any unknown or missing key |
| `tools/blender/build_bridge_props.py` | The hard-surface kit the build imports (`prism`, `obox`, `Prop.cut`, `Prop.union`, `apply_modifiers`): never copy it |
| `tools/materials/postprocess.py` | The materials' post-process, whose `reduce_palette`, `gpu_bytes`, `seam_ratio` and `save_png` the build imports |
| `assets/textures/panels/<px>/<stem>.png` | The layers, at 256 px (128 px per metre over 2 m) and 128 px (64 px per metre). Stems: `<finish>_<module>`, `<finish>_strips`, `<finish>_ceiling_<module>`, `<finish>_floor_<module>`, `<finish>_trims` |
| `assets/textures/panels/manifest.json` | Written by the build, never by hand: every file's sha256 and bytes, the layers' glow and seam ratios, GPU bytes, Blender, Pillow and numpy versions |
| `docs/screenshots/materials/panels-contact-sheet.png` | The contact sheet: look at it before anyone else does |
| `docs/mockups/lib/shipkit.js` | The placement: `dressWall`, `dressFlat` (ceilings and floors), `walkwayPaths`, `stripBox` and `coveStrip` (trims), `loadPanels`, behind `opts.panels` |
| `tools/mockups/kit_report.mjs --panels` | Counts what the dressing costs, per compartment: triangles today and dressed, bays, cells, walkway cells, pillars, and the modules the rules chose |
| `docs/mockups/wall-panels.html` | The comparison page: today against panels, both densities, three lighting states, close views, the layers |

## The system

Everything is a 2 m layer (`layers.span_m`) of the deck's one texture array, after the eleven
materials. A layer either shows once (a module) or tiles along one axis (a strip or a trim row).

**Walls** (wall-panels design 1-5). A wall face is cut into **bays** at its ribs, stiffeners,
corners, openings and wall fixtures; a bay over 2.4 m is split. From the floor up it is banded: a
0.5 m base strip, a 2 m module band, then for each further whole 2.5 m a louvre strip and another
module band, then the top strip to the cove. Each bay cell takes one module, centred, with
bay-local coordinates. A bay 1.6-2.4 m wide takes a full module, 0.6-1.6 m the narrow one, under
0.6 m plate. Rule order: opening or fixture in the cell, plate; beside a door, flank (mirrored to the
door's side); narrow; otherwise drawn by weight.

**Ceilings and floors** (ceilings-and-trims design 1, floor-panels design 1). A bay is the strip
between two frames (2 m). Each bay is cut into 2 m **cells** centred on x = 0, so the middle cell of
a room on the centreline is centred on it. Coordinates are cell-local (u along +x, v along +z, the
bow), so a cut only crops. Rule order, per cell:

1. A ceiling cell a lamp housing spans takes `lamp_surround`, a light channel across the whole cell
   with v centred on the lamp, so the housing sits in it wherever the lamp rule put it.
2. A cell holding a floor portal (its hole plus its collar or rim) takes `plate`.
3. A floor cell the walkway band crosses takes `walkway`.
4. A cell whose core (`cells.core_m`, 0.6 m either side of its centre) the room's outline or a hole
   cuts takes `plate`: a cove cropping 0.35 m off a cell against a wall only crops its margin.
5. Otherwise a module is drawn by weight.

**The walkway** (floor-panels design 1): a band 0.9 m wide from each door (a wall portal of
`rule.door_kinds`) to the centroid of the brushes on its floor; in a corridor, down its long axis,
with each door joining it square. The walkway module tiles both ways, so a path reads as one path.
Where floors are dressed the corridor runner is not drawn.

**Trims** (ceilings-and-trims design 3-4). Ribs, beams, coves, baseboards, door and window frames
keep their geometry; only their texture changes. Each finish has one trim layer of seven rows
(`trims.rows`): `side`, `baseboard`, `rib`, `rib_ends`, `beam`, `frame`, `cove`. A member's front
face takes its member's row, its long sides the `sides` row, its ends `side`. u runs along the
member, world-projected, so a 13 m beam tiles with no seam; v runs across the face inside the row,
stretched at most 1.6 times, and a deeper face (a tall room's 0.6 m cove) repeats the row whole.

**Pillars.** A rib is the wall's pillar (Undercity: "Pillars always have a base and a capital"). Its
front face is split into a base (0.3 m, 0.6 m in a room over 3.6 m), the tiling shaft and a capital
under the cove (`trims.pillar`). Base and capital are pieces of the `rib_ends` row (`trims.pieces`):
0.45 m of row each, centred at -0.5 m and +0.5 m, mapped onto the member's base or capital. It costs
4 triangles a rib.

**The rule.** One rule for everything drawn: FNV-1a 32 of a key, divided by 2^32, onto the cumulative
weights of the drawable modules in layer order, minus the excluded ones. Keys:
`seed|compartment|brush|edge|bay|level` for a wall cell, `seed|compartment|brush|ceiling|bay|cell`
(or `floor`) for a cell, bay and cell being world frame and cell indices. Excluded: a wall bay's
neighbour on the face and the cell below; a cell's left neighbour and the cell in the same place in
the bay before. The key holds nothing from any other wall or brush, so an edit stays local. Weights
are data: change them without rebuilding a texture.

**Two finishes.** `crew` (warm mid grey, clean; light, quiet ceilings; a lighter walkway; dark
gunmetal trims) and `working` (dark grey-brown, rust and streaks, hazard borders and edges,
diamond-plate floors). A finish lists its modules; crew floors have no `hazard`. A kind (ceiling,
floor, trims) may override a finish's colours and has its own wear.

## Designing a module from the references

The references are `docs/analysis/texture-references.md` (X1 Elite Force, X2 Alien Resurrection, X3
the wall sheet, X4 Quake, Halo, UT). Take the shape, never the art (CLAUDE.md 15).

1. **Say what it is in one line** (the module tables in the designs): "a round fan in a square frame",
   not "some detail".
2. **Keep the frame.** A module is 2 m x 2 m in panel space (x right, y up, z out of the surface; a
   wall module's y is up, a ceiling or floor module's y is the bow). Features stay 0.2 m inside its
   edges (`cells.margin_m`), the main one inside 0.6 m of the centre (`cells.core_m`), except runs that
   cross a ceiling cell from beam to beam (trays, pipes), whose ends the beams hide.
3. **Read at 64 px per metre.** A texel there is 1.6 cm: a feature under about 3 cm vanishes. The
   crew walkway's first studs (3.4 cm, 7 mm high) disappeared; its dashes are 7 cm.
4. **One seam per joint.** Ceiling and floor modules cut a groove on their right and top edges only
   (`cell_seams`), so each joint between two cells is one groove. The walkway has none: it tiles.
5. **Light is part of the panel.** Emissive roles (`light_panel`, `amber`, `accent`, `screen`) bake
   into the mask; a module that glows says `"emissive": true` and the build refuses a mismatch.
6. **Text reads right.** Stencils are allowed on walls and ceilings (a ceiling's u is +x, which reads
   unmirrored looking up with the bow at the top). Keep floors free of text.

## Modelling and baking it

Model with the hard-surface kit, the same CSG steps as a prop (the `blender-hard-surface` skill),
in `build_wall_panels.py`:

- `plate_slab(P)` is the backing plate; `P.cut(target, label, cutters)` carves recesses, grooves,
  slots and holes (Exact solver, materials transferred, so a recess floor comes out dark);
  `P.box`, `P.cyl`, `P.torus`, `P.cable`, `P.text`, `cplate` (a plate with chamfered corners),
  `frame_ring`, `ring`, `bolts`, `rivet_row`, `slats`, `keypad`, `octagon_grille` build the pieces.
- `studs(P, what, items, role)` makes hundreds of small raised bumps as one mesh without booleans
  (tread plate, anti-slip dashes); `tread(..., keep_out)` lays a diamond-plate lattice that repeats
  every 2 m, kept off the openings listed; `deck_plate(P, F, keep_out)` is the floor modules' deck
  (diamond plate in working spaces).
- Chamfer raised pieces (`bevel=` or `bevel_ob`): triangles do not matter in a texture, edges catching
  light do.
- A cutter reaches 5 cm out of the face it cuts (`P.recess` does it for you); never let a cutter face
  lie on the face it cuts.
- Roles are colours, not materials: `bulkhead` the plate's paint, `paint2` a second paint, `machinery`
  dark recesses, `trim` bare metal, `hazard` stripes, `walk` walkway plate, `rubber`, `stencil`,
  `safety`, and the emissive ones. `ROLE_COLOUR` maps them to `colours_srgb`.

Add the builder to `MODULE_BUILDERS` (walls), `CEILING_BUILDERS`, `FLOOR_BUILDERS` or
`TRIM_BUILDERS` (a row: `t_<row>(P, ox, h, F)`, built at x offsets -2, 0 and 2 so the render tiles,
returning cutters for the slab).

**The bake** (`render_target`): orthographic and face-on at 512 px per metre, Cycles on the CPU,
fixed seed, no denoiser, `render.samples` samples, two bounces, a sun key light and a uniform ambient
scaled so a flat unoccluded face renders at exactly its colour. Walls are lit from the upper left;
ceilings and floors nearly square on, as a room's lamps light them (`render.ceiling`,
`render.floor`); trims along +x, which is up a pillar, so a base's top chamfer catches the light and
a capital's underside is in shadow (`render.trims`). Wear is procedural and baked: grime, crevice
occlusion, edge wear, and for walls streaks and rust running down; ceilings, floors and trims get the
same in blotches and patches (`worn_blotchy`), since they do not hang. Strips and trim rows use noise
periodic in x; the walkway uses 4D noise on a torus, periodic in x and y. A second render with
emission only is the mask.

**The post-process** (`post`): area-average the render in linear light to 256 and 128 px, encode
sRGB, reduce to `layers.colours` (48) colours by median cut, write the mask into alpha. Trim rows
are stacked into one layer at their `v0_m`, each gap filled by repeating the nearest row's edge so a
mip only ever blends a row with its own edge.

```sh
PY=<python with the bpy 4.5 module and Pillow>
$PY tools/blender/build_wall_panels.py                         # everything (about 45 min on 4 cores)
$PY tools/blender/build_wall_panels.py --only crew_ceiling_fan,working_trims --samples 6   # a quick look
$PY tools/blender/build_wall_panels.py --post-only             # layers and sheet from the raw renders
```

`--only` names targets (`<finish>_<module>`, `<finish>_strips`, `<finish>_ceiling_<module>`,
`<finish>_floor_<module>`, `<finish>_trims`, `<finish>_ui`, `<finish>_keys`); the post-process still
writes every layer from the raw renders it finds in `tools/materials/raw/panels/` (gitignored), so
render everything once in a session before iterating on a few.

## Densities, mask and budget

| Item | Value |
| --- | --- |
| Layer size | 256 px at 128 px per metre (the mockups' setting, survey S1 open), 128 px at 64 |
| Layers | 55: walls 22 (10 modules and a strip layer a finish), ceilings 16, floors 15, trims 2 |
| GPU bytes with mips, panel layers | 19,223,820 at 256 px; 4,805,900 at 128 px |
| Whole array with the 11 materials | 23,068,584 bytes at 256 px (23.1 MB of the 96 MB budget); 5,767,080 at 128 px |
| Emission | Alpha: 255 shows the texel at full brightness whatever the light. `glow` scales it per lighting state (1.0, 1.0, 0.3 on emergency power) |
| Draw calls | None added: every layer is in the one array, so a compartment stays one draw |
| Triangles | `kit_report.mjs --panels` measures them; quote it, never estimate |

## The contact sheet, and how to judge it

`docs/screenshots/materials/panels-contact-sheet.png`, per finish: every wall module and the strip
layer, the screen and key images, an illustrative 12 m wall; every ceiling and floor module and the
trim layer; an illustrative 6 m x 8 m ceiling (cells, beams, lamps in their channels) and floor (a
walkway down the middle); and the trims as members (a crew pillar, engineering's 9.4 m pillar, a
beam, a cove, a baseboard, a door jamb). Read the PNG yourself, then judge:

- **Against the references, not the last version.** Elite Force, Alien Resurrection, Quake, UT: is
  every panel designed, is the relief deep enough to read, is light part of the panel?
- **Nothing is a uniform grid.** No module should dominate a room except the rest panels (plate,
  lamp channels where the lamp rule is dense, the walkway).
- **Ceilings quieter than walls** in crew spaces; **trims read as steel members** (flanges, rivet
  lines, lightening holes, cable trays), darker than the plating they frame; **pillars have a base
  and a capital** that read at the far end of engineering.
- **Floors**: the walkway lighter than the rest, grates not so many that the floor reads as one big
  grate.
- **Tiling**: strips, trim rows and the walkway have `seam_ratio` about 1 or under in the manifest
  (the colour step across the wrap against the mean step inside).

Then render the pages (`node tools/mockups/shoot.mjs docs/mockups/wall-panels.html --wait 300`) and
look at every shot: seams, stretched strips, cells cut badly at hull-following edges, z-fighting, a
ceiling too busy.

## Adding a module to panels.json

1. Add its builder (above) and its row to the finish's `modules`, `ceiling.modules` or
   `floor.modules`: `layer`, `weight` (0 for a module a rule places), `placed` (`draw` or
   `rule:<what>`), `emissive`.
2. Renumber: panel layers run from `layers.first_layer` (the number of materials) without gaps, once
   each. The build refuses anything else. Adding a layer in the middle moves every later layer;
   pages pick up the numbering from the data, but the manifest's digests all change.
3. Build it (`--only`), then `--post-only` for the sheet, look, then a full build before committing,
   then `python3 tools/mockups/inline.py` (the pages carry the layers inline) and the shots.
4. A new trim row: add it to `TRIM_ROWS`, `trims.rows` (`v0_m`, `h_m` whole multiples of 1/64 m, at
   least 2/64 m between rows) and `TRIM_BUILDERS`; members name rows in `trims.members`.

## Turning panels on in a page

- Inline the data: `<!-- INLINE panels BEGIN --><!-- INLINE panels END -->` after the materials
  block, then `python3 tools/mockups/inline.py`.
- `const pmats = await K.loadPanels(THREE, mats, { px: 256 })` (128 px per metre); build with
  `{ panels: true }`; `geometryOf` and `surfaceMaterial` with `pmats`; set
  `material.userData.panelGlow.value` to `panels.json` `glow[state]` when the lighting state changes.
- A dressed role carries a layer per vertex (`vlayer`): anything that filters, subdivides or adds to
  a part must carry it (`K.subdivideParts` does). A page that adds its own triangles to a kit role
  (a dais into `floor`) must use a role of its own (deck-plan's `fx_<role>`).
- Without the option a page gets exactly the geometry it always had (checked by hashing every
  compartment's parts with the old kit and the new).

## Pitfalls actually hit

| You see | Cause | Fix |
| --- | --- | --- |
| A ceiling of plain plate with a few modules | Lamp cells centred on lamps cropped every neighbour, and coves cropped every cell against a wall | Lamp cells are the grid cells a housing spans, with a channel that runs edge to edge; crop only when the core is cut |
| The walkway's anti-slip studs gone in the page | Features under about 3 cm vanish at 64 px per metre | Make them 7 cm |
| Trims as pale as the old tiles | The first trim colours were lighter than the walls | Dark gunmetal with bright worn edges |
| Pillar ends flat | Square-on light shades a step the same on both sides | Light the trims along +x (up a pillar) and make the pieces 7 cm proud |
| A floor of grates | Grates are the strongest module | Weight 1 in crew spaces |
| `could not broadcast input array` in the sheet | An illustration tiled a strip too few times | Tile to the image width |
| An edited module not in the layers | The build loads the script once: an edit during a run is not in that run's renders | Re-render the target |
| A page throws "no texture layer undefined" | Triangles without a layer added to a dressed role | A role of the page's own |
| Diamond-plate lugs floating over a grate's pit or a drain, reading as a mesh | The tread was laid over the whole cell, openings included | `deck_plate(P, F, keep_out)`: list every opening the module cuts |
| Black ceilings in the comparison shots | The stand-in bake has no bounce and the lamps sit at the ceiling | The comparison page's floor-bounce stand-in; judge ceilings on the light-baking page too |
| A floor cell full of grates in one crew room | The hash, not the weight: the bridge drew 9 grates in 33 drawn cells at weight 1 | Weights are data; judge the whole ship (`kit_report.mjs --panels`), not one room |

## Checks before calling it done

| Check | How |
| --- | --- |
| Data valid | The build (it validates `panels.json` before anything) |
| Reproducible | Build twice (or `--only` the changed targets twice) and compare sha256 of the PNGs; the manifest records the versions |
| Walls untouched by a ceiling change | The wall layers' sha256 in the manifest against git HEAD |
| No change without the option | `node tools/mockups/kit_report.mjs` before and after; hash every compartment's parts |
| What it costs | `node tools/mockups/kit_report.mjs --panels` |
| It reads | The contact sheet and the page shots, looked at |
| Pages hold the current data | `python3 tools/mockups/inline.py --check` |
| Dashes | CLAUDE.md 5 grep |
| On the Pi | Not from here: a cloud render is not a Pi measurement (CLAUDE.md 2) |
