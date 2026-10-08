## ADDED Requirements

### Requirement: Escape pods carry the whole crew off
The ship SHALL carry three escape pods of four seats; the captain's ABANDON SHIP SHALL arm them, and an
armed pod SHALL launch 10 s after its hatch seals, or at once by its own handle, with or without the
ship's power, handing its bodies to the exterior frame with the ship's velocity plus 8 m/s outward.

#### Scenario: Abandoning ship
- **WHEN** the captain holds ABANDON SHIP and four crew board the deck A pod and seal its hatch
- **THEN** the pod launches 10 s later and its beacon shows on the other pods' screens

### Requirement: One airlock docks and does EVA
The deck C airlock's outer door SHALL be a docking collar: docked with both sides at pressure it SHALL
open straight through, and otherwise the airlock SHALL cycle to the other side's pressure as for EVA.

#### Scenario: Docking with a station
- **WHEN** a station at 101 kPa is docked at the collar
- **THEN** the outer door opens without a cycle and bodies walk across

### Requirement: A second airlock for EVA
A second airlock on deck A SHALL serve EVA when the deck C airlock is docked, lost or cycling.

#### Scenario: EVA while docked
- **WHEN** the deck C airlock is docked and a crew member must go outside
- **THEN** they suit at the deck A airlock and cycle out there
