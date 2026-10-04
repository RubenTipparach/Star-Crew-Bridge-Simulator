# Design: the power grid

Status: **proposed** (2026-10-04). Nothing here is built. The data file
`data/ships/tern/power.json` is proposed by this change; the systems mockup reads it. Every table
of results below was computed by running the proposed library `docs/mockups/lib/shipsystems.js`
headless in node 22 on the proposed data (scratch scripts, not committed): these are results of
the design's own formulas, not measurements of an engine, and certainly not of a Pi. Budget
numbers are against `openspec/changes/engine-stack/design.md` section 5 (the Pi 5 table, the one
source).

## Context

The Tern (`data/ships/tern/layout.json`) puts its fusion reactor through three deck heights in
engineering (POI 18, [0.0, 1.5, -25.0]), its main switchboard beside it, the coolant pumps on the
lower floor, and its battery bank and forward switchboard 25 m forward on deck C (POI 26). Between
them lie the hangar and the turret rooms, so every forward load is fed through compartments that
can be hit. The Tern design (`openspec/changes/reference-ship-tern`, section 7) already counts on
this: if engineering is lost, "battery bank and forward switchboard in 26, at the opposite end of
the ship, with the emergency bus (`power-grid`)".

From star-crew-64 (`docs/analysis/star-crew-64.md`): three channels summing to 100%, every system
scaled by `power / 33`, rebalanced by taking the delta from the other two in proportion. It was
easy to read and it made engineering a real job. It had no reactor, no limit except the sum, no
wire to cut, no heat, and its timings counted frames.

Other changes assumed draws "see power-grid"; section 15 reconciles them.

## Goals / Non-Goals

**Goals:**
- Power that can be followed from the reactor through a named switchboard, conduit and panel to a
  load, with every MW on every console coming from one solve (CLAUDE.md 6.1, 7).
- Real causes: a cut conduit, a destroyed switchboard section, a tripped breaker, a scram, a flat
  battery, an overheated system.
- A budget that forces choices in combat, without making cruise a chore.
- Deterministic, numerically stable at 10 Hz, cheap on one Cortex-A76 core.

**Non-Goals:**
- Electrical engineering detail: voltages, phases, frequency, power factor, fault currents. Power
  is a flow in MW with capacities, like a transport network.
- The air, the plant's chemistry and the bay pumps' physics (`life-support`); hit resolution,
  repairs and fire (`damage-control`); console layouts (`bridge-stations`).
- Thrust, shields and weapons physics: those changes take this change's supply ratio.

## Decisions

### 1. What is simulated, and when

The ship systems step at 10 Hz inside the server's 30 Hz tick (`engine-stack` section 6): every
third tick runs one sub-step of `dt = 0.1 s`. One sub-step, in this order (one function,
`systems::step`, shared by the server, a solo listen server and the mockup):

1. Automation: dampers, doors, suppression, portal motion (`life-support`, `damage-control`).
2. **Power**: reactor control, the solve, battery energy, fuel, ignition.
3. **Heat**: thermal nodes, the reactor blanket, the coolant loop, the radiators.
4. Life support plant, bay and airlock pumps, fire, crew, room heat, gas flow, mixing.
5. Scram checks.

