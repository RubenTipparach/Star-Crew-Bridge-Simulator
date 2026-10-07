# Netcode

## Purpose

How clients and the authoritative server exchange state and intent: the transport, snapshots,
prediction, commands, seats and sessions.

## ADDED Requirements

### Requirement: One authoritative server per ship
The server SHALL be the only process that changes ship state. Clients SHALL send intents and
render the state the server sends, predicting only their own avatar and a craft they fly. A solo
game SHALL run the same server with one client.

#### Scenario: Two engineers trip the same breaker
- **WHEN** two clients send "open breaker 7" in the same tick
- **THEN** the server applies them in seat order, the second is a no-op, and both clients render the breaker open from the next snapshot

#### Scenario: Solo play
- **WHEN** one player starts a mission alone
- **THEN** a listen server runs in their client and automation holds the other stations

### Requirement: Commands are validated by the server and previewed by the same rule
The server SHALL apply a console command only if the sending seat holds a station allowed to
issue it, SHALL clamp its value to the legal range, and SHALL resolve it with the same `sc-core`
function the client used to preview it. A refused command SHALL be answered with a reason.

#### Scenario: A gunner tries to set power
- **WHEN** a client seated as the dorsal gunner sends a power allocation command
- **THEN** the server refuses it and the client shows the reason

### Requirement: Snapshots fit the network budget
Server snapshots SHALL be sent at 20 Hz, delta-compressed against the last snapshot the client
acknowledged, and SHALL stay within the Pi 5 budget's downstream allocation per client with
eight players aboard.

#### Scenario: A full crew in combat
- **WHEN** eight avatars move and twelve bodies are near the ship for 60 s
- **THEN** the measured average downstream rate per client, headers included, is at most 80 kbit/s

### Requirement: Seats belong to the server and never freeze
The server SHALL own the seat table. When a player leaves a seat or disconnects, the station
SHALL pass to automation or its merge target on the next tick, and the player's avatar SHALL
remain aboard as an NPC.

#### Scenario: The helm disconnects mid-turn
- **WHEN** the client holding the helm disconnects while steering hard to port
- **THEN** helm automation takes the seat on the next tick and the ship does not keep turning on the last input

### Requirement: Every client connects over WebRTC
Every client, native or in a browser, SHALL connect to the server with one WebRTC peer connection
carrying four data channels (input and snapshot unreliable and unordered, command and bulk reliable
and ordered), and the channel layer above the transport SHALL be the same code for both kinds of
client.

#### Scenario: A browser player and a Pi player in one crew
- **WHEN** a player in a browser and a player on a 1 GB Pi 5 join the same session
- **THEN** both connect through WebRTC data channels, and the server cannot tell their messages apart except by the client kind each declares in its hello

#### Scenario: An unreliable message that would fragment
- **WHEN** a snapshot delta would exceed 1,200 bytes
- **THEN** it is sent as more than one message, each under 1,200 bytes, never as one fragmented message

### Requirement: Packets are validated before use
Every received field SHALL be range-checked and non-finite numbers SHALL be rejected; a packet
that fails SHALL be dropped and counted without changing any state.

#### Scenario: A corrupt snapshot
- **WHEN** a snapshot arrives with a NaN in a body's velocity
- **THEN** the client drops it, counts it, and keeps interpolating from the previous snapshots

### Requirement: Joining checks versions
A client joining a session SHALL be refused, with a message naming the mismatch, unless its
protocol version, the ship's layout digest and the data digest match the server's.

#### Scenario: An old client
- **WHEN** a client built before a change to `layout.json` joins
- **THEN** the join is refused with a message naming the layout digest
