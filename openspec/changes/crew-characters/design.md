# Design: original cartoon crew

Written before the generator, 2026-10-10. The owner's request authorizes building
the models; screenshot approval separately authorizes replacing figures aboard.

## Source and shape

### Narrower body revision, requested 2026-10-10

The owner finds the body too wide in the current review. Reduce the shared
shoulder-width multiplier from 1.16 to 0.90, about 22% less upper-torso width,
and the waist multiplier from 0.86 to 0.80, about 7% less waist width.
Concentrate the reduction on the overly broad torso and retain the lower-body
profile and spacing. Rebuild garments and joint locations from these existing data values for all
twelve variants. Preserve the exaggerated head, hand and foot sizes.

Capture a before/after comparison and refresh the viewer, rigs and review
screenshots. Reuse the exported coverage, intersection, pose and budget checks.
This changes dimensions only, with no additional triangles, bones or draws.
The models remain pending screenshot approval and ship occupants stay blocky.

### Anatomy repair and paired variants, requested 2026-10-10

The owner rejected the detached limb appearance, hoof-shaped boots, trousers
crossing the tunic hem and scalp breaking through the bob hair. The earlier
covered-skin test did not detect these garment and hair intersections.

Replace the assembled torso/sleeves with a continuous welded tunic surface,
including shoulder and underarm bridges. Build trousers as one continuous pelvis,
crotch and two shaped legs. Use anatomical profiles for deltoids, upper arms,
elbows, forearms, thighs, knees, calves and ankles. Covered torso and limb skin stays
inside the continuous garment surfaces. Shared waist measurements and
weights keep the tunic hem outside the trouser waist in rest and review poses.
Boots need an asymmetric human footprint, a rounded toe box, instep, narrow ankle,
heel and thin sole, with their size and profile in data.

Fit the hair shell to the actual faceted head surface with a data-held clearance,
including the forehead cutout and side/back length. Remove intersecting add-on
hair locks. Add adult male and female variants for every department, using shared
anatomical topology and data-held shoulder, chest, waist and hip proportions.
Retain the cartoon head/eye ratios, original uniforms, thirty exported bones,
one material/draw and the 3,000-triangle ceiling for every variant.

Validate connected garment components in the exported file, inspect the waist,
shoulders, hips, footwear and hair at rest and in the named bent poses, and add
regressions for the rejected geometry. The viewer selects male/female variants
and compares all six departments. Rebuild the editable rigs and captures for all
twelve models. All remain pending; no ship occupants are replaced.

`data/crew/characters.json` owns proportions, body dimensions in metres, palette,
eye ratios and twelve character rows. Connected topology, hair clearance and male/female
anatomy multipliers live in `data/crew/anatomy.json`. Department colours come directly from
`data/crew/company.json`, never a second palette. The reference is the owner's
written brief: original cartoon shapes, no actor likeness or franchise insignia.
The two cards themselves are not present in this session; their supplied ratios
are the source, not a claim of pixel measurement or an exact visual match.

Use broad shoulders, a tapered waist, a large softly faceted head, a short thick
neck, heavy brows, large hands and chunky boots under short legs. Hair and
skin vary by row. Nose and mouth stay simple. A small rectangular badge with two
inset bars is original, with no franchise symbol. A shared builder varies bodies
through data. The copied humanoid skill's MakeHuman adult silhouettes and 53-bone
rig do not fit this brief, so use an original low-poly loft builder instead.

## Rig upgrade requested 2026-10-10

The owner likes the character and requests a full rig like the MakeHuman Blender
workflow. The existing twenty-bone rest rig has insufficient elbow loops and
largely rigid torso weights, so merely exposing its bones is not enough.

Build an animation-ready Blender control rig around the original cartoon shape.
Keep the owner's 30-bone game-file ceiling: twenty body bones plus one curl bone
for each of ten separate fingers. This is a compact cartoon hand, with one hinge
per finger rather than the three phalanges of MPFB's 53-bone game_engine rig.
Root, pelvis, spine, chest, neck, head, clavicles, arms, wrists, thighs, knees and
feet are all poseable. Facial expressions are outside this body-rig step.

Eight non-deforming Blender controls provide hand/foot IK targets and elbow/knee
poles. Per-limb IK/FK switches, custom control shapes, named bone collections and
finger curl limits make the saved `.blend` usable for posing. These authoring controls
are excluded from the exported skin; the GLB still contains at most 30 joints,
one material and one avatar draw. Save an editable `.blend` for both variants of every department.

