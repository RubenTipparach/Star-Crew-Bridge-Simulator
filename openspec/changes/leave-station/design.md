# Design: leave a station with one key and one button

## Context

| Decided elsewhere | Where |
| --- | --- |
| Stand up: hold E 0.5 s, a tap does nothing while seated | `bridge-stations` 6 and 8.6 |
| Helm's roll on Z and C | `bridge-stations` 10.1 |
| The title band: the station's chip, OBJECTIVES, the captain's order, the condition, the clock | `bridge-stations` 8.1, the approved `consoles.html` |
| The touch layer, its STAND button and ring | `touch-controls` 3 and 4 |
| The law: the camera rises with the body when it stands; the ship keeps running | CLAUDE.md, THE LAW |
| Keys and buttons are data and can be rebound | `bridge-stations` 8.6, `crew-on-deck` design (`data/input/bindings.json`) |

## 1. What stopped a player getting up (measured 2026-10-15 on Pi 1)

Single player, `sc-client --solo --station helm`, seated at Helm in Engage:

| Tried | What happened | Why |
| --- | --- | --- |
| Hold E | The Tern rolled to starboard; the body stayed seated | `drill.rs` `stand_up` took E at every station but Helm, where `seat.rs` reads E as roll |
| Hold Backspace | Stood up after 0.5 s | The only stand key at Helm, shown nowhere |
| Tap E | Nothing | Stand is a hold; nothing said so |
| Look for a prompt | None (screenshot `docs/screenshots/engine/leave-station/0-before-helm-seated.png`) | Only the touch layer drew STAND |

Checked and not a cause: `Drill::stand` has no phase rule; seated, the drill gives the mouse back (`release_mouse`), so
nothing holds it captured; the game menu takes the keys only while open; on foot, W A S D and a click to look work in
every phase, Engage included (`step_foot` is phase blind), and the body walks every deck (the patrol and drill
playtests walk the decks mid-fight).

## 2. One key sits and stands

`bindings.use` (E) is the key that sits at a seat in reach, and, held `seats.stand_hold_s` (0.5 s) while seated, stands
the body up, at every station and in every phase. The hold is armed only by a press made while seated: a key still
down from sitting does not count until it is let go and pressed again. While the game menu is open the hold resets.

The hold is one small state machine (`sc-client::stand::StandHold`): `step(seated, held, dt)` returns the share of the
hold (for the ring) and whether to stand now. Clicking LEAVE STATION stands at once (section 4).

## 3. Helm's keys

| Action | Before | Now |
| --- | --- | --- |
| Roll to port, to starboard | Q, E | Z, C |
| Strafe to port, to starboard | Z, C | Left, Right arrows |
| Strafe up, down | Space, Ctrl | Space, Ctrl |
| Yaw, pitch, speed, all stop | A D, R F, W S, X | unchanged |

Q is free at Helm now. The flipper's tip reads "Flipper: roll left or right (Z, C)".

## 4. LEAVE STATION

In the title band of every console, the same place at every station, right-aligned 8 lp left of the condition chip:

- a chip 24 lp high, `34 + 13 x 9.4` lp wide (156 lp), drawn as the band's other chips (5 lp corners, the band's line
  colour, transparent fill);
- at its left a keycap: a circle of radius 8 lp at (x + 15, 16), its stroke the dim line colour, the key's label in
  it (11 lp, bold);
- round the keycap, the hold's ring: an arc from 12 o'clock, clockwise, `share x 360` degrees, 2 lp wide, in the warn
  colour (the touch ring's amber), drawn only while the key is held;
- then `LEAVE STATION` at 13 lp, bold, letter spaced 1.5, in the dim text colour (the text colour while held);
- tip: "Leave the station (hold E)".

The captain's order chip shortens to end 8 lp before it (`ow = min(300, leave_x - 8 - ox)`). The swap chips, the
single player tabs and OBJECTIVES sit left of 470 lp and never reach it on the 1280 lp band.

A click or a tap on it stands the body up at once, in any phase (the briefing's and the debrief's console too). It is
taken before the console's own input, so on a crewed station's console in single player it is not a touch that takes
the NPC's station MANUAL.

`ConsoleView` carries it as `leave: { key: "E", share: 0.0 }` (the mockup's `consoleState().leave`), so the parity states
draw it on both sides.

## 5. Touch

The touch layer's seated STAND button goes, and `touch.json`'s `stand_bar_min_px` with it. Seated, the layer shows MENU;
the console takes every other finger (`touch-controls` 4), and a tap on LEAVE STATION is a tap on a console control
(`reach_px` 22 makes its 24 lp band easy to hit). The ring fills only from the key; a tap stands at once.

## 6. Bindings

`data/input/bindings.json`, schema `starcrew.bindings/1`, unknown keys refused, every key a name the client knows
(SDL's key names: letters, `Left`, `Space` and so on), no key bound twice within one scope:

```json
{
  "schema": "starcrew.bindings/1",
  "use": "E",
  "helm": { "roll_port": "Z", "roll_starboard": "C", "strafe_port": "Left", "strafe_starboard": "Right" }
}
```

A file that fails stops the client with its path and field (CLAUDE.md 6.5). The Options > Controls panel will read
and write it (`crew-on-deck` task 2.3, `bridge-stations` task 2.3); until then it is edited by hand. The on-foot
prompts still print "E": they follow the binding when the panel lands.

## 7. Pi 5 budget

One chip, its keycap and arc: about 60 UI vertices in the console's batch, no clip rectangle and no draw call. The
touch layer draws one button fewer.

## 8. Checks

- `stand.rs`: a hold of 0.5 s stands; a shorter one does not; the key held from sitting does not count until pressed
  again; the menu resets it.
- `bindings.rs`: the shipped file loads; an unknown key name, an unknown field and a key bound twice are refused.
- `seat.rs`: at Helm, E sends no roll and Z and C roll; the Left and Right arrows strafe.
- `console`: LEAVE STATION is in the title band of every station's console, with a hit that stands; the order chip
  ends before it.
- The parity check passes with the button in the mockup and the engine.
- The walking playtest (`docs/videos/README.md`) shows the player stand mid-Engage and walk on.
