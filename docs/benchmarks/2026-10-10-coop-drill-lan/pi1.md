# Co-op drill on Pi 1, 2026-10-10

Pi 5 8 GB (`raspberrypi`, Raspberry Pi OS bookworm desktop, Wi-Fi at 192.168.0.210), release build with
`-C target-cpu=cortex-a76`. `sc-server`, a Helm client and a Tactical client, all on Pi 1 and all connecting to the
Pi's LAN address, so the packets go through the whole WebRTC stack (DTLS, SCTP, ICE) but not over the air. The
two-machine runs are separate files here.

## Drills

Rounds played on Pi 1 in this session (server log), all with bots at both stations: every one a victory.

| Run | Engage | Tern turret hits | Missiles | Tern hull left |
| --- | ---: | --- | --- | ---: |
| three processes, first real-time run | 70 s | 227 / 712 | 5 / 6 | 92 MJ |
| captures (headless clients) | 66 s | 222 / 672 | 4 / 6 | 98 MJ |
| video, take 1 | 67 s | 249 / 684 | 4 / 6 | 94 MJ |
| video, take 2 | 55 s | 184 / 556 | 4 / 4 | 96 MJ |
| video, take 3 (the committed video) | 58 s | 190 / 580 | 4 / 4 | 93 MJ |

The persistent server also ran rounds 0-10 with a Helm bot and, for most of them, Tactical on automation or a
headless client; its log was not kept.

## Network and server, from `GET /status` at the end of the video's take 3 (95 s connected)

| Per client | Measured | Budget (`netcode-and-sessions` 10, `engine-stack` 5) |
| --- | ---: | ---: |
| Down (server to client), whole UDP datagrams | 79.7-80.3 kbit/s | 80 kbit/s |
| Up (client to server) | 7.0-8.3 kbit/s | 32 kbit/s |
| Stick loss, stale sticks, messages dropped on decode, refused writes | 0, 0, 0, 0 | |

| Server | Measured | Budget |
| --- | ---: | ---: |
| Tick cost p50 / p99 / max (the drill's step, events, snapshots) | 0.014 / 0.060 / 0.114 ms | 2 ms per ship per 30 Hz tick |

The down rate sits on the budget: the design's estimate was 69 kbit/s of snapshots plus about 6 of events. The
snapshot is sent whole at 20 Hz to every client (no deltas yet), about 370 bytes plus WebRTC's headers, and every
bolt is a reliable `fired` event (a full fight is about 25 bolts a second). Deltas, or sending turret aims only to
the Tactical console, are the first savings when it matters.

## Client frame rate (Mesa's `GALLIUM_HUD`, native Wayland, vsync off, 2560 x 1080)

| Run | Median FPS | 5th percentile |
| --- | ---: | ---: |
| Helm client with the bot, while `wf-recorder` captured the screen (take 1) | 27.6 | 20.3 |
| the same, take 3 | 26.4 | 21.8 |

The drill draws far less than the walkable ship (the sky, a 140-triangle Hound, a few dozen bolts, the UI, and the
target camera's second view), yet runs at about the same rate. The cost is not the scene: the 1280 x 720 3D target
with 4x MSAA, its resolve and blit to a 2560 x 1080 output, the sky's full-screen shader (twice, with the target
camera) and egui. To be measured without the recorder, and against the walk test's 28 FPS (`../2026-10-10-pi5-8gb-walk`).