Inputs that change between sub-steps (a setpoint, a breaker, helm's throttle) are applied in a
stable order at the tick boundary before the sub-step (`netcode-and-sessions`).

### 2. The reactor

| Quantity | Value | Notes |
| --- | ---: | --- |
| Type | Deuterium and helium-3 fusion | Aneutronic enough that engineering is a room crew can stand in |
| Thermal output, rated | 80 MW | At throttle 100% and full integrity |
| Conversion to electric | 60% | 48 MW electric at 100% |
| Throttle range | 10-120% | Below 10% the plasma is not held; 100-120% is overdrive |
| Ramp | +2%/s, -10%/s | 44% to 100% takes 28 s; the battery covers the gap |
| Fuel load | 4.0 kg | 353,000,000 MJ/kg; 204 days at full output. A campaign consumable, not a mission one |
| Overdrive wear | 1%/min of integrity at 120% | In proportion to the amount above 100% |
| Blanket heat capacity | 10 MJ/K | |
| Blanket to loop | 110 kW/K x loop flow | |
| Blanket to the room | 250 W/K | Engineering warms when the reactor runs hot |
| Blanket warning, scram | 750 K, 820 K | |

`P_th = throttle x 80 MW x integrity`. The grid takes `P_e` (the reactor edge's flow in the solve,
at most `0.6 P_th`). The blanket receives `P_th - P_e`: the conversion loss, plus any power the grid
could not take.

**Load following.** In automatic mode (the default, and what automated engineering uses) the
throttle target is the load the grid can actually take from the reactor: a probe solve with the
reactor at its rated electric output and the battery held back, plus the battery's charge want,
divided by 48 MW and clamped to 10-100%. Automation never overdrives. An earlier draft followed
total demand instead; with both midships trunks cut it ran the reactor at 100% into a grid that
could take 27 MW, and the blanket reached 816 K and scrammed (found by the walkthroughs, fixed).

**Manual mode.** A player sets the throttle on the console (or the bay panel), 10-120%. Power the
grid cannot take heats the blanket; the console shows "unused" MW when `0.6 P_th - P_e` exceeds
0.5 MW.

### 3. Scram and restart

**Causes** (each with a hold time, so a transient does not trip it):

| Cause | Threshold | Hold |
| --- | --- | ---: |
| Coolant loop over-temperature | loop above 380 K | 2 s |
| Blanket over-temperature | above 820 K | immediate |
| Coolant flow lost | flow under 30% while the throttle is above 20% | 5 s |
| Auxiliaries lost | both trains under 80% of their supply | 2 s |
| Reactor damaged | integrity under 25% | immediate |

**The auxiliaries are two full trains.** `reactor_aux_p` on the port switchboard section and
`reactor_aux_s` on the starboard section each carry the reactor alone; the scram rule takes the
better train's supply. An earlier draft summed the two, so losing either switchboard section
scrammed the reactor (found in walkthrough W1 of `damage-control`, fixed).

**Restart is hands-on.** Only the reactor panel in engineering (fixture `reactor_panel`, layout
patch in section 16) resets a scram: a crew member holds RESET for 3 s. The reset is refused while
the loop is at or above 350 K, the blanket at or above 700 K, or integrity under 25%, and the panel
says which. Then:

1. **Ignition**: the two auxiliary trains draw 8 MW together for 20 s (160 MJ) from whatever
   supplies them, normally the battery's restart reserve. Ignition progresses at the pace the
   trains' supply allows (one train alone takes 40 s).
2. **Running** at 10%, ramping at 2%/s: full output 45 s after ignition.

From the bridge (`reference-ship-tern` section 6): 41.6 s walking or 23.1 s running to the
reactor's lower floor. A scram in combat therefore costs about 23 + 3 + 20 + 45 = 91 s before the
reactor is back at full output, if the engineer runs at once.

### 4. The battery bank

| Quantity | Value | Notes |
| --- | ---: | --- |
| Capacity | 1,800 MJ | 500 kWh; starts a mission at 90% |
| Discharge, charge | 30 MW, 12 MW | 95% efficient each way; losses go to the coolant loop |
| Restart reserve | 350 MJ | Held for the loads below unless engineering releases it |
| Node | The emergency bus `eb`, forward switchboard | Its own breaker |

**The restart reserve** is spent only by: emergency lighting (always), the reactor auxiliaries
while igniting, and the coolant pumps while the reactor is not running. It is sized for the worst
case found: a scram from loop over-temperature with radiators at 60% health, where the pumps must
run 170 s (144 MJ) before the loop falls below 350 K and the reset is accepted, then ignition
(160 MJ). An earlier draft held 200 MJ and kept the pumps off the reserve: the loop never cooled,
the reset was refused forever, and the ship was dead (found, fixed). With 350 MJ the same case
ignites with 21 MJ to spare.

**Charging** uses reactor surplus only, after every load is served, and never while the battery
is discharging.

**Endurance** with the reactor scrammed, from 90% (1,620 MJ) to the reserve:

| Running | Draw | Endurance |
| --- | ---: | ---: |
| Essentials only (priorities 0 and 1; everything else off) | 3.5 MW | 5.8 min |
| Silent preset, the ship idle | 12.4 MW | 1.6 min |
| Emergency preset, shields holding, two turrets firing | 24.0 MW wanted, 12.7 MW at the battery's share | 0.8 min |

The battery bridges a gap: a reactor ramp, a scram, a combat deficit. It does not run the ship. A
reactor that cannot be restarted ends the ship's fighting within minutes, by design (question P8).

### 5. Topology

```text
                         REACTOR  80 MW thermal / 48 MW electric
                    gen_p 30 MW |                    | gen_s 30 MW
   PORT SIDE OF ENGINEERING   msb_p ==== bt_main 40 MW ==== msb_s   STARBOARD SIDE
              k_drive_p 20 /    | k_aft_p 30              k_aft_s 30 |    \ k_drive_s 20, k_a_aft 8 (to dp_a)
                  dp_drive      dp_hangar_p (hangar, port)  dp_hangar_s (hangar, stbd)
                                | k_mid_p 30                       | k_mid_s 30
                                dp_port (port turret access)       dp_stbd (stbd turret access)
                                | k_fwd_p 30                       | k_fwd_s 30
              FORWARD        fsb_p ======= bt_fwd 30 MW ======= fsb_s
              SWITCHBOARD       \ eb_p 15             eb_s 15 /
                                 eb (emergency bus) <== battery 1,800 MJ, 30 MW
   fsb_p feeds: k_a_fwd 8 (dp_a), k_c_fwd_p 30 (dp_c_fwd), k_ls_n 6 (dp_ls)
   fsb_s feeds: k_b_fwd 12 (dp_b_fwd), k_c_fwd_s 30 (dp_c_fwd);   eb feeds: k_ls_e 3 (dp_ls)
```

**Nodes** (`power.json` `nodes`): two main switchboard sections in engineering, on opposite sides
(port at [7.5, 0.5, -20.6], starboard at [-7.5, 0.5, -21.0]); two forward switchboard sections and
the emergency bus in the forward switchboard room; ten distribution panels in the compartments
they serve.

**Why the main switchboard sections are 15 m apart.** In the first draft both sat side by side on
the port side, and walkthrough W1 (a 60 MJ hit on engineering's port side) destroyed both, which
isolated the reactor from the whole ship. Separated, the same hit destroys the port section and the
reactor carries on through the starboard section at its generator's 30 MW. Real warships separate
their main switchboards for the same reason. The layout's single `main_switchboard` system needs a
starboard twin (section 16).

**Conduits, ties and generators:**

| Conduit | Between | Capacity MW | Route |
| --- | --- | ---: | --- |
| `k_drive_p` Drive feeder, port | `msb_p` to `dp_drive` | 20 | engineering, drive |
| `k_drive_s` Drive feeder, starboard | `msb_s` to `dp_drive` | 20 | engineering, drive |
| `k_aft_p` Port main trunk, aft | `msb_p` to `dp_hangar_p` | 30 | engineering, hangar |
| `k_aft_s` Starboard main trunk, aft | `msb_s` to `dp_hangar_s` | 30 | engineering, hangar |
| `k_mid_p` Port main trunk, midships | `dp_hangar_p` to `dp_port` | 30 | hangar, port_turret |
| `k_mid_s` Starboard main trunk, midships | `dp_hangar_s` to `dp_stbd` | 30 | hangar, stbd_turret |
| `k_fwd_p` Port main trunk, forward | `dp_port` to `fsb_p` | 30 | port_turret, shield_room, c_spine, switchboard |
| `k_fwd_s` Starboard main trunk, forward | `dp_stbd` to `fsb_s` | 30 | stbd_turret, switchboard |
| `k_a_fwd` Command deck feeder, forward | `fsb_p` to `dp_a` | 8 | switchboard, c_spine, b_spine, a_corridor |
| `k_a_aft` Command deck feeder, aft | `msb_s` to `dp_a` | 8 | engineering, a_aft_passage, dorsal_turret, a_corridor |
| `k_b_fwd` Main deck forward feeder | `fsb_s` to `dp_b_fwd` | 12 | switchboard, c_spine, b_spine |
| `k_c_fwd_p` Lower deck feeder, port | `fsb_p` to `dp_c_fwd` | 30 | switchboard, c_spine |
| `k_c_fwd_s` Lower deck feeder, starboard | `fsb_s` to `dp_c_fwd` | 30 | switchboard, c_spine |
| `k_ls_n` Life support feeder, normal | `fsb_p` to `dp_ls` | 6 | switchboard, c_spine, life_support |
| `k_ls_e` Life support feeder, emergency | `eb` to `dp_ls` | 3 | switchboard, c_spine, life_support |
| `bt_main` Main bus tie | `msb_p` to `msb_s` | 40 | inside engineering |
| `bt_fwd` Forward cross-tie | `fsb_p` to `fsb_s` | 30 | inside the forward switchboard |
| `eb_p`, `eb_s` Emergency feeds | `fsb_p`, `fsb_s` to `eb` | 15 each | inside the forward switchboard |
| `gen_p`, `gen_s` Reactor generators | reactor to `msb_p`, `msb_s` | 30 each | engineering |

Each conduit carries its `path_m` (a polyline in ship metres) so `damage-control` can tell whether a
hit passed within reach of it, and the mockup can draw it. The redundancy is deliberate: two trunks
down the ship's sides, cross-tied at both ends; the command deck fed from both ends; life support
fed normally and from the emergency bus.

### 6. Loads

Forty loads: thirty in the table plus ten lighting panels. "Drops below" is the minimum supply
ratio under which the load switches off rather than run weakly (section 9).

| Load | Node | Priority | Nominal MW | Standby MW | Max setpoint | Max MW | Drops below | Heat |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `reactor_aux_p` Reactor auxiliaries, port | `msb_p` | 0 | 1 | 0.3 | 1 | 1 | never | loop 100% |
| `reactor_aux_s` Reactor auxiliaries, starboard | `msb_s` | 0 | 1 | 0.3 | 1 | 1 | never | loop 100% |
| `coolant_pump_a` Coolant pump A | `msb_p` | 0 | 0.4 | 0.4 | 1 | 0.4 | never | loop 100% |
| `coolant_pump_b` Coolant pump B | `msb_s` | 0 | 0.4 | 0.4 | 1 | 0.4 | never | loop 100% |
| `emergency_lighting` Emergency lighting | `eb` | 0 | 0.0043 | 0.0043 | 1 | 0.0043 | never | none |
| `computer` Computer core | `dp_a` | 1 | 1.5 | 1.5 | 1.5 | 2.25 | 50% | own mass 4.3 MJ/K, 95%, 47.5 kW/K to loop |
| `bridge_consoles` Bridge consoles | `dp_a` | 1 | 0.007 | 0.007 | 1 | 0.007 | 50% | the bridge |
| `o2_generator` Oxygen generator | `dp_ls` | 1 | 1 | 0.005 | 1.5 | 1.5 | never | own mass 0.9 MJ/K, 30%, 10 kW/K to loop |
| `co2_scrubbers` CO2 scrubbers | `dp_ls` | 1 | 0.25 | 0.25 | 1.5 | 0.375 | never | loop 100% |
| `air_handler` Air handler fans | `dp_ls` | 1 | 0.08 | 0.08 | 1.25 | 0.1 | 20% | supply air |
| `thermal_control` Thermal control | `dp_ls` | 1 | 0.75 | 0.15 | 1 | 0.75 | never | none (it moves heat) |
| `medbay` Medbay beds | `dp_b_fwd` | 1 | 0.008 | 0.002 | 1.5 | 0.012 | 25% | the medbay |
| `shield_generator` Shield generator | `dp_c_fwd` | 2 | 12 | 2 | 1.5 | 18 | 25% | own mass 12.6 MJ/K, 35%, 140 kW/K to loop |
| `turret_dorsal` Dorsal turret | `dp_a` | 2 | 4 | 0.1 | 1.5 | 6 | 20% | own mass 4.8 MJ/K, 40%, 53.3 kW/K to loop |
| `turret_ventral` Ventral turret | `dp_c_fwd` | 2 | 4 | 0.1 | 1.5 | 6 | 20% | as the dorsal turret |
| `turret_port` Port turret | `dp_port` | 2 | 4 | 0.1 | 1.5 | 6 | 20% | as the dorsal turret |
| `turret_stbd` Starboard turret | `dp_stbd` | 2 | 4 | 0.1 | 1.5 | 6 | 20% | as the dorsal turret |
| `sensors` Sensor array | `dp_b_fwd` | 2 | 3 | 0.5 | 1.5 | 4.5 | 15% | own mass 4.5 MJ/K, 50%, 50 kW/K to loop |
| `inertial_dampers` Inertial dampers | `dp_drive` | 2 | 8 | 2 | 1.5 | 12 | 10% | own mass 12 MJ/K, 50%, 133.3 kW/K to loop |
| `gravity_generator` Gravity generator | `dp_c_fwd` | 2 | 4 | 4 | 1.25 | 5 | 30% | own mass 6 MJ/K, 50%, 66.7 kW/K to loop |
| `impulse_drive` Impulse drive | `dp_drive` | 3 | 16 | 0.2 | 1.5 | 24 | 10% | own mass 7.2 MJ/K, 15%, 80 kW/K to loop (the rest leaves in the exhaust) |
| `rcs_aft` RCS, aft block | `dp_drive` | 3 | 4 | 0.1 | 1.5 | 6 | 10% | own mass 3.6 MJ/K, 30%, 40 kW/K to loop |
| `rcs_fwd` RCS, forward block | `dp_b_fwd` | 3 | 4 | 0.1 | 1.5 | 6 | 10% | as the aft block |
| `missile_hoist` Missile hoist and loaders | `dp_b_fwd` | 3 | 0.03 | 0.001 | 1.5 | 0.045 | 20% | the magazine |
| `comms` Comms array | `dp_a` | 3 | 0.5 | 0.1 | 1.5 | 0.75 | 20% | own mass 0.75 MJ/K, 50%, 8.3 kW/K to loop |
| `hangar_pumps` Bay pumps | `dp_hangar_s` | 3 | 6 | 0 | 1 | 6 | never | own mass 7.2 MJ/K, 40%, 80 kW/K to loop |
| `airlock_pump` Airlock pump | `dp_c_fwd` | 3 | 0.15 | 0 | 1 | 0.15 | never | cargo |
| `cradle_p` Port launch cradle | `dp_hangar_p` | 3 | 0.6 | 0.05 | 1.5 | 0.9 | 20% | loop 30% |
| `cradle_s` Starboard launch cradle | `dp_hangar_s` | 3 | 0.6 | 0.05 | 1.5 | 0.9 | 20% | loop 30% |
| `shuttle_pad` Shuttle lift pad | `dp_hangar_p` | 3 | 0.05 | 0 | 1 | 0.05 | 20% | the hangar |
| Lighting, 10 panels | one per panel | 1 | 0.0256 in all | same | 1 | same | never | the rooms lit |

**Totals**: 81.35 MW nominal, 12.92 MW standing by, 116.12 MW with every setpoint at its maximum.
By priority, nominal: 2.80 MW (0, vital), 3.62 MW (1, crew), 43.00 MW (2, fighting), 31.93 MW (3,
moving and support). Normal lighting is 12 W/m^2 over 2,135 m^2 (25.6 kW); emergency lighting
2 W/m^2 (4.3 kW) on the emergency bus.

**What a load does with its supply** (the effects other changes implement, from `power.json`):

| Load | Under-powered | Over-powered (setpoint above 1) |
| --- | --- | --- |
| Reactor auxiliaries | Two full trains: scram when neither has 80% for 2 s; ignition at the pace their supply allows | Not adjustable |
| Coolant pumps | Loop flow in proportion; each pump is half the flow | Not adjustable |
| Emergency lighting | Below 50%: rooms with no normal lighting go dark | Not adjustable |
| Computer core | Automation competence falls with supply; below 50% every automated station stops and targeting solutions are lost | Automation reacts up to a third faster |
| Bridge consoles | Below 50% the bridge stations go dark; the bay console and damage board still work | Not adjustable |
| Oxygen generator, scrubbers, fans, thermal control | Output in proportion (`life-support`); fans stop below 20% and every vent damper shuts | Up to 1.5x (fans 1.25x) |
| Medbay beds | Healing in proportion; off below 25% | Up to 1.5x faster |
| Shield generator | Regeneration and capacity in proportion; faces collapse below 25% | Up to 1.5x regeneration, with heat |
| Turrets | Capacitor charge rate in proportion (4 MW nominal against 6 MW of sustained fire); cannot fire below 20% | Up to 1.5x: 6 MW matches sustained fire |
| Sensors | Active range as the square root of supply; passive only at standby | Range up to 1.22x |
| Inertial dampers | The felt share of acceleration is 1 minus supply ratio (`ship-frames`) | Up to 1.5x the rated acceleration |
| Gravity generator | Gravity in g equals the supply ratio; free fall below 30% | Up to 1.25 g; crew 10% slower |
| Impulse drive, RCS | Thrust or turn rate in proportion (`flight-and-navigation`) | Up to 1.5x, with heat and wear |
| Hoist and loaders | Reload time as 1 / supply; by hand crank unpowered (`weapons-and-shields`) | Up to 1.5x faster |
| Comms | Range in proportion; short-range voice below 20% | Up to 1.5x range |
| Bay pumps, airlock pump, cradles, pad | Speed in proportion | Cradles up to 1.5x |

### 7. Demand

For each load at each sub-step:

```text
sp     = min(setpoint, setpoint_max)                      engineering's request, 0..1.5
a      = activity (0..1), set by the system's user        see below
cap    = capability from damage-control: 1 at or above 75% integrity,
         integrity / 75 between 25% and 75%, 0 below 25%
demand = 0                                                if sp = 0, its breaker is open, or its node is destroyed
       = (standby + max(0, nominal x sp - standby) x a) x cap   otherwise
```

Activities come from the systems' users: helm's thrust and turning (`flight-and-navigation`,
`|F_used| / F_rated`), the dampers' load (`ship-frames`, `max |A_k| / C_rated`), a turret's
capacitor below full, shield faces below capacity, an active scan, comms transmitting, the hoist or
a loader moving, a cradle or the pad moving. Life support computes its own: the oxygen generator's
output want, the heater's want, the bay and airlock pumps' work (`life-support`). Loads without an
activity draw `nominal x sp`.

### 8. Allocation: setpoints, priorities, presets, overdrive

**Engineering's levers**, from the bridge console or the engineering bay console:

- **Setpoint** per load group, 0-150% in 5% steps (`power.json` `groups`, eleven groups matching
  `bridge-stations` panel E1). 0 switches a group off; above 100% is overdrive.
- **Priority** per group, 1-3. Class 0 (reactor auxiliaries, coolant pumps, emergency lighting) is
  vital and fixed.
- **Presets** (`power.json` `presets`), applied by engineering or by automated engineering on a
  condition change (`bridge-stations`):

| Group | Cruise | Combat | Silent | Emergency |
| --- | ---: | ---: | ---: | ---: |
| Impulse drive and RCS | 100% | 75% | 20% | 30% |
| Shields | 100% | 125% | 20% | 50% |
| Turrets | 100% | 100% | 0% | 50% |
| Tubes and hoist | 100% | 100% | 100% | 100% |
| Sensors | 100% | 60% | 17% (passive) | 17% |
| Comms | 100% | 50% | 20% | 20% |
| Gravity and dampers | 100% | 100% | 100% | 80% |
| Computer core | 100% | 125% | 100% | 100% |
| Bays and craft charging | 100% | 100% | 0% | 0% |
| Life support, medbay | 100% | 100% | 100% | 100% |

- **The reactor throttle** in manual, up to 120%.
- **Breakers** on every conduit, tie, generator and the battery; the reserve release.

**Overdrive** costs heat and wear. A load above nominal (`over = alloc / nominal - 1`) puts out
`fraction x alloc x (1 + over)` of heat, and wears at 2% of integrity a minute at 150% (in
proportion to `over / 0.5`). The reactor above 100% wears 1%/min at 120%. Section 12 gives the
times.

**star-crew-64's zero-sum split, and why setpoints instead.** star-crew-64 had engineering pump one
of three channels while the other two gave way in proportion (`engineering_console.c:135-166`);
every channel always summed to 100% of an abstract budget. That feel is worth keeping, and it
returns here where it matters: when demand exceeds supply, raising one group's setpoint takes power
from lower priorities first and then in proportion from its own class, and the console shows who
lost it. But a fixed 100% cannot represent a reactor that scrams, a trunk that is cut, a battery that
runs flat or a switchboard section that is gone, and a percentage of nothing is still 100%. So
engineering sets requests (setpoints) and an order (priorities); the solve turns the physical supply
into shares. At cruise there is plenty and nothing is traded (21 MW wanted of 48 MW); in combat the
trade is real (56 MW wanted of 48 MW, section 12), which is the zero-sum moment star-crew-64 made
permanent. An explicit "percent of reactor" slider per group was considered and rejected for the
same reason; E1's footer shows each group's share of the present supply instead.

### 9. The solve

`power::solve(state, setpoints) -> allocation` is the one implementation (CLAUDE.md 6.1). The
server's sub-step, the solo listen server, every console preview and the mockup call it.

**The graph** for one sub-step: a source `S`; the reactor `RX` (edge `S -> RX` with capacity
`0.6 P_th`); the battery `BAT` (edge `S -> BAT`, capacity set per phase below); the fourteen power
nodes. Generators (directed, 30 MW each, closed or open), ties (undirected), conduits (undirected,
`capacity x health`, zero if severed, its breaker open, or either end destroyed) and the battery's
link to `eb` (zero if its breaker is open). Loads are sinks at their nodes.

**Passes**, in this order:

1. Reserve users: emergency lighting; the reactor auxiliaries while igniting; the coolant pumps
   while the reactor is not running.
2. Priority 0, then 1, 2 and 3.

**Phases** within a pass: first the reactor alone; then the reactor and the battery, the battery
offering `min(30 MW x health, (charge - reserve) x 0.95 / dt)` (the whole charge for reserve
users).

**Rounds** within a phase (up to 3): with `D` the class's unmet demand and `R` what the sources can
still give, every unblocked load asks for `lambda x` its unmet demand, `lambda = min(1, R / D)`, in
data order. Each request is routed by augmenting paths (breadth-first over residual capacities, in
the adjacency's data order, at most 16 paths per request). A load whose path cannot carry its
request keeps what it got and is blocked; the others are offered the rest in the next round. So
within a class, loads share in proportion to demand, except where a conduit limits them.

**Drop-out.** A load with a minimum ratio that receives less than `min_ratio x demand` switches off
(demand 0), and the whole solve runs once more without it. Readouts keep its wanted MW and show it
dropped. This is a brownout's two outcomes: a load at or above its minimum runs weakly (its
under-power effect); below it, it stops.

**Charging.** After all passes, if the battery did not discharge, the reactor's remaining capacity
is routed to `eb` up to `min(12 MW, (capacity - charge) / (0.95 dt))`.

**Severed conduits and destroyed nodes** need nothing special: their capacities are zero in the next
sub-step's graph, and the paths go round them if any exist.

**Determinism and stability.** No randomness. Orders are the data's (loads, nodes, edges,
adjacency). The same state and inputs give the same allocation on the same build, which is what
the authoritative server and replays need (clients display the server's numbers; they never re-run
the solve). There is no iteration to converge: a fixed number of passes, phases and rounds, each
monotone, so nothing oscillates between sub-steps unless its inputs do. Tolerance 0.0005 MW.

**Battery energy**: `charge -= out x dt / 0.95` and `charge += in x dt x 0.95`, clamped to
0-1,800 MJ.

### 10. Heat and coolant

Every MW delivered becomes heat somewhere, by each load's `heat` route:

| Route | Where the heat goes |
| --- | --- |
| `loop` | Straight into the coolant loop (pumps, auxiliaries, scrubbers, cradles 30%) |
| `node` | A thermal mass of its own (`capacity_mj_per_k`), cooled by the loop (`loop_kw_per_k x flow`) and leaking to its room (`room_w_per_k`); the rest of its power is work that leaves the ship (thrust, shield field, bolts) |
| `room` | Into a named compartment's air (`life-support` takes it) |
| `duct`, `lights` | Into the supply air; into the rooms each lighting panel lights |
| `none` | Moved, not made (thermal control), or negligible |

**A system's thermal node:**

```text
Q_in   = fraction x alloc x (1 + over)
dT/dt  = (Q_in - G_loop x flow x (T - T_loop) - G_room x (T - T_room)) / C
```

`G_loop` is set so that a system at nominal sits 30 K above the loop (`fraction x nominal / 30 K`),
and `C` gives each a 90 s time constant. Warning at 363 K; above 393 K the system loses 0.01% of
integrity a second for each kelvin over, which `damage-control` reads.

**The loop**: 40 MJ/K of pressurized water and glycol, 740 kg/s at full flow (40 MW at a 15 K
rise), two pumps of half the flow each. **The radiators**:

```text
capacity = 40 MW x health x flow x (T^4 - 4^4) / (330^4 - 4^4)
bypass   = clamp((T - (330 K - 5 K)) / 5 K, 0, 1)
rejected = capacity x bypass
dT_loop/dt = (sum of heat into the loop - rejected) / 40 MJ/K
```

The bypass valve holds the loop between 325 and 330 K whenever there is less heat than the
radiators can reject: at cruise the loop sits at 328 K rejecting 24.4 MW of a 39.1 MW capacity, and
the console shows that margin. Without it (the first draft) the loop ran down to 293 K at cruise,
which no engineer would design.

**The rating is fictional, and stated as such**: a radiator rejecting 40 MW at 330 K with emissivity
0.9 needs about 33,000 m^2 of two-sided panel (`0.9 x 5.67e-8 x 330^4 = 605 W/m^2` a side), against
a hull of about 6,000 m^2 (an 84 x 24 x 13 m box has 6,840 m^2). The game keeps 40 MW so the loop has the right time constants; question
P4.

**Numerical stability at 10 Hz.** Explicit Euler on each thermal state. The shortest time constant
is the loop under its bypass: `40 MJ/K / (8 MW/K + 0.5 MW/K) = 4.7 s`, 47 sub-steps; the fastest
system node (the oxygen generator) is 90 s. Both are far above the stability limit `dt < 2 tau`.

### 11. Console readouts and previews (preview = resolver)

Every number engineering sees is the state the solve and the heat step produced, or a call to the
same functions on a copy (CLAUDE.md 6.1). `bridge-stations` owns the layout; this is the content.

| Panel | Readout | Source |
| --- | --- | --- |
| E1 Allocation | Per group: setpoint, wanted MW (tick), delivered MW (fill), dropped loads, priority; footer: supply and demand MW | The last solve |
| E1 ghost bar | Delivered MW if this group's setpoint were the slider's value, while dragging | `power::preview_group(group, sp)` = the solve on a copy of the setpoints |
| E2 Buses | Flow MW on every edge, node health, breaker states (closed, open, tripped, locked), severed conduits | The last solve; `damage-control` |
| E3 Reactor | State and scram cause, mode, throttle and target, thermal and electric MW, unused MW, blanket K, fuel kg, integrity | The reactor state |
| E3 preview | Blanket and loop temperature in 30 s at the proposed throttle | `heat::project`: the heat step at 1 s steps for 30 steps on a copy, loads held at the solve's allocation |
| E4 Coolant | Loop K, flow % and kg/s, radiators MW of capacity MW, pumps, systems over warning | The heat state |
| Battery | Charge MJ and %, out and in MW, reserve held or released, time to the reserve at the present draw | The battery state |
| Status strip | Reactor MW (and red when scrammed) | The reactor state |

The mockup demonstrates the rule: dragging an allocation slider calls `previewGroup` on the
library's solve, and in the scenario run "combat, shields to 150%" the preview said 18.00 MW wanted
and 18.00 MW delivered, and the next sub-step delivered 18.00 MW.

### 12. Walkthroughs, from the simulation

All start from a ship warmed up at cruise for 300 s (battery full at 1,800 MJ). Combat means the
combat preset with helm at 75% thrust, the dampers at 60% load, RCS at 50%, the shields
regenerating, three turrets firing, an active scan.

**Cruise.** 21.2 MW wanted and delivered; reactor at 44% (35.3 MW thermal); loop 328 K; radiators
24.4 of 39.1 MW; blanket 456 K.

**Combat.** 56.2 MW wanted against 48 MW. The reactor ramps from 44% to 100% in 28 s; the battery
covers up to 25.4 MW meanwhile, then 8.2 MW. It reaches the reserve 124 s after combat starts.
Then priority 3 is shed: the drive gets 3.1 of the 9.05 MW it wants (34% of the thrust helm asked
for), the RCS blocks 0.54 of 1.55 MW. The radiators saturate: the loop climbs to 344 K at 300 s and
353 K at 600 s (warning 360 K). The shield generator and turret nodes sit at 381-397 K, between
warning and damage. With every setpoint left at 100% instead of the preset, combat wants 58.1 MW.

**All out** (every activity at 1, setpoints 100%): 73.6 MW wanted. Priority 2 takes 43 MW, priority
3 would get 0.2 MW and drops out entirely: the drive stops. This is the moment engineering's
priorities decide whether the ship fights or runs.

**Overdrive** (throttle 120%; shields, turrets and drive at 150%; all activities at 1): 95.6 MW
wanted; the reactor delivers 56.8 MW at first, falling to 49.9 MW after 14 minutes as its integrity
wears to 85.5%. The shield generator and turrets reach their warning in 64-71 s and their damage
temperature in 221-224 s; the loop reaches its warning at 348 s. After 14 minutes the shield
generator and all four turrets are at 42% integrity. Overdrive is a minutes-long sprint that leaves
repairs behind it.

**A conduit cut: the port trunk midships (`k_mid_p`), in combat.** Nothing is lost: the starboard
trunk takes 27.9 MW, the forward cross-tie 8.1 MW and the main tie 7.3 MW, and every load keeps its
supply.

**Both midships trunks cut, in combat.** The forward ship is fed only by the command deck's aft
feeder (8 MW) and the battery. The reactor, following the load it can deliver, drops to 56% (27.1 MW
electric), the battery gives 29.1 MW and reaches the reserve in 24 s. Then the forward ship gets
what the aft feeder and the reserve users carry: the shields 5.6 of 15 MW, and the turrets, sensors,
gravity generator and forward RCS drop out (free fall forward of the hangar). Splicing the port trunk
at half capacity (15 MW, `damage-control`) brings the ship back to 31.9 MW: priority 2 at 69%,
priority 3 dropped.

**Scram in combat.** The battery gives its 30 MW limit against 54.8 MW wanted: priority 2 at 68%,
priority 3 dropped. It reaches the reserve 18 s later. Reset at once (an engineer already at the
panel): ignition and the pumps draw 8.8 MW from the reserve, everything else drops for 20 s, including the
computer core (automation stops) and the shields. Running 25 s after the scram; full output 70 s
after it. From the bridge, add the run to engineering (23 s).

**Scram from loop over-temperature** (overdrive with radiators at 60% health): scram after 248 s;
the reset is refused ("loop too hot") for 170 s while the pumps cool the loop on the reserve
(144 MJ); ignition then completes with 21 MJ left.

**A hit on the main switchboard**: `damage-control` walkthrough W1. The port section is destroyed,
the reactor carries on through the starboard section at 30 MW, the battery bridges 24.8 MW for 30 s,
then priority 3 drops and priority 2 runs at 68% until the section is rebuilt.

### 13. The data: `data/ships/tern/power.json`

Units are in the keys or in the file's `units` block. A missing optional field takes the code
default (one source); a present zero is zero; an unknown key, a non-finite number, a node or load id
that does not exist, a conduit whose route names a compartment not in the layout, or a negative
capacity stops startup with the file and field (CLAUDE.md 6.5).

| Field | Meaning |
| --- | --- |
| `solve.substep_hz` | Sub-steps a second (10) |
| `solve.quanta_rounds`, `solve.epsilon_mw`, `solve.dropout_resolves` | Rounds per phase (3), tolerance (0.0005 MW), re-solves after drop-out (1) |
| `reactor.system` | The layout system it is (`reactor`) |
| `reactor.thermal_rated_mw`, `conversion_efficiency` | 80 MW, 0.6 |
| `reactor.throttle_min`, `throttle_max`, `throttle_default` | 0.1, 1.2, 1.0 |
| `reactor.ramp_up_per_s`, `ramp_down_per_s` | 0.02, 0.1 (fraction of rated per second) |
| `reactor.fuel_energy_mj_per_kg`, `fuel_load_kg` | 353,000,000, 4.0 |
| `reactor.overdrive_wear_pct_per_min_at_max` | 1.0 |
| `reactor.heat.*` | Blanket capacity (MJ/K), loop and room couplings (kW/K, W/K), warning and scram (K) |
| `reactor.scram.*` | The five causes' thresholds and holds (section 3); `aux_rule` names the best-train rule |
| `reactor.reset.*` | The fixture, hold (3 s), loop and blanket limits (K) |
| `reactor.restart.*` | Ignition MW and s, starting throttle |
| `battery.*` | System, node, capacity, initial charge, rates, efficiencies, reserve, `heat_to` |
| `nodes[]` | `id`, `name`, `kind` (switchboard, emergency bus, panel), `compartment`, `center_m` |
| `generators[]` | `id`, `node`, `capacity_mw` |
| `ties[]` | `id`, `between`, `capacity_mw`, `closed` |
| `conduits[]` | `id`, `between`, `capacity_mw`, `route` (compartments, in order), `path_m` (polyline) |
| `loads[]` | `id`, `name`, `system` or `mount` or `center_m`, `node`, `priority`, `nominal_mw`, `standby_mw`, `setpoint_max`, `min_ratio`, optional `activity`, `heat` (`to`, `fraction`, and for `node`: capacity, couplings, `room`, warning and damage K), `under`, `over` (text for the console's help, not rules) |
| `groups[]` | `id`, `name`, `loads` |
| `presets` | Setpoint per group for each preset |
| `lighting` | Priority, W/m^2 normal and emergency, the normal-lighting threshold, and which compartments each panel lights |
| `coolant.*` | System, capacity (MJ/K), nominal, initial and warning K, pumps, flow per pump, design flow (kg/s) |
| `radiators.*` | Rating, rating temperature, background, bypass band, the law (as text) |
| `overdrive.*` | Heat factor, wear at 150% |

### 14. The Pi 5 budget this change spends

**Server CPU** (2 ms per ship per 30 Hz tick on one Cortex-A76). Measured in the instrument (node
22 on the cloud container's Intel Xeon at 2.8 GHz, five runs of 5,000 sub-steps): a whole systems
sub-step (power, heat, atmosphere, fire, crew) took 112-122 us, and 109-179 us in other runs and
with a breach and a fire burning; the power solve with drop-out took 29-45 us, and a console preview
33 us. Each sub-step runs the solve two or three times (the reactor's probe, the solve, one
drop-out re-solve). **Estimate for the Rust core on a Pi 5**: an A76 at 2.4 GHz is roughly 1.5-2
times slower per thread than that Xeon on branchy scalar code, and compiled Rust on typed arrays is
roughly 1.5-3 times faster than the JIT, so about 60-240 us per sub-step. Run once every three
ticks, that is at most 0.24 ms on the tick that runs it (12% of 2 ms), or about 0.08 ms a tick if
the sub-step's three parts are spread over the three ticks. Estimated, not measured: the probe
(`engine-stack` section 11) measures it.

**Memory**: per ship, the graph and state are under 64 kB (31 gas nodes, 75 links, 40 loads, 24
power edges, a 31 x 31 matrix, scratch). Allocated once at load, no per-sub-step allocation.
Negligible against 512 MB per session.

**Network** (`netcode-and-sessions`): a client showing an engineering console needs, at 2 Hz, each
load's wanted and delivered share (one byte each, 80 bytes), each edge's flow as a share of its
capacity (24 bytes), reactor, battery and loop values (about 24 bytes) and node health (14 bytes):
about 142 bytes, 2.3 kbit/s. Every client needs each compartment's lighting state (2 bits each),
sent on change. Breaker and setpoint changes are events.

**Client**: the console and the bus diagram are immediate-mode UI (`bridge-stations`); no rendering
cost in the 3D scene except lighting states, which are the baked sets of `light-baking`.

### 15. Reconciling the draws other changes assumed

`power-grid` owns every draw; the others take the supply ratio, so these changes rescale nothing in
their formulas.

| Change | Assumed | Proposed here | Effect there |
| --- | --- | --- | --- |
| `weapons-and-shields` | Turret charge 4 MW | 4 MW nominal, 6 MW at 150% | Aligned; 150% now matches sustained fire |
| `weapons-and-shields` | Shield generator 12 MW | 12 MW | Aligned |
| `weapons-and-shields` | Hoist 1.5 MW moving; loader 2 MW loading | 0.03 MW for the hoist and loaders together | Reloading is a crew and time decision, not a power one (question P5); its table should say so |
| `flight-and-navigation` | Main engines 60 MW at full thrust, 2 MW standing by | Impulse drive 16 MW nominal, 24 MW at overdrive, 0.2 MW standing by | Thrust is still `F_rated x s`; its power column should read 16 MW (question P6) |
| `flight-and-navigation` | RCS 8 MW at full use | Two blocks of 4 MW | Aligned |
| `flight-and-navigation` | Jump spool 40 MW for 20 s (800 MJ) | Not a load yet | A 40 MW spool is 83% of the reactor: a jump means everything else at standby for 20 s, or 27 s of battery. If the jump drive is accepted, it joins `loads` at priority 3 on `dp_drive`, whose feeders (40 MW together) would need raising |
| `ship-frames` | Dampers 2 MW standing by plus up to 6 MW | 8 MW nominal, 2 MW standby | Aligned |
| `bridge-stations` | Scram reset held 3 s; priorities 1-3; presets CRUISE, COMBAT, SILENT, EMERG; coolant flow in kg/s | As there | Aligned |
| `shuttle-bay-and-fighters` | Cradle and pad draws, gun capacitors | Cradles 0.6 MW, pad 0.05 MW; a craft's own capacitors are the craft's | Aligned |

### 16. Layout patches proposed

For `reference-ship-tern` to apply to `data/ships/tern/layout.json` (this change does not edit it):

1. Split the main switchboard: add a starboard section to `systems`:
   `{ "id": "main_switchboard_s", "name": "Main switchboard, starboard section", "kind": "power_distribution", "compartment": "engineering", "center_m": [-7.5, 0.0, -21.0] }`
   and rename the existing `main_switchboard` to "Main switchboard, port section".
2. Add the reactor panel to `fixtures`:
   `{ "id": "reactor_panel", "kind": "panel", "compartment": "engineering", "center_m": [0.0, 0.0, -21.4], "size_m": [1.6, 1.2], "facing_yaw_deg": 0, "note": "Hands-on reactor control on the mezzanine ring: scram reset (hold 3 s), throttle, coolant branch valves (power-grid)." }`
3. Add the coolant valve manifold to `fixtures`:
   `{ "id": "coolant_valves", "kind": "panel", "compartment": "engineering", "center_m": [-6.5, -3.5, -27.4], "size_m": [1.2, 1.4], "facing_yaw_deg": 0, "note": "Manual branch valves of the coolant loop, beside the pumps (power-grid)." }`

## Risks / Trade-offs

- **Combat over budget may frustrate rather than engage.** Combat wants 17% more than the reactor
  gives. Mitigation: the battery bridges two minutes, automation sheds the drive first, the console
  shows time to the reserve; question P1 asks with the shot.
- **Forty loads is a lot to show.** Mitigation: eleven groups on E1, with loads in a detail view.
- **The radiators are fictional.** Stated in the data and on the console; question P4.
- **The solve's cost grows with ships.** A Hound or a station gets a much smaller graph (a few
  loads, one bus); only the player ship runs this full model. Enemy ships use the same code with
  smaller data (CLAUDE.md 6.1).
- **Float results differ across platforms.** The server is authoritative and clients never re-run
  the solve, so this only matters for replays on another build, which the replay format must pin.

## Mockup shots

`docs/mockups/systems.html` (presents `power-grid`, `life-support`, `damage-control`), screenshots
in `docs/screenshots/mockups/`:

| Shot | Shows |
| --- | --- |
| `systems` (default) | Cruise, normal lighting: conduits by MW, the allocation bars, the bus table |
| `power-overview` | Combat, red alert, 60 s in: the battery bridging 8.2 MW, every conduit's load, priorities and setpoints |
| `power-severed` | The port trunk cut aft and midships in combat: the starboard trunk carrying the ship, the hangar and the port launch bay on emergency lighting |

## Open questions

Ids P (power). Questions with a shot go to the owner's survey with the shot beside them; the rest
are recommendations taken (ask only with screenshots, CLAUDE.md 13).

| Id | Question and the fact it turns on | Options | Recommendation | Shot |
| --- | --- | --- | --- | --- |
| P1 | Should combat fit in the reactor? The combat preset wants 56.2 MW against 48 MW: the battery bridges 124 s, then the drive gets a third of its thrust. | (a) As designed: combat is a power crunch that engineering manages. (b) A smaller combat preset that fits 48 MW. (c) A bigger reactor. | (a): it is the zero-sum moment, and it gives engineering its job. | `power-overview` |
| P2 | What does a room on a lost panel look like? The hangar and the port launch bay when their panel loses both feeds. | (a) Amber emergency lighting from the emergency bus. (b) Dark until repaired. | (a): readable and still dramatic; dark comes when the emergency bus fails too. | `power-severed` |
| P3 | Can engineering change priorities? `bridge-stations` E1 has a priority button per group. | (a) Groups 1-3 editable, the vital class fixed. (b) Fixed by the data. | (a). | `power-overview` |
| P4 | The radiators' 40 MW at 330 K is fictional (33,000 m^2 of real panel). | (a) Keep it, stated in the data and the console. (b) A hotter loop with plausible area. (c) Saturating heat sinks. | (a): the loop's time constants are what play needs. | none: recommendation taken (ask only with screenshots) |
| P5 | Hoist and loader draw: physics says tens of kW; `weapons-and-shields` assumed 1.5 MW and 2 MW. | (a) 0.03 MW: reloading is about crew and time. (b) Megawatts, to make reloading a power choice. | (a). | none: recommendation taken (ask only with screenshots) |
| P6 | The impulse drive's draw: 16 MW nominal here, 60 MW in `flight-and-navigation`'s table. | (a) 16 MW (24 MW at overdrive). (b) 60 MW and a 100 MW reactor. | (a): thrust scales with the supply ratio either way; a full burn should not need the whole reactor. | none: recommendation taken (ask only with screenshots) |
| P7 | Is restart hands-on only? | (a) Only at the reactor panel, as `bridge-stations` has it. (b) Remote after a delay. | (a): it is why the engineering bay is a place. | none: recommendation taken (ask only with screenshots) |
| P8 | Battery size: essentials run 5.8 min on it; a dead reactor ends the fighting. | (a) 1,800 MJ as designed. (b) 5,000 MJ, so a dead reactor still leaves a fight. | (a): the reactor is the ship's heart, and the battery buys the time to restart it. | none: recommendation taken (ask only with screenshots) |
