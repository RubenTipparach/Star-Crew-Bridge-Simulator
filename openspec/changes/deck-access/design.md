# Design: deck access

Status: **proposed** (2026-10-06). Nothing here is built, and the layout does not hold it yet. The
proposal is a patch, `data/ships/tern/deck_access.json`, applied after the command suite's
(`command_suite.json`), written and checked by `tools/deck_access.py` and drawn by
`docs/mockups/deck-plan.html`. Every number below is from that tool on the layout patched with
both, unless the line says otherwise.

## Context

The owner, 2026-10-06: "how do the levels connect? can we get starways on the sides? elevators?",
and then "we need to install multiple ways to go up and down since various parts of the ship can
be damaged".

Damage works compartment by compartment (`damage-control`): a hit breaches, sets fire to, floods
with smoke or seals one compartment, and its doors close. A route through it is lost while it is
damaged. So a ship survives damage when no single compartment carries every route to somewhere
else.

## Goals / Non-Goals

**Goals:**
- More than one way between every pair of decks, through different compartments, with routes
  that fail in different ways: by power, by breach or fire, by a jammed door.
- A measured rule that one lost compartment cuts off no room that is not a dead end by nature.
- Stairs and a lift, as the owner asked, sized for the crew collider, low poly and cheap on the Pi 5.

**Non-Goals:**
- The rooms the routes pass through do not change their jobs. They give up the floor the trunks
  stand on (section 4).
- How a player climbs a spiral stair or calls the lift: `crew-on-deck`'s, once it has the fixture
  kinds (task 2.3).
- What the lift's power is worth to the grid: `power-grid`'s, with the load stated here.

## Decisions

### 1. How the decks connect, and what one loss cuts off

Today the routes between decks are:
- **Forward:** one ladder, 1.0 x 1.0 m on the centreline at z 11, from the command passage (A) to
  the main corridor (B) to the lower corridor (C);
- **Aft:** engineering's own stairs, from its catwalk (A) to its mezzanine (B) to its lower floor (C);
- **Between B and C only:** the hangar's two gallery stairs.

The instrument (`single_losses` in the tool) takes each compartment away in turn and counts the
rooms the bridge can no longer reach through crew portals (doors, hatches and ladders):

| Lost compartment | Today | With the command suite | With deck access | With deck access, the lift unpowered |
| --- | ---: | ---: | ---: | ---: |
| Command passage (A) | 28 | 29 | 0 | 0 |
| Main corridor (B) | 9 | 9 | 0 | 0 |
| Lower corridor (C) | 7 | 7 | 1, the ventral turret pod | 1 |
| Cargo (C) | 1, the airlock | 1 | 1 | 1 |
| Hangar | 2, the launch bays | 2 | 2 | 2 |
| Engineering | 1, the drive section | 1 | 1 | 1 |
| A turret access room | 1, its pod | 1 | 1 | 1 |

What is left is the **dead ends**: a turret pod behind its access room, the launch bays behind the
hangar, the drive section behind engineering, and the airlock behind cargo. Each has one way in by
its nature, a hatch to a turret or a door to a bay, and a second would weaken what the room is for.

### 2. Two stair towers

**Where.** One each side of the spine, x 1.25-3.85 m (port) and its mirror, z 9.0-11.6 m: beside
the old ladder, in the middle of the forward section, so a tower is about as far from the bridge as
the ladder is. A scan over z (a scratch instrument over `room_at` and the layout's systems, fixtures
and doors) found that stretch is where, on all three decks and on both sides at once, no system,
fixture or corridor door stands in a 2.6 m tower's way. Further aft the air handler and the spare
parts rack are in the way; further forward the side rooms' corridor doors and the thermal control
unit are.

