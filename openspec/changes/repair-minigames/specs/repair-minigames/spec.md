# repair-minigames

## ADDED Requirements

### Requirement: A finished round lands its share at once

A player's repair SHALL be cut into rounds by the job's state (3 damaged, 4 disabled, 6 destroyed), and finishing a
round SHALL raise the system's integrity by the round's share immediately, with the last round landing on the target.

#### Scenario: Three clean rounds on a damaged turret
- **WHEN** a player repairs a turret from 25% and finishes its first round without a fumble
- **THEN** the turret is at 50% as the round ends, with no wait for a bar to fill

### Requirement: Every round of a job is the same game, harder

Round n of a job SHALL be the same mini-game as round 1, played at level min(n, 6), with only that game's difficulty
knobs changed between levels.

#### Scenario: The second round of an impulse unit
- **WHEN** a player starts the second round of an impulse unit's repair
- **THEN** it is the injector timing game again, with pulses at 280 px a second and a 27 px firing window

### Requirement: A fumble costs a set share and the system's hazard

A mistake in a repair game SHALL take 5% of the job back (`damage.json` `repair.fumble_share`) and apply that
system's hazard, and three fumbles in one round SHALL restart the round.

#### Scenario: A crossed pair in a splice
- **WHEN** a player joins two wires of different colours while splicing a conduit
- **THEN** the conduit's job loses 5%, the player takes 5 HP from the spark, and the breaker trips

### Requirement: Bots repair without a game

A bot or a damage control team SHALL repair at a rating's rate with no game and no fumbles.

#### Scenario: An engineering bot on a scrubber
- **WHEN** an engineering bot repairs a life support scrubber at 50%
- **THEN** it reaches 100% at 0.6% a second with no fumble

### Requirement: The medic treats wounds by tool

The medic's treatment SHALL show each wound on a body chart, SHALL heal a wound only with its tool, and SHALL give HP
at `medical-officer`'s rates.

#### Scenario: A burn in the field
- **WHEN** the medic paints burn gel over a burn on a body at 30 HP
- **THEN** the body gains HP at 4.0 HP a second up to 75 HP, one dose spent per 25 HP
