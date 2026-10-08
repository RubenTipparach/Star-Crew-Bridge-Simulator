# Design: normal maps

## Context

The owner, 2026-10-08: "Do you have normals for stuff? Should probably add normal maps to give things
that modern textured look".

| Decided elsewhere | Where |
| --- | --- |
| No normal maps, no PBR, no per-pixel lighting beyond an emissive term; relief baked into the colour | CLAUDE.md section 9 (this change proposes to amend it, once measured) |
| Light baked into vertex colours, three sets (normal, red alert, emergency power) blended by a per-compartment uniform; the 28-byte deck vertex whose three `RGBA8` sets keep their alpha reserved; up to four dynamic lights in the uniform block | `light-baking` design, `deck-pipeline` |
| One texture array: 11 materials, 55 panel layers, every prop's atlas; 256 px layers in the mockups, 128 px the Pi's choice (survey S1); 96 MB of texture memory | `surface-materials`, `wall-panels`, `ship-props` 4c, `engine-stack` "The Pi 5 budget" |
| OpenGL ES 3.0 (GLSL ES 3.00) on a Pi 5's VideoCore VII; ETC2 and EAC compressed textures are core in ES 3.0 | `engine-stack` |

## Goals / Non-Goals

**Goals:** relief that is lit from where the room's lamps actually are, in each lighting state; a
sheen on metal; the same look in the mockups and the engine; within the Pi 5's frame and memory.

**Non-Goals:** PBR (roughness, metalness, image-based lighting); real-time shadows; per-pixel
lighting of every fixture; parallax or displacement.

## 1. What a normal map needs

A normal map tilts each pixel's surface. It shows only if the light falling on the pixel is computed
from that tilt. Today every pixel takes its light from its vertices (baked) and its relief from a
fixed key light baked into the colour, so a normal map added alone changes nothing on screen. The
choice is where the pixel gets a light direction from.

## 2. The options

| | A. Today | B. Per-pixel lamps | C. Directional bake (recommended) |
| --- | --- | --- | --- |
| How | Relief in the colour under a fixed key light; light per vertex | Each pixel loops over its room's lamps (8 to 20) with the normal map; the bake keeps only bounce and occlusion | The bake stores per vertex the direction most of the light comes from and how directional it is, per state; the pixel shades the baked light by its normal map against that direction |
| Relief lit from the real lamp | No: always the upper left | Yes, from every lamp | Yes, from the strongest light at that spot (two lamps either side read as one between them) |
| Red alert and emergency light the relief from their own fixtures | No | Yes | Yes: a direction per state |
| Sheen on metal | No | Yes | Yes, a specular term from the same direction |
| Moving lights (muzzle flash, hand lamp) | Per vertex only | Yes | Yes: the four dynamic lights of the uniform block, per pixel |
| Pixel cost on the Pi 5 | One array fetch | A loop of 8 to 20 lights a pixel: far over budget on a 1 GB Pi | One more array fetch, a tangent transform, two dot products |
| Baked bounce and occlusion | Kept | Must be split from direct light | Kept |

**Recommendation: C**, with the four dynamic lights per pixel. It is the shape late-1990s and
early-2000s engines used on hardware weaker than a Pi 5's GPU (Half-Life 2's radiosity normal
mapping stored three baked directions a texel; this stores one a vertex, cheaper still), and it keeps
everything the bake already does.

## 3. The shading, option C

