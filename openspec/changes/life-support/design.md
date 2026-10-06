# Design: life support

Status: **proposed** (2026-10-04). Nothing here is built. The data file
`data/ships/tern/atmosphere.json` is proposed by this change (its `fire` block by
`damage-control`); the systems mockup reads it. Every table of results below was computed by
running the proposed library `docs/mockups/lib/shipsystems.js` headless in node 22 on the proposed
data (scratch scripts, not committed): they are results of the design's formulas, not engine
measurements. Power draws are `power-grid`'s; budgets are against `engine-stack` section 5.
**Rerun on layout v2 (2026-10-05)**, the plan whose rooms follow the hull: the same harness and
rules, unchanged, on the new volumes, floor and hull areas. Every number below that depends on the
plan is from that run; the same harness on the v1 plan converted to brushes reproduces the v1
tables exactly, so every difference is the new plan.

## Context

The Tern holds 9,824.8 m^3 of air in 30 compartments (`python3 tools/layout_check.py`), from the
12.9 m^3 turret pods to engineering's 2,538 m^3 and the hangar's 1,682.4 m^3. Its 40 portals are 23
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
- Stable at 10 Hz for every case, including a 2.2 m^2 hole in a 12.9 m^3 pod.
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
the ship holds about 408,600 mol (11.8 t) of air. The fittings' 4,000 J/K per m^3 (decks, consoles,
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
(`atmosphere.json` `graph_additions`); on acceptance they move into `layout.json` (section 19).

### 3. Doors keep the ship in compartments

**Closed by default.** Doors, hatches and ladder hatches are shut unless a crew member is passing
(they open on approach, `reference-ship-tern`'s "ordinary doors open on approach (0 s)") or the
damage control board holds them open. Every compartment is therefore its own pressure boundary: a
breach empties one room, not the ship. Pressure doors (launch bays, airlock) never open on approach;
they open on a command.

**Interlock.** A door does not open on approach, or on an ordinary command, across more than 20 kPa
of difference. The board, or a crew member holding the door's override for 3.0 s (`crew-on-deck`'s
hold), can open it anyway: that is how a crew member is pulled out of a breached room, at the cost
of the air that follows. Reconciled 2026-10-05: life-support owns the interlock (`crew-on-deck`
cites the 20 kPa in place of its first 5 kPa and 30 kPa); crew-on-deck owns the hold (was 2 s
here).

**Self-closing.** A door between two compartments closes itself when either side is below 85 kPa and
falling faster than 1 kPa/s, unless the board holds it. Doors to space are commanded, never
automatic. Travel times: doors open in 0.6 s and close in 0.8 s, pressure doors 2.0 s, hatches and
ladder hatches 1.0 s (`crew-on-deck` section 5), the hoist 2 s, drop doors 4 s, the pad door 8 s.
Reconciled 2026-10-05: crew-on-deck owns the door, pressure door and hatch times (they were 1.5 s
and 3 s here); `atmosphere.json` carries them until `data/crew.json` exists.

The first rule written was "close when one side is below 85 kPa and the other above": with a door
open, a breach drops both sides together and the rule never fired. Falling, not lopsided, is the
alarm. Measured on a 1 m^2 breach in the quarters with its door open (a crew member passing): the
door shut itself 1.7 s later with the main corridor at 73.9 kPa; the corridor's damper then refilled
it from the duct; 422 kg of air was lost. With the door held open by the board, the corridor emptied
too and 597 kg was lost. (Rerun 2026-10-05 with `crew-on-deck`'s door times, and again on layout v2:
on v1 it read 1.6 s, 73.7 kPa, 374 kg and 548 kg, and with the first 1.5 s door 2.4 s, 62.9 kPa and
399 kg.)

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

