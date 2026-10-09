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

Revised 2026-10-09 by the owner's two notes (section 1a): "If a repair needs more than one round, like say you need 3
rounds to repair something I would prefer that repair minigame to be harder variants of the same minigame", and "I
also dont like waiting for the slider to slowly go up, if I repair something it should go up in health immediately".

A repair job (`damage-control` 6) is cut into **rounds**, three to six by the job's state (the table at the end of
section 2). A round is the system's whole mini-game, played once.

- **A round lands at once.** Finishing a round raises the system's integrity by the round's share there and then:
  the job's bar jumps in 0.2 s, never fills at a rate while the player waits. The share is the job's span over its
  rounds (25% to 100% in 3 rounds is 25% a round). The last round lands exactly on the target.
- **Rounds get harder** (section 1a): every round of a job is the same game, at the next level of difficulty.
- **Fumbles.** A mistake (a wrong wire, a dropped coil, a spike past the line) costs **5% of the job** (data,
  `damage.json` `repair.fumble_share`) at once, and that system's own hazard, said in section 2: a spark that stings
  for 5 HP, a puff of coolant, a heat spike in the reactor. Three fumbles in one round and the round restarts.
- **Interruptions.** Taking a hit, being pushed or walking away pauses the round where it is; rounds already landed
  stay landed. Under fire the screen shakes with the ship (`ship-frames`' lurch), which is the game getting harder by
  itself, not a rule. Where the player is in the ship while this happens is `repairs-on-deck`.
- **Two hands.** Two players on one job each play their own rounds from the same list: each takes the next round
  left, at its level, and each lands its own share. A bot helping a player adds its rating's rate between rounds,
  with no game.
- **No player, no game.** A bot or a damage control team repairs at its rate (`damage::repair_time(job, who)`, a
  rating's 0.6% a second) with no game and no fumble.
- **Parts and rebuilds.** A disabled system's first round opens by fitting the spare part (each game's own part move,
  section 2), then plays the round. A destroyed system's rebuild is its rounds from level 1, the first opening with
  the parts.
- **The screen.** One screen, 1280 x 720 like a console: the system's picture in the middle, the job's bar along the
  top (integrity now and where this round will take it), the round dots, and the one action. No instructions on the
  screen at rest (CLAUDE.md 10); the how-to card (6g) is a button.
- **Input.** Mouse, touch and pad: every game is pointing and one button, or a stick and a trigger. Keys mirror the
  pad. Nothing needs a fast double-click.
- **What the board previews.** For a player the damage board shows the rounds left ("2 rounds"), which is exact;
  for a bot or a team it shows the time at its rate, which is exact too (CLAUDE.md 6.1: a preview is computed by
  the code that resolves it). A player's time is the player's own play, so the board does not invent one.

## 1a. Rounds: the same game, harder (owner, 2026-10-09)

**The rule.** Round *n* of a job (counted from 1) is played at **level** *n*, capped at 6. Level is the only thing
that changes between rounds: the same machine, the same moves, the same how-to card, with the game's own knobs
turned. A round never switches to a different activity; a game that used to alternate two activities now plays both
in every round, in order, or (the heads) becomes two jobs.

**Why it replaces the old timing.** The old rule held a finished step until a bar had filled at the officer's 1.8% a
second, so a fast player waited and the time was fixed. Now the time is the play: three rounds of rising level take
longer than three of level 1, so a badly damaged system is still a longer job, and a skilled crew is faster. That
retires the officer's 3 times rate for a player's own repairs (`damage-control` 6a): it still sets the board's time
for anyone repairing without a game. Recommendation taken (ask only with screenshots): the rating's 0.6% a second
and the bots are unchanged.

**What turns up, by game.** Level 1 is the round the game always had; each level after turns the knobs listed, until
level 6. In the mockups the knobs are constants in each game's script; in the engine they are data
(`data/ships/tern/repairs.json`, `levels`, one row a game; repairs-on-deck 11).

| Game | A round is | What each level turns up (as built in the mockups) |
| --- | --- | --- |
| Gravity generator | Lay the field on the ghost and hold it | Drift 15% faster a level; a second ripple across the first from level 3; the hold 0.4 s longer a level |
| Galley synthesizer | Fill four columns to their bands, purge | Bands 7.5% wide, 1% narrower a level (floor 4%) |
| Life support scrubbers | Swap the cartridge, trim three valves into green | Green bands 10% wide, 1.5% narrower a level (floor 5%) |
| Fighters | Panel off, leads to their sockets, fasteners in the lit order | Leads 3, one more a level to 5, told apart by fewer shapes (4, 3, then 2) so the stripe has to be read; fasteners 6, then 8 |
| Shuttle | Sniff, isolate, patch, pump up and hold | The simple valve set at level 1, the full network from level 2; hold band 9% wide, 1.5% narrower a level (floor 4.5%); pressure decay 0.1, +0.025 a level |
| Reactor core | Swap a coil while holding the plasma centred | Drift 0.3 rad/s, +0.1 a level; drift size +14 px a level |
| Impulse engines | Time eight injectors | Pulse speed 240 px/s, +40 a level; window 30 px, 3 px less a level (floor 18) |
| Warp pylons | Clip along the handholds, unbolt, slide in | Clip window 3.4 s, 0.4 s shorter a level (floor 1.8 s); the swing faster each level |
| Toilet (its own job) | Plunge the clog in rhythm, flush | Strokes to clear 6, 9, then 12; the safe gap between strokes 0.5-3.0 s, 0.65-2.5 s, then 0.8-2.1 s |
| Shower (its own job) | Turn the tiles into one run, open the valve | Grid 5 x 4, 6 x 4, 6 x 5, then 7 x 5 |
| Medbay biobeds | Tune both channels onto their references | Tolerance 0.045, 0.006 less a level (floor 0.025); drift and trace noise up a step a level |
| Twin pulse cannon | Focus both lenses, then seat the coupling | Shimmer +1.3 a level; the spot's tolerance 15 px, 1.5 px less a level (floor 9); the coupling's good speed band narrower each level |
| Gannet tubes | Clear the ring's jam, then the arming sequence | Notches 8.5% wide, 1% narrower a level (floor 4.5%); the ring kicks back harder; from level 3 the switches are in a shuffled order |
| Shield generator | Match a face's wave by phase and gain | Phase tolerance 0.24, 0.025 less a level; gain 0.13, 0.012 less; wander +0.06 a level |
| Sensors | Find the return, clean the trace | A false return from level 2; the true return 70 px wide, 8 px less a level (floor 42); filter tolerance 0.15, 0.02 less a level (floor 0.08); noisier sky |
| Electrical conduits | Strip the burns, join the pairs; a disabled job reroutes round its burnt box every round too | Pairs 3, one more a level to 6; the clamp closes sooner; the reroute's map busier |
| Switchboard and breakers | Rack out, fit the rated fuse, rack in on green | Cartridges to choose from 3, 4 from level 3; the synchro light faster (+0.35 rad/s a level) and its green window shorter (0.34, 0.04 less a level, floor 0.16) |
| Doors | Crank the leaf against the waves | Turns 5, one more a level; waves stronger (+0.6 a level) |
| Hull plating | Cut, drop the plate, weld, bolt: a new plate each round | The good weld band 0.35-0.85, 0.025 off each end a level; burn-through after 0.3 s over it, 0.03 s sooner a level |
| Coolant balance | Both legs into their bands against the reactor's state | Damaged: cruise, surge, then pump B lost; disabled: a leak, overdrive, a radiator pump down, then a surge |
| Coolant pipes | Isolate, rebuild, refill and bleed a new segment | Grid 5 x 4 at level 1, 7 x 5 from level 2; crossings and cracked tiles one more a level (to 4) |
| Coolant pump | Guard off, align the feet, then speed up to the band | Foot rings 20 px, 3 px smaller a level (floor 10); the suction dial's red edge nearer (+0.03 a level) |
| Chiller | Scrub the plate, then restack and torque the pack | Scale patches 8, two more a level; plates out of order 1, then 2 from level 3 |

The medic's treatment is not rounds: each wound is the job's step, and a body's wounds are what they are
(section 3).

**The heads become two jobs.** A broken toilet and a broken shower are different fixtures with different repair
points (`ship-interactables` 2), so each is its own job with its own game. The old alternating game is retired.

## 2. The mini-games

| System (where) | The game | A step | Fumble and hazard | When it is down |
| --- | --- | --- | --- | --- |
| **Gravity generator** (engineering, deck C) | Field alignment: the field is a 3D wave surface over the deck, drawn beside the reference shape it should hold. Turn, shift and stretch the live wave until it lies on the reference, and keep it there against its drift (section 6d) | One field held aligned for the step | The field surges past the red line: everyone near floats for 2 s | Section 4: everyone floats, slowly |
| **Galley synthesizer** (mess) | A recipe card shows four nutrient columns; open valves to fill each column to its band without spilling over, then purge | One recipe balanced | Overfill: a splash of paste, the step's column drains | Section 4: hunger between missions |
| **Life support scrubbers** (life support) | Swap the spent CO2 cartridge for a fresh one (drag out, drag in), then trim three gas valves until O2, N2 and CO2 sit in their green bands | One cartridge, or one bank trimmed | Cartridge dropped, or a valve left in the red: a hiss, the room's CO2 rises by the share | `life-support`'s clock: the air goes bad |
| **Fighters** (hangar, a Swift on its cradle) | Lift the panel, then match the avionics plugs to their sockets by shape and colour stripe; torque the panel's fasteners in a star order | One panel: plugs, then fasteners | A plug forced into the wrong socket: a spark (5 HP), the plug bent | The fighter cannot launch |
| **Shuttle** (hangar, the Petrel) | Fuel line leak: sniff along the lines to find it, close the valve on the tank's side of it, patch it, then pump the line up and hold the needle in band for 3 s (6f) | One leak found, isolated, patched and tested | Wrong valve: the leak still fed, fuel mist, the bay's fire risk up | The Petrel cannot launch |
| **Reactor core** (engineering) | Keep the plasma ball centred in the containment ring by trimming four magnet coils (one stick or four sliders) while a fifth coil is swapped out; it drifts with the core's load | One coil swapped while held centred | The plasma touches the wall: a heat spike, the core's temperature jumps | `power-grid`: the ship runs on batteries |
| **Impulse engines** (drive, the two impulse units) | Injector timing: pulses run along a scrolling trace; tap as each crosses the firing line, the rhythm set by the unit's tune | Eight injectors timed | Off the line: a misfire, a soot cough, the unit's heat up | Half thrust per unit down (`flight-and-navigation`) |
| **Warp pylons** (outside, EVA only) | On the hull, tethered: clip from handhold to handhold to the damaged coil segment, unbolt it in a pattern, slide the new one in against a drift | One segment changed | Unclipped too long: the tether snaps you back; a dropped bolt is lost | Section 5: no warp; not repairable in combat |
| **Toilets and showers** (crew quarters; two jobs since 2026-10-09, section 1a) | A toilet: plunge the clog down the trap in a safe rhythm, then flush. A shower: rotate the deckhead pipe tiles so one run joins the main to the shower, then open the valve | One toilet unclogged and flushed, or one shower running | Plunging too fast or flushing a clog: overflow, the floor is wet. Opening the valve on an open run: a loose joint, you are soaked (a moodlet) | Section 4: comfort and morale |
| **Medbay biobeds** (medbay) | Sensor calibration: tune two knobs until the bed's trace lies on the reference trace | One sensor channel | Over-driven: the bed alarms; nothing hurt | Beds heal at the unpowered rate |
| **Twin pulse cannon** (each turret's access room) | Focus the emitter: align two lenses so the test beam's spot is smallest and centred, then reseat the capacitor bank's coupling (drag it home at the right speed) | One lens pair, or the coupling | Coupling slammed: the bank arcs (10 HP) and loses its charge | The turret does not fire |
| **Gannet tubes and hoist** (magazine) | The tube's breech: clear the jam by rotating the locking ring through its notches, then run the load-and-arm sequence (guide rail, latch, interlock) in order | One tube cleared and cycled | A step out of order: the interlock trips, back to the latch | The tube cannot load |
| **Shield generator** (deck C, under the drive) | Six emitter segments on a ring each hum at a frequency; match each segment's wave to the ring's master wave by phase and size | One face's emitter matched | A segment over-driven: a crackle, that face's charge drains | `weapons-and-shields` 11: that face is down |
| **Sensors** (the sensor bay and the dish) | Align the array: sweep a dish across a noisy sky to find the strongest return, then filter the noise with three band sliders until the contact's blip is clean | Azimuth and elevation, then the filter | Driven past a stop: the dish jams for a step | Science and tactical lose range (`bridge-stations`) |
| **Electrical conduits** (anywhere a conduit runs: walls, ceilings, the switchboards) | Splice: strip the burnt length, then join the wire pairs colour to colour before the clamp closes, and route around a burnt junction box by drawing the new run on the conduit map | One splice, or one reroute | A crossed pair: a bang, a spark (5 HP), the breaker trips | `power-grid`: the loads beyond it are cut |
| **Switchboard and breakers** (main switchboard) | Rack the breaker out, replace the fuse cartridge by its rating (the label matches the bus), rack it back in and close it on a green synchro light | One breaker | Closed on red: an arc flash (10 HP); a cartridge of the wrong rating blows as it seats (recommendation taken (ask only with screenshots): otherwise every cartridge could be tried for free) | `power-grid` |
| **Doors** (any door, jammed) | Hand-crank the leaf: turn the crank against a resistance that comes in waves, never past the slip | One door | Slipped: the leaf drops back | The door is stuck |

Seventeen systems; the medic's game is section 3. A job's rounds by its state (revised 2026-10-09, section 1a; was a
separate part step, and a destroyed system's 6 rebuild steps then 3):

| Integrity | Rounds | Levels |
| --- | --- | --- |
| Damaged (25-75%) | 3 | 1, 2, 3 |
| Disabled (under 25%) | 4, the first opening with the spare part | 1 to 4 |
| Destroyed (0%) | 6, the first opening with the parts | 1 to 6 |

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
  picks the next) and ends by screwing it back (clockwise) before the job counts as done. One full loop a screw (2.5 at first; the owner, 2026-10-09: loops were not "properly detecting", so every pointer sample now counts and a loop the wrong way flashes the arrow and says "Other way"); a ring
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

### 6d. The gravity generator redesigned, the reactor's reset, and phones (2026-10-08)

The owner: "Gravity generator is too slow and basic, redesign that completely. Something about realigning a 3d wave
form graph might be fun"; "When reactor ball hits the wall reset the sliders"; "Do these work on mobile too".

**Why the rings were slow.** Three taps played a round in a few seconds, and then the player waited for the step's
share to fill at the rate (section 1): most of the job was watching a bar. A round must keep hands busy for about as
long as its share takes, so the new game is a hold against drift, like the reactor, and its hold time is sized to the
share.

**Field alignment** (`repairs/gravity.js`, replacing the rings):

- **The picture.** The generator's field over the deck as a 3D wave surface: a wireframe grid in perspective, its
  height the field's ripple. The reference shape (the field the generator should make) is drawn as a faint ghost in
  the same space. Where the live surface lies on the ghost it is green; where it is off it shades to amber, then red,
  so the picture says where it is wrong, not only how much.
- **The controls**, the same on every round and each one picture (section 10 of CLAUDE.md):
  - **Turn**: a dial that rotates the wave's crests across the deck (its direction);
  - **Shift**: a horizontal slider that slides the crests along their direction (the phase);
  - **Stretch**: a vertical slider for the spacing of the crests (the wavelength), from round 2;
  - dragging on the surface itself turns (sideways) and shifts (up and down) as a shortcut.
- **The match.** One number drives everything: the root mean square difference between the live surface and the
  ghost over the grid, divided by the ghost's own. Under 0.15 the field holds: a ring round the graph fills while it
  stays there, and the round is played when it fills. Above 0.85 the field surges: the fumble, and the drift is
  kicked to a new heading.
- **The drift.** The live wave wanders in all three settings, smoothly, faster each round and faster in combat, so a
  held field has to be nursed, never set and left.
- **Rounds.** Round 1: turn and shift. Round 2: stretch joins. Round 3 and on: the reference has a second, smaller
  ripple across the first and the live field has its own, aligned by the same three controls (a tab picks which
  ripple the controls hold). The small ripple drifts at about a third of the big one's speed and never in stretch
  (its stretch still starts off the mark), so two ripples are nursed, not chased.
- **The hold** is sized from the step's share at the officer's rate: 0.62 of the share for one ripple (5-9.5 s, plus
  0.4 s a round) and 0.5 of it for two, because catching both takes longer. Measured with a scripted player, a damaged
  job's rounds are played in 10-14 s against a 13.9 s share, so play and the bar end together. A disabled job's part
  step still waits out its share after the coil is fitted: that is the kit's step rule (section 1), not this game.
- **The part step** of a disabled generator fits the new field coil from the crate into the hub, as before.

**The reactor resets its trims on a spike.** When the plasma or its ring touches the wall, both sliders spring back
to centre and let go of the hand on them, so the recovery starts from neutral, not from the setting that caused it.

**Phones.** Every game is driven by pointer events, so a finger plays it as a mouse does. The page puts the game
first on a narrow or touch screen, with a game picker and a Full screen button above it (full screen fits the canvas
to the screen, and asks a phone to hold landscape where the browser allows), and tells a phone held upright to turn
sideways. Each game is checked by playing it to the end with touch alone on a phone-sized screen.

Checked 2026-10-09 (an emulated iPhone 13 held sideways, full screen, touch events only): all eighteen games play to
the end in both job states, and a mistake can be made by touch in each. What touch needed: the kit follows only the
finger that started a press (a second finger or a palm no longer jumps or ends a drag), a tap shorter than a frame
counts as a press, and packed targets (wire ends, fuses, valves, fasteners, handholds, bolts, the medic's tray, the
turret's lenses) take the nearest one within a reach of at least 30 canvas px (`KIT.TOUCH_R`, `KIT.nearest`).
Still to fix: text drawn at 15-18 canvas px (the bar's place line, time left and mistake note, and several small
labels) is 7-9 CSS px on a phone, too small to read; the mistake note matters most. Shots in
`docs/screenshots/repairs/mobile/`.

### 6e. Hull plating, coolant pipes and coolant balance (2026-10-09)

Three more games, designed in their own changes: **Hull plating** (`hull-repair` 3: cut out, fit, weld, bolt), and
the reactor's coolant system, **Coolant pipes** and **Coolant balance** (`reactor-cooling` 4 and 5). The reactor's
game is now **Reactor: magnetic core** (`reactor-cooling` 1). All three play by mouse, keys and touch.

### 6f. The shuttle's fuel line, made readable (2026-10-09)

The owner: "im not really sure how the shuttle mini game works". The first version found the leak by closing valves
outward from the tank and reading a gauge: holding meant the leak was past the valve. That is the reverse of what a
player guesses, and nothing on screen says it. Rebuilt, keeping the job (find, seal, prove):

1. **Find**: drag the leak sniffer along the Petrel's fuel lines. Its ring pulses faster and brighter the nearer the
   leak, warmer and colder, and a faint mist shows once it is within reach. No mistake is possible here: it is the
   looking.
2. **Isolate**: tap the valve on the tank's side of the leak. The line past it empties (drawn hollow) and the mist
   stops. Closing a valve that leaves the leak still fed (on the far side of it, or on another line) is the mistake:
   "Fuel mist: the bay's fire risk rises", and the valve springs back.
3. **Patch**: drag the patch onto the leak.
4. **Pressure test**: open the valve, hold the pump, keep the needle in the green band for 3 s (as before).

Later steps put the leak deeper in the lines and narrow the band. A disabled shuttle's first step still fits the new
isolation valve into the gap in the main line.

### 6g. A how-to guide in every game (owner, 2026-10-09)

The owner: "can you add a little info guide for players to figure stuff out a bit easier?". Every game gets a **?**
button in its bar, beside the fumble pips. It pauses nothing and hides nothing at rest; pressed (or F1, gamepad
Back), it lays a card over the game:

- **Up to four steps, each a picture and a few words** ("Drag round the screw", "Hold the needle in green"), drawn
  from the game's own art, numbered in the order they come, the current step lit.
- **One line on the mistake**: what causes it and what it costs ("Wrong valve: fuel mist, 5% lost").
- **Tap anywhere to close.** The first time a player opens a game the card shows once on its own; after that only on
  the button.

The guide is data in each game's registration (`guide: { steps: [{ icon, text }], mistake }`), drawn by the kit,
so every game's card looks the same (CLAUDE.md 6.1). It is the one place a repair game holds sentences, and only on
demand: at rest the game stays glance first (CLAUDE.md 10).

### 6h. The coolant pump and the chiller (owner, 2026-10-09)

The owner: "make a pump and chiller repair game". Until now the reactor system screen sent a pump or the chiller to the
scrubbers' cabinet as a stand-in (`reactor-cooling` 6a). Two games replace it, both in the Engineering group, both
played by mouse, keys and touch, both following section 1 (a fumble costs 5%, three restart the step).

**Coolant pump** (`repairs/pump.js`; the core pumps A and B, the radiator pumps and the makeup pump). The pump from
the side: the volute (the snail-shell casing) on the left, its impeller behind a window, the coupling, the motor on
two feet on the right. A guard over the coupling is screwed on (the kit's cover, 4 screws). Two rounds, alternating:

1. **Align the shaft.** A laser on the pump's shaft throws two dots on two targets on the motor, near and far. Each
   foot moves up and down by dragging its shim handle (or W and S on the selected foot, Tab to the other): the front
   foot moves the near dot most, the rear foot the far one, and each pulls the other's dot a little (0.35 of its
   move), as a real two-foot alignment does. Both dots held in their centre rings 0.6 s: aligned. A dot left off its
   target plate 1.2 s is the fumble "Coupling knocks: the bearings heat". Later steps shrink the rings.
2. **Spin it up.** The speed lever is the biggest control; the suction gauge beside it has a red band at the bottom.
   Speed lowers suction (`p = 1 - 0.55 v^2`, followed with a 0.8 s lag) and a fast ramp drops it further while it
   ramps (0.6 x the speed's rate). Raise the speed to the band at 100% and hold it there 1 s with the needle out of
   the red: running. The needle in the red is the fumble "Cavitation: the impeller pits", bubbles in the window and
   the speed knocked back to 40%. Later steps raise the red band, so the ramp has to be gentler.

A disabled pump's first step fits a new impeller: drag it from the crate onto the shaft. Its vanes must curve away
from the casing's rotation arrow (backward-curved, as every centrifugal pump's are); tap it in the crate to turn it
over. Fitted the wrong way round is the fumble "Impeller backwards: no flow", and it drops back in the crate.

**Heat exchanger (chiller)** (`repairs/chiller.js`). A plate exchanger: a pack of seven plates between a fixed frame
and a pressure plate, held by a top and a bottom tie bolt. Two rounds, alternating:

1. **Scrub the plates.** The fouled plate is on the bench, face on: grey scale over its chevron pattern, a black
   rubber gasket round its edge and its two ports. Drag the brush over it: the scale under the brush comes off. The
   brush held on the gasket 0.3 s is the fumble "Torn gasket: coolant weeps". 95% of the scale off: clean, and it
   slides back into the pack. Later steps put more scale on, and closer to the gasket.
2. **Restack and clamp.** The pack's plates must alternate, chevrons up and down; one or two hang the wrong way: tap a
   plate to turn it over. Then close the pack to its mark: tap a tie bolt's nut to turn it a quarter, which moves its
   end of the pressure plate in. The two ends more than two quarters apart is the fumble "Skewed pack: a plate
   cracks", and that nut backs off. The bolts do not turn until the plates alternate. Both ends on the mark:
   clamped.

A disabled chiller's first step fits a new plate: drag it from the crate into the gap in the pack.

Both games' guides follow section 6g. The reactor system screen opens them from a pump's or the chiller's FIX.

## 7. The Pi 5 budget

A mini-game is a UI screen: a few hundred to a few thousand UI triangles in a handful of draws, the 3D pass behind it
as at a console. No texture beyond the UI's atlas. Not measured: a cloud session renders on a CPU (CLAUDE.md 2).

## Risks / Trade-offs

- **Seventeen games is a lot to build and to learn.** Each is small (one picture, one move); the first meeting shows
  the move once; and a bot or a team can always do the job without a game.
- **A game in combat is a player not at their station.** That is the trade the owner asked for: repairs are a job.
- **A game that rewards speed would break the board's promise.** The step waits for the rate (section 1).
EOF