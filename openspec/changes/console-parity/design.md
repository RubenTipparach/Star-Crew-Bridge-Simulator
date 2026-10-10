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

## 3. One look, from shared data

The look is data both sides read, so a colour, a radius or an icon is changed once (CLAUDE.md 6.1):

- **`data/ui/palette.json`**: the console palette, the mockup's `C` (bg, panel, panel2, line, line2, text, dim,
  faint, ok, warn, danger, alert, emergency, hostile, friendly, neutral, unknown) and the role colours
  (`K.PALETTE.role`), sRGB hex.
- **`data/ui/console_style.json`**:
  - the canvas (1280 x 720 lp) and its bands (title 32, look 240, grid from y 280, status 32);
  - the grid (12 columns of 96 lp, 4 rows of 94 lp, 8 lp gutters);
  - the panel (radius 9, border 1, title tick 3 x 10 at 12, 9, title 13 px weight 700 letter-spacing 2);
  - the button (radius 6, label sizes, the on, off and unavailable fills);
  - the hold time (0.6 s).
- **`data/ui/icons.json`**: the mockup's `ICONS` (24 x 24 SVG fragments, stroked), moved out of `consoles.html`.
  - `tools/ui/icons.py` rasterizes them to an atlas (`assets/ui/icons.png`, 64 px a cell, white on transparent, with
    mips) and its manifest. The engine tints a cell like egui text.
  - The mockup keeps drawing them as SVG from the same file.
- **The typeface:** Barlow Semi Condensed from `assets/fonts/barlow-semi-condensed` (the mockup's inlined `font:` block),
  loaded into egui as the proportional family.

`tools/mockups/inline.py` gains `data:ui/*` blocks; `consoles.html` reads the three files instead of its own
constants, and its shots must come out the same (pixel compared before and after, task 1.3).

## 4. The check

A layout manifest is the list of what a console shows in one state:

```json
{ "schema": "starcrew.console-layout/1", "station": "helm", "state": "engage",
  "items": [ { "id": "thrust", "kind": "panel", "rect": [8, 280, 304, 400], "title": "THRUST" },
             { "id": "thrust.stop", "kind": "button", "rect": [22, 618, 276, 50], "label": "STOP", "icon": "stop" } ] }
```

- **The mockup's side.** `consoles.html` records every `panel()` and `button()` it draws (they are the mockup's only
  drawers of panels and controls) and other controls by their `data-act`, `data-drag` or `data-hold` group and its
  bounding box. `window.MOCKUP_LAYOUT(station, state)` returns the manifest; `tools/mockups/shoot.mjs --layout` writes
  it to `docs/screenshots/consoles/layout/mockup-<station>-<state>.json`.
- **The engine's side.** The console module records the same items as it draws them, by the same ids (the ids are
  the mockup's `data-act` names). `sc-client --console-layout <station> --state <state>` writes
  `engine-<station>-<state>.json`. The states are named scenarios: a fixed snapshot of the ship and its contacts, the
  same numbers on both sides, from `data/consoles/states.json`.
- **The comparison.** `tools/consoles/parity.py` matches items by id and fails when:
  - a mockup item is missing in the engine;
  - an item moved more than 4 lp, or its size differs by more than 4 lp;
  - its label or icon differs;
  - the engine has an item the mockup does not.

  It prints a table per station. It runs in `scripts/check.sh`, so a console that drifts from its mockup fails CI.
- **The picture.** The same states captured on both sides go on the comparison page, mockup and engine side by side,
  for the owner to sign off. A console is done only when the check passes and the owner has signed off its pictures.

## 5. What the drill added, mapped to the approved controls

These need no new mockup (recommendation taken, ask only with screenshots):

| Engine today | Becomes | Why |
| --- | --- | --- |
| Target camera inset | The viewscreen strip showing the feed pad's TARGET view | The mockup's camera feed already is a target camera |
| LOCK and RELOCK | Tap a target card to target it, as in the mockup; the selected card is the lock | Same command (`Command::Lock`) |
| WEAPONS FREE bar | Each turret's mode: AUTO (free), HOLD; PD shown unavailable until point defence exists | The mockup's per-mount mode |
| Shield presets as words | The five icon buttons | The mockup's icons |
| Tube LOAD and EMPTY words, "9 left" | Tube pills and magazine pips (one pip a missile in the magazine) | The mockup's pictures |
| Drill header (mission name, BOT) | The mission name on the briefing and debrief only; the clock stays in the title band; a bot shows as the operator's name "Bot" in the status strip | The title band has no mission name |
| Round trip and loss on the console | The F3 overlay | Not console content (section 2) |
| TARGET, LEVEL, STOP in Orient | LEVEL, FLIP, TARGET in Orient; STOP at the foot of Thrust | The mockup's places |

## 6. What the missing controls need in the core

Drawn first in their unavailable state (section 2), then made to work in this order, each with its tests in
`sc-core::combat`:

| Control | Core rule (from `flight-and-navigation`) | Order |
| --- | --- | --- |
| Strafe pad | Lateral and vertical set points, RCS acceleration 4 m/s^2, limit 50 m/s (FN 3) | 1 |
| Orient thumbwheels, GO, FLIP | An attitude order slewed at the rate limits; GO shows its time; FLIP the reciprocal heading (FN 6a) | 2 |
| Autopilot Hold, Course, Chase, Match, Evade | FN 5; Chase is today's Target mode, Hold today's set point | 3 |
| Turret modes AUTO, HOLD, PD | AUTO and HOLD are today's weapons free per mount; PD waits for point defence | 4 |
| Scanner range zoom and tilt | Display only (the plot's range and camera) | 1 |
| Contact list | The snapshot carries every contact; the drill has one | 1 |

## 7. How it is built and tested

- **Order of work.**
  1. The shared data and the mockup reading it.
  2. The mockup's layout hook.
  3. The parity tool.
  4. The engine console module with Helm, then Tactical.
  5. The core rules in section 6's order.
- **Where it is tested** (CLAUDE.md 12). The cloud session writes the data, the mockup hook, the parity tool and the
  code. The Mac, the most powerful connected device, builds, runs the parity check and takes the engine's captures;
  its results come back through git. The Pi 5 measures the console's cost.
- **Pi 5 budget.** The mockup states 6 UI draw calls a console (`bridge-stations` 8.7). The egui console must stay
  inside the console's 16 draw calls:
  - one icon atlas texture, 512 x 512 RGBA with mips, about 1.4 MB;
  - the vector navball, joystick and shield bubble as tessellated meshes, a few thousand triangles.

  Measured on the Pi before it is called done.

## Risks / Trade-offs

- **egui is not SVG.** Gradients, dashes and the mockup's 3D vector drawings (the navball, the joystick, the shield
  bubble) need their own drawing code in Rust. The check compares positions, sizes, labels and icons, not pixels; the
  pixels are the owner's sign-off on the pictures.
- **The drill's play changes.** A Tactical without Weapons Free and Relock plays as the mockup intends (tap a card,
  set each mount's mode). The drill's bot uses the same commands, so its numbers are re-measured after the change.
