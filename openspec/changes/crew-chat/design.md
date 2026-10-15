# Design: crew chat

The owner's ask, 2026-10-14: "create a chat window for the bots to communicate orders, acknowledgements and intents like
firing torpedos, targeting enemy, warping to coordinates, repairing hull etc....". It extends the First Mission Gate's
steps 1 and 2 (2026-10-10): "Everyone joins a lobby room and chats", and the bots say what they are doing.

## 1. The line

A line is who said it, its kind, its text and the game time:

| Field | On the wire | Notes |
| --- | --- | --- |
| slot | 1 byte, 255 for none | The speaker's slot; none for a line the server speaks (automation, damage control, the computer) |
| name | string, at most 24 characters | The speaker's name as the crew list has it; kept with the line, so a late joiner sees who said it after they left |
| station | 1 byte, 255 for none | The station the speaker held when they said it; the name takes its colour |
| npc | 1 byte | A bot, automation or the server: shown "(NPC)" after the name |
| kind | 1 byte | SAY, ORDER, ACK, INTENT or REPORT (section 2) |
| text | string, at most 80 characters | Control characters stripped, trimmed, an empty line dropped |
| at_s | f32 | The drill's clock when the server took it, seconds since it started (`tick x DT`) |

`ClientMsg::Chat { kind, text }` goes on the reliable command channel; the server checks the sender's pace (section 4),
stamps it and sends `ServerMsg::Chat(line)` to everyone, the sender too. A player's typed line is SAY; a bot's lines
carry their kind. It keeps the last `keep_lines` (40) and sends them, oldest first, to a client right after its Welcome.
The snapshot does not change: chat never rides in it (netcode-and-sessions 2, the datagram budget).

## 2. The kinds and the vocabulary

