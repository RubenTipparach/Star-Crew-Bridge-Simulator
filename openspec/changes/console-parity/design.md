# Design: the engine's consoles look like their approved mockups

## Context

The approved consoles are `docs/mockups/consoles.html` (the `bridge-stations` design 8, approved by the owner
2026-10-07 and 2026-10-08), shot in `docs/screenshots/mockups/consoles-*.png`. The engine's Helm and Tactical are the
co-op drill's (`coop-drill` design 5, PR #13), drawn with egui in `crates/sc-client/src/drill.rs` and captured on a Pi 5
in `docs/screenshots/engine/coop-drill/`. The comparison page (https://claude.ai/artifact/7fVi9j3e4Vkt8AFqrQUzLp)
shows them side by side.

## 1. What went wrong

**Two hand-written copies, compared by nobody.**
- **Words for a design.** `coop-drill` design 5 described each console in words ("THRUST (the set point lever and the
  speed), SCANNER ..."), and `drill.rs` was written from those words. Neither looked at the mockup's pictures or its code.
- **No check.** Nothing compared the engine to the mockup. The drill's captures were judged as a drill working, not
  as the approved screen. CLAUDE.md 6.6 asks for exactly this check: when two things must agree, compare the actual
  artifacts.

**Where the two differ.** Measured on the captures named. The mockup is `consoles-helm.png` and
`consoles-tactical.png`; the engine is `4-engage-b-helm.jpg` and `4-engage-b-tactical.jpg`.

| Part | Approved mockup | Engine today |
| --- | --- | --- |
| Canvas | 1280 x 720 lp in four bands: title band (32), look band (240), the 4 x 12 panel grid, status strip (32) | Panels over a full-screen 3D bow view; no look band |
| Title band | The station's chip with its icon, open stations as dashed chips to swap to, the captain's newest order (amber, with Acknowledge), the lighting-state badge, the clock | Station word, drill name, clock, BOT, the other station's chip |
| Look band | The viewscreen strip (620 x 200 lp) with its feed name; the feed widget (the ship's attitude thumbnail with the camera's cone, a six-button camera pad); the navball with roll readout | A 448 x 252 px target camera inset; a target bracket on the 3D view |
| Status strip | Hull bar and %, six shield face bars, power MW, speed m/s, fires, breaches, the operator and station | Hull and shield bars, round trip, loss, both crew names |
| Panel style | Radius 9, a 3 x 10 lp coloured tick before the title, titles 13 px letter-spaced capitals, Barlow Semi Condensed throughout, icons on every control | egui frames, mixed-case titles, another typeface, no icons |

Helm, panel by panel:

| Panel | Approved mockup | Engine today |
| --- | --- | --- |
| THRUST (3 x 4) | Speed 38 px with "m/s"; the lever -100 to +400 m/s (handle = set point, fill = speed, hatched reverse); the strafe pad (puck = set point, dot = drift); STOP as the panel's full-width bottom button | Speed in amber with "now 53"; a lever with a tick scale; no strafe; STOP in Orient |
| SCANNER (5 x 4) | Tilted 3D plot with range zoom (- 20 km +) and reset; contacts with elevation stalks; the tubes' cone; the waypoint; the camera's cone; autopilot row Hold, Course, Chase, Match, Evade | A flat plot, fixed range, "1.4 km above" as text; no autopilot |
| ATTITUDE (4 x 2) | The joystick and the twist lever drawn in 3D; yaw, pitch and roll with rate bars and the rate set point as a ghost | A square stick pad with a dot |
| ORIENT (4 x 2) | Three thumbwheels (yaw, pitch, roll order) with values; the order's quaternion; GO with the slew time; LEVEL, FLIP, TARGET | HDG PIT ROL now and the quaternion as text; TARGET, LEVEL, STOP |

Tactical, panel by panel:

| Panel | Approved mockup | Engine today |
| --- | --- | --- |
| TARGETS (3 x 4) | Up to four contact cards, hostiles first, by range: the ship glyph, id, range, hull bar (unknown until scanned); tap one to target it | One card (name, range), a LOCK ring, hull and shield bars, RELOCK |
| PLOT (5 x 4) | The tilted plot with the ship in its shield bubble in 3D (faces lit by their strength), contacts, missiles, the firing line, range zoom; presets as five icon buttons | A 2D ring of six arcs and two bars; presets as five words |
| TURRETS (4 x 2) | Four mounts, each a dial with its heat arc and aim needle, its letter (D V P S), its gunner or the automation icon, and its mode (AUTO, HOLD, PD), tap to change | Four needle rings; WEAPONS FREE as one green bar |
| TUBES (4 x 2) | Two tube pills (empty, loading fill, ready icon), the magazine as pips, FIRE as the biggest control with its chance-to-hit ring, held 0.6 s | LOAD and EMPTY as words, "9 left", LOAD and FIRE buttons |

## 2. The rule

**The approved mockup is the screen's specification** (CLAUDE.md 10, "Mockup first", made checkable).
- **What it shows.** An engine screen shows every panel and control its approved mockup shows, in the same place
  (within 4 lp on the 1280 x 720 canvas), with the same label or icon, the same colours for the same states and the
  same typeface.
- **Nothing the mockup lacks.** The engine adds no control the mockup does not have. A new control is first drawn in
  the mockup, shown to the owner with its screenshots, approved, and then built.
- **Not built yet.** A control whose rule the engine does not have yet is drawn in the mockup's unavailable state
  (faint, `C.faint`, with no action), and its tooltip says it is not in this build. The picture stays the approved one,
  and the gap is visible rather than missing.
- **Debug numbers** (round trip, loss, frame time) are not part of a console; they go on the F3 overlay.

## 3. One look, read out of the mockup (built)

The look is data the engine compiles in, and that data is read out of the mockup's own text, so there is one source
(CLAUDE.md 6.1) and nothing to keep in step by hand. This replaces the first plan of three hand-made files that both
sides would read: the mockup already holds the look, so a tool copies it out and a check holds the copy to it.

- **`data/ui/console_style.json`** (`starcrew.console-style/1`), written by `tools/ui/console_style.py` from
  `consoles.html` (`const C`, `const ICONS`) and `shipkit.js` (`PALETTE.role`): the 17 colour roles and 8 role colours
  as sRGB hex, and every icon as its 24-unit SVG fragment. `--check` fails when the file is not what the mockup says.
  The client compiles it in (`crates/sc-client/src/vg.rs`).
- **Icons are drawn as vectors, not an atlas.** `vg.rs` parses each icon's SVG (paths with every command the icons
  use, arcs included; circles, rects, lines, polylines) and strokes it 2 units wide with round caps and joins, as the
  browser does. No texture, so no atlas tool and no 1.4 MB of texture: an icon costs its triangles.
