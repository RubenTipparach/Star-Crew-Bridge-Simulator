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
| *Quake III Arena* and ioquake3 | Lightmapped brush levels, compiled with a light tool: the generation of game the Raspberry Pi 5 runs comfortably, and a reference point for our triangle budget (to verify the figures). |
| *Descent* (Parallax, 1995) | Levels of connected cube segments rendered through portals, in a six-degrees-of-freedom game. |
| The Build engine (*Duke Nukem 3D*) | Sectors joined by portals. |
| *Thief: The Dark Project* | Portal-based rendering, and sound propagating through rooms and doors: the shape for sound in our compartment graph. |
| *Doom 3* | Areas and portals, with closed doors cutting visibility at runtime: the shape of our portal culling. |
| Raspberry Pi documentation | The Pi 5's CPU, memory, GPU, display stack and cooling (to verify clocks, CMA defaults and throttling when writing the probe). |
| Mesa `v3d` and `v3dv` driver documentation | What OpenGL ES 3.1 and Vulkan features the Pi 5 exposes. |

## Raspberry Pi 5 graphics

Gathered 2026-10-04 for `engine-stack` ("What 60 frames a second buys on a Pi 5"). Numbers as
the sources state them; settings are often not stated, so they bound expectations and do not
replace the probe.

| Source | Cited for |
| --- | --- |
| [Raspberry Pi, "Benchmarking Raspberry Pi 5"](https://www.raspberrypi.com/news/benchmarking-raspberry-pi-5/) | glmark2 202 against 97 on a Pi 4; OpenArena timedemo 27.05 fps against 8.77 (Core Electronics' tests). |
| [Phoronix, Raspberry Pi 5 graphics review](https://www.phoronix.com/review/raspberry-pi-5-graphics/2) | glmark2 about 4.3 times a Pi 4 at 1080p; YQuake2 above 230 fps (from search results; the page refused a direct fetch, to verify). |
| [Mesa 24.3 adds Vulkan 1.3 conformance for V3DV](https://www.linuxtoday.com/blog/mesa-24-3-open-source-graphics-stack-adds-vulkan-1-3-conformance-for-v3dv/) | Vulkan 1.3 on the Pi 5 through Mesa. |
| [Xonotic forums, Xonotic on Raspberry Pi](https://forums.xonotic.org/showthread.php?tid=7724) | About 65 fps on an overclocked Pi 5 (user report, settings not stated). |
| [Jeff Geerling, an external GPU on a Raspberry Pi 5](https://www.jeffgeerling.com/blog/2024/use-external-gpu-on-raspberry-pi-5-4k-gaming/) | SuperTuxKart struggles at 1080p on maximum settings on the Pi 5's own GPU. |
| [Godot forum, Godot performance on Raspberry Pi](https://forum.godotengine.org/t/how-is-the-performance-of-godot-games-on-raspberry-pi-nowaday/112953) | Users find the Pi 5's GPU weak for 3D with a general engine's defaults. |

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
