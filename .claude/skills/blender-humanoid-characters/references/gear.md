# Rigid gear: armour, helmets, masks, goggles, packs

MPFB's CC0 wardrobe has no faction outfits, and a disguise system needs outfits that read at a
glance. So the build authors gear itself: low-poly rigid shells that wrap the body with a
clearance, each skinned 100 % to one bone, painted as colour swatches into the outfit atlas.
A body keeps 3 materials and its texture count, whatever it wears. Code: `build_gear`,
`fit_shell`, `shell_mesh`, `poke_through` in `tools/blender/build_npcs.py`.

## A piece

A piece is a grid wrapped around an origin: `angle_deg` around the frame's up axis (0 = the
body's front, +90 = its left) by `along_m` (a cylinder: metres up the axis) or `elevation_deg`
(a sphere: degrees above the front-left plane).

```json
"chest_plate": {"bone": "spine_03", "shape": "cylinder", "origin": "bone_head",
  "angle_deg": [-52.0, 52.0], "along_m": [0.05, 0.26], "segments": [8, 5],
  "clearance_m": 0.012, "thickness_m": 0.014, "reach_m": 0.32,
  "fit_bones": ["spine_03", "spine_02", "clavicle_l", "clavicle_r"],
  "colour": "armour_greyblue", "stripe": {"colour": "cloth_hivis", "v": [0.66, 0.78]}},
"back_plate": {"like": "chest_plate", "angle_deg": [132.0, 228.0], "stripe": null}
```

| Field | Meaning |
|---|---|
| `bone` | the one bone the piece follows (100 % weight) |
| `shape` | `cylinder` (takes `along_m`) or `sphere` (takes `elevation_deg`) |
| `frame` | `world` (up is up) or `bone` (up along the bone) |
| `origin` | `bone_head`, `bone_tail`, `centroid` (of the vertices the bone dominates), `eyes`, `eye_l`, `eye_r` |
| `offset_m` | `[left, front, up]` from the origin |
| `segments` | grid cells `[around, along]`; low: 8 x 5 is a chest plate |
| `clearance_m` | gap outside the farthest body point, 2-12 mm in practice |
| `thickness_m` | shell thickness |
| `reach_m` | body points farther than this from the origin are ignored |
| `fit_bones` | the body parts the piece clears (default: its own bone) |
| `over` | pieces on the same bone, worn before it, that it also clears (a visor over a helmet) |
| `colour`, `colour2` + `mix`, `stripe` | the swatch: a colour, patches of a second, a band across v |
| `like` | start from another piece and override fields |

## How it's fitted

1. Sample every body part's surface every `gear_style.sample_m` (1 cm) along each triangle's
   edges, not only at vertices, so a long thin triangle can't slip through a band of gear.
2. Keep points dominated by `fit_bones`, within `reach_m`; put each in the grid by its angle
   and height (or elevation).
3. Each grid node's inner radius is the farthest point in its neighbouring cells. Empty cells
   fill from their neighbours; then two smoothing passes that only ever raise a node.
4. Add `clearance_m`; build a closed solid `thickness_m` thick with walls on its borders; UVs
   0-1 over the grid, later moved inside the piece's swatch with a tenth-of-a-swatch margin.

## The poke-through test

After fitting, the body is posed at rest and at `stress_pose` (arms up and forward, a knee
raised, spine and neck turned, fingers bent). At each, a BVH of every gear piece is overlapped
with a BVH of the posed body, and with every other piece on the same bone. Any intersecting
triangle pair fails the build:

```
guard: gear intersects the body or other gear (chest_plate at stress pose: 14 triangle pairs)
```

Fixes, in order: raise `clearance_m`; add the bone whose flesh pokes through to `fit_bones`;
narrow `angle_deg` or `along_m` away from a joint that bends into it; move it to a bone that
doesn't bend there. Never lower the stress pose to pass.

Only authored gear is tested. MPFB's own clothes against the body at extreme poses are not
(they stretch acceptably at game distance).

## Swatches

Each distinct `(colour, colour2, mix, stripe)` is one swatch in the gear tile of the outfit
atlas, `swatches_per_row` squared at most (3 x 3 in Undercity). Value noise (`noise`,
`noise_cells`) keeps a flat colour from looking plastic; the noise is seeded from the body seed
and the swatch, so it rebuilds identically.

## Designing a set

- Silhouette first: a helmet dome and visor, shoulder pads and plates read from across a street;
  small pieces (a loupe, filters) read in conversation.
- One accent colour per faction (MerSec hi-vis, Drain Rats green lenses).
- Layer with `over`: `helmet_rear` over `helmet_dome`, `visor` over `helmet_dome`.
- Render with `render_character.py --table ...` and look at the stress-pose figure.
