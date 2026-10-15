# Proposal: crew-chat

## Why

The First Mission Gate (owner, 2026-10-10), steps 1 and 2: "Everyone joins a lobby room and chats", and the bots say
in chat what they are doing. Today the crew can see each other's names in the briefing but cannot say anything, and a
bot's actions show only on its own console.

The owner widened it on 2026-10-14: "create a chat window for the bots to communicate orders, acknowledgements and
intents like firing torpedos, targeting enemy, warping to coordinates, repairing hull etc....". The chat is the crew's
radio: the captain's orders, the stations' "aye", what each station is doing as it does it, and what came of it.

## What changes

- A chat line in the protocol: a crew member sends a line with its kind, the server stamps it with who said it (name and
  station) and the game time, sends it to everyone, and keeps the last lines for whoever joins late.
- Four kinds of line besides plain talk, each with its colour and its shape: ORDER (the captain to a station), ACK (a
  station's "aye"), INTENT (a station saying what it does: a torpedo away, a lock, power rerouted, a team sent to a
  fire) and REPORT (an outcome: shields at 40%, fire out, target destroyed).
- A vocabulary as data, `data/crew/chat.json`: every line the NPC crew can say, by kind, with the limits that keep the
  channel readable (line length, lines kept, how often a speaker may talk, how long a line stays in the strip).
- The NPC crew speak from what they actually decide, never invented: a bot client (`sc-net::bot`) through the same chat
  message a player sends, the drill's automation for an empty seat, and the server's own events (damage control's
  teams, the ship's computer for hull, shields, breaches and a kill). The bot captain gives orders.
- The room: the briefing panel in Muster gains a CHAT column, the lines and a field to type in.
- In the game: the last three lines over the bottom left, fading after 6 s; T opens the chat log, a panel of the last
  lines that scrolls, with a field to type in.

## Impact

- `sc-core` (the vocabulary, the kinds, the pacing and the kept lines; the drill's automation and events speak; the bot
  captain's orders as a `bot` profile switch in `data/stations.json`), `sc-net` (two messages, `PROTOCOL` up by one,
  what a bot says), `sc-server` (the lines kept and sent to a late joiner, the drill's lines broadcast), `sc-client`
  (the lines printed to stdout until the screens are approved; then the room's column, the strip and the log).
- A mockup first: `docs/mockups/mission-room.html` (states 1, 3, 4, 7 and 8). The protocol, the server, the data and the
  bots are not a screen and are built before the approval; the screens wait for it (CLAUDE.md 10).
- Network: a line is at most 120 bytes on the reliable command channel, never in the snapshot.