- **The typeface** is the mockup's Barlow Semi Condensed 600 and 700. `tools/ui/fonts.py` turns the two inlined woff2
  files into TrueType for egui with the mockup's tabular figures and its kerning baked in (egui's ab_glyph reads only
  a legacy `kern` table), `--check` holds them, and the client compiles them in. Text sizes are the mockup's pixel
  sizes; letter spacing is egui's `extra_letter_spacing`; text coverage is linear, as the browser's.
- **The geometry** (bands, the 104 x 102 lp grid, the panel's radius 9 and title tick, button radius 6) is the
  mockup's numbers in `console/mod.rs` and `console/kit.rs`, each beside the mockup function it ports. They are held by
  the picture check (section 4), not by a second data file.

## 4. The check: the real pictures, compared (built)

The first plan compared layout manifests (rectangles and labels written by both sides). What was built compares the
pictures themselves, which also catches a wrong colour, a wrong icon or a control drawn wrong in the right place.

- **The mockup's side.** `consoles.html` has `window.consoleState()`: the state its console is drawing (ship, contacts,
  orders, turrets, tubes, the viewscreen camera, each number the panels show). `tools/consoles/parity.mjs` loads the
  page headless, steps through the named shots in `window.MOCKUP_SHOTS` and writes, for each, `<shot>-mockup.png`
  (1280 x 720) and `<shot>.json`, into `docs/screenshots/parity/`. Seven shots: `helm`, `helm-orient`, `helm-stick`,
  `red-helm-evading`, `tactical`, `red-tactical`, `red-tactical-turned`.
