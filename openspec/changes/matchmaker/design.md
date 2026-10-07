# Design: the matchmaker

Status: **proposed** (2026-10-07). Nothing here is built. Facts about outside services are as of
2026-10-07 and are verified again before deploying; their sources are in `docs/references.md`,
"Transport and matchmaking".

The owner, 2026-10-07: "what if we stood up a matchmaking service on fly io? would that help
unify web vs desktop users?", then "I want to use webrtc if posible to do multiplayer on web and
desktop".

## 1. What it is, and what it is not

| It is | It is not |
| --- | --- |
| A directory of running sessions, each found by a join code | A game server: it runs no `sc-core` and holds no ship |
| The signalling relay that carries a WebRTC connection's setup between a client and a server | On the path of game traffic: that goes client to server directly, or through TURN |
| The source of each join's ICE servers (STUN addresses, TURN credentials) | A store: it keeps nothing on disk, so it has nothing to back up and no saves to lose |
| The first version check, so a mismatched client is refused before any connection is made | The authority on versions: the server checks again (`netcode` "Joining checks versions") |

## 2. A join, step by step

```text
  sc-server (home Pi)            matchmaker (Fly.io)                 client (Pi, PC or browser)
        |                               |                                       |
        |-- wss: register ------------->|                                       |
        |<- code "K7M2QX", ICE list ----|                                       |
        |   (heartbeat every 10 s)      |                                       |
        |                               |<-- wss: join "K7M2QX", versions ------|
        |                               |--- ICE list (STUN, TURN creds) ------>|
        |<- client wants in, its id ----|                                       |
        |                               |<-- offer (SDP) -----------------------|
        |<- offer ----------------------|                                       |
        |-- answer -------------------->|--- answer --------------------------->|
        |<-> candidates, both ways, as each end finds them (trickle ICE) <----->|
        |                               |<-- connected; socket closed ----------|
        |<============== WebRTC data channels, direct or through TURN =========>|
```

1. **The server registers** when it starts: it opens a WebSocket over TLS to the matchmaker and
   sends its session (name, protocol version, ship id, layout digest, data digest, seats free) and
   its host key. The matchmaker answers with a join code and the ICE server list. The server shows
   the code in its log and on the mess console, the lobby (`netcode-and-sessions` section 8).
2. **A player types the code.** Their client opens a WebSocket to the matchmaker and sends the code
   and its versions. A wrong code or a version mismatch is refused there, with the reason
   (`netcode`: the message names the mismatch).
3. **The matchmaker introduces them.** It gives the client the ICE server list, tells the server a
   client wants in, and relays the client's offer, the server's answer and both ends' candidates.
4. **WebRTC connects them** directly when it can (STUN finds each end's public address; most home
   routers then let the two ends punch through), or through a TURN relay when it cannot. The
   client closes its WebSocket once the data channels open. Everything after that is netcode's.

The client makes the offer and the server answers, because every client, a browser included, can
make an offer, and the server must answer up to eight.

## 3. Join codes

- **Six characters from Crockford's base-32 alphabet** (digits and letters without I, L, O and U, so
  nothing reads as something else): 32^6, about 1.07 billion codes, drawn from the operating
  system's secure random source.
- **One code per registered session.** It lasts while the server is registered, and survives
  restarts: a reconnecting server asks for its previous code back, and gets it unless it has been
  free for over 10 minutes. The host can ask for a new code from the server console (to end a
  leaked one).
- **Guessing is limited**, not just unlikely: at most 10 join attempts a minute and 200 a day from
  one address. With a thousand live sessions, a random guess hits one about once in 1.07 million
  tries, so one address needs about 5,400 days (15 years) on average. A leaked code is ended by the
  host asking for a new one.
- **Codes only, no public list.** It is a game among friends (recommendation taken, MM2).

The limits are data, with units in the keys, in the matchmaker's own config file
(`data/matchmaker/config.json`: `join_attempts_per_min`, `join_attempts_per_day`, `code_hold_s`,
`heartbeat_s`, `turn_credential_s`), validated on load like every data file (CLAUDE.md 6.5).

## 4. ICE servers: STUN and TURN

