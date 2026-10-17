# Proposal: the helm's NAV map, and where to jump next

## Why

The owner, 2026-10-16: "Does the helmsman have a navigation map?" "Like where to warp to next". Today the answer is
no. Helm's console (`crates/sc-client/src/console/helm.rs`, approved mockup `docs/mockups/consoles.html` helm) has
THRUST, SCANNER (a local plot, 1 to 50 km, with the mission's waypoint and the COURSE autopilot), ATTITUDE and ORIENT.
The patrol's JUMP (`docs/mockups/patrol-consoles.html`, awaiting approval) has no destination of its own: the route's
stops are fixed in data (`patrol-mission` design 5 and 12c, `data/missions/patrol-shoals.json`,
`data/space/systems/shoals.json`), and the server jumps to "the next place". The system map that would show them
is designed (`flight-and-navigation` design 9) but has never been mocked up or built.

The same day the owner set three rules for the star systems the map shows:

> "Here are some rules to help with nav: 1. Each star system has a unique background 2. Each star system will have at
> least 1 station 3. Each star system will consist of several rocky worlds, at least 1 gas giant, and multiple
> asteroids"

The Shoals passes rules 1 and 2 (the Bloom sky, five stations) but its file has no kind on any body and no asteroids,
so nothing can check rule 3 or draw a gas giant differently from a rock.

## What Changes

- **NAV, a mode of the helm console** (design 3): a second tab on the helm's own chip in the title band (HELM | NAV),
  the way the captain's console has COMMAND | SHIP. NAV opens over the four panels, as OBJECTIVES does, and keeps the
  look band (the viewscreen and the attitude ball) and the status strip. It is not a new screen: it lives in the
  approved console's grid and the approved title band.
- **Three panels** (design 4): MAP (eight columns by four rows), TARGET and JUMP (four by two each).
- **The map** (design 5): the system from its data file, top down, from the whole system to 1 km: the star, each body
  drawn by its kind (rocky, gas giant with bands and a ring, asteroids as small irregular marks, belts as a dotted
  arc), orbits, stations, points of interest, the route's stops with their kind and cleared state, the ship, mass
  lock circles, science's contacts, the plotted course and its ghost while choosing.
- **Choosing the next jump** (design 6): a stop, a station, a point of interest, a body (its parking point) or a free
  point outside every mass lock. A tap previews it as a ghost; PLOT sends it. The route's next stop is preselected after
  every arrival, so bots and new players never get stuck.
- **Who decides** (design 7): helm plots, the captain's JUMP order stays the go (Star Trek: Bridge Crew's split). The
  captain sees the plotted course on a NAV tab of their own (design 9).
- **Alignment** (design 8): the jump vector on the map and on the viewscreen, the ALIGN cue (the bow inside the 5
  degree cone) and an ALIGN autopilot that points the bow, feeding the existing ALIGN and JUMP flow unchanged.
- **Server-authoritative target** (design 10): a new helm `Command::NavTarget`, checked by `sc-core` (the target's
  arrival clear of every mass lock by the arrival scatter), refused with the existing `MassLocked`; the patrol jumps to
  the plotted target instead of "the next place".
- **The owner's three system rules** (design 11): a `kind` on every body, a `belts` list, named asteroids, and data
  checks in `sc-core::system` that refuse a system file breaking them; a proposed Shoals delta (Orrin a gas giant with
  rings, the Shoals belt, the asteroids Skerry, Holm and Wick). Proposed only: the data file is not edited by this
  write-up.

## Impact

- `sc-core::system`: `BodyKind`, `belts`, the rules' checks, `parking_point`, `target_clear`. `sc-core::combat::patrol`:
  the target in `PatrolState`, preselection, `Command::NavTarget`, the jump to the target.
- `sc-net`: the command's tag, the snapshot's target byte, a reliable `NavPlotted` event for a free point; protocol
  bump. `sc-server`: none beyond the command path.
- `sc-client`: the NAV tab on helm and captain (`console/helm.rs`, `console/captain.rs`, a new `console/navmap.rs`),
  the ALIGN autopilot button, the viewscreen's target marker.
- Data: `data/space/systems/shoals.json` (the delta, after approval), `data/missions/patrol-shoals.json` (unchanged by
  this change; phase 3's stops are `patrol-mission`'s).
- `docs/mockups/helm-nav-map.html`: the screens for approval, before anything is built (CLAUDE.md 10).
- Pi 5 budget (design 12): 2D console UI only, inside `bridge-stations` 8.7's ceiling (16 draw calls, 6,000 triangles
  a console); the map adds about 3,000 UI triangles and one clip rectangle. No 3D. One command of under 30 bytes a
  plot, one byte a snapshot.
