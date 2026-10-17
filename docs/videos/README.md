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

## `2026-10-15-tamsin-undock-pi5.mp4` (344 s, 1280 x 678, 30 fps; `patrol-mission` design 12b, spacedock tasks 2.9)

Undocking from Tamsin Station on foot at the helm, in single player on Pi 1 (Pi 5 8 GB), `sc-client --solo --mission
patrol-shoals --station helm --playtest --window`: the scripted player walks onto the bridge, sits at the helm and
presses UNDOCK from the seat once the drive is green and the captain gives the order (the console's UNDOCK control
awaits its mockup's approval, so the script sends the command it will send, by `combat::patrol::decide`); then it gets
up and walks the ship while the Tern leaves the round bay, goes down the throat and clears the station. The NPC crew
play their stations. The game window's content alone, recorded with `wf-recorder` and re-encoded to H.264 High, yuv420p
(`ffmpeg -vf fps=30,scale=1280:-2,format=yuv420p -c:v libx264 -crf 30`). Times are the video's, about 4 s ahead of the
drill's clock.

| Time | Shows |
| --- | --- |
| 0:00-0:04 | Loading (black) |
| 0:04-0:12 | Muster: the body walks out of the corridor onto the bridge, past the captain's chair to the helm; through the forward windows the round bay's wall, its flood rings and ribs, and the throat at its end with the star in it |
| 0:12-0:36 | Sat at the helm, the console up from the desk: the briefing, then the helm console, its viewscreen on the bay and the throat |
| 0:36-0:41 | Engineering's NPC drops shore power, the drive comes on line, the captain orders "Helm, take us out", the script presses UNDOCK at the helm, the clamps release |
| 0:41-1:00 | The helm console while the Tern rises off its berth toward the axis at the bay's 5 m/s |
| 1:00-1:30 | Up and across the bridge: the bay's wall and its rings sliding past the windows |
| 1:30-2:30 | Down to the mess on deck B and back to the helm; the viewscreen ahead down the axis to the throat and the star |
| 2:30-4:00 | Up to the bridge windows in the throat, then across to Tactical (the drill's tour) |
| 4:00-5:44 | The tour of the decks (the hangar, the corridors); the computer reports "Clear of the berth" at about 5:04 |

Not in it yet: the iris's leaves (it stands open), the habitats' land and see-through windows, docked ships other
than the two Herons, the console controls for UNDOCK, JUMP, SHORE and SWEEP (awaiting approval).

## `2026-10-16-captains-viewscreen-on-foot-pi5.mp4` (160 s, 1280 x 678, 30 fps; `leave-station`, `captains-viewscreen`, `relieve-bots`)

The owner's three asks from the browser build ("why cant I get up during combat? and walk around? and see the captains
screen on the main view screen"), played on foot in single player on Pi 1 (Pi 5 8 GB), `sc-client --solo --station
captain --playtest --window`, commit 207fa21. The scripted player works the console through the same pointer and keys a
person uses: it clicks SCREEN where a pointer would, and gets up by holding E, the stand key, through the same hold
that fills LEAVE STATION's ring. The game window's content alone, recorded with `wf-recorder` and re-encoded to H.264
High, yuv420p (`ffmpeg -vf crop,fps=30,scale=1280:-2,format=yuv420p -c:v libx264 -crf 30`). Shots of each step are in
`docs/screenshots/engine/leave-station/`.

| Time | Shows | Shot |
| --- | --- | --- |
| 0:00-0:09 | Muster: the body walks onto the bridge to the captain's chair and sits; the console rises from the desk | 1 |
| 0:09-0:44 | The briefing over the captain's console, LEAVE STATION in its title band; the crew sit, READY, the countdown | 2 |
| 0:44-0:52 | Engage: the captain's console in the fight | 3 |
| 0:52-0:58 | TARGET on his viewscreen, then SCREEN (lit): his picture goes on every screen | 4 |
| 1:04-1:06 | E held: LEAVE STATION's keycap ring fills, and he stands; the captain's chair passes to automation, and a bot takes it | 5, 6 |
| 1:06-1:32 | On foot mid-fight, he walks to the bridge's main viewscreen and watches his picture on it: the Hound on TARGET | 7 |
| 1:32-1:44 | On to the starboard window | 8 |
| 1:44-1:49 | To the helm: "E  Take Helm from Lt. Sil Saren"; Saren stands and walks to the nearest unmanned station, Laser port (the captain's chair he would swap into was taken already) | 9 |
| 1:49-2:16 | Seated at the helm, the captain's TARGET picture on the helm's own VIEWSCREEN panel | 10 |
| 2:16-2:40 | Up again with E and back to the main viewscreen; the helm's NPC returns to the helm | |

Not in it: the CHASE press of the script did not take (the picture stays on TARGET), and bodies pass through each other
(Saren stands up through the player's eye at 1:45).

## 2026-10-16-browser-single-player-walk.mp4

The browser build (`openspec/changes/browser-page-fit`), played by mouse and keys in headless Chromium on the Pi 5's
GPU at 1280 x 720 (`tools/web/smoke.mjs` style, recorded by Playwright, re-encoded to yuv420p). A browser frame rate
says nothing about a Pi client; the game ran at about 6 frames a second here.

| Time | Shows |
| --- | --- |
| 0:00-0:15 | Loading, the front menu with no Quit, SINGLE PLAYER and BEAM ABOARD clicked where they are drawn |
| 0:15-0:31 | Muster: the body walks up the corridor onto the bridge to the captain's chair; the round waits for it |
| 0:31-0:40 | "E  Take Captain from ..."; sitting is the player's READY, and the countdown starts once the crew are seated |
| 0:40-0:44 | Engage: the captain's console |
| 0:44-0:52 | E held: up from the chair, on foot on the bridge in red alert, walking back |
| 0:52-0:56 | Esc: the game menu, Resume, Options and Return to menu, with no Exit in the browser |

## `2026-10-16-chase-smooth-before-after.mp4` (14 s, 1280 x 880, 30 fps; `chase-orbit`)

The owner's "can we get some smooth ness to the rotation there?": the captain's viewscreen through the chase-camera
film's cameras (FWD off and on, TARGET, PORT, AFT, STBD with CHASE on), the same seeded single-player round rendered
headless on Pi 1 (Pi 5 8 GB) by `sc-client --headless --solo --station captain --film --shots DIR`. Top: before
(commit e4c43b3), snapshots drawn by their arrival time, the 10 Hz shudder measured in `chase-orbit` design 1 (the
heading drawn advanced 6.2, 10.6 and 8.8 mrad a frame in turn), and each camera a cut. Bottom: after, the snapshots
played back on the server's clock, the chase camera following the turn with its 0.4 s lag, and each change easing over
0.5 s. Encoded H.264 High, yuv420p. The film steps the game 1/30 s a frame, so it shows what is drawn, not the Pi's
frame rate.

## `2026-10-16-chase-orbit.mp4` (45 s, 1280 x 720, 30 fps; `chase-orbit`)

ORBIT, filmed the same way with `sc-client --headless --solo --station captain --film-orbit --shots DIR` and
`tools/video/film.sh`, then scaled and encoded H.264 High, yuv420p. The scripted orbit moves the hand's goal as a drag
and the wheel would; the rig smooths it as it does a player's. Shots in `docs/screenshots/engine/chase-orbit/`.

| Time | Shows | Shot |
| --- | --- | --- |
| 0:00-0:13 | The walk to the captain's chair, sitting, the countdown | |
| 0:13 | FWD from the bow | |
| 0:14-0:20 | CHASE on: the camera eases out behind the Tern and follows its turns; ORBIT's keycap becomes available | 1 |
| 0:20-0:24 | TARGET: the camera eases round to look past the Tern at the Hound | 2 |
| 0:24-0:34 | ORBIT on (lit): once round the ship, below and above it, in and out | 3, 4, 5 |
| 0:34-0:37 | ORBIT off: the camera eases back to the chase position | 6 |
| 0:37-0:45 | On foot behind the chair: the bridge's main viewscreen shows the captain's orbit | 7 |

## `2026-10-16-nebula-skies-pi5.mp4` (45 s, 1280 x 720, 30 fps; `nebula-skies`)

The nebula sky in the engine: the Shoals' sky A, Bloom, and its vector stars, filmed with `sc-client --headless --solo
--station captain --film-orbit --shots DIR` on Pi 1 (Pi 5 8 GB) and `tools/video/film.sh`, then scaled and encoded
H.264 High, yuv420p. The film steps the game 1/30 s a frame, so it shows what is drawn, not the Pi's frame rate (that
is `docs/benchmarks/2026-10-16-nebula-skies-pi5/`). Shots in `docs/screenshots/engine/nebula-skies/`.

| Time | Shows | Shot |
| --- | --- | --- |
| 0:00-0:06 | The walk to the captain's chair on the bridge, the nebula in the viewscreens | |
| 0:06-0:13 | Seated: the console comes up, the countdown | |
| 0:13-0:20 | FWD, then CHASE on: the Tern in front of the cloud and the sun; the stars streak as the camera turns with the ship | 4 |
| 0:20-0:24 | TARGET: the camera eases round, the stars streak, then settle to points | |
| 0:24-0:34 | ORBIT: the camera turned round the Tern, every star a short line toward where it was | 5, 6 |
| 0:34-0:37 | ORBIT off: the camera eases back | |
| 0:37-0:45 | On foot behind the chair: the nebula through the bridge's windows and on the main viewscreen | 3 |


## `2026-10-17-manual-start.mp4` (29 s, 1280 x 720, 30 fps; `start-mission`, the gunner's pod)

The round waiting for START MISSION, and the gunner climbing out of the dorsal pod, played on foot in single player on
Pi 1 (Pi 5 8 GB), `sc-client --solo --station gunner --playtest` (full screen), commit f38fcfe with the deck compiled
by its `deckc` (the pods cut out of the hull). The scripted player walks, climbs and sits as a person does, presses
START MISSION 5 s after the chip shows, and gets up by holding E. Recorded with `wf-recorder` from the centre 1920 x
1080 of the 2560 x 1080 monitor (the HUD anchored to the monitor's edges is cut at the left), then cut, captioned and
re-encoded to H.264 High, yuv420p. Two cuts: 0:00-0:17 is the take's 0:03-0:20, 0:17-0:29 its 0:52-1:04. Shots in
`docs/screenshots/engine/manual-start/`.

| Time | Shows | Shot |
| --- | --- | --- |
| 0:00-0:05 | Deck A's command passage to the dorsal turret access, up the pod's ladder ("Ladder [E] Climb up"), E at Gunner | 1 |
| 0:05-0:11 | Seated: the briefing, START MISSION (Return, 2/8 ready) in the title band, READY; no countdown, the round waits | 2 |
| 0:11-0:16 | START MISSION pressed: "Launch in" counts 0:05 to 0:01 | 3 |
| 0:16-0:17 | Engage: the gunner's sight, red alert | 4 |
| 0:17-0:20 | In the fight, E held: LEAVE STATION, the body stands in the pod | |
| 0:20-0:29 | Standing: "Gunner [E] Sit down [Q] Climb down"; the body climbs out to deck A and walks forward | 5, 6 |
