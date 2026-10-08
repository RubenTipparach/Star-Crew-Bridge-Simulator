# Proposal: ship interactables, a ship you can use

## Why

The owner, 2026-10-08: "I'd like more interactive stuff on the ship in general like toilets that you can
flush.. showers you can turn on. Ability to change lighting in bedrooms. RGB, dim lights, etc. I'll let
you come up with more ideas."

Today the ship's furniture is scenery: a toilet stall, a shower, a galley counter and a bunk are props a
body walks round. Only doors, hatches, ladders, the lift and the seats answer Use (`crew-on-deck`
sections 4-6, `bridge-stations`). A crew on a long watch spends time away from the consoles, and a ship
that answers their hands is a ship they live on: the reason to walk to the head, the mess and the
quarters between fights.

## What Changes

- **One interaction model for every fixture** (design section 1): a prop declares its use points, Use
  (E, gamepad A) within reach acts on it, the server owns its state and replicates it, the same as a
  door. One rule, one code path (CLAUDE.md 6.1).
- **The owner's three**: a toilet that flushes (sound, a swirl, 6 L from the grey-water tank); a shower
  that runs (water, steam that fogs the stall, 8 L/min of heated water drawn from the tank and power for
  the heater); room lighting with on/off, a dimmer and a colour, by a panel at each cabin door (presets
  Work, Relax, Night and Off, and any colour).
- **More, proposed** (design section 3): sinks and basins; the galley's dispenser (a mug of coffee or a
  ration tray, carried and set down); lockers that open and hold items; bunk curtains; window shutters,
  which close by themselves on red alert; light switches in every room; an intercom panel per room; a
  radio in the quarters and the mess; the medbay bed's height and back; the mess table's game screen;
  the Petrel's folding seats (`ship-props` 4h).
- **The sim behind them**: a small water loop (potable and grey tanks, recycled by life support) and
  each fixture's power draw, through `life-support` and `power-grid`, so a long shower on emergency power
  is a small, real choice.
- **Room lighting is cheap**: the baked lamp light of a compartment is scaled and tinted by a
  per-compartment uniform the deck shader already has a slot for (the dimmer, `deck-pipeline`), so a
  player's colour costs no geometry and no re-bake.

## Impact

- `crew-on-deck` (Use and its prompt), `ship-props` (use points in props.json), `life-support` (the water
  loop it left out), `power-grid` (loads), `light-baking` and `deck-pipeline` (the room tint), `netcode`
  (fixture state in the snapshot), the mockups (the deck plan shows them), the engine (`sc-core::fixture`).
