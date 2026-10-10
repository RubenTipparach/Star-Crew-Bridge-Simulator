## Exterior design assets

- [x] Author the player attachments and two companion silhouette profiles.
- [x] Build textured Tern GLBs with two warp pylons and outboard nacelles, plus medium and distant models.
- [x] Build the two companion exterior concept models.
- [x] Save an editable Blender scene and a manifest with measured costs.
- [x] Add a three.js inspection mockup that loads the actual GLBs.
- [x] Capture and inspect the designs, including the underside and warp assemblies.
- [x] Check geometry, byte reproducibility, OpenSpec, layouts and the new page's inline blocks. The global inliner reports an existing stale deck lighting cache, recorded in design.md.
- [x] Record results and link the assets from the design hub.

## Owner revision, 2026-10-10

- [x] Place pylon roots, tips and nacelle centres at the hull's middle height.
- [x] Increase nacelle length to 56 m and lengthen the swept pylon chords.
- [x] Generate original paint art, import it into Material Maker and replace the exterior grid materials.
- [x] Bake separate medium and distant atlases with emission masks and preserved high-detail markings.
- [x] Rebuild and inspect all captures, including a side view and lower-detail dorsal views.
- [x] Verify revised bounds, texture provenance, draw counts and reproducibility.

## Owner revision, detailed surface artwork

- [x] Generate a detailed original surface atlas using the owner's image as a surface reference.
- [x] Preserve the generated source and prompt in an editable Material Maker input graph.
- [x] Fit distinct dorsal, ventral, side and mechanical regions to the exported assemblies.
- [x] Bake the updated design into both lower-detail textures and validate exported assets.
- [x] Review the actual updated models in all inspection views and record measured texture costs.

## Owner revision, painted UV layout and interior windows

- [x] Inspect the owner's Fallen Tribes UV textures and record the reference.
- [x] Reshape the central hull's exterior armour while preserving the approved pylons and interior envelope.
- [x] Export the actual UV wireframe, paint that template, and rebuild all Tern LODs.
- [x] Align genuine hull openings and reveals with the existing command-suite windows.
- [x] Display correctly scaled interiors, furniture and crew in the exterior HTML preview.
- [x] Review window sightlines, cutaway, interior lighting and exterior captures; validate budgets and assets.

## Owner correction: fins and false windows

- [x] Remove the command shoulder blades and crown from all detail levels.
- [x] Remove painted window shapes from the side UV island with an image edit.
- [x] Fit physical window rims to the actual portal/hull intersections.
- [x] Rebuild assets and inspect oblique window close-ups, silhouette and LODs.
- [x] Validate delivered geometry, inlining and reproducibility; update the measured counts.

## Owner correction: shallow window returns

- [x] Fit the forward upper hull to the real command-room outlines with 0.32 m clearance.
- [x] Make window cutters, frames and liners follow that surface without visible gaps or texture strips.
- [x] Rebuild full and reduced models, UV guide and preview; inspect every window from both sides.
- [x] Verify shallow window depth, interior clearance, frame rays, overlap, budgets and reproducibility.

## Owner revision: windows on all decks

- [x] Author and validate the lower-deck window layout patch and room furnishings.
- [x] Cut and frame the new openings, then bake their detail into both LOD textures.
- [x] Add furnished lower-deck rooms and deck/window inspection controls to the preview.
- [x] Review every new opening and all deck cutaways; verify reproducibility, visibility, overlap and budgets.

## Owner correction: broad bow

- [x] Replace the pointed forward sections and fit exterior sensor and missile mouths to the new face.
- [x] Rebuild the UV guide, all Tern models and both LOD textures; check room containment and window sightlines.
- [x] Inspect side, top, bow and LOD captures; verify reproducibility, overlap and budgets.

## Owner revision: rounded bow and quieter paint

- [x] Derive the lower bow from the bridge outline and soften its front rim without clipping rooms.
- [x] Export the revised UV guide and generate a quieter hull/fittings painting through imagegen and Material Maker.
- [x] Rebuild all LODs, inspect the ship and window views, and validate reproduction, overlap and budgets.

## Owner revision: angular armor and liveries

- [x] Model shallow armor breaks and remove the raised registration plate.
- [x] Paint angular armor over a grey subhull in three UV-guided liveries and render their Material Maker graphs.
- [x] Rebuild all livery LODs and add selection to the inspection page.
- [x] Inspect livery, bow, lettering and window captures; validate geometry, reproduction and budgets.

## Owner correction: continuous mechanical subhull

- [x] Replace disconnected grey side regions with continuous recessed service bands.
- [x] Add pipes, couplings and mechanical details; repaint all three UV-guided liveries.
- [x] Rebuild all detail levels and verify windows, skin depth, geometry, budgets and reproducibility.
- [x] Inspect updated close-ups and livery/LOD captures in the preview.

## Owner revision: cyan blue livery

- [x] Repaint the blue livery cyan and retain the generated source and edit prompt.
- [x] Match fittings, rebake both LOD textures and make Cyan the preferred preview livery.
- [x] Inspect the cyan ship at all detail levels and verify exports, controls and budgets.

## Owner correction: connected secondary structure

- [x] Paint aligned grey returns from roof channels across walls and bow corners.
- [x] Update all liveries and bake both lower-detail textures.
- [x] Inspect both sides and forward joins; verify windows, model exports and budgets.

## Owner correction: centered mess windows

- [x] Lower the shared mess window centers and fit their exterior frames to the service recess.
- [x] Rebuild all liveries and LODs; inspect the mess from inside and outside.
- [x] Verify the actual window heights, clear openings and matching interior portals.

## Owner correction: armor coverage

- [x] Restore broad armor panels and narrow the exposed grey connections in the UV painting.
- [x] Inspect the Cyan full-detail result, update all liveries and rebake LODs.
- [x] Review final captures and verify window alignment, reproduction and budgets.

## Owner correction: plated sides and projected livery

- [x] Replace wide side gaps and stretched UVs with ordinary plated walls.
- [x] Project seams and wrapping stripes on the 3D hull; retain and inspect the unenhanced bake.
- [x] Enhance the surface finish, update all liveries and rebake both LODs.
- [x] Validate windows, UV continuity, reproducible models and final captures.

## Owner correction: aligned roof and wall seams

- [x] Replace old painted roof seams with the shared projected station layout.
- [x] Inspect shoulder joins in 3D and constrain enhancement to the baked boundaries.
- [x] Update all liveries and LOD textures; verify shared-edge samples and captures.

## Owner refinement: clean connected paint and equator

- [x] Complete the cyan roof segment and lock clean paint edges to the 3D mask.
- [x] Restore a narrow equatorial subhull band without obstructing windows.
- [x] Review final seam, paint and equator views; rebake and verify every livery/LOD.

## Separate engine implementation

- [ ] Review the exterior art with the owner.
- [ ] Load these exterior assets in the Rust renderer in a separate implementation request.
- [ ] Update weapon occlusion and shields to account for the chosen attachments.
- [ ] Measure exterior rendering and memory on a Pi 5.
