# Design: the reactor's cooling, its damage and its operator

## Context

`power-grid` 2 and 10: the reactor makes `P_th = throttle x 80 MW x integrity`, 60% of it electric; the rest heats
the blanket (10 MJ/K), which gives it to the loop at `110 kW/K x flow`. The loop is 40 MJ/K of pressurized water and
glycol, 740 kg/s at full flow, two pumps of half the flow (0.4 MW each, class 0, never shed). Radiators reject
`40 MW x health x flow x (T^4 - 4^4) / (330^4 - 4^4)` through a bypass that holds the loop at 325-330 K. Scram on
the loop above 380 K for 2 s, the blanket above 820 K, or flow under 30% at a throttle above 20% for 5 s. The repair
game for the reactor (`repair-minigames` 6b) holds the plasma ball and its ring with two sliders.

## 1. Two components

| Component | Parts (each a damage-control system with integrity) | Where | Its repair game |
| --- | --- | --- | --- |
| **Magnetic core** | The containment coils: today's reactor integrity, `P_th` scales with it | Engineering, the reactor | "Reactor: magnetic core" (the plasma ball and ring, unchanged) |
| **Coolant system** | Two coolant tanks (port and starboard); eight loop pipe segments (four hot leg: core to exchanger, four cold leg: exchanger to core); core pumps A and B; the heat exchanger (the chiller); radiator pumps port and starboard | Engineering and its pipe runs (`engineering-fitout`), the radiator pumps in the drive | Pipes: "Coolant pipes" (4). Tanks: the pipe game on the tank's own fittings. Pumps and exchanger: the kit's screw panel and a part step, as other machines |

A hit's march damages these parts by the same rule as any system in a room (`damage-control` 1). The core keeps
today's rules.

## 2. Coolant: inventory, leaks and flow

- **Inventory.** The loop holds 11,100 kg of water and glycol (40 MJ/K at 3.6 kJ/(kg K)); each tank holds 2,000 kg of
  reserve. The loop's heat capacity follows its inventory: `C_loop = 3.6 kJ/(kg K) x m_loop`.
- **Leaks.** A pipe segment or tank below 75% integrity leaks `20 kg/s x (75 - integrity) / 75`: a segment at 0% leaks
  20 kg/s, the loop's 11.1 t in under 10 minutes. The leak is hot (the segment's leg temperature): it heats
  engineering's air and fills it with steam (`life-support` takes the heat and water vapour as humidity; a scald of
  0.5 HP/s within 2 m of a leak above 343 K, through `crew::injure`).
- **Makeup.** The makeup valve and pump (0.05 MW) feed the loop from the tanks at up to 10 kg/s while it is open and
  the loop is under 100%. Automation opens it below 95%.
- **Flow.** `flow = pumps x min(1, (m_loop / m_full - 0.6) / 0.2) x segments`, where `pumps` is the two core pumps'
  speeds and capabilities summed (each half the flow), the middle term is cavitation (full flow at 80% inventory and
  above, none at 60%), and `segments` is the worst segment on each leg (a segment's capability, `damage-control` 2).
  An isolated segment (design 4) carries nothing, and its leg goes through the bypass at half flow.
- **The chiller** (the heat exchanger to the radiator loop): radiator rejection becomes
  `capacity x exchanger capability x radiator pumps`, where `radiator pumps` is the two radiator pumps' speeds and
  capabilities summed (each half). The radiators' own health stays `power-grid`'s.

## 3. The two legs, in words a player uses

- **Cold leg**: chilled coolant pumped **into the core**. Its temperature is the loop's (325-330 K at cruise).
- **Hot leg**: heated coolant **out of the core** to the chiller: `T_cold + Q_blanket / (flow x 3.6 kJ/(kg K))`, 15 K
  above the cold leg at full flow and full heat. The hot leg over 380 K for 2 s is today's loop scram.

## 4. The coolant pipe game (`repairs/pipes.js`)

A damaged segment, its job in the kit's step rule (a fumble costs 5% and the system's hazard). More than the shower's
puzzle: two legs, valves, and the order matters.

