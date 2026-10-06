# Proposal: static light baking

## Why

The owner, 2026-10-04: "Some sort of tool or skill for light mapping/static light baking would be
nice too". On the stack: "That's fine..this game doesn't need high end graphics. Your stack
sounds like a solid plan". On the hardware: "I'm sorry we're running on a pi5 1gb-4gb, 4 GB can
be used as main server too". And on the look: "I'm all for low poly aesthetics."

CLAUDE.md section 9 already decides the shape: flat-shaded, vertex-coloured low poly, "lighting
is baked into vertex colours, with one set per lighting state (normal, red alert, emergency
power) blended by a per-compartment uniform", and "light is baked offline, from fixtures that are
data". `deck-pipeline` decides where the baked light lives (fixtures as entities, three `RGBA8`
colour sets per vertex, the per-compartment blend uniforms, the `.deck` file) and calls the baker
as step 4 of its compile. Nothing yet decides **how light is computed**: what casts shadows, how
screens and strips light a room, how bounce and corners are handled, how a vertex-lit deck gets
light pools and console shadows without a lightmap, how crew and craft that move are lit to match
the walls, what a few runtime lights add, and how a bake stays reproducible and fast. This change
designs that, measured with a working CPU baker in the mockup before any engine code exists
(CLAUDE.md section 4).

A Pi 5 can afford more than a Pi 3 could: lightmaps and a few runtime lights fit its budget. The
question is not what fits but what is the simplest thing that reads well on this look, and what
each step up costs and buys. This change answers it with measurements and screenshots.

## What Changes

