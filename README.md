# Star Crew Bridge Simulator

A starship bridge simulator for a crew of friends online: four players at the core, up to
eight, one can play alone. Walk a full 3D ship, sit at a station, route power, keep the air
breathable, repair damage, man or automate the turrets, load the missile tubes, and drop out
of the hangar in a fighter. Low poly, on a custom engine whose floor is a **Raspberry Pi 5
with 1 GB of RAM**, with a 4 GB Pi 5 as the main server.

**Status: design, and the engine's first light.** The game is written up first, as OpenSpec
changes with three.js mockups, and built on request (CLAUDE.md section 4). The engine (Rust, SDL3,
sokol_gfx on OpenGL ES 3.0) has its foundations, the Pi 5 probe, and a client that flies through the
whole Tern, lit by the bake in the three lighting states (`engine-stack` design section 14,
`deck-pipeline` section 13). There is no game to play yet: no walking, crew or systems.

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

## Building the engine

```sh
cargo build --release                     # SDL 3.4 is built from source the first time (cmake, a C compiler)
node tools/deck/export_deck.mjs           # the ship as the deck plan builds it (Node and Playwright)
./target/release/sc-tools deckc           # compiled/tern.deck
./target/release/sc-client --window       # fly the Tern: click, W A S D, Space and C, Shift; 1 2 3 the lighting states; F12 a capture
./target/release/sc-probe --out docs/benchmarks/$(date +%F)-pi5-probe   # on a Pi 5, full screen
./scripts/check.sh                        # every check, in order (also runs the rows below)
```

On Linux the build needs the OpenGL ES and EGL development files (on Debian and Ubuntu:
`libegl-dev libgles-dev libdrm-dev libgbm-dev libudev-dev libasound2-dev`, plus the X11 or
Wayland ones for a desktop window). Without a display, `--headless` draws through SDL's offscreen
driver (Mesa's llvmpipe in a cloud session): `sc-client --headless --shots DIR` writes the three
states, and the render tests run that way. Shaders are compiled with `python3 tools/sokol_shaders.py`,
which fetches the pinned `sokol-shdc`; the generated modules are committed.

**On a Raspberry Pi 5:** [docs/engine/pi-setup.md](docs/engine/pi-setup.md) (the card, packages,
building on the Pi, running the probe and the client).

## Checks

```sh
openspec validate --all                 # npm install -g @fission-ai/openspec
python3 tools/layout_check.py           # the ship layouts
python3 tools/mockups/inline.py --check # mockups hold the current layout and budget
node tools/mockups/shoot.mjs            # screenshot every mockup (needs Playwright)
LC_ALL=C.UTF-8 grep -rnIP '\x{2014}|\x{2013}' --exclude-dir=.git --exclude-dir=.claude --exclude-dir=target --exclude-dir=third_party . && echo FAIL
```

## Opening a mockup

Open `docs/mockups/<name>.html` in a browser. It loads three.js from jsDelivr; everything else
is inside the page. After changing `data/ships/*/layout.json` or `docs/mockups/lib/shipkit.js`,
run `python3 tools/mockups/inline.py` to update every page.
