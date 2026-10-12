# power-routing

## ADDED Requirements

### Requirement: Every room has a conduit box and every load a feeder

Every room SHALL have a conduit box (its switchboard section or panel where it has one), runs SHALL join the boxes
through the doorways, and every load in a room SHALL be wired to its room's box.

#### Scenario: The plan connects everything
- **WHEN** the grid is drawn on the floor plan
- **THEN** every load has a feeder to a box, and every box a run to another box

### Requirement: Every panel connects to at least two others

Every conduit box, switchboard section and panel SHALL be joined by runs or ties to at least two others, so every room
has a second way for its power to come.

#### Scenario: A room with one door
- **WHEN** the bridge's doorway run is cut
- **THEN** a spare through its wall to the computer core or the ready room can feed it

### Requirement: Every system has two feeds, the nearest first

Every system SHALL be wired to two panels: its nearest (its own room's, else its deck's) and the next nearest, fed from
the first at the factory and swappable to the second. A system swapped SHALL draw from its second panel only.

#### Scenario: Swap round a cut
- **WHEN** the shield room's feed is severed and the shield generator is swapped to its second feed
- **THEN** it is supplied from the forward switchboard's port section

### Requirement: The reactor feeds five panels

The reactor SHALL feed five panels, each through its own breaker, so the ship is carried with any two of them open.

#### Scenario: Both generators open
- **WHEN** both generators' breakers are open at cruise
- **THEN** the three reactor feeds carry every load

### Requirement: Boxes and runs route power

A box switched off SHALL pass no power, and a run SHALL carry power only while it is closed, whole, and both its boxes
are on. A spare run closed SHALL carry power round a cut feed.

#### Scenario: Round the long way
- **WHEN** the dorsal turret room's feed is severed and the spare through the aft passage is closed
- **THEN** the dorsal turret is supplied from engineering

### Requirement: Too many live runs trip the grid and restore the factory settings

The main protection SHALL allow at most `routing.max_live_runs` live runs. One more SHALL trip the grid: every box
but the switchboard sections and the emergency bus off and every run open for `routing.trip_dark_s`, nothing
switchable, the switchboards' own loads still supplied; then every switch SHALL return to its factory setting.

#### Scenario: Overload
- **WHEN** an engineer closes spares until 43 runs are live
- **THEN** the ship past the switchboards goes dark for 8 s, the reactor runs on, and the grid comes back with every
  spare open

### Requirement: Engineering's automation routes round a cut

With Engineering on automation, a switched-on box with a load cut off from every source SHALL be reconnected by
closing a spare run after the automation's reaction time, never past the protection's limit.

#### Scenario: A dark panel's system
- **WHEN** a system's panel loses its power and its other panel has power
- **THEN** after the automation's reaction time the system is swapped to its other feed

#### Scenario: A cut feed
- **WHEN** a hit severs the only closed run into a room with loads and a spare reaches it
- **THEN** 3 s later the spare is closed and the room has power
