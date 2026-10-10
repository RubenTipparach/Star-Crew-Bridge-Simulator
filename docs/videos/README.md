# Videos

## `2026-10-10-coop-drill-pi5.mp4` (100 s, 1280 x 720)

The first co-op drill (`openspec/changes/coop-drill`), recorded on Pi 1 (Pi 5 8 GB, Raspberry Pi OS desktop) with
`wf-recorder` from the centre 1920 x 1080 of the 2560 x 1080 monitor, scaled. Everything ran on Pi 1: `sc-server`,
the Helm client shown (`sc-client --connect 192.168.0.210 --station helm --bot`, native Wayland) and a headless
Tactical `sc-bot`, all over WebRTC on the Pi's LAN address. The capture ran at about 15 frames a second (the Pi
copying and encoding the screen while it draws); the game itself held 26 FPS median.

| Time | Shot |
| --- | --- |
| 0:00-0:10 | The briefing: the drill's situation, objectives, Helm's orders, the crew reading, then READY and the countdown |
| 0:10-0:20 | Engage: the Hound appears 3.8 km out and above; the target camera picks it up; Helm closes at 300 m/s |
| 0:20-0:45 | In turret range: Helm slows to 80 m/s and weaves with the bow on the Hound; bolts both ways, missiles off the bow |
| 0:45-1:07 | The Hound orbits above and below; the scanner shows it on its stalk; it breaks up at 0:57 (round time) |
| 1:07-1:27 | The debrief: victory in 57 s, 190 of 580 turret hits, 4 of 4 missiles, hull 93 of 120 MJ |
| 1:27-1:40 | The next round's briefing |

## `2026-10-10-crew-on-foot-pi5.mp4` (132 s, coop-drill design 9, task 6.4)

Pi 1's screen (2560 x 1080), `sc-client --connect 192.168.0.210 --watch`: the bridge's overview from the bow end,
looking aft over the seats to the muster point. Recorded with `wf-recorder`, then re-encoded to H.264 High, yuv420p,
1920 x 810 at a constant 30 fps (`ffmpeg -vf fps=30,scale=1920:-2,format=yuv420p -c:v libx264 -crf 23`):
wf-recorder's own file was 4:4:4 at a variable rate, which most players show as black. The server
was restarted first, so every body starts standing at the back. Commit 6c260b3, protocol 4.

| Time | Shows |
| --- | --- |
| 0:00 | The empty bridge in Muster; the band names every station "auto" |
| 0:04 | Lt. Arlo Venn (Pi 1's Helm bot) joins at the back, "to Helm" under his name, walks forward round the captain's chair and sits at Helm |
| 0:10 | Alone and ready, he starts the round: Countdown, then Engage |
| 1:06 | Ens. Dara Holt (Pi 2's Tactical bot, over the LAN) joins mid-fight at the back, "to Tactical", walks to the starboard seat and sits; Tactical leaves automation |
| 2:00 | Round 0 ends: victory in 103 s, Tern hull 27 MJ |

## `2026-10-10-first-mission-baseline-pi5.mp4` (104 s, 1280 x 720, 30 fps; `first-mission-gate` 5)

The baseline for the first mission gate: today's whole mission, played and recorded in one take on Pi 1, the game's own
full-screen window captured with `wf-recorder` (the monitor at 1280 x 720 for the take, `first-mission-gate` 6). No
headless captures and no scripted cameras. On screen is Capt. Ilsa Marr's client (`sc-client --connect --station captain
--bot`: the game's automation plays her seat); the other four stations are `sc-bot` crew on Pi 1 (Lt. Arlo Venn at Helm,
Ens. Dara Holt at Tactical, Lt. Cmdr. Oren Vask at Engineering, Ens. Mira Solace at Science), joining 2 s apart. Pi 2
was asked to fly Tactical and could not start a background process on its side, so this take is one machine. Result:
Victory in 70 s, 68 of 120 MJ of hull left. Re-encoded to H.264 High, yuv420p, from the recorder's full-range file.

| Time | Shows | Gate step |
| --- | --- | --- |
| 0:00 | The Captain walks onto the bridge to her chair ("Walking to Captain") | 4 in part (the bridge only) |
| 0:02 | The briefing; the crew list fills as each bot joins and readies; "ENGAGING IN" | 1 in part (no lobby room, no chat) |
| 0:09 | The Captain console at NORMAL | |
| 0:11 | Red alert from the Captain's ALERT panel: the band turns red, the button becomes STAND DOWN | 5 in part (the bridge's light stays normal) |
| 0:11 | CREW panel: a bot at Helm, Tactical, Engineering and Science | 6 in part (who holds each station; no view of their dashboards) |
| 0:14-1:18 | The fight against the Hound from the Captain's seat: viewscreen, attitude, orders | 9 in part (hull damage only) |
| 1:20 | VICTORY: 1:09, 203 of 569 turret hits, 5 of 6 missiles, 479 MJ dealt, 283 MJ taken | 15 |
| 1:42 | The next drill's briefing, everyone READY | |

Not in it, because the game cannot do them yet: the lobby room and chat, a mission choice, walking the decks, the
captain looking in on or taking another station, power, NPC crew, system damage, repairs, fires and warp
(`first-mission-gate` 3).
