# Design: SCS Tern, the reference ship's floor plan

## Context

- **The one source** is `data/ships/tern/layout.json` (schema `starcrew.ship-layout/1`,
  section 10). Every number in this document comes from it, through `tools/layout_check.py`
  (volumes, floor areas, hull margins) or a scratch measurement over the same file (routes,
  shared walls, articulation points). Nothing here is hand-placed (CLAUDE.md section 8).
- **The maps** are `docs/design/maps/tern-deck-A.svg`, `tern-deck-B.svg` and `tern-deck-C.svg`,
  drawn by `tools/deck_plans.py` from the layout and shipkit's colour roles: bow to the right,
  port at the top, POI numbers in circles, systems as letters, stations by role code, areas
  open to the deck below hatched. The 3D view is `docs/mockups/deck-plan.html`.
- **The standard** is Undercity's for hub maps (`/home/user/fps-game-demo/CLAUDE.md` section 1,
  after Deus Ex: Mankind Divided's Prague map): numbered points of interest and a legend, and a
  document deep enough to build from (`sump-market-hub`, `the-drains` in that repository).
- **Axes** (from the layout): +X port, +Y up, +Z bow, metres; the origin is on the centreline,
  on deck B's floor, at the hangar's forward wall.
- **Proposed versus decided.** The layout as it stands is decided data. Everything marked
  "proposed" (the patches of section 9) is a recommendation for the coordinator to apply; the
  open questions of the last section ask the owner only where a mockup shot shows the choice.

## Goals / Non-Goals

**Goals:**

- Every compartment's purpose, contents, numbers, doors and reason for its place, by POI.
- Walk times for the routes the systems designs depend on, at stated speeds.
- The big layout decisions argued, and every single point of failure named with a mitigation.
- The schema documented field by field, and the checker's rules as requirements.

**Non-Goals:**

- System behaviour (power, air, weapons, craft): their changes own it; this plan places them.
- Geometry budgets and the deck build: `deck-pipeline` (it measures its per-compartment table
  on this plan).
- Walking speeds and ladder rules: `crew-on-deck` owns them; this document first assumed values
  and now quotes its speeds (T5, done 2026-10-04).

## Decisions

### 1. The ship at a glance

| Quantity | Value | Source |
| --- | ---: | --- |
| Length overall (hull) | 84.0 m (z -42.0 to +42.0) | hull sections |
| Beam (hull) | 24.4 m; 27.0 m across the side turret pods | hull, pods 29 and 30 |
| Height (hull) | 14.0 m (y -5.8 to +8.2); 16.0 m across the dorsal and ventral turret mounts | hull, mounts |
| Decks | A command +3.5 m, B main 0.0 m, C lower -3.5 m; 3.0 m clear, 0.5 m slabs | `decks` |
| Compartments | 30: 17 rooms, 4 corridors, 3 bays, a crawlspace, an airlock and 4 turret pods | `compartments` |
| Portals | 40 (by kind below) | `portals` |
| Air | 8,858.8 m^3; 2,135.0 m^2 of floor | `layout_check.py` |
| Stations | 14 seats: 7 on the bridge (4 core), 3 elsewhere, 4 gunners | `stations` |
| Systems | 25 placed systems (3 outside the hull: radiators, sensor and comms arrays) | `systems` |
| Mounts | 4 twin pulse cannon turrets, 2 missile tubes, 2 main engines | `mounts` |
| Craft | 2 Swift fighters, 1 Petrel shuttle | `craft` |
| Tightest hull margin | 0.08 m (the airlock, 24); next 0.25 m (engineering, 18) | `layout_check.py` |

Portals by kind: door 23, pressure door 4 (two launch bays, two airlock doors), hatch 5 (four
pods and the drive), ladder 2 (one trunk at z = 11 through three decks), hoist 1, window 2, bay
door 3 (two drop doors, the shuttle pad).

The plan in one sentence: **a fore-and-aft spine on every deck, crossed by one ladder trunk
forward, ending aft at a double-height hangar and a three-deck engineering space, with the
bridge on top at the bow and the magazine at the bottom at the bow.**

### 2. Deck A, Command (floor +3.5 m)

Map: `docs/design/maps/tern-deck-A.svg`. Doors are named by their portal id; "to" is the POI on
the other side.

