# Design: life support

Status: **proposed** (2026-10-04). Nothing here is built. The data file
`data/ships/tern/atmosphere.json` is proposed by this change (its `fire` block by
`damage-control`); the systems mockup reads it. Every table of results below was computed by
running the proposed library `docs/mockups/lib/shipsystems.js` headless in node 22 on the proposed
data (scratch scripts, not committed): they are results of the design's formulas, not engine
measurements. Power draws are `power-grid`'s; budgets are against `engine-stack` section 5.

## Context

The Tern holds 8,858.8 m^3 of air in 30 compartments (`python3 tools/layout_check.py`), from the
15.6 m^3 turret pods to engineering's 2,520 m^3 and the hangar's 1,698 m^3. Its 40 portals are 23
doors, 4 pressure doors (the two launch bays, the two airlock doors), 5 hatches, 2 ladders, a hoist,
3 bay doors to space and 2 bridge windows. The plant sits in life support (POI 22, deck C): oxygen
generator, CO2 scrubbers, air handler, thermal control. Reserve gas is in cargo (23), the bay pumps
and their receiver in the hangar (15), the airlock (24) off cargo.

From the other changes: `shuttle-bay-and-fighters` assumed a 30 s launch bay pump-down to 1 kPa, a
120 s hangar pump-down and 25 s repressurizations, "assumed, see life-support"; `bridge-stations`
previews "time to pump a bay down to 1 kPa or up to 101 kPa" through `life_support::time_to_pressure`;
`reference-ship-tern` gave this change "the endurance numbers" for a lost life support room.

## Goals / Non-Goals

**Goals:**
- Air that is conserved and physical: moles, pressure, temperature, composition, moved by pressure
  through openings of known size.
- Numbers a crew can act on: how long until a room is unbreathable, how long a pump-down takes, how
  much reserve a refill costs.
- Stable at 10 Hz for every case, including a 2.2 m^2 hole in a 15.6 m^3 pod.
- One model for every compartment, bay and airlock (CLAUDE.md 6.1).

**Non-Goals:**
- Computational fluid dynamics inside a room: a compartment is one well-mixed volume.
- Fire's growth and suppression rules (`damage-control`, which this model carries the gases and
  heat of); suits, carrying and movement (`crew-on-deck`).
- Water, food and waste loops beyond the oxygen generator's water use.

## Decisions

### 1. The state

Each node (30 compartments and the duct) holds moles of four species, oxygen, nitrogen, carbon
dioxide and smoke, and an internal energy `U`. Space is a node held at 0 Pa.

```text
n      = n_O2 + n_N2 + n_CO2 + n_smoke                      mol
C_room = n x c_v + 4,000 J/K per m^3 x V                    the fittings' heat capacity dominates
T      = U / C_room                                          K
P      = n R T / V                                           Pa, R = 8.314 J/(mol K)
```

`c_v` is 20.79 J/(mol K), `c_p` 29.10 J/(mol K), `gamma` 1.4 (diatomic air; CO2 and smoke are a
small fraction). Standard air is 101.3 kPa at 294.15 K, 20.9% oxygen, 79.06% nitrogen, 0.04% CO2:
the ship holds about 368,000 mol (10.7 t) of air. The fittings' 4,000 J/K per m^3 (decks, consoles,
bulkheads) is why decompression cools a room only to about -1 C rather than far below.

### 2. One compartment graph

Everything that moves air is a link in the one compartment graph (CLAUDE.md 7):

| Link | Count | Area | Discharge coefficient |
| --- | ---: | --- | ---: |
| Doors, pressure doors, hatches, ladders, the hoist | 33 | The layout's `size_m` | 0.62-0.65 |
| Bay doors to space | 3 | 43.2 m^2 (each drop door), 72.0 m^2 (the pad door) | 0.7 |
| Windows | 2 | Closed; a hit can make them a breach | 0.6 |
| Vents, room to duct | 30 | 0.0006 m^2 per m^3 of room, 0.02-0.6 m^2 | 0.6 |
| Overboard dump, duct to space | 1 | 0.5 m^2, normally shut | 0.6 |
| Bay vent valves, bay to space | 3 | 0.1 m^2 | 0.6 |
| Airlock equalizing valve, cargo to airlock | 1 | 0.02 m^2 | 0.6 |
| Breaches | as made | `damage-control` sizes them | 0.6 |

