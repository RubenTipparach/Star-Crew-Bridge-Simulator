# hull-repair

## ADDED Requirements

### Requirement: Walls are sections that take damage

Every wall bay of every room SHALL be a wall section with an integrity of 0-100%, and a hit's march SHALL damage the
sections of each room it deposits energy in, by the share of the energy within the blast radius over the section's
toughness.

#### Scenario: A blast in engineering
- **WHEN** a hit deposits 33.7 MJ in engineering
- **THEN** the wall sections within 3.2 m of the deposit lose integrity and show damaged panels, and those farther away do not

### Requirement: An open bulkhead section is a hole between rooms

A bulkhead section at 0% SHALL be a 0.05 m^2 opening between the two rooms it divides, through which air, heat, smoke
and fire pass.

#### Scenario: Smoke through a wrecked wall
- **WHEN** a bulkhead section between the mess and the spine reaches 0% while the mess is on fire
- **THEN** smoke reaches the spine through the hole with the mess's door shut

### Requirement: Plating from inside restores a hull section's armour to half

Plating every hull-side section behind a hull section to 100% SHALL raise that hull section's armour to 50% if it was
lower, and SHALL NOT raise it above 50%.

#### Scenario: Patching the port side
- **WHEN** a crew plates the four hull-side sections behind a port hull section at 10% armour
- **THEN** the hull section's armour is 50%

### Requirement: Wall damage waits behind every other job

The damage board's queue SHALL list plating jobs after fires, breaches, power and systems, and automation SHALL send
a team to a plating job only when no other job waits and the ship is not in combat.

#### Scenario: A quiet watch
- **WHEN** the queue holds only plating jobs and the ship is at cruise
- **THEN** automation sends a team to the nearest one