Everything below is **proposed**. What the mockup measured is labelled **measured** (this cloud
machine's CPU, one browser thread, not a Pi 5 and not the engine baker); the rest is design or
estimate.

- **What is baked.** Ceiling lamps and other fixtures as lights with a radius (soft shadows from
  consoles, seats, door leaves and walls); emissive surfaces (console screens, light strips, the
  viewscreen, the reactor's glow bands, indicator panels) as area lights; ambient occlusion on a
  small fill term; one diffuse bounce, gathered once on a coarse irradiance cache and
  interpolated. Units are photometric (candela, candela per square metre, lux).
- **All three lighting states from one set of rays.** Light is linear in its sources, so every
  shadow ray is traced once and its result is weighted by each fixture's normal, red-alert and
  emergency colour. Three states cost the rays of one (measured).
- **Into what: vertex colours with adaptive subdivision (recommended, question G1).** Each brush
  face starts as a 2 m grid and splits down to 0.25 m only where light changes faster than the
  vertex colours can follow (5 of 255 display levels), within a per-compartment triangle cap. The
  bridge goes from 412 to 3,160 triangles and gains its light pools and console shadows
  (measured on the v1 boxes; on the v2 plan, with the kit's detail and 21 lamps, from 1,698 to
  6,070). A 0.25 m lightmap atlas gives the same picture at playing distance for 252 KB of
  texture and one texture fetch per pixel (measured on the bridge); it stays a measured fallback,
  not the plan.
- **What each step up costs and buys**, in a table: deck-mesh vertex light, adaptive vertex light,
  bounce, lightmaps (everywhere or hybrid), runtime lights, and what stays out (interior shadow
  maps, per-pixel lighting of every fixture).
- **Power loss and the three states.** The blend, dimmer and flicker of `deck-pipeline` drive a
  power-loss sequence: a seeded flicker, then the emergency set, with lamps off the emergency bus
  dark and screens dimmed.
- **Light probes** (ambient cubes, as in Half-Life 2) on a 1 m grid in every compartment, three
  states each, so crew avatars, craft in the hangar and carried props match the walls.
- **A few runtime lights over the bake:** up to four per compartment (the slot count
  `deck-pipeline` gives its uniform block), per vertex, unshadowed, darkened by the baked
  occlusion: muzzle flashes, sparks, alarm beacons, fire glow and the spill of light through an
  open door.
- **The exterior:** the sun stays a runtime light; the hull bakes ambient occlusion only.
- **No light leaks:** each compartment is baked alone with its doors closed, occlusion is
  two-sided, a ray that meets the back of a face reads black, and sample points sit inside their
  face. An open door is a runtime spill light coloured by the neighbour's bake.
- **The engine baker, `sc-tools bake` (planned, not built):** inputs, outputs, threads, seeded
  quasi-random sampling keyed by stable ids, a digest test, a bake report per compartment, and an
  estimate of the Tern's bake time.
- **Fixtures and bake settings as data**, units in the keys: `data/lighting/fixtures.json` (fixture
  types and their three state colours), `data/lighting/bake.json` (quality settings, caps, probe
  grid), with the fixture records `deck-pipeline`'s detail files carry.
- **Debug views** and a list of the artefacts a bake is judged for (leaks, banding, acne, dark
  corners, hot spots, splotches, pinholes).
- **The mockup and its baker:** `docs/mockups/lighting.html` with `docs/mockups/lib/lightbake.js`, a
  small deterministic CPU baker in the method above, comparing unlit, runtime lights, deck-mesh
  vertex light, adaptive vertex light (capped and uncapped), and lightmaps at three texel sizes,
  in all three states, on the bridge and in engineering, with a Pi 5 budget meter.
- **A skill**, `.claude/skills/light-baking`: how to place a lamp, bake, compare and judge a bake
  today with the mockup baker, and with `sc-tools bake` once it exists.

## Capabilities

### New Capabilities
- `light-baking`: how static light is computed for decks and the hull: fixtures and emissive
  surfaces, shadows, occlusion and bounce; the three lighting states from one bake; adaptive
  vertex subdivision; light probes; runtime lights over the bake; leak rules; determinism and the
  bake report.

### Modified Capabilities
None. `openspec/specs/` holds nothing yet.

## Impact

- **Data (proposed):** `data/lighting/fixtures.json`, `data/lighting/bake.json`; fixture records
  in `data/ships/<id>/detail/*.json` (the detail-file schema is `deck-pipeline`'s, task 1.2).
- **Tools (proposed):** `sc-tools bake` in the `sc-tools` crate (`engine-stack` section 3), called by
  `deckc` per compartment; a bake report and digests per compartment.
- **Client (proposed):** probe sampling for moving bodies, runtime lights added over the bake in
  the deck shader, debug views. The deck shader, its blend and dimmer are `deck-pipeline`'s.
- **Mockup tooling (built with this write-up):** `docs/mockups/lib/lightbake.js`,
  `docs/mockups/lighting.html`, screenshots in `docs/screenshots/mockups/lighting-*.png`.
- **Other changes:** `deck-pipeline` (it calls the baker and stores the result; this change
  proposes it a probe chunk and a portal spill record, and offers its lamp rule a photometric
  spacing), `engine-stack` (the budget table and the `sc-tools` crate; the probe could measure a
  lightmap fetch), `bridge-stations` (question B3 decides the red-alert look this bakes),
  `power-grid` (the lighting bus voltage behind the dimmer), `ship-frames` (the exterior sun).
- **Pi 5 budget (estimates from measured rooms):** no extra draw calls and no texture memory on
  the recommended path. Adaptive subdivision adds about 2,500 triangles to the bridge (measured on the v1 boxes;
  about 4,400 on the v2 plan)
  and about 6 triangles per square metre of lit surface elsewhere, capped per compartment; with the
  caps, about 30,000 to 50,000 triangles across the Tern and 10,000 to 25,000 in a frame's visible
  set, inside `deck-pipeline`'s proposed 80,000-triangle interior pass. About 1 MB more vertex
  buffer (of 64 MB), 0.5 MB of probes in client memory (of 384 MB), a few multiply-adds per vertex
  for the blend and the runtime lights, and nothing per pixel. The breakdown is in design.md
  section 14.
