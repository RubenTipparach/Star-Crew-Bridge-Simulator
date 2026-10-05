# Design: bridge stations

Status: **proposed** (2026-10-04). Nothing here is built. Where a fact comes from the layout
(`data/ships/tern/layout.json`) or the brief (`docs/design/vision.md`) it is **decided** and says
so; everything else is this change's proposal. Every number carries its unit. Budgets are against
the provisional Pi 5 table in `openspec/changes/engine-stack/design.md` section 5, the one source
for those numbers. The mockup that presents this design is `docs/mockups/bridge.html`.

## Context

The brief asks for "a starship bridge with multiple stations", "a core of 4 players, possibly
more", fighters, "manable or automated turrets", and a full 3D bridge the crew walk. The layout
already places fourteen seats: seven on the bridge, three elsewhere on the decks and four in
turret pods. What it cannot say is what each seat does, what happens to an empty one, how a crew
of one to eight spreads across fourteen seats, and what a seated player sees.

The genre has answers worth taking the shape of (CLAUDE.md 15, `docs/references.md`):

- *Artemis* put each station on its own screen, with the captain watching a main screen. We keep
  one console per station, but the main screen is a real object in a real room.
- *EmptyEpsilon* combines stations into fewer consoles for smaller crews. That is our merge rule's
  shape; the presets here are ours, written for the Tern's systems (its own presets are marked
  "to verify" in `docs/references.md`, so nothing here depends on them).
- *Pulsar: Lost Colony* has crew on foot in a walkable ship with bridge stations. It is the
  closest existing game to "a full 3D bridge".
- star-crew-64, our own prototype, proved the shape and showed the bugs to avoid
  (`docs/analysis/star-crew-64.md`, section 15 below).

## Goals / Non-Goals

**Goals:**
- Every station has a job no other seat does well, and the console shows only what that job
  needs.
- One player can fly the whole ship; four is the intended crew; up to eight all have work.
- An empty station never stalls the ship: automation holds it at a stated, lower competence and
  never takes a decision the crew should own.
- Nobody is ever locked out of a seat by a body that is not playing.
- A console's preview is the outcome: the same function computes both (CLAUDE.md 6.1).
- The bridge is a room with sightlines, lamps, sound and a viewscreen that shows the real
  exterior, decoupled from the ship's motion.

**Non-Goals:**
- The rules of the systems the consoles drive. Power flow is `power-grid`'s, the atmosphere is
  `life-support`'s, hits and repairs are `damage-control`'s, turrets, missiles and shields are
  `weapons-and-shields`', the flight model is `flight-and-navigation`'s, bays and craft are
  `shuttle-bay-and-fighters`'. This change states what each console reads and writes.
- Walking, ladders, injury and carrying: `crew-on-deck`.
- Voice chat and text chat: `netcode-and-sessions`.
- The fighter cockpit: `shuttle-bay-and-fighters`.

## Decisions

### 1. Words used here

