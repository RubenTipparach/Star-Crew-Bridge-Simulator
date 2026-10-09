# Design: wall damage and hull repairs from inside

## Context

`damage-control` 1 resolves a hit: it finds the hull section (54 on the Tern), takes `min(E, 4 MJ x integrity)` into
the armour (which loses up to 10 points), marches the rest inward in 0.25 m steps with a 4 m decay, deposits energy
in the rooms it crosses (a bulkhead takes 1.5 MJ to cross), and makes a breach in the first room entered from outside.
Section 6 repairs a breach with plates from inside (15 s a plate) and armour only by EVA (0.5% a second).
`wall-panels` already cuts every wall into bays at its ribs, about 2 m wide, for its panel modules. `repair-minigames`
gives every repair job a short game on the repairer's rate.

## 1. Wall sections

- **What a section is.** One wall bay of `wall-panels` (between two ribs, corners or openings), floor to ceiling, of
  one room. A wall between two rooms is one section seen from both sides, so damage on it shows in both. A section
  over the hull belongs to the hull section behind it (by the layout's hull spans and faces). The Tern has about 900
  sections (the placement tool counts them; this is an estimate).
- **Integrity** 0-100%, starting at 100.
- **Damage from a hit's march.** Each 0.25 m step that deposits energy in a room spreads it as today; every section
  of that room within the step's blast radius (`damage-control` 1, 3.2 m for 34 MJ) takes
  `points = 100 x dE_share / toughness`, where `dE_share` is the step's energy falling linearly to 0 at the radius,
  shared over the sections in reach, and `toughness` is 3 MJ for a bulkhead section and 6 MJ for a hull-side one
  (`damage.json` `walls`). The march still loses 1.5 MJ crossing a bulkhead, as today, and the section it crosses takes
  that 1.5 MJ as damage.
- **Damage from the hull.** When a hull section's armour falls, the hull-side sections behind it lose the same points:
  the inside shows where the outside was hit.
- **Fire and heat** do not damage walls (the panels are steel); a fire chars the floor it burned (`fire-spread` 4), which is the room's fire damage, refitted like plating (3).

## 2. What a damaged section does

| Integrity | What it looks like | What it does |
| ---: | --- | --- |
| 75-100 | As built | Nothing |
| 50-74 | Scorched panels, a cracked lamp cover | Nothing |
| 25-49 | Buckled panels, a hanging trim, sparks from a cut cable tray | Nothing; the section's panel lights are out |
| 1-24 | Torn panels, the frame showing | Nothing yet: one more hit makes a hole |
| 0 | Open to the frame | A bulkhead section: a 0.05 m^2 hole between the two rooms (a `breach` portal between them in `life-support`'s graph: air, heat, smoke and fire pass). A hull-side section whose hull section's armour is also 0: a 0.05 m^2 breach to space, patched with one plate as today |

Damage never lowers a system's capability. The looks come from damaged variants of the wall panel modules (`wall-panels`:
three per finish, chosen by the band), swapped per bay in the room's mesh by its texture layer, so a hit draws nothing
new.

## 3. Repairs from inside

| Job | Needs | Time (an officer; a rating at a third) | Result |
| --- | --- | --- | --- |
| Plating a section at 1-99% | A repair kit and a hull plate (cargo holds 30; 12 in the damage control locker) | `damage::repair_time` at 1.8% a second for an officer, as every repair | The section to 100% |
| Plating a section at 0% | Two plates | The same, from 0 | A hole sealed and the section to 100% |
| Refitting a charred floor (`fire-spread`: cells a fire burned) | A repair kit | At the plating rate, 0.5 m^2 of charred floor counting as 1% | The char cleared; until then the room shows where it burned and the damage board counts it |
| A hull section's armour | Plating every hull-side section behind it to 100% | | The hull section's armour rises to **50%** if it was below; above 50% only EVA (`damage-control` 6) |

**The plating game** (`repairs.html`, `hull.js`), four steps in the kit's step rule (a step's share fills at the rate,
a fumble costs 5%):

1. **Cut out**: trace the cutter round the buckled panel's outline (a closed line, either way); what the torch passes
   over is cut, drifting off pauses. Too fast leaves a thin line to go over again.
2. **Fit**: drag the new plate from the stack onto the opening and turn it until its bolt holes meet the frame's.
3. **Weld**: run the bead along each seam. A heat gauge on the torch: too slow burns through (the fumble: "Burned
   through: a hiss of air", 2 HP of burns), too fast leaves a cold bead (grey gaps) to go over again.
4. **Bolt**: the frame bolts in star order (`KIT.starOrder`, the next one lit).

## 4. Priority

- **The damage board** (`bridge-stations` D1, D3) lists wall damage in its own dim row under each compartment
  ("walls: 3 damaged, 1 open") and in the repair queue after fires, breaches, power and systems, with the plates it
  needs. Its preview time is `damage::repair_time`, the same function.
- **Automation** sends a team to plating only when the queue holds nothing else, and never in combat. A hole (0%) is a
  breach and goes in the breach row, at the breach's priority.
- **Players** can do it any time; between fights it is the work that keeps a crew busy.

## 5. The Pi 5 budget

| Item | Cost | Against |
| --- | --- | --- |
| State | About 900 sections x 1 byte, plus their hull section and room ids in the deck file | Server memory, negligible |
| Network | A section's integrity on change, 3 bytes, reliable | Negligible |
| Drawing | No new geometry: a damaged bay is the same quads with another texture layer; 6 more layers (3 damaged variants x 2 finishes) in the one array, about 2.1 MB at 256 px with mips | Texture memory |
| Updating | A changed bay rewrites its quads' layer attribute in the room's vertex buffer, a few hundred bytes | Upload |

## Risks / Trade-offs

- **900 sections is a lot to show.** The board groups them by room and the walls themselves show the damage; nobody
  reads a list of 900.
- **Plating could be busywork.** It is low priority by design, gives the hull's armour back only to 50%, and is the
  thing to do between fights, not during them.
