# Proposal: spacedock

## Why

The owner, 2026-10-14: "are there multiple planets in the system? also can you mockup some versions of spacedock? I want
a big structure where the ship can actually enter rather than something flimsy. I want there to be multiple ships
docket there too". With it, a reference picture of a mushroom-shaped orbital spacedock: a wide domed cap on a long
ringed tower, rows of lit windows and blue accent lights, over a planet, a small starship approaching the doors in the
cap. Taken for its scale and spirit only; every design here is original.

Later the same day: "what about an oneil cylinder space station? can you make some textures for that? a giant cylinder?
wiht a big space station attached at one end. theres usually two of them to counter balance rotational effects", with two
reference pictures of O'Neill cylinders; and then "I'd also like you to light it properly as well, texturing is really
important to sell scale, we should have 2 layers of texture, detail layer, and the big albiedo color map, and maybe
normal maps on the details". On the first draft of it: "What oneil cylinders are hollowed cylinders. Didn't you research
what they looked like and how they work?", "Also notice how the solar panels open up on one side they are blooming
slightly like aflower", and "The ends are round and should have an opening for ships to enter".

Today the Tern lies in an open girder cradle (`data/space/exterior.json` `dock`), and `patrol-mission` design 3 hangs
that cradle off a placeholder starbase's berth arm. Seen from the bridge it is a few thin beams against black space:
the "something flimsy" the owner does not want. Nothing else is docked there, and there is nothing to fly into.

**Are there multiple planets?** Yes. The Shoals system (`patrol-mission` design 2) has the star Shoal, three planets
and a moon: Cinder (a dark rock at 0.37 AU), Tamsin (the terran world the station orbits, 0.80 AU), Orrin (a banded gas
giant at 2.0 AU) and its icy moon Pell, with the belt called the Shoals beyond. From Tamsin's orbit only Tamsin is a
disc (17.5 degrees in radius); Orrin and Cinder are bright points of light, and the mockup shows them so.

## What Changes

- **Tamsin Station becomes a spacedock: a big, solid structure with an enclosed bay the Tern flies into through doors**,
  its doors shut behind it, berthed among other ships. Five versions are drawn for the owner to choose between (design
  2-5a): **A. Bastion** (a domed cap 1.8 km across on a ringed tower, four doors in the cap's rim into one round hall),
  **B. Wheel** (a habitat wheel 1.75 km across round a hub hangar entered along the axis, ships berthed all round its
  curved wall), **C. Drydock** (a long armoured hall 1.7 km long, berths down both walls, ships lying bow out) and
  **D. Hollow** (a captured asteroid 2.2 km long, an armoured gate into a cavern with built berths) and **E. O'Neill
  Pair** (two hollow, counter-rotating O'Neill cylinders 6.4 km long and 1.8 km across, as the cited sources describe
  them: axes on the star, three land valleys inside under clouds between three glazed windows, three mirrors blooming
  from the far end like petals, domed ends; ships enter on the axis through a non-rotating port at one sunward dome,
  into an enclosed hall). The choice is the owner's (design, open question S1).
- **E's textures and light** (design 5b): two layers of texture made in Material Maker, macro maps (the outside, the land
  inside, the clouds, the domes) over tiled detail layers with normal maps, blended by distance; the star on the axes,
  so the sunward domes lit, the sides at a graze and earthshine, the mirrors blazing, the land inside lit through the
  windows and seen through them, haze inside and far off. Per-pixel lighting for station exteriors is proposed as an exception
  to the Pi 5 rule, with its cost, to be measured on the Pi before the engine takes it.
- **Entering and berthing on foot** (design 6, the law): helm flies the Tern in by hand or hands it to the dock
  autopilot (`flight-and-navigation` 10); the doors open on traffic control's clearance; inside, the bay's speed limit
  and the docking corridor's limits hold to the berth; the crew walk the ship meanwhile and see it through the windows
  and on the viewscreens. At the berth a pier lies along the Tern's starboard side, clamps reach the hull and a crew
  tube meets the docking collar on deck C.
- **Other ships at the berths** (design 7): six to eight per version, from the existing models (the Tern's liveries as
  sister Heron-class frigates, the Osprey courier, the Federation concepts Pursuer, Splitjaw and Bulwark), with free
  berths lit and waiting.
- **What the windows show while berthed** (design 8): the bay's lit walls with their tiers of control-room windows,
  the neighbouring ships in their berths' light frames, the crew tubes, gantries and cranes; never the black sky.
- **Data**: `data/space/spacedock.json` holds the five versions' sizes, doors and berths, the sky at the station and
  the bay's traffic limits (design 10).
- **A Pi 5 budget per version** (design 9): the station's near model in three to five draws, docked ships at their
  middle level of detail; every version well inside the frame's 200,000 triangles and 300 draws.
- **A mockup** (`docs/mockups/spacedock.html`): each version from outside with the Tern on approach and Tamsin below,
  the doors opening and the Tern entering, inside the bay with the Tern berthed at its gangway, and the view out of a
  bridge window while berthed, with its Pi 5 cost.

## Capabilities

### New Capabilities

- `spacedock`: the starbase's enclosed bay: its doors, the approach, the berths, the ships docked there, and what the
  crew see from inside the Tern while berthed.

### Modified Capabilities

None yet. Once the owner chooses a version, `patrol-mission`'s placeholder starbase and cradle (its design 3) and its
undocking (design 4.2) follow the chosen version (design 11 here says how); that edit is made in `patrol-mission` then.

## Impact

- `data/space/spacedock.json` (new); `docs/mockups/spacedock.html` and its shots (new).
- Later, on the owner's choice: the station generator (`sc-client/src/ships3d.rs` `StationKind::Starbase`, or the
  `starbase` model when it lands), the deck compiler's dock (`sc-tools/src/dock.rs`, today's girder cradle), the dock
  autopilot (`flight-and-navigation` 10) and `patrol-mission`'s start and undock phases.
- Versions A-D take no new texture: the exterior paints, deck plate, hazard and light panel layers already in the
  texture array. E adds nine textures (17.0 MiB with mips): `tools/materials/ptex/oneill_*.ptex`,
  `tools/materials/oneill_atlases.py`, `assets/textures/oneill/`, `docs/screenshots/materials/oneill-contact-sheet.png`,
  and a `tex:` block in `tools/mockups/inline.py` to put them in the page.
