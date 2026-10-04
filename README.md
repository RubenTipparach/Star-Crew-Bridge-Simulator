# Star Crew Bridge Simulator

A starship bridge simulator for a crew of friends online: four players at the core, up to
eight, one can play alone. Walk a full 3D ship, sit at a station, route power, keep the air
breathable, repair damage, man or automate the turrets, load the missile tubes, and drop out
of the hangar in a fighter. Low poly, on a custom engine whose floor is a **Raspberry Pi 5
with 1 GB of RAM**, with a 4 GB Pi 5 as the main server.

**Status: design.** There is no engine code yet. The game is being written up first, as
OpenSpec changes with three.js mockups, and built on request (CLAUDE.md section 4).

## Start here

| | |
| --- | --- |
| [CLAUDE.md](CLAUDE.md) | The rules for anyone (human or agent) working here. The only copy. |
| [docs/design/vision.md](docs/design/vision.md) | The game in one page: crew, stations, the reference ship, the list of changes. |
| [docs/design/README.md](docs/design/README.md) | The design hub: every change, every mockup, the survey. |
| [openspec/changes/](openspec/changes/) | What is designed but not built, one change per capability. |
| [openspec/specs/](openspec/specs/) | What the game does today (empty until the first change lands). |
| [data/ships/tern/layout.json](data/ships/tern/layout.json) | The reference ship's floor plan: the one layout source. |
| [docs/mockups/](docs/mockups/) | three.js mockups, built from the layout. |
| [docs/analysis/](docs/analysis/) | What carries over from star-crew-64, Pale-Blue-Dot and Undercity. |
| [docs/references.md](docs/references.md) | The games and engines this design borrows from. |

## Checks

```sh
openspec validate --all                 # npm install -g @fission-ai/openspec
python3 tools/layout_check.py           # the ship layouts
python3 tools/mockups/inline.py --check # mockups hold the current layout and budget
node tools/mockups/shoot.mjs            # screenshot every mockup (needs Playwright)
LC_ALL=C.UTF-8 grep -rnIP '\x{2014}|\x{2013}' --exclude-dir=.git --exclude-dir=.claude . && echo FAIL
```

## Opening a mockup

Open `docs/mockups/<name>.html` in a browser. It loads three.js from jsDelivr; everything else
is inside the page. After changing `data/ships/*/layout.json` or `docs/mockups/lib/shipkit.js`,
run `python3 tools/mockups/inline.py` to update every page.