- **The engine's side.** `sc-client --headless --console-fixture <shot>.json --shots DIR` draws that state with the
  drill's own console code (`console::paint`, the same call the drill makes) and writes `<shot>-engine.png`. It prints
  the console's UI draw calls, vertices and indices.
- **The comparison.** `tools/consoles/compare.py DIR` writes `<shot>-compare.png` (mockup, engine, difference) and
  two numbers per region (title band, look band, each panel, status strip), leaving the viewscreen's inside out (the
  engine shows its 3D feed there):
  - *pixels*: the share differing by more than 48 in a channel; font smoothing alone leaves 1-8 %, so it says where to
    look;
  - *missing*: the share of the region's ink (pixels not its background, in either picture) where the two pictures,
    each blurred by 2.5 px, differ by more than 32. It forgives smoothing and sub-pixel placement and nothing else.
    Measured: 0-2.6 % on the seven shots; a panel's contents moved 2 px 7 %, 4 px 20 %, 6 px 29 %; a panel drawn
    empty 86-94 %.
- **In `scripts/check.sh`** (steps 11-13): the fonts and the style data are the mockup's, then every saved state is
  drawn by the engine into a temporary folder and compared with `--max-pct 2.5 --max-missing 10`. A control missing,
  extra, recoloured or moved by more than about 3 px fails it.
- **The pictures for the owner.** The compare images and the live drill's captures go on the comparison page, mockup
  and engine side by side. A console is done only when the check passes and the owner has signed off its pictures.

## 5. What the drill added, mapped to the approved controls

These needed no new mockup (recommendation taken, ask only with screenshots):

| Engine before | Built as | Why |
| --- | --- | --- |
| Target camera inset | The look band's viewscreen strip, showing the feed pad's camera (TARGET by default, as the mockup), rendered by the 3D renderer into the strip's rectangle under the UI | The mockup's camera feed already is a target camera |
| LOCK and RELOCK | Tap a target card to target it; the selected card is the lock | Same command (`Command::Lock`) |
| WEAPONS FREE bar | Each turret's mode, tapped round AUTO, TARGET, PD, HOLD | The mockup's per-mount mode |
| Shield presets as words | The five icon buttons | The mockup's icons |
| Tube LOAD and EMPTY words, "9 left" | Tube pills and magazine pips | The mockup's pictures |
| Drill header (mission name, BOT) | The title band as the mockup's; a bot shows as the operator "Bot" in the status strip | The title band has no mission name |
| Round trip and loss on the console | A corner readout on F3 | Not console content (section 2) |
| TARGET, LEVEL, STOP in Orient | LEVEL, FLIP, TARGET in Orient; STOP at the foot of Thrust | The mockup's places |

## 6. What the missing controls needed in the core (built)

Each rule is in `sc-core::combat` with its test, and the server resolves it; the console's preview of it is the same
function (CLAUDE.md 6.1).