| Need | Choice | Why |
| --- | --- | --- |
| STUN (each end learns its public address) | Public STUN servers listed in the config (Cloudflare's and Google's), handed out with each join | Free, run by large operators, no state. A STUN server of our own on Fly is possible later; it needs a dedicated IPv4 address there for UDP |
| TURN (a relay when no direct path exists) | A managed TURN service (Cloudflare's, Twilio's or similar), with credentials minted per join through its API and valid for 4 hours | A TURN relay hands out a relay port per connection. Fly forwards UDP only on ports an app declares, only on a dedicated IPv4 address, and only to an app bound to its `fly-global-services` address: a TURN server there is a fight with the platform. coturn on a plain virtual server is the alternative if a managed service's terms do not suit (MM1) |
| A server that opens its own port | UPnP or NAT-PMP on the home router, when the router allows it (`netcode-and-sessions` M1's later step) | The server becomes directly reachable, so every client connects without hole punching or TURN |

Order of preference, which ICE finds by itself: a direct path (LAN, an opened port, or a punched
hole), then TURN. Most home connections get a direct path; TURN is for the rest (a player behind
carrier-grade NAT, a strict firewall).

## 5. Hosting on Fly.io

- **One app, one Machine**: the smallest shared-CPU size with 256 MB, in the region nearest the
  owner, kept running (`min_machines_running = 1`, auto-stop off: a server's WebSocket must not be
  cut by an idle stop). One Machine holds thousands of WebSockets; a second region is a later
  measurement, not a plan.
- **HTTPS and WebSockets only**, on Fly's shared IPv4 and IPv6, with TLS ended at Fly's proxy. No
  UDP on Fly, so no dedicated IPv4 is needed.
- **In memory only.** Sessions are live sockets. If the Machine restarts, servers reconnect with
  backoff (1, 2, 4 ... up to 30 s) and re-register with their codes; players already in a mission
  notice nothing, since their data channels never touched the matchmaker.
- **Written in Rust** in the workspace (`sc-matchmaker`), with the signalling messages from
  `sc-net`: the server, native clients, the browser's JavaScript module and the matchmaker read one
  schema (CLAUDE.md 6.1). The server's LAN signalling endpoint speaks the same messages.
- **Deployed** with `fly deploy` from a Dockerfile in the crate. Secrets (the TURN service's API key)
  are Fly secrets, never in the repository. A CI deploy is a later task, its logic in a script.

## 6. Messages

All JSON over the WebSocket, each with a `type`, validated like any network input (CLAUDE.md 6.6):
unknown fields refused, sizes capped (an SDP at most 8 KiB, a candidate at most 512 bytes).

| From | Message | Carries |
| --- | --- | --- |
| Server | `register` | Session name, protocol version, ship id, layout and data digests, seats free, host key, previous code if any |
| Matchmaker | `registered` | Code, ICE server list |
| Server | `heartbeat` | Seats free (every 10 s) |
| Client | `join` | Code, protocol version, digests, client kind (native or browser) |
| Matchmaker | `join_ok` / `join_refused` | ICE server list and a join id / the reason |
| Matchmaker | `incoming` | To the server: a join id, the client's kind |
| Either | `offer`, `answer`, `candidate` | Join id and the SDP or candidate, relayed unchanged |
| Either | `bye` | Join id: setup finished or abandoned |

## 7. Security

- **TLS everywhere** on the WebSockets; WebRTC encrypts the game traffic itself (DTLS).
- **The host key** is generated by `sc-server` on first run and kept with its saves. The matchmaker
  ties a code to the key that registered it, so nobody else can register as that session.
- **The matchmaker is trusted for signalling**, as every WebRTC signalling service is: it could, in
  principle, swap the fingerprints in an offer. We run it. A later hardening: the server signs its
  DTLS fingerprint with its host key and a client checks the signature against the key the
  matchmaker showed it at the first join.
- **Nothing personal is kept.** Logs hold codes, counts and errors, not addresses beyond the rate
  limit's one-minute window.

## 8. Costs

| Cost | Estimate |
| --- | --- |
| The Fly Machine | The smallest always-on size; Fly's current price to check before deploying |
| Matchmaker traffic | A join is a few kilobytes; a heartbeat is under 100 bytes every 10 s |
| TURN, worst case | All eight players relayed for a 3-hour mission: each relayed client moves about 73 kbit/s down and 26 kbit/s up (`netcode-and-sessions` section 4), about 99 kbit/s through the relay; eight for 10,800 s is about 1.1 GB. Usually most players connect directly and the figure is a fraction of that |

**Against the Pi 5 budget** (`engine-stack` section 5): the server holds one WebSocket with a
heartbeat (negligible CPU, a few kilobytes of memory); a client holds one only while joining. No
change to any row of the table.

## 9. Open questions

Per CLAUDE.md section 13 these take the recommendation; none has anything to look at.

| # | Question | Options | Recommendation | Status |
| --- | --- | --- | --- | --- |
| MM1 | Where TURN comes from | A managed TURN service / coturn on a virtual server / coturn on Fly | A managed service, credentials minted per join; coturn on a virtual server if its terms do not suit | Recommendation taken (ask only with screenshots) |
| MM2 | Codes only, or also a public list of sessions | Codes only / a public list | Codes only: a game among friends | Recommendation taken (ask only with screenshots) |
| MM3 | Code length | 4 / 6 / 8 characters | 6 (section 3) | Recommendation taken (ask only with screenshots) |
