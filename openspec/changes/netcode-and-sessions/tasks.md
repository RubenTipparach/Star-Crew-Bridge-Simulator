# Tasks

Nothing here starts until the owner asks for implementation (CLAUDE.md section 4), and it
follows `engine-stack` tasks 2 and 3.

## 1. Transport

- [ ] 1.1 `sc-net` packet header, challenge and response, keep-alive and timeout; a test that a packet from the wrong session token is dropped.
- [ ] 1.2 Acknowledgement bitfield and the reliable ordered channel with resend; a test over a simulated link with 10% loss and 150 ms jitter that every command arrives once, in order.
- [ ] 1.3 Bulk channel with fragmentation and reassembly; a test that a 40 kB transfer survives 10% loss.
- [ ] 1.4 Field validation on decode: ranges, non-finite rejection, a counter of dropped packets.

## 2. Snapshots

- [ ] 2.1 Snapshot groups and quantization (design section 4) in one schema shared by both ends; round-trip tests at the quantization limits.
- [ ] 2.2 Delta compression against the last acknowledged snapshot; keyframe on join and after 1 s without acknowledgement.
- [ ] 2.3 Per-group send rates and priorities; a measurement of bytes per second down with eight avatars and twelve bodies, recorded in this design.

## 3. Prediction and interpolation

- [ ] 3.1 `sc-core` avatar movement and craft flight as pure functions of state and input, used by server and client.
- [ ] 3.2 Client-side prediction with replay from the last acknowledged state; correction blending under 5 cm.
- [ ] 3.3 Interpolation 100 ms in the past, extrapolation up to 100 ms, then hold.
- [ ] 3.4 Lag compensation for manned turrets: 300 ms of exterior history, rewind capped at 200 ms.

## 4. Commands and seats

- [ ] 4.1 Command messages, server-side seat checks and clamping, stable per-tick ordering, "refused" replies with reasons.
- [ ] 4.2 Seat table on the server; claim, release, automation hand-over; disconnect leaves the avatar as an NPC; a test for star-crew-64's frozen-helm bug.

## 5. Sessions

- [ ] 5.1 Listen server on a client thread; dedicated `sc-server` binary; solo as a listen server with one client.
- [ ] 5.2 LAN discovery broadcast; join by address and port.
- [ ] 5.3 Join in progress with protocol, layout digest and data digest checks.
- [ ] 5.4 Campaign save at mission end through a temporary file and rename, versioned, with a migration test.

## 6. Measure and move to specs

- [ ] 6.1 Measure bandwidth and server CPU with eight clients against a Pi 3 listen server on Wi-Fi; record in `docs/benchmarks/`.
- [ ] 6.2 Move each `netcode` requirement into `openspec/specs/netcode/spec.md` with the test that proves it.