| Word | Meaning |
| --- | --- |
| **Role** | A kind of job: command, helm, tactical, engineering, science, comms, flight_ops, gunner (the layout's `role` field, decided). |
| **Station** | One entry in the layout's `stations` list: a seat with a console, in a compartment, with a role. Fourteen on the Tern (decided). |
| **Seat** | The chair of a station. It has an **occupant**: a body or nobody. |
| **Operator** | Who drives a station: a **player**, **automation**, or nobody because the station is **merged** onto a manned console as a tab and automation runs it underneath. |
| **Setpoint** | A value the crew asks for (a heading, a load group's power setpoint in percent, a turret mode). Automation and control loops maintain setpoints; players change them. |
| **Body** | A crew avatar in the interior (`crew-on-deck`). A player drives one body. A body nobody drives is an **NPC body**. |
| **Condition** | The ship's alert state set by command: normal or red alert. |

### 2. The station roster

Seat positions and facings are decided by the layout; "Core" is decided by the layout's `core`
flag. Purpose, merge host and automation are proposed here.

| Station | Role | Where (layout) | Core | Purpose | When nobody sits here |
| --- | --- | --- | --- | --- | --- |
| `helm` | helm | Bridge, front left (port), faces bow | yes | Fly the ship: attitude, throttle, course, evasive patterns, docking. | Automation holds course and speed and obeys orders; tab on the first manned of Tactical, Captain, Science, Engineering. |
| `tactical` | tactical | Bridge, front right (starboard), faces bow | yes | Targets, turret modes and assignments, missile tubes, shield preset. | Automation fights turrets and point defence, never fires missiles unordered; tab on Helm, Captain, Science, Engineering. |
| `engineering` | engineering | Bridge, port wall, faces port | yes | Power setpoints and priorities per load group, breakers, reactor, coolant, life support setpoints, repair priorities. | Automation applies presets and resets breakers; tab on Science, Captain, Helm, Tactical. Manned if `eng_main` is manned. |
| `science` | science | Bridge, starboard wall, faces starboard | yes | Sensors and scans, shield face balance and frequency, the viewscreen feed. | Automation scans passively and leans shields toward fire; tab on Engineering, Captain, Helm, Tactical. |
| `captain` | command | Bridge, centre dais (0.3 m), faces bow | no | Overview, condition (red alert), orders, viewscreen override, brace. | No automation of judgement; auto-condition only. Command functions appear in every manned bridge console's title band. |
| `comms` | comms | Bridge, port aft, faces port | no | Hails, replies, clearances, intercepts, distress. | Automation answers routine hails only; tab on Science, Captain, Engineering, Helm, Tactical. |
| `flight_ops` | flight_ops | Bridge, starboard aft, faces starboard | no | Bay pressure, doors, launch and recovery sequences, craft tasking. | Automation runs sequences on order only; tab on Tactical, Captain, Helm, Science, Engineering. Manned if `bay_control` is manned. |
| `eng_main` | engineering | Engineering, mezzanine beside the reactor, faces aft | no | The engineering console plus hands-on reactor control: scram reset, manual coolant valves, local breakers. | A second seat of the Engineering role. |
| `damage_board` | engineering | Damage control, faces starboard | no | Damage control teams, the repair queue, compartment status, door locks. | Automation dispatches teams; tab on Engineering (bridge), Engineering bay, Captain, Science. |
| `bay_control` | flight_ops | Hangar forward landing, faces aft over the hangar | no | Local launch control with a line of sight into the hangar; the bay lockout. | A second seat of the Flight ops role. |
| `gunner_dorsal`, `gunner_ventral`, `gunner_port`, `gunner_stbd` | gunner | Turret pods | no | Aim and fire one twin pulse cannon by hand. | The turret is automated at lower accuracy; Tactical sets its mode. No remote gunnery. |

**Two seats, one role.** `engineering` and `eng_main` are two seats of the Engineering role; both
issue Engineering commands and the console attributes each change ("set by Ana at Engineering
bay"). Only `eng_main` has the hands-on reactor panel. `flight_ops` and `bay_control` work the
same way, and only `bay_control` has the lockout (section 10.10). A role's automation runs only
when none of its seats is manned.

### 3. Operators

The server's seat table (owned by `netcode-and-sessions`, section 7 there) holds, per station:

| Field | Values | Bytes |
| --- | --- | ---: |
| operator | player slot 0-7, `AUTO`, `MERGED` | 1 |
| occupant | body id or none | 1 |

This is one byte more per station than `netcode-and-sessions` first listed ("occupant 1 B"):
14 B more on a keyframe, and a change only when someone sits or stands. That change's seat table
carries it since 2026-10-04 (its section 4).

**The rule that fixes star-crew-64's locked seat:** the operator is never a body. A body in a chair
is what other players see; what drives the station is the operator field. star-crew-64 kept
`occupant_pid` on the console singleton and left it set when a player swapped away, so the seat
stayed locked to an NPC body and the helm's last steer value kept turning the ship
(`src/main.c:562-597`, `:794`, `:940-942`).

**Operator changes take effect at the next 30 Hz tick, from the current state.** When a player
stops operating a station (stands, disconnects, is downed, swaps body, or is relieved), the
station's operator becomes `AUTO` or `MERGED` at the start of the next tick (33 ms). Continuous
inputs do not freeze: the helm's stick input is dropped and its setpoints become the ship's
current heading and speed, so the ship holds course instead of turning on a stale stick. A turret
keeps its mode and target. Nothing a player set is reverted.

### 4. Automation: the ship's computer

**What it is.** Automation is one module in `sc-core` (`automation`), stepped with the ship
systems at 10 Hz on the server. It is the ship's computer running a station, not an NPC. It reads
the same state a console shows and issues the same commands a console sends, through the same
validation (`netcode-and-sessions` section 6). There is no automation-only path into a system:
if a console cannot do it, automation cannot.

**It works on setpoints.** Players change setpoints; automation and the systems' control loops
maintain them. A player who opens a merged Engineering tab and raises the shields' power setpoint
has set a setpoint; automation keeps it and stops applying its preset to that load until the
condition changes or a player picks a preset. The rule: **a manual setpoint wins until a player
or a condition change replaces it.**

**It needs the computer core.** Automation draws on the `computer` system in `computer_core`
(layout, decided). Proposed: while the core is powered and above 50 % integrity, automation runs
at the competence below; between 1 % and 50 %, its reaction times and its turret aim error double;
unpowered or destroyed, automation stops deciding, and every automated station holds its last
setpoints passively (the flight control loop still holds heading and speed; turrets stop firing;
scans stop; breakers are not reset). The load and its priority are `power-grid`'s; integrity is
`damage-control`'s.

**Competence** (proposed, tuned in `data/stations.json`; a manned station has none of these
limits):

| Station | Reacts in | Quality | Does | Will not |
| --- | ---: | --- | --- | --- |
| Helm | 1.5 s | Holds heading within 2 deg and speed within 2 % of maximum; turns at 70 % of the maximum turn rate | Holds course and speed; follows the plotted course; obeys orders: come to heading, intercept, match speed, keep station, orbit, all stop; turns the bow so the tubes bear on Tactical's target when Tactical asks | Fly evasive patterns; ram; enter a hazard (atmosphere, debris, a star's corona); dock; choose a destination; exceed the dampers' capacity |
| Tactical | 2.0 s | Target choice by threat score (below) | Raises shields at red alert; assigns each turret to the highest-threat hostile in its arc; sets point defence against inbound missiles; applies the shield preset facing the nearest threat | Fire missiles without a "missiles free" order or a direct order naming the target; fire on a contact not identified hostile; fire on a contact that has surrendered or is disabled |
| Turret (automated) | 0.8 s to acquire | Tracks at 50 % of the mount's slew rate; leads for constant velocity only; aim error 4 mrad (1 sigma) | Engages its assigned target, else the nearest hostile in arc | Fire within 50 m of a friendly craft's line of fire; fire beyond 3 km |
| Engineering | 3.0 s | | Applies the condition's preset; resets a tripped breaker 10 s after its fault clears; sheds load by priority when supply falls short; leaves the reactor in `power-grid`'s automatic mode, which follows the load the grid can take at 10-100 % of rated and never overdrives (reconciled 2026-10-05: power-grid owns this; it read "keeps the reactor under 90 % of its thermal limit") | Scram the reactor (the reactor's own interlock does that, it is physics, not a decision); vent; close a breaker a player locked open; shed life support or the computer core |
| Science | 2.5 s | Scans take 1.5 times as long as a manned scan | Passive-scans new contacts nearest first; moves shield allocation toward the face taking fire at 5 % of capacity per second; keeps the viewscreen on the forward feed unless the captain overrides | Ping actively (it reveals the ship); change shield frequency |
| Comms | 5.0 s | | Answers routine hails with standard replies; requests clearances on order; logs intercepts | Negotiate; offer or accept surrender; send a distress call unordered; decode intercepts |
| Flight ops | 4.0 s per sequence step | Standard step times | Runs launch and recovery sequences on order; keeps empty bays pressurized; recovers returning craft | Launch without an order; depressurize a bay with an unsuited person in it (a hardware interlock refuses this anyway); open a drop door on an unsecured craft |
| Captain | 1.0 s | | Auto-condition: red alert when a contact identified hostile comes within 15 km; normal 60 s after the last hostile is beyond 25 km (hysteresis, so it does not flicker) | Give orders; anything else |
| Damage board | 5.0 s | | Dispatches damage control teams by priority: fire, breach, power, other systems; closes doors to contain fire and vacuum | Send teams into vacuum without suits; vent; lock crew in |

**Threat score** (Tactical's automation, proposed): for each contact identified hostile,

```text
threat = class_weight x aim_factor x (1 - range_m / sensor_range_m)
class_weight: missile 3.0, frigate 2.0, fighter 1.0, other 0.5   (data/stations.json)
aim_factor:   1.0 if it is targeting this ship, else 0.5
```

The automated turret's 4 mrad aim error is 4 m at 1 km against a 7 m fighter. The intended feel is
that automation lands about half the hits a practised gunner does against a jinking fighter, and
nearly as many against a ship flying straight; that is a target for `weapons-and-shields`' test
range to measure, not a claim.

**Orders to automation.** An automated station acknowledges an order in its reaction time when
`automation::can_execute(order)` says it can, and replies "unable" with the reason when it cannot
(the same function the captain's order composer calls to grey out an order before it is sent:
preview is resolver). A direct order is an authorization: "Tactical: fire tube 1 at T3" makes the
automation fire, though it would never choose to.

### 5. Merging and player counts

**Merging** (proposed): every station that has no seated player keeps its automation running and
appears as a **tab** on a manned console: the first manned station in its merge list
(`data/stations.json`, flat lists, no chains). The host player can open the tab and change any
setpoint; automation keeps running underneath and keeps what the player set (section 4). With one
player every station is a tab on that player's console, which is how one player runs the ship.

| Station | Merge list (first manned wins) |
| --- | --- |
| Helm | Tactical, Captain, Science, Engineering, Comms, Flight ops |
| Tactical | Helm, Captain, Science, Engineering, Comms, Flight ops |
| Engineering | Science, Captain, Helm, Tactical, Comms, Flight ops |
| Science | Engineering, Captain, Helm, Tactical, Comms, Flight ops |
| Comms | Science, Captain, Engineering, Helm, Tactical, Flight ops |
| Flight ops | Tactical, Captain, Helm, Science, Engineering, Comms |
| Damage board | Engineering, Captain, Science, Helm, Tactical |
| Captain | Not a tab: its command functions appear in the title band of every manned bridge console while no captain is seated |
| Gunners | No merge: automation, with modes set from Tactical. A remote turret sight would fit the budget's secondary view, but it would halve the point of the pods (question B10) |

**Player counts** (proposed recommended seating; any seating works):

| Players | Seated (recommended) | Tabs on each console | Automated without a tab |
| ---: | --- | --- | --- |
| 1 | Helm | Helm: Tactical, Engineering, Science, Comms, Flight ops, Damage board; command in the title band | Gunners |
| 2 | Helm, Engineering | Helm: Tactical, Flight ops. Engineering: Science, Comms, Damage board | Gunners |
| 3 | Helm, Tactical, Engineering | Tactical: Flight ops. Engineering: Science, Comms, Damage board | Gunners |
| 4 | Helm, Tactical, Engineering, Science (the core) | Tactical: Flight ops. Engineering: Damage board. Science: Comms | Gunners |
| 5 | Core four and Captain | Captain gets no tabs (it is in every list's second place, but the role's own station comes first); command leaves the title bands | Gunners |
| 6 | Core five and one of: Flight ops (missions with fighters), Comms (diplomatic missions), a gunner, a Swift pilot | As above, less the seated station | The rest |
| 7 | Six and one of the above, or the Damage board | | |
| 8 | Seven and one of the above | | |

With five or more players, off-bridge seats give work away from the room: the Damage board, the
Engineering bay console, two Swift pilots (`shuttle-bay-and-fighters`) and four gunners. Eight
players never run out of seats: the Tern has fourteen stations and two cockpits.

### 6. Seats: claim, release, relieve, swap, switch

All of this is server state applied at tick boundaries (`netcode-and-sessions` sections 6 and 7).

**Claim (sit).** The player uses a seat. The server grants it when:
- the player's body is standing within 1.5 m of the seat point (the layout's `seat_m`), not
  downed, not carrying a two-handed load, and not climbing;
- the seat is empty, or held by an NPC body (section "Relieve"), or the station is automated.

On the grant, the body snaps into the seat over 0.4 s (a baked clip, `crew-on-deck`), the occupant
becomes that body and the operator becomes that player. If two players claim one seat in the same
tick, the lower seat-table order of their commands wins (`netcode-and-sessions`: by seat id, then
arrival), and the other gets "refused: taken".

**Release (stand).** Holding Stand (section 8.6) for 0.5 s stands the body up beside the seat over
0.4 s. The operator becomes `AUTO` or `MERGED` at the next tick (section 3). Release also happens,
with the same effect, when the player:
- disconnects: the body stays seated as an NPC body (netcode's rule); on reconnect within 120 s
  the player gets that body back and, if the seat still holds it, the station;
- is downed: the body slumps out of the chair onto the floor beside it, so a downed body never
  blocks a seat (`crew-on-deck` owns the downed state);
- swaps body (below);
- picks another station from the mess console before a mission (a reservation, below).

**Relieve.** A seat held by an NPC body is always claimable. The claiming player's body walks up,
the NPC body stands and walks to the next station on its post list (section 6.1), and the player
sits: 0.8 s in all. An NPC body never holds a station against a player.

**Body swap** (proposed, on by default; a session setting). A player may swap into any NPC body
aboard from the crew panel (F9 or the gamepad's Back button), with a 10 s cooldown:
- the player's old body becomes an NPC body where it stands or sits; if it was seated, its station
  goes to automation at the next tick (it keeps the chair cosmetically, and anyone may relieve it);
- if the new body is seated, the player becomes the operator of that station at the next tick;
- a downed body cannot be swapped into; a player may swap out of their own downed body (they can
  then walk over and revive it, `crew-on-deck`);
- a player never swaps into another player's body.

This keeps star-crew-64's body swap, which made one to four players work (`src/main.c:562-597`),
and removes its bug (section 3).

**Switching station.** Four ways, cheapest first:
1. **A tab**: operate a merged station from your own console without moving (section 5).
2. **Walk**: stand, walk, sit. Bridge seats are 2.2-11 m apart: 1-6 s at 1.8 m/s
   (`crew-on-deck`).
3. **Body swap** into an NPC body that is already seated there.
4. **At muster**: in the mess before a mission, the mess console lists stations; picking one
   reserves it. When the host starts the mission, every player with a reservation starts seated
   at it (the mission opens at stations; walking from the mess is not the first thing anyone
   does). A reservation does nothing else; automation holds the station until the player sits.

#### 6.1 NPC bodies

The Tern carries a **watch of four bodies** (proposed): at mission start there is one body per core
station, a player's or an NPC's. Players five to eight bring their own bodies, spawned in the
quarters. NPC bodies are presentation and spare bodies: they sit at automated core stations, can
be hurt, downed and revived, count toward "every body down" (`crew-on-deck`), and are what body
swap swaps into. They never operate a station; automation does that. An NPC body without a seat
walks its post list (helm, tactical, engineering, science, then the mess) to the first core seat
with no occupant.

### 7. Command: condition, orders, alerts

**Condition** (proposed): normal or red alert, set by the captain, or, while no captain is seated,
by any player seated at a bridge console (from the title band), or by the captain's auto-condition
(section 4). Setting red alert is a guarded control (hold 0.6 s).

Red alert does, ship-wide, at the next tick:
- **Lighting.** Every compartment whose lamps are powered blends from its normal to its red alert
  vertex colour set over 0.5 s (`light-baking` bakes the sets; the blend is a per-compartment
  uniform, so this costs no geometry). Floor strips pulse at 0.5 Hz while red alert holds (also a
  uniform).
- **Sound.** A klaxon: three whoops in 3.0 s on entering red alert, then one whoop every 30 s.
- **Automation presets.** Engineering automation applies the Combat preset; Tactical automation
  raises shields and sets automated turrets to engage hostiles; Science automation stops
  low-priority scans.
- **Consoles.** Every console's title band and condition chip turn the alert colour role.

Returning to normal reverses the lighting over 2.0 s and applies the Cruise preset to automated
Engineering. A player's manual setpoints survive both changes (section 4) except where the player
picked a preset, which the condition then replaces.

**Emergency power is not a condition.** A compartment shows its emergency lighting set when its
lighting load is unpowered and the emergency bus is up (`power-grid` decides which). It overrides
red alert in that compartment, and the condition chip still reads RED ALERT.

**Brace** (proposed): a captain's (or title band) control that announces "brace" ship-wide with
a 3 s countdown. Standing crew who are braced (`crew-on-deck`) are not knocked down by a lurch
under three times grip (17.7 m/s2 of residual). It costs nothing to call and makes helm and
command talk.

**Orders.** An order is `{ to: station, verb, object, issued_at }`, sent as a reliable command. The
verbs per station are data (`data/stations.json`):

| To | Verbs |
| --- | --- |
| Helm | come to heading H; intercept T; match speed with T; keep station on T at R m; orbit B; all stop; evasive; bring the tubes to bear on T |
| Tactical | target T; weapons free; hold fire; missiles free; fire tube N at T; shields preset P |
| Engineering | prioritise load L; preset P; repair system S |
| Science | scan T; viewscreen feed F; balance shields to face X |
| Comms | hail T; send distress; request clearance from T |
| Flight ops | launch C; recover C; task C to escort, attack T, patrol, return |
| Damage board | send a team to compartment K; seal compartment K |

The recipient's title band shows the newest order with a chime. Its life:
**sent**, then **acknowledged** or **unable** (with a reason from a list: no power, out of arc,
busy, cannot comply), then **done** (set automatically when the server can measure it: the heading
is reached, the craft has launched), **superseded** by a newer order to the same station, or
**expired** after 60 s. A player acknowledges with Y; automation answers within its reaction time
(section 4). The captain sees every order's state in the Orders panel.

**Viewscreen override.** The captain can take the viewscreen's feed for 30 s or until released;
Science's feed control shows "held by Captain" meanwhile.

### 8. The console UI framework

CLAUDE.md 10: consoles are full-screen 2D when seated, the bridge stays visible behind or beside
them, panels are a fixed size, and the engine's immediate-mode UI draws them, not render-to-texture.
`engine-stack` decides the library (section 4 there, question E3, recommendation taken): **`egui`
with `egui_glow`, inside our fixed-panel rules**, measured by the Pi 5 probe, with a minimal panel
layer of our own as the fallback. The rules below are ours and hold whichever library draws them:

- **egui draws; the grid decides.** Each panel is a child `Ui` given a fixed rectangle from the
  grid (section 8.1) and its own clip rectangle. egui's automatic layout works inside a panel and
  never sets a panel's size or position; a panel never grows to fit its content.
- **One logical pixel is one egui point.** egui's `pixels_per_point` is 1.5 on the 1920 x 1080
  output, so the lp numbers below are the numbers the console code uses.
- **Text is egui text, clipped.** Labels truncate with an ellipsis at the panel's inner width;
  lists are `ScrollArea`s inside the panel body that scroll in whole rows.
- **The theme is ours.** `data/ui/theme.json` (section 8.3) sets egui's `Visuals` and text styles
  once at startup; no console sets a colour or a font size of its own.
- **If the probe rejects egui** (question E3), the console data files, the grid, the bands, the
  widgets and the bindings below do not change: only the layer that draws them does.

#### 8.1 The canvas and its bands

The console is laid out on a **1280 x 720 logical canvas** (logical pixels, lp). The Pi 5 drives a
1920 x 1080 display with the 3D scene rendered at 1280 x 720 and scaled up (`engine-stack`); the UI
draws at the output resolution, 1.5 device pixels per lp (egui's `pixels_per_point`), from a font
atlas egui rasterizes at that scale, so text stays crisp. Every number below is in lp.

```text
 y    0 +--------------------------------------------------------------------------+
        | TITLE BAND 32: tabs (station first, then merged) | newest order | COND | clock |
 y   32 +--------------------------------------------------------------------------+
        |                                                                          |
        | LOOK BAND 240: no UI. The 3D view from the seat's eye toward the         |
        | viewscreen, rendered into a 1280 x 240 viewport.                         |
        |                                                                          |
 y  272 +--------------------------------------------------------------------------+
 y  280 |  PANEL GRID: 12 columns x 4 rows, x 20-1260, y 280-680                  |
        |  column 96 lp, row 94 lp, gutter 8 lp                                    |
 y  680 +--------------------------------------------------------------------------+
 y  688 | STATUS STRIP 32: hull, six shield faces, reactor MW, speed, fires, breaches |
 y  720 +--------------------------------------------------------------------------+
```

- **Title band** (y 0-32): x 0-420 the tab chips (64 lp each: the station's own tab first, then
  merged stations in merge-list order, at most six visible, then a "+N" chip); x 420-940 the
  newest order to this station; x 940-1280 the condition chip, then the mission clock (mm:ss).
- **Look band** (y 32-272): the bridge stays visible above the console (CLAUDE.md 10). The 3D
  passes render into a 1280 x 240 lp viewport from the seat's eye (seat point plus 1.20 m) with a
  horizontal field of view of 100 deg, aimed at the viewscreen's centre. For the forward stations
  (helm, tactical, captain) that is roughly where the player faces. The side stations face the
  walls, so their look band shows the view swivelled toward the viewscreen; their avatar's head
  turns at most 80 deg and the rest is presentation (question B1). Rendering only this band is a
  fill saving: 33 % of the screen's 3D pixels while seated.
- **Panel grid** (y 280-680, x 20-1260): 12 columns by 4 rows. A panel at column c, row r, w
  columns wide and h rows tall sits at `x = 20 + 104c`, `y = 280 + 102r`, size
  `(104w - 8) x (102h - 8)` lp. Widths: 1 col 96, 2 cols 200, 3 cols 304, 4 cols 408, 5 cols 512,
  6 cols 616, 12 cols 1240. Heights: 1 row 94, 2 rows 196, 4 rows 400. Panels never overlap, never
  change size, and every console fills the grid exactly (a validator checks both, section 13).
- **Status strip** (y 688-720): the same strip on every console, drawn by one function: hull %,
  six shield face mini-bars, reactor output MW, speed m/s, counts of fires and breaches, and the
  operator's name.
- **Look up.** Tab (or the right stick click) lowers the console to its title band and frees the
  mouse to look around from the seat (yaw +/-100 deg from the seat's facing, pitch +/-60 deg). It
  is how a seated player sees the room.

#### 8.2 Panel anatomy and text bands

Inside a panel (sizes in lp; text sizes are glyph cell heights):

| Band | y from panel top | Holds |
| --- | --- | --- |
| Header | 0-22 | Panel title, 13 lp bold capitals, with a 2 lp rule in the station's role colour |
| Body | 26 to height - 32 (or to height - 4 without a footer) | Rows of 18 lp (13 lp text); big numerals 28 lp (monospaced digits); plots and diagrams |
| Footer | height - 28 to height - 4 | Buttons, 24 lp tall |

Padding is 6 lp on each side. Text is clipped to the panel's inner width with an ellipsis, numbers
are right-aligned in fixed-width fields so digits never jitter, every number shows its unit, and
lists scroll in whole rows inside the body (CLAUDE.md 10).

#### 8.3 Colour roles

Consoles name colour roles; only the theme (`data/ui/theme.json`, proposed; the mockup's
`ShipKit.PALETTE`) says what they are.

| Role | Used for |
| --- | --- |
| `ui.bg` | The canvas behind panels |
| `ui.panel` | Panel fill |
| `ui.line` | Panel borders, grid lines, plot rings |
| `ui.text`, `ui.text_dim` | Values and labels |
| `ui.accent` | The station's role colour (`PALETTE.role`): headers, the active tab, selections |
| `ui.ok`, `ui.warn`, `ui.danger` | Healthy, degraded, failing states |
| `ui.alert` | Red alert: the title band and condition chip |
| `ui.preview` | The ghost of a pending change: the accent at 45 % alpha, hatched |
| `ui.focus` | The 2 lp outline of the focused control (gamepad and keyboard) |
| `ui.disabled` | A control that cannot act now; its reason shows in its row |
| IFF: `iff.friendly`, `iff.neutral`, `iff.hostile`, `iff.unknown` | Contacts on every plot |

#### 8.4 Widgets

Label; value (number and unit); bar (fill for the actual value, a tick for the setpoint, a ghost
for the preview); slider (a bar you drag); button; guarded button; toggle; list (fixed rows,
scrolls); plot (a 2D top-down or polar view with a world-to-panel transform); gauge; diagram
(nodes and edges, for the power buses). Every widget has a stable id (panel id and index, which is
also its egui `Id`) so focus and network commands can name it. Each is a small function over egui's
painter and response types, written once and shared by every console.

#### 8.5 Guarded controls

A control that is hard to undo needs a deliberate act: **hold to confirm** for 0.6 s (a ring fills)
or **arm, then fire** within 3 s. Guarded: fire a missile, scram, open a main bus breaker, vent a
compartment, open a drop door, depressurize a bay, active ping, red alert, distress call, launch.

#### 8.6 Input

Every input device drives every console (CLAUDE.md 10): whichever keyboard, mouse or pad the
player is using. egui takes pointer and keyboard events from SDL3 (the platform layer,
`engine-stack` section 4); our input layer turns the gamepad's D-pad into directional focus moves
between widgets and A into activation, so a pad needs no mouse emulation. Gamepad names follow
SDL3's gamepad API positions (A south, B east, X west, Y north: `SDL_GAMEPAD_BUTTON_SOUTH` and so
on), so a layout reads the same on any pad. Shared bindings while seated:

| Action | Keyboard and mouse | Gamepad |
| --- | --- | --- |
| Point at a control, activate it | Mouse; left click | D-pad moves focus to the nearest control in that direction; A |
| Adjust the value under the pointer or focus | Mouse wheel; Left and Right arrows | D-pad left and right on a focused bar |
| Back, cancel, close | Right click; Backspace | B (tap) |
| Stand up | Hold E 0.5 s (a tap does nothing while seated) | Hold B 0.6 s |
| Look up (lower the console) | Tab | Right stick click |
| Previous and next tab | Ctrl+Page Up and Ctrl+Page Down; F1-F8 jump to a tab | LB and RB |
| Acknowledge the newest order; unable | Y; Shift+Y | Left stick click; hold it 0.6 s |
| Crew panel (station table, body swap) | F9 | Back |
| Menu | Esc | Start |

Station bindings are in section 10. Keys and buttons are data (`data/input/bindings.json`,
proposed) and can be rebound.

#### 8.7 Cost

egui tessellates a frame into meshes, one per run of shapes that share a clip rectangle and a
texture, and `egui_glow` draws each mesh with one call. Under the fixed-panel rules a panel is one
clip rectangle, so the calls follow the panel count:

| Part | Draw calls |
| --- | ---: |
| Title band and status strip | 2 |
| Panels (at most 7 per console, a validator rule; Engineering's 6 are the most in section 10) | 7 |
| A scrolling list inside a panel (its own clip rectangle) | at most 3 |
| A texture in a panel: Science's viewscreen thumbnail, or one secondary feed (section 11.2) | 1 |
| Headroom for egui splitting a run when text and shapes interleave | 3 |
| **Ceiling** | **16** |

Geometry: at most 6,000 triangles (about 3,000 glyph and panel quads, plus egui's anti-aliasing
feather on lines and rings). egui's vertex is 20 bytes (position 2 x f32, UV 2 x f32, colour RGBA8),
so the frame streams at most about 240 KB of vertices and 72 KB of 32-bit indices. The font atlas
holds the three text sizes of section 8.2 at 1.5x: estimated 1024 x 1024 RGBA8, 4 MB of the 96 MB
texture budget. Building and tessellating a console stays inside `engine-stack`'s UI share of the
client main thread (2 ms); the probe measures it with 200 panels and 4,000 glyphs, far more than any
console here, and a console that does not fit is cut down, not the rule.

### 9. Preview is resolver, console by console

Each preview calls the `sc-core` function the server calls to resolve the action (CLAUDE.md 6.1),
on the client's replicated state with the proposed change applied. Previews read "about" (a
leading `~`) because replicated state is up to 100 ms old (`netcode-and-sessions` section 5).

| Console | Preview shown | Function (owner) |
| --- | --- | --- |
| Helm | Time to reach an ordered heading; peak acceleration; whether the dampers hold it | `flight::plan_turn` (`flight-and-navigation`), `frames::damper_load` (`ship-frames`) |
| Helm | Maximum speed and acceleration at the impulse drive's current power | `flight::performance(power)` |
| Tactical | Hit chance of each turret and tube on the selected target | `weapons::hit_chance` (`weapons-and-shields`) |
| Tactical | Time for the tubes to bear (a helm turn) and a missile's time to impact | `flight::plan_turn`, `weapons::missile_intercept` |
| Engineering | Delivered MW for every load if a setpoint, priority or breaker changes | `power::solve` (`power-grid`) |
| Engineering | Reactor heat in 30 s at the proposed output | `heat::project` (`power-grid`) |
| Science | Time to scan a contact at the sensors' current power | `sensors::scan_time` (`weapons-and-shields` or a sensors section there) |
| Science, Tactical | Each face's capacity in MJ after a rebalance or preset | `shields::allocate` (`weapons-and-shields`) |
| Flight ops, Bay control | Time to pump a bay down to 5 kPa (launch permitted, where the pumps stop) or up to 101 kPa | `life_support::time_to_pressure` (`life-support`; corrected 2026-10-04 from 1 kPa) |
| Flight ops | Total launch sequence time | `bays::sequence_time` (`shuttle-bay-and-fighters`) |
| Damage board | Time for a team to repair a system | `damage::repair_time` (`damage-control`) |
| Captain | Whether a station can execute an order (greys it out before sending) | `automation::can_execute` (this change) |
| Gunner | The lead reticle | `weapons::intercept_point`, the same solve automated turrets use |

### 10. The stations in detail

Wireframes use the grid of section 8.1: `(c, r, w, h)` in columns and rows, then `x, y, w x h`
in lp. All four core consoles, the captain's, comms' and flight ops' are drawn in the mockup.

#### 10.1 Helm

**Purpose.** Fly the ship: attitude, throttle, course, evasive patterns, docking. Helm is the only
seat that turns the ship by hand, which is why Tactical asks it to bring the tubes to bear.

| Panel | Grid | lp | Contents |
| --- | --- | --- | --- |
| H1 Heading | (0,0,4,2) | 20, 280, 408 x 196 | Heading tape (30 deg per 100 lp) with the heading in 28 lp digits and the ordered heading's bug; pitch and roll in deg; turn rate deg/s |
| H2 Throttle | (4,0,2,4) | 436, 280, 200 x 400 | Vertical throttle from -25 % to 100 % with the setpoint tick; speed m/s in 28 lp digits; maximum speed at current power (preview); impulse power delivered / requested MW; damper load %; ALL STOP |
| H3 Navigation | (6,0,6,4) | 644, 280, 616 x 400 | Top-down plot at 5, 20 or 100 km: the ship, the plotted course, waypoints, contacts in IFF colours, bodies; ETA; footer: HOLD, COURSE, INTERCEPT, MATCH, ORBIT |
| H4 Manoeuvre | (0,2,4,2) | 20, 484, 408 x 196 | Evasive patterns JINK, SPIRAL, BREAK PORT, BREAK STBD; the turn preview ("to 090: ~7.4 s, peak 2.1 g, dampers 78 %"); "tubes bear on T2 in ~4.1 s" when Tactical has a target |

| Action | Keyboard and mouse | Gamepad |
| --- | --- | --- |
| Yaw | A, D | Left stick X |
| Pitch (nose down on W, invertible) | W, S | Left stick Y |
| Roll | Z, C | Right stick X |
| Throttle up, down (tap 5 %, hold slews 25 %/s) | R, F | RT, LT (slew rate follows the trigger) |
| All stop | X | X |
| Evasive pattern | 1-4 | Hold Y, pick with the D-pad |
| Autopilot hold, course, intercept, match | H, G, I, M | Footer buttons by focus |

Automation and merging: section 4 and section 5. star-crew-64's helm let the ship reverse
(stick Y in [-1, 1]) and coast when empty (`bridge_panel.c:137-166`, `main.c:794`); here reverse is
limited to -25 % and an empty helm holds course.

#### 10.2 Tactical

**Purpose.** Choose targets, set what each turret does and who mans it, load and fire the two
missile tubes, and set the shield preset. Tactical decides; gunners and automation shoot.

| Panel | Grid | lp | Contents |
| --- | --- | --- | --- |
| T1 Targets | (0,0,3,4) | 20, 280, 304 x 400 | Contacts sorted by threat: id, class, range km, bearing deg, closing m/s, IFF; the selected row expands: hull %, the shield face it shows us, hit chance per weapon (preview) |
| T2 Tactical plot | (3,0,5,4) | 332, 280, 512 x 400 | Ship-centred top-down at 2, 5 or 10 km: the four turret arcs, the tubes' 10 deg arc, target vectors, inbound missiles; the shield ring (bow, stern, port, starboard as ring segments, dorsal and ventral as two marks); footer: preset BAL, BOW, STN, PRT, STB, DOR, VEN |
| T3 Turrets | (8,0,4,2) | 852, 280, 408 x 196 | Dorsal, ventral, port, starboard: operator (player name or AUTO), mode (AUTO, HOLD, PD, on T2), heat bar, capacitor bar, hit chance on the selected target |
| T4 Tubes | (8,2,4,2) | 852, 484, 408 x 196 | Tube 1 and tube 2: EMPTY, LOADING 7.2 s, READY, FIRED; Gannets in the magazine; hoist state; lock and arc on the target; FIRE (guarded) with "impact ~11.4 s, hit ~72 %" |

| Action | Keyboard and mouse | Gamepad |
| --- | --- | --- |
| Select a target | Click it; T next by threat, Shift+T previous | Right stick moves a cursor that snaps to contacts; A; X next by threat |
| Select a turret, cycle its mode, assign it to the target | 1-4; M; Shift+M | LT with D-pad up and down, then left and right for mode; LT+A assigns |
| Select a tube, load it, fire it | 5, 6; L; hold G 0.6 s | Hold Y and pick with the D-pad; RT held 0.6 s fires |
| Shield preset; balanced | S cycles; Shift+S | Footer buttons by focus |
| All turrets hold fire / weapons free | Hold K 0.6 s | Footer buttons by focus |

**Shared with Science: the shield allocation.** Tactical's preset and Science's fine balance both
write one value: the six faces' share of shield capacity, summing to 100 %. A preset sets all six in
one press; Science adjusts them face by face. The last command wins, and both consoles show who set
it and when ("set by Science 4 s ago"). Question B5 asks whether Tactical should keep the presets.

#### 10.3 Engineering

**Purpose.** Decide where the reactor's power goes and keep the plant alive: power setpoints and
priorities per load group, breakers, the reactor's output, coolant, life support setpoints and the
order of repairs. Engineering's power budget decides whether shields or turrets win (vision,
pillar 4).

| Panel | Grid | lp | Contents |
| --- | --- | --- | --- |
| E1 Power allocation | (0,0,5,4) | 20, 280, 512 x 400 | One row per load group (28 lp each), the eleven groups of `power-grid`'s `power.json`: impulse drive and RCS, shields, turrets, tubes and hoist, sensors, comms, life support, gravity and dampers, computer core, bays and craft charging, medbay. Each row: name, priority 1-3, the setpoint in percent (0-150 % in 5 % steps; 0 is off, above 100 % overdrive), a bar with wanted MW (tick), delivered MW (fill) and the preview (ghost), and wanted / delivered MW beside it. Footer: CRUISE, COMBAT, SILENT, EMERG presets; supply and demand MW. Reconciled 2026-10-05: power-grid owns this (it read ten groups with requests in MW) |
| E2 Buses | (5,0,4,2) | 540, 280, 408 x 196 | One-line diagram: reactor to the main switchboard, the port and starboard main buses, the battery bank through the forward switchboard to the emergency bus, the cross-tie; breakers as boxes coloured closed, open, tripped or locked; MW on each line |
| E3 Reactor | (9,0,3,2) | 956, 280, 304 x 196 | `power-grid`'s reactor readout: state (and a scram's cause), mode (AUTO, following the load, or MANUAL), throttle and target in percent of rated, electric and thermal MW, blanket K, fuel kg, integrity; the blanket in 30 s at the proposed throttle (`heat::project`); AUTO, MANUAL, SCRAM (guarded). Reconciled 2026-10-05: power-grid owns this (it read an output setpoint in MW and a containment temperature) |
| E4 Coolant | (5,2,3,2) | 540, 484, 304 x 196 | Loop temperature in and out deg C; flow kg/s; pumps 1 and 2; radiator rejection MW; heat sink % |
| E5 Life support | (8,2,2,2) | 852, 484, 200 x 196 | Ship O2 kPa, CO2 kPa, temperature deg C against their setpoints; the worst compartment |
| E6 Repairs | (10,2,2,2) | 1060, 484, 200 x 196 | The repair queue's top four with progress; teams |

| Action | Keyboard and mouse | Gamepad |
| --- | --- | --- |
| Select a load group | Click; 1-0 and the minus key (eleven groups) | D-pad up and down in E1 |
| Raise, lower its setpoint (`power-grid`'s 5 % step; Shift 25 %) | W, S or the wheel; drag the bar | D-pad left and right; hold RT for 25 % steps |
| Cycle its priority | P | X |
| Preset | Shift+1-4 | Hold Y, pick with the D-pad |
| Toggle the focused breaker (main bus breakers guarded) | Click, hold 0.6 s for a guarded one | A, hold 0.6 s for a guarded one |
| Reset every tripped breaker | Ctrl+R | Footer button |
| Scram | Arm with Shift+X, confirm with X within 3 s | Footer button, arm then confirm |

**Engineering bay console** (`eng_main`) adds a **Reactor (hands-on)** tab: scram reset (hold 3 s,
only here: a scrammed reactor is restarted by someone standing at it), manual coolant valves, and
local overrides of main switchboard breakers that work when the bridge's control circuit is cut.

star-crew-64's engineering pumped one of three channels with a stick and rebalanced the others in
proportion (`engineering_console.c:135-166, 218-229`). The zero-sum feel carries over through
priorities: when demand exceeds supply, `power::solve` sheds the lowest priority first. How it
sheds within a priority is `power-grid`'s.

#### 10.4 Science

**Purpose.** See and understand: sensors and scans, the shields' face balance and frequency, and
the viewscreen's feed. Science replaces star-crew-64's rhythm game (`science_console.c:168-209`)
with the real face balance the prototype's design document proposed.

| Panel | Grid | lp | Contents |
| --- | --- | --- | --- |
| S1 Sensors | (0,0,5,4) | 20, 280, 512 x 400 | Polar plot at 5, 25 or 100 km; contacts in IFF colours with unknown ones as hollow marks; the scan cone; PASSIVE or ACTIVE; footer: range, PING (guarded) |
| S2 Contact | (5,0,3,4) | 540, 280, 304 x 400 | Designation, class, IFF, range, bearing; hull %; shield faces (after a scan); systems and weaknesses (after a full scan); scan progress with "full scan ~6.0 s at 1.2 MW"; SCAN |
| S3 Shields | (8,0,4,2) | 852, 280, 408 x 196 | The six faces on a top view (bow, stern, port, starboard) and a side view (dorsal, ventral): stored MJ / capacity MJ and share %; frequency A-F; recharge MW; who set the allocation and when |
| S4 Viewscreen | (8,2,4,2) | 852, 484, 408 x 196 | Feed buttons FWD, AFT, PORT, STBD, TARGET, CHASE; zoom 1x-8x; a 256 x 128 lp thumbnail of the viewscreen texture (the render target itself, no extra pass) |

| Action | Keyboard and mouse | Gamepad |
| --- | --- | --- |
| Select a contact; next contact | Click; C | Right stick cursor and A; X next |
| Scan the selected contact | Q | Hold X 0.3 s |
| Active ping | Hold P 0.6 s | Footer button, hold |
| Select a shield face; raise, lower its share 5 % | 1-6; W, S | D-pad in S3 |
| Balance all faces evenly | B | Footer button |
| Cycle frequency | F | Footer button |
| Viewscreen feed; zoom | V cycles; Z, Shift+Z | Hold Y, pick with the D-pad; LT, RT |

#### 10.5 Captain

**Purpose.** See the whole ship, decide, and say so: condition, orders, brace, and the viewscreen.
The captain has no system of their own, which is the point: the seat is for the player who wants
to command rather than operate.

| Panel | Grid | lp | Contents |
| --- | --- | --- | --- |
| C1 Ship | (0,0,4,4) | 20, 280, 408 x 400 | The three decks in plan with each compartment coloured by state (fire, breach, no power, crew present); hull %; the shield ring |
| C2 Stations | (4,0,4,2) | 436, 280, 408 x 196 | Every station: operator (player, AUTO, or "tab on Helm"), and its newest order's state |
| C3 Orders | (4,2,4,2) | 436, 484, 408 x 196 | Composer: station, verb, object, SEND (greyed by `automation::can_execute` for automated stations); the last four orders |
| C4 Condition | (8,0,4,2) | 852, 280, 408 x 196 | NORMAL, RED ALERT (guarded), BRACE, auto-condition on or off, viewscreen override |
| C5 Mission | (8,2,4,2) | 852, 484, 408 x 196 | Objectives, timers, the debrief score so far |

Bindings: hold R 0.6 s red alert, hold N 0.6 s normal, B brace, O opens the composer, V takes or
releases the viewscreen, 1-7 focus a station in C2. Gamepad: hold Y for the condition menu, X for
the composer, the D-pad and A for everything else.

#### 10.6 Comms

| Panel | Grid | lp | Contents |
| --- | --- | --- | --- |
| M1 Channels | (0,0,4,4) | 20, 280, 408 x 400 | Contacts in comms range: name, channel state (open, hailing, jammed), standing |
| M2 Conversation | (4,0,5,4) | 436, 280, 512 x 400 | The transcript and up to four reply options, each greyed with its reason when its check fails (the same evaluator that resolves it) |
| M3 Intercepts | (9,0,3,2) | 956, 280, 304 x 196 | Intercepted traffic and decode progress |
| M4 Clearances | (9,2,3,2) | 956, 484, 304 x 196 | Docking and passage clearances; DISTRESS (guarded) |

Bindings: click, H hail, 1-4 replies, D decode, hold Shift+D 0.6 s distress; gamepad X hail, the
D-pad and A for replies. Merged into Science when unmanned (vision, decided).

#### 10.7 Flight operations

| Panel | Grid | lp | Contents |
| --- | --- | --- | --- |
| F1 Bays | (0,0,4,4) | 20, 280, 408 x 400 | Hangar, port and starboard launch bays: pressure kPa with the pump-down preview, pressure door and drop door states, who is inside and whether they are suited, pumps, lockout state |
| F2 Craft | (4,0,4,2) | 436, 280, 408 x 196 | Swift 1, Swift 2, Petrel: pilot, state (stowed, ready, launched, returning), fuel %, ammunition, hull % |
| F3 Sequence | (4,2,4,2) | 436, 484, 408 x 196 | The selected craft's launch steps: bay clear, pressure door sealed, pump down (~24 s, life-support's 23.6 s to 5 kPa on layout v2; corrected 2026-10-04 from ~38 s and 2026-10-05 from ~29 s), drop door open, cradle released; each with a tick and its time; LAUNCH (guarded), RECOVER |
| F4 Tasking | (8,0,4,4) | 852, 280, 408 x 400 | Orders to launched craft (escort, attack T, patrol, return) on a small plot |

Bindings: 1-3 select a craft, hold L 1.0 s launch, R recover, hold P 0.6 s pump down or up, hold
O 1.0 s drop door, T tasking. Merged into Tactical when unmanned (vision, decided). The pilots' own
controls are `shuttle-bay-and-fighters`'.

#### 10.8 Engineering bay console

The Engineering console (10.3) plus the Reactor (hands-on) tab. The seat is on the mezzanine
facing aft toward the reactor, so its look band shows the reactor, not the viewscreen.

#### 10.9 Damage control board

| Panel | Grid | Contents |
| --- | --- | --- |
| D1 Compartments | (0,0,6,4) | The 30 compartments on three deck plans with state: fire, smoke, breach, pressure kPa, power, crew |
| D2 Teams | (6,0,3,2) | Damage control teams: location, task, progress |
| D3 Queue | (6,2,3,2) | Repairs by priority, with time to repair (preview) |
| D4 Doors | (9,0,3,4) | Doors and hatches by compartment: open, closed, locked; LOCK (hold 0.6 s); SEAL COMPARTMENT |

Merged into Engineering when unmanned. Whether teams are crew bodies or abstract, and who may vent,
are `damage-control`'s; this seat is where those controls live.

#### 10.10 Bay control

The Flight ops console's F1 and F3 panels plus a **lockout**: while it is on (hold K 1.0 s), no
console can open a drop door or depressurize a bay. Its look band shows the hangar through the
forward landing's window, which is the reason the seat exists: the person who can see into the bay
decides whether it is clear.

#### 10.11 Gunners

The pod seat is a turret sight, not a panel console: the exterior seen from the turret
(`ship-frames` composes the pass), a lead reticle from `weapons::intercept_point`, heat and
capacitor bars, the target's range and closing speed, the turret's arc limits, and Tactical's
assignment. Mouse aim is the raw displacement with no smoothing; the left button fires, the right
button zooms 2x, Shift halves sensitivity. Gamepad: right stick aims, RT fires, LT zooms. A manned
turret tracks at the mount's full slew rate with no aim error beyond the player's own and the
weapon's spread (`weapons-and-shields`). star-crew-64 had no aim reticle at all
(`weapons_console.c`).

### 11. The bridge as a room

#### 11.1 Dimensions and arrangement (decided by the layout)

The bridge is a wedge at the bow (layout schema v2, 2026-10-05; it was one box, x -7.0 to 7.0 m,
y 3.5 to 6.5 m, z 20.0 to 31.0 m). It is 12.4 m deep, from the aft wall at z 20.0 m to the forward
wall at z 32.4 m; 15.2 m across the aft wall and 17.2 m at its widest, just forward of the
chamfered aft corners; the side walls rake in with the hull to 13.2 m across at z 29.4 m, and two
angled walls close it to the 6.8 m forward wall. It stands 3.5 m high (y 3.5 to 7.0 m): 173.9 m2
of floor and 608.6 m3 of air, 0.63 m inside the hull at its tightest, the forward corners
(`tools/layout_check.py`; the box was 14.0 x 11.0 m and 3.0 m high, 154.0 m2, 462.0 m3 and
0.42 m). Its portals: the aft door into the command passage (`p_bridge_aft`, 1.6 x 2.3 m at
z 20.0) and two windows, one in each angled wall.

```text
                                              bow (+Z)
                                 z 32.4: forward wall, 6.8 m across
                              +-----[==== VIEWSCREEN 6.0 m ====]-----+
        [port window]      /                                            \      [stbd window]
                        /   angled walls either side, facing 43 deg out    \
          z 29.4     +                                                        +
                    /      [helm] (1.8, 28.2)     [tactical] (-1.8, 28.2)      \
  port             /                                                            \             stbd
  (+X)            /  [engineering] (6.1, 26.2)           [science] (-6.1, 26.2)  \            (-X)
                 /   faces the raked wall                  faces the raked wall   \
                /                 +------ dais 3.2 x 2.4 m ------+                 \
               /                  |    [captain] (0.0, 24.2)     |                  \
              /                   +------------------------------+                   \
             /  [comms] (6.9, 22.4)                        [flight ops] (-6.9, 22.4)  \
            /   faces the raked wall                            faces the raked wall   \
  z 21.0   +                                                                            +
             \                                                                        /
  z 20.0       +-------------------------[ aft door 1.6 m ]-------------------------+
                     aft wall, 15.2 m across, its corners chamfered (not to scale)
```

Sightlines from each seated eye (seat point plus 1.20 m) to the viewscreen's centre (0.0, 5.2,
32.3), measured from the layout (v2 plan, 2026-10-05, by a scratch instrument that gives the v1
table to within 0.1 deg; on the v1 box the screen's centre was (0.0, 5.0, 30.9), and the seats
were the captain (0.0, 23.2), helm and tactical (+/-1.8, 27.2), engineering and science
(+/-5.4, 25.5) and comms and flight ops (+/-5.0, 21.6), the side four facing straight out at
90 deg):

| Station | Seat (x, z) m | Facing | Distance m | Screen subtends (h x v) | Head turn to screen |
| --- | --- | --- | ---: | --- | ---: |
| Captain | 0.0, 24.2 (dais, eye 5.00 m) | bow | 8.10 | 40.6 x 16.8 deg | 0 deg |
| Helm | 1.8, 28.2 | bow | 4.51 | 65.5 x 29.7 deg | 24 deg right |
| Tactical | -1.8, 28.2 | bow | 4.51 | 65.5 x 29.7 deg | 24 deg left |
| Engineering | 6.1, 26.2 | the port raked wall (yaw 77 deg) | 8.64 | 29.2 x 15.8 deg | 122 deg right |
| Science | -6.1, 26.2 | the starboard raked wall (yaw -77 deg) | 8.64 | 29.2 x 15.8 deg | 122 deg left |
| Comms | 6.9, 22.4 | the port raked wall (yaw 77 deg) | 12.08 | 23.5 x 11.3 deg | 112 deg right |
| Flight ops | -6.9, 22.4 | the starboard raked wall (yaw -77 deg) | 12.08 | 23.5 x 11.3 deg | 112 deg left |

On the v1 box these were 7.70 m and 42.6 x 17.7 deg (captain), 4.13 m, 70.3 x 32.4 deg and 26 deg
(helm, tactical), 7.64 m, 33.3 x 17.8 deg and 135 deg (engineering, science), and 10.56 m,
28.6 x 13.0 deg and 118 deg (comms, flight ops): the screen moved 1.4 m forward with the forward
wall, further than the seats, so every seat sees it a little smaller.

The captain's chair has the classic view: the whole screen at 41 deg, both windows beside it, and
helm and tactical in front. From the captain's eye (5.00 m) the tops of their heads (seat plus
1.32 m, 4.82 m) fall on the screen's plane 4.64 m up but 3.42-3.87 m either side of the
centreline, just past the screen's edges at 3.0 m, so they hide none of it
(`tools/bridge_variants.py --sightlines`, 2026-10-05; this said "they cover only its lower 0.7 m",
which took their height and forgot that they sit to either side). The dais is what keeps them
below the captain's line. Helm and tactical sit close, so the screen fills 66 deg of their view
(v2 plan; was 43 deg, a 3.8 m bottom edge, 0.9 m and 70 deg).
The side stations face the walls with their backs to the room, which gives the captain a view of
every console over its operator's shoulder, and puts the viewscreen behind them: their look band
swivels (section 8.1, question B1).

Proposed furniture (the deck pipeline builds it; the mockup draws it):

| Item | Size | Placement |
| --- | --- | --- |
| Seat | 0.55 m wide, 0.50 m deep, seat at 0.45 m, back to 1.05 m | At the layout's `seat_m`, facing `yaw_deg` |
| Console desk | 1.40 m wide (core) or 1.10 m (others), 0.60 m deep, top at 0.75 m | Centre 0.65 m ahead of the seat |
| Console screen | 1.20 x 0.50 m (core) or 0.90 x 0.45 m, tilted 20 deg back, top at 1.30 m | On the desk's far edge |
| Captain's chair | as a seat, with armrest panels 0.15 m wide | On the dais (layout fixture, decided: 3.2 x 2.4 m, 0.3 m high) |

Clearances these give: 2.2 m between the helm and tactical desks; 4.6 m between the dais and the
nearest side seat; 2.2 m from the aft stations to the aft wall's chamfered corner; 0.27 m between
the engineering and science desks and their raked walls, 0.38 m for comms and flight operations
(v2 plan, 2026-10-05: the side seats sit on the raked walls and face them; were 3.5 m, 1.6 m to
the aft bulkhead and 0.65 m). Every seat and the route to it is checked against the crew collider
(`crew-on-deck`: radius 0.30 m) by the deck compiler (CLAUDE.md 8).

#### 11.2 The viewscreen

The layout places it (decided): 6.0 x 2.4 m, centred at (0.0, 5.2, 32.3), facing aft, 0.10 m
proud of the 6.8 m forward wall (so it never shares the wall's plane), spanning 4.0-6.4 m in height
(v2 plan, 2026-10-05; was centred at (0.0, 5.0, 30.9), spanning 3.8-6.2 m).

- **What it shows** (proposed feeds): FORWARD from the sensor array at the bow (layout system
  `sensor_array`, (0.0, 1.0, 41.5)), AFT, PORT and STARBOARD from hull points, TARGET (the forward
  camera aimed at the selected contact with 1-8x zoom), CHASE (a virtual camera 60 m behind and
  15 m above the ship), and COMMS (the hailing party's card during a conversation, drawn as 2D: no
  3D pass at all).
- **Who steers it.** Science owns the feed and zoom; the captain can override (section 7); Comms
  can ask for it during a hail. With nobody at Science, automation keeps it on FORWARD.
- **How it is drawn.** The exterior scene rendered from the feed's camera into the one viewscreen
  render target of `engine-stack`'s budget: **1024 x 512 texels at 30 Hz**. The camera's aspect is
  the screen's (2.5:1), so each texel is 1.25 times wider than tall: 171 texels per metre across.
  The camera follows the ship's exterior pose (`ship-frames`), so the picture turns when the ship
  turns while the room does not: the interior and exterior are decoupled.
- **When it is skipped.** The render target is per client and only presentation. A client skips
  the pass when no local view can see the screen (the player is off the bridge, or seated at a
  side station with the look band and Science's thumbnail both not showing it).
- **Secondary feeds on consoles.** `engine-stack`'s budget allows up to two more views at 512 x 256
  and at most 15 Hz. The bridge uses at most one per client, and only while a console shows it:
  Flight ops' F4 can swap its plot for the tasked craft's camera (CRAFT CAM, a footer toggle), and
  Bay control's F1 can show the hangar camera. Science's S4 thumbnail is the viewscreen's own
  texture and costs no pass. Tactical has no turret view (question B10). The second slot is left
  to `shuttle-bay-and-fighters` (a cockpit's rear view). The mockup does not draw a secondary
  feed.

#### 11.3 The windows

Two windows, one in each angled wall either side of the forward wall (layout portals
`p_bridge_window_p` and `_s`, decided): 2.8 x 1.2 m each, centred at (+/-5.0, 5.0, 30.9) in the
middle of its 4.4 m wall and facing 43 deg out from the bow, 5.0 m high (sill 4.4 m, head 5.6 m),
their inner edges 1.1 m round the corner from the viewscreen's (v2 plan, 2026-10-05; were in the
forward bulkhead at z 31.0, facing the bow, 0.6 m outboard of the viewscreen's edges). They show
the real exterior: `ship-frames` draws the exterior pass first, scissored to the windows' screen
rectangle, from the interior camera transformed by the ship's
exterior pose, then the interior on top. The glass is one blended quad per window in the glass
pass. A window costs nothing when it is off screen.

#### 11.4 Lighting

Proposed fixtures; `light-baking` bakes them into the three vertex colour sets (CLAUDE.md 9):

| Fixture | Count | Where | Normal | Red alert | Emergency power |
| --- | ---: | --- | --- | --- | --- |
| Ceiling lamp panel, 1.2 x 0.4 m | 6 | x -4.5, 0.0, 4.5 m at z 22.5 and 28.0 m, on the ceiling | Warm white, full | Red, 75 % | Off |
| Emergency lamp | 2 | Over the aft door (0.0, 6.8, 20.2) and front centre (0.0, 6.8, 32.0) | Off | Off | Amber, 35 %, on the emergency bus |
| Floor strip | 4 runs | Along both raked side walls, 0.15 m in, and both sides of the aisle from the aft door to the dais | Cool blue, 60 % | Red, pulsing at 0.5 Hz | Amber, steady: the way out |
| Console screens | 7 | Each desk | Role colour glow | Same; title bands in the alert colour | Core four only (on the emergency bus); captain, comms and flight ops dark |
| Viewscreen | 1 | | Full | Full | 60 % |

Positions follow the v2 wedge (2026-10-05): the emergency lamps hang 0.2 m under the 7.0 m ceiling
and the front one 0.4 m from the forward wall, as they did under the v1 box's 6.5 m ceiling and
its forward bulkhead at z 31.0 (were at y 6.3 and z 30.6); the side strips keep 0.15 m from the
raked walls (were at x +/-6.85 m). The ceiling panels stand where they were. `deck-pipeline`'s kit
rule now generates 21 lamps on this bridge, 7 of them on the emergency bus (`kit_report.mjs`);
which set the bake uses is `light-baking`'s question G5.

Which state a compartment shows: emergency when its lighting load is unpowered and the emergency
bus is up (`power-grid`), else red alert when the condition is red, else normal.

#### 11.5 Sound

Proposed (mixed on `engine-stack`'s 32 voices): the bridge ambience (air handler hum, 1 voice);
console ticks and confirmations positioned at each seat (at most 4 at once); the klaxon (1); the
order chime for its recipient only (1); hull hits (1). At most 8 voices on the bridge.

#### 11.6 Console faces: screens and keys (proposed 2026-10-05)

The owner, 2026-10-05, on the variants' consoles: "you should come up with better UI place
holders, keyboards etc ... for the panel textures I mean". Today the Blender consoles'
screens are flat role colour and their desks are blank, so from the room a console reads as a
coloured box.

- **A screen shows its own station.** Each console screen in the room shows a baked image of
  its station's console (section 8): the panel grid band (y 272-720 lp of the canvas), with its
  header band in the role colour and each panel's widgets drawn as they are laid out:
  - helm's heading tape and course plot;
  - tactical's polar plot;
  - engineering's bus diagram and bars;
  - science's scan trace;
  - comms' channel list;
  - flight operations' bay boards.

  It is rendered offline from the console definitions (`data/consoles/<role>.json`, task 2.2;
  until they exist, from the mockup's console overlays in `docs/mockups/bridge.html`). So a
  screen in the room previews the console a player gets when they sit, from the same source.
- **No live content.** Consoles are not rendered into textures on the bridge every frame
  (CLAUDE.md 10). A screen is a static image. A few status pills blink by the emission mask
  and one time uniform, which costs no per-pixel lighting.
- **The upper screens** of a wall bank show the station's secondary displays: the ship
  schematic, a system status page or a sensor plot.
- **Keys are texture; levers are geometry.** A desk's working face carries a key panel: two or
  three blocks of square keys, a few lit in the role colour, and a trackpad. The controls a
  player's hand reaches for are low-poly geometry on the prop, within its triangle budget:
  - helm's throttle lever and stick;
  - tactical's guarded fire buttons;
  - engineering's row of breaker toggles;
  - comms' slider bank.
- **One set of images for consoles and walls.** The `screen` and keypad wall panels
  (`wall-panels`) use the same screen and key images.

**Format:** a screen is 256 x 128 px (2:1), two to a 256 px layer of the texture array, with the
emission mask in alpha. Measured on the mockup's set (task 1.7, `assets/textures/screens/screens.json`):

| Images | Size | Layers of 256 x 256 RGBA8 |
| --- | --- | ---: |
| Seven main screens, one per station | 256 x 128 px each | 3.5 |
| Seven upper images, one per station: the upper pair, a 128 x 128 half for each screen | 256 x 128 px each | 3.5 |
| The generic screen, for boards without a console (the wall panels' `ui_screen_crew.png`) | 256 x 160 px | 1 |
| The key panel (the wall panels' `keys_crew.png`: 12 x 3 keys, 0.50 x 0.125 m) | 256 x 64 px | 0.5 |
| **All** | | **9 (8.5 used)** |

That is 2,359,296 bytes (2.25 MB), 3,145,728 bytes (3.0 MB) with mipmaps, of the 96 MB texture
budget. The 14 station images are 147,309 bytes of PNG source (64 colours each, 5.6-14.6 KB).
The key panel is one image reused at its real size on every desk, not two layers. In the engine
the faces are layers of the room's own array, so the deck stays one draw per compartment; the
mockup keeps its screens in a separate atlas, so its faces are one more draw (52 triangles on
today's bridge, 104 on A, 118 on B, 108 on C).

**Built in the mockup (2026-10-05, task 1.7).** `tools/mockups/console_screens.py` crops each
station's console from its shot of `docs/mockups/bridge.html` (the canvas found from
`layoutConsole()` and confirmed on the shot; the budget HUD's corner rebuilt from the panel grid):

- **Main screen:** the title band (y 0-32 lp) over the panel grid band (y 272-720 lp), 1280 x 480 lp
  area-averaged to 256 x 96 px and letterboxed.
- **Upper pair:** two of the station's own panels, fitted whole into the two halves: helm's
  navigation plot and heading, tactical's plot and turrets, engineering's buses and reactor,
  science's sensors and shields, the captain's ship and stations, comms' channels and intercepts,
  flight operations' tasking plot and bays.
- **Boards without a console:** the status boards show the captain's images; the curved helm's
  hooded viewer shows tactical's plot; repeaters and spares show the generic screen.
- **Placement:** each prop's screens row in `props.json` says what it shows (`console`, `upper`
  with its half, or `keys`) and which way is up. The page fits the image whole inside the
  recess, 1 cm proud of its floor, and leaves the rest of the recess as black glass.
- **Key panels:** blocks of keys at their real size, as many as fit across, with a trackpad below
  or beside them. They sit in the free console's touch panel, the curved helm's two panels, the
  captain's arm panels (laid along the arm) and a keyboard well 15 cm deep let into every wall
  bank's desk (1.20 m wide on a core bank, 0.86 m on the others).
- **Hand controls:** geometry, as a station's own variant of the shared prop (`variant_of` and
  `stations` in `props.json`). `tools/bridge_variants.py` and the page put a station's variant
  wherever the base prop would stand (triangles against each prop's budget):

  | Prop | Controls | Triangles |
  | --- | --- | ---: |
  | `free_console_helm` | Throttle lever (operator's left) and stick (right) | 284 / 420 |
  | `free_console_tactical` | Two guarded fire buttons: hazard housing, button, open flip cover | 272 / 420 |
  | `wall_bank_core_engineering` | Five breaker toggles behind the keyboard, the third tripped | 404 / 420 |
  | `wall_bank_comms` | A bank of four faders behind the keyboard | 408 / 420 |
  | `helm_arc` | All of helm's and tactical's (one prop seats both) | 478 / 600 |

  The controls are larger than life (a toggle 6 cm tall, a fader cap 2.8 cm) so they read from
  across the room. The keyboard wells took the plain wall banks from 304 to 320 triangles, and the
  double bank from 368 to 400 / 700.

**Not yet:** the status pills do not blink, since the images are static. The lit keys are the key
image's own green, amber and cream, not the role colour.

**Question B12 (with the prototype's shots):** do screens show the station's own console, or
generic placeholder UI shared by all? Recommendation: the station's own, because then the room
previews the game. The shots: `bridge-variants-{A,B,C}-helm.png` and
`bridge-variants-{A,B,C}-engineering.png` (up close), `bridge-variants-{A,B,C}-ring.png` (across
the room).

#### 11a. Three ways to build the bridge (proposed 2026-10-05, for the owner to choose)

The owner, 2026-10-05, on nine Star Trek bridge references: "notice how elevation is dynamic? the
upper deck is for work, the lower one is walk way to move aorund, and the captains chair is
raised on a platform, with two consoles for helms and tactical a sub platform. notice how majority
of bridge consoles are buit into walls? notice the shape of the bridge, usualy this eiter confirms
to the bridge shape which is circular on star trek ships or the ship shape liket he one found on
the defiant", then "make a few variations, and I'll give you my feed back on whats the best".
The references, and what they have in common, are described in `docs/analysis/star-trek-bridges.md`
(R1-R9). Against them, today's bridge (11.1) already has a ship's shape (R5, R6), but it is one
flat floor with a 0.3 m dais, and its seven stations are desks standing in the room.

Three variants apply the lessons. Each one is data in `data/ships/tern/bridge_variants.json`,
written by `tools/bridge_variants.py`. The tool also checks each variant against the layout's
rules: a copy of the layout patched with it passes `tools/layout_check.py`, and every platform,
stair and console stands inside the room. Its `--sightlines` mode measures the tables below the
same way as 11.1's (it reproduces 11.1's table to the printed digit). `docs/mockups/bridge-variants.html`
draws all three beside today's bridge, from a cutaway, the captain's chair, the aft door and the
stations, with the generated detail, the Blender-built consoles (`assets/models/bridge`) and the
baked light in the three lighting states. **The layout does not change until the owner picks one**
(question B11). The pick becomes a layout patch in its own commit, and 11.1 is rewritten from it.

**What the three share:**

- **Unchanged:** the bridge stays at the bow, with its aft door to the command passage, its seven
  stations and its 3.5 m height (y 3.5 to 7.0 m).
- **Levels are platforms.** A level is a raised floor that is solid, like today's dais, not air,
  so the room stays one brush and its air is the brush's. Steps rise 0.225 m, and the levels are:
  - the **walkway**: the floor;
  - the **sub-platform** for helm and tactical: one step up (+0.225 m);
  - the **work level**: two steps up (+0.45 m);
  - the **captain**: two or three steps up.

  The platforms' risers, nosings, rails and stairs come from `deck-pipeline` 5a's platform rule.
- **Rails mark the drops.** A drop of two steps or more has a railing, 1.0 m high, with a gap at
  each stair (R1, R2, R8). A single step has a hazard nosing only.
- **Side stations are built into the walls.** Each side station is a **wall bank** (R1, R5, R9):
  a desk with its displays set into the wall between two frames. The generated ribs, coves and
  baseboards stay clear of it: a wall fixture keeps them off its width plus 0.25 m either side.
  Engineering and science have core banks 1.4 m wide; comms, flight operations and the extra
  boards have banks 1.1 m wide. The double bank (2.4 m) is an unseated status board, one either
  side of the aft door, in A and C.
- **Only a few consoles stand free:** the helm group, and the captain's chair.
- **Every console is a Blender model** built with boolean cutters (the `blender-hard-surface` skill).

**A. Wedge, tiered** (after R5 and R6, with the levels of R1 and R2). This keeps today's wedge.

- **Work rings:** two of them, two steps up and about 2.0 m deep, run along the raked side walls
  from the aft corners to the windows. Each ring carries two wall banks: comms and engineering to
  port, flight operations and science to starboard. Their operators sit 0.95 m from the wall
  (where the bank's model puts its operator), with their backs to the room, as today. That leaves
  0.55 m between a chair's back and the ring's rail, less than the crew collider's 0.60 m: the
  ring is for work and the walkway for moving around, as the owner put it.
- **Walkway:** runs between the rings, from the aft door to the viewscreen.
- **Captain:** on an octagonal platform 3.0 m across, three steps up, railed round. Its stairs
  go aft to the door aisle and forward to the sub-platform.
- **Helm and tactical:** on a sub-platform one step up (6.8 x 4.1 m), at free consoles facing the
  screen.
- **Stairs:** each ring has one by the door and one by its window.

**B. Round** (after R1, R3 and R8, with R2's dais and helm well).

- **Room:** sixteen-sided, 12.4 m across, centred 6.2 m forward of the aft wall. Its bow segment
  is flattened to 6.3 m to take the viewscreen, which moves 0.8 m aft to z 31.5. The windows
  shrink to 2.0 m wide to fit the segments either side of the bow.
- **Ring:** two steps up and 2.0 m deep, running round both sides with a wall bank in every
  segment, ten in all (R1's unbroken ring of wall stations):
  - engineering and science;
  - comms and flight operations;
  - two status boards by the door;
  - two repeaters by the windows;
  - two spares.

  A rail runs along the ring's inner edge, with gaps at four stairs.
- **Captain:** the aisle from the aft door leads into the well. The captain's dais, an octagon
  2.2 m across, is at ring height, railed behind.
- **Helm and tactical:** share one curved console (R1's single helm console) on a sub-platform
  one step up in front, seated 0.46 m either side of the centreline, where the console's model
  puts its two operators.
- **Cost:** the round room gives up 192.7 m3, 32 % of today's bridge air. The corners between it
  and the hull become service space, which the layout does not use yet: ducts, the computer
  core's cable runs, or a larger ready room.

**C. Split level** (after R9, with R2's sunken helm). The wedge is split across by a two-step
riser 5.6 m forward of the aft wall (z 25.6).

- **Upper level, at the back:** the aft door opens at walkway height onto a flight 2.2 m wide up
  to it. It holds:
  - comms and flight operations in the walls;
  - the status boards on the aft wall;
  - two stand-up repeaters at the riser's ends (R9's pedestal consoles);
  - the captain, at the centre of the riser's edge, with rails either side.
- **Lower front:** holds:
  - helm and tactical on the floor, at free consoles facing the screen;
  - engineering and science, in the forward raked walls.
- **Stairs:** two flights at the riser's sides lead down.
- **Simplest to build:** one platform, three flights and 15.0 m of rail.

**The numbers** (the room's from `K.measure` and the variants file, as the mockup's panel shows
them; the sightlines from `tools/bridge_variants.py --sightlines`; a seated eye is the seat plus
1.20 m, a head's top the seat plus 1.32 m):

| | Today | A. Wedge, tiered | B. Round | C. Split level |
| --- | --- | --- | --- | --- |
| Plan | wedge | wedge | sixteen sides, a flat bow | wedge |
| Air / floor | 608.6 m3 / 173.9 m2 | 608.6 m3 / 173.9 m2 | 415.9 m3 / 118.8 m2 | 608.6 m3 / 173.9 m2 |
| Levels above the walkway | +0.30 m (dais) | +0.225, +0.45, +0.675 m | +0.225, +0.45 m | +0.45 m |
| Raised floor | 7.7 m2 | 88.4 m2 | 64.0 m2 | 87.9 m2 |
| Railing | none | 29.5 m | 22.7 m | 15.0 m |
| Stair treads | none | 7 | 5 | 3 |
| Consoles in the walls / standing free | 0 / 6 | 6 / 2 | 10 / 1 | 6 / 2, and 2 stand-up repeaters |
| Captain's eye over helm's | +0.30 m | +0.45 m | +0.23 m | +0.45 m |
| Captain to the screen; the screen's width seen | 8.10 m; 40.6 deg | 8.30 m; 39.7 deg | 6.50 m; 49.5 deg | 7.40 m; 44.1 deg |
| Screen hidden from the captain by heads | none | none | 0.90 m2, 6.3 % (helm's and tactical's heads: its lower 0.9 m, 0.8-1.3 m either side of the centre) | none |
| Helm and tactical: distance; the screen's width seen; turn to it | 4.51 m; 65.5 deg; 24 deg | 4.67 m; 63.6 deg; 23 deg | 3.64 m; 78.9 deg; 7 deg | 4.69 m; 63.4 deg; 23 deg |
| Engineering and science: distance; turn to the screen | 8.64 m; 122 deg | 8.81 m; 123 deg | 7.46 m; 135 deg | 7.84 m; 128 deg |
| Comms and flight operations: distance; turn to the screen | 12.08 m; 112 deg | 11.80 m; 114 deg | 9.75 m; 157 deg | 11.90 m; 114 deg |
| Room mesh with consoles and crew, one draw | 9,202 | 12,238 | 10,698 | 11,952 |

The room mesh is what the mockup draws: the room, its generated detail and platforms, the
Blender consoles and chairs and the crew figures, cut to 0.9 m for the mockup's stand-in bake.
Against the bridge's ceiling of 30,000 triangles (`deck-pipeline` 11) that is 31-41 %, and A
costs 3,036 more than today. Every variant stays one draw call. (With the console faces of 11.6,
2026-10-05: the keyboard wells and hand controls are in these counts; the faces' own quads are
52-118 more triangles.)

**What each one trades:**

- **A** is the owner's description as it stands:
  - a work level up the walls;
  - a walkway down the middle;
  - the captain raised over everyone;
  - helm and tactical on a sub-platform;
  - consoles in the walls;
  - the ship's own shape (R5, R6, the owner's "like the one found on the defiant").

  It keeps all the air, and nothing hides the screen from the captain, whose eye is 0.45 m over
  the helm's. It costs the most rail (29.5 m) and stairs (seven treads), and the captain climbs
  three steps. The side stations still turn 114-123 deg to see the screen. Their look band
  answers that (B1).
- **B** is the iconic round bridge, with the most consoles in the walls and the screen closest
  to the captain. It costs:
  - a third of the room's air;
  - side operators 135-157 deg from the screen;
  - a captain only 0.23 m over the helm, so the helm group's heads hide 6.3 % of the screen;
  - smaller windows;
  - corners the layout must find a use for.
- **C** is the simplest to build and the clearest front and back (R9). It is two of the owner's
  four levels, though: no captain's platform of its own, and no sub-platform.

**Recommendation: A**, with R7's inlaid floor stripes down the walkway (a decal, added when it is
built). If the owner prefers the round room, B needs one fix first: helm and tactical move from
0.46 m (where the curved console's model seats them) to 1.5 m either side of the centreline, at
a wider curved console. Their heads then fall
past the screen's edges from the captain's eye, as they do today. Raising the dais does not
help: it takes a captain 1.2 m over the walkway to clear them where they sit.

What a pick changes elsewhere, in its own commit:

- **The layout:** the bridge's brush (B), its stations' seats and yaws, the dais fixture
  (replaced by the platforms, a new layout field the checker learns), and for B the windows and
  the viewscreen.
- **11.1:** rewritten from the pick.
- **`crew-on-deck`'s routes to bridge seats:** rerun, with the stairs' climbing time.
- **`deck-pipeline` 11:** the bridge's budget row.
- **The mockups that draw the bridge:** they take the platforms from the layout.
- **This change's open questions:** B1 (look band) still stands for every variant.

### 12. The Pi 5 budget this change spends

Against `engine-stack`'s provisional Pi 5 table (200,000 triangles and 300 draw calls per frame,
all passes; bridge geometry 30,000 triangles; avatar 3,000; viewscreen 1024 x 512 at 30 Hz).
Low poly stays the style: these are ceilings, and the estimates sit well under them.

| Item | Triangles | Draw calls | Memory | Owner |
| --- | ---: | ---: | --- | --- |
| Bridge shell, trims, ribs and detail | 8,000 | | | `deck-pipeline` builds |
| Captain's dais, steps and rail | 600 | | | |
| Seven seats (300 each) | 2,100 | | | |
| Seven desks with screens (700 each) | 4,900 | | | |
| Viewscreen housing, window frames, door frame | 1,400 | | | |
| Lamps (8) and floor strips | 1,000 | | | |
| **Bridge geometry** | **18,000** (60 % of 30,000) | **3** (opaque vertex-lit, emissive, door) | about 1.5 MB (54,000 unshared vertices x 28 B) | |
| Command passage seen through the aft door | 8,000 | 2 | | `deck-pipeline` |
| Viewscreen surface | 2 | 1 | Target 1024 x 512 RGBA8 2 MB, depth 2 MB | |
| Window glass | 4 | 1 | | |
| Crew avatars on the bridge, up to 8 | 24,000 | 8 | | `crew-on-deck` |
| Exterior through the windows (scissored) | 30,000 | 30 | | `ship-frames` |
| Viewscreen pass, every frame at 30 Hz | 30,000 | 30 | | `ship-frames` |
| Secondary feed on a console (Flight ops' craft camera), 15 Hz | 15,000 | 15 | Target 512 x 256 RGBA8 0.5 MB, depth 0.5 MB | `ship-frames` |
| Console UI while seated (egui) | 6,000 | 16 | About 312 KB per frame streamed; font atlas about 4 MB | this change |
| **Worst frame on the bridge** | **131,006 (66 %)** | **106 (35 %)** | | |

- **Fill.** Standing, the interior covers the 1280 x 720 3D frame; the windows add about 8 % of it
  scissored; the viewscreen pass is 524,288 pixels (57 % of a 3D frame) every frame at 30 Hz.
  Seated, the 3D passes draw only the 1280 x 240 look band (33 %), the console panels are opaque,
  and a secondary feed adds 131,072 pixels (14 %) on the frames it renders at 15 Hz.
- **The worst frame is a sum of things that do not all happen at once**: the console UI and a
  secondary feed only while seated, when the look band has already cut the 3D fill by two thirds;
  the windows' exterior pass only when a window is in view. Against `engine-stack`'s guidance to
  plan scenes at about half the ceiling, the standing frame (no UI, no feed: 110,006 triangles and
  75 calls) sits at 55 % and 25 %, and the seated worst case above at 66 % and 35 %.
- **CPU, server.** Automation for every station steps at 10 Hz inside the systems step; proposed
  ceiling 0.2 ms per systems step on one A76 core (inside `engine-stack`'s 2 ms per ship per tick).
- **CPU, client.** Console UI build and tessellation inside the 2 ms UI share (section 8.7).
- **Network.** The seat table grows by 1 byte per station (section 3); orders are reliable commands
  of about 16 bytes; a seated player's console commands are at most 10 a second of about 12 bytes
  (about 1 kbit/s up, inside 16 kbit/s); the helm's stick rides the input channel at 20 Hz.

### 13. Data (proposed)

- `data/stations.json`: roles; per station: role, console layout, merge list, automation tuning
  (`react_s`, `scan_time_scale`, `aim_error_mrad`, `track_rate_scale`, `range_limit_m`), order
  verbs; threat weights; auto-condition ranges (`red_alert_range_m`, `stand_down_range_m`,
  `stand_down_after_s`); NPC post list; body swap settings (`swap_enabled`, `swap_cooldown_s`).
  Stations are matched to the layout's `stations` by id; a missing or extra id stops startup
  (CLAUDE.md 6.5).
- `data/consoles/<console>.json`: panels as `{ id, title, col, row, w, h, widgets }`. A validator
  (in `sc-core`'s tests and `tools/`) checks every panel sits inside the 12 x 4 grid, no two panels
  overlap, every widget's text band fits its panel, the grid is filled, and a console has at most 7
  panels (the draw call ceiling of section 8.7).
- `data/input/bindings.json`: per station and shared, keyboard, mouse and gamepad.
- `data/ui/theme.json`: the colour roles of section 8.3 and the font sizes.

An excerpt of `data/stations.json`:

```json
{
  "stations": {
    "helm": {
      "console": "helm",
      "merge": ["tactical", "captain", "science", "engineering", "comms", "flight_ops"],
      "automation": { "react_s": 1.5, "turn_rate_scale": 0.7, "heading_tol_deg": 2.0, "speed_tol_frac": 0.02 }
    },
    "tactical": {
      "console": "tactical",
      "merge": ["helm", "captain", "science", "engineering", "comms", "flight_ops"],
      "automation": { "react_s": 2.0, "missiles_need_order": true }
    }
  },
  "turret_automation": { "acquire_s": 0.8, "track_rate_scale": 0.5, "aim_error_mrad": 4.0, "range_limit_m": 3000.0, "friendly_clearance_m": 50.0 },
  "threat_weights": { "missile": 3.0, "frigate": 2.0, "fighter": 1.0, "other": 0.5 },
  "auto_condition": { "red_alert_range_m": 15000.0, "stand_down_range_m": 25000.0, "stand_down_after_s": 60.0 },
  "body_swap": { "enabled": true, "cooldown_s": 10.0 }
}
```

### 14. Interfaces to other changes

| Change | This change needs | This change gives |
| --- | --- | --- |
| `power-grid` | `power::solve` for previews; load groups and priorities; which buses feed which consoles and lamps; the computer core load | The engineering consoles' commands: setpoints, priorities, presets, breakers, the reactor's mode and throttle, scram |
| `weapons-and-shields` | `weapons::hit_chance`, `intercept_point`, `missile_intercept`; `shields::allocate`; turret slew rates and arcs; tube load times; sensors and scans if they live there | Tactical's commands (targets, modes, assignments, tubes, presets); Science's shield allocation and frequency; the automated turret's operator parameters |
| `flight-and-navigation` | `flight::plan_turn`, `flight::performance`; the control loop that holds heading and speed | Helm's commands and setpoints; the helm automation's orders |
| `shuttle-bay-and-fighters` | Bay sequences and `bays::sequence_time`; craft states; the cockpit | Flight ops and bay control commands; the lockout |
| `life-support` | `life_support::time_to_pressure`; setpoints for O2, CO2, temperature | Engineering's life support setpoints; Flight ops' pump commands |
| `damage-control` | Compartment and system state; `damage::repair_time`; teams; who may vent | The damage control board's console; console integrity going dark at 0 % |
| `netcode-and-sessions` | The seat table, commands, ordering, refusals, reliable events | The operator field (1 more byte per station), orders, condition |
| `ship-frames` | The exterior pose, the viewscreen camera, the window pass and its scissor, the turret sight's pass; `frames::damper_load` | The viewscreen feed selection; brace |
| `crew-on-deck` | Bodies, the seat snap clip, downed state, braced state, the crew collider | Seat claim, release, relieve and swap rules; NPC posts |
| `deck-pipeline`, `light-baking` | The compiled bridge mesh in budget; three baked colour sets; per-compartment state uniforms | Fixture positions and furniture sizes (section 11) |
| `engine-stack` | The Pi 5 budget; egui and `egui_glow` with the probe's measurement of them (E3); the viewscreen target and the secondary views; the audio voices | This budget's spend (section 12); the console UI's workload for the probe |

### 15. Lessons from star-crew-64

| Prototype (file) | What happened | Here |
| --- | --- | --- |
| `src/main.c:45-67` | Station positions hard-coded in C | Stations come from the layout; consoles are data |
| `src/bridge_panel.c:10,24,115-166` | Engage within 35 u, snap to a seat 18 u in front, one occupant, per-pad edge detection | Kept: reach 1.5 m, 0.4 s seat snap, one operator per station, any device |
| Every console module | File-static singletons | One console type, many instances, built from data |
| `src/main.c:562-597, 794, 940-942` | Swapping away left the seat locked to an NPC body and the helm's steer frozen | Operator is never a body; operator changes at the next tick from the current state; an NPC is always relievable (section 3, 6) |
| `src/main.c:794` | An empty helm decayed to idle and the ship coasted | An empty helm holds course and speed (automation) |
| `src/engineering_console.c:135-166` | Zero-sum power by proportional rebalance | Kept as priorities in `power::solve` |
| `src/engineering_console.c:296-312` | A "repair pulse" button that did nothing but glow | No cosmetic controls: every control changes simulated state |
| `src/science_console.c:168-209` | Science was a rhythm game | Real face balance, scans and the viewscreen |
| `src/weapons_console.c` | No aim reticle | The gunner's lead reticle comes from the resolver |
| `src/main.c:613-622` | A station at 0 HP stopped working | Kept: a console at 0 % integrity goes dark; its role is reachable from its other seat or a tab |
| `src/lobby.c:97-206` | Any connected pad readies and drives menus | Kept: every input device drives every console |
| `src/ship_view.c` (120 x 90 corner view) | Space only in a picture-in-picture | A 6 m viewscreen and two windows showing the real exterior |

## Risks / Trade-offs

- **Tabs could make walking pointless.** With every unmanned station a tab, why stand up? Because
  only a seat gives the full console at once, a tab shows one station at a time, the hands-on
  reactor, bay lockout and turret pods are places, and fires and breaches are fixed on foot
  (`crew-on-deck`, `damage-control`). If playtests show crews never move, the merge lists can
  shrink to the vision's three merges (Captain into any, Comms into Science, Flight ops into
  Tactical) for crews of four or more.
- **Automation that is too good** makes players optional; too weak makes small crews miserable.
  Every competence number is data, and the turret's is a target for measurement.
- **Two seats on one role** (Engineering, Flight ops) can fight over setpoints. Attribution on
  every change and "last command wins" keep it legible; it is also a conversation the game wants.
- **The look band costs a third of the console's height.** Side stations would gain two panel rows
  without it. CLAUDE.md 10 asks for the bridge to stay visible, and the band is also a fill saving.
- **The viewscreen pass at 30 Hz** is 57 % of a 3D frame's pixels. If the Pi 5 probe finds fill
  binding, it drops to 15 Hz before anything else does, and the secondary feed goes before it.
- **egui on an A76** is estimated at a millisecond or two a frame (`engine-stack` E3) and is not
  measured yet. The console rules are written so that swapping it for our own panel layer changes
  only the drawing code.

## Open questions

Per CLAUDE.md 13, a question goes to the owner only with something to look at; the shots are in
`docs/screenshots/mockups/`. The rest take the recommendation, recorded here as "recommendation
taken (ask only with screenshots)".

| Id | Question, and the fact it turns on | Options | Recommendation | Shot |
| --- | --- | --- | --- | --- |
| B1 | The side stations (engineering, science, comms, flight ops) face the walls, so the viewscreen is 118-135 deg behind them. Keep the layout and swivel their look band toward the screen, or turn engineering and science to face the bow (yaw 0) so the screen is 45 deg to one side? | Keep wall-facing with a swivelled look band / turn engineering and science forward | Keep wall-facing: the captain sees every console over its operator's shoulder, the centre stays open, and the look band already shows the screen | `bridge-engineering-console.png`, `bridge-alt-forward-wings.png` |
| B2 | Should a seated console keep a 3D look band above the panels (the bridge stays visible; 33 % fill while seated), or fill the screen with panels (two more rows)? | Look band / full-screen panels | Look band | `bridge-helm-console.png`, `bridge-science-console.png` |
| B3 | Red alert lighting: lamps turn red at 75 % with pulsing red strips, or lamps stay white and only strips and beacons go red (easier to read consoles and faces)? | Red lamps and strips / white lamps, red strips | Red lamps and strips: the room should feel different at a glance | `bridge-red-alert.png` |
| B4 | Emergency power: which consoles stay lit on the emergency bus? | Core four and the viewscreen / every console / none (consoles dark until power returns) | Core four and the viewscreen at 60 % | `bridge-emergency.png` |
| B5 | Shield allocation is written by both Tactical (presets) and Science (fine balance), last command wins. Keep both, or give it to Science alone? | Both / Science only | Both, with attribution on each console | `bridge-tactical-console.png`, `bridge-science-console.png` |
| B6 | Should NPC bodies sit at automated core stations (the room never looks empty, and body swap has someone to swap into), or should automated seats stay empty with an AUTO console? | NPC bodies / empty seats | NPC bodies | `bridge-captain-view.png` |
| B7 | Body swap on by default? | On / off / per session | On, as a session setting (a solo player needs it). Recommendation taken (ask only with screenshots) | none |
| B8 | Viewscreen refresh: 30 Hz (the budget allows it) or 15 Hz (half the pass)? | 30 Hz / 15 Hz | 30 Hz, dropping to 15 Hz if the probe finds fill binding. Recommendation taken (ask only with screenshots) | none |
| B9 | Merge every unmanned station to a tab (one player runs everything), or only the vision's three merges? | All, by merge list / only Captain, Comms and Flight ops | All, by merge list. Recommendation taken (ask only with screenshots) | none |
| B10 | Gunner turrets: no remote gunnery from Tactical? The Pi 5 budget's secondary view (512 x 256 at 15 Hz) could carry a sight, so this is a design choice, not a cost | None / remote sight from Tactical | None: the pods are the reason to leave the bridge. Recommendation taken (ask only with screenshots) | none |
| B11 | Which bridge: the wedge with three levels, the round room with a ring, or the wedge split front and back (section 11a)? All three put the side consoles in the walls, raise the captain and keep the 3.5 m room; they differ in shape, levels and what the captain sees | A. Wedge, tiered / B. Round / C. Split level / today's flat bridge | A: the owner's levels exactly, the ship's shape, all the air, nothing hides the screen | `bridge-variants-{today,A,B,C}-cutaway.png`, `bridge-variants-{A,B,C}-captain.png`, `bridge-variants-{A,B,C}-door.png` |
| B12 | Console screens in the room (section 11.6): each shows its own station's console, or all show one generic placeholder UI? | Own console / generic | Own console: the room previews the game a player gets when they sit | `bridge-variants-{A,B,C}-helm.png`, `bridge-variants-{A,B,C}-engineering.png`, `bridge-variants-{A,B,C}-ring.png` |
