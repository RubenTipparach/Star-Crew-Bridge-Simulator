# Design: the walking minimap

## Context

The owner, 2026-10-17: "I need to be able to see a map of where I am at all times on the ship when I'm away from a sit
station".

| Decided elsewhere | Where |
| --- | --- |
| The ship map: M (gamepad Back) any time on foot, the decks pulled apart, a figure and a name for every body; its deck view is orthographic from above, the bow to the right | `ship-plan-view` 1, 4, 5 |
| The map's markers: a disc with a wedge for facing, role or department colour, a square when seated, a cross flashing at 1 Hz when down | `ship-plan-view` 3 |
| Damage on the map: an amber diamond for a damaged system, a room's floor hatched red at a breach; fire as the burning cells | `ship-plan-view` 6, `fire-spread` 4 |
| The walking HUD: the objective card top left, the alert chip top right at alert only, the centre dot, the prompt plate, the first-time card at the middle left | `ui-art-direction` 11, Q6 (approved) |
| The 1920 x 1080 reference frame, anchors, the 96 x 54 px safe area, four type sizes, the palette | `ui-art-direction` 3 to 5 |
| The chat on foot: the last three lines at the bottom left | `crew-chat` 5 |
| The touch layer on foot: MAP top left, MENU top right, USE, JUMP and DOWN bottom right, the stick in the left 40 % below 15 % | `touch-controls` 3, `data/input/touch.json` |
| Deck A is the command suite in the compiled deck, not `layout.json`'s older rooms | `command-suite`, `deck-access` (the patches the deck build applies) |
| Consoles and HUD are glance first: a picture, few words, colour always with a shape | CLAUDE.md 10 |

## 1. When it shows

| The player is | Minimap |
| --- | --- |
| On foot, anywhere aboard (walking, on a ladder or stair, in the lift, carrying an extinguisher or a person) | Shown |
| Sitting down or standing up (the console rising or going down) | Fades out over the move (160 ms, `panel_in`), back when standing |
| Seated at a console | Hidden: the console takes the screen (CLAUDE.md 10) |
| The full ship map, the menu (Esc), the briefing or the debrief open | Hidden |
| Flying a fighter, or outside the ship | Hidden |

It is on by default. Options keeps one switch for it (open question M3).

## 2. Size and place

All sizes in reference px of the 1920 x 1080 frame, scaled with it (`ui-art-direction` 3).

- **The panel**: 384 x 256 px, opaque (`panel`), one hairline (`line2`); a band 40 px tall at its top; under the band
  the map, 384 x 216 px on the `ground` colour.
- **On the desktop**: anchored bottom right, in the safe area: x 1440 to 1824, y 770 to 1026. Nothing else lives there
  on foot: the objective card is top left, the alert chip top right (54 to 118), the first-time card middle left (x 96
  to 556, y 388 to 692), the prompt plate at the bottom centre (about x 760 to 1160, y 640 to 800), the chat's three
  lines bottom left (`crew-chat` 5).
- **On a phone** (touch is the last input used): the minimap takes the MAP button's corner and is the button. 168 x 112
  px of `touch.json`'s 390 px frame (`buttons.mini_px`, new), 12 px from the top and left edges (`corner_edge_px`, as
  MAP's): about 0.44 of the reference size, a little larger than the desktop's share of the screen, with the band 52 px
  and the room's name 30 px (reference) so the name is about 13 px tall on a 390 px screen. The room name the touch
  layer showed beside MAP moves into the band. MENU stays top right; USE, JUMP and DOWN bottom right.
  - It takes a finger before the stick, as MAP does today. It covers x 12 to 180, y 58 to 124 of the stick's zone (the
    left 40 % below 15 %): on an 844 x 390 phone 168 x 66 px of the zone's 338 x 332, about 10 %, at its top corner where
    a thumb seldom lands.
  - The objective card on a phone hangs to the right of the minimap (x = its right edge + 8 px) instead of at the safe
    corner, where today it would sit under MAP (finding F3).

## 3. The picture

Glance first: one picture, two words at most (the room), one letter (the deck).

- **The view**: the deck the feet are on, from above, the bow to the right and port up: the ship map's deck view
  (`ship-plan-view` 1), so the two maps never turn against each other. **12 px a metre**: the map shows a window 32 m
  along the ship by 18 m across that follows the body, centred on it. It does not turn with the player: the arrow does
  (open question M1). A deck is about 75 m by 27 m (deck B), so the window is about a third of its length: the whole deck
  is the full map's job.
- **Rooms**: every room's floor filled `raised` (#141d29) with its walls a 2 px `line2` edge; **the room you are in**
  lighter (#22324a); stair towers and the lift shaft hatched. Doors are gaps in the wall, the door's width (at most
  1.2 m).
- **Ways off the deck**: one glyph each, 22 px, a round plate (a square one for the lift) with a chevron: up, down, or
  both for a ladder, stair or lift that goes both ways. The deck's ladders and hatches (the walk's `ladders` and
  `hatches`), the stair towers, the lift (`lifts`), and the hatches up into the turret pods.