| Control | Core rule as built | Test |
| --- | --- | --- |
| Strafe pad | `Command::Strafe`: lateral and vertical set points held to `strafe_max_mps` (50 m/s), reached at the drill's `tau_v_s` (see 8) | `the_strafe_pad_drifts_the_ship_sideways_to_its_set_point` |
| Orient thumbwheels, GO, FLIP, LEVEL | `combat/attitude.rs`: heading, pitch and roll against the quaternion, the shortest single-axis slew at the rate and acceleration limits, held inside `held_deg` turning slower than `held_dps` (FN 6a). GO's seconds are `slew().time_s` | `an_attitude_order_turns_the_ship_there_in_about_its_planned_time_and_holds_it`, and the round trip, slew time and whole-degree tests in `attitude.rs` |
| Autopilot HOLD, COURSE, CHASE, MATCH, EVADE | `HelmMode` (FN 5): HOLD flies the stick; COURSE puts the bow on the mission's waypoint (the relay, `waypoint_m`); CHASE on the target; MATCH takes its course and speed; EVADE throws set points up to `jink_mps` every `jink_period_s`, seeded from the round | `chase_puts_the_bow_on_the_target_and_course_on_the_waypoint`, `match_takes_the_targets_course_and_speed`, `evade_throws_jinks_from_the_round_seed_and_leaving_it_stops_them` |
| Turret modes AUTO, TARGET, PD, HOLD | `TurretMode` per mount (WS 7). PD holds fire: the Hound carries no missiles | `a_turret_on_hold_never_fires_and_auto_fires_at_the_locked_target` |
| Turret heat arcs | Each bolt heats its turret's sink; at `heat_sink_mj` it locks out until below `heat_resume_frac` (WS 2; `data/weapons.json`) | `heat_locks_a_turret_out_and_it_fires_again_when_cool` |
| Four turret dials D V P S | The drill's turrets are the layout's four mounts (dorsal, ventral, port, starboard) at their `centre_m`, each bearing on the half of the sky it faces | `the_drills_turrets_are_the_layouts_four_mounts` |
| The 3D shield plot | A hit's face is chosen on the shield ellipsoid (WS 11; `data/ships/tern/shields.json`) | `a_hit_on_the_port_side_forward_of_midships_is_port_not_bow` |
| Scanner range zoom, tilt, contact list, camera pad | Display only: the client's seat state | `seat.rs` tests |

The network carries them: protocol 2 (`sc-net`) adds the strafe, attitude-order, helm-mode, turret-mode and camera
fields to the commands and the snapshot.

**Balance after the change.** Isolating each rule showed the RCS clamp of FN 4 (sideways velocity limited by thruster
acceleration) cut the bots' wins from 8 of 8 seeds to 2; heat never triggered. The drill keeps its own drift model
(section 8) and the bots win 8 of 8 again (a survey over 8 seeds while building it; `lan_drill` holds a victory).

## 7. How it was built and tested

- **Where it was tested** (CLAUDE.md 12). No session on the owner's hardware was connected while this was built, so
  the cloud container built it, ran the tests and the parity check, and rendered the captures on Mesa llvmpipe. The Mac
  (the most powerful device) repeats the parity check when it is next connected; the Pi 5 measures the cost.
- **The live drill.** A server and two bot clients ran the drill headless (won in 66 s); the clients' captures of both
  consoles through every phase are in `docs/screenshots/engine/coop-drill-parity/`.
