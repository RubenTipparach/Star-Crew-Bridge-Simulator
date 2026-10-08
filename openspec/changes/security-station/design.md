# Design: a security station, boarders and crew orders

## Context

- `command-suite` puts eleven consoles on the round bridge; each side wall has a status board at
  -22.5 degrees and a repeater at +22.5 degrees. A status board is a spare: it shows what another
  station owns.
- `crew-on-deck` section 5 locks a door from the damage board or at its panel (hold Use 1.0 s; an
  override of the board's lock takes 3.0 s and is logged). Section 2 caps the bodies at 8 (four NPC
  watch bodies plus the players); damage control's two teams of two are outside that count.
- `ship-frames`: a boarder is a crew body in the boarded ship's interior frame.

## Goals / Non-Goals

**Goals**
- Boarding is something the crew see coming, fight and win or lose, with clear places to defend.
- One player at security can run the defence of the whole ship without walking.

**Non-Goals**
- Boarding the enemy (a later change, the same rules from the other side).
- Hostages, capture, surrender. A downed boarder is out of the fight.

## Decisions

### 1. The station

- Role `security`, station `security` on the bridge, in the port status board's place. The
  starboard status board stays a status board.
- Without a player, automation: locks every entry point that opens with a hostile near it, and sends
  the watch to the nearest defended compartment (below). Competence as `bridge-stations` section 6
  states for automation: it reacts after 5 s.

### 2. Internal sensors

- Every compartment's sensor counts the bodies inside it each tick, split by side (crew, hostile,
  unknown for a suited body the sensor cannot tell). It is a load on `power-grid` (5 W each) and a
  target of `damage-control` (a damaged sensor reads "?").
- What the console shows is exactly that count; a body moving through a "?" compartment vanishes
  from the plan until it reaches a working sensor.

### 3. Ways in

| Entry point | How a boarder uses it | What the crew see |
| --- | --- | --- |
| The airlock (POI 24) | Cycles in: 39.3 s out, 16.1 s in (`life-support`) | The outer door's state, the cycle's progress |
| The docking port (`escape-pods-and-docking`) | A docked enemy craft opens it from outside after a 20 s override | DOCKED on the plan, the override's progress |
| A hull breach | Walks in from space in suits | The breach on the plan (`damage-control`) |
| A boarding pod | Clamps to the hull, cuts a 1.2 m breach in 30 s | An alarm when it clamps, the cut's progress |

An alarm sounds on every console and the security plan flashes the entry point when it opens with a
hostile body within 10 m of it.

### 4. Lockdown

- LOCK on any door, SEAL on any pressure door, from the plan: the same lock as the damage board's
  (`crew-on-deck` section 5), so a boarder at a locked door must force it: 20 s for a door, 60 s for
  a pressure door, shown on the plan as a bar.
- LOCK DECK locks every door on a deck at once; an override at a door still takes 3.0 s for crew.

### 5. Crew orders

- An order is { body or team, compartment, GO | HOLD | CLEAR }. GO walks there; HOLD stays and
  fights what enters; CLEAR walks through it and fights what is in it, then holds.
- NPC watch bodies and damage control teams obey (a team leaves its repair to obey, and the damage
  board shows it). A player sees the order as a marker, a line on the HUD and the compartment's
  name, and may ignore it: players are never moved.
- Path and time come from `crew-on-deck`'s walk graph, so the console shows "Ana: 18 s" before the
  order is given (one implementation, CLAUDE.md 6.1).

### 6. Boarders

- A boarding party is 2 to 6 bodies with the armory's kit. Each picks a target system (the bridge
  first, then the reactor, then the shield generator), walks there, and sabotages it at 2 % of
  integrity a second while standing at it (`damage-control`'s integrity).
- They fight bodies in their way with the armory's weapons; a downed boarder is out of the fight.

### 7. The console

Glance first (`bridge-stations` 8.0), four panels:
1. **DECKS**: the three decks, each compartment a cell with its counts (crew blue, hostile red,
   "?" grey), entry points ringed, alarms flashing.
2. **ORDERS**: the watch and the teams as chips; drag a chip onto a compartment, choose GO, HOLD or
   CLEAR (the biggest buttons).
3. **LOCKS**: LOCK DECK A, B, C; the doors being forced with their bars.
4. **ALARM**: the boarders' count and where the nearest is, the biggest number on the console.

## Risks / Trade-offs

- **Body count.** Six boarders plus eight crew plus four team members is 18 bodies on the server.
  `netcode` tests twelve; this change asks for eighteen and says so in its tasks.
- **Lockdown is strong.** Balanced by forcing times and by boarders cutting their own way in.

## Pi 5 budget

Up to 6 more bodies drawn (about 1,500 triangles each at the far LOD, one draw call each) only when
boarders are aboard; one more console of the existing kind; internal sensors are a count per
compartment per tick on the server (30 compartments, negligible).
