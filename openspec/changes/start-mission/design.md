# Design: START MISSION

## Context

| Decided elsewhere | Where |
| --- | --- |
| Muster, Countdown, Engage and Debrief; the countdown's length (`mission.countdown_s`) | `coop-drill` 1 |
| On foot the round waits for every person to sit, and sitting in Muster is their READY | `browser-page-fit` design 6, finding 8 |
| The title band: tabs, OBJECTIVES, the order, LEAVE STATION, the condition, the clock | `bridge-stations` 8.1, `leave-station` 4, the approved `consoles.html` |
| Single player: one person, an NPC at every other station | `single-player` |
| Keys are data | `leave-station` 6 (`data/input/bindings.json`) |

The owner's words (survey 2, Q3, 2026-10-16): "approve, but need a start mission button when everyone is ready, or
captain can start even when everyone is not ready. it doesnt matter."

## 1. The round starts by itself, as today

Nothing changes in the automatic start: in Muster, once at least one player is seated, every seated player is ready,
and (on foot) every person aboard is seated, the drill moves to Countdown, and after `mission.countdown_s` to Engage (or
to the patrol). "When everyone is ready" in the owner's words is this rule; the button adds the other half.

## 2. Who may start

START MISSION moves Muster to Countdown at once, whoever is ready or seated. The countdown then runs as it always does.
A request is taken only when all of these hold, else refused and nothing changes:

| Check | Refused with |
| --- | --- |
| The drill is in Muster (in Countdown the start is already under way) | `WrongPhase` ("Not now") |
| The sender is a person, not a bot client | `NotYourStation` |
| The sender is seated at a station (the control is on a console) | `NotYourStation` |
| The sender holds the captain's seat, or no person holds it | `CaptainsCall` ("Captain's call") |

The last row is the owner's "captain can start", made to hold in every crew:

- **A person is captain**: only they may start. Everyone else waits for the captain or for all to be ready.
- **No person is captain** (an NPC or automation holds the seat): any seated person may start. In single player that is
  the player, at any seat: "it doesnt matter".

A bot never starts a round: it is ready once seated, which the automatic start already counts.

## 3. One rule

`sc_core::combat::may_start(crew, slot)` takes the crew as `(slot, station, bot)` and the sender's slot and returns
`Ok(())` or the refusal of the table's last three rows. `Drill::start_mission(slot)` checks the phase, then calls it on
its players. The client calls the same function on the crew list the server sends, only to decide whether to draw the
control and listen for its key; the server's answer is the one that counts (CLAUDE.md 6.1, 6.3).

## 4. The control

In the title band, while the drill is in Muster and `may_start` passes for this client, on whatever console is shown
(single player's crewed consoles too). It takes the order's place: at 470 lp, or after the tabs when they reach past
it, ending at least 8 lp before LEAVE STATION. No order is ever pending in Muster, since commands are taken only in
Engage and the patrol, so the two never meet; START wins if they did.

- a chip 24 lp high, `34 + 13 x 9.4 + 46` lp wide (202 lp), 5 lp corners, filled with the ok colour at 0x26 alpha and
  stroked in it: the "go" of the band, the same green as READY and NORMAL;
- a keycap at its left: a rounded box 16 lp high, 4 lp corners, `10 + 6.5 x` the key's letters lp wide, at (x + 6, 8),
  stroked in the dim colour, the key's name in it at 10 lp bold (the band's circle keycap holds one letter; Return
  needs the box);
- `START MISSION` after it at 13 lp, bold, letter spaced 1.5, in the text colour;
- at its right end, right aligned at 10 lp inside, `ready/aboard` (for example `3/8`) in the dim colour: the one
  number the captain acts on (wait, or go);
- tip: "Start now: 3 of 8 ready".

When the tabs leave it less than its width it is not drawn; its key still works. A click or a tap on it, or the key,
sends the request once; the server's refusal, if any, shows as every refusal does.

`ConsoleView` carries it as `start: { key: "Return", ready: 3, aboard: 8 }` (the mockup's `consoleState().start`, none
when not drawn), so the parity states draw it on both sides.

## 5. The key

`data/input/bindings.json` gains `"start": "Return"`. Return is bound nowhere in the seated game (the consoles' keys
are letters, Tab, Space, Ctrl, Shift and the arrows), and the front menu's Enter is a different screen. The check
that no key is bound twice covers it.

## 6. The wire

`ClientMsg::StartMission`, tag 14, no body. `PROTOCOL` 17 to 18. The server's handler calls `Drill::start_mission`
and sends `ServerMsg::Refused` on an error, as `Claim` does. `Refusal::CaptainsCall` is 17.

## 7. Checks

- `sc-core`: the captain's request starts the countdown with nobody else ready; another seated person's request is
  refused while a person is captain and nothing changes; with no person in the captain's seat a seated person starts
  it; a bot's request is refused; outside Muster it is refused; the round still starts by itself when everyone is
  ready.
- `sc-net`: the message round trips; a server test: the captain's message moves the drill to Countdown, another
  station's gets `Refused(CaptainsCall)`.
- `sc-client`: in single player the player seated at Helm, a bot in the captain's seat and the crew still walking,
  presses Return and the round counts down; the chip's hit area starts it; the parity states `start-captain` and
  `solo-start` within the limits.
- Captures in `docs/screenshots/engine/start-mission/`.

## 8. Pi 5 budget

One title-band chip: a few dozen UI vertices in the console's existing batch, no clip rectangle, so no new draw call.
One message of one byte per press. One pass over at most 8 players a request on the server. Nothing in 3D, no memory
beyond the view's three fields.

## Open questions

| Id | Question | Recommendation | Status |
| --- | --- | --- | --- |
| S1 | With no person in the captain's seat, may any seated person start? | Yes: single player needs it, and "it doesnt matter" | Recommendation taken (ask only with screenshots); the captures go with the next survey |
| S2 | In Countdown, a second press skips the rest of the countdown? | No: the countdown is a few seconds and is the crew's warning | Recommendation taken (ask only with screenshots) |