Per vertex, per lighting state, the bake writes:
- `E`, the light arriving, RGB (today's colour set);
- `L`, the direction it mostly arrives from, as two bytes (octahedral encoding);
- `k`, how directional it is, 0 (all bounce, from everywhere) to 1 (one lamp), in the alpha byte the
  `light-baking` design reserved.

The pixel, with `n` its normal from the normal map in tangent space turned into the world, and `n0`
the face's own normal:

```text
direct  = k * max(0, n . L) / max(0.25, n0 . L)      // relief re-lit; 1 on a flat pixel
diffuse = E * ((1 - k) + direct)
spec    = gloss * k * pow(max(0, n . H), 24) * E      // H the half vector to the eye; gloss from the normal layer
colour  = albedo * diffuse + spec + emissive
```

On a flat pixel (`n = n0`) `direct` is `k`, so the colour is exactly today's baked colour: nothing
gets brighter or darker on average, only the relief moves. The `0.25` keeps a grazing face from
exploding. The three states' `E`, `L` and `k` blend by the same per-compartment weights as today.

## 4. Data

**The vertex** grows from 28 to 36 bytes: `L` per state (3 x 2 bytes) and a tangent (`RGBA8` snorm,
its handedness in w), plus 2 bytes of padding. On the Tern's measured deck (`deck-pipeline` 13a:
the deck file read whole is 62.2 MB) that is about 29% more vertex memory; the deck is streamed by
compartment and the budget table gets the measured figure.

**The normal layers.** Each colour layer gets a normal layer of the same size: tangent-space X and Y
(Z rebuilt in the shader) and a gloss mask, `RGB8` or `ETC2 RGB` compressed. In memory, with mips:

| Layers | Colour today (256 px, RGBA8) | Normal, RGB8 | Normal, ETC2 (0.5 byte a texel) |
| --- | ---: | ---: | ---: |
| 11 materials + 55 panel layers | 23.1 MB | 17.3 MB | 2.9 MB |
| Prop atlases (86, most 256 px, a few 512-1024) | 53.1 MB | 39.8 MB | 6.6 MB |
| At the Pi's 128 px layers | a quarter of each | | |

So the normal layers cost 9.5 MB compressed against the 96 MB texture budget. Uncompressed they would
not fit beside today's colour layers (76 MB already, which is itself over what a 1 GB Pi should hold at
256 px: the Pi's 128 px choice quarters it); compressing the colour layers too is a separate step.

**Where the normals come from.** Nothing is drawn by hand:
- Panels (`build_wall_panels.py`) and props (`hs_kit.py`, every set) are already modelled in 3D for
  their bakes: Blender bakes the normal layer from the same detail geometry, selected to active, in
  the same pass as the colour.
- Materials (`surface-materials`): a Material Maker graph has a normal output; it is exported
  instead of being folded into the colour.
- The colour bakes stop applying their key light (`bake.key_tangent`, the panels' sun) and keep
  occlusion, wear and the albedo: otherwise the light is counted twice. A page or a deck without the
  normal layers falls back to today's baked-relief colour layers, so both are built until the switch.

## 5. The Pi 5 budget

Not measured: a cloud session has no Pi and no GPU (CLAUDE.md 2). Before anything ships, the probe
(`engine-stack` task, the deck shader) measures the deck pass at 1280 x 720 on a Pi 5, today's
shader against option C's, interleaved, repeats and spread reported. Option C goes ahead only if the
difference fits the frame budget; if it does not, the fallbacks in order are: normal maps only within
10 m (a distance fade to today's look), then only on props, then not at all.

## 6. Steps

1. **A comparison in the deck plan** (`?normals=1`): one room, the bridge, with normal layers baked
   for its panels and props, today beside option C in the three lighting states. Screenshots in
   `docs/screenshots/mockups/normal-maps/`, for the owner to judge.
2. On the owner's word and the probe's: the bake writes `L` and `k`; every Blender bake writes its
   normal layer; the mockups take them; CLAUDE.md section 9 is amended in the same commit.
3. The engine's deck shader and `deckc` (the vertex grows), measured on the Pi.

Recommendation taken (ask only with screenshots): option C, step 1 first. The owner judges it on the
comparison's shots.

## Risks / Trade-offs

- One direction a vertex: between two lamps the relief reads as lit from between them, and across a
  big face lit unevenly the direction swings at vertex spacing (the bake's 2 m cells, finer where
  subdivided). Acceptable for relief; a light per texel (a directional lightmap) is the next step up
  and costs a lightmap.
- Rebaking every texture: the panel build takes about 45 minutes, each prop set 10 to 60. Done once,
  in the background.
- Two sets of colour layers during the switch: kept only until every page and the engine use the
  normal layers.
