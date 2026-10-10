# Proposal: repairs on deck

## Why

The owner, 2026-10-09: "lets call out how we would perform these mechanics in the games 3d environment". The repair
mini-games (`repair-minigames`) are flat 2D screens, played in a browser menu. In the game a repair happens in the
ship: somebody notices the damage, takes a kit, walks to the machine, kneels at it and works while the ship is
fighting around them. Nothing yet says how a player gets from walking the decks into a mini-game and back, what the
rest of the crew sees while it happens, or what the server checks.

## What Changes

- **The loop in the ship** (design 1): see the damage (the board, the ship map, the machine itself), take a kit and
  a part, walk there, dock at the repair point, play the rounds, stand up.
- **The repair point** (design 2): every repairable thing gets a `repair` use point on its prop, with a posture, a
  camera framing and a service face that opens.
- **Docking** (design 3): Use with a kit snaps the body to the point, opens the service face and moves the camera to
  the framing; the game is drawn over the live 3D view, which keeps running at its edges.
- **What the room sees** (design 4): the repairer kneels with the kit open, each round that lands changes the machine
  (lamp, sound, smoke), and a fumble's hazard happens in the room, hurting whoever is near.
- **Every system's place and cues** (design 5): one table, the twenty-two repair jobs.
- **The server's part** (design 6): the game is played on the client; the server owns the job, checks every landed
  round and applies fumbles and hazards.
- **A three.js mockup** (design 9): `docs/mockups/repairs-on-deck.html`, engineering in 3D, walk up to the coolant
  pump and the reactor, dock, play the real mini-game over the live room, stand up.

## Impact

- `repair-minigames` (its games are what is played; its rounds rule, 1 and 1a), `damage-control` (the board, kits,
  parts, 6a), `crew-on-deck` (hands, Use, postures, input), `ship-interactables` (use points and fixture state),
  `ship-props` and `engineering-fitout` (the service faces on the props), `crew-npcs` (bots repair without a screen),
  `netcode-and-sessions` (the intents), `ship-plan-view` (job markers), `bridge-stations` (consoles are drawn the
  same way: 2D over the live view).
