# Design: ship interactables

## Context

The owner, 2026-10-08: "I'd like more interactive stuff on the ship in general like toilets that you can
flush.. showers you can turn on. Ability to change lighting in bedrooms. RGB, dim lights, etc. I'll let
you come up with more ideas."

| Decided elsewhere | Where |
| --- | --- |
| Use is E or gamepad A, tap or hold; a body acts within reach; bindings are data | `crew-on-deck` sections 6 and 13 |
| The server owns every door, breaker and valve; a client sends intents | CLAUDE.md 6.3 |
| What a console previews is computed by the code that resolves it | CLAUDE.md 6.1 |
| Tuning lives in data with units in the keys | CLAUDE.md 6.5 |
| Props and their placements (the head's two toilets and two showers, the quarters' bunks, the galley counter) | `command-suite` 5, `ship-props`, `data/ships/tern/crew_rooms.json` |
| The deck shader's per-compartment uniform block (state weights, a dimmer, a flicker, four dynamic lights) | `deck-pipeline`, `light-baking` |
| Life support leaves water and waste loops out | `life-support` Non-Goals |

## 1. One model for every fixture

**A fixture** is a placed prop with one or more **use points**. A prop's use points live in its
`props.json` row, written by its Blender build (`uses_m`: a point in prop space, a facing, a kind),
so they move with the model and the placement rule never hand-places one (CLAUDE.md 8). A fixture's
**state** is a few bytes the server owns (`sc-core::fixture`): an on flag, a level 0-255, a colour as
RGB8, a timer. A Use is an intent naming the fixture; the server applies the kind's rule and the
snapshot carries the state; the client animates it (a lid, a water stream, a light).

| What | Rule |
| --- | --- |
| Reach | Within 1.2 m of a use point and facing it within 60 degrees (`crew-on-deck`'s reach) |
| Prompt | The fixture's name and its verb, one line, at the use point (CLAUDE.md 10: names, no explanations): "Toilet: Flush" |
| Tap | The kind's verb (flush, on/off, open/close) |
| Hold (0.4 s) | The kind's panel, when it has one (the lighting panel): glance-first, at most four controls |
| Who | Any body, a player's or an NPC's (`crew-npcs`): the same intent, the same rule |
| Network | State changes only: 1 byte id class + 2 bytes id + up to 5 bytes state, in the reliable channel (`netcode-and-sessions`) |
| Saves | A fixture's state is saved with the ship (CLAUDE.md 7: a ship persists) |

## 2. The owner's three

**Toilet.** Tap: flush. 3.0 s of sound and a swirl in the bowl (a turning texture on the water
disc), 6 L from the potable tank to the grey tank. The lid is down while nobody stands at it and up
when a body uses it. A toilet cannot be flushed again for 4 s (it refills). On no water: a dry
gurgle, nothing moves.

**Shower.** Tap: on or off. While on: a cone of water (a few dozen particles a frame, sprites, not
geometry), its sound, steam that fogs the stall's glass over 20 s and clears over 40 s after (a
uniform on the glass), and a wet floor for 60 s (a darker floor tint, a uniform). It draws 8 L/min
from the potable tank to the grey tank and 4 kW for the heater from its bus. Hold: the panel, with
temperature (cool, warm, hot: the sound and the steam) and a timer (off after 5 min). It turns off by
itself when the potable tank is empty or its bus is dead.

**Room lighting.** A panel by each cabin door (the quarters, the captain's quarters, the ready room,
the head, the medbay) and the room's light switch. Tap: lights on or off. Hold: the panel:

| Control | Range | Default |
| --- | --- | --- |
| Dimmer (a lever) | 5-100 % | 100 % |
| Colour (a ring of 12 swatches and white) | Any RGB8 | White |
| Presets (four buttons) | Work (white, 100 %), Relax (warm 2700 K, 50 %), Night (deep red, 10 %), Off | Work |

