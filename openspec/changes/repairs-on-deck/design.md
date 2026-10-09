# Design: repairs on deck

## Context

The owner, 2026-10-09: "lets call out how we would perform these mechanics in the games 3d environment". The
mechanics are the repair mini-games and the medic's treatment (`repair-minigames`), with the owner's two rules of
the same day: a finished round lands its share of health at once, and every round of a job is the same game, harder
(`repair-minigames` 1 and 1a).

| Decided elsewhere | Where |
| --- | --- |
| A repair is a body with a kit at the system's repair point; no console repairs anything | `damage-control` 6a |
| Kits are in the damage control locker (6), spare parts in cargo (24) | `damage-control` 6 |
| Use is E or gamepad A, within reach, with a clear line; a body holds one thing | `crew-on-deck` 6, 13 |
| A fixture's use points live on its prop, written by its Blender build; the server owns fixture state | `ship-interactables` 1 |
| A seated console is drawn 2D over the live bridge | CLAUDE.md 10, `bridge-stations` |
| Bots repair at a rating's rate with no game | `repair-minigames` 1, `crew-npcs` 3 |
| Every input device drives every screen | CLAUDE.md 10 |

## 1. The loop in the ship

1. **See it.** Three places show a damaged system, all reading the same job (`damage-control` 12):
   - the damage board (damage control, and the engineering console): the job, its state and its rounds left;
   - the ship map (M, `ship-plan-view`): a wrench marker on the system, red when disabled, amber when damaged, with
     its rounds left as dots;
   - the machine itself (section 4): its status lamp, its sound, smoke and sparks. A player walking past a damaged
     pump hears it labour before any screen tells them.
2. **Claim it** (optional). Use on a job on the damage board puts it on the player's HUD: one small marker at the
   repair point, an edge arrow when it is off screen, and the job's name. Walking up to a job claims it as well.
   Two players may claim one job (section 7).
3. **Take a kit.** A kit from the damage control locker or any kit bracket (`kit.json`). A disabled or destroyed job
   needs parts: Use on a parts bin in cargo **while holding the kit** puts a part in the kit's pouch (it holds 3).
   Recommendation taken (ask only with screenshots): the pouch, so the one-thing-in-hand rule (`crew-on-deck` 6)
   still holds and nobody juggles a kit and a part.
4. **Walk there.** At walking speed with the kit (8 kg, full speed).
5. **Dock** (section 3). Crosshair on the repair point, the prompt reads "Coolant pump: Repair, 3 rounds" (or
   "Needs a part" with the kit's pouch empty), Use.
6. **Play the rounds.** Each landed round raises the system's health at once, and the machine shows it (section 4).
7. **Stand up.** The last round lands, the service face closes, the body stands, the camera returns to the eye. Or
   the player leaves early (section 3), and the job keeps the rounds already landed.

## 2. The repair point

Every repairable thing's prop carries a `repair` use point (`ship-interactables` 1's model, a new kind), written by
the prop's Blender build so it moves with the model:

