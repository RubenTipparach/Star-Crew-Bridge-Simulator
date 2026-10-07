# Design: ceilings and trims as baked panels

## Context

The owner, 2026-10-05: "Ooh now that is good. Walls and ceilings still look awkward update those
as well using your new textures skill". The wall panel system is `wall-panels`:
- bays at the ribs;
- bands from the floor;
- ten modules per finish;
- the FNV-1a rule;
- bay-local texture coordinates;
- Blender-baked layers in the deck's one texture array.

This change applies the same system to the surfaces the walls left behind.

What the kit builds today (`deck-pipeline` 5a, `data/ships/tern/detailing.json`):

| Member | Size | Where | Texture today |
| --- | --- | --- | --- |
| Ceiling | the brush's footprint | every room | `ceiling` (crew) or `machinery` (working), tiled at 2 m |
| Beam | 0.24 x 0.18 m (0.38 m deep in rooms over 3.6 m) | across the room at every frame (2 m) | `trim` (crew) or `machinery` (working) |
| Rib | 0.24 x 0.12 m (0.22 m deep in tall rooms) | up fore-and-aft walls at every frame | `trim` or `machinery` |
| Cove | 0.35 m (0.6 m in rooms over 3.6 m), 45 deg | between solid walls and the ceiling | `trim` |
| Baseboard | 0.14 m high, 0.03 m proud | along every solid wall | `trim` or `hazard` |
| Door frame | jambs 0.16 m, lintel 0.22 m | round every door | `trim` and `hazard` |

The Tern has about 2,445 m^2 of ceiling, about 610 cells of 2 x 2 m (measured from the brush
footprints, 2026-10-05).

## Goals / Non-Goals

**Goals:**
- No ceiling is one tiled texture. Neighbouring cells differ, and lamps sit in surrounds made
  for them.
- Trims read as steel members: flanges, rivet lines, lightening holes, cable trays.
- Same layout and seed, same ceilings, as for walls. An edit stays local.
- One draw per compartment. The texture budget is stated.

**Non-Goals:**
- **Floors.** They are `floor-panels` (owner, 2026-10-05: "Floors and ceilings in mean"), built with this change.
- **New geometry.** Members keep their sizes. Only their textures and texture coordinates change.
- **Railings, ladders, rims, collars and lamp housings.** They keep their materials. They are
  small, and their current look fits.

## Decisions

### 1. The ceiling cell

A ceiling **bay** is the strip of ceiling between two adjacent beams, or a beam and the room's
end. It is cut into **cells** 2 m across, starting from the room's centreline (x = 0) and working
outward, so the cells are symmetric port and starboard. A cell that the room's polygon or a
cove cuts keeps its module: the texture coordinates are cell-local, so the cut only crops
the cell.

- A cell holding a **lamp** (the kit's `lampsFor` places lamps in the bays between frames) takes
  `lamp_surround`: a plain mounting plate with a frame, which the lamp housing sits on.
- A cell holding a **floor portal's collar** (a ladder well or hoist overhead) takes `plate`.
- Otherwise the module is drawn by weight with the wall rule's hash. The bay key is (ship seed,
  compartment id, brush index, "ceiling", bay index, cell index). The draw excludes the module of
  the cell before it in the bay and of the cell in the bay before.

### 2. The ceiling modules

Each set has eight modules, designed as 2 m x 2 m panels with 0.2 m plain margins:

