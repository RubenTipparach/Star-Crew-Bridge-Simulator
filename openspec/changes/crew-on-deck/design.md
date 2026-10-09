# Design: crew on deck

Status: **proposed** (2026-10-04). Nothing here is built. Where a fact comes from the layout
(`data/ships/tern/layout.json`), the brief (`docs/design/vision.md`) or another change, it is
**decided there** and says so; everything else is this change's proposal. Every number carries
its unit and lives in data (`data/crew.json`, section 15), so a playtest retunes it without code.
Budgets are against the provisional Pi 5 table in `openspec/changes/engine-stack/design.md`
section 5, the one source for those numbers. The mockup that shows a body at its eye height is
`docs/mockups/bridge.html` (walk mode; `bridge-stations` presents it).

## Context

The brief: "Crew stand up, walk to engineering, carry an extinguisher, climb into a turret and
drop out of the hangar in a fighter. The interior is a space you inhabit, decoupled from the
ship's motion through space" (vision, pillar 3). The owner: "a full 3D starship bridge is vital"
and "a core of 4 players, possibly more can join" (2026-10-04). Everything a player does away
from a console happens on foot: reaching a seat, putting out a fire, patching a breach, loading a
tube by hand, carrying an incapacitated friend to the medbay, floating hand over hand when the gravity
generator loses power.

Other changes have already fixed the frame of this work:

- **`ship-frames`** puts every crew body in the **interior frame**: ship-local, `f32`, stepped at
  the server's 30 Hz tick against the deck's brushes with gravity `(0, -g_art, 0)`. The ship's
  motion reaches a body only through the dampers' **residual** `F_k` per compartment and the
  **hit shake** trauma `T_k`, and ship-frames proposes the body's response to them (section 4
  there). This change owns that response.
- **`deck-pipeline`** supplies one swept capsule query against convex brushes, ladder volumes,
  door movers, and three checks that use this change's capsule: people stand clear, walkable
  (a flood fill), and clear width (section 6 there). Until now it assumed a 0.30 m radius and
  1.80 m standing, 1.30 m seated.
- **`netcode-and-sessions`** predicts a player's own avatar on the client "through `sc-core`'s
  movement function" and sends avatars at 20 Hz as 10 bytes each (sections 4 and 5 there).
- **`bridge-stations`** claims seats within 1.5 m with a 0.4 s seat snap, hands an incapacitated
  player's station to automation, keeps NPC bodies and body swap, and walks the bridge at
  1.8 m/s.
- **`reference-ship-tern`** assumed 1.6 m/s walking, 4.0 m/s running and 0.8 m/s on ladders for
  its route tables and asked this change to decide (its question T5).
- **star-crew-64** (`docs/analysis/star-crew-64.md`) proved bodies on foot, NPC bodies, revival
  by a teammate and "all crew down" as a loss, and left lessons: timings in frames, a walk of
  0.6 units a frame (about 3.6 m/s at 60 frames a second, with 1 unit = 0.1 m), revive only at
  full health after 13 presses, extinguishers that no level placed (section 20).

The changes that own the air (`life-support`), fire and repair (`damage-control`) and power
(`power-grid`) were written after this change's first draft. This change names the values it reads
from them and states how a body presents each to its player; where one of them owns a number (the
air's effects on a body and the door interlock are `life-support`'s), this change cites it.

## Goals / Non-Goals

**Goals:**
- One movement function in `sc-core`, run by the server and by the client's prediction, with
  every speed and limit in data and in SI units.
- A body that fits every crew portal of the Tern, checked against the layout, not assumed.
- Moving around the ship costs time worth planning for, and never so much that a player waits:
  every seat within 30 s of a launch bay at walking pace.
- The ship's state is felt on foot, by name: the dampers' residual (a lurch), a hit's shake, low
  gravity and zero gravity, hypoxia, carbon dioxide, smoke, cold, heat, vacuum and the pull of a
  breach.
- Injury that makes crew act for each other: incapacitated bodies, a medkit to stabilize and a carry to the medbay (not at
  full health), the medbay, and "every body down" as the loss.
- An avatar that fits the Pi 5: 3,000 triangles, at most 48 bones, baked clips, one draw call.

**Non-Goals:**
- Seats, stations, operators, body swap and NPC posts: `bridge-stations` (this change gives it
  the body).
- What fire, a breach or a damaged system is and how fast work fixes it: `damage-control`. This
  change gives the hands (reach, posture, tools, carrying) and takes the rates.
- The air: `life-support` computes pressure, gases, temperature, smoke and flow, and owns what
  they do to a body (its `crew_effects` thresholds and rates); this change says how the player
  experiences each (section 9).
- Fighter cockpits and the launch sequence: `shuttle-bay-and-fighters`.
- The deck's geometry and its checks: `deck-pipeline` (this change gives it the capsule and the
  walking rules it checks).
- Hand-to-hand combat and boarders. Nothing here assumes them; a later change may add them on
  the same body.

## Decisions

### 1. Words used here

| Word | Meaning |
| --- | --- |
| **Body** | A crew avatar in the interior frame: a player's, or an NPC body nobody drives (`bridge-stations` 6.1). At most 8 aboard (four NPC watch bodies plus players five to eight; `netcode-and-sessions` sends 8). |
| **Posture** | What the body is doing with its legs: standing, crouched, seated, climbing, in a hatch, braced, floating, knocked down, incapacitated, critical, on a bed. |
| **Hands** | What the body holds: nothing, a one-handed item, a two-handed load, a casualty, or a trolley it pushes. |
| **Felt gravity** | `g_felt = g_art + F_k.y`, along -Y (ship-frames): the generator's output plus the vertical part of the residual. |
| **Grip** | `mu * g_felt`, the most horizontal acceleration the floor can give a foot. `mu` = 0.6 (ship-frames), so grip is 5.9 m/s^2 at full gravity. |
| **Residual** | `F_k`, the part of the ship's acceleration the dampers do not cancel in compartment `k` (ship-frames section 4). Its horizontal size is `h`. |
| **Crew portal** | A layout portal a body can pass: `door`, `pressure_door`, `hatch`, `ladder` (not `window`, `bay_door` or `hoist`). |
| **HP** | Health points, 0-100: a body's condition. |

### 2. The body: capsule, eye and posture

The body collides as `deck-pipeline`'s vertical capsule. Sizes by posture (proposed; the first
three confirm `deck-pipeline`'s working values):

| Posture | Capsule radius | Capsule height | Eye above the floor | Notes |
| --- | ---: | ---: | ---: | --- |
| Standing, walking, running | 0.25 m | 1.80 m | 1.65 m | The mockup's walk mode uses this eye height (`bridge-walk-aft.png`). The radius was 0.30 m; walking the deck plan, a body could not pass between a bridge chair and the ring's railing (owner, 2026-10-08: "my collider is too wide, this chair is blocking me from walking"). 0.25 m is still wider than a person's half shoulders. |
| Crouched | 0.25 m | 1.20 m | 1.05 m | Toggle; under a low obstruction it stays crouched. |
| Seated | 0.25 m | 1.30 m | 1.20 m | `bridge-stations`' seated eye (seat point plus 1.20 m). |
| Suited (any of the above) | 0.35 m | standing 1.90 m, crouched 1.30 m | +0.05 m | The suit's pack and helmet. |
| Carrying a casualty | 0.40 m | 1.90 m | 1.65 m | The carried body over the shoulder. |
| Incapacitated, critical, knocked down | a box 1.80 x 0.50 x 0.30 m | | 0.25 m | Lower than a step (section 3): another body steps over it, so **an incapacitated body never blocks a doorway or a corridor**. |
| Floating | 0.25 m | 1.80 m, kept upright | | Section 11: "up" stays the ship's +Y, so the same vertical capsule query serves. |

**Every crew portal of the Tern passes the body that must use it** (the layout's sizes, decided;
the check is proposed for `tools/layout_check.py` and `deckc`):

