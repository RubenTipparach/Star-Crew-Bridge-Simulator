# Design references

What Star Crew borrows from, and what each source is cited for. CLAUDE.md section 15: cite,
don't recall; take the shape, not the text. A claim marked "to verify" is from memory and must
be checked against the source before a design depends on it.

## Bridge simulators

| Source | Cited for |
| --- | --- |
| *Artemis Spaceship Bridge Simulator* (Incandescent Workshop, 2011) | The genre's template: one server, each station on its own screen (helm, weapons, engineering, science, comms, the captain's view). Engineering's per-system power allocation with heat and coolant, and damage control teams sent through the ship (to verify the details). |
| *EmptyEpsilon* (open source, C++) | An open-source Artemis-style simulator; stations combined into fewer consoles for smaller crews, which is our merge rule's shape (to verify its presets). Useful to read for networking a bridge simulator. |
| *Starship Horizons* | A bridge simulator with flight operations and small craft (to verify). |
| *Pulsar: Lost Colony* | A walkable ship interior with crew classes and bridge stations while the ship flies: the closest existing game to "a full 3D bridge" with crew on foot. |

## Ship systems games

| Source | Cited for |
| --- | --- |
| *FTL: Faster Than Light* (Subset Games, 2012) | Per-room oxygen, power as bars per system, doors opened to space to put out fires, crew repairing systems in place. Our compartments are its rooms with real gas and real watts. |
| *Barotrauma* | A wired power grid with junction boxes, and per-hull oxygen and water flowing through gaps: the closest model for `power-grid` and `life-support` (to verify its flow model). |
| *Space Station 13* | How far atmospheric simulation can go: gas mixtures per tile and decompression. We choose per-compartment for the Pi's sake. |

## Engines and level technology

| Source | Cited for |
| --- | --- |
| *Quake* (id Software, 1996) | Brush-built levels compiled offline into a BSP tree with a potentially visible set, and collision against brush planes. |
| *Quake III Arena* and ioquake3 | Lightmapped brush levels; ioquake3 running on the Raspberry Pi's VideoCore IV is the published reference point for our triangle budget (to verify the figures). |
| *Descent* (Parallax, 1995) | Levels of connected cube segments rendered through portals, in a six-degrees-of-freedom game. |
| The Build engine (*Duke Nukem 3D*) | Sectors joined by portals. |
| *Thief: The Dark Project* | Portal-based rendering, and sound propagating through rooms and doors: the shape for sound in our compartment graph. |
| *Doom 3* | Areas and portals, with closed doors cutting visibility at runtime: the shape of our portal culling. |
| Raspberry Pi documentation | The Pi 3's CPU, memory, GPU and display stack (to verify clocks and CMA defaults when writing the probe). |
| Mesa `vc4` driver documentation | What OpenGL ES 2.0 and OpenGL 2.1 features the Pi 3 driver exposes. |

## Flight and networking

| Source | Cited for |
| --- | --- |
| *Elite Dangerous* | Newtonian flight with a flight-assist switch. |
| Glenn Fiedler, *Gaffer on Games* | UDP reliability over acknowledgement bitfields, snapshot interpolation, snapshot compression. |
| Valve, "Source Multiplayer Networking" | Interpolation delay, client-side prediction and lag compensation. |

## Bridge layout

| Source | Cited for |
| --- | --- |
| *Star Trek: The Next Generation Technical Manual* (Sternbach and Okuda) | Conventions for a bridge's arrangement: flight control and operations forward of the captain, tactical behind, side stations along the walls (to verify). Shape only. |
