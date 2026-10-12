# Cartoon crew, awaiting screenshot approval

Twelve original characters built with headless Blender 4.4.3: male and female
variants for every department. These are review assets.
The engine and ship mockups keep the blocky figures of `crew-npcs` section 7.
No character is approved and no build result or blank answer supplies approval.

The revised bodies have about 22% narrower upper torsos and 7% narrower waists.
[Before and after](../../../docs/screenshots/crew-characters/body-width-comparison.png)
shows both body variants at the same scale. The existing shared proportion data
drives the garments and rebuilt shoulder joints.

[Interactive 3D viewer](../../../docs/mockups/crew-characters.html): open the HTML
file in a browser, select Male or Female and a department or All crew, then drag to orbit, right-drag
to pan and scroll or pinch to zoom. Choose Idle, Walk, Seated, Reach or Wave, then
play, pause or scrub the actual baked GLB animation. The Skeleton checkbox shows
the deform bones. Front, side, back and eye presets, wireframe, auto rotation and
three illustrative lighting states are available. Model files
and data are embedded; three.js loads from the pinned CDN, so internet is needed
on opening. No local asset server is needed. Downloads preserve the original GLB
bytes. The page cannot approve a character or change ship occupants.

[Browser lineup](../../../docs/screenshots/mockups/crew-characters-lineup.png) |
[Phone view](../../../docs/screenshots/mockups/crew-characters-mobile.png) |
[Browser checks](../../../docs/screenshots/crew-characters/viewer-validation.json) |
[Inspected browser captures](../../../docs/screenshots/crew-characters/viewer-captures.json)

[Lineup](../../../docs/screenshots/crew-characters/lineup.png) |
[Female lineup](../../../docs/screenshots/crew-characters/lineup-female.png) |
[Eyes close-up](../../../docs/screenshots/crew-characters/eyes-closeup.png) |
[Offline probe states](../../../docs/screenshots/crew-characters/probe-states.png)

[Hair clearance](../../../docs/screenshots/crew-characters/hair-clearance.png) |
[Waist clearance](../../../docs/screenshots/crew-characters/waist-clearance.png) |
[Boot shape](../../../docs/screenshots/crew-characters/boots.png)

[Posed render](../../../docs/screenshots/crew-characters/rig-poses.png) |
[Female poses](../../../docs/screenshots/crew-characters/rig-poses-female.png) |
[Skeleton view](../../../docs/screenshots/mockups/crew-characters-skeleton.png) |
[Rig checks](../../../docs/screenshots/crew-characters/rig-validation.json)

[Verification summary](../../../docs/screenshots/crew-characters/rig-review.json)

Every variant has 2,940 triangles.

| Department | Male model / rig | Female model / rig |
| --- | --- | --- |
| Command | [GLB](command_01.glb) / [Blender](blender/command_01.blend) | [GLB](command_02.glb) / [Blender](blender/command_02.blend) |
| Engineering | [GLB](engineering_01.glb) / [Blender](blender/engineering_01.blend) | [GLB](engineering_02.glb) / [Blender](blender/engineering_02.blend) |
| Deckhands | [GLB](deckhands_01.glb) / [Blender](blender/deckhands_01.blend) | [GLB](deckhands_02.glb) / [Blender](blender/deckhands_02.blend) |
| Medical | [GLB](medical_01.glb) / [Blender](blender/medical_01.blend) | [GLB](medical_02.glb) / [Blender](blender/medical_02.blend) |
| Security | [GLB](security_01.glb) / [Blender](blender/security_01.blend) | [GLB](security_02.glb) / [Blender](blender/security_02.blend) |
| Galley | [GLB](galley_01.glb) / [Blender](blender/galley_01.blend) | [GLB](galley_02.glb) / [Blender](blender/galley_02.blend) |

Open a `.blend` in Blender. It starts in Pose Mode with the root selected. Body
FK rings rotate the torso, head and limbs. The root's Bone Custom Properties hold
`ik_arm_L`, `ik_arm_R`, `ik_leg_L` and `ik_leg_R`: 0 uses FK and 1 uses IK.
With IK enabled, move a `CTRL_hand_*` or `CTRL_foot_*` cube with G and rotate it
with R. Move the matching elbow or knee pole to choose the bend direction.
Bone collections separate Body FK, Fingers, IK targets and IK poles.
Each finger has an independent local X curl with a 0-100 degree limit.

The Action Editor contains `Idle_Loop`, `Walk_Loop`, `Seated`, `Reach` and
`Wave_Loop` at 24 fps. Set all four IK properties to 0 to play these FK actions.
For new posing, unlink the action, reset transforms with Alt-G, Alt-R and Alt-S,
then pose or key the controls. Switching IK/FK does not automatically snap the
targets to a posed limb. These are editable review motions, not locomotion with
runtime foot locking or automatic retargeting.

