# Design: the first mission gate

## Context

The owner, 2026-10-10: "document this as the steps required to pass this next quality gate. I expect no harness
videos this time. I want ACTUAL in game videos." The flow is quoted whole in the proposal. This design turns it into
steps a recording can be checked against, says where the game stands on each, and orders the work.

| Decided elsewhere | Where |
| --- | --- |
| Every station works with or without a player; one player can play | CLAUDE.md 7 |
| A solo game is a local server with one client; nothing pauses | CLAUDE.md 6.3 |
| Test on real hardware; the Pi measures the Pi | CLAUDE.md 12 |
| Write it up before touching code; one change per capability | CLAUDE.md 3, 4 |
| The drill: server, bots, five consoles, one fight, crew on foot on the bridge | `coop-drill` |

## 1. The gate

The gate passes when one recording (section 2) shows every step below, in this order, in one session. "Crew" means
players and NPCs together; "bot" is a client driven by automation (`sc-bot` or `sc-client --bot`).

| # | Step | The recording shows |
| --- | --- | --- |
| 1 | Everyone joins a lobby room and chats | A room before the ship with at least three crew in it, from at least two machines, and chat lines from them |
| 2 | In-game chat says what bots are doing | A chat panel in the game; each bot posts what it does ("At Helm", "Shields to forward", "Fire out in Engineering") |
| 3 | The captain picks a mission | A mission list; the captain picks the demo mission against a Grabon ship; the mission starts for everyone |
| 4 | Crew walk around and explore the ship | Bodies walking on more than one deck in the networked game before stations are taken |
| 5 | The captain goes to the bridge and sets red alert | The captain's body reaches the bridge; red alert set from the captain's console; the bridge's light turns red |
| 6 | Bots take Helm and Tactical; the captain sees them | A bot seated at Helm, another joining Tactical; the captain opens the Helm and Tactical dashboards from his console and sees each one's operator named |
| 7 | The captain hops stations and takes any free one | The captain moves between stations from his console and operates a station no one holds |
| 8 | The captain sets power, then works shields | Power allocation changed on the Engineering station (a real effect on the ship), then shields set |
| 9 | The ship is attacked and damaged | Hits that damage systems, not only the hull: a system's output drops, a room reads damaged |
| 10 | At least 8 crew aboard | Eight or more crew in the ship: the players and bots plus at least five NPCs, each named |
| 11 | Repairs: ordered to NPCs or players, or done by the captain | The captain orders an NPC to repair a system and it walks there and fixes it; an order to a player; the captain repairing one himself (a repair game) |
| 12 | Fires are put out | A fire breaks out from damage and a crew member puts it out |
| 13 | The ship is repaired after the fight | Damaged systems brought back after the enemy is gone |
| 14 | Warp out ends the scenario | The crew jumps away and the mission ends on that |
| 15 | The crew survives | The end screen says the mission was won and the crew lived |

## 2. What counts as a recording

- **The game's own window, on real hardware**: a Pi 5 (`wf-recorder` on its desktop, the settings in section 5), or
  the owner's PC or Mac. Never headless captures (`--shots`), scripted camera poses, `--watch` fly-throughs presented
  as play, or a test harness.
- **One take from the lobby room to the end screen**, unedited. Cuts are allowed only between whole steps and are
  listed in the shot list.
- **At least two machines** in the session (the gate is a multiplayer game): Pi 1 and Pi 2, or a Pi and the PC.
- **A shot list** in `docs/videos/README.md`: each step's time in the video, so the owner can jump to it.
- **Played, not staged**: bots are the game's automation and players are clients. Nothing is moved by a test script.

## 3. Where the game stands (2026-10-10, main at 5569944)

Found by reading the engine, not the mockups. "Drill" is the networked game (`sc-server`, `sc-client --connect`,
`sc-bot`); "walk" is single-player `sc-client`.

