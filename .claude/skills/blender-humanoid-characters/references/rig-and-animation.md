# The rig, retargeting, one clip library, new clips, ragdolls

## The rig

MPFB's `game_engine` rig: 53 deforming bones with Unreal mannequin names, rest pose an A-pose.

| Part | Bones |
|---|---|
| Trunk | `Root`, `pelvis`, `spine_01`, `spine_02`, `spine_03`, `neck_01`, `head` |
| Each arm (`_l`, `_r`) | `clavicle`, `upperarm`, `lowerarm`, `hand` |
| Each hand | `thumb_01-03`, `index_01-03`, `middle_01-03`, `ring_01-03`, `pinky_01-03` |
| Each leg | `thigh`, `calf`, `foot`, `ball` |

No eye, jaw or face bones: faces are static. MPFB's larger rigs have face bones; they weren't
measured.

## Retargeting in Godot

Every body and every clip library land on Godot's `SkeletonProfileHumanoid` (56 bones; 53
mapped, all but `LeftEye`, `RightEye`, `Jaw`). Source: Godot's "Retargeting 3D skeletons" page.

- Godot's `BoneMapper` auto-mapper is editor-only (not in `ClassDB`), so the two maps are
  committed `.tres` files written by `tools/godot/setup_npc_import.gd`:
  `game/animations/bonemaps/mpfb_game_engine.tres` and `ual_rigify_def.tres` (UAL's Rigify
  `DEF-` bones).
- Each glb's import options: the bone map, rename the skeleton to `%GeneralSkeleton`, and the
  rest fixer (Overwrite Axis, Fix Silhouette). MPFB rests in an A-pose; both rigs come out in a
  T-pose, so clips transfer.
- The retarget options are keyed by the skeleton's path before the rename. A key naming
  `GeneralSkeleton` matches nothing at import and leaves the body un-retargeted;
  `tools/godot/test_npc_imports.py` guards it.
- Order: `godot --import`, `setup_npc_import.gd`, `godot --import` again.

## One library for every body

UAL Standard (Quaternius, CC0): 46 clips, one `AnimationLibrary`, all 53 tracks resolve on every
body. Loop clips end in `_Loop`; Godot's import strips the suffix and marks them looping.

| Game state | Clip |
|---|---|
| Idle, walk, run | `Idle`, `Walk` (`Walk_Formal`), `Jog_Fwd`, `Sprint` |
| Crouch | `Crouch_Idle`, `Crouch_Fwd` |
| Talk, sit | `Idle_Talking`; `Sitting_Enter`, `Sitting_Idle`, `Sitting_Talking`, `Sitting_Exit` |
| Pistol | `Pistol_Idle`, `Pistol_Aim_Up/Neutral/Down`, `Pistol_Shoot`, `Pistol_Reload` |
| Melee | `Punch_Enter`, `Punch_Jab`, `Punch_Cross`, `Sword_Idle`, `Sword_Attack` |
| Hit, death | `Hit_Chest`, `Hit_Head`, `Death01` then the ragdoll |

Missing from UAL: rifle holds, strafing, turns in place, more deaths, and (until Undercity
keyed them) surrender and cower. The state-to-clip map is data: `game/data/npc_bodies.json`
`clips`, with a 0.25 s cross-fade (`blend_s`) through an `AnimationPlayer`.

## Authoring a clip on UAL's rig

`tools/blender/build_npc_clips.py` keys clips from `tools/blender/npc_clips.json` onto UAL's own
armature, imported from the pinned pack (same bone names and rest pose), so the UAL bone map
retargets them and every body plays them.

```json
"Surrender_Loop": {"length_s": 2.0, "hips_offset_m": [0.0, 0.0, -0.01], "plant_feet": true,
  "bones": [
    {"bone": "DEF-neck", "rot": [{"axis": "X", "deg": 4.0}]},
    {"bone": "DEF-upper_arm.L", "frame": "DEF-spine.003", "aim": [0.8, 0.2, 0.3]},
    {"bone": "DEF-forearm.L", "frame": "DEF-spine.003", "aim": [-0.2, 0.08, 1.0], "twist_deg": -75.0}],
  "motions": [{"bone": "DEF-spine.002", "axis": "X", "deg": 1.2, "cycles": 1, "phase_deg": 0.0}]}
```

- A bone either **rotates** (degrees about armature axes, after its parent: X positive bends
  forward, Y positive tips a left-side bone down, Z positive turns toward the body's left) or
  **aims** its head-to-tail direction at `[left, front, up]` in a frame bone's posed frame,
  with a twist about that direction.
- `plant_feet`: legs are two-bone IK that keeps each ankle where it rests, so the hips can drop
  (`hips_offset_m`) without the feet sliding.
- `motions` add sines with a **whole number of cycles per clip**, so the last frame equals the
  first and the loop is seamless. The build checks every channel ends where it starts.
- Name looping clips `..._Loop`. glTF has no loop flag.
- The glb carries one tiny triangle skinned to `DEF-hips`: Godot builds a `Skeleton3D` only for
  bones a skin uses, and without it the clip library has no skeleton to retarget.
- Output is byte-identical on rebuild; the build checks names, lengths and joints against UAL's
  own glb.

## Scenes and ragdolls

`tools/godot/gen_npc_scenes.gd` writes `game/scenes/undercity/npcs/<id>.tscn` as text: an
inherited scene of the glb plus two nodes (writing text keeps it 17 KB; packing an instanced glb
from a script embeds the whole body).

- **Anim**: an `AnimationPlayer` with the UAL library (no prefix) and `undercity/`.
- **Ragdoll**: a `PhysicalBoneSimulator3D` of 20 capsules, 80.5 kg, from
  `game/data/npc_bodies.json`. Cone joints, except knees and elbows: hinges limited to -140 to
  0 degrees. Every bone from the hips out is simulated, no gaps in the chain.
- **Grip** on the right hand (weapons: origin at the web of the hand, barrel along -Z), fitted
  from `Pistol_Aim_Neutral`; accessory **mounts** from the data's `mounts`.

`NpcRagdoll.cs` does what can't be saved in a scene: builds the joints at the rest pose two
physics frames after load, and adds collision exceptions for parent, sibling and grandparent
pairs (without them the spine opened an 8.5 cm gap). On death it simulates `settle_s` (3.0 s)
and then freezes the pose; waiting for sleep left bodies twitching.

Measured, a standing body dropped off a 0.45 m step at 60 Hz:

| Engine | Peak speed | Joint gap max | Speed at 3 s |
|---|---|---|---|
| Jolt | 6.9-7.4 m/s | 3.1-5.2 cm | 0.06-0.12 m/s |
| GodotPhysics3D | 2,090 m/s | 42.7 m | 414 m/s |

Use Jolt. `ragdoll_test.tscn` repeats the drop for every body.
