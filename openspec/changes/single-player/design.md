# Design: single player

## Context

The owner's words are in the proposal. The menu that starts it is `front-menu`.

| Decided elsewhere | Where |
| --- | --- |
| A solo game is a local server with one client; one code path for every game | CLAUDE.md 6.3 |
| Every station works with or without a player; the drill's stations, automation and bot profiles | CLAUDE.md 7, `bridge-stations` 3, `coop-drill` 3 |
| A bot is a session played by the core's automation at the `bot` profile; on foot a claim is a walk | `coop-drill` 3, 9 |
| An open station is a dashed tab in the title band; press it to swap to it | `console-parity` (the approved `consoles.html`) |
| The captain's orders; an automation acknowledges and does not act on them | `console-parity` 10.2 |

## 1. A game on the player's machine

Beam aboard (Single Player) or `sc-client --solo` starts the drill server inside the client: `sc-server`'s
`DrillServer`, bound to 127.0.0.1 on a port the system picks, polled and ticked from the client's frame loop at the
drill's 30 Hz (`FixedClock`, as `sc-server`'s own loop). The player joins it as a person over loopback, and each NPC
joins as a bot session in the same process. Nothing in the drill knows it is local: the server, the sessions, the
snapshots and the commands are the multiplayer game's. Leaving (Esc) closes the server and the sessions and returns
to the menu.

## 2. The crew

| Who | Seat | Name |
| --- | --- | --- |
| The player | The station chosen in Single Player (Any: Helm) | The officer's |
| One NPC per other station of the mission's roster | That station | From `sc-core::names`, seeded by the session, the slot and "npc" (CLAUDE.md 6.4); a Lieutenant, as the lobby's default rank |

They join as bots, so the server marks them and they walk to their seats on the bridge (`coop-drill` 9). The
mission's roster is today Helm, Tactical, Engineering, Science and Captain: four NPCs. Combat 2 adds LASER PORT, LASER
STBD and GUNNER (`laser-banks` 3, `enemy-fighters`), seven NPCs, with no change here.

## 3. Competent

Each NPC is `sc-net::bot::Bot` at the `bot` profile (`data/stations.json`): Helm reacts in 0.3 s, hunts and weaves;
Tactical in 0.4 s, fires missiles and turns the shields to the threat; Science scans; the Captain calls red alert.
These are the bots that won six rounds of six over the Pis' LAN (`coop-drill` 5.1). Engineering has nothing to run
until the power grid is built (`console-parity` 10.2).

## 4. AUTO and MANUAL

Each NPC's station has a mode, the player's choice, kept by the client that hosts the game:

| Mode | The NPC | The station |
| --- | --- | --- |
| AUTO | Seated, decides and sends its station's commands | Played by the NPC |
| MANUAL | Seated, sends nothing of its own; acknowledges nothing | Does what the player does on its console: the player's presses go to the server as the NPC's commands |

- Every NPC station starts on AUTO. The player's own station is theirs: it has no NPC and no mode.
- **A station works only while its NPC is seated** (the owner: "auto does still need a npc crew to sit there"). While
  the NPC walks to the seat, the server refuses its commands as it refuses anyone's who is not seated: on AUTO it
  waits, on MANUAL the player's presses do nothing, and the tab says so (design 5).
- Pressing a control on an AUTO station's console takes it to MANUAL: the NPC hands over rather than the press being
  lost. Recommendation taken (ask only with screenshots).
- On MANUAL the captain's orders to that station wait for the player's acknowledgement (Y), as a person's do.
- While a MANUAL or AUTO console is shown and not touched, the player's seat sends nothing for it (no helm stick at
  rest); the helm's set points follow the ship's while it is on AUTO, so taking it over does not jump the speed.
- **The round starts with the whole crew seated.** The core runs a station whose holder is still walking on its
  automation (`coop-drill` 9). In single player that never acts in a round: an NPC readies only once seated, and the
  player's READY is held until every NPC is seated, so the countdown starts with every seat filled. No NPC gets up in a
  round: a bot moves only to a post nobody holds (`coop-drill` 9), and every post is held.