The Blender rig has 30 deform bones and eight authoring controls. Only the
30 deform bones are exported. To keep the requested game budget, each finger has
one curl hinge; there are no facial, toe or extra finger joints. This is an
original cartoon rig following the MakeHuman style of body posing, not an
MPFB-generated human or a requirement to install that plugin.

Every GLB has 30 bones, one diffuse palette material, one indexed mesh primitive
and geometric eye boundaries measured at 0.55 iris/eye and 0.40 pupil/iris. The
primitive is ready for one avatar draw when adopted; this step measures the file,
not an engine frame. Proportions, palette, eye ratios and body sizes live in
[characters.json](../../../data/crew/characters.json). Department colours come
from [company.json](../../../data/crew/company.json). UV0 is a geometric part tag,
not a texture coordinate to sample. COLOR_0 is linear albedo, separate from light.
The GLBs include normalized skin weights and all five baked animation clips.
Rig proportions, control sizes, blend spans and poses live in
[rig.json](../../../data/crew/rig.json). The avatar shader applies probe lighting
after skinning the normal, including during playback.

The tunic shares vertices across shoulders, sleeves and underarms. The trousers
join through one pelvis and crotch, and boots have a forward toe box, heel,
instep, ankle and thin sole. The hair follows the actual skull with clearance.
Connected profiles, male/female proportions, hair fitting and waist clearances
live in [anatomy.json](../../../data/crew/anatomy.json).

[characters.json](characters.json) is the measured asset manifest, including
file hashes, sizes and rest-pose garment coverage. The build checks closed garment
shells and 1,972 covered-skin samples per character at rest and at six samples in
each of five exported clips. The rig report checks 360 poses across the twelve GLBs,
loop closure, floor contact at keyed poses, hierarchy and weights, then opens
each saved `.blend` and measures all four IK targets and their poles. This covers
the supplied motions; arbitrary poses still need visual review.
[validation.json](../../../docs/screenshots/crew-characters/validation.json)
records deliberate artifact failures: excess triangles, incorrect iris or pupil
geometry, skin pushed through a tunic, and a second material/primitive.
It also rejects scalp/hair intersections, trousers crossing the tunic and
closed but disconnected garments. Rest and sampled posed garments pass self
intersection checks. The waist check excludes internal closure caps, identified
from the bind pose; it tests the visible tunic and trouser walls.
[captures.json](../../../docs/screenshots/crew-characters/captures.json) ties the
review screenshots to these exact GLB hashes. Turnarounds cover front, both
sides, back and three-quarter; the pose sheet covers all five clips.

The normal, red-alert and emergency image uses illustrative ambient cubes from
the table, multiplied into imported albedo according to exported normals.
This is an offline response check. Live deck probe sampling/interpolation,
in-engine draw measurements and Pi 5 performance are not implemented by it.

Build and verify, from the repository root:

```powershell
$crewBlender = 'C:/Program Files/Blender Foundation/Blender 4.4/blender.exe'
& $crewBlender -b --factory-startup --python-exit-code 1 -P tools/blender/build_crew_characters.py -- --blend assets/models/crew_review/blender
& $crewBlender -b --factory-startup --python-exit-code 1 -P tools/blender/build_crew_characters.py -- --check
& $crewBlender -b --factory-startup --python-exit-code 1 -P tools/blender/check_crew_characters.py
& $crewBlender -b --factory-startup --python-exit-code 1 -P tools/blender/check_crew_rigs.py
& $crewBlender -b -t 4 --factory-startup --python-exit-code 1 -P tools/blender/render_crew_characters.py -- --samples 24
& $crewBlender -b -t 4 --factory-startup --python-exit-code 1 -P tools/blender/render_crew_rigs.py -- --samples 24
python tools/mockups/inline.py docs/mockups/crew-characters.html
# With Playwright available through NODE_PATH:
node tools/mockups/check_crew_viewer.cjs
node tools/mockups/shoot.mjs docs/mockups/crew-characters.html
```

The generator and data are the source; rebuilding replaces the generated rigs.
Save hand edits under a different filename. Counts, budgets and posed clothing
are read back from staged GLBs before any requested batch is written here.
`--check` verifies identical GLB and manifest bytes. `.blend` binary identity is
not claimed. The global inliner currently reports an unrelated stale deck bake;
the crew page's own embedded assets and budget are checked separately.

The spec, design, approval gate and remaining adoption work are in
[crew-characters](../../../openspec/changes/crew-characters/design.md).