1. **Isolate.** The engineering pipe run as a one-line plan with its valves. Close the two valves either side of the
   cracked segment (it is dripping, the plan's leak marker on it). The bypass opens on its own. Closing a valve that
   cuts the core's feed instead is the fumble "Core starved: the blanket heats" (the blanket +20 K).
2. **Rebuild.** The segment's run as a tile grid with two pairs of ends: hot (red) and cold (blue). Turn tiles to join
   each pair without the two legs meeting; crossover tiles let one leg pass over the other. Cracked tiles drip and
   must be swapped for a new one from the parts tray (drag it on) before they carry anything. Larger runs on later
   steps; more crossovers.
   **Coolant flows through it** (owner, 2026-10-09: "pipes on coolant need work to actually function like the shower
   thing"): as in the shower's run, coolant enters at each leg's inlet and fills the tiles it is connected to, hot
   red with chevrons and cold blue with dots, so a player sees a leg reach its outlet, stop at an open end (and spray
   there), or meet the other leg. The FILL button runs the coolant through what is built; the step completes when both
   legs reach their outlets, unmixed, with no cracked tile carrying.
3. **Refill and bleed.** Open the valves downstream first, then upstream, then hold the bleed valve at the run's high
   point until the gauge's needle settles (air out, coolant in). Upstream first is the fumble "Water hammer: the
   joint jumps" (the segment loses 10%, and 2 HP to anyone within 2 m).

## 5. The coolant balance game (`repairs/coolant.js`), the operator's duty

Not a repair: the engineer's hand on the loop when the automation's is not good enough. It is the engineering
bench's E4 panel turned into the job, at the `eng_main` seat or the bridge's Engineering console.

- **The picture.** The loop as a ring: the core at the top, the chiller at the bottom, the cold leg up one side (blue),
  the hot leg down the other (red to orange with its temperature), the tanks beside it with their levels, the
  radiators beyond the chiller. Two needles that matter: the hot leg's temperature against its band and limit, and the
  core's heat against the flow's capacity to carry it.
- **The controls**: pump speed (one lever for both core pumps, or split), the chiller's bypass (how much of the hot
  leg goes through the exchanger), the radiator pumps' speed, and the makeup valve. Each is one picture; the exact
  values on hover.
- **The goal**: the hot leg between 335 and 350 K and the cold leg between 320 and 335 K, with the makeup keeping the
  loop over 80%, **whatever state the reactor is in**: cruise, a combat surge to 100%, overdrive at 120%, a scram and
  restart, one pump lost, a leak, a radiator pump down. Held in band, the job's bar fills; the hot leg past 370 K is
  the fumble "Loop over-temperature warning", and past 380 K for 2 s is a real scram.
- **In the game proper** this is not a mini-game with steps but the panel itself: the same controls on the console,
  the same physics. The mockup plays it as a job so it can be tried alone.
- **Automation** (`bridge-stations` 9, Engineering): holds the cold leg at 327 K with both pumps at the flow the heat
  needs and the radiator pumps at full, reacting every 3.0 s. With a pump or segment lost it does not rebalance the
  bypass or open the makeup early, so a damaged loop under load drifts hot: the player does better.

**As built in the mockups** (2026-10-09): `repairs/pipes.js` plays design 4 (the dripping segment is one of the middle
two on its leg, so isolating it never needs a main valve; a disabled job adds fetching the spool and a larger
rebuild). `repairs/coolant.js` plays design 5 with the loop at six times ship time, so a step's drift shows within
its share; the console's panel runs at ship time. It simplifies the blanket away (the hot leg follows the heat with a
6 s lag), and the hold pauses rather than drains out of band. The HP harm and the segment's 10% on a water hammer are
named in the fumbles but not simulated there; each fumble costs the kit's 5%.

## 6. Cooling on the engineering bench

- **A twelfth load group, Cooling**, on E1: core pumps A and B (0.4 MW each), radiator pumps port and starboard
  (0.6 MW each, new), the makeup pump (0.05 MW, new). 2.05 MW at full speed.