| # | Today | Evidence | Builds it |
| --- | --- | --- | --- |
| 1 | No: the walk's lobby is single-player and its Join is off; the drill's only room is its briefing panel; no chat message in the protocol | `sc-client/src/lobby.rs`, `sc-client/src/drill.rs` (briefing), `sc-net/src/msg.rs` | `lobby` (networked), `netcode-and-sessions` 8, new `crew-chat` |
| 2 | No: bots print to their own terminal only | `sc-server/src/bin/sc-bot.rs` | new `crew-chat` |
| 3 | No: one mission, fixed at server start (`drill-hound`, the Hound corvette); nothing named Grabon | `data/missions/drill-hound.json` | new `missions` |
| 4 | Walk only (three decks, ladders, the lift). In the drill bodies walk from the muster point to the seats, on the bridge, by themselves | `sc-core/src/combat/bodies.rs` | `coop-drill` 9, `crew-on-deck`, `netcode-and-sessions` 5 |
| 5 | Partly: the server owns red alert, the Captain's ALERT control sets it, the console bands turn red; the bridge's lighting stays normal and it works only in the fight | `sc-core/src/combat/mod.rs`, `console/captain.rs` | `bridge-stations` 7, `light-baking` (states) |
| 6 | Partly: bots take Helm and Tactical, and every console names each station's operator. No console shows another station's dashboard | `sc-net/src/bot.rs`, `console/captain.rs` | `bridge-stations` 5 (it says today that no console carries another's tab: the owner's flow changes that) |
| 7 | Partly: the title band offers free stations and a click claims one (a walk on foot); a bot's station is never offered | `console/mod.rs`, `seat.rs` | `bridge-stations` 5-6 |
| 8 | Shields yes (Tactical, Science). Power no: the Engineering console is drawn unavailable | `console/engineering.rs` | `power-grid` 8, 11 |
| 9 | Hull only: shields then a 120 MJ hull; no system damage, no breaches | `sc-core/src/combat/mod.rs` | `damage-control` 1-2, 12 |
| 10 | Walk only (nine NPCs). The drill has only connected clients | `sc-core/src/crew.rs` | `crew-npcs` 1, 4, 7 in the server |
| 11 | No: repair games run alone (`sc-client --repairs`); the rules exist in `sc-core::repair` but nothing in the drill calls them; SEND REPAIRS is drawn unavailable | `sc-core/src/repair/`, `console/captain.rs` | `repairs-on-deck`, `repair-minigames` 8, `damage-control` 6-7, `crew-npcs` 4 |
| 12 | No: fire exists only in the mockup | `docs/mockups/fire.html` | `fire-spread`, `damage-control` 3-4 |
| 13 | No: the next round resets the ship | `sc-core/src/combat/mod.rs` | `repairs-on-deck`, `hull-repair` |
| 14 | No: a fight ends by a kill, a loss or 600 s | `sc-core/src/combat/mod.rs` | `flight-and-navigation` 7 (jump), new `missions` |
| 15 | Yes: the debrief shows the outcome, time, hits and hull left | `sc-client/src/drill.rs` | `coop-drill` |

Five of fifteen are there in part (5, 6, 7, 8, 15); ten are not built.

## 4. The build order

Each group is its own change or changes, written up and approved before it is built (CLAUDE.md 4), and each ends in a
recording of the steps it adds, made the way section 2 says.

