# Proposal: the game menu on Esc

## Why

The owner, 2026-10-10, trying the game on the Pi 5: "also I didnt have a menu when exiting", and "esc should bring up a
menu to return to menu, options, or exit".

Today Esc in the walk first lets go of the mouse and then quits to the desktop on a second press, and Esc in the
co-op drill quits at once. There is no way back to the lobby without restarting the game, and no options at all.
`bridge-stations` 8.6 and `crew-on-deck` already reserve Esc (and a pad's Start) for a menu; this change designs it.

## What Changes

- **Esc opens the game menu** (design 1) wherever the player is aboard: walking, seated at a console, in the drill.
  It lists **Resume, Options, Return to menu, Exit**. Esc again, or Resume, closes it.
- **The ship keeps running** while it is open (design 2): the game is shared, so the menu pauses nothing. Seated, the
  station stays yours; the console is only covered.
- **Return to menu** leaves the ship for the lobby, and **Exit** closes the game; each asks once, Yes or No (design 1).
  In the drill, leaving hands the station back to automation, as a dropped connection does today (`coop-drill` 5).
- **Options** (design 3): Display (Full screen or Window), Names (On or Off, `crew-nameplates`), Look speed and Field of
  view, kept in `settings/options.json`.
- **Esc steps back one layer first** (design 4): it closes a repair game, the map or Options before it opens the menu.

## Impact

- `sc-client`: one menu, drawn by the egui layer the lobby uses (`lobby` 3), opened by the walk (`main.rs`) and the
  drill (`drill.rs`); the lobby gains Options and Exit.
- `LOOK_RAD_PX` (`sc-client/src/main.rs`, 0.0025 rad per pixel) and `fov_deg` (`data/space/exterior.json`, 50) stay
  the defaults; the options scale them per player and never rewrite the data.
- `bridge-stations` 8.6, `crew-on-deck` (the Menu binding), `repairs-on-deck` 4 (Esc goes back a step), `lobby`.
- Pi 5 cost (design 5): one fixed panel while open, nothing while closed.
