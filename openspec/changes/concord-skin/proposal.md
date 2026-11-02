# Proposal: the Concord skin

## Why

The owner, 2026-11-02: "Can you redo the UI? Pull in UI guide from fleet-command and adopt the concord skin here".

Concord is the player faction's console skin in the owner's Fleet Command (`openspec/changes/console-skins` there, at
`a45a1bb`): a flat, back-lit vector design language built on LCARS's principles, with a palette sampled from
Starfleet Command and Starfleet Academy. The owner described what drew them to it: "the sleek, utilitarian, LCARS blue
version of the federation, very sci-fi very minimal". Star Crew's screens today follow `ui-art-direction`
(approved 2026-10-16): one cyan accent on near-black panels in Barlow. They read as a clean modern game, not as a
starship's own interface.

## What changes

- **The guide comes over.** Fleet Command's `game-ui-frontend` skill is copied with an "In Star Crew" note, and
  Concord's rules (the sweep, the pill, five colours with one meaning each, three type sizes, two spacing constants,
  fades under 150 ms, take the principles and never LCARS's artwork) become this change's design.
- **One UI kit, drawn by name.** Every screen draws named pieces (a panel frame, a pill button, a tab, a bar, a
  strip, a keycap, an objective card, lamps, icons), as Fleet Command's kit does, so the look lives in one place.
- **Every screen is redrawn in Concord**: the front menu, Single Player, Multiplayer (the room finder), Options and
  Controls, the briefing, the HUD on foot (objective card, prompt, hint, compass, minimap), the Esc menu with the
  crew's name, the profile, loading, and the consoles' frame (title band, status strip, panel frames and keys).
- **What does not change**: every layout, position and size approved in `ui-art-direction`, the consoles'
  instruments and their glance-first rules, and what each control does. A skin swaps the look, never the layout.
- **Mockup first** (CLAUDE.md section 10): `docs/mockups/concord-ui.html` draws every screen in Concord beside
  today's. The engine follows the owner's approval of it.

## Not in this change

- Other factions' skins. Star Crew has one crew and one look.
- The consoles' instruments (the helm's lever, the scanner, the shield ring): they take the new palette and
  frames, and keep their drawings.
