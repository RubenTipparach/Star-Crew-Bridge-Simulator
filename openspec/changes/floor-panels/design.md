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

| Item | Cost | Budget |
| --- | --- | --- |
| Texture memory | 16 layers: 5.59 MB at 128 px per metre (1.40 MB at 64). With the materials, walls, ceilings and trims, the array is 23.4 MB at 128 px per metre | 96 MB (24 %) |
| Triangles | About 2 a floor cell, about 1,200 on the Tern, less the corridors' runners | Each compartment keeps more than 75 % of its ceiling |
| Draw calls | None | |

Measured numbers replace these when the build lands.

## Risks / Trade-offs

- **Busy floors.** Grates are weighted 2 in 10 and plate 3. Weights are data.
- **A walkway that bends.** Paths from several doors meet at the centre, so a room with doors on
  three sides gets a T of walkway cells. That is intended: it is where people walk.

## Open questions

| Id | Question | Options | Recommendation | Shots |
| --- | --- | --- | --- | --- |
| R1 | Do the floors, with their walkways, sit with the walls and ceilings? | as built / with changes | As built | `wall-panels-*-after.png` once this lands, with before shots |
