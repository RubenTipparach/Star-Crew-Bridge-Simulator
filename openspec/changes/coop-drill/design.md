# Design: the first co-op drill

Status: **being built** (2026-10-10). The numbers come from `flight-and-navigation` (FN), `weapons-and-shields` (WS)
and `bridge-stations` (BS) where those changes give them; a number marked *placeholder* is this drill's own, in data,
and is replaced when its owning change is built. Progress and findings are in `log.md` beside this file.

## 1. The drill

`data/missions/drill-hound.json`. One ship, one enemy, two stations that matter.

| Phase | What happens | Ends |
| --- | --- | --- |
| Muster | Players join and claim stations; every client shows the briefing; the server holds the ship still | Every station held by a player is ready, and at least one player is aboard |
| Countdown | 5 s, shown on every screen | Timer |
| Engage | The Hound appears 6 km ahead and attacks | The Hound's hull reaches 0 (victory), the Tern's does (defeat), or 600 s pass (the Hound withdraws: defeat) |
| Debrief | Result, time, shots and hits by station; 20 s | Timer, then a new Muster with the next seed |

The drill repeats until the server stops, so a soak test is the same binary left running.

**Why it needs both stations.** The turrets fire only at a locked target, each by its mode (AUTO or TARGET; HOLD and
PD hold fire; Tactical sets them), and only the turrets whose arc holds the target fire (section 2.3): each of the
layout's four mounts (dorsal, ventral, port, starboard) bears on the half of the sky it faces, which Helm arranges. Missiles leave the bow tubes and their seeker takes a target only
within 30 deg of the bow, which Helm arranges, once Tactical has locked, loaded and armed. The Hound's aim error grows
with how fast the Tern crosses its line of sight (2.3), which Helm makes happen. Automation does none of the helm's
part (BS:143: it holds heading and speed and never flies evasive) and never fires missiles (BS:144), so an empty
station costs the crew most of what it adds.

## 2. `sc-core::combat`

Steps at the server's 30 Hz tick in `f64`, in the system frame of the drill (one origin; positions within 20 km).
Randomness is `Rng::for_purpose(session seed, entity id, purpose)`. Order of work in a tick: commands (by seat, then
arrival), automation, flight, weapons, projectiles, damage, mission.

### 2.1 Flight (FN section 4, full assist only)

Both ships fly by the same function from their own `flight` block. A helm intent is a forward speed set point and
three stick axes in [-1, 1].

- Forward speed approaches the set point with `tau_v`, the change per second limited to `accel_fwd` (up) and
  `accel_rev` (down). The velocity's sideways part decays with the same `tau_v` (full assist cancels drift), toward
  the strafe set points when the helm gives them (lateral and vertical, each held to `strafe_max_mps`, 50 m/s; added
  by `console-parity`). FN 4's RCS clamp on sideways acceleration is not built: it cut the bots' wins from 8 of 8
  seeds to 2.
- Each body rate approaches `stick x rate_limit` with `tau_w`, limited to the axis's angular acceleration.
- The attitude quaternion integrates the body rates (semi-implicit Euler, normalized every tick).

| Tern (`data/ships/tern/flight.json`, FN) | Value |
| --- | ---: |
| Speed set point | -100 to +400 m/s |
| `accel_fwd`, `accel_rev` | 15, 5 m/s^2 |
| `tau_v`, `tau_w` | 1.0, 0.25 s |
| Rate limits yaw, pitch, roll | 18, 18, 36 deg/s |
| Angular acceleration yaw, pitch, roll | 12, 12, 30 deg/s^2 |

The Hound's block is in `data/enemies.json` (*placeholder*: 300 m/s, 20 m/s^2, 30 deg/s).

Helm orders (BS section 7, the subset): **Target** (bow on target: the stick is taken by a rate controller that turns
the bow to the target, until the pilot moves the stick), **Level** (roll and pitch to zero), **All stop** (set point
0). The flight rule, not the order, decides how fast it happens.

