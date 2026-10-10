# Design: static light baking

Status: **proposed** (2026-10-04). Nothing here is built in the engine. The mockup baker
(`docs/mockups/lib/lightbake.js`) is documentation tooling that follows the method below so the
pictures and numbers are real bakes, not paintings (CLAUDE.md section 4). Numbers are labelled:
**measured** means measured by that baker in `docs/mockups/lighting.html` on this cloud machine's
CPU, in one browser thread (not a Pi 5, not the engine baker; the GPU plays no part in a bake);
**estimate** means worked out from those measurements or from published figures; **decided**
means decided by the owner or by CLAUDE.md.

## Context

The owner, 2026-10-04: "Some sort of tool or skill for light mapping/static light baking would be
nice too"; "That's fine..this game doesn't need high end graphics. Your stack sounds like a solid
plan"; "I'm sorry we're running on a pi5 1gb-4gb, 4 GB can be used as main server too"; "I'm all
for low poly aesthetics."

| Decided elsewhere | Where |
| --- | --- |
| Flat-shaded, vertex-coloured low poly; no normal maps, no PBR; light baked into vertex colours, one set per lighting state, blended by a per-compartment uniform; light baked offline from fixtures that are data | CLAUDE.md section 9 |
| Rust, SDL3 (3.4 or later) and sokol_gfx on OpenGL ES 3.0 (glow until 2026-10-07); the Pi 5 budget table; shader 1 (deck) "three baked vertex colour sets blended by per-compartment uniforms ... an optional lightmap (`light-baking`)"; `sc-tools` holds `bake` | `engine-stack` sections 3, 5 and 7 |
| Fixtures are entities with a colour and intensity per state and an emergency-bus flag; the bake is step 4 of `deckc`; the 28-byte vertex with three `RGBA8` sets whose alpha is reserved for this change; the per-compartment uniform block (state weights eased over 0.5 s, a dimmer from the lighting bus voltage, a damage flicker, up to four dynamic lights); the `.deck` file | `deck-pipeline` sections 5, 7 and 8 |
| The red-alert look (red lamps and strips, or white lamps with red strips) | `bridge-stations` question B3 |

This change owns **how light is computed**: what is baked, into what, at what density, how the
states and probes come out of one bake, what runtime lights add, how leaks are prevented, and
the baker's inputs, outputs, determinism and speed.

### References, and what each is cited for

