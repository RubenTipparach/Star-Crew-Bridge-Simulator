# Tasks

The owner asked for implementation on 2026-10-23 ("lets build it", "I want a server browser"); the
server browser is design section 10. Deploying needs the owner's Fly.io account.

## 1. Messages

- [ ] 1.1 The signalling messages (design section 6) in `sc-net`, with size caps and unknown fields refused; round-trip tests; the browser's JavaScript module generated from or checked against the same schema.

## 1b. The browser (design 10.7)

- [x] 1.2 The web build's link: `web/rtc.js` drives `RTCPeerConnection` with the three channels, drops `.local` candidates, POSTs the offer to the matchmaker over HTTPS as `text/plain`, applies the answer and reports open, data and closed with a reason; `ClientNet` uses it for `Dial::Join` on emscripten; an address is refused in the browser with a plain reason.
- [x] 1.3 The matchmaker's replies carry `Access-Control-Allow-Methods` and `Access-Control-Allow-Headers`; a test reads them.
- [x] 1.4 The server field takes a bare code or `join:CODE` (the configured matchmaker) as well as `join:CODE@URL`.
- [x] 1.5 Proven: the real web build in headless Chromium joins a listed local server by code through a local matchmaker and is welcomed (`node tools/web/join_test.mjs`, 2026-10-24: welcomed at Helm, the round's Muster heard; `docs/screenshots/web/join/aboard-by-code.png`).

## 2. The service

- [x] 2.0 The server browser's HTTP API (design 10.4) in `sc-matchmaker`: listings with heartbeats and a time to live, the public list, codes held for a host key, the offer and answer relay with long polls; names cleaned; `data/matchmaker/config.json` validated on load; tests in the crate.
- [ ] 2.1 `sc-matchmaker`: the WebSocket endpoints, the session directory, codes (design section 3), the relay of offers, answers and candidates; `data/matchmaker/config.json` validated on load.
- [ ] 2.2 TURN credentials minted per join from the managed service's API (MM1); STUN servers from the config.
- [ ] 2.4 The native client and server speak HTTPS to the matchmaker (rustls), so they reach it on Fly.
- [ ] 2.3 Tests: a register and a join end to end with a native client and a headless-Chromium client against a local matchmaker and a local server; a version mismatch refused before any offer; the rate limit; a code kept across a matchmaker restart.

## 3. The server's side

- [x] 3.0 `sc-server --matchmaker URL [--list NAME]`: registers (listed or by code only), heartbeats its phase and crew, long-polls for offers and answers them through the same path as the LAN's `POST /rtc`, logs its code; the host key generated on first run and kept.
- [ ] 3.3 The peers' UDP sockets on a fixed range (`--udp-ports A-B`), so a public server can forward or declare it.
- [x] 3.4 Server-reflexive candidates from STUN on each peer socket, so two ends behind home routers can connect.
- [ ] 3.1 `sc-server` registers, shows its code on the mess console and in its log, sends heartbeats, reconnects with backoff and asks for its code back; the host key generated on first run.

## 3b. The client's side

- [x] 3.5 `sc-client --browse` prints the list; `--join CODE` joins through the matchmaker; a joined session in any phase puts the body at the muster point (hot join).
- [ ] 3.8 Create crew: the drill server in the creator's client on a real endpoint, listed under the creator's name, the creator aboard over loopback; a rename command from anyone aboard, its name in the heartbeat (design 10.2a).
- [ ] 3.6 The Multiplayer page as a server browser, the first-start name prompt and Options > Profile, after the owner approves their mockup (CLAUDE.md 10).
- [ ] 3.7 Test: a matchmaker, a listed server, one client joining by the list and a second joining in the fight phase, all on one machine.

## 4. Deploy and measure

- [x] 4.1 A Dockerfile and `fly.toml` (one Machine, kept running, HTTPS only); `fly deploy` from a script; secrets as Fly secrets.
- [ ] 4.3 A dedicated `sc-server` on Fly.io (dedicated IPv4, the UDP range declared, bound to `fly-global-services`), listed by the matchmaker.
- [ ] 4.2 With the owner: a crew of a Pi 5 client, a desktop and a browser join a home Pi 5 server by code with no port forwarded; record which connections went direct and which through TURN, and the join time from code to data channels open, in `docs/benchmarks/`.

## 5. Move to specs

- [ ] 5.1 Move each `matchmaking` requirement into `openspec/specs/matchmaking/spec.md` with the test that proves it.
