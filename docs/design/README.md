# Star Crew design hub

Every design in this repository, where to read it and where to see it. This page presents the
OpenSpec changes; it is not a second source (CLAUDE.md section 3). Start with
[vision.md](vision.md), the game in one page.

## Survey

Questions for the owner, with their options, a recommendation and an answer column:
**https://claude.ai/artifact/D1C3ZBxsCjRHJC6HAvJFzJ** (a Claude Doc; the `owner-survey` skill says
how it is kept). Keep editing it rather than starting another.

## The changes and their mockups

| Change | Designs | Mockup | Published |
| --- | --- | --- | --- |
| [engine-stack](../../openspec/changes/engine-stack/) | The custom engine (Rust, SDL3, glow), the OpenGL ES 3.0 floor, crates, the Pi 5 budget table, the probe | | |
| [netcode-and-sessions](../../openspec/changes/netcode-and-sessions/) | Authoritative server, snapshots, prediction, seats, sessions | | |
| [reference-ship-tern](../../openspec/changes/reference-ship-tern/) | The Tern's floor plan, deck by deck | [deck-plan.html](../mockups/deck-plan.html) | [3YJXPa...](https://claude.ai/artifact/3YJXPa7vZuNtAB3YyWQSPV) |
| [deck-pipeline](../../openspec/changes/deck-pipeline/) | Brush-built decks, compartments and portals, baked vertex light, portal culling | [deck-plan.html](../mockups/deck-plan.html) | [3YJXPa...](https://claude.ai/artifact/3YJXPa7vZuNtAB3YyWQSPV) |
| [surface-materials](../../openspec/changes/surface-materials/) | Material Maker graphs to 128 px layers of one texture array: texel density, relief baked into colour, emission alpha | [contact sheet](../screenshots/materials/contact-sheet.png); every mockup is textured | |
| [light-baking](../../openspec/changes/light-baking/) | The static light baker: shadows, bounce, vertex lighting or lightmaps, probes, lighting states | [lighting.html](../mockups/lighting.html) | [Txz9NC...](https://claude.ai/artifact/Txz9NCjZ3fpvEYWdmf6B2R) |
| [bridge-stations](../../openspec/changes/bridge-stations/) | The station roster, automation, console UI, the bridge as a room | [bridge.html](../mockups/bridge.html) | [LfCtvp...](https://claude.ai/artifact/LfCtvpnbofmcTQ667zcczu) |
| [crew-on-deck](../../openspec/changes/crew-on-deck/) | Walking, ladders, seats, carrying, injury, zero-g | [bridge.html](../mockups/bridge.html) | [LfCtvp...](https://claude.ai/artifact/LfCtvpnbofmcTQ667zcczu) |
| [power-grid](../../openspec/changes/power-grid/) | Reactor, buses, breakers, batteries, allocation, heat and coolant | [systems.html](../mockups/systems.html) | [532Ciw...](https://claude.ai/artifact/532CiwiBRnCnsN5C6KvtCS) |
| [life-support](../../openspec/changes/life-support/) | Gases, pressure, flow, breaches, bay pump-down | [systems.html](../mockups/systems.html) | [532Ciw...](https://claude.ai/artifact/532CiwiBRnCnsN5C6KvtCS) |
| [damage-control](../../openspec/changes/damage-control/) | Hits, fires, repairs, damage control teams | [systems.html](../mockups/systems.html) | [532Ciw...](https://claude.ai/artifact/532CiwiBRnCnsN5C6KvtCS) |
| [ship-frames](../../openspec/changes/ship-frames/) | The interior decoupled from the exterior; craft hand-off; pass composition | [exterior.html](../mockups/exterior.html) | [7gQwxT...](https://claude.ai/artifact/7gQwxTJF8b1ZFiFU17VxTM) |
| [weapons-and-shields](../../openspec/changes/weapons-and-shields/) | Turrets manned and automated, missiles, six shield faces | [exterior.html](../mockups/exterior.html) | [7gQwxT...](https://claude.ai/artifact/7gQwxTJF8b1ZFiFU17VxTM) |
| [shuttle-bay-and-fighters](../../openspec/changes/shuttle-bay-and-fighters/) | Launch bays, launch and recovery, the Swift and the Petrel | [exterior.html](../mockups/exterior.html) | [7gQwxT...](https://claude.ai/artifact/7gQwxTJF8b1ZFiFU17VxTM) |
| [flight-and-navigation](../../openspec/changes/flight-and-navigation/) | The flight model, helm controls, in-system travel | [exterior.html](../mockups/exterior.html) | [7gQwxT...](https://claude.ai/artifact/7gQwxTJF8b1ZFiFU17VxTM) |

## Data and tools

| | |
| --- | --- |
| [data/ships/tern/layout.json](../../data/ships/tern/layout.json) | The one layout source for the Tern. |
| [tools/layout_check.py](../../tools/layout_check.py) | Validates every ship layout (schema v2: hull-following brushes) and prints compartment volumes. |
| [data/ships/tern/detailing.json](../../data/ships/tern/detailing.json) | The generated detail's rules and the finish table (`deck-pipeline` section 5a). |
| [data/materials/materials.json](../../data/materials/materials.json) | The surface materials (`surface-materials`); tools in [tools/materials/](../../tools/materials/). |
| [tools/mockups/kit_report.mjs](../../tools/mockups/kit_report.mjs) | Counts the kit's shell and detail triangles per compartment. |
| [docs/mockups/lib/shipkit.js](../mockups/lib/shipkit.js) | The shared mockup kit. |
| [tools/mockups/](../../tools/mockups/) | `inline.py` (copy the layout into the pages) and `shoot.mjs` (screenshots). |
| [docs/screenshots/mockups/](../screenshots/mockups/) | The latest screenshots of every mockup. |