- **People** (`ship-plan-view` 3's markers, at 6 px): every crew member aboard a disc in their role colour (a player)
  or department colour (an NPC) with a wedge for their facing; a square round a seated one; a red disc with a white
  cross, flashing at 1 Hz between full and 35 %, for one who is down.
- **You**: an arrow of your facing, 30 px long, in the ship map's colour for you (255, 184, 77, `main.rs` `draw_map`),
  outlined in `ground`, with a ring. A different shape from the crew's discs, because engineering's role colour is an
  amber close to it (CLAUDE.md 10: colour always with a shape).
- **Hazards**, under the people, each the shape it has on the other screens:
  - fire: the flame glyph in the fire colour over a soft disc, at each burning room's flames (`fire-spread`; the
    engine's `FireFx`);
  - a damaged system: the amber diamond (`ship-plan-view` 6, Systems);
  - a breach: the room's floor hatched red (`ship-plan-view` 6, Rooms);
  - someone down: their marker as above (once `medical-officer` gives the drill a downed body).
- **The band**: the room's name in Label (20 px, caps, `text`), squeezed to fit, never under the deck's letter; the
  deck's letter in a small box at the right; on a phone the map icon at the far right.
- **Red alert or emergency power**: the panel's edge turns the state's colour, 3 px (the alert chip's pen). Nothing else
  changes: the alert chip says the word.
- **No numbers, no compass, no legend.** Exact names come in the full map.

## 4. Seen beside everything else

The mockup shots show it with the first-time card up (no overlap, `ui-art-direction-minimap-after.png`), at the short
browser embed of 985 x 455 (`ui-art-direction-minimap-after-985x455.png`, where it scales with the rest of the HUD), and
on the phone beside the stick, USE and MENU (`touch-controls-minimap*.png`).

## 5. Up and down

The minimap shows the deck the feet are on: `NavGrid::deck_of` on the body's feet, the walk grid's own rule (the map's
`deck_of_y` in `main.rs` and `sc-tools`' `deck_of` are copies of it with other margins: finding F1; both move onto the
one rule). It switches when the body steps off a ladder, a stair or the lift car onto another deck's floor; while
climbing it shows the deck the climb started from, so it does not flicker mid ladder. A switch crossfades in 160 ms
(`panel_in`). In a turret pod (a floor above or below a deck that a ladder reaches) it shows the deck the pod's ladder
leaves from, the arrow at the ladder.

## 6. To the full map

A click on it with the mouse, or a tap on a phone, opens the ship map exactly as M (or the MAP button) does, on the deck
you are on (`MapView::deck` set to it; Tab still steps through the decks); closing the map brings the minimap back. M
and the gamepad's Back are unchanged. On a phone the minimap is the MAP button, so the one tap target opens the map.

The minimap only asks for the map: what the map shows when opened is the ship map's work. Today the drill's touch MAP
toggles the bridge overview (`drill.rs` `Btn::Map => self.overview`), not the ship map (finding F2): the owner's report
"Map button on mobile isn't pulling up the map", fixed separately; the minimap inherits that fix.

## 7. One picture, one source

Nothing here is a second copy of the ship (CLAUDE.md 6.1, 8):

| What | From | Not from |
| --- | --- | --- |
| The deck's floor and its rooms | The compiled deck the body walks: the walk grid's cells of that deck (`sc_core::nav::NavGrid`, built in `onfoot.rs` from `deck.walk` and its triangles; 0.5 m cells) for where the floor is, and the compartment whose drawn floor is under each cell (the floor triangles `firefx::open_floors` already reads, every compartment's, since deck A's rooms are drawn as the command suite's) for which room it is | `layout.json` alone (deck A would be its older rooms) |
| Room names | The compiled deck's compartments (`DeckCompartment::name`) | |
| Doors, ladders, hatches, lifts | The compiled walk's entities (`DeckWalk::doors`, `ladders`, `hatches`, `lifts`); stairs from `NavGrid::stairs_of` (the command suite's stairs) | |
| People | The ship map's bodies (`main.rs` `draw_map` builds them: every bot's feet, yaw, department and name, and you), moved into one function both maps call | |
| Fires | The burning rooms the drill already sends (`FireFx::step`'s `burning`) | |
| Damage, breaches | The damage state the damage control console reads (`damage-control`), when the snapshot carries it | |

Built once per deck at load: each deck's cells traced into closed outlines per compartment (marching squares on the
cell grid, then merged collinear edges), kept as paths in reference metres; drawn each frame with the console vector
painter (`vg`) clipped to the panel, the window's offset and scale applied. The building is one function in `sc-core`
beside the walk grid, so a test can check it without a GPU.

The mockups do the same with the layout and the patches the deck build applies (`ShipKit.applyPatch` with
`command_suite`, `deck_access`, `engineering`, `armory`, `exterior_windows`), in one `docs/mockups/lib/minimap.js`.

**Considered and not taken**: the ship map's own 3D draw in a corner (its deck mode, orthographic). It would be one
more pass of a deck's rooms every frame (about 60,000 triangles and 17 draws, a third of the map's 175,000), and textured
floors and furniture at 12 px a metre read as noise. A flat plan reads in a glance.