The duct, the vents and the valves are proposed additions to the graph
(`atmosphere.json` `graph_additions`); on acceptance they move into `layout.json` (section 16).

### 3. Doors keep the ship in compartments

**Closed by default.** Doors, hatches and ladder hatches are shut unless a crew member is passing
(they open on approach, `reference-ship-tern`'s "ordinary doors open on approach (0 s)") or the
damage control board holds them open. Every compartment is therefore its own pressure boundary: a
breach empties one room, not the ship. Pressure doors (launch bays, airlock) never open on approach;
they open on a command.

**Interlock.** A door does not open on approach, or on an ordinary command, across more than 20 kPa
of difference. The board, or a crew member holding the door's override for 2 s (`crew-on-deck`), can
open it anyway: that is how a crew member is pulled out of a breached room, at the cost of the air
that follows.

**Self-closing.** A door between two compartments closes itself when either side is below 85 kPa and
falling faster than 1 kPa/s, unless the board holds it. Doors to space are commanded, never
automatic. Travel times: doors and hatches 1.5 s, pressure doors 3 s, the hoist 2 s, drop doors 4 s,
the pad door 8 s.

The first rule written was "close when one side is below 85 kPa and the other above": with a door
open, a breach drops both sides together and the rule never fired. Falling, not lopsided, is the
alarm. Measured on a 1 m^2 breach in the quarters with its door open (a crew member passing): the
door shut itself 2.4 s later with the main corridor at 62.9 kPa; the corridor's damper then refilled
it from the duct; 399 kg of air was lost. With the door held open by the board, the corridor emptied
too and 548 kg was lost.

### 4. Flow through openings

For an open link of area `A` (times its opening fraction) between an upstream node at `P_u, T_u, M_u`
and a downstream node at `P_d`:

```text
r        = P_d / P_u
psi(r)   = sqrt(gamma) (2 / (gamma + 1))^((gamma + 1) / (2 (gamma - 1)))      if r <= 0.528 (choked)
         = sqrt(2 gamma / (gamma - 1) (r^(2/gamma) - r^((gamma + 1)/gamma)))  otherwise
n_dot    = C_d A P_u psi(r) / sqrt(M_u R T_u)                                   mol/s
```

Below 20 Pa of difference the flow is taken as linear in the difference, so the equation has no
singular slope at zero. A choked 1 m^2 hole at 101.3 kPa passes about 4,900 mol/s (142 kg/s).

**The step is implicit.** A 2.2 m^2 hole in a 15.6 m^3 pod empties it with a time constant of about
0.06 s, shorter than the 0.1 s sub-step: an explicit step would remove more gas than the pod holds.
So each sub-step:

1. Computes every open link's secant conductance `G = n_dot / delta_P` from the start-of-step state
   (mol/(s Pa)).
2. Solves for the end-of-step pressures `P'`: `(V_i / (R T_i)) P'_i + dt sum_j G_ij (P'_i - P'_j) =
   n_i`, with space fixed at 0 Pa. The matrix is symmetric positive definite; at 31 nodes a dense
   Cholesky factorization is a few thousand operations. Negative results are clamped to zero.
3. Computes each link's moles over the step from the new pressures, `F = dt G (P'_a - P'_b)`.
4. Moves gas **upwind**, visiting nodes from the highest new pressure to the lowest: each node sends
   its outflows in proportion from what it holds plus what has already arrived, never more, carrying
   its composition and `c_p T` per mole. Nothing can go negative.

This is backward Euler on a linearized network: unconditionally stable, monotone (a draining room's
pressure only falls; across 90 decompression runs, every compartment at 0.1, 1 and 2.2 m^2,
pressure fell monotonically down to 1 Pa, below which the last traces rewarm slightly from the
fittings), and accurate to the sub-step. Against a 1 kHz reference (1 ms steps, the same model):

| Case | 10 Hz: 16 kPa O2 / 50 kPa / 6.3 kPa, s | 1 kHz, s | Coldest air, 10 Hz / 1 kHz |
| --- | --- | --- | --- |
| Hangar, 1 m^2 | 4.1 / 10.0 / 40 | 4.0 / 9.8 / 40 | -0.6 / -0.6 C |
| Port launch bay, 1 m^2 | 0.7 / 1.6 / 5.7 | 0.6 / 1.4 / 5.5 | -0.9 / -0.8 C |
| Medbay, 0.1 m^2 | 3.5 / 8.3 / 33 | 3.4 / 8.2 / 32 | 0.8 / 0.8 C |
| Bridge, 2.2 m^2 | 0.6 / 1.4 / 5.3 | 0.5 / 1.3 / 5.0 | -0.9 / -0.8 C |
| Port turret pod, 2.2 m^2 | 0.1 / 0.1 / 0.4 | 0.0 / 0.0 / 0.2 | -2.8 / -0.6 C |

The sub-step is within 0.3 s of the reference everywhere; the pod's coldest air differs because a
0.4 s event is four sub-steps. A crew member's fate in a pod blown open is decided by pressure
(Armstrong's limit in under half a second), not temperature.

### 5. Mixing and ventilation

Pressure flow moves gas only where pressures differ. Two more exchanges move composition and heat at
equal moles both ways (pressure unchanged):

- **Open doorways**: `q = 0.08 m^3/s per m^2 x A + 0.22 x A sqrt(g H delta_T / T)` (a small base
  rate plus the buoyant two-way flow a hot room drives through a door of height `H`), with `g` the
  gravity generator's field (`power-grid`): in free fall, hot smoke does not rise through a door.
  Capped at 25% of the smaller room's gas per sub-step.
- **Ventilation**: every room trades air with the duct at its design air changes times the fans'
  supply ratio. Air changes per hour: rooms 10, corridors 8, bays 4, crawlspaces 6, the airlock 4,
  pods 20, engineering 15 (its heat). The bridge's 462 m^3 at 10 an hour is 1.28 m^3/s.

### 6. Dampers

Each vent has a damper. It shuts when:

- **its net flow is excessive**: more than 0.02% of the room's gas a second (a leak, or a fire's
  expansion; ordinary heating moves about a hundred times less). The damper latches shut;
- the room is below 85 kPa, the room's smoke is above 2,000 ppm, the duct is below 85 kPa, or the
  fans are stopped.

A latched damper waits until its room has stopped falling (no faster than 10 Pa/s) for 30 s, then
reopens if the room is within 3 kPa of the duct, or **refills** it from the duct if the room is above
10 kPa (the damper held open until the room reaches 100.3 kPa, abandoned if the room falls while
refilling: it is still leaking). A room below 10 kPa stays isolated until the board refills it,
because a room at vacuum may still have its hole.

This was the hardest rule to get right, and the simulation showed why. With dampers that shut only
below 85 kPa, a 0.1 m^2 breach in the hangar was fed through its 0.6 m^2 vent by the whole ship:
1,900 kg lost and the duct drawn down to 91 kPa. With a rule on the difference to the duct, the
bridge's 0.28 m^2 vent kept its breached room within 2.5 kPa of the duct, below any sensible
threshold. Excess flow, scaled to the room, catches both; it shuts every breached room's damper
within 2 s, from 0.01 m^2 in engineering to 2.2 m^2 in a pod, and never trips in 20 minutes of cruise
or combat.

### 7. The plant

| Unit | Rule | Rating | Power (`power-grid`) |
| --- | --- | --- | --- |
| Oxygen generator | Electrolysis; holds the duct's oxygen partial pressure at 21.2 kPa (proportional over a 1 kPa band) | 1.25 mol/s, 0.8 MJ per mol of oxygen (water's 0.57 MJ at 70% efficiency), 0.036 kg of water per mol | Up to 1.0 MW, 1.5 MW at overdrive |
| CO2 scrubbers | Treat 4 m^3/s of duct air: 80% of its CO2 (at most 0.6 mol/s) and 90% of its smoke | Regenerative; no consumables | 0.25 MW |
| Thermal control | Conditions the supply air to 20 C | 1.2 MW cooling, 0.6 MW heating | 0.15 MW standing, 0.75 MW at full heat |
| Air handler | Moves the ventilation exchange | | 0.08 MW |
| Make-up | Holds the duct at 101.3 kPa from reserve bottles when it falls below 100.3 kPa | 40 mol/s at most | none (stored pressure) |

