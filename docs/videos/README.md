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
looking aft over the seats to the muster point. Recorded with `wf-recorder` at 30 fps while the game drew. The server
was restarted first, so every body starts standing at the back. Commit 6c260b3, protocol 4.

| Time | Shows |
| --- | --- |
| 0:00 | The empty bridge in Muster; the band names every station "auto" |
| 0:04 | Lt. Arlo Venn (Pi 1's Helm bot) joins at the back, "to Helm" under his name, walks forward round the captain's chair and sits at Helm |
| 0:10 | Alone and ready, he starts the round: Countdown, then Engage |
| 1:06 | Ens. Dara Holt (Pi 2's Tactical bot, over the LAN) joins mid-fight at the back, "to Tactical", walks to the starboard seat and sits; Tactical leaves automation |
| 2:00 | Round 0 ends: victory in 103 s, Tern hull 27 MJ |
