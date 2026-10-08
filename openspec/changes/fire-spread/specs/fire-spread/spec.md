# fire-spread

## ADDED Requirements

### Requirement: A room's fire is its burning floor cells

A room's heat release SHALL be the sum of the heat release of its burning floor cells (0.5 m squares, each with the
fuel class of what stands on it), and the room model SHALL take that sum as its fire's heat release for oxygen,
smoke, heat and damage.

#### Scenario: A fire set on a bunk
- **WHEN** a 50 kW fire is set on a bunk in the quarters
- **THEN** the cells under the bunk burn, their neighbours ignite as their dose passes their class's figure, and the
  quarters' heat release is the sum of the burning cells

### Requirement: Unattended growth matches the measured curve

With nothing aimed at it, a 50 kW fire in the quarters with the door shut SHALL pass 1 MW 113 s after ignition, give
or take 10%, as `damage-control`'s room model does.

#### Scenario: The calibration case
- **WHEN** the fire cases harness runs the quarters, door shut, 50 kW seed
- **THEN** it reports 1 MW between 102 s and 124 s

### Requirement: Flashover at autoignition

Every cell with fuel in a room SHALL ignite when the room's air passes the autoignition temperature (300 C).

#### Scenario: A sealed room heats past 300 C
- **WHEN** the quarters' air passes 300 C
- **THEN** every cell with fuel left in the quarters is burning in that step

### Requirement: An extinguisher cuts only the cells it covers

An extinguisher's cut SHALL be shared among the burning cells in its cone's footprint in proportion to their heat
release, and SHALL touch no cell outside it.

#### Scenario: Spraying the floor beside the fire
- **WHEN** a player discharges an extinguisher at a part of the floor with no burning cell
- **THEN** the room's heat release is not cut, and the agent is spent

### Requirement: A knocked-down cell can reflash

A cell put out with fuel left SHALL gather no dose while the agent lies on it (20 s), and SHALL be able to reignite
after that from a burning neighbour or the hot layer.

#### Scenario: A line left burning behind
- **WHEN** a player knocks down half a fire and leaves a burning cell beside the knocked-down ones
- **THEN** the knocked-down cells beside it reignite once their 20 s pass
