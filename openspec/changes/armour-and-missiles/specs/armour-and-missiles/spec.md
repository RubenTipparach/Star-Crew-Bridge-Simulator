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
