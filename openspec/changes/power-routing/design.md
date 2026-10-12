# Design: power-routing

## Context

`power-grid` gave the Tern four switchboard sections, an emergency bus and nine distribution panels, with every load
hanging off a panel. On the grid mockup's floor plan (2026-10-11) the owner saw the loads as loose dots and most panels
bunched on decks B and C, and asked for a grid that connects everything through breaker boxes throughout the ship,
boxes that route power when switched, and a protection that trips the ship when too many conduits are live and
brings it back at its factory settings.

## Decisions

### 1. A conduit box in every room, runs through the doorways, a feeder to every load

`tools/power/boxes.py` derives them from `layout.json` and the hand-authored part of `power.json`, by the rules in
`power.json` `distribution`; `--check` fails when the file is stale. It never edits a hand-authored node, trunk or tie.

- **Boxes.** A room with a switchboard section or a distribution panel uses it as its box. Every other room gets one
  (`bx_<room>`, kind `box`): on the wall beside the doorway its feed comes in by, 1.2 m along the wall from the
  doorway's centre (clear of the frame), 0.3 m off the wall, 1.3 m over the floor. 20 new boxes; every one of the 30
  rooms has a box.
- **Runs.** One conduit through every doorway between two rooms (`r_<doorway>`, 29 of them), box to box under the
  ceilings (0.35 m under, or under the next deck in a room taller than a deck), unless a trunk or a tie already joins
  the two rooms' nodes. Windows and doors to space carry none.
