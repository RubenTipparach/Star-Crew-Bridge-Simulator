# laser-banks

## ADDED Requirements

### Requirement: A bank charges over about 30 seconds

Each laser bank SHALL charge its capacitor from empty to full in its data's time (30 s), and SHALL fire only with at
least its least charge.

#### Scenario: Charging
- **WHEN** a bank fires and then waits 30 s
- **THEN** its capacitor is full again

### Requirement: Armour stops a laser

A laser hit SHALL take what the facing shield can hold, and the rest SHALL reach the hull only where that face's
armour is gone.

#### Scenario: Armour left
- **WHEN** a full bank hits a face with no shield and armour left
- **THEN** the hull loses nothing

#### Scenario: Armour gone
- **WHEN** a full bank hits a face with no shield and no armour
- **THEN** the hull loses the shot's energy
