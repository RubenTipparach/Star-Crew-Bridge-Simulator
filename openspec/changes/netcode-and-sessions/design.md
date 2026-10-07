# Design: netcode and sessions

Status: **proposed** (2026-10-04). Numbers are estimates sized against the Pi 5 budget in
`engine-stack` (80 kbit/s down and 32 kbit/s up per client since 2026-10-07, section 4; the main
server on a 4 GB Pi 5);
the first networked build measures them. Re-floored the same day on the owner's correction:
"I'm sorry we're running on a pi5 1gb-4gb, 4 GB can be used as main server too".

**2026-10-07: WebRTC for every client.** The owner asked about browser playtest builds, then
"what if we stood up a matchmaking service on fly io? would that help unify web vs desktop
users?", then decided: "I want to use webrtc if posible to do multiplayer on web and desktop".
The transport (section 2) moved from plain UDP to WebRTC data channels for every client, native
and browser alike; the matchmaker that finds sessions and carries the connection setup is its
own change, `matchmaker`; the per-client network budget rose to pay for WebRTC's headers
(section 4).

References, cited for shape only: Glenn Fiedler's "Gaffer on Games" articles on UDP
reliability, snapshot interpolation and snapshot compression; Valve's "Source Multiplayer
Networking" article on interpolation delay, prediction and lag compensation; the Quake 3
network model (delta snapshots against the last acknowledged state). To verify against the
sources before the first networked build.

## 1. Topology

```text
                         matchmaker (Fly.io: session codes, connection setup, ICE servers)
                           ^   ^   ^                         ^
                 wss, join |   |   |                         | wss, kept open by the server
                           |   |   |                         |
   client (Pi 5 or PC)  \  |   |   |                         |
   client (Pi 5 or PC)   >----- WebRTC data channels -----  sc-server  (the main server:
   client (browser)     /    (direct, or through TURN)       |          a 4 GB Pi 5 on Ethernet)
                                                             +-- sc-core: the one authoritative ship
```

The matchmaker only introduces a client to a server. Game traffic goes between them directly,
or through a TURN relay that forwards encrypted bytes it cannot read; the matchmaker never sees
game state and decides nothing.

| Mode | Where the server runs | Use |
| --- | --- | --- |
| **Main server** | `sc-server`, headless, on a 4 GB Pi 5 (or any desktop) | **The normal way to play with friends.** Always on at home, on Ethernet, with the Active Cooler; holds the campaign saves; every player is a client like the others |
| Solo | A thread in the player's client | One player, automation holds the other stations. Fits a 1 GB Pi 5 (64 MB, `engine-stack` section 5) |
| Listen server | A thread in the host's client | A crew without a main server, from a client with 2 GB or more |

The main server on a 4 GB Pi 5 spends one A76 core on the simulation (at most 2 ms per ship per
30 Hz tick, `engine-stack` section 5) and about 0.5 Mbit/s up for eight players on gigabit
Ethernet. Both fit with room to spare; how many sessions one board can hold at once is a
measurement (task 6.1).

## 2. Transport