- **At least two other panels each** (the owner, 2026-10-11: "there should be a panel in every room, panels should
  connect to at least 2 other panels, giving you some ideas for re routing power"). Every box joined to fewer than
  `min_runs_per_box` (2) other boxes by trunks, ties and doorway runs gets a spare to the nearest box in a room that
  touches its own: through a shared wall (walls within 0.8 m of each other for at least 1 m) on the same deck first, else
  through a deck (one room over the other), else along the hull to the nearest box on the nearest deck. 14 such runs:
  the bridge to the computer core and the ready room, medbay and damage control to the torpedo room, the quarters to
  medbay, the mess to damage control, each launch bay to the far hangar panel, the airlock up to the mess, the shield
  room to life support (a second upstream: the life support panel hangs off the forward switchboard's port section and
  the emergency bus, the shield room off the C spine) and down to the ventral pod, the dorsal pod to the A corridor, the
  side pods to their hangar panels. Every panel now joins 2 to 7 others, 3.6 on average.
- **Every system on two panels** (the owner, 2026-10-11: "systems should be connected to 2 or more panels, so we can
  swap with a different route, it should generally connect to the nearest panel"). Each system's `node` is its nearest
  panel as wiring runs, not as the crow flies: its own room's first, then its own deck's, then the nearest deck's,
  nearest first; its `alt_node` is the next. Each has a feeder (`feeder_m`, `alt_feeder_m`). The factory feeds every
  system from its node; an engineer swaps it to the other, and Engineering's automation swaps a system whose panel is
  dark to one that has power before it spends a live run on a spare (a swap is not a run, so it never counts against
  the protection). Examples: each reactor train and coolant pump cross-fed from the other main switchboard section; the
  shield generator from its room's box or straight from the forward switchboard's port section; each laser bank from
  its turret room or its pod. A room's lights stay on its own box: a box off puts its room on emergency light.
- **The reactor on five panels** (the owner: "reactor should connect to 4 -5 panels, since it is a critical resource
  to power the ship"): the two generators on the main switchboard's sections (30 MW each), and three reactor feeds to
  the drive panel and the two hangar panels (20 MW each), every one with its breaker. With both generators' breakers
  open the three feeds still carry the ship at cruise.
- **Factory configuration.** Breadth first from the rooms with switchboards and panels, in the layout's order of rooms
  and doorways: the doorway each other room is first reached by is its feed, closed at the factory (20 runs). The other
  9 doorway runs and the 14 runs through walls and decks are spares, open (23): the routes an engineer closes to bring
  power round a cut.
- **Ratings.** A feed run is rated for everything beyond it (each load at its `setpoint_max`, each room's lights)
  times 1.25, rounded up to 5, 10, 15, 20, 30 or 40 MW: the shield room's feed is 30 MW, a turret pod's 10 MW, the
  bridge's 5 MW. A spare takes 10 MW or the larger feed rating of its two rooms, so a spare can carry what it replaces.
- **Loads and lights.** Every load in a room is wired to that room's box (its panel where the room has one), with a
  feeder from the box under the ceiling to the load (`feeder_m`). The sensors and the comms array, outside every room,
  keep their panels. Each box lights its own room (30 lighting loads, the same 29.3 kW).

### 2. Switching and the trip

- **A box switched off** passes nothing: its loads get nothing, and neither does anything fed only through it. Switched
  on, it passes power again. Hand-authored switchboards and panels are boxes too.
- **A run** (a conduit or a tie) is closed or opened at its box. It is **live** when it is closed, not severed, and both
  its boxes are on.
- **The trip** (the owner: "Too many conduits open can cause the ship to shut down temporarily and reset the
  configuration to factory settings"). "Open" is read as switched on, carrying power (recommendation taken, ask only
  with screenshots). Every live run adds its charging current to the main protection, which allows `max_live_runs`
  (42) at once; the factory configuration is 39 (15 trunks, 4 ties, 20 feeds). One more trips the grid: every box but
  the switchboard sections and the emergency bus switched off and every run opened, for `trip_dark_s` (8 s). The
  switchboards' own loads keep their power through it, so the reactor's auxiliaries and the coolant pumps run on and
  a trip does not scram the reactor. Nothing can be switched while the ship is dark. Then every switch returns to its
  factory setting: every box on, every run as the factory left it, and the grid comes back.
- A severed run is not live, so a cut makes room for a spare.
- **A run open at its box carries no surge**: the damage cascade stops at it as at a severed one (`ship-damage` 3).
  The damage state mirrors the grid's switches each sub-step.

### 3. Engineering's automation routes round a cut

When a switched-on box with a load (or an island of them joined by closed runs) is cut off from the generators' and
the battery's nodes, and closing a spare would join it to the powered grid within the protection's limit, Engineering's
automation closes the first such spare, in the data's order, after its reaction time (3 s). It never closes a spare
that would trip the grid. A player at Engineering will route from the console once its routing view is approved.

### 3a. The engineer's schematic (owner, 2026-10-11)

The owner: "Engineer will have a view of the schematics on the side. But can click on an expand button to see the 3d
schematic in full screen. They can toggle between 2d and 3d view. 2d is a simplified graph view".

- **On the side**: Engineering's BUSES panel (2 x 2 cells, the one-line diagram and the battery) becomes SCHEMATIC: the
  2D graph in small (the switchboards, the trunks, the battery and the live-run count against the limit) with an
  EXPAND button. The console keeps its four panels (`bridge-stations` 8.0).
- **Full screen**: EXPAND opens the schematic over the console, with a 2D / 3D toggle and COLLAPSE back. 3D: the decks
  in perspective at their heights, an orbit camera, rooms as translucent floors, the boxes, runs, spares and feeders
  where they are. 2D: the simplified graph: boxes as nodes, runs as edges (spares dashed), each box's loads as bars
  under it, laid out for reading, not to scale. In both, a tap switches a box or a run, the live runs show against the
  limit, and a trip shows its countdown.
- **The 2D graph by deck** (the owner: "group 2d view by deck", "show how each deck is connected to each other", "im
  not sure i can read that 2d preiview you have there, too much overlappying", "when multiple lines cross the decks
  dont hesitate to show multiple parallels"; drawn since as a circuit board, 3c, which keeps a board a deck and the
  parallels between them): a band a deck, A over B over C, each deck one row of its boxes in order
  along the ship, evenly spaced so nothing covers anything. A run inside a deck is an arc over its row (a spare, open,
  a dotted arc under it), higher the longer it spans, so arcs cross but never lie on one another; each box's runs
  leave it from their own points along its edge. A run between decks is its own line from box to box across the gap,
  side by side with the others; the gap names its two decks and counts their runs and how many carry power.
- **One colour for electricity** (the owner: "black lines dont make sense to me at all, it should be lit up", "why not
  just use all yellow or all green? its the same concept, electric flowing"): every run, feeder, closed switch, box
  that is on and system with its power is yellow, wider the more a run carries, its dashes faster. The other colours
  are for trouble only: orange hatched short, red cut or dropped, blue an open spare, grey nothing reaches it, a dim
  outline switched off.
- The full-screen schematic is `docs/mockups/power-grid.html`; the side panel is drawn in `consoles.html`. Both wait
  for the owner's approval before the engine builds them (CLAUDE.md 10).

### 3b. The rooms overlay (owner, 2026-10-11)

The owner: "in your mockup add the ability to overlay rooms, so that if panels are turned off the lights in that room
go to emergency". The schematic's ROOMS toggle fills every room with the light the simulation gives it (lit by its own
box, red alert, emergency light, dark) and names the rooms off their own light; on the 2D graph every box carries a lamp
in its room's light and the overlay tints the box. A box switched off, or one nothing reaches, puts its room on the
emergency bus's light (the library's `lighting`, the same rule the engine's lighting states will read).

### 3c. The 2D schematic as a circuit board (owner, 2026-10-11)

The owner: "2d power grid should have that circuit board look. Maybe look up to make those online. I'm not sure how you
can programmatically place the tracer lines. Id expect you to be able to visually do that yourself". The owner then saw
the first board: "Beautiful!".

- **Placed by hand, kept in data.** Every part and every trace is in `data/ships/tern/power_board.json`
  (`starcrew.power-board/1`): a part is its centre and size on the board, a trace its points from pad to pad. Nothing
  is laid out by code; the mockup reads the file, and the engine's console will read the same file once the schematic is
  approved. A run added to `power.json` with no trace on the board is reported in the console log.
- **The look, from PCB practice** ([TI layout guidance](https://www.ti.com/document-viewer/lit/html/SLLA284G/GUID-70A502C2-0DE5-4A92-8C3A-BE16D8AB564C),
  [Flux, routing techniques](https://www.flux.ai/p/blog/pcb-routing-techniques),
  [Circuits DIY, routing tips](https://www.circuits-diy.com/beginners-guide-to-pcb-routing-tips-best-practices/),
  [ALLPCB, routing strategies](https://www.allpcb.com/ar-SA/allelectrohub/mastering-pcb-routing-top-strategies-for-efficient-designs),
  [All About Circuits forum, design practices](https://forum.allaboutcircuits.com/threads/design-practices-for-pcbs.129181/)):
  a green solder-mask board a deck with mounting holes in its corners; traces only at 0, 45 and 90 degrees with every
  corner cut at 45; power traces widest (a trace's width by its run's rating, 5 MW thin, 40 MW wide); a pad where a
  trace meets a part; a via where a trace leaves its board; the parts as chips with their names in white silkscreen.
- **Two layers.** A working run is copper on the top layer, yellow, with its flow moving along it as before (3a). A
  spare, open, runs on the bottom layer: thin, blue, dashed, free to cross anything. Where two top-layer traces must
  cross, the lighter one drops through two vias to the bottom layer and back, as a lighter trace is routed under a
  heavier one.
- **How it was routed.** The hub first: the reactor in the middle of deck B with the main switchboard's port section
  above it, the starboard section below, the drive aft and the hangar panels forward, so its five feeds and the two
  generators leave it on five sides and cross nothing; the main bus tie runs round the drive. Port panels on deck B's
  top row, starboard on its bottom row. The traces between decks run straight across the gap, never along it: the A
  deck trunks run round the board's edges (aft to the main switchboard, forward to the forward switchboard), and the
  port feeders reach deck C through the gap between the turret rooms and the crew rooms. On deck C the forward
  switchboard's sections sit side by side with the emergency bus and the battery under them, life support between the
  bus and the C panel, and the C panel's rooms forward. Two crossings are left, each a hop: the port launch bay's feed
  under the starboard hangar trunk, and the B spine's feed under the port forward feeder. A check while routing:
  nothing crosses a part or a mounting hole, no two traces run closer than 9 points side by side.
- **What stays the same.** Every tap and colour of 3a and 3b: tap a part's switch to switch its box, a trace to open or
  close its run, a system's LED to swap its feed; the room lamps and the rooms overlay; the gap between two boards
  counts its traces and the live ones.

### 4. What it does to the fight (2026-10-11, bots at Helm and Tactical, Engineering on automation, seeds 1-8)

With the doorway runs alone: 2 victories (in 454 and 514 s), 3 defeats, 3 fights to the 600 s limit with the Hound at
72-108 of its 250 MJ. With every panel joined to at least two others the automation has more ways round a cut: 3
victories (509-541 s), 2 defeats, 3 to the limit with the Hound at 6-128 MJ. With every system on two feeds and the
reactor on five panels: 2 victories (438 and 541 s), 5 defeats, 1 to the limit. The runs all start at their panels, so
they bundle there: in seed 3 one hit by the B spine's panel severed its trunk and 14 runs at once, every way into the
rooms forward of it, and later hits took the main trunks (question R2). The bot test now holds a win in six, every
fight 4-10 minutes, and the Hound damaged in every one. The
losing fights turn on the forward switchboard room: the two forward switchboard sections and the emergency bus share it,
nine trunks end there, and the spine's spares run past it, so one hit (seed 2, 270 s) severed 15 conduits at once and
starved the bow (the shields at 5.6 of 15 MW). The bot test now holds the drill winnable without an engineer (a win in
six), every fight 4-10 minutes, and the Hound below 60 % in every one.

### 5. Pi 5 budget

20 nodes and 29 conduits more in the solve (34 nodes, 44 conduits): the augmenting paths stay breadth first over a
graph that small, well inside the sub-step's estimate (`power-grid` 14). The live count and the reroute check walk the
same graph once a sub-step. No new network traffic until the console's routing view: the switches will ride in the grid
snapshot at 2 Hz (a bit a box and a run, 10 bytes).

## Risks / Trade-offs

- **The forward switchboard room is a single point of failure** (section 4). Splitting it into port and starboard rooms
  is a layout change to `reference-ship-tern`: open question R1.
- **One feed a room until a spare is closed**: rooms with one doorway (the bridge, the shield room, the pods) have no
  spare, so a cut there waits for the repair teams. Runs through walls between rooms with no doorway would give them
  one; not built.
- **A player at Engineering cannot route yet**: the console's routing view is a new screen and waits for its mockup.

## Open questions

| Id | Question | Options | Recommendation | Shot |
| --- | --- | --- | --- | --- |
| R2 | One hit at a panel severs every run bundled there (the damage march takes each conduit its reach touches). Should runs leaving a panel be harder to cut? | (a) As it is: a hit at a panel is a hit on everything it feeds. (b) Armoured runs within 2 m of a panel: a conduit takes a share of a hit by its length inside the reach. (c) Runs leave a panel in two bundles, port and starboard, so a hit takes one. | (c): it is how the trunks already run, and the routing game keeps its spares | `power-grid.html`, its 3D view at a panel |
| R1 | Split the forward switchboard room, so one hit cannot cut the bow off? | (a) As it is: a hit there is the ship's weak spot. (b) Port and starboard switchboard rooms on deck C, the emergency bus with one. | (b) | `power-grid.html`, its "Port switchboard lost" and "Both trunks cut" scenarios |
