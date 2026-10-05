# Proposal: the Tern's power grid

## Why

The owner, 2026-10-04: "We're aiming for peak simulation systems here", and the ship has "a
starship energy simulation system" and "an engineering bay". Power is the system every other
system draws on: shields, turrets, the drive, the dampers, gravity, life support and the computer
that runs automation all compete for one reactor's output. The vision's first pillar is "a hit in
the switchboard cuts a bus, the bus feeds the scrubbers, the CO2 climbs in the quarters"
(`docs/design/vision.md`), and its fourth is "engineering's power budget decides whether shields
or turrets win". Neither is possible with star-crew-64's model, three channels whose percentages
sum to 100 and scale every system by `power / 33` (`docs/analysis/star-crew-64.md`): there was no
reactor, no wire, no breaker and no heat, so nothing could be cut, tripped, overloaded or
repaired.

This change makes power a flow network that can be followed from the reactor to a bolt, with
every number a console shows computed by the same solve that delivers it (CLAUDE.md 6.1 and 7).

## What Changes

- **A fusion reactor** in engineering: 80 MW thermal, 48 MW electric at 60% conversion, throttle
  10-120%, ramping at 2%/s up and 10%/s down, 4.0 kg of deuterium and helium-3 (204 days at full
  output, a campaign consumable). Automation follows the load the grid can actually take; a player
  can run it by hand, and power the grid cannot take heats the blanket.
- **Scram and hands-on restart**: five automatic causes (loop over 380 K, blanket over 820 K,
  coolant flow under 30%, both auxiliary trains under 80% supply, integrity under 25%). A scrammed
  reactor is restarted only by someone at the reactor panel in engineering (hold 3 s), refused
  while a cause persists, then ignites for 20 s on 8 MW from the battery's restart reserve and
  ramps to full in 45 s.
- **A battery bank** in the forward switchboard: 1,800 MJ, 30 MW out, 12 MW in, 95% each way, with
  a 350 MJ restart reserve sized for one cool-down and one ignition.
- **A real topology**: port and starboard main switchboard sections on opposite sides of
  engineering, a forward switchboard with a cross-tie and the emergency bus, ten distribution
  panels, and fifteen conduits routed through named compartments (`data/ships/tern/power.json`), so
  a hit that crosses a conduit's route can cut it.
- **Forty loads**, each with nominal, standby and maximum draw, a priority, a minimum supply below
  which it switches off, a heat route and its under- and over-power effects: 81.35 MW nominal in
  all, 12.92 MW standing by, 116.12 MW at every overdrive limit.
- **Allocation**: engineering sets each load group's setpoint (0-150%) and priority (1-3); four
  presets (cruise, combat, silent, emergency); overdrive above 100% costs heat and wear.
- **The one solve** at 10 Hz: priority classes in order, the reactor before the battery,
  proportional shares within a class routed over conduit capacities, drop-out below a load's
  minimum, battery charging from surplus only. Severed conduits and destroyed switchboards change
  capacities, and flows reroute.
- **Heat**: every delivered MW ends as heat in the coolant loop, a system's own thermal mass, a
  room, the supply air or the lights. A 40 MJ/K loop, two pumps, and radiators rated 40 MW at
  330 K with a bypass that holds the loop near 330 K at low load.
- **Console readouts and previews from the solve**: engineering's allocation bars, bus diagram,
  reactor and coolant panels, and every "what if" ghost bar call the solve that steps the ship.
- **A live mockup**, `docs/mockups/systems.html`, running the proposed library
  `docs/mockups/lib/shipsystems.js` on the proposed data; every table in the design comes from
  running that library headless in node.

## Capabilities

### New Capabilities

- `power-grid`: the reactor, battery, switchboards, buses, conduits and breakers, the loads and
  their allocation, the per-sub-step solve, heat and coolant, scram and restart, and the console
  readouts and previews that come from the solve.

### Modified Capabilities

None.

## Impact

- **Code (when built)**: `sc-core::power` (topology, demand, solve, battery, reactor, scram),
  `sc-core::heat` (thermal nodes, loop, radiators), validated loading of `power.json`; the
  engineering console and engineering bay panel in `sc-client`.
- **Data**: `data/ships/tern/power.json` (proposed here; the mockup reads it today).
- **Other changes**:
  - `weapons-and-shields`: turret charging is aligned at 4 MW nominal; this change proposes the
    hoist and loaders at 0.03 MW, not the 1.5 MW and 2 MW it assumed (design section 15; adopted
    there 2026-10-04).
  - `flight-and-navigation`: the impulse drive is 16 MW nominal and 24 MW at overdrive, not the
    60 MW it assumed; RCS is aligned at 8 MW; a 40 MW jump spool would take 83% of the reactor
    (section 15). Adopted there 2026-10-04, with the spool resized to 20 MW for 40 s.
  - `ship-frames`: the dampers are aligned at 2 MW standing by plus up to 6 MW.
  - `bridge-stations`: the engineering console's panels E1-E4 and the bay console's reactor tab
    show this change's readouts; scram reset is held 3 s, as there.
  - `life-support` and `damage-control`: the plant's loads, the bay pumps, lighting, and every hit
    on a system, switchboard or conduit.
  - `reference-ship-tern`: a layout patch moves the main switchboard's starboard section to the
    starboard side of engineering and adds the reactor panel fixture (design section 16).
- **Pi 5 budget** (`engine-stack` section 5): about 0.1-0.2 ms per ship per 30 Hz tick averaged on
  the server, under 64 kB of state, about 2.3 kbit/s to a client showing an engineering console.
  Estimated, to be measured by the probe.
