---
name: blender-humanoid-characters
description: Generate game-ready humanoid characters in Blender from a JSON table, headless and byte-reproducible. MPFB2 (MakeHuman) builds each body with a 53-bone game_engine rig, a low-poly proxy, CC0 clothes and hair, fitted height and tinted atlases; the build adds rigid faction gear with poke-through tests, enforces a triangle, material, texture and bone budget, and exports one skinned glb. Godot then retargets it to SkeletonProfileHumanoid, plays one shared animation library (UAL), and gives it a Jolt ragdoll. Use when adding or changing an NPC or civilian body, outfit, gear, hair or skin, fitting height or phenotype, authoring a new animation clip on the UAL rig, retargeting or ragdolling characters in Godot, rendering a character capture, or taking this pipeline to another project.
metadata:
  author: Undercity (Claude Code), distilled from the npc-characters and crowd-variety changes
  version: "1.0"
---

## In Star Crew

Copied from the Undercity repository (`fps-game-demo`, `.claude/skills/blender-humanoid-characters`
at `f6cd25c`, 2026-10-04); provenance in `.claude/skills/PROVENANCE.md`. Everything below this
note is the source's text, kept so a later copy can be diffed. It is the reference for building
crew bodies from a table, headless and reproducible.

What does not apply here:
- **Its budgets.** Undercity's bodies (53-bone rig, 1024 px atlases) are a desktop's. A Star
  Crew avatar is at most 600 triangles and 24 bones, with palette colours instead of atlases
  (`openspec/changes/engine-stack/design.md`, "The Pi 3 budget"; `crew-on-deck`). A body built
  with this kit must be decimated and re-rigged to that budget.
- **The Godot tail** (retargeting to SkeletonProfileHumanoid, Jolt ragdolls, `.tscn` scenes):
  Star Crew has a custom engine with baked animation clips.
- **Its file paths** (`tools/blender/npcs.json`, `game/models/`): those are Undercity's.

# Humanoid characters: generation and rigging in Blender

Every Undercity body is a row in `tools/blender/npcs.json`. One command turns the table into
`game/models/characters/<id>.glb`, and the same table and packs always give the same bytes. No
one models, rigs or weight-paints by hand: the rig and the skin weights come from MakeHuman,
the look from the table, the animation from one shared library.

```
tools/deps/character_packs.json   pinned MPFB2 + MakeHuman CC0 assets + UAL, by SHA-256
        | fetch_character_tools.py -> ~/.cache/undercity/deps (never in the repository)
tools/blender/npcs.json -> build_npcs.py (Blender, private user folder, MPFB services)
   per row: phenotype -> height fit -> MPFB human (game_engine rig, proxy, clothes, hair)
            -> delete groups, decimate -> rigid gear (BVH poke test at rest + stress pose)
            -> 3 materials on 1024 px WebP atlases -> join -> one skinned glb -> budget check
Godot: setup_npc_import.gd (bone map, retarget) -> gen_character_materials.py (skin, outfit)
       -> gen_npc_scenes.gd (inherited scene + Jolt ragdoll + AnimationPlayer: UAL + our clips)
Clips UAL lacks: npc_clips.json -> build_npc_clips.py -> undercity_clips.glb (same UAL rig)
```

## Setup (once per machine)

```
python3 tools/deps/fetch_character_tools.py      # MPFB2 and the CC0 assets download (325 MB)
# UAL is "manual": copy a verified ual_standard.zip into ~/.cache/undercity/deps/packs/
```

Blender 4.2 or later (tested on 5.2.2 LTS). The build restarts Blender once with
`BLENDER_USER_RESOURCES` in the cache, so MPFB never touches your own Blender settings.
A cloud session can fetch the two downloadable packs itself; the whole build runs there.

## Build, verify, look

```
blender -b --factory-startup --python tools/blender/build_npcs.py                 # every body
blender -b --factory-startup --python tools/blender/build_npcs.py -- --only guard,civ_a
blender -b --factory-startup --python tools/blender/build_npcs.py -- --verify     # byte for byte
blender -b --factory-startup --python tools/blender/build_npcs.py -- --table my.json --report stats.json
blender -b --factory-startup -P .claude/skills/blender-humanoid-characters/scripts/render_character.py -- \
    out.png game/models/characters/guard.glb [more.glb ...] --table tools/blender/npcs.json
```

About 12-15 s a body. `--verify` rebuilds and fails unless each glb matches the committed one;
run it after touching the build or the packs. `--save-blend <dir>` keeps a `.blend` per body
for debugging (never committed; CLAUDE.md 13). The render shows front, three-quarter, side,
back and the stress pose per body, in Cycles on the CPU (no GPU needed); read it before you
say a body is done. `tools/blender/render_npcs.py` makes the full lineup (EEVEE, needs Xvfb).

After any body change, the Godot steps in README ("Undercity's NPC bodies"): import, setup
import, import again, `gen_npc_scenes.gd`, then the ragdoll test.

## Recipes

**A named NPC.** Add a row without `seed`: `sex`, `age_years`, `height_m`, `race` weights,
`skin` (`<young|middleage|old>_<african|asian|caucasian>_<male|female>`), `eyes`, `eyebrows`,
`hair` (or null), tints from the palette, `clothes` (each `asset` with `tint`, or `top` and
`bottom`), `gear` sets. Sliders `muscle`, `weight`, `proportions`, `cupsize`, `firmness` are
0-1. Start from [templates/example_table.json](templates/example_table.json).

