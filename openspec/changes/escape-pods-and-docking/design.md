# Design: escape pods and a docking port

## Context

- The airlock (POI 24, deck C, 26.4 m3) cycles out in 39.3 s to 5 kPa and in with a 14.1 s fill
  (`life-support` section 14). Its doors `p_airlock_inner` and `p_airlock_outer` are pressure doors.
- `ship-frames` section 9: docking portals join two interiors; a craft leaving the ship is handed to the
  exterior frame at one tick with its velocity.
- The crew aboard: up to eight bodies plus the damage control teams' four (`crew-on-deck`, `damage-control`).

## Goals / Non-Goals

**Goals**
- Losing the ship is a scene the crew play out, not a screen: they run for the pods.
- One chamber does EVA and docking; a second exists in case the first is lost.

**Non-Goals**
- Rescue and the campaign after a lost ship (the campaign change).
- Docking flight: how a craft lines up with the collar is `flight-and-navigation`'s.

## Decisions

### 1. Escape pods

| Item | Value |
| --- | --- |
| Pods | 3, four seats each: two on deck B either side of the spine near the quarters and the mess, one on deck A by the bridge |
| Mass | 1.2 t each |
| Air and power | 72 h for four, a beacon |
| Boarding | Through a 0.9 m hatch from the corridor, 1.5 s each; a seat takes 1.0 s |
| Launch | Armed by the captain's ABANDON SHIP (a held 3 s on the captain's console), the pod launches 10 s after its hatch seals; its red handle launches it at once |
| Ejection | 8 m/s straight out of the hull, then the beacon and station keeping |
| Without power | They launch: each pod has its own battery and a spring ejector |

A launched pod is a craft handed to the exterior frame (`ship-frames`) with the Tern's velocity plus its
ejection, and its crew are bodies in its interior frame; a mission ends lost if every body is aboard a
pod or downed.

### 2. The docking port

- The airlock's outer door becomes a docking collar: a mount `docking_port` on the starboard hull at deck
  C, its axis the door's normal, 2.0 m across.
- Docked and both sides at pressure: the outer door opens straight through. Docked with the other side
  at another pressure: the airlock cycles to match it, as for EVA.
- An enemy craft docked forces the collar (`security-station`: 20 s from outside), and the docking port
  is an entry point.

### 3. A second airlock

- On deck A aft of the command passage, a 2 x 2 x 2.4 m chamber with two pressure doors and EVA suit
  stowage for two, cycling by `life-support`'s rule (about 14 s out for its 9.6 m3 at the airlock's
  pump rate, measured when it is placed).
- It is the EVA airlock when the deck C airlock is docked, lost or cycling.

## Risks / Trade-offs

- **Pods make death cheap.** A launched pod ends the crew's part in the mission; it is an escape, not a
  respawn.
- **Space.** Pod bays take about 3 m3 each from rooms beside the hull; the layout patch says from where.

## Pi 5 budget

Three pod props (about 800 triangles each) on the decks; launched, one pod at a time in the exterior
scene's craft budget. The second airlock is one more small compartment with two lamps.
