# Proposal: a minimap while you walk

## Why

The owner, 2026-10-17: "I need to be able to see a map of where I am at all times on the ship when I'm away from a sit
station".

Today a player on foot learns where they are from the room's walls and nothing else. The ship map (M, `ship-plan-view`
5) shows everything, but it is a screen you open: the walk stops behind it, and on a phone the MAP button is the only way
to it. The Tern has three decks, about 70 m long, two stair towers, a lift, ladders and turret pods (`deck-access`): a
crew sent to a fire on deck C, or a new player looking for the helm, walks blind between one opened map and the next.

## What Changes

- **A small map of the deck you are on, always there while you walk** (design 1, 2): bottom right on the desktop, in
  the MAP button's corner on a phone. It goes when you sit (the console takes the screen), when the full map or the menu
  is open, and comes back when you stand.
- **What it shows** (design 3): the deck from above with the bow to the right (the ship map's deck view), a window about
  32 m by 18 m that follows you; every room's floor, the one you are in lighter; doors as gaps in the walls; every way off
  the deck (ladder, hatch, stair, lift) as one glyph with an up or down chevron; the crew as the ship map's markers; you
  as an arrow of your facing; fire, damage and the injured in the state colours with their shapes; the room's name and
  the deck's letter in a band at its top. No other words.
- **It follows you up and down** (design 5): it shows the deck your feet are on, switching when you step off a ladder,
  a stair or the lift onto another deck.
- **It opens the full map** (design 6): a click or a tap on it (or M, as today) opens the ship map on the deck you are on.
  On a phone it is the MAP button: the button's corner holds the minimap with the map icon in its band.
- **One picture, from the walk's own floor** (design 7): drawn from the compiled deck the body walks (its walk grid and
  compartments), never from a second room list, so deck A is the command suite the player stands in. The markers are the
  ship map's, from one function.
- **Mockups** (design 9): `ui-art-direction.html` (the "Minimap (proposed)" page: desktop on deck A with the first-time
  card, deck B, red alert) and `touch-controls.html` (`?mini=1`, or the menu's Minimap button: the phone in the corridor,
  on the bridge, at red alert), drawn by one `docs/mockups/lib/minimap.js`.

## Impact

- `sc-client`: a HUD element on foot (`hud.rs`), the deck's plan built once at load beside the walk grid (`onfoot.rs`),
  the ship map's bodies list moved into one function both maps call (`main.rs` `draw_map`), a click or tap target (the
  touch layer's MAP button grows into it on a phone, `touch.rs`).
- `sc-core::nav`: the walk grid's cells of one deck and its `deck_of` made public, so the map and the minimap ask one rule.
- Data: the minimap's sizes in `data/ui/ui_style.json` (generated from the mockup once approved) and the phone's size in
  `data/input/touch.json`.
- Cost: one UI batch of a few hundred vertices on foot (rooms as cached paths, about 30 markers and glyphs); the deck's
  plan built once at load from the walk grid (a few thousand cells a deck at 0.5 m); nothing on the network (the
  positions are in the snapshot already); nothing in 3D.
- Not built until the owner approves the mockup (CLAUDE.md 10).
