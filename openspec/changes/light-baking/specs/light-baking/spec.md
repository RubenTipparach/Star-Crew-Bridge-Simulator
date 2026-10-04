# Light Baking

## Purpose

How static light is computed for a ship's decks and hull: fixtures and emissive surfaces,
shadows, occlusion and bounce; all three lighting states from one bake; adaptive vertex
subdivision; light probes for moving things; runtime lights over the bake; the rules that stop
light leaking; and a reproducible, reported bake.

## ADDED Requirements

### Requirement: Light comes from fixtures and emissive surfaces described as data
The baker SHALL take every light from a fixture record or an emissive face whose type is defined
in `data/lighting/fixtures.json`, with photometric units in the keys (`intensity_cd`,
`luminance_cd_m2`, `radius_m`, `range_m`) and a colour and scale for each of the three lighting
states. Bake settings SHALL come from `data/lighting/bake.json`. An unknown key, a missing state,
a negative or non-finite number, or a fixture whose type is not defined SHALL stop the bake with
the file, the path and the field. A present zero SHALL mean zero.

#### Scenario: A misspelt intensity
- **WHEN** a fixture type in `data/lighting/fixtures.json` has the key `intensity` instead of `intensity_cd`
- **THEN** the bake stops and names `data/lighting/fixtures.json`, the type and the key `intensity`

#### Scenario: A lamp with no emergency colour
- **WHEN** a fixture type gives colours for `normal` and `red_alert` but not `emergency`
- **THEN** the bake stops and names the type and the missing state

### Requirement: Fixtures cast soft shadows and emissive surfaces light the room
Direct light from a point fixture SHALL be the average over shadow rays to sample points on a disc
of the fixture's `radius_m` facing the receiver, weighted by the beam `cos^beam_exponent` about the
fixture's axis, the cosine at the receiver and a windowed inverse-square falloff that reaches zero
at `range_m`. An emissive face SHALL light the room as an area light sampled at one point per
`emitter_sample_area_m2` and at least one per `emitter_sample_spacing_m` of its longest edge, with
the cosine at both ends. Every face, prop, detail brush and closed door leaf of the compartment
SHALL occlude, from either side.

#### Scenario: The floor under a console desk
- **WHEN** the bridge is baked with a ceiling lamp above the helm console
- **THEN** the floor under the helm desk receives less direct light from that lamp than the open floor 1 m behind the seat, and the edge between them is spread over at least 0.1 m (a penumbra, not a hard line)

#### Scenario: A screen lights its desk
- **WHEN** the bridge is baked with every ceiling lamp and strip removed
- **THEN** each console's desk top still receives light, in the colour of that console's screen

### Requirement: Occlusion and one bounce are baked
The bake SHALL include an ambient fill per state multiplied by ambient occlusion (the fraction of
`ao_rays` cosine-weighted rays of length `ao_radius_m` that hit nothing), and `bounces` diffuse
bounces (0, 1 or 2; 1 by default). Bounce SHALL be gathered at a uniform irradiance cache of
`cache_spacing_m` on every face, filtered only within each face, and interpolated to the outputs.

#### Scenario: A corner is darker than the middle of the wall
- **WHEN** a compartment is baked with its lamps removed, leaving only the ambient fill
- **THEN** a vertex 0.05 m from the corner between the floor and a wall is darker than a vertex at the middle of the floor

#### Scenario: Bounce lights a shadow
- **WHEN** the bridge is baked once with `bounces` 0 and once with `bounces` 1
- **THEN** the floor under the helm desk is brighter with one bounce, and no vertex is darker with one bounce than with none

### Requirement: All three lighting states come from one set of rays
The baker SHALL trace each shadow, occlusion and bounce ray once and weight its result by every
light's colour in each of the three states (normal, red alert, emergency), writing three colour
sets. A light that is off in a state SHALL contribute nothing to that state's set. A lamp that is
not on the emergency bus SHALL be off in the emergency set.

#### Scenario: An emergency-bus lamp and a main-bus lamp
- **WHEN** the bridge is baked with `bridge_lamp_2` on the emergency bus and `bridge_lamp_1` not
- **THEN** the floor under `bridge_lamp_1` is darker in the emergency set than the floor under `bridge_lamp_2`, and in the normal set the two are equal within 2 levels

#### Scenario: Three states cost the rays of one
- **WHEN** a compartment is baked with three states and again with the red-alert and emergency colours set to the normal colours
- **THEN** both bakes trace the same number of rays

### Requirement: Vertex light is refined where it changes
For vertex output, each receiving face SHALL start as a grid of cells about `adaptive.base_m` on a
side and split cells into four, highest error first, while the difference between the light at a
cell's centre or edge midpoints and what its corners interpolate exceeds
`adaptive.max_error_levels` 8-bit display levels in any state, down to cells of `adaptive.min_m`,
until the compartment's `max_added_triangles` is spent. The result SHALL have no T-junction inside
a face, SHALL share vertices only inside a face, and SHALL report its triangles before and after,
the largest residual error, and whether the cap was reached.

#### Scenario: The bridge converges inside its cap
- **WHEN** the Tern's bridge is baked with the settings of design.md section 11
- **THEN** its report shows a residual of at most 5 levels, the cap not reached, and fewer triangles than a uniform 0.25 m grid of the same faces

