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
- **Floors.** Framed grates with a solid walkway (X1, X2) are next, after the owner sees this.
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
| `rib` | 0.25 m | An I-beam flange: a raised centre web, rivet lines at both edges, lightening holes every 0.5 m |
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

| Item | Cost | Budget |
| --- | --- | --- |
| Texture memory | 18 layers: 6.29 MB at 128 px per metre (1.57 MB at 64). The whole array with the 11 materials and the 22 wall layers is 17.8 MB at 128 px per metre | 96 MB (19 %) |
| Triangles | About 2 a ceiling cell, about 1,200 on the Tern before the bake's subdivision (which already splits ceilings at 2 m); trims none | Each compartment keeps more than 75 % of its ceiling |
| Draw calls | None | |
| Fragment work | None | |

Measured numbers replace these estimates when the build lands.

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
| U2 | Floors next: framed grates with a solid walkway (X1, X2)? | yes / keep today's tiles | Yes, with screenshots before it is asked. Recommendation taken (ask only with screenshots) | none yet |
