# Design: an OBJECTIVES tab on every station

## 1. The tab

Every console's title band has an OBJECTIVES chip after the station's own tabs, before the swap chips. It opens a
panel over the console's four panels (the title and status strip stay), and the station's own chip or Esc goes back.
On the gunner's sight it is the same chip over the sight.

## 2. The panel

| Row | Shows |
| --- | --- |
| Title | The mission's title and the fight's clock |
| Each objective | Its words, a state mark (a tick when done, a cross when failed, a ring filling while in progress) and its number: the Hound's hull (percent), the Tern's hull, fighters down of two |

Glance-first (CLAUDE.md 10): at most six rows, one number each. The objectives gain a kind in the mission file
(`destroy` with a ship, `survive`, `destroy_fighters` with a count) so the console can measure them; an objective
with no kind shows its words only.

## 3. Cost

One panel drawn while open; no new network data.
