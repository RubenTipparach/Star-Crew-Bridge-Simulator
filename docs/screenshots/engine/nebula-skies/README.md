# Nebula skies in the engine: captures

`openspec/changes/nebula-skies`, as built on 2026-10-16: the Shoals' sky A, Bloom (NS1), its 1,600 vector stars
streaking on any turn of the view (NS2, NS3) and its colour on the hulls (NS4). Rendered headless on Pi 1 (Pi 5 8 GB,
Mesa V3D) by the release build, 1920 x 1080, and kept in full colour: these are never run through
`tools/mockups/shrink_png.py`, whose 256-colour palette bands the nebula's gradients (design 3).

The patrol's shots: `sc-client --headless --solo --mission patrol-shoals --playtest --shots DIR` (its scripted shots,
patrol-mission 12b). The viewscreen's and the bridge's: frames of `sc-client --headless --solo --station captain
--film-orbit --shots DIR`, the film of `docs/videos/2026-10-16-nebula-skies-pi5.mp4`.

| Capture | What it shows | Against the mockup |
| --- | --- | --- |
| `1-patrol-tamsin-station.png` | Tamsin Station 14 km off, in front of Bloom's rose and violet cloud, its dark lane and the stars round it | `nebula-skies-A-station.png` |
| `2-patrol-tamsin-from-76-km.png` | The same line from 76 km: the station whole in front of the sky (the sky layer's depth), the cloud behind it | |
| `3-bridge-windows.png` | On foot on the bridge in the drill: the nebula through the port and starboard windows, and the main viewscreen showing the captain's orbit | `nebula-skies-A-bridge.png` |
| `4-viewscreen-chase.png` | The captain's console, the viewscreen with CHASE on: the Tern from behind, the cloud and the sun beyond it, the planet's limb | `nebula-skies-A-tern.png` |
| `5-viewscreen-orbit-streaks.png` | ORBIT turning the camera round the Tern: every star a short line toward where it was (FreeSpace's streak) | `nebula-skies-A-turn.png` |
| `6-viewscreen-orbit-streaks-detail.png` | The viewscreen of 5 at its own pixels: the streaks pale over the cloud, white over black | `nebula-skies-A-turn.png` |
| `7-patrol-throat.png` | Leaving Tamsin Station's throat: the bay's light strips, and the sky's blue edge and stars through the opening | |

The render tests' own frames (`SC_RENDER_SHOTS=DIR cargo test -p sc-client --test render`): the nebula and its
streaking stars drawn before and after a ship through a feed and through a bridge window, not one of the ship's pixels
changed, and the streak cases measured in pixels.
