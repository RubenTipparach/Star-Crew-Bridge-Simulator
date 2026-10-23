# Design: warp-to-mission

The owner, 2026-10-17: "I need a button to actually start the mission. Not just have the mission autostart", and
"Missions also shouldn't start right away. Helm needs to warp somewhere and warping somewhere takes a few seconds."

Presented in `docs/mockups/consoles.html` (shots `consoles-warp-*.png`, the contact sheet
`docs/screenshots/mockups/warp-to-mission/contact-sheet.png`). Nothing here is built. Numbers marked FN come from
`flight-and-navigation`, PM from `patrol-mission`, NM from `helm-nav-map`, SM from `start-mission`, CD from
`coop-drill`, M from `missions`, WP from `warp-pylons`, BS from `bridge-stations`.

## 1. What exists, and the one warp model

| Decided and where | What it says |
| --- | --- |
| The jump drive (FN 7, built by PM 5 in `sc-core::combat::patrol`, `data/ships/tern/jump.json`) | 800 MJ spooled at 20 MW times the drive's capability (40 s on a whole, powered drive); the bow within 5 degrees of the jump vector and turning under 2 deg/s for the spool's last 3 s; mass lock within 3 radii of a body, 5 km of a hostile ship over 500 t, or a station's `mass_lock_m`; arrival at the place's arrival point scattered within 1.5 km; transit instant at one tick; 60 s cooldown |
| How the crew see a jump (PM 11.1, 11.5, designed, not built) | The stars ahead stretch over the spool's last 3 s, a white flash of 0.3 s at the jump's tick, the new sky settling in 1 s: streaks through every window and on the viewscreen |
| The two stages (NM 2, Star Trek: Bridge Crew, cited there) | Charge, then align, then engage; "nobody shouts engage at a cold drive" |
| Helm's JUMP (PM 7 `patrol-consoles.html`; NM 4 and 8, the JUMP panel at grid 8, 2, 4 by 2 on the NAV tab) | Held 0.6 s (BS 8.5), the spool ring and its seconds, the ALIGN cue (the bow in the 5 degree cone) |
| Who says go (NM 7, PM 9) | Helm jumps; the captain's `Patrol(Jump)` order is the go for bots and lights helm's order chip; with no captain aboard the helm bot takes it itself |
| START MISSION (SM, rule changed on 2026-10-17) | The round waits in Muster until it is pressed, by the captain or, with no person as captain, by any seated person |
| Tamsin Station (PM 2, 3; `data/space/systems/shoals.json`) | The O'Neill pair; `mass_lock_m` 2,780 m from its frame's origin; `arrival_m` [0, 0, 5,780] in its frame: 3,000 m outside the lock, on its axis |
| M 3 | A second warp model: `jump_spool_s` 10 s and no alignment. **Superseded here**: it predates FN 7's build, and keeping it would be two rules for one drive (CLAUDE.md 6.1). Its jump-out step becomes FN 7's jump |

**This change adds no warp model.** The first jump of a mission is FN 7's jump, through `patrol::` code that already
spools, aligns, locks and arrives. What is new is where the ship starts (section 2), how much charge the drive starts
with (section 4), and where helm's control stands on the HELM tab (section 5).

## 2. Where the ship starts

A mission gains a `departure` block: `{ "from": <place id>, "to": <place id> }`, places of its system file
(`SystemFile::place`).

| Mission | From | To | Then |
| --- | --- | --- | --- |
| `drill-hound` | `tamsin_station`: its arrival point, 5,780 m out on the station's axis | `shoals_relay`: its arrival point (PM 2: the relay at `waypoint_m` from it in the Tern's arrival frame, so the drill's geometry holds) | The drill's Engage: the Hound `enemy_start_m` 6 km ahead of the arrival, its Jackals 20 s in (CD 1, as today) |
| `patrol-shoals` | No `departure`: it starts in the berth, docked, as built (PM 4) | | The patrol, as built; its first jump takes the berth's charge (section 4, question W4) |

- **At the departure point the Tern is still**: velocity zero relative to the station, the attitude with its bow on
  the jump vector to the destination's arrival point (computed at load from the two places, never typed in, CLAUDE.md
  6.6), roll so the station's +y is the ship's up. The bow is on the vector from the first tick, so a crew that does
  nothing but press JUMP is aligned: the first mission needs no flying (keep it simple).
