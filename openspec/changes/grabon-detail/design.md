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
engine-stack/design.md. Target at most 18,000 triangles per detailed hull,
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

## Aft propulsion and exposed structure revision

Owner review requests central impulse exhausts, removal of Shrike's projecting
engine boxes, mirrored aft detailing, and more connected armor breaks like Tern.
Keep the occupied hull envelopes and purple/orange paint. Fit a shallow central
impulse cassette to the aft command tier on Shrike and Vark and the engineering
tier on Gorath. Gorath's exhaust axis is at x=0 m, y=5.2 m. Keep hangar doors
below the exhaust and Vark/Gorath's outboard pods. Remove Shrike's two old boxes.
Use mirrored emitter cells, graphite throats and steel heat vanes, rather than
painting an exhaust over a hangar door. Mirror the outboard pods' UV finish too.

Cut continuous 1.10 m service belts into the body, command, engineering and head
armor. Recess their graphite backing 0.26 m. Two longitudinal roof channels on
the command/engineering tier turn down its aft face and meet that belt. Keep
machinery within the original armor envelope, with angular ends, small pipes,
couplings and junction boxes. Existing occupied interior dimensions stay intact.
These are exterior review shells, not interior collision meshes.

Restore the original UVs onto editable source assemblies before CSG, so the
generated paint survives unchanged on remaining armor. Use existing Material
Maker graphite for the newly modeled machinery. All cuts are authored in hull
space. Retain named cutters and source assemblies in the finished Blender file.
The revised export may use four materials, at most 18,000 triangles and the
existing atlas plus one 128 px material texture. Inspect actual exported GLBs
from the aft, side and upper quarter; check symmetry, centers and overlaps.

## Gorath lower deck inset

Reduce Gorath's body footprint to 80 percent of its former width and length,
then move it 0.8 m forward. The bottom tier becomes 20.16 m wide and 28.24 m
long, wholly inside the engineering tier above (25.6 m wide and 32.1 m long).
Keep its height, centered upper impulse, access shafts and upper decks intact.
Refit lower rooms against the revised envelope with the existing 0.85 m margin.
Move the two full-size fighters inward and forward inside the shortened hangar.

Apply the same footprint transform to the native lower armor and hangar fittings
after restoring their painted UVs; rebuild its recessed service belt from the
new footprint. Remove roof equipment that the overhanging upper hull encloses.
No extra materials or textures are required for the footprint revision. Keep
four draws and measured texture allocation. Inspect the exterior
and lower deck, and check room fit, access, mesh integrity and coplanar overlap.

## Grabon interior identity and windows

The current detailed ships reuse Federation working-space panels and furniture,
and their exported hulls have no openings onto those rooms. The owner requests
distinct Grabon interiors and windows using the Claude texture workflow.

Use the panel-textures skill's existing Blender pipeline: modeled relief and
CSG recesses, orthographic Cycles beauty and emission renders, linear-light
downsampling, a 48-color palette and 128/256 px layers. Add a Grabon catalogue
that inherits shared placement and bake rules while supplying its own modeled
panel designs and palette. Keep Federation assets byte-identical. Grabon walls
use interlocking plum armor, diagonal braces, bronze clamps, recessed pipe banks
and orange lamps. Ceilings use angular crossmembers and narrow light channels;
floors use broad charcoal tread plates and inset amber route markers. Add faction
console and seat silhouettes, rather than relying on recolored Federation props.

Keep the approved ship silhouettes, including Gorath's inset bottom tier.
Derive symmetric window pairs from actual outboard room walls. Maintain a shared
eye-level row on each deck, narrow glazing and shallow reveals. Fit window rooms
to the pressure envelope where needed; preserve room access and full crew scale.
The same portal records cut interior walls and native exterior armor, including
the recessed subhull. Hollow the exterior shells and reserve armor around each
opening so pipes cannot cross the view. Inspect actual GLB openings with rays.
Sample each aperture and frame from 2 m outside the hull, within the space
between the hull and Gorath's outboard wing fittings. Clear attached service
hardware through that approach. Distant wings remain visible outside the glass.
Union the overlapping hull tiers into one pressure skin before carving the
joined room air volumes. Separate hollow tiers leave overlapping interior caps.
When furnished rooms are present, reuse their corridor floors and ladders;
the isolated access presentation supplies those surfaces only for its own view.

