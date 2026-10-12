# Proposal: All hands, a mode where the crew fight the fires and fix the ship

## Why

The owner, 2026-10-12, after the schematic went into the game: "Playtest that..I want a nice long footage with lots of
events. For this extra mode remove NPCs so.players would be the ones putting out fires and fixing stuff."

Today a hit's fires and broken systems are handled by two NPC damage-control teams of two (`damage.json` `teams`:
dispatched 5 s after a job appears, walking from the damage control room, a fire out in 20 s, a system back at a
rating's kit rate). The players watch the dots on the ship plan. The vision's third pillar is the opposite: "crew stand
up, walk to engineering, carry an extinguisher" (`docs/design/vision.md`). Every piece that pillar needs is designed
(`crew-on-deck`, `fire-spread`, `repairs-on-deck`, `repair-minigames`), and some are built: the repair rounds rule
(`sc-core::repair`), the 24 repair games in the engine (`sc-repairs`), bodies that walk the bridge in the drill.
Nothing puts them together into a fight.

## What Changes

- **A game option, All hands** (design 1): chosen with the mission, off by default. On, the ship carries no
  damage-control teams: `teams.count` is 0 for that game, from data, through the code that runs the teams today.
  Station NPCs stay at their stations (recommendation taken, ask only with screenshots: the owner's reason is "so
  players would be the ones putting out fires and fixing stuff", which the teams do; the bridge NPCs do neither).
- **Leaving a station** (design 2): a player stands up and the station goes to its automation (CLAUDE.md 7), or to its
  NPC on AUTO in single player; sitting back down takes it again.
- **The whole ship on foot during a fight** (design 3): bodies leave the bridge, down the ladders to decks B and C,
  on `crew-on-deck`'s walk and ladder rule, which `--walk-test` already walks.
- **Fires put out by hand** (design 4): an extinguisher taken from a damage-control locker; at a burning room's
  door or inside it, held on the fire, it puts the fire out by the same rule a team does, counted in hands: a team
  is its members, a player one. One player alone takes 40 s, three extinguishers; two players 20 s.
- **Repairs by hand** (design 5): a player docks at a damaged system, box, panel or switchboard section and plays its
  repair game's rounds (`repairs-on-deck`, `repair-minigames`), each landed round checked and applied by the server
  (`sc-core::repair`). With no teams, nothing else repairs anything.
- **Seeing the work** (design 6): the ship plan, Engineering's schematic and the alarms already show fires and
  damage; a new on-foot marker points to the nearest job. New screens get mockups first (CLAUDE.md 10).
- **The take** (design 8): the owner's footage, a whole fight in All hands, recorded once the loop is built.

## Capabilities

### New Capabilities

- `all-hands`: the mode, leaving a station, and the crew doing the damage control the teams do today.

### Modified Capabilities

None yet: `damage-control`'s teams and `fire-spread`'s extinguisher are both still changes.

## Impact

- `damage-control` and `ship-damage` (the teams' and the fire's rule, counted in hands), `crew-on-deck` (bodies on
  every deck in the drill, ladders), `fire-spread` (the extinguisher, later its cells), `repairs-on-deck` (docking,
  the repair points), `repair-minigames` (the games, with the three panel-box games when approved), `single-player`
  (the option on its page), `missions` (the option with the mission), `ship-interactables` (the damage-control
  locker).
- Pi 5 budget: no new geometry beyond the extinguisher prop (`crew-on-deck` 14); two new client intents (take an
  extinguisher, spray) of a few bytes; the snapshot already carries fires, teams and integrity, and loses the teams'
  bytes in this mode. Server work falls (no team planning).
