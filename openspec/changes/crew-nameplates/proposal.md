# Proposal: a name over everyone aboard

## Why

The owner, 2026-10-10, trying the game on the Pi 5: "I want to always see peoples name in the game, I saw a captain
but I didnt know who that was?", and then "show if they are (npc)".

Today the walk shows the nine bot crew (`crew-npcs` 7) as figures in their department's tunic with no name, so a red
tunic reads as "a captain" and nothing more. The co-op drill (`coop-drill` 9) names the players' bodies, but never
says which of them is a bot. The names already exist: every bot is named by the lobby's generator (`crew-npcs`,
`sc-core::names`), and every player names an officer in the lobby (`lobby`).

## What Changes

- **A nameplate over every crew member in view** (design 1), in the walk and in the drill: the name, and after it
  **(NPC)** when a bot or the station's automation drives the body. A player's own body has none.
- **Seen, not sensed** (design 2): a nameplate shows when the body is on screen, within 25 m and in line of sight on
  the walk grid, so names never show through a bulkhead. On the drill's bridge (one room) every body is named.
- **The drill's "to Helm" line stays** under the name while a body walks to a seat.
- **A setting to turn them off** lives in the game menu's Options (`game-menu` 3); on by default.

## Impact

- `sc-client`: the walk's frame (`main.rs`, the bot crew pass) and the drill's labels (`drill.rs`, `bridge_overlay`)
  draw one shared nameplate routine.
- `sc-core`: one line-of-sight test on the walk grid (design 2), used by nothing else yet.
- `crew-npcs` 7 (the figures), `coop-drill` 9 (the drill's labels), `lobby` (where a player's name comes from).
- Pi 5 cost (design 4): one egui text mesh per frame for at most 16 names, and at most 9 grid walks of 50 cells.
