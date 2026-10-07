# Design: floors as baked panels

## Context

The owner, 2026-10-05: "Floors and ceilings in mean." The ceiling system is `ceilings-and-trims`
section 1: bays at the frames, 2 m cells from the centreline, the FNV-1a rule, and cell-local
coordinates. Floors use the same cut, so a floor cell sits under a ceiling cell.

Floors today:
- `deck_tiles` (crew) or `deck_plate` (working) tile at 2 m;
- a raised 0.9 m runner (`detailing.json` `runner`) runs down each corridor;
- hazard rims surround floor portals (`floor_portal`);
- platforms (the bridge variants' rings and dais) are solid raised floors with their own
  `platform` material.

There are about 2,445 m^2 of floor, about 610 cells (measured from the brush footprints,
2026-10-05).

## Goals / Non-Goals

**Goals:**
- No floor is one tiled texture.
- A readable walkway: where people walk is solid plate, and grates and service plates are off to
  the sides (X1).
- Same layout and seed, same floors. An edit stays local.

**Non-Goals:**
- **See-through grates.** A grate is texture: its gaps are baked dark. Real openings would need
  geometry and alpha testing.
- **Platforms.** Platform tops keep `platform` until a bridge variant is picked (B11).

## Decisions

### 1. Cells and the walkway

Floors are cut exactly like ceilings: bays at the frames, and cells 2 m across from the
centreline outward. A cell is **walkway** when the 0.9 m-wide path through it, from any of the
room's doors to the room's centroid, crosses it. In a corridor that path is its length, so the
middle column of cells is walkway. A cell holding a floor portal (a hatch, ladder well, hoist
or bay door) takes `plate`, inside its existing rim.

### 2. The modules

Each set has eight modules, designed as 2 m x 2 m panels:

| Module | Content (after the references) | Weight | Emissive |
| --- | --- | ---: | --- |
| `walkway` | Solid plate with a non-slip pattern and edge bolts | rule | no |
| `plate` | Plain deck plate with seams: the rest panel | 3 | no |
| `grate` | Four square grates, each in its own raised frame (X2), gaps baked dark | 2 | no |
| `access` | A bolted access plate with a recessed handle | 1 | no |
| `trench` | A cable trench cover: a long plate with finger slots | 1 | no |
| `drain` | A round drain grille in plate | 1 | no |
| `vent` | A floor vent grille by the walls (X2) | 1 | no |
| `hazard` | Plate with a hazard-striped border (working set only; crew takes `plate` for it) | 1 | no |

The **crew** floors are mid grey and clean: the walkway lighter than the grates, so the path
reads. The **working** floors are diamond plate and grates with rust and wear, and hazard
borders.

### 3. Texture coordinates

These are cell-local, as for ceilings (`ceilings-and-trims` section 4). Since the walkway is a
set of cells, it runs continuously only if the walkway module tiles across cell edges. So its
side margins are walkway plate, not a seam.

### 4. The Pi 5 budget this change spends

Measured 2026-10-06 in the mockup: texture bytes by the build's manifest
(`assets/textures/panels/manifest.json`), triangles by
`node tools/mockups/kit_report.mjs --panels` (before the bake's subdivision). Not measured on a Pi.

| Item | Cost | Budget |
| --- | --- | --- |
| Texture memory | 15 layers (8 working, 7 crew: crew has no hazard module): 5,242,860 bytes with mips at 128 px per metre (256 px layers; 5.24 MB), 1,310,700 at 64. With the materials, walls, ceilings and trims, the array is 23,068,584 bytes (23.1 MB) at 128 px per metre | 96 MB (24 %) |
| Triangles | Floors 168 to 1,650 on the Tern (+1,482) over 795 cells, 2 a whole cell and up to 6 a cell the outline or a hole cuts; the corridors' runners give back 70: +1,412 | Under each compartment's ceiling; with walls, ceilings and trims dressed engineering is the fullest, 6,074 of 8,000 (`ceilings-and-trims` design 7) |
| Disk | 15 PNG layers, palette-reduced to 48 colours: 786,711 bytes at 128 px per metre, 237,423 at 64 | |
| Draw calls | None | |

### 5. How the prototype reads this design

Built 2026-10-06 with `ceilings-and-trims` (the same build, data file and kit option):

- **Cells** are the ceiling's: centred on x = 0, in the bays at the frames, cell-local coordinates.
  The rule order is: a portal's cell (its hole and its rim) plate; walkway; a cell whose core (0.6 m
  either side of its centre) the outline or a hole cuts, plate; otherwise drawn. Measured on the
  Tern: 139 walkway cells, 83 at a portal, 212 cropped, 361 drawn.
- **The walkway** is a band 0.9 m wide that overlaps a cell by at least 0.1 m (`walkway`). In a room
  it runs from each door (a wall portal of `rule.door_kinds`) to the area-weighted centroid of the
  brushes on that door's floor. In a corridor it runs down each brush's long axis through its
  centroid, and each door joins it square to the axis, so a side door does not draw a diagonal down
  the corridor.
- **The walkway module tiles in both directions**: its pattern repeats every 2 m and its wear noise is
  periodic in x and y (4D noise on a torus), so walkway cells read as one path. It is lighter than
  the deck (`walk` colour). Crew walkways have raised dashes, working ones five-bar tread plate.
- **Working plate is diamond plate**, a lattice of lugs that repeats every 2 m; crew plate is plain
  with countersunk bolts. Every module but the walkway cuts one seam groove on its right and top
  edges, so each joint between two cells is one groove.
- **Crew grates weigh 1, not 2**: at 2 the bridge's floor read as a field of grates in the first
  shots. Weights are data (Risks). Crew plate weighs 4, taking the hazard module's share.
- **The runner** is not drawn where floors are dressed; pages without the option keep it.
- **Fixtures keep their own floors**: a dais, a mezzanine or a catwalk a page adds keeps its tiled
  material (the deck plan draws them in roles of their own), platforms being a non-goal.

### 6. Floors around platforms, and vent bands at their feet (2026-10-07)

The owner, 2026-10-07, on a first-person shot of the bridge: "can we make custom floor vents and
stuff for the bridge floor? since the elements arent grid aligned they get cutoff by various decks
on or rafters on the bridge". The cells of section 1 sit on a 2 m grid from the centreline and know
nothing of what stands on the floor, so a raised ring, the dais or a sub-platform crosses grates and
vents part way: half a grate shows at a platform's foot, the rest under it.

**Covers.** A page passes the footprints that stand on a floor and hide it (`floorCovers`: each
platform's outline, and the edge bands below), with the floor's height. The cell rule (section 5)
gains one step after the portal's:

1. a ceiling cell a lamp housing spans: `lamp_surround`;
2. a cell holding a floor portal: `plate`;
3. **a cell wholly under a cover is not drawn** (it cannot be seen); **a cell whose core
   (`cells.core_m` either side of its centre) a cover reaches takes `plate`**;
4. the walkway; 5. a cropped cell, `plate`; 6. otherwise drawn by weight.

So no grate, vent, drain or access plate is ever cut by a platform: the cells left drawn are those a
platform does not reach, and the ones it does are plain plate, which reads the same cut or whole.

**Vent bands.** Where a platform meets the floor, the floor gets fittings made for that edge, not
for the grid:
- **Where.** Along every platform edge of kind `rail` or `riser` (`deck-pipeline` 5a), on the floor
  at the foot of the face. Not along a `wall` edge, and not along a `step` edge, where the stair
  stands.
- **What.** Under a `rail` edge a **vent** band: return-air grilles, where the crew stand at the
  rail and the air is drawn down (X2's grilles along the consoles). Under a `riser` edge a
  **trench** band: a cable trench cover with finger slots, where the cables to a platform's consoles
  come up.
- **Size.** 0.3125 m wide (20 texels at 64 px per metre), raised 1.2 cm, as the nosing is
  (CLAUDE.md 8: deliberately parallel surfaces at least 1 cm apart).
- **Whole fittings.** A band's row tiles with a 0.5 m period: one grille, or one trench plate,
  between plain frame ends. An edge of length L holds n = max(1, round(L / 0.5)) periods, its u
  stretched by L / (0.5 n), so every band ends on a frame and no grille is cut. An edge under
  0.25 m takes no band.
- **Corners.** At a convex corner between two banded edges, the gap between them is a wedge mapped
  onto the frame. At a reflex corner both bands are mitred on the bisector: the mitre takes
  w tan(a / 2) off each band's end, inside its 5 cm frame while the turn a is under 18 degrees.
- **Edges.** A band is clipped to its room's floor, so it never runs into a wall.
- **The layer.** One per finish, `<finish>_floor_edges`, holding the two rows like the platform
  layer (`edges.rows`). The panel build models them with the hard-surface kit, lit as floors.

The bands are covers too: the cells under them follow step 3.

**Crowded platforms.** Built, the round bridge's sub-platform stood 0.2 to 0.7 m from each ring, so
its trench band and the ring's vent band overlapped in the gap (0.43 m^2 of z-fighting found by
`tools/mockups/zfight.mjs`), and a band ran under the next platform. So a band that would run under
another platform, or over a band already laid (platforms in data order), is **dropped whole**: a
narrow gap between platforms stays plain deck. Cropping the band instead would show a cut fitting,
the thing this section removes.

**As built (2026-10-07), on the round bridge** (`command-suite`), in the deck plan, the command deck
and the bridge variants:

| Item | Measured |
| --- | --- |
| Bands | 18 laid of 25 candidate edges (14 vent, 4 trench); 7 dropped where platforms crowd; 4 corner wedges; 8.83 m^2 |
| Triangles | The deck plan's whole-ship count rose by 24 (92,634 to 92,658): the bands' triangles less the floor cells now hidden under platforms |
| Texture memory | 2 layers (`crew_floor_edges` 70, `working_floor_edges` 71): 699,048 bytes with mips at 256 px, 174,762 at 128 px; the panel layers now 21,320,964 and the whole array with the materials 25,165,728 bytes at 256 px (25.2 MB of 96 MB) |
| Disk | 4 PNGs, 32,117 bytes |
| Draw calls | None |
| z-fighting | Back to each page's slivers: deck plan 0.073 m^2, command deck 0.046, bridge variants 0.078 |

Not measured on a Pi 5 (a cloud session has no GPU, CLAUDE.md 12). Shots before and after:
`docs/screenshots/mockups/bridge-floor-before/`, `bridge-floor-after/`.

## Risks / Trade-offs

- **Busy floors.** Grates are weighted 2 in 10 and plate 3. Weights are data.
- **A walkway that bends.** Paths from several doors meet at the centre, so a room with doors on
  three sides gets a T of walkway cells. That is intended: it is where people walk.

## Open questions

| Id | Question | Options | Recommendation | Shots |
| --- | --- | --- | --- | --- |
| R1 | Do the floors, with their walkways, sit with the walls and ceilings? | as built / with changes | As built | `wall-panels-*-after.png` once this lands, with before shots |