**The step is implicit.** A 2.2 m^2 hole in a turret pod empties it with a time constant shorter
than the 0.1 s sub-step (about 0.06 s for the v1 plan's 15.6 m^3 pod; the v2 pod is 12.9 m^3, so
shorter still): an explicit step would remove more gas than the pod holds.
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
| Hangar, 1 m^2 | 4.1 / 9.9 / 40 | 4.0 / 9.8 / 39 | -0.6 / -0.6 C |
| Port launch bay, 1 m^2 | 0.6 / 1.3 / 4.8 | 0.5 / 1.2 / 4.6 | -0.9 / -0.8 C |
| Medbay, 0.1 m^2 | 4.7 / 11 / 44 | 4.5 / 11 / 44 | 1.2 / 1.2 C |
| Bridge, 2.2 m^2 | 0.8 / 1.9 / 6.8 | 0.7 / 1.7 / 6.6 | -0.8 / -0.8 C |
| Port turret pod, 2.2 m^2 | 0.1 / 0.1 / 0.3 | 0.0 / 0.0 / 0.1 | -3.5 / -0.6 C |

The sub-step is within 0.3 s of the reference everywhere (the largest gap on layout v2 is 0.2 s);
the pod's coldest air differs because a 0.3 s event is three sub-steps. A crew member's fate in a pod blown open is decided by pressure
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
  pods 20, engineering 15 (its heat). The bridge's 608.6 m^3 at 10 an hour is 1.69 m^3/s.

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
bridge's 0.28 m^2 vent (the v1 plan's; it is 0.365 m^2 on v2, and these replaced rules were not
rerun) kept its breached room within 2.5 kPa of the duct, below any sensible
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
0.02-0.03%; air 18-25 C (the aft passage coolest at 17.8 C, then the launch bays at 18.8 C, with
four air changes an hour and much hull; the medbay warmest at 25.4 C, from its beds' 8 kW). The
generator makes 0.042 mol/s of oxygen (33.5 kW) and uses 2.3 kg of water in 30 minutes; the
scrubbers take 0.030 mol/s of CO2; thermal control removes 112 kW.

### 8. Metabolism

Per crew member at rest: 0.304 mmol/s of oxygen (0.84 kg a day), 0.264 mmol/s of CO2 (1.0 kg a day),
100 W of heat; working, 2.5 times the gases and 300 W. Suited crew breathe from the suit.

### 9. The thermal model

Each room gains the heat of its loads, lights and crew (`power-grid` section 10), exchanges heat with
its neighbours through shared bulkheads (5 W/(m^2 K) over the facing area computed from the layout's
brushes, within 0.6 m) and loses heat through the hull to a 250 K skin (0.4 W/(m^2 K) over the
exterior area: the bridge has 306 m^2, engineering 989 m^2). The supply air arrives at 20 C.

### 10. What the plant's loss costs, from the simulation

| Lost | With eight crew working | When it matters |
| --- | --- | --- |
| Oxygen generator and scrubbers, fans running | Bridge CO2 0.048 kPa after 1 h, 0.062 kPa after 4 h; oxygen 21.09 kPa after 4 h | Days: 408,600 mol of air dilutes eight people's breath |
| The whole plant, fans too (rooms isolated) | Bridge (four crew) CO2 0.080 kPa after 1 h, 0.199 kPa after 4 h; 29 C after 4 h (the hottest room engineering, 36 C) | Many hours: heat first, CO2 much later |
| Thermal control only, in combat | Turret pods reach 30 C in 6 min; after 1 h the magazine is at 38 C, the pods 39-45 C, the medbay 39 C, engineering 39 C | Within a mission: 45 C impairs (section 12) |

Real metabolism makes a dead plant a slow problem. The fast killers are breaches, fire and smoke,
and heat from equipment without thermal control; question L4 asks whether to accelerate metabolism
for play.

### 11. Breaches and decompression

A breach is a link to space of the area `damage-control` gives it. Every compartment, alone with its
door shut (the default), from 101.3 kPa (layout v2, rerun 2026-10-05):

- **16 kPa O2**: oxygen partial pressure below 16 kPa (hypoxia impairment begins).
- **50 kPa**: crew impaired by pressure.
- **6.3 kPa**: Armstrong's limit; body fluids boil, unconscious within seconds, dead after 90 s.
- **Own door**: the compartment's largest door to space opened (time to 50 kPa / to 6.3 kPa).
- **Crew at 1 m^2**: an unsuited crew member inside, time to unconscious / dead.

| POI | Compartment | Volume m^3 | 0.1 m^2: 16 kPa O2 / 50 kPa / 6.3 kPa, s | 1 m^2: same, s | 2.2 m^2 (a door-sized hole): 50 / 6.3 kPa, s | Own door to space | Crew at 1 m^2: unconscious / dead, s | Coldest air at 1 m^2, C |
| ---: | --- | ---: | --- | --- | --- | --- | --- | ---: |
| 1 | Bridge | 608.6 | 14 / 35 / 142 | 1.7 / 3.8 / 15 | 1.9 / 6.8 | none | 17 / 105 | -1 |
| 2 | Captain's ready room | 143.9 | 3.7 / 8.7 / 34 | 0.4 / 1.0 / 3.7 | 0.5 / 1.9 | none | 11 / 94 | -1 |
| 3 | Computer core | 143.9 | 3.7 / 8.7 / 34 | 0.4 / 1.0 / 3.7 | 0.5 / 1.9 | none | 11 / 94 | -1 |
| 4 | Command passage | 120.0 | 3.1 / 7.3 / 28 | 0.4 / 0.8 / 3.1 | 0.4 / 1.6 | none | 11 / 93 | -1 |
| 5 | Dorsal turret access | 45.8 | 1.3 / 2.9 / 11 | 0.2 / 0.4 / 1.4 | 0.2 / 0.7 | none | 9.8 / 91 | -1 |
| 6 | Aft passage | 135.0 | 3.5 / 8.1 / 32 | 0.4 / 0.9 / 3.5 | 0.5 / 1.8 | none | 11 / 94 | -1 |
| 7 | Torpedo room | 354.6 | 8.5 / 21 / 83 | 1.0 / 2.3 / 8.6 | 1.1 / 4.1 | none | 14 / 99 | -1 |
| 8 | Medbay | 187.2 | 4.7 / 11 / 44 | 0.5 / 1.3 / 4.7 | 0.6 / 2.3 | none | 12 / 95 | -1 |
| 9 | Damage control | 187.2 | 4.7 / 11 / 44 | 0.5 / 1.3 / 4.7 | 0.6 / 2.3 | none | 12 / 95 | -1 |
| 10 | Crew quarters | 271.5 | 6.6 / 16 / 64 | 0.8 / 1.8 / 6.7 | 0.9 / 3.2 | none | 13 / 97 | -1 |
| 11 | Mess | 271.5 | 6.6 / 16 / 64 | 0.8 / 1.8 / 6.7 | 0.9 / 3.2 | none | 13 / 97 | -1 |
| 12 | Port turret access | 233.0 | 5.7 / 14 / 55 | 0.7 / 1.6 / 5.8 | 0.8 / 2.8 | none | 12 / 96 | -1 |
| 13 | Starboard turret access | 233.0 | 5.7 / 14 / 55 | 0.7 / 1.6 / 5.8 | 0.8 / 2.8 | none | 12 / 96 | -1 |
| 14 | Main corridor | 195.0 | 4.9 / 12 / 46 | 0.6 / 1.3 / 4.9 | 0.6 / 2.4 | none | 12 / 95 | -1 |
| 15 | Hangar | 1682.4 | 38 / 96 / 391 | 4.1 / 9.9 / 40 | 4.6 / 18 | `p_hangar_pad` 72.0 m^2: 0.2 / 0.7 | 31 / 130 | -1 |
| 16 | Port launch bay | 190.1 | 4.7 / 11 / 45 | 0.6 / 1.3 / 4.8 | 0.6 / 2.4 | `p_drop_p` 43.2 m^2: 0.1 / 0.3 | 12 / 95 | -1 |
| 17 | Starboard launch bay | 190.1 | 4.7 / 11 / 45 | 0.6 / 1.3 / 4.8 | 0.6 / 2.4 | `p_drop_s` 43.2 m^2: 0.1 / 0.3 | 12 / 95 | -1 |
| 18 | Engineering | 2538.0 | 57 / 145 / 590 | 5.9 / 15 / 59 | 6.8 / 27 | none | 42 / 149 | 0 |
| 19 | Drive section | 307.2 | 7.4 / 18 / 72 | 0.9 / 2.1 / 7.5 | 1.0 / 3.6 | none | 13 / 98 | -1 |
| 20 | Lower corridor | 150.0 | 3.8 / 9.0 / 35 | 0.4 / 1.0 / 3.8 | 0.5 / 1.9 | none | 11 / 94 | -1 |
| 21 | Magazine | 501.1 | 12 / 29 / 117 | 1.4 / 3.2 / 12 | 1.6 / 5.7 | none | 16 / 102 | -1 |
| 22 | Life support | 367.8 | 8.8 / 22 / 86 | 1.0 / 2.4 / 9.0 | 1.2 / 4.2 | none | 14 / 99 | -1 |
| 23 | Cargo and stores | 349.1 | 8.4 / 20 / 82 | 1.0 / 2.3 / 8.5 | 1.1 / 4.0 | none | 14 / 99 | -1 |
| 24 | Airlock | 26.4 | 0.8 / 1.8 / 6.6 | 0.1 / 0.3 / 0.9 | 0.2 / 0.5 | `p_airlock_outer` 2.2 m^2: 0.2 / 0.5 | 9.6 / 91 | -1 |
| 25 | Shield generator | 167.8 | 4.2 / 10.0 / 40 | 0.5 / 1.2 / 4.3 | 0.6 / 2.1 | none | 11 / 94 | -1 |
| 26 | Forward switchboard | 167.8 | 4.2 / 10.0 / 40 | 0.5 / 1.2 / 4.3 | 0.6 / 2.1 | none | 11 / 94 | -1 |
| 27 | Dorsal turret pod | 12.9 | 0.5 / 1.1 / 3.6 | 0.1 / 0.2 / 0.6 | 0.1 / 0.3 | none | 9.4 / 91 | -2 |
| 28 | Ventral turret pod | 18.1 | 0.6 / 1.4 / 4.7 | 0.1 / 0.2 / 0.7 | 0.2 / 0.4 | none | 9.4 / 91 | -1 |
| 29 | Port turret pod | 12.9 | 0.5 / 1.1 / 3.6 | 0.1 / 0.2 / 0.6 | 0.1 / 0.3 | none | 9.4 / 91 | -2 |
| 30 | Starboard turret pod | 12.9 | 0.5 / 1.1 / 3.6 | 0.1 / 0.2 / 0.6 | 0.1 / 0.3 | none | 9.4 / 91 | -2 |

Times scale with volume over area: the hangar takes 40 s to reach Armstrong's limit through 1 m^2,
a pod 0.6 s. A crew member caught unsuited in a breached room is unconscious in 9.4-17 s in every
room but the two largest (31 s in the hangar, 42 s in engineering; the slowest of the rest are the
bridge, 17 s, and the magazine, 16 s) and dead 90 s after Armstrong's limit.

**Venting deliberately** through the duct (`damage-control`'s fire tool): the board shuts the
room's doors, forces every other damper shut and opens the overboard dump; any room reaches 20 kPa
in 24 s (vents are sized in proportion to volume), engineering in 82 s.

**Refilling** a sealed room from vacuum through its vent: the medbay in 170 s for 193 kg of reserve
gas, the quarters in 248 s for 283 kg, a launch bay in 174 s for 197 kg. **Engineering cannot be
refilled**: it holds 3,032 kg and the reserves 2,740 kg; after a large breach it stays in vacuum
until resupply, and the crew work it suited (question L5).

### 12. Crew effects

From the crew member's room, each sub-step (`atmosphere.json` `crew_effects`; `crew-on-deck` shows
them). Reconciled 2026-10-05: life-support owns these thresholds and rates, and `crew-on-deck`
section 9 cites them in place of its own; it presents each to the player (screen effects, no
running, slower tools), maps "unconscious" to its downed state and "dead" to its critical state
(no permanent death, its question C3).

| Effect | Impaired | Toward unconsciousness | Death |
| --- | --- | --- | --- |
| Hypoxia | pO2 below 16 kPa | Time of useful consciousness by pO2: 60 min at 14.6 kPa (the onset; corrected 2026-10-05 from a 0 s row that the interpolation read as instant), 30 min at 12 kPa, 20 min at 10.6, 5 min at 8.9, 3 min at 7.9, 1 min at 6.3, 15 s at 3.9, 9 s below 3.4 (a dose accumulating `dt / TUC`) | Unconscious for 240 s with pO2 below 10.6 kPa |
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
launch is permitted; the valve takes the rest. On layout v2 (rerun 2026-10-05; a launch bay was
231 m^3 and 28.6 s to launch on v1, the hangar 1,698 m^3 and 208.4 s):

| | Port launch bay (190.1 m^3, 228 kg) | Hangar (1,682.4 m^3, 2,019 kg) |
| --- | ---: | ---: |
| To 50 kPa | 5.7 s | 47.5 s |
| To 10 kPa | 18.1 s | 158.0 s |
| **Pumps stop at 5 kPa: launch permitted** | **23.6 s** | **206.5 s** |
| To 1 kPa (vent valve) | 50.6 s | 441.5 s |
| Pump energy, peak power | 53 MJ, 3.46 MW | 826 MJ, 6.00 MW |
| Air kept in the receiver | 97.7% (receiver at 415 kPa) | 95.3% (receiver at 2,808 kPa) |
| Air vented | 12 kg | 105 kg |
| **Repressurize from the receiver** | **13.0 s** | **104.6 s** (plus make-up through the duct) |

**Emergency vent** instead (the drop door or pad door opened with the bay full): the launch bay
passes Armstrong's limit 0.9 s after its door starts to open and loses 228 kg; the hangar 2.8 s and
2,019 kg, 74% of the ship's reserve gas. Through its vent valve alone a launch bay takes 45 s.

The pumps refuse to start while an unsuited crew member is in the bay (`bay_pumps.interlock_unsuited_crew`),
vision pillar 4.

### 14. The airlock

Cycle out: both doors shut, the airlock pump (2 m^3/s into cargo, 0.15 MW) pumps to 5 kPa in 39.3 s
using 3.8 MJ, then the outer door opens in 2.0 s, venting 1.7 kg. Cycle in: the outer door shuts
(2.0 s), the 0.02 m^2 equalizing valve fills the airlock from cargo in 14.1 s, the inner door opens.
(Rerun 2026-10-05 with `crew-on-deck`'s 2.0 s pressure door, and again on layout v2, whose airlock
is 26.4 m^3: on v1's 24.0 m^3 it was 35.7 s, 3.5 MJ, 1.5 kg and 13.0 s, and with the first 3 s door
the fill took 13.9 s, because air left through the closing door. The pump-down does not depend on
the door.)

### 15. Reconciling `shuttle-bay-and-fighters`

| Its assumption | Computed here | Proposal |
| --- | --- | --- |
| Pump-down to 1 kPa in 30 s | 23.6 s to 5 kPa, 50.6 s to 1 kPa | Launch permitted at 5 kPa, the pumps' stop: the drop door vents the last 12 kg. Its sequence takes the 23.6 s (question L2) |
| Repressurize in 25 s | 13.0 s | Shorter |
| Emergency vent in 6 s, 277 kg | Armstrong at 0.9 s after the door starts to open (4 s travel), 228 kg | Aligned (a launch bay holds 228 kg on layout v2) |
| Hangar pump-down 120 s; vent 20 s, 2,040 kg | 207 s; Armstrong at 2.8 s, 2,019 kg | The Petrel launch takes 86.5 s longer (question L3) |
| Hangar doors to the corridors, engineering and galleries close for a Petrel launch | Every door is closed by default and interlocked | No layout change needed |

`bridge-stations` F3 listed the pump-down at "~38 s"; it should read 24 s to launch (corrected
there 2026-10-04 to 29 s, and 2026-10-05 to 24 s on layout v2, as were `shuttle-bay-and-fighters`'
sequence times: 37.6 s cold, 31.6 s from ready 5).

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
| `portals` | Discharge coefficient and default state per kind, opening and closing times (and per-portal overrides; the door, pressure door and hatch times are `crew-on-deck`'s), self-closing threshold and fall rate, the kinds that self-close, the interlock kPa |
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

### 19. Layout patches proposed

For `reference-ship-tern` to apply to `layout.json` (this change does not edit it), so the one
compartment graph holds every link that moves air:

1. A duct node, which is not walkable and has no boxes: a new top-level list
   `"ducts": [ { "id": "duct", "name": "Air duct trunk and plenum", "volume_m3": 40.0, "center_m": [3.0, -0.8, 9.0] } ]`.
2. One vent portal per compartment, `{ "id": "vent_<compartment>", "kind": "vent", "between": ["<compartment>", "duct"], "area_m2": <0.0006 x volume, 0.02-0.6> }`
   (the bridge's is 0.365 m^2, each pod's 0.02 m^2, engineering's and the hangar's 0.6 m^2), and
   `{ "id": "duct_dump", "kind": "dump", "between": ["duct", "space"], "area_m2": 0.5 }`.
3. The bay vent valves `valve_hangar`, `valve_launch_bay_p`, `valve_launch_bay_s` (kind `valve`,
   bay to space, 0.1 m^2) and `valve_airlock_eq` (cargo to airlock, 0.02 m^2).
4. The reserve bottles in `fixtures`:
   `{ "id": "reserve_bottles", "kind": "rack", "compartment": "cargo", "center_m": [-8.0, -3.5, 8.8], "size_m": [1.4, 3.6], "facing_yaw_deg": -90, "note": "Reserve nitrogen 75,000 mol and oxygen 20,000 mol (life-support)." }`.
   The bay receiver is part of the existing `hangar_pumps` system ("Bay pumps and reserve tank").

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
| `breach-launch-bay` | A 40 MJ hit from below into the port launch bay, 4 s later: a 0.46 m^2 breach, the bay at 33.9 kPa and falling, a fire (1.34 MW) dying for want of oxygen, red alert |
| `hangar-pumpdown` | The hangar 100 s into its pump-down: 23.1 kPa, the air at 4.4 C from expansion, the receiver at 2,274 kPa, the pumps drawing 4.4 MW |

## Open questions

Ids L (life support). Questions with a shot go to the owner's survey with the shot beside them; the
rest are recommendations taken (ask only with screenshots, CLAUDE.md 13).

| Id | Question and the fact it turns on | Options | Recommendation | Shot |
| --- | --- | --- | --- | --- |
| L1 | Are doors shut by default? Shut, a breach costs one room; open, a 1 m^2 breach anywhere would take the whole ship's air (408,600 mol, time constant about 82 s, the v1 run's 74 s scaled by the air aboard) to Armstrong's limit in about 3.9 minutes. | (a) Shut unless someone passes or the board holds them, open on approach. (b) Open, closing on an alarm only. | (a). | `atmosphere-normal` |
| L2 | When may a fighter drop? The pumps stop at 5 kPa after 23.6 s; 1 kPa takes 51 s. | (a) At 5 kPa, venting the last 12 kg. (b) At 1 kPa. | (a): keeps `shuttle-bay-and-fighters`' sequence (37.6 s cold, from its earlier 44 s at an assumed 30 s pump-down). | `hangar-pumpdown` |
| L3 | The hangar takes 207 s to pump down at 6 MW. | (a) Accept: the Petrel is not a combat launch. (b) Pumps twice the size (12 MW, about 103 s). (c) Vent it (2,019 kg, 74% of the reserve). | (a). | `hangar-pumpdown` |
| L4 | Real metabolism makes a dead plant harmless for hours. | (a) Real rates: life support is about breaches, fire, smoke and heat. (b) Metabolism accelerated 20 times so a dead plant matters within a mission. | (a). | none: recommendation taken (ask only with screenshots) |
| L5 | The reserves (2,740 kg) cannot refill engineering (3,032 kg). | (a) As designed: a breached engineering stays in vacuum until resupply. (b) Reserves of 4,000 kg. | (a): the suited engineer is a strong scene, and the debrief resupplies. | none: recommendation taken (ask only with screenshots) |
| L6 | The door interlock: 20 kPa, with an override that costs air. | (a) As designed. (b) No override: a breached room is sealed until repaired. | (a): rescue at a cost is the better choice to give players. | `breach-launch-bay` |
