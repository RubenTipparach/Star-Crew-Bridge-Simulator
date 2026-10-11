# ship-damage

## ADDED Requirements

### Requirement: A hit damages what it reaches

A hit's energy past the shield and armour SHALL march along its path into the ship and take integrity from the
systems, nodes and conduits within reach, by `damage.json`'s propagation rule.

#### Scenario: A beam through the drive room
- **WHEN** a hit with energy past the armour enters the drive room along a line through the impulse drive
- **THEN** the impulse drive loses integrity and the ship's thrust falls with its capability

### Requirement: A damaged system cascades

A hit on a system already below nominal SHALL send a surge through its node and conduits into the components wired
to it, and on from any of them that was already below nominal.

#### Scenario: Two damaged systems on one panel
- **WHEN** a damaged system is hit again and another damaged system shares its node
- **THEN** both lose integrity, and the surge goes on from the second

### Requirement: Teams bring systems back

Damage-control teams SHALL go to fires and damaged systems and restore them over time.

#### Scenario: A repair
- **WHEN** a system is disabled and a team reaches it
- **THEN** its integrity rises until it is back at nominal