| POI | Compartment | Purpose and contents | Volume m^3 | Floor m^2 | Doors | Why here |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 1 | Bridge | The command centre. Seven stations facing the bow: captain on a 0.3 m dais (3.2 x 2.4 m) at the centre; helm and tactical forward; engineering and science on the port and starboard walls; comms and flight operations aft at the sides. The 6.0 x 2.4 m viewscreen on the forward wall, between two 2.8 x 1.2 m windows | 462.0 | 154.0 | `p_bridge_aft` (1.6 x 2.3 m) to 4 | Top deck, at the bow: windows over the bow and a forward wall for the viewscreen; as far as the ship allows from the reactor and the hangar, its two biggest hazards; straight down the command passage to engineering |
| 2 | Captain's ready room | Office, briefing table, private console | 85.5 | 28.5 | `p_ready` to 4 | Off the passage, 6 m from the bridge door: the captain is one door from the bridge, never in its traffic |
| 3 | Computer core | Processing for sensors, targeting and automation (system `computer`); automated stations draw on it | 85.5 | 28.5 | `p_core` to 4 | Inside the hull's upper middle, beside the bridge it serves; a hit that reaches it has already crossed the hull and the passage |
| 4 | Command passage | Deck A's spine, z 4-20, with the ladder trunk down to B and C at z = 11 | 120.0 | 40.0 | `p_bridge_aft` to 1, `p_ready` to 2, `p_core` to 3, `p_dorsal_fwd` to 5, ladder `p_ladder_ab` to 14 | The bridge's only way out joins every other route here |
| 5 | Dorsal turret access | The hatch up into the dorsal pod | 48.0 | 16.0 | `p_dorsal_fwd` to 4, `p_dorsal_aft` to 6, hatch `p_pod_dorsal` up to 27 | On the spine, so the dorsal gunner is 20.6 s from the helm |
| 6 | Aft passage | The always-pressurized route aft, z -18 to 0, over the hangar's ceiling | 135.0 | 45.0 | `p_dorsal_aft` to 5, `p_aft_eng` to 18 (onto the catwalk) | Section 5.2 |
| 27 | Dorsal turret pod | The dorsal gunner's seat under the turret head (mount at y 9.0) | 15.6 | 6.2 | hatch `p_pod_dorsal` (0.9 x 0.9 m) down to 5 | Section 5.3 |
| 18 | Engineering, upper level | The catwalk (4.0 x 2.0 m) at deck A level inside engineering's forward wall, where the aft passage enters | (see 18) | | `p_aft_eng` | Section 4 |

### 3. Deck B, Main (floor 0.0 m)

Map: `docs/design/maps/tern-deck-B.svg`.

| POI | Compartment | Purpose and contents | Volume m^3 | Floor m^2 | Doors | Why here |
| ---: | --- | --- | ---: | ---: | --- | --- |
| 7 | Torpedo room | Two forward missile tubes and their breeches (systems `tube_1`, `tube_2` at z = 31; mounts at z = 37), the ready racks, the hoist from the magazine below (`p_hoist`, 1.2 x 3.0 m) | 210.0 | 70.0 | `p_torpedo` (1.6 x 2.3 m) to 14; hoist to 21 | The tubes fire along the keel, so the breeches are at the bow; the magazine is directly below, so the hoist is a straight lift (20 s a missile, `weapons-and-shields`) |
| 8 | Medbay | Two beds that revive and heal (`medbay_beds`) | 138.0 | 46.0 | `p_medbay` (1.2 x 2.2 m) to 14 | Forward on the main corridor, 8.5 s from the quarters, and on the same corridor as the ladder trunk, so a casualty from the bridge comes down one ladder |
| 9 | Damage control | Repair kits, extinguishers, EVA suits; the damage control board (station `damage_board`) | 138.0 | 46.0 | `p_damage_control` (1.2 x 2.2 m) to 14 | Mid-ship forward, beside the trunk: 19.0 s to the forward switchboard, 37.3 s to the main switchboard by the aft passage today (29.1 s through the hangar once T1 is applied) |
| 10 | Crew quarters | Bunks and lockers; the mission spawn | 232.5 | 77.5 | `p_quarters` (1.0 x 2.2 m) to 14 | Port, in the protected middle of deck B; 18.9 s to the helm |
| 11 | Mess | Galley and tables; the lobby before a mission | 232.5 | 77.5 | `p_mess` (1.6 x 2.3 m, wide for muster) to 14 | Opposite the quarters; the crew muster here and walk to stations |
| 12 | Port turret access | The hatch into the port pod and the turret's capacitor bank (no ammunition: pulse cannons) | 234.0 | 78.0 | `p_port_turret` to 14, hatch `p_pod_port` (0.9 x 1.4 m) to 29 | The widest point of the hull (12.2 m half beam) puts the pod furthest out for its arc |
| 13 | Starboard turret access | Mirror of 12 | 234.0 | 78.0 | `p_stbd_turret` to 14, hatch `p_pod_stbd` to 30 | Mirror of 12 |
| 14 | Main corridor | Deck B's spine, z 0-26, ten portals: the trunk at z = 11 up and down | 195.0 | 65.0 | `p_torpedo`, `p_medbay`, `p_damage_control`, `p_quarters`, `p_mess`, `p_port_turret`, `p_stbd_turret`, `p_spine_hangar` (to 15's landing), ladders `p_ladder_ab` up and `p_ladder_bc` down | Every forward room on deck B opens onto it (section 7) |
| 19 | Drive section | Crawlspace for the impulse drive and the inertial dampers | 288.0 | 96.0 | hatch `p_drive` (1.0 x 2.0 m) to 18's mezzanine | Behind the reactor, in front of the engines; maintenance only, so one hatch |
| 29 | Port turret pod | The port gunner's seat (mount at x 13.5) | 15.6 | 6.2 | hatch `p_pod_port` to 12 | Section 5.3 |
| 30 | Starboard turret pod | Mirror of 29 | 15.6 | 6.2 | hatch `p_pod_stbd` to 13 | Section 5.3 |
| 15 | Hangar, upper level | The landing (10.0 x 2.0 m) inside the forward wall with bay control (station `bay_control`) overlooking the floor; the galleries over the launch bays | (see 15) | | `p_spine_hangar` onto the landing; `p_gallery_eng_p`, `p_gallery_eng_s` from the galleries to 18 | Section 4 |
| 18 | Engineering, mezzanine | A ring at deck B level around the reactor (inner radius 3.4 m): the engineering bay console (`eng_main`) and the main switchboard | (see 18) | | `p_gallery_eng_p`, `p_gallery_eng_s`, hatch `p_drive` | Section 4 |

