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
- **A Pi 5 at either end.** A 1 GB Pi 5 is the client floor, often on Wi-Fi; a 4 GB Pi 5 is
  the main server (owner, 2026-10-04: "4 GB can be used as main server too").

## What Changes

- **Client and server.** `sc-server` is the authority. Normally it runs headless as the main
  server on a 4 GB Pi 5; a host can also run it inside their client (a listen server). A solo
  game is a listen server with one client.
- **WebRTC data channels, for every client** (owner, 2026-10-07: "I want to use webrtc if posible
  to do multiplayer on web and desktop"; plain UDP until then). Unreliable unordered channels for
  snapshots and input, carrying our acknowledgements, and reliable ordered channels for commands
  (open this door, set this allocation, claim this seat) and bulk transfers. `str0m` on native
  ends, the browser's own WebRTC in a browser; one channel layer above both.
- **Snapshots at 20 Hz**, delta-compressed against the last snapshot the client acknowledged,
  with ship systems sent at lower rates by priority, inside 80 kbit/s down per client (64 until
  WebRTC's headers, 2026-10-07).
- **Prediction for what a player drives:** their own avatar and a fighter they fly. Everything
  else is interpolated 100 ms in the past.
- **Commands are intents, validated by the server.** A console sends "set shields to 120%"; the
  server checks the seat, applies it through the same `sc-core` function the console previewed
  with, and the result comes back in a snapshot.
- **Seats are server state.** Claiming, releasing and automation hand-over happen on the server,
  so a disconnect hands the seat to automation instead of freezing it (star-crew-64's bug).
- **Sessions:** join by a code through the matchmaker on Fly.io (the `matchmaker` change), on a LAN
  with discovery, or by address; join in progress with a full state transfer; up to eight
  players.

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
