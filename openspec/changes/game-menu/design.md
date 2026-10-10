# Design: the game menu on Esc

## Context

The owner, 2026-10-10: "also I didnt have a menu when exiting", "esc should bring up a menu to return to menu, options,
or exit". The mockup is `docs/mockups/game-menu.html` (states 3, 4 and 5).

| Decided elsewhere | Where |
| --- | --- |
| Menu is Esc on a keyboard and Start on a pad | `bridge-stations` 8.6, `crew-on-deck` |
| Esc goes back a step from a repair game | `repairs-on-deck` 4 |
| Menus name things; every input device drives every menu; panels a fixed size | CLAUDE.md 10 |
| The UI layer is egui on a sokol painter; the officer is kept in `settings/` | `lobby` 3, 4 |
| A solo game is a local server and one client: nothing pauses it | CLAUDE.md 6.3 |

## 1. The menu

A panel 480 x 420 px in the middle of the screen over a 50 % dark veil, in the lobby's colours:

| Row | Does |
| --- | --- |
| MENU | The title, and under it where you are: `SCS Tern, Helm` or `SCS Tern, Deck A` |
| Resume | Closes the menu (the first row, focused when it opens) |
| Options | Opens Options in the same panel (design 3) |
| Return to menu | Asks `Leave the ship?` with Yes and No in the same row; Yes goes to the lobby |
| Exit | Asks `Exit Star Crew?` with Yes and No; Yes closes the game |

Opening it lets go of the mouse; Resume takes it back in the walk. Up and Down (or a pad's d-pad) move the focus, Enter
or A presses, Esc or B goes back. The mouse works on every row.

On the lobby, Esc opens the same panel with Options and Exit only (there is nothing to resume or return to).

## 2. The game keeps running

The ship is shared, so nothing pauses: other crew keep moving, the drill's round keeps its clock, and a bot keeps
walking. The menu only stops this player's input to the game underneath. Seated, the station stays held by the player
while the menu is up. A player who goes to the lobby leaves the crew, and the station passes to automation the way a
lost connection does today (`coop-drill` 5).

## 3. Options

The same panel, a row each, a label and a control (CLAUDE.md 10):

| Row | Control | Default | Kept as |
| --- | --- | --- | --- |
| Display | Full screen, Window | Full screen | `display` |
| Names | On, Off | On | `names` (`crew-nameplates` 3) |
| Look speed | A slider, 25 % to 200 % | 100 % (0.0025 rad per pixel) | `look_speed_pct` |
| Field of view | A slider, 40 to 80 degrees | 50 degrees (`data/space/exterior.json`) | `fov_deg` |
| Back | Returns to the menu | | |

Changes apply at once and are written to `settings/options.json` (the browser's storage later, like the officer).
A file that fails to parse loads the defaults with a logged warning: player data is repaired (CLAUDE.md 6.5).
Volume is left out until the game has sound. Recommendation taken on the rows (ask only with screenshots).

## 4. Esc steps back one layer at a time

Esc closes the innermost thing first, then the next press opens the menu:

1. Options, back to the menu.
2. The menu, back to the game.
3. A repair game, back to the docked view (`repairs-on-deck` 4).
4. The map (M), back to the walk.
5. Otherwise, the menu opens.

The walk's first Esc no longer only lets go of the mouse: the menu opening does that.

## 5. Pi 5 budget

| Cost | Amount |
| --- | --- |
| Draw calls | 1-2 while open (the veil and the panel, one clip rectangle each); none while closed |
| Triangles | Under 1,500 while open |
| CPU | One egui pass of six rows, which the lobby already runs: about 0.2-0.5 ms on a desktop, to measure on the Pi |
| Memory | `settings/options.json` is under 200 bytes; nothing else held |
| Network | Return to menu sends the leave the drill already sends on closing |

## 6. Checks

- A headless test opens the menu with Esc, steps the focus to Return to menu, confirms, and lands on the lobby.
- A headless test sets Names Off, restarts the client and reads it back from `settings/options.json`.
- Captures of the menu and Options in the walk and seated, beside the mockup's, in `docs/screenshots/engine/`.

## Open questions

| Id | Question | Recommendation | Status |
| --- | --- | --- | --- |
| Z1 | Approve the mockup (`docs/mockups/game-menu.html`, states 3, 4 and 5) | Approve | In the survey, with the shots |
| Z4 | The Options rows: Display, Names, Look speed, Field of view | Those four; Volume once there is sound | Recommendation taken (ask only with screenshots) |
| Z5 | Leaving asks once, with No in focus | Ask once | Recommendation taken (ask only with screenshots) |
