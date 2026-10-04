# Proposal: the bridge, its stations and the people who run them

## Why

"The game simulates a starship bridge with multiple stations and activities the crew can do to
make sure the ship is operating at peak capacity" and "a full 3D starship bridge is vital to
making this game work" (owner, 2026-10-04). A bridge simulator lives or dies on its stations:
what each seat can do that no other seat can, what happens to a seat nobody is sitting in, and
how a crew of one to eight fills fourteen seats without anyone waiting for a job.

star-crew-64 proved that four stations reached on foot, one occupant per console and NPC bodies
in empty seats make a game (`docs/analysis/star-crew-64.md`). It also left a seat locked to an
NPC body with the helm frozen mid-turn, consoles as file-static singletons, station positions
hard-coded in C and a science station that was a rhythm game rather than a system. This change
designs the stations properly, before any engine code exists (CLAUDE.md section 4).

## What Changes

Everything below is **proposed** unless it says it comes from the layout or the brief, which are
**decided**.

- **The station roster** (decided by `data/ships/tern/layout.json`): seven bridge stations
  (helm, tactical, engineering, science, captain, comms, flight operations), the engineering bay
  console, the damage control board, bay control and four gunner pods. For each one this change
  states its purpose, every control and readout, its keyboard and mouse and gamepad bindings,
  what automation does when nobody sits there (reaction time, accuracy and the decisions it will
  not make) and which console absorbs it when it is merged.
- **Operators, not bodies.** A station's operator is a player, automation, or a merged tab on
  another console. The body in the chair is presentation. Standing, disconnecting, being downed
  or swapping body hands the station to automation in the same 30 Hz tick, starting from the
  station's current state (the fix for star-crew-64's locked seat and frozen helm).
- **Automation** is the ship's computer, drawing on the computer core. It works on setpoints the
  crew give it, at a stated, lower competence, and refuses a written list of decisions
  (missiles, evasive flying, venting, negotiation). It issues the same commands a console does,
  through the same validation: one implementation.
- **Merging.** A station nobody sits at keeps its automation and appears as a tab on a manned
  console, chosen by a merge list in data. One player can run the whole ship from one seat; four
  players are the core crew; players five to eight take seats automation was holding.
- **Seat rules**: claim, release, relieve, body swap, switching station, and what the server
  holds (netcode-and-sessions).
- **Command**: the captain's condition (normal, red alert), orders to stations with acknowledge
  and unable, and what red alert changes ship-wide (lighting, automation presets, klaxon).
- **The console UI framework**: immediate-mode 2D on a 1280 by 720 logical canvas (drawn at
  1.5 device pixels per logical pixel on the Pi 5's 1920x1080 output), a title band, a look band
  that keeps the bridge and viewscreen visible above the console, a 12 by 4 panel grid with
  fixed panel sizes, a status strip, colour roles, text bands, guarded controls, and a pixel
  wireframe for every bridge console. Every preview on a console calls the function that
  resolves it.
- **The bridge as a room**: its dimensions from the layout, sightlines from every seat to the
  viewscreen, the viewscreen (feeds, who steers it, its 512x256 render target at 15 Hz), the two
  windows, lamps and strips for normal, red alert and emergency power, sound, and its Pi 5 cost.
- **A mockup**, `docs/mockups/bridge.html`: the bridge built from the layout with a live
  exterior on the viewscreen and through the windows, three lighting states, walk mode, and every
  bridge console as an HTML overlay matching the wireframes.

## Capabilities

### New Capabilities
- `bridge-stations`: the station roster, operators and automation, merging, seat claim and
  release, command and alerts, the console UI framework and the bridge as a room.

### Modified Capabilities
None. `openspec/specs/` holds nothing yet.

## Impact

- **Data (proposed):** `data/stations.json` (roles, stations, merge lists, automation tuning with
  units), `data/consoles/<role>.json` (panel grids, validated against the framework),
  `data/input/bindings.json` (keyboard, mouse and gamepad bindings per station).
- **Core (proposed):** a `crew` module in the simulation core owning stations, operators, seats,
  orders and condition; an `automation` module; the console previews call `power`, `flight`,
  `weapons`, `shields`, `sensors`, `life_support` and `damage` functions those changes own.
- **Client (proposed):** the immediate-mode UI module (one font atlas, one streamed vertex
  buffer, at most 3 draw calls), the console layouts, the look band viewport, the viewscreen
  render target.
- **Other changes:** `power-grid`, `weapons-and-shields`, `flight-and-navigation`,
  `shuttle-bay-and-fighters`, `life-support`, `damage-control`, `netcode-and-sessions`,
  `ship-frames`, `crew-on-deck`, `deck-pipeline`, `engine-stack` (the Pi 5 budget table). The
  interfaces are listed in design.md.
- **Pi 5 budget:** the bridge's own geometry is about 18,000 triangles in 3 draw calls (60 % of
  its 30,000 ceiling). With the command passage seen through the aft door, eight crew avatars,
  the exterior through the windows, the viewscreen pass and the console UI, the worst frame on
  the bridge is about 116,000 triangles and 79 draw calls: 58 % and 26 % of the provisional
  Pi 5 budget. The breakdown is in design.md.
