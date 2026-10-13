# single-player

## ADDED Requirements

### Requirement: The browser hosts single player

Beam aboard in the browser build SHALL start the same single-player game as the desktop: the drill server, a bot crew
member at every station but the player's, and the player's session, all in the page.

#### Scenario: Beam aboard on itch.io
- **WHEN** the player opens the browser build, chooses Single Player at Helm and presses Beam aboard
- **THEN** the page hosts the drill, a crew member walks to and sits at each other station, and a round can be played
  to its end with them

#### Scenario: Played on foot in the browser
- **WHEN** a single-player game starts in the browser
- **THEN** the player is a body on the ship, walks to their station, and its console opens only once they sit at it

### Requirement: A single-player game connects through a local link

A single-player game's sessions SHALL reach its server through a link inside the process that opens no socket and
starts no thread, and that delivers each channel's messages in the order they were sent with none lost.

#### Scenario: A server and a client on a local link
- **WHEN** a client connects to a server over a local link and each sends messages on all three channels
- **THEN** both ends report the connection open, each receives the other's messages in the order sent, and no thread
  or socket was created

### Requirement: A paused game drops no one

The drill server SHALL measure a peer's silence in its own ticks, so a pause of the whole game, such as a hidden
browser tab that runs no frames, drops no peer.

#### Scenario: A hidden tab
- **WHEN** a single-player game in the browser is hidden for 30 s and shown again
- **THEN** the player and every crew member are still aboard, and the round goes on from where it stopped