| Reference | Cited for |
| --- | --- |
| Quake's `light` tool (id Software, 1996; source released 1999) | Light baked offline from point-light entities into lightmaps, direct light only, shadows by tracing rays against the level's own geometry. Lightmap density of one sample per 16 texture units (to verify). |
| Quake II's `qrad3`, and Half-Life's `hlrad` derived from it (Valve, 1998) | Radiosity bounces, and surfaces that emit light from their texture ("texlights", Half-Life's `lights.rad`): our emissive screens and strips as area lights (to verify the file and option names). |
| q3map2 (the Quake III map compiler, GtkRadiant and NetRadiant) | Bounce passes (`-bounce`), supersampling (`-samples`), a filter over the lightmap (`-filter`), area lights from shaders, and "dirtmapping" (`-dirty`), an ambient-occlusion term for corners (to verify the options). Also the known artefacts (leaks under brushes, splotches) that our checks list. |
| PlayStation and Nintendo 64 era games | Light baked into per-vertex colours on low-poly meshes, with detail bought by splitting geometry where light changes: the look CLAUDE.md section 9 chose (to verify per title before citing one). |
| Half-Life 2 and the Source engine (Valve, 2004; Mitchell, McTaggart and Green, "Shading in Valve's Source Engine", SIGGRAPH 2006 course) | The ambient cube: six colours, one per axis direction, sampled at points in the level and used to light moving models so they match the static lighting (to verify the paper's details). |
| Ward, Rubinstein and Clear, "A Ray Tracing Solution for Diffuse Interreflection", SIGGRAPH 1988 | Irradiance caching: indirect light is smooth, so gather it at sparse points with many rays and interpolate, instead of gathering at every output sample. |
| Roberts, "The Unreasonable Effectiveness of Quasirandom Sequences" (2018) | The R2 low-discrepancy sequence used for every sample pattern. |
| Duff et al., "Building an Orthonormal Basis, Revisited", JCGT 2017 | The branchless tangent frame used for disc and hemisphere samples. |
| Moller and Trumbore, "Fast, Minimum Storage Ray/Triangle Intersection", 1997 | The ray-triangle test. |
| Karis, "Real Shading in Unreal Engine 4", SIGGRAPH 2013 course | The windowed inverse-square falloff, so a light has a finite range with no visible edge (to verify the exact form). |
| Interior lighting practice (CIBSE and IES guides) | Downlight spacing of about 1 to 1.5 times the mounting height for an even floor (to verify the figures). |

## Goals / Non-Goals

**Goals**

- Rooms that read well on the low-poly look: light pools under lamps, soft shadows under consoles
  and seats, dark corners, coloured spill from screens, strips and the reactor, in all three
  lighting states.
- The simplest thing that does that on a 1 GB Pi 5, with the cost and benefit of each step up
  stated and measured.
- Moving things lit to match the walls; a few runtime lights for events.
- No leaks; reproducible bakes; a bake time that keeps iteration fast.

**Non-Goals**

- Where light lives, the vertex format, the blend uniforms and the `.deck` layout: `deck-pipeline`.
- Real-time global illumination, shadow maps for interior lights, per-pixel lighting of fixtures,
  normal or specular maps (CLAUDE.md section 9; `engine-stack` "Out").
- The red-alert colour scheme itself: `bridge-stations` B3 decides it, this bakes it.
- Which compartments are powered: `power-grid` simulates the buses; this reads the result.

## Decisions

### 1. What is baked

Every term is computed by **one irradiance function** at a point with a normal and a seed key.
Vertices, lightmap texels, irradiance-cache points and probe faces all call it, so every output
agrees by construction (in the mockup, `LightBake.irradiance`).

| Term | Source | How | Units |
| --- | --- | --- | --- |
| Direct, point fixtures | Ceiling panels, high-bay lamps, wall lamps | A light of radius `radius_m` (a disc facing the receiver), sampled with up to `shadow_samples` shadow rays: 4 first, the rest only when the first 4 disagree (a penumbra). Beam `cos^n` about the fixture's axis. | Intensity `intensity_cd` (candela on the axis) |
| Direct, emissive surfaces | Light strips, console screens, the viewscreen, the reactor's glow bands, indicator panels | The emitting quad sampled with one point per 0.06 m^2 and at least one per 0.25 m of its longest edge (4 to 64 points), cosine at both ends | Luminance `luminance_cd_m2` (candela per square metre) |
| Ambient fill | A small per-state fill so nothing is pure black | Times ambient occlusion: the fraction of `ao_rays` cosine-weighted rays of length `ao_radius_m` (0.8 m) that hit nothing | Lux |
| Bounce | Light reflected by every surface | One diffuse bounce by default (0 to 2), gathered at the irradiance cache and interpolated (below) | Lux |

**Direct light from a fixture**, averaged over its N samples k on the disc:

```text
E = (1/N) * sum_k  V_k * I * cos^n(theta_axis,k) * cos(theta_r,k) * W(d_k) / max(d_k^2, 0.04 m^2)
W(d) = (1 - (d / range_m)^4)^2 for d < range_m, else 0
```

`V_k` is 1 when the shadow ray reaches the sample, `theta_r` the angle at the receiver,
`theta_axis` the angle from the beam axis. The 0.04 m^2 floor stops a hot spot on a surface that
touches a lamp. **Emissive surfaces** add `L * cos(theta_e) * cos(theta_r) * dA * W(d) / max(d^2, dA)`
per sample point, `dA` being the quad's area over its sample count.

**Bounce, by an irradiance cache** (Ward et al. 1988). Every surface is also tessellated on a
uniform 0.5 m grid, the cache. Direct light is computed at each cache vertex; then one gather of
`cache_gather_rays` (128) cosine-weighted rays per cache vertex reads the direct light where each
ray lands, times that surface's albedo (`E_bounce = (1/M) * sum albedo(hit) * E_direct(hit)`; the
cosine sampling's pi cancels the diffuse surface's 1/pi). A 3 x 3 tent filter, run once over each
face's own cache grid and never across faces, smooths the result, and every output point takes the
bounce bilinearly from the four cache vertices around it. A second bounce gathers once more
before that. Why: a surface lit mostly by bounce sees the bright lamp pools through only a few of
its rays, and gathering 48 rays at every output point left splotches on engineering's upper walls
(seen in the first shots). Gathering 128 rays on the cache and filtering is quieter. It is not
cheaper on a small room: the cache gather is a fixed 1.6 s on the bridge, and engineering's 0.25 m
lightmap went from 12.1 s to 11.4 s (measured). Quality is the reason.

**Rays.** One bounding volume hierarchy per compartment over every face, prop and closed door
leaf (median split, leaves of 4 triangles). Occlusion is **two-sided**: the back of a wall blocks
light as well as its front. A gather ray that meets the **back** of a face reads black, because
that point is inside a wall or a prop. Ray origins are offset 4 mm along the normal (no shadow
acne) and sample points sit 1 cm inside their face's edges (no light read off the next wall).

**Three states from one bake.** Light is linear in its sources. Each fixture carries a colour
for each state (`[0, 0, 0]` where it is off), and every ray's result is multiplied by all three at
once, so every receiver stores nine numbers (three states times RGB). Baking three states costs
the rays of one (by construction: the ray count does not depend on the number of states; only
the 8-bit encoding runs three times).

**Encoding.** The stored value for a state is `x = E / reference_lux` (100 lx shows a surface at
exactly its palette colour), gamma-encoded `x^(1/2.2)`, with a soft shoulder above 1.5 that rolls
off to 2.0 instead of clipping, stored in 8 bits as `d / 2 * 255` (2x overbright: byte 128 is the
palette colour, as Quake III's overbright bits did, to verify). A seeded dither of plus or minus
half a level breaks up banding in slow gradients. The deck shader computes `albedo * 2 * (c0 w.x +
c1 w.y + c2 w.z) * dimmer`; blending encoded values during the 0.5 s crossfade is a small,
invisible approximation. The **alpha** channel, reserved for this change by `deck-pipeline`,
holds ambient occlusion (0 to 1 in 0 to 255); runtime lights and probes use it
(sections 6 and 7).

### 2. Into what: the options, on a Pi 5

Measured on the v1 boxes, before the page took the kit's geometry and lamps: the Tern's bridge
(154 m^2, 6 lamps, 10 emissive surfaces) and engineering (252 m^2 over three decks, 16 lamps, 44
emissive surfaces), one bounce, all three states. What the page prints on the v2 plan is in
section 13; the rest of this table and those of sections 3 and 4 are not rerun. Vertex memory uses
`deck-pipeline`'s 28-byte vertex and 16-bit indices; a lightmap adds a second UV (4 bytes).

| | a. Vertex, deck mesh | **b. Vertex, adaptive (recommended)** | c. Lightmap 0.25 m everywhere | d. Hybrid: b, plus c in named large rooms | e. Runtime lights only |
| --- | --- | --- | --- | --- | --- |
| Bridge triangles | 412 | **3,160** | 412 | 3,160 | 412 |
| Engineering triangles | 354 | **5,193** (capped; 12,776 uncapped) | 354 | 354 (lightmapped) | 354 |
| Bridge memory | 25 KB vertices | **86 KB vertices** | 28 KB vertices + 252 KB texture | 86 KB | 25 KB |
| Engineering memory | 21 KB | **128 KB** (286 KB uncapped) | 24 KB + 561 KB texture | 24 KB + 561 KB | 21 KB |
| Tern, estimate | about 2.3 MB vertices and indices (`deck-pipeline`) | **about 3.3 MB** (+1 MB) | 2.5 MB + about 3.7 MB textures | about 3.3 MB + about 1.5 MB | 2.3 MB |
| Per pixel on the Pi 5 | Nothing extra | **Nothing extra** | One more bilinear fetch per deck pixel, two during a crossfade (cost to measure, estimate small) | As c, in those rooms only | Per-pixel lighting of every fixture: out of budget beyond a few lights |
| Per vertex | 9 multiply-adds (the blend) | **The same** | The same, plus the UV | The same | Every fixture's falloff |
| Looks (shots) | Flat and dark: a face's light is its corners' (`lighting-bridge-vertex.png`) | **Pools, console and seat shadows, corners** (`lighting-bridge-vertex-subdivided.png`) | The same at playing distance; smoother shadow edges up close (`lighting-split-vertex-vs-lightmap.png`) | As b and c | Highlights but no shadows; light passes through consoles (`lighting-bridge-dynamic.png`) |
| Bake, bridge (one thread) | 1.7 s | **2.2 s** | 3.3 s | as b | none |
| Bake, engineering | 4.8 s | **6.0 s** (7.3 s uncapped) | 11.4 s | 11.4 s | none |
| Engine work beyond b | | | A second UV in `deckc` and the vertex, an atlas packer with gutters, a texture per ship, a shader variant, T-junction welding that keeps UVs | All of c, for a few rooms | Not a bake |

**Recommendation (question G1): b, vertex colours with adaptive subdivision**, as CLAUDE.md
section 9 already says. It is the simplest thing that reads well: the pools and shadows that
make a room look lit, for a few thousand triangles per room, no texture memory and nothing per
pixel. At playing distance it looks the same as a 0.25 m lightmap (`lighting-split-vertex-vs-lightmap.png`,
`lighting-split-vertex-vs-lightmap-room.png`). The baker keeps the lightmap output (one
irradiance function, a different set of sample points), so c or d stays a measured fallback if a
room cannot reach its error target inside its triangle cap; adopting it changes CLAUDE.md section
9 and the vertex format, which is the owner's call.

**What each step up costs and buys:**

| Step | Costs on a Pi 5 | Bake | Buys | Verdict |
| --- | --- | --- | --- | --- |
| 0. Vertex light on the deck mesh | Nothing | 1.7 s bridge | A colour per brush-face corner: no pools, no shadows | Not enough (`lighting-split-deck-mesh-vs-adaptive.png`) |
| 1. Adaptive subdivision | About 2,500 triangles on the bridge, about 6 per m^2 of lit surface elsewhere, capped | +0.5 s bridge | Pools, console and seat shadows, contact darkening | **Take** |
| 2. One bounce | Nothing at run time | +1.6 s bridge, +4.7 s engineering | Light under desks and in shadow, colour bleed from screens and the reactor, a less harsh contrast (`lighting-bridge-vertex-no-bounce.png` against `lighting-bridge-vertex-subdivided.png`) | **Take** |
| 3. A second bounce | Nothing at run time | +0.6 s bridge, +2.3 s engineering | Almost nothing visible (`lighting-bridge-vertex-two-bounces.png`) | Leave at 1 (G4) |
| 4. Runtime lights, up to 4 per compartment, per vertex | A few dozen operations per vertex for the ones in use | None | Muzzle flashes, sparks, beacons, fire, door spill (section 7) | **Take** |
| 5. Lightmaps (c or d) | +4 bytes per vertex, 0.25 to 0.6 MB per large room, about 3.7 MB for the Tern at 0.25 m, one fetch per pixel, a shader variant, an atlas in `deckc` | +1.1 s bridge, +5.4 s engineering over b | Sharper shadow edges up close; a flat triangle count in big rooms | Fallback (G1, G2) |
| Out | Shadow maps for interior lights, per-pixel lighting of every fixture, real-time GI | | | Not this game |

### 3. Adaptive subdivision

Each receiving face (a planar quad from `deckc`) starts as a grid of cells about `base_m` (2.0 m)
on a side. A cell is tested by computing the light at its centre and edge midpoints and comparing
each with what the cell's corners would interpolate there; the error is the largest difference,
in 8-bit display levels, over the three states. Cells split into four, highest error first (a
priority queue, ties broken by stable key), while the error exceeds `max_error_levels` (5) and the
cell is larger than `min_m` (0.25 m), until the compartment's `max_added_triangles` is spent. The
quadtree is then balanced (neighbouring cells differ by at most one level) and a cell next to a
finer one is fanned from its centre, so there is no T-junction inside a face. Each cell is split
along the diagonal whose ends agree best, which keeps zig-zags out of gradients. Refinement looks
at direct light and occlusion; bounce, which is smooth, is added to the final vertices only.

Vertices are shared inside a face (one normal, one colour each) and never across faces, which
keeps the flat-shaded look and lets the index buffer reuse vertices: the bridge's 3,160 triangles
use 2,479 vertices (measured on the v1 boxes; 6,070 and 5,404 on the v2 plan, section 13).

| Measured | Base 1 m, 5 levels | **Base 2 m, 5 levels** | Base 2 m, 3 levels | Base 2 m, 8 levels | Uniform 0.25 m grid |
| --- | ---: | ---: | ---: | ---: | ---: |
| Bridge triangles | 6,625 | **3,160** | 3,789 | 2,663 | 17,596 |
| Engineering triangles (no cap) | 14,185 | **12,776** | 19,790 | 9,355 | 59,928 |

A 2 m base reaches the same error with half the bridge's triangles, because even areas stay
coarse; a uniform grid fine enough for the shadow edges would cost five times more.

**The cap.** `max_added_triangles` per compartment is proposed as the smaller of a quarter of
its ceiling in the `engine-stack` table and what its kit geometry leaves under that ceiling
(`deck-pipeline` section 11), set in `data/lighting/bake.json`. With it, the bridge converges
(2,472 added of 7,500; 4.9 levels residual) and engineering stops at its 2,000 with a residual of
20 levels in places that are hard to find by eye (`lighting-engineering-cap-vs-no-cap.png`: left
capped at 5,193 triangles, right converged at 12,776; v1 boxes). On the v2 plan, with the kit's
detail and lamps, the bridge still converges (6,070 triangles, 5.0 levels) and engineering stops
at its cap with a residual of 36.8 levels (7,514 triangles); the uncapped figure and the
comparison shot are not rerun. The bake report prints the residual and whether the cap was reached; `deckc`'s budget check counts triangles after subdivision
(`deck-pipeline` section 7). Question G2 asks whether engineering keeps its cap.

**The cap is not yet a ceiling** (found 2026-10-05). The mockup baker (`lightbake.js`
`bakeAdaptive`) counts only the splits it chooses by error against `max_added_triangles`. The
balancing pass and the fans beside finer cells come after, and the cap does not see them. On the
v2 plan engineering's base grid is 3,718 triangles and its bake 7,514: 3,796 added against a cap
of 2,000, of which 1,998 are error-driven splits (333 splits of 6 triangles) and 1,798 come from
balancing and fans. The bake `deckc` runs SHALL count every triangle it adds, balancing and fans
included, and stop splitting while the total still fits: a split is taken only if it and the
balancing it forces fit what is left. Until then the mockup's "cap reached" means the
error-driven share is spent, not the total.

### 4. The lightmap alternative

Kept as the measured fallback and as a debug comparison. Every receiving face gets a block of
texels of at most `lightmap_texel_m` (0.25 m) on a side plus a one-texel gutter copied from its
edge (bilinear filtering never reads a neighbour's light), shelf-packed into a power-of-two-wide
atlas. Three `RGBA8` layers, one per state, in one texture array (allowed by OpenGL ES 3.0), so the
deck shader binds one texture and samples two layers during a crossfade. The vertex gains a
second UV (`2 x u16`, normalized, 4 bytes).

| Measured | 0.5 m texels | 0.25 m | 0.125 m |
| --- | --- | --- | --- |
| Bridge atlas, 3 states | 128 x 59, 89 KB, 0.5 s | 256 x 84, 252 KB, 1.8 s | 256 x 223, 669 KB, 6.7 s |
| Engineering atlas, 3 states | 256 x 73, 219 KB, 1.6 s | 256 x 187, 561 KB, 6.7 s | 512 x 338, 2.0 MB, 24.5 s |

Bake times above exclude the shared irradiance cache (1.6 s bridge, 4.7 s engineering). A whole
Tern atlas at 0.25 m is about 3.7 MB for three states (estimate: about 10,500 m^2 of lit surface
including props, 16 texels per m^2, 55 % fill), 4 % of the 96 MB texture budget: memory is not
what argues against it; the extra per-pixel fetch, the second UV, the packer and the seams are.

### 5. Three lighting states, power loss and the dimmer

- **The states** are `normal`, `red_alert` and `emergency` (CLAUDE.md section 9). Every fixture
  type has a colour and scale per state (section 11). A lamp not on the emergency bus is black in
  the emergency set; one on it uses its type's `emergency` colour and `emergency_scale`. Screens
  stay lit in every state, dimmer in emergency (`bridge-stations` B4 decides which).
- **The blend** is `deck-pipeline`'s: per-compartment weights eased over 0.5 s, times the
  dimmer. Red alert is the ship's alert level; emergency is the compartment's lighting bus on the
  emergency bus or batteries.
- **The dimmer** is the lighting bus voltage over its nominal from `power-grid`, 0 to 1. It scales
  the encoded (display) value, so 60 % voltage reads as 60 % brightness, which is the number the
  engineering console shows. A dead bus is dimmer 0: only emissive faces, probes' emissive share
  and runtime lights remain.
- **Power loss** (the mockup's "Power loss" button): the normal set flickers by a seeded pattern
  for 0.6 s (in the mockup: off at 0 ms, on at 90, off at 160, 70 % at 260, off at 330, 35 % at
  520, off at 600), then the emergency set fades in over 0.5 s. The flicker's seed comes from `sc-core`'s session seed,
  the compartment id and the event tick (CLAUDE.md 6.4), so every client sees the same flicker.
  The pattern's timings are data in `bake.json`'s `power_loss` block, read by the client.
- **One failed lamp** is not representable in a three-set bake; damage dims or flickers a whole
  compartment through the uniform block, and a sparking fixture is a runtime light (section 7).
  Recommendation taken (ask only with screenshots): per-compartment only.

### 6. Light probes for things that move

Crew avatars, craft in the hangar, carried props and opening door leaves are not in the bake.
They are lit by **ambient cubes** (Half-Life 2): six irradiance values per point, one for a
surface facing each of +X, -X, +Y, -Y, +Z, -Z, computed by the same irradiance function as the
walls (direct, occlusion, bounce), for all three states.

- **Grid:** every 1.0 m on all three axes inside each compartment's air brushes, offset 0.5 m from
  the walls. The v1 plan's 8,859 m^3 of air gave about 8,900 probes (estimate); the v2 plan's
  9,824.8 m^3 is 10.9 % more air, and the probe count, storage and time below are not re-estimated.
- **Storage:** 6 directions x 3 states x RGB8 = 54 bytes per probe; about 0.48 MB for the Tern in a
  `PROB` chunk of the `.deck` (proposed to `deck-pipeline`), client only.
- **Invalid probes:** a probe whose gather rays hit back faces more than half the time is inside a
  prop; it is marked invalid and its neighbours fill in.
- **At run time,** each moving body takes the eight probes around its origin (trilinear, invalid
  ones skipped and the weights renormalized), blends the three states with its compartment's
  weights and dimmer, and passes six colours to the vertex shader, which computes `n.x^2 * cube[+-X]
  + n.y^2 * cube[+-Y] + n.z^2 * cube[+-Z]` (the mockup's crew figure does exactly this).
- **Cost, measured:** 3.6 ms per probe on the bridge and 4.4 ms in engineering (one JS thread), so
  about 35 s for the Tern's grid in the mockup baker; at run time a few microseconds per body.

### 7. Runtime lights over the bake

Up to **four** per compartment, the slots `deck-pipeline` gives its uniform block. Computed per
vertex, unshadowed, with the same windowed inverse-square falloff as the bake, and added in light
units rather than display units: the vertex shader decodes the baked value (`pow(c, 2.2)`), adds
`E_dynamic / reference_lux` multiplied by the vertex's baked occlusion (alpha), and encodes again.
Two `pow` per vertex keeps a muzzle flash in a lit room from looking washed out and keeps corners
dark. Moving bodies add the same terms to their probe light.

| Use | Light | Driven by |
| --- | --- | --- |
| Muzzle flash, weapon impact | White-yellow, 0.05 to 0.1 s | Weapon events (`weapons-and-shields`, boarding) |
| Sparks from a damaged console or junction box | Blue-white, flickering by a seeded pattern | Damage state (`damage-control`) |
| Alarm beacon | Amber or red, rotating: intensity follows the beam's angle to the room | Alert and alarm state |
| Fire glow | Orange, flickering | The fire simulation |
| Door spill | The neighbour's light through an open door (section 9) | Door state, the neighbour's weights and dimmer |
| The viewscreen | The feed's average colour (optional), over the baked average | The viewscreen pass |

The client picks the four by intensity at the compartment's centre and fades a light in or out
over 0.1 s when it enters or leaves the four, so nothing pops. A hand torch in a dark compartment
needs a per-pixel spotlight (per vertex it is a blob on 2 m cells); that is one light for the local
player only and is left for `crew-on-deck` to ask for.

### 8. The exterior

The ship's exterior turns relative to the sun, so its direct light is not baked: the sun is a
runtime Lambert term per vertex (`engine-stack` program 2). The baker bakes the hull's **ambient
occlusion only** (the hull's own geometry, `ao_radius_m` 2.0 m for the exterior), into the alpha of
the hull's vertex colour, and it multiplies the ambient and planet-shine term so recesses, turret
wells and the hangar mouth stay dark. Running lights and lit windows are emissive faces. A sun
shadow map is on `engine-stack`'s "maybe, after the probe" list and is not part of this change.

### 9. Light leaking, doors and windows

- **Each compartment is baked alone.** Its scene is its own faces, props, detail and door leaves;
  the neighbours are not in it. Partitions have no thickness (`deck-pipeline` K1), so light cannot
  leak through one: there is nothing on the far side to light. Compartments also bake in
  parallel this way (section 10).
- **Doors are baked closed** in all three states: leaves, hatches and pressure doors are occluders
  and receivers in their closed position (the mockup bakes the door leaf in every opening).
- **An open door spills light** through a runtime light (section 7): placed 0.3 m inside the opening
  on each side, coloured by the neighbour's mean baked irradiance over the opening, which the baker
  stores per portal side and per state (a spill record, proposed for `deck-pipeline`'s `PRTL`
  chunk: three RGB8 colours per side, 18 bytes per portal), times the neighbour's state weights,
  its dimmer and the door's open fraction. A closed door's viewport (`deck-pipeline` K4) shows the
  neighbour drawn with its own bake.
- **Inside a compartment** leaks come from points under or inside props and from points on a
  wall's edge. Two-sided occlusion, black back-face hits, the 1 cm inset and the 4 mm ray offset
  handle them; a floor vertex under a console pedestal reads dark, which shows as a soft contact
  shadow around the pedestal and is wanted.
- **Windows** show space, which is dark: no sunlight is baked through them, and a planet in view
  does not light the bridge. Recommendation taken (ask only with screenshots); revisit if a scene
  puts a sun in a window.
- **T-junctions.** Faces split at portal openings meet unsplit faces; where they do, a pixel-wide
  crack can show (seen: pinholes along a seam on engineering's aft wall in the mockup, which does
  not weld). `deckc`'s T-junction pass (`deck-pipeline` section 5) runs after the baker's
  subdivision, and a vertex it inserts on an edge takes the colour interpolated along that edge,
  so the weld changes no light.

### 10. The engine baker: `sc-tools bake` (planned, not built)

A module of the `sc-tools` crate (`engine-stack` section 3), called by `deckc` per compartment
(compile step 4) and runnable alone for previews.

```text
sc-tools bake --ship tern [--compartment bridge] [--preset preview|final] [--threads N]
              [--report out/bake-report.json] [--lightmap 0.25]   (the comparison output)
```

**Inputs** (per compartment, from `deckc`): the receiving faces as planar polygons with their
palette albedo and a stable face index; the occluders (colliding and detail brushes, props, door
leaves closed); the fixture records (section 11) and the emissive faces with their fixture type;
`data/lighting/fixtures.json`; `data/lighting/bake.json`; the compartment's portals and, for the
exterior, the hull mesh. **Outputs** (back to `deckc`): the tessellated faces with three `RGBA8`
colours per vertex (alpha: occlusion); the probe grid; the portal spill records; a bake report
per compartment (triangles before and after, residual, cap reached, rays, time, digest); on
request the lightmap atlas for comparison.

**Threads.** Compartments are independent, so a work queue hands them to `N` threads (default: the
machine's cores). Inside a large compartment the cache, the probes and the final vertices are
evaluated in parallel batches. Subdivision runs level by level: all cells of one level are
evaluated in parallel, then split in priority order on one thread, so the result does not depend
on scheduling.

**Determinism.** No sample depends on thread, order or time. Every sample pattern is an R2
sequence rotated by a hash of (bake seed, a stable key, purpose); the key is built from the
compartment id, `deckc`'s stable face index and the grid coordinates; results are written to their
index; sums run in a fixed order. The same inputs, seed and baker version give byte-identical
output (measured in the mockup: the same digests from separate browser sessions, for instance
`2f8f52d6` for the bridge's adaptive bake, and a different digest with seed 2). Across x86-64 and
aarch64 the baker uses no fast-math, no fused multiply-add it did not ask for, and the `libm`
crate for `pow`, `sin`, `cos` and `tanh`, so desktop and Pi bakes match (to verify; if they do
not, the digest test is pinned per platform). A **digest test** pins the SHA-256 of each Tern
compartment's bake output; a deliberate change updates it in the same commit. A cache keyed by the
SHA-256 of a compartment's inputs, the fixture types, the settings and the baker version skips
unchanged compartments.

**Speed, estimate.** The mockup baker traces 0.4 to 0.65 million rays per second in one browser
thread (measured) and bakes about 5 ms per square metre of lit surface at the final settings with
one bounce (measured: bridge 2.2 s, engineering 6.0 s). The Tern had about 8,100 m^2 of shell
(measured from the v1 layout; not re-measured on v2) and perhaps 10,500 m^2 with props: about 1 minute in the mockup baker,
plus about 35 s of probes. A Rust BVH tracer is usually 10 to 30 times faster per thread than this
JavaScript one (to verify with the first build), so a full Tern bake should take a few seconds on
an 8-thread desktop and well under a minute on a Pi 5's four cores. A single compartment, the
usual edit, is under a second on a desktop. The target written into the spec is the report, not a
time: every bake prints its time per compartment, and a regression shows there.

### 11. Fixtures and bake settings as data

Units are in the keys (CLAUDE.md 6.5); an unknown key is an error, a present zero is zero, a
non-finite number stops the load with its path and field.

**`data/lighting/fixtures.json`** (fixture types; values are the mockup's):

```json
{
  "schema": "starcrew.fixtures/1",
  "types": {
    "ceiling_panel": {
      "kind": "point", "intensity_cd": 850, "radius_m": 0.25, "beam_exponent": 8, "range_m": 9.0,
      "lens_size_m": [0.6, 0.6], "emergency_scale": 1.6,
      "states": {
        "normal": { "color_srgb": "#ffe9c8", "scale": 1.0 },
        "red_alert": { "color_srgb": "#ff3030", "scale": 0.75 },
        "emergency": { "color_srgb": "#ff8a1c", "scale": 0.35 }
      }
    },
    "high_bay": {
      "kind": "point", "intensity_cd": 2200, "radius_m": 0.3, "beam_exponent": 10, "range_m": 14.0,
      "lens_size_m": [0.7, 0.7], "emergency_scale": 1.6, "states": "same as ceiling_panel"
    },
    "cove_strip": {
      "kind": "area", "luminance_cd_m2": 1400, "width_m": 0.08, "range_m": 6.0,
      "states": {
        "normal": { "color_srgb": "#9fd8ff", "scale": 0.6 },
        "red_alert": { "color_srgb": "#ff2020", "scale": 1.0 },
        "emergency": { "color_srgb": "#ff8a1c", "scale": 0.8 }
      }
    },
    "console_screen": { "kind": "area", "luminance_cd_m2": 160, "range_m": 3.5, "color": "station role", "emergency_scale": 0.6 },
    "viewscreen": { "kind": "area", "luminance_cd_m2": 45, "range_m": 14.0, "color_srgb": "#6f8fd0" },
    "reactor_glow": { "kind": "area", "luminance_cd_m2": 420, "range_m": 9.0, "color_srgb": "#48d6ff", "emergency_scale": 0.2 },
    "indicator_panel": { "kind": "area", "luminance_cd_m2": 90, "range_m": 2.5, "color_srgb": "#ffb347", "emergency_scale": 0.8 }
  }
}
```

(`"same as ceiling_panel"` and `"station role"` stand for the full objects and for the palette's
role colour; the real file spells them out.) The state colours here are the mockups'
`ShipKit.LIGHTING`; `bridge-stations` B3 may change red alert.

**`data/lighting/bake.json`** (settings; values are the mockup's final preset):

```json
{
  "schema": "starcrew.bake/1",
  "seed": 1,
  "reference_lux": 100,
  "ambient_lux": { "normal": [3.3, 4.7, 7.3], "red_alert": [2.7, 0.5, 0.6], "emergency": [0.5, 0.3, 0.2] },
  "shadow_samples": 12,
  "emitter_sample_area_m2": 0.06,
  "emitter_sample_spacing_m": 0.25,
  "ao_rays": 24,
  "ao_radius_m": 0.8,
  "exterior_ao_radius_m": 2.0,
  "bounces": 1,
  "cache_spacing_m": 0.5,
  "cache_gather_rays": 128,
  "cache_filter_passes": 1,
  "ray_bias_m": 0.004,
  "sample_inset_m": 0.01,
  "dither": true,
  "adaptive": { "base_m": 2.0, "min_m": 0.25, "max_error_levels": 5 },
  "max_added_triangles": { "default": 2000, "bridge": 7500 },
  "probes": { "spacing_m": 1.0, "wall_offset_m": 0.5, "invalid_backface_fraction": 0.5 },
  "lightmap_texel_m": 0.25,
  "power_loss": { "flicker_s": 0.6, "fade_s": 0.5 },
  "presets": { "preview": { "shadow_samples": 4, "cache_gather_rays": 32, "bounces": 0, "adaptive": { "max_error_levels": 8 } } }
}
```

**A fixture record**, as `deck-pipeline`'s deckgen places it or a detail file overrides it (the
layout holds no lights; `deck-pipeline` section 7):

```json
{ "id": "bridge_lamp_2", "compartment": "bridge", "type": "ceiling_panel", "center_m": [0.0, 6.38, 22.75], "emergency_bus": true }
{ "id": "bridge_strip_p", "compartment": "bridge", "type": "cove_strip", "from_m": [6.99, 6.4, 20.6], "to_m": [6.99, 6.4, 30.4], "facing_yaw_deg": -90, "emergency_bus": true }
```

**Placement.** The mockup lights the kit's lamps (2026-10-05): `deck-pipeline`'s rule
(`ShipKit.lampsFor`, `detailing.json` `lamps`) puts a panel in every frame bay, one per 8 m^2 of
floor, a high-bay lamp in a room taller than 3.6 m, and every third lamp on the emergency bus;
cove strips run along every cove the kit builds. On the v2 plan that is 21 lamps on the bridge (7
on the emergency bus) and 56 in engineering (20; the page applies the rule to the level under the
mezzanine too), against 6 and 16 when the page placed ceiling panels on its own 4.5 m grid (about
1.5 times their 2.9 m height above the v1 bridge floor, which lighting practice gives for an even
floor between pools; to verify) and high-bay lamps on a 6 m grid. The kit's density is about 2.5
times the grid's, so each lamp is that much dimmer: a ceiling panel 850 cd (was 2,600), a high-bay
lamp 2,200 cd (was 11,000). The baker lights whatever is placed, and `intensity_cd` is what makes
either density right. Recommendation taken (ask only with screenshots): offer `deck-pipeline` the
spacing rule "no wider than 1.5 times the mounting height" for downlights.

### 12. Quality controls and debug views

| Control | Trades | Final | Preview |
| --- | --- | --- | --- |
| `shadow_samples` | Soft-shadow noise against time | 12 (4 first, 12 in penumbra) | 4 |
| `emitter_sample_*` | Strip and screen light noise near the emitter | 0.06 m^2, 0.25 m | same |
| `cache_gather_rays`, `cache_filter_passes` | Bounce splotches against time and softness | 128, 1 | 32, 1 |
| `bounces` | Fill in shadow against time | 1 | 0 |
| `ao_rays`, `ao_radius_m` | Corner darkening noise and reach | 24, 0.8 m | 12, 0.8 m |
| `adaptive.*`, `max_added_triangles` | Shadow and pool sharpness against triangles | 2 m, 0.25 m, 5 levels | 8 levels |
| `dither` | Banding against a faint grain | on | on |

**Debug views** (dev builds of the client, and the mockup's toggles): lighting only (albedo white);
occlusion only (the alpha); one state solo; vertex density (wireframe, `lighting-bridge-vertex-density.png`);
residual heat map (the error left per cell); lightmap texel checker (`lighting-bridge-lightmap-texels.png`);
probe cubes drawn as small six-coloured cubes; fixture gizmos with their range; a leak finder that
marks vertices whose gather rays hit back faces more than 30 % of the time; and "isolate", a bake
with one fixture on.

### 13. What the mockup measured

`docs/mockups/lighting.html`, baking in the page with `lightbake.js` at the final settings, one
bounce, three states, this cloud machine, one browser thread:

| | Bridge | Engineering |
| --- | ---: | ---: |
| Floor, lamps, emissive surfaces | 154 m^2, 6, 10 | 252 m^2, 16, 44 |
| Irradiance cache (0.5 m): points, triangles, time | 3,345, 4,504, 1.56 s | 9,708, 15,124, 4.71 s |
| Cache time with 0 / 2 bounces | 0.03 s / 2.11 s | 0.05 s / 7.01 s |
| Vertex, deck mesh: triangles, vertices, light bytes, time | 412, 824, 9.9 KB, 0.16 s | 354, 708, 8.5 KB, 0.13 s |
| Vertex, adaptive: triangles (base), vertices, light bytes, residual, time | 3,160 (688), 2,479, 29.7 KB, 4.9 levels, 0.64 s | 5,193 (1,306), 3,570, 42.8 KB, 20.2 levels (cap), 1.30 s |
| Vertex, adaptive, no cap | same (converged) | 12,776, 7,738, 92.9 KB, 5.0 levels, 2.61 s |
| Lightmap 0.25 m: atlas, texels, bytes, time | 256 x 84, 9,150, 252 KB, 1.77 s | 256 x 187, 31,236, 561 KB, 6.69 s |
| Ray throughput | 0.4 to 0.65 M rays/s | 0.45 to 0.65 M rays/s |
| One ambient-cube probe | 3.6 ms | 4.4 ms |
| Determinism | Same digest in separate sessions; seed 2 differs | same |

The table is the v1 boxes, with the page's own 4.5 m lamp grid. **On the v2 plan** (2026-10-05)
the page bakes the kit's shell, detail and lamps, and its panel prints, for the bridge: 173.9 m^2,
21 lamps (7 on the emergency bus), 16 emissive surfaces; adaptive 6,070 triangles (base 1,698),
5,404 vertices, 63.3 KB of light, residual 5.0 levels. For engineering: 253.8 m^2, 56 lamps (20),
55 emissive surfaces; adaptive 7,514 triangles (base 3,718), 7,993 vertices, 93.7 KB of light,
residual 36.8 levels, cap reached. The other rows and every bake time are not rerun (the cloud
machine was shared when the panel was read, so its times are not a result).

### 14. The Pi 5 budget this change spends

Against `engine-stack`'s table; estimates from the two measured rooms.

| Budget | Spends | Notes |
| --- | --- | --- |
| Visible triangles per frame (200,000) | About 10,000 to 25,000 more in a busy interior view | Subdivision, capped per compartment; counted by `deckc` after the bake, inside `deck-pipeline`'s proposed 80,000 interior pass |
| Compartment geometry (bridge 30,000; others about 8,000) | Bridge about +4,400 over the kit's 3,894 on the v2 plan (the page's adaptive bake, 1,698 to 6,070; on the v1 boxes +2,500 over 5,630), far under 30,000; others up to their cap | The cap rule of section 3 keeps each under its ceiling |
| Draw calls (300) | None | The colours ride in the vertex; runtime lights are uniforms |
| Texture memory (96 MB) | None on the recommended path (about 3.7 MB if lightmaps were adopted everywhere) | |
| Vertex and index buffers (64 MB) | About 1 MB more for the Tern | 28-byte vertex, shared inside a face |
| Client memory (384 MB) | About 0.5 MB of probes, 18 bytes per portal of spill records | |
| Per vertex | The blend (9 multiply-adds, `deck-pipeline`), plus two `pow` and about 15 operations per active runtime light | Vertex-rate work, a small share of an A76-class GPU's vertex rate (to measure) |
| Per pixel | Nothing | A lightmap would add one fetch |
| Server | Nothing | Lighting is client-side; alert level and bus voltage are already simulation state |

### 15. Baking the Tern in the mockups (2026-10-07)

The owner, 2026-10-07: "begin setting up light baking for our 3 lighting conditions, normal, red
alert, and emergency", then, after the floor and texture work, "commence light baking". Until now
only `lighting.html` used the baker, on two rooms; the deck plan, the one page that shows the whole
ship and that the owner walks, lit every room with `ShipKit.bakeDirect` (direct light with no
shadows, no occlusion and no bounce). This section is the first step of the bake pipeline that can
be taken before the engine exists. All of it is documentation tooling (CLAUDE.md 4): it decides
nothing `sc-tools bake` (section 10) will not decide again, and it reads the same data that baker
will read.

**The questions this needed** (G1 to G4) have shots, so they are in the owner survey, with them
(its Lighting section, unanswered on 2026-10-07). The owner said to start, so the work proceeds on the recommendations, recorded as
"recommendation taken (ask only with screenshots), pending the survey": G1 b (vertex colours, with
adaptive subdivision in the engine), G2 a (keep the cap), G3 pools, G4 one bounce. An answer that
differs changes `bake.json` or the fixture types, not this work.

**1. The data, one source** (section 11, task 2.1). `data/lighting/fixtures.json` holds the fixture
types, `data/lighting/bake.json` the bake settings, each validated by `tools/lighting_check.py`
(unknown keys, missing states, non-finite or negative numbers, a fixture type a record names but
the file lacks, each with its path and field). Two changes from section 11's sketch, so nothing has
two sources:
- **State colours stay in `ShipKit.LIGHTING`**, the state palette every mockup already shares. A
  fixture type names which of its colours it takes (`"light": "lamp"`, `"strip"`, or a fixed
  `"color_srgb"`), and keeps its own scale per state and its `emergency_scale`.
- **`bake.json` gains `ambient_scale_lux`** (260: the palette's ambient colour times its weight
  times this is the fill in lux) and `mockup_cell_m` (point 2).

`lighting.html` and the deck plan read both files (inlined by `tools/mockups/inline.py`, which now
also takes `data:lighting/<name>`), in place of the constants `lighting.html` carried.

**2. The deck plan bakes every compartment** with `lightbake.js`, at `bake.json`'s final settings,
three states from one set of rays:
- **Each compartment alone, doors closed** (section 9): its shell and detail, its fixtures, its
  props and the craft in it, door leaves as occluders in its door openings.
- **Its lights:** the kit's lamps (ceiling panels and high-bay lamps, the emergency bus by the kit's
  rule), the cove strips along every cove, the lower floor's lamps under engineering's mezzanine,
  and each station's screens as a small steady light in its role colour. Lenses, status strips and
  screens keep the colours they show today; they are drawn, not lit.
- **Albedo** for the bounce is each texture layer's mean colour times a prop's tint.
- **Storage, a stand-in:** colours on the deck plan's own vertices, with the shell's floors, walls
  and ceilings first split to cells no wider than `mockup_cell_m` (1.0 m) so a lamp's pool reads on
  a floor. Section 3's adaptive subdivision (error-driven, capped per compartment) stays the
  engine's: it re-tessellates faces, and carrying every texture coordinate and layer through a
  re-tessellation is `deckc`'s job (`deck-pipeline` task 3.2). The uniform split costs more
  triangles than the adaptive one for the same look; every compartment's added triangles are
  reported against its ceiling.
- **Bounce** at a vertex is read from the irradiance cache (the nearest cache points on the same
  plane, by distance), as section 1 has it.
- **In the page:** the room the viewer stands in or has selected is baked first, then the rest in
  the background, yielding to the frame; until its bake arrives a room shows the quick light, and
  a badge says how many rooms are baked. Each room's bake has a digest (`LightBake.digest`), and
  the same inputs give the same digest in any session.

**3. The ship bake** (`tools/mockups/bake_ship.mjs`, a measurement instrument): it opens the deck
plan headless, waits for every compartment's bake, and writes
`docs/benchmarks/<date>-tern-bake/report.json` and `report.md`: per compartment its lit floor
area, lamps (on the emergency bus), emitters, triangles before and after the split against its
ceiling, vertices baked, rays, time and digest. Times are this cloud machine's CPU in one browser
thread, never a Pi 5 or engine number (CLAUDE.md 12). It also shoots named rooms in the three
states for `docs/screenshots/`.

**4. What the ship bake measured (2026-10-07)**, `docs/benchmarks/2026-10-07-tern-bake/report.md`,
in this cloud container's headless Chromium (SwiftShader, one thread: never a Pi 5 or engine
number):

| Quantity | Value |
| --- | ---: |
| Compartments baked | 37 of 37, each with its own digest |
| Floor lit | 2,573 m^2 |
| Lamps | 368, 126 of them on the emergency bus |
| Cove strips | 204 |
| Triangles in the rooms | 160,207, of which the 1 m split added 70,851 |
| Vertices baked | 177,536, nine values each (three states) |
| Rays | 86.6 million |
| Time | 273 s of baking, 278 s from opening the page |
| Slowest rooms | Engineering 84.8 s (199,453 cache points), the hangar 26.2 s, the bridge 14.8 s |

The irradiance cache is about 80% of the time in every room (engineering 65.3 of 84.8 s), so a
faster cache gather is where the engine baker's time goes first, not the vertex pass.

**Two rooms go past their ceiling, both because of the split:**

| Room | Before the split | With the split | Ceiling |
| --- | ---: | ---: | ---: |
| Engineering | 27,762 | 37,207 | 30,000 |
| Hangar | 5,188 | 12,895 | 8,000 |

Every other room stays under its ceiling with the split. The uniform 1 m split is the stand-in that
section 3's adaptive subdivision replaces. The engine's cap (`max_added_triangles`, 2,000 a room and
7,500 on the bridge) allows each of them 2,000 added triangles, inside both ceilings (engineering
has 2,238 to spare, the hangar 2,812), where the split adds 9,445 and 7,707. So the big open rooms are the test of the
adaptive pass: task 3.4's `the_bridge_converges_inside_its_cap` gains engineering and the hangar
beside the bridge.

**What the shots show** (`docs/screenshots/mockups/bake/`, one strip a view: the quick light,
then the bake in the three states):
- **The bridge** reads in every state. Lamp pools on the floor, the cove line on the ceiling,
  consoles lit by their own screens, shadows under the rails and the platforms. Red alert and
  emergency power keep every station readable.
- **Corridor B** keeps its lamp rhythm down the corridor, and the door strips light the frames.
- **Engineering's mezzanine** reads in all three states.
- **Engineering's lower floor is too dark on red alert and on emergency power.** Its lamps hang
  under the mezzanine, and on emergency power only every third of them stays lit
  (`emergency_every`), so the reactor's base and the pumps go nearly black. The quick light hid
  this behind its flat fill. The fix is data, not baker code: the lower floor's lamps take the
  emergency bus as a stairwell does (every lamp a crew member must find a valve by), and the
  reactor's glow lights its own base. It is task 1.11, and the bake shows when it is right.

**5. The bake is cached (2026-10-07).** The owner, on the deck plan baking as it opened: "I see
light is baking in real time, can we cache those results? or do you prefer to do it in real time?"
Cached: in the engine the light is never baked at run time (`deckc` bakes it offline into the
deck), so baking in the page was only ever the mockup standing in. `tools/mockups/bake_ship.mjs
--write-cache` bakes every room and writes `docs/mockups/cache/deck-plan-bake.bin`: gzip of a
header, an index (each room's key, its place in the data, its stats, and the sha256 of the
`lightbake.js` that baked it) and one byte a value, the display multiplier from 0 to 2 as the
engine's deck vertex colours store it (section 5). `tools/mockups/inline.py` inlines it into the
page (1.31 MB; the page is 13.2 MB, under the 16 MB an artifact allows).

- **A room's key** is a digest of its vertices, normals and every bake input (surfaces and their
  albedos, lights, strips, ambient, settings). A room whose key matches takes the cached light at
  once; any other room bakes live as before. A stale cache never shows wrong light: it shows the
  quick light, then a fresh bake.
- **`inline.py --check` fails** when the cache was baked by another `lightbake.js`, naming the
  command that rewrites it.
- **Measured:** the deck plan opens lit, all 37 rooms from the cache, 6.6 s after the page starts
  loading in this cloud container, against 278 s baking live. `?bake=fresh` ignores the cache;
  `bake_ship.mjs` always bakes fresh, since it measures the baker.
- **When to rewrite it:** after any change to the rooms, their props, the fixtures or the bake
  settings, rerun `bake_ship.mjs --write-cache` in the same commit, so the page opens lit.

**Not in this step:** the engine baker, adaptive subdivision in a page, probes for moving things,
runtime lights over the bake, and portal spill light (sections 6, 7 and 9); power loss stays the
lighting page's.

### 16. Probes in the engine, first step (2026-10-10)

The owner, 2026-10-10, after the MakeHuman crew were deleted: "for now use blocky people. implement
light probes so the people get lit properly". The bot figures (`crew-npcs` 7) carry a light baked into
their own vertices from one fixed direction, so a bot looks the same in a dark corridor, under a bridge
lamp and at red alert. This step lights them from section 6's ambient cubes, made by the mockup baker
(section 15) and carried into the engine the way the walls' light is.

- **The grid.** Per compartment, over the bounding box of its brushes: points `probes.spacing_m` (1.0 m)
  apart on all three axes, the first `probes.wall_offset_m` (0.5 m) in from the box's low corner, so a
  2.5 m deck takes two layers, at 0.5 m and 1.5 m above its floor. A point inside no brush of the
  compartment is outside its air and is stored invalid.
- **The cube.** `lightbake.js`'s `probeCube` on the room's own bake scene, after the scene is prepared
  as for its walls (`prepare`: the irradiance cache and its bounce): the same irradiance function, all
  terms, for the three states. Each value is encoded as the walls' vertex colours are (the display
  multiplier, 0 to 2, in a byte), 54 bytes a probe.
- **Invalid probes.** From each point, `gather_rays` (48) rays spread evenly over the sphere (a seeded
  Fibonacci spiral) are cast with the bake's BVH; a probe whose rays hit back faces more than
  `probes.invalid_backface_fraction` (0.5) of the time is inside something and is stored invalid. These
  rays are their own, beside the cube's gather rays, which `irradiance` does not report.
- **The cache.** Probes cost each room its scene preparation (211 s for the ship in the last bake report)
  plus about 4 ms a probe, too long for every push's deploy. So they are cached like the light:
  `tools/deck/cache/deck-plan-probes.bin` (gzip: `"SCPR"`, a version, an index as JSON, then the
  bytes), one entry a room, keyed by the room's bake key (section 15) and the probe settings. A room whose
  key matches takes its cached probes; any other is baked during the export, and
  `export_deck.mjs --write-probe-cache` writes the file back.
- **The deck file** (`deck-pipeline`'s `PROB`, version 5): each compartment names its grid (origin in
  ship coordinates, spacing, the counts on each axis, and where its probes start), and one blob after the
  walk's triangles holds every probe's 54 bytes then a byte that is 1 when it is valid.
- **Sampling** is a rule in `sc-core` (`sc_core::probes`), tested headless. A body at a point takes the
  grid whose box (grown half a spacing) holds the point, the eight probes around it, trilinear weights,
  invalid probes skipped and the weights renormalized; then the three states blended by the same weights
  the walls use. With no valid probe around it, it takes the fixed light the figures had before (the
  fallback, below).
- **The figures.** Each figure's vertex colour is now its albedo, unlit. A new program, `deck_probe`,
  turns each vertex normal into the ship's frame (the bot's yaw), weights the six colours by the squared
  normal (`n.x^2 * cube[+-X] + n.y^2 * cube[+-Y] + n.z^2 * cube[+-Z]`, section 6) and multiplies the
  albedo. A bot is sampled at its chest, 1.0 m above its feet. The ship map (`M`) draws the figures with
  the fallback cube: a diagram, not a room.
- **The fallback cube** is the figures' old light, written as a cube: a key from above and the front,
  `0.45 + 0.65 max(n . l, 0)` along each axis, `l = (0.3, 0.8, 0.52)`, times two.

**Pi 5 cost:** about 55 bytes a probe of client memory (the count and the bytes are printed by `deckc`
and at load); per bot per frame, eight lookups and a blend in the core (microseconds, not measured on a
Pi); one uniform block of six colours per figure's draw; no new draw calls, no texture. Not measured on a
Pi (CLAUDE.md 2).

**Not in this step:** the dimmer and power loss (the client has no dimmer yet), runtime lights, the
player's own body (first person, unseen), craft and door leaves, and the engine baker (section 10): the
probes come from the mockup baker until `sc-tools bake` exists.

## Risks / Trade-offs

- **Vertex lighting needs triangles where light is busy.** Big rooms with many fixtures
  (engineering, the hangar) can hit their cap and keep a residual. Mitigation: the report prints
  residual and cap; G2 decides engineering; the lightmap output exists for the hybrid.
- **Shadow edges are as sharp as the smallest cell (0.25 m).** A console's shadow is soft, which
  suits the look; a thin rail's shadow can vanish. Mitigation: `min_m` per compartment in
  `bake.json` if a hero room needs it.
- **Bounce is approximate** (one gather, filtered on a 0.5 m cache): colour bleed is right in tone,
  not exact. Nobody can tell on palette colours.
- **The mockup baker is not the engine baker.** It mirrors the method so the pictures are honest;
  the engine baker is written fresh in Rust and judged by the same shots and its own digest test.
- **Bake times above are a JavaScript baker on a cloud CPU.** The Rust estimate is to verify with
  the first build.
- **Dimmer on encoded values** is a display-space approximation of a lamp's response to voltage.
  It matches what the console shows, which is the point (CLAUDE.md 7: a readout shows the simulated
  value).

## Open questions

Questions go to the owner only with something to look at (CLAUDE.md 13); the shots are in
`docs/screenshots/mockups/`. The rest take the recommendation, recorded as "recommendation taken
(ask only with screenshots)".

| Id | Question, and the fact it turns on | Options | Recommendation | Shot |
| --- | --- | --- | --- | --- |
| G1 | How is baked light stored? Adaptive vertex light and a 0.25 m lightmap look the same at playing distance; vertex light costs triangles (bridge +2,500 on the v1 boxes, about +4,400 on the v2 plan), a lightmap costs a texture fetch per pixel, a second UV and an atlas (bridge 252 KB) | a. vertex colours on the deck mesh; b. vertex colours with adaptive subdivision; c. lightmaps everywhere; d. hybrid: b, with lightmaps in named large rooms | **b**, as CLAUDE.md 9 already says; c and d stay measured fallbacks | `lighting-split-vertex-vs-lightmap.png`, `lighting-split-vertex-vs-lightmap-room.png`, `lighting-split-deck-mesh-vs-adaptive.png`, `lighting-bridge-vertex.png`, `lighting-bridge-lightmap.png` |
| G2 | Engineering converges at 12,776 triangles of light alone, over its 8,000 ceiling; capped at 2,000 added it is 5,193 with a residual hard to see | a. keep the cap; b. raise engineering's ceiling (`engine-stack` table) to about 16,000; c. lightmap engineering (hybrid) | **a** | `lighting-engineering-cap-vs-no-cap.png` |
| G3 | Light pools: narrow downlights (beam exponent 8) draw pools with darker floor between, as in the shots; a wide beam lights the floor evenly and flatter | Pools / even wash | **Pools**: they give a low-poly room depth for free | `lighting-bridge-vertex-subdivided.png`, `lighting-bridge-eye-level.png`, `lighting-engineering-eye-level.png` |
| G4 | Bounces: none is harsher, two is not visibly different from one and costs 25 % to 40 % more bake time | 0 / 1 / 2 | **1** | `lighting-bridge-vertex-no-bounce.png`, `lighting-bridge-vertex-subdivided.png`, `lighting-bridge-vertex-two-bounces.png` |
| G5 | Lamp spacing rule for `deck-pipeline`'s kit: one per 8 m^2 (21 on the v2 bridge, which the lighting page now bakes; 19 on the v1 boxes) or no wider than 1.5 times the mounting height (6 on the v1 bridge) | Kit rule / spacing rule | Spacing rule, offered to `deck-pipeline`. Recommendation taken (ask only with screenshots) | none of the 19-lamp layout |
| G6 | Open doors: a runtime spill light from the neighbour's bake, or nothing | Spill light / nothing | Spill light. Recommendation taken (ask only with screenshots) | none |
| G7 | Sunlight through windows | None baked / runtime window light | None. Recommendation taken (ask only with screenshots) | none |
| G8 | Probe grid spacing | 0.5 m / 1 m / 2 m | 1 m (0.48 MB for the Tern). Recommendation taken (ask only with screenshots) | none |
| G9 | One failed lamp in a compartment | Per-compartment dimming only / a fourth colour set | Per compartment only. Recommendation taken (ask only with screenshots) | none |
