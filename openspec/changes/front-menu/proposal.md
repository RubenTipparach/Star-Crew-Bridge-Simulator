# Proposal: the front menu

## Why

The owner, 2026-10-11: "make sure first buttons on menu are single player, multiplayer, options, quit". Today the game
opens on the lobby's one panel (officer, station, Beam aboard, and Join a crew switched off), Beam aboard only tours the
ship, and the drill is reached only with `--connect` on the command line. There is no way to quit but Esc in the walk,
and no options.

## What Changes

- **The game opens on four buttons** (design 1), in this order: **Single Player, Multiplayer, Options, Quit**, under
  the title, over the dock view the lobby already shows.
- **Single Player** (design 2) opens the lobby's officer panel (rank, name, roll, station) with **Beam aboard**, which
  starts the single-player game (`single-player`), and **Tour**, which walks the ship as Beam aboard does today.
- **Multiplayer** (design 3) opens the same officer panel with a **server** field and **Join**, which joins a drill
  server by its address, as `--connect` does today. Joining by a six-character code stays off until the matchmaker
  exists (`matchmaker`).
- **Options** (design 4) holds **Display** (Full screen or Window), applied at once and kept in
  `settings/options.json`; `game-menu`'s other rows join it when that change is built.
- **Quit** closes the game.

## Impact

- `sc-client`: `lobby.rs` gains its pages; `main.rs` hands the window to the drill when a game starts
  (`platform::hand_over`); the officer file keeps the last server.
- `lobby` (design 1) and `game-menu` (design 1 and 3): the lobby's panel becomes the Single Player and Multiplayer
  pages; Options is `game-menu`'s, started here with its first row.
- Pi 5 cost (design 6): a panel of four buttons, as the lobby's.
