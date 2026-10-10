# Design: crew bodies, cartoon low poly from MakeHuman

## Context

| Fact | Where |
| --- | --- |
| The avatar: 3,000 triangles at LOD 0, 1,000 at LOD 1, 300 at LOD 2; 30 of 48 bones; flat-shaded, vertex-coloured from the palette; the shirt takes the role colour as a per-body uniform; one draw call | `crew-on-deck` 17 |
| The body the walk world moves: a capsule 1.80 m tall, radius 0.25 m, the eye at 1.65 m | `data/crew/walk.json` |
| The role colours | `docs/mockups/lib/shipkit.js` `PALETTE.role` |
| Today's figures: about 100 triangles a department, built by `deckc` | `crew-npcs` 7, `crates/sc-tools/src/figure.rs` |
| MakeHuman in Blender, headless, from a table: phenotype sliders, a proxy mesh, the `game_engine` rig, CC0 clothes and hair | Undercity's `tools/blender/build_npcs.py` and the `blender-humanoid-characters` skill |

## 1. The body table

`data/crew/bodies.json`, one row a body, validated by the build (an unknown key or a missing one stops it):

| Key | Meaning |
| --- | --- |
| `id` | The body's name, and its file `assets/models/crew/<id>.glb` |
| `sex` | `male` or `female` (MakeHuman's gender slider at 1 or 0) |
| `age_years` | 18-70; MakeHuman's age slider by its anchors (25 years is 0.5) |
| `muscle`, `weight` | 0-1, MakeHuman's sliders |
| `height_m` | The finished body's height, cartoon head included; at most the walk body's 1.80 m |
| `skin` | A skin tone by name from the table's palette (`skin_tones_srgb`) |
| `hair` | A MakeHuman CC0 hair asset, or null; its colour by name (`hair_colours_srgb`) |
| `role` | The department whose colour the prototype paints the tunic in (the game sets it per body) |

The table also holds the shared settings: the proxy mesh per sex, the cartoon scales (section 3), the
region colours (section 4) and the budget.

## 2. The build

`tools/blender/build_crew_bodies.py`, run with the Blender Python module (no Blender install needed):

1. Verify the pinned packs (`tools/deps/character_packs.json`, the MPFB 2.0.17 add-on and MakeHuman's CC0
   system assets, by SHA-256) and unpack them into a private Blender user folder in the cache, never in
   the repository and never in the user's own Blender settings (Undercity's rule).
2. Per row, MPFB builds the human: the phenotype sliders and face targets, the low-poly proxy mesh for the
   sex (`male1591`, `female1605`), the `game_engine` rig with its skin weights, the hair, the brows, and the
   **clothes** (section 4a), fitted by MPFB's clothes service.
3. The skin the clothes cover is deleted, the suit split into tunic and trousers, and every part decimated
   to its share of the budget.
4. Cartoon proportions (section 3) on the rig, then the rig is applied as the rest pose; the eyes are rebuilt
   big (section 4b) and the brows lifted over them.
5. Colour by region (section 4c), the rig cut to 30 bones, one glb a body: one mesh, one material, vertex
   colours, the skin and its 30 bones, no animation. The budget is read back from the written file, and a
   body over it is refused, named with its count.

Same table, same packs, same bytes (`PYTHONHASHSEED=0`, sorted iteration), as the prop builds.

## 3. Cartoon proportions

A real adult is about seven and a half heads tall. A cartoon reads at a glance because the parts a
player watches (the face, the hands that work a console, the boots) are bigger than life. The style is
after the owner's references (2026-10-10, two "Star Trek Pixar" fan cards): a big head on a short thick
neck, a broad chest, heavy brows, big eyes. The shape only: no likeness and no Starfleet insignia
(CLAUDE.md 15). The build scales bones in pose mode, each bone inheriting no scale, then applies the pose to
the mesh and makes it the rest pose, so the skin weights carry the change and nothing is sculpted by hand:

