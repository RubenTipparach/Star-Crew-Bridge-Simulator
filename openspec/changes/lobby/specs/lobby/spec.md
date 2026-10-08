# lobby

## ADDED Requirements

### Requirement: A game starts from the lobby

The client SHALL open on the lobby, and SHALL load the ship and put the player aboard only when the player beams
aboard.

#### Scenario: Beaming aboard
- **WHEN** the player presses Beam aboard with a name
- **THEN** the ship loads and the player stands on the bridge with that name

### Requirement: Generated names are seeded and never a character's

The name generator SHALL draw from `data/crew/names.json` by a seed, the same seed giving the same name, and SHALL
never return a name on the table's deny list.

#### Scenario: The same seed
- **WHEN** two names are generated with the same seed
- **THEN** they are the same name

#### Scenario: A denied name
- **WHEN** a draw produces a name on the deny list
- **THEN** the generator draws again