| Field | What it is |
| --- | --- |
| `p_m`, `facing` | Where the body's feet go and which way it faces |
| `posture` | `kneel` (most), `stand` (switchboards, chiller), `reach` (deckhead runs: the shower pipes, overhead conduits), `under` (lying under a fighter or the shuttle's belly), `float` (zero gravity and EVA) |
| `frame` | The docked camera: an eye point, a look point and a field of view, in prop space, set so the service face fills the middle of the screen |
| `service` | The prop's service face: the cover plate, its screws (where, how many, turns to free each) and the inner detail it reveals (section 3a) |
| `game` | The mini-game's id (`repair-minigames` 2) |
| `workers` | 1, or 2 where two can work side by side (section 7) |
| `handhold` | The handhold beside it, required: with gravity off a body can only dock where it can hold on |

The reach rule is `ship-interactables`': within 1.2 m of the point, facing it within 60 degrees, a clear line.

## 3. Docking

**In.** Use at a repair point, holding a kit:

- The server checks the reach, the kit, the parts the job needs and the posture (not incapacitated, not carrying a
  casualty), then docks the body: it slides to `p_m` and turns to `facing` over 0.3 s and takes the posture.
- The camera moves from the eye to the `frame` pose over 0.3 s, a fixed cut-in, never a smoothed look: mouse look is
  off while docked, and when it comes back it is raw again (`crew-on-deck` 13).
- The cover comes off in 3D (section 3a), on the machine, in the room.
- Then the mini-game's 1280 x 720 panel comes up over the docked view, starting at the work behind the cover.

### 3a. Off with the cover, in 3D (owner, 2026-10-09)

The owner: "The metal panels should just be in 3D. Where you have to unscrew stuff". The cover plate and its screws
are part of the prop, and taking them off is played on the prop in the docked 3D view, not drawn on the 2D panel:

- **Unscrew.** The docked frame shows the whole cover. Drag round a screw head anticlockwise (it turns with the
  pointer, one full loop frees it, a ring round the head shows how far), or hold Left on a pad or keys to turn the
  lit screw, Tab to pick the next. A screw backs out of its hole as it turns and drops into the kit's tray when free.
  Turning the wrong way does nothing but flash the head's arrow. It is never a fumble: a screw turns only while you
  turn it (as `repair-minigames` 6b's 2D cover).
- **Lift off.** With every screw out the plate lifts off and leans against the machine below the face, where the room
  sees it. Then the panel comes up for the rounds.
- **Back on.** When the last round lands, the panel goes, the plate goes back on in 3D and every screw is driven home
  clockwise the same way. Only then is the job done and the body stands. Leaving with the job unfinished leaves the
  cover off (a crewmate sees the machine open); the next to dock skips straight to the rounds.
- **Every service face.** A machine whose game had no 2D cover still has a cover in the room (the coil housing on the
  reactor ring, the injector cover, the breech door's latches): every repair starts by opening it in 3D. A latch is a
  screw that takes a quarter turn. The count and the turns are the prop's (`service` in section 2): 4 screws on most,
  6 on a switchboard cubicle.
- **The 2D covers stay in `repairs.html`**, which has no ship to put them on; in the game the kit's cover step is this.

**The screen while docked.** The panel covers the middle of the screen (1280 x 720 on a 1920 x 1080 output, or 2/3
of the height on any other). The docked 3D view keeps rendering around it, darkened 35%, with no blur (the Pi 5 does
not pay for one): smoke rolling in, a fire spreading, a crewmate running past, the red-alert lighting. The machine
behind the panel is the same machine, with its service face open. The ship's lurch shakes the panel and the view
together (`ship-frames`).

**Look up.** Holding the brace input (right mouse, gamepad LT) drops the panel to 15% and frees the look within
70 degrees of the frame, still kneeling: a glance at the door without leaving the job. Letting go brings the panel
back where it was.

**Out.** The player leaves with Esc or gamepad B, or by holding a move key for 0.4 s; the camera returns to the eye
over 0.2 s and the body stands. The round in progress pauses and is kept for 30 s, for this player only: back
within that, the round resumes where it was; later, the next dock starts it again. Landed rounds always stay.

**Pushed out.** The server undocks a body that:
- takes a hit of 10 HP or more, or is knocked down (`crew-on-deck` 7);
- has fire reach its cell (`fire-spread`);
- is in air under 50 kPa without a suit (`crew-on-deck` 9);
- loses gravity with no handhold reach (it should always have one, section 2).

The round pauses as for leaving.

## 4. What the room sees and hears

A repair is a thing that happens in the room, not on a private screen.

- **The repairer.** In the posture, the kit open on the deck beside them, a work light on its lid lighting the
  service face (a runtime light from the kit, one of the few over the bake, `light-baking`).
- **The service face.** Its screws turning out one by one, then the plate leaning against the machine, the inner
  detail showing (the vanes of the pump, the coil stack of the reactor ring, the fuse rack), until the plate goes back
  on and its screws in.
- **The machine's state.** Its status lamp, sound and effects follow its integrity band:

| Band | Lamp | Sound | Effects |
| --- | --- | --- | --- |
| Nominal (75% and up) | Green, steady | Its normal hum | None |
| Damaged (25-75%) | Amber, slow blink | Its hum, with a labour or a rattle | Sparks now and then from the service face |
| Disabled (under 25%) | Red, fast blink | Silent but for ticks | A thin smoke, sparks |
| Destroyed (0%) | Dark | Silent | Heavy smoke, charred texture layer, sparks |

- **A landed round** is an event everyone near sees: the lamp steps, the sound changes up a band where it crosses
  one, a burst of sparks or a puff stops, and a short rising tone plays at the point. The last round brings the
  machine back to life: its hum starts, its animated parts move again (a pump's shaft, the reactor's plasma ring).
- **A fumble** is the system's own hazard, in the room, at the service face (section 5's last column), hurting
  whoever is within its radius, not only the repairer.
- **Bots** dock and kneel the same way, with no panel; their repairs move the lamp and sound at the band crossings.

## 5. Every job's place and cues

| Job (game) | Where (layout) | Posture, workers | Service face | A landed round in the room | A fumble in the room |
| --- | --- | --- | --- | --- | --- |
| Gravity generator | `cargo`, `gravity_generator` | Kneel, 2 | The field housing's side panel | The field's glow ring steadies a notch; loose things settle | A field surge: everyone within 6 m floats for 2 s |
| Galley synthesizer | `mess` | Stand, 1 | The front hatch, nozzle bank showing | A clean purge hiss | A splash of paste on the deck and the repairer |
| Life support scrubbers | `life_support`, `co2_scrubbers` | Kneel, 2 | The cartridge bay door | The fan spins up a step | A hiss; the room's CO2 rises by the share |
| Fighters | `launch_bay_p` and `_s`, the cradled Swift | Under, 1 | The belly avionics panel lifts off | A ready light on the cradle | A spark: 5 HP within 1 m |
| Shuttle | `hangar`, the Petrel on its pad | Under, 1 | The fuel line access along the belly | The line's pressure gauge on the pad climbs | Fuel mist: the bay's fire risk up for 20 s |
| Reactor core | `engineering`, `reactor` | Stand on the catwalk, 2 | A coil housing on the ring | The plasma ring in the window band steadies and brightens | A heat spike: the core's temperature jumps; 4 HP within 2 m |
| Impulse engines | `drive`, each impulse unit | Kneel, 1 a unit | The injector cover | The unit's idle note evens out | A misfire: a soot cough, the unit's heat up |
| Warp pylons | Outside, EVA (`repair-minigames` 5) | Float, 1 | The coil segment's fairing | The coil's glow returns in a segment | The tether snaps the body back (no damage) |
| Toilet | `quarters`, each toilet | Kneel, 1 | The bowl and its trap cover | A gurgle and the bowl clears a step | An overflow: the deck is wet (slippery, `crew-on-deck` 3) |
| Shower | `quarters`, each shower | Reach, 1 | The deckhead panel over the cubicle | Water runs a moment, then stops | A loose joint sprays: the repairer is soaked |
| Medbay biobeds | `medbay`, each bed | Stand, 1 | The bed's sensor panel | The bed's trace on its screen settles | The bed alarms |
| Twin pulse cannon | Each turret's access room | Kneel, 1 | The emitter and capacitor hatch | A charge whine climbs a step | The bank arcs: 10 HP within 1 m |
| Gannet tubes | `torpedo_room`, each tube | Kneel, 1 | The breech door | The breech's lock lamp goes amber, then green | The interlock trips with a clunk |
| Shield generator | `shield_room`, `shield_generator` | Kneel, 2 | One emitter segment's cover | That face's hum comes back on its emitter | A crackle: that face's charge drains |
| Sensors | `computer_core`, the array's processing rack | Kneel, 1 | The rack's front | The rack's lights settle from flicker to steady | The dish jams (a thud from the hull) |
| Electrical conduits | Any conduit run, wall or deckhead | Kneel or reach, 1 | The conduit cover over the burnt length | The lights on that run come back where the round reaches | A spark: 5 HP, the breaker trips |
| Switchboard and breakers | `engineering`, `switchboard`, each section | Stand, 2 | The breaker cubicle door | The section's bus lamp steps up | An arc flash: 10 HP within 1.5 m, a white flash |
| Doors | Any jammed door | Stand, 1 | The crank cover beside the door | The leaf moves open a share | The leaf drops back with a bang |
| Hull plating | At the breach, inside | Kneel or reach, 1 | The breach itself (no cover) | A plate seam glows as it cools | The torch burns through: 2 HP |
| Coolant balance | `engineering`, the coolant panel | Stand, 1 | The panel's valve board | The loop's flow note steadies | An over-temperature alarm |
| Coolant pipes | `engineering`, the coolant runs | Kneel or reach, 1 | The run's lagging, peeled back | Flow sounds return along the run | A scald spray: 5 HP within 1 m |
| Coolant pump | `engineering`, `coolant_pumps` | Kneel, 1 a pump | The coupling guard | The shaft turns and its rattle fades a step | Cavitation: a hammer knock, the pump's health drops |
| Chiller | `engineering`, the heat exchanger | Stand, 1 | The plate pack's end cover | The outlet pipe frosts a little | A plate cracks: a puff of coolant |

The medic's treatment is docked the same way at a patient, on a bed or on the deck (kneel, 1): the framing is over
the body, the panel is the treatment screen (`repair-minigames` 3), and a slip makes the patient flinch where the
room can see it.

## 6. The server's part

The game is played on the client, where the input is; the server owns the job (CLAUDE.md 6.3: one authoritative
simulation). Each round's puzzle is seeded from the session seed, the job and the round (CLAUDE.md 6.4), so a
restarted round is the same puzzle and a replay agrees.

| Intent | The server checks | Then |
| --- | --- | --- |
| `Dock { point }` | Reach, kit, parts, posture, a free worker slot | Docks the body, opens the service face, sends the job's next round and level |
| `RoundDone { job, round, fumbles, play_s }` | The body is docked at that job; `round` is its next; `play_s` is at least the level's `min_round_s` (data) | Lands the round's share less the fumbles' at once; a landed round in the snapshot |
| `Fumble { job, round }` | As above | Takes 5% back, applies the hazard (section 5) to everyone in its radius |
| `Undock` | Nothing | Stands the body, closes the face if no one else is docked |

`min_round_s` is a floor no human beats (about 40% of a steady hand's time at that level, measured in the mockups'
play), so a modified client cannot land six rounds in a second. A refused round is not landed; the client is told
and plays it again. Recommendation taken (ask only with screenshots): the floor, rather than the server re-running
the game, which would mean the game's whole input stream on the network.

**Network cost.** A `RoundDone` is 12 bytes and a `Fumble` 6, reliable; there are a few a minute. The job's
integrity is already in the snapshot (`damage-control`); the fixture state carries the open face (`ship-interactables`).

## 7. Two at one job

A system with `workers: 2` has two repair points side by side. Each docked player plays their own rounds: each
takes the next round left and its level, and lands its own share. Two players on a damaged pump's 3 rounds play
rounds 1 and 2 at once, and whoever finishes first plays round 3. A bot helping adds its rating's rate between
landed rounds, with no panel.

## 8. Gravity off, and outside

- **Gravity off** (`crew-on-deck` 11). Docking needs the point's handhold: the body clips to it and floats in the
  posture's place. The panel is the same, with the view drifting a little on the hold. Loose parts float, so a
  fumble that drops a part (the fighter's plug) sends it drifting in the room.
- **Outside** (the warp pylons). The EVA walk along the handholds is the pylon game's own first move; docked at the
  segment the frame looks along the hull, with the stars and the ship behind the panel.

## 9. The mockup

`docs/mockups/repairs-on-deck.html`, a three.js page (CLAUDE.md 11), presenting this change:

- Engineering from the one layout (`shipkit.js`), lit by its own fixtures, normal and red alert, with the Pi 5 cost
  shown.
- First person: walk up to the coolant pump and the coolant valve board; the prompt; Use docks.
- Docked: the camera's cut-in, the cover's screws turned out in 3D and the plate lifted off (section 3a), the real
  mini-game (`repairs/pump.js` and `repairs/coolant.js` through `kit.js`, its own 2D cover skipped) drawn on the panel
  over the darkened live room, the look-up hold, leaving and resuming, and the cover screwed back on at the end.
- A landed round: the lamp steps and the machine's effects change; a fumble: the hazard's flash in the room.
- Shots of each in `docs/screenshots/repairs-on-deck/`.

## 10. The Pi 5 budget

| What | Cost |
| --- | --- |
| The docked view | The main camera moved; no second render target. One full-screen quad darkens it |
| The panel | The UI layer, as a console: at most 40 draw calls, the same budget as `bridge-stations`' consoles |
| Service faces | The open state's inner detail: at most 400 triangles a prop, drawn only while open, inside the prop's budget (`ship-props`) |
| Effects | Sparks, smoke and puffs from one particle batch, at most 200 particles across the ship |
| The kit's work light | One of the runtime lights over the bake, at most 2 at once ship-wide (`light-baking`) |
| Network | Section 6: a few reliable messages a minute |

## 11. Data

- Each prop's `uses_m` gains the `repair` kind with section 2's fields, written by its Blender build.
- `data/ships/tern/repairs.json`: each job's game and `workers`, the rounds by state (`repair-minigames` 2), each
  game's levels (`repair-minigames` 1a) and `min_round_s` by level, the hazards' radius and damage.
- `kit.json` gains the parts bins and the kit's `pouch_parts: 3`.

## 12. Order of work

1. The write-up (this) and the mockup (section 9), for the owner to judge.
2. The `repair` use point and service faces on the engineering props first (pump, reactor ring, switchboard), then
   the rest.
3. Docking, the panel over the live view and the intents in the engine (`sc-core` owns the job and the checks,
   `sc-client` the docked camera and the UI).
4. The games, one at a time, in the UI layer, each against its mockup.

## Risks / Trade-offs

- **A private screen hides the room.** The live view around the panel, look up, and the push-out rules keep the
  repairer in the fight; if it still feels cut off, the panel can shrink to half the screen (a setting).
- **Docked bodies are easy targets.** Intended: a repair under fire is a risk the crew covers.
- **The 30 s resume is per player.** A second player cannot finish someone else's half-played round; they take the
  next round instead. Simpler and fair.
- **`min_round_s` must be measured, not guessed**: from the mockups' played rounds at each level, before it is data.
