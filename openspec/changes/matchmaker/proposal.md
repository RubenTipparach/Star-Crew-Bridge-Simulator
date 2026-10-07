# Proposal: a matchmaker on Fly.io, so browser and desktop players find a crew by code

## Why

The owner, 2026-10-07: "what if we stood up a matchmaking service on fly io? would that help
unify web vs desktop users?", then "I want to use webrtc if posible to do multiplayer on web and
desktop".

Every client now connects to the server with WebRTC (`netcode-and-sessions` section 2). WebRTC
cannot set up a connection on its own: the two ends must first swap a description of themselves
(an offer and an answer) and their candidate addresses, through something both can reach. That
is a **signalling service**, and a browser makes it unavoidable:
- a page served over HTTPS cannot open a connection to a home address like `192.168.1.20`, so a
  browser player can never reach the main server's own endpoint;
- the main server is a Pi behind a home router, which nobody outside can reach without port
  forwarding.

A small service on the public internet that the server keeps a connection open to solves both:
players reach the service, the service introduces them to the server, and WebRTC then finds the
direct path between them (or a relay). Fly.io suits it: one small always-on machine, TLS at its
edge, WebSockets supported.

**Does it unify web and desktop players?** Yes, as the meeting point. A browser player and a Pi
player join the same way, by typing a six-character code, and reach the same server over the same
transport. What makes them equal once connected is WebRTC everywhere (the owner's decision); the
matchmaker is what lets both of them find the server without anyone forwarding a port.

## What Changes

- **A matchmaker service**, `sc-matchmaker`, in the workspace, deployed to Fly.io: a session
  directory keyed by join codes, the signalling relay (offers, answers, ICE candidates), and the
  ICE server list (STUN, and short-lived TURN credentials) handed to both ends of each join.
- **The main server keeps a WebSocket open to it**, registers its session, and is given a join
  code. It reconnects and re-registers with the same code after either end restarts.
- **Clients join by code**: native and browser alike. The matchmaker checks the protocol and
  content versions before introducing anyone; the server checks again.
- **It holds no game state and decides nothing.** It never sees game traffic, apart from a TURN
  relay forwarding encrypted bytes for the connections that need one. A mission keeps going if
  the matchmaker goes down.
- **TURN from a managed service**, not on Fly (design section 4).

## Capabilities

### New Capabilities

- `matchmaking`: finding a session by code, the connection setup between a client and a server,
  and the ICE servers each end is given.

### Modified Capabilities

None. (`netcode` owns the transport and the session rules; this change adds how a client finds
and reaches a server.)

## Impact

- New crate `sc-matchmaker` (a headless service; never in the game client).
- `sc-net` gains the signalling message types, one schema for the matchmaker, the server, native
  clients and the browser's JavaScript module.
- `sc-server` gains the matchmaker connection; the server's LAN signalling endpoint
  (`netcode-and-sessions` section 8) speaks the same messages.
- A Fly.io app (the owner's account), a managed TURN account, and their secrets, kept out of the
  repository.
- No code exists yet. Nothing here is built until the owner asks for it (CLAUDE.md section 4).
