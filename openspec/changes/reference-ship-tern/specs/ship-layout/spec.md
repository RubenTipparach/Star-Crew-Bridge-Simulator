# Ship layout

## Purpose

The one layout source per ship (`data/ships/<id>/layout.json`, schema
`starcrew.ship-layout/1`), the rules every layout must pass, the deck plans drawn from it, and
the properties of the reference ship, SCS Tern, that other changes depend on.

## ADDED Requirements

### Requirement: One layout source per ship
A ship's floor plan (decks, hull, compartments, portals, stations, fixtures, systems, mounts and
craft) SHALL live in one file, `data/ships/<id>/layout.json`, whose `schema` is
`starcrew.ship-layout/1`. The mockups, the deck plans, the deck build and the simulation SHALL
read it and SHALL NOT keep a second list of rooms or doors.

#### Scenario: A door moves
- **WHEN** a portal's `center_m` changes in the layout
- **THEN** the deck plans, the mockups (after inlining) and the deck build all show the door in its new place with no other file edited

### Requirement: Ids are unique and every reference resolves
Within each list every `id` SHALL be unique, every compartment's `poi` SHALL be unique, and every
reference (a portal's compartments, a station's compartment and mount, a mount's crew seat, a
system's compartment, a craft's bay, cradle and drop door, a compartment's decks) SHALL name an
existing record, or `space` where the schema allows it.

#### Scenario: A station in a missing room
- **WHEN** a station names compartment `gym` and no compartment has that id
- **THEN** the layout check fails naming the station and `gym`

### Requirement: Boxes are well formed and compartments do not overlap
Every box SHALL have `min < max` on each axis, and no two boxes, of one compartment or of two,
SHALL share volume (touching faces are allowed).

#### Scenario: Two rooms overlap
- **WHEN** the medbay's box is widened 0.5 m into the main corridor
- **THEN** the layout check fails naming both compartments and box indices

### Requirement: Portals sit on a shared face and fit it
Every portal SHALL lie on a face that each of its compartments has, at its `center_m` on its
`axis`, and its clear opening (`size_m`) SHALL fit inside that face. A portal along `y` between
two compartments MAY cross one deck slab (`deck_slab_m`). A portal to `space` SHALL lie on a
face of its one compartment.

#### Scenario: A door in mid-air
- **WHEN** a door's centre is moved 1 m off the wall it belongs to
- **THEN** the layout check fails naming the door and the compartment whose face it misses

### Requirement: Compartments stay inside the hull
Every compartment except a pod SHALL lie inside the hull, its walls (`wall_thickness_m`) and
slabs (`deck_slab_m`) included, and the check SHALL report each compartment's least margin.

#### Scenario: A room pokes through
- **WHEN** a launch bay is widened 0.5 m outboard
- **THEN** the layout check fails naming the bay's corner and how far outside the hull it is

### Requirement: Every compartment is reachable from the bridge
Every compartment SHALL be reachable from the bridge through crew portals (doors, pressure
doors, hatches, ladders).

#### Scenario: An island room
- **WHEN** the shield generator's only door is removed
- **THEN** the layout check fails naming the shield generator

### Requirement: Placed things sit inside their compartment
Every station seat, fixture centre, system centre and craft centre SHALL lie inside its
compartment's boxes (a seat or system up to 0.6 m below the box's floor, for a seat that sits
on a dais or a system on a floor, is allowed; a craft has no tolerance).

#### Scenario: A console outside its room
- **WHEN** the damage control board's seat is moved into the main corridor
- **THEN** the layout check fails naming the station

### Requirement: Every compartment states its numbers
The layout check SHALL print, per compartment by POI, its decks, volume in cubic metres, floor
area in square metres and hull margin in metres, and the ship's totals and counts.

#### Scenario: The Tern's report
- **WHEN** `python3 tools/layout_check.py` runs on the Tern
- **THEN** it prints 30 compartments with their volumes, floor areas and hull margins, the total air (8,858.8 m^3 before the proposed patches), and "ok"

### Requirement: Every station and level is walkable
Inside a compartment with more than one walkable level (a landing, a mezzanine, a catwalk), the
layout SHALL place stairs or ladders joining the levels, and every station seat, door sill and
system SHALL be reachable on foot from every spawn.

#### Scenario: The engineering console
- **WHEN** a crew member walks from the bridge to the engineering bay console by the aft passage
- **THEN** they enter on engineering's catwalk and reach the console by the stair down to the mezzanine

### Requirement: Deck plans are drawn from the layout
`tools/deck_plans.py` SHALL draw one SVG per deck from the layout and shipkit's colour roles,
with compartments filled by kind, POI numbers, portals coloured by kind, ladders, stations by
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
