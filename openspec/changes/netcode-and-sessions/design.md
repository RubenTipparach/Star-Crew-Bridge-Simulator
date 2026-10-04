# Design: netcode and sessions

Status: **proposed** (2026-10-04). Numbers are estimates sized against the Pi 3 budget in
`engine-stack` (64 kbit/s down and 16 kbit/s up per client); the first networked build
measures them.

References, cited for shape only: Glenn Fiedler's "Gaffer on Games" articles on UDP
reliability, snapshot interpolation and snapshot compression; Valve's "Source Multiplayer
Networking" article on interpolation delay, prediction and lag compensation; the Quake 3
network model (delta snapshots against the last acknowledged state). To verify against the
sources before the first networked build.

## 1. Topology

```text
   client (Pi or PC)  \
   client (Pi or PC)   >---- UDP ----  sc-server  (inside the host's client, or headless)
   client (Pi or PC)  /                   |
                                          +-- sc-core: the one authoritative ship
```

| Mode | Where the server runs | Use |
| --- | --- | --- |
| Solo | A thread in the player's client | One player, automation holds the other stations |
| Listen server | A thread in the host's client | Friends on a LAN or the internet; the host's machine carries the server |
| Dedicated | `sc-server` on any machine, a Pi 3 included | A crew that wants the host to be a player like the others, or a home server |

A Pi 3 hosting a listen server for eight players spends one core on the server (at most 4 ms
per 30 Hz tick, `engine-stack` section 5) and about 0.5 Mbit/s up. Both fit.

## 2. Transport

- **UDP**, one socket per process. Payloads at most 1,200 bytes so a packet never fragments on
  the internet's common paths.
- **Every packet** carries: protocol id and version (4 bytes), session token (4), sequence
  number (2), latest received remote sequence (2) and a 32-bit acknowledgement bitfield, then
  messages.
- **Channels:**

| Channel | Delivery | Carries |
| --- | --- | --- |
| Input | Unreliable, newest wins; each packet repeats the last 4 input frames | Avatar movement, fighter flight controls, turret aim |
| Snapshot | Unreliable, sequenced | Server state deltas |
| Command | Reliable, ordered, resent until acknowledged | Console actions, seat claims, door use, chat |
| Bulk | Reliable, fragmented and reassembled | Join-in-progress state, the save at session end |

- **Connection:** a challenge and response with the protocol version and a session token
  (cheap protection against spoofed packets); keep-alives at 4 Hz when idle; a timeout of 5 s.
- **Validation:** every field is range-checked on arrival and non-finite numbers are rejected
  (CLAUDE.md 6.6). A malformed packet is dropped and counted, never trusted.

## 3. Time and ticks

- The server ticks at 30 Hz and numbers its ticks. Ship systems step every third tick (10 Hz).
- Snapshots go out at 20 Hz. Each names its server tick.
- The client estimates the server's tick from snapshot arrival times, smoothed, and renders
  remote entities at `server tick - 100 ms` (two snapshot intervals), interpolating between
  the two snapshots around that time.

## 4. What a snapshot holds, and what it costs

State is split by frame and by how fast it changes. Rates are maximums; a delta that holds no
change for an item costs one bit.

| Group | Frame | Items | Encoding | Rate | Bytes per send (changed) |
| --- | --- | --- | --- | ---: | ---: |
| Crew avatars | Interior | up to 8 | id 1 B; position 3 x int16 in cm (+/-327 m); yaw, pitch 2 x 8 bit; posture and animation 1 B | 20 Hz | 80 |
| Own ship pose | System | 1 | position 3 x f64 relative to the session's origin body; orientation smallest-three quaternion 32 bit; velocity 3 x f32; angular velocity 3 x int16 | 20 Hz | 48 |
| Other bodies near the ship | System, relative to own ship | up to 64 | id 2 B; position 3 x f32 relative to own ship; orientation 32 bit; velocity 3 x int16 | 20 Hz near, 5 Hz far | 20 each |
| Projectiles | System | | Not sent: spawned by a reliable "fired" event (shooter, time, direction) and simulated on the client | | 0 |
| Doors, hatches, breakers, valves | Interior | about 120 | 1-2 bits each, as a bitset | on change | about 30 |
| Compartment air | Interior | 30 | pressure, O2 fraction, CO2, temperature: 4 x 8 bit quantized | 5 Hz | 120 |
| Power loads and buses | | about 30 | supply ratio 8 bit, draw 8 bit | 5 Hz | 60 |
| Systems damage, fires, heat | | about 40 | 8 bit each | 2 Hz | 40 |
| Seats and stations | | 14 | occupant 1 B | on change | 14 |

**Budget check.** A full keyframe is about 1.9 kB, sent only on join and when acknowledgement
is lost for 1 s. A typical delta, with eight avatars moving, a dozen bodies near and the systems
groups on their slower rates, is about 330 bytes plus 28 bytes of header: 358 B x 20 Hz = 7.2
kB/s = **57 kbit/s down**, inside the 64 kbit/s budget. Input up: 4 repeated input frames of 12
bytes plus header at 30 Hz is 2.3 kB/s = **18 kbit/s**, just over the 16 kbit/s budget, so input
is sent at 20 Hz with 3 repeats (**11 kbit/s**). The first networked build measures both.