| Crew portals (layout) | Count | Clear opening | Standing body (0.70 m with 0.1 m margin, 1.80 m) | Suited (0.80 m, 1.90 m) | Casualty carry (0.90 m, 1.90 m) |
| --- | ---: | --- | --- | --- | --- |
| Doors `p_ready`, `p_core`, `p_quarters`, `p_port_turret`, `p_stbd_turret`, `p_gallery_eng_p`, `p_gallery_eng_s`, `p_shield`, `p_switchboard` | 9 | 1.0 x 2.2 m | passes | passes | passes |
| Doors `p_dorsal_fwd`, `p_dorsal_aft`, `p_aft_eng`, `p_medbay`, `p_damage_control`, `p_life_support` | 6 | 1.2 x 2.2 m | passes | passes | passes |
| Doors `p_bridge_aft`, `p_torpedo`, `p_mess`, `p_spine_hangar`, `p_hangar_eng`, `p_magazine` | 6 | 1.6 x 2.3 m | passes | passes | passes |
| Doors `p_cargo` (2.0 x 2.3 m), `p_hangar_c` (2.0 x 2.5 m) | 2 | | passes | passes | passes |
| Pressure doors `p_bay_p`, `p_bay_s`, `p_airlock_inner`, `p_airlock_outer` | 4 | 1.0 x 2.2 m | passes | passes | passes |
| Ladder trunks `p_ladder_ab`, `p_ladder_bc` | 2 | 1.0 x 1.0 m | passes (climbing) | passes | passes, at half ladder speed |
| Vertical hatches `p_pod_dorsal`, `p_pod_ventral` | 2 | 0.9 x 0.9 m | passes (climbing) | passes | passes at the limit, at half ladder speed |
| Side hatches `p_pod_port`, `p_pod_stbd` | 2 | 0.9 x 1.4 m, sill 0.40 m above the floor | **by the climb-through clip only** (1.80 m does not fit; the 0.40 m sill is over a step) | by the clip | by the clip, twice its time |
| Hatch `p_drive` | 1 | 1.0 x 2.0 m | passes (0.20 m over the head) | passes (0.10 m) | passes |

The narrowest door (1.0 m) leaves 0.25 m each side of a standing body and 0.15 m of a suited
one. Nothing on the Tern today is too small; the check exists so that no later layout edit
makes it so.

### 3. Walking and running

All of this is one function, `sc-core::crew::step(body, input, deck, movers, felt, dt)`, stepped
at the server's 30 Hz tick and replayed by the client's prediction (section 14). Speeds are
proposed and replace `reference-ship-tern`'s assumptions (its question T5).

| Quantity | Value | Why |
| --- | ---: | --- |
| Walk | 2.4 m/s | Brisker than a human walk (1.8 m/s until the owner, 2026-10-08, in the engine: "walk speed is also a little slow"). The default: a player moving through the ship is walking. `bridge-stations` uses it for seat to seat times. |
| Run (hold) | 4.8 m/s (4.0 until 2026-10-08, raised with the walk) | A run, not a sprint. Unlimited while healthy: no stamina bar to manage; the air and injuries limit it instead (sections 7, 9). |
| Crouched | 0.9 m/s | |
| Backwards | 0.7 x the forward speed | |
| Wounded (under 50 HP) | 1.4 m/s, no running | Section 7. |
| Suited | 1.5 m/s, no running | Section 10. |
| Carrying a casualty or a two-handed load | 1.2 m/s, no running | Section 6. |
| Pushing a loaded trolley | at most 0.8 m/s | Section 6. |
| Stairs | 0.7 x the speed above, along the slope | `reference-ship-tern`'s assumption, kept. |
| Step up without stairs | 0.35 m | The captain's dais is 0.30 m high (layout, decided): one step. |
| Acceleration | `max(0, 20.0 m/s^2 - h)` | 0 to walking in 0.09 s, 0 to running in 0.20 s. |
| Braking | `max(0, 30.0 m/s^2 - h)` | Running to a stop in 0.13 s over 0.27 m. The first rule held both at the feet's grip (`min(6.0, grip - h)`: running to a stop in 0.68 s over 1.36 m), and walking the deck plan felt like ice (owner, 2026-10-08: "walking is to much like ice skating fix the walking please"). A player's body answers the controls at once, as in every first-person game; the residual still takes its share, and grip still decides when the lurch knocks a body over (section 12). |
| Turn | Mouse: the frame's raw displacement, never smoothed or eased. Gamepad stick: at most 200 deg/s yaw and 140 deg/s pitch, with a squared response curve. | Pale-Blue-Dot's look rule, adopted by `ship-frames`. |
| Pitch limits | +/-85 deg | |
| Jump | 3.0 m/s up: 0.46 m at full gravity, 0.61 s in the air | The first rule had none (a ship's interior has no gaps to clear). Walking the deck plan without it felt wrong (owner, 2026-10-08: "bring back jumping, its kinda weird without it"). A jump is a jump of the body's own legs, so it is higher in low gravity and none at zero. Not from a ladder, a lift car or a held action; no control in the air; the head stops at a ceiling. Question C5, answered. |
| Head bob | none | Comfort; the lurch (section 12) already moves the head when the ship does. |

**Doors cost no time at walk or run.** A door opens when a body comes within 3.0 m of it
(section 5) and takes 0.6 s; a runner's capsule (0.25 m radius) reaches the leaf 0.69 s after
entering that zone.

**Low gravity changes traction, not top speed.** With the generator at half output (`g_art` =
4.9 m/s^2), grip is 2.9 m/s^2: the same speeds, reached and shed in twice the time, and a
residual that would only sway a body at full gravity now makes it stumble. Below 2 m/s^2 of felt
gravity, feet lose the floor: section 11.

**Falling.** A body without floor under it falls with `g_felt`. Landing faster than 6 m/s costs
8 HP per m/s above 6. Railings (`deck-pipeline`'s kit) line every edge of the mezzanine, the
catwalk, the galleries and the landing, so on the Tern a fall happens only when gravity returns
under a floating body (a 3 m fall at full gravity lands at 7.7 m/s: 14 HP).

### 3a. The controller: Rapier's character controller, stairs as ramps

The owner, 2026-10-07, after walking the deck plan: "I get stuck in the stair well, need better collisders for
those stairs. I also dont like bouncing up and down on stairs if you can make that smoother that would be nice.
this fps controller is also kinda poor, is there a better controller we can use?"

**What was wrong** with the mockups' first controller (`docs/mockups/lib/shipwalk.js`, hand-written over three.js's
Octree):
- **Stuck in stairwells.** Walls were found with a capsule starting 0.65 m above the feet, and stairs collided as
  their solid steps. On a steep or spiral flight, the treads ahead hit that capsule and pushed the body back.
- **Bouncing.** The feet snapped to each tread's height, and the eye with them: a 0.2 m jump per step.

**The controller is Rapier's kinematic character controller** (dimforge Rapier: `rapier3d` in Rust, the same
library compiled to WebAssembly for the mockups, `@dimforge/rapier3d-compat`). It moves the standing capsule of
section 2 by a desired translation and returns the corrected one, with:
- **autostep** up to the step height (0.35 m) over a ledge at least 0.15 m deep;
- **snap to ground** within 0.4 m, so walking down stairs and off a ledge does not leave the floor;
- **slopes** climbed to 50 degrees and slid down beyond 60;
- **sliding** along walls instead of stopping at them.

The engine's simulation core can use the same crate (`rapier3d`, pure Rust, no GPU), so the mockup and the game
move a body by one implementation (CLAUDE.md 6.1). Recommendation taken (ask only with screenshots); whether the
core takes Rapier whole or only its character controller is `engine-stack`'s call.

**Stairs collide as ramps.** Each flight's collision is the sloped plane over its nosings (a helical ramp for a
spiral stair), and the steps are drawn but not collided with. A body walks a flight like a slope: no tread can
catch it, and its height changes smoothly. This is common practice in first-person games for the same two
reasons.

**The eye is smoothed** where the feet still change height in a step (a ledge, a platform's riser, getting off a
lift): it follows the feet with a 0.08 s time constant, and never lags them by more than 0.25 m.

**Spiral stairs reach the trunk's walls** (the owner, the same day: "make sure the edge of sprial stairs extend in
to the well as well, so as a player I cant just falkl off the side and get stuck"); `deck-access` section 2.

**A spiral's ramp is cut in rings** 0.12 m wide from the column to the wall (owner, 2026-10-08: "i cant walk up
spiral stairs"). It was one quad a tread from the column to the wall, and of the two triangles that split it,
one took its slope from its short edge at the column: about 75 degrees across the walk line, past the 62 the
controller climbs, so a body stopped a third of the way up. Cut in rings, each piece has the helix's own slope at
its radius: 44 degrees on the walk line, 57 at 0.55 m from the column, steeper only inside 0.5 m, where the
treads are too narrow to stand on anyway. Measured headless on both towers: up and down each deck, along lines
0.55, 0.85 and 1.05 m from the column, all reach the next landing.

**A lift's landing door is a wall while it is shut, and the car's floor holds the feet** (owner, 2026-10-08: "elevator
is severly broken still, I keep falling through the geometry and colliders are unreliable, I cant walk out when the
doors are open", "im stuck in the elevator well. this should be impossible"). The first walk gave the car a floor
of its own, a moving box flush with the deck, and only a soft check kept a body out of a shaft whose car was
elsewhere. Rapier's controller jammed on the seam between that box and the deck (contact at time zero, every
step), so a body that had walked in could not walk out; and the soft check froze any move near the shut door,
backing away included. Now:
- each landing door is a collider like any door, standing while the door is less than 90% open, so the shaft is
  shut on every deck where the car is not standing open;
- while the body's centre is over the shaft, nothing pulls it down and its feet are held on the car's floor (a
  jump lands there); riding, the car carries them;
- a step toward a shut shaft stops, a step away never does;
- calling the car up or down from inside first moves the body clear of the open side.

