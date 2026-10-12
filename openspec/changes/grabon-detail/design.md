# Detailed Grabon assets

## Sources and scope

The approved source is grabon-concepts/layouts.json at 82e8d0c. Main was fetched
and is contained in this branch. A deterministic builder derives a detailed
catalogue, preserving the original plans for comparison. Shared ShipConcepts and
ShipKit functions remain authoritative for hull geometry, rooms and portals.
The detailed viewer uses an optional extension, leaving rough and Federation
viewers intact. Blender packages the actual generated meshes, UVs and textures.

## Deck access

Carve a 4.5 x 4.3 m stair tower and a 3.2 x 3.0 m elevator shaft beside each
2.4 m corridor. Their XZ footprints in ship coordinates are identical across
decks. Split neighboring convex brushes around the shafts and relocate doors
only where their former opening is removed. Both shafts have a corridor door on
every deck. Retain the independent central ladder with real slab openings.

Use the shared spiral stair builder: 20 risers per 3.6 m deck (0.18 m rise),
1.7 m outside radius, 0.12 m center post and 270 degree sweep. At a 1.25 m
walking radius the going is about 0.31 m. Provide landings and continuous
handrails. The lift car is 2.6 x 2.3 m with 2.2 m clear height, opening toward
the corridor, with guide rails and landing thresholds. A preview control calls
the car to each actual floor. This is a visual mechanism, not runtime simulation.
An access view exposes the shafts and deck levels together.

## Hull and materials

Keep each hull's occupied envelopes and approved outline. Add restrained armor
breaks, service runs, hatch frames, engine collars, vents and weapon fittings.
Project panel seams and livery boundaries on the actual surfaces before baking
UV guides. Save the guide, region masks, editable projection and UV wireframe.
Enhance these locked guides using image generation for paint variation, wear and
machinery detail. Preserve orange bands and broad quiet purple plates. Keep
textures separate from fixed sun vertex lighting. Use baked color and emission,
with no PBR or normal maps. Export the assets and editable Blender source.

Dress interior shells with the existing Blender-baked wall, floor and trim
panels and use the repository's modeled furnishings. Doors, landings and floor
holes follow the layout rather than independent decorative geometry. Lighting
uses the room fixtures and shared bake, with normal and red-alert review states.

## Budget and verification

The scene ceiling remains 200,000 triangles, 300 draws and 96 MiB textures from
engine-stack/design.md. Target at most 16,000 triangles per detailed hull,
60,000 per visible deck and 120 draws. The projection guides are 2048 px; image
generation returned 1254 px enhanced atlases, each costing 7.998 MiB with mips.
Only the selected hull's texture is resident; panel textures
and props must keep the measured viewer below 96 MiB. Report actual costs in the
HUD and manifest. No Pi performance claim or engine workload is added.

Validate brush overlap, reachable rooms, shaft alignment, door fit, stair rise,
going and headroom, lift travel and ladder holes. Measure exported meshes and
texture allocation. Run the shared coplanar overlap checker after geometry
changes. Capture each hull, every deck, access assemblies, close material views
and phone views. Inspect screenshots before delivery. Validate OpenSpec and
inlining before pushing the existing review branch and PR.

## Review results

The three exported hulls have 7,232, 7,360 and 8,312 triangles, each in one draw.
The detailed viewer allocates 59.00 MiB of texture storage including mipmaps.
The layouts have 16, 18 and 24 connected rooms, including access shafts, and
retain two fighter berths each. Ladders have a 1.30 m corridor bypass. Shaft
ends meet the existing z=-2 m bulkhead so no inaccessible room slivers remain.
Ladder portals are centered inside the 0.8 m structural slabs; browser checks
verify an actual floor opening and solid adjacent bypass on each upper deck.

The overlap check covers each hull's individual triangles, every deck and the
three access views. The final 13 states have zero coplanar overlap. The browser
check verifies finite geometry, budgets, exported sun lighting, elevator travel
and phone controls. Captures and reports live in docs/screenshots/grabon-detail.