## 5. Every console from one seat

In single player the title band's open-station tabs (dashed, "swap to it") become crew tabs, one per NPC station:

| Part | Shows |
| --- | --- |
| The tab | The station's icon and word, solid edged: crewed |
| Its mark, at the right | A robot head: AUTO. A hand: MANUAL. Dim while its NPC is not seated |
| The tab of the console shown | Filled in its role colour, as the station's own chip |
| The mode switch | After the tabs, while another station's console is shown: two halves, AUTO and MANUAL, the current one filled |

- Press a crew tab, or Tab and Shift+Tab, to show that station's console; press the player's own chip, or Tab past
  the last, to come back. The player stays in their seat: this is the console, not a walk.
- Press the switch, or M, to change the shown station's mode.
- The tooltips are the details (CLAUDE.md 10): "Tactical: Lt. Dal Varen, on AUTO".

The mockup is `docs/mockups/consoles.html` (its `solo` state); the engine is held to it by the parity check.

## 6. Where it lives

| Part | Where |
| --- | --- |
| Hosting, the NPC sessions and their modes | `sc-client/src/solo.rs` |
| MANUAL: a bot that sends nothing of its own | `sc-net/src/bot.rs` (`Bot::manual`) |
| Showing another station, routing its commands to its NPC | `sc-client/src/drill.rs` |
| The crew tabs and the switch | `sc-client/src/console/mod.rs` (the title band) |

No rule moves: the server decides what a command does and refuses one from a body not seated.

## 7. The browser

Single player runs on `sc-net`'s local link (`browser-single-player`): the server and every session in one process,
with no socket and no thread, so a browser page hosts the drill as a desktop does, and Beam aboard starts this game
there too. The desktop uses the same link. Before it, the server and the sessions met over loopback WebRTC, which a
page cannot open, and the browser's Beam aboard toured the ship instead.

## 8. Pi 5 budget

| Cost | Amount |
| --- | --- |
| CPU | The server's 30 Hz tick and one session per NPC, on the local link since `browser-single-player` (before it, each was a WebRTC peer with a thread at both ends). Measured on the Pi in task 2.7 |
| Memory | Measured headless (task 2.6): a single-player client with a crew of four peaked at 336.8 MB resident against 324.9 MB for a multiplayer client alone, so hosting costs about 12 MB (the server, its walk grid, five loopback peers). That was x86-64, a debug build and llvmpipe's software GL, 45 s from launch into the round; it is not a Pi measurement. A 1 GB Pi 5 measures it in task 2.7 |
| Network | Loopback only |
| Rendering | None added beyond the crew tabs: a few rectangles and icons in the title band, the same draw call |

## 9. Checks

- An in-process test hosts a solo game, sees an NPC seated at every other station, sets one to MANUAL and sees it send
  nothing while the player's command goes through as its own.
- `sc-client --headless --solo --shots DIR`: captures of the crew walking to their seats, a console with the crew
  tabs, another station shown, and a station on MANUAL (`docs/screenshots/engine/solo-*`).
- The parity check holds the title band's crew tabs to the mockup's `solo` state.

## Known limits

- None left from hosting: since `browser-single-player` a single-player game opens no listener, socket or thread.

## Open questions

| Id | Question | Recommendation | Status |
| --- | --- | --- | --- |
| Q1 | A press on an AUTO station's console takes it to MANUAL | Yes, the NPC hands over | Recommendation taken (ask only with screenshots) |
| Q2 | The browser tours the ship until it can host | Yes | Recommendation taken (ask only with screenshots) |
| Q3 | Approve the crew tabs and the switch (the captures beside the mockup) | Approve | Approved, the owner, 2026-10-16 (survey 2): "approve, but need a start mission button when everyone is ready, or captain can start even when everyone is not ready. it doesnt matter." The crew tabs stand; the START MISSION control is the `start-mission` change |
