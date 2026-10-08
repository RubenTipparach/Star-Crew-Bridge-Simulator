# crew-npcs

## ADDED Requirements

### Requirement: The berths a player does not take are crewed

At mission start every berth no player takes SHALL hold an NPC crew member generated from the session
seed, with a name, a department and a job.

#### Scenario: Four players on the Tern
- **WHEN** a session of four players starts on the Tern
- **THEN** eight NPC crew are aboard, and the same seed gives the same eight

### Requirement: NPC work runs through the players' rules

An NPC's actions SHALL be issued as the same intents a player's input produces and resolved by the same
code, at the NPC's competence where the rule has a rate.

#### Scenario: An engineer repairs
- **WHEN** an engineering NPC repairs a damaged breaker with competence 0.6
- **THEN** the repair completes in the player's repair time divided by 0.6

### Requirement: NPCs go to their stations on red alert

When red alert is set, every NPC SHALL go at a run to its department's red-alert station, and return to
its schedule when the condition returns to normal.

#### Scenario: Red alert
- **WHEN** the captain sets red alert
- **THEN** the medic heads for the medbay and the deckhands for the magazine

### Requirement: The bot crew walk the ship between their department's rooms

Each bot crew member SHALL walk, by a path that keeps its body on the walk world, from place to place among its
department's rooms, staying at each for a time drawn from its seeded stream, the same seed giving the same choices.

#### Scenario: An engineer on rounds
- **WHEN** an engineering bot has stood its time in the switchboard
- **THEN** it walks to another engineering room by a path and arrives there on its feet
