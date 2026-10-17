# helm-nav-map

## ADDED Requirements

### Requirement: The helm console has a NAV mode with the system map

The helm console SHALL have a NAV tab beside HELM on the station's own chip in the title band. NAV SHALL open over the
four panels, keep the look band and the status strip, and show three panels: MAP, TARGET and JUMP.

#### Scenario: Helm opens NAV
- **WHEN** the player seated at helm presses NAV
- **THEN** the map of the current system replaces the four panels and the viewscreen stays in view

### Requirement: The map shows the system from its data file

The map SHALL draw the system file's star, bodies by their kind, orbits, belts, stations, points of interest, the
mission route's stops with their kind and state, the ship, mass lock circles, science's contacts and the plotted
course, from the data and from nothing written into the client.

#### Scenario: The route at the start
- **WHEN** the Tern is docked at Tamsin Station at the start of the patrol
- **THEN** the map shows every stop of the route in order with home, and the first stop as the plotted target

### Requirement: The route's next stop is preselected

After the patrol starts, after undocking and after every arrival, the server SHALL set the jump target to the route's
first stop not yet done, or home when none is left.

#### Scenario: A bot helm never plots
- **WHEN** a bot or automation holds helm and the captain orders JUMP
- **THEN** the Tern spools toward the preselected stop without any NAV command

### Requirement: Helm plots the jump target and the server decides

Helm SHALL set the jump target with a `NavTarget` command naming a place, a body or a free point. The server SHALL
refuse it with `MassLocked`, changing nothing, when the target's arrival lies within the arrival scatter of any mass
lock, and with `NotReady` while the drive is spooling.

#### Scenario: A free point inside a planet's lock
- **WHEN** helm plots a free point 5,000 km from Cinder's centre (Cinder's lock is 7,200 km)
- **THEN** the server refuses it with `MassLocked` and the previous target stays plotted

#### Scenario: Another stop than the next
- **WHEN** helm plots the Pell beacon while Orrin's rings is preselected, and the Tern jumps
- **THEN** the Tern arrives at the Pell beacon's arrival point and Orrin's rings stays open

### Requirement: The captain's order is the go

The captain's JUMP order SHALL mean "jump to the plotted target"; the captain's console SHALL show the plotted target and
course on its own NAV tab, read only.

#### Scenario: The captain sees helm's plot
- **WHEN** helm plots Orrin's rings
- **THEN** the captain's NAV tab shows the course to Orrin's rings and the JUMP order's PLOT light lit

### Requirement: Alignment is shown and can be flown automatically

The JUMP panel SHALL show the bow against the 5 degree alignment cone using the same rule the server's spool uses, and
an ALIGN autopilot SHALL turn the bow onto the jump vector and hold it.

#### Scenario: ALIGN
- **WHEN** helm presses ALIGN with the bow 14 degrees off the jump vector
- **THEN** the bow comes inside 5 degrees, turning under 2 deg/s, and the cue lights as aligned
