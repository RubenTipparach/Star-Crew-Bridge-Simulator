---
name: light-baking
description: Bake, compare and judge Star Crew's static lighting the repository's way - fixtures and emissive surfaces as data with photometric units, soft shadows, occlusion and one bounce, all three lighting states (normal, red alert, emergency) from one bake, vertex colours with adaptive subdivision (lightmaps as the measured fallback), ambient-cube probes for moving things and a few runtime lights over the bake. Today with the mockup baker (docs/mockups/lib/lightbake.js and lighting.html); later with sc-tools bake (planned). Use whenever placing or tuning a lamp, strip or screen light, baking or rebaking a room, comparing vertex lighting with lightmaps, setting up lighting states or power loss, or diagnosing a bake ("bake lighting", "light map", "lightmap", "vertex lighting", "place a lamp", "lighting states", "red alert lighting", "why is this room dark", "light leaking").
metadata:
  author: Star Crew (Claude Code)
  version: "1.0"
---

# Light baking

The owner, 2026-10-04: "Some sort of tool or skill for light mapping/static light baking would be
nice too", and "this game doesn't need high end graphics". CLAUDE.md section 9 has the rule (light
is baked offline into vertex colours, one set per lighting state, from fixtures that are data);
`openspec/changes/light-baking` is the design and has every number quoted here; `deck-pipeline`
owns where the result lives. This skill is how to work with it.

## What is baked, and why

| Term | From | Why it matters on this look |
| --- | --- | --- |
| Direct light with soft shadows | Ceiling panels, high-bay lamps (point lights with a radius, a beam) | Light pools on the floor and shadows under consoles and seats are what make a flat-shaded room read as lit |
| Emissive surfaces as area lights | Screens, cove strips, the viewscreen, the reactor's glow, indicator panels | Coloured spill on desks and walls; screens still light a room on emergency power |
| Ambient occlusion on a small fill | Every surface | Dark corners and contact shadows; nothing goes pure black |
| One bounce | Every surface, from a filtered 0.5 m irradiance cache | Light under desks and colour bleed; a second bounce adds nothing visible |

All three states come from one set of rays (light is linear in its sources), so red alert and
emergency power cost nothing extra to bake. Stored as three `RGBA8` vertex colours: gamma 2.2,
2x overbright (byte 128 is the palette colour at 100 lx), ambient occlusion in alpha.

## The rules

- **Fixtures are data.** A fixture type (`data/lighting/fixtures.json`, proposed) has units in its
  keys: `intensity_cd`, `luminance_cd_m2`, `radius_m`, `range_m`, `beam_exponent`, and a colour and
  scale per state. A fixture record (deckgen's placement or a deck detail file, `deck-pipeline`
  section 7) names its type, position and `emergency_bus`. **The layout holds no lights**; never add
  them to `layout.json`.
