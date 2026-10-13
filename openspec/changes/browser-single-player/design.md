# Design: the browser hosts single player

## Context

The owner's words are in the proposal.

| Decided elsewhere | Where |
| --- | --- |
| A solo game is a local server with one client; one code path for every game | CLAUDE.md 6.3 |
| Single player: the drill hosted in the client, an NPC at every other station, AUTO and MANUAL | `single-player` |
| Single Player's Beam aboard and Tour | `front-menu` 2 |
| WebRTC for every player; the browser's own `RTCPeerConnection` for multiplayer | `netcode-and-sessions` 2 |
| The browser build: Emscripten, WebGL 2, all of `data/` embedded, the deck fetched by `web/pre.js`, deployed to itch.io on every merge | `engine-stack` 10a, 10b |
| The browser is a playtest target, never the measure of the game's speed | CLAUDE.md 1 |

## 1. What the player gets

Beam aboard in the browser does what it does on the desktop: the crew of four NPCs (seven with Combat 2's stations)
walk to their seats, the player sits at their station, READY waits for the crew, and the drill's rounds run against
the Hound, with every console, the crew tabs and AUTO and MANUAL. Tour stays the walk. Esc comes back to the menu.
Nothing about the game is different in a browser; only its speed is the browser's (section 7).

## 2. A local link

How single player is hosted today (`sc-client/src/solo.rs`, `drill.rs`):

1. `SoloHost::start` starts `DrillServer` in the client's process, listening for WebRTC signalling on
   `127.0.0.1:0`.
2. Each NPC is a `Session` that connects to that address; so is the player.
3. Every frame the client polls the server, runs its 30 Hz ticks (`FixedClock`, at most 8 a frame), then updates each
   NPC's session and bot, then the player's session.

All of that is one thread, the frame loop, except the transport underneath it (`sc-net/src/transport.rs`): the
signalling listener is a thread on a TCP port, and every connection is a WebRTC peer on its own UDP socket and thread,
at both ends. With four NPCs and the player that is 10 peers, 11 threads and 11 sockets for a game that never leaves
the process. A page has no sockets and no threads, so the browser cannot start any of it.

The local link replaces only that layer:

| | WebRTC (today) | Local |
| --- | --- | --- |
| Connect | HTTP offer and answer over TCP, then ICE, DTLS and SCTP on a UDP socket | A call: the client end and the server end are made together |
| Carries | `Chan::Command` reliable and ordered; `Input` and `Snapshot` unreliable | All three in order, none lost (nothing can be lost in a process) |
| Threads, sockets | One thread and one socket a peer, plus the listener | None |
| Events | `Open` a moment after connect, `Data`, `Closed` | The same: `Open` on each end's first poll, `Data` in order, `Closed` when either end is dropped or closed |
| Encryption | DTLS | None needed: nothing leaves the process |
| Counters (`PeerStats`) | Bytes and messages | The same counters, so the status line and tests read them unchanged |
| Impairment (`--impair`) | Applied | Not applied: it tests the network, and a local game has none |

Shape:

- `ServerNet` and `ClientNet` keep their names and methods (`poll`, `send`, `close`, `set_status`, `peer_stats`,
  `is_open`, `stats`). Inside, each holds its kind of link: the WebRTC parts it has today, or the local queues.
  `DrillServer` and `Session` call the same methods and do not know which they have (CLAUDE.md 6.1: the server and
  the sessions stay one implementation).
- A local server is started without an address: `ServerNet::local()`. It hands out a `LocalDoor`, a cheap handle a
  client connects through: `ClientNet::local(&door)`.
- `Session::connect` takes a `Dial`: `Dial::Addr(SocketAddr)` for a real server (multiplayer, `sc-bot`, the tests) or
  `Dial::Local(LocalDoor)` for one in the process. `DrillServer::start` takes the same choice in `ServerConfig`
  (`listen` becomes `Listen::Addr` or `Listen::Local`), and `DrillServer::dial()` returns what a session connects to.
- The queues are shared by both ends behind an `Arc<Mutex<..>>`, so a `ClientNet` is as `Send` as it is today. In a
  page there is one thread and the lock never waits.
- A local peer's address in `peer_stats` is `0.0.0.0:0`. A local server serves no status page (there is no port to
  serve it on); `set_status` keeps the text, so the drill's status line is unchanged.

## 3. Who uses it

Single player, in the browser and on the desktop. One way to host a local game on every platform, rather than WebRTC
on the desktop and something else in the browser (CLAUDE.md 6.1). On the desktop it also:

- ends the leak `single-player` task 3.3 was for: leaving a game frees everything, since there is no listener thread
  or port;
- takes the 10 peers' encryption and threads off a Pi 5 client hosting single player (section 7).

What it costs: single player no longer exercises WebRTC. Multiplayer does, and so do `sc-bot`, the six-round Pi
playtests (`coop-drill` 5.1) and the transport's own test, so nothing goes untested.

Recommendation taken (ask only with screenshots): the desktop's single player moves to the local link with the
browser's.

Multiplayer is unchanged: a desktop joins over WebRTC as today. A browser cannot join yet; it needs the browser's own
`RTCPeerConnection` (`netcode-and-sessions` 2), which is that change's work, not this one's.

