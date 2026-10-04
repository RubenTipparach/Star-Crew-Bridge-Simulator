# Proposal: an authoritative server, predicted crew, and sessions friends can join

## Why

The owner, 2026-10-04: "A bridge simulator game that can be played with friends online. We'll
aim for a core of 4 players, possibly more can join." A bridge simulator is many people
operating one machine. If two clients disagree about whether the breaker is closed, the ship
has two states and the game is broken. So one process owns the ship, every other process asks
it to do things, and everybody renders what it says.

Three things make this harder than a shooter:

- **Two frames.** Crew walk the ship's interior while the ship moves through space. Avatars are
  sent in the interior frame and ships in the system frame, and a craft crosses between them
  (`ship-frames`).
- **A lot of slowly changing state.** Thirty compartments of air, twenty-five power loads,
  breakers, doors, valves, damage. Most of it changes slowly and must arrive reliably in the
  end; little of it must arrive at once.
- **A Pi 3 at either end.** A Pi may be a client, a listen server or both, on 2.4 GHz Wi-Fi.

## What Changes

- **Client and server.** `sc-server` is the authority; a host can run it inside their client
  (a listen server) or as a dedicated headless process on any machine, a Pi 3 included. A solo
  game is a listen server with one client.
- **UDP with channels.** An unreliable sequenced channel for snapshots and input, and a
  reliable ordered channel for commands (open this door, set this allocation, claim this seat),
  with acknowledgements carried in every packet.
- **Snapshots at 20 Hz**, delta-compressed against the last snapshot the client acknowledged,
  with ship systems sent at lower rates by priority, inside 64 kbit/s down per client.
- **Prediction for what a player drives:** their own avatar and a fighter they fly. Everything
  else is interpolated 100 ms in the past.
- **Commands are intents, validated by the server.** A console sends "set shields to 120%"; the
  server checks the seat, applies it through the same `sc-core` function the console previewed
  with, and the result comes back in a snapshot.
- **Seats are server state.** Claiming, releasing and automation hand-over happen on the server,
  so a disconnect hands the seat to automation instead of freezing it (star-crew-64's bug).
- **Sessions:** host on LAN with discovery, or by address and port over the internet; join in
  progress with a full state transfer; up to eight players.

## Capabilities

### New Capabilities

- `netcode`: the transport, the channels, snapshots, prediction, commands, seats and sessions.

### Modified Capabilities

None.

## Impact

- `sc-net` holds the message schema used by both ends, so the client and the server cannot
  disagree about a packet's layout.
- `sc-core` exposes deterministic avatar and craft movement functions shared by server and
  client prediction.
- `bridge-stations` defines what each command does; this change defines how it travels.
- No code exists yet. Nothing here is built until the owner asks for it.
