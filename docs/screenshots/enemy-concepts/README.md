# Hound: three concept studies

[Open the HTML viewer](../../mockups/enemy-ships.html).

Select **Pursuer**, **Splitjaw** or **Bulwark**, then use **Floor plan**, **Hull**
and **Fitted plan** to follow the process. Choose a deck, click a room for its
area and purpose, or select **3D cutaway**. Drag to orbit, right-drag to pan and
scroll to zoom. Touch supports orbit, pinch and pan. **Original outline** shows
the first room shapes over the fitted plan. **Normal** and **Red alert** switch
the interior light.

| Study | Review |
|---|---|
| Pursuer | Long raider with an aft twin-fighter hangar, two decks, 8 crew |
| Splitjaw | Forked carrier with separate forward launch bays, two decks, 10 crew |
| Bulwark | Broad armored corvette with a forward weapons deck, three decks, 12 crew |

All three start with the Hound and Jackal work from main at `7653f7a`. The initial
plans precede the hull pass; the fitted plans shape the rooms to that hull.
These are rough studies with existing small Material Maker textures and scale
furnishings. The fighters reserve 8 x 8 m footprints, people are 1.8 m tall, and
bunks are 2 m long. Production art and enemy behavior await a chosen direction.

- [Hulls and silhouettes](hulls-overview.png)
- [All fitted plans](plans-overview.png)
- [All deck cutaways](cutaways-overview.png)
- [Red-alert interiors](alert-overview.png)
- [Phone review](mobile/splitjaw-fit-plan.png)
- [Initial plan captures](initial/)
- [Full final captures](review/)

The initial screenshots retain the nominal dimensions used before the hull pass.
The finished viewer reports measured exterior dimensions, including fittings.

Validation: [layout and scale](layout-validation.json), [browser and budgets](browser-validation.json),
[coplanar overlap](zfight-validation.json). All captures were inspected. The geometry
check found zero coplanar overlap in the three hulls and all seven deck cutaways.
This is desktop visual verification, not a Pi performance measurement.

Reproduce from the repository root:

```powershell
python tools/design/fit_enemy_concepts.py --check
python tools/design/check_enemy_concepts.py --out docs/screenshots/enemy-concepts/layout-validation.json
python tools/mockups/inline.py docs/mockups/enemy-ships.html
node tools/mockups/check_enemy_ships.cjs
node tools/mockups/shoot.mjs docs/mockups/enemy-ships.html --only 'hull*,top*,fitted*,cutaway*,alert*' --out docs/screenshots/enemy-concepts/review
node tools/mockups/zfight.mjs docs/mockups/enemy-ships.html --setup "ENEMY_VIEW.show('splitjaw','fit','B','cutaway')"
```

The browser tools require Playwright. The HTML itself requires no local server;
its pinned Three.js modules are loaded from jsDelivr.
