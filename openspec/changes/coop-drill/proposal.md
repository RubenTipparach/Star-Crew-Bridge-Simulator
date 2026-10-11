# Proposal: the first co-op drill, Helm and Tactical against one Hound

## Why

The owner, 2026-10-10: "deploy a multiplayer bot to both pis. Assume various stations on a starship and take down
an enemy ship. This mission should be simple if one player assumes helm and the other assumes tactical. This will
test out and harden the basic networking co-op combat scenario. One of the first many scenarios. Make sure to include
mission briefings and implement officer stations in client games. Whatever pi has more ram can be the server." And,
the same hour: "make enemy ship 3d we'll just use that as placeholder for now".

Nothing networked exists yet: `sc-net` is a doc comment and `sc-server` a 30 Hz loop with no state. Every later
scenario needs the same spine: a server that holds one authoritative ship and its enemies, clients that take
stations and send commands, snapshots that come back, and a mission that starts, ends and starts again. The smallest
scenario that exercises all of it with two people who must work together is one ship, one enemy and two stations.

## What Changes

- **A drill mission, `drill-hound`** (design section 1): the Tern meets one Hound corvette. Helm keeps it in the
  turrets' arcs and the tubes' cone and flies to spoil its aim; Tactical locks it, sets the turrets to fire, picks the
  shield facing and loads and fires Gannet missiles. With both stations played it is won in about two minutes; left to
  automation it is slow and can be lost. A briefing comes first and a debrief after, then the drill resets.
- **`sc-core::combat`** (section 2): the Tern's assisted flight, the Hound's, the twin pulse cannon's fire rule with
  the shared hit chance, six shield faces, Gannet missiles, and the mission's phases, at the 30 Hz tick, from data
  with units. A subset of `flight-and-navigation`, `weapons-and-shields` and `bridge-stations`, built to their
  numbers where they give them and marked placeholder where they do not.
- **`sc-core::automation`** (section 3): one function per station that turns the ship's state into the same commands
  a console sends. The server runs it for an empty seat at the automation's lower competence; a bot client runs it at
  a player's competence and sends what it decides over the network like a person would.
- **`sc-net`** (section 4): the messages (a binary codec with validation, one implementation for both ends) and the
  WebRTC transport `netcode-and-sessions` chose (`str0m`), with a LAN signalling endpoint on the server.
- **`sc-server`** runs the drill: seats, automation, snapshots at 20 Hz, a status page.
- **`sc-client`** joins a server (`--connect`), shows the briefing, puts you at your station's console (Helm or
  Tactical, the `bridge-stations` 8.0 layouts) over a 3D bow view where the Hound is a low-poly placeholder model, and
  shows the debrief. `--bot` lets the station's automation play it at a player's competence.
- **A two-Pi test** (section 6): the 8 GB Pi 5 hosts the server and the Helm bot, the 4 GB Pi 5 runs the Tactical
  bot; the run is logged, measured and recorded on video.

## Impact

- Builds the first slices of `netcode-and-sessions` (tasks 1.1, 1.2 partly, 1.4, 4.1, 4.2 partly, 5.1 partly),
  `flight-and-navigation` (full assist only), `weapons-and-shields` (pulse cannon, Gannet, shields, Hound) and
  `bridge-stations` (two consoles, automation for two stations). Those changes stay the designs; this one says what
  of them is built and where it simplifies.
- `lobby`: Join a crew takes a server address.
- The console layouts are `bridge-stations` 8.0, whose approval (B13) is still open: built because the owner asked
  for officer stations in the client, and changed when the owner answers.