| Module | Content (after the references) | Weight | Emissive |
| --- | --- | ---: | --- |
| `plate` | Two plates with a seam and rivets: the rest panel | 3 | no |
| `grille` | A framed ventilation grille (X2's framed floor and ceiling vents) | 1 | no |
| `fan` | A round fan grille in a square frame | 1 | no |
| `cable_tray` | Two cable runs in an open tray across the cell | 1 | no |
| `pipes` | Three pipes crossing the cell with hangers | 1 | no |
| `hatch` | A square access hatch with a hazard border | 1 | a status pill |
| `ribbed` | Corrugated sheet | 1 | no |
| `lamp_surround` | A mounting plate and frame for a lamp housing | rule | no |

The **crew** set is lighter and quieter than the crew walls, so the room does not close in. The
**working** set is darker, with rust and hazard borders, matching the working walls.

### 3. Trim strips

A trim member is a box (`Builder.box`). Each of its long faces takes a **strip**: u runs along
the member's length at the space's texel density (it tiles, 2 m period), and v runs across the
face within the strip's row of the trim layer.

| Strip | Height in the layer | Content |
| --- | --- | --- |
| `rib` | 0.25 m | An I-beam flange: a raised centre web, rivet lines at both edges, lightening holes every 0.5 m. Ribs are the wall's pillars (owner, 2026-10-05: "some wall pillars might need retouching up too"), so a rib's front face is split into a 0.3 m **base**, the tiling shaft and a 0.3 m **capital** under the cove, each with its own row (`rib_base`, `rib_capital`), after Undercity's "Pillars always have a base and a capital" (+4 triangles a rib) |
| `beam` | 0.25 m | As `rib`, with a hazard edge in the working set |
| `cove` | 0.375 m | A cable tray with two cable runs and hangers |
| `baseboard` | 0.15 m | A kick plate with bolts and a slot vent every 1 m |
| `frame` | 0.25 m | A door frame's face: a stepped bevel, with a hazard edge in the working set |
| `side` | 0.125 m | Plain steel for the narrow side faces of members |

The rows share one trim layer per finish, with guard rows between them as the wall strips have.
Members that are deeper than a row is tall (the tall rooms' 0.38 m beams) take the row
stretched in v, at most 1.6 times. A member's end faces take `side`.

### 4. Texture coordinates and layers

As in `wall-panels` section 5, the vertex already carries a layer and a texture coordinate, so
neither the vertex nor the shader changes. Ceiling cells use cell-local coordinates (u 0-1 across
the cell, v 0-1 along the room). Trim faces use member-local coordinates (u along the length,
world-projected so a long beam tiles continuously; v within the strip's row). `finishes` in
`detailing.json` keep naming the materials of surfaces this change does not dress.

### 5. Made the same way

The panel build (`tools/blender/build_wall_panels.py`) models the ceiling modules and the trim
strips with the hard-surface kit, and bakes them under the same key light and occlusion. For a
ceiling the light comes from below, as a ceiling is lit by the room's lamps. The build
post-processes them the same way and writes them into the same manifest (`data/materials/panels.json`)
and texture folder. The method gets a skill of its own, `panel-textures`, because the owner
asked for a "textures skill" and walls, ceilings and trims now share it.

### 6. Turned on everywhere

V1 is answered, so every interior mockup dresses its rooms with the wall panels, the ceilings
and the trims, at 128 px per metre provisionally (S1 is open):
- the bridge;
- the bridge variants;
- the deck plan;
- light baking, where its bake can take the extra cells.

The comparison page (`wall-panels.html`) keeps its "today" toggle, so before and after can still
be compared.

### 7. The Pi 5 budget this change spends

Measured 2026-10-06 in the mockup: texture bytes by the build's manifest
(`assets/textures/panels/manifest.json`) and `loadPanels`, triangles by
`node tools/mockups/kit_report.mjs --panels` (before the bake's subdivision). Not measured on a Pi.

| Item | Cost | Budget |
| --- | --- | --- |
| Texture memory | 18 layers (16 ceiling, 2 trim): 6,291,432 bytes with mips at 128 px per metre (256 px layers; 6.29 MB), 1,572,840 at 64. The whole array with the 11 materials and the 22 wall layers is 17,825,724 bytes (17.8 MB) at 128 px per metre; with `floor-panels`' 15 layers as well, 23,068,584 (23.1 MB) | 96 MB (24 % with floors) |
| Disk | 18 PNG layers, palette-reduced to 48 colours: 573,439 bytes at 128 px per metre (ceilings 502,496, trims 70,943), 192,600 at 64 | |
| Triangles, ceilings | 150 to 1,462 on the Tern (+1,312) over 688 cells: 2 a whole cell, up to 6 a cell the outline or a hole cuts | |
| Triangles, trims | 10,910 to 12,802 (+1,892): 1,508 for the 377 ribs' bases and capitals (4 a rib) and 384 where a face deeper than 1.6 rows repeats its row (a tall room's 0.6 m cove, a 0.22 m rib side) | |
| Per compartment | With walls, ceilings, floors and trims dressed, against `deck-pipeline` section 11's totals: engineering 5,156 to 6,074 of its 8,000 (64 % to 76 %, the fullest, on that table's first-estimate props, which count its door frames and ladders twice), the hangar 3,240 to 3,934 (49 %), every other compartment under 30 % | Each compartment under its ceiling |
| Draw calls | None: every layer is in the one array | |
| Fragment work | None: same fetch, same emission term | |

### 8. How the prototype reads this design

Built 2026-10-06: `tools/blender/build_wall_panels.py` (the ceiling modules and trim rows),
`data/materials/panels.json` (`cells`, `trims`, and per finish `ceiling` and `trims`), shipkit's
`opts.panels` (`dressFlat`, `stripBox`, `coveStrip`), and every interior page but the bridge
variants. Where the text above left a choice, the prototype made the nearest one that works:

- **Cells are centred on the centreline** (x = 0, 2, 4 m, so their edges fall at odd metres). A
  single lamp in a room on the centreline sits there, so its cell is whole.
- **A lamp's cell is every cell its housing spans**, and `lamp_surround` is a light channel that runs
  the cell's whole width between bolted rails, with v centred on the lamp. The lamp rule puts lamps
  anywhere across a bay, so a surround centred on its cell would leave most housings off its frame,
  and a 2 m cell centred on each lamp cropped every neighbour to plate. Two lamp cells side by side
  join into one channel.
- **A cut cell takes plate when its core is cut**: the outline or a hole crossing within
  `cells.core_m` (0.6 m) of its centre. A cove crops 0.35 m off a cell against a wall, which only
  takes its margin. Measured on the Tern: 400 lamp cells, 186 cropped, 4 at a collar, 98 drawn.
  The lamp rule's density (one lamp a 8 m^2) makes lamp channels the commonest ceiling module.
- **The key** is (seed, compartment, brush index, "ceiling", frame index, cell index), the indices
  being the world's, so an edit to one brush moves no other brush's cells.
- **Seven trim rows in one 2 m layer** (`trims.rows`, from the bottom): `side`, `baseboard`, `rib`,
  `rib_ends`, `beam`, `frame`, `cove`, each a whole number of texels at 64 px per metre (the
  baseboard's 0.15 m is 0.15625 m), two to three texels of guard between them filled with each
  row's own edge. Eight rows of section 3's heights would leave under a texel of guard, so the
  base and the capital share `rib_ends`, as two 0.45 m pieces at -0.5 m and +0.5 m
  (`trims.pieces`).
- **Pillars**: base and capital are 0.3 m (0.6 m in a room over 3.6 m, engineering's and the
  hangar's), each piece mapped onto it; a rib shorter than both plus 0.4 m keeps a plain shaft.
  Bases are hazard-striped in working spaces.
- **Which face takes which row**: a member's front face its own row; its long sides `sides` (a
  beam's sides `beam`, everything else `side`); its ends `side`. A face deeper than 1.6 rows
  repeats the row whole: a tall room's 0.6 m cove is two trays.
- **Pressure doors keep their hazard jambs** (`frame_hazard`); their reveals and lintels take the
  frame row. Window frames and sills take the frame row too.
- **Light**: a ceiling module is lit nearly square on from below (`render.ceiling`); a trim row
  along +x, which is up a pillar, so a base's top chamfer catches the light and a capital's
  underside is shaded. Ceilings and trims take grime, edge wear and rust in patches, not streaks.
- **Colours**: crew ceilings a light warm grey (0.50, 0.49, 0.46 sRGB), working ceilings dark with
  rust; crew trims dark gunmetal (0.31, 0.31, 0.30) with bright worn edges, darker than the walls
  they frame, working trims dark brown steel with hazard edges.
- **The comparison page** adds a stand-in for the floor's bounce to its stand-in bake (each lamp
  also lights the room from the floor under it at 30 % of its strength), for "today" and the
  panels alike: with the lamps at the ceiling and no bounce, every ceiling was black. The light
  baking page bakes real bounce.

### 9. Platform faces (2026-10-07)

The owner, on the bridge's raised rings: "the wall on these platforms kinda suck, please use better uv tiling, and redo
to make the texture conform to the height better, and look better. Usually these areas are made of vents of some kind,
or small metal panels that should fit vertically on the geometry uv". A platform's riser (deck-pipeline 5a: the face
under a raised floor's edge) took the finish's `trim` material projected flat in metres, so it stretched, and nothing on
it was sized to its 0.45 m.

**A platform layer per finish** (`<finish>_platform`, panel layers 66 and 67, after the 55 that exist), built like the
trim layer: rows stacked into one 2 m layer, each tiling along x with the layer's period.

| Row | Face | Height | What it shows |
| --- | --- | ---: | --- |
| `riser` | a riser two steps high (the rings, the dais) | 29/64 m | a dark toe kick, a lip under the nosing, and between them louvred vents and small bolted panels as tall as the face's clear height |
| `riser_low` | a riser one step high (the helm sub-platform) | 15/64 m | the same family designed at its own height, not a crop |
| `step` | a stair step's front | 15/64 m | a quieter bolted kick plate with a slot vent band |

**Mapped to fit.** A riser's bottom edge takes the row's bottom and its top edge the row's top, exactly: the vents end at
the kick and the lip whatever the face. A face is given the row nearer its height (`riser` from the mean of the two rows'
heights up), stretched by at most `trims.stretch_max`. u runs along the platform's outline from its first corner, in
metres over the layer's 2 m, so the rhythm of vents carries round a ring of short segments without a seam at every
corner. The kit draws a riser this way when a page passes `riserLayer` (with `topLayer`, the tread on top); its role is
`platform_riser`, the page's own, as `platform_top` is.

**What it costs.** Two layers, 0.35 MB each at 256 px with mips (87 KB at 128 px). The same two triangles a riser
always had; no draw call.

## Risks / Trade-offs

- **A busy ceiling over busy walls.** The crew ceiling set is deliberately quieter, with plate
  weighted 3 in 9. If rooms feel cluttered, the busy modules' weights go down in data, without
  rebuilding the textures.
- **Stretch on deep members.** Up to 1.6 times in v. A dedicated deep-beam row is the fix if it
  shows.

## Open questions

| Id | Question | Options | Recommendation | Shots |
| --- | --- | --- | --- | --- |
| U1 | Do the ceilings and trims now sit with the panel walls? | as built / with changes (say which) | As built | `wall-panels-*-after.png` once this lands, with before shots |
| U2 | Floors next: framed grates with a solid walkway (X1, X2)? | yes / keep today's tiles | **Answered 2026-10-05** (owner: "Floors and ceilings in mean"): yes, now, as `floor-panels` | none |
