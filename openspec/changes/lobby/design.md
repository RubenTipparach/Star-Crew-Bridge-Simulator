# Design: the lobby

## Context

The owner, 2026-10-08: "we'll need a lobby menu before starting a game, ability to name your player, maybe
autogenerate from some name list, make it star trek themed", with "we're no longer messing with the JS stuff, we're
all in real life GLES web assembly now!": the lobby is built in the engine, judged on its screenshots.

| Decided elsewhere | Where |
| --- | --- |
| Menus name things, they don't explain them; every input device drives every menu; panels a fixed size | CLAUDE.md 10 |
| Take the shape, not the text: names belong to their owners | CLAUDE.md 15 |
| A solo game is a local server with one client | CLAUDE.md 6.3 |
| The mess is the lobby inside the ship; join by a six-character code | `netcode-and-sessions` 8, `matchmaker` |
| The console UI is the engine's immediate-mode UI | CLAUDE.md 10, `engine-stack` |

## 1. The screen

One screen over a slow view of the ship in its dock (the engine's exterior, `deck-pipeline` 13b):

| Part | What |
| --- | --- |
| Title | STAR CREW, and under it the ship: SCS TERN |
| Your officer | Rank (a list), name (a text field, 24 characters at most), a dice button that rolls a new name |
| Station | Captain, Helm, Tactical, Engineering, Science, Comms, Flight ops, or Any: the seat you would like |
| Beam aboard | The main action, the biggest control: starts a solo game |
| Join a crew | A six-character code and Join: off until the network exists |

Keys: Enter beams aboard, Tab moves between fields, a pad's A presses what has focus. The officer's name and station
travel with the player into the game (the crew page, the plan view, the seat claim).

## 2. Names

`data/crew/names.json` holds:

| Table | Use |
| --- | --- |
| `ranks` | Ensign, Lieutenant junior grade, Lieutenant, Lieutenant commander, Commander, Captain, with short forms (Ens., Lt. jg, Lt., Lt. Cmdr., Cmdr., Capt.) |
| `styles` | Each a pattern of parts and the syllables each part draws from: a human given and family name, a two-part name with an apostrophe (Ke'ral, T'sana), a family name first and a given name (Dal Varen), a single name with a clan suffix |
| `weights` | How often each style is drawn |

The generator (`sc-core::names`) draws a style by weight, then each part's syllables, from a seed (CLAUDE.md 6.4: the
session seed, the player's slot and the purpose), so a crew's names are stable for a replay and NPCs (`crew-npcs`) use
the same generator. The roll button uses a fresh seed each press. A generated name that equals any name in the
table's `deny` list (canon characters from every series, checked case-blind) is rolled again, so no generated name is
someone else's character.

The default rank follows the station: Captain for the captain's chair, Lieutenant for the others; the player changes it.

## 3. The UI layer

egui, drawn by `sc-render`'s UI painter: its triangles (positions in points, a texture and colour each) streamed into
one vertex and one index buffer a frame, one draw per clip rectangle, the font atlas a texture updated when egui
changes it. Input: SDL3's mouse, keys and text input mapped to egui's events. On the Pi: egui's tessellation runs on
the CPU (about 0.2-0.5 ms for a screen like this on a desktop, to measure on the Pi), and a menu is a few thousand
triangles in a handful of draws.

## 4. Kept between sessions

The officer (rank, name, station) is saved to `settings/officer.json` beside the game on a Pi or a desktop. In the
browser the file system is the page's memory: it is kept for the page's life, and the browser's storage is a later
step.

## Risks / Trade-offs

- **A font for every screen.** egui's default font (Ubuntu Light and Hack) is used for now; the game's own typeface
  (`assets/fonts/`, the consoles') replaces it when the console UI moves to the engine.
