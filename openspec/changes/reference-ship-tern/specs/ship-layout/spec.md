# Ship layout

## Purpose

The one layout source per ship (`data/ships/<id>/layout.json`, schema
`starcrew.ship-layout/2`: compartments built of convex prism brushes that follow the hull), the
rules every layout must pass, the deck plans drawn from it, and the properties of the reference
ship, SCS Tern, that other changes depend on.

## ADDED Requirements

### Requirement: One layout source per ship
A ship's floor plan (decks, hull, compartments, portals, stations, fixtures, systems, mounts and
craft) SHALL live in one file, `data/ships/<id>/layout.json`, whose `schema` is
`starcrew.ship-layout/2`. The mockups, the deck plans, the deck build and the simulation SHALL
read it and SHALL NOT keep a second list of rooms or doors. A loader SHALL refuse a file whose
`schema` is not one it reads, naming the schema it found.

#### Scenario: A door moves
- **WHEN** a portal's `center_m` changes in the layout
- **THEN** the deck plans, the mockups (after inlining) and the deck build all show the door in its new place with no other file edited

#### Scenario: An old file
- **WHEN** the layout check reads a file whose schema is `starcrew.ship-layout/1`
- **THEN** it fails naming the schema it found and the one it expects

### Requirement: Ids are unique and every reference resolves
Within each list every `id` SHALL be unique, every compartment's `poi` SHALL be unique, and every
reference (a portal's compartments, a station's compartment and mount, a mount's crew seat, a
system's compartment, a craft's bay, cradle and drop door, a compartment's decks) SHALL name an
existing record, or `space` where the schema allows it.

#### Scenario: A station in a missing room
- **WHEN** a station names compartment `gym` and no compartment has that id
- **THEN** the layout check fails naming the station and `gym`

### Requirement: Brushes are convex prisms and compartments do not overlap
A compartment's air SHALL be the union of its brushes. Every brush SHALL be a prism: a footprint
polygon in plan of at least three finite corners, convex with no straight corners, wound with
positive signed area, and a floor below its ceiling. No two brushes, of one compartment or of
two, SHALL share volume (touching faces are allowed; two brushes of one compartment that touch
are open to each other).

#### Scenario: Two rooms overlap
- **WHEN** the medbay's outer corner is moved 0.5 m into the main corridor
- **THEN** the layout check fails naming both compartments and brush indices

#### Scenario: A room that is not convex
- **WHEN** a brush's footprint has a corner that turns the wrong way
- **THEN** the layout check fails naming the compartment and the brush

### Requirement: Portals sit on a shared wall facing their normal and fit it
Every portal SHALL carry a unit `normal` from its first compartment into its second (or into
`space`, which is always second). A wall portal (horizontal normal) SHALL lie, within a
millimetre, on a wall of each of its compartments whose outward normal matches the portal's
within half a degree (so a portal may sit in an angled wall), and its clear opening (`size_m`:
width along the wall, height) SHALL fit inside that wall's length and height. A floor portal
(normal straight up or down) SHALL fit inside the footprint of a brush of each compartment, on
the floor or ceiling it leaves through, and MAY cross one deck slab (`deck_slab_m`).

#### Scenario: A door in mid-air
- **WHEN** a door's centre is moved 1 m off the wall it belongs to
- **THEN** the layout check fails naming the door and the compartment whose wall it misses

#### Scenario: A window in the bridge's angled wall
- **WHEN** the layout check reads `p_bridge_window_p`, whose normal is about 46 degrees off the bow
- **THEN** it finds the bridge's forward port wall facing the same way and passes

### Requirement: Compartments keep the hull clearance
Every compartment except a pod SHALL keep at least `hull.clearance_m` (0.5 m on the Tern: the
plating, frames and insulation) between its air and the hull's skin at every corner of every
brush, tested segment by segment along the hull's sections, and the check SHALL report each
compartment's least margin.

#### Scenario: A room pokes through
- **WHEN** a launch bay is widened 0.5 m outboard
- **THEN** the layout check fails naming the bay's corner and its distance to the hull, less than the clearance

### Requirement: The Tern's rooms follow the hull
Every compartment of the Tern that reaches the ship's side, top or bottom (the bridge, the ready
room and computer core, the torpedo room, medbay, damage control, quarters, mess, the turret
access rooms, the hangar, the launch bays, engineering, the drive section, the magazine, life
support, cargo, the airlock, the shield room and the forward switchboard) SHALL have its least
hull margin between `hull.clearance_m` and 1.6 m, so its outer walls follow the hull's line
rather than standing inside it as a box (owner, 2026-10-05: "the floor plan shouldnt consist of
square rooms, confirm better to ship shape").

#### Scenario: The Tern's margins
- **WHEN** `python3 tools/layout_check.py` runs on the Tern
- **THEN** every compartment listed has a hull margin from 0.5 m to 1.6 m (the bridge's is 0.63 m, the drive section's 1.58 m)

### Requirement: Every compartment names a finish
Every compartment SHALL name a `finish` that the ship's `data/ships/<id>/detailing.json`
defines, and every role of every finish SHALL name a material in
`data/materials/materials.json`.

#### Scenario: A misspelt finish
- **WHEN** the mess's finish is set to `crwe`
- **THEN** the layout check fails naming the mess and the finishes that exist

### Requirement: Every compartment is reachable from the bridge
Every compartment SHALL be reachable from the bridge through crew portals (doors, pressure
doors, hatches, ladders).

#### Scenario: An island room
- **WHEN** the shield generator's only door is removed
- **THEN** the layout check fails naming the shield generator

### Requirement: Placed things sit inside their compartment
Every station seat, fixture centre (and every corner of a fixture's `poly`), system centre and
craft centre SHALL lie inside one of its compartment's brushes (a seat or system up to 0.6 m
below the brush's floor, for a seat that sits on a dais or a system on a floor, is allowed; a
craft has no tolerance).

#### Scenario: A console outside its room
- **WHEN** the damage control board's seat is moved into the main corridor
- **THEN** the layout check fails naming the station

### Requirement: Every compartment states its numbers
The layout check SHALL print, per compartment by POI, its decks, brush count, volume in cubic
metres, floor area in square metres and hull margin in metres, and the ship's totals and counts.
Volumes are the brushes' air: generated detail (frames, coves, trims) is not air.

#### Scenario: The Tern's report
- **WHEN** `python3 tools/layout_check.py` runs on the Tern
- **THEN** it prints 30 compartments in 34 brushes with their volumes, floor areas and hull margins, the total air (9,824.8 m^3 before the proposed patches), and "ok"

### Requirement: Every station and level is walkable
Inside a compartment with more than one walkable level (a landing, a mezzanine, a catwalk), the
layout SHALL place stairs or ladders joining the levels, and every station seat, door sill and
system SHALL be reachable on foot from every spawn.

#### Scenario: The engineering console
- **WHEN** a crew member walks from the bridge to the engineering bay console by the aft passage
- **THEN** they enter on engineering's catwalk and reach the console by the stair down to the mezzanine

### Requirement: Deck plans are drawn from the layout
`tools/deck_plans.py` SHALL draw one SVG per deck from the layout and shipkit's colour roles,
with compartments filled by kind as their footprint polygons, wall portals along their (possibly
angled) walls, POI numbers, portals coloured by kind, ladders, stations by
role, systems by letter, craft, the hull outline at the deck's mid height, a grid, a scale bar
and a legend listing each POI with its decks, volume and floor area.

#### Scenario: Regenerating the Tern's plans
- **WHEN** `python3 tools/deck_plans.py` runs
- **THEN** it writes `docs/design/maps/tern-deck-A.svg`, `tern-deck-B.svg` and `tern-deck-C.svg`, and each POI on a deck appears once on that deck's map and in its legend

### Requirement: The Tern reaches engineering two ways, one of them always pressurized
The Tern SHALL have two routes from the bridge to the engineering bay console: one through the
command passage, the dorsal turret access and the aft passage on deck A, which crosses no
compartment that vents in normal operation (the hangar, a launch bay, the airlock), and one
through the hangar. Each SHALL take under 40 s at the walking speed `crew-on-deck` sets.

#### Scenario: A shuttle launch
- **WHEN** the hangar is at vacuum for a shuttle launch
- **THEN** the engineer walks from the bridge to the engineering bay console by the aft passage without passing a pressure door

### Requirement: The Tern's crew reach their places quickly
At walking speed, the four core stations SHALL be under 30 s from the crew quarters, and both
launch bays SHALL be under 40 s from every bridge station.

#### Scenario: Scramble
- **WHEN** the helm officer leaves the helm for the port launch bay at walking speed
- **THEN** they reach Swift 1's cradle in under 40 s

### Requirement: The bridge has two ways out
The Tern's bridge SHALL have a second exit (the scuttle to damage control) so that losing the
bridge door or the command passage does not cut the bridge off from the rest of the ship.

#### Scenario: Fire in the command passage
- **WHEN** the command passage is impassable
- **THEN** the bridge crew reach damage control through the scuttle and the rest of the ship through the main corridor
