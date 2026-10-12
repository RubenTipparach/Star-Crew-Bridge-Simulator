# Proposal: the first mission gate

## Why

The owner, 2026-10-10, after playing the co-op drill on the Pi 5: "play a session with your friends, go on a mission,
fight an enemy ship, I dont want a short clip this time, I want a longer clip. I want the entire mission", and
"document this as the steps required to pass this next quality gate. I expect no harness videos this time. I want
ACTUAL in game videos."

The owner's flow, as written:

> everyone joins a lobby room, they chat up the room. in game chat should be enabled so I know what bots are doing.
> game starts, the captain picks a mission, for now a demo mission to fight a grabon ship is on the table. the mission
> is selected and mission starts. game starts. peopele are walking around exploring the ship. The captain gets on
> deck, and switches the ship to red alert. The bots join in and begin the fight. A bot is controlling the helms. The
> captain can view his operations on the helm dash board. another bot joins the tactical. the captain can view there
> too, and tactical would report someone is on there. The captain can freely hop betweem statiions on his console. and
> assume control of any stations not in use. The captain modifies the power station and hops on to shields. the
> battle begins, the ship is attacked and damaged. the captain can order crew members to repair stuff (be sure to add
> NPCs on the ship too, at least 5 extra crew memebers, this ship requires at least 8 crew) or the captain can order
> a player to repair, or the captain can repair stuff himself. once the engagement is done and tehc rew survives. If
> there are fires, players or bots or captain can put out fires, repairt the ship, and warp out to end the scenario.

The co-op drill (`coop-drill`) is the closest the game comes today: a server, bots and players, the bridge on foot,
five consoles and one fight. It has no lobby room, no chat, no mission choice, no ship damage beyond the hull, no
repairs or fires in the fight, no crew of eight and no warp. This change is the gate: the flow above, step by step,
each step with what the recording must show, and the changes that build each one.

## What Changes

- **The gate** (design 1): fifteen steps from the lobby room to warping out, each with its pass condition and what
  the video must show.
- **What counts as a recording** (design 2): the game's own window on real hardware, recorded with `wf-recorder`,
  one take from the lobby to the end, unedited; never headless captures, scripted camera shots or a test harness.
- **Where the game stands** (design 3): each step today, with the evidence, and the change that builds it.
- **The build order** (design 4, tasks): the work grouped into changes, each small and single-purpose, each ending in
  a recording of the steps it adds.
- **The baseline recording** (design 5): the whole of today's mission, recorded on the Pi 5 the way the gate asks,
  so the gap is seen, not described.

## Impact

- Every change named in design 3, chiefly `coop-drill`, `lobby`, `netcode-and-sessions`, `bridge-stations`,
  `crew-npcs`, `damage-control`, `repairs-on-deck`, `repair-minigames`, `fire-spread`, `power-grid`,
  `weapons-and-shields` and `flight-and-navigation`.
- New, proposed here as names only: `crew-chat` (the lobby room's and the game's text chat) and `missions` (the
  mission list, the Grabon demo mission, warp out). Each gets its own write-up before it is built (CLAUDE.md 4).
- `docs/videos/`: the baseline recording and its shot list.
