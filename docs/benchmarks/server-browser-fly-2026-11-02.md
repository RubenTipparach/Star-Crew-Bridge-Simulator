# Joining through the deployed matchmaker on Fly.io (2026-11-02)

The owner asked: "see if fly.io will let you make a game and connect". A game server was started in a Claude Code
cloud container and listed with the deployed matchmaker; the web build, in headless Chromium in the same container,
joined it by typing its code (`openspec/changes/matchmaker` design 10.7).

## Where it ran

- `main` at `95a49b2` (PR #42 merged). The web build from `tools/web/build.sh`; `target/release/sc-server`.
- The matchmaker: `https://star-crew-bridge-simulator.fly.dev`, running the merged code (its replies carry the
  `Access-Control-Allow-Methods` and `Access-Control-Allow-Headers` lines PR #42 added).
- A cloud container with no outbound UDP and no GPU. Its HTTPS goes through a proxy, so the page's requests to Fly
  were relayed through `curl` by the test (method, body, status and reply unchanged); the WebRTC connection itself
  was between two processes on the same machine.

## Results

| Step | Result |
| --- | --- |
| `sc-server --matchmaker http://star-crew-bridge-simulator.fly.dev --list "Cloud test crew"` | Listed; `GET /v1/servers` showed it in Muster, 0 of 8 aboard |
| The listing over time | **Two Fly Machines**: the same server was listed under two codes at once (J9PB1T and YRCDFF), and its code changed whenever its poll for offers reached the other Machine (MBA3PA, 9E4NNW, RK6VK2, 4HC65A in ten minutes) |
| The browser types `join:RK6VK2` | First POST: `404 no crew holds the code RK6VK2` (the other Machine); the page tried again 10 s later: `200`, the server's answer |
| WebRTC | Data channels open; the server said `Lt. Saan Quinarberg joined ... wanting Helm`; the page heard `round 0 Muster` and stood aboard |

Screenshots: `docs/screenshots/web/multiplayer-fly/` (front menu, the Multiplayer page, the code typed, aboard).

## What it shows

- **Making a game and connecting through Fly works**: listing, the public list, the offer and answer relay over
  HTTPS from a browser page, and the join.
- **The deployed matchmaker runs on two Machines**, which halves every join and makes a server's code change every
  half minute, so a code given to a friend goes stale. Fixed in the deploy workflow (design 5, task 4.4): one Machine,
  checked after every deploy. It takes effect on the next deploy (a merge that touches the matchmaker, or running the
  workflow by hand).
- **Not tested**: two networks. This container has no UDP, so it cannot test STUN hole punching, and the WebRTC path
  here was on one machine. A browser at home joining a server elsewhere needs a real second network, and players
  behind strict routers still need TURN (task 2.2).
- **The room finder is not built.** The game's Multiplayer page today is the officer, the station and a server field
  that takes a code; the list of crews is the `browser` mockup, waiting on survey 2's MM4.
