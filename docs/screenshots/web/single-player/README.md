# Single player in the browser

`openspec/changes/browser-single-player`. The browser build (`tools/web/build.sh`) in headless Chromium with WebGL 2
on SwiftShader, 1280 x 720, driven by key presses as a player would: Enter (Single Player), Enter (Beam aboard),
then W A S D and E. Taken in a Claude Code cloud session with no GPU, so they show what the page draws, not how fast;
a browser's speed is never the game's measure (CLAUDE.md 1).

Beam aboard hosts the drill in the page on the local link (`sc-net::local`), with an NPC at each of the seven other
stations, and the player walks the ship from the deck B lift to the bridge.

| File | What it shows | Requirement |
| --- | --- | --- |
| `1-corridor-crew-walking.png` | MUSTER: the player on foot in the deck B corridor; an NPC ahead walking to Tactical | The browser hosts single player; played on foot in the browser |
| `2-bridge-crew-to-seats.png` | MUSTER: on the bridge, the crew walking to their stations (Captain, Tactical) | The browser hosts single player |
| `3-at-the-captains-chair.png` | COUNTDOWN: the player walked onto the dais to the captain's chair; "E Take Captain from Lt. Sa'ral" | Played on foot in the browser: a console opens only once they sit |
| `4-sat-captain-console.png` | E pressed: seated, and the Captain console comes up, red alert as the round engages | Played on foot in the browser |
| `5-engage-red-alert.png` | ENGAGE: red alert on the bridge with the crew at their posts, the player standing behind the dais | A round can be played with the crew |

The same moments on the desktop are in `docs/screenshots/engine/single-player/` (`0-walking-helm.png`,
`0-bridge-helm.png`, `3-engage-a-helm.png`, the consoles in `s1-s3`). Those were retaken in this change with
`sc-client --headless --solo --playtest --shots`, hosted over the same local link; `6-debrief-helm.png` is still the
earlier one, since the retake was stopped once its playtest walked off the bridge.

`tools/web/smoke.mjs` checks the same thing before every release: the page hosts single player, loads the bridge for
the crew on foot, and the round reaches Engage (54 s from opening the page in this session).
