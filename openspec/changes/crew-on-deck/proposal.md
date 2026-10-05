# Proposal: crew on deck

## Why

"Crew stand up, walk to engineering, carry an extinguisher, climb into a turret and drop out of
the hangar in a fighter. The interior is a space you inhabit, decoupled from the ship's motion
through space" (`docs/design/vision.md`, pillar 3). The owner asked for "a full 3D starship
bridge" and a crew of four or more friends (2026-10-04): what they do away from a console happens
on foot, and nothing has said yet how fast a body moves, what it fits through, what it can carry,
what the air does to it, or how a friend gets it back on its feet.

Other changes are waiting on those numbers: `ship-frames` proposes the body's response to the
dampers' residual and hands it here; `deck-pipeline` checks the decks against a crew capsule it
can only assume; `netcode-and-sessions` predicts "my avatar" through a movement function nobody
has written; `reference-ship-tern` computed its walk times on assumed speeds (its question T5);
`bridge-stations` needs a downed state, a seat clip and a brace. star-crew-64 proved bodies on
foot, NPC bodies, revival and "all crew down" as the loss, and left the lessons this change
fixes: frame-counted speeds, revival only at full health, extinguishers no level placed
(`docs/analysis/star-crew-64.md`).

## What Changes

Everything below is **proposed** unless it says it comes from the layout, the brief or another
change, which are **decided**.

- **The body**: a capsule of radius 0.30 m and height 1.80 m standing (1.20 m crouched, 1.30 m
  seated), eye at 1.65 m (1.05 m crouched, 1.20 m seated); suited 0.35 m and 1.90 m. Checked
  against every crew portal of the Tern: every door, pressure door, ladder trunk and hatch passes
  the body that must use it, the two side pod hatches (0.9 x 1.4 m with a 0.40 m sill) by a
  climb-through clip.
- **Movement in the interior frame**, by one `sc-core` function run on the server and in the
  client's prediction: walk 1.8 m/s, run 4.0 m/s, crouch 0.9 m/s, stairs at 70 %, a 0.35 m step,
  acceleration capped by grip (0.6 x felt gravity, so low gravity is slippery), raw mouse look,
  no jump. Ladders at 0.8 m/s up and 1.0 m/s down with 0.5 s on and off.
- **Doors and hatches**: doors open on approach in 0.6 s and never close on a body; pressure
  doors open on Use in 2.0 s; hatches by hand in 1.0 s; locking from the damage board or the
  door's panel. **The pressure interlock** is `life-support`'s: nothing opens across more than
  20 kPa; an ordinary door or hatch can be overridden by holding Use 3 s (logged), at the cost of
  the air; a pressure door never by hand (reconciled 2026-10-05: life-support owns this).
- **Hands**: Use within 1.5 m with a clear line; one held thing: an extinguisher (15 s of agent,
  3 m reach), a repair kit, a patch kit (two-handed), a casualty over the shoulder (1.2 m/s, half
  speed on ladders), or a Gannet on the magazine trolley (300 N push, at most 0.8 m/s, a dead-man
  brake, never leaving the magazine or the torpedo room).
- **Health, downed and revive**: 100 HP; wounded under 50 (no running); downed at 0 with a 120 s
  stabilize window; **revived by hand in 5 s at 25 HP** (not star-crew-64's full health after 13
  presses); critical bodies only on a medbay bed; field recovery to 50 HP; every body down is the
  loss; no permanent death, a campaign injury instead.
- **The medbay**: two beds, 2.0 HP/s powered and 0.5 HP/s unpowered; a downed or critical body
  revived on a bed after 20 s.
- **The air on a body, by name**: hypoxia, hypercapnia, smoke, cold, heat, low pressure and
  vacuum at `life-support`'s thresholds and rates (its `crew_effects`), presented to the player
  with screen effects and movement limits; and the pull of a breach (reconciled 2026-10-05:
  life-support owns the air's effects).
- **EVA suits**: eight in three lockers, 20 s to don, 1,800 s of oxygen, 1.5 m/s, magnetic boots,
  punctures; outside the hull a tethered magnetic-boot walk that feels the ship's full
  acceleration.
- **Zero gravity** under 2 m/s^2 of felt gravity: kick off at up to 2.0 m/s, hand over hand along
  rails at 1.5 m/s, a slow flail so nobody is stranded, "up" kept as the ship's +Y.
- **The lurch**: `ship-frames`' residual table adopted as the body's response, with bracing (up to
  three times grip, 17.7 m/s^2) and ladders added; knockdowns drop what is held; walls struck
  above 3 m/s injure.
- **Walk times** computed from the layout: every seat within 28.4 s of a launch bay walking; the
  core four seated from the spawn in about 19 s; the helm to the engineering bay console 33.0 s
  or 35.4 s by its two routes (the first through `reference-ship-tern`'s T2 stairs, applied
  2026-10-04; the second waits on its T1 galleries).
- **The avatar**: 3,000 triangles with LODs of 1,000 and 300, 30 bones of the 48 allowed, two
  influences, one draw call, about 55 baked clips in locomotion, full-body, upper-body and
  additive layers; first-person arms for the player.
- **Network**: the avatar predicted on the client (walking, ladders, zero gravity, doors on
  approach); outcomes decided by the server; a crew status group and the client's own felt
  residual added to `netcode-and-sessions` (about 1 kbit/s).

## Capabilities

### New Capabilities
- `crew-on-deck`: the crew body and its movement in the interior frame, ladders, doors and
  hatches with the pressure interlock, using and carrying, health and revival, the medbay, the
  air's effects on a body, EVA suits, zero gravity, the response to the dampers' residual, walk
  times on the Tern, and the avatar's geometry, animation and network form.

### Modified Capabilities
None. `openspec/specs/` holds nothing yet.

## Impact

- **Data (proposed):** `data/crew.json` (every number above, units in the keys),
  `data/ships/tern/kit.json` (extinguisher brackets, kits, suit lockers, beds, trolleys),
  `data/input/bindings.json` (on-foot bindings beside `bridge-stations`' seated ones).
- **Core (proposed):** a `crew` module in `sc-core`: `crew::step` (movement), `crew::injure`,
  postures, hands, effects, revive and beds; items as interior objects.
- **Client (proposed):** first-person camera with the lurch, prediction and correction, the
  avatar renderer (skinned, LODs), clip playback and layering, the status line.
- **Tools (proposed):** `tools/walk_times.py` (the measurement in design section 16); the portal
  clearance check in `tools/layout_check.py`; the clip baker in `sc-tools`.
- **Other changes:** `ship-frames` (adopts its residual table), `deck-pipeline` (the capsule,
  handhold rails, ladder deck hatches, the hull as an EVA surface), `life-support` (the values
  this change reads), `damage-control` (fire, repair, patch rates; `crew::injure` callers),
  `power-grid` (gravity, doors, beds), `weapons-and-shields` (the trolley path),
  `shuttle-bay-and-fighters` (boarding, EVA), `bridge-stations` (seats, downed, brace),
  `netcode-and-sessions` (two snapshot groups), `reference-ship-tern` (its T5 speeds and route
  tables).
- **Pi 5 budget:** at worst 26,802 triangles and 15 draw calls (13 % and 5 % of the provisional
  budget) with eight bodies in view; about 1 MB of meshes and clips of the client's 384 MB; under
  0.1 ms of server time per tick; about 1 kbit/s down. The breakdown is in design.md section 18.
