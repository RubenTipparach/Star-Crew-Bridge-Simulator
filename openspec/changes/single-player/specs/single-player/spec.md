# single-player

## ADDED Requirements

### Requirement: A single-player game crews every station

A single-player game SHALL host the drill on the player's machine and SHALL seat a bot crew member at every station of
the mission's roster but the player's, each playing at the `bot` profile.

#### Scenario: Starting single player at Helm
- **WHEN** the player starts a single-player game at Helm
- **THEN** a bot crew member walks to and sits at each other station, and the round is played by them and the player

### Requirement: Each station is AUTO or MANUAL

The player SHALL be able to set each bot-crewed station to AUTO, where its crew member plays it, or MANUAL, where its
crew member stays seated and carries out only the player's commands on that station's console.

#### Scenario: Taking Tactical by hand
- **WHEN** the player sets Tactical to MANUAL and presses Fire on Tactical's console
- **THEN** Tactical's crew member sends no command of its own, and the fire command reaches the server as Tactical's

### Requirement: A station works only with its crew member seated

A bot-crewed station SHALL do nothing, on AUTO or MANUAL, while its crew member is not seated.

#### Scenario: Before the crew member sits
- **WHEN** Tactical's crew member is still walking to the seat and the player presses Fire on Tactical's console
- **THEN** the server refuses the command and nothing fires
