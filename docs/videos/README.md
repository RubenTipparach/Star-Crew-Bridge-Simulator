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

## `2026-10-11-first-mission-fight-pi5.mp4` (8 min 3 s, 1280 x 720, 30 fps; `first-mission-gate` 5)

One whole fight against the Hound, played and recorded in one take on Pi 1: the game's own full-screen window captured
with `wf-recorder` (the monitor at 1280 x 720 for the take), re-encoded to H.264 High, yuv420p. No headless captures.
Single player at the Captain's seat (`sc-client --solo --station captain --bot --look-in 20`) with seven NPC officers,
eight crew in all, each named; the game's automation plays the captain's own seat, and `--look-in` goes round the
crewed consoles through the crew tabs every 20 s, as a captain looking in on each station does. Commit 89a9a82, protocol
7. Result: Victory in 7:31, the Tern at 222 of 400 MJ of hull, 8 of 14 missiles hit (both ships).

The fight clock is in each console's title band; the video's time is the clock plus 12 s.

| Fight clock | Shows |
| --- | --- |
| before 0:00 | The briefing: the drill, its three objectives, the captain's orders, eight crew reading and readying |
| 0:00-0:20 | The captain's COMMAND page at red alert: the CREW panel names all seven officers |
| 0:20-0:40 | The ship plan, three decks, the damage-control teams as dots |
| 0:40-1:00 | Lt. Sil Saren's Helm console on AUTO: thrust, the scanner, attitude |
| 1:00-1:20 | Lt. Jaya Strandraari's Tactical: J1, J2 and T1 targeted, four turrets, two tubes, the 20-round magazine and DECOY |
| 1:20-2:00 | Engineering (unavailable: no power grid in the drill yet), then Science: the shield model and the contact |
| 2:00-2:40 | Both laser stations: CHARGE filling to fire, ARC, SPOT on the Hound's face, FIRE |
| 2:40-3:00 | The gunner's sight on its target |
| 3:00-3:20 | OBJECTIVES: the Hound at 72 %, the hull at 100 %, both fighters shot down |
| 3:20-3:40 | The bridge itself, lit red, the crew in their seats, the fight through the windows |
| 3:40-7:31 | The round again: the Tern's hull falls to 73 % by 4:30 and 56 % by 7:28; at 7:28 the ship plan shows a breached corridor and a fire in the medbay on deck B with a team at it, and the strip counts 1 fire and 2 rooms open to space |
| 7:31 | VICTORY and the debrief |

Against the First Mission Gate (`openspec/changes/first-mission-gate` 2):

| Step | In this take |
| --- | --- |
| 1-3 Lobby and chat, bots' chat, mission choice | No: not built (one mission, no chat) |
| 4 Crew walk the ship | Partly: the crew now start in rooms across three decks and walk up to the bridge (captures in `docs/screenshots/engine/`), but on screen here the briefing covers the walk |
| 5 Red alert, the bridge goes red | Yes: the band, STAND DOWN, and the bridge lit red at 3:20 |
| 6 Bots at Helm and Tactical, the captain sees them | Yes: the CREW panel and each console with its officer named |
| 7 The captain hops stations | Partly: he looks in on every station; he takes none (all stay on AUTO) |
| 8 Power, then shields | No: no power grid in the drill; shields are on Science's and Tactical's consoles but not set by the captain |
| 9 Attacked and damaged | Yes: hull to 56 %, a breach and a fire, damaged rooms on the plan |
| 10 Eight crew, five or more NPCs | Yes: eight crew, seven NPC officers |
| 11 Repairs | Partly: the teams go to damage and fires by themselves; no order and no repair game in this take |
| 12 A fire put out | Partly: a team at the medbay's fire at 7:28; the take ends before it is out |
| 13 Repaired after the fight | No: the round ends at the debrief |
| 14 Warp out | No: not built |
| 15 The crew survives | Yes: VICTORY |

## `2026-10-14-patrol-pi5.mp4` (333 s, 1280 x 696, 30 fps; `patrol-mission` tasks 6.1)

The patrol played on foot in single player on Pi 1 (Pi 5 8 GB), `sc-client --solo --mission patrol-shoals --station
tactical --playtest --window`: the scripted player walks the ship, looks out of the windows and sits at Tactical,
while the NPC crew walk to their seats and play the patrol by the bots' rule (`combat::patrol::decide`), saying so in
the chat. The game window alone, recorded with `wf-recorder` and re-encoded to H.264 High, yuv420p (`ffmpeg -vf
fps=30,scale=1280:-2,format=yuv420p -c:v libx264 -crf 30`); the capture ran at about 14 frames a second beside the
game. Times are the video's, about 27 s ahead of the drill's clock in the log.

| Time | Shows |
| --- | --- |
| 0:00-0:25 | Muster: the body walks up to the bridge and looks out of its starboard and port windows: the berth's cradle round the bow, the Tern's own hull below, docked at Tamsin Station; the countdown |
| 0:25-0:50 | To Tactical and sat down; engineering's NPC drops shore power, the computer reports the drive on line, the captain orders the helm to take her out, the clamps release |
| 0:50-1:30 | The departure flown under the berth's limits, the status strip's speed at 1 m/s, then 4 m/s; up again to watch the cradle slide past the bridge windows |
| 1:30-2:30 | Clear of the berth; to the mess's windows on deck B; back to Tactical; the jump drive spools for 40 s and the Tern jumps to the Pell beacon; science sweeps it |
| 2:30-4:10 | Up to the bridge windows at the beacon; the second spool and jump to the Shoals relay |
| 4:10-5:33 | The Hound at the relay: red alert, the drill's fight, and the playtest's tour of the decks |

Not in it yet: the jump's streaks and flash (the windows simply change), the starbase's model (a placeholder ring is
far off the starboard side and dark against the star), the console controls for UNDOCK, JUMP, SHORE and SWEEP (a
mockup awaits approval; the NPCs play them).