Measured in the deck plan, headless: boarding, riding and leaving on every pair of the three decks (six rides),
running and pushing diagonally at a shut door on a deck the car is not on, backing away from it, jumping in the
car, and riding while standing at its open side: all pass.

**Measured in the deck plan** (`window.MOCKUP_WALK`, headless, 2026-10-07):
- every straight flight is walked up and down (engineering's two, the hangar's two), and a stair tower from deck C to deck A;
- the eye moves at most 9 mm a frame on a straight flight and 17 mm on a spiral, against 0.2 m a step before;
- running at every wall of every compartment in eight directions for 4 s (288 runs): the first controller left the
  ship twice (through the torpedo room's and deck B's main corridor's forward walls; three.js's Octree capsule test
  only sees a triangle from its front, so a wall wound the wrong way let a body through), Rapier's none;
- a deck up a ladder takes 2.1 s, against 5.4 s.

**Found on the way:** the hangar gallery's railing stood across the tops of its stairs, and walking up them hit it.
The kit now leaves a gap where a stair lands on a railed edge.

### 4. Ladders

The Tern's ladder trunk at z = 11 m runs from deck A through deck B to deck C (`p_ladder_ab`,
`p_ladder_bc`, layout, decided); the dorsal and ventral pods are reached by short ladders through
their hatches.

| Quantity | Value |
| --- | ---: |
| Get on (walk into the ladder volume facing it within 45 deg, or Use within 1.0 m) | 0.15 s clip |
| Climb up | 2.0 m/s |
| Climb down | 2.5 m/s |
| Get off at the top or bottom | 0.15 s clip |

