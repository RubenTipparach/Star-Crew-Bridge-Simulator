# Design: crew chat

## 1. The line

`ClientMsg::Chat(text)` on the reliable channel: at most 120 characters, control characters stripped, an empty line
dropped. The server stamps it with the sender's slot and the drill's clock and sends `ServerMsg::Chat { slot, at_s,
text }` to everyone, the sender too. It keeps the last 20 lines and sends them to a client that joins. A crew member
sends at most one line a second; more are dropped (the line is the game's, not a firehose).

Lines the server speaks itself, damage control's, carry no slot and the speaker's name ("Damage control").

## 2. Who says what

| Speaker | Says, when |
| --- | --- |
| A player | What they type |
| An NPC at a station (`sc-net::bot`) | "At Helm" on sitting down; one line for each of its station's decisions that changes the ship: "Shields to the bow", "Locked on the Hound", "Missile away", "Decoy out", "Laser fired", "Evading" |
| Damage control (the server) | "Team to the drive", "Fire out in the medbay", "Drive back to nominal" |

A bot speaks through the same `Chat` message a player sends (CLAUDE.md 6.3: NPC work runs through the players' rules).
At most one line from a bot each 3 s, its newest.

## 3. On screen

- **The room** (Muster's briefing panel): a CHAT column at the right, 12 lines, the newest at the bottom, each "Name:
  line" with the name in its station's colour and (NPC) after a bot's; a field under it, Enter sends.
- **At a console**: the newest line for 8 s in the title band's free space, after the chips and the captain's order,
  short of the condition, clipped with an ellipsis (CLAUDE.md 10: a console's panels are never covered). T opens the
  chat over the look band's left side (12 lines and the field); Enter sends, Esc closes it.
- **On foot** (the bridge's view): the last three lines at the bottom left, 17 px on a dark band, each fading 8 s after
  it came; T opens the field.

## 4. Cost

A line is at most 125 bytes on the reliable channel; 20 kept on the server. On screen: one egui layer, three lines.
