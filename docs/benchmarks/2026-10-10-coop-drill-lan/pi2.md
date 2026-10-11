# Co-op drill across two Pis, 2026-10-10 (openspec/changes/coop-drill task 5.1)

The first run on two machines: the server and the Helm bot on Pi 1, the Tactical bot on Pi 2, over the house LAN.

| | Pi 1 | Pi 2 |
| --- | --- | --- |
| Board | Raspberry Pi 5, 8 GB | Raspberry Pi 5, 4 GB (4049 MB, about 3300 available, 2047 MB swap) |
| Address | 192.168.0.210 | 192.168.0.216 |
| Runs | `sc-server` (crew on foot, paths from the compiled deck) and `sc-bot --station helm` | `sc-bot --station tactical` |
| Build | 25e160c, release | 25e160c, `cargo build --release -p sc-server --bin sc-bot -j 3`, 4 min 45 s on the Pi |

Pi 2's session could not run a prebuilt bundle served over the LAN (its permission check blocked the download), so it
built the bot from source. The bot joined mid-fight (round 3, Engage), took Tactical at the next Muster and stayed.

## Rounds with both machines aboard

| Round | Result | Engage s | Tern bolts hit | Hound bolts hit | Missiles hit | Tern hull left, MJ |
| --- | --- | --- | --- | --- | --- | --- |
| 4 | Victory | 68 | 212/692 | 112/338 | 4/6 | 96 |
| 5 | Victory | 70 | 232/708 | 114/344 | 5/6 | 83 |
| 6 | Victory | 59 | 204/592 | 89/290 | 4/4 | 107 |
| 7 | Victory | 52 | 179/524 | 106/258 | 4/4 | 68 |
| 8 | Victory | 70 | 240/712 | 136/344 | 5/6 | 45 |
| 9 | Victory | 49 | 175/484 | 99/240 | 4/4 | 67 |

Six of six won, 49-70 s. The three rounds before Pi 2 joined (Helm bot, Tactical on automation) were won in 115-127 s
with 2-47 MJ of hull left: the second machine's crew member is worth it, as design 1 says it should be.

## Network (the server's `/status`, at 1197 s)

| Connection | Seconds | Down kbit/s | Up kbit/s | Dropped | Stick loss |
| --- | --- | --- | --- | --- | --- |
| Pi 1's Helm bot (loopback address) | 1197 | 84.2 | 8.7 | 0 | 0 |
| Pi 2's Tactical bot (LAN) | 625 | 84.2 | 7.2 | 0 | 0 |

84 kbit/s down per client against 80 measured before the bodies: the 10-byte bodies at 20 Hz add about 3 kbit/s for
two bodies, as design 9 estimated. Server tick: 0.020 ms median, 0.044 ms p99, 0.10 ms max.

Not yet: the same with `--loss 0.1 --delay-ms 50` between the machines (task 5.2), and the Mac, which has not
answered.