- **Every light has a visible fixture** (CLAUDE.md 11, `deck-pipeline`'s check) and a colour for each
  of the three states. A lamp off the emergency bus is dark in the emergency set.
- **Each compartment is baked alone, doors closed.** Light through an open door is a runtime spill
  light, not a bake.
- **Vertex colours with adaptive subdivision** is the recommended storage (question G1): a 2 m base
  grid split to 0.25 m where the light changes by more than 5 display levels, within a cap per
  compartment. Lightmaps are the measured fallback, not the plan, until the owner says otherwise.
- **No shadows at run time.** Up to four runtime lights per compartment, per vertex, darkened by the
  baked occlusion: muzzle flashes, sparks, beacons, fire, door spill.
- **Moving things use probes**: ambient cubes on a 1 m grid, from the same bake.
- **Bakes are deterministic**: seeded quasi-random samples keyed by stable ids, never `Math.random`
  or thread order; a digest proves it.
- **Every bake states its Pi 5 cost** (CLAUDE.md 2): triangles added, bytes, and the residual if a
  cap was reached.
- **Numbers say what they are**: a bake time from the mockup is this machine's CPU in one browser
  thread, not a Pi 5 and not the engine baker.

## Bake and compare today (the mockup baker)

`docs/mockups/lib/lightbake.js` is a small CPU baker in the design's method; `docs/mockups/lighting.html`
uses it on the Tern's bridge and engineering. It is documentation tooling, not engine code.

1. **Open** `docs/mockups/lighting.html` (from disk or the published artifact). It bakes on load
   (a few seconds per room) and caches every bake.
2. **Compare** with the controls: Compartment; Camera (Cutaway drops the ceiling and near walls by
   back-face culling, so the floor's light is readable; Close; Eye level); Mode (Unlit, Dynamic
   runtime lights, Baked vertex on the deck mesh, Baked vertex adaptive, Adaptive with no cap, Baked
   lightmap, Split with any two); State (Normal, Red alert, Emergency power, Power loss); Bounces;
   Texel size; Toggles (AO in the bake on or off, Debug: occlusion only, Debug: vertex wireframe
   or lightmap texel checker, the probe-lit crew figure). The panel on the right gives triangles, vertices, bytes, atlas size, bake time,
   rays, residual and digest; the meter gives the Pi 5 budget.
3. **Place or tune a lamp** in the page: fixture types and their photometry are `FIXTURE_TYPES`
   (a ceiling panel 850 cd, a high-bay lamp 2,200 cd); placement is the kit's lamp rule
   (`ShipKit.lampsFor`, from `data/ships/tern/detailing.json` `lamps`: a panel in every frame bay,
   one per 8 m^2 of floor, high-bay lamps in rooms taller than 3.6 m, every third lamp on the
   emergency bus), turned into lights by `addLamps()`, with cove strips along every cove the kit
   builds (`coveStrip()`): 21 lamps on the bridge and 56 in engineering (the level under its
   mezzanine included) on layout v2. Quality and caps are `BAKE`. The shell and detail are the
   kit's `buildCompartment()`; props and emissive faces are added in `compartmentScene()`,
   `bridgeProps()` and `engineeringProps()`.
4. **Change the baker** only in `docs/mockups/lib/lightbake.js`, then
   `python3 tools/mockups/inline.py docs/mockups/lighting.html`. Never edit between the INLINE
   markers.
5. **Shoot** `node tools/mockups/shoot.mjs docs/mockups/lighting.html`, or one shot with
   `--only bridge-vertex-subdivided --out <scratch dir>` while iterating (a full run is about 4
   minutes). If a run fails with `ERR_TOO_MANY_RETRIES` and no `MOCKUP_READY`, the CDN fetch
   failed; run it again.
6. **Look at every PNG** before calling a bake done (section below). Shots worth knowing:
   `lighting-split-vertex-vs-lightmap.png`, `lighting-split-deck-mesh-vs-adaptive.png`,
   `lighting-engineering-cap-vs-no-cap.png`, `lighting-bridge-red-alert-baked.png`,
   `lighting-engineering-emergency-baked.png`, `lighting-bridge-vertex-density.png`.

**The library**, for a new page or a measurement: `LightBake.scene({ patches, occluders, lights,
emitters, ambient_lux, settings })`, `await LightBake.prepare(S)` (the irradiance cache), then
`bakeVertices(S, LightBake.tessellate(patches, Infinity))`, `bakeAdaptive(S, { base_m, min_m,
max_error_levels, max_added_triangles })` or `bakeLightmap(S, { texel_m })`; `probeCube(S, point,
key)` for an ambient cube; `digest(arrays)` to prove two bakes match. Each returns `stats` (time,
rays, triangles, residual). To measure without the page's UI, copy the page to the scratchpad,
expose `compartmentData` on `window`, and drive it with a short Playwright script; never commit
that copy.

## Once `sc-tools bake` exists (planned, not built)

The engine baker is designed in `light-baking` design.md section 10; none of this runs yet.

- `deckc` calls it per compartment as compile step 4; alone it runs as
  `sc-tools bake --ship tern [--compartment bridge] [--preset preview|final] [--threads N] [--report <file>]`.
- It reads `data/lighting/fixtures.json`, `data/lighting/bake.json` and the fixture records, and
  writes colours per vertex, probes, portal spill records and a report per compartment
  (triangles before and after, residual, cap reached, rays, time, digest).
- Use `--preset preview` while placing lamps, `final` before committing. A pinned-digest test fails
  when a bake changes; update the digests in the same commit as the change that moved them, and
  say why.
- The client's debug views (lighting only, occlusion only, a state alone, vertex density, residual
  heat map, probe cubes, fixture ranges, leak view) are the engine's versions of the mockup's
  toggles.

