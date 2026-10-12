# The Tern's hull in the engine, from its windows and from outside

`openspec/changes/tern-exterior`, design 8. Headless captures from `sc-client --headless --shots DIR --poses ...`
on the compiled deck (the hull compiled from `assets/models/exteriors/liveries/cobalt/tern.glb`, the Cyan livery,
and the 16 lower-deck windows applied to the rooms). Rendered on lavapipe in a Claude Code cloud session: they show
what the game draws, not how fast. The Pi 5 measures the frame time (task 4.3).

| File | What it shows | Requirement |
| --- | --- | --- |
| `quarters-window.png` | Crew quarters (deck B, port), two of the three windows: the pylon close outside the aft one, the planet past the dock beside it. The lockers that stood here moved to the bunk island's ends. | Every window meets the hull; the hull drawn from its model |
| `quarters-window-aft.png` | The quarters' aft window, looking aft along the hull: the port pylon and the front of its nacelle. | The hull drawn from its model |
| `quarters-window-aft-red-alert.png` | The same in red alert: the room's light changes, the hull outside does not. | The hull drawn from its model |
| `mess-window.png` | Mess (deck B, starboard), its three windows above the tables. | Every window meets the hull |
| `medbay-window.png` | Medbay (deck B, port forward), its two windows. | Every window meets the hull |
| `life-support-window.png` | Life support (deck C, port), looking up and aft through its window to the hull above. | Every window meets the hull |
| `ready-room-window.png` | The ready room (deck A), aft onto the port nacelle's front. | The hull drawn from its model |
| `bridge-port-window.png` | The bridge's port window, down onto the bow's rim. | The hull drawn from its model |
| `outside-port-bow.png`, `outside-starboard-bow.png`, `outside-port-aft.png`, `outside-above.png` | The whole ship in the dock's girders from four places outside. | The hull drawn from its model; the hull compiled with the decks |

The model's own preview of the same rooms is in `docs/screenshots/exteriors-window-row/` (`inside-quarters.png`,
`inside-mess.png`, `inside-medbay.png`).
