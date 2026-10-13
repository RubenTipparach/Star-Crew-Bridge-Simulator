# Design: the front menu

## Context

The owner, 2026-10-11: "make sure first buttons on menu are single player, multiplayer, options, quit". The lobby
(`lobby`) was built in the engine and judged on its captures (the owner, 2026-10-08: "we're all in real life GLES web
assembly now"); these pages are too: they extend the lobby's panel, in its colours and type.

| Decided elsewhere | Where |
| --- | --- |
| Menus name things; every input device drives every menu; panels a fixed size; the main action is the biggest control | CLAUDE.md 10 |
| The officer: rank, name, roll, station, kept in `settings/officer.json` | `lobby` 1, 4 |
| Options' rows and `settings/options.json`; a bad file loads the defaults with a warning | `game-menu` 3 |
| A solo game is a local server with one client | CLAUDE.md 6.3 |

## 1. The four buttons

Under STAR CREW and SCS TERN, a panel 420 x 360 lp in the middle, four buttons a row each, 380 x 64 lp:

| Row | Does |
| --- | --- |
| SINGLE PLAYER | Opens the Single Player page (design 2). The first row, focused when the menu opens, and the only filled one: the main action |
| MULTIPLAYER | Opens the Multiplayer page (design 3) |
| OPTIONS | Opens Options (design 4) |
| QUIT | Closes the game. In the browser it is off: a page cannot close itself |

Up and Down move the focus, Enter presses; the mouse works on every row. Esc on a page goes back to these four.

## 2. Single Player

The lobby's officer panel (rank, name, roll, station) with BACK, and two actions under it:

| Control | Does |
| --- | --- |
| BEAM ABOARD | The main action: starts the single-player game (`single-player`) at the chosen station |
| TOUR | Walks the ship, as Beam aboard does today (`lobby`) |

The stations offered are the lobby's eight; Comms and Flight ops are off (their consoles are not built), and Any takes
the first station of `bot_posts` (Helm). In the browser Beam aboard starts the same game (`browser-single-player`).

## 3. Multiplayer

The officer panel with BACK, then:

| Control | Does |
| --- | --- |
| SERVER | A text field: a host or host:port, the last one used kept with the officer |
| JOIN | The main action: joins that drill server at the chosen station, as `--connect` does |
| CODE | The six-character code and its Join, off until the matchmaker exists |

## 4. Options

| Row | Control | Default | Kept as |
| --- | --- | --- | --- |
| Display | Full screen, Window | Full screen | `display` |
| Back | Returns to the four buttons | | |

A change applies at once (SDL's window mode) and is written to `settings/options.json`; a file that fails to parse
loads the defaults with a logged warning (CLAUDE.md 6.5: player data is repaired). The other rows of `game-menu` 3
(Names, Look speed, Field of view) join with that change.

## 5. From the menu into a game

The menu and the walk are one `App`, the drill another, each with its own renderer. Starting a game hands the window
over: `platform::hand_over` drops the menu (its renderer shuts sokol down) and builds the drill, which sets it up
again. Leaving the drill (Esc) hands back to the menu.

## 6. Pi 5 budget

| Cost | Amount |
| --- | --- |
| Draw calls | The lobby's: a few while the menu is up, none in the game |
| Memory | `settings/options.json` under 100 bytes |
| CPU | One egui pass of a few rows, as the lobby's |

## 7. Checks

- `--lobby-test` (the lobby's own test, extended): the client opens on the four buttons, shows Multiplayer, Options
  and Single Player, rolls a name and tours the ship, capturing each (`docs/screenshots/engine/menu-*.png`,
  `lobby-*.png`).
- A test reads a damaged `settings/options.json` as the defaults.
- `--hand-over-test DIR`: the menu, Beam aboard into the single-player game in the same window, and back to the menu,
  as Esc does, with a capture of each (`docs/screenshots/engine/hand-over-*.png`).

## Open questions

| Id | Question | Recommendation | Status |
| --- | --- | --- | --- |
| X1 | Approve the four buttons and their pages (the captures) | Approve | In survey 2 with the screenshots (https://claude.ai/artifact/JQm65V7LHVWDJF8dPdZYqY) |
