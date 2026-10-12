# Design: All hands

## Context

| Decided elsewhere | Where |
| --- | --- |
| Fires, the teams, their jobs and rates; a fire no team reaches spreads after 90 s | `damage-control` 3 and 6, `ship-damage` 5, `sc-core::combat::damage`, `damage.json` |
| Every station works with or without a player; automation plays below a person | CLAUDE.md 7, `bridge-stations` 3 |
| Single player: an NPC at every other station, AUTO or MANUAL | `single-player` |
| Bodies, postures, Use, ladders, the extinguisher's body (9 kg, 6 kg of agent over 15 s) | `crew-on-deck` 3, 4 and 14 |
| The extinguisher's cut, and fire on the floor in cells | `fire-spread` 5 |
| Docking at a repair point, the game over the live room, the server's checks | `repairs-on-deck` 1-7 |
| A job cut into rounds, an officer plays, a rating repairs at a rate | `repair-minigames` 1 and 1a, `sc-core::repair` |
| The three panel-box games (awaiting approval, survey RM1-RM3) | `repair-minigames` 2a |

## 1. The mode

One game option, **All hands**, chosen with the mission (the briefing's mission page in multiplayer, the Single
Player page in single player), off by default. It is the session's, set before Muster, and the debrief names it.

| | Teams (default) | All hands |
| --- | --- | --- |
| Damage-control teams | `teams.count` (2 on the Tern) | 0 |
| Who puts out a fire | A team, or a player | A player |
| Who repairs | A team at its rate, or a player through the game | A player through the game |
| Station NPCs (single player) | At their stations | At their stations |

The option is data: `missions.json` gains `"damage_control": "teams" | "crew"` with `"teams"` the code default
(CLAUDE.md 6.5), and the lobby may override it for the session. The server builds the ship's damage state with
`teams.count` replaced by 0 when it is `"crew"`, so every team rule runs unchanged with no teams (one code path,
CLAUDE.md 6.1). Nothing else in `damage.json` changes: fires start, burn and spread as today, and with no team they
spread after 90 s unless a player is at them.

Which NPCs go (recommendation taken, ask only with screenshots): the teams. The owner's reason is "so players would be
the ones putting out fires and fixing stuff", and the teams are what does that. The bridge NPCs fly, fight and route
power; removing them would leave a single player alone on a ship of empty consoles.

## 2. Leaving a station

A seated player stands (the Use key, or the console's title band) and walks. The station goes to its automation at
its stated competence (`bridge-stations` 3). In single player the other stations keep their NPCs; the player's own
seat has none, so automation plays it. Sitting down at any free station takes it. The crew tabs show a left station dashed, as an
open one is today. The title band's STAND control is new: it goes into `consoles.html` for approval first.

## 3. The whole ship on foot during a fight

Today the drill's bodies walk the bridge only (`sc-core::combat::bodies`). All hands needs every deck: the bridge's
door, the command passage, both ladders to decks B and C, and every room with a repair point or a fire. The walk
grid and the ladders are `crew-on-deck`'s (`sc-core::walk`, `sc-core::lift`, walked end to end by `--walk-test`); the
server's bodies take them in place of the bridge's straight lines. A body in a burning room takes fire's harm
(`damage-control` 3: heat and smoke), the same harm a team member would.

## 4. Fires put out by hand

The fire rule today counts a team's seconds at the fire: `work_s += dt`, out at `put_out_s` (20 s). All hands counts
**hands** instead, kept on the fire, so a team and a player run the same line:

- a fire keeps `out_s`, the hand-seconds put into it;
- each frame every team working it adds `members x dt` (2 on the Tern), every player spraying it adds `dt`;
- it is out at `put_out_s x teams.members` hand-seconds (40 on the Tern);
- `unreached_s`, the clock to spreading, stops while any hand is at it, as it does for a team.

A team of two still takes 20 s, as today (the core's tests hold). One player takes 40 s; two take 20 s.

A player sprays with an extinguisher taken from the damage control room's locker (`dc_lockers`, six). Held on, it
discharges its 15 s of agent (`crew-on-deck` 14), so one player alone needs three for one fire, two players two
each. The spray counts only from inside the room or its doorway, facing the room's centre within 45 degrees (the
cone of `fire-spread` 5). When `fire-spread`'s cells are built, the spray's hand-seconds become its 60 kW a second on
the cells it covers, and the room's `hrr` decides out; until then this rule is the whole of it.

## 5. Repairs by hand

A player docks at a damaged system's repair point (`repairs-on-deck` 2 and 3) with a kit from the same locker
(`kits_in_locker` 6), plays the system's game, and each landed round raises its integrity by the round's share,
checked by the server (`sc-core::repair`, `Who::Officer`). A box, a panel or a switchboard section plays its panel-box
game (`repair-minigames` 2a) once approved; until then they repair with the breakers game, as the main switchboard's.
A disabled or destroyed system needs its parts from the cargo store (`parts_on_board` 24), as `damage-control` 6a
says.

## 6. Seeing the work

What already shows a job: the ship plan (captain, SHIP), Engineering's schematic (a burnt board, a damaged box, a
cut run), the alarms. New for a body on foot: a marker at the screen's edge pointing to the nearest fire, then the
nearest damaged system, with its distance in metres. It is a new screen element and gets a mockup in
`repairs-on-deck.html` before it is built.