Added by `console-parity` (its design 6), for the approved Helm console: an **attitude order** (heading, pitch and roll
from the thumbwheels, slewed on the shortest single axis at the rate and acceleration limits and held inside
`held_deg`, FN 6a; GO shows the slew's time from the same function; FLIP is the reciprocal heading) and the
**autopilot** HOLD, COURSE (bow on the mission's waypoint, `waypoint_m`), CHASE (bow on the target; the old Target
order), MATCH (the target's course and speed) and EVADE (set points up to `jink_mps` either way every
`jink_period_s`, seeded from the round).

### 2.2 Shields and hull (WS section 9)

Six faces: bow (+Z), stern (-Z), port (+X), starboard (-X), dorsal (+Y), ventral (-Y), ship-local. A hit is taken by
the face whose axis is nearest the direction from the ship's centre to the hit point. The face absorbs what it holds;
the rest reaches the hull. Faces regenerate at the ship's total regeneration rate, shared by the preset's weights, up
to each face's cap. A preset sets the caps (`data/ships/tern/shields.json`):

| Preset | Bow | Stern | Port | Starboard | Dorsal | Ventral |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Balanced | 40 | 40 | 40 | 40 | 40 | 40 |
| Bow | 80 | 15 | 36 | 36 | 36 | 37 |
| Stern | 15 | 80 | 36 | 36 | 36 | 37 |
| Port | 36 | 36 | 80 | 15 | 36 | 37 |
| Starboard | 36 | 36 | 15 | 80 | 36 | 37 |

(MJ; every row sums to 240 MJ, WS's capacity; the 15-80 MJ clamp is WS's.) Changing preset clamps each face to its
new cap at once; what was cut is lost (WS's 10 MJ/s shunt is later). Regeneration 3 MJ/s (WS: `0.25 x 12 MW`).
Tern hull 120 MJ (*placeholder*: hull damage is `damage-control`'s). Hound: six 50 MJ faces, hull 150 MJ (WS),
regeneration 1.5 MJ/s (*placeholder*).

### 2.3 Turrets and the hit chance (WS sections 1-4)

The twin pulse cannon (`data/weapons.json`, WS): 4 bolts/s, 1,500 m/s plus the ship's velocity, 1.6 s life (2,400 m),
1.2 MJ a bolt, a capacitor of 24 MJ per turret charged at 4 MW, 1.5 MJ drawn per bolt (12 s at full rate, then the
charge rate). Heat is below.

- **Mounts and arcs** (as built by `console-parity`, design 6): the Tern's turrets are the layout's four mounts,
  `turret_dorsal`, `turret_ventral`, `turret_port` and `turret_stbd`, at their `centre_m` (a test holds
  `data/ships/tern/combat.json` to `layout.json`). Each bears on the half of the sky its mount faces, from
  `arc_overlap_deg` past the mount's plane: the *placeholder* for WS's hull mask. The Hound has two that bear
  everywhere. (First built as two dorsal and two ventral turrets.)
- **Modes** (WS 7): AUTO and TARGET fire at the locked target (the drill has one hostile, so they act alike); PD fires
  only at inbound missiles, so it holds fire here (the Hound carries none); HOLD holds fire. The Tern's turrets start on
  HOLD.
- **Heat** (WS 2): each bolt puts `heat_per_bolt_mj` into its turret's sink, which sheds `heat_shed_mw`; at
  `heat_sink_mj` the turret locks out until it is below `heat_resume_frac` of it (`data/weapons.json`).
- **Aim**: the lead solution for a target at constant velocity, then an angular error drawn from a 2D normal of
  `sigma_aim = sigma_base + k_rate x the target's angular rate across the line of sight` (WS: 0.15 deg and 0.02).
- **Hit chance**: `P_hit = 1 - exp(-R^2 / (2 sigma^2))`, `sigma = range x sigma_aim` in radians, `R` the target's
  hit radius (*placeholder* spheres: Tern 12 m, Hound 9 m). The same function decides whether a turret fires (only at
  `P_hit >= 4 %`, WS) and is what the Tactical console shows. With a 2D normal aim error the share of bolts that pass
  within `R` is exactly this `P_hit`, so the preview is the outcome by construction (CLAUDE.md 6.1), and a test
  checks it.
- **Bolts** are simulated: a pool of 256, swept sphere against the target's hit sphere each tick. The server sends a
  reliable `fired` event and a `hit` event; the client flies the same straight line.

### 2.4 Gannet missiles (WS section 6, simplified)

Two bow tubes, 12 in the magazine. Loading 18 s, arming 3 s (WS). Fire needs an armed tube, a lock, and the target
within the seeker's 30 deg of the bow. Out at 30 m/s, boost 120 m/s^2 for 6 s, sustain 40 m/s^2 for 20 s,
proportional navigation with N = 4 limited to the motor's acceleration, 60 s life. The warhead's 60 MJ falls linearly
to 0 at 30 m from the closest approach. No point defence yet.

**Lock** takes 2.0 s (*placeholder*) and holds while the target is within 10 km.

### 2.5 The Hound

Approach to 1,800 m (WS's orbit), then orbit the Tern at 200 m/s on an orbit whose axis leans 50 deg from the drill's up (both *placeholder*), so it passes above and below the Tern's plane, with its turrets firing whenever
`P_hit >= 4 %`, its aim `sigma_base` 0.3 deg (WS's veteran Jackal). It fights to the end (WS's retreat at 25 % is
later). Its turrets carry a heavy pulse cannon (2.6 MJ a bolt, 3 a second, *placeholder*) standing in for the
Lance missiles it does not have yet; without it the Hound could not beat the Tern's shield regeneration (`log.md`).

## 3. `sc-core::automation`

`helm(state, profile, memory) -> Vec<Command>` and `tactical(...)`: the station's decisions as console commands.
Profiles in `data/stations.json`:

| Profile | Helm | Tactical |
| --- | --- | --- |
| `automation` (an empty seat, BS:143-144) | reacts in 1.5 s; holds heading and speed; never evasive | reacts in 2.0 s; locks the nearest hostile, turrets AUTO within 2,400 m and HOLD beyond; balanced shields; never missiles |
| `bot` (a bot client, a decent player) | reacts in 0.3 s; bow on target at 1,500 m, weaving across the Hound's line of sight | reacts in 0.4 s; locks, turrets AUTO in range, the preset facing the Hound, loads both tubes and fires when armed and in the cone |

Commands go through the same validation and apply path whoever sent them, so an empty seat, a bot and a person
differ only in what they decide.

## 4. `sc-net`

### 4.1 Messages

A binary codec, little-endian, one implementation used by both ends. Every message starts with a kind byte; every
float is checked finite and in range on decode, every count against its cap, every string against its length; a
message that fails is dropped and counted. Protocol version 1.

| Channel (WebRTC, negotiated id) | Delivery | Messages |
| --- | --- | --- |
| 0 command | reliable, ordered | client: `Hello`, `Claim`, `Ready`, `Order`, `Tactical`; server: `Welcome`, `Refused`, `Seats`, `Fired`, `Hit`, `Phase` |
| 1 input | unordered, no retransmit | client: `Stick` (set point and three axes) at 20 Hz |
| 2 snapshot | unordered, no retransmit | server: `Snapshot` at 20 Hz |

The unreliable channels carry the 8-byte header of `netcode-and-sessions` section 2 (sequence, latest received,
32 acknowledgement bits), from which each end counts loss.

A snapshot holds the tick, the phase and its timer, the Tern (position f64, attitude, velocity, body rates, set point,
faces, hull, capacitors, turret aims, tubes, lock), and each contact (id, class, position f64, attitude, velocity,
faces, hull) and each missile in flight: about 330 bytes with one contact. Full state every time; deltas
(`netcode-and-sessions` 2.2) are later, and not needed at this size.

### 4.2 Transport

`str0m` (sans I/O WebRTC, `netcode-and-sessions` section 2) on our own UDP socket and network thread; the game thread
exchanges messages with it through channels and never blocks. One peer connection per client. On a LAN the server
also listens on TCP for signalling: `POST /rtc` with the client's SDP offer returns the answer (all candidates in the
SDP, no trickle; the WebSocket form of the matchmaker's messages is later). `GET /status` returns the drill's phase,
seats and per-client loss, round trip and bytes as JSON.

Ports: 7700 TCP (signalling and status), 7701 UDP (WebRTC). A test option on the server drops and delays a share of
unreliable messages (`--loss 0.1 --delay-ms 50`), to harden without a second machine.

## 5. Clients

`sc-client --connect <host> --station helm|tactical [--name N] [--bot]`. Without `--connect` the lobby's Join a crew
takes the address.

- **Briefing** (Muster and Countdown): the drill's title, the situation, the objectives, this station's orders, the
  crew with their stations and ready marks, READY (a bot presses it after 3 s).
- **Station console** (Engage). Superseded for the consoles' look by `console-parity` (owner, 2026-10-10: the drill's
  consoles did not match the approved mockups); what follows is what the drill built: the `bridge-stations` 8.0 layout for the station over a 3D bow view: the sky, the
  Hound as a placeholder low-poly model (generated, about 200 triangles, flat-coloured by the deck program's baked
  colours with the sun as its flash light), bolts and missiles. The view's centre is lifted into the look band
  (y 32-272 lp) by shifting the projection. Remote bodies are drawn 100 ms in the past, interpolated between
  snapshots.
  - Helm: THRUST (the set point lever and the speed), SCANNER (contacts round the ship, the tube cone), ATTITUDE (the
    stick pad and rates), ORIENT (heading, pitch and roll with the quaternion's four numbers, TARGET, LEVEL, STOP).
    Keys FN: W/S set point, A/D yaw, R/F pitch, Q/E roll, Backspace all stop.
  - Tactical: TARGETS (the Hound's card: range, faces, hull, LOCK), PLOT (the shield bubble's six faces, presets),
    TURRETS (four needles in capacitor rings, WEAPONS FREE), TUBES (two pills that fill while loading, LOAD, FIRE
    with its hit ring). Keys BS: T lock, K weapons, S preset, L load, G fire.
  - Title band: station, drill, phase and clock, the other station as a chip that swaps to it. Status strip: hull,
    shields, round trip and loss.
  - **Rebuilt by `console-parity`** (2026-10-10): both consoles are now the approved mockup's, panel for panel
    (`console-parity` design 3-6), with the 3D view drawn only in the look band's viewscreen, from the camera the
    seat's feed pad picks. Helm's keys are the mockup's: W and S the set point, X all stop, A and D yaw, R and F pitch,
    Q and E roll, Z and C strafe to port and starboard and Space and Ctrl up and down while held. Tactical's: T lock,
    L load, G held to fire. Weapons free and the preset key are gone: each turret has its mode, the presets are the
    PLOT's buttons. Round trip and loss are on F3.
  - **Five seats** (`console-parity` 10.2, owner 2026-10-10: "continue and do all stations"): Engineering, Science
    and the Captain can be taken too (`--station engineering|science|captain`). Science scans, pings, tunes the shield
    band and sets the shields; the Captain calls red alert, braces, takes the viewscreen and sends orders that the
    stations acknowledge (Y); an empty seat runs its automation. Engineering is drawn unavailable until the power grid
    is built.
- **Debrief**: the result and the numbers, then back to the briefing.

## 6. The two-Pi test

| Machine | Role |
| --- | --- |
| Pi 5 8 GB (`raspberrypi`, 192.168.0.210) | `sc-server` and the Helm bot client (most RAM of the Pis: the server, owner's rule) |
| MacBook | the Tactical bot client (owner, 2026-10-10: "pi2 might be offline, so just carry on between this pi and the mac") |
| Pi 5 4 GB (`tom-lander-4`, 192.168.0.216) | a second client when it is back online |

The Pis run the same release binaries (built on the 8 GB Pi, served on the LAN while the home link was down); the
Mac builds its own. The run records: drills played and their results, snapshot loss and round trip, bytes per second each way per client,
server tick time, client frame rate. Results go in `docs/benchmarks/2026-10-10-coop-drill-lan/`; the video in
`docs/videos/`.

## 7. Pi 5 budget

| Cost | Estimate | Budget (`engine-stack` 5, `netcode-and-sessions` 10) |
| --- | ---: | ---: |
| Down per client | 330 B + 101 B headers at 20 Hz, 69 kbit/s; fired and hit events about 6 kbit/s | 80 kbit/s |
| Up per client | `Stick` 24 B + 101 B at 20 Hz, 20 kbit/s; commands under 1 kbit/s | 32 kbit/s |
| Server CPU | the drill's step is two ships and under 64 bolts: well under 1 ms a tick, measured | 2 ms per ship per tick |
| Client draw calls | Hound 1, bolts 1 each (under 40), missiles 1 each, sky 1, UI | inside the console's 16 and the 3D pass's budget |
| Client memory | the Hound mesh (about 6 kB), snapshot buffer (1 s, 7 kB) | inside 384 MB |

## 8. What is left out, on purpose

Deltas and keyframes, prediction (no avatar moves in this drill), lag compensation (bolts are server-side),
matchmaker codes, LAN discovery broadcast, heat, the shield shunt, the hull mask, point defence, Lance missiles, the
walk to the seat (players start seated, BS:266-269), more than one ship. Each is in its own change's tasks.