Add elbow/knee loops and consistent torso, garment and limb weights. Separate
fingers replace the mitten tips within the 3,000-triangle ceiling. Rig settings,
hand dimensions, joint blend spans and shared review poses/clips live in data.
Export baked idle, walk, seated and reaching/waving examples, and play those
actual GLB animation tracks in the HTML viewer with pause/scrub and a skeleton
overlay. The viewport must frame deformed geometry and retain probe lighting.

Validation reads the actual exported hierarchy, weights, normalized influence
sums, animation tracks and budget; proves hand/foot IK targets move their limbs;
and checks clothing coverage in the named stress poses. Render exported poses,
inspect them and verify deterministic GLB rebuilding before updating the review
package. The source rig may have 38 bones including controls; the game skin is
limited to 30. No engine adoption is part of this rig request.

The local reference is `.claude/skills/blender-humanoid-characters/references/
rig-and-animation.md`: MakeHuman's game_engine hierarchy and the shared animation
workflow. This implementation keeps the original cartoon mesh and its stricter
budget; it does not claim to be an MPFB-generated human.

## File and measurements

One skinned mesh, one opaque diffuse palette material and one indexed triangle
primitive per GLB. Vertex colour holds albedo, normals stay available for probe
lighting. No emission, unlit extension, texture, PBR effects or face bones.
The generator imports the existing `hs_kit.read_glb` rather than copying its reader.
Unused UV0 tags identify geometric parts for artifact measurements and containment
checks; they do not add a material or a draw. The hierarchy has thirty deform
bones, with five shared baked clips. Blender's eight authoring controls are
excluded from the GLB. Tracks begin at zero seconds so browser playback and
scrubbing use the same duration as the exported file measurements.

Each eye is about 0.20 of the head width. A domed eye surface has explicit edge
loops at the sclera, iris and pupil boundaries. Initial iris width / eye width
is 0.55; pupil width / iris width is 0.40. The validator measures extrema of each
region's vertices in the exported mesh, not the input settings or a painted mask.

Clothes use closed lofts with separate hem, cuff, collar and trouser-leg edge
loops. Hidden torso, arm and leg skin remains under the clothes. After export,
the checker samples their vertices, triangle centroids and edge midpoints and
requires them inside a garment shell. Renders from front, both sides, back and
three-quarter are the complementary visual check. The same containment check
runs on six samples per exported clip. This proves the sampled poses; arbitrary
animation poses still need review.

## Review and approval

All rows begin `pending`. Build output is in `assets/models/crew_review/`, outside
the deck compiler's figure path. The compiler and client still use
`sc-tools::figure` from `crew-npcs` section 7. File presence, a successful build,
or a blank review answer never counts as approval. Only an explicit owner decision
on named screenshots can authorize adoption. No character is approved by this task.

Render the written GLBs in headless Blender with CPU Cycles. Save a department
lineup, individual turnarounds and a close-up of the eyes. Also show an offline
ambient-cube response in normal, red-alert and emergency light. That preview
multiplies palette albedo by normal-dependent six-face irradiance. It checks the
asset's diffuse response; live deck probe selection/interpolation and engine
adoption are still proposed and must not be marked implemented.

## The Pi 5 budget

The shared source is `engine-stack` section 5: 3,000 triangles per avatar,
200,000 visible triangles, 300 draws and 64 MB of vertex/index buffers. The owner
tightens this asset to 30 bones, one material and one primitive (one avatar draw).
Read all counts from the written file and refuse the character by name and count
if any ceiling is exceeded. Worst case nine visible bots spend 27,000 triangles
and nine draws. At a conservative 80 bytes per exported vertex and 4 bytes per
index, a fully split 3,000-triangle avatar is under 756 KB. The manifest reports
actual binary/file sizes and exported vertices. Review images and editable
Blender files are offline artifacts and spend no client memory. Runtime costs
stay unchanged until adoption. No desktop capture measures Pi performance.

## Validation

Build all rows; compare a second build byte for byte; read back GLB counts and eye
ratios; reject a deliberately over-budget fixture; check closed garment coverage;
render exported files and inspect the captures; run OpenSpec and the dash check.

## Open questions

| # | Decision | State |
| --- | --- | --- |
| CC1 | Original body and uniform shape, after screenshots | Pending explicit owner approval |
| CC2 | First iris ratio 0.55 | Recommendation taken (ask only with screenshots); adjustable in data |

## Built review evidence (2026-10-10)

