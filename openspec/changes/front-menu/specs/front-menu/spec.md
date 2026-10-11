# front-menu

## ADDED Requirements

### Requirement: The game opens on four buttons

The client SHALL open on a menu whose first buttons are, in order, Single Player, Multiplayer, Options and Quit.

#### Scenario: Starting the game
- **WHEN** the game starts without a command line that names a mode
- **THEN** the menu shows Single Player, Multiplayer, Options and Quit, in that order, with Single Player focused

### Requirement: Each button leads where it says

Single Player SHALL open the officer page whose main action starts a single-player game; Multiplayer SHALL open the
officer page whose main action joins a server by address; Options SHALL open the options; Quit SHALL close the game.

#### Scenario: Back from a page
- **WHEN** the player is on the Single Player page and presses Back or Esc
- **THEN** the four buttons show again

### Requirement: Options are kept and repaired

The client SHALL apply a changed option at once, keep it in `settings/options.json`, and load the defaults with a
warning when that file fails to parse.

#### Scenario: A damaged options file
- **WHEN** `settings/options.json` holds text that is not the options
- **THEN** the client starts with the defaults and logs a warning
