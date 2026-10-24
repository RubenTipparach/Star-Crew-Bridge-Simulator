# Design: the matchmaker

Status: **proposed** (2026-10-07), **amended 2026-10-23** for the server browser (section 10), which
replaces sections 2, 3 and 6 where they differ. Being built on `feat/server-browser`. Facts about outside services are as of
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
- ~~Codes only, no public list~~: replaced by the server browser (section 10; owner, 2026-10-23).
  A code still joins a session that is not listed.

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
| MM2 | Codes only, or also a public list of sessions | Codes only / a public list | Codes only: a game among friends | **Owner's answer, 2026-10-23: a public list** ("I want a server browser, we can host server stuff on fly io"). Section 10 |
| MM3 | Code length | 4 / 6 / 8 characters | 6 (section 3) | Recommendation taken (ask only with screenshots) |
| MM4 | The Multiplayer page as a server browser (`ui-art-direction.html` state `browser`) | Approve as drawn / change it | Approve | Asked in survey 2, "Server browser", with its screenshot (2026-10-24) |
| MM5 | The Esc menu's crew name and join code (state `crewname`) | Approve as drawn / change it | Approve | Asked in survey 2, with its screenshot (2026-10-24) |
| MM6 | The first-start name and Options > Profile (state `profile`) | Approve as drawn / change it | Approve | Asked in survey 2, with its screenshot (2026-10-24) |

## 10. The server browser (owner, 2026-10-23)

The owner, 2026-10-23: "can players see servers someone started? and hot join?", "lets build it",
"I want a server browser, we can host server stuff on fly io", and "no login needed, just local
profile storage, ask players for player name, make it editable in settings".

### 10.1 What the player sees

The Multiplayer page lists the sessions running now: each row is the session's name, its mission,
its phase (muster, fight, debrief) and its crew as filled seats of eight. Picking a row and Join
joins it, whatever its phase: **a crew can be joined in progress** ("hot join"). The joining body
starts at the muster point on foot and walks to a free station; a station held by automation is
taken over as any claim is (`coop-drill`, `bridge-stations` 6). A typed code still joins an
unlisted session, and a typed address still joins on the LAN with no matchmaker at all.

Joining in progress is already the server's rule (`sc-core` `Drill::join` takes a player in any
phase while a seat is free, and `crew-chat` sends a late joiner the last lines); the browser only
makes it findable. A full session is listed, greyed, with no Join.

### 10.2 No login: a local profile

There are no accounts. The profile is the officer kept in `settings/officer.json` (`lobby` design
4), on the player's own machine (the browser's storage in the web build). The first start asks for a
name (with the dice to roll one); Options gains a Profile tab where the name, rank and station are
changed later. The name is what the server and the other players see; the server cleans it and
refuses nothing for a clash (two players may share a name; slots tell them apart).

### 10.2a Creating a crew, and its name (owner, 2026-10-23)

- **Create crew hosts on the creator's machine** (owner's answer: "On the creator's machine"; "fly io
  just exposes servers for players to join"). The client starts the drill server in its own process
  with a real network endpoint, lists it, and puts its creator aboard over loopback. The crew lasts
  while its creator's game runs. Fly.io runs only the matchmaker.
