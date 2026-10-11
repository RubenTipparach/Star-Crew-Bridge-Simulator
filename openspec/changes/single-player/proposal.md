# Proposal: single player, one person and a crew of bots

## Why

The owner, 2026-10-11: "I also want you to implement a mode where everyone on the ship is a bot, single player: 1
human 7 bots", then "bots should competently man stations, player can toggle each station to auto or manual", and
"auto does still need a npc crew to sit there".

Today one person can only play the drill by running a server and bot programs by hand, and a station nobody holds is
run by automation that has no body and plays below a person (`bridge-stations` 3). The crew should be people aboard:
an NPC in every seat, playing it well, and the player deciding which stations they run themselves.

## What Changes

- **Single Player starts a game on the player's machine** (design 1): the client hosts the drill and joins it, with a
  bot crew member for every station but the player's, through the same server and sessions a multiplayer game uses.
- **A full crew** (design 2): one NPC per station, named by the name generator, walking to their seats on the bridge.
  With today's five stations that is the player and four; with Combat 2's eight (`laser-banks`, `enemy-fighters`) the
  player and seven, the crew of eight the owner asked for. The count comes from the roster, not a constant.
- **Competent** (design 3): every NPC plays at the `bot` profile, the one whose crews won six of six rounds on the
  Pis (`coop-drill` 5.1), not automation's lower one.
- **AUTO and MANUAL, station by station** (design 4): on AUTO the NPC in the seat plays it; on MANUAL the NPC stays in
  the seat and does what the player does on that station's console. Either way a station works only while its NPC is
  seated.
- **Every console from one seat** (design 5): the title band shows each crewed station as a tab with its mode; the
  player looks at any of them from their own seat, and switches a station between AUTO and MANUAL there.

## Impact

- `sc-client`: a `solo` module that hosts the drill server and the NPC sessions in the client's frame loop; the drill
  switches the console it shows and sends a MANUAL station's commands as its NPC's; the title band's crew tabs.
- `sc-net::bot`: a bot on MANUAL keeps its seat and sends nothing of its own.
- `sc-client` depends on `sc-server` (the server as a library, CLAUDE.md 6.3: one code path).
- `docs/mockups/consoles.html`: the crew tabs and the mode switch in the title band, a parity state.
- The browser cannot host yet (design 7). Pi 5 cost (design 8).
