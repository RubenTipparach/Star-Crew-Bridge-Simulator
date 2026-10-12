# Design: original cartoon crew

Written before the generator, 2026-10-10. The owner's request authorizes building
the models; screenshot approval separately authorizes replacing figures aboard.

Current status, 2026-10-10: game adoption is on hold at the owner's request.
The complete review package is retained. A later request to resume adoption and
explicit screenshot approval are required before replacing crew aboard.

## Source and shape

### Shoulder edge flow repair, requested 2026-10-10

The owner identified the angular inner-arm notch that the previous visual review
missed. The sleeve is welded, but its first ring meets a rectangular torso opening
in one coarse strip. The lower opening has a sharp corner and every attachment
vertex carries the same chest/upper-arm blend. Connectivity and intersection
tests alone do not establish a useful deformation surface.

Primary references consulted for this repair:
- [Autodesk: Checking the model topology](https://www.autodesk.com/learn/ondemand/tutorial/checking-the-model-topology):
  follow the natural muscle lines and place enough loops around bending regions
  to avoid sharp, stretched surfaces.
- [Autodesk: Weighting the Character's Upper Half](https://download.autodesk.com/us/3dsmax/2012tutorials/files/GUID-5FC55910-D704-47E8-A11A-2DC9A2B8BA3-86.htm):
  distribute shoulder influence between collarbone and upper arm, retain even
  loop spacing, and work through several poses when adjusting weights.

Add two eight-vertex socket loops per shoulder, shape and locally relax the
armhole/inner-arm transition, and retain quad strips in the authoring mesh.
Grade chest, clavicle and upper-arm weights around the opening so the underarm
does not move as one uniformly weighted ring. Keep the hidden arm fitted inside
the resulting sleeve. Topology and weighting controls remain in anatomy data.
Remove one redundant hidden upper-arm skin ring per side to allocate the detail
to the visible bend. This costs 32 net additional triangles, reaching 2,972 per avatar, with the existing
30 deform bones and one material/draw. Global subdivision is not needed.

Review close-ups with authoring edges and front/back shaded views. Test lowered,
horizontal and raised arms as well as forward reach on both body variants,
including garment self intersections and skin coverage. Add a geometric check
of the underarm contour so the old sharp notch is not accepted just because the
surface is connected. Rebuild all twelve rigs and refresh the HTML review page
after the representative shoulders pass. All approval gates remain pending.

The exported middle-depth contour must turn by at most 75 degrees at any sampled
inner-arm corner. The rejected male/female meshes measured approximately 110/102
degrees; the revised command variants measure approximately 62/61 degrees.
Six additional arm poses from `data/crew/shoulder_review.json` exercise both arms
on all twelve exported rigs, adding 72 checks to the 360 baked-clip samples.
The 75-degree ceiling is a local acceptance limit for this stylized asset, not a
universal modelling rule. Visual review of the shaded and authoring-edge sheets
remains required.

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
for each of six departments. Each exported file contains 2,972 triangles, 30
bones, one material and one indexed primitive. Both eyes measure 0.550000
iris/eye and 0.400000 pupil/iris.

The exported tunic and trousers are each one connected closed surface. At rest
and in all 360 exported clip samples and 72 additional shoulder stress poses,
the checks find no garment self
intersections, visible tunic/trouser intersections or hair/scalp intersections.
The waist check excludes internal horizontal closure caps, identified in the
bind pose. Each sample also checks 1,828 covered-skin points. These sampled
motions pass; arbitrary poses still require review.

The twelve source `.blend` files pass all 48 hand/foot IK target and pole checks.
Exported skin weights are normalized with at most three influences. All five
clips preserve their loops and keyed floor contact. A second build reproduces
every GLB and the measured manifest byte for byte; editable `.blend` binary
identity is not claimed.

The twelve animated files total 6,067,768 bytes; the largest binary buffer
is 399,104 bytes including animation tracks. These are artifact sizes, not
measured runtime allocations. A skeleton overlay adds one review draw per visible
avatar. Runtime ship geometry is unchanged pending approval.

Nine deliberately invalid exported files are rejected: excess triangles,
incorrect iris and pupil ratios, covered skin outside the tunic, an extra
material, hair intersecting the scalp, trousers crossing the hem, a closed
but disconnected garment island, and a connected but sharply pinched underarm.
Results and artifact hashes are in
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
plinth, in two draws (one avatar and one plinth). The lineup is 18,024 triangles
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

## Original requirements and rejected prototype

The following brief predates the review assets above. It remains the source for
the visual requirements, approval gate, LOD budgets and rejected MakeHuman attempt.
The review build fulfills the requested new attempt; adoption and live probe
integration remain separate tasks.

### Original context

| Fact | Where |
| --- | --- |
| The crew today: blocky figures, about 100 triangles, the tunic in the department's colour | `crew-npcs` 7, `crates/sc-tools/src/figure.rs` |
| The avatar budget: 3,000 triangles at LOD 0, 1,000 at LOD 1, 300 at LOD 2; 30 of 48 bones; palette colours; one draw | `crew-on-deck` 17 |
| The walk body: 1.80 m tall, radius 0.25 m, the eye at 1.65 m | `data/crew/walk.json` |
| Lighting for anything that moves: ambient-cube probes | `light-baking` sections 6 and 16 |

## 1. For now: blocky people

The owner, 2026-10-10: "for now use blocky people". The figures of `crew-npcs` 7 are the crew in the
engine and the mockups until a character attempt is approved on screenshots (CLAUDE.md 13). They are lit by
the probes around them, so they take each room's light like the walls.

## 2. The look

- **Stylised cartoon, after the references**: the owner's two "Star Trek Pixar" fan cards ("like this"). A
  big head on a short thick neck, a broad chest, heavy brows. The shape only: no likeness of any actor or
  character, no franchise insignia (CLAUDE.md 15).
- **Exaggerated proportions** ("exagerate portions a bit"): a bigger head, hands and feet; a short neck;
  shoulders broad and the waist taken in; short legs under the big head. The amounts are data, so "a bit"
  can be turned without code.
- **Low poly**, smooth or flat shaded, coloured from the palette, the tunic in the role colour (`crew-on-deck`
  17).

## 3. The eyes

"just big eyes I think might make them fun": the eyes are the feature that carries the face. Nose and mouth
stay simple.

The owner asked for the pupil ratio to be measured off the references. Estimated off the pictures (they were
not measured in pixels):

| Ratio | The Kirk card | Elsa | Requirement |
| --- | --- | --- | --- |
| Iris width over the eye's width | about 0.45 | about 0.58 | 0.45-0.58; 0.55 as the first choice |
| Pupil width over the iris's width | about 0.4 | about 0.4 | 0.4 |
| Iris colour | blue-grey | blue | A colour per body, the pupil near black |

The eye is big against the head: the references' eyes are about a fifth of the face's width each. The iris
and pupil edges are geometry, so the ratios are exact on a low-poly eye; a stock low-poly eyeball whose face
rings fall elsewhere cannot hold them (section 6).

## 4. Clothes

"the shirt and pants dont look right": clothes are garments, not colour painted on skin. The hem, the cuffs,
the collar and the trouser legs are edges of real geometry over the body, and nothing of the body shows
through them. The uniform is its own design: a tunic with a collar band and a chest badge, trousers into
boots. The tools the owner asked about are compared in section 6; a uniform modelled for this game (fitted
to every body) is the recommendation, since no free pack has one.

## 5. Budget and light

- At most 3,000 triangles, 30 bones, one material and one draw a body (`crew-on-deck` 17), read back from the
  built file and refused over it.
- Lit by the light probes (`light-baking` 16), like the blocky figures, never by a light of its own.

## 6. What was tried and rejected (2026-10-10)

The first attempt (commit `3419c0b`, PR #6, closed, its files deleted):

- MakeHuman through the MPFB 2 add-on in Blender, headless, from a table of six bodies;
- cartoon proportions by bone scales applied as the rest pose (4.3-4.6 heads tall);
- MakeHuman CC0 garments fitted by MPFB (a long-sleeved top and trousers; the pack has no uniform), the skin
  under them deleted;
- eyeballs rebuilt with rings on the iris and pupil edges (iris 0.55, pupil 0.40, measured);
- 2,930 triangles and 30 bones a body.

The owner: "delete these guys they look stupid lol". The faces kept MakeHuman's realistic shape under
cartoon eyes, and the bodies stood in a stiff A-pose with long realistic hands, which read as uncanny rather
than cartoon. A next attempt should start from a stylised base, modelled for the style, rather than a
realistic human pushed toward one.

Clothing tools compared in that round:

| Tool | Fit here |
| --- | --- |
| MPFB clothes with MakeHuman's CC0 pack | Fits every body; civilian garments only, 3,000-17,000 triangles each |
| MakeHuman community garments | Licences vary per asset; each must be checked and pinned |
| MPFB MakeClothes | Turns a garment modelled over the base mesh into a fitted asset: the way to a game's own uniform on MakeHuman bodies |
| Marvelous Designer, CLO 3D | Commercial and interactive; dense output |
| Blender cloth simulation | Bakes one pose; nothing at this budget needs drape |

## Risks / Trade-offs

- **The references are high-detail renders.** A 3,000-triangle body can take their proportions and their
  eyes, not their surface. The owner judges on screenshots before anything replaces the figures.
