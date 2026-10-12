# Detailed Grabon ship review

[Open the interactive review](../../mockups/grabon-ships.html).
Select Shrike, Vark or Gorath, then use **Hull** for the exterior or
**Fitted plan > 3D cutaway** for the furnished decks. Select a room and press
**Room** to inspect its textures and equipment. **Deck access** exposes the
stairs, ladders and elevators together. **CALL LIFT A/B/C** moves the car to
that floor. Drag to orbit, right-drag to pan, and scroll to zoom.

All three retain their approved shapes, purple armor and orange markings.
Gorath's engines remain centered. Each hull has its own UV texture atlas and
modeled hatch hardware, service pipes, vents and engine fittings. Interior
walls and floors use the existing Blender-baked panel set; consoles, chairs,
bunks, medical beds, machinery and fighters use the shared textured models.

- [Hull comparison](hulls-overview.png)
- [All seven furnished deck cutaways](decks-overview.png)
- [Stairs, ladders and elevators](access-overview.png)
- [Interior close views](interiors-overview.png)
- [Normal and red-alert captures](review/)
- [Editable Blender sources and GLBs](../../../assets/models/grabon/README.md)
- [Texture guides, UV wireframes and generation prompt](../../../assets/textures/grabon/GENERATION.md)
- [Detailed layouts](../../../data/ships/grabon-detailed/layouts.json)

The three hulls use 7,232 / 7,360 / 8,312 triangles. The largest visible deck is
10,184 triangles; the maximum draw count is 15. All 58 rooms are reachable,
with 0.18 m stair risers, about 0.31 m going at the walking line, 3.54 m clearance
between flights, and a 1.30 m bypass beside the ladders. Every main deck has a
stair landing and lift entrance. The live HUD reports visible triangles, draw
calls and 59.00 MiB texture storage including mipmaps. These counts are design
costs, not Pi performance measurements.

Validation artifacts: [layout and scale](layout-validation.json),
[actual geometry, floor holes, lift travel and UI](browser-validation.json),
[coplanar overlap](overlap-validation.txt). All 13 hull/deck/access states have
zero measured overlap. The browser check also verifies the floor remains solid
beside every upper-deck ladder hole. The rough Grabon and Federation
viewers retain their shared renderer and pass their layout and capture checks.

These are detailed review assets and animated preview mechanisms. They have
not been adopted by the game engine. Interiors remain editable layout data
and shared geometry builders; the exported GLBs contain the exterior hulls.

To repeat the checks from the repository root, with Playwright available:

```powershell
python tools/design/check_grabon_detail.py
node tools/mockups/check_grabon_detail.cjs
node tools/mockups/check_grabon_overlap.cjs
python tools/mockups/inline.py --check
openspec validate --all
```
