# Proposal: wall damage and hull repairs from inside

## Why

The owner, 2026-10-09: "What about hull repairs? I would think each section of wall could be damaged? And so
general hull repairs could be possible. But probably not as prioritized as subsystems."

`damage-control` 1 already wears the hull: 54 hull sections (9 spans by 6 faces) each hold armour, a hit strips up to
10 points, and a hole through it is a breach that plates seal from inside. But armour comes back only by EVA (0.5% a
second), and nothing inside the ship shows or keeps the damage: a blast that wrecks a room leaves its walls as they
were. There is no hull work for a crew between fights, and no way to see where the ship has been hit.

## What Changes

- **Wall sections** (design 1): every wall of every room, hull side and bulkhead alike, is cut into sections at its
  ribs (the wall panels' bays, about 2 m wide), each with an integrity of 0-100%. A blast in a room damages the
  sections near it; a hull hit damages the hull-side sections behind its hull section.
- **What damage does** (design 2): a damaged section looks it (buckled, scorched, torn panels); a bulkhead section at
  0% is a small hole between rooms; a hull-side section at 0% over a hull section whose armour is gone is a breach.
  Nothing loses capability: it is structure, not a system.
- **Repairs from inside** (design 3): a plating job at the wall, with a repair kit and a hull plate, played as a
  mini-game (cut out, fit, weld, bolt). Plating the hull-side sections of a hull section also brings its armour back
  to 50% from inside; above that is still EVA's.
- **Low priority** (design 4): the damage board lists wall damage after fires, breaches, power and systems; automation
  sends teams to it only when nothing else waits.
- **A mockup**: the plating game in `docs/mockups/repairs.html`.

## Capabilities

### New Capabilities

- `hull-repair`: wall sections, their damage, and plating repairs from inside, including the hull's armour to 50%.

### Modified Capabilities

- None in `openspec/specs/`. `damage-control` 1 and 6 gain a wall section step in the hit resolution and a plating
  job; its armour rule and EVA repair are unchanged.

## Impact

- `damage::resolve` (one more step), `damage.json` (`walls`), the damage board's lists (`bridge-stations` D1, D3),
  the wall panel modules (`wall-panels`: damaged variants), `docs/mockups/repairs/hull.js`.