- **Its setpoint is the pumps' speed** (0-120%; 120% is overdrive: 1.2 times the flow for 1.44 times the power, and
  wear as any overdrive). Automatic by default: Cooling follows the heat (design 5's automation).
- **Priority 0** by default (never shed, as today's class 0), but shown, and the engineer can lower it, which lets
  load shedding take it when supply falls short. A breaker on the Cooling feed can be opened or locked like any other
  (`power-grid` 5), which stops the pumps: the loop's flow falls to its natural circulation (5%).
- **E4 Coolant** becomes the balance panel of design 5: the ring, the two needles, the four controls, AUTO and MANUAL.
- **In the console mockup** (`consoles.html`, 2026-10-09) the balance panel and the reactor readout are one panel,
  REACTOR (6,0,4,4): the core's output dial sits at the top of the ring, so the console keeps the glance-first rule of
  four panels (`bridge-stations` 8.0); POWER narrows to six columns for twelve faders and the battery's charge moves to
  BUSES. Recommendation taken (ask only with screenshots): `docs/screenshots/mockups/consoles-engineering-cooling.png`
  and `consoles-engineering-cooling-breaker.png`. The mockup's automation never runs the core pumps below full speed
  (`power.json` `coolant.automation.pump_speed_min`), so `power-grid`'s walkthroughs, measured at full flow, still hold;
  slowing them at low heat waits for task 2.3.

## 6a. The reactor system screen (owner, 2026-10-09)

The owner: "I'd like a screen to show the entire reactor system. And if flow control of pumps aren't working
properly I can diagnose this by looking at the master reactor screen and click into the different parts to fix it."

- **Where.** The reactor's own console at the `eng_main` seat beside the reactor, and full screen from the bridge's
  Engineering console (a tap on the REACTOR panel's ring opens it). One screen, the whole system:
  the magnetic core, both tanks, all eight pipe segments, core pumps A and B, the chiller, both radiator pumps, the
  radiators, the makeup valve, the bypass, and the Cooling feed's breaker with the power each part draws.
- **Diagnosis by looking.** Every part is drawn where it sits in the loop, coloured and shaped by its state
  (`damage-control` 2's bands: sound, damaged, disabled, destroyed), with what flows through it: the pipes as thick as
  their flow, dashes moving with it, the hot leg red to orange by temperature; a leak sprays from its segment; a pump
  turns at its speed, or stands still. A fault shows where it is: a pump at half speed turns slowly and the flow
  past it thins; a cracked segment drips and the inventory falls; a fouled chiller passes hot coolant back into
  the cold leg; an open Cooling breaker stops every pump at once.
- **Click into a part to fix it.** Tapping a part shows its card (integrity, what it does to the loop, the repair's
  time and parts from `damage::repair_time`) and a FIX button that opens its repair game: a segment or a tank the
  coolant pipes game (4), a pump or the chiller its screw panel and part step, the core the magnetic core game, and
  the balance itself the coolant balance (5). Finishing the game repairs that part in the simulation, and the screen
  shows the loop recover. In the game proper the repair is done at the part, by a body that walks there; the screen is
  where the fault is found and the job sent (the damage board's queue).
- **What the simulation needs.** The parts' integrity, the segment leaks, the exchanger's capability and the pumps'
  speed are design 1-2's, written once in `shipsystems.js` (the mockups) and `sc-core` (the engine). The screen reads
  them; it computes nothing of its own.
- **The mockup**: `docs/mockups/reactor-system.html`, with scenario buttons that break things (a pump at half
  capability, a cracked hot leg segment, a fouled chiller, a radiator pump down, the Cooling breaker open, a tank
  leak) so the diagnosis can be tried, and the repair games embedded from `repairs/`.

## 7. The Pi 5 budget

| Item | Cost | Against |
| --- | --- | --- |
| Simulation | 14 more parts in the damage state, the loop's inventory and two tanks, the leak and makeup terms in the heat step: a few dozen flops at 10 Hz | Server step, negligible |
| Network | The loop's state at 5 Hz (inventory, two leg temperatures, flow, four control positions): about 16 bytes | Negligible |
| Console | E4 redrawn as the ring: about 40 shapes, within the console's UI draw budget | UI draw calls |

## Risks / Trade-offs

- **A loop the automation runs well at full health is a panel nobody touches.** That is intended: the duty matters
  when the system is hurt, which is when the engineer is needed.
- **Leaks drain the loop fast** (10 minutes at a segment's worst). The makeup and the tanks give minutes, and isolating
  a segment stops the leak at once, so the first act is always step 1 of the pipe game.
- **Two more games to learn.** Both use the kit's controls (valves are buttons, tiles turn, levers drag), and the
  balance game is the console's own panel.
