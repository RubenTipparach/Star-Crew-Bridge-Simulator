# Design: a name over everyone aboard

## Context

The owner, 2026-10-10: "I want to always see peoples name in the game, I saw a captain but I didnt know who that
was?", and "show if they are (npc)". The mockup is `docs/mockups/game-menu.html` (states 1 and 2).

| Decided elsewhere | Where |
| --- | --- |
| Bots carry generated names, a department and a tunic | `crew-npcs` 7 |
| A player's name is the lobby's officer | `lobby` 1 |
| The drill names bodies and says where they walk ("to Helm") | `coop-drill` 9 |
| Menus and labels name things; colour means state, always with a shape | CLAUDE.md 10 |

## 1. The nameplate

| Part | What |
| --- | --- |
| Name | The crew member's name, 20 px (the drill's 17 px is small on a wide screen), the UI's text colour, centred 2.05 m above the feet |
| (NPC) | After the name, in the dim colour, when the body is a bot or automation: `Ens. Dara Holt (NPC)` |
| Walking to | The drill's second line, `to Helm`, in amber at 16 px, while a body walks to a seat |
| Backing | A dark band behind the text at 60 % opacity, so a name reads against a bright lamp or console |

A player's own body never gets one (the drill already hides it). A name longer than 28 characters is clipped with an
ellipsis, so a plate is never wider than 300 px (CLAUDE.md 10, fixed bands).

"NPC" is the owner's word, used as written. In the drill, a station on automation has no body today, so (NPC) there
marks the `sc-bot` crew (`coop-drill` 9). The walk's nine bot crew are all (NPC).

## 2. When a name shows

A name shows when all of these hold, tested each frame:

1. **On screen.** The head point projects inside the view (the drill's `project` today).
2. **Within 25 m** of the eye. From 20 m to 25 m it fades out linearly, so names don't pop.
3. **In line of sight on the walk grid.** The eye and the head are on the same deck, and a straight walk across the
   deck's grid cells (`sc-core::nav`, Bresenham over its 0.5 m cells, at most 50 steps at 25 m) crosses only walkable cells (a doorway is one).
   That is the test for "not through a bulkhead" without reading the depth buffer, which a Pi 5 should not stall on.
   A body on another deck (seen down a ladder well) is not named: recommendation taken (ask only with screenshots).

On the drill's bridge every body is in the one room, so step 3 is skipped there.

## 3. Turning them off

Options has a Names row, On or Off (`game-menu` 3), on by default and kept in `settings/options.json`.

## 4. Pi 5 budget

| Cost | Amount |
| --- | --- |
| Draw calls | 1: every plate is text and a band in one egui layer with one clip rectangle and the font atlas |
| Triangles | 2 per glyph and 2 per band: about 1,000 for 16 plates of 28 characters |
| CPU | At most 9 grid walks of 50 cells and 16 text layouts: well under 0.1 ms, to measure on the Pi with the probe |
| Memory | None held: the plates are built each frame from the snapshot and the crew list |
| Network | None: names already travel in the crew list (`coop-drill`) and bot names from the session seed (`crew-npcs`) |

## 5. Checks

- A unit test on the grid walk: a wall between two cells blocks the line, a doorway does not.
- A headless capture of the walk (`--shots`) with a bot in view shows its plate, saved in `docs/screenshots/engine/`.
- A drill capture with an `sc-bot` aboard shows `(NPC)` after its name.

## Open questions

| Id | Question | Recommendation | Status |
| --- | --- | --- | --- |
| Z1 | Approve the mockup (`docs/mockups/game-menu.html`, states 1 and 2) | Approve | In the survey, with the shots |
| Z2 | A body seen on another deck, down a ladder well: named or not | Not named (design 2) | Recommendation taken (ask only with screenshots) |
| Z3 | Name size: 20 px, up from the drill's 17 px | 20 px | Recommendation taken (ask only with screenshots) |
