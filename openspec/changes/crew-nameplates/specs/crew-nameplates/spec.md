# crew-nameplates

## ADDED Requirements

### Requirement: Every crew member in sight is named

The client SHALL draw the name of every crew member's body that is on screen, within 25 m of the eye and in line of
sight on the walk grid, and SHALL never draw one over the player's own body.

#### Scenario: A bot in the same room
- **WHEN** a bot crew member stands 6 m in front of the player in the same compartment
- **THEN** its name is drawn above its head

#### Scenario: A bot behind a bulkhead
- **WHEN** a bot crew member is 6 m away on the same deck with a wall between it and the player
- **THEN** no name is drawn for it

### Requirement: A bot is marked as an NPC

The nameplate of a body driven by a bot or by automation SHALL end with "(NPC)", and a player's SHALL NOT.

#### Scenario: A bot and a player in the drill
- **WHEN** a player and an `sc-bot` crew member are both on the bridge
- **THEN** the bot's plate reads its name followed by "(NPC)" and the player's reads the name alone

### Requirement: Names can be turned off

The client SHALL draw no nameplates when the Names option is Off, and SHALL default it to On.

#### Scenario: Names off
- **WHEN** the player sets Names to Off in Options
- **THEN** no nameplate is drawn until it is turned back On