**What it is.** A trunk compartment (kind `trunk`) from deck C's floor to deck A's ceiling,
2.6 x 2.6 m and 10 m tall: 67.6 m^3 of air each, open from top to bottom like engineering. Inside is
a spiral stair (fixture kind `spiral_stair`):
- a column 0.15 m in radius, and treads out to 1.2 m;
- per deck, 18 risers of 0.194 m (3.5 m), 17 treads sweeping 240 degrees forward from the landing;
- a landing at each deck, 1.1 m deep across the tower's aft end, and the treads arrive on the
  landing above.

It is compact, because a straight stair needs about 2.6 x 5 m on each deck, and that does not fit
here (section 2a).

**Light.** A trunk is lit at every deck, not once at its top. `deck-pipeline`'s lamp rule gives
lamps by floor area, which in a 10 m shaft of 6.8 m^2 is one high-bay lamp at the top: drawn that
way, the treads of decks B and C were near black (a mean baked light of 0.05, against 0.3 on the
floors of the rooms around them). So a trunk takes one lamp a deck: it hangs under the landing of
the deck above, lighting the flight that climbs to it, and the top deck's hangs at the ceiling. Every
third lamp is on the emergency bus, so each tower keeps one on emergency power. The lift's shaft
takes the same rule, at its centre.

**Doors**, all 1.0 x 2.2 m, on every deck's landing:
- one onto the spine corridor;
- one into the room beside it: the captain's quarters, the crew quarters and life support to port,
  and the computer core, the mess and cargo to starboard.

On deck A a third door, in the landing's aft wall, opens into the head (port) or the bridge locker
(starboard), so those rooms keep a way out when the passage is lost.

**Time.** A deck takes 3.4 s: the 240-degree walk line at 0.85 m radius at stair speed (0.7 x
1.8 m/s), plus the landing at a walk. A ladder takes 5.4 s up and 4.5 s down (`crew-on-deck`). Up the
port tower from the crew quarters' door to the bridge door is 14.0 s. The ladder takes 11.7 s,
because the quarters' door is beside the ladder. From deck C's corridor to the bridge door, a tower
takes 15.3 s and the ladder 15.9 s.

Walked in the deck plan's walk mode (`docs/mockups/lib/shipwalk.js`: crew-on-deck's capsule, its 0.35 m
step and its speeds, colliding with the treads as drawn), the port tower from deck B's landing round the
walk line to deck A's takes 3.34 s, against the 3.4 s estimated here. That checks the estimate in a mockup,
not in the game.

### 2a. Why spiral

| | Spiral, 2.6 x 2.6 m | Straight flights, about 2.6 x 5 m a deck |
| --- | --- | --- |
| Fits beside the spine | Yes, at z 9.0-11.6 on both sides | Only by moving the air handler or the thermal control unit, and four corridor doors |
| Rooms give up | 6.8 m^2 each | about 13 m^2 each |
| A deck takes | 3.4 s | about 5 s (estimated: two flights of 2.7 m along the slope at stair speed, and a landing) |
| Looks like | A ship's companionway: the sci-fi spiral in a round shaft | An office building's stair |

Recommendation taken (ask only with screenshots), Y2: spiral.

### 3. The lift

**Where.** Starboard, x -3.3 to -1.25 m, z 17.1-19.0 m, its door on the passage 2 m aft of the bridge
door: the bridge's lift, as R1's turbolift is the bridge's (`docs/analysis/star-trek-bridges.md`). It
stands between the briefing room's two doors, clear of both their zones, and its outboard wall meets the
backs of the briefing table's inboard chairs. The first lift, at z 17.6-20.0 m, stood 0.6 m behind the
briefing room's door from the bridge: walking the deck plan found it (2026-10-06), and the tool now checks
that no trunk stands in any door's clear zone, on any deck.
On deck B its door is 4.7 m from the medbay's, and on deck C it is near the magazine's.