#### Scenario: A cap is reached
- **WHEN** a compartment's subdivision would need more triangles than its `max_added_triangles`
- **THEN** the bake stops splitting at the cap, the mesh stays crack-free, and the report says the cap was reached and gives the residual

### Requirement: Baked light is stored in the deck's colour sets
Each output vertex SHALL carry three `RGBA8` colours, one per state, in `deck-pipeline`'s vertex.
RGB SHALL be the irradiance over `reference_lux`, gamma-encoded with exponent 1/2.2 and 2x
overbright with a soft shoulder, so that a byte value of about 128 shows the surface at its palette
colour. A seeded dither of at most half a level SHALL be applied. Alpha SHALL hold the ambient
occlusion from 0 to 255.

#### Scenario: Reference irradiance shows the palette colour
- **WHEN** a vertex receives exactly `reference_lux` of white light in the normal state
- **THEN** its normal-state RGB bytes are each 127 or 128

### Requirement: Light does not leak between or into solids
Each compartment SHALL be baked with only its own geometry, its doors closed. A gather or probe ray
that meets the back of a face SHALL contribute no light. Sample points SHALL lie at least
`sample_inset_m` inside their face's edges and rays SHALL start `ray_bias_m` off the surface.

#### Scenario: A lit neighbour does not light the next compartment
- **WHEN** engineering is baked with every lamp on and the aft passage beyond its door has none
- **THEN** the aft passage's bake is unchanged by engineering's lamps, because engineering is not in its scene

#### Scenario: Under a pedestal
- **WHEN** a console pedestal stands on the bridge floor
- **THEN** a floor sample point inside the pedestal's footprint receives no direct light and no bounce

### Requirement: Open doors spill the neighbour's light at run time
The baker SHALL write, per portal side and per state, the mean baked irradiance over the opening
as seen from the neighbouring compartment. The client SHALL place a runtime light just inside an
open door on each side, coloured by the neighbour's record times the neighbour's state weights and
dimmer and scaled by how far the door is open.

#### Scenario: Opening the bridge's aft door at red alert
- **WHEN** the bridge is at red alert and the command passage is normal, and the aft door opens fully
- **THEN** the command passage's floor by the door gains red light and the bridge's floor by the door gains the passage's normal light, and both fade out as the door closes

### Requirement: Moving things are lit by ambient-cube probes from the same bake
The baker SHALL compute ambient cubes (six irradiance values, one per axis direction, three states
each) on a grid of `probes.spacing_m` inside every compartment's air, with the same irradiance
function as the walls, and SHALL mark a probe invalid when more than
`probes.invalid_backface_fraction` of its gather rays hit back faces. The client SHALL light a
crew avatar, a craft or a moving part from the valid probes around it, blended by its
compartment's state weights and dimmer.

#### Scenario: A crew member walks under a lamp
- **WHEN** a crew avatar walks from between two bridge lamps to directly under one
- **THEN** the top of its head gets brighter, matching the floor beneath it within 10 levels

#### Scenario: A probe inside a prop
- **WHEN** a probe point falls inside the reactor
- **THEN** it is marked invalid and a body near it is lit from its valid neighbours only

### Requirement: Runtime lights add over the bake without shadows
The client SHALL add up to four runtime lights per compartment per vertex (muzzle flashes,
sparks, alarm beacons, fire, door spill), with the bake's falloff, in light units: the baked colour
decoded, the runtime irradiance over `reference_lux` times the vertex's baked occlusion added, and
the sum encoded again. A light entering or leaving a compartment's four SHALL fade over 0.1 s.

#### Scenario: A muzzle flash in a corner
- **WHEN** a weapon fires 1 m from a room's corner
- **THEN** the open floor beside it brightens more than the corner, because the corner's baked occlusion is lower

### Requirement: The hull bakes occlusion only
For the ship's exterior, the baker SHALL bake ambient occlusion with `exterior_ao_radius_m` into the
alpha of the hull's vertex colours and no direct light; the sun SHALL be a runtime light.

#### Scenario: A turret well in sunlight
- **WHEN** the ship turns so the sun shines straight into a turret well
- **THEN** the well's floor is lit by the sun at run time and its ambient term stays darker than the open hull's

### Requirement: Bakes are reproducible and reported
Every sample pattern SHALL be seeded from the bake `seed`, a stable key built from the
compartment id, the face's stable index and the grid coordinates, and the purpose, never from
thread, order or time. The same inputs, seed and baker version SHALL give byte-identical output on
one platform, with any number of threads. Every bake SHALL write a report per compartment: triangles
before and after subdivision, residual, cap reached, rays traced, time and a SHA-256 digest of the
output. A test SHALL pin the digests of the reference ship's compartments.

#### Scenario: Eight threads or one
- **WHEN** the Tern is baked with one thread and again with eight
- **THEN** every compartment's digest is the same

#### Scenario: A changed seed
- **WHEN** the bake seed changes from 1 to 2
- **THEN** the digests change, and the pinned-digest test fails until it is updated in the same commit

### Requirement: The bake has debug views
Development builds of the client SHALL offer, per compartment: lighting only (albedo white),
occlusion only, one state alone, vertex density as a wireframe, the residual as a heat map, probe
cubes, fixture positions with their range, and a leak view marking vertices whose gather rays hit
back faces more than 30 % of the time.

#### Scenario: Finding a leak
- **WHEN** a detail brush is placed so that it pierces the bridge floor and the leak view is on
- **THEN** the floor vertices inside the brush are marked