- **The crew's name starts as its creator's name** and **anyone aboard can change it** ("server name
  by default inherits its creator, but anybody can change serve name"): the Esc menu's crew panel
  holds the name with a pencil and the join code beside it; a rename goes to the server as a command,
  the server tells the crew and puts the new name in its next heartbeat, and the list shows it.
- **A crew takes a player while a seat is free** ("as long as theres a slot availible someone can
  join"), in any phase (10.1).
- Mockups: `ui-art-direction.html` states `browser` (with Create crew) and `crewname`.

### 10.3 Listing

- **A server lists itself when started with `--list "NAME"`** and a matchmaker URL (`--matchmaker`,
  default from `data/matchmaker/client.json`). Without `--list` it registers unlisted: joinable by
  code only. Without a matchmaker it is a LAN server, as today.
- **The list is public and holds no addresses**: name, mission, phase, seats filled and free,
  protocol version and code. A server's IP is never shown; a client reaches it through the
  signalling relay below.
- **A listing lives while its server heartbeats**: every `heartbeat_s` (10 s) with its phase and
  crew; one silent for `listing_ttl_s` (30 s) drops off the list. The code is held `code_hold_s`
  (600 s) for the same host key, as section 3.
- **Names are cleaned** like officer names (`sc-core::names`: length capped, the deny list), so the
  public list shows nothing the game would not show in the crew list.

### 10.4 Plain HTTP, not WebSockets

Section 2 drew a WebSocket. The transport's signalling today is one HTTP POST of a complete offer
answered with a complete answer (no trickle: `sc-net` `transport.rs`), so the matchmaker relays that
same exchange over HTTP, which the native client already speaks and a browser's `fetch` speaks too:

| Who | Request | Does |
| --- | --- | --- |
| Server | `POST /v1/servers` | Register: name, listed or not, mission, protocol, digests, seats, host key, previous code. Answers the code and a listing id |
| Server | `POST /v1/servers/{id}/heartbeat` | Phase, crew, seats; refreshes the listing |
| Server | `GET /v1/servers/{id}/offers` | Long poll (at most 20 s): the offers waiting for this server, each with a join id |
| Server | `POST /v1/servers/{id}/answers` | The answer for a join id |
| Server | `DELETE /v1/servers/{id}` | Unlist on a clean stop |
| Client | `GET /v1/servers` | The public list |
| Client | `POST /v1/join/{code}` | The client's offer and versions; held until the server answers (at most 10 s), then the answer, or a refusal naming the reason |

Every request from a server carries its host key (a header); the matchmaker keeps a listing tied to
the key that made it. Bodies are JSON with unknown fields refused and sizes capped (section 6).
Fly's proxy ends TLS in front of it, so the browser reaches it over HTTPS; the native client's TLS
is task 2.4 (until then it speaks plain HTTP, to a local matchmaker or to Fly's HTTP port).

### 10.5 What reaches across the internet, and when

| Server | Client | Connects with what is built here |
| --- | --- | --- |
| On the LAN | On the LAN | Yes: host candidates, as today |
| With a public address (a cloud machine, a forwarded fixed UDP range) | Anywhere | Yes, once the server's peer sockets use a fixed port range it can forward or declare (task 3.3) |
| Behind a home router | Behind another router | Needs STUN (server-reflexive candidates) and, for the rest, TURN: tasks 3.4 and 2.2 |

**Dedicated servers on Fly.io** (the owner's "we can host server stuff on fly io"): an `sc-server`
Machine with a dedicated IPv4 and its UDP range declared in `fly.toml`, bound to
`fly-global-services`, is a public-address server in the table above. The matchmaker lists it like
any other. That deployment is task 4.3.

### 10.6 Costs

A listing is under 300 bytes; the list of a hundred sessions is about 30 kB. A heartbeat is under
200 bytes every 10 s; a long poll is one idle connection per server. The matchmaker keeps
everything in memory: a thousand listings are well under 1 MB.

### 10.7 The browser joins (owner, 2026-10-24)

The owner, on the itch.io build failing with "no route to 172.29.1.0:7700: Host is unreachable (os error 23)":
"shouldnt it connect too fly io?", then "do it". The browser build dialled an address with the desktop's transport,
which opens UDP sockets a page cannot open, so every multiplayer join from the browser failed whatever was typed.

**The browser's own WebRTC.** In the web build `ClientNet` has a third link beside the native one and the local
link: a small JavaScript library (`web/rtc.js`, linked with `--js-library`) that drives the browser's
`RTCPeerConnection`. It makes the same three channels as the native client (`command` reliable and ordered,
`input` and `snapshot` unordered with no retransmits), gathers its candidates (no trickle, as section 10.4), and
POSTs `{protocol, offer}` to the matchmaker's `/v1/join/{code}`; the answer it gets back is the server's, which it
applies. Nothing on the server or the matchmaker changes: the offer and answer are the same
`{"type": "offer", "sdp": ...}` JSON the native client sends (str0m's form is the browser's).

| Step | Native client | Browser |
| --- | --- | --- |
| Offer | str0m, a UDP socket, host and STUN candidates | `RTCPeerConnection` with the STUN servers of `data/matchmaker/client.json` |
| Candidates offered | Host (LAN) and server-reflexive | Server-reflexive and any plain-address host candidate; Chrome's `.local` names are dropped, since the server's parser reads IP addresses only |
| Signalling | Plain HTTP to the matchmaker (task 2.4 for TLS) | `fetch` over HTTPS to the same host (the page is HTTPS, so plain HTTP would be blocked) |
| Waiting | Blocks until the answer, then a thread runs the peer | Returns at once; the link reports open, data or closed as the page's event loop delivers them |
| A failure | `dial` returns the reason | The link closes and keeps the reason (a refused code, a failed connection), which the loading screen shows |

**Cross-origin.** The page is served by itch.io and the matchmaker by Fly.io, so every request is cross-origin.
The browser sends its JSON as `text/plain`, a request that needs no preflight, and the matchmaker's replies also
carry `Access-Control-Allow-Methods` and `Access-Control-Allow-Headers` (`content-type` and the host key's header)
so a preflighted request passes too.

**What the player types, until the server browser page is approved.** The Multiplayer page's server field takes a
join code: a bare six-character code, or `join:CODE`, goes through the matchmaker of `data/matchmaker/client.json`;
`join:CODE@URL` names another matchmaker. In the browser an address cannot be dialled, so it is refused with a
plain reason ("In the browser, join a crew by its code") instead of the socket error. The list of crews is the
`browser` mockup state (task 3.6), built once the owner approves it.

**Proven by** a headless-Chromium run of the real web build against a local matchmaker and a listed local server:
the browser joins by code, its channels open, the server welcomes it and it walks aboard (task 2.3's browser half).

