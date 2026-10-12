# Grabon direction and Federation filing

## Collections

Move the earlier catalogue, page and capture archive to Federation concept paths.
Keep a link between the Federation and Grabon pages. Share the existing concept
renderer, fitting tool and checks through catalogue parameters rather than cloning
their implementations. The enemy-ships.html entry now presents the Grabons.

## Design sequence

Author three connected initial plans in metres, capture them, and then build
distinct outer hulls. Shape the outer rooms to their hull envelopes and retain the
first polygons for comparison. Main passages remain 2.4 m wide, doors 1.2 x 2.1 m,
rooms at least 2.8 m high, and fighter bays 3.2 m high. Every design reserves two
8 x 8 m fighter berths. The fitter must handle separate hull zones on one deck:
the head, neck and engineering body have real connected rooms, while thin wing
tips and external engines are uninhabited equipment.

- Shrike: compact forward head, short neck, broad aft body, down-swept weapon
  wings and close-set aft exhausts. Two decks and eight crew.
- Vark: longer narrow access neck, isolated command pod, large aft engineering
  mass, swept spars and long outboard engines. Two decks and ten crew.
- Gorath: broad hammer-shaped command pod, armored neck, high engineering core,
  thick wing shoulders and paired heavy engines. Three decks and twelve crew.

Use olive and charcoal armor with small oxide-red areas and warm red-orange
exhausts. Simple faceted plates, structural ribs and recessed-looking dark panel
bands establish scale. These are rough Material Maker surfaces, not final UV art.
Surface boundaries and accents are authored in hull coordinates. No federation
white/cyan paint, saucer mass or stacked passenger-deck silhouette is carried over.

The owner requested Klingon inspiration. Visual references, used for proportions
and silhouette rather than copied models or textures:
- [Bird-of-Prey orthographic views](https://www.ex-astris-scientia.org/scans/shipcollection/klingon-bop-views.jpg)
- [D7 and Bird-of-Prey comparison](https://gmd3ddesigns.files.wordpress.com/2020/07/5-bottom.jpg)

## Verification and budget

The layout instrument checks convexity, connected portals, occupied hull
containment and clearance, and actual furniture and fighter footprints. Extend
its zone lookup to use each room's envelope when multiple zones share a deck.
The browser instrument measures geometry and real bounds, tests all stages,
deck selection, cutaway links, faction navigation and phone controls. Capture
and inspect every ship/deck; run the repository coplanar-overlap checker.

Per visible concept: at most 12,000 triangles, 120 draws and 4 MiB textures.
The scene budget remains 200,000 triangles, 300 draws and 96 MiB, sourced from
engine-stack/design.md. Existing texture layers are reused. No game runtime
allocation or simulation/network cost is added. Browser counts are not a Pi 5
performance measurement.

## Review results, 2026-10-12

The fitted Grabon collection contains 52 reachable rooms. Fitting changes 10
Shrike rooms, 9 Vark rooms and 12 Gorath rooms, while retaining each initial
polygon. The minimum hull-to-air clearance is 0.55 m. All doors align with shared
walls, ladder openings align across decks, and fighter and furniture footprints
fit. Shrike has 14 rooms, Vark 16 and Gorath 22.

| Study | Length x beam | Hull triangles / draws | Decks |
|---|---|---|---|
| Shrike | 53.1 x 60.7 m | 1,964 / 1 | 2 |
| Vark | 67.6 x 61.4 m | 2,092 / 1 | 2 |
| Gorath | 57.9 x 68.3 m | 2,080 / 1 | 3 |

The largest Grabon cutaway is 2,002 triangles and 11 draws. All concepts reuse
1.0834 MiB of texture layers including mipmaps. The surface check now retains
component roles inside merged meshes, and thus checks their internal overlaps
as well. It exposed rough primitive overlaps, repaired by separating turret and
gun covers, insetting seat and bunk pieces, and drawing each ladder on its lower
deck only. Federation service bands are open side strips offset by 4 cm, avoiding
caps that overlapped the adjacent hull. The original Federation room layouts and
silhouettes remain intact. All twenty hull and deck states across both collections
have zero measured coplanar surface overlap.

The final review and reports are under `docs/screenshots/grabon-concepts/` and
`docs/screenshots/federation-concepts/`. Each viewer links to the other collection.
Direct cutaway URLs include the ship, stage, deck and view mode.
