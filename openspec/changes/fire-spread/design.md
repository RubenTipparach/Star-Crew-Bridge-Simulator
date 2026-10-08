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
  layout's prisms, as `shipkit` builds them). The Tern's floors make about 6,400 cells; the quarters about 110.
- **Fuel.** A cell takes the fuel class of what stands on it, from the props and fixtures that cover its centre
  (`fire.cells.classes`), else the room's `bare` class:

  | Class | What | Fuel MJ/m^2 | Ignites (dose, s at full exposure) | Peak kW/m^2 |
  | --- | --- | ---: | ---: | ---: |
  | `bedding` | bunks, couches, the medbay's beds | 500 | 12 | 250 |
  | `furniture` | tables, chairs, desks, lockers | 350 | 25 | 250 |
  | `cables` | cable trays, switchboard cabinets, consoles | 300 | 30 | 250 |
  | `stores` | crates, racks, the magazine's missiles | 400 | 35 | 250 |
  | `machinery` | pumps, generators, engines | 150 | 60 | 250 |
  | `bare` | deck plating and its coatings | the remainder | 70 | 250 |

  The room's total fuel stays `fuel_mj_per_m2[room] x floor` (`damage-control` 3): the `bare` cells take what the
  furnished ones leave, never below 20 MJ/m^2, so a room burns the energy the table was measured with.
- **State.** Each cell holds its heat release `q` (W), its fuel left (J), its dose (s of exposure), and a
  suppression timer. It is unburnt, burning, knocked down (fuel left, `q` 0) or burnt out (no fuel). 6,400 cells
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
- **The rest of the table** is reported, not tuned: a furnished room now burns faster where its bedding is and
  slower on bare deck, which is the point. Where a case moves by more than 15%, the design says so beside it, and
  `damage-control`'s table is replaced by the harness's output when the engine takes this model.
- **Suppression cases** are run with the extinguisher aimed by a script that sweeps the nearest burning cells (the
  competent player) and again aimed at the room's centre (the careless one), so the table says what aim is worth.

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

One deck of the Tern (deck B: the quarters, the mess, the medbay, damage control and the spine corridor between them),
built from the one layout by `shipkit.js`, lit by its own fixtures in the normal and red-alert states (the alarm
turns red alert on), walked by `shipwalk.js`, its air and fire run by `shipsystems.js` with `firespread.js`.

- **Walk** (first person, mouse and keys or touch): take an extinguisher from the damage control locker (E), walk
  to the fire, hold the left button (or the spray button on a touch screen) to discharge. The spray is drawn, and
  its footprint is a ring on the floor. At rest the screen shows three pictures and no words: the agent left (a
  ring on the extinguisher), the room's heat and smoke (a fill), your health.
- **Board** (from above, the damage control board's view): the deck cut away above the floor, every cell coloured by
  its state, the hot layer as a tint, each door a toggle, and per room the means it has (vent, mist, inert gas).
  Time runs at 1x, 4x or 16x. Click a cell to set a fire; scenario buttons start the table's cases.
- **Both** show the room's numbers on hover only (heat release, air C, oxygen, smoke), and the cost on the Pi 5:
  triangles and draw calls from `renderer.info`. A desktop frame rate is not a Pi measurement.

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