## 7. Single player alone

One player in All hands is the whole damage-control party. The NPCs keep their stations; the player's own station
is played by automation while they are away. With two fires and a disabled system, one player has to choose, which is
the point of the mode. Nothing makes it easier for one: the fight's numbers are the mission's.

## 8. The take

The owner asked for "a nice long footage with lots of events". Once sections 2-5 are built: one whole fight in All
hands, recorded on the Pi as the earlier takes (`docs/videos/README.md`), with the player putting out at least two
fires and repairing at least two systems, each listed with its time in the video's notes. A take of today's game,
with the teams, was made on 2026-10-12 (`2026-10-12-schematic-fight-pi5.mp4`).

## 9. Order of the build

| Step | Needs | Proves it |
| --- | --- | --- |
| 1. The option and no teams | `missions.json`, the server's damage state | A core test: with `"crew"` no team leaves home, a fire spreads after 90 s |
| 2. Hands on a fire | `damage.rs` fire rule | Core tests: a team of two still 20 s; one player 40 s; two players 20 s |
| 3. Standing and sitting | `bridge-stations`, the title band's STAND (mockup first) | A seat test, the parity check |
| 4. Every deck on foot in the drill | `crew-on-deck`'s walk and ladders in the server's bodies | An in-process test: a body walks bridge to engine room |
| 5. The locker and the extinguisher | `ship-interactables`, `crew-on-deck` 14 | A server test: take, spray, a fire out |
| 6. Docking and repair rounds | `repairs-on-deck` 2-6 | A server test: a landed round raises integrity, a forged one is refused |
| 7. The marker | Its mockup approved | The parity check |
| 8. The take | All of it | The video and its notes |

## 10. The Pi 5 budget

No new geometry but the extinguisher prop (`crew-on-deck` 14's budget). Two intents (take, spray on and off), a few
bytes each when sent. The snapshot already carries fires, integrity and bodies; with no teams it carries none of
theirs. The server does less (no team planning); bodies walking every deck cost what `crew-on-deck` states.

## Open questions

| # | Question | Recommendation |
| --- | --- | --- |
| AH1 | Which NPCs go in All hands | The damage-control teams only (recommendation taken, ask only with screenshots) |
| AH2 | Where the option is chosen | With the mission: the briefing in multiplayer, the Single Player page alone (taken; both pages get the toggle in their mockups first) |
| AH3 | A lone player's own station while they are away | Its automation (taken: CLAUDE.md 7) |
