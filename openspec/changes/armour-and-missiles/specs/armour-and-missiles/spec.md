# armour-and-missiles

## ADDED Requirements

### Requirement: Missiles strip armour and reach the hull

A missile hit SHALL take integrity from the armour section it hits and pass the energy the armour does not absorb into
the ship.

#### Scenario: A missile on fresh armour
- **WHEN** a missile bursts against a section at full integrity with no shield on that face
- **THEN** the section loses integrity and the ship takes the rest

### Requirement: Twenty missiles, a minute to load

Each ship SHALL carry 20 missiles for the scenario, and a tube SHALL take 60 s to load.

#### Scenario: Loading
- **WHEN** Tactical loads an empty tube
- **THEN** it is armed 63 s later and the magazine has one fewer

### Requirement: Countermeasures can defeat a missile

A ship SHALL have 5 countermeasures, and a countermeasure thrown in a missile's last 3 s SHALL pull it off or set it
off early more often than one thrown earlier.

#### Scenario: A decoy in time
- **WHEN** a decoy is thrown 2 s before a missile's impact
- **THEN** the missile misses or bursts early with a chance of 0.85

### Requirement: A turret fires a hundred rounds, then reloads

Every turret, on either ship, SHALL fire at most 100 bolts before it reloads for 30 seconds, during which it fires
none; then it has 100 again. Its numbers SHALL come from its gun in `data/weapons.json`.

#### Scenario: Point defence runs dry
- **WHEN** a turret fires its hundredth bolt at a salvo of missiles
- **THEN** it fires nothing for the next 30 seconds, and the missiles it would have shot fly on

### Requirement: Missiles hit along the hull

A missile SHALL aim at a point along its target's hull drawn when it is launched, so that the hits of a fight land
from bow to stern rather than all amidships.

#### Scenario: A fight's hits
- **WHEN** the Hound's missiles hit the Tern over a fight
- **THEN** systems at both ends of the ship and in its middle are damaged, not only the hangar's