- **The departure is clear.** 3,000 m outside Tamsin Station's lock and nothing hostile in the system's first scene,
  so no mass lock (FN 7) holds the spool; the check runs anyway (`patrol::` as built), so a mission whose departure is
  locked fails its data check at load.
- **Weapons**: the station's `weapons_safe_m` 5 km (PM 4.1) does not reach 5,780 m; the turrets are free, and there is
  nothing to shoot.
- The drill's frame (CD, PM 5.3: a floating origin at the place the Tern is at) starts at the departure point and
  moves to the destination's arrival at the jump, exactly as a patrol's jump moves it.

## 3. The phases

```
Muster --START MISSION--> Departure --helm holds JUMP 0.6 s--> Warp (6.0 s) --jump tick + 1.3 s--> Engage
                                                                                                   (or the patrol)
```

| Phase (what the player sees) | On the server | Ends | Time |
| --- | --- | --- | --- |
| Muster | `Phase::Muster` (SM: waits for START MISSION) | START MISSION | as long as the crew like |
| Departure | `Phase::Patrol`, the patrol block's stage `Underway`, at the `from` place, `next` the `to` place, `spooling` false | Helm holds JUMP (a person), or a bot or automation does (section 6) | a person at helm: as long as they like; a bot: 5 s |
| Warp | The same, `spooling` true | The spool full and the bow aligned for the last 3 s (FN 7) | 6.0 s at full power (section 4) |
| Engage | `patrol_jump` to the `to` place: the encounter there starts the drill's Engage (PM 5.2, as built for the Shoals relay) | The drill's ends (CD 1): victory, defeat, withdrawal | the fight |

- **No new `Phase` byte.** Departure and Warp are the patrol phase's `Underway` stage, not spooling and spooling. One
  phase machine runs both missions (CLAUDE.md 6.1); the client names them from the stage and the flag.
- **The countdown goes.** START MISSION moves Muster to Departure at once. The 5 s countdown was the crew's warning
  before the enemy appeared (SM S2); now the enemy appears only after a 6 s spool that every console shows (section
  7), and the departure itself is the wait. `mission.countdown_s` stays in the data with a new meaning: the seconds a
  helm bot or automation waits in Departure before it jumps (section 6), so a crew without a person at helm sees the
  same 5 s before anything moves that it sees today (question W1).
- **Back to Muster**: the built rule that a Countdown with nobody seated returns to Muster (SM) becomes: a Departure
  with nobody seated returns to Muster, the Tern put back at the departure point. A Warp under way goes on (the drive
  does not care who is seated).
- **The crew keep walking.** Departure and Warp are live phases (`Phase::live`): bodies step, consoles take commands,
  fires burn, a player may stand up and walk to a window to watch the jump (the law). Nobody is held in a seat.
- **The drill ends as it does today**: at the fight's outcome, Debrief. It has no jump home (a one-way departure);
  the patrol keeps its route and its jump home.

## 4. How long the warp takes: 7.3 s

The owner's "takes a few seconds". The numbers, all FN 7's except one:

| Step | Seconds | Why |
| --- | ---: | --- |
| JUMP held | 0.6 | The guarded control's hold (BS 8.5), as every JUMP |
| Spool | 6.0 | The drive starts with the berth's charge, 680 of its 800 MJ (`jump.json` `departure_charge_mj`, new); the last 120 MJ at FN 7's 20 MW is 6.0 s |
| of which: the stretch | the last 3.0 | FN 7's alignment hold (`align_last_s`), and PM 11.5's stretch of the stars toward the bow over the same 3 s |
| The jump | one tick | FN 7: the ship's position moved at a tick boundary; the one-tick 5 m/s^2 jolt through the dampers |
| Flash and settle | 1.3 | PM 11.5: 0.3 s white, 1.0 s of the new sky coming in |
| **From the hold to the fight on screen** | **7.3** | 7.9 s from the finger going down |