**Stores**: reserve nitrogen 75,000 mol (2,100 kg) and oxygen 20,000 mol (640 kg) in cargo; the bay
receiver, a 60 m^3 tank in the hangar rated to 3,000 kPa. Make-up stops if the duct itself falls
below 60 kPa (a leak the reserves would only feed), except while a compartment is being refilled.

**At cruise with eight crew working** (30 minutes): every room at 101.2 kPa, oxygen 21.16 kPa, CO2
0.02-0.03%; air 16-27 C (the launch bays coolest, with four air changes an hour and much hull; the
medbay warmest, from its beds' 8 kW). The generator makes 0.041 mol/s of oxygen (32.6 kW) and uses
2.2 kg of water in 30 minutes; the scrubbers take 0.028 mol/s of CO2; thermal control removes
110 kW.

### 8. Metabolism

Per crew member at rest: 0.304 mmol/s of oxygen (0.84 kg a day), 0.264 mmol/s of CO2 (1.0 kg a day),
100 W of heat; working, 2.5 times the gases and 300 W. Suited crew breathe from the suit.

### 9. The thermal model

Each room gains the heat of its loads, lights and crew (`power-grid` section 10), exchanges heat with
its neighbours through shared bulkheads (5 W/(m^2 K) over the facing area computed from the layout's
boxes, within 0.6 m) and loses heat through the hull to a 250 K skin (0.4 W/(m^2 K) over the
exterior area: the bridge has 288 m^2, engineering 1,012 m^2). The supply air arrives at 20 C.

### 10. What the plant's loss costs, from the simulation

| Lost | With eight crew working | When it matters |
| --- | --- | --- |
| Oxygen generator and scrubbers, fans running | Bridge CO2 0.050 kPa after 1 h, 0.065 kPa after 4 h; oxygen 21.08 kPa after 4 h | Days: 368,000 mol of air dilutes eight people's breath |
| The whole plant, fans too (rooms isolated) | Bridge (four crew) CO2 0.093 kPa after 1 h, 0.251 kPa after 4 h; 32 C after 4 h | Many hours: heat first, CO2 much later |
| Thermal control only, in combat | Turret pods reach 30 C in 8 min; after 1 h the magazine is at 46 C, the pods 41-43 C, the medbay 42 C | Within a mission: 45 C impairs (section 12) |

Real metabolism makes a dead plant a slow problem. The fast killers are breaches, fire and smoke,
and heat from equipment without thermal control; question L4 asks whether to accelerate metabolism
for play.

### 11. Breaches and decompression

A breach is a link to space of the area `damage-control` gives it. Every compartment, alone with its
door shut (the default), from 101.3 kPa:

- **16 kPa O2**: oxygen partial pressure below 16 kPa (hypoxia impairment begins).
- **50 kPa**: crew impaired by pressure.
- **6.3 kPa**: Armstrong's limit; body fluids boil, unconscious within seconds, dead after 90 s.
- **Own door**: the compartment's largest door to space opened (time to 50 kPa / to 6.3 kPa).
- **Crew at 1 m^2**: an unsuited crew member inside, time to unconscious / dead.

| POI | Compartment | Volume m^3 | 0.1 m^2: 16 kPa O2 / 50 kPa / 6.3 kPa, s | 1 m^2: same, s | 2.2 m^2 (a door-sized hole): 50 / 6.3 kPa, s | Own door to space | Crew at 1 m^2: unconscious / dead, s | Coldest air at 1 m^2, C |
| ---: | --- | ---: | --- | --- | --- | --- | --- | ---: |
| 1 | Bridge | 462.0 | 11 / 27 / 108 | 1.3 / 3.0 / 11 | 1.4 / 5.3 | none | 15 / 101 | -1 |
| 2 | Captain's ready room | 85.5 | 2.3 / 5.3 / 20 | 0.3 / 0.6 / 2.3 | 0.3 / 1.2 | none | 10 / 92 | -1 |
| 3 | Computer core | 85.5 | 2.3 / 5.3 / 20 | 0.3 / 0.6 / 2.3 | 0.3 / 1.2 | none | 10 / 92 | -1 |
| 4 | Command passage | 120.0 | 3.1 / 7.3 / 28 | 0.4 / 0.8 / 3.1 | 0.4 / 1.6 | none | 11 / 93 | -1 |
| 5 | Dorsal turret access | 48.0 | 1.3 / 3.1 / 12 | 0.2 / 0.4 / 1.4 | 0.2 / 0.7 | none | 9.8 / 91 | -1 |
| 6 | Aft passage | 135.0 | 3.5 / 8.1 / 32 | 0.4 / 0.9 / 3.5 | 0.5 / 1.8 | none | 11 / 94 | -1 |
| 7 | Torpedo room | 210.0 | 5.2 / 12 / 49 | 0.6 / 1.4 / 5.3 | 0.7 / 2.6 | none | 12 / 95 | -1 |
| 8 | Medbay | 138.0 | 3.5 / 8.3 / 33 | 0.4 / 1.0 / 3.6 | 0.5 / 1.8 | none | 11 / 94 | -1 |
| 9 | Damage control | 138.0 | 3.5 / 8.3 / 33 | 0.4 / 1.0 / 3.6 | 0.5 / 1.8 | none | 11 / 94 | -1 |
| 10 | Crew quarters | 232.5 | 5.7 / 14 / 55 | 0.7 / 1.6 / 5.8 | 0.8 / 2.8 | none | 12 / 96 | -1 |
| 11 | Mess | 232.5 | 5.7 / 14 / 55 | 0.7 / 1.6 / 5.8 | 0.8 / 2.8 | none | 12 / 96 | -1 |
| 12 | Port turret access | 234.0 | 5.8 / 14 / 55 | 0.7 / 1.6 / 5.8 | 0.8 / 2.8 | none | 12 / 96 | -1 |
| 13 | Starboard turret access | 234.0 | 5.8 / 14 / 55 | 0.7 / 1.6 / 5.8 | 0.8 / 2.8 | none | 12 / 96 | -1 |
| 14 | Main corridor | 195.0 | 4.9 / 12 / 46 | 0.6 / 1.3 / 4.9 | 0.6 / 2.4 | none | 12 / 95 | -1 |
| 15 | Hangar | 1698.0 | 38 / 97 / 395 | 4.1 / 10.0 / 40 | 4.6 / 18 | `p_hangar_pad` 72.0 m^2: 0.2 / 0.7 | 31 / 130 | -1 |
| 16 | Port launch bay | 231.0 | 5.7 / 14 / 54 | 0.7 / 1.6 / 5.7 | 0.8 / 2.8 | `p_drop_p` 43.2 m^2: 0.1 / 0.3 | 12 / 96 | -1 |
| 17 | Starboard launch bay | 231.0 | 5.7 / 14 / 54 | 0.7 / 1.6 / 5.7 | 0.8 / 2.8 | `p_drop_s` 43.2 m^2: 0.1 / 0.3 | 12 / 96 | -1 |
| 18 | Engineering | 2520.0 | 57 / 143 / 586 | 5.9 / 15 / 59 | 6.7 / 27 | none | 41 / 149 | -0 |
| 19 | Drive section | 288.0 | 7.0 / 17 / 68 | 0.8 / 1.9 / 7.1 | 0.9 / 3.4 | none | 13 / 97 | -1 |
| 20 | Lower corridor | 150.0 | 3.8 / 9.0 / 35 | 0.4 / 1.0 / 3.8 | 0.5 / 1.9 | none | 11 / 94 | -1 |
| 21 | Magazine | 240.0 | 5.9 / 14 / 56 | 0.7 / 1.6 / 6.0 | 0.8 / 2.9 | none | 12 / 96 | -1 |
| 22 | Life support | 325.5 | 7.8 / 19 / 76 | 0.9 / 2.2 / 8.0 | 1.0 / 3.8 | none | 13 / 98 | -1 |
| 23 | Cargo and stores | 325.5 | 7.8 / 19 / 76 | 0.9 / 2.2 / 8.0 | 1.0 / 3.8 | none | 13 / 98 | -1 |
| 24 | Airlock | 24.0 | 0.7 / 1.7 / 6.0 | 0.1 / 0.2 / 0.8 | 0.2 / 0.5 | `p_airlock_outer` 2.2 m^2: 0.2 / 0.4 | 9.5 / 91 | -1 |
| 25 | Shield generator | 103.5 | 2.7 / 6.3 / 25 | 0.3 / 0.7 / 2.8 | 0.4 / 1.4 | none | 11 / 93 | -1 |
| 26 | Forward switchboard | 103.5 | 2.7 / 6.3 / 25 | 0.3 / 0.7 / 2.8 | 0.4 / 1.4 | none | 11 / 93 | -1 |
| 27 | Dorsal turret pod | 15.6 | 0.5 / 1.2 / 4.2 | 0.1 / 0.2 / 0.6 | 0.1 / 0.4 | none | 9.4 / 91 | -1 |
| 28 | Ventral turret pod | 21.9 | 0.7 / 1.6 / 5.6 | 0.1 / 0.2 / 0.8 | 0.2 / 0.4 | none | 9.5 / 91 | -1 |
| 29 | Port turret pod | 15.6 | 0.5 / 1.2 / 4.2 | 0.1 / 0.2 / 0.6 | 0.1 / 0.4 | none | 9.4 / 91 | -1 |
| 30 | Starboard turret pod | 15.6 | 0.5 / 1.2 / 4.2 | 0.1 / 0.2 / 0.6 | 0.1 / 0.4 | none | 9.4 / 91 | -1 |

Times scale with volume over area: the hangar takes 40 s to reach Armstrong's limit through 1 m^2,
a pod 0.6 s. A crew member caught unsuited in a breached room is unconscious in 9-15 s in every room
but the two largest (31 s in the hangar, 41 s in engineering) and dead 90 s after Armstrong's limit.

**Venting deliberately** through the duct (`damage-control`'s fire tool): the board shuts the
room's doors, forces every other damper shut and opens the overboard dump; any room reaches 20 kPa
in 24 s (vents are sized in proportion to volume), engineering in 82 s.

**Refilling** a sealed room from vacuum through its vent: the medbay in 125 s for 142 kg of reserve
gas, the quarters or a launch bay in 213 s for 244 kg. **Engineering cannot be refilled**: it holds
3,040 kg and the reserves 2,740 kg; after a large breach it stays in vacuum until resupply, and the
crew work it suited (question L5).

### 12. Crew effects

From the crew member's room, each sub-step (`atmosphere.json` `crew_effects`; `crew-on-deck` shows
them):

| Effect | Impaired | Toward unconsciousness | Death |
| --- | --- | --- | --- |
| Hypoxia | pO2 below 16 kPa | Time of useful consciousness by pO2: 30 min at 12 kPa, 20 min at 10.6, 5 min at 8.9, 3 min at 7.9, 1 min at 6.3, 15 s at 3.9, 9 s below 3.4 (a dose accumulating `dt / TUC`) | Unconscious for 240 s with pO2 below 10.6 kPa |
| Hypercapnia | pCO2 above 3 kPa | 30 min at 5 kPa, 5 min at 7, 1 min at 10, 20 s at 15 | Unconscious for 300 s with pCO2 above 10 kPa |
| Smoke | | Fractional effective dose (Purser): `sum(ppm x dt) / 60 / 30,000 ppm min` reaching 1 | Dose 2.5 |
| Heat, cold | Air above 45 C or below 5 C | 0.05 HP/s per kelvin above 60 C; 0.2 HP/s below -20 C | 0 HP is unconscious; -50 HP dead |
| Pressure | Below 50 kPa | A drop faster than 50 kPa/s knocks a standing crew member down (10 HP) | 90 s below 6.3 kPa |

Times of useful consciousness follow the published altitude tables (FAA, the US Air Force) in
partial pressure; hypercapnia follows the NASA and OSHA limits; smoke uses Purser's fractional
effective dose. They are cited in the data file's comments when `docs/references.md` gains them.
Recovery: hypoxia dose falls 3.3% a second in good air, hypercapnia 1.7%.

### 13. The bays

**Pump-down.** The bay pumps (in the hangar, `hangar_pumps`, 6 MW at most) move bay air into the
receiver: displacement 24 m^3/s, power by isothermal compression `P = S p ln(p_receiver / p) / 0.6 +
0.1 MW`. They stop at 5 kPa (or a full receiver), when the bay's 0.1 m^2 vent valve opens and the
launch is permitted; the valve takes the rest.

| | Port launch bay (231 m^3, 277 kg) | Hangar (1,698 m^3, 2,038 kg) |
| --- | ---: | ---: |
| To 50 kPa | 6.8 s | 47.9 s |
| To 10 kPa | 21.9 s | 159.5 s |
| **Pumps stop at 5 kPa: launch permitted** | **28.6 s** | **208.4 s** |
| To 1 kPa (vent valve) | 61.4 s | 445.6 s |
| Pump energy, peak power | 68 MJ, 3.71 MW | 835 MJ, 6.00 MW |
| Air kept in the receiver | 97.4% (receiver at 481 kPa) | 95.3% (receiver at 2,832 kPa) |
| Air vented | 14 kg | 106 kg |
| **Repressurize from the receiver** | **15.7 s** | **105.8 s** (plus make-up through the duct) |

**Emergency vent** instead (the drop door or pad door opened with the bay full): the launch bay
passes Armstrong's limit 1.0 s after its door starts to open and loses 277 kg; the hangar 2.8 s and
2,038 kg, 74% of the ship's reserve gas. Through its vent valve alone a launch bay takes 54 s.

The pumps refuse to start while an unsuited crew member is in the bay (`bay_pumps.interlock_unsuited_crew`),
vision pillar 4.

### 14. The airlock

Cycle out: both doors shut, the airlock pump (2 m^3/s into cargo, 0.15 MW) pumps to 5 kPa in 35.7 s
using 3.5 MJ, then the outer door opens in 3 s, venting 1.5 kg. Cycle in: the outer door shuts (3 s),
the 0.02 m^2 equalizing valve fills the airlock from cargo in 13.9 s, the inner door opens.

### 15. Reconciling `shuttle-bay-and-fighters`

| Its assumption | Computed here | Proposal |
| --- | --- | --- |
| Pump-down to 1 kPa in 30 s | 28.6 s to 5 kPa, 61.4 s to 1 kPa | Launch permitted at 5 kPa, the pumps' stop: the drop door vents the last 14 kg. Its sequence time stands (question L2) |
| Repressurize in 25 s | 15.7 s | Shorter |
| Emergency vent in 6 s, 277 kg | Armstrong at 1.0 s after the door starts to open (4 s travel), 277 kg | Aligned |
| Hangar pump-down 120 s; vent 20 s, 2,040 kg | 208 s; Armstrong at 2.8 s, 2,038 kg | The Petrel launch takes 88 s longer (question L3) |
| Hangar doors to the corridors, engineering and galleries close for a Petrel launch | Every door is closed by default and interlocked | No layout change needed |

`bridge-stations` F3 lists the pump-down at "~38 s"; it should read 29 s to launch.

### 16. Console readouts and previews (preview = resolver)

| Where | Readout | Source |
| --- | --- | --- |
| Engineering E5, damage control D1 | Per compartment: pressure kPa, oxygen kPa, CO2 % and kPa, air temperature C, smoke ppm, breaches; the worst compartment | The state |
| Damage control D1 | Time to 50 kPa and to Armstrong's limit for a breached compartment | `life_support::time_to_pressure` on a copy |
| Damage control | A refill's time and reserve gas cost before it is ordered | The same, with the vent forced open |
| Flight ops, bay control | Bay pressure, pump state, receiver kPa; time to launch-permitted and to full pressure | `life_support::time_to_pressure(bay, target)`: the bay's pump model stepped at 1 s on a copy |
| Engineering E5 | Plant rates (oxygen, CO2, heat), reserve gas kg, duct kPa | The state |

The preview steps the same functions as the sub-step, on a copy of one compartment, its pump and the
receiver, at 1 s steps (the implicit flow allows it), until the target or 1,800 s: about 200 small
steps, well under a millisecond.

### 17. The data: `data/ships/tern/atmosphere.json`

| Block | Fields |
| --- | --- |
| `gas` | Gas constant, `c_v`, `c_p`, `gamma`, species and their molar masses |
| `standard_air` | Pressure kPa, temperature K, mole fractions |
| `graph_additions` | The duct (volume, centre), vent sizing (m^2 per m^3, minimum, maximum), the overboard dump |
| `portals` | Discharge coefficient and default state per kind, travel times (and per-portal overrides), self-closing threshold and fall rate, the kinds that self-close, the interlock kPa |
| `flow` | Linear band Pa, door mixing, buoyant coefficient, gravity, mixing cap, minimum pressure |
| `thermal` | Fittings J/(K m^3), hull and bulkhead U values, hull skin K, adjacency gap, initial K |
| `ventilation` | Air changes per hour by kind and overrides; damper thresholds: low pressure, smoke, duct low, trip fraction, reset kPa, retry s, falling rate, automatic refill floor |
| `plant` | Oxygen generator, scrubbers, thermal control, make-up |
| `stores` | Reserve bottles (species, mol), the bay receiver (volume, rating, temperature, initial) |
| `bay_pumps` | Bays served, displacement, efficiency, base MW, stop kPa, vent valve, refill rate, the unsuited-crew interlock |
| `airlock` | Doors, pump, destination, efficiency, stop kPa, equalizing valve |
| `metabolism` | Oxygen and CO2 at rest, work multiplier, heat |
| `crew_effects` | The tables of section 12 |
| `fire` | `damage-control`'s |

Validation as in `power-grid` section 13: unknown keys, non-finite numbers and missing ids stop
startup with the path and field.

### 18. The Pi 5 budget this change spends

**Server CPU**: the atmosphere is part of the systems sub-step measured and estimated in
`power-grid` section 14 (60-240 us on the Pi for the whole sub-step, estimated). Its parts: 75 link
conductances, a 31 x 31 Cholesky solve (about 10,000 floating-point operations), 75 fluxes and an
ordered upwind pass, 31 ventilation exchanges, the plant. The dense solve is chosen over a sparse one
because at 31 nodes it is smaller than the bookkeeping; a ship with over 100 nodes would switch to a
sparse factorization with the same matrix.

**Memory**: 31 nodes x 9 numbers, 75 links x about 20, a 31 x 31 matrix: about 20 kB, allocated at
load. A breach adds a link to a fixed pool of 64.

**Network**: a client showing a compartment panel (damage control, engineering E5) needs each
compartment's pressure, oxygen, CO2, temperature and smoke (2 bytes each, 310 bytes) at 1 Hz, and
5 Hz for compartments changing faster than 1 kPa/s: about 2.5 kbit/s. Every client needs its own
compartment's values at 5 Hz for its avatar's effects (10 bytes). Door states are events.

## Risks / Trade-offs

- **Closed doors make the ship feel shut.** Doors open on approach with no delay, so walking is not
  slowed; question L1 asks with the shot.
- **Real metabolism makes the plant matter little.** Stated, with the fast killers named; question L4.
- **Fittings' heat capacity is a single number per volume.** It is what keeps decompression from
  freezing rooms; a better number would come from `deck-pipeline`'s materials.
- **A well-mixed room hides gradients.** A fire's smoke fills its room at once; a crew member by the
  door and one by the fire breathe the same air. Accepted for the cost.

## Mockup shots

`docs/mockups/systems.html`, screenshots in `docs/screenshots/mockups/`:

| Shot | Shows |
| --- | --- |
| `atmosphere-normal` | Cruise: every room at 101 kPa, two doors open with crew passing, the plant's rates and the table of every compartment |
| `breach-launch-bay` | A 40 MJ hit from below into the port launch bay, 4 s later: a 0.46 m^2 breach, the bay at 41 kPa and falling, a fire dying for want of oxygen, red alert |
| `hangar-pumpdown` | The hangar 100 s into its pump-down: 23 kPa, the air at 4.5 C from expansion, the receiver at 2,284 kPa, the pumps drawing 4.4 MW |

## Open questions

Ids L (life support). Questions with a shot go to the owner's survey with the shot beside them; the
rest are recommendations taken (ask only with screenshots, CLAUDE.md 13).

| Id | Question and the fact it turns on | Options | Recommendation | Shot |
| --- | --- | --- | --- | --- |
| L1 | Are doors shut by default? Shut, a breach costs one room; open, a 1 m^2 breach anywhere would take the whole ship's air (368,000 mol, time constant 74 s) to Armstrong's limit in about 3.5 minutes. | (a) Shut unless someone passes or the board holds them, open on approach. (b) Open, closing on an alarm only. | (a). | `atmosphere-normal` |
| L2 | When may a fighter drop? The pumps stop at 5 kPa after 28.6 s; 1 kPa takes 61 s. | (a) At 5 kPa, venting the last 14 kg. (b) At 1 kPa. | (a): keeps `shuttle-bay-and-fighters`' 44 s sequence. | `hangar-pumpdown` |
| L3 | The hangar takes 208 s to pump down at 6 MW. | (a) Accept: the Petrel is not a combat launch. (b) Pumps twice the size (12 MW, about 105 s). (c) Vent it (2,038 kg, 74% of the reserve). | (a). | `hangar-pumpdown` |
| L4 | Real metabolism makes a dead plant harmless for hours. | (a) Real rates: life support is about breaches, fire, smoke and heat. (b) Metabolism accelerated 20 times so a dead plant matters within a mission. | (a). | none: recommendation taken (ask only with screenshots) |
| L5 | The reserves (2,740 kg) cannot refill engineering (3,040 kg). | (a) As designed: a breached engineering stays in vacuum until resupply. (b) Reserves of 4,000 kg. | (a): the suited engineer is a strong scene, and the debrief resupplies. | none: recommendation taken (ask only with screenshots) |
| L6 | The door interlock: 20 kPa, with an override that costs air. | (a) As designed. (b) No override: a breached room is sealed until repaired. | (a): rescue at a cost is the better choice to give players. | `breach-launch-bay` |
