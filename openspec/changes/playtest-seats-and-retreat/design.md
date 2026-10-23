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
| Missile tubes, load time | 2, 60 s | 4, 30 s |
| Orbit, m | 1,800 | 1,400 |

Flight is the Hound's but slower (top speed 260 m/s against the Tern's 400), so a retreat can open the range.
`drill-mastiff` is `drill-hound` with this enemy and its own title. Recommendation taken (ask only with screenshots):
these numbers; the captures show what they do.

## 3. The retreat and repair

`Playtest::retreat()`, at the helm: `To(Helm)`, `Sit`, `Engaged(30 s)` (the exchange that hurts the ship), then the
retreat: `Press(Flip)` (the bow away), `Press(FullAhead)` (the throttle dragged to its top as a pointer would),
`Work(25 s)` while the range opens; `Stand`, then `Locker`, `Fight`, `Repair` until nothing burns and nothing is broken,
then back to the helm. Two new presses: FLIP's button, and the throttle lever pressed at its foot and dragged to its
top. Run with `--all-hands` so the crew, not damage-control teams, fix the ship.

## 4. The recorded runs

- **PI-1, the seat tour, real time:** `sc-client --solo --playtest-seats --window`, recorded with the Pi's screen
  recorder, committed as `docs/videos/seat-tour-pi1.mp4` with the run's log beside it.
- **The cloud, the retreat:** `sc-client --headless --solo --station helm --mission drill-mastiff --all-hands
  --playtest-retreat --record --shots DIR`, every frame at 30 a second through the player's own eyes, made into `docs/videos/retreat-and-repair.mp4`.

## 5. Checks

- `playtest` unit tests: the seat tour visits every seat once and ends; the retreat script's beats in order.
- `drill-mastiff` loads and validates (the drill's data loader).
- The runs' logs: every seat `sat` and `stood`; in the retreat, fires out and repairs done.
