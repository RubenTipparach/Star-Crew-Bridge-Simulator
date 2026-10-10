# Design: crew characters

## Context

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
