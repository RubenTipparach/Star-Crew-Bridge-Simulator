# Long play on Pi 1, 2026-10-14

Pi 5 8 GB (`raspberrypi`, Wi-Fi at 192.168.0.210), release build of `feat/combat-2` at 63d9d06 (fire, safety
points, console ship model, main merged in). One `sc-server --rounds 6 --seed 1014` and two headless bot clients
(asked for Helm and Engineering), all on Pi 1 and all connecting to the Pi's LAN address, so every packet went
through the whole WebRTC stack. A sampler read `GET /status` and every process's memory once a minute
(`samples.txt`, 55 samples). PI-2 (Tactical) and the Mac (Science) were asked to join; neither did, so this is a
two-client run.

## Rounds

56 minutes, six rounds, no crash, no disconnect.

| Round | Outcome | Engage | Tern bolts hit | Hound bolts hit | Missiles hit | Dealt | Taken | Tern hull left |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 0 | Victory | 154 s | 111 / 856 | 267 / 583 | 2 / 4 | 1,593 MJ | 900 MJ | 400 MJ |
| 1 | Victory | 563 s | 896 / 2,912 | 699 / 1,856 | 9 / 12 | 4,896 MJ | 3,787 MJ | 14 MJ |
| 2 | Victory | 495 s | 763 / 2,516 | 653 / 1,655 | 9 / 12 | 4,703 MJ | 3,478 MJ | 273 MJ |
| 3 | Victory | 504 s | 723 / 2,488 | 639 / 1,653 | 7 / 12 | 4,143 MJ | 3,767 MJ | 17 MJ |
| 4 | Withdrew | 600 s | 975 / 3,029 | 783 / 1,936 | 7 / 12 | 5,573 MJ | 4,291 MJ | 181 MJ |
| 5 | Victory | 534 s | 932 / 2,849 | 668 / 1,753 | 8 / 14 | 5,171 MJ | 2,566 MJ | 150 MJ |

Rounds 1 to 5 run far longer than the 55 to 70 s drills of 2026-10-10: the Hound now fights through repairs, fires
and shields, and twice left the Tern under 20 MJ of hull. Round 4 hit the 600 s limit and the Hound withdrew.

The Engineering bot sat at Tactical every round. That is the bot posts rule (`data/stations.json` `bot_posts`,
coop-drill design 9): a bot takes the most important empty station, and Tactical outranks Engineering.

## Server

| | Measured |
| --- | ---: |
| Tick p50, whole run | 0.28 ms |
| Tick p99, whole run (per-minute p99 ranged 0.53 to 1.57 ms) | 1.07 ms |
| Tick max, whole run (per-minute max reached 8.31 ms once) | 4.88 ms |
| `sc-server` memory | 9 MB, flat for 56 minutes |

## Clients

| | Measured |
| --- | ---: |
| Headless client memory | 125-126 MB at the start, 117-120 MB at the end: no growth |
| Down (server to client), averaged over 3,315 s | 122.6 and 123.5 kbit/s |
| Up (client to server) | 7.1 and 9.0 kbit/s |
| Stick loss, stale sticks, messages dropped, refused writes | 0, 0, 0, 0 |

## Findings

1. **The downstream is over its budget.** `netcode-and-sessions` 10 budgets 80 kbit/s down per client; the
   2026-10-10 drill measured 80. This hour averaged 123 kbit/s, 54 % over. Long fights carry more bolts and
   missiles, and protocol 13 added fires, smoke and bracket state to the damage snapshot. Which share is which is
   not yet measured; it needs a per-message byte count from the server before anything is cut.
2. **Nothing leaks.** Server and clients held their memory for the whole hour.
3. **The tick has room.** A worst minute of 1.57 ms p99 is under 5 % of the 33 ms tick.
4. **Peers.** The run waited on two Remote Control sessions that never picked up the message. The next
   long play should be started on each machine by the owner (CLAUDE.md: the owner starts the work).

Files: `server.log` (rounds and the final `/status`), `samples.txt` (per minute), `pi1-helm.log`,
`pi1-engineering.log`.