Use the same furnished room builder inside the exterior preview and in cutaways.
Provide window close views and an interior view with ceilings. Fixture positions
and colors come from Grabon detail data. Reuse the shared lighting and panel
renderers; compare normal and red-alert states. No per-pixel lights, PBR or
normal maps. The texture pack replaces the Federation pack in this viewer;
keep the 96 MiB texture and 200,000 triangle scene ceilings, 120 draws per deck,
and 18,000 exterior triangles per ship. The additional 2,000-triangle hull
reservation covers fitted window frames and joined service hardware at their
openings. Validate measured costs against the unchanged scene ceilings.

Before delivery, inspect the panel contact sheet, all three hulls through their
windows, furnished deck cutaways and eye-level rooms. Check portals, furniture,
symmetry, unobstructed window rays, UVs, emission, mesh integrity and overlaps.
Rebuild changed assets twice and compare their hashes; inline and push PR 25.

The room preview reuses ShipKit's direct vertex-light review path, with 1 m
subdivision on room surfaces. Cycles supplies panel relief shadows, wear and
emission. This pass does not claim the engine's planned occlusion and bounce
bake. The existing FreeCam owns walking and eye-level looking; the room's
brushes and furniture footprints constrain movement at a 1.65 m eye height.

## Review results before the window pass

The revised hulls have 9,728, 12,328 and 13,712 triangles, each in four draws.
The detailed viewer allocates 59.08 MiB of texture storage including mipmaps.
The layouts have 16, 18 and 24 connected rooms, including access shafts, and
retain two fighter berths each. Ladders have a 1.30 m corridor bypass. Shaft
ends meet the existing z=-2 m bulkhead so no inaccessible room slivers remain.
Ladder portals are centered inside the 0.8 m structural slabs; browser checks
verify an actual floor opening and solid adjacent bypass on each upper deck.

The overlap check covers each hull's individual triangles, every deck and the
three access views. The final 13 states have zero coplanar overlap. The browser
check verifies finite geometry, budgets, exported sun lighting, elevator travel
and phone controls. Captures and reports live in docs/screenshots/grabon-detail.

## Ceiling visibility correction

The deck cutaway currently removes ceiling faces, leaving lamps suspended over
open rooms when the viewer moves below the roof. Retain the existing textured,
downward-facing ceiling panels in detailed cutaways. Their front-side material
shows them from inside and culls them from above, preserving the deck overview.
Keep the existing cutaway treatment of beams and coves. No new textures, draws
or runtime lights are needed; measure the added ceiling triangles against the
existing 60,000-triangle deck check. Verify ceilings face inward on every deck,
then inspect interior and overhead captures in normal and red-alert lighting.

The correction retains inward-facing ceiling panels in all 66 deck room slices
across seven decks. The largest deck now submits 17,552 triangles, still at
15 draws and 55.75 MiB of textures. Nine captures cover the three overhead
views and all three bridges in normal and red-alert lighting.

## Window and interior review results

The delivered hulls have 16 / 14 / 20 real windows and 10,870 / 13,566 / 16,406
triangles. All 1,150 aperture, frame and neighboring-skin rays pass against the
exported GLBs. Deck rows share a 1.45 m center and 0.66 m height. All native
meshes are closed, with finite UVs and colors, mirrored geometry and four materials.

The Grabon collection has 61 panel layers at two resolutions and four UI images.
All 126 PNG hashes match an independent Cycles render. Its four furniture props
and all three native hull GLBs also pass identical-hash rebuilds. Existing
Federation panels and furniture are unchanged.

With furnished rooms included, the largest hull view is 68,218 triangles and
33 draws; the largest deck is 13,495 triangles and the deck maximum is 15 draws.
Texture storage is 55.75 MiB including mipmaps. Sixteen overlap states cover the
native exteriors, full furnished interiors, all seven decks and access assemblies;
all have zero measured coplanar overlap. Room views retain ceilings, provide a
65-degree field of view and place the walking camera 1.65 m above the floor.
