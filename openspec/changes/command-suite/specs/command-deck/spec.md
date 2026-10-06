# command-deck

## ADDED Requirements

### Requirement: The bridge is round, with its side rooms at walkway level

The Tern's bridge SHALL be the sixteen-sided room of `bridge-stations` design 11a, variant B. A
work ring two steps up SHALL run along each side wall. The ring SHALL leave each aft diagonal wall
at walkway level, so that the doors to the ready room and the briefing room open from the walkway
like the aft door.

#### Scenario: Three doors from the aft walkway
- **WHEN** a crew member stands on the walkway inside the aft door
- **THEN** the aft door, the ready room's door and the briefing room's door are all on the
  walkway's level, and none of them opens onto a step or a ring

#### Scenario: The ring's stairs keep clear of the side doors
- **WHEN** the ring's aft stair is checked against the door beside it
- **THEN** no part of the flight stands in the door's clear zone (1.0 m deep)

### Requirement: Helm and tactical leave the screen clear

Helm and tactical SHALL sit far enough apart that, from the captain's seated eye, their heads hide
none of the main viewscreen.

#### Scenario: The captain sees the whole screen
- **WHEN** the sightlines are measured from the captain's seated eye (seat plus 1.20 m) past the
  tops of helm's and tactical's heads (seat plus 1.32 m) to the screen's plane
- **THEN** the heads fall outside the screen's edges, and 0 % of the screen is hidden

### Requirement: The side rooms open onto the bridge and the passage

The captain's ready room and the briefing room SHALL each have a door onto the bridge and a door
onto the command passage. The captain's quarters SHALL have a door onto the passage and a door
into the ready room.

#### Scenario: A way off the bridge when the aft door is jammed
- **WHEN** the bridge's aft door is jammed shut
- **THEN** the crew reach the command passage through the ready room or the briefing room

#### Scenario: From the bed to the chair
- **WHEN** the captain walks from the quarters to the captain's chair
- **THEN** the route through the ready room is shorter than the route by the passage

### Requirement: The briefing room seats eight facing one screen

The briefing room SHALL hold a table with eight seats, every one of which faces along the table to
one wall screen at its end, so that a full crew of eight can be briefed together.

#### Scenario: A full crew is briefed
- **WHEN** eight players gather for a mission briefing
- **THEN** each has a seat at the table, and the briefing screen is at the table's end

### Requirement: Furniture keeps the doors clear

Furniture SHALL stand inside its room, apart from other furniture. It SHALL also keep out of a
zone in front of every door, 1.0 m deep and 0.1 m wider than the door on each side, on both sides
of the door.

#### Scenario: A piece in a doorway is refused
- **WHEN** a piece of furniture, or a stair on the bridge, overlaps a door's clear zone
- **THEN** the suite's check fails and names the piece and the door

### Requirement: The bridge crew suit up on deck A

The bridge crew's EVA suits SHALL be in a locker on deck A, so a bridge crew member reaches a suit
without the ladder trunk.

#### Scenario: Helm to a suit
- **WHEN** helm walks from the seat to the bridge locker's suits
- **THEN** the walk is shorter than the walk to damage control's suit locker on deck B