- **Pi 5 budget (CLAUDE.md 2), not measured on a Pi.** A console draws in 8-10 UI draw calls (the budget's UI row is
  10), with no texture but the font atlas. It is heavier in geometry than the first estimate: 26,000-28,000 vertices and
  98,000-118,000 indices a frame (about 1.0 MB streamed a frame at egui's 20 bytes a vertex and 4 an index), most of it
  the anti-aliased strokes of the navball, the joystick and the shield bubble. The UI's CPU share is 2 ms of the
  client's 8; whether the tessellation fits it is the Pi's measurement to make, and if it does not, the first lever is
  caching the static strokes (the grid, the panel frames, the navball's sphere) as one mesh between frames.

## 8. Decisions taken

Each is a recommendation taken under the owner's rule (ask only with screenshots); each is visible in the captures
for the sign-off.

| Decision | Why |
| --- | --- |
| The FIRE ring shows the lock's progress, and its note reads NO LOCK, LOCK n %, OUT OF CONE or LOCKED; it is green only when locked with the target in the tubes' 5 deg cone | The mockup's ring is a hit chance, but the Gannet has no hit-chance rule in WS yet; the lock is the number the crew acts on |
| The turrets are the layout's four mounts, not the drill's two pairs | The mockup draws D V P S; the layout is the one source |
| PD holds fire | The Hound carries no missiles |
| Turrets start on HOLD; Tactical's automation sets AUTO in range and HOLD out of it | The mockup has no WEAPONS FREE; HOLD is the safe default |
| Sideways velocity relaxes to its set point at the drill's `tau_v_s`; FN 4's RCS clamp is not built | It cut the bots' wins from 8 of 8 to 2 (section 6) |
| COURSE's waypoint is the relay at the Shoals (`drill-hound.json` `waypoint_m`) | The drill has one place to go |
| The Hound's hull bar is shown as scanned | The drill has no Science seat to scan it |
| The alert state stays NORMAL in the drill | Nobody holds the captain's seat to set it |
| The clock shows the engage time | The drill has no ship's clock |
| The viewscreen feed is per seat | Each player picks their own camera, as the mockup's pad does |
| Round trip and loss are an F3 corner readout | Section 2 |
| The magazine shows 12 pips | The mockup's magazine |

## 9. Status

- **Built:** sections 3-6 for Helm and Tactical, and section 10 for Engineering, Science and the Captain: all five
  consoles drawn as the mockup's eighteen shots, the check in `scripts/check.sh`; Science's and the Captain's rules in
  the drill.
- **Open:** the owner's sign-off of the side-by-side pictures (task 3.4); the Pi's measurement of the UI's cost; the
  Mac's run of the check.
- **Not built:** Engineering's simulation (the power grid, the reactor and its cooling loop, the atmosphere): its
  console is drawn, and stays unavailable in the drill, until `power-grid`, `reactor-cooling` and `life-support` are
  built in the engine.

## 10. Every station (owner, 2026-10-10: "continue and do all stations")

### 10.1 The look

Engineering, Science and the Captain are ported from `engineering()`, `reactorLoop()`, `science()`, `captain()` and
`captainShip()` exactly as Helm and Tactical were (section 3), into `console/engineering.rs`, `science.rs` and
`captain.rs`. The mockup's `window.consoleState()` gains what they draw: `sci` (the picked contact, the scan, the
ping, the shield view's camera, the frequency band, who set the shields), `captain` (the tab, the picked room, brace,
the viewscreen, the order being written, the last orders, the crew rows, every compartment's state word, where the
crew are and the picked room's air) and `eng` (the power solve's supply and demand, each load group's fader, the
reactor, the cooling loop, the buses and the air), plus `t` (the clock the sweep, the pumps and the pipes move at) and
`red_alert` (the look band's glow follows the captain's alert even on emergency power, as the mockup's does). The
check (section 4) takes all eighteen of the mockup's shots; the captain's SHIP tab has its own regions.

The captain's deck plans read the ship's one layout (`data/ships/tern/layout.json`, CLAUDE.md 8): hull sections,
decks and each compartment's brushes, compiled in. The captain's order verbs are data in `data/stations.json`
(bridge-stations 5, "The verbs per station are data"); `tools/ui/console_style.py --check` fails when the mockup's
`ORDER_VERBS` and that file disagree.

### 10.2 The seats in the drill

The drill's mission lists all five stations; an empty one runs its automation (bridge-stations 3). Each rule below is
in `sc-core::combat` with its test, its numbers in data, and the console's controls send the same commands a bot or
the automation does.

| Station | What works | Rule and source | Data |
| --- | --- | --- | --- |
| Science | SCAN the Hound: the scan rises from 0 to 1 in `scan_s`, 1.5 times as long under automation, which passive-scans when the seat is empty. Tactical's target card shows the hull from 30 % (the mockup's rule, replacing "shown as scanned"); a full scan names the Hound's weapon band in the contact's tip. | bridge-stations 3; weapons-and-shields 11 | `combat.json` `sensors.scan_s` 6 (the mockup's), `stations.json` `science.scan_factor` 1.5 |
| Science | PING, held 0.6 s: a ring on the scanner for `ping_s`. "It reveals the ship": the Hound already knows where the Tern is, so nothing else follows in this drill. | bridge-stations 3 | `sensors.ping_s` 2 |
| Science | The shield presets, as Tactical's (the same command); the console shows who set them last. Tactical's automation sets the shields only while Science is empty. | weapons-and-shields 11, "Automation" | |
| Science | The frequency band A-D: a face loses `match_factor` of a hit whose weapon band matches, and `retune_factor` while retuning for `retune_s`. | weapons-and-shields 11 | `shields.json` `frequency`; `weapons.json` each gun's `band` |
| Science | The viewscreen's camera and zoom, per seat (as every console's). | | |
| Captain | RED ALERT, held 0.6 s, and STAND DOWN: every console's title band and look band. Empty, the automation calls red alert when a hostile comes within `red_within_m`, and normal `normal_after_s` after the last is beyond `normal_beyond_m`. | bridge-stations 3, "Captain" | `stations.json` `captain` |
| Captain | BRACE, a toggle. It changes nothing in the drill: what it protects (standing crew, `crew-on-deck`) is not built. | bridge-stations 5 and 8 | |
| Captain | SCREEN takes the viewscreen: every console's viewscreen shows the captain's camera until it is given back. | bridge-stations 3 | |
| Captain | ORDERS: a station and a verb, SEND; the order shows in that station's title band, Acknowledge (Y) or a press on it marks it done, and the captain sees the tick. An empty station's automation acknowledges after its reaction time; it does not act on the order (bridge-stations 3 lists what each automation obeys; that is not built). | bridge-stations 5 | `stations.json` `orders` |
| Captain | SHIP: the three decks with every compartment, the crew as dots (each seated player on the bridge). | | |
| Engineering | Nothing yet: no power grid, reactor, cooling loop or atmosphere is simulated in the engine. The console is drawn in the mockup's unavailable state (section 2), its controls faint and inert, and the status strip's power reads `-- MW`. | | |

Not built, and drawn as unavailable: the captain's room readouts (`--`) and SEND REPAIRS (no atmosphere or damage
control), and all of Engineering. The mockup's weak-point ring on a fully scanned contact is drawn, but no design
gives weak points a rule yet. Each is recommendation taken (ask only with screenshots).