**What it is.**
- **Trunk:** kind `trunk`, 2.05 x 1.9 m, 38.9 m^3.
- **Car:** fixture kind `lift`, 1.9 m deep from its door, 2.2 m high and 1.6 m wide: a 1.9 m stretcher fits lengthwise.
- **Doors:** 1.2 x 2.2 m onto each deck's spine corridor.
- **Movement (assumed until `power-grid` and `crew-on-deck` set them):** 1.5 m/s, and 2.0 s for its
  doors to open and again to close.

**What it is for.**
- **Casualties.** From helm's seat to a medbay bed, carrying a casualty, takes 20.3 s by the lift and
  34.8 s down the ladder.
- **Gear.** A spare part on deck C reaches deck A without a ladder.
- **Not speed.** For a crew member alone the lift (14.9 s from deck C's corridor to the bridge door)
  is no faster than a tower (15.3 s).

**How it fails.** It stops without power, and it is sealed with its trunk when the trunk is
breached or burning. The towers and the ladder cover both cases: the single-loss table holds with the
lift unpowered.

**Its load (assumed, for `power-grid`):** 15 kW while moving, on the forward distribution node that
feeds deck A.

### 4. Scuttles and second ways out

| Portal | Between | Where | Size m |
| --- | --- | --- | --- |
| `p_scuttle_ready` | ready room, medbay | hatch and ladder at (7.6, 19.2) | 0.9 x 0.9 |
| `p_bridge_scuttle` | briefing room, damage control | hatch and ladder at (-7.6, 19.2); `reference-ship-tern` T3 | 0.9 x 0.9 |
| `p_port_turret_quarters` | port turret access, crew quarters | the wall at z 8.0, x 5.5 | 1.0 x 2.2 |
| `p_stbd_turret_mess` | starboard turret access, mess | the wall at z 8.0, x -5.5 | 1.0 x 2.2 |
| `p_dc_torpedo` | damage control, torpedo room | the wall at z 26.0, x -4.5 | 1.0 x 2.2 |
| `p_shield_life_support` | shield room, life support | the wall at z 6.0, x 5.5 | 1.0 x 2.2 |
| `p_switchboard_cargo` | forward switchboard, cargo | the wall at z 6.0, x -6.3 | 1.0 x 2.2 |
| `p_life_support_magazine` | life support, magazine | the wall at z 20.0, x 5.0 | 1.0 x 2.2 |

**The scuttles.** They are how the bridge reaches deck B when the passage is lost: bridge, a side
room, a ladder down. The briefing room's lands in damage control, as T3 asked; the ready room's in
the medbay, so a casualty from the bridge has a second way there.

**Second ways out.** Each door is in a wall two rooms already share. Together they leave no room
whose only door is on a spine corridor, apart from the dead ends of section 1.

### 5. The rooms the trunks stand in

A trunk takes its footprint out of every room it crosses. The tool cuts each room into convex
brushes, splitting across the trunk's x span first so a room's long outer wall, and the doors and
windows on it, stay whole:

| Room | Air m^3, before / after | Floor m^2, before / after | Brushes |
| --- | --- | --- | --- |
| Captain's quarters (A) | 131.9 / 111.6 | 44.0 / 37.2 | 1 to 2 |
| Computer core (A) | 131.9 / 111.6 | 44.0 / 37.2 | 1 to 2 |
| Briefing room (A) | 201.3 / 189.7 | 67.1 / 63.2 | 4 to 6 |
| Crew quarters (B) | 271.5 / 251.2 | 90.5 / 83.7 | 1 to 3 |
| Mess (B) | 271.5 / 245.7 | 90.5 / 81.9 | 1 to 4 |
| Damage control (B) | 187.2 / 181.1 | 62.4 / 60.4 | 1 to 2 |
| Life support (C) | 367.8 / 347.5 | 122.6 / 115.8 | 1 to 3 |
| Cargo (C) | 349.1 / 317.1 | 116.4 / 105.7 | 3 to 7 |

The ship's air goes from 10,284.7 m^3 (with the suite) to 10,302.1 m^3. The trunks add the deck
slabs they pass through, 17.4 m^3.

Four suite pieces moved to stay clear, in `tools/command_suite.py`:
- the wardrobe;
- the server racks, which also move outboard;
- the head's and the bridge locker's locker banks, with the EVA suit fixture: helm now reaches the
  suits in 14.7 s, not 14.2 s;
- the briefing room's passage door, to z 16.4.

The suite's checks still pass, and so do this tool's on both patches together: the layout rules,
every piece of furniture inside its room and clear of every door, and no system or fixture inside
a trunk.

### 6. The Pi 5 budget this change spends

Counted with the kit on the patched layout (`node tools/mockups/kit_report.mjs --layout`, and the
kit's spiral and car builders, 2026-10-06):

| Part | Triangles | Note |
| --- | ---: | --- |
| A stair tower | 856 | shell and generated detail 352 (with three lamps, one a deck), panel dressing 124 more, the spiral 380 |
| The lift | 400 | shell and detail 216, panels 112, the car 72 |

That is against `deck-pipeline` section 11's 8,000 for a compartment. The three trunks add three
compartments, so three draws where they are in view. They need no new textures.

### 7. What applying the patch changes elsewhere

With `command-suite`'s task 2, in the same rerun:
- **`layout.json` and `layout_check.py`:** the compartment kind `trunk`, and the fixture kinds
  `spiral_stair` and `lift`, which the checker learns. It checks the treads inside their trunk and
  the landings against the doors.
- **`reference-ship-tern`:**
  - the deck maps;
  - section 2-3's tables;
  - section 6's routes;
  - T3, answered by the owner's direction.
- **`crew-on-deck`:**
  - climbing a spiral and riding the lift;
  - `tools/walk_times.py`'s routes, which can now go by a tower or the lift.
- **`damage-control`:**
  - the three trunks as compartments (fire load, a breach in a 10 m shaft);
  - ten more doors to seal and jam;
  - the scuttles as escape routes.
- **`life-support`:** three new volumes, and the rooms in section 5.
- **`power-grid`:** the lift's load.
- **`deck-pipeline`:** section 11's rows, and section 5's lamp rule gains the trunk's case (a lamp a
  deck, section 2).

## Risks / Trade-offs

- **A trunk spans three decks,** so smoke or a breach in it reaches every deck's landing. Its doors
  close, as every door does, and the other tower and the ladder stay.
- **Ten more doors and three more volumes** for damage control to manage. That is the point:
  more ways round.
- **A spiral is slower to carry a casualty up.** That is what the lift is for.
- **Rooms shrink** by 5-15 % of their air: the captain's quarters and the computer core most (15 %), life support least (6 %).

## Open questions

Per CLAUDE.md 13, a question goes to the owner only with something to look at. The shots are
`docs/screenshots/mockups/deck-plan-access-*.png`.

| Id | Question, and the fact it turns on | Options | Recommendation | Shot |
| --- | --- | --- | --- | --- |
| Y1 | The ways up and down as drawn: a spiral stair tower each side of the spine, a lift by the bridge door, scuttles from the ready room and the briefing room, six second doors. With them no single lost compartment cuts the bridge off from anything but a dead end; today losing the command passage cuts off all 28 rooms | As drawn / without the lift / towers at the ship's sides instead / other | As drawn | `deck-plan-access-exploded.png`, `deck-plan-access-towers-deck-C.png`, `deck-plan-access-port-tower.png`, `deck-plan-access-lift.png`, `deck-plan-access-deck-B-plan.png` |
| Y2 | Spiral stairs or straight flights in the towers (section 2a) | Spiral / straight | Spiral. Recommendation taken (ask only with screenshots) | none |
| Y3 | The lift's speed, door times and load: 1.5 m/s, 2 s each way, 15 kW while moving (assumed) | As assumed / set by `power-grid` and `crew-on-deck` | As assumed until those changes set them. Recommendation taken (ask only with screenshots) | none |
