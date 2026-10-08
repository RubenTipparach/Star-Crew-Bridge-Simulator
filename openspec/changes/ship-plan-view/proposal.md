# Proposal: the ship plan view, in the engine

## Why

The owner, 2026-10-08: "I kind of like the floor plan of the ship view as well. But we may need to
downscale some of the geometry on furniture for that view so it'll run on a pi. Otherwise having the
whole ship to explore. Or monitor players moving about on the ship would be cool."

The deck plan mockup's plan, exploded and whole-ship views (`reference-ship-tern`, `docs/mockups/deck-plan.html`)
show the whole Tern at once: about 196,000 triangles with every prop at full detail, the Pi 5's entire
frame budget (200,000 triangles, `engine-stack` 5) before a single console is drawn. In first person the
engine draws one or two rooms; seen whole, every chair and console of 38 compartments is on screen.

## What Changes

- **A plan view in the engine** (design section 1): the ship from above, one deck or all three pulled apart,
  orbit and zoom; the same decks the walk draws, cut at a height so the rooms read as a floor plan.
- **A second level of detail for every prop** (section 2): each Blender build writes, beside a prop's
  model, a light one (its silhouette in at most 24-60 triangles, by size), in the same atlas. The plan view,
  and the walk beyond 15 m, draw the light one. The whole ship in plan comes to about 60,000 triangles.
- **People on the plan** (section 3): every body aboard as a marker on its deck, a player's in its role
  colour with its name, an NPC's (`crew-npcs`) smaller, a downed body flashing; doors, fires and breaches
  from the simulation. Live, from the snapshot the client already has.
- **Where it is seen** (section 4): the captain's and the security station's crew page; the lobby and a
  spectator before joining; a player's own map (M) on foot, which shows only their deck unless they hold a
  console.

## Impact

- `ship-props`, every Blender prop build (`hs_kit`: the light model), `deck-pipeline` (deckc writes both levels),
  `bridge-stations` and `security-station` (the crew page), `crew-npcs`, the engine (a plan camera, the
  cut, markers).
