# Tasks

Nothing here starts until the owner asks for implementation (CLAUDE.md section 4). It follows
`netcode-and-sessions` task 1 (the transport) and needs the owner's Fly.io account.

## 1. Messages

- [ ] 1.1 The signalling messages (design section 6) in `sc-net`, with size caps and unknown fields refused; round-trip tests; the browser's JavaScript module generated from or checked against the same schema.

## 2. The service

- [ ] 2.1 `sc-matchmaker`: the WebSocket endpoints, the session directory, codes (design section 3), the relay of offers, answers and candidates; `data/matchmaker/config.json` validated on load.
- [ ] 2.2 TURN credentials minted per join from the managed service's API (MM1); STUN servers from the config.
- [ ] 2.3 Tests: a register and a join end to end with a native client and a headless-Chromium client against a local matchmaker and a local server; a version mismatch refused before any offer; the rate limit; a code kept across a matchmaker restart.

## 3. The server's side

- [ ] 3.1 `sc-server` registers, shows its code on the mess console and in its log, sends heartbeats, reconnects with backoff and asks for its code back; the host key generated on first run.

## 4. Deploy and measure

- [ ] 4.1 A Dockerfile and `fly.toml` (one Machine, kept running, HTTPS only); `fly deploy` from a script; secrets as Fly secrets.
- [ ] 4.2 With the owner: a crew of a Pi 5 client, a desktop and a browser join a home Pi 5 server by code with no port forwarded; record which connections went direct and which through TURN, and the join time from code to data channels open, in `docs/benchmarks/`.

## 5. Move to specs

- [ ] 5.1 Move each `matchmaking` requirement into `openspec/specs/matchmaking/spec.md` with the test that proves it.
