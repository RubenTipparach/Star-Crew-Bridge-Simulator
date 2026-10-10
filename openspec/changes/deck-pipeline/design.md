# Design: the deck pipeline

## Context

**What is decided elsewhere and used here.**

| Fact | Source |
| --- | --- |
| One layout source per ship, `data/ships/<id>/layout.json`, validated by `tools/layout_check.py` | CLAUDE.md section 8 |
| Decks are convex brushes compiled offline; visibility is portal culling through the compartment graph; no z-fighting; people stand clear; every compartment states its numbers | CLAUDE.md section 8 |
| Low poly, lit by light baked into vertex colours (one set per lighting state: normal, red alert, emergency power, blended per compartment) that multiply a Material Maker texture from one texture array (owner, 2026-10-05) | CLAUDE.md section 9; `surface-materials` |
| Compartments are convex prism brushes that follow the hull (layout schema v2, 2026-10-05) | `reference-ship-tern` sections 1a and 10 |
| How a bake is computed (shadow rays, emissive surfaces, occlusion, bounce, subdivision for lighting detail, probes for moving things, determinism, bake time) | `light-baking` (this change only calls it) |
| Rust; `sc-core` (simulation, no I/O), `sc-render` (GL, portal culling), `sc-tools` (`deckc`, `meshc`), `sc-server`, `sc-client` | `engine-stack`, sections 2-3 |
| Pi 5 floor: 1 GB client, 4 GB Pi 5 may host the server; OpenGL ES 3.0 floor (32-bit indices, instancing, VAOs, uniform buffers allowed; no compute or geometry shaders); 3D at 1280 x 720, 60 fps target, 30 fps floor | `engine-stack`, section 5 (provisional, owner 2026-10-04) |
| Per frame, all passes: 200,000 visible triangles, 300 draw calls; 64 MB vertex and index buffers; bridge geometry 30,000 triangles; other compartments about 8,000 | `engine-stack`, section 5 |
| Frames and the rendering composition: viewscreen target, exterior far and near layers scissored to the visible windows, depth clear, interior pass, glass and UI | `ship-frames`, proposal |
| The Tern's floor plan, POI numbers and routes | `reference-ship-tern` |

**What the owner said.** "I want this to run on pi3 1gb ram! So definitely bsp style level
design would be on the table!", then "I'm sorry we're running on a pi5 1gb-4gb, 4 GB can be
used as main server too", and "this game doesn't need high end graphics". The budgets below
are ceilings, not targets: the kit's density is chosen for the low-poly look, and the ceilings
are what the compiler refuses past.

**How the numbers were measured.** A scratch measurement (a measurement instrument in the sense
of CLAUDE.md section 4; it changes nothing) read the Tern's layout and computed, per
compartment, the surface areas net of every opening, the shell's triangles when tessellated to
a world-aligned grid split at every opening's edge, the kit's detail from the densities in
section 5, the props from the layout's stations, systems, fixtures and portals at the costs in
section 5, and the draw calls by pass. Its method is fully stated here so the numbers can be
rebuilt; `deckc --report` (task 6.1) replaces the estimate with the compiled truth.

**References and what each is cited for** (shapes taken, not text; `docs/references.md`).

| Reference | Cited for | Status |
| --- | --- | --- |
| Quake (id Software, 1996): `qbsp`, `vis`, `light` | Convex brushes as the authoring primitive; CSG to drop faces hidden between brushes; T-junction repair after splitting faces; a precomputed potentially visible set (PVS) because its levels have thousands of leaves; collision against brush planes expanded by the mover's size ("hulls") | The hull method and the T-junction pass (`tjunc.c` in the released source) to verify |
| Quake III Arena | Brushes kept in the compiled file for collision, traced against with a box or capsule, found through the BSP's leaf brush lists; per-surface patches tessellated for lighting | Capsule traces to verify (ioquake3 `cm_trace.c`) |
| Quake II | Area portals: a PVS combined with door state at run time | To verify |
| Descent (Parallax, 1995) | A level of connected cube segments rendered by walking segment sides with a narrowing screen window, no PVS: the closest shape to a ship of box compartments | To verify the clipping details |
| The Build engine (Duke Nukem 3D) | Sectors joined by portal walls, drawn by walking portals with screen-space clipping; walls are sector boundaries with no thickness of their own | To verify |
| Thief: The Dark Project (Dark engine) | Cells and portals for rendering, and sound through the same rooms and doors | To verify |
| Doom 3 (id Tech 4) | Areas and visportals walked at run time with screen-space portal rectangles; doors close their portals | To verify |
| Undercity, `/home/user/fps-game-demo/CLAUDE.md` 7.1, 7.2, 7.4 and the `blender-csg-levels` skill | One layout source shared by the map and the build; the z-fighting rule and its checker (`detailing.assert_no_zfighting`, `PLANE_TOL` 5 mm); frame props with inset clear openings (`REVEAL` 0.1 m); people tested with the body's own collider; ENT_ empties for entities; Blender as the owner's chosen level tool | Read from the repository |
| star-crew-64, `docs/analysis/star-crew-64.md` | The grid level format, its compiler (`compile-levels.py`), a per-cell room id driving room-scoped systems, and the browser level editor | Read from the analysis |

## Goals / Non-Goals

**Goals:**

- A deck is compiled from the one layout source and nothing else about rooms; a change to the
  layout regenerates the deck, and a deck that disagrees with its layout is refused at load.
- Every refusal the rules ask for is made by the compiler, with the offending ids named.
- The run-time cost of visibility and collision is microseconds, and is stated.
- Every Tern compartment has a number, and the frame's interior pass has a ceiling the
  compiler enforces.

**Non-Goals:**

- How light is computed. `light-baking` owns the baker; this change owns where the result
  lives, how it is stored and how it is blended (section 7).
- The ship's exterior mesh, the exterior pass and the viewscreen picture (`ship-frames`).
- Crew movement rules: speeds, step height, ladders, carrying (`crew-on-deck`). This change
  supplies the collision query and the checks that use the crew capsule.
- Destructible geometry. Damage changes state (doors jam, lights fail, breaches open as
  portals declared in the layout), never brushes.
- An in-game editor.

## Decisions

### 1. Authoring: four options weighed

The owner's word on level tools, from Undercity (`/home/user/fps-game-demo/CLAUDE.md`,
section 1): Blender is "definitely" the winner; TrenchBroom "has some cool stuff"; Godot CSG is
"the weakest by far".

| Option | One layout source | Convex brushes (collision, checks) | Hero detail | Reproducible in CI | Cost to build | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| **A. Layout plus a generator (`deckgen`) and a kit** | Yes: the layout is the input | Yes: it emits them | Scripted only | Yes: deterministic, no external tool | Small: a generator and a kit table | **Backbone** |
| B. Blender CSG kit, as in Undercity | Only if the shell is built from the layout | No: booleans produce triangle soup, so collision and checks need a second, convex model | Strong | Blender headless, version pinned | Port the kit; booleans must never carve | **For hero detail only, without booleans** |
| C. TrenchBroom `.map` with a game configuration | No: a `.map` holds its own rooms, a second source that drifts | Yes: native convex brushes | Strong for brush work | `.map` text is diffable | Game config, entity definitions, importer, a round trip with the layout | **Borrow its ideas** (an entity definition table, brush entities, clip brushes), not the tool |
| D. A browser deck editor from star-crew-64's | Could edit the layout directly | No: star-crew-64's is a 2 m tile grid, single deck | Weak until a brush editor is written | Yes | Large: an editor is a project | **Later**, as a layout inspector built on `shipkit.js` |

**Recommended: A as the backbone, B for hero detail** (recommendation taken, ask only with
screenshots; see K3).

- **The layout is the plan.** Compartments, portals, stations, systems, fixtures, mounts and
  craft come from `layout.json` and nowhere else (CLAUDE.md section 8).
- **`deckgen` is the builder.** It emits every compartment's shell brushes, door frames,
  ladders, stairs, trims, ribs, beams and lamp fixtures from the kit (section 5), and places a
  prop for every station, system and fixture the layout names.
