# Design: repair mini-games, and the medic's treatment

## Context

The owner, 2026-10-08 (the proposal quotes them in full): a different mini-game for every component that needs
repairs, mockups of them in the browser, and "a medbay minigame for healing crew, someone needs to be medical".

| Decided elsewhere | Where |
| --- | --- |
| Repairs are in person, with a kit, at the system's repair point; nothing is repaired from a console | `damage-control` 6a |
| An officer (every player) restores 1.8% of integrity a second, a rating 0.6%; two together 1.6 times the faster | `damage-control` 6a |
| Disabled (under 25%) needs a spare part, destroyed needs a rebuild (180 s to 25%) | `damage-control` 6 |
| `damage::repair_time(job, who)` is the one rate; the board's preview is it | `damage-control` 6a, CLAUDE.md 6.1 |
| The medic's rates: revive 2.0 s, field healing 4.0 HP/s to 75 HP, beds 6.0 HP/s tended, stabilize 3.0 s, 20 doses | `medical-officer` 2 |
| Consoles are glance first: a picture, few words, colour with a shape, the main action biggest | CLAUDE.md 10, `bridge-stations` 8.0 |
| Every input device drives every screen | CLAUDE.md 10 |
| Bots (NPC crew) repair at a rating's rate | `crew-npcs` 3 |

## 1. One rule for every mini-game

A repair job (`damage-control` 6) is cut into **steps**, three to six by system (section 2). A step is one round of
that system's mini-game.

- **Time.** The job's time at the repairer's rate, `damage::repair_time(job, who)`, is shared out over its steps.
  A step's game is tuned so a steady hand finishes it in about that share; finishing sooner does not finish the
  repair sooner: the step's progress bar fills at the rate, and the game's next step waits for it. So a player can
  never repair faster than the rate the board promised, and the board stays true (CLAUDE.md 6.1).
- **Fumbles.** A mistake (a wrong wire, a dropped coil, a spike past the line) costs **5% of the job** (data,
  `damage.json` `repair.fumble_share`) and that system's own hazard, said in section 2: a spark that stings for
  5 HP, a puff of coolant, a heat spike in the reactor. Three fumbles in one step and the step restarts.
- **Interruptions.** Taking a hit, being pushed or walking away pauses the game where it is; the work done stays
  done. Under fire the screen shakes with the ship (`ship-frames`' lurch), which is the game getting harder by
  itself, not a rule.
- **Two hands.** Two players on one job each play their own steps from the same list, and the job's rate is
  `damage-control`'s 1.6 times; a bot helping a player adds its rating's share with no game.
- **No player, no game.** A bot or a damage control team repairs at its rate with no game and no fumble: a player's
  game is how a player earns the officer's 3 times, not a tax on the ship.
- **Parts and rebuilds.** A disabled system's first step is fitting the part (the part's own little step in each
  game, section 2); a destroyed one's rebuild is its own longer game (the same game with every step, then the kit).
- **The screen.** One screen, 1280 x 720 like a console: the system's picture in the middle, the job's bar along the
  top (integrity now, where the step will take it, the time left at your rate), the step dots, and the one action.
  No instructions on the screen at rest (CLAUDE.md 10); the first time a player meets a game, a ghost hand shows the
  move once.
- **Input.** Mouse, touch and pad: every game is pointing and one button, or a stick and a trigger. Keys mirror the
  pad. Nothing needs a fast double-click.

## 2. The mini-games

