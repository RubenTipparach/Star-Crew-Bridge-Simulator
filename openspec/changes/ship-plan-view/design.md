# Design: the ship plan view

## Context

The owner, 2026-10-08: "I kind of like the floor plan of the ship view as well. But we may need to
downscale some of the geometry on furniture for that view so it'll run on a pi. Otherwise having the
whole ship to explore. Or monitor players moving about on the ship would be cool." Later the same day: "any crew
member should be able to pull up a 3d map of the ship anytime as well to see where all their friends are."

| Decided elsewhere | Where |
| --- | --- |
| The Pi 5 frame budget: 200,000 triangles and 300 draw calls a frame, all passes; plan scenes at about half | `engine-stack` 5 |
| The deck file: compartments, one draw per compartment, the texture array | `deck-pipeline` 9, 13 |
| The mockup's views: whole ship, one deck in plan, exploded (decks 8 m apart), a cut at a height | `reference-ship-tern`, `docs/mockups/deck-plan.html` |
| Bodies in the snapshot, 10 bytes each at 20 Hz | `netcode-and-sessions` |
| Consoles are glance first: a picture, few words, colour with a shape | CLAUDE.md 10 |

## 1. The view

| Mode | Camera | Geometry |
| --- | --- | --- |
| Deck | Orthographic, from above, the bow to the right (the mockup's convention) | One deck, cut 1.6 m above its floor: walls stand to the cut, ceilings and everything above are not drawn |
| Exploded | Perspective, orbit and zoom | All three decks, 8 m apart, each cut the same way |
| Whole | Perspective, orbit | The ship as built, ceilings and all: the light models only |

The cut is a clipping plane per deck (the deck shader's existing clip, `deck-pipeline`), so no geometry is
rebuilt for it. Wall tops along the cut get a flat band in the room's plan colour (the mockup's `CAP_M`
band) so a wall reads as a line from above.

## 2. Two levels of detail for props

Every prop build (`hs_kit.py`, every set) writes, beside the prop, a **light model** in the same glb and the
same atlas (its UVs map into the full model's charts, so it costs no texture):

| Prop size (its bounds' longest side) | Light model budget |
| --- | --- |
| Under 0.6 m (a stool, a valve) | Not drawn in plan |
| 0.6-1.5 m (a chair, a console, a locker) | 24 triangles |
| 1.5-4 m (a bunk, a bank, a pump) | 40 triangles |
| Over 4 m (the reactor, a fighter, the Petrel) | 60 triangles, the Petrel and the reactor 200 |

It is made by the build, not by hand: the convex hull of the prop's pieces simplified to the budget with
Blender's planar and collapse decimation, checked closed and inside the full model's bounds. The deck
compiler stores both levels per compartment; the walk draws the light one beyond 15 m (and the full one
nearer), the plan view always draws the light one.

**The whole ship in plan, estimated** from the deck plan's counts (`ship-props` 5): shells and trims about
45,000 triangles, 420 props at their light models about 12,000, doors at a 12-triangle light leaf 1,400:
about 60,000 triangles, in one draw per compartment and deck slice (about 80). To measure on the Pi with
the probe's scene 6.

## 3. People on the plan

Each body is a marker at its position on its deck: a disc 0.5 m across with a wedge for its facing,

| Who | Marker |
| --- | --- |
| A player | Role colour (`bridge-stations`' role palette), a ring, the name above at the plan's text size |
| An NPC (`crew-npcs`) | Department colour, smaller, no name unless pointed at |
| Downed | The marker flashes at 1 Hz with a cross |
| In a suit, in zero gravity | A second ring |
| Seated | A square around the disc |

Fires, smoke, breaches and closed pressure doors come from the simulation as they do on the damage-control
console (`damage-control`): one picture, one source (CLAUDE.md 6.1). Markers are instanced quads: one draw.

## 4. Where it is seen

| Where | Mode | Shows |
| --- | --- | --- |
| The captain's crew page, the security station | Deck or exploded, tap a deck | Every body, every hazard; tap a marker to see who and what they are doing |
| The lobby, a spectator | Exploded, slowly turning | The players and the NPCs at work |
| On foot, the map (M, gamepad Back), any time | Exploded, orbiting; Tab steps to one deck | Everyone aboard: every player with their name, every bot crew member (`crew-npcs`), you marked; doors and hazards once the simulation has them |

## 5. The first version in the engine (2026-10-08)

Built before the light models (section 2), so it draws the full rooms and costs what the walk costs:

- **M toggles the map** wherever the player is (walking, flying, in the lift); the body keeps standing where it was,
  and the simulation (lift, bots) runs on. The mouse orbits, the wheel zooms; Tab steps through all decks, A, B and C.
- **Exploded:** the three decks 8 m apart, each drawn with its rooms cut 1.6 m above its floor by a clip height in
  the deck shader (a uniform, so nothing is rebuilt). A room that spans decks is cut at its lowest deck's cut, so the
  upper part of engineering is not shown in this version. The outside (sky, dock) is not drawn: the map is on a dark
  ground.
- **People:** a marker a body, a pillar 0.5 m across and 2 m tall in the role or department colour, its name drawn
  over it by the UI layer (`lobby` 3) for players and, smaller, for bots; you in white with a ring. One draw a marker
  in this version (instancing later).
- **Cost:** the whole ship at full detail, about 175,000 triangles in about 50 draws: inside the frame budget but
  not the plan's half of it (section 2's estimate assumes light models). Not measured on a Pi (CLAUDE.md 2).

## Risks / Trade-offs

- **Light models that look wrong from above.** A hull simplified to 24 triangles loses a chair's back.
  From above at plan scale it reads; the walk's 15 m switch is where it can show, so the switch distance is
  data and is judged on shots.
- **A second model per prop adds build time** (about a tenth of the bake: no new atlas).
