# The body table and what MPFB does with it

`tools/blender/npcs.json` is read by `npc_data.load()` (schema, code defaults, cross-checks) and
built by `build_npcs.py`. Every unknown key is an error, a missing optional field takes its code
default, a present zero is zero (CLAUDE.md 5.6). `python3 tools/blender/npc_data.py [file]`
checks a table without Blender.

## Top-level blocks

| Block | Holds |
|---|---|
| `out_dir` | where glbs go, relative to the repository (an absolute path works, for scratch builds) |
| `rig` | `game_engine` (53 bones, Unreal mannequin names) |
| `seed` | the table seed; each body's seed is `crc32("<seed>:<id>")` |
| `atlas_px`, `texture_format`, `webp_quality` | 1024, `webp`, 85: about 0.8 MB a body against about 4 MB as PNG |
| `budget` | triangles 16000, materials 3, texture_px 1024, textures 5, bones 53 |
| `height_fit` | `tolerance_m` 0.005, `max_iterations` 30 |
| `civilian_range` | the draw for seeded rows: heights per sex (mean 1.76 / 1.64 m, sd 0.07, clamp 1.55-1.95), ages 19-74, age bands, race pin weight 0.7, brows and eyes to pick from |
| `proxies` | the low-poly body per sex: `male1591`, `female1605` |
| `eyes_model` | `low-poly` |
| `decimate` | triangle ratio per heavy asset (shoes 0.35-0.4, bob and ponytail 0.45-0.6) |
| `tint` | how garments take a colour: contrast 0.85, top and bottom split, texel ratio clamp |
| `skin_tone_strength`, `skin_normal`, `skin_roughness` | skin look: tone blend, pore normal from the albedo (strength 16), T-zone roughness |
| `outfit_roughness` | cloth 0.7, eyes 0.08 (the eyes share the outfit atlas) |
| `texture_fixes` | UV rects painted out of a CC0 texture before tinting (logos) |
| `gear_style`, `gear_pieces`, `gear_sets` | rigid gear (references/gear.md) |
| `palette` | named sRGB colours; rows and gear use names, never hex |
| `stress_pose` | bone rotations the gear test poses the body in |
| `bodies` | the rows |

## A row

Named (no `seed`): everything pinned.

```json
{"id": "guard", "role": "a helmeted guard", "sex": "male", "age_years": 32, "height_m": 1.82,
 "muscle": 0.7, "weight": 0.6, "proportions": 0.6,
 "race": {"african": 0.3, "asian": 0.2, "caucasian": 0.5},
 "skin": "young_caucasian_male", "skin_tone": "skin_olive", "eyes": "grey", "eyebrows": "eyebrow008",
 "hair": null, "clothes": [{"asset": "male_worksuit01", "tint": "cloth_navy"}, {"asset": "shoes03"}],
 "gear": ["guard"]}
```

Civilian (`seed`): pins `sex`, `race` (one of three), clothes, hair, tints, gear; the rest is drawn.

```json
{"id": "passerby", "seed": 4242, "sex": "male", "race": "asian", "skin_tone": "skin_tan",
 "hair": "short01", "hair_tint": "hair_black",
 "clothes": [{"asset": "male_casualsuit05", "top": "cloth_raincoat", "bottom": "cloth_black"}, {"asset": "shoes04"}]}
```

- `id` is lower snake_case and is the glb's name; the game loads `models/characters/<id>.glb`.
- A garment takes either `tint` (one colour) or `top` and `bottom`. A connected piece of the
  garment is its bottom when its mean height is under `split_height_frac` of the body or it
  reaches below `legwear_below_frac` (about the knee).
- `hair: null` with a `hair_tint` is an error. Capped heads carry no hair.

## From a row to MPFB

`resolve_body` turns a row into MPFB's macro sliders:

| Row | MPFB | Mapping |
|---|---|---|
| `sex` | `gender` | male 1.0, female 0.0 |
| `age_years` | `age` | piecewise linear: 1 y = 0, 11 y = 0.1875, 25 y = 0.5, 90 y = 1.0 |
| `race` weights | `race` | normalised to sum 1 |
| `muscle`, `weight`, `proportions`, `cupsize`, `firmness` | same names | 0-1 as given |
| `height_m` | `height` | bisection on the slider until the mesh measures within 5 mm |

MPFB's skin bands switch at age slider 0.65 and 0.85 (44.5 and 70.5 years): a civilian's skin is
`<band>_<race>_<sex>`, and a row's `age_band` must agree with the age it draws, or the build
stops and says so.

`build_human` fills MPFB's `HumanService.deserialize_from_dict` info: phenotype, rig, proxy,
eyes, brows, lashes, hair, skin `.mhmat` with `skin_material_type = GAMEENGINE`, clothes, and the
eye colour as an alternative material. Settings: `subdiv_levels 0`, clothes and eyes as
`GAMEENGINE` models.

## From MPFB's human to a game body

1. Delete the base mesh; keep the proxy. Apply each part's Mask modifiers (MPFB's delete
   groups remove the skin hidden under clothes); drop subdivision.
2. Decimate the listed heavy assets (symmetric).
3. Build gear and test it (references/gear.md).
4. Bake three materials on atlases, each tile padded 4 px with edge-extended pixels so mips
   don't bleed:

| Material | Atlas | Holds |
|---|---|---|
| `<id>_skin` | skin albedo 1024 + derived normal (alpha = roughness) | the proxy body, toned |
| `<id>_outfit` | outfit albedo + normal, template of 8 tiles | clothes by size, eyes, gear swatches |
| `<id>_hair` | hair albedo, alpha-clipped | hair, brows, lashes, tinted |

   Alpha clip is a Math ROUND node on the texture's alpha: the glTF exporter reads that as
   `alphaMode MASK`, cutoff 0.5.
5. Join everything into one skinned mesh, drop vertex groups that aren't bones, keep one UV set.
6. Export:

```python
bpy.ops.export_scene.gltf(filepath=tmp, export_format='GLB', use_selection=True, export_yup=True,
                          export_skins=True, export_animations=False, export_morph=False,
                          export_image_format='WEBP', export_image_quality=85,
                          export_image_webp_fallback=False, export_materials='EXPORT',
                          export_rest_position_armature=True, export_def_bones=False)
```

7. Read the written glb's JSON (triangles, materials, images and their pixel sizes, joints,
   animations) and check it against the budget. `glb_stats()` does it without Blender.

The exporter warns "more than 4 joint vertex influences": it keeps the 4 heaviest and
normalises, which is what Godot wants.

## Axes

MPFB builds facing -Y with the character's left at +X and Z up. The glTF export is +Y up, so
in Godot a body faces +Z. Gear directions are `[left, front, up]` in the body's frame.
