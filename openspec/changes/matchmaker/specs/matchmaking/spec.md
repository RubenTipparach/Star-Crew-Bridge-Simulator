# Matchmaking

## Purpose

How a player finds a session and how the matchmaker sets up the WebRTC connection between a
client and a server, without holding any game state.

## ADDED Requirements

### Requirement: A session is joined by code without port forwarding
A server SHALL register with the matchmaker and receive a six-character join code, and a client,
native or in a browser, SHALL be able to join that session by the code alone, with no port
forwarded on the server's router.

#### Scenario: A browser player joins a home server
- **WHEN** a main server on a home Pi 5 with no forwarded ports is registered, and a player types its code into the browser build
- **THEN** the client's data channels to the server open, directly or through TURN, and the player appears in the mess

#### Scenario: A wrong code
- **WHEN** a client sends a code no session holds
- **THEN** the matchmaker refuses the join with a reason, and no server is contacted

### Requirement: The matchmaker holds no game state
The matchmaker SHALL NOT run the simulation, store any session data on disk, or carry game traffic
other than through a TURN relay, and a running mission SHALL NOT depend on it.

#### Scenario: The matchmaker restarts mid-mission
- **WHEN** the matchmaker's Machine restarts while a crew of four is in a mission
- **THEN** the mission continues without a dropped connection, and the server re-registers with the same code once the matchmaker is back

### Requirement: Versions are checked before an introduction
The matchmaker SHALL refuse a join whose protocol version, layout digest or data digest differs
from the registered session's, naming the mismatch, before relaying any offer; the server SHALL
still check them itself.

#### Scenario: An old client
- **WHEN** a client built before a change to `layout.json` sends a join with the right code
- **THEN** the matchmaker refuses it naming the layout digest, and the server never sees an offer

### Requirement: Codes resist guessing
Join codes SHALL be drawn from a secure random source, and the matchmaker SHALL limit join attempts
per address per minute and per day, at limits read from its validated config.

#### Scenario: Guessing codes
- **WHEN** one address sends 11 join attempts within a minute
- **THEN** the eleventh is refused as rate limited without being checked against any code

### Requirement: TURN credentials are short-lived
Each join SHALL receive its own TURN credentials, valid for at most 4 hours, and no long-lived TURN
secret SHALL reach a client or the repository.

#### Scenario: A credential after its time
- **WHEN** a client uses a TURN credential 4 hours and 1 minute after its join
- **THEN** the TURN service refuses a new allocation with it
