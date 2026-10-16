# Stars never draw over ships: captures

`openspec/changes/sky-behind-solids`. Headless, through the render test's real GL context (Mesa on the Pi 5, 256 x
256 px): `SC_RENDER_SHOTS=DIR cargo test -p sc-client --test render` keeps each sky case's frame. The test's sky is
deliberately extreme, a bright star in every cell, so any star that lands on a solid shows.

| Capture | What it shows |
| --- | --- |
| `before-feed-sky-last.png` | The old sky pipeline (depth "always") drawn after a ship through a feed's lens: the ship is gone under the stars (14,071 of its 14,081 pixels changed). |
| `before-bridge-window-sky-last.png` | The same through a bridge window: the wall and the ship beyond it both starred over (34,415 of 34,436 pixels). |
| `after-feed-sky-last.png` | The sky layer's depth state, the same draw order: the ship whole, not one of its pixels changed. |
| `after-bridge-window-sky-last.png` | The bridge window: the wall (left) and the ship beyond it whole, the stars only round them. |
| `after-feed-farthest-reach.png` | A solid 120 km off, the farthest any solid may stand, with the sky drawn after it: still whole in front of the sky. |

Not yet captured: the patrol's new shot `p6-pair-far.png` (Tamsin Station from 76 km, where the old 80 km far plane
cut it open). It needs the scripted patrol to reach open space; on the working tree of 2026-10-15 the scripted body
did not reach its seat and the patrol stayed in its muster, for reasons outside this change. The far plane's numbers
are pinned by `sc-render`'s `depth` tests and the stations' load check instead.