### 4. Spanning decks: the hangar, the launch bays, engineering

| POI | Compartment | Decks | Purpose and contents | Volume m^3 | Floor m^2 | Doors |
| ---: | --- | --- | --- | ---: | ---: | --- |
| 15 | Hangar | C+B | A 10 x 18 m bay 6.5 m tall (y -3.5 to +3.0) with the Petrel shuttle on its lift pad over the ventral pad door (6.0 x 12.0 m); the bay pumps and reserve tank; the landing and bay control at deck B level; two galleries (5.5 x 16 m each) at deck B level over the launch bays | 1,698.0 | 356.0 | `p_hangar_c` (2.0 x 2.5 m) from 20 onto the floor; `p_spine_hangar` from 14 onto the landing; pressure doors `p_bay_p`, `p_bay_s` to 16 and 17; `p_hangar_eng` to 18's lower floor; `p_gallery_eng_p`, `p_gallery_eng_s` to 18's mezzanine; bay door `p_hangar_pad` |
| 16 | Port launch bay | C | Swift 1 on its cradle over a 4.8 x 9.0 m ventral drop door | 231.0 | 66.0 | pressure door `p_bay_p` (1.0 x 2.2 m) to 15; bay door `p_drop_p` |
| 17 | Starboard launch bay | C | Swift 2, mirror of 16 | 231.0 | 66.0 | `p_bay_s`, `p_drop_s` |
| 18 | Engineering | C+B+A | 18 x 14 m and 10 m tall (y -3.5 to +6.5): the fusion reactor (radius 2.2 m) through all three levels; the lower floor (deck C) with the coolant pumps; the mezzanine ring (deck B) with the engineering bay console and the main switchboard; the catwalk (deck A) where the aft passage enters; the radiators outside above (y 8.2) | 2,520.0 | 252.0 | `p_aft_eng` (deck A, catwalk), `p_gallery_eng_p` and `_s` (deck B, mezzanine), `p_hangar_eng` (deck C, floor), hatch `p_drive` (deck B) |

**Why engineering is one tall space.** The reactor is 10 m tall; one compartment around it
means one atmosphere, one fire, one set of lights for everything that keeps it running, and
three entrances on three decks, so the engineer arrives from the bridge (deck A), the hangar's
galleries (deck B) or the hangar floor (deck C) without changing deck outside. It is also the
ship's largest air volume (2,520 m^3, 28 % of the ship), which slows a fire's oxygen use and a
breach's pressure drop there (numbers are `life-support`'s).

### 5. Design reasoning

#### 5.1 The hangar is double height, with galleries

- **The Petrel needs it.** The shuttle is 3.2 m tall and 10.0 m long; a 3.0 m deck cannot take
  it, so the hangar spans decks C and B (6.5 m clear).
- **One floor for craft, one level for people.** The floor (deck C) is where the shuttle, the
  pumps and the launch bay doors are; the landing and galleries (deck B) are where flight
  operations stands, out of the shuttle's way, looking down on the whole bay. Bay control on
  the landing sees the pad, both pressure doors and the engineering door at once.
- **The galleries carry deck B aft.** They run over the launch bays to the engineering
  mezzanine, so deck B has a route aft that does not touch the hangar floor (and, with the
  patch of section 9, starts at the landing).
- **Small launch bays, big hangar.** A fighter launches from a 231 m^3 bay, not the 1,698 m^3
  hangar: a seventh of the air to move per launch (pump-down to 5 kPa 28.6 s against 208.4 s for
  the hangar, `life-support` section 13; corrected 2026-10-04 from an assumed 30 s and 120 s).
  Fighters can launch while crew work in the hangar.

#### 5.2 The aft passage on deck A stays pressurized

