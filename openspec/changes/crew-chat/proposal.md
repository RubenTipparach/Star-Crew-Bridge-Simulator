# Proposal: crew-chat

## Why

The First Mission Gate (owner, 2026-10-10), steps 1 and 2: "Everyone joins a lobby room and chats", and the bots say
in chat what they are doing. Today the crew can see each other's names in the briefing but cannot say anything, and a
bot's actions show only on its own console.

## What changes

- A chat line in the protocol: a crew member sends a line, the server sends it to everyone with who said it, and keeps
  the last lines for whoever joins late.
- The room: the briefing panel in Muster gains a CHAT column, the lines and a field to type in.
- In the game: the last three lines over the bottom of the screen, fading after 8 s; T opens the field to type.
- The NPC crew say what they do, as a player would type it: "At Helm", "Shields to the bow", "Fire out in the medbay".
  Damage control's teams speak as "Damage control".

## Impact

- `sc-net` (two messages), `sc-server` (the lines kept, damage control's lines), `sc-net::bot` (what a bot says),
  `sc-client` (the room's column, the in-game strip, the field). A mockup first: `docs/mockups/mission-room.html`.
