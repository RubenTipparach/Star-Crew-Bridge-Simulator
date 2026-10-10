# Design: extinguishers and first aid kits round the ship

## Context

`crew-on-deck` 6 gives the extinguisher (9 kg, one hand, 6 kg of agent over 15 s in a 30 degree cone reaching 3.0 m,
refilled at damage control in 10 s) and proposed 24 brackets in `kit.json` (section 15). `crew-on-deck` 7 (the
owner's rule of 2026-10-09) makes a medkit the thing that stops an incapacitated body's vitals falling: any body
holding one stabilizes in 5.0 s for a dose. `medical-officer` 2 gives the medic a 20 dose bag. `damage-control` 3
says a fire caught in its first 90 s is one extinguisher's work, by 120 s two. The layout and `deck_access.json` give
every room's walls and doors and every deck's walk graph.

## 1. A safety point

- **What it is.** On one wall bay, side by side 0.5 m apart: an **extinguisher** in a wall bracket, its handle at
  1.1 m; a **first aid cabinet** (0.45 x 0.35 x 0.14 m, white, a green cross), holding one first aid kit; above
  them a **sign** 0.30 x 0.15 m at 1.9 m, red flame and green cross, photoluminescent: it glows in the dark, through
  red alert and under smoke (an emissive mask, `light-baking`'s emission alpha).
- **Glance first.** Nothing to read: the sign is two pictures. An empty bracket or an open, empty cabinet is the
  state, seen from across the room.

## 2. Where they go: one rule

`tools/safety_points.py` places them from the layout, deterministically, in this order:

1. **Inside every room's main door** (the room's widest door to a passage): on the wall beside the door, on the latch
   side, 0.4 m from the frame, so a crew member running in to a fire or a casualty takes them on the way. A room
   whose main door wall has no clear bay takes the nearest clear bay within 2 m.
2. **Big rooms** take another for every 80 m^2 of floor beyond the first 80 (engineering, the hangar, cargo), on the
   wall bay farthest by walk from the points already placed.
3. **Passages and the rest**: while any walkable floor point on a deck is more than **10 m's walk** from a safety
   point (by the deck's walk graph, `deck_access`), place one on the passage wall nearest that point.
4. **Never** in a doorway's swing or slide, in front of a console, a hatch, a ladder or a fixture, or on a hull
   window; never in an airlock, a turret pod or an escape pod (they have their own kit).

The tool reports the count, every point's position, and the longest walk to a point on each deck. Expected on the
Tern: about 30 points (the tool's count is the number; this is an estimate).

**Plus what was there:** the damage control locker keeps its six extinguishers (and refills any extinguisher in
10 s); the medbay cabinet keeps the medic's bag and refills first aid kits in 4 s.

## 3. The first aid kit

| Item | Mass | Hands | Doses | Its action | Refill |
| --- | ---: | --- | ---: | --- | --- |
| First aid kit | 3 kg | One | 3 | Stabilize an incapacitated body: hold Use within 1.2 m for 5.0 s, one dose (`crew-on-deck` 7). Gives no HP. | At the medbay cabinet, 4 s; a kit put back in a cabinet empty stays empty |
| Extinguisher | 9 kg | One | 15 s of agent | `crew-on-deck` 6; where it lands, `fire-spread` 5 | At damage control, 10 s |

- **Taking and returning.** Use on a bracket or cabinet takes its kit (0.5 s); Use with the same kind of item in hand
  puts it back (0.5 s). Kits are items (`crew-on-deck` 6): carried, dropped, persisted with the ship (CLAUDE.md 7).
- **Bots.** NPC crew and damage control teams take kits from the nearest point by walk, the same rule as players
  (`crew-npcs`); the medic bot uses its bag.
- **Restock.** Between missions every point is restocked; during one, only the refill stations refill.

## 4. Data and checks

`data/ships/tern/kit.json` (written by the tool, never by hand):

```json
{
  "schema": "starcrew.kit/1",
  "items": {
    "extinguisher": { "mass_kg": 9.0, "hands": 1, "agent_kg": 6.0, "discharge_s": 15.0, "refill_at": "damage_control", "refill_s": 10.0 },
    "first_aid_kit": { "mass_kg": 3.0, "hands": 1, "doses": 3, "refill_at": "medbay", "refill_s": 4.0 }
  },
  "points": [
    { "id": "sp_quarters_1", "compartment": "quarters", "deck": "B", "rule": "main_door",
      "wall_point_m": [0.0, 0.0, 0.0], "normal": [1, 0, 0], "holds": ["extinguisher", "first_aid_kit"] }
  ],
  "lockers": [
    { "id": "dc_locker", "compartment": "damage_control", "holds": { "extinguisher": 6 } }
  ]
}
```

`tools/layout_check.py` validates every point: inside its compartment, on a wall, clear of every door opening by
0.4 m, of consoles, hatches and ladders, and the 10 m walk rule met on every deck. Unknown keys stop the check.

## 5. Props and drawing

- **Models** (`tools/blender/build_safety_props.py`, the `blender-hard-surface` kit): the extinguisher (a cylinder
  with a domed top, the valve, the hose and horn), its bracket (a strap and a back plate), the cabinet (a box with a
  chamfered door and a cross inset) and the sign. Budgets: extinguisher 120 triangles, bracket 40, cabinet 48, sign 2.
  Textured from the materials array (`surface-materials`: a red paint and a white enamel, the sign's glow in alpha).
- **Mockups**: shipkit (or propkit) draws every point from `kit.json`; `deck-plan.html` and `fire.html` show them, and
  `fire.html` uses them (take the extinguisher from the quarters' point).
- **Engine**: `deckc` writes the points into the deck as static props (the bracket, cabinet and sign), and the
  extinguisher and kit as item entities that can be taken (`crew-on-deck` 6). An empty bracket is the bracket alone.

## 6. The Pi 5 budget

| Item | Cost | Against |
| --- | --- | --- |
| Static props | About 30 points x 90 triangles (bracket, cabinet, sign) = 2,700 triangles, in the deck's own batches: no draw call added | Deck triangles |
| Items | About 30 extinguishers and 30 kits, 120 and 30 triangles, one instanced draw per kind in view | 2 draw calls |
| Server | 60 items with a pose and a holder; the dose and agent counts, 8 bytes each | Negligible |
| Network | An item's take, drop and refill are reliable events of about 8 bytes | Negligible |

## Risks / Trade-offs

- **Too many signs make a room busy.** One point per room at its door, and passages only where the 10 m rule needs
  them; the tool's report lets the count be judged before any prop is built.
- **Kits everywhere make injury cheap.** A kit only stabilizes: the carry to the medbay and the bed are still the
  cost, and three doses run out in a bad fight.
