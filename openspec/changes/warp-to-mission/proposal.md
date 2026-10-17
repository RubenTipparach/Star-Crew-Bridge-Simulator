# Proposal: warp to the mission

## Why

The owner, 2026-10-17:

> "I need a button to actually start the mission. Not just have the mission autostart"

> "Missions also shouldn't start right away. Helm needs to warp somewhere and warping somewhere takes a few seconds."

The first is `start-mission`'s rule change (the round waits in Muster until START MISSION is pressed). This change is
the second. Today START MISSION runs the 5 s countdown (`mission.countdown_s`, `coop-drill` 1) and the drill's Engage
begins with the Hound already 6 km ahead: the crew never travel to the fight. The patrol (`patrol-mission`) does
travel, but only after a docked start and a 4.4 minute undock, and with a 40 s spool for every jump.

The jump drive is already designed and built once (`flight-and-navigation` 7, built by `patrol-mission` 5 in
`sc-core::combat::patrol`, numbers in `data/ships/tern/jump.json`), and helm's control for it is designed twice
(`patrol-consoles.html`'s JUMP, `helm-nav-map`'s JUMP panel). This change uses that drive and that panel for the
mission's first jump. It does not add a second warp model (CLAUDE.md 6.1).

## What Changes

- **The ship starts away from the fight** (design 2): a mission names a departure place and a destination place in
  the system file. The drill starts at Tamsin Station's arrival point, 5,780 m out on the station's axis and outside
  its 2,780 m mass lock, with the bow already on the jump vector to the Shoals relay.
- **The phases** (design 3): Muster, then START MISSION, then Departure (the ship at the station, helm may jump), then
  Warp (the spool), then Engage at the destination (or the patrol). START MISSION goes straight to Departure: the old
  5 s countdown becomes the wait a helm bot takes before it jumps.
- **The warp takes 7.3 s** (design 4): the station's berth leaves the drive charged to 680 of its 800 MJ, so the
  spool needs 120 MJ at 20 MW: 6.0 s, the last 3 s of them with the bow held on the vector (FN 7's alignment rule),
  then the jump in one tick with PM 11.5's 0.3 s flash and 1.0 s settle. Less power stretches it, as every spool.
- **Helm jumps** (design 5): seated at helm, on foot. JUMP, held 0.6 s, on helm-nav-map's JUMP panel, which stands in
  ORIENT's cell on the HELM tab during Departure and Warp. The destination is set already: nothing to plot.
- **Bots, automation and single player** (design 6): a bot or automation at helm holds JUMP 5 s after Departure
  begins; in single player the player at helm does it, or the helm NPC does when the player sits elsewhere.
- **Every console sees it** (design 7): a chip in START MISSION's place in the title band names the destination and
  shows who it waits on, then the seconds left; the viewscreen and every window show the spool's stretch and the jump.
- **On the wire** (design 8): no new client message (helm's `Command::Jump`, the captain's `Patrol(Jump)` order); the
  drill's snapshot carries the patrol block in Departure and Warp, which gains the seconds left. `PROTOCOL` 18 to 19.

## Impact

- `data/missions/drill-hound.json`: `departure` (`from`, `to`). `data/ships/tern/jump.json`: `departure_charge_mj`.
  `data/input/bindings.json`: `helm.jump` (J).
- `sc-core::combat`: Muster to Departure on START MISSION; the drill run as a one-jump patrol from its departure;
  `patrol::spool_left_s`; the helm bot's and automation's jump in Departure.
- `sc-net`: the patrol block on the drill, its seconds left, `PROTOCOL` 19. `sc-server`: nothing new.
- `sc-client`: the JUMP panel in ORIENT's cell, the title band's jump chip, PM 11.5's stretch, flash and settle.
- `docs/mockups/consoles.html`: the helm's JUMP panel in its three states and the band chip (shots `warp-ready`,
  `warp-spool`, `warp-arrived`, `warp-band-tactical`), for approval before anything is built (CLAUDE.md 10).
- Cost (design 10): one panel swapped for another and one title band chip, no new draw call; the jump's streaks are
  PM 11.5's one draw of 800 triangles for 4.3 s; two bytes more a snapshot in Departure and Warp.