**Why 6 s of spool, and why as a charge.** The spool must last at least FN 7's 3 s alignment hold, or the jump would
fire before the hold was met. Twice that gives the crew 3 s to see the ring start and hear "Spooling the jump drive"
before the stars begin to stretch, and keeps the whole warp inside the owner's "a few seconds" (under 8 s). It is a
charge and not a shorter spool time because a charge keeps FN 7's formula, `t = (800 MJ - charge) / P_delivered`:
engineering's power still sets the time (at 10 MW delivered, 12 s; Artemis's "the duration of the countdown depends on
power the Engineer has allocated", cited in NM 2), a damaged drive still stretches it, and a damaged pylon still stops
or slows it (WP 3). Where the charge comes from: a ship leaves a station's berth with its jump drive topped up by the
station; it is a starting state, not a new power source, because the grid does not model shore power as a source (PM
4.1).

**What the 680 MJ is not**: it is not kept across jumps. The jump spends the whole 800 MJ (FN 7, as built:
`spool_mj` to zero at the jump), so every later jump is the full 40 s.

**An abort** (JUMP held again during the spool, as built) puts the spool back to the departure charge while the Tern
is still at its departure point, not to zero: a stray hold must not cost the crew 34 s. Anywhere else an abort empties
the spool, as built.

**Not chosen**:

| Option | Against |
| --- | --- |
| A separate warp timer (M 3's 10 s `jump_spool_s`) | A second warp model; no alignment, no power; the reason M 3 is superseded |
| The full 40 s spool from cold | Not "a few seconds"; 40 s of nothing before the first fight of a drill |
| A transit of a few seconds in streaks (the ship "in warp") | FN 7's transit is instant at one tick, chosen for the frames (one rebase, nothing moving fast near anything). The 3 s stretch, the flash and the settle are 4.3 s of streaks already (question W2) |

## 5. Helm's control

- **Where**: the HELM tab, in ORIENT's cell (grid 8, 2, 4 by 2), during Departure and Warp and for 3 s after arrival.
  It is helm-nav-map's JUMP panel (NM 4), the same panel in the same cell the NAV tab puts it, so the control is one
  picture wherever helm meets it. ORIENT is not needed before the fight (there is no target to point at), and comes back
  3 s after arrival. A new player at helm never has to open NAV.
- **What the panel holds** (NM 8, unchanged): the ALIGN cue (the bow and the 5 degree cone, the off angle in whole
  degrees: helm acts on it), the ALIGN autopilot button (NM 8; unlit, since the bow starts on the vector), JUMP as the
  biggest control. Added: the destination's name at the panel's top right, in the text colour (the briefing's place:
  "THE SHOALS"). Nothing to plot: the destination is the mission's `to`.
- **States** (the mockup's shots):

| State | JUMP | Under it | Ring | The cue |
| --- | --- | --- | --- | --- |
| Ready (Departure) | Lit, the accent; held 0.6 s fills it from the left | `hold` | The drive's charge, 85 %, dim | `ok`: the bow inside the cone, 1 degree off |
| Spooling (Warp) | Lit, `warn` | The seconds left, rounded up (`2 s`) | The charge rising from 85 % to full, `warn` | `ok`; in the last 3 s the stars on the viewscreen above stretch |
| Arrived (3 s after the jump) | ARRIVED with a tick, `ok`, not pressable | `cooldown 60 s` (FN 7: when helm may jump again) | none | faint |

- **The key**: `helm.jump`, J, held 0.6 s (`data/input/bindings.json`; J is bound nowhere in the seated game: helm's keys
  are W S A D R F Z C X, the arrows, Space and Ctrl). Options > Controls gains the row "Jump" under Helm. On touch the
  panel's JUMP is a hold control, which takes the finger at once (`consoles.html`'s finger rule: a drag or a hold
  control takes the finger as it takes the mouse).
- **Refusals**: as built (`NotReady` in the cooldown, `MassLocked`, `WrongPhase`), shown as every refusal.
- **The captain's order**: START MISSION counts as the captain's `Patrol(Jump)` order (it is the captain's go, SM 2), so
  helm's band shows the order chip `CAPTAIN: JUMP` lit until helm acts, as PM 7 draws it; a captain who pressed START
  MISSION has nothing more to press.

## 6. Who jumps when helm is not a person

| Helm is | Jumps | After |
| --- | --- | --- |
| A person (multiplayer or single player), seated at helm | When they hold JUMP | No timer: the person is playing |
| A bot client (`sc-net::bot`) | Holds JUMP itself | `mission.countdown_s`, 5 s after Departure begins |
| Nobody: the station on automation | Automation jumps (FN 10's "never jumps on its own" holds: START MISSION is the order) | 5 s after Departure begins |
| Single player, the player seated elsewhere | The helm NPC on AUTO (`single-player` 4) | 5 s after Departure begins; on MANUAL it waits for the player to press JUMP on helm's console from their seat's swap |

- The wait is `patrol::decide`'s helm rule (one rule for bots and automation, PM 9) with one more condition in
  Departure: `phase_s >= mission.countdown_s`. 5 s is the old countdown's length, so a bot crew's first fight starts
  5 + 0.6 + 6.0 + 1.3 = 12.9 s after START MISSION, against today's 5 s: the warp the owner asked for, and no wait of its
  own beyond the countdown the crew already knew.
- The helm bot says "Spooling the jump drive" (`chat.json` `jump_spool`, built) as it holds JUMP, and the computer
  "Arrived at the Shoals relay" (`arrived`, built) on arrival.
- A person at helm who never jumps: the captain sees the band chip waiting on HELM (section 7) and talks to them. No
  timeout takes the control from a person (question W5).

## 7. What each console, the viewscreen and the windows show

| Where | Departure | Warp | Arrival |
| --- | --- | --- | --- |
| Every console's title band | In START MISSION's place (SM 4: right against LEAVE STATION): the jump chip, the jump icon, the destination (`THE SHOALS`) and, at its right end, who it waits on: the helm icon and `HELM` | The same chip in `warn`, a small ring with the charge and the seconds left (`2 s`) | Gone with the flash |
| Helm | The JUMP panel in ORIENT's cell, Ready | Spooling | ARRIVED for 3 s, then ORIENT |
| Captain | The chip; the order chip's place shows nothing (the order was START MISSION) | The chip | |
| Engineering | The chip; the spool is FN 7's priority 3 load on the drive panel (`dp_drive`), drawn there as built: engineering sees the 20 MW and the seconds on the chip stretch if it takes power away | | |
| Tactical, science, the lasers, the gunner | The chip | The chip | The fight's contacts appear on the plots |
| The viewscreen (the feed, PM 11.7) | The bow camera: the stars, the destination's diamond where the jump vector meets the view (NM 8), no contacts | In the last 3 s the stars stretch toward the bow (PM 11.5) | The flash, then the Shoals: the relay 18.6 km off, the Hound 6 km ahead |
| The windows (PM 11.1) | Tamsin Station 5.8 km off on the side that faces it (the pair 1.2 km across: about 12 degrees), Tamsin's disc, the star | The stretch through every window, streaks running aft along the side windows (PM 11.1's Spool row) | The flash, the new sky settling in 1 s, the fight (PM 11.1's Shoals row) |
| On foot | A player standing anywhere feels the jump's one-tick jolt (FN 7, the dampers' effect, CLAUDE.md 7) | | |

Words at rest added to a console: the destination's name (a place name) and `HELM` on the chip, `JUMP`, `ALIGN` on
helm's panel (as NM's), `ARRIVED`: inside CLAUDE.md 10's 30.

## 8. On the wire

- **No new client message.** Helm's JUMP is `Command::Jump(true)` (a helm command, built); the abort is
  `Command::Jump(false)`; START MISSION is `ClientMsg::StartMission` (tag 14, SM 6), which now also sets the patrol
  block's `ordered` to `Jump`.
- **The snapshot**: the drill's snapshot carries the patrol block (PM 7, about 50 bytes, built for the patrol) in
  Departure and Warp, so the client has the stage, the place it is at and the next, the spooling flag and the charge's
  fraction. The block gains **the seconds left on the spool**, a `u16` in tenths of a second (0 while not spooling), from
  `patrol::spool_left_s(spool_mj, spool_mw, capability, jump)`: the one function the server's spool and the client's
  number share (CLAUDE.md 6.1: the preview is computed by the code that resolves it).
- **`PROTOCOL` 18 to 19** (the next free: `start-mission`'s rule change on 2026-10-17 does not change the wire).
- A bot client's Departure wait reads the phase's seconds from the snapshot's clock, as it reads `bot_ready_s` today.

## 9. Data

| File | Change |
| --- | --- |
| `data/missions/drill-hound.json` | `departure`: `{ "from": "tamsin_station", "to": "shoals_relay" }`; `countdown_s` documented as the helm bot's wait in Departure |
| `data/ships/tern/jump.json` | `departure_charge_mj` 680.0 (under `spool_mj`, checked at load: 0 to `spool_mj`) |
| `data/input/bindings.json` | `helm.jump` J, and the Helm row in Options > Controls |
| `data/missions/patrol-shoals.json` | Nothing; its first spool takes the berth's charge if W4 is taken |

## 10. What it costs

| Cost | Amount |
| --- | --- |
| Console UI | ORIENT's panel swapped for the JUMP panel (about the same shapes), one title band chip: no new draw call, a few dozen UI vertices |
| 3D | PM 11.5's streaks: one draw of 800 triangles for 4.3 s, and the flash (the sky's colour lifted). Nothing else |
| Server | The patrol's spool and alignment, built, run for the drill too: a few operations a tick |
| Network | The patrol block's 50 bytes in the drill's snapshot during Departure and Warp (about 13 s), plus 2 bytes for the seconds left |
| Load | The drill reads the system file (already read by the patrol) and computes one attitude |

## 11. Checks (when built)

- `sc-core`: START MISSION moves Muster to Departure with the Tern at Tamsin Station's arrival point and the bow on the
  vector to the relay; Departure stays put until helm jumps; a person's JUMP spools and the jump comes 6.0 s later at
  full power and 12.0 s at half; an abort at the departure puts the spool back to the charge; a bot helm jumps 5 s into
  Departure; with nobody seated Departure goes back to Muster; arrival at the relay starts the drill's Engage with the
  Hound 6 km ahead; a later jump spools the full 40 s.
- `sc-net`: the drill's snapshot carries the patrol block in Departure and Warp; the seconds left round trip.
- `sc-client`: the JUMP panel stands in ORIENT's cell in Departure and Warp and ORIENT is back 3 s after arrival; the
  band chip in every console's START MISSION place; parity states `warp-ready`, `warp-spool`, `warp-arrived` and
  `warp-band-tactical` within the limits.
- A playtest on foot (CLAUDE.md 12): walk to helm, sit, START MISSION, the Departure with the station in the windows,
  JUMP, the stretch, the flash, the Shoals; and a take standing at the mess's window for the jump.

## Open questions

| Id | Question | Options | Recommendation | Status |
| --- | --- | --- | --- | --- |
| W1 | Keep the 5 s countdown between START MISSION and Departure? | Keep it; drop it (Departure is the wait) | Drop it: the departure and the 6 s spool are the warning; `countdown_s` becomes the helm bot's wait | Recommendation taken (ask only with screenshots); with the contact sheet in the next survey |
| W2 | The transit: instant at one tick (FN 7) with 4.3 s of stretch, flash and settle, or a few seconds "in warp" | Instant; a timed transit | Instant: FN 7's frames reason, and the streaks already last 4.3 s | Ask with the contact sheet |
| W3 | 7.3 s from the hold to the fight (a 6.0 s spool from a 680 MJ charge) | 5 s (a 4.4 s spool: only 1.4 s before the alignment hold); 7.3 s; 10 s | 7.3 s | Ask with the contact sheet |
| W4 | Does the patrol's first jump take the berth's charge too (6 s instead of 40 s after the undock)? | Yes, one rule for leaving a berth; no, the patrol unchanged | Yes: one rule, and the patrol's 4.4 minute undock is the wait there | Recommendation taken (ask only with screenshots) |
| W5 | A person at helm who never jumps: a timeout that hands it to automation? | None; after 60 s | None: the captain talks to them | Recommendation taken (ask only with screenshots) |
| W6 | The control's word: JUMP (the game's word: the jump drive, the JUMP order) or WARP (the owner's word) | JUMP; WARP | JUMP, one word across helm, the captain's order and the patrol | Ask with the contact sheet |
| W7 | M 3's 10 s `jump_spool_s` warp out | Keep as a second model; supersede with FN 7's jump | Superseded (section 1) | Recommendation taken (ask only with screenshots) |