The baked light of the compartment's own lamps is multiplied by `tint * dim` in the deck shader, a
per-compartment uniform beside the dimmer it already has; emissive faces (screens, strips) are not
tinted. Light that came through a door from a neighbour is baked into the same colour, so it tints too;
that is the accepted cost of not baking the lamps separately (section 5). Red alert overrides a room's
setting while it holds, and the room returns to it after. The power a room's lamps draw scales with the
dimmer.

## 3. More, proposed (recommendation taken: ask only with screenshots)

| Fixture | Where | Tap | Hold | Sim |
| --- | --- | --- | --- | --- |
| Basin, sink | Head, galley, medbay | Water on for 5 s | | 1 L |
| Galley dispenser | Mess | Coffee: a mug in hand after 4 s | Choose: coffee, tea, ration tray | 0.3 L, 1.5 kW for 4 s |
| Mug, tray | Carried (`crew-on-deck` 6) | Drink or eat (a few seconds); set down on a table | | A full body heals a little faster while fed and rested (`crew-on-deck` 7, proposed) |
| Locker | Quarters, locker room, armory | Open or close | | Holds items; armory lockers lock (security) |
| Bunk curtain | Quarters | Open or close | | Privacy; a closed bunk is where an off-watch NPC sleeps |
| Window shutter | Every window | Open or close | | Closes by itself on red alert and on a hull breach next to it |
| Light switch | Every room's door | Lights on or off | | The room's lamps off: dark rooms for drills and ambushes |
| Intercom | Every room | Talk to the bridge (push to talk) | Pick a room | `netcode`'s voice, proximity off |
| Radio | Quarters, mess | Music on or off | Station, volume | Sound only |
| Medbay bed | Medbay | Raise or lower | Back up or down | `medical-officer` |
| Mess table screen | Mess | A card game for up to four | | Nothing |
| Folding seat | Petrel cabin | Fold up or down | | Cargo space (`ship-props` 4h) |
| Plant | Mess, ready room | Water it | | 0.2 L; it wilts in 3 days without |
| Pressure door, panel | Airlock, bays | Already `crew-on-deck` 5 | | |

## 4. The water loop

`life-support` left water out. This change adds the smallest loop that makes the fixtures honest:

| Tank | Capacity | Starts | Refilled by |
| --- | --- | --- | --- |
| Potable | 2,000 L | 1,800 L | The recycler, from grey, 2 L/min at 3 kW, while grey holds water and its bus is live |
| Grey | 1,000 L | 100 L | Every basin, shower and toilet |

The oxygen generator's water (`life-support`, 0.036 kg per mol) is drawn from the potable tank. The
engineering console shows the two tanks as fills; nothing else about water is on a console. The tanks
are `life-support` systems in `layout.json` (its next patch), placed in life support's room.

## 5. The Pi 5 budget

- **Geometry**: animated parts (a toilet lid, a shower head's water, a curtain, a shutter, a seat pan) are
  small separate meshes like the door leaves, drawn only within 15 m of the camera: under 40 a room.
- **Particles**: water and steam are sprites, at most 200 alive in a view, one draw.
- **Lighting**: a vec4 tint per compartment in the uniform block (16 bytes each; 64 compartments, 1 KB).
- **State**: about 200 fixtures on the Tern, 8 bytes each: 1.6 KB in the snapshot's reliable store.
- **CPU**: fixture rules run at the server's tick only when used; the water loop is one step a tick.
Measured nowhere yet; a cloud session does not measure the Pi (CLAUDE.md 12).

## 6. Order of work

1. `sc-core::fixture`: kinds, states, rules, the water loop, with tests (a flush moves 6 L, a shower on
   an empty tank turns off, a room tint survives red alert).
2. `uses_m` in the head's, quarters' and galley's props; the deck plan's walk acts on them (the mockup).
3. The engine: the room tint uniform, the panels, the animated parts, sound.

## Risks / Trade-offs

- **A room's tint also tints light it received from a neighbour** (through an open door), because each
  compartment's baked colour holds all the light that reached it. The corridor's own colour is untouched:
  it has its own uniform. Accepted.
- **Scope creep**: every fixture is cheap alone; together they are a lot of animation and sound. The
  three the owner named come first; section 3's list is done in the order a playtest asks for.
