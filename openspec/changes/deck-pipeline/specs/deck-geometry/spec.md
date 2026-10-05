# Deck geometry

## Purpose

How a ship's decks are authored as convex brushes from the one layout source, compiled
offline by `deckc` into a checked, versioned `.deck` file, and drawn and collided against at
run time through the compartment graph, within the Pi 5 budget.

## ADDED Requirements

### Requirement: Decks compile from the one layout source
A ship's deck SHALL be compiled by `deckc` from `data/ships/<id>/layout.json`, the kit
(`data/decks/kit.json`) and the ship's detail files, and from nothing else that describes
compartments or portals. The `.deck` file SHALL record the SHA-256 of the layout's canonical
JSON, and the loader SHALL refuse a deck whose hash differs from the layout the server runs.

#### Scenario: A stale deck is refused
- **WHEN** a door is moved in `layout.json` and the game is started with the old `.deck`
- **THEN** the load fails naming the deck file and both hashes, and nothing is drawn from it

#### Scenario: A compartment is regenerated from the layout
- **WHEN** a compartment's box grows by 1 m in the layout and `deckc` runs
- **THEN** its shell brushes, trims, lamps and collision follow the new box with no hand edit

### Requirement: Brushes are convex and stay inside their compartment
Every brush SHALL be convex and closed: at least four planes, and every vertex on or behind
every plane within 0.1 mm. Every detail brush, prop bound and light fixture SHALL lie inside
its compartment's air boxes within 1 mm and outside every portal's clear opening. `deckc`
SHALL refuse a deck that breaks either rule, naming the brush and compartment.

#### Scenario: A non-convex detail object
- **WHEN** a hero detail file holds a brush whose vertex lies 2 mm in front of one of its planes
- **THEN** `deckc` refuses the deck and names the brush, its compartment and the plane

#### Scenario: A console that blocks a door
- **WHEN** a prop's bounds cross the clear opening of a door portal
- **THEN** `deckc` refuses the deck and names the prop and the portal

### Requirement: No z-fighting
`deckc` SHALL refuse a deck in which two faces of one compartment lie on planes within 5 mm of
each other, face the same way (normals within 0.5 degrees) and overlap by more than 1 mm^2.
Surfaces that are parallel on purpose SHALL be at least 1 cm apart. Faces pressed back to back
(opposite normals) SHALL be allowed.

#### Scenario: A trim flush with a wall
- **WHEN** a baseboard's front face lies 3 mm from the wall face it runs along and faces the same way
- **THEN** the compile fails naming both faces and the compartment

#### Scenario: A partition between two compartments
- **WHEN** two compartments share a wall plane and each draws its own face of it, facing opposite ways
- **THEN** the check passes

### Requirement: People stand clear of the deck
`deckc` SHALL place the crew capsule from `crew-on-deck` (standing at every spawn, ladder top
and bottom, stair head and foot, and 0.5 m either side of every door threshold; seated at every
seat) and SHALL refuse a deck where it intersects a colliding brush other than its own seat's
proxy. Every crew portal's clear width SHALL be at least the capsule's diameter plus 0.1 m.
Every seat and spawn SHALL be reachable on foot from every spawn through crew portals and
stairs.

#### Scenario: A seat too close to a wall
- **WHEN** a station's seat point lies 0.2 m from a wall and the capsule radius is 0.3 m
- **THEN** the compile fails naming the station and the wall brush

#### Scenario: A console no one can walk to
- **WHEN** a compartment's mezzanine holds a station and no stair or ladder joins the mezzanine to the level its doors open onto
- **THEN** the compile fails naming the station and the unreachable level

### Requirement: Every compartment states its numbers
`deckc --report` SHALL print, for every compartment, its volume in cubic metres, floor area in
square metres, triangles after the bake's subdivision, draw calls, vertex count and the portals
that leave it, and the ship's totals. `deckc` SHALL refuse a deck in which a compartment passes
its triangle ceiling from the `engine-stack` budget table (the bridge 30,000, others 8,000 while
that table is provisional) or needs more than four draw calls.

#### Scenario: A compartment over its ceiling
- **WHEN** engineering compiles to 8,400 triangles against an 8,000 ceiling
- **THEN** the compile fails naming engineering, its count and its ceiling, and the report still prints

### Requirement: The worst visible set fits the interior pass
`deckc` SHALL compute, for every compartment, the set of compartments visible through any
sequence of open portals, and SHALL refuse a deck whose largest set's triangles pass the
interior pass ceiling (80,000 triangles while the budget is provisional). The visible set
SHALL NOT be used for culling at run time.

