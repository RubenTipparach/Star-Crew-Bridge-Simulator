# enemy-fighters

## ADDED Requirements

### Requirement: The enemy launches two fighters

The Hound SHALL launch two fighters 20 s into Engage while its hangar works.

#### Scenario: Launch
- **WHEN** a fight has run 25 s and the Hound's hangar is undamaged
- **THEN** two enemy fighters are in flight

### Requirement: Cannons are for fighters

A cannon bolt SHALL do its full damage to a fighter and its capital factor of it to a ship.

#### Scenario: A bolt on each
- **WHEN** one bolt hits a fighter and another hits the Hound's shield
- **THEN** the fighter takes the bolt's damage and the Hound's face a quarter of it

### Requirement: A player can be the gunner

A player at GUNNER SHALL aim and fire the dorsal turret by hand.

#### Scenario: Shooting a fighter down
- **WHEN** the gunner holds the trigger with the sight on a fighter's lead pip
- **THEN** the turret's bolts fly along the aim and can destroy the fighter
