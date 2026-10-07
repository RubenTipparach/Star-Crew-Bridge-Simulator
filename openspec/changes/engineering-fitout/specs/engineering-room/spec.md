# engineering-room

## ADDED Requirements

### Requirement: The plant is drawn as the simulation runs it

Engineering SHALL show each flow `power.json` simulates through it (the reactor's fuel, the coolant loop and its two
pumps, the secondary loop to the radiators, the generators, the switchboards and the feeders) as machines joined by
pipe, cable tray or busbar, each machine on the loop it serves; a feeder SHALL be drawn along its `path_m`, and a
machine the simulation does not have SHALL show no readout that claims a number.

#### Scenario: A feeder drawn where it is simulated
- **WHEN** the deck plan draws a conduit's tray in engineering
- **THEN** the tray's centreline is the conduit's `path_m`

#### Scenario: Each coolant pump on its loop
- **WHEN** engineering is fitted out
- **THEN** each coolant pump joins its heat exchanger's cold leg to a return into the reactor

### Requirement: Pipe runs are routed by checked rules

Every pipe run SHALL start and end on what it names (a port within 1 cm and 2 degrees, the reactor at one of its entry
azimuths, a tee, or a penetration), stay inside the room and clear of its walls, leave 2.1 m of headroom over every
walk zone, stay out of every door's zone, stair and catwalk volume, keep clear of every other run by 5 cm and of every
machine it does not connect to, and have room for its bends; the fit-out tool SHALL refuse a patch that breaks one.

#### Scenario: A pipe through a head
- **WHEN** a run's underside is less than 2.1 m above a walk zone's floor
- **THEN** the tool reports the run and the zone and writes nothing

#### Scenario: A pipe in a doorway
- **WHEN** a run enters a door's zone
- **THEN** the tool reports the run and the door

### Requirement: Every open edge is railed

The edge of every opening a person can fall through in engineering (the well round the reactor, the stair holes, the
catwalks' open sides and the ring catwalk's outer edge) SHALL carry a railing of `detailing.json`'s height, except
where a stair or a bridge meets it.

#### Scenario: The well
- **WHEN** the mezzanine is drawn
- **THEN** a railing runs round the whole edge of the well

### Requirement: Engineering stays within its triangle ceiling

Engineering's geometry (shell, detail, props and runs) SHALL stay within 30,000 triangles, as counted by the deck plan.

#### Scenario: Counted in the page
- **WHEN** the deck plan builds engineering
- **THEN** its triangle count is at most 30,000