`data/crew/chat.json` (`starcrew.chat/1`) holds the kinds, the limits, the server's speakers and every line the NPC crew
can say. A line is a template with named slots filled from the game: `{target}` (the enemy's class name), `{tube}`
(1-based), `{station}`, `{preset}` (a shield or power preset), `{band}` (A-D), `{room}` and `{system}` (their names in
`layout.json`), `{pct}`, `{heading}` and `{pitch}` (whole degrees, the heading in three figures). A template with an
unknown slot, a kind that is not one of the five, a line id the engine does not speak, a missing order or ack for a
verb in `data/stations.json`, or a text longer than `line_chars` stops the load with the file and the field.

| Kind | Who | Colour (the console's) | Shape | Examples |
| --- | --- | --- | --- | --- |
| SAY | A player typing | text white | none | "Evening all, ready when you are" |
| ORDER | The captain, to a station | cyan | a chevron | "Tactical, engage the Hound corvette", "Red alert, all hands to stations" |
| ACK | A station, to the captain | green | a tick | "Aye, engaging", "Aye, closing" |
| INTENT | A station, saying what it does as it does it | amber | an arrow | "Locking the Hound corvette", "Torpedo away, tube 1", "Power to combat settings", "Fire in the mess, on my way" |
| REPORT | Anyone, on an outcome | lilac | a square | "At Helm", "Shields at 50%", "Fire out in the medbay", "Target destroyed" |

Colour means the same everywhere and always comes with its shape (CLAUDE.md 10). The speaker's name keeps its
station's colour.

The vocabulary (the file is the one source; this is its shape):

| Source | Line id | Kind | Text |
| --- | --- | --- | --- |
| Sitting down | `seated` | REPORT | At {station} |
| Captain: Alert | `alert_on`, `alert_off` | ORDER | Red alert, all hands to stations / Stand down from red alert |
| Captain: Brace | `brace` | ORDER | Brace for impact |
| Captain: Order | `orders.<station>.<VERB>` | ORDER | Helm, close on the {target}; Tactical, engage the {target}; Engineering, repairs first; Science, scan the {target} ... one per verb |
| A station: Ack | `acks.<station>.<VERB>` | ACK | Aye, closing; Aye, engaging; Aye, repairs first ... one per verb |
| Helm: modes | `helm_course`, `helm_chase`, `helm_match`, `helm_evade`, `helm_hold`, `helm_stop` | INTENT | Setting course for the waypoint; Closing on the {target}; ... |
| Helm: attitude order | `helm_orient` | INTENT | Coming to {heading} mark {pitch} |
| Helm: a bot starts to hunt | `helm_hunt` | INTENT | Bringing her about onto the {target} |
| Tactical | `lock`, `lock_clear`, `turrets_free`, `turrets_hold`, `shields`, `tube_load`, `tube_fire`, `decoy` | INTENT | Locking the {target}; Torpedo away, tube {tube}; Shields to {preset} ... |
| Lasers | `laser_fire` | INTENT | {station} firing |
| Science | `scan`, `ping`, `band` | INTENT | Scanning the {target}; Ping out; Shields to band {band} |
| Engineering | `power_preset`, `power_reroute` | INTENT | Power to {preset} settings; Rerouting power round the damage |
| Damage control: a team sent | `fire_team`, `repair_team` | INTENT | Fire in the {room}, on my way; Repair team to the {system} |
| Damage control: done | `fire_out`, `fire_spread`, `repaired` | REPORT | Fire out in the {room}; Fire spreading to the {room}; {system} back to nominal |
| The computer | `target_destroyed`, `hull`, `shields_low`, `breach` | REPORT | Target destroyed; Hull at {pct}%; Shields at {pct}%; Hull breach in the {room} |

Warping: the Tern has no warp drive in the drill yet. COURSE to the waypoint and an attitude order are its "warp to
coordinates" today ("Setting course for the waypoint", "Coming to 090 mark 10"). The jump of `missions` 3 adds JUMP to
Helm's verbs, and with it `orders.helm.JUMP` ("Helm, jump us out") and `acks.helm.JUMP` ("Aye, spinning up the jump
drive"); the mockup shows them (state 8) as that change's lines. An NPC taking an extinguisher speaks `fire_team` when
bots fight fires on foot (all-hands); until then damage control's teams are the NPCs who do.

## 3. Who says what, and never invented

Every NPC line comes from a decision the game made, through one mapping in `sc-core::combat::chat` from a command or an
event to its line (CLAUDE.md 6.1). Its three callers:

| Speaker | Says | Through |
| --- | --- | --- |
| A player | What they type (SAY); the captain's orders and a station's ack given from the console (ORDER, ACK), as theirs | `ClientMsg::Chat`; the server says the console's orders and acks for a person by the same mapping |
| A bot client (`sc-net::bot`) | "At {station}" when it sits; the line of each command its station's automation sends (a lock, a torpedo, a preset, a scan, a laser); the ack of the captain's newest order to it; the bot captain's orders; a hunting helm's "Bringing her about" | `ClientMsg::Chat`, the same message a player sends (CLAUDE.md 6.3) |
| The drill's automation (an empty seat) | The line of each command it applies and each ack, as "{station} (NPC)" | `DrillEvent::Said`, broadcast by the server |
| Damage control (the server) | A team sent to a fire or a repair, a fire out or spreading, a system back to nominal | The same, from `damage::DamageEvent` |
| The computer (the server) | The enemy destroyed; the hull and the shields falling past each of `report_steps_pct` (75, 50, 25, 10); a new breach | The same, from the drill's state |

The bot captain gives orders (a `bot` profile switch, `captain.orders` in `data/stations.json`; the drill's automation
captain stays auto-condition only, bridge-stations 3). At most one order every `gap_s` (4 s), each once a round, the
first that applies of: red alert with a hostile in the fight: Tactical ENGAGE, then Helm CHASE; a hostile not yet
scanned: Science SCAN; the hull under `repair_below_frac` (0.6) of full: Engineering REPAIR; red alert over after a
fight: Engineering CRUISE. Each order is a real `Command::Order`: Engineering's automation sets the preset it names,
REPAIR raises the worst system for the teams (ship-damage 5), and every station acknowledges it, so each order is
followed by its "aye".

## 4. Keeping it readable

| Rule | Value (`limits` in the file) |
| --- | --- |
| A line's length | `line_chars` 64 for the NPCs' templates; a player's line at most 80 (the wire's cap) |
| A player's pace | `speaker_gap_s` 1.0: a second line within a second of the last is dropped (the server's check) |
| An NPC's pace | `npc_gap_s` 2.5: an NPC says at most one line in that time; of the lines it had to say, the most important (`rank`, 1 first, the newest of a tie) |
| No repeats | `repeat_s` 20: an NPC does not say the same text again within it (a laser bank firing each charge says so once) |
| Lines kept | `keep_lines` 40 on the server and in the log |
| The strip | `strip_lines` 3, each fading `strip_fade_s` 6 s after it came |
| The log | `log_lines` 14 seen at once, scrolling back through the kept lines |

The pacing is one `Voice` in `sc-core::combat::chat`, used by the bot and by the drill for each automated station,
damage control and the computer; the server's check of a player's pace is the kept log's (`ChatLog::post`).

## 5. On screen

- **The room** (Muster's briefing panel): a CHAT column at the right, 12 lines, the newest at the bottom, each the kind's
  shape, "Name: line" with the name in its station's colour and (NPC) after an NPC's; a field under it, Enter sends.
- **At a console**: the newest line for 6 s in the title band's free space, after the chips and the captain's order,
  short of the condition, clipped with an ellipsis (CLAUDE.md 10: a console's panels are never covered).
- **On foot**: the last three lines at the bottom left, 25 px on a dark band, each with its kind's shape and colour,
  fading 6 s after it came.
- **The log (T)**: a panel over the left of the view, on foot or at a console: the game time, the shape, the speaker and
  the line, the newest at the bottom, 14 lines and a scroll bar back through the 40 kept; the field under it; Enter
  sends, Esc closes it. Mouse wheel or the pad's right stick scrolls.

Until the owner approves the mockup the client prints each line it receives, `sc-client: chat: [mm:ss] KIND Name
(Station): text`, so playtests show the crew's talk.

## 6. Cost

A line is at most 1 + 1 + 25 + 1 + 1 + 1 + 81 + 4 = 115 bytes on the reliable channel. Forty lines kept on the server
(about 5 KB) and in each client. NPC lines are paced to one each 2.5 s a speaker: with eight stations, damage control
and the computer, at most 4 lines a second in the worst fight, 0.5 KB/s, far under the command channel's share. The
snapshot is unchanged. On screen: one egui layer, three lines in the strip, one panel for the log (1-2 draw calls).
