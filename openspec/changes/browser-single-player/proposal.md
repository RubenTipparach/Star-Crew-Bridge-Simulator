# Proposal: the browser hosts single player

## Why

The owner, 2026-10-13, on the itch.io build: "I launched the game in single player where is the FUCKIN GAME!"

Two things stood between Beam aboard and the game in the browser. The first was a bug: the build stopped embedding a
data file the walk had begun to read, so the client fell back to first light's grey test room. That is fixed (pull
request 27: all of `data/` is embedded, and `tools/web/smoke.mjs` boards the page before every release). The second
is by design and still true: in the browser, Beam aboard only tours the ship (`single-player` design 7, `front-menu`
2), because the desktop's single player hosts the drill over real WebRTC peers, each with a UDP socket and a thread,
and a page has neither. So the itch.io player walks an empty ship and never meets the crew, the consoles, the Hound
or a round. The game is the drill; the browser should play it.

## What Changes

- **A local link in `sc-net`** (design 2): `ServerNet` and `ClientNet` gain a second kind of connection that lives
  in the process, opens no socket and starts no thread, and carries the same three channels and the same messages.
  `DrillServer` and `Session` keep their code: only where they are connected changes.
- **Single player uses it, in the browser and on the desktop** (design 3): one way to host a local game on every
  platform. Multiplayer stays on WebRTC.
- **Beam aboard in the browser starts the game** (design 1): the same drill, crew of NPCs, consoles and rounds as the
  desktop. Tour stays the walk.
- **A paused game drops nobody** (design 4): the server counts a peer's silence in its own ticks, not on the wall
  clock, so a hidden browser tab (which stops the page's frames) pauses the game instead of emptying it.
- **The release proves it** (design 8): the smoke test plays into a round in the browser before every push to
  itch.io, and fails the release if it cannot.

## Impact

- `sc-net::transport`: the local link; `Session::connect` takes where to connect (an address, or a local server).
- `sc-server`: starts on a local link; the silence rule in ticks.
- `sc-client::solo`: hosts over the local link; the browser's Beam aboard no longer turns into the tour.
- `tools/web/smoke.mjs`: plays into a round.
- `single-player` design 7 and tasks 3.1 and 3.3, and `front-menu` design 2: point here.
- Pi 5: less than today's single player, not more (design 7).