**Snappy, not realistic** (owner, 2026-10-07, after walking the deck plan: "the travel down ladders or up
ladders is too slow, make that snappy"). The first values (0.8 m/s up, 1.0 m/s down, 0.5 s on and off) were a
real person's; a deck took about 5 s and felt like waiting. A deck now takes about 2 s up and 1.7 s down. The
route times other changes quote from the old values (`deck-access`'s ladder rows, `walk_times.py`,
`command_suite.py`) are re-measured with the tools when this change is applied, not edited by hand.
| Rung spacing (the clip's stride) | 0.30 m |
| Bodies on one trunk | One per 1.8 m of ladder; a climber stops behind another |

- **Through, not off.** A climber going from deck A to deck C passes deck B without getting off:
  7.0 m down takes 2.8 s plus 0.3 s on and off.
- **Hands on a ladder.** A one-handed item rides clipped to the belt. A two-handed load or a
  trolley cannot be taken onto a ladder. A casualty can, over the shoulder, at half the ladder
  speeds.
- **No running, no tools, no seat claims** while climbing. A climber counts as braced (section
  12): the hands are on the rungs.
- **Climbing is predicted** like walking (section 14): the ladder volume is static deck data.

### 5. Doors, hatches and pressure

The layout decides each portal's kind and size; this change decides how a body works it.
Opening and closing times are proposed here and owned here: `life-support`'s atmosphere step uses
the same times (`atmosphere.json` `portals`, reconciled 2026-10-05: crew-on-deck owns this).
`deck-pipeline` builds the movers, `power-grid` feeds the door motors, `life-support` supplies
the pressures and owns the interlock.

| Kind (layout) | How it opens | Open, close | Unpowered |
| --- | --- | ---: | --- |
| `door` | Sliding leaf, on approach: a body within 3.0 m of the door's plane, inside its width plus 0.5 m, not locked out (below) | 0.6 s, 0.8 s; closes 2.0 s after its zone is empty | Stays as it is; a body pulls it open or shut by hand in 3.0 s (hold Use) |
| `pressure_door` | Heavy leaf, on Use only (never on approach), from either side or from its console | 2.0 s, 2.0 s | Cranked by hand: 20 s |
| `hatch` | Hinged lid, by hand: Use | 1.0 s, 1.0 s | Always by hand |
| `ladder` | An open trunk; a deck hatch at each end stands open and closes like a door on a breach (proposed; see "Interfaces") | 1.0 s by hand from the ladder | By hand |

**A door never closes on a body.** While any capsule is inside the opening, a closing leaf stops
and reopens. This holds even when a breach orders the door shut: the door waits, and the person
in the doorway is the one keeping the compartments joined (and is being pulled by the flow,
section 9).

**Locking.** A door can be locked from the damage control board (`bridge-stations` D4) or at its
own panel (hold Use 1.0 s). A locked door does not open on approach and shows red at both
panels. Anyone at the panel unlocks a door they locked by holding Use 1.0 s; overriding the
board's lock takes 3.0 s and is logged on the board ("forward switchboard door unlocked by Ana").
`damage-control` locks doors to contain fire and vacuum (its automation already does,
`bridge-stations` section 4); this is how a body meets that decision.

**The pressure interlock** is `life-support`'s (its section 3 and question L6: 20 kPa, with an
override that costs air; `atmosphere.json` `portals.interlock_max_dp_kpa`). Reconciled 2026-10-05:
life-support owns this; this change first proposed 5 kPa, with an override only up to 30 kPa. What
a body does at it is this change's:

| Pressure difference across the portal | `door`, `hatch`, ladder hatch | `pressure_door` |
| --- | --- | --- |
| Up to 20 kPa | Opens normally | Opens on Use |
| Over 20 kPa | Refuses; the panel shows both pressures. **Override**: hold Use 3.0 s at the panel (a guarded act, logged on the damage board); the opening then equalizes at the flow `life-support` computes, and the pull of section 9 applies | Refuses. No override by hand at the door: it opens only in its own cycle (an airlock or a bay sequence, `shuttle-bay-and-fighters`) or by the board's override (`life-support` section 3) |

So a body standing at a closed door with vacuum on the other side meets a red panel reading "0.4
kPa beyond: interlocked". Holding Use for 3.0 s opens it anyway, and the near compartment's air
rushes through, pulling at anyone by the door (section 9): that is how a crew member is pulled out
of a breached room, at the cost of the air (`life-support` L6), not a way to walk on. The ways to
walk on are a suit and the airlock, or `damage-control`'s repair and `life-support`'s
repressurization.

**Hatches.** The side hatches into the port and starboard pods (0.9 x 1.4 m with a 0.40 m sill)
are passed by a 1.5 s climb-through clip: the body crouches, steps over the sill and comes out
facing into the pod. The dorsal and ventral pod hatches open onto short ladders (3.5 m from
deck A up to the dorsal seat, 3.0 m from deck C down to the ventral seat). The drive hatch
(1.0 x 2.0 m) is walked through once open.

### 6. Hands: using, carrying and pushing

**Use** is one input (keyboard E, gamepad A). It acts on whatever the crosshair rests on within
**1.5 m of the eye**, with a clear line from the eye through the deck's brushes. The client shows
the object's name and the verb ("Door, locked: Open") and the pending result; the server decides
(a reliable command, `netcode-and-sessions` section 6) after checking reach, line, posture (not
incapacitated, climbing or floating without a hold) and hands. A tap uses; some things need a hold, and
the hold's time is the thing's (a door override 3.0 s, a stabilize 5.0 s, a scram reset at the
reactor 3.0 s). Sitting is a Use on a seat (`bridge-stations` section 6).

**Items** are interior objects (ship-frames section 3): they fall with `g_felt`, slide under a
residual over grip like a stumbling body, float in zero gravity, and sit in wall brackets and
lockers listed in `data/ships/tern/kit.json` (section 15). A body holds **one** thing.

| Thing | Mass | Hands | Moving with it | Its action (primary: left mouse, gamepad RT) |
| --- | ---: | --- | --- | --- |
| Extinguisher | 9 kg | One | Full speed | Spray: 6 kg of agent over 15 s, a 30 deg cone reaching 3.0 m. What it does to a fire is `damage-control`'s (`damage::extinguish`); refilled at the damage control station in 10 s. |
| Repair kit | 8 kg | One | Full speed | Repair the system under the crosshair while held: the body kneels or stands at the system's repair point and cannot move; the rate is `damage::repair_time`, the same function the damage board previews (CLAUDE.md 6.1). |
| Patch kit (plates and sealant) | 15 kg | Two | 1.2 m/s, no running, no ladders | Patch a breach under the crosshair while held; the rate is `damage-control`'s. In a venting compartment the patcher must hold position against the pull (section 9). |
| A casualty (an incapacitated body) | about 80 kg | Two, over the shoulder | 1.2 m/s; ladders at half speed; hatches at twice their clip time | Put down (Use), or onto a medbay bed (Use on the bed). Picking up takes 1.5 s, putting down 1.2 s. |
| Gannet on the magazine trolley | 1,250 kg loaded (missile 1,100 kg, `weapons-and-shields`; trolley 150 kg) | Two, pushing | Section 6.1 | Section 6.1. |

Pick up (Use) takes 0.5 s; drop (keyboard Q, gamepad Y) sets the item at the feet in 0.4 s. A
body that is knocked down or incapacitated drops what it holds. The other hand must be empty to stabilize, to climb a
side hatch, to claim a seat and to don a suit.

#### 6.1 A missile on a trolley

`weapons-and-shields` moves Gannets from the magazine racks to the ready racks by the hoist (20 s
each) and from the ready racks into a tube by the autoloader or by crew at the breech (section 10
there). The trolley is the body's part of the path when the hoist is down (unpowered or damaged):
a crew member rolls a Gannet from a rack to the hoist cage in the magazine, and from the cage to a
ready rack in the torpedo room. When that path applies, and the hoist's hand winch, are
`weapons-and-shields`'; the pushing is this change's.

- **Pushing.** Use on the trolley's handle to take it (0.5 s); then forward and back push and
  pull, and turning steers the trolley about its rear axle at up to 30 deg/s. A body pushes with
  300 N; rolling resistance is 0.01 x weight (123 N loaded), so a loaded trolley gathers speed at
  0.14 m/s^2 and is capped at 0.8 m/s: about 6 s to full speed.
- **Dead-man brake.** Released, the trolley brakes at 1.0 m/s^2 and holds. It holds against a
  residual up to 2 x grip; above that it slides as a stumbling body does, and a trolley moving
  faster than 1.0 m/s that strikes a body deals 10 HP per m/s.
- **It stays in its rooms.** The trolley is 4.6 x 0.8 m. The magazine (8 x 10 m) and the
  torpedo room (10 x 7 m) hold its work; it cannot turn through a door, so it never leaves them
  (one trolley in each). In zero gravity its rail clamps engage and it does not float.

### 7. Health, incapacitated and rescued

star-crew-64 gave officers 100 HP, let a teammate heal a downed one by +8 a press, and revived
them only at full health, 13 presses later (`main.c:88-91, 718-739`). That made revival a chore
done in a hazard. The owner, 2026-10-09, set the rule this section follows: "50% health the player
moves at half speed; lower than 20% the player or crew bot is incapacitated and requires medical
attention. Their vitals will begin dropping unless they receive medical attention and need to be
moved to the infirmary", and "Medkits can help stabilize a player's vitals". It replaces the first
draft's revive by hand: an incapacitated body is never got back on its feet where it fell.

| State | HP | What the body can do |
| --- | ---: | --- |
| Healthy | 51-100 | Everything. |
| Wounded | 20-50 | Moves at **half speed** (walk 1.2 m/s, run 2.4 m/s, half of `data/crew/walk.json`'s), ladders and stairs at half their speed too; tool work 25 % slower; the screen's edge darkens. |
| Incapacitated | 1-19 | Nothing but look (+/-60 deg), talk by voice and open the crew panel (body swap, `bridge-stations`). Lying low: the body is stepped over. **Its vitals fall** (below). |
| Stabilized | 1-19, after a medkit | As incapacitated, but its vitals hold. It still has to be carried to the medbay. |
| Critical | 0 | As incapacitated, vitals gone. Only a medbay bed brings it back, and it carries a serious injury into the campaign. |

The same states apply to players and to NPC crew (`crew-npcs`): a bot below 20 HP lies where it fell
and needs the same rescue.

- **Falling vitals.** An incapacitated body loses **0.17 HP a second** (20 HP over 120 s, the first
  draft's stabilize window, kept as the time a rescue has), on top of whatever still hurts it (it
  still lies in the heat, the smoke or the fire). At 0 HP it is critical. Bad air (smoke, vacuum,
  no oxygen) makes it critical at `life-support`'s death condition if that comes first (section 9).
  The medical console and the body's crew panel show the time left at the current rate, from the
  same function that drains it (CLAUDE.md 6.1).
- **Stabilize with a medkit.** Any body with a medkit (`safety-points`: a first aid kit from a
  cabinet, or the medic's bag) and empty other hand, within 1.2 m, holds Use for **5.0 s** (the medic
  2.0 s, `medical-officer`); the hold breaks if either body moves or is knocked down. It costs one
  dose. The falling stops: the body is stabilized until it is hurt again, which starts the fall
  afresh. A stabilize gives no HP back: the body stays incapacitated.
- **Carry to the infirmary.** A body carries an incapacitated or stabilized one over the shoulder
  (section 6: two hands, 1.2 m/s, ladders at half speed) to a medbay bed. An unstabilized casualty
  keeps losing vitals on the way, so a long carry needs a medkit first. A wounded carrier moves at
  half of that.
- **The medbay brings them round** (section 8): on a bed an incapacitated, stabilized or critical
  body is brought round after 20 s (8 s tended by the medic) at 25 HP, wounded, and the bed heals it
  on from there.
- **Field recovery.** A wounded body out of every hazard (section 9) and unhurt for 30 s recovers
  0.2 HP/s up to 50 HP: it walks at full speed again without the medbay. It never lifts an
  incapacitated body: below 20 HP only the medbay heals.
- **Every body down is the loss.** When every body aboard the ship and in its craft is
  incapacitated, stabilized or critical, players' and NPC bodies alike, the mission ends as lost
  (star-crew-64's rule, `bridge-stations` counts NPC bodies in it).
- **No permanent death.** A body critical when the mission ends carries a serious injury into the
  campaign: its maximum HP is 80 until the next resupply (vision: "crew injuries persist").
  Question C3.
- **An incapacitated player's station** passes to automation at the next tick (`bridge-stations`
  section 6), and the body slumps beside the seat, never in it.

**What hurts a body** comes from the change that owns the cause, through one function,
`crew::injure(body, hp, cause)`, which also records the cause for the debrief:

| Cause | Owner | Proposed default |
| --- | --- | --- |
| Air: hypoxia, carbon dioxide, smoke, cold, heat, vacuum | `life-support`: its thresholds, doses and rates (`atmosphere.json` `crew_effects`; reconciled 2026-10-05: life-support owns this) | Section 9 |
| Fire: standing in a fire's burning volume | `damage-control` | 8 HP/s (suited 4 HP/s) |
| A hull hit reaching a compartment | `damage-control` | Up to 40 HP at the hit point, falling linearly to 0 at 4 m |
| A console struck while seated at it | `damage-control` (star-crew-64 passed station damage to its occupant) | 10 HP to the operator |
| Striking a wall or the floor when thrown | This change (section 12) | 5 HP per m/s above 3 m/s |
| A landing | This change (section 3) | 8 HP per m/s above 6 m/s |
| A runaway trolley | This change (section 6.1) | 10 HP per m/s above 1 m/s |

### 8. The medbay

The medbay (POI 8, deck B, layout) holds the system `medbay_beds` (layout, decided). Proposed:
two beds at the system, 2.0 x 0.9 m each.

- **Onto a bed.** A body uses a free bed (0.8 s to lie down), or a carrier puts a casualty on it
  (Use on the bed while carrying).
- **Healing.** 2.0 HP/s while the `medbay_beds` load is powered (its draw is `power-grid`'s:
  8 kW nominal, 2 kW standing by; corrected 2026-10-04 from a proposed 0.3 MW), 0.5 HP/s
  unpowered. From the bed's 25 HP to full: 37.5 s.
- **Incapacitated, stabilized or critical on a bed.** Brought round after 20 s at 25 HP, then healed
  as above. The bed is the only cure below 20 HP (section 7).
- **The bed also clears the air's effects**: oxygen mask and warming, so hypoxia, carbon dioxide
  and cold stop at once on a bed.
- **Getting up** at any time: Use, 0.8 s. A player on a bed can open the crew panel and swap body
  (`bridge-stations`), so lying in the medbay is never dead time unless they choose it.

### 9. What the air does to a body

`life-support` owns what the air does to a body: every threshold, dose and rate in the middle
column is its `atmosphere.json` `crew_effects` (its design section 12), read per compartment
(sent at 5 Hz by `netcode-and-sessions`: pressure, O2, CO2, temperature and smoke). This change
owns the right-hand column: what the player sees and how the body moves. Reconciled 2026-10-05:
life-support owns this; this change first proposed its own thresholds and HP rates for the air
(hypoxia under 16, 12 and 8 kPa at 0.5 and 3 HP/s, carbon dioxide over 2, 5 and 8 kPa, smoke by a
0-1 density, cold under 10, 0 and -20 deg C, heat over 45, 60 and 80 deg C, vacuum 10 HP/s), and
those are withdrawn.

| Effect | `life-support`'s threshold and rate | What the player experiences (this change) |
| --- | --- | --- |
| **Hypoxia** | Impaired below 16 kPa of O2. Toward unconsciousness, a dose of `dt / TUC` with the time of useful consciousness by O2 partial pressure (30 min at 12 kPa, 20 min at 10.6, 5 min at 8.9, 1 min at 6.3, 9 s below 3.4); unconscious at 1. Dead after 240 s unconscious below 10.6 kPa | Impaired: no running, grey vision edges. As the dose climbs the edges close in (tunnel vision past half) and the hypoxic sway clip plays; at 1 the body is incapacitated (section 7) |
| **Hypercapnia** (carbon dioxide) | Impaired above 3 kPa of CO2. A dose by 30 min at 5 kPa, 5 min at 7, 1 min at 10, 20 s at 15; unconscious at 1. Dead after 300 s unconscious above 10 kPa | Impaired: no running, swaying vision. The dose as hypoxia's: incapacitated at 1 |
| **Smoke** | No impairment threshold. Purser's fractional effective dose, `sum(ppm x dt) / 60 / 30,000 ppm min`: unconscious at 1, dead at 2.5. Extinction coefficient `K` = 2,000 per metre times the smoke's mole fraction (`smoke.visibility_k_per_mole_fraction`) | A fog that limits sight to `3 / K` m (3 m at 500 ppm); coughing (an additive clip) while the dose rises; incapacitated at 1 |
| **Cold** | Impaired below 5 deg C (278.15 K). Harm below -20 deg C: 0.2 HP/s | Impaired: shivering clip, tool work 15 % slower. Harm through `crew::injure` (section 7) |
| **Heat** | Impaired above 45 deg C (318.15 K). Harm above 60 deg C: 0.05 HP/s per kelvin over (1 HP/s at 80 deg C) | Impaired: no running. Harm through `crew::injure` |
| **Low pressure, vacuum** | Impaired below 50 kPa. Armstrong's limit 6.3 kPa: dead after 90 s below it. A fall faster than 50 kPa in 1 s knocks a standing body down and costs 10 HP | Impaired: no running. Below 6.3 kPa the hypoxia dose climbs at its fastest (9 s), so an unsuited body is incapacitated in under 10 s; the knockdown is section 12's (1.5 s on the floor, a 1.0 s get-up, what it held dropped) |
| **The pull of a breach** | The flow toward a breach (`life-support`) | A body within 4 m of a breach in a venting compartment is pushed along the flow at up to 3 m/s (this change's rule: it is a body's motion, not an air effect) |

- **Incapacitated and critical.** `life-support`'s "unconscious" is this change's **incapacitated** (section 7),
  and its "dead" is **critical**: no permanent death (question C3). A body down in bad air turns
  critical at whichever comes first, its vitals running out or `life-support`'s death
  condition (240 s of hypoxia, 300 s of carbon dioxide, a smoke dose of 2.5, 90 s below 6.3 kPa).
- **Recovery** is `life-support`'s too: in good air the hypoxia dose falls 3.3 % a second and the
  carbon dioxide dose 1.7 %, and the screen effects fade with them.
- **Reference values** (`life-support` section 7, at cruise with eight crew working): 101.2 kPa,
  O2 21.16 kPa, CO2 0.02-0.03 kPa, air 16-27 deg C, no smoke. A healthy body in normal air takes no
  harm and is not impaired.
- **Holding on beats the pull.** A body holding a handhold or braced against a console (section
  12) is not pushed. The person who holds the doorway while others get out is a choice the game
  wants to offer.
- **A suit stops all of these** except fire (halved) and heat over 80 deg C (the suit's limit).
  A medbay bed stops hypoxia, carbon dioxide and cold (section 8).
- **The client shows them by name**: a small status line ("Hypoxia", "Smoke", "Cold") beside the
  HP, and the presentation in the table. Effects are server state (section 14).

### 10. EVA suits

The Tern keeps suits in lockers (proposed counts; `reference-ship-tern` already puts EVA suits in
damage control): four in damage control, two beside the airlock in cargo, two on the hangar
landing for pilots and bay crew. Eight suits for at most eight bodies.

| Quantity | Value |
| --- | ---: |
| Don at a locker | 20 s (legs, torso, gloves, helmet: four clips) |
| Doff | 10 s |
| Oxygen | 1,800 s; refilled at any locker in 30 s; the HUD shows minutes left |
| Speed | 1.5 m/s, no running; tools 20 % slower (gloves) |
| Capsule | radius 0.35 m, height 1.90 m |
| Protects against | vacuum, hypoxia, carbon dioxide, smoke, cold to -100 deg C; fire at half rate; heat to 80 deg C |
| Magnetic boots | On by default in a suit: walk at 1.0 m/s on any deck in zero gravity |
| Puncture | A hit that costs a suited body over 20 HP punctures the suit: its oxygen lasts 120 s unless patched (a patch kit, 5 s, on oneself or another) |

**Outside the hull** (proposed, presented here because `shuttle-bay-and-fighters` hands EVA to
this change): a suited body leaves through the airlock after its cycle, or through an open bay
door, onto the hull. It stays in ship coordinates (the interior frame's axes, so positions fit
`netcode`'s int16 centimetres), walks on the hull's outer surface with its magnetic boots at
1.0 m/s, and is tethered to the door it left by: 30 m of tether, drawn as a line. Outside, the
dampers do not reach: the body feels the full demand `D` at its position (ship-frames section 4).
Boots hold up to 6 m/s^2; beyond that the body is torn from the hull and hangs on its tether until
the demand falls, then reels in at 1.0 m/s (Use). A ship flying hard with crew outside is
therefore helm's and the captain's decision, which is the cooperation the brief asks for. EVA
flight with a thruster pack is not proposed (question C6). What work needs EVA (a sensor array,
the radiators, a jammed drop door) is `damage-control`'s and `shuttle-bay-and-fighters`'; the
hull walk surface and its collision are `deck-pipeline`'s to compile (see "Interfaces").

### 11. Zero gravity and handholds

When felt gravity falls under **2 m/s^2** (the gravity generator unpowered or failing,
`power-grid`'s; ship-frames section 3), standing bodies lose the floor and float. Seated bodies
are strapped in and stay seated.

| Movement | Value |
| --- | ---: |
| **Kick off** a surface the body touches or a rail it holds, along the look direction (the run input: Shift, gamepad left stick click) | up to 2.0 m/s |
| **Hand over hand** along a handhold rail (hold grab: keyboard Space or right mouse, gamepad LT) | 1.5 m/s along the rail with the move keys |
| **Grab** a handhold within reach | 1.0 m from the body's centre |
| **Flail** with nothing in reach | 0.1 m/s^2 in the move direction: a body stranded 2 m from a wall reaches it in about 6 s, so nobody is ever stuck |
| **Strike** a surface faster than 3 m/s | 5 HP per m/s above 3 (as section 12) |
| Bounce | restitution 0.2 |
| Suited, magnetic boots on | Walk on any deck surface at 1.0 m/s |

- **Handholds**: a rail at 1.0 m height along both walls of every corridor and on both sides of
  every door, plus the ladders, console desks and seat backs. Proposed for `deck-pipeline`'s kit
  (one generated trim, about 40 triangles per metre-run budgeted with the walls).
- **Loose items float** and drift; the trolley clamps to its rails (section 6.1).
- **When gravity returns** (the generator's ramp is `power-grid`'s), floating bodies fall and land
  as in section 3. A body holding a rail when gravity returns lands on its feet.
- **The camera floats too**: free rotation about the look axis is not offered (the body keeps
  "up" as the ship's +Y), so the decks never turn upside down on screen. Question C7.

### 12. The lurch: what the dampers' residual does to a body

`ship-frames` computes the residual `F_k` per compartment at 30 Hz and the hit shake trauma `T_k`
(sections 4 and 5 there), and proposes the body's response. This change adopts that table, with
the brace and the ladder added. `h` is the residual's horizontal size; grip is 5.9 m/s^2 at full
gravity.

| Residual `h` | Standing or walking | Braced or climbing | Seated (strapped in) |
| --- | --- | --- | --- |
| Under 0.3 m/s^2 | Nothing | Nothing | Nothing |
| 0.3 m/s^2 to grip | Camera lurch; walking speed x `(1 - 0.5 h / grip)` | Camera lurch | Camera lurch |
| Grip to 2 x grip (5.9-11.8 m/s^2) | **Stumble**: input off 0.5 s, the body slides at `(h - grip)` m/s along `-F_k` | Camera lurch | Camera lurch, clamped |
| 2 x grip to 3 x grip (11.8-17.7 m/s^2) | **Knocked down**: 1.5 s on the floor, then a 1.0 s get-up; what it held is dropped | **Stumble** | Camera only |
| Over 3 x grip (17.7 m/s^2) | Knocked down | **Knocked down**; a climber falls from the ladder | Camera only |
| Vertical: felt gravity under 2 m/s^2 | Zero gravity (section 11) | Holds on | Strapped in |
| Vertical: felt gravity over 25 m/s^2 | Forced crouch | Forced crouch | Camera only |

- **Bracing.** Hold grab (Space or right mouse, gamepad LT) near a handhold, console, seat back
  or wall within 1.0 m, or crouched anywhere: the body braces and cannot walk. When the captain
  calls "brace" (`bridge-stations` section 7), the 3 s countdown shows on every HUD. A braced body
  stands up to three times grip (17.7 m/s^2), which is `bridge-stations`' brace threshold.
- **Thrown into a wall.** A sliding or knocked-down body that strikes a wall faster than 3 m/s
  takes 5 HP per m/s above 3.
- **The camera lurch** is ship-frames' head spring (an offset up to 0.12 m and a tilt up to
  10 deg, at 1.5 Hz), added after the raw look rotation, so aiming stays exact.
- **Hit shake.** Trauma over 0.7 on a hit makes standing bodies stumble as for a residual over
  grip (ship-frames section 5); braced, climbing and seated bodies do not stumble.
- **The residual a client predicts with** is its own compartment's `F_k`, sent to that client
  only (section 14), so the prediction and the server push a body the same way.

**What it means on the Tern** (ship-frames' worked numbers): with the dampers at full power
nothing is felt in steady flight; at half power a full burn (2.5 m/s^2 left over) sways, and the
worst manoeuvre (6.1 m/s^2 in the drive section) makes standing crew stumble; at a quarter, a
full burn (8.75 m/s^2) makes everyone standing stumble and the worst case (12.35 m/s^2) knocks
them down unless they brace. Engineering's power to the dampers decides how hard helm can fly
with people on their feet.

### 13. Interaction and input

Every input device drives the body (CLAUDE.md 10): whichever keyboard, mouse or pad the player
uses. Gamepad names follow SDL3's gamepad API (A south, B east, X west, Y north; the platform
layer is SDL3, `engine-stack` section 4). Keys and buttons are data (`data/input/bindings.json`,
shared with `bridge-stations`) and can be rebound.

| Action | Keyboard and mouse | Gamepad |
| --- | --- | --- |
| Move | W, A, S, D | Left stick |
| Look | Mouse (raw) | Right stick |
| Run (hold) | Shift | Left stick click (toggle) |
| Crouch (toggle) | C or Ctrl | B |
| Use (tap or hold) | E | A |
| Item action: spray, repair, patch | Left mouse (hold) | RT (hold) |
| Grab, brace, hand over hand | Space or right mouse (hold) | LT (hold) |
| Drop the held thing | Q | Y |
| Crew panel (body swap) | F9 | Back |
| Menu | Esc | Start |

Seated bindings are `bridge-stations`' (section 8.6 there): sitting down is Use on a seat, and
standing up is that change's hold (E for 0.5 s, gamepad B for 0.6 s).

### 14. Network: the avatar is predicted

`netcode-and-sessions` decides the model (sections 4 and 5 there); this change fills in the body.

- **One movement function.** `sc-core::crew::step` (section 3) runs on the server for every body
  and on the client for its own body, from the same input frames, deck brushes, door states and
  felt residual. Corrections follow netcode: under 5 cm blended over 100 ms, above that a snap.
- **What is predicted**: walking, running, crouching, stairs, ladders, kicking off and hand over
  hand in zero gravity, and a door opening on approach (the door rule is deterministic from the
  body's position and the door's replicated state, so the client opens it for its own body
  without waiting 100 ms; a locked or interlocked door it did not know about is a correction).
- **What is not**: anything with an outcome (use, pick up, stabilize, seat claims, a stumble or a
  knockdown, damage). Those are server decisions; the client shows the pending clip at once and
  the server's result in the next snapshot.
- **Input frame** (netcode's 12 bytes): move 2 x int8, look yaw and pitch 2 x int16 (absolute,
  1/65,536 of a turn), buttons 16 bits (run, crouch, use, item action, grab, drop, and spares),
  tick 2 bytes.
- **Avatar in a snapshot** (netcode's 10 bytes): id 1 B, position 3 x int16 cm, yaw and pitch
  2 x 8 bit, posture and action 1 B (4 bits each: section 2's postures; none, use, spray, repair,
  patch, stabilize, carry, push, pick up, drop, don, doff).
- **Proposed additions to `netcode-and-sessions`:**

| Group | Encoding | Rate | Cost |
| --- | --- | --- | ---: |
| Crew status | per body: HP 8 bit, effects bitset 8 bit (section 9), held thing 8 bit, suit oxygen 8 bit | on change, at most 2 Hz | at most 64 B/s (0.5 kbit/s) |
| Felt residual, own compartment | `F_k` as 3 x int8 in 0.2 m/s^2 steps (+/-25.4 m/s^2), to each client for the compartment its body is in | 20 Hz | 60 B/s (0.5 kbit/s) |
| Commands: use, pick up, drop, stabilize start and cancel, don, doff, override | reliable, about 8 B each | on action | negligible |

That adds about 1 kbit/s to netcode's typical delta. `netcode-and-sessions` carries both groups
since 2026-10-04; its redone check is 56 kbit/s typical and about 61 kbit/s in a full engagement,
inside the 64 kbit/s budget (on WebRTC since 2026-10-07: 68 and about 73 kbit/s, inside 80).
Up, nothing changes: the input frame is the one netcode already sizes.

### 15. Data (proposed)

- `data/crew.json`: capsule and eye per posture (section 2); speeds, acceleration and braking, grip `mu`,
  step height, turn rates (section 3); ladder speeds and clip times (section 4); door and hatch
  times and the override hold (section 5; the interlock's 20 kPa is `life-support`'s
  `portals.interlock_max_dp_kpa`); item masses, hands and actions (section 6); trolley
  forces and limits (6.1); HP thresholds, the wounded speed, the vitals' fall, the stabilize hold, field recovery
  (section 7); bed rates (section 8); the breach pull and the smoke fog's visibility constant
  (section 9; the air's thresholds and rates are `life-support`'s `crew_effects`, reconciled
  2026-10-05); suit values (section 10); zero-gravity values (section 11); the lurch thresholds
  as multiples of grip (section 12). Units in the keys (`walk_m_s`, `stabilize_hold_s`, `pull_m_s`).
  Unknown keys and non-finite values stop startup (CLAUDE.md 6.5).
- `data/ships/tern/kit.json`: where items and lockers are, by compartment and bracket position:
  24 extinguisher brackets (two on the bridge, two in the main corridor, two in damage control,
  two in the hangar, three in engineering, one in every other room and passage), 4 repair kits
  (damage control 2, engineering 2), 4 patch kits (damage control 2, hangar 1, engineering 1), 3
  suit lockers (section 10), 2 medbay beds, 2 trolleys. Proposed; `damage-control` may move
  them. A validator checks each against its compartment's box and clear of door openings, as the
  layout check does for systems.

An excerpt of `data/crew.json`:

```json
{
  "capsule": { "radius_m": 0.25, "standing_height_m": 1.80, "crouched_height_m": 1.20, "seated_height_m": 1.30,
               "standing_eye_m": 1.65, "crouched_eye_m": 1.05, "seated_eye_m": 1.20 },
  "move": { "walk_m_s": 1.8, "run_m_s": 4.0, "crouch_m_s": 0.9, "back_scale": 0.7, "stair_scale": 0.7,
            "accel_m_s2": 20.0, "stop_m_s2": 30.0, "grip_mu": 0.6, "step_m": 0.35, "stick_yaw_deg_s": 200.0, "stick_pitch_deg_s": 140.0 },
  "ladder": { "up_m_s": 2.0, "down_m_s": 2.5, "mount_s": 0.15, "dismount_s": 0.15, "casualty_scale": 0.5 },
  "doors": { "sensor_m": 3.0, "door_open_s": 0.6, "door_close_s": 0.8, "close_delay_s": 2.0,
             "pressure_door_s": 2.0, "hatch_s": 1.0, "override_hold_s": 3.0 },
  "health": { "max_hp": 100, "wounded_at_or_below_hp": 50, "wounded_speed_scale": 0.5, "incapacitated_below_hp": 20,
              "vitals_fall_hp_s": 0.17, "stabilize_hold_s": 5.0, "bed_bring_round_s": 20.0, "bed_bring_round_hp": 25,
              "recover_hp_s": 0.2, "recover_to_hp": 50, "recover_after_s": 30.0, "bed_hp_s": 2.0, "bed_unpowered_hp_s": 0.5 },
  "air": { "pull_radius_m": 4.0, "pull_m_s": 3.0, "smoke_sight_constant": 3.0 }
}
```

### 16. Walk times on the Tern

Computed from the layout (a measurement instrument, CLAUDE.md 4: the script reads
`data/ships/tern/layout.json` and is proposed as `tools/walk_times.py`, task 1.3) with this
change's speeds: walk 1.8 m/s, run 4.0 m/s, stairs at 70 %, ladders 0.8 m/s up and 1.0 m/s down
plus 0.5 s on and 0.5 s off, a side hatch 1.5 s; standing up from a seat 0.9 s (a 0.5 s hold and
the 0.4 s clip) and sitting 0.4 s are included. Paths are straight lines between portal centres
at floor height, ignoring furniture, so real times are a little longer.

**As laid out today, the engineering mezzanine is an island.** The engineering bay console
(`eng_main`) and the main switchboard stand on the mezzanine (deck B level). Its only doors lead
to the hangar's galleries, which do not meet the landing; the catwalk from the aft passage has no
stair down. `reference-ship-tern` found this and proposes the patches **T1** (the galleries meet
the landing) and **T2** (stairs in engineering and the hangar); `deck-pipeline`'s walkable check
would refuse the deck without them. Routes that need a patch say so.

**Regenerated on the v2 plan, 2026-10-05** (`python3 tools/walk_times.py` on the layout whose rooms
follow the hull, `reference-ship-tern` section 1a). Only route ends moved: the bridge's forward
seats and the captain's dais 1.0 m forward, its side seats onto the raked walls, the damage
control board and the EVA lockers onto damage control's outer wall. Every portal a route uses
kept its centre, and the script's route check passes on v2 with no new waypoint. So routes from
those seats are 0.5-1.9 s longer at a walk and every other route is the same (v1 figures: quarters
to the helm 18.9 s; the helm to the engineering bay console 33.0 s and 35.4 s; the damage control
board to the forward switchboard 19.0 s; any bridge seat to a launch bay 25.7-28.2 s).

| Route | Needs | Path m | Walk s | Run s |
| --- | --- | ---: | ---: | ---: |
| Helm to the engineering bay console, by the aft passage | T2 | 55.8 | 33.5 | 15.8 |
| Helm to the engineering bay console, through the hangar | T1 | 57.7 | 35.9 | 19.3 |
| Helm to the reactor's lower floor (by the trunk and the hangar floor) | today | 57.0 | 36.7 | 21.4 |
| Quarters (spawn) to the helm | today | 28.1 | 19.5 | 11.9 |
| Mess (muster) to the helm | today | 28.1 | 19.5 | 11.9 |
| Quarters to bay control on the hangar landing | today | 21.7 | 12.5 | 5.8 |
| Quarters to the hangar floor beside the Petrel | today | 32.1 | 20.4 | 11.7 |
| Damage control board to the forward switchboard | today | 31.3 | 20.8 | 12.3 |
| Damage control board to the battery bank | today | 32.8 | 21.7 | 12.7 |
| Damage control board to the main switchboard, through the hangar | T1 | 54.0 | 30.9 | 14.4 |
| Damage control board to the main switchboard, by the aft passage | T2 | 60.4 | 39.1 | 21.1 |
| Magazine racks to tube 1's breech, on foot (the hoist takes 20 s per missile) | today | 37.2 | 24.1 | 13.8 |

**Carrying a casualty** (1.2 m/s, ladders at half speed): from the middle of the bridge to a
medbay bed, 31.8 m, **31.6 s** (20.2 s walking alone); from the engineering bay console, 51.1 m,
43.5 s (needs T1).

**Any station to the launch bays**, to the hangar side of the nearer bay's pressure door (the bays
are symmetric to within 0.6 s); boarding from there is `shuttle-bay-and-fighters`' step 1 (6 s):

| From | Needs | Walk s | Run s |
| --- | --- | ---: | ---: |
| Captain | today | 26.3 | 16.7 |
| Helm, tactical | today | 28.6 | 17.8 |
| Engineering, science (bridge) | today | 28.8 | 17.8 |
| Comms, flight operations | today | 28.0 | 17.5 |
| Engineering bay console | T2 | 15.6 | 7.5 |
| Damage control board | today | 25.2 | 14.3 |
| Bay control (by the ladder trunk today; by the gallery stair with T1 and T2) | today; T1, T2 | 23.6; 22.1 | 13.6; 10.5 |
| Dorsal gunner | today | 28.4 | 20.2 |
| Ventral gunner | today | 11.2 | 8.2 |
| Port or starboard gunner | today | 27.0 | 15.9 |
| Quarters (spawn) | today | 18.6 | 10.8 |

What the numbers say:
- **Every seat on the ship is within 28.8 s of a launch bay at walking pace** and 20.2 s running
  (the engineering bay console once T2 gives it a route): a player who decides to fly is in a
  fighter within about half a minute.
- **The core four reach their seats from the spawn in 19.5-19.6 s** (the captain in 17.1 s). The
  muster and the quarters are equally far from the helm.
- **The two routes from the helm to the engineering bay console are 2.4 s apart** (33.5 s and
  35.9 s walking), so losing either costs little. The aft passage route needs T2 and the hangar
  route needs T1; T2 was applied to the layout on 2026-10-04, so the aft passage route exists
  and the hangar route waits on T1.
- **A casualty from the bridge reaches the medbay in about 32 s**, well inside the 120 s
  stabilize window, so revival by hand on the spot and the medbay afterwards are both open.
- These replace `reference-ship-tern`'s tables, which used 1.6 m/s walking (its question T5); its
  requirement that the engineering routes stay under 40 s holds.

### 17. The avatar: geometry, skeleton, clips

**Budget** (`engine-stack`, decided): 3,000 triangles and at most 48 bones, skinned in the vertex
shader.

- **Mesh.** One crew body and one suited body, each 3,000 triangles at LOD 0, 1,000 at LOD 1
  (beyond 12 m) and 300 at LOD 2 (beyond 30 m; the main corridor is 26 m long). Flat-shaded,
  vertex-coloured from the palette atlas; the shirt takes the role colour (`ShipKit.PALETTE.role`)
  as a per-body uniform, so every crew member is one mesh. Lit like everything that moves: the
  compartment's baked light probe in its current lighting state (`light-baking`). One draw call
  per body.
- **Skeleton: 30 bones** of the 48 allowed: root, pelvis, spine 3, neck, head, jaw, clavicles 2,
  upper arms 2, forearms 2, hands 2, finger groups 2, thumbs 2, thighs 2, shins 2, feet 2, toes 2,
  and three prop bones (right hand, left hand, back for the suit pack). The palette is 30 x 3
  vec4 (90) uniforms, well inside OpenGL ES 3.0's minimum of 256 vertex uniform vectors. Two bone
  influences per vertex: a low-poly body is mostly rigid parts.
- **Skinned vertex, 20 bytes**: position 3 x int16 (with padding), normal 3 x int8, colour RGBA8,
  bone indices 2 x uint8, one weight uint8.
- **First person.** A player never sees their own body: they see first-person arms (600
  triangles, the same arm bones and clips) holding the item, as in Quake. Seated, they see the
  console (`bridge-stations`).

**Animation is baked clips** (CLAUDE.md 9, star-crew-64's rule: the base motion comes from a
file; procedural work only layers on top). Sources are committed (a Blender file or a generator
script per clip set, following the `blender-humanoid-characters` skill's reference with this
change's budgets); `sc-tools` bakes them to binary clips at 30 Hz with keyframe reduction and
int16 quaternions.

| Layer | Clips (seconds) |
| --- | --- |
| Locomotion (legs, blended by speed; playback scaled by speed so feet do not skate) | idle 2.0; walk 1.0 (one stride cycle covers 1.8 m); run 0.7; crouch idle 2.0; crouch walk 1.2; walk back 1.0; strafe left and right 1.0; limp 1.2; suited walk 1.2; turn in place 0.6 |
| Full body | sit down 0.4; stand up 0.4; seated idle 3.0; console work 2.0 (loop); ladder on at the bottom 0.5, at the top 0.5; climb up 0.75 (two rungs); climb down 0.6; ladder off at the top 0.5, at the bottom 0.5; side hatch 1.5; stumble 0.5; knocked down 0.6; get up 1.0; fall incapacitated 0.8; incapacitated idle 3.0; stabilize (kneeling helper) 1.0 loop; casualty pick up 1.5; casualty carry 1.0 loop; casualty put down 1.2; brace 0.3 and braced idle 2.0; float idle 3.0; kick off 0.4; hand over hand 0.8 loop; don suit 20.0 (four clips); doff 10.0 (two clips); onto a bed 0.8; bed idle 3.0; off a bed 0.8; push trolley 1.2 loop; repair 1.5 loop; patch 1.5 loop |
| Upper body (masked at the spine) | carry one-handed (pose); extinguisher spray 1.0 loop; use a panel 0.6; pick up 0.5; drop 0.4 |
| Additive | cough 1.0; shiver 1.0; hypoxic sway 2.0 |
| Procedural, on top | head and neck turn toward the look direction (+/-70 deg yaw, +/-40 deg pitch); ship-frames' lurch tilt on the camera only |

About 55 clips and 90 s of motion: 90 s x 30 Hz x 30 bones x 8 bytes = 0.65 MB before reduction,
about 0.35 MB after. No inverse kinematics: feet on stairs follow the walk clip (the low-poly style
forgives it, and it saves the CPU). Other players' bodies play the clip their posture and action
byte names, at the speed their interpolated positions give.

### 18. The Pi 5 budget this change spends

Against `engine-stack`'s provisional table (200,000 triangles and 300 draw calls per frame; crew
avatar 3,000 triangles, at most 48 bones; client resident memory 384 MB; 64 kbit/s down, 80 since 2026-10-07).

| Item | Triangles | Draw calls | Memory | Notes |
| --- | ---: | ---: | --- | --- |
| Eight bodies in view at LOD 0 (the bridge with every seat taken) | 24,000 | 8 | | The worst case; `bridge-stations` counts the same 24,000 |
| First-person arms and the held item | 800 | 2 | | |
| Loose items in view (extinguishers, kits), instanced by kind | about 2,000 | at most 4 | | |
| EVA tether | 2 | 1 | | Only outside |
| **Worst frame from this change** | **26,802 (13 %)** | **15 (5 %)** | | |
| Meshes: two bodies, three LODs each, and the arms | | | about 0.6 MB (25,800 + 1,800 unshared vertices x 20 B) | |
| Clips | | | about 0.35 MB | |
| Body and item state | | | under 16 KB | 8 bodies, 40 items |

- **CPU, server.** Movement for 8 bodies and up to 40 loose items at 30 Hz, four capsule traces
  each, is inside `deck-pipeline`'s estimate of under 0.1 ms per tick; air effects step with the
  systems at 10 Hz (a table lookup per body).
- **CPU, client.** Prediction replays at most 9 ticks (300 ms of round trip) of one body: under
  0.05 ms. Posing 8 bodies (two layers, 30 bones) takes about 0.05 ms on an A76.
- **GPU.** Skinning with two influences in the vertex shader; 30 bones fit the uniform palette.
- **Network.** About 1 kbit/s down more than netcode's table (section 14); nothing more up.

### 19. Interfaces to other changes

| Change | This change needs | This change gives |
| --- | --- | --- |
| `ship-frames` | `F_k` and `T_k` per compartment; `g_art`; the interior frame; the camera lurch spring; the EVA view composed like a turret sight | The body's response to the residual and the shake (section 12), adopting its table; bodies and items as interior objects |
| `deck-pipeline` | The capsule query, ladder volumes, door and hatch movers, the walkable and clear-width checks | The capsule per posture (section 2), step height, stair speed; handhold rails for the kit (section 11); a deck hatch at each end of a ladder trunk (section 5); the hull's outer surface as an EVA walk surface (section 10) |
| `life-support` | Per compartment: total pressure, O2 and CO2 partial pressures, temperature, smoke; what each does to a body (`crew_effects`: thresholds, doses, rates); the door interlock (20 kPa, with its override); the flow toward a breach; the airlock cycle; door and hatch states as openings | How the player experiences each effect (section 9); door and hatch times and the override hold (section 5), which its atmosphere step uses. Reconciled 2026-10-05: life-support owns the air's effects and the interlock, this change the door times |
| `damage-control` | Fire volumes, hull hits, `damage::extinguish`, `damage::repair_time`, breach patching rates, door locks; whether damage control teams are bodies | Hands (reach, posture, tools), `crew::injure`, the kit list (section 15) |
| `power-grid` | The gravity generator's output; door motors; the medbay beds' load | Nothing it computes; it decides what loses power |
| `weapons-and-shields` | When the trolley path applies (hoist down) and the hoist's hand winch; the breech actions | Pushing a trolley (section 6.1); the body at the breech |
| `shuttle-bay-and-fighters` | Boarding (its step 1), the suited-pilot rule, the bay interlocks | Walk times to the bays (section 16); EVA movement (section 10) |
| `bridge-stations` | Seat claim, release, relieve and swap; NPC posts; brace | The body: seat snap clip, incapacitated state, brace threshold, walk speed |
| `netcode-and-sessions` | Input frames, avatar snapshots, prediction and correction, commands | The crew status group and the felt residual (section 14) |
| `reference-ship-tern` | The layout and its patches T1 and T2 | The speeds its question T5 waits for; the walk-time table (section 16) |
| `engine-stack` | The Pi 5 budget, SDL3 input, the vertex skinning floor | This budget's spend (section 18) |
| `light-baking` | Light probes per compartment and lighting state | Nothing |

### 20. Lessons from star-crew-64

| Prototype (`docs/analysis/star-crew-64.md`) | What happened | Here |
| --- | --- | --- |
| `MOVE_SPEED 0.6` per frame at 60 frames a second | About 3.6 m/s, counted in frames | Walk 1.8 m/s, run 4.0 m/s, in seconds, in data |
| Axis-separated collision against a walkability grid | Bodies slid along grid edges | A capsule against convex brushes (`deck-pipeline`), the same query on server and client |
| Revive at +8 a press, up only at full HP (13 presses) | A chore, done standing in the hazard | Below 20 HP: a medkit stabilizes in 5 s, a carry to the medbay brings them round (owner, 2026-10-09); 120 s of falling vitals |
| All four bodies down loses the game | A good rule | Kept: every body aboard, NPC bodies included |
| Station damage passed in full to its occupant | Sitting at a console was the dangerous thing | A struck console costs its operator 10 HP (`damage-control`'s default); hits injure by distance |
| A burning room dealt 2 HP/s to everyone in it | Fire was a room-wide timer | Fire hurts in its burning volume (8 HP/s); smoke and heat hurt by their levels |
| Extinguishers single use, clearing the room; the level placed none | The system was inert | Extinguishers with 15 s of agent and a 3 m reach, refillable; brackets listed in data and validated |
| The stick rotated 45 deg for a 3/4 camera | | First person, raw mouse look |

## Risks / Trade-offs

- **Walking could be dead time.** Thirty seconds to a fighter is a long time in a fight. Against
  it: tabs on consoles mean nobody has to walk to keep a station running (`bridge-stations`), the
  run halves most times, and the routes that matter in a crisis (damage control to the
  switchboard, the bridge to the medbay) are under 32 s. If playtests find it too slow, walk and
  run are two numbers in data.
- **Revival could be too cheap.** Five seconds and up at 25 HP makes going down a setback, not a
  disaster. The 120 s window, the critical state, the medbay's 37.5 s and the campaign injury keep
  it costly. Every number is data.
- **Prediction against a ship that lurches.** The client predicts with a residual 100 ms old.
  During a hard manoeuvre that means small corrections; stumbles and knockdowns are the server's,
  so the big moves are never mispredicted, only shown 100 ms late.
- **EVA is the largest new surface** (hull collision, tether, the exterior view). It is proposed
  but can ship after the interior without changing anything else here.
- **No inverse kinematics** means feet float a little on stairs and ladders line up by timing.
  The low-poly style forgives it; a later change can add two-bone IK if the probe leaves time.
- **The mezzanine island** blocked two key routes until `reference-ship-tern`'s T2 stairs were
  applied (2026-10-04); the hangar route still waits on T1.

## Open questions

Ids C (crew). Per CLAUDE.md 13 a question goes to the owner only with something to look at; the
shots are in `docs/screenshots/mockups/`. The rest take the recommendation, recorded as
"recommendation taken (ask only with screenshots)".

| Id | Question, and the fact it turns on | Options | Recommendation | Shot |
| --- | --- | --- | --- | --- |
| C1 | Does the bridge read at the right scale from a 1.65 m eye, with a 0.30 m body radius? The room is 14.0 x 11.0 x 3.0 m and the aft door 1.6 x 2.3 m (layout). | 1.65 m eye / a taller 1.75 m eye / a shorter 1.55 m eye | 1.65 m: an average adult's eye, and the 3.0 m ceilings read as a ship, not a hall | `bridge-walk-aft.png`, `bridge-captain-view.png` |
| C2 | Walking by default with a held run, or running by default with a held walk? | Walk 1.8 m/s default / run 4.0 m/s default | Walk by default: the ship feels its size and a run is a decision. Recommendation taken (ask only with screenshots) | none |
| C3 | Can a body die for good in a mission? | No: incapacitated, critical, and a campaign injury / yes, after a critical timer | No permanent death: friends playing together should not lose a player to one bad fire. Recommendation taken (ask only with screenshots) | none |
| C4 | Revive by hand in 5 s at 25 HP, or with a medkit only? | By hand / medkit only / both (medkit faster) | Superseded by the owner, 2026-10-09: no revive in the field at all; a medkit stabilizes and the medbay brings them round (section 7) | none |
| C5 | Jumping? | None / a small hop | A small hop (section 3): the owner, 2026-10-08, "bring back jumping" | A small hop |
| C6 | EVA outside the hull: a magnetic-boot walk on a tether, or free flight with a thruster pack? | Boots and tether / thruster pack / both | Boots and tether: simpler to build, and the ship's motion stays a danger that helm controls. Recommendation taken (ask only with screenshots) | none |
| C7 | In zero gravity, may a body roll freely (decks upside down on screen)? | Keep "up" as the ship's +Y / free roll | Keep "up": readable and kind to stomachs. Recommendation taken (ask only with screenshots) | none |
| C8 | Run unlimited, or a stamina bar? | Unlimited / stamina | Unlimited: the air and injuries already limit it (section 9). Recommendation taken (ask only with screenshots) | none |
| C9 | Should ladder trunks carry deck hatches that close on a breach (a pressure boundary between decks)? | Hatches / open trunks | Hatches: otherwise one breach on deck A empties three decks. Recommendation taken (ask only with screenshots) | none |