| Part | Bones | Scale | Why |
| --- | --- | --- | --- |
| Head | `head` | 1.70 | The face reads across a room |
| Neck | `neck_01` | 0.80 along, 1.25 across | Short and thick, so the big head sits on the shoulders |
| Hands | `hand_l/r` and every finger bone | 1.45 | The hands work consoles and tools: the player watches them |
| Feet | `foot_l/r`, `ball_l/r` | 1.40 | Chunky boots plant the figure |
| Shoulders | `clavicle_l/r` | 1.25 along | A heroic, readable silhouette |
| Chest | `spine_03`, `spine_02` | 1.20, 1.10 across | A broad chest |
| Waist | `spine_01` | 0.92 across | Taper from the chest |
| Arms | `upperarm`, `lowerarm` | 0.95 along, 1.20 and 1.15 across | Short, thick arms |
| Legs | `thigh`, `calf` | 0.86 along, 1.15 and 1.10 across | A shorter stride under the bigger head |

Face targets (MPFB's own, `face_targets` in the table) widen the eye sockets and the mouth and push the chin.
After the scales the body is lifted so its lowest point is on the floor, then scaled as a whole to the
row's `height_m`. **Measured:** the six bodies are 4.28-4.58 heads tall (the manifest's `heads_tall`).
Every number is in the table, so the owner's "a bit" can be turned up or down without touching code.

## 4. Clothes, eyes, colour: the Pi 5 avatar

### 4a. Clothes, fitted

The owner, 2026-10-10, on the first lineup: "the shirt and pants dont look right, are you using the human
maker clothing tool properly?". The first build painted the uniform onto the skin: leggings, and a tunic
whose hem was a zigzag along the triangles where the bone weights changed. It now fits real garments:

- **MPFB's clothes service** fits each garment of `clothes` (a MakeHuman CC0 `.mhclo`) to the body through
  MakeHuman's one base mesh, so a garment fits either sex and every phenotype, and skins it to the rig.
- **The pack has no uniform.** Its twelve suits are civilian: tees and jeans, a crop top, business suits,
  overalls (`docs/screenshots/crew-bodies/makehuman-cc0-suits.png`, each fitted with its own texture). The
  nearest uniform shape is `male_casualsuit02`, a long-sleeved top worn over trousers; its texture is not
  used, the palette colours it. Shoes are `shoes02`.
- **The skin under the clothes is deleted**: each garment's delete group (MPFB's mask), then every body face
  whose strongest bone is in the tunic, trousers or boots region. No triangle is spent under the clothes and
  no skin pokes through them. What stays is the head, the neck and the hands.
- **The suit is split** into its two loose pieces: the upper is the tunic, the lower the trousers. The hem,
  the cuffs and the neckline are the garment's own edges.
- **Decimated to shares of the budget**: the tunic 700 triangles, the trousers 420, the boots 200, the hair
  520, the skin the rest. The pack's garments are dense (the suit 4,120 triangles, the shoes 3,064), so this
  is where most of the cut falls.

### 4b. Big eyes

The owner, 2026-10-10: "just big eyes I think might make them fun". MPFB's low-poly eyeball is replaced by
the build's own sphere, at the old eyeball's centre and mean radius, scaled `eye_scale` (2.1), moved
`eye_forward_m` (7 mm) out of the socket so the whole white shows past the lids, the brows lifted 12 mm and
brought 8 mm forward over it. The stock eyeball has face rings only at 0.20, 0.38, 0.71 and 0.92 of its
width, so no boundary falls where the references put the iris; the build's sphere puts its rings exactly
there: a pupil fan, a ring out to the iris's edge, a ring to the equator and a fan behind, 12 sides, 48
triangles an eye (the stock one is 86).

| Ratio | The references (estimated off the pictures) | Table | Built, measured |
| --- | --- | --- | --- |
| Iris width over eye width | 0.45 (the Kirk card) to 0.58 (Elsa) | `iris_ratio` 0.55 | 0.55 on all six |
| Pupil width over iris width | about 0.4 on both | `pupil_ratio` 0.40 | 0.40 on all six |
| Eyeball width | | | 91-100 mm (a real eye is about 24 mm) |

The build prints the measured ratios for every body (the vertex extents of the painted faces). Each body
names an eye colour (`eye_colours_srgb`); the pupil is near black.

