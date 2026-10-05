# Design: wall panels, not wall tiles

## Context

The owner, 2026-10-05: "notice how they dont have square panels on the walls? its all paneling
with different shapes, vents, pipes etc to make the room feel industrial, mechancial and lived
in". The references and the diagnosis of today's walls are in
`docs/analysis/texture-references.md` (X1-X5).

What exists:

- **`surface-materials`** (proposed, built in the mockups): one texture array of 128 x 128 px
  layers, 64 px per metre inside. UVs are projected from the world, so a material runs
  continuously across a wall. Relief is baked into the colour, and alpha is the emission mask.
- **`deck-pipeline` 5a** (proposed, built in the mockups): frames every 2.0 m along the keel. A
  fore-and-aft or raked wall carries a rib at every frame, an athwartship wall carries
  stiffeners, coves sit under the ceiling and baseboards at the floor. `finish` (`crew` or
  `working`) picks each generated surface's material.
- **The Tern's walls:** about 1,130 m of brush edge and 4,040 m^2 of wall, which is about 650
  bays of up to 2 m. These are measured from the layout's brushes, counting each side of a
  shared wall and the open faces between brushes of one room, so they are an upper bound.

## Goals / Non-Goals

**Goals:**
- No wall is a grid of one square. Neighbouring bays differ, and every bay has a band
  structure.
- The panels carry the references' vocabulary: vents, pipes, hatches, junction boxes, light
  columns, ribbed sheet, screens, door surrounds.
- Same layout and seed, same walls. A change to one wall changes no other wall.
- One draw per compartment, the texture budget stated, no per-pixel lighting.