#### Scenario: A long sightline through open doors
- **WHEN** a layout change lines up four doors so that six large compartments are visible from one corridor and their sum is 85,000 triangles
- **THEN** the compile fails naming the corridor and the compartments in the set

### Requirement: Light comes from fixtures and blends per compartment
Every light SHALL come from a fixture entity with a visible fixture brush or prop and a colour
for each of the three lighting states (normal, red alert, emergency). Each vertex SHALL carry
three baked colours from the `light-baking` baker, and the renderer SHALL blend them by the
compartment's state weights and scale them by the compartment's lighting-bus dimmer from the
simulation.

#### Scenario: Red alert
- **WHEN** the ship goes to red alert
- **THEN** every compartment's weights ease from the normal set to the red-alert set over 0.5 s, with no change to geometry

#### Scenario: A browned-out bus
- **WHEN** the simulation reports a compartment's lighting bus at 60 % voltage
- **THEN** that compartment's baked light is drawn at 60 % and its emissive faces are unchanged

### Requirement: Visibility is portal culling through the compartment graph
The renderer SHALL start from the compartment holding the eye and visit compartments through
portals whose screen rectangles intersect the current rectangle, narrowing it at each portal,
skipping closed portals that are not see-through and portals behind the eye. A door SHALL be
open for visibility when the simulation reports it more than 5 % open. Each visited
compartment SHALL be drawn with a scissor of its rectangle.

#### Scenario: A closed door hides the room behind it
- **WHEN** the eye is in the main corridor and the medbay door is closed
- **THEN** no medbay geometry is submitted that frame

#### Scenario: The door opens
- **WHEN** the medbay door reports 6 % open
- **THEN** the medbay is drawn, scissored to the door's narrow screen rectangle

### Requirement: Windows and open bay doors hand rectangles to the exterior pass
A portal to space (a window, an open bay door, an open outer airlock door) that passes the
traversal SHALL add its screen rectangle to the frame's exterior rectangles, which
`ship-frames`' exterior layers use as their scissor. The traversal SHALL run before the
exterior layers are drawn.

#### Scenario: No window in view
- **WHEN** the eye is in the magazine with every door closed
- **THEN** the frame has no exterior rectangle and the exterior layers draw nothing

#### Scenario: The bridge windows
- **WHEN** the eye is at the helm facing the bow
- **THEN** the two bridge windows' rectangles are the exterior layers' scissor

### Requirement: Crew collide with brush planes in their compartment
`sc-core` SHALL provide one swept capsule query against a compartment's colliding brushes, with
planes pushed out by the capsule's support distance, returning the earliest hit and its plane.
A body SHALL test its own compartment's brushes, and the neighbour's when within its radius of
an open portal's opening, and SHALL change compartment when its centre crosses a portal's plane
inside the opening. A compartment with more than 32 colliding brushes SHALL be searched
through a compiled k-d tree.

#### Scenario: Walking into a wall
- **WHEN** a crew capsule of radius 0.3 m walks at a wall at 1.8 m/s (`crew-on-deck`'s walking speed)
- **THEN** it stops 0.3 m from the wall's face and slides along it

#### Scenario: Walking through a door
- **WHEN** a crew capsule walks through an open door's clear opening
- **THEN** it passes without touching the frame and its compartment changes at the tick its centre crosses the door's plane

### Requirement: The deck file is versioned, chunked and checked
A `.deck` file SHALL be little endian with the 64-byte header of the design (magic `SCDK`,
major and minor version, sizes, chunk count, flags, the layout's SHA-256 and a header CRC-32)
followed by a chunk table with a CRC-32 per chunk. The loader SHALL refuse a different major
version, a failed CRC, an offset outside the file, a missing required chunk and any non-finite
float, and SHALL skip an unknown optional chunk. The server SHALL read only the core chunks.

#### Scenario: A corrupted chunk
- **WHEN** one byte of the vertex chunk is flipped
- **THEN** the client refuses the deck naming the chunk `VBUF` and the file

#### Scenario: A newer minor version
- **WHEN** a version 1.1 deck adds an optional chunk a 1.0 loader does not know
- **THEN** the 1.0 loader skips it and loads the rest

### Requirement: Indices fit their width
Every draw range SHALL use 16-bit indices when it references at most 65,535 vertices and
32-bit indices otherwise, and no index SHALL reference a vertex outside its range.

#### Scenario: A small compartment
- **WHEN** a compartment's opaque draw holds 8,400 vertices
- **THEN** it is written with 16-bit indices
