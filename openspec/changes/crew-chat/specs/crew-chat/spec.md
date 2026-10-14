# crew-chat

## ADDED Requirements

### Requirement: The crew can talk

A crew member SHALL be able to send a line of chat that every crew member sees with the sender's name, in the room
before the fight and during it. Each line SHALL carry who said it (name and station), its kind, its text and the game
time, and SHALL travel on the reliable command channel, never in the snapshot.

#### Scenario: Two machines in the room
- **WHEN** a player on Pi 2 types "Ready at Tactical" in the room
- **THEN** the captain on Pi 1 sees "Ens. Dara Holt: Ready at Tactical" in the CHAT column

#### Scenario: A typed line keeps its sender
- **WHEN** a player at Tactical sends a line
- **THEN** every client receives it with the player's slot, name and station, the kind SAY and the server's game time

### Requirement: A late joiner sees the last lines

The server SHALL keep the last `keep_lines` lines of `data/crew/chat.json` and SHALL send them, oldest first, to a
client right after its Welcome.

#### Scenario: Joining mid-conversation
- **WHEN** a player joins after three lines were said
- **THEN** their client receives those three lines, in the order they were said, before any new one

#### Scenario: Only the last lines are kept
- **WHEN** more lines than `keep_lines` have been said
- **THEN** the server keeps only the newest `keep_lines`

### Requirement: Every line has a kind with its colour and shape

A line SHALL be one of five kinds: SAY (a player's typing), ORDER (the captain to a station), ACK (a station's
acknowledgement), INTENT (a station saying what it does as it does it) and REPORT (an outcome). Each kind SHALL have
one colour and one shape, from the vocabulary file, the same on every screen.

#### Scenario: A kind on the wire
- **WHEN** a line arrives with a kind byte that is not one of the five
- **THEN** the message is dropped as undecodable

### Requirement: The vocabulary is data and is checked at load

Every line the NPC crew can say SHALL come from `data/crew/chat.json`, a template per line id with named slots. The load
SHALL fail, naming the file and the field, on an unknown key, an unknown kind, a slot the engine does not fill, a line
id the engine does not speak, a line id the engine speaks that is missing, a captain's verb in `data/stations.json`
without its order or its ack, or a template longer than `line_chars`.

#### Scenario: A verb without its ack
- **WHEN** `data/stations.json` gives Helm a verb that `chat.json` has no ack for
- **THEN** the drill's data does not load and the error names `acks.helm`

#### Scenario: A misspelt slot
- **WHEN** a template says `{taget}`
- **THEN** the load fails naming that line

### Requirement: The NPC crew say what they do

An NPC crew member SHALL post a line for what it decides, through one mapping from a command or an event to its line,
and SHALL never post a line for something it did not do. A bot SHALL speak through the same chat message a player
sends; the drill's automation, damage control and the computer SHALL speak through the server.

#### Scenario: The helm's NPC sits down
- **WHEN** an NPC sits down at Helm
- **THEN** every crew member sees "Lt. Sil Saren (NPC): At Helm" as a REPORT

#### Scenario: A bot fires a torpedo
- **WHEN** a bot at Tactical fires tube 1
- **THEN** it says "Torpedo away, tube 1" as an INTENT

#### Scenario: A bot acknowledges an order
- **WHEN** the captain orders Tactical to ENGAGE and a bot holds Tactical
- **THEN** the bot says "Aye, engaging" as an ACK when it acknowledges

#### Scenario: Damage control sends a team to a fire
- **WHEN** a damage-control team is sent to a fire in the mess
- **THEN** "Damage control" says "Fire in the mess, on my way" as an INTENT, and "Fire out in the mess" as a REPORT when
  it is out

#### Scenario: Automation speaks for an empty seat
- **WHEN** no one holds the captain's seat and a hostile comes inside red-alert range
- **THEN** "Captain (NPC)" says "Red alert, all hands to stations" as an ORDER

#### Scenario: A person's order from the console
- **WHEN** the captain, a person, sends Tactical ENGAGE from the ORDERS panel
- **THEN** every crew member sees "Lt. Player: Tactical, engage the Hound corvette" as an ORDER

### Requirement: The bot captain gives orders

A bot at the captain's seat SHALL give orders by its profile's `captain.orders`: at most one each `gap_s`, each once a
round, the first that applies of Tactical ENGAGE and Helm CHASE at red alert with a hostile in the fight, Science SCAN
while the hostile is not scanned, Engineering REPAIR with the hull under `repair_below_frac`, and Engineering CRUISE
when red alert ends after a fight. The drill's automation captain SHALL give none.

#### Scenario: Red alert brings an order
- **WHEN** a bot captain has called red alert with the Hound in the fight
- **THEN** its next order is "Tactical, engage the Hound corvette", and no second order follows within `gap_s`

### Requirement: The channel stays readable

A player's line within `speaker_gap_s` of their last SHALL be dropped by the server. An NPC SHALL say at most one line
each `npc_gap_s`, the most important of those it had to say, and SHALL not repeat the same text within `repeat_s`.

#### Scenario: Typing too fast
- **WHEN** a player sends two lines 0.2 s apart
- **THEN** the server keeps and sends the first and drops the second

#### Scenario: A busy tactical officer
- **WHEN** a bot locks, sets the turrets free and fires a torpedo in one decision
- **THEN** it says "Torpedo away, tube 1" (the most important) and nothing else for `npc_gap_s`
