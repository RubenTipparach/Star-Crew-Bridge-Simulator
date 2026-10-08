# repair-minigames

## ADDED Requirements

### Requirement: A repair game never beats the rate

A player's repair SHALL advance by steps whose progress fills at `damage::repair_time(job, who)`'s rate, so that no
play finishes a job sooner than the time the damage board previews for that player.

#### Scenario: A steady hand on a damaged turret
- **WHEN** an officer repairs a turret from 25% and plays every step without a fumble
- **THEN** the turret reaches 100% in the board's previewed time, give or take one frame

### Requirement: A fumble costs a set share and the system's hazard

A mistake in a repair game SHALL take 5% of the job back (`damage.json` `repair.fumble_share`) and apply that
system's hazard, and three fumbles in one step SHALL restart the step.

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