## 8. Cost

- On foot: one UI batch (the HUD's existing one), a few hundred vertices: about 30 room outlines, 10 glyphs, 10 markers.
  No new draw call, no render target, nothing in 3D.
- At load: the outlines from the walk grid, a few thousand cells a deck: well under the grid's own build time.
- Network: nothing new (positions and fires are in the snapshot already).
- Memory: the outlines, a few kilobytes.

## 9. The mockups

- `docs/mockups/ui-art-direction.html`, page "Minimap (proposed)": deck A on the bridge with the first-time card (shot
  `minimap-after`), deck B in the main corridor at the ladder (`minimap-deck-b-after`), red alert with a fire next door, a
  damaged core and someone down (`minimap-red-after`), and the smaller sizes (`minimap-after-1280x720`,
  `minimap-after-985x455`). The worlds behind are engine captures.
- `docs/mockups/touch-controls.html` with `?mini=1` or the menu's "Minimap (proposed)": the phone in the command passage
  (`minimap`), on the bridge by the helm (`minimap-bridge`), at red alert (`minimap-red-alert`). Off by default, so the
  approved layer's own shots are unchanged.
- The contact sheet: `docs/screenshots/mockups/walk-minimap/contact-sheet.png` (the six, and each minimap enlarged).

## 10. Checks (when built)

- `sc-core`: a deck's outlines from the walk grid cover every walkable cell of that deck and no other; deck A's rooms
  are the command suite's (the briefing room is there, the old ready room's shape is not); `deck_of` is one function the
  map and the minimap call.
- `sc-client`: the minimap is drawn on foot and not seated; a click inside it opens the map on the current deck; the
  release check (`release.rs`) holds it clear of every other HUD element at 1280 x 720 and 1920 x 1080 and on the phone
  layout; the parity check against the mockup's states.
- Captures in `docs/screenshots/engine/walk-minimap/`, from a walk on all three decks and up a ladder (CLAUDE.md 12).

## Findings

| Id | Finding | Where |
| --- | --- | --- |
| F1 | Three copies of "which deck is this height on": `NavGrid::deck_of` (margin 0.2 m), `main.rs` `draw_map`'s `deck_of_y` (0.3 m), `sc-tools` `safety_points::deck_of`. Reported, not changed here (CLAUDE.md 6.1) | `nav.rs` 259, `main.rs` 822, `safety_points.rs` 531 |
| F2 | The drill's touch MAP toggles the bridge overview, not the ship map; the ship map lives only in the tour client (`main.rs`) | `drill.rs` 1025 |
| F3 | On a phone the objective card (top left at 96, 54 reference px, about 35, 19 px on a 390 px screen) sits under the MAP button (12 to 60 px) | `hud.rs` `objective_card`, `touch.rs` `place` |
| F4 | Engineering's 3D schematic draws the rooms from `layout.json` without the patches, so its deck A is the older rooms | `console/schematic.rs` `build` |

## Open questions

Each with its screenshot; asked in the next survey with them.

| Id | Question | Recommendation | Screenshot | Status |
| --- | --- | --- | --- | --- |
| M1 | Does the map turn with the player (your facing always up), or stay bow right with the arrow turning? | Bow right: it matches the full map and the consoles' plans, and a room stays where it was as you turn | `docs/screenshots/mockups/ui-art-direction/ui-art-direction-minimap-deck-b-after.png` | Recommendation taken (ask only with screenshots) |
| M2 | Desktop bottom right, and on a phone in the MAP button's corner (the minimap is the button)? | Yes: the desktop's bottom right is empty on foot, the bottom left is the chat's; a phone's bottom corners are the stick and USE | `docs/screenshots/mockups/walk-minimap/contact-sheet.png` | Recommendation taken (ask only with screenshots) |
| M3 | Always on, or a switch in Options? | On by default, one switch in Options (Interface) for a player who wants the screen clear | `docs/screenshots/mockups/ui-art-direction/ui-art-direction-minimap-after.png` | Recommendation taken (ask only with screenshots) |
| M4 | A window of 32 m by 18 m around you, or the whole deck? | The window: at the panel's size a whole deck (75 m) is 5 px a metre and a corridor 15 px wide; the whole deck is the full map | `docs/screenshots/mockups/touch-controls-minimap.png` | Recommendation taken (ask only with screenshots) |
| M5 | You as an amber arrow (the full map's colour for you) next to engineering's amber discs? | Keep the map's colour and tell them apart by shape (arrow and ring against disc and wedge) | `docs/screenshots/mockups/ui-art-direction/ui-art-direction-minimap-after.png` | Recommendation taken (ask only with screenshots) |
