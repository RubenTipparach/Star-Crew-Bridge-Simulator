# Proposal: the lobby, and naming your officer

## Why

The owner, 2026-10-08, playing the browser build: "we'll need a lobby menu before starting a game, ability to name
your player, maybe autogenerate from some name list, make it star trek themed".

Today the client drops you on the bridge with no name and no way to choose anything. Every later step needs a front
door: a name the crew page, the plan view (`ship-plan-view`) and other players see; a choice of game; and, with the
network (`netcode-and-sessions`), the place a join code is typed.

## What Changes

- **A title and lobby screen in the engine** (design section 1), before the ship loads: the game's name, your officer
  (rank, name, a station you would like), a button that rolls a new name, and "Beam aboard", which starts a solo game
  (a local server and one client, CLAUDE.md 6.3). "Join a crew" with a six-character code is shown and stays off until
  the network is built (`matchmaker`).
- **Names in the spirit of Star Trek, not taken from it** (section 2, CLAUDE.md 15): ranks of a starfleet, and names
  built from syllable tables in several styles (human given and family names, clipped two-part names with an
  apostrophe, family name first), seeded, in `data/crew/names.json`. No character's name from any series.
- **The engine's UI layer** (section 3): egui drawn by a sokol painter in `sc-render`, the immediate-mode UI
  `engine-stack` chose for consoles; the lobby is its first screen, and the same layer draws the walk's prompts.
- **Your name and station preference are kept** between sessions where the platform allows (a file on a Pi or a
  desktop; the browser's storage later).

## Impact

- `engine-stack` (the UI layer), `netcode-and-sessions` 8 (the mess stays the in-ship lobby; this is the screen
  before it), `matchmaker` (the code field), `bridge-stations` (the preferred station), `crew-npcs` (NPC names
  come from the same generator).
