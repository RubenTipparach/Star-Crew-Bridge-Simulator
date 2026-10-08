# Proposal: a security station, boarders and crew orders

## Why

The owner, 2026-10-08: "security station: on the bridge there should be a security station monitoring
bording parties or ship security making sure enemies arent bording the ship and asssigning crew
members to attack somewhere if there are intruders".

Today every change excludes boarding (`crew-on-deck` non-goals, `weapons-and-shields`,
`damage-control`), and nothing can give an order to a body but the damage control teams'
automation. `ship-frames` already says how a boarder exists: "crew bodies in the boarded ship's
interior frame, simulated exactly like its own crew". This change gives the ship a station that
watches its own decks, sees boarders come in, locks them out, and sends crew to meet them.

## What Changes

- **A security station on the bridge.** The command suite's port status board slot (-22.5 degrees on
  the port wall, `command-suite` design 2) becomes the security console; the bridge seats eight.
- **Internal sensors.** Every compartment counts the bodies in it, ours and others, from the
  compartment graph; the count of a compartment whose sensor is damaged or unpowered reads "?".
- **Boarding.** Boarders enter through an airlock, the docking port, a hull breach, or a boarding pod
  that cuts its own breach. Each way in is an entry point on the security plan, and an alarm sounds
  on every console when one opens with a hostile body near it.
- **Lockdown.** The security console locks and unlocks any door and seals any pressure door through
  the damage board's lock (one implementation, `crew-on-deck` section 5), and can lock down a whole
  deck at once.
- **Crew orders.** The security officer orders a body or a team to a compartment: go, hold, or clear.
  NPC watch bodies and damage control teams obey it; a player sees it as a marker and a line on the
  HUD and may ignore it.
- **Boarders' behaviour**, kept simple: they go for a target (the bridge, the reactor, the shield
  generator), sabotage it, and fight what is in the way, with the weapons of `armory`.

## Capabilities

### New Capabilities
- `security`: the security console, internal sensors, entry points and alarms, lockdown, crew
  orders and boarders.

### Modified Capabilities
None now. `bridge-stations` (a role and its console), `command-suite` (a slot on the bridge) and
`crew-on-deck` (bodies that take orders) take this change's deltas when it is built.

## Impact

- Depends on `armory` for weapons and on `crew-on-deck` for bodies, doors and locks.
- `bridge-stations`: role `security`, its console, automation (locks entry points and sends the
  watch when no player holds it).
- `ship-frames`: a boarding pod or a docked enemy craft hands its bodies into the Tern's interior frame.
- `netcode-and-sessions`: hostile bodies are bodies; up to 6 boarders at once (budget below).
- Pi 5 budget: up to 6 more bodies (avatars, 1 draw call each, about 1,500 triangles each at the
  far LOD) and their simulation on the server.
- Recommendation taken (ask only with screenshots) for every number here.