| System (where) | The game | A step | Fumble and hazard | When it is down |
| --- | --- | --- | --- | --- |
| **Gravity generator** (engineering, deck C) | Three field rings turn at their own speeds; stop each so its bright arc lines up with the marker, outer to inner | One ring set | Released off the mark: the field lurches, everyone nearby floats for 2 s | Section 4: everyone floats, slowly |
| **Galley synthesizer** (mess) | A recipe card shows four nutrient columns; open valves to fill each column to its band without spilling over, then purge | One recipe balanced | Overfill: a splash of paste, the step's column drains | Section 4: hunger between missions |
| **Life support scrubbers** (life support) | Swap the spent CO2 cartridge for a fresh one (drag out, drag in), then trim three gas valves until O2, N2 and CO2 sit in their green bands | One cartridge, or one bank trimmed | Cartridge dropped, or a valve left in the red: a hiss, the room's CO2 rises by the share | `life-support`'s clock: the air goes bad |
| **Fighters** (hangar, a Swift on its cradle) | Lift the panel, then match the avionics plugs to their sockets by shape and colour stripe; torque the panel's fasteners in a star order | One panel: plugs, then fasteners | A plug forced into the wrong socket: a spark (5 HP), the plug bent | The fighter cannot launch |
| **Shuttle** (hangar, the Petrel) | Fuel line pressure test: close valves in order along a branching line to isolate the leak, then patch it and hold the pressure needle in band for 3 s | One branch isolated and patched | Wrong valve: fuel mist, the bay's fire risk up | The Petrel cannot launch |
| **Reactor core** (engineering) | Keep the plasma ball centred in the containment ring by trimming four magnet coils (one stick or four sliders) while a fifth coil is swapped out; it drifts with the core's load | One coil swapped while held centred | The plasma touches the wall: a heat spike, the core's temperature jumps | `power-grid`: the ship runs on batteries |
| **Impulse engines** (drive, the two impulse units) | Injector timing: pulses run along a scrolling trace; tap as each crosses the firing line, the rhythm set by the unit's tune | Eight injectors timed | Off the line: a misfire, a soot cough, the unit's heat up | Half thrust per unit down (`flight-and-navigation`) |
| **Warp pylons** (outside, EVA only) | On the hull, tethered: clip from handhold to handhold to the damaged coil segment, unbolt it in a pattern, slide the new one in against a drift | One segment changed | Unclipped too long: the tether snaps you back; a dropped bolt is lost | Section 5: no warp; not repairable in combat |
| **Toilets and showers** (crew quarters) | Rounds alternate. A toilet: plunge the clog down the trap in a safe rhythm, then flush. A shower: rotate the deckhead pipe tiles so one run joins the main to the shower, then open the valve | One toilet unclogged and flushed, or one shower running | Plunging too fast or flushing a clog: overflow, the floor is wet. Opening the valve on an open run: a loose joint, you are soaked (a moodlet) | Section 4: comfort and morale |
| **Medbay biobeds** (medbay) | Sensor calibration: tune two knobs until the bed's trace lies on the reference trace | One sensor channel | Over-driven: the bed alarms; nothing hurt | Beds heal at the unpowered rate |
| **Twin pulse cannon** (each turret's access room) | Focus the emitter: align two lenses so the test beam's spot is smallest and centred, then reseat the capacitor bank's coupling (drag it home at the right speed) | One lens pair, or the coupling | Coupling slammed: the bank arcs (10 HP) and loses its charge | The turret does not fire |
| **Gannet tubes and hoist** (magazine) | The tube's breech: clear the jam by rotating the locking ring through its notches, then run the load-and-arm sequence (guide rail, latch, interlock) in order | One tube cleared and cycled | A step out of order: the interlock trips, back to the latch | The tube cannot load |
| **Shield generator** (deck C, under the drive) | Six emitter segments on a ring each hum at a frequency; match each segment's wave to the ring's master wave by phase and size | One face's emitter matched | A segment over-driven: a crackle, that face's charge drains | `weapons-and-shields` 11: that face is down |
| **Sensors** (the sensor bay and the dish) | Align the array: sweep a dish across a noisy sky to find the strongest return, then filter the noise with three band sliders until the contact's blip is clean | Azimuth and elevation, then the filter | Driven past a stop: the dish jams for a step | Science and tactical lose range (`bridge-stations`) |
| **Electrical conduits** (anywhere a conduit runs: walls, ceilings, the switchboards) | Splice: strip the burnt length, then join the wire pairs colour to colour before the clamp closes, and route around a burnt junction box by drawing the new run on the conduit map | One splice, or one reroute | A crossed pair: a bang, a spark (5 HP), the breaker trips | `power-grid`: the loads beyond it are cut |
| **Switchboard and breakers** (main switchboard) | Rack the breaker out, replace the fuse cartridge by its rating (the label matches the bus), rack it back in and close it on a green synchro light | One breaker | Closed on red: an arc flash (10 HP); a cartridge of the wrong rating blows as it seats (recommendation taken (ask only with screenshots): otherwise every cartridge could be tried for free) | `power-grid` |
| **Doors** (any door, jammed) | Hand-crank the leaf: turn the crank against a resistance that comes in waves, never past the slip | One door | Slipped: the leaf drops back | The door is stuck |

Seventeen systems; the medic's game is section 3. A system's steps by its job's state:

| Integrity | Steps |
| --- | --- |
| Damaged (25-75%) | 3 |
| Disabled (under 25%) | 1 part step, then 4 |
| Destroyed (0%) | 6 (the rebuild), then 3 |

## 3. The medic's treatment

The medic (`medical-officer`) treats a body on a bed or in the field on one screen:

- **Triage.** A body chart, front and back, with each wound marked by kind and size: a cut, a burn, a fracture, a
  bleed, smoke in the lungs. The vitals strip along the top: heart rate, blood oxygen, the HP bar and the window
  left for a downed or critical body (`crew-on-deck` 7).
- **The tool.** Each wound takes one tool: the dermal sealer for cuts (sweep along the cut until it is sealed),
  burn gel for burns (paint the area until it is covered), the bone knitter for a fracture (drag the loose end into
  its socket and hold it while it knits), a clamp then the sealer for a bleed (the bleed's spurts are timed: clamp between them),
  the inhaler for smoke (press as the chest rises). The wrong tool does nothing and costs a moment.
- **Rates.** Each treated wound gives back its share of HP at `medical-officer`'s rates: field healing 4.0 HP/s to
  75 HP, the bed 6.0 HP/s tended; a revive is the first wound treated on a downed body (2.0 s with the kit at a
  steady hand, the rate's time). A dose is spent per 25 HP given, as there.
- **Slips** (a clamp in a spurt, a puff on the out-breath, the bone yanked out while it knits): the patient flinches, 2 HP lost, the wound reopens a
  little. Never a death: a slip costs time, not a life.
- **Several patients.** On the medbay's beds the screen tabs between them; the vitals of the ones not being treated
  keep running (a critical body's window keeps falling), which is the triage.

## 4. What a broken system does to the crew

| System down | Effect | Status |
| --- | --- | --- |
| Life support | The air goes bad on `life-support`'s clock | Decided (`life-support`) |
| Gravity generator | **Proposed:** everyone aboard floats. A body moves by pushing off surfaces and handholds at up to 1.5 m/s, drifts until it touches something, and turns slowly; ladders and handholds are the way along; loose things drift. Repairs are 1.5 times slower floating (no footing) | New: `crew-on-deck` gains a zero-gravity mode |
| Galley synthesizer | **Proposed:** the crew's **fed** need (`crew-npcs` 2) falls between missions instead of being met; a hungry crew member works at 0.8 times their rate on the next mission until fed. Bots and players alike | New: a ship-wide supply between missions |
| Heads and showers | **Proposed:** comfort falls; the crew's mood (a later change) carries it; for now, bots queue at the working ones | New, small |
| Medbay biobeds | Beds heal at the unpowered rate | Decided (`medical-officer`) |
| Everything else | As its own change says (the table in section 2's last column) | Decided |

## 5. Warp pylons

The Tern has no warp pylons today (`reference-ship-tern`). **Proposed:** two pylons on the hull's dorsal flanks
carrying the warp coils, reached only on EVA through the dorsal airlock (`life-support` 14). Repairs happen outside,
tethered; an EVA is refused while the ship is in combat (any hostile within weapons range), so a damaged pylon waits
for a lull, and the ship cannot jump until it is repaired. Adding them is a layout change to the hull and the
exterior; its own change (`warp-pylons`) follows the owner's view of this one.

## 6. Mockups

`docs/mockups/repairs.html`: a menu of the seventeen systems and the medic's treatment; each opens its game on a
1280 x 720 panel, with the job's bar, its steps, a fumble counter and a "combat" switch that shakes the panel as the
ship would under fire. Each game is its own script under `docs/mockups/repairs/` (one file a game, so they can be
built and judged apart), sharing `docs/mockups/repairs/kit.js` (the bar, the step rule of section 1, the input).
Flat 2D on a canvas, like the consoles (CLAUDE.md 11). Shots in `docs/screenshots/repairs/`.

### 6a. As built (2026-10-08)

All seventeen games and the treatment are playable in `docs/mockups/repairs.html`; each was played to the end
headless, with a deliberate mistake checked to fumble. Choices made while building them, recommendation taken (ask only
with screenshots):

- **Breakers:** a cartridge of the wrong rating blows as it seats (a fumble), so cartridges cannot be tried for free.
- **Fighter:** a fastener out of the star order is refused (a red cross), not a fumble; after it, the next one glows.
- **Pylons:** a bolt out of order is lost ("a dropped bolt is lost", the table's second hazard); a mistimed clip is a
  miss, not a fumble. In combat the game shows the EVA refused and takes no input.
- **Shuttle:** the needle leaving its band only restarts the 3 s hold.
- **The medic** finishes on "Treated"; a job a game overrides (the medic's HP) has no part step.
- The kit holds the drawing every game shares (hatching, a turn arrow); the part-step drag, a star order and a drag
  helper are still repeated in several games and move into the kit next (task 1.5).

### 6b. After the owner played them (2026-10-08)

The owner: "the tools dont respond well and just end up dragging the frame"; "some of the panel ones would be cool to
like screw or unscrew stuff"; "the fighter thing needs some way to tell me what the next screw is, the highlight doesnt
tell me whats wrong until I click on the wrong screw"; "wheres my toilet minigame"; "the medical one kinda sucks".

- **The pointer is the game's.** A drag on a game's canvas no longer drags or selects the page: the kit stops the
  browser's own drag, selection, panning and menu, and takes the pointer until it is released. Every game gets it.
- **Access panels with screws** (kit). A game that names a `panel` has its machine behind a cover plate: the job
  starts by unscrewing it (turn each screw anticlockwise by dragging round it, the wheel over it, or holding Left; Tab
  picks the next) and ends by screwing it back (clockwise) before the job counts as done. 2.5 turns a screw; a ring
  round each head shows how far it has turned. Not a fumble anywhere: a screw only turns while you turn it. On the
  breaker cabinet (6 screws), the scrubber cabinet and the biobed's control box (4 each). In the engine the same
  plates are where the layout's repair points are.
- **The order is shown before the first fastener.** Star orders (the fighter's panel, the pylon's bolt plate) have one
  order, from the first fastener round the star (`KIT.starOrder`); every fastener carries its number and the next one
  is lit and ringed at rest (`KIT.draw.orderBadge`). A fastener out of order is still refused (the pylon's bolt is
  still lost), but it is never a guess.
- **The reactor core in two controls and two stages** (owner: "warp core stabilizer should just be a vertical and
  horizontal dial for simplicity", "there should be a stage 2 to stabilize plasma ring too"). One vertical and one
  horizontal slider replace the four coil sliders: each pulls the field the way its handle is pushed, and the coils
  glow with it. Each round is two stages on the same controls: hold the core in its centre band, then the plasma ring
  round it, which the load pulls out of round (the horizontal slider sets its width, the vertical its height), held
  round in its band. The ring bulging into the wall or collapsing onto the core is the same heat spike as the core
  touching the wall.
- **Toilets and showers** and **the medic**: section 6c.

### 6c. Toilets and showers, and the medic (2026-10-08)

**Toilets and showers** (`repairs/heads.js`; the id stays `heads`). The owner: "wheres my toilet minigame". Rounds
alternate, a toilet then a shower:

- **The toilet.** A clogged toilet, lid up, a plunger in the bowl, and a cutaway of the trap beside it showing the
  clog. Drag the plunger down and up (or Down then Up, or Space for a whole stroke); each full stroke pushes the clog
  along the trap. The ring on the plunger's knob is the rhythm: dashed while it is too soon, solid when the next stroke
  is safe. A stroke too soon sloshes the water up the bowl; over the rim is the fumble "Overflow: the floor is wet".
  Flushing a clog fills the bowl too. When the clog clears the culprit floats up (a rubber duck) and the handle glows:
  flush it and the bowl swirls clean. Later rounds take more strokes and a slower, narrower rhythm.
- **The shower.** A pipe puzzle in the deckhead: turn tiles a quarter at a time until one run joins the main to the
  shower with no open end, then open the valve on the main. Opening it on an open run is the fumble "Loose joint: you
  are soaked". Later rounds have bigger grids.
- **The part step** of a disabled toilet fits a new flush valve (the flapper) from the crate onto its seat in the open
  cistern.

**The medic** (`repairs/medic.js`). The owner: "the medical one kinda sucks"; "the tools dont respond well and just
end up dragging the frame". Rebuilt so every tool answers on the frame it is pressed, and only timing is ever a slip:

- **Bigger wounds.** The close-up takes most of the screen; a ring in its corner fills as the wound heals. Treated
  wounds are ticked on the body chart, and clicking a marker opens its wound.
- **The tray** shows each tool's key (1-5). A press counts on the wound only if it starts in the close-up, so a click
  on the tray or the chart never touches it.
- **The wrong tool** is drawn greyed with a red cross at the pointer; pressing it says "Wrong tool" and does nothing.
- **Cut:** press near the cut and sweep along it either way; what the tip passes over seals, 95% to close. Drifting
  off pauses, never slips.
- **Burn:** a 50 px brush; gel off the burn is wasted, not a slip; 85% covered to close.
- **Fracture:** grab the loose bone anywhere along it, drag it to its ghost; it snaps in within about 25 px, and
  holding it knits. Letting go pauses. Only yanking it out of the socket in the same hold slips.
- **Bleed:** a hand goes round a ring once a beat, the spurt hatched red and the gap solid green; clamp in the gap,
  then seal the tear like a cut.
- **Smoke:** the chest widens and a breath gauge climbs on the in-breath; puff then, once a breath.

## 7. The Pi 5 budget

A mini-game is a UI screen: a few hundred to a few thousand UI triangles in a handful of draws, the 3D pass behind it
as at a console. No texture beyond the UI's atlas. Not measured: a cloud session renders on a CPU (CLAUDE.md 2).

## Risks / Trade-offs

- **Seventeen games is a lot to build and to learn.** Each is small (one picture, one move); the first meeting shows
  the move once; and a bot or a team can always do the job without a game.
- **A game in combat is a player not at their station.** That is the trade the owner asked for: repairs are a job.
- **A game that rewards speed would break the board's promise.** The step waits for the rate (section 1).
EOF