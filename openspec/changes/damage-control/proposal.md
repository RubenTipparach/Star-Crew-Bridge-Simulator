# Proposal: damage control

## Why

The owner, 2026-10-04: "peak simulation systems", a ship with "an engineering bay", "life support"
and "a starship energy simulation system", played by "a core of 4 players". Damage is where those
systems meet the crew: a hit has a place, the place has systems, conduits and people in it, and what
breaks there decides what the crew does next. The vision's first pillar is the chain "a hit in the
switchboard cuts a bus, the bus feeds the scrubbers, the CO2 climbs in the quarters", and its third
is "crew stand up, walk to engineering, carry an extinguisher" (`docs/design/vision.md`).

star-crew-64 proved the shape and showed its limits (`docs/analysis/star-crew-64.md`): fire that
persists until crew act, an extinguisher carried by a player, a vent that trades crew safety for a
clear room, and station damage passed to its occupant all worked. But damage was routed by a
four-way angle test, a room caught fire after three hits in 180 frames, a fire never went out on its
own, a vent dealt a flat 30 HP, and repair was a held button scaled by engineering's own health,
with a cosmetic pulse. `weapons-and-shields` now hands this change a hull hit with a point, a
direction and an energy in megajoules; this change decides what it does.

## What Changes

- **One hit resolution** for every ship: a hull section's armour absorbs up to 4 MJ (worn by each
  hit), the rest opens a breach sized by its energy and marches inward along the hit's direction,
  losing energy with distance and at each bulkhead, depositing it step by step; systems, switchboard
  sections, conduits, doors and crew near each step take damage with distance; fire may start.
- **Hull sections** (9 spans by 6 faces) with armour that wears: 24 pulse bolts wear one through,
  then bolts hole the room behind it.
- **System damage states** (nominal, damaged, disabled, destroyed) with the capability `power-grid`
  reads, and damage from heat and overdrive.
- **Fire** from energy, fuel and oxygen: t-squared growth, a ceiling set by floor area and oxygen,
  combustion chemistry that uses oxygen and makes CO2 and smoke for `life-support`, spread through
  open doors and hot air, and the magazine's cook-off rule.
- **Suppression**: six extinguishers, water mist in engineering, the hangar and the drive, inert gas
  in five rooms, and venting, each with a measured effect.
- **Repair**: kits at 1% a second for a player, parts for disabled and destroyed systems, conduit
  splices, switchboard rebuilds, breach plates from inside and EVA from outside, with the time
  previewed by the function that resolves it.
- **Damage control teams**: two teams of two NPC crew bodies with speeds, rates and the same risks
  as players, dispatched from the damage control board.
- **The engineering bay as a place**: the reactor panel, the coolant valves, the switchboard
  sections' local breakers that still work when the bridge's control is lost, and the fires and
  breaches that happen there.
- **Three combat walkthroughs**, simulated: a 60 MJ hit on the main switchboard, a 40 MJ hit under
  a launch bay, and pulse fire raking the forward switchboard.
- **Proposed data**: `data/ships/tern/damage.json` and the `fire` block of `atmosphere.json`.

## Capabilities

### New Capabilities

- `damage-control`: hit resolution, hull sections, breaches, system states, fire and its
  suppression, the magazine's cook-off, repair, damage control teams, and the engineering bay's
  hands-on controls.

### Modified Capabilities

None.

## Impact

- **Code (when built)**: `sc-core::damage` (the hit resolution `weapons-and-shields` calls with its
  `HullHit`), `sc-core::fire`, `sc-core::repair`, team behaviour in `sc-core::automation`; the damage
  control board and the engineering bay panels in `sc-client`.
- **Data**: `data/ships/tern/damage.json` (proposed here) and `atmosphere.json`'s `fire` block.
- **Layout** (`reference-ship-tern`): fixtures for the damage control lockers, EVA suits and spare
  parts (design section 15).
- **Other changes**: `weapons-and-shields` (the `HullHit` it sends; the magazine; turret and tube
  states), `power-grid` (capability, switchboard and conduit damage), `life-support` (breaches,
  fire gases, venting, refills), `crew-on-deck` (carrying kits and extinguishers, suits, injury,
  the teams' bodies), `bridge-stations` (the damage control board D1-D4), `shuttle-bay-and-fighters`
  (cradle and drop door damage).
- **Pi 5 budget**: fire and repair are part of the systems sub-step costed in `power-grid` section
  14; a hit that passes the armour costs about 0.55 ms in the JavaScript instrument and an estimated
  0.1 ms in the Rust core, capped at four such hits a tick.