## 4. One thread, and a tab that is hidden

The frame loop already runs the whole game on one thread, in the order above, so the browser runs it as the desktop
does. Two things change because a page is not a window:

- **A hidden tab runs no frames.** The browser stops `requestAnimationFrame` for a tab that is not shown. Nothing of
  the game runs, so the game is paused: the clock drops time beyond 8 ticks a frame (`FixedClock`), and the frame's
  step is held to 0.1 s (`drill.rs`), so coming back does not race the game forward.
- **The silence rule must not drop a paused game.** The server drops a peer it has not heard from in 5 s
  (`SILENT_S`, measured on the wall clock with `Instant`). After a hidden tab, the server's first poll would find every
  peer, the player included, silent for as long as the tab was hidden, and drop the whole crew. So the server counts
  silence in its own ticks: a peer is dropped after `SILENT_S` worth of ticks (150 at 30 Hz) with nothing from it. A
  dedicated server ticks without stopping, so for it the rule reads as before; a whole game that stops (a hidden tab,
  a debugger, a laptop lid) drops nobody, because the server stopped too. The 5 s itself does not change.

## 5. Startup in the browser

Everything a single-player game reads is already in the page:

| Read | By | In the browser |
| --- | --- | --- |
| The drill's data, `data/missions/drill-hound.json`, `data/engine/render.json`, `data/space/exterior.json`, `data/crew/names.json` | `drill.rs` | Embedded under `/data` (all of `data/`, since 2026-10-13) |
| The repairs on deck | `sc-server` (`RepairData::load_for_ship`) | The same |
| The walking bridge: `data/ships/tern/layout.json`, `data/crew/walk.json`, `data/ships/tern/command_suite.json` | `sc-server::bridge` | The same |
| The compiled deck, `compiled/tern.deck` | `sc-server::bridge`, the drill's bridge view | Fetched and unpacked by `web/pre.js` before the client starts |

The paths are relative and the page's working directory is `/`, as the walk already relies on. Nothing new is
embedded.

## 6. Where it lives

| Part | Where |
| --- | --- |
| The local link, `LocalDoor`, `Dial` | `sc-net/src/transport.rs` (it may grow a `local.rs` beside it) |
| `Session::connect` taking a `Dial` | `sc-net/src/session.rs` |
| `Listen`, `DrillServer::dial`, the silence rule in ticks | `sc-server/src/lib.rs` |
| Hosting over the local link | `sc-client/src/solo.rs` |
| Beam aboard no longer becomes the tour in the browser | `sc-client/src/main.rs` (the `web && Solo` gate goes) |
| Playing into a round in the smoke test | `tools/web/smoke.mjs` |

No rule moves: the server decides every outcome, the sessions and the bots are the multiplayer game's, and the drill
does not know it is local.

## 7. Pi 5 budget

| Cost | Amount |
| --- | --- |
| CPU | Less than today's single player on a Pi 5 client: the same server ticks and bots, without 10 WebRTC peers' DTLS, SCTP and threads. The Pi measures it (task 3.2) |
| Memory | Less than today's: single player's hosting cost was about 12 MB resident with its loopback peers (`single-player` 8, x86-64, a debug build, not a Pi), and the local link has none of the peers' buffers. The Pi measures it (task 3.2) |
| Network | None |
| Rendering | None |
| Browser | The page's main thread runs the server, four or seven bots and the client. The browser is a playtest target and its speed is not the game's (CLAUDE.md 1); a cloud session does not measure it |

## 8. Checks

- `sc-net`: a server and a client on a local link open, exchange messages on all three channels in order, and close
  from either end, with no thread or socket (the WebRTC loopback test's twin).
- `sc-server`: a peer silent for `SILENT_S` worth of ticks is dropped; a server that is not ticked for 30 s of wall
  clock drops nobody.
- `sc-client`: the in-process single-player test (`single-player` 9) runs over the local link, and leaving the game
  leaves no thread behind.
- `tools/web/smoke.mjs` plays into a round in the browser: Beam aboard, the crew seated, READY, and the round's first
  phase line ("round 1 ..."), failing the release on "no ship (", a page error, or no round. It already runs before
  every push to itch.io (`tools/web/release.sh`, `.github/workflows/web.yml`).
- Captures in the browser (headless Chromium, SwiftShader): the crew walking to their seats and a console with the
  crew tabs, beside the desktop's `docs/screenshots/engine/single-player/` shots of the same moments, in
  `docs/screenshots/web/single-player/`.
- `tools/web/build.sh` and `scripts/check.sh` before the merge.

## Known limits

- A browser cannot join a multiplayer game yet (section 3).
- The browser's frame time is not measured in a cloud session (no GPU); the owner judges it on itch.io.

## Open questions

| Id | Question | Recommendation | Status |
| --- | --- | --- | --- |
| B1 | The desktop's single player also moves to the local link | Yes: one way to host on every platform, no threads or ports to leak, less work on a Pi | Recommendation taken (ask only with screenshots) |
| B2 | Silence counted in the server's ticks rather than the wall clock, for every server | Yes: a dedicated server reads the same; a paused game drops nobody | Recommendation taken (ask only with screenshots) |