### 4c. Colour, mesh and rig

- **One colour a face**, smooth shaded, one material, no texture: the palette is the colour. Skin, hair and
  brows, the eye's white, iris and pupil, the tunic, the trousers, the boots. The **collar** is the band of
  tunic faces within `collar_width_m` (35 mm) of the tunic's neck opening (its open edge loop highest on the
  body), so it follows the neckline. The **badge** is the tunic faces round a point on the left chest.
- **The tunic carries a flag** in the colour's alpha (255 tunic, 0 the rest) so the game paints the role
  colour per body with one uniform, as `crew-on-deck` 17 asks; the prototype bakes the role colour in.
- **The rig cut to 30 bones**: the `game_engine` rig's 53 bones less its 30 finger bones, with each hand's
  fingers merged into one finger bone and one thumb bone (their weights summed), plus three prop bones
  (right hand, left hand, back) at no weight: 27 deforming bones and 3 props, within the 48 allowed. Two
  influences a vertex.

**Pi 5 cost** (measured on the build and stated in its manifest): 2,929-2,932 triangles a body and one draw
call; eight bodies aboard is about 23,500 triangles of the 200,000 a frame; 30 bones is 90 uniform vectors
of the 256 OpenGL ES 3.0 guarantees; no texture memory; about 150-180 KB a glb. Not measured on a Pi
(CLAUDE.md 2).

## 5. What the owner judges

- A lineup of six bodies (men and women, five skin tones, builds and ages) rendered front and
  three-quarter, beside a 1.80 m rail (the walk body's height) and a door's 2.1 m frame, at night under a
  key, a fill and a rim (CLAUDE.md 11), and two faces close: `docs/screenshots/crew-bodies/`.
- A page that orbits the same glbs at true scale (task 1.3, not built yet).
- The proportions, the eye ratios and the suit are first choices (recommendation taken, ask only with
  screenshots); the shots are what the owner answers.

## 6. Other clothing tools, and the uniform we should make

The owner asked whether there are other tools for clothing. What is available, for a 3,000-triangle body
built headless:

| Tool | What it gives | Fit here |
| --- | --- | --- |
| MPFB clothes, MakeHuman CC0 system assets | Fitted, rigged garments that follow every phenotype | In use; no uniform in the pack, garments are 3,000-17,000 triangles |
| MakeHuman community assets | Many more `.mhclo` garments by users | Licences vary per asset (CC0 or CC-BY, some not stated): each must be checked and pinned before use |
| **MPFB MakeClothes** | Turns any mesh modelled over the base mesh into an `.mhclo`, with a delete group, that fits every body the way the CC0 suits do | **Recommended**: model the uniform itself |
| Marvelous Designer, CLO 3D | Sewing-pattern garments with real drape | Commercial and interactive, cannot run headless; their output is dense and would be decimated anyway |
| Blender cloth simulation | Drape over a posed body | Bakes one pose; nothing here needs drape at 700 triangles |

**The next step (recommendation taken, ask only with screenshots):** a Star Crew uniform modelled low poly in
Blender by a committed generator (`tools/blender/build_crew_uniform.py`): a tunic with a set-in collar band,
a hem at the hip and cuffs, trousers that tuck into short boots, about 1,100 triangles for the lot, made into
an `.mhclo` by MakeClothes so it fits all six bodies. Its own edges then carry the collar and the badge
(a modelled patch), which ends the badge's ragged outline (below).

## Risks / Trade-offs

- **The CC0 hair is dense.** Decimated to 520 triangles it still holds; at 420 it opened holes (seen on
  `helm_f`), so 520 is the floor.
- **The badge is a patch of decimated tunic faces**, so its outline differs body to body (a pentagon on one,
  a zigzag on another). The modelled uniform (section 6) fixes it.
- **A garment fitted to the base mesh can clip** where the cartoon scales stretch a bone the garment's
  weights do not follow; the deleted skin underneath means a clip shows the garment's back face, not skin.
- **Faces are static**: the `game_engine` rig has no face bones (Undercity's note). Expression is the eyes,
  the brows and the body's clips.
