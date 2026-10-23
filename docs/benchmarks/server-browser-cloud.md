# Server browser: the cloud client (2026-10-23)

Asked by the Mac session for the owner's cross-network test (`openspec/changes/matchmaker` design 10): join the
crew PI-1 hosts at home, from a Claude Code cloud container on another network. No code was changed.

## Where it ran

- Branch `feat/server-browser` at `09e8552` (after `125fd41`), `cargo build --release -p sc-client`.
- A Claude Code cloud container (Linux, no GPU). Its outbound traffic goes through an HTTP(S) proxy.

## Results

| Step | Result |
| --- | --- |
| Matchmaker `GET http://star-crew-bridge-simulator.fly.dev/v1/servers` | HTTP 200, `{"servers":[]}` at 22:48 UTC and every 20 s until 23:01 UTC |
| `sc-client --browse` | `sc-client: no crews listed` |
| "PI-1 test crew" listed | No: not seen in the 12 minutes checked, so there was no code to join |
| `--join CODE --station tactical --bot` | Not run: nothing to join |
| Outbound UDP (STUN binding request) | Blocked: no answer from `stun.l.google.com:19302` (74.125.250.129) or `stun.cloudflare.com:3478` (162.159.207.0), 3 s each |

## What it shows

- The matchmaker on Fly answers from outside the owner's network, and the client's `--browse` reads it.
- This cloud container cannot test STUN hole punching: it has no outbound UDP, so the client would offer no
  server-reflexive candidate, and a WebRTC join would stop after the offer and answer, with the data channels never
  opening. That is the container's network policy, not the game; allowing it needs the cloud environment's network
  access widened (https://code.claude.com/docs/en/cloud-environments#network-access), and plain UDP may still not be
  offered there.
- The cross-network test needs a client on a real network other than PI-1's: a phone hotspot, or a machine away from
  home.
