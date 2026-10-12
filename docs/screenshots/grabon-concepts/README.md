# Grabon concept review

[Open the Grabon viewer](../../mockups/enemy-ships.html) or
[open Shrike's 3D command deck](../../mockups/enemy-ships.html#shrike/fit/A/cutaway).
The earlier Pursuer, Splitjaw and Bulwark designs are now in
[Federation concepts](../../mockups/federation-ships.html).

| Study | Direction | Length x beam | Decks / crew |
|---|---|---|---|
| Shrike | Short command neck, down-swept weapon wings, compact aft engines | 53.1 x 60.7 m | 2 / 8 |
| Vark | Long habitable neck, forward command pod, swept spars and outboard engines | 67.6 x 61.4 m | 2 / 10 |
| Gorath | Broad command hammer, deep wing shoulders and a three-deck armored core | 57.9 x 68.3 m | 3 / 12 |

Use **Floor plan**, **Hull**, then **Fitted plan** to inspect the design sequence.
In Fitted plan, choose **3D cutaway** and a deck. Drag to orbit, right-drag to pan,
and scroll to zoom. Click a room for its purpose, area and height. **Original
outline** compares initial and fitted room shapes. Normal and red-alert lighting
are available in the interiors. Copied URLs retain the selected ship, stage, deck
and 3D mode.

The initial plans were captured before exterior construction. Their Pending
dimensions mean the hull was not built yet. Hulls were then captured and the
rooms fitted to their head, neck and body envelopes. The delivered catalogue
retains both sets of polygons. These are rough material and scale studies, with
simple furniture. Thin outer wings contain equipment, not walkable decks.

The hull palette is purple with dark plum secondary plating and burnt-orange
liveries. Gorath's thruster centers are raised to 5.2 m, near the main hull's
vertical midpoint, with the swept wings meeting the lower engine casings.

- [Hull and silhouette comparison](hulls-overview.png)
- [Gorath side](review/enemy-ships-side-gorath.png) and [rear](review/enemy-ships-rear-gorath.png)
- [All fitted plans](plans-overview.png)
- [All 3D deck cutaways](cutaways-overview.png)
- [Red-alert views](alert-overview.png)
- [Initial plan captures](initial/)
- [Current full captures](review/)
- [Phone cutaway](mobile/shrike-fit-cutaway.png)

Validation: [layout](layout-validation.json), [browser and budgets](browser-validation.json),
[coplanar surfaces](zfight-validation.json). There are 52 connected rooms, 31
modified in the hull-fitting pass, with at least 0.55 m hull-to-air clearance.
Every ship holds two 8 x 8 m fighter footprints. All ten visible hull/deck states
have zero measured coplanar overlap, including components merged into one mesh.
Hull costs are 1,964 / 2,092 / 2,080 triangles, each in one draw. The largest
Grabon deck is 2,002 triangles and 11 draws. Textures occupy 1.0834 MiB with
mipmaps. These are browser geometry counts, not Pi timing measurements.

The review uses the latest fetched main at `7653f7a` for existing Hound functions
and fighter scale. The [design](../../../openspec/changes/grabon-ship-concepts/design.md)
records the owner's Klingon reference direction. All new geometry is authored
in the concept catalogue; no borrowed ship mesh or texture is used.

From the repository root, with Playwright available to Node:

```powershell
python tools/design/fit_enemy_concepts.py --check
python tools/design/check_enemy_concepts.py --out docs/screenshots/grabon-concepts/layout-validation.json
python tools/mockups/inline.py docs/mockups/enemy-ships.html
node tools/mockups/check_enemy_ships.cjs grabon
node tools/mockups/shoot.mjs docs/mockups/enemy-ships.html --only 'hull*,top*,fitted*,cutaway*,alert*' --out docs/screenshots/grabon-concepts/review
node tools/mockups/zfight.mjs docs/mockups/enemy-ships.html --setup "SHIP_VIEW.show('shrike','fit','A','cutaway')"
```

The HTML opens from disk with no local server. It loads pinned Three.js modules
from jsDelivr. The authoritative source is `data/ships/grabon-concepts/layouts.json`;
the shared interpreter is `docs/mockups/lib/shipconcepts.js`.
