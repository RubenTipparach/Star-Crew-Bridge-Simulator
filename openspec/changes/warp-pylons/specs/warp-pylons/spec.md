# warp-pylons

## ADDED Requirements

### Requirement: A pylon takes the hits that reach it

A shot whose path meets a warp pylon's or its nacelle's hit volume before the hull SHALL damage that pylon's integrity
by the energy left after the shield, at the pylon's armour, and SHALL go on to the hull with what passes through.

#### Scenario: A bolt through the starboard nacelle
- **WHEN** a bolt that passes the shield crosses the starboard nacelle's volume on its way to the hull
- **THEN** `warp_pylon_starboard` loses integrity, and only the energy left after its armour reaches the hull

### Requirement: Warp needs both pylons

The ship SHALL refuse a warp jump while either pylon's capability is 0, and SHALL spool a jump in its time divided by
the lower pylon's capability otherwise.

#### Scenario: The port pylon shot away
- **WHEN** `warp_pylon_port` is at 10 integrity and Helm orders a jump
- **THEN** the jump is refused and Helm's console shows the port pylon down

### Requirement: A pylon is mended outside

A damaged pylon SHALL be repaired only by a suited crew member docked at its coil, outside the hull, reached through
the airlock on magnetic boots and a tether; the airlock's outer door SHALL NOT open while a hostile is within weapons
range.

#### Scenario: Going out after the fight
- **WHEN** no hostile is within weapons range and a suited crew member cycles the airlock and walks to the damaged
  starboard pylon's coil
- **THEN** docking there starts the pylons game at its bolts, and finishing it restores the pylon's integrity

#### Scenario: Going out under fire
- **WHEN** a hostile is within weapons range and a crew member in the airlock asks the outer door to open
- **THEN** the door stays shut and the refusal is shown
