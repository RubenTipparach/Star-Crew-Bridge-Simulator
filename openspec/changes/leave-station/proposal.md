# Proposal: leave a station with one key and one button

## Why

The owner played single player in the browser on 2026-10-15: "also why cant I get up during combat? and walk around?".
Then: "need a "Leave station button"", and on a key for it: "a key would be nice too". The coordinator passed on the
shape: the key is the one you sit with, the same at every station with no exceptions, drawn on the button, and kept in
the data file the Options > Controls panel will read.

From the player's side there was no way up:

- **At Helm, the seat a single player gets by default, E rolls the ship.** The stand rule took E everywhere else but
  only Backspace at Helm, and nothing said so. Holding E at the helm rolled the Tern to starboard and never stood the
  body up.
- **Nothing on screen says how.** A keyboard and mouse player saw no prompt while seated. Only the touch layer had a
  STAND button with a ring.
- **A tap does nothing.** E taps to sit but has to be held to stand, so the key that sat you down seemed dead once
  seated.

Nothing else blocks it: the core's `stand` has no rule against combat, the mouse is not captured while seated, the menu
takes keys only while it is open, and on foot the walk takes W A S D and the mouse in every phase (design 1).

## What Changes

- **One key sits and stands at every station** (design 2): E, the use key, from `data/input/bindings.json`. Held 0.5 s
  (`stations.json` `seats.stand_hold_s`) while seated it stands the body up; the hold counts only from a press made in
  the seat, so the E that sat you down never stands you straight back up. Backspace no longer stands.
- **Helm's roll moves off E** (design 3): roll on Z and C, as `bridge-stations` 10.1's table already has it; the
  lateral strafe moves from Z and C to the Left and Right arrows. Space and Ctrl still strafe up and down.
- **LEAVE STATION in every console's title band** (design 4): on the right, before the condition and the clock, the
  same place at every station. It shows the stand key as a keycap, and the keycap's ring fills while the key is held.
  A click or a tap stands the body up at once.
- **The touch STAND button becomes LEAVE STATION** (design 5): the seated touch layer keeps MENU only; a tap on LEAVE
  STATION stands, as a click does. One control, one implementation.
- **Bindings are data** (design 6): `data/input/bindings.json`, validated at start, read by the seat, the walk's sit and
  the button's keycap.

The station passes to automation the moment the body stands, as it always has (`bridge-stations` 6).

## Impact

- `sc-client`: the stand hold (`stand.rs`), the bindings (`bindings.rs`), the drill's stand and sit keys, Helm's keys
  in `seat.rs`, the title band in `console/mod.rs`, the touch layer's seated buttons.
- `data/input/bindings.json` (new), `data/input/touch.json` (the STAND button's placement goes).
- `docs/mockups/consoles.html`: the title band's LEAVE STATION, Helm's keys and the flipper's tip; the parity states
  carry the button.
- Pi 5 budget: one title-band control, a few dozen UI vertices in the console's existing batch; no draw call (no clip
  rectangle), nothing in 3D.
