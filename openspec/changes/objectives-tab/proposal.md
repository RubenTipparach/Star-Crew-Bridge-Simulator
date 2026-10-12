# Proposal: an OBJECTIVES tab on every station

## Why

The owner, 2026-10-10: "always have objectives as a tab in the station dashboard."

The mission's objectives are shown only on the briefing panel before the fight (`coop-drill`); no console has them,
and the captain's planned Mission panel (`bridge-stations` 10.5, C5) was never built.

## What Changes

- **A tab on every console** (design 1): OBJECTIVES, in each station's title band beside its own tabs, on every
  station (Helm, Tactical, Engineering, Science, Captain, the laser stations, the gunner's sight).
- **What it shows** (design 2): the mission's title, each objective with its live state (done, failing, in progress)
  and the number it turns on (the Hound's hull, the Tern's hull, fighters down), and the time in the fight.

## Impact

- `sc-client` consoles (the title band, a full-console panel); the mockup first (`docs/mockups/consoles.html`).
- The objectives' live numbers come from the snapshot already sent; `data/missions/*.json` objectives gain a kind so
  each can be measured.
