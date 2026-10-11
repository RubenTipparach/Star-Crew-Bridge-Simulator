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

### Requirement: Only the captain vents a room

Venting a room SHALL be a guarded command of the captain's console; the damage control board SHALL be able to request
it and SHALL NOT be able to vent, and automation SHALL never vent.

#### Scenario: The damage board asks
- **WHEN** the damage control officer requests a vent of the quarters
- **THEN** the request shows on the captain's console and the quarters' dump stays shut until the captain arms and fires it

### Requirement: Crew in a venting room take damage as oxygen and heat fall

Every crew member in a room being vented SHALL lose health at the rates of `atmosphere.json` `crew_effects` for the
room's oxygen partial pressure and air temperature, each step, from the moment the dump opens.

#### Scenario: Caught in the quarters
- **WHEN** the captain vents the quarters with a crew member inside who does not leave
- **THEN** that crew member's health falls as the quarters' oxygen and temperature fall, and they are down before the room reaches 20 kPa's 90 s limit

### Requirement: Fires break out at fire-prone spots

The simulation SHALL start a fire of `seed_kw` at one of the ship's fire-prone spots (`fire.outbreaks.points`) when no
outbreak fire is burning and a seeded interval drawn from `fire.outbreaks.interval_s` has passed since the last one
went out, choosing among the spots that can catch now and never the spot that caught last. The draws SHALL come from a
generator seeded by the session seed and the purpose, so the same seed breaks out the same fires at the same times.

#### Scenario: The galley catches
- **WHEN** the ship has run with no outbreak fire for the drawn interval and the mess galley is the spot drawn
- **THEN** a 50 kW fire starts on the galley counter's cell and grows by the cell rules

#### Scenario: Tactical's wiring only while it is damaged
- **WHEN** tactical's console is repaired
- **THEN** the spot under its desk is not drawn until the console is damaged again

#### Scenario: A replay
- **WHEN** two runs start with the same session seed and nobody fights the fires
- **THEN** the same spots break out at the same times in both