Twelve pending GLBs are built in `assets/models/crew_review/`: a male and female
for each of six departments. Each exported file contains 2,940 triangles, 30
bones, one material and one indexed primitive. Both eyes measure 0.550000
iris/eye and 0.400000 pupil/iris.

The exported tunic and trousers are each one connected closed surface. At rest
and in all 360 exported pose samples, the checks find no garment self
intersections, visible tunic/trouser intersections or hair/scalp intersections.
The waist check excludes internal horizontal closure caps, identified in the
bind pose. Each sample also checks 1,972 covered-skin points. These sampled
motions pass; arbitrary poses still require review.

The twelve source `.blend` files pass all 48 hand/foot IK target and pole checks.
Exported skin weights are normalized with at most three influences. All five
clips preserve their loops and keyed floor contact. A second build reproduces
every GLB and the measured manifest byte for byte; editable `.blend` binary
identity is not claimed.

The twelve animated files total 6,053,176 bytes; the largest binary buffer
is 397,888 bytes including animation tracks. These are artifact sizes, not
measured runtime allocations. A skeleton overlay adds one review draw per visible
avatar. Runtime ship geometry is unchanged pending approval.

Eight deliberately invalid exported files are rejected: excess triangles,
incorrect iris and pupil ratios, covered skin outside the tunic, an extra
material, hair intersecting the scalp, trousers crossing the hem, and a closed
but disconnected garment island. Results and artifact hashes are in
`docs/screenshots/crew-characters/validation.json` and `rig-validation.json`.

The review package and regeneration commands are
`assets/models/crew_review/README.md`. Screenshot approval, runtime selection,
live deck-probe interpolation and Pi measurements remain
pending. No engine, deck compiler or ship-occupant mockup code was changed.

## Interactive review page (2026-10-10)

The owner's request for an HTML 3D view adds a local three.js review page at
`docs/mockups/crew-characters.html`. It carries the actual Blender-built GLBs and
their measured manifest through the shared inliner. It supports selecting each
department and male/female body, a six-character lineup for either body, orbit/pan/zoom, front/side/back/eye views and
wireframe inspection, five baked clips with play/pause and scrubbing, and a
skeleton overlay. All characters remain pending; opening or interacting with
the page cannot approve them or replace ship occupants.

The page reads department names and colours from `data/crew/company.json`, and
the illustrative ambient cubes from `data/crew/characters.json`. A vertex shader
evaluates squared normal components against the six cube faces in world axes,
after skinning, then multiplies the exported linear albedo. Normal, red-alert and emergency are
review lighting states, not live compiled deck probes. Orbiting the camera must
not rotate the probe light. There are no shadow maps or per-pixel lights.

Shared ShipKit chrome supplies closable panels, screenshot hooks and a measured
frame budget from `renderer.info`. The asset panel shows file counts separately
from visible frame costs; a lineup spends six avatar draws. Desktop rendering
does not establish Pi performance. Assets and data are inlined so double-click
opening needs only the pinned three.js CDN, with no local asset server.

Validate disk opening, all twelve selections, camera presets, lighting and wireframe
controls, mobile layout, manifest freshness and the visible triangle/draw counts.
Capture named views with the repository screenshot tool and inspect them before
handing over the page. Approval and engine integration stay pending.

Browser verification opens the page from disk in headless Chromium. Every
selection renders the character's manifest triangles plus the 192-triangle
plinth, in two draws (one avatar and one plinth). The lineup is 17,832 triangles
in seven draws. Downloads match the original GLB bytes. All camera presets,
orbit, pan, zoom, auto rotation, wireframe, three lighting states and panel
close/restore pass. Actual skinned vertex positions verify clip deformation,
playback, pause, reverse scrubbing, loop closure and returning to rest. Skeleton
visibility adds the expected one draw per avatar. The 390 x 844 phone controls fit within the viewport; extra
panels start closed. The garment and scalp intersection checks read the actual exported triangles. Desktop and phone captures are inspected for this local view.

`docs/screenshots/crew-characters/viewer-validation.json` records the browser
checks. The review page's INLINE blocks are current and the shared budget matches
its source. The inliner's global cache audit still fails on the pre-existing
`docs/mockups/cache/deck-plan-bake.bin`, whose baker hash differs from the current
`lightbake.js`. The character viewer does not use that deck cache. It remains
untouched by this task; no passing repository-wide inline audit is claimed.

The final rig review is recorded in `docs/screenshots/crew-characters/rig-review.json`.
The separate rig report ties each saved Blender file and exported GLB to its hash.
Checks are scoped to the character files; unrelated working-tree edits remain outside this change.
