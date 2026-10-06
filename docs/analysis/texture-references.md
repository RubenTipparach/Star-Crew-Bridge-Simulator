# Wall textures: what the owner's references teach

The owner, 2026-10-05, looking at the mockups' walls: "whats going on with the walls textures?
heres some inspiring textures star trek elitre force and alien resurrection", "additional
inspiration: Quake 1, 2, 3, Halo, Unreal Tournament", a link to
https://timothywilson.artstation.com/projects/58dYqg, and then "notice how they dont have square
panels on the walls? its all paneling with different shapes, vents, pipes etc to make the room
feel industrial, mechancial and lived in".

This note describes what the owner sent and draws the lessons. The images belong to their
studios and artists. They are cited here and never copied into the repository (CLAUDE.md
section 15: take the shape, not the art). The plan that applies the lessons is the OpenSpec change
`wall-panels`.

## What the walls are today

The interior walls take one material, `bulkhead`. It is Undercity's `tech_panel` graph
(`data/materials/materials.json`): a single square panel with a bevelled edge, four rivets and
curved white scratches, at 128 px over 2 m, so one panel 1 m square. The same graph, larger,
is `trim` (ribs, frames and baseboards) and the hull. So every wall in the ship is a grid of
identical 1 m squares, and from the camera they read as tiles:

- **The scratches read as marble.** The curved white strokes, repeated on every square, look
  like veins in stone, not wear on metal.
- **It is pale and cool.** The ramp runs to RGB (0.60, 0.65, 0.70); the references are dark and
  warm.
- **There is one panel, so there is no composition.** No vents, pipes, hatches, grilles, labels
  or lights. A 15 m wall shows the same square 45 times.
- **Trim matches the wall.** Ribs and frames carry the same squares, so the structure the
  generator builds (`deck-pipeline` 5a) does not stand out from the plating.

## The references

Numbers read off an image are estimates from its proportions, marked "about".

### X1. Star Trek: Elite Force: a Borg interior (Raven Software, 2000; Quake III engine)

A screenshot from the first-person game, looking down a wide hall toward a raised far end.

- **Massive forms, dense relief.** The hall is framed by thick pillars clad in stacked horizontal
  blocks, each block with a row of round sockets, with cables and conduit running up their
  edges. Over the opening, a deep lintel band is packed with mechanical relief: boxes,
  cylinders, vertical tubes. The relief is all texture on simple, large shapes.
- **A focal point lit from inside.** At the far end, up two steps, four tall alcoves glow green,
  each with a figure standing in it, under white light bars. One amber lamp hangs in the middle
  distance.
- **Floor:** dark square grate tiles with a lighter solid walkway down the middle, and pale
  step edges in the foreground.
- **Colour:** desaturated grey-green and olive, near-black shadows; green and amber are the only
  saturated colours, and they are lights.

### X2. Alien Resurrection: a service shaft (the game; source to verify)

A low-polygon corridor seen from above at an angle: a tall square shaft with a door at its end.

- **Walls in bands.** The side walls are worn brown and rust plate. A long horizontal band of
  louvres runs along each at about waist height, with a darker band of ribbing above.
- **The door is the hero.** The end wall is a tall dark green-grey door with vertical ribs. It is
  flanked by panels holding stacked octagonal vent grilles, and by small amber light strips on
  either side.
- **Recessed, framed panels.** The right wall has a large recess under a stepped bracket
  overhang. It holds vertical grille columns and a column of four small glowing windows.
- **Floor:** square grates, each in its own raised frame, with dark channels between them; two
  raised floor vents by the far wall.
- **Colour:** warm brown, umber, rust and dark olive; small amber and white lights.

### X3. A sheet of about thirty wall textures (sent with X1 and X2; source to verify)

An atlas of tall and wide wall textures in the same era's style, about 512 x 830 px as sent.
None of them is a uniform tile. Each is a designed panel:

- **Pipes and conduits:** tall grey panels with vertical rods, horizontal pipe runs and clamps,
  and a vent grille band at the foot.
- **Louvres and grilles:** framed vent panels with horizontal slats; dark ribbed panels.
- **Lights in the texture:** a large yellow-white panel of lamp cells; a pale green glass panel
  lit along its top; vertical strips of small lit "pills"; a framed row of white lights; single
  amber slits.
- **Hatches and doors:** dark door-like panels with vertical light bars; stepped bevelled
  frames.
- **Wear:** rust-brown plate with streaks, and one with a red splatter (a horror game's
  lived-in mark).
- **Colour:** dark greys, browns and olive with warm highlights; light is white, amber or
  yellow.

### X4. Games named, not sent (recalled, to verify against the games before a detail is quoted)

- **Quake (1996), Quake II (1997), Quake III Arena (1999):** brown, rust and grey "base" metal
  sets: plates with rivets, trims, and light fixtures painted into textures.
- **Halo: Combat Evolved (2001):** ship corridors of grey-blue panels with recessed lights.
- **Unreal Tournament (1999):** tech sets of panels, trims and light strips (already this
  project's style reference through Undercity's `docs/ut99_reference.md`).

### X5. Timothy Wilson's ArtStation project

https://timothywilson.artstation.com/projects/58dYqg, linked by the owner. It returns HTTP 403
to this cloud session, so it is not described here. To read on a machine that can open it.

## What they have in common

| Lesson | Where it shows | What it means for the Tern |
| --- | --- | --- |
| **No grid of identical squares** | X1-X3 (the owner: "they dont have square panels on the walls") | A wall is a sequence of different panels: plate, vent, pipes, hatch, louvre, light column, screen. Neighbours differ. |
| **The wall is banded** | X2 (a louvre band at waist height, ribbing above), X3 (grille bands at a panel's foot) | Each wall has a base band (kick plate, grille), a main band (the panel), a top band (pipes, conduit), so the eye reads height and scale. |
| **The texture carries the detail; the geometry stays simple** | X1 (relief on large blocks), X2 (low-poly walls) | Detail belongs in the texture of a plain quad. Our generated ribs, beams and coves stay the structure, and the panels fill the bays between them. |
| **Light is part of the panel** | X1 (light bars), X2 (amber strips by the door), X3 (lamp cells, pills, slits) | Panels carry small emissive strips and windows, in the emission mask the materials already have. |
| **Doors and features are framed** | X2 (a door flanked by vents and lights) | The bays beside a door, a console bank or the viewscreen take matching surround panels. |
| **Floors are grates with a path** | X1, X2 | Working spaces take framed grates, with a solid plate path where people walk. |
| **Dark, warm, worn, with saturated light** | X1-X3 | A darker, warmer palette, wear and streaks, with saturated colour only in lights and hazard marks. |
| **Variety hides repetition** | X3 (thirty panels) | About eight panel designs per kind of space, chosen per bay so that the same panel never sits next to itself. |