The hangar vents for every shuttle launch (`p_hangar_pad`). If the only way from the bridge to
engineering crossed it, a shuttle launch would cut the ship in two. The aft passage (6) runs
over the hangar's ceiling, on deck A, with a 0.5 m slab between them (y 3.0 to 3.5); its two
doors open onto the dorsal turret access (5) and engineering's catwalk (18), and neither of
those ever vents in normal operation. So the route from the helm to the engineering bay console by
the aft passage is 33.0 s walking (15.6 s running) whatever the hangar's pressure; through the
hangar it is 35.4 s (with T1) when the hangar is at pressure and closed when it is not (section 6,
`crew-on-deck`'s speeds). The two routes together are the ship's
main loop (section 7).

#### 5.3 Turrets are manned from pods

- **The gun head sits outside the hull** for its arc: dorsal at y 9.0, ventral at y -7.0, port
  and starboard at x +/-13.5 (`mounts`). The pods are the only compartments outside the hull
  check, because that is their point.
- **A pod is its own pressure volume** (15.6-21.9 m^3) behind a hatch. A hit on the turret vents
  the pod, not the access room; the hatch shuts and the access room stays at pressure.
- **Access rooms hold the capacitor banks.** Pulse cannons need no ammunition; the access room
  is where the turret's power is stored and where damage control repairs it without entering
  the pod.
- **The walk is short.** Quarters to the port or starboard gunner's seat 15.7-15.9 s; the helm to
  the dorsal seat 20.8 s, to the ventral seat 26.9 s (section 6).

#### 5.4 The launch bays drop ventrally

- **Gravity does the first metre.** The interior's artificial gravity points along -Y
  (`ship-frames`); a fighter released from its cradle falls out of a floor door with no
  catapult, then lights its engine clear of the hull.
- **No room to turn.** A launch bay is 5.5 m wide and the Swift spans 4.6 m. A ventral door
  needs no taxiing and no turn, and the bay can be the size of the fighter.
- **Clear of everything that fires.** The drop doors are in the bottom hull at z -10: away from
  the dorsal turret, the radiators on top, the missile tubes at the bow and the engines' plume
  at the stern.
- **The same for the Petrel.** The shuttle's lift pad lowers through the hangar's own ventral
  pad door (6.0 x 12.0 m).

### 6. Routes and walk times

**Speeds are `crew-on-deck`'s** (its sections 3-5 and 16; question T5, done 2026-10-04): walk
1.8 m/s, run 4.0 m/s; stairs along their slope at 70 % of walking or running speed; ladders
0.8 m/s up and 1.0 m/s down, plus 0.5 s to get on and 0.5 s to get off; a side hatch into a pod
1.5 s; ordinary doors open on approach (0 s); standing up from a seat 0.9 s and sitting down 0.4 s
are included. Paths are straight lines from portal to portal at floor height, ignoring furniture,
so real times are a little longer. Every number below is printed by `python3 tools/walk_times.py`
(`crew-on-deck`'s instrument, which reads the layout); it replaces the first table, made at an
assumed 1.6 m/s walk with ladders at 0.8 m/s both ways and 1.0 s on and off. T2 (the stairs) was
applied on 2026-10-04; T1 (the galleries) was not, so routes that need it are marked.

| Route | Path m | Walk s | Run s | Through (POI) | As laid out today |
| --- | ---: | ---: | ---: | --- | --- |
| Quarters (spawn) to helm | 27.1 | 18.9 | 11.7 | 10, 14, ladder, 4, 1 | Same |
| Quarters to captain | 22.9 | 16.6 | 10.6 | 10, 14, ladder, 4, 1 | Same |
| Helm to engineering console, by the aft passage | 54.8 | 33.0 | 15.6 | 1, 4, 5, 6, 18 (catwalk, stair down) | Same (T2 applied) |
| Helm to engineering console, through the hangar | 56.7 | 35.4 | 19.1 | 1, 4, ladder, 14, 15 (landing, gallery), 18 | **No route** until T1; with T2 alone, 41.7 s by the hangar floor and the gallery stair |
| Helm to the reactor's lower floor | 56.0 | 36.1 | 21.2 | 1, 4, ladder, 14, ladder, 20, 15 (floor), 18 | Same |
| Helm to the reactor panel (scram reset), by the aft passage | 59.4 | 35.2 | 16.3 | 1, 4, 5, 6, 18 (catwalk, stair down) | Same (T2 applied) |
| Damage control board to forward switchboard | 28.0 | 19.0 | 11.5 | 9, 14, ladder, 20, 26 | Same |
| Damage control board to battery bank | 29.5 | 19.8 | 11.9 | 9, 14, ladder, 20, 26 | Same |
| Damage control board to main switchboard, through the hangar | 50.7 | 29.1 | 13.6 | 9, 14, 15 (landing, gallery), 18 | **No route** until T1 |
| Damage control board to main switchboard, by the aft passage | 57.1 | 37.3 | 20.2 | 9, 14, ladder, 4, 5, 6, 18 | Same (T2 applied) |
| Magazine racks to torpedo tube 1, on foot | 37.2 | 24.1 | 13.8 | 21, 20, ladder, 14, 7 | Same (missiles ride the hoist: 20 s each) |
| Quarters to a medbay bed | 15.2 | 8.5 | 3.8 | 10, 14, 8 | Same |

**Any station to the launch bays**, to the hangar side of the nearer bay's pressure door (both bays
are symmetric to within 0.6 s; boarding from there is `shuttle-bay-and-fighters`' step 1, 6 s):

| From | Walk s | Run s |
| --- | ---: | ---: |
| Captain | 25.7 | 16.5 |
| Helm, tactical | 28.1 | 17.5 |
| Engineering, science (bridge) | 28.2 | 17.6 |
| Comms, flight operations | 26.9 | 17.0 |
| Engineering bay console | 15.6 | 7.5 |
| Damage control board | 23.4 | 13.5 |
| Bay control (by the ladder trunk today; by the gallery stair once T1 is applied) | 23.6; 22.1 | 13.6; 10.5 |
| Dorsal gunner | 28.4 | 20.2 |
| Ventral gunner | 11.2 | 8.2 |
| Port or starboard gunner | 27.0 | 15.9 |
| Quarters (spawn) | 18.6 | 10.8 |

**Gunners to their seats**: from the quarters 15.7 s (port), 15.9 s (starboard), 20.2 s (dorsal),
17.4 s (ventral); from the helm 20.8 s (dorsal), 26.5 s (port, starboard), 26.9 s (ventral).

What the numbers say: every bridge station is within 28.2 s of a launch bay's pressure door at
walking pace (17.6 s running), well inside the 40 s the requirement allows; the core four reach
their seats from the spawn in 18.9-19.1 s; and the two routes from the helm to the engineering
console are 2.4 s apart (33.0 s and 35.4 s), so losing either costs little once T1 gives the
hangar route.

### 7. Single points of failure

Measured on the crew-portal graph (doors, hatches, ladders, pressure doors): a compartment or
portal whose loss cuts some compartments off from the bridge.

| What fails | What it cuts off | Mitigation today | Proposed |
| --- | --- | --- | --- |
| The bridge door `p_bridge_aft` | The bridge from the whole ship | Every core role has a second console (engineering bay console, damage control board, bay control) | **Scuttle** `p_bridge_scuttle`: a 0.9 m ladder hatch from the bridge's aft starboard corner down into damage control (T3). With it, losing the command passage cuts off only the ready room and the computer core |
| The command passage (4) | Everything but the bridge | As above | The scuttle (T3) |
| The main corridor (14) | Torpedo room, medbay, quarters, mess, both turret accesses and side pods | Two exits (the trunk, the landing); 2.5 m wide; damage control beside it | Accepted: every forward room on deck B has one door by design (one pressure boundary each); the scuttle gives damage control a second way out |
| The lower corridor (20) | Magazine, life support, cargo, airlock, shield generator, forward switchboard, ventral pod | Two exits (the trunk, the hangar floor) | Accepted for the same reason |
| The ladder trunk at z = 11 | The only vertical link forward of the hangar | Engineering's three entrances are the vertical link aft | The stairs (T2) make the aft link walkable; the scuttle adds a second link A to B forward |
| Engineering (18) | The drive section (19) | Maintenance only | Accepted |
| The hangar (15) | The launch bays (16, 17) | Inherent: the bays are entered from the hangar floor | Accepted |
| Cargo (23) | The airlock (24) | Inherent | Accepted |

**Systems that live in one compartment**, and what keeps the ship alive when it is lost:

| Compartment lost | Systems in it | What remains |
| --- | --- | --- |
| Engineering (18) | Reactor, main switchboard, coolant pumps | Battery bank and forward switchboard in 26, at the opposite end of the ship, with the emergency bus (`power-grid`) |
| Life support (22) | Oxygen generator, scrubbers, air handler, thermal control | 8,859 m^3 of air in the ship; reserve gas bottles in cargo (23); the bay pumps' reserve tank in the hangar; EVA suits in damage control (`life-support` owns the endurance numbers) |
| Forward switchboard (26) | Battery bank, the bus cross-tie | The main switchboard in engineering |
| Shield generator (25) | The shield generator | None: shields are lost (`weapons-and-shields`) |
| Computer core (3) | Processing for automation | Manned stations keep working; automated ones degrade (`bridge-stations`) |

### 8. What the layout checker should add (proposed)

`tools/layout_check.py` enforces the rules in the spec's first eight requirements. Measuring
this plan showed three rules it cannot yet see, which the coordinator may add (they are
documentation tooling, CLAUDE.md section 4):

1. **Walk levels inside a compartment.** A multi-level compartment (the hangar, engineering)
   needs its walkable levels and the stairs between them, and every station, door sill and
   system on a level reachable from every other. Today the checker sees one node per
   compartment, so it passes a plan whose engineering console cannot be walked to.
2. **Sizes for systems** (T4). A system is a point today, so "stands clear" cannot be checked.
3. **Frames fit.** A door's frame (clear opening plus 0.2 m each side and at the head) fits its
   wall; measured here, every door on the Tern passes with at least 0.05 m to spare.

### 9. Proposed layout patches (exact JSON for the coordinator)

Each patch was applied to a scratch copy and passes `tools/layout_check.py` (all together: 30
compartments, 41 portals, 8,924.8 m^3, ok).

**T1. Galleries meet the landing.** In `compartments`, `hangar`, change boxes 2 and 3's `z`
from `[-18.0, -2.0]` to `[-18.0, 0.0]`, and the landing's note:

```json
{ "x": [5.0, 10.5], "y": [0.0, 3.0], "z": [-18.0, 0.0] },
{ "x": [-10.5, -5.0], "y": [0.0, 3.0], "z": [-18.0, 0.0] }
```

```json
{ "id": "hangar_landing", "kind": "landing", "compartment": "hangar", "center_m": [0.0, 0.0, -1.0], "size_m": [10.0, 2.0],
  "note": "Deck B level landing inside the hangar's forward wall. It runs onto both galleries; stairs at the galleries' aft ends go down to the floor." }
```

Effect: the hangar becomes 1,764.0 m^3 and 378.0 m^2; the helm to the engineering console
through the hangar drops from 41.7 s to 35.4 s at `crew-on-deck`'s speeds (section 6; first
measured as 47.2 s to 36.9 s at the assumed 1.6 m/s).

**T2. Stairs inside engineering and the hangar.** *Applied to `layout.json` on 2026-10-04.* Append to `fixtures` (a new fixture kind,
`stair`: `top_m` and `foot_m` are the centres of the flight's top and bottom edges, `width_m`
its width; `center_m` is their midpoint):

```json
{ "id": "eng_stair_upper", "kind": "stair", "compartment": "engineering", "center_m": [4.0, 1.75, -18.6], "top_m": [2.0, 3.5, -18.6], "foot_m": [6.0, 0.0, -18.6], "width_m": 1.2,
  "note": "From the catwalk (deck A level) down to the mezzanine, along the forward wall to port." },
{ "id": "eng_stair_lower", "kind": "stair", "compartment": "engineering", "center_m": [-4.0, -1.75, -18.6], "top_m": [-2.0, 0.0, -18.6], "foot_m": [-6.0, -3.5, -18.6], "width_m": 1.2,
  "note": "From the mezzanine down to the lower floor, along the forward wall to starboard, through a hole in the mezzanine." },
{ "id": "hangar_stair_p", "kind": "stair", "compartment": "hangar", "center_m": [3.15, -1.75, -17.4], "top_m": [5.0, 0.0, -17.4], "foot_m": [1.3, -3.5, -17.4], "width_m": 1.2,
  "note": "From the port gallery's aft end down to the hangar floor beside the engineering door." },
{ "id": "hangar_stair_s", "kind": "stair", "compartment": "hangar", "center_m": [-3.15, -1.75, -17.4], "top_m": [-5.0, 0.0, -17.4], "foot_m": [-1.3, -3.5, -17.4], "width_m": 1.2,
  "note": "From the starboard gallery's aft end down to the hangar floor beside the engineering door." }
```

Clearances checked: the upper stair's foot is 1.25 m from the port gallery door and 1.3 m from
the engineering console; the lower stair's hole is 1.25 m from the starboard gallery door; the
hangar stairs end 0.5 m beside the engineering door's jambs, 0.8 m aft of the pad door, clear of
the shuttle, the launch bay doors and the bay pumps. Stairs at the landing itself (as the
landing's current note says) would block the launch bays' pressure doors or sit over the pad
door; the aft position avoids both.

**T3. The bridge scuttle.** Append to `portals`:

```json
{ "id": "p_bridge_scuttle", "kind": "hatch", "between": ["bridge", "damage_control"], "axis": "y", "center_m": [-6.2, 3.0, 20.8], "size_m": [0.9, 0.9] }
```

It sits in the bridge's aft starboard corner, 1.44 m from the flight operations seat, and comes
down in damage control 2.08 m from the damage control board.

**T4. Sizes for systems.** Add `size_m: [x, y, z]` (metres, the system's bounding box, centred
on `center_m` in x and z and standing on it in y) to every system except those with
`radius_m` and `height_m`. Values belong to the system's owning change (`power-grid`,
`life-support`, `weapons-and-shields`, `shuttle-bay-and-fighters`), so this patch adds the field
to the schema first; the layout checker then tests each box inside its compartment and clear
of door openings.

### 10. The layout file, field by field (`starcrew.ship-layout/1`)

Units are SI; lengths in metres, angles in degrees. Every id is unique within its list and
lower snake case. The checker's rules are in the spec.

**Top level**

| Field | Type | Meaning |
| --- | --- | --- |
| `schema` | string | `"starcrew.ship-layout/1"`. A loader refuses another value |
| `ship` | object | Identity: `id` (the directory name), `name`, `class`, `summary`, `presented_by` (the change that presents it) |
| `conventions` | object | Human-readable statement of units, axes (+X port, +Y up, +Z bow), origin, yaw, box and portal conventions; `wall_thickness_m` (0.25) and `deck_slab_m` (0.5) are numbers the checker uses for hull margins |
| `decks` | list | Deck records |
| `hull` | object | The hull's sections |
| `compartments` | list | The compartments: the nodes of the compartment graph |
| `portals` | list | The openings between compartments, or to `space`: the edges |
| `stations` | list | Seats where a crew member operates a console |
| `fixtures` | list | Built-in furniture that shapes the space (viewscreen, dais, landing, mezzanine, catwalk; proposed: stair) |
| `systems` | list | Ship systems placed in compartments (or in `space`, outside the hull) |
| `mounts` | list | Weapon and engine hardpoints on the hull |
| `craft` | list | Craft carried aboard |

**`decks[]`**: `id` (a letter), `name`, `floor_y_m` (the deck's floor height), `clear_height_m`
(floor to ceiling).

**`hull.sections[]`**: octagonal cross-sections lofted linearly along Z. `z_m` (station along
the ship), `half_beam_m`, `top_m`, `bottom_m`, `chamfer_m` (each corner cut at 45 degrees by this
much). The hull is closed at the first and last section.

**`compartments[]`**

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | string | Stable id, used by portals, stations, systems, saves and the deck file |
| `poi` | integer | Point-of-interest number on every map and mockup, unique |
| `name` | string | Display name |
| `decks` | list of deck ids | Every deck the compartment spans |
| `kind` | string | `room`, `corridor`, `bay`, `crawlspace`, `airlock` or `pod` (pods are exempt from the hull check) |
| `purpose` | string | One sentence, from the player's side |
| `boxes` | list | Axis-aligned air boxes `{x: [min, max], y: [...], z: [...]}`: the inner surfaces. Boxes of one compartment that touch are open to each other |

**`portals[]`**

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | string | Stable id |
| `kind` | string | `door`, `pressure_door`, `hatch`, `ladder`, `hoist`, `window`, `bay_door`. Crew pass through doors, pressure doors, hatches and ladders; a hoist moves cargo; windows never open; bay doors open to space |
| `between` | two ids | The two compartments, or one and `space` |
| `axis` | `x`, `y` or `z` | The axis the opening's plane is normal to (`y` for floor and ceiling openings) |
| `center_m` | [x, y, z] | The opening's centre, on the plane where the two sides meet (a vertical opening may sit anywhere within the deck slab between them) |
| `size_m` | [a, b] | The clear opening: for axis `x` [z, y], for `z` [x, y], for `y` [x, z] |

**`stations[]`**: `id`, `name`, `role` (`command`, `helm`, `tactical`, `engineering`,
`science`, `comms`, `flight_ops`, `gunner`), `compartment`, `seat_m` (the seat's floor point),
`yaw_deg` (0 faces the bow, +90 faces port), `core` (one of the four core stations), `mount` (a
gunner's turret), `note`.

**`fixtures[]`**: `id`, `kind`, `compartment`, `center_m`, `size_m` ([x, z] footprint, or
[width, height] for a viewscreen), and by kind `facing_yaw_deg`, `height_m` (a dais),
`ring_inner_radius_m` (a mezzanine around the reactor), `note`; proposed for `stair`: `top_m`,
`foot_m`, `width_m`.

**`systems[]`**: `id`, `name`, `kind` (`power_source`, `power_distribution`, `power_storage`,
`thermal`, `propulsion`, `dampers`, `shields`, `life_support`, `computer`, `sensors`, `comms`,
`gravity`, `medical`, `magazine`, `missile_tube`, `launch`), `compartment` (or `space`),
`center_m`, optional `radius_m` and `height_m` (a cylinder), optional `mount`; proposed:
`size_m`.

**`mounts[]`**: `id`, `kind` (`turret`, `missile_tube`, `engine`), `center_m`, `facing` (a unit
axis), `weapon`, `crew_seat` (the station that mans it).

**`craft[]`**: `id`, `name`, `class`, `bay` (its compartment), `cradle` (a system), `drop_door`
(a portal), `center_m`, `length_m`, `span_m`, `height_m`, `seats`.

### 11. The maps and the mockup

**Deck plans** (`python3 tools/deck_plans.py`, standard library only, rendered and checked by
eye):

| File | Shows |
| --- | --- |
| `docs/design/maps/tern-deck-A.svg` | Deck A: POI 1-6, 18 (the catwalk level, open below) and 27 (dorsal pod above, dashed); 8 seats; systems a-b |
| `docs/design/maps/tern-deck-B.svg` | Deck B: POI 7-14, 19, 29, 30, the hangar's landing and galleries over the open bay (15) and engineering's mezzanine ring (18); systems a-g |
| `docs/design/maps/tern-deck-C.svg` | Deck C: POI 15-18, 20-26 and 28 (ventral pod below, dashed); the craft on their cradles; systems a-o |

A badge that cannot sit inside its compartment (a pod, the airlock) sits beside it with a
leader line; a bay door is drawn as a dashed outline under its craft.

**The 3D mockup**, `docs/mockups/deck-plan.html`: every compartment by `ShipKit.roomShell`
with its ceiling off, inside an x-ray hull, coloured by kind; numbered POI badges and a legend
by deck; deck filters, an exploded view, a plan view per deck, an info card per compartment,
door markers by portal kind, ladders, seats in role colours, systems as nominal blocks (T4),
craft; normal, red-alert and emergency lighting from lamp fixtures placed by the kit rule; the
patches the layout does not hold yet as ghosts (T1 and T3; T2's stairs are in the layout and drawn
solid, and the button names only the patches still drawn; reconciled 2026-10-05); and a route
tool. Shots, in `docs/screenshots/mockups/`:

| Shot | Shows | Open question |
| --- | --- | --- |
| `deck-plan-overview` | The whole interior at once, with the Pi 5 meter | K2 |
| `deck-plan-exploded` | The three decks pulled apart, ladders as dashed links | |
| `deck-plan-deck-A-plan` | Deck A from above; the bridge's one door and the proposed scuttle | T3 |
| `deck-plan-deck-B-plan` | Deck B from above; partitions as single lines; the proposed gallery extension (T1, a ghost) | K1, T1 |
| `deck-plan-deck-C-plan` | Deck C from above; pressure doors in orange, bay doors in red, craft | K4 |
| `deck-plan-route-bridge-to-engineering` | The route tool, helm to the engineering bay console | T2, T5 |
| `deck-plan-red-alert` | The forward half in red alert | |
| `deck-plan-engineering-closeup` | Engineering's three levels, the T2 stairs as applied to the layout (solid, no longer a ghost), the info card | K3, T2 |

**The route tool reproduces section 6** to within 1.5 s. It is a fastest-path search over crew
portals (a search weighted by time) with `crew-on-deck`'s speeds of section 6, on the layout as it
stands; its `SPEED` constants were set to `tools/walk_times.py`'s on 2026-10-04 (T5), ladders by
direction and the seat's stand and sit included. Measured from the page: quarters to helm 18.9 s
(27.2 m), to captain 16.6 s; helm to the dorsal gunner's seat 20.2 s, to the ventral seat 28.4 s
(the page counts the trunk's two ladders as two climbs, 1.5 s over the table); quarters to the port
gunner's seat 15.8 s. Helm to the engineering bay console is 32.1 s (52.4 m) by the aft passage:
the page costs the 3.5 m drop from the catwalk to the mezzanine in a straight line at the stairs'
70 %, where section 6 follows the T2 stair along the wall (33.0 s, 54.8 m). Helm to engineering's
catwalk door is 26.1 s.

### 12. The Pi 5 cost of this plan

The plan spends nothing by itself; `deck-pipeline` measures its geometry: 58,573 triangles and
91 draw calls for the whole ship at the proposed kit density, every compartment under its
ceiling (the bridge 5,630 of 30,000; engineering, the busiest, 7,028 of 8,000), and a worst
visible set of 53,184 triangles (the main corridor with every door open, a crude upper bound)
against an 80,000-triangle interior pass. The proposed patches add about 700 triangles (four
stairs at about 140 each, the gallery extension and the scuttle's frame).

## Risks / Trade-offs

- **Walk times rest on `crew-on-deck`'s proposed speeds**, which playtests may move. Mitigation:
  `tools/walk_times.py` regenerates every table from the layout and those speeds; the requirements
  quote limits with margin (under 40 s), not the exact times.
- **One door per room** keeps pressure boundaries simple and makes the corridors single points
  of failure. Accepted, with two exits per corridor and the scuttle; a fire in a corridor is
  meant to be a crisis.
- **The bay sizes are tight.** The Swift's 4.6 m span in a 5.5 m bay leaves 0.45 m a side; the
  airlock's hull margin is 0.08 m. Both are deliberate and both are checked.
- **Patches move numbers other changes quote** (the hangar's volume, the total air). Mitigation:
  they are proposed here with the new numbers and applied once, by the coordinator, in one
  commit with the changes that quote them.

## Open questions

Questions go to the owner only with something to look at (CLAUDE.md 13). Rows without a shot
proceed on the recommendation ("recommendation taken, ask only with screenshots").

| Id | Question and fact | Options | Recommendation | Mockup shot |
| --- | --- | --- | --- | --- |
| T1 | The hangar's galleries meet the landing only at a corner, so deck B's route aft over the launch bays starts in engineering | a. extend the galleries forward 2 m to the landing; b. leave them as engineering's balconies | a: the through-hangar route from the helm to the engineering console drops from 41.7 s to 35.4 s (`crew-on-deck`'s speeds; corrected 2026-10-04 from 47.2 s to 36.9 s at the first assumed speeds) | `deck-plan-deck-B-plan` |
| T2 | Engineering has no stair between its catwalk, mezzanine and lower floor, so its console cannot be walked to; the hangar's landing stairs have no place that does not block a door | a. the four stairs of section 9 (engineering's along its forward wall, the hangar's at the galleries' aft ends); b. ladders instead (faster to fit, 0.8 m/s, no carrying) | a. **Applied 2026-10-04** by the coordinator as a fix, since the engineering console could not be walked to (recommendation taken; ask only with screenshots) | `deck-plan-engineering-closeup` |
| T3 | The bridge has one door | a. the scuttle to damage control; b. a second door to the ready room; c. none | a: it also gives damage control a second exit | `deck-plan-deck-A-plan` |
| T4 | Systems are points, so nothing checks that a scrubber or a pump stands clear of a door | add `size_m` to systems, values set by each system's change | add it. Recommendation taken (ask only with screenshots) | none |
| T5 | Walking speeds (first assumed: 1.6 m/s walk, 4.0 m/s run, 0.8 m/s ladders) | `crew-on-deck` decides | take `crew-on-deck`'s; regenerate the tables. Recommendation taken (ask only with screenshots). **Done 2026-10-04**: walk 1.8 m/s, run 4.0 m/s, ladders 0.8 m/s up and 1.0 m/s down, stairs at 70 %; section 6 regenerated with `tools/walk_times.py`, and the mockup's route tool uses the same speeds | none |
