# game-menu

## ADDED Requirements

### Requirement: Esc opens the game menu

Aboard the ship, Esc on a keyboard or Start on a pad SHALL open the game menu with Resume, Options, Return to menu and
Exit, and SHALL NOT quit the game.

#### Scenario: Esc while walking
- **WHEN** the player presses Esc while walking with nothing else open
- **THEN** the game menu opens and the mouse is released

#### Scenario: Esc again
- **WHEN** the game menu is open and the player presses Esc
- **THEN** the menu closes and the game takes the input again

### Requirement: The menu pauses nothing

While the game menu is open the simulation SHALL keep running, and a seated player SHALL keep their station.

#### Scenario: Menu open in the drill
- **WHEN** a seated player opens the menu during a round
- **THEN** the round's clock keeps running and the station stays theirs

### Requirement: Leaving asks once

Return to menu and Exit SHALL each ask once, Yes or No, before acting; Yes on Return to menu SHALL leave the crew and
show the lobby, and Yes on Exit SHALL close the game.

#### Scenario: Return to menu
- **WHEN** the player chooses Return to menu and then Yes
- **THEN** the client leaves the ship and shows the lobby

### Requirement: Options are kept

The client SHALL apply a changed option at once, SHALL keep the options in `settings/options.json`, and SHALL load the
defaults with a warning when that file fails to parse.

#### Scenario: A kept option
- **WHEN** the player sets Names to Off and starts the game again
- **THEN** Names is still Off
