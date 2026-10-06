# Proposal: the Tern's air

## Why

The owner, 2026-10-04: "We're aiming for peak simulation systems here", and the ship has "life
support" and "a fully working shuttle bay". The vision's first pillar follows a cut bus to "the
CO2 climbs in the quarters", and its fourth says "flight ops cannot launch until the bay is empty
of air, and the bay cannot empty while someone stands in it unsuited" (`docs/design/vision.md`).
Both need air that is a quantity: moles of gas in a volume at a temperature, moving through
openings, made and cleaned by a plant that draws power, and breathed by crew whose state depends on
it. star-crew-64 had none of this: a room was on fire or not, and a vent killed the fire and dealt
30 HP to everyone in the room (`docs/analysis/star-crew-64.md`).

CLAUDE.md section 7 requires it: "Air moves between compartments by pressure difference through
open portals. A readout on a console shows the simulated value."

## What Changes

- **A gas state per compartment**: moles of oxygen, nitrogen, carbon dioxide and smoke, and
  internal energy, giving pressure and temperature by the ideal gas law, with the fittings' heat
  capacity; 30 compartments, the air duct, and space at 0 Pa.
- **Flow through every opening** of the one compartment graph: doors, hatches, ladders, the hoist,
  bay doors, vents, valves and breaches, by the compressible orifice equation with choked flow,
  solved implicitly so a breach to vacuum never overshoots, oscillates or makes negative gas.
- **Doors that keep the ship in compartments**: closed unless someone passes or the damage control
  board holds them, an interlock above 20 kPa of difference, and self-closing on a pressure alarm.
- **Ventilation**: an air handler and a 40 m^3 duct exchanging air with every room, with dampers
  that trip on a leak or smoke and refill a sealed room afterwards.
- **The plant**: an oxygen generator, CO2 scrubbers, thermal control, and make-up from 2,740 kg of
  reserve gas, each drawing power from `power-grid`.
- **Crew**: metabolism, and effects from oxygen partial pressure, CO2, smoke dose, heat, cold and
  pressure, with times of useful consciousness from published tables.
- **Breaches, bays and the airlock**: decompression times for every compartment; launch bay
  pump-down in 23.6 s and hangar pump-down in 207 s into a receiver that returns the air; emergency
  venting; a 39 s airlock cycle (layout v2, 2026-10-05).
- **Previews from the model**: time to pump down or refill, time to Armstrong's limit, a refill's
  gas cost.
- **Proposed data**: `data/ships/tern/atmosphere.json`; the live mockup
  `docs/mockups/systems.html`.

## Capabilities

### New Capabilities

- `life-support`: the gas state, flow through openings, doors and ventilation, the plant and the
  reserves, metabolism and crew effects, breaches, bay pump-down and venting, the airlock cycle, and
  the readouts and previews.

### Modified Capabilities

None.

## Impact

- **Code (when built)**: `sc-core::atmosphere` (state, implicit flow, mixing, ventilation), `sc-core::plant`
  (generator, scrubbers, thermal control, make-up, pumps), `sc-core::crew_effects`; the life support
  readouts on the engineering, flight ops, bay control and damage control consoles.
- **Data**: `data/ships/tern/atmosphere.json` (proposed here; its `fire` block is `damage-control`'s).
- **Layout** (`reference-ship-tern`): the duct node and thirty vent portals move into the one
  compartment graph on acceptance, plus a fixture for the reserve bottles (design section 19).
- **Other changes**:
  - `shuttle-bay-and-fighters`: its assumed times are replaced by computed ones (launch bay
    pump-down 23.6 s to 5 kPa, hangar 207 s, repressurization 13.0 s); proposes launching at the
    pumps' 5 kPa stop rather than 1 kPa (design section 15).
  - `power-grid`: the plant's loads and the bay pumps' power.
  - `damage-control`: breaches, fire chemistry and suppression gas, venting a compartment.
  - `crew-on-deck`: suits, carrying crew out, the effects' presentation.
  - `bridge-stations`: E5 and the flight ops and damage control panels show these readouts.
- **Pi 5 budget**: the atmosphere step is part of the systems sub-step costed in `power-grid`
  section 14 (about 60-240 us per sub-step on the Pi, estimated); about 2.5 kbit/s to a client
  showing a damage control or life support panel.
