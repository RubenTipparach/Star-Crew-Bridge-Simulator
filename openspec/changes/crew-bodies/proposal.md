# Proposal: crew bodies, cartoon low poly from MakeHuman

## Why

The owner, 2026-10-10: "using make human in blender can you make better low poly characters? exagerate
portions a bit to make them look more cartoony".

The crew aboard today are the first version's figures (`crew-npcs` 7): about 100 triangles of boxes, a
tunic in the department's colour, built by `deckc`'s generator. `crew-on-deck` 17 already budgets a real
avatar (3,000 triangles, 30 of 48 bones, palette colours, one draw call) but no body has been made. The
owner's other project, Undercity, builds its people from MakeHuman through MPFB 2 in Blender, headless
and from a table (the `blender-humanoid-characters` skill); its bodies are a desktop's (16,000
triangles, 53 bones, 1024 px atlases), five times the Pi 5's avatar.

## What Changes

- **A body table and a build** (design sections 1 and 2): `data/crew/bodies.json` names each crew body
  (sex, age, build, skin tone, hair, height) and `tools/blender/build_crew_bodies.py` makes each one
  from MakeHuman's CC0 assets through MPFB, the same pinned packs Undercity fetches.
- **Cartoon proportions** (section 3): the head, hands and feet are scaled up on the rig before the rig is
  applied, the shoulders broadened and the waist taken in, so a body reads at a glance across a room:
  about four and a half heads tall instead of seven and a half.
- **Fitted clothes and big eyes** (section 4): MakeHuman garments fitted by MPFB with the skin under them
  deleted (owner, 2026-10-10: "the shirt and pants dont look right"), and big cartoon eyes whose iris and
  pupil ratios are measured off the owner's references ("just big eyes", "measure pupil ratio").
- **Fitted to the Pi 5 avatar** (section 4): one mesh of at most 3,000 triangles, smooth shaded, coloured
  per face from the palette (skin, hair, eyes, tunic, collar, badge, trousers, boots), the tunic flagged for
  the role colour; the rig cut from 53 bones to 30 (the fingers merged, three prop bones added).
- **Prototypes first**: a lineup of six bodies rendered beside today's figure and a door for scale, and a
  page to orbit them, for the owner to judge before any body goes into `deckc` or the client.

## Impact

- `crew-on-deck` 17 (the avatar): this change builds it; its budget is unchanged.
- `crew-npcs` 7: the bot figures are replaced once the owner approves, not before.
- New: `data/crew/bodies.json`, `tools/blender/build_crew_bodies.py`, `assets/models/crew/`,
  `docs/mockups/crew-bodies.html`, shots in `docs/screenshots/crew-bodies/`.
- Packs: MPFB 2.0.17 and MakeHuman's CC0 system assets, pinned by SHA-256 in Undercity's
  `tools/deps/character_packs.json`; copied here as `tools/deps/character_packs.json` with provenance.
