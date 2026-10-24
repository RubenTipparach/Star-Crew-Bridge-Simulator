# Design: recorded playtests of every seat, and of a retreat and repair

## Context

| Decided elsewhere | Where |
| --- | --- |
| The scripted player is a capture instrument: it plays through the same body and keys a player does | `playtest.rs`, CLAUDE.md 4 |
| A playtest shows the game as it is played, walking the ship | CLAUDE.md 12 |
| Standing up: hold E, or LEAVE STATION; the gunner's pod ladder on the down key | `leave-station`, the 2026-10-17 regression |
| Fires and repairs on deck, with no damage-control teams under `--all-hands` | `all-hands` |
| Machines on the owner's hardware test the game; the cloud coordinates | CLAUDE.md 12, 13 |

## 1. The seat tour

`Playtest::seats()`: for each station in `Station::ALL` that has a seat aboard, `To(station)`, `Sit`, `Work(4 s)`,
`Stand`, then the next; after the last, the run ends. Each beat already logs as it finishes; the tour adds one line a
seat, `playtest: seat <id>: sat in <t> s, stood in <t> s`, or `stuck at <beat> after <t> s` when a beat runs past
60 s, and skips to the next seat, so one bad seat does not hide the others. The gunner's seat is in the dorsal pod at
the top of a ladder: getting there and back climbs it, which is the owner's bug.

## 2. The Mastiff

A heavy corvette, the Hound's model and insides (the client draws every enemy with the Hound's mesh and
`data/ships/hound/layout.json` holds its compartments), with the numbers of a ship that wins a straight fight:

| | Hound | Mastiff |
| --- | ---: | ---: |
| Hull, MJ | 250 | 700 |
| Shield faces, MJ | 50 x 6 | 90 x 6 |
| Shield regeneration, MJ/s | 1.5 | 3.0 |
| Turrets | 2 | 4 |
| Gun aim spread, deg | 0.3 | 0.2 |
| Missile tubes, load time | 2, 60 s | 2, 60 s |
| Orbit, m | 1,800 | 1,400 |

Flight is the Hound's but slower (top speed 160 m/s against the Tern's 400, approach 160 m/s, orbit 140 m/s), so a
retreat can open the range. The first numbers were 260, 260 and 180: the second run (section 3a) showed a Mastiff that
kept the fleeing Tern inside its guns' 2.5 km.
`drill-mastiff` is `drill-hound` with this enemy and its own title. Recommendation taken (ask only with screenshots):
these numbers; the captures show what they do.

## 3. The retreat and repair

`Playtest::retreat()`, at the helm: `To(Helm)`, `Sit`, `Engaged(30 s)` (the exchange that hurts the ship), then the
retreat: `Press(Flip)` (the bow away), `Press(FullAhead)` (the throttle dragged to its top as a pointer would),
`Work(25 s)` while the range opens; `Stand`, then `Locker`, `Fight`, `Repair` until nothing burns and nothing is broken,
then back to the helm. Two new presses: FLIP's button, and the throttle lever pressed at its foot and dragged to its
top. Run with `--all-hands` so the crew, not damage-control teams, fix the ship.

## 3a. What the first run found, and the fixes

The first filmed run (2026-10-24, cloud, 720p, `--record`) lost the Tern: "Defeat in 148 s", hull 0 MJ, 1,914 MJ taken.
FLIP and the throttle worked (the bow came round to 180 degrees in 12 s and the speed rose from 60 to 115 m/s), but the
retreat was undone twice over:

- **The script got up too soon.** A fire burned 8 s into the 20 s run, and the seated "answer a fire" rule stood the
  player up with the range still closing. In the retreat that rule waits: seated, the script does not get up for a
  fire before its run beat is done; on foot it answers every fire and every repair as soon as it sees one.
- **The bot that took the empty helm threw the player's course away.** Within seconds it said "Bringing her about onto
  the Mastiff heavy corvette" and flew bow on into the fight, which breaks a rule CLAUDE.md section 7 already states:
  "Leaving a station changes who operates it, never what it holds." The fix: **a bot that sits at the helm during an
  engagement holds the heading and speed it found there** for the rest of the round. It steers nothing, and it does
  not acknowledge an order it is not flying (a holding helm that said "Aye, closing on the Mastiff" would be saying
  something it is not doing, crew-chat design 3). A player who sits back down flies again, and a new round starts as
  every round does. A bot seated at the helm from the round's start hunts as before. Recommendation taken (ask only
  with screenshots).
- **The run did not end with the round.** After the defeat it went on fighting fires aboard a dead ship. A scripted
  run now ends 8 s into the Debrief.

The second run (the holding helm in) flew the retreat right: the bow at 180 degrees, the helm held by the bot that sat
down, 272 m/s at the end of the run with the hull at 81%. It still lost the ship in 118 s, because a Mastiff at
260 m/s kept the Tern inside its guns' 2.5 km. Its speeds come down (section 2): a heavy corvette wins a straight
fight and loses a chase. Recommendation taken (ask only with screenshots).

The third run, measured (`retreat_trace` in sc-core, an ignored test that prints the bare drill's retreat, and a line
every 5 s in a recorded run's log): the retreat now works. The range passes the guns' 2.5 km at 55 s and is 18 km at
120 s, the helm holding 400 m/s with the bow 179 degrees off the Mastiff. The ship was still lost at 123 s, to
missiles: four tubes reloading every 30 s chase a fleeing Tern for their 60 s and took 200 MJ and then the last
165 MJ at 9 and 18 km. The Mastiff's tubes go back to the Hound's (two, 60 s): its edge is its guns, hull and shields,
which a retreat outruns; its missiles are as many as point defence already meets. In the bare drill the Tern then
ends the run with 325 of 400 MJ. Recommendation taken (ask only with screenshots).

The filmed run (2026-10-24, `docs/videos/retreat-and-repair.mp4`, 960 x 540 at 15 frames a second, 5 min 24 s): the
retreat holds (61 km from the Mastiff at 300 s, nothing hitting the Tern after the run), and on foot the player fights
ten fires and plays the Gannet tubes and hoist repair game. The Tern is still lost at 302 s, burned from about 300 MJ
to 0 by the fires: with `--all-hands` there are no damage-control teams, the seven bot crew stay at their stations, and
one player with an extinguisher that runs dry after 15 s does not keep up with fires that spread. That is the
all-hands rule working as written, not a fault in the retreat; whether one player alone should be able to save the
ship (bots that leave a station to fight fires, a bigger extinguisher, slower spread) is a question for the owner with
the video to look at.

## 4. The recorded runs

- **PI-1, the seat tour, real time:** `sc-client --solo --playtest-seats --window`, recorded with the Pi's screen
  recorder, committed as `docs/videos/seat-tour-pi1.mp4` with the run's log beside it.
- **The cloud, the retreat:** `sc-client --headless --solo --station helm --mission drill-mastiff --all-hands
  --playtest-retreat --record --shots DIR`, every frame at 30 a second through the player's own eyes, made into `docs/videos/retreat-and-repair.mp4`.

## 5. Checks

- `playtest` unit tests: the seat tour visits every seat once and ends; the retreat script's beats in order.
- `drill-mastiff` loads and validates (the drill's data loader).
- The runs' logs: every seat `sat` and `stood`; in the retreat, fires out and repairs done.
