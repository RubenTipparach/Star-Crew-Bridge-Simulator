# Proposal: the reactor's cooling, its damage and its operator

## Why

The owner, 2026-10-09: "Let's also upgrade the reactor repair system a bit too. Reactor has two components the
magnetic core and the coolant tanks. As an operator cooling pipes can be damaged requiring players to play a pipe
repair mini game, similar to the shower one but maybe more complex or interesting. Then there should be a coolant
balance/flow mini game to make sure whatever state the reactor is in, a proper coolant flow is pumping into the core
and heated water is being chilled. The cooling system will need to be powered, so that's another thing to add to the
engineering bench."

`power-grid` 10 already has the loop: 40 MJ/K of water and glycol, 740 kg/s at full flow from two pumps of half the
flow each (0.4 MW each, never shed), radiators of 40 MW capacity by health and flow, and a bypass valve that holds the
loop at 325-330 K by itself. The reactor is one system with one integrity. Nothing in the loop can be damaged except
the radiators' health, nobody operates it, and its power is invisible on the engineering console.

## What Changes

- **Two components** (design 1): the reactor's **magnetic core** (containment: today's reactor integrity and today's
  repair game) and its **coolant system**: two coolant tanks, the loop's pipe segments, the two core pumps, the heat
  exchanger (the chiller) and its radiator pumps. Each part can be damaged and repaired on its own.
- **Coolant that leaks** (design 2): the loop has an inventory in kilograms; a damaged pipe segment or tank leaks it
  (hot, into engineering), and below 80% the pumps cavitate and flow falls. The tanks top it up through a makeup valve.
- **The pipe repair game** (design 4): isolate the cracked segment with the right valves, rebuild the run with two legs
  (hot and cold) that must not mix, swap the cracked pieces, then refill and bleed in order.
- **The coolant balance game** (design 5): an operator's duty at the engineering bench: pump speed, the chiller's
  bypass and the makeup valve, so the core gets the flow its heat needs and the hot leg is chilled, through every
  reactor state. Automation does it at full health and does it worse when the system is hurt.
- **Cooling on the engineering bench** (design 6): a twelfth load group, **Cooling** (the two core pumps, the two
  radiator pumps and the makeup pump), on the power allocation panel with its own setpoint, priority 0 by default
  (never shed) but visible, breakable and lockable like any other.
- **Mockups**: two new games in `repairs.html` (coolant pipes, coolant balance), the reactor game renamed "Reactor:
  magnetic core", and the Cooling group on the Engineering console in `consoles.html`.

## Capabilities

### New Capabilities

- `reactor-cooling`: the coolant system's parts, inventory, leaks and damage; manual coolant control; the cooling load
  group.

### Modified Capabilities

- None in `openspec/specs/`. `power-grid` 10 keeps its loop equations and gains inventory, segments and the
  radiator pumps' power; `repair-minigames` gains two games; `bridge-stations` E1 and E4 gain the Cooling group and
  manual control.

## Impact

- `power.json` (the coolant block, the Cooling group's loads), `damage.json` (the coolant system's parts), the heat
  step (`heat::step`), the engineering console's E1 and E4, `docs/mockups/repairs/pipes.js` and `coolant.js`,
  `docs/mockups/consoles.html`, `docs/mockups/lib/shipsystems.js`.
