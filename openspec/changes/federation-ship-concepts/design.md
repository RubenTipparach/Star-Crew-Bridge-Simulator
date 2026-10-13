# Three studies, one review

Owner classification, 2026-10-12: these three earlier Hound studies are retained
as Federation concepts. Their original brief and measurements below record the
study history. Grabon concepts now have a separate review.

## Design sequence

1. Define the mission, crew, decks, room areas and connected circulation. Save
   the initial compartment polygons and capture all three plans.
2. Wrap the occupied volume in a distinct hull. Pursuer concentrates machinery
   aft and carries long recessed gun shoulders; Splitjaw carries two forward
   launch jaws around a sheltered service pocket; Bulwark has a transverse
   forward shield and separated aft engine blocks. Capture all three hulls.
3. Adjust the outer room edges to the hull's usable inner envelope, retain door
   access and human clearances, and record what moved. Capture fitted plans and
   cutaways. Keep the original polygons visible as a comparison option.

Recommendation taken (ask only with screenshots): these are three alternatives for the existing Hound, based on main at 7653f7a and the Hound/Jackal mockups. They retain the same combat functions, with different room arrangements and silhouettes. Pursuer is compact, Splitjaw prioritizes
flight circulation, and Bulwark gives its magazines, guns and reactor space.

## Source and geometry

`data/ships/federation-concepts/layouts.json` is the authoritative concept catalogue.
Each entry holds a `starcrew.ship-layout/2` layout, the retained first-pass
brushes, hull envelopes, fittings, scale references and adjustment notes. These
are study layouts, not an additional copy of an engine ship.

`docs/mockups/lib/shipconcepts.js` interprets this catalogue for every view.
Room shells and portal cuts use ShipKit; plan diagrams use the same polygons
and portals. No room is positioned in HTML. Hull panels use explicit metre-space
polygons and plain Material Maker paint/machinery layers. There are no generated
illustrations, detailed UV atlases, false windows or decorative raised lettering.
Windows and final LOD artwork belong to a later production pass.

Use +X port, +Y up, +Z bow, metres. Minimum deck headroom is 2.8 m, deck spacing 3.6 m,
main passages are 2.4 m wide (Splitjaw's short flight cross-passage is 2.0 m), and doors are 1.2 x 2.1 m. A 1.8 m figure,
2.0 m bunk and 5 m plan scale make room size reviewable. Hull-to-air clearance
is at least 0.45 m, separate from bevels and non-habitable machinery volumes.

The measurement instrument checks actual catalogue polygons for convexity,
overlap, containment, portal alignment, connected room routes, deck clearances,
retained initial plans and changed fitted rooms. It reports area and volume.
The browser check inspects all three designs and all stages, budgets, controls,
mobile layout, console errors and coplanar overlap in visible cutaways and hulls.

## Review and budget

One self-contained HTML file inlines source data, shared libraries, fonts and
rough textures; Three.js 0.169.0 is loaded from the repository's pinned CDN.
URL fragments select a ship and stage. The viewer offers normal and red-alert
interior light, room selection, top/side/orbit views, and closable information.

Concept ceiling per visible ship: 12,000 triangles, 120 draw calls and 4 MiB
textures, measured from the actual renderer including the scale figure. The
scene ceilings remain 200,000 triangles, 300 draw calls and 96 MiB. These are
design counts, not a Pi 5 speed measurement. No runtime memory, simulation,
network traffic or render targets are added to the game.

## Scope after review

The owner can choose, combine or redirect these studies. Production Blender
models, detailed projected UV painting, LODs, furnished walkable interiors and
enemy behavior require the next design pass.

## Existing enemy sources

- `data/ships/hound/layout.json`: seven combat damage rooms and their systems.
- `data/enemies.json`: twin turrets, two missile tubes and side laser banks.
- `data/fighters.json`: two Jackals launched from the Hound.
- `crates/sc-client/src/ships3d.rs`: 8 m wide, 8 m long Jackal geometry, plus the current Hound's swept shoulders and paired engines.
- `docs/mockups/exterior.html`: earlier Hound and Jackal combat silhouettes.

Each concept reserves two 8 x 8 m fighter footprints, 3.2 m hangar headroom,
service clearance and a launch face. This explains the larger habitable envelopes
than the current 30 m damage proxy. Hit bounds and balance stay with the current
game until a hull is selected and its gameplay implications are reviewed.

## Review results (2026-10-12)

The three first plans were captured before their hull pass. The fitted pass then
changed 10 Pursuer rooms, 7 Splitjaw rooms and 9 Bulwark rooms. All 49 rooms are
reachable from their bridge, their door openings lie on shared walls with the
correct normals, and their fitted air keeps at least 0.50 m from the outer hull.
Both fighter footprints and the simple scale furnishings fit each design.

| Study | Actual exterior length x beam | Decks | Rooms | Hull triangles / draws |
|---|---|---|---|---|
| Pursuer | 45.1 x 29.2 m | 2 | 15 | 1,532 / 1 |
| Splitjaw | 53.3 x 33.0 m | 2 | 16 | 1,536 / 1 |
| Bulwark | 45.5 x 38.1 m | 3 | 18 | 1,652 / 1 |

The largest visible deck cutaway is 2,828 triangles and 12 draws. The shared
texture array occupies 1.0834 MiB including mipmaps. These are actual browser
renderer counts; SVG plans are diagram UI and do not enter the Three.js count.
All ten hull/deck geometry states have 0.000 m2 coplanar surface overlap under the
repository checker. Desktop and 390 x 844 phone controls and framing were checked.

The review is `docs/mockups/federation-ships.html`. Captures and the layout, browser
and coplanar-overlap reports are in `docs/screenshots/federation-concepts/`.
The first-plan captures are retained as stage evidence; `review/` contains the
current finished mockup. Source data and tool changes are confined to concept
review tooling. No enemy tuning, production model or engine source was changed.

The faction refiling also removes hidden service-band caps, separates coplanar
rough prop faces and draws each ladder on its lower deck. Component spans are
retained in merged meshes for the surface checker. Current counts above and the
refreshed reports include these corrections; original room polygons are unchanged.