**WebRTC data channels, for every client** (owner, 2026-10-07: "I want to use webrtc if posible to
do multiplayer on web and desktop"). One transport means one connection path to test, one set of
channel semantics, and a browser player and a Pi player in the same crew on equal terms.

- **One peer connection per client, to the server.** The server holds up to eight. Clients never
  connect to each other.
- **Native ends** (the Pi client, desktops, `sc-server`) use `str0m`, a WebRTC library in Rust
  written sans I/O: it owns no socket and no thread, and is fed datagrams and the time by our
  network thread, which keeps the frame non-blocking (`engine-stack` section 6) and the memory
  ours to size. Version 0.24.1 (2026-10-03) has data channels (SCTP through `sctp-proto`) and
  pluggable crypto: its pure Rust backend (`rust-crypto`, DTLS by `dimpl`) needs no OpenSSL on the
  Pi; `aws-lc-rs` and OpenSSL are the alternatives. Which backend, and its cost on an A76 (which
  has the ARMv8 crypto extensions), is measured in the first networked build (task 1.1).
- **The browser** uses its own `RTCPeerConnection`, called from the WebAssembly through a small
  JavaScript module. `sc-net` has one transport interface with these two implementations, and
  that is the only fork: the channels, messages, acknowledgements and everything above are one
  code path (CLAUDE.md 6.1).
- **Setting up a connection** (ICE: STUN to find each end's public address, a direct path when
  one exists, a TURN relay otherwise) is carried by the `matchmaker` change; on a LAN with no
  internet, native clients do the same exchange with the server's own small endpoint
  (section 8).

**Channels.** Each of section 2's channels is a WebRTC data channel, negotiated up front by id so
neither end waits for the other to announce it:

| Channel | WebRTC settings | Delivery | Carries |
| --- | --- | --- | --- |
| Input | `ordered: false`, `maxRetransmits: 0` | Unreliable, newest wins; each message repeats the last 3 input frames | Avatar movement, fighter flight controls, turret aim |
| Snapshot | `ordered: false`, `maxRetransmits: 0` | Unreliable; our sequence number drops anything older than the newest applied | Server state deltas |
| Command | ordered, reliable | Reliable, ordered, by SCTP | Console actions, seat claims, door use, chat |
| Bulk | ordered, reliable, its own channel | Reliable, so a join's keyframe never queues ahead of commands | Join-in-progress state, the save at session end |

- **Our header** is carried only on the two unreliable channels: sequence number (2 bytes), latest
  received remote sequence (2) and a 32-bit acknowledgement bitfield (4), 8 bytes. Snapshot deltas
  need to know which snapshot the client holds, and SCTP does not tell an application what an
  unreliable channel delivered. The reliable channels need no header: SCTP delivers them in order.
  The old header's protocol id and session token are gone: DTLS authenticates the connection, and
  the protocol version is checked once, in the first command (section 8).
- **Message sizes.** An unreliable message stays at most 1,200 bytes, so it is one SCTP chunk in
  one datagram: SCTP drops a fragmented unreliable message whole if any fragment is lost. A
  reliable message is at most 16 KiB, a size every browser accepts; the bulk channel cuts larger
  transfers into 16 KiB messages.
- **Liveness:** WebRTC's own consent checks keep the path open; the server also drops a client
  silent for 5 s on the input channel, as before.
- **Validation:** every field is range-checked on arrival and non-finite numbers are rejected
  (CLAUDE.md 6.6). A malformed message is dropped and counted, never trusted.
- **What WebRTC costs per packet** (estimates, measured in task 2.3): IPv4 and UDP 28 bytes, a
  DTLS 1.2 record with AES-GCM 37, SCTP's common and DATA chunk headers 28, our header 8: about
  101 bytes, against the plain UDP design's 42. Section 4 carries the difference.

### 2a. WebRTC, considered (2026-10-06; decided for every client 2026-10-07)

*This section is the case as it was argued on 2026-10-06, kept as the record. The owner decided
WebRTC for every client the next day, and section 2 above is now the transport.*

The owner asked, 2026-10-06: "is webrtc any good?". WebRTC's data channels carry game traffic over
SCTP inside DTLS inside UDP (RFC 8831), and a channel can be unordered with no retransmission,
so both kinds of channel above fit it. Its connection setup, ICE (RFC 8445), finds a path through
home routers with STUN, and falls back to a TURN relay (RFC 8656) when no direct path exists. It
also encrypts every packet.

What it would cost here:
- **A signalling service** to exchange connection offers, so it needs something hosted anyway.
  That is M1's rendezvous service, plus STUN, plus a TURN relay for the routers no path gets through.
- **A large protocol stack** in a 1 GB client and in `sc-core`'s server, against the plain UDP
  layer of this section, about 1,000 lines (M2).
- **More overhead per packet** than this section's 14-byte header, from the DTLS and SCTP headers.

What it buys:
- **Browser clients.** A browser can only send game datagrams through WebRTC (or WebTransport).
- **No port forwarding,** through ICE and TURN.
- **Encryption.**

The topology needs little of that. Clients always connect out to the main server, so only the
server must be reachable: one forwarded port, or a port the server opens itself through the
router's UPnP or NAT-PMP. Players behind carrier-grade NAT need a relay whichever protocol is used.

**Recommendation taken (ask only with screenshots), M4, 2026-10-06; superseded 2026-10-07 by the
owner's decision for WebRTC:** keep plain UDP. Reach the main server
through LAN broadcast, a forwarded port, then UPnP or NAT-PMP (a later change), and then M1's
rendezvous and a relay. Choose WebRTC if browser clients become a goal. This section's
channels, at most 1,200-byte payloads with reliable and unreliable delivery, map onto WebRTC data
channels one to one, so that choice stays open.

## 3. Time and ticks

- The server ticks at 30 Hz and numbers its ticks. Ship systems step every third tick (10 Hz).
- Snapshots go out at 20 Hz. Each names its server tick.
- The client estimates the server's tick from snapshot arrival times, smoothed, and renders
  remote entities at `server tick - 100 ms` (two snapshot intervals), interpolating between
  the two snapshots around that time.

## 4. What a snapshot holds, and what it costs

State is split by frame and by how fast it changes. Rates are maximums; a delta that holds no
change for an item costs one bit. A console's own group goes only to a client whose seat shows
that console (`power-grid` section 14, `life-support` section 18). Rows marked as another change's
quote its figures (added or corrected 2026-10-04).

| Group | Frame | Items | Encoding | Rate | Bytes per send (changed) |
| --- | --- | --- | --- | ---: | ---: |
| Crew avatars | Interior | up to 8 | id 1 B; position 3 x int16 in cm (+/-327 m); yaw, pitch 2 x 8 bit; posture and animation 1 B | 20 Hz | 80 |
| Crew status (`crew-on-deck` section 14) | Interior | up to 8 | HP 8 bit, effects bitset 8 bit, held thing 8 bit, suit oxygen 8 bit | on change, at most 2 Hz | 32 (at most 64 B/s, 0.5 kbit/s) |
| Felt residual (`crew-on-deck` section 14) | Interior | 1, the compartment the client's own body is in | `F_k` as 3 x int8 in 0.2 m/s^2 steps; the client's prediction reads it | 20 Hz | 3 (60 B/s, 0.5 kbit/s) |
| Own ship pose | System | 1 | position 3 x f64 relative to the session's origin body; orientation smallest-three quaternion 32 bit; velocity 3 x f32; angular velocity 3 x int16 | 20 Hz | 48 |
| Other bodies near the ship | System, relative to own ship | up to 64 | id 2 B; position 3 x f32 relative to own ship; orientation 32 bit; velocity 3 x int16 | 20 Hz near, 5 Hz far | 20 each |
| Projectiles | System | | Not sent: spawned by a reliable "fired" event (shooter, time, direction) and simulated on the client | | 0; the fired and hit events cost about 4.5 kbit/s in a full engagement (`weapons-and-shields` section 15) |
| Doors, hatches, breakers, valves | Interior | about 120 | 1-2 bits each, as a bitset | on change | about 30 |
| Compartment air (`life-support` section 18) | Interior | 30, and the duct | pressure, oxygen, CO2, temperature, smoke: 5 x 2 B | the client's own compartment at 5 Hz to every client; every compartment to a client showing a compartment panel (damage control, engineering E5) at 1 Hz, and 5 Hz for those changing faster than 1 kPa/s | 10; 310 (about 2.5 kbit/s) |
| Lighting state (`power-grid` section 14) | Interior | 30 | 2 bits each (normal, red alert, emergency, dark) | on change | 8 |
| Power loads and buses (`power-grid` section 14) | | 40 loads, 24 edges, 14 nodes | each load's wanted and delivered share 2 x 8 bit, each edge's flow share 8 bit, reactor, battery and loop about 24 B, node health 8 bit; to a client showing an engineering console | 2 Hz | 142 (2.3 kbit/s) |
| Systems damage, fires, heat | | about 40 | 8 bit each | 2 Hz | 40 |
| Seats and stations | | 14 | operator 1 B (`bridge-stations` section 3: player slot, `AUTO` or `MERGED`), occupant 1 B | on change | 28 |

**Budget check** (redone 2026-10-04 with the crew status and felt residual of `crew-on-deck`, the
operator byte of `bridge-stations`, the air and power groups at `life-support`'s and `power-grid`'s
own figures, and `weapons-and-shields`' events; it read 57 kbit/s down without them). A full
keyframe is about 2.2 kB, sent only on join and when acknowledgement is lost for 1 s. A typical
delta, with eight avatars moving, a dozen bodies near and the systems groups on their slower rates,
was about 330 bytes. For the busiest client, one seated at an engineering console, the air and
power groups now average 32 B a snapshot (against 45 B for the first table's 5 Hz groups), and
crew status and the felt residual add 6 B: about 324 bytes plus 28 bytes of header, 352 B x 20 Hz
= 7.0 kB/s = **56 kbit/s down**. In a full engagement the fired and hit events add about
4.5 kbit/s: **about 61 kbit/s**, inside the 64 kbit/s budget with about 3 kbit/s (5%) to spare. A
client away from the engineering and damage control consoles takes about 5 kbit/s less. The margin
is thin, so the first networked build measures it (task 2.3); if it runs over, the bodies group
gives first, being about half of the delta, by sending far bodies less often. Input up: 4 repeated
input frames of 12 bytes plus header at 30 Hz is 2.3 kB/s = **18 kbit/s**, just over the 16 kbit/s
budget, so input is sent at 20 Hz with 3 repeats (**11 kbit/s**); a seated player's console
commands add about 1 kbit/s (`bridge-stations` section 12): **12 kbit/s up**. The first networked
build measures both.

**Redone for WebRTC, 2026-10-07.** With about 101 bytes of headers a packet (section 2) in place
of 28, a snapshot is about 425 bytes: 425 B x 20 Hz = 8.5 kB/s = **68 kbit/s down**, and **about
73 kbit/s** in a full engagement. Up, an input message of 3 frames (36 B) with 101 B of headers at
20 Hz is 2.7 kB/s = 22 kbit/s; SCTP's acknowledgements of the snapshots ride in the same
datagrams (about 16 B each, 2.6 kbit/s), and commands add about 1 kbit/s: **about 26 kbit/s up**.
That is over the 64 and 16 kbit/s budget, so the budget moves rather than the design: **80 kbit/s
down and 32 kbit/s up per client** (`engine-stack` section 5, the one source, changed in the same
commit). On any connection a player has today that is nothing, and the main server sends about
0.6 Mbit/s to eight players. The headers are the price of encryption, NAT traversal and browser
players.

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
- **Finding a game** (revised 2026-10-07 for WebRTC):
  - **By code, from anywhere:** the main server registers with the matchmaker on Fly.io and gets a
    six-character join code; a player in a browser or on a desktop types it, and the matchmaker
    carries the connection setup (`matchmaker`). No port forwarding. This is the normal way in.
  - **On a LAN, with or without internet:** a UDP broadcast every second lists servers to native
    clients, and they set up the connection through the server's own small signalling endpoint
    (the same messages as the matchmaker's, over a WebSocket on the LAN). A browser cannot reach a
    LAN address from a page served over HTTPS, so browser players always come through the
    matchmaker.
  - **By address** (native clients): the server's signalling endpoint and its WebRTC port, both
    forwarded, for a host who wants no third party involved.
- **Version check:** the first command is a hello carrying the protocol version, the ship's
  layout digest and the data digest. The matchmaker checks them before introducing anyone (an early
  refusal); the server checks again, because it is the authority.
- **Join in progress:** the joining client receives a keyframe over the bulk channel (about 2.2 kB
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
| Down per client | 68 kbit/s typical, about 73 kbit/s in a full engagement, with WebRTC's headers (section 4; 56 and 61 kbit/s on plain UDP) | 80 kbit/s (64 until 2026-10-07) |
| Up per client | about 26 kbit/s (inputs 22, SCTP acknowledgements 3, console commands 1) | 32 kbit/s (16 until 2026-10-07) |
| Server up, 8 clients | about 0.58 Mbit/s in a full engagement | (Pi 5 gigabit Ethernet) |
| Server CPU for encryption, 8 clients | about 4,000 DTLS records a second; to measure with `str0m`'s crypto backend on an A76 (task 1.1) | inside the networking millisecond below, to verify |
| Server CPU for networking, 8 clients | under 1 ms per tick (delta encoding of about 2 kB per client) | beside the simulation's 2 ms per ship, in the 33 ms tick (`engine-stack` section 5; corrected 2026-10-04 from "the 4 ms tick", which that table does not have) |
| Server memory for history | 300 ms of 64 bodies at 30 Hz, about 40 kB; per-client acknowledged baselines, 8 x 2 kB | inside 64 MB |
| Client memory | snapshot buffer of 1 s, about 40 kB | inside 384 MB |

## Open questions

Per CLAUDE.md section 13 these take the recommendation; none has anything to look at yet. M4 the
owner decided in chat.

| # | Question | Options | Recommendation | Status |
| --- | --- | --- | --- | --- |
| M1 | Internet play without port forwarding needs a rendezvous service someone hosts. The 4 GB Pi 5 main server is always on. | Port forwarding only / a rendezvous service on the main server later / a relay / a matchmaker on Fly.io (the owner asked, 2026-10-07) | A matchmaker on Fly.io, the `matchmaker` change: browsers need a signalling service anyway, and a home Pi cannot be one for them | Recommendation taken (ask only with screenshots), 2026-10-07; port forwarding and LAN first until then |
| M2 | Our own UDP layer or a crate such as `renet`. Since 2026-10-07: our own channel layer over WebRTC. | Ours / renet | Ours, now smaller: SCTP gives reliability, ordering and fragmentation, so ours is the 8-byte header on the unreliable channels, acknowledgement tracking for snapshot baselines and the channel mapping, a few hundred lines; `renet` does not run over WebRTC | Recommendation taken (ask only with screenshots) |
| M4 | Plain UDP or WebRTC data channels (section 2a; the owner asked "is webrtc any good?", 2026-10-06). | UDP / WebRTC / both, WebRTC for browsers | UDP, with WebRTC only if browser clients become a goal (2026-10-06) | **Decided 2026-10-07 by the owner: WebRTC for every client**, "I want to use webrtc if posible to do multiplayer on web and desktop" (section 2) |
| M3 | Maximum players. Eight seats with work for each exist on the Tern (four core, captain, comms, flight ops, gunners, pilots). | 8 / more | 8 | Recommendation taken (ask only with screenshots) |
