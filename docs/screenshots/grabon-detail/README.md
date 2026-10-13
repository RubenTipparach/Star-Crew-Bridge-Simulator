# Detailed Grabon ship review

[Open the interactive review](../../mockups/grabon-ships.html).
Choose Shrike, Vark or Gorath:

- **Hull > Windows** opens a selector for every exterior window. Orbit and zoom
  to look through its glass into the furnished room.
- **Fitted plan > Inside** shows the selected room with its ceiling. Choose a
  room from the dropdown, drag to look, and use WASD, arrows or scroll to move.
  The camera stands 1.65 m above the floor and stays clear of walls and furniture.
- **3D cutaway** exposes a whole furnished deck. Select a room, then **Room**
  to inspect it. Ceiling panels remain visible when the camera moves into a
  room, and are culled from above. **Deck access** shows stairs, ladders and elevators together;
  **CALL LIFT A/B/C** moves the car to that floor.
- **Rear** shows impulse placement; **Armor** shows the recessed service channels.
  **Normal** and **Red alert** compare interior lighting.

The interiors use a separate Grabon texture collection and four new furniture
models: interlocking plum armor, diagonal bronze stays, dark tread floors,
guarded amber lamps, tapered consoles, faceted chairs, armored bunks and mess
tables. The Claude panel-textures workflow models relief in Blender, renders
Cycles beauty and emission, then downsamples in linear light to 256/128 px with
a 48-color palette. Federation panel assets are unchanged. Shared engineering,
medical and fighter assets remain where their functions fit.

The 50 windows are actual hull openings, connected to the same rooms used by
the interior viewer: 16 on Shrike, 14 on Vark and 20 on Gorath. Every deck has
a level row centered 1.45 m above its floor, with 0.66 m tall glazing. Measured
surface corners fit the shallow liners and frames to the tapered hull. Armor
surrounds and cleared service pipes keep the openings unobstructed.

The approved purple/orange exteriors, centered impulse units and recessed
machinery belts remain. Gorath's bottom tier is 20.16 x 28.24 m beneath its
25.6 x 32.1 m engineering tier. All 58 rooms are reachable and each ship retains
two full-size fighter berths, stairs, ladders and elevator access.

- [Hull comparison](hulls-overview.png)
- [All seven furnished decks](decks-overview.png)
- [Stairs, ladders and elevators](access-overview.png)
- [Eye-level interiors and windows](interiors-overview.png)
- [Individual normal and red-alert captures](review/)
- [Ceiling correction, normal and red-alert captures](ceiling-fix/)
- [Grabon panel contact sheet](../materials/grabon-panels-contact-sheet.png)
- [Exterior Blender sources and GLBs](../../../assets/models/grabon/README.md)
- [New interior panels](../../../assets/textures/grabon-panels/README.md)
- [New furniture and atlases](../../../assets/models/grabon_furniture/README.md)
- [Layouts](../../../data/ships/grabon-detailed/layouts.json) and
  [interior/window settings](../../../data/ships/grabon-detailed/interior.json)

The exterior hulls use 10,870 / 13,566 / 16,406 triangles in four materials each.
With all furnished rooms visible, the largest whole ship is 68,218 triangles
and 33 draws. With ceilings retained, the largest deck is 17,552 triangles;
no deck exceeds 15 draws.
The viewer reports 55.75 MiB of texture storage including mipmaps. These are
asset costs, not Pi performance measurements.

Validation covers 1,150 window rays, aligned deck rows, mirrored hull geometry
and UVs, native mesh integrity, furniture scale, clear standing views, ladder
holes and their solid bypasses, elevator travel and phone controls. All 52
selectable rooms have clear standing views; keyboard walking passes on all
three ships. All 16
native hull, furnished interior, deck and access overlap states have zero
measured coplanar overlap. The panel pack's 126 PNGs, four furniture assets
and three hull GLBs reproduce with identical hashes in repeat builds.

Reports: [layout](layout-validation.json), [browser](browser-validation.json),
[native export](exterior-validation.json), [GLB rebuild](rebuild-validation.json),
[texture rebuild](texture-rebuild-validation.json), [overlap](overlap-validation.txt).
The ceiling pass retested all seven decks with zero overlap. A native Gorath
check timed out; the [native rerun](native-overlap-validation.txt) passed all
three hulls. The browser check validates inward-facing ceilings in all 66 deck
room slices, and the nine ceiling captures show the result from above and below.

These are review assets. Rooms use the shared direct vertex-light preview,
with 1 m surface subdivision; Cycles supplies panel relief shadows and emission.
The preview does not implement the planned engine room bounce/occlusion bake.
Interiors remain editable layout data and shared builders; exported hull GLBs
contain the exterior. Engine integration is separate from this review.

To repeat the checks, with Blender and Playwright available:

```powershell
python tools/design/check_grabon_detail.py
blender -b --factory-startup --python-exit-code 1 -P tools/blender/check_grabon_exteriors.py
node tools/mockups/check_grabon_detail.cjs
node tools/mockups/check_grabon_overlap.cjs
python tools/mockups/inline.py --check
openspec validate --all
```
