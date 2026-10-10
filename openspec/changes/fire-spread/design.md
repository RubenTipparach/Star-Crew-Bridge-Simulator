# Design: fire that spreads across a room, and putting it out

## Context

`damage-control` section 3 owns fire, and `docs/mockups/lib/shipsystems.js` runs it per room: the heat release
`hrr` grows as `dQ/dt = 2 sqrt(alpha Q)` (the t-squared "fast" class, `alpha = 0.047 kW/s^2`) toward a ceiling of
`250 kW/m^2 x floor x f_O2`, burns 0.419 MJ per mol of oxygen, makes carbon dioxide and smoke, heats the room's air
and damages what is in it above 150 C. A neighbour ignites when its air passes 300 C. Extinguishers cut `hrr` by
60 kW a second each for 15 s, water mist by 100 kW a second, inert gas and venting starve it through the room's
oxygen and pressure. `crew-on-deck` section 14 gives the extinguisher its body: 9 kg, 6 kg of agent over 15 s, a
30 degree cone reaching 3.0 m.

This change keeps every one of those rules and adds the one thing the room model lacks: where in the room the fire
is. It changes how `hrr` grows and where an extinguisher's cut lands; it changes nothing about what `hrr` does once
it exists.

## 1. Cells

- **The grid.** Each room's floor is cut into square cells of 0.5 m (`fire.cells.size_m`), on the deck's own x and z,
  so cells line up across a doorway. A cell belongs to a room when its centre is on the room's floor polygon (the
  layout's prisms, as `shipkit` builds them). The Tern's floors make 10,074 cells (measured by `firespread.js` on layout v2; the first estimate said 6,400), the
  quarters 373 (its 92 m^2; the first estimate's 110 was wrong).
- **Fuel.** A cell takes the fuel class of what stands on it, from the props and fixtures that cover its centre
  (`fire.cells.classes`), else the room's `bare` class:

  | Class | What | Fuel MJ/m^2 | Ignites (dose, s at full exposure) | Peak kW/m^2 |
  | --- | --- | ---: | ---: | ---: |
  | `bedding` | bunks, couches, the medbay's beds | 500 | 36 | 250 |
  | `furniture` | tables, chairs, desks, lockers | 350 | 50 | 250 |
  | `cables` | cable trays, switchboard cabinets, consoles | 300 | 60 | 250 |
  | `stores` | crates, racks, the magazine's missiles | 400 | 70 | 250 |
  | `machinery` | pumps, generators, engines | 150 | 120 | 250 |
  | `bare` | deck plating and its coatings | the remainder | 110 | 250 |

  The doses are the harness's tuned figures (section 4; the first draft had 12, 25, 30, 35, 60 and 70 s, which grew
  the quarters' fire to 1 MW in 60 s). Which prop is which class is `fire.cells.props`; what stands where is the
  mockups' furnishing (`crew_rooms.json`, the medbay's beds, the layout's lockers), through `FireSpread.placements`.

  The room's total fuel stays `fuel_mj_per_m2[room] x floor` (`damage-control` 3): the `bare` cells take what the
  furnished ones leave, never below 20 MJ/m^2, so a room burns the energy the table was measured with.
- **State.** Each cell holds its heat release `q` (W), its fuel left (J), its dose (s of exposure), and a
  suppression timer. It is unburnt, burning, knocked down (fuel left, `q` 0, smouldering), out (fuel left, cold) or burnt out (no
  fuel); a cell that ever burned is charred. 6,400 cells
  of 16 bytes are 100 KB; only rooms with a burning cell or a hot layer are stepped.

## 2. Burning and spreading

Per step (`dt`, the room model's step):

- **A burning cell** rises toward `peak x f_O2 x area` with a 10 s time constant (`fire.cells.rise_s`) and spends its
  fuel at `q`. Its fuel gone, it is burnt out.
- **Dose.** An unburnt or knocked-down cell (its suppression timer at 0) gathers dose at
  `sum over burning neighbours (q_n / q_peak_cell) x w(d)` plus `layer(T)`, where the neighbours are the cells within
  1.0 m, `w` is 1 for an edge neighbour, 0.7 for a corner one and 0.35 at 1.0 m, and `layer(T)` is the room's hot
  layer's preheat: 0 below 100 C, rising linearly to 1 at 300 C. It ignites when its dose passes its class's figure.
  Dose cools at 2% a second when nothing feeds it, so a passing flicker does not ignite a bunk a minute later.
- **Flashover.** At 300 C room air (`autoignition_k`, the rule that already spreads fire between rooms) every cell
  with fuel ignites at once: the room's ceiling, already in the room model, then holds it.
- **The room's heat release** is the sum of its cells' `q`. The oxygen cap, the ceiling and the decay above it stay
  the room model's: when the room caps `hrr` (half its oxygen in a step, or `f_O2` falling), every burning cell is
  scaled by the same factor, so a starved room's flames shrink everywhere together.
- **Spread between rooms** is unchanged (open doors carry the hot layer; a room ignites at 300 C). The new fire
  seeds at the cell nearest the portal whose flow brought the heat in, or for a hit at the cell under the hit.
- **A seed** of `seed_kw` (50 kW) ignites the cell at the point and, if one cell's peak cannot hold it, its nearest
  neighbours until their peaks can, so a seed is the same heat release it is today.

## 3. What a player sees

- **Flames** stand on burning cells, as tall as their `q` (0.3 m at ignition, 1.8 m at peak), leaning with the
  room's flow toward an open door. Knocked-down cells smoulder (embers, no flame); burnt-out cells are charred.
- **The hot layer and smoke** gather under the ceiling and come down as the room's smoke rises: the layer's depth
  is the room's smoke over its volume, so a player standing up in a smoky room is in the dark, and crouching keeps
  their eyes under it. Visibility falls with the smoke, the same number that impairs the crew (`life-support`).
- **Heat** reaches a player from the cells near them and from the layer: the screen's edge glows, and the room
  model's harm applies as it does today.

## 4. Calibration

The cell model replaces the room's `2 sqrt(alpha Q)` growth, so it has to grow the same way where nothing aims at
it. `tools/mockups/fire_cases.mjs` (a measurement instrument, CLAUDE.md 4) runs `damage-control`'s cases through
`shipsystems.js` with the cell model on, headless, and prints the table's columns beside the current ones.

- **The target**: the quarters, door shut, 50 kW seed: 1 MW at 113 s within 10%, peak heat release, peak air
  temperature and the time out within 15%. The dose figures and the neighbour weights above are the knobs; the
  harness tunes them once, and they are data (`fire.cells`).
- **Met** (2026-10-09, `fire-cases.md`), with the seed on the quarters' forward-wall bunk: 1 MW at 113 s (0%), peak
  3.48 MW against 3.71 (-6%), peak air 295 C against 307 (-4%), out at 284 s against 321 (-12%). The neighbour
  weights stayed as designed (1, 0.7, 0.35); the knobs that moved were every class's dose (about doubled) and the hot
  layer's preheat, which now starts at 50 C and is full at 180 C (it was 100 C to 300 C). With the first preheat
  range no set of doses passed: a fire slow enough to reach 1 MW at 113 s then peaked near 2 MW, because the room
  model's growth ignores oxygen until its ceiling and the cells' does not, so the cells must involve the whole floor
  by the time the oxygen falls. Peak heat release is the tightest of the four (within 6%).
- **The rest of the table** is reported, not tuned: a furnished room now burns faster where its bedding is and
  slower on bare deck, which is the point. Where a case moves by more than 15%, the design says so beside it, and
  `damage-control`'s table is replaced by the harness's output when the engine takes this model.
- **Suppression cases** are run with the extinguisher aimed by a script that sweeps the nearest burning cells (the
  competent player) and again aimed at the room's centre (the careless one), so the table says what aim is worth.

**A fire that is out stays out** (owner, 2026-10-09: "The cells in firefighting never get fully extinguished. You
should fix that. Leave charred spots after the fire goes out. That should represent that room has damaged there"). A
knocked-down cell with no burning neighbour for `fire.cells.smoulder_s` (30 s) after its agent wears off is **out**:
cold, charred, its fuel left, lit again only by a flame beside it or a flashover. The hot layer preheats only unburnt
cells, and only while something in the room burns. With that, the cell model's late suppression matches the room
model's table (two extinguishers at 120 s out at 130 s, the table's 129; `fire-cases.md`). Every cell that ever burned
stays **charred**: the floor shows where the fire was, and `FireSpread.damage(room)` (charred cells, area, share of the
floor) is the room's fire damage, which the damage control map shows (`ship-plan-view` 6) and a refit clears
(`hull-repair` 3, at the plating rate, the lowest priority).

## 5. Putting it out

- **The extinguisher** discharges its 6 kg over 15 s while the trigger is held (0.4 kg a second), in a 30 degree
  cone reaching 3.0 m from the nozzle (`crew-on-deck` 14). The cone lands on the floor where it is pointed; the
  cells it covers within reach are its footprint.
- **The cut**: 60 kW a second (`extinguisher.hrr_cut_kw_per_s`), shared among the burning cells in the footprint in
  proportion to their `q`. Cells outside it are not touched. Pointed at nothing burning, the agent is wasted. Two
  extinguishers on one fire add, as today.
- **Knocked down**: a cell whose `q` falls below 1 kW goes out with its fuel left and a suppression timer of 20 s
  (`fire.cells.agent_s`): the agent on it, during which it gathers no dose. After that it can reignite from a
  burning neighbour or the hot layer, which is a reflash. So the way to win is the fire fighter's: hit the base,
  work from the near edge, and leave nothing burning behind the line.
- **Water mist** cuts 100 kW a second across every burning cell in the room in proportion to `q`, and its cooling
  holds the layer under 100 C, so knocked-down cells stay down while it runs. **Inert gas** and **venting** act on
  the room's oxygen and pressure: `f_O2` falls, every cell's peak with it, and the fire dies everywhere at once.
- **Starvation** is the same: a sealed room's oxygen falls and its flames shrink together.

## 6. The mockup: `docs/mockups/fire.html`

One deck of the Tern (deck B: the quarters, the mess, the medbay, damage control and the spine corridor between them,
and the hangar beside them, the nearest room with water mist, so the board can show mist acting),
built from the one layout by `shipkit.js`, lit by its own fixtures in the normal and red-alert states (the alarm
turns red alert on), walked by `shipwalk.js`, its air and fire run by `shipsystems.js` with `firespread.js`.

- **Walk** (first person, mouse and keys or touch): take an extinguisher from a room's safety point or the damage
  control locker (E; `safety-points`), walk
  to the fire, hold the left button (or the spray button on a touch screen) to discharge. The spray is drawn, and
  its footprint is a ring on the floor. At rest the screen shows three pictures and no words: the agent left (a
  ring on the extinguisher), the room's heat and smoke (a fill), your health.
- **Board** (from above, the damage control board's view): the deck cut away above the floor, every cell coloured by
  its state, the hot layer as a tint, each door a toggle, and per room the means it has (vent, mist, inert gas).
  Time runs at 1x, 4x or 16x. Click a cell to set a fire; scenario buttons start the table's cases.
- **Both** show the room's numbers on hover only (heat release, air C, oxygen, smoke), and the cost on the Pi 5:
  triangles and draw calls from `renderer.info`. A desktop frame rate is not a Pi measurement.

## 6a. Venting: the captain's call

The owner, 2026-10-08: "The captain can also vent a room of oxygen but all crew members will take damage as oxygen
and heat drops". This answers `bridge-stations`' open "who may vent" (section 10.9 leaves it to `damage-control`).

- **Who.** The captain vents, from the captain's console, any room with a vent (all of them). It is a guarded
  control (`bridge-stations` 8.5: arm, then fire within 3 s). The damage control board can ask for it (the request
  lights on the captain's console) but cannot vent on its own, and automation never vents (`damage-control` 7).
- **What the captain sees first.** Arming shows the room on the ship plan with everyone in it, by name, and the time
  the room takes to empty (from the same solve that empties it: the preview is the outcome, CLAUDE.md 6.1).
- **The warning.** Firing shuts the room's doors and dampers, sounds the room's klaxon and strobes its lights red for
  5 s, then opens the dump (`damage-control` 4's venting: the room to 20 kPa in 24 s, engineering in 82 s). For those
  5 s the doors open from inside on a press, so the crew can run; after that they hold against the pressure.
- **Everyone in the room takes damage as the air goes**, by `life-support`'s harm table (section 8), which this
  change proposes to extend so the fall is felt as health lost, not only as a slide toward unconsciousness:

  | What falls | Harm (proposed) | From |
  | --- | --- | --- |
  | Oxygen | 0 HP/s above 10.6 kPa of oxygen, rising in a straight line to 2.0 HP/s at 6.3 kPa and below | New; the hypoxia dose toward unconsciousness stays as it is. It begins at 10.6 kPa, where `life-support`'s unconsciousness rule does, so a room flooded with inert gas (about 12.7 kPa) impairs but does not kill (recommendation taken, ask only with screenshots, 2026-10-09) |
  | Heat | 0 HP/s above 5 C, rising 0.04 HP/s per kelvin below it (1.0 HP/s at -20 C) | Replaces the cold row's 0.2 HP/s below -20 C |
  | Pressure | As today: knocked down (10 HP) by a fall faster than 50 kPa/s; dead after 90 s below 6.3 kPa | Unchanged |

  The air cools as it expands out of the dump, so oxygen and heat fall together. A crew member who stays in a
  venting room from the first second should be down (0 HP) within about a minute, so the warning matters and staying
  is a real cost. Measured by the fire harness (`fire-cases.md`): from the captain's command, the dump opens at 5 s,
  oxygen passes 6.3 kPa at 23 s and the air 5 C at 26 s; a crew member who stays is incapacitated at 57 s, at 0 HP at
  66 s and critical at 87 s.
- **The fire goes out** below 20 kPa, as today, with every cell's flame shrinking together as oxygen falls (5).
- **Refilling** is `life-support`'s: the room is refilled from the reserve through its vent when the captain closes
  the dump, the medbay in 170 s.

Recommendation taken (ask only with screenshots): the 5 s warning with the doors opening from inside, the damage
board's request without its own vent, and the harm rates above. They are data in `atmosphere.json` `crew_effects`.

## 7. The Pi 5 budget

| Item | Cost | Against |
| --- | --- | --- |
| Cells, the whole ship | 6,400 x 16 B = 100 KB on the server; a room is stepped only while it burns or is hot | Server memory |
| Simulation | At most a few rooms burning: about 500 cells x 9 neighbours a step, under 0.1 ms on a Cortex-A76 (estimate, to be measured by the probe) | Server step |
| Network | A burning room sends its cells' states as 2 bits each and `q` as one byte each on change, at most 4 Hz: about 140 B a second for the quarters | `netcode-and-sessions` budget |
| Flames | One instanced draw of billboards for the room in view, at most 256 quads (512 triangles) | Client draw calls and triangles |
| Smoke and layer | One translucent quad a room, plus the fog the renderer already has | Client fill rate |

These are estimates against `engine-stack`'s table until the Pi probe measures them.

## Risks / Trade-offs

- **The table moves.** Furnished cells burn faster and bare deck slower; a room's growth now depends on where the
  seed lands. That is the gameplay this change is for, and the harness says by how much each case moves.
- **Aim makes suppression harder than today's rule** for a careless player; the harness's two scripted players say
  how much. If the owner finds it too hard, the knob is the cone's width, not the cut.
- **Cells do not see height.** Flames on a top bunk and on the deck are one cell; the hot layer stands in for upward
  spread. A shelf-by-shelf model is not worth its cost on a Pi.