| Group | Steps | Changes | Ends in |
| --- | --- | --- | --- |
| A. The room | 1, 2, 3 | `lobby` networked, `crew-chat` (new), `missions` (new: the list and the Grabon demo mission) | Three crew from two machines in the room, chatting; the captain picks the Grabon mission |
| B. Aboard | 4, 10 | `coop-drill` 9 on every deck, `crew-npcs` in the server, a crew of at least 8 | The crew and five NPCs walking the decks before stations |
| C. Command | 5, 6, 7, 8 | `bridge-stations` 5-7 (the captain's view of any station, relieving a bot, red alert light), `power-grid` 8, 11 on Engineering | The captain at red alert, looking in on Helm and Tactical, taking Engineering, setting power, then shields |
| D. Damage and repair | 9, 11, 12, 13 | `damage-control`, `fire-spread`, `repairs-on-deck` in the networked game | A hit that breaks a system and starts a fire; an NPC sent to fix it; the captain repairing one himself |
| E. The end | 14, 15 | `missions` (warp out), `flight-and-navigation` 7 | The jump away and the end screen |
| Gate | 1-15 | None new | The whole flow in one take, section 2 |

A first, then B and C (they can run side by side), then D, then E. A is first because every later recording starts
in the room.

## 5. The baseline recording

To show where the game stands, today's mission was recorded the way the gate asks: on Pi 1, the game's own window,
`wf-recorder`, one take from the crew joining to the debrief. The monitor is switched to 1280 x 720 for the take
(section 6), the size the consoles are designed at, and back afterwards:

```sh
wlr-randr --output HDMI-A-1 --mode 1280x720@60.000000
wf-recorder -o HDMI-A-1 -c libx264 -p preset=ultrafast -p crf=20 -p threads=2 -F "fps=30,format=yuv420p" -f mission.mp4
wlr-randr --output HDMI-A-1 --mode 2560x1080@60.000000
```

The session and its shot list are in `docs/videos/README.md` (`2026-10-10-first-mission-baseline-pi5.mp4`).

Three takes were made, 2026-10-10, each a whole mission (about 70 s of fighting, 104 s from the crew joining to the
next briefing). The first, at 2560 x 1080, captured 15 frames a second (section 6). In the first and second the
Captain bot got up from her chair when the fight began and took the empty Tactical or Engineering seat: a bot moves to
a more important station that stands empty (`sc-net/src/bot.rs`, `bot_posts.reassign_s`), and the second take spent
the fight on the Engineering console, which the drill draws unavailable. The third, the baseline, crews all five
stations, so the Captain stays at her console. Pi 2 was asked to fly Tactical; its session was not allowed to start a
background process, so the baseline is one machine and the gate's two-machine rule (section 2) is still to meet.

## 6. Recording on a Pi 5

The Pi 5 has no hardware video encoder, and `wf-recorder` 0.3.0 copies every frame out of the compositor (labwc)
through shared memory. That copy, not the encoder, sets the frame rate. Measured on Pi 1 (Pi 5, 8 GB), 2026-10-10,
with the drill on screen, as frames written per second over 8-10 s:

| Monitor mode | Pixels | Frames captured per second |
| --- | ---: | ---: |
| 2560 x 1080 | 2.76 million | 15 (raw, no encoding: 15.5; `ultrafast` with 2 or 3 threads: 15; a 1920 x 1080 region of it: 14) |
| 1920 x 1080 | 2.07 million | 18 |
| 1280 x 720 | 0.92 million | 30 |

So a smooth recording on a Pi is made at 1280 x 720. The encoder keeps up: `ultrafast` with two threads encoded a moving
2560 x 1080 picture at 36 frames per second, and scaling first was slower (24), because the scaler costs more than it
saves. The first take, at 2560 x 1080, used 2.3 cores for the recorder and 17 % of one core for the game client, with
31 % of the CPU idle; on screen it changed only 5-8 times a second while the crew walked, though the game runs
smoothly when not recorded. A recording at the monitor's full size needs another machine: the owner's PC or Mac, or a
capture card.

At 1280 x 720 the recorder took about one core (97 %) and the CPU was 61 % idle. The consoles change on every
captured frame; the 3D bridge changed on about 18 of 30 a second while the crew walked, which is the game's own rate:
the ship walk draws at a median 26-28 FPS on this Pi without recording (`docs/benchmarks/2026-10-10-pi5-8gb-walk`).

## Open questions

| Id | Question | Recommendation | Status |
| --- | --- | --- | --- |
| Q1 | What is a Grabon ship? Nothing in the repository names one | A heavier enemy than the Hound (a cruiser that takes several minutes to beat), designed in `missions` with a mockup | Recommendation taken (ask only with screenshots); the `missions` mockup will show it |
| Q2 | The captain viewing any station's dashboard reverses `bridge-stations` 5 ("no console carries another station's tab") | Follow the owner's flow: the Captain console gets a view of each station, read-only until he takes it | Recommendation taken (ask only with screenshots) |
| Q3 | "This ship requires at least 8 crew": a rule that blocks a mission with fewer, or NPCs filling to 8 | NPCs fill the crew to 8, so one player can still play (CLAUDE.md 7) | Recommendation taken (ask only with screenshots) |