## 5. Prediction, interpolation and correction

| Thing | Who renders it how |
| --- | --- |
| My avatar | Predicted from my unacknowledged inputs through `sc-core`'s movement function; corrected by replaying inputs from the server's last acknowledged position. A correction under 5 cm is blended over 100 ms; above that it snaps. |
| My fighter | The same, with `sc-core`'s craft flight function. |
| Other avatars, ships, craft, missiles | Interpolated 100 ms in the past between snapshots. If a snapshot is late, extrapolate up to 100 ms, then hold. |
| Doors, breakers, consoles | Shown as the server says. A console shows a pending state ("closing") the moment the player acts, from the same preview function the server will apply. |
| Ship systems readouts | Interpolated between their 5 Hz values, so bars move smoothly. |

The interior frame makes avatars cheap: the decks do not move, so avatar positions are small
integers, and a ship's violent manoeuvre does not make crew rubber-band.

## 6. Commands

Every console action is a command: `{ seat, kind, target id, value, client tick }`.

1. The client previews it with the `sc-core` function that will resolve it (CLAUDE.md 6.1) and
   shows the pending result.
2. The server receives it on the reliable channel, checks the seat holds the station that may
   issue it, clamps the value to its legal range, and applies it at the start of the next tick.
3. The result appears in the next snapshot that covers the item. A refused command comes back
   as a reliable "refused" message with a reason the console shows.

Commands from different players are applied in a stable order each tick: by seat id, then by
arrival (CLAUDE.md 6.4).

**Lag compensation for gunners.** A manned turret's shot is judged against the target where the
gunner saw it: the server keeps 300 ms of history for exterior bodies and rewinds to the
gunner's view time, capped at 200 ms. This is a co-operative game, so it favours the shooter.

## 7. Seats and automation

- The server holds the seat table: station id to player id or automation.
- **Claim:** a player walks to a seat and uses it, or picks a station from the mess console. The
  server grants it if the seat is free or held by automation, and hands automation's state over
  (for example the current turret target).
- **Release:** standing up, choosing another station, or disconnecting returns the seat to
  automation on the next tick. The avatar of a disconnected player stays where it was, as an
  NPC, and keeps its injuries.
- **Merged stations** (a station folded into another console when nobody holds it) are rules in
  `bridge-stations`; the seat table records them.

## 8. Sessions

- **The mess is the lobby.** Players spawn there and claim stations; the host starts the
  mission.
- **Finding a game:** a LAN broadcast every second lists hosts; a host can also be joined by
  address and UDP port (the host forwards one port on their router). A small rendezvous service
  for NAT hole punching and session codes is a later change.
- **Join in progress:** the joining client receives a keyframe over the bulk channel (about 2 kB
  of state plus the ship's id and versions; the client loads its own copy of the compiled ship),
  then deltas. A version mismatch in the protocol, the ship's layout digest or the data digest
  refuses the join with a message naming which.
- **Players:** up to 8; the four core stations are offered first.
- **Saves:** the server writes the campaign state (ship damage, consumables, crew injuries) at
  mission end and on host request, versioned, through a temporary file and a rename (CLAUDE.md
  7, "A ship persists").

## 9. Determinism and replays

The game is not lockstep: clients render snapshots, not their own simulation. But `sc-core` is
deterministic for a given seed and command stream, so the server can record commands and
replay a mission to the same state hash. That is how simulation bugs are reproduced and how
tests pin the systems' behaviour.

## 10. Costs

| Cost | Estimate | Budget |
| --- | ---: | ---: |
| Down per client | 57 kbit/s | 64 kbit/s |
| Up per client | 11 kbit/s | 16 kbit/s |
| Server up, 8 clients | 0.46 Mbit/s | (Pi 3 Ethernet 100 Mbit/s) |
| Server CPU for networking, 8 clients | under 1 ms per tick (delta encoding of about 2 kB per client) | inside the 4 ms tick |
| Server memory for history | 300 ms of 64 bodies at 30 Hz, about 40 kB; per-client acknowledged baselines, 8 x 2 kB | inside 64 MB |
| Client memory | snapshot buffer of 1 s, about 40 kB | inside 256 MB |

## Open questions

Per CLAUDE.md section 13 these take the recommendation; none has anything to look at yet.

| # | Question | Options | Recommendation | Status |
| --- | --- | --- | --- | --- |
| M1 | Internet play without port forwarding needs a rendezvous service someone hosts. | Port forwarding only / a rendezvous service later / a relay | Port forwarding and LAN first; rendezvous as a later change | Recommendation taken (ask only with screenshots) |
| M2 | Our own UDP layer or a crate such as `renet`. | Ours / renet | Ours: about 1,000 lines, and it must match the snapshot design exactly | Recommendation taken (ask only with screenshots) |
| M3 | Maximum players. Eight seats with work for each exist on the Tern (four core, captain, comms, flight ops, gunners, pilots). | 8 / more | 8 | Recommendation taken (ask only with screenshots) |