**Pi 5 budget.** No geometry or texture; the consoles stay at 8-10 UI draw calls, from 17,000 to 33,000 vertices a
frame (Engineering the most, its faders and pipes). The snapshot grows by the bridge's state: about 30 bytes.

### 10.3 What the live drill showed

Five bot clients, one a station, against `sc-server` in the cloud session (`docs/screenshots/engine/coop-drill-all-stations`):
victory in 69-71 s over four runs. It found three things the fixtures could not:

- **A long name dropped the peer.** "Engineering officer (bot)" is 25 characters, one over `MAX_NAME_CHARS`, so the
  hello failed to decode and the server never seated it. The session now cuts a name to the limit
  (`combat::clean_name`) before it is sent, with a test; and a bot's default name no longer repeats "(bot)", which the
  server already adds in the crew list.
- **A fight's hits smeared Science's shield view.** The drill lands about 1.6 hits a second, so the 8 s the view keeps
  holds a dozen from nearly one bearing: a dozen arrows and labels stacked into one smear, and, from the bow, past
  the panel's edge. The mockup has the same flaw (its own fight fires far less often) and is fixed with it, a bug fix
  inside the approved design (CLAUDE.md 10):
  - one arrow a bearing: a hit within `HIT_MERGE_DEG` (10 degrees) of a newer one is not drawn again, so the arrow
    takes the newest hit's colour (red when it went through) and label;
  - a hit's label is held inside the view, as the faces' labels already were, and steps up 16 lp clear of the labels
    before it (the mockup's `place`, which was written but never called);
  - the engine cuts each line to the view by hand where the mockup's nested `<svg>` clips it, since a clip rectangle
    is a UI draw call.
  Tactical's small view takes the merge too. `kit.rs` tests both rules; the eighteen shots still pass the check.

**Where it was tested.** In the cloud session, on lavapipe: what the consoles look like and that five seats play a
drill through, never how fast (CLAUDE.md 12). The same drill on a Pi 5 is task 5.5.

## Risks / Trade-offs

- **egui is not SVG.** The mockup's gradients, dashes and 3D vector drawings are ported by hand into `vg.rs` and the
  console modules. The picture check is what holds the port to the mockup; anti-aliasing still differs at the pixel
  level (1-8 % of pixels by region), which the blurred score forgives and the owner judges on the pictures.
- **A mockup change needs new pictures.** When the mockup changes, `parity.mjs` re-captures its shots and the engine
  has to follow before the check passes. That is the point of the check.