## Judge a bake

| Artefact | Looks like | Usual cause | Fix |
| --- | --- | --- | --- |
| Leak | Light on a floor under a wall or prop, a bright line along a wall's foot, a lit patch inside a closed cupboard | A sample point inside solid or on a seam; a missing occluder; a neighbour baked into the scene | Each compartment alone, doors closed; inset and ray bias; back-face hits read black; check the occluder list |
| Banding | Steps in a smooth gradient on a big wall | 8-bit storage of a slow ramp | Keep the dither on; never turn `reference_lux` so low that rooms sit near byte 0 |
| Acne | Speckled self-shadow on lit faces | Ray bias too small | `ray_bias_m` 4 mm; never 0 |
| Dark corners | Corners black, rooms gloomy at the edges | Ambient occlusion too strong or too long, or no bounce | `ao_radius_m` 0.8 m, one bounce; raise the fill per state, not the radius |
| Hot spot | A blown-out blob where a lamp meets a surface | A lamp nearly touching a face | Move the lamp; the 0.04 m^2 distance floor only limits it |
| Splotches | Cloudy blobs about 1 m across on walls lit mostly by bounce | Too few gather rays seeing bright pools | Bounce from the cache: `cache_gather_rays` 128 and one filter pass |
| Strip-end mottling | Grain near the end of a long light strip | Too few samples along the emitter | `emitter_sample_spacing_m` 0.25 m |
| Pinholes | One-pixel cracks along a seam between faces | T-junctions where a face split at a door meets an unsplit one | `deckc`'s T-junction pass after subdivision (the mockup does not weld; seen on engineering's aft wall) |
| Zig-zags | Saw-tooth steps along a shadow edge, triangles visible in the light | Vertex light on cells too large for the edge | Lower `max_error_levels` or `min_m` there, or accept it; it is the look at a distance |
| Flat room | No pools, no shadows, every face one colour | Vertex light on the deck mesh with no subdivision | Adaptive subdivision (`lighting-split-deck-mesh-vs-adaptive.png`) |
| Capped room | The report says "cap reached" and a residual over 5 levels | The room needs more triangles than its cap | Look at it first (`lighting-engineering-cap-vs-no-cap.png`): often invisible; else raise the cap within the compartment's ceiling, or ask (G2) |

**"Why is this room dark?"** In order: is the compartment's state or dimmer what you think (red
alert, emergency bus, a dead bus is dimmer 0)? Is each lamp on the emergency bus in the emergency
state? Is the lamp inside the room's air box and above its lens, not inside the ceiling? Does the
beam point down (`beam_exponent` too high makes a spotlight)? Is `range_m` long enough to reach the
floor? Is the palette colour itself dark (the bridge floor is `0x3a404a`)? Then check the bake's
residual and the AO-only view.

## Do not

- Do not put lights in `layout.json`; the layout is the plan, fixtures belong to `deck-pipeline`'s
  placement and detail files.
- Do not bake neighbouring compartments into one scene or bake doors open.
- Do not add shadow maps, per-pixel fixture lighting or a post-processing pass for interiors; they
  are out of the Pi 5 budget and out of the style.
- Do not use `Math.random`, time or thread order in a bake; seed from stable ids.
- Do not hand-edit between INLINE markers; edit `lib/lightbake.js` and run the inliner.
- Do not quote a mockup bake time as a Pi 5 or engine number.
- Do not adopt lightmaps, change the vertex format or change CLAUDE.md section 9 without the owner's
  answer to G1.
- No em or en dashes in pages, data or docs (CLAUDE.md 5).