**A civilian.** A row with a `seed`: it pins `sex`, `race`, clothes, hair and tints; age,
height, phenotype, brows and eyes are drawn from the seed inside `civilian_range` (adults only,
height per sex, clamped). `age_band` pins the age inside one of MPFB's skin bands.

**Gear.** Define pieces in `gear_pieces` (a cylinder or sphere wrap around a bone), group them
in `gear_sets`, list sets on a row. Read [references/gear.md](references/gear.md) first.

**A clip UAL lacks.** Add it to `tools/blender/npc_clips.json`, run `build_npc_clips.py`, map
the game state to it in `game/data/npc_bodies.json`. See
[references/rig-and-animation.md](references/rig-and-animation.md).

**Finding asset names.** They are folder names in the unpacked packs:
`ls ~/.cache/undercity/deps/blender/<ver>/extensions/.user/user_default/mpfb/data/{clothes,hair,skins,eyebrows,eyes,proxymeshes}`.
The CC0 pack has 12 outfits (casual, elegant, work and sport suits), 6 shoes, 2 hats, 10 hair
styles and 12 skins by age, race and sex.

## Rules that came from real bugs

- **The table is the source; the glb is generated** (CLAUDE.md 6.1, 11). Never edit a glb.
  The `.blend` is not committed, by the documented exception in CLAUDE.md 13.
- **Same table, same bytes.** Seeds come from `crc32("<table seed>:<id>")`, never Python's
  `hash()`; the build runs with `PYTHONHASHSEED=0`; sets and dicts are sorted before they
  choose anything. Anything new you add must keep `--verify` green.
- **Only allowlisted pack files go in.** An asset counts only if its path is a member of a
  pinned zip with a matching size and CRC-32. A file planted in the unpacked folder is refused.
- **The budget is read from the written glb**, not from Blender: 16,000 triangles, 3
  materials, 5 textures at 1024 px, 53 bones, no animations (`budget` in the table). Over it,
  the build fails naming the body and the number. Decimate only the heavy pieces (shoes,
  bob and ponytail hair, some suits) in `decimate`.
- **Height is fitted, not guessed.** Bisection on MPFB's height slider to `height_m` within
  5 mm; a height the phenotype can't reach is refused.
- **Gear never pokes through.** Rigid shells, 100 % on one bone, fitted a clearance outside
  the body and the pieces under them, then tested triangle against triangle with a BVH at rest
  and at `stress_pose`. A failure names the piece, the pose and the pair count.
- **Look lessons:** a cap over MPFB hair looks like a mushroom, so capped heads carry no hair;
  men's clothes fit women's bodies well; the plain tee's logos are painted out
  (`texture_fixes`); the game shows no brand marks.
- **Faces are static.** The game_engine rig has no face, eye or jaw bones.
- **Physics is Jolt.** GodotPhysics3D exploded every ragdoll in the measurement (2,000 m/s).

## Checks before calling a body done

| Check | How |
|---|---|
| The table | `python3 tools/blender/npc_data.py` (schema, palette, gear references) |
| The build | `build_npcs.py` passes its own budget and poke-through checks |
| Reproducible | `build_npcs.py -- --verify` |
| It looks right | `render_character.py` still, read by you, then committed under `docs/screenshots/` |
| Imports | `python3 -m unittest discover -s tools/godot -p 'test_*.py'` |
| In Godot | `ragdoll_test.tscn`, then an AutoTest capture of the body in the level |

## When it goes wrong

| You see | Cause | Fix |
|---|---|---|
| `height_m ... outside what this phenotype reaches` | a height the sliders can't give | change the height or the phenotype |
| `asset ... is in no allowlisted pack` | a misspelt asset name, or a new pack not pinned | check the folder name; pin a new pack in `character_packs.json` |
| `gear intersects the body` | clearance too small, or a fit bone missing | raise `clearance_m`, add the bone to `fit_bones`, or narrow the angles |
| `over budget: N triangles` | heavy clothes or hair | add the asset to `decimate` |
| `the rebuild differs` on `--verify` | a nondeterministic change, or the packs changed | find the unsorted choice; re-pin deliberately |
| A body stands in T-pose or doesn't animate | un-retargeted import | rerun `setup_npc_import.gd` and reimport; `test_npc_imports.py` names the key |
| The ragdoll flies apart | GodotPhysics3D, or missing neighbour exceptions | Jolt; `NpcRagdoll` adds the exceptions at runtime |
| `SkeletonProfile` bones missing on a clip glb | Godot builds a Skeleton3D only for skinned bones | keep the clip glb's placeholder triangle |

## Another project

```
python3 .claude/skills/blender-csg-levels/scripts/copy_kit.py \
    .claude/skills/blender-humanoid-characters/kit.json <target_root>
```

It copies [kit.json](kit.json)'s files with this skill and writes
`KIT_PROVENANCE_blender-humanoid-characters.md` with the source revision and the list of things
to adapt (cache name, table, physics layers, namespaces). The GPL tool (MPFB) runs at build
time only and never ships; its output and the CC0 assets are free to ship.

## References

- [references/body-table.md](references/body-table.md): the table, MPFB's inputs, textures and budget
- [references/gear.md](references/gear.md): rigid gear, fitting and the poke-through test
- [references/rig-and-animation.md](references/rig-and-animation.md): the rig, retargeting, the clip library, authoring clips, ragdolls
- `openspec/changes/archive/2026-09-28-npc-characters/design.md`: the decisions and measurements