- **Hero detail is a detail file.** `data/ships/<id>/detail/<compartment>.json` lists extra
  convex brushes, prop placements and light fixtures in ship metres. A Blender script loads the
  layout and draws the compartment's air brushes and portals as locked reference, the artist
  places detail objects (each must be convex: the export compares each object's volume with its
  convex hull's and refuses a difference over 1 %), `PROP_<mesh>_<n>` empties and
  `LIGHT_<kind>_<n>` empties (Undercity's ENT_ convention), and the export writes the JSON with
  one rotation from Blender's Z-up axes to ship axes. The JSON is the committed source;
  `deckc` never opens a `.blend`. A detail file can also be written by hand or by a generator.
- **Nothing carves.** No boolean operation runs anywhere in the pipeline. Openings come from
  the layout's portals; a shape that would need carving is a set of convex brushes.

### 2. The pipeline end to end

```
data/ships/tern/layout.json ----+                       data/ships/tern/detailing.json
  (the plan: one source)        |                         (detail rules, sizes, finishes)
data/materials/materials.json --+  (texture layers; surface-materials)
                                v                                 |
                         deckgen (sc-tools) <---------------------+
                                | brushes, props, fixtures, entities per compartment
data/ships/tern/detail/*.json --+  (hero detail: brushes, props, lights)
meshc props (compiled/*.mesh) --+
                                v
                          deckc (sc-tools)
   1. validate the layout (the same rules as layout_check, from sc-core's layout module)
   2. per compartment: brushes -> faces; drop hidden faces; split at openings
   3. checks: convexity, inside-air, z-fighting, frames fit, people stand clear, walk graph
   4. tessellate and bake: light-baking's baker, three states -> three colour sets
   5. T-junction repair, weld, merge by pass, pack vertices and indices
   6. collision: brushes, contents, k-d tree where a compartment is large
   7. budgets: per compartment, worst visible set
   8. write compiled/tern.deck (chunks, CRCs, layout SHA-256) and the report
                                |
           +--------------------+---------------------+
           v                                          v
  sc-server: core chunks (graph refs, brushes,  sc-client / sc-render: everything
  entities): collision, seats, spawns           (meshes, draws, movers, lights)
```

`deckc` uses `sc-core`'s compartment graph and layout validation, so the rules exist once
(CLAUDE.md 6.1, and `engine-stack`'s note that `sc-tools` "uses sc-core's compartment graph").
`tools/layout_check.py` remains the documentation-time check until that module exists; when
it does, the Python checker either calls `deckc --check-layout` or is retired, so two copies
of the rules never live side by side (task 1.3).

### 3. Partitions: walls between compartments have no thickness in the layout

**Finding.** The layout's brushes are the air of each compartment, and neighbouring compartments
touch: the ready room's port wall and the command passage's starboard wall are the same plane,
`x = 1.25`. The convention's `wall_thickness_m` (0.25) is used only against the hull. Counting
each side, 1,849.9 m^2 of wall is shared between compartments (v2 plan, measured 2026-10-05 by a
scratch instrument over the brushes that gives the v1 boxes' 1,667 m^2 exactly; v2's rooms grew
outboard, so their athwartship partitions are longer). Two horizontal faces are shared with no
slab: the dorsal and ventral pods sit directly on their access spaces (5.2 m^2 each). On the v1
boxes the hangar's galleries also sat directly on the launch bays (66 m^2 each, at `y = 0`); v2
stops the bays' brushes at `y = -0.5`, so that slab is in the layout.

| Option | What it means | Cost |
| --- | --- | --- |
| **a. Zero-thickness partitions** | A shared face is drawn back to back, once from each side, and each side's collision slab lies outside its own air. A door's depth comes from its frame, which stands 0.1 m proud on each side. | No layout number moves. A sliding leaf needs a jamb casing to slide behind. |
| b. Inset at shared faces | The compiler shrinks each compartment by half a wall (0.125 m) at every shared face. | 231 m^3 of air (2.4 % of the ship; v2 plan, 2026-10-05, was 208 m^3), and 11 % of each 2.5 m corridor's volume; the simulated volumes and the drawn rooms would disagree unless the layout is re-measured. |
| c. Gaps in the layout | Move every brush apart by 0.25 m. | Every portal plane and most brushes move; every other change's numbers move with them. |

**Recommended: a** (K1). It is how sector and segment engines treat walls (the Build engine's
walls are sector boundaries; Descent's segment sides likewise, to verify), it keeps every
volume the life-support design quotes, and back-to-back faces are not z-fighting because they
face opposite ways (Undercity 7.2: "Faces pressed back to back are fine"). Collision is per
compartment (section 4), so a slab that reaches into the neighbour's air is never tested from
the neighbour. For a horizontal shared face, the slab lies below the upper floor, inside the
lower compartment, which is drawn that much lower than its brush while its volume in the core
keeps the brush's number. On the v1 boxes that was the launch bays under the galleries (drawn
3.0 m clear, not 3.5 m; K1 asked whether to re-measure). Since v2 (2026-10-05) the layout
carries that slab itself: the bays' brushes are 3.0 m tall and their volumes are the drawn ones,
and only the pods' hatch faces remain.

### 4. Brushes and collision

**A brush** is a convex polyhedron: a list of planes (normal and distance, in ship metres,
`f32`), its bounds and a contents mask. Contents:

| Bit | Contents | Collides crew | Drawn |
| --- | --- | --- | --- |
| 0 | `solid` (shell slab, prop proxy, stair, railing) | Yes | Its faces |
| 1 | `crew_clip` (smooths a cluttered wall: Quake's clip brush) | Yes | No |
| 2 | `detail` (trim, rib, beam, lamp housing, under 0.15 m proud) | No | Its faces |
| 3 | `ladder` (a climbable volume) | Climb | No |
| 4 | `glass` | Yes | Transparent pass |
| 5 | `mover` (door leaf, hatch lid) | When closed | Mover pass |

**The query.** `sc-core` exposes one swept test: a vertical capsule (radius `r`, half-height
of the straight part `h`) moving from `p0` to `p1` inside one compartment, against that
compartment's brushes whose contents match a mask. For each brush, each plane is pushed out by
the capsule's support distance `r + h * |n_y|`, and the segment `p0 p1` is clipped against the
expanded planes; the earliest entering fraction over all brushes is the hit (the shape of
Quake III's brush traces, to verify against `cm_trace.c`; the expanded-plane test is
conservative at a brush's edges by at most `r * (1 - 1/sqrt 2)`, which a bevel plane per edge
removes, as Quake's compilers add axial bevels, to verify). Crew movement (`crew-on-deck`)
slides along the hit plane and steps up, calling the query at most four times per tick.

**Which brushes.** A capsule whose centre is in compartment `C` tests `C`'s brushes, plus the
neighbour's when it is within `r` of an open portal's plane and inside its opening. Crossing a
portal's plane inside its opening changes the body's compartment at that tick. Since each
compartment's shell slabs lie outside its own air, no partition needs real thickness (section
3).

**A k-d tree where it pays.** A compartment with more than 32 colliding brushes gets an
axis-aligned split tree (leaves of at most 8 brushes), built offline; a trace walks only the
leaves its swept box touches. On the Tern that is the hangar and engineering (stairs,
railings, the reactor, the mezzanine ring, cradles, the landing); every other compartment is a
flat list.

**Cost.** About 800 colliding brushes for the Tern at 7 planes each (estimate). A trace tests
at most about 30 brushes of 7 planes at about 10 floating point operations a plane: about
2,100 operations; four traces per body per tick, 16 bodies (8 crew, 8 automation stand-ins),
30 ticks a second: about 4 million operations a second, under 0.1 ms per tick on one
Cortex-A76 core (estimate, to measure with `sc-probe`). Collision data on the server: about
0.2 MB.

### 5. The kit and the render meshes

**The kit** is now concrete: `data/ships/<id>/detailing.json` (units in keys, validated,
CLAUDE.md 6.5), one per ship because the detail is a ship's style, and its rules are section 5a
(2026-10-05; this replaces the proposed `data/decks/kit.json`). The table below is the first
estimate the section 11 numbers were measured with, kept for the record; section 5a's rules and
`tools/mockups/kit_report.mjs`'s counts supersede it for the shell and detail (props, stations,
systems and fixtures are still the table's):

| Piece | Rule | Triangles |
| --- | --- | --- |
| Shell faces | Every brush face not shared with the same compartment's other brushes, minus the portal openings | 2 per cell of the lighting grid (section 7) |
| Baseboard and cornice | Along every metre of wall at the floor and the ceiling | 4 + 4 per metre |
| Wall rib | Every 2.0 m along walls, 0.2 m wide, 0.08 m proud | 12 each |
| Ceiling beam | Across the short span every 2.0 m | 6 per metre of beam (3 per m^2 of ceiling) |
| Lamp fixture | One per 8 m^2 of floor in rooms and bays, one per 3 m along corridors, one per pod | 12 each (plus its emissive lens) |
| Door frame and leaves | Per side of a door | 36 (pressure door 68, hatch 40) |
| Ladder | Per side of a ladder portal | 40 |
| Hoist | Per side | 120 |
| Window frame and pane | Per window | 34 |
| Bay door | Frame and two leaves | 56 |
| Station | Seat, console, screen face; bridge stations are hero props | 260; bridge 420 |
| System | Per system, by kind: the reactor 1,600, impulse drive 800, shield generator 600, missile racks 520, others 120-420 | per kind |
| Fixtures | Viewscreen 24, dais 48, landing 160, mezzanine ring 960, catwalk 120, stair 140 | per kind |

**Frames fit their openings** (Undercity 7.2). The layout's portal size is the clear opening
crew pass through and air flows through. The wall is opened at the clear size plus the frame:
a jamb each side (0.16 m; a pressure door's 0.26 m) and a lintel (0.22 m), and the frame fills
that ring, standing proud of the wall on each side (0.06 m; a pressure door 0.12 m), so the
clear opening is never narrowed (section 5a, revised 2026-10-05 from a 0.1 m reveal plus 0.1 m
casing). Measured on the Tern, every frame fits its wall below the cove. Sills above the floor
are deliberate where a door opens onto a mezzanine, a landing or a pod coaming.

**From brushes to faces.** Each brush's faces are clipped against the compartment's other
brushes, and a face (or part of one) that lies inside another brush or against a face pointing
the other way is dropped (the shape of `qbsp`'s CSG step). Shell faces are split at portal
openings. The light baker then tessellates (section 7); afterwards `deckc` repairs
T-junctions: every vertex that lies on another face's edge is inserted into that edge, so no
crack shows where a split face meets an unsplit one (`qbsp`'s T-junction pass, to verify).
Vertices are welded within 0.1 mm.

**Passes and draw calls per compartment.** Static geometry is merged per compartment, so a
compartment is at most four draws:

| Pass | Holds | Shader |
| --- | --- | --- |
| Opaque | Shell, detail, props: everything static | The vertex's texture layer, times three colour sets blended by the compartment's uniforms |
| Emissive | Lamp lenses, screens, status lights, strips | The texel (its alpha is the emission mask) times the state's emissive tint; not dimmed by the baked light |
| Movers | Door leaves, hatch lids, valve wheels, breaker handles | Opaque, each vertex moved by its mover's transform from a uniform buffer |
| Transparent | Window panes, glass partitions | Back to front (section 9), alpha blended |

Merging per compartment works because every material is a layer of one texture array: one
draw holds a room's floor, walls, trims and lamps whatever their materials (the mockups draw
each compartment in one call for the same reason). Static geometry is not instanced, because
baked colours are unique per vertex; instancing is
for crew, craft and projectiles (`engine-stack`).

**The vertex** (28 bytes; OpenGL ES 3.0 formats; revised 2026-10-05 for the texture array of
`surface-materials`, the palette coordinate and the unused flags byte giving way to a texture
coordinate and a layer index):

| Attribute | Format | Bytes | Why |
| --- | --- | ---: | --- |
| Position | 3 x `i16`, compartment-local, 1/1024 m (range +/-32 m, step about 1 mm) | 6 | Every Tern compartment is under 32 m across (the hangar is 21 x 18 m) |
| Mover index, texture layer | 2 x `u8` | 2 | Mover 0 is static; the layer indexes the one texture array (`surface-materials`, at most 256 layers) |
| Normal | `INT_2_10_10_10_REV`, normalized | 4 | Dynamic lights (fire glow, muzzle flash) and light probes |
| Colour, normal state | `RGBA8` | 4 | Alpha reserved for `light-baking` |
| Colour, red alert | `RGBA8` | 4 | |
| Colour, emergency | `RGBA8` | 4 | |
| Texture coordinate | 2 x `i16`, 1/1024 of a layer's span | 4 | World-projected (section 5a), so texel density is the same on every surface of a space; +/-32 spans of 2 m is +/-64 m, more than any compartment |

Indices are 16-bit wherever a compartment's draw holds at most 65,535 vertices (every Tern
compartment: the largest, engineering, is about 8,400 vertices on the first estimate with its
lighting grid) and 32-bit otherwise, chosen
per draw range by `deckc`. OpenGL ES 3.0 allows both; 16-bit halves index memory. One vertex
array object per compartment binds its ranges.

### 5a. Generated detail (2026-10-05)

The owner, 2026-10-05: "look at our fps thing to design better levels, add more geometry to make
things more interesting looking". The fps project, Undercity (`/home/user/fps-game-demo`), does
not dress its rooms by hand: `tools/godot/detailing.py` generates baseboards, cornices, hazard
bands, pilasters and girders from each room's walls and openings, against a written style
checklist (`docs/ut99_reference.md` section 2(e): trim at every floor-wall edge, thick framed
doorways inset from the wall, a structural ceiling never a flat plane, 45 degree chamfers,
oversized pipes and hazard stripes, visible light fixtures). Star Crew takes the shape: **the
detail is a function of the layout**, so a ship of one class reads alike room to room, a moved
wall moves its trims, and nobody models a room twice. Where Undercity's rooms are boxes, ours
are hull-following prisms, so the rules work on any wall direction.

**The rules** (sizes in `data/ships/<id>/detailing.json`; implemented today by
`docs/mockups/lib/shipkit.js` `buildCompartment`, by `deckc` once it is built, and the two must
agree, which `deckc`'s tests check against the same data file):

| Piece | Rule | Tern size | Lineage |
| --- | --- | --- | --- |
| Frames | Ship frames every `spacing_m` along the keel (frame k at z = 2k m). A wall running fore and aft carries a rib at every frame; an athwartship wall carries stiffeners between frames' x positions; a rib that would cross an opening is dropped | 2.0 m; ribs 0.24 x 0.12 m, 0.22 m deep in tall rooms | UT99 pilasters, Undercity's `detailing` pilasters; naval frame numbering |
| Beams | Under the ceiling at every frame across the room, butting the coves, split round a ceiling hatch's collar and a tall system (the reactor) | 0.24 x 0.18 m; 0.38 m deep in rooms over 3.6 m | "the ceiling is structural, never a flat plane" (UT99 checklist 9); Undercity's girders lined up with the pilasters, so wall and ceiling read as one frame |
| Coves | A 45 degree panel between every solid wall and the ceiling; none along an edge open to another brush of the room or where an opening reaches it | 0.35 m; 0.6 m in rooms over 3.6 m | The hull's own chamfered octagon, echoed inside; UT99 "low-poly 45 degree chamfers" |
| Baseboards | Along every solid wall at the floor, cut at ribs and doors (trims stop at the faces they meet) | 0.14 m high, 0.03 m proud; hazard striped in working spaces | UT99 checklist 1, Undercity 7.2 |
| Door frames | Jambs and lintel round every door, wall hatch and pressure door, filling a ring cut round the clear opening, proud on both sides; a glowing status strip on each lintel; pressure doors heavier and hazard striped | section 5 above | UT99 checklist 2 ("thick framed jamb and a lintel, inset from the wall"); Undercity's doorway kit and its status light bar |
| Window frames | Frame and sill round every window | 0.14 m, 0.08 m proud | UT99 "few, small, heavily framed" windows |
| Floor openings | A hazard rim on the floor round every hatch, ladder well, hoist and bay door; a collar under the ceiling round it; rails and rungs up through every ladder well | rim 0.15 m wide, 0.012 m raised | UT99 hazard stripes; Undercity's railed pit |
| Railings | Along every edge where a brush opens onto a lower floor of the same room (the hangar's galleries) | 1.05 m, posts every 1.5 m, kick plate | Undercity's catwalks |
| Platforms (proposed with the bridge variants, `bridge-stations` 11a) | A raised floor that is solid, not air: a dais, a work ring, a sub-platform. Each edge of its footprint names its kind: `wall` (against the room's wall, nothing drawn), `riser` (a step face), `rail` (a step face with a railing, posts spaced evenly, gaps at the stairs) or `step` (a riser with a stair at it). Every edge not against a wall takes a hazard nosing; a stair divides its rise into the whole number of equal steps nearest `tread_rise_m`, and its rails stop clear of the stair | rise 0.225 m; nosing 0.05 m, 1.2 cm up; rail 0.75 m, mid rail 0.375 m (both 1.0 and 0.5 m until the owner's "can we lower the railing by about 25%", 2026-10-07), posts spaced evenly about 1.2 m apart | The references' rails at every level change (`docs/analysis/star-trek-bridges.md`); Undercity's stairs and the UT99 hazard stripe |
| Corridors | Two conduits along the top of each fore-and-aft wall, clear of the ribs; a deck plate runner down the middle | radii 0.06 and 0.04 m, 8 sides; runner 0.9 m | UT99 "pipes, vents and ducts wrapping the room", "8-16 sided cylinders" |
| Lamps | In the bays between frames, never on a beam: round(bay area / 8 m^2) across each bay, at least one per corridor bay, one per pod (beside its hatch if the hatch is central); every third on the emergency bus; high-bay lamps hanging 0.4 m in rooms over 3.6 m | panels 0.9 x 0.45 m, corridors 0.7 x 0.35 m | UT99 checklist 4 ("light fixtures are recessed or bracketed"); Undercity 7.3 ("every light has a visible fixture", lights in the bays between girders); this change's lamp rule, kept at one per 8 m^2 |
| Finishes | Each compartment's `finish` (`crew` or `working`) picks the material of every generated surface (`finishes` in detailing.json): crew spaces tiled floors and steel trims, working spaces diamond plate, machinery ceilings and hazard baseboards | | UT99 "one warm key colour, one cool fill colour, at most one saturated accent" |

**No z-fighting by construction.** Every detail face is either pressed back to back against a
wall, floor or ceiling and not drawn (a rib's back, a beam's top, a lamp housing's top), or
stands at least 1 cm clear of any parallel face (raised plates are 1.2 cm up; status strips
1 cm proud of their lintel). Trims stop at the faces they meet: baseboards are cut at ribs and
doors, runners at floor openings, beams at collars. `deckc`'s z-fighting check (section 6) is
still the authority; the mockups only follow the same construction.

**UVs are world-projected**: a floor or ceiling takes (x, z), every other face its horizontal
tangent and y, divided by the material's span, so a texture runs continuously across segments
of one wall and texel density is the same on every surface of a space (64 px per metre inside,
`surface-materials`).

**What it costs**, measured on the Tern by `node tools/mockups/kit_report.mjs` (2026-10-05;
props not counted): the shell of all 30 compartments is 1,232 triangles before the light
baker's subdivision, the generated detail 15,688, together 16,920, with 306 lamps (110 on the
emergency bus). The busiest compartments are the hangar (1,828), the main corridor (1,236) and
engineering (1,072); baseboards (4,888, cut at every rib), ribs (3,016) and lamp housings
(2,448) are most of it. Against section 11's ceilings every compartment keeps more than three
quarters of its budget for props and the bake's subdivision: the hangar, the busiest, keeps 77 %,
every other compartment at least 84 % (corrected 2026-10-05 from "more than 85 %", which the
hangar and the main corridor do not meet).

### 6. Checks: what `deckc` refuses

Each refusal names the compartment, the ids and the coordinates, and writes nothing.

| Check | Rule | Source |
| --- | --- | --- |
| Layout | Every rule of `ship-layout` (ids, overlaps, portals on shared faces, hull, reachability, points inside) | `reference-ship-tern` spec |
| Brushes | Convex and closed: every vertex of a brush lies on or behind every plane within 0.1 mm; at least 4 planes; finite numbers | Quake brush rule |
| Inside the air | Every detail brush, prop bound and fixture lies inside its compartment's air brushes (1 mm tolerance) and outside every portal's clear opening | CLAUDE.md 8 |
| No z-fighting | Two faces whose planes are within 5 mm of each other, facing the same way (normals within 0.5 degrees), overlapping by more than 1 mm^2, in one compartment: refused. Deliberately parallel surfaces are at least 1 cm apart. Back-to-back faces are allowed. Faces are bucketed by quantized plane, so the check is close to linear in faces | Undercity 7.2 and `detailing.py` (`PLANE_TOL` 5 mm); CLAUDE.md 8 (1 cm) |
| Frames fit | A frame's casing stays at least 1 cm inside the face it is on, or the face is all frame | Undercity 7.2 |
| People stand clear | The crew capsule (radius and height from `crew-on-deck`; until then 0.30 m and 1.80 m standing, 1.30 m seated) placed at every seat, spawn, ladder top and bottom, stair head and foot, and 0.5 m either side of every door threshold, intersects no colliding brush except its own seat's proxy | Undercity 7.4; CLAUDE.md 8 |
| Walkable | A flood fill of capsule-valid floor cells (0.25 m grid) through crew portals and stairs reaches every seat and spawn from every spawn | New: it catches the gap `reference-ship-tern` found (the engineering console cannot be reached from the catwalk as laid out) |
| Clear width | Every crew portal's clear width is at least the capsule's diameter plus 0.1 m (0.7 m) | Undercity 7.4 |
| Budgets | Triangles and draw calls per compartment under its ceiling (section 11); the worst visible set under the interior pass ceiling | `engine-stack` table |
| Lights | Every light fixture has a visible fixture brush or prop (Undercity 7.3) and a colour for each of the three states | CLAUDE.md 11 |
| Layout hash | The SHA-256 of the layout's canonical JSON is written into the deck; the loader refuses a deck whose hash differs from the layout the server runs | CLAUDE.md 6.6 "validate the real artifact" |

**The worst visible set.** For each compartment, `deckc` finds every compartment visible
through a sequence of open portals (a conservative portal-flow pass in the shape of Quake's
`vis`, run offline over about 30 compartments and 40 portals) and sums their triangles. A
PVS is computed but not shipped: it is a budget check, not a culling structure (section 9).
Its upper bound on the Tern (every neighbour and every neighbour's neighbour drawn whole, no
narrowing) peaks at 31,838 triangles from the main corridor on the v2 plan before the bake's
subdivision (section 11, 2026-10-05; 53,184 on the first estimate, whose lighting grid stood in
for the subdivision), under the 80,000 ceiling.

### 7. Light: fixtures as data, the bake as a step

`light-baking` decides how light is computed. This change decides where it comes from, where
it is stored and how it is blended.

- **Fixtures are entities.** A light fixture has a position, a kind (ceiling panel, strip,
  wall lamp, console glow, emergency lamp), a colour and intensity for each of the three
  states, a flag saying it is on the emergency bus, its compartment, and the fixture brush or
  prop that shows it. `deckgen` places one per the kit rule (306 on the Tern's v2 plan, 110 of
  them on the emergency bus, by `kit_report.mjs` on 2026-10-05; 272 on the v1 boxes); a detail
  file adds or replaces fixtures in hero rooms. The layout holds no lights: it is the plan.
- **The bake is step 4 of the compile.** `deckc` hands `light-baking` each compartment's faces,
  its colliding and detail brushes (occluders), its fixtures and emissive faces, and the three
  states; it gets back the tessellated faces and three `RGBA8` colours per vertex. Triangle
  counts in the report are counted after the baker's subdivision. The estimate in section 11
  assumes a uniform grid, 1 m in compartments up to 3.6 m tall and 2 m in the hangar and
  engineering, as a stand-in for the baker's adaptive subdivision.
- **Each compartment blends its own state.** A uniform block per compartment holds the state
  weights `w = (normal, red alert, emergency)` (summing to 1, eased over 0.5 s by the client), a
  dimmer from 0 to 1 (the simulated voltage of the compartment's lighting bus from
  `power-grid`, so a brownout dims the lights the console says are browning out), a damage
  flicker, the movers' transforms and up to four dynamic lights. The vertex shader computes
  `colour = (c0 * w.x + c1 * w.y + c2 * w.z) * dimmer + dynamic(normal, position)`: nine
  multiply-adds and the dynamic terms per vertex.
- **Which state a compartment is in** is simulation state from `sc-core`: red alert is the
  ship's alert level; emergency is the compartment's lighting bus running on the emergency bus
  or batteries. A dark compartment (bus dead) has `dimmer = 0`, leaving only emissive faces
  and dynamic lights.
- **If `light-baking` stores some light another way.** The vertex keeps its three colour sets
  (CLAUDE.md 9 makes baked vertex colour the default). Whatever else the baker produces travels
  in optional chunks of the `.deck` file (section 8), so a deck baked without them loads
  unchanged: `UV2` and `LMAP` (a second texture coordinate stream and one lightmap atlas per
  lighting state, only for the compartments `light-baking` lightmaps) and `PROB` (light probes:
  an ambient cube of six colours per state at each probe point, for crew, craft and props that
  move). Which compartments use them, the texel size and the probe spacing are `light-baking`'s
  decisions; their memory comes out of the 96 MB texture budget, and `deckc --report` prints
  it per compartment.
- **The mockup draws this way.** `docs/mockups/deck-plan.html` puts every static surface of
  the Tern through one shader of this shape (three colour sets per vertex, a weight vector and
  a dimmer per compartment in a uniform array, one draw per deck slice), with lamp fixtures
  placed by the kit rule (306 on the v2 plan; 272 on the v1 boxes) and a direct-light stand-in
  for the bake (no shadows or bounce; `light-baking`'s `lighting.html` shows the real method).
  Its normal, red-alert and emergency buttons change only the weights, never the geometry.

### 8. The `.deck` file

Little endian throughout; every offset is from the start of the file; every chunk starts on a
16-byte boundary. Format version 1.0.

**Header** (64 bytes):

| Offset | Field | Type |
| ---: | --- | --- |
| 0 | Magic `SCDK` | 4 bytes |
| 4 | Format major, minor | `u16`, `u16` |
| 8 | Header size (64) | `u32` |
| 12 | File size | `u32` |
| 16 | Chunk count | `u32` |
| 20 | Flags (bit 0: render chunks present) | `u32` |
| 24 | SHA-256 of the layout's canonical JSON | 32 bytes |
| 56 | CRC-32 of the header with this field zeroed | `u32` |
| 60 | Reserved, zero | `u32` |

**Chunk table**, after the header: per chunk a four-character code, offset, size and CRC-32
(IEEE) of the payload, 16 bytes each. A loader refuses a different major version, a header or
chunk whose CRC fails, an unknown chunk code it is told is required, an offset outside the
file, and any non-finite float. It skips an unknown optional chunk (minor versions add
chunks).

| Code | Read by | Holds | Tern size (estimate) |
| --- | --- | --- | ---: |
| `META` | both | Ship id, compiler and kit versions, units, axes | under 1 KB |
| `STRS` | both | String table (ids) | 3 KB |
| `CMPT` | both | Per compartment: layout index and id, POI, kind, deck mask, bounds, local origin, brush range, k-d range, entity range, draw range | 3 KB |
| `PRTL` | both | Per portal: layout index and id, kind, flags (see-through when closed, crew passable, airtight), the two compartment indices (space is `0xFFFF`), plane, clear-opening polygon, viewport polygon if any | 5 KB |
| `BRSH`, `PLNS` | both | Colliding brushes (bounds, plane range, contents) and their planes (`f32` x 4) | 115 KB |
| `BKDT` | both | k-d tree nodes and leaf brush lists | 2 KB |
| `ENTS` | both | Seats, spawns, ladders, stairs, door movers, light fixtures, system anchors, with poses and the layout ids they belong to | 20 KB |
| `VBUF` | client | Vertices, 28 bytes each (about 70,300 for the Tern, sized on section 11's first estimate, whose lighting grid stands in for the bake's subdivision) | 1.97 MB |
| `IBUF` | client | Indices, 16-bit where allowed (about 175,700) | 0.35 MB |
| `DRAW` | client | Per draw: compartment, pass, index format, vertex offset, index range, bounds | 3 KB |
| `MOVR` | client | Movers: hinge or slide axis, open offset, open time, the portal or system that drives them | 4 KB |
| `LGHT` | client | Fixture records for dynamic relighting and the debug view | 17 KB |
| `TBSP` | client | Glass BSP nodes per compartment that needs one | under 1 KB |
| `UV2` | client, optional | Lightmap coordinates (2 x `u16`, normalized) for the vertices of lightmapped compartments, a parallel stream so the 28-byte vertex never changes | 0 (none planned) |
| `LMAP` | client, optional | Lightmap atlases, one per lighting state, if `light-baking` adopts them for a compartment | 0 (none planned) |
| `PROB` | client, optional | Light probes: position and a six-colour ambient cube per state (`RGBA8`) | about 11 KB at one probe per 16 m^2 of floor (estimate; the spacing is `light-baking`'s) |
| | | **Total** | **about 2.5 MB** |

The compartment graph itself is built by `sc-core` from the layout; `CMPT` and `PRTL` carry
render and collision data keyed to the layout's indices and ids, and the loader checks every
id against the graph. Nothing keeps a second room list (CLAUDE.md 7).

### 9. Rendering a frame: portal culling

```
start = the compartment holding the eye (tracked as crew-on-deck moves the avatar;
        a brush lookup on load or teleport)
visit(start, full screen rectangle, depth 0)

visit(c, rect, depth):
    record c with scissor = rect (grow the record's rectangle if c is reached again)
    for each portal p of c, except the one we came through:
        if p is closed and not see-through: skip       (door state from the snapshot)
        if the eye is behind p's plane: skip
        r = screen bounds of p's opening (clipped to the near plane; the whole screen
            if the near plane cuts the opening) intersected with rect
        if r is empty: skip
        if p leads to space (window, open bay door, open outer airlock door):
            add r to the exterior rectangles; skip
        if depth < 8: visit(other side of p, r, depth + 1)
```

- **Doors.** A door portal is open for visibility when its leaves are more than 5 % open,
  read from the core's snapshot (the server owns every door). A closed door with a viewport
  (proposed for pressure doors, K4) uses the viewport's smaller polygon.
- **Cost.** A portal test is four corner projections and a rectangle intersection, about 150
  floating point operations. The Tern's worst case is under 60 portal tests a frame: under
  10,000 operations, well under 0.05 ms on a Cortex-A76 (estimate, to measure).
- **Drawing.** Compartments are drawn in visit order (near to far) with `glScissor` set to
  their rectangle, which bounds fill rate as well as triangles. Then the transparent pass back
  to front.
- **Windows and open bay doors** produce scissor rectangles for `ship-frames`' exterior far and
  near layers, which draw before the depth clear and the interior pass (`ship-frames`'
  composition). The traversal therefore runs first, on the CPU, before any exterior drawing.
  The viewscreen is not a portal: it is an emissive face whose texture is `ship-frames`'
  render target.

**Why no precomputed PVS.** Quake needed one because its levels split into thousands of BSP
leaves and walking portals between them every frame was too slow for 1996 hardware. A ship of
30 compartments and 40 portals walks its graph in microseconds, and its doors open and close
in play, which a static PVS cannot know (Quake II added area portals on top of the PVS for
exactly that, to verify). Doom 3 dropped the PVS and walks areas and portals at run time; that
is the shape here. `deckc` still computes a visible set offline, as a budget check (section 6).

**Where a BSP still helps.** Two places, both small and offline-built:

1. **Collision inside a large compartment**: the k-d tree of section 4 (an axis-aligned BSP).
2. **Sorting glass**: a compartment with more than two transparent polygons that can overlap on
   screen gets a BSP of just those polygons, walked back to front from the eye, the classic
   painter's order. The Tern's bridge (two windows) sorts by distance and needs none.

The depth buffer handles opaque geometry; there is no BSP over a compartment's opaque faces.

**Honest note on the Pi 5.** The whole Tern is 35,180 triangles in 91 draws before the bake's
subdivision on the v2 plan, and was 58,573 on the first estimate with its lighting grid (section
11, 2026-10-05), which a Pi 5 could likely draw every frame without culling (to measure with
`sc-probe`).
Portal culling stays because it is nearly free, the graph exists for the simulation anyway,
and it bounds overdraw at 1280 x 720, leaves headroom for crew, craft, the exterior and the
viewscreen, and scales to bigger ships and to two ships docked.

### 10. Run-time memory and loading

- The client loads the whole `.deck` (2.5 MB for the Tern), uploads `VBUF` and `IBUF` to one
  vertex buffer and one index buffer (2.3 MB of the 64 MB budget), and frees the CPU copy.
- The server reads only the core chunks (about 0.15 MB) and keeps them.
- Loading is a read, a CRC pass and two uploads: no parsing of JSON, no tessellation, no bake.

### 11. Budgets: every Tern compartment

The ceilings are `engine-stack`'s: the bridge 30,000 triangles; every other compartment 8,000.

**Re-stated on the v2 plan, 2026-10-05** (the rooms follow the hull, `reference-ship-tern`
section 1a). Volume and floor are `layout_check.py`'s; "Wall" is the solid wall net of every
opening, measured over the brushes by the scratch instrument of section 3 (it gives the first
estimate's 3,485.2 m^2 on the v1 boxes exactly). **Shell and Detail are measured**, by
`node tools/mockups/kit_report.mjs` from the brushes and section 5a's rules: "Shell" is the
floor, walls and ceiling before the light baker's subdivision; "Detail" is ribs, beams, coves,
baseboards, door and window frames, railings, conduits, ladders and lamps. **Props are the first
estimate's, unchanged**: stations, systems, fixtures, frames, ladders and the stairs at section
5's costs. They still count door frames and ladders, which the kit now generates too (1,618 of
its detail triangles: `frame`, `frame_hazard`, `window_frame` and `ladder` in its report), so
those are counted twice, on the safe side. Draws are the passes of section 5 a compartment
needs. The first estimate (v1 boxes, 2026-10-04) tessellated the shell to the lighting grid of
section 7 as a stand-in for the baker's subdivision; its totals are kept below for comparison,
and its per-compartment volumes and floors are `reference-ship-tern` sections 2-4's "was"
figures.

| POI | Compartment | Decks | Volume m3 | Floor m2 | Wall m2 | Portals | Shell (kit_report) | Detail (kit_report) | Props (first estimate) | Total | Ceiling | Use | Draws |
| ---: | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | Bridge | A | 608.6 | 173.9 | 167.6 | 3 | 60 | 718 | 3,116 | 3,894 | 30,000 | 13 % | 4 |
| 2 | Captain's ready room | A | 143.9 | 48.0 | 80.5 | 1 | 30 | 320 | 36 | 386 | 8,000 | 5 % | 3 |
| 3 | Computer core | A | 143.9 | 48.0 | 80.5 | 1 | 30 | 320 | 456 | 806 | 8,000 | 10 % | 3 |
| 4 | Command passage | A | 120.0 | 40.0 | 100.3 | 5 | 42 | 690 | 184 | 916 | 8,000 | 11 % | 3 |
| 5 | Dorsal turret access | A | 45.8 | 15.3 | 38.5 | 3 | 58 | 320 | 112 | 490 | 8,000 | 6 % | 3 |
| 6 | Aft passage | A | 135.0 | 45.0 | 117.7 | 2 | 28 | 612 | 72 | 712 | 8,000 | 9 % | 3 |
| 7 | Torpedo room | B | 354.6 | 118.2 | 131.9 | 2 | 42 | 596 | 596 | 1,234 | 8,000 | 15 % | 3 |
| 8 | Medbay | B | 187.2 | 62.4 | 92.5 | 1 | 24 | 374 | 316 | 714 | 8,000 | 9 % | 3 |
| 9 | Damage control | B | 187.2 | 62.4 | 92.5 | 1 | 24 | 374 | 296 | 694 | 8,000 | 9 % | 3 |
| 10 | Crew quarters | B | 271.5 | 90.5 | 112.3 | 1 | 24 | 452 | 36 | 512 | 8,000 | 6 % | 3 |
| 11 | Mess | B | 271.5 | 90.5 | 110.8 | 1 | 24 | 452 | 36 | 512 | 8,000 | 6 % | 3 |
| 12 | Port turret access | B | 233.0 | 77.7 | 101.6 | 2 | 36 | 414 | 76 | 526 | 8,000 | 7 % | 3 |
| 13 | Starboard turret access | B | 233.0 | 77.7 | 101.6 | 2 | 36 | 414 | 76 | 526 | 8,000 | 7 % | 3 |
| 14 | Main corridor | B | 195.0 | 65.0 | 148.1 | 10 | 60 | 1,176 | 368 | 1,604 | 8,000 | 20 % | 3 |
| 15 | Hangar | C+B | 1,682.4 | 350.8 | 400.6 | 8 | 110 | 1,718 | 1,412 | 3,240 | 8,000 | 41 % | 3 |
| 16 | Port launch bay | C | 190.1 | 63.4 | 98.0 | 2 | 42 | 430 | 284 | 756 | 8,000 | 9 % | 3 |
| 17 | Starboard launch bay | C | 190.1 | 63.4 | 98.0 | 2 | 42 | 430 | 284 | 756 | 8,000 | 9 % | 3 |
| 18 | Engineering | C+B+A | 2,538.0 | 253.8 | 601.9 | 5 | 66 | 1,006 | 4,084 | 5,156 | 8,000 | 64 % | 3 |
| 19 | Drive section | B | 307.2 | 102.4 | 120.9 | 1 | 36 | 568 | 1,240 | 1,844 | 8,000 | 23 % | 3 |
| 20 | Lower corridor | C | 150.0 | 50.0 | 114.7 | 8 | 54 | 1,004 | 296 | 1,354 | 8,000 | 17 % | 3 |
| 21 | Magazine | C | 501.1 | 167.0 | 147.1 | 2 | 42 | 718 | 676 | 1,436 | 8,000 | 18 % | 3 |
| 22 | Life support | C | 367.8 | 122.6 | 132.3 | 1 | 36 | 592 | 1,136 | 1,764 | 8,000 | 22 % | 3 |
| 23 | Cargo and stores | C | 349.1 | 116.4 | 137.5 | 2 | 70 | 672 | 504 | 1,246 | 8,000 | 16 % | 3 |
| 24 | Airlock | C | 26.4 | 8.8 | 32.8 | 2 | 28 | 174 | 136 | 338 | 8,000 | 4 % | 3 |
| 25 | Shield generator | C | 167.8 | 55.9 | 88.8 | 1 | 30 | 352 | 636 | 1,018 | 8,000 | 13 % | 3 |
| 26 | Forward switchboard | C | 167.8 | 55.9 | 88.8 | 1 | 30 | 352 | 596 | 978 | 8,000 | 12 % | 3 |
| 27 | Dorsal turret pod | A | 12.9 | 5.2 | 20.7 | 1 | 34 | 114 | 300 | 448 | 8,000 | 6 % | 3 |
| 28 | Ventral turret pod | C | 18.1 | 5.2 | 29.0 | 1 | 34 | 114 | 300 | 448 | 8,000 | 6 % | 3 |
| 29 | Port turret pod | B | 12.9 | 5.2 | 19.5 | 1 | 30 | 106 | 300 | 436 | 8,000 | 5 % | 3 |
| 30 | Starboard turret pod | B | 12.9 | 5.2 | 19.5 | 1 | 30 | 106 | 300 | 436 | 8,000 | 5 % | 3 |
| | **Tern** | | 9,824.8 | 2,445.5 | 3,626.6 | 80 sides | 1,232 | 15,688 | 18,260 | **35,180** | 262,000 | 13 % | 91 |

First estimate on the v1 boxes, for comparison: 8,858.8 m^3, 2,135.0 m^2 of floor, 3,485.2 m^2 of
wall; shell 16,936 (tessellated to the lighting grid), detail 23,377 (estimated densities), props
18,260: 58,573 triangles, 22 % of 262,000, 91 draws.

**Detail re-measured, 2026-10-06** (`ship-props` section 4a). The kit now leaves out the end caps that skirting
and beams pressed into a wall (they fought the next room's wall), so `kit_report` gives 14,996 detail triangles
on today's layout where this table was made at 15,688. The table's Detail and Total columns stay as an upper
bound until it is re-measured whole; no compartment's use rises.

**What the table says.**

- **Every compartment is under its ceiling; none is exceeded.** The busiest is engineering at
  64 % (5,156: the reactor, the mezzanine and the stairs are most of its props), then the hangar
  at 41 %; the bridge is at 13 % of its 30,000 (3,894, leaving 26,106 for the subdivision and for
  hand detail it will not need at this style).
- **The whole ship is 35,180 triangles in 91 draws before the bake's subdivision.** That is not a
  saving on the first estimate's 58,573: the first estimate drew every surface at a 1 m lighting
  grid (2 m in the hangar and engineering), 16,936 triangles of shell, where the kit counts the
  shell untessellated (1,232); and its detail was a density guess (23,377), where section 5a's
  rules measure 15,688. The subdivision `light-baking` adopts is paid from the headroom: at least
  2,844 triangles in every compartment (engineering's; its grid shell in the first estimate was
  908).
- **Memory**: section 8's vertex and index sizes (1.97 MB and 0.35 MB, 2.3 MB of the 64 MB
  budget) were sized on the first estimate and stay the conservative figure until
  `deckc --report` prints the compiled truth.
- **The interior pass ceiling is 80,000 triangles and 120 draws per frame** (proposed, K2):
  40 % of the 200,000-triangle and 300-draw frame, leaving the rest for the exterior layers,
  the viewscreen, crew, craft, glass and UI (their owners' budgets).

**Worst visible sets** (upper bounds with every door open and no narrowing; v2 plan, before the
bake's subdivision, by the same neighbour sums that give the first estimate's figures):

| From | Itself | With its neighbours | With neighbours' neighbours | Draws (depth 2) |
| --- | ---: | ---: | ---: | ---: |
| Main corridor (14) | 1,604 | 11,832 | 31,838 | 79 |
| Hangar (15) | 3,240 | 12,866 | 27,946 | 66 |
| Lower corridor (20) | 1,354 | 13,088 | 25,728 | 63 |
| Command passage (4) | 916 | 8,096 | 18,568 | 52 |
| Engineering (18) | 5,156 | 10,952 | 15,912 | 27 |
| Bridge (1) | 3,894 | 4,810 | 8,096 | 19 |

Even the crudest bound fits the interior ceiling, with 48,162 triangles to spare for the
subdivision in the worst set (the first estimate, with its lighting grid, put the main
corridor's bound at 53,184 and the bridge's at 12,628); in play, closed doors and narrowing cut
it to the compartment and the one or two seen through open doors.

### 12. What measuring the Tern taught

1. **Partitions have no thickness in the layout** (section 3, K1).
2. **Inside a multi-level compartment the layout says nothing about how to walk between
   levels.** As laid out, the engineering console on the mezzanine cannot be reached from the
   catwalk or the lower floor, and the hangar's galleries do not meet the landing. The walk
   check (section 6) would refuse this deck; `reference-ship-tern` proposes the stairs and the
   gallery extension (T1, T2).
3. **Systems are points.** The layout gives a system's centre but not its size, so the
   people-stand-clear and inside-air checks cannot test a scrubber or a pump.
   `reference-ship-tern` proposes a `size_m` per system (T4); until then `deckc` takes each
   system prop's bounds from its mesh.
4. **Lighting dominates the shell.** At a 1 m grid the shell was 29 % of the ship's triangles
   (first estimate, v1 boxes); untessellated, the v2 shell is 1,232 triangles, 3.5 % of the
   35,180 (section 11, 2026-10-05), so nearly all of a shell's cost is the bake's subdivision.
   `light-baking`'s adaptive subdivision can keep it low where light is even.

### 13. The first deck in the engine: the kit's own output (2026-10-07)

The owner, 2026-10-07: "just build it, we'll worry about performance later. is the entire level in?"
It was not: the engine drew a test room. **Decision (the owner's "just build it"):** the engine draws
the whole Tern now, from the geometry the mockups already build, rather than after a Rust `deckc`
reimplements the kit. One implementation of the deck rules stays the rule (CLAUDE.md 6.1): the kit
(`docs/mockups/lib/shipkit.js`) is that implementation today, so the engine takes its output instead of
a second copy of it.

| Step | Tool | What it does |
| --- | --- | --- |
| Export | `node tools/deck/export_deck.mjs` | Opens the deck plan headless, waits until every room is lit (from the bake cache), and writes `build/deck/<ship>/`: each compartment's triangles in ship coordinates, normals, texture coordinates, layers, the three state colours (the bake, tints included), and the texture arrays |
| Compile | `cargo run --release -p sc-tools -- deckc` | Packs every vertex with `sc-core`'s one packer into `compiled/<ship>.deck` (`sc-core::deck`): positions relative to each compartment's centre, texture coordinates shifted by whole spans, colours as display multipliers, identical vertices merged behind 32-bit indices, the prop atlases resampled into the one texture array, a mip chain |
| Draw | `sc-client` | Loads the deck, draws every compartment through the deck pipeline with the camera subtracted in `f64` (the frames rule), the panel layers' emission masks glowing by `panels.json` `glow` |

**What it is, measured** (`compiled/tern.deck`): 37 compartments, 160,247 triangles, 195,813 vertices
(5.5 MB), 142 texture layers of 256 px with 9 mips (49.6 MB), 57 MB in all. Every compartment is
drawn every frame: one draw each, no portal culling yet, and 256 px layers where the budget plans
128 (section 8, `surface-materials`). The owner: performance later.

**What it is not yet:** the `.deck` file of section 8 (it is a simpler first format, version 1, without
chunks, collision, portals, probes or CRCs); collision and walking (the client flies); portal culling;
the console faces (the screens' atlas is a separate texture in the deck plan, not exported yet); the
viewscreen and the windows (the deck plan paints space into them; here they are dark); doors and
movers. The export needs Node with Playwright and the deck plan; a Pi draws the compiled deck, it does
not compile it. `build/` and `compiled/` are build output and are not committed.

**When the Rust `deckc` of sections 2-8 is built**, it replaces the export step and this section's
format, and the kit's rules move into it with tests; until then a change to the kit reaches the
engine by re-running the two commands.

### 13a. The first walk in the engine (2026-10-08)

The owner, after the deck plan's second walk: "back to building the engine". The client flies; the next
step is to walk the Tern in it as the deck plan does. The same decision as section 13: the engine takes the
walk world the deck plan already builds, rather than a second copy of its rules.

**The walk world, exported.** The deck plan's walk collides against one triangle soup built by its own
rules (`deck-plan.html`: `collisionSoups`, `stairRamps`, `walkCovers`, `propBoxes`): every room's shell
without its ceilings and trims, every stair as a ramp, covers over the openings a body must not fall
through, props as boxes, round machines as prisms, chairs as pedestals, the Petrel as its own triangles.
`MOCKUP_EXPORT_DECK` exports that soup and the walk's entities beside the render meshes:

| Entity | Fields | From |
| --- | --- | --- |
| Start | the floor point and facing a walk starts at (the bridge, by its door) | `walkStart` |
| Ladders and floor hatches | x, z, the lower and upper floor | `walkLadders` |
| Wall hatches | x, z, the wall's normal, the sill | `walkHatches` |
| Doors | centre, normal, width, height, kind, portal id | `walkDoors` |
| The lift | footprint, stops, the car's floor | `walkLifts` |

**The file.** `compiled/<ship>.deck` goes to version 2: the index gains `walk`, the triangle soup in ship
coordinates (`f32`, the ship is under 100 m across, so the frames rule is kept) and the entities as JSON, the
triangles' range checked like every other blob. A version 1 file is refused with the command that rebuilds it.

**The body: `sc-core::walk`.** Rapier's kinematic character controller (`rapier3d`, pure Rust, no GPU), as
`crew-on-deck` section 3a decided, over one static triangle mesh of the soup. One step of `walk::step`
takes the input (move, run, jump, use) and a fixed time step and does what `shipwalk.js` does in its
Rapier path: speed up at 20 m/s^2 and brake at 30, walk 1.8 m/s and run 4.0, 70 % backwards, climb steps and
ramps (autostep 0.35 m, snap 0.4 m, slopes to 62 degrees), slide along walls, fall at 9.81 m/s^2, jump at
3.0 m/s, climb a ladder or go through a hatch with Use. The values are **`data/crew/walk.json`**, units in
the keys, loaded with `deny_unknown_fields`; the deck plan's `shipwalk.js` reads the same file (inlined),
so the mockup and the engine cannot walk differently by their numbers. The eye follows the feet smoothed
(0.08 s, never more than 0.25 m behind) in the client, as the camera is the client's.

**What this step leaves out**, each for its own change: doors stand open (the engine draws no leaves yet,
so they are not walls either); the lift's car stays at deck A where it is drawn, and the shaft is walled at
every landing it is not at; no other bodies, no network, no prediction (`crew-on-deck` 4.2); the console
faces and windows (7.5).

**The client.** `sc-client` starts on its feet on the bridge, by its door. F toggles flying; E uses; Space
jumps; Shift runs. Headless shots walk a scripted path (the bridge, the stair down, the corridor) and
capture along it, so a shot proves the body got there.

**Tests** (`sc-core`, on small built meshes, not the Tern): a body reaches walking speed in about a tenth
of a second and stops as quickly; a wall stops it and it slides along; it walks up a 41 degree ramp and a
0.3 m step; it falls off a ledge and lands; a jump rises about 0.46 m; a ladder takes it to the floor above.
On the real deck, the client's headless run walks its path and fails if the body ends anywhere but where
the path ends.

**Built (2026-10-08).** `compiled/tern.deck` version 2: 173,655 render triangles and a walk world of 93,393
(93,277 after the degenerate ones are dropped), 62.2 MB in all. The client builds the walk world in 43 ms
(llvmpipe container, release build). `sc-core` has eight tests of the body (speed, braking, a wall, a ramp and a
0.3 m step, a ledge, a jump, a ladder, a trunk of two ladders, a lift landing); `sc-client --headless --walk-test`
walks the Tern from the bridge down both ladders to deck C's corridor and arrives within 3 cm of the route's end.
Shots: `docs/screenshots/engine/walk-*.png`.

**Found on the way.**
- Rapier 0.36's own autostep never steps. It first casts the body straight up with the skin gap as its
  tolerance, and a body standing against a riser at that gap always counts as blocked. `sc-core::walk` steps up
  itself: a grounded body stopped by a wall is lifted by the step height, carried a step's depth onto the ledge if
  there is room, and set down there, never into anything.
- Where two ladders meet (the trunk at z 11 runs A to B and B to C), Use took whichever came first, back up the
  way the body came. Now E prefers the way up and Q the way down, in the engine and in the deck plan.

**The Pi 5 budget.** The soup is the deck plan's walk world, 93,393 triangles, which Rapier holds with its
bounding volume tree. Measured (2026-10-08, the owner: "you can try testing it in your env to see how much memory
is used") in the cloud container, x86-64, release build, llvmpipe:

| What | Measured | Against the budget table (`engine-stack` section 5) |
| --- | ---: | --- |
| The walk world on the heap, built (`sc-tools walk-report`, a counting allocator) | 12.7 MB | 3 % of the client's 384 MB |
| The most on the heap while it is built | 22.8 MB | For under a tenth of a second at load |
| Building it | 51 ms | Load time only |
| A body's 1/60 s step, two substeps (3,600 steps, a minute walking and running round the start) | mean 0.26 ms, median 0.14, 99th 1.3, worst 2.3 | Eight bodies on the server: about 12 % of one core here |
| The heap while walking | 32 KB more in a minute | No allocation per step to speak of |
| The deck file read whole at load | 62.2 MB | Freed after the upload; the load's peak |
| The whole client walking the Tern (peak resident set, `/proc` sampled) | 277 MB at load, 249 MB walking | Includes llvmpipe's copies of the GPU data (textures 50.7 MB, meshes 6 MB) and Mesa's JIT, which a Pi keeps in CMA and its driver; not a Pi number |

So the walk is not where the client's memory goes: the textures are (256 px layers where the budget plans 128,
section 13). Reading the deck file in pieces would take the 62 MB peak at load away; that and the Pi's own numbers
are the probe's (`engine-stack` 2.4), run on the hardware with the same `sc-tools walk-report`. A body's step is one character-controller query a substep at 120 Hz. One body in the client
today; the server will run every body (about eight). Memory and step time are reported by the client at
load and in the headless run; the budget table gains a walk row when the probe has measured it.

### 13b. The lift, the screens and the outside, in the engine (2026-10-08)

The owner, playing the browser build: "fix all screens", "the view screen blank, needs a planet and space in the
exterior, maybe space dock", "elevator not functioning, theres also a metal bar in the back too", "I need elevator
buttons to go up and down!", and "we're no longer messing with the JS stuff, we're all in real life GLES web
assembly now!". Everything below is built in the engine; the deck plan's kit stays only as the generator of the deck's
geometry until `deckc` grows its own (task 2.x), and changes there are limited to what the export must carry.

**What was wrong.** The exported lift room held its car, fixed at deck A, so the engine's car never moved and the
other decks saw an empty shaft. The kit put the ship's frames (a rib on the shaft's back wall, a beam under its top) in
the lift's shaft as in any room: that was the bar. The console faces, the starfield behind the windows and the
viewscreen were separate meshes in the mockup and never reached the export; the windows were holes onto the clear
colour.

**The lift.**
- The export carries the shaft without its car and the car as its own room, flagged as mover 1; `deckc` writes the
  mover into the compartment and into each of its vertices (`sc-core::vertex`, the byte reserved for this). As built,
  the car is one compartment and one draw, so the client moves it by that draw's translation (the car's height less
  the height it was exported at) and the shader is unchanged; the vertex byte stays for movers that will share a draw
  (door leaves). The shaft keeps no frames (the kit: no ribs or beams in a lift's compartment).
- `sc-core::lift` owns the car: idle at a stop, its doors closing, moving at the fixture's speed (1.5 m/s), its doors
  opening (2.0 s each way, `deck_access.json`). One rule for every caller, a player's key or a test's.
- **Buttons.** In the car, E sends it one deck up and Q one deck down; at a landing, standing within 1.8 m of its door
  (`walk.json` `lift_call_m`), E calls it. A line at the bottom of the screen names what E and Q do there (with the
  UI layer, `lobby` 3; until then the browser page's key strip says it).
- The walk: a landing's door is a wall unless the car stands there with its doors open; in the shaft the feet ride the
  car's floor (13a), now wherever the car is.

**The outside.** The ship lies in a space dock in orbit. A sky pass draws first, every frame, from the camera's
direction: stars (a hashed field, 3 sizes), the sun, and a planet below the ship to port (a sphere lit by the sun with
bands of cloud and an atmosphere's rim). Then the dock: a frame of girders round the hull, ring frames every 20 m with
work lights. As built (`data/space/exterior.json`, `sc-tools` `dock.rs`): 120 x 52 x 40 m, because a bay must fit the
deck vertex's +/-32 m about its centre; six bays, each a compartment of the deck `outside` in the deck file, textured
with the hull's dark material and lit by the sun when `deckc` builds them, 1,392 triangles in six draws. The rooms draw over
both, so the outside shows only through the windows' openings.

**The viewscreen** shows a camera on the bow looking forward (as built: turned a little to port and down,
`exterior.json` `viewscreen.look`, so the planet's limb is in the corner): the same sky and dock drawn into a 512 x 256 target each
frame, then onto the viewscreen fixtures (`layout.json` fixtures of kind `viewscreen`, which the export now carries)
as a quad 3 cm in front of each, with a faint scan line. Cost: one more pass of the sky and the dock at 512 x 256, and
two triangles a viewscreen.

**The console screens.** The export carries each room's console faces (the stations' console images on their screens,
`bridge-stations` 11.6) and the screens' atlas; `deckc` appends the atlas to the texture array as 256 px layers (about
8) after the panel layers, so a face glows by the alpha mask as the panels do, in the room's one draw.

**The Pi 5 budget.** Sky: one full-screen triangle a frame at 1280 x 720 and one at 512 x 256, a few dozen ALU
operations a pixel; dock: about 1,500 triangles twice; viewscreen and faces: a few hundred triangles and about 8 texture
layers (0.7 MB with mips). Not measured: a cloud session renders on a CPU (CLAUDE.md 2).

## Risks / Trade-offs

- **The kit can make every room look the same.** Mitigation: hero detail files for the bridge
  and engineering, a kit variant per compartment kind (bay, crawlspace, corridor), and the
  palette atlas's role colours per department.
- **Blender convexity is easy to break.** Mitigation: the export refuses non-convex objects
  with their names, and `deckc` re-checks every brush.
- **Zero-thickness partitions look thin at an opening.** Mitigation: every opening has a
  proud frame; a window or glass partition gets a frame on both sides.
- **The estimate's densities are guesses.** Mitigation: `deckc --report` prints the real table;
  the ceilings, not the estimate, are what the compiler enforces.
- **Two layout checkers until `sc-core` exists** (`layout_check.py` and the core module).
  Mitigation: task 1.3 retires or wraps the Python copy in the same commit that adds the core
  one.
- **The Pi 5 numbers are provisional.** Mitigation: `sc-probe` measures them; the ceilings move
  in `engine-stack`'s table and nowhere else.

## Open questions

Questions go to the owner only with something to look at (CLAUDE.md 13). Rows marked
"recommendation taken" have no picture to judge and proceed on the recommendation.

| Id | Question and fact | Options | Recommendation | Mockup shot |
| --- | --- | --- | --- | --- |
| K1 | How thick are walls between compartments? The layout's air brushes touch (1,849.9 m^2 of shared wall, counting each side); insetting costs 231 m^3 (2.4 %) and 11 % of each corridor (v2 plan, 2026-10-05; was 1,667 m^2 and 208 m^3). The launch bays' ceilings, which dropped from 3.5 m to 3.0 m under the gallery slab on the v1 boxes, are 3.0 m in the layout since v2 | a. zero-thickness partitions with proud door frames; b. inset half a wall; c. move the brushes apart | a; keep the core's volumes as the layout's brushes | `deck-plan-deck-B-plan` (partitions drawn as single lines) |
| K2 | The interior pass ceiling per frame | 60,000 / 80,000 / 100,000 triangles (120 draws) | 80,000 and 120 draws: the Tern's crudest bound is 31,838 before the bake's subdivision on the v2 plan (53,184 on the first estimate with its lighting grid; section 11, 2026-10-05) | `deck-plan-overview` (the meter shows the whole ship at once on the v2 page: 21,910 triangles in 73 draw calls, ceilings cut away, labels included, the kit's generated detail in, no light-bake subdivision; the v1 page drew about 47,800 in 43, because its lighting grid subdivided every room) |
| K3 | Hero detail authoring | a. Blender places convex detail, props and lights, exported to a detail file; b. TrenchBroom `.map` for detail only; c. generator only | a | `deck-plan-engineering-closeup` |
| K4 | Do closed pressure doors keep a viewport (0.3 x 0.4 m) that visibility and the crew can see through? | yes / no | yes: flight operations can see into a launch bay before opening it | `deck-plan-deck-C-plan` (pressure doors in amber) |
| K5 | `deckc` in Rust in `sc-tools`, sharing the format module with the loader, or in Python beside the other tools | Rust / Python | Rust: the writer and the reader share one definition of the file. Recommendation taken (ask only with screenshots) | none |
| K6 | Index width | 16-bit where a draw allows it, else 32-bit / always 32-bit | per draw, 16-bit where allowed. Recommendation taken (ask only with screenshots) | none |
