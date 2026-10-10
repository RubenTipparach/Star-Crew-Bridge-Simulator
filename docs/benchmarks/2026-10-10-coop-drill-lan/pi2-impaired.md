# Co-op drill with 10 % loss and 50 ms delay, 2026-10-10 (openspec/changes/coop-drill task 5.2, one machine)

The drill under the server's test impairment (`--loss 0.1 --delay-ms 50`), against a run without it on the same
machine, build and seed.

| | |
| --- | --- |
| Board | Pi 2: Raspberry Pi 5, 4 GB (192.168.0.216) |
| Runs | `sc-server`, `sc-bot --station helm` (to 127.0.0.1) and `sc-bot --station tactical` (to 192.168.0.216), all on Pi 2 |
| Build | d23d683, release |
| Seed | 1 (the default) for both runs |
| Crew on foot | straight paths: no compiled deck on Pi 2 (Node is not installed, so `tools/deck/export_deck.mjs` cannot run) |

Not yet the second machine: Pi 1 had no server running and could not be reached from Pi 2 (SSH refused its host
key), so both bots ran beside the server. The impairment sits in the server's transport, so it hits every client
the same way whether it is on the same machine or not; the second machine adds only the real Wi-Fi.

## Rounds

| | Impaired (1500 s) | Unimpaired (700 s) |
| --- | --- | --- |
| Rounds finished | 15, all Victory | 7, all Victory |
| Engage s | 64-70 (13 of 15 at 70) | 70 (all) |
| Tern bolts hit | 3008/8456, 35.6 % | 1385/3981, 34.8 % |
| Hound bolts hit | 34.4 % | 34.0 % |
| Missiles hit | 80/88 | 37/42 |
| Tern hull left, MJ | mean 50, 18-93 | mean 49, 30-67 |

10 % loss and 50 ms on snapshots and stick input change nothing a round shows: the bots act on the snapshots that
arrive, and automation on the server flies and fights between them.

## Network (the server's final `/status`)

| Connection | Seconds | Down kbit/s | Up kbit/s | Dropped | Stick loss | Impaired messages |
| --- | --- | --- | --- | --- | --- | --- |
| Helm bot, impaired | 1497 | 84.8 | 9.3 | 0 | 9.4 % | 3316 |
| Tactical bot, impaired | 1497 | 84.2 | 7.8 | 0 | 0 | 2995 |
| Helm bot, unimpaired | 700 | 91.4 | 8.6 | 0 | 0 | 0 |
| Tactical bot, unimpaired | 700 | 90.7 | 7.1 | 0 | 0 | 0 |

The bots saw 10.0-10.1 % snapshot loss through the run, as asked; the down rate falls by about the share dropped.
Tactical sends no stick, so its stick loss stays 0. Server tick: 0.015 ms median, 0.043 ms p99, 0.076 ms max
(impaired); 0.008, 0.024, 0.033 ms (unimpaired).

**Round trip does not show the delay.** The bots reported 15-17 ms impaired and 17-20 ms unimpaired. Ping and Pong ride
the reliable Command channel (`sc-net/src/session.rs`), and the impairment touches only the unreliable ones, so the
F3 round trip is the link's alone; nothing yet measures how old a snapshot is when it arrives.

## Still to do for task 5.2

The same run with the Tactical bot on Pi 1 (`sc-bot --connect 192.168.0.216 --station tactical`), and with the
compiled deck's walk paths.
