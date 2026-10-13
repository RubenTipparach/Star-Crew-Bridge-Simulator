# all-hands

## ADDED Requirements

### Requirement: All hands is a game option with no damage-control teams

A game SHALL have an All hands option, off by default and set with the mission, and a game with it on SHALL carry no
damage-control teams, every other damage rule unchanged.

#### Scenario: A fire nobody reaches
- **WHEN** a hit starts a fire in a game with All hands on and no player goes to it
- **THEN** no team leaves the damage control room, and the fire spreads to a neighbouring room after 90 s

### Requirement: A fire is put out by hands, a team's or a player's

A fire SHALL be put out when the hand-seconds spent on it reach the fire's rule (a team counting its members, a player
spraying an extinguisher counting one), and SHALL NOT spread while any hand is at it.

#### Scenario: One player at a fire
- **WHEN** one player sprays a fire from its doorway, facing it, with fresh extinguishers
- **THEN** the fire is out after 40 s of spraying, and it did not spread meanwhile

#### Scenario: A team of two, as before
- **WHEN** a team of two works a fire in a game with the teams on
- **THEN** the fire is out after 20 s, as it is today

### Requirement: A player may leave a station and its automation plays it

A seated player SHALL be able to stand and walk away, and the station SHALL then be played by its automation until a
player sits at it again.

#### Scenario: Engineering goes to a fire
- **WHEN** the player at Engineering stands up during a fight
- **THEN** Engineering's automation keeps routing the grid, and the player walks the decks