**Non-Goals:**
- **New wall geometry**, such as recessed door surrounds, pipe runs that stand proud, or pillars
  (X1's forms). That is a later step for `deck-pipeline` 5a, once the textures are agreed.
- **Floors and ceilings.** Framed grates with a solid path (X1, X2) is the next look question
  after walls.
- **Decals** (stencils, signage, wear marks placed by hand). They are in `surface-materials`'
  decal set.

## Decisions

### 1. The bay is the unit

A **bay** is the stretch of one wall face between two adjacent ribs or stiffeners, or between
a rib and a corner, an opening's frame or a wall fixture's keep-out. Fore-and-aft and raked
walls have ribs every 2.0 m, so most bays are 2.0 m wide, and a panel's sides meet ribs that
hide its seams. This is the X3 sheet's way of working: one designed panel per bay of
structure.

- A bay **1.6-2.4 m** wide takes a full module, centred. Every module has 0.2 m of plain plate
  at each side, so cropping to 1.6 m, or wrapping 0.2 m past its edge, never cuts a feature.
- A bay **0.6-1.6 m** wide takes a narrow module, whose feature is 0.6 m wide in the middle of
  plain plate.
- A bay **under 0.6 m** takes plate.
- A bay **over 2.4 m** (a wall without ribs) is split evenly into sub-bays of at most 2.4 m.

### 2. The wall is banded

From the floor up:

| Band | Height | What it is | Source |
| --- | --- | --- | --- |
| Base | 0.5 m | Kick plate with a grille every 2 m, behind the baseboard | A strip that tiles along the wall |
| Module | 2.0 m | The bay's panel | The bay's module |
| More modules | 2.0 m each | Another module for each further whole 2.5 m of height (tall rooms: engineering, the hangar) | Chosen by the same rule, one level up |
| Top | the rest, up to the cove | Pipe and conduit run; on walls over 0.5 m it repeats in 0.5 m quads, the last cropped | A strip that tiles along the wall |

A crew room 3.0 m high has 2.65 m of flat wall under its 0.35 m cove: a base, a module and a
0.15 m top band. The bridge (3.5 m, 3.15 m of flat wall) has a 0.65 m top band. Strips are 0.5 m
tall and tile with a 2 m period. The four strips of a set (base, top, a louvre band and a spare)
share one layer, stacked with guard rows so mipmaps do not bleed between them.

### 3. The module catalogue

Each set has ten modules, all designed as 2 m x 2 m panels with plain margins:

| Module | Content (after the references) | Weight | Emissive |
| --- | --- | ---: | --- |
| `plate` | Two plates with a seam, rivets and a stencilled label: the rest panel | 3 | no |
| `vent` | A framed louvre grille, or stacked octagonal grilles (X2) | 1 | no |
| `pipes` | A vertical pipe pair with clamps and a valve wheel (X3) | 1 | no |
| `hatch` | A square access hatch, a handle, a hazard border | 1 | no |
| `junction` | A conduit box with cables in and out | 1 | a status pill |
| `light` | A recessed vertical light column (X3's pills) | 1 | yes |
| `ribbed` | Horizontal ribbed sheet (X1's stacked blocks, flattened) | 1 | no |
| `screen` | A small inset status screen with buttons | 1 | yes |
| `flank` | Plate with a vertical amber light strip, beside doors (X2) | rule | yes |
| `narrow` | A 0.6 m pipe-and-light column for narrow bays | rule | yes |

The **crew** set is a warm mid grey, clean, with white and amber light. The **working** set is a
darker grey-brown with rust streaks, hazard borders and amber light.

### 4. The rule: which bay takes which module

The rule is deterministic, local and stable. For each bay, in this order:

1. A bay holding a **window**, a **wall fixture** (a console bank, the viewscreen) or a
   **portal's frame** takes `plate`, since its features would be cut. The fixture covers it
   anyway.
2. A bay touching a **door's frame** takes `flank`, mirrored so its light strip is on the
   door's side.
3. A bay holding a **vent portal** (once `life-support` section 19 puts vents in the layout)
   takes `vent`. Until then vents are decoration, like the rest.
4. A **narrow** bay (0.6-1.6 m) takes `narrow`. A bay under 0.6 m takes `plate`.
5. Otherwise the module is drawn by weight from the set, with two exclusions: the module of the
   bay before it on the same wall face, and the one below it in a tall wall. The draw is the
   FNV-1a hash of (ship seed, compartment id, brush index, edge index, bay index, band level),
   mapped onto the cumulative weights.

The key holds nothing from any other wall, so editing one wall changes only that wall's
bays. Hash order never chooses anything else (CLAUDE.md 6.4).

### 5. Texture coordinates and layers

A module quad spans its bay and its band. Its u runs from the bay's centre (u = 0.5 at the centre,
at the space's texel density, so a 1.6 m bay shows u 0.1-0.9), and its v runs from 0 to 1 over the
band. A strip quad takes world-projected u along the wall (it tiles), and v within its strip's
quarter of the layer.

The vertex already carries a layer and a texture coordinate (`deck-pipeline` section 5), so
nothing in the vertex or the shader changes. The difference from `surface-materials` is that
these coordinates are bay-local, not world-projected, so a module does not tile. That is
`surface-materials`' "every layer tiles" relaxed for panel layers only.

Panel lights are the layer's emission mask. They glow but are not light sources: the bake's
emitters stay the fixtures, which are data (`light-baking`). How much they glow in each state
is set in `panels.json`. Proposed: 100 % in normal, 100 % at red alert, 30 % on emergency power.
The alert itself is carried by the lamps (R4 in `star-trek-bridges.md`: alert is light).

### 6. How panels are made (question V2)

| Option | How | Runs in a cloud session | Look | Fit with the rules |
| --- | --- | --- | --- | --- |
| **A. Material Maker graphs** | A graph per module from its 2D SDF nodes (boxes, circles, boolean union and subtract, repeat, bevel to height), rendered and post-processed like today's materials | No: Material Maker needs Godot 4.7, and GitHub release downloads return 403 here | Clean vector shapes; relief from the height | CLAUDE.md section 9 as written |
| **B. Blender, modelled and baked (the prototype)** | Each module modelled as low relief with boolean cutters and bevels (the `blender-hard-surface` kit, bpy 4.5 headless), rendered orthographically under a fixed key light with ambient occlusion, plus an emission pass, then the same recolour and reduce as today | Yes | Real depth: occlusion in vents, highlights on bevels; closest to X1-X3 | Needs section 9 amended: "panel modules are modelled and baked in Blender" |
| **C. Blender shapes, Material Maker wear** | B's render multiplied by a Material Maker grime and rust graph | Only the shapes | B plus procedural wear | Both tools, each for what it does best |

**Recommendation: B now, C when Material Maker can run.** B is what the prototype shows, and
it uses one toolset for props and panels (the bridge consoles are already built this way). The
wear layer of C can follow without redoing the shapes.

### 7. What the prototype shows (`docs/mockups/wall-panels.html`)

The prototype exists so the owner can judge by eye. Nothing in the layout or the other mockups
changes until the owner picks.

- **Views:**
  - the command passage (a crew corridor);
  - the bridge as it stands;
  - the crew quarters;
  - engineering's lower floor (working, tall walls);
  - the hangar (working, tall walls).
- **Toggles:**
  - today's tiled `bulkhead`;
  - panels at 64 px per metre;
  - panels at 128 px per metre (survey S1);
  - normal, red alert and emergency power.
- **A contact sheet** of both sets.
- **Each shot named**, before and after, into `docs/screenshots/mockups/wall-panels-*.png`.

### 8. The Pi 5 budget this change spends

| Item | Cost | Budget |
| --- | --- | --- |
| Texture memory | 22 layers: 1.92 MB at 128 px (64 px per metre); 7.69 MB at 256 px (128 px per metre), with the other 11 layers at 256 px as well, 11.5 MB in all | 96 MB (2-12 %) |
| Triangles | At most +4 per bay for the band splits, plus 2 per extra 0.5 m of top band: about +2,600 on the Tern at most, before the bake's subdivision, which splits walls at 2 m anyway | Each compartment keeps more than 75 % of its ceiling (`deck-pipeline` 5a) |
| Draw calls | None | |
| Fragment work | None: same fetch, same emission term | |
| Disk | Estimated 0.3-1 MB of PNG for 22 layers | |

### 9. Data

`data/materials/panels.json` (proposed, schema `starcrew.panels/1`): for each finish, its
modules (id, layer, weight, emissive, mirrored for `flank`), its strip layer and the strips'
rows; the band heights (`base_m` 0.5, `module_m` 2.0, `strip_m` 0.5); the bay widths (`full_min_m`
1.6, `full_max_m` 2.4, `narrow_min_m` 0.6); the glow per lighting state; the source of each
module (the Blender script and its version, or a Material Maker graph). It follows the rules of
CLAUDE.md 6.5: units in the keys, validated, one source.

## Risks / Trade-offs

- **Repetition at scale.** About 650 bays split between two sets is about 325 a set. With `plate`
  weighted 3 in 10, it shows about 100 times a set and each other drawn module about 30 times,
  so the busy modules stay rare. A
  third set, or alternates of the busiest modules, is the fix if a long corridor still reads
  as repeating.
- **Crops.** The 0.2 m margins limit how far a module can be cropped. Bays from 1.2 to 1.6 m
  fall back to `narrow`, which is plainer.
- **Strip bleed.** Four strips in one layer can bleed at the smaller mip levels. Guard rows of
  the plate colour hide it at the distances where those mips are used.
- **Two toolchains.** If V2 picks B, materials (Material Maker) and panels (Blender) are made
  differently. C joins them.

## Open questions

Per CLAUDE.md 13, a question goes to the owner only with something to look at.

| Id | Question | Options | Recommendation | Shots |
| --- | --- | --- | --- | --- |
| V1 | Do the panel walls read as the industrial, lived-in ship the references show? | panels as prototyped / panels, with changes (say which) / today's tiles | Panels as prototyped | `wall-panels-*-before.png` and `-after.png` |
| V2 | How are panels made from now on? | A. Material Maker graphs / B. Blender modelled and baked / C. Blender shapes with Material Maker wear | B now, C when Material Maker runs | The prototype's contact sheet |
| V3 | Panel lights on emergency power | 30 % / off / full | 30 %: the room dims but stays readable. Recommendation taken (ask only with screenshots) | none |
