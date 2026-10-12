# Proposal: crew-uniforms

## Why

The owner, 2026-10-11: "Crew members should have different colored skirts based on roles like start Trek:
operations, science, engineering, and support", then "Gold, blue, red, green". Today the walk's bots wear six
department colours and every bot on the drill's bridge wears the same red, so a role cannot be read at a glance.

## What changes

- Four divisions, each a tunic colour: operations gold, science blue, engineering red, support green, in
  `data/crew/company.json`.
- Every department belongs to a division, and every bridge station does (`data/stations.json`).
- The deck compiler builds one crew figure a division; the walk dresses each bot by its department's division, the
  drill's bridge dresses each body by the station it holds, and the ship map's markers take the same colours.

## Impact

- Data: `data/crew/company.json` (divisions; a department names its division instead of its own colour),
  `data/stations.json` (`divisions`).
- Code: `sc-core::crew`, `sc-core::combat::data`, `sc-tools deckc`, the walk and the drill's bridge in `sc-client`.
- The compiled deck is rebuilt (four division figures and the player's map marker, in place of seven figures).
