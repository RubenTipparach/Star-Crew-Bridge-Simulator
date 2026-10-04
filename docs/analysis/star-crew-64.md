# star-crew-64: what carries over to the PC and Raspberry Pi 3 bridge simulator

An analysis of `RubenTipparach/star-crew-64` at `b9f9c9a` (2026-10-04), the N64 prototype of
this game (libdragon and tiny3d, C, four players on one console). Paths are relative to that
repository. Timings there are in frames at an assumed 60 Hz.

The main files are `src/main.c` (1,594 lines, all the game policy), `src/ship_view.c/.h`
(1,506 + 401 lines: the space combat simulation **and** the corner-viewport renderer), the four
console modules `src/{bridge_panel,weapons_console,engineering_console,science_console}.c`,
`src/fire.c`, `src/extinguisher.c`, `src/missions.c`, `src/lobby.c`, `src/level.c`,
`tools/compile-levels.py`, `tools/gen-*.py`, `gamedesign.md` and `CLAUDE.md`.

**Note:** `gamedesign.md` marks phases 1-9 as done, and the code confirms it. Almost all of
that work landed in one 4,500-line commit, `b07fe65 "two player yay"` (32 files). The doc's
tuning table (`gamedesign.md:424-441`) is out of date against the code: shields are 40 per
face, not 60; regeneration is 1/s per face, not 4/s; enemy fire is about one shot per 14-20 s
per fighter, not 2 s.

## 1. Gameplay systems

### Crew, stations, input (`main.c`)

- **Four bodies are always spawned** (`main.c:399-428`), with shirt tints red, blue, yellow
  and green (`:392-397`). `PlayerSlot.controlling_port` is -1 for an NPC (`:113-132`).
  Officers have 100 HP. Walking speed is `MOVE_SPEED 0.6` (`:28`). The stick is rotated 45
  degrees to match the 3/4 camera (`:160-164`). Collision is axis-separated against grid
  walkability and console boxes (`:169-182`).
- **Stations are hard-coded in `main.c`, not taken from level entities** (`:45-67`): helm
  (-180, 10) in the bridge room, weapons (110, 0) in engines, engineering (-30, 90) in the
  mess, science (-30, -90) in quarters.
- **Common console pattern:** engage radius 35 u; walk up and press A, and the character snaps
  to a seat 18 u in front, facing the panel. B leaves. One occupant per console
  (`occupant_pid`), with per-pad edge detection (`bridge_panel.c:10,24,115-166`). Each
  console is a file-static singleton.
- **Helm** (`bridge_panel.c:137-166`): stick X sets steer and stick Y sets impulse, both in
  [-1, 1], so the ship can reverse; lerped at 0.25 and 0.10. Ship physics
  (`ship_view.c:20-23,419-439`): turn 0.02 rad/frame, acceleration 0.06 u/frame^2, damping
  0.985/frame, maximum speed 1.5 u/frame; turn and acceleration are both multiplied by
  `power[ENGINES]/33`, so 0% engines means no control. The ship coasts when the seat is empty.
- **Weapons** (`weapons_console.c:10-18,141-180`): stick X aims +/-25 degrees off the bow
  (`AIM_LIMIT 0.4363`) at 0.045 rad/frame, auto-centring when idle. A fires a phaser
  (12-frame cooldown); Z fires a torpedo (45 frames). There is no aim reticle anywhere.
- **Engineering** (`engineering_console.c`): energy defaults 33/33/34 (`:80-82`). Stick X
  flicks pick the channel; stick Y pumps it at 1.6 per frame (`:13,16,218-229`).
  `rebalance_energy` takes the delta from the other two channels in proportion to their
  values, then renormalises to 100 (`:135-166`). Hold Z for repair mode: stick X cycles the
  target station; repair runs at 5 HP/s times engineering's own station HP share
  (`:201-212,296-312`), and power edits are frozen while repairing. C-down vents the target
  station's room: clears its fire, deals 30 HP to everyone in the room, 600-frame cooldown
  (`engineering_console.h:30`, `main.c:652-675`). A "repair pulse" (240-frame buff) is
  cosmetic only.
- **Science** (`science_console.h:17-32`, `.c:168-209`) is a rhythm minigame: notes travel
  for 90 frames with a hit window of +/-0.10; a hit adds 6 HP to the selected shield face, a
  miss costs 3. It does not move capacity between faces as the design doc proposed.
- **Gating** (`main.c:613-622,636-644,687-701,926-942`): a station at 0 HP stops working.

### Power, shields, heat, weapons (`ship_view.h`, `ship_view.c`)

- **One scaling rule drives everything:** `share = power[ch] / 33` (`ship_view.h:66-70`,
  `.c:502-503`). At 100% in one channel, that channel runs at about 3x.
- **Shields:** six faces, 40 HP each times the shield share, regeneration 1.0/s per face
  times the share, with a fractional accumulator (`.h:80-82`, `.c:504-519`). Face order: bow
  +Z, stern -Z, port +X, starboard -X, dorsal +Y, ventral -Y (`.h:86-93`).
- **Heat:** maximum 100; phaser +8, torpedo +25; dissipates 15/s times the weapons share;
  firing refused at 100 (`.h:95-98`, `.c:949,962`).
- **Projectiles** (`.h:26-35`): phaser speed 4.0, 60 frames, damage 1; torpedo speed 1.8, 150
  frames, damage 3; enemy bullet speed 2.5, 120 frames, damage 5. All share one 16-slot pool.
  Combat is effectively planar.
- **Hit spheres and HP:** ship radius 12, hull 100; station HP 50 each; fighter radius 10, 3
  HP; capital radius 20, 30 HP (`.h:40-49,107-124`).

### Damage routing (`ship_view.c:870-939`, `main.c:949-1037`)

1. The bullet's world offset is rotated into ship-local space.
2. **Shield face** = the dominant axis of that offset (`.c:909-915`).
3. **Station** = a four-way angle test, independent of the face (`.c:897-903`).
4. Only the face hit absorbs; **overflow** hits the hull and the station for the same amount.
5. `main.c` passes any station HP drop in full to the seat's occupant, and feeds the room's
   fire counter (`main.c:961-997`).

### Officers, healing, loss

- A walking officer pressing A within 18 u of a downed teammate heals +8 per press. The
  teammate revives only at full HP (13 presses) (`main.c:88-91,718-739`).
- The game is lost when the hull is destroyed or when all four bodies, NPCs included, are down
  (`main.c:1478-1494`).

### Fire, extinguisher, vent

- **Three hits within 180 frames ignites a room** (`fire.h:25-29`, `fire.c:31-50`). A fire
  never goes out on its own. A burning room deals 2 HP/s to every officer in it and 0.5 HP/s
  to its station (`main.c:1004-1037`).
- Extinguishers are single use and clear the room you stand in. The shipped level places none,
  so the system is inert there.

### Enemy AI (`ship_view.c:556-835`)

- **Fighter:** `ORBIT` 3-5 s at radius 80 with an unclamped radial spring; `ATTACK_RUN` until
  within 40 u; `FIRE` for 30 frames, one unled bullet; `RETREAT` 10 s. Timers start
  randomised; half circle each way.
- **Capital:** drifts on a radius-120 orbit, fires a four-bullet burst every 180 frames.
- **Dummy:** stationary targets on a ring, for a tutorial.

### Missions, lobby

- Seven static missions, each a roster of up to four enemy types; the win is "kill the
  roster" (`missions.c:3-53`).
- Any connected pad readies with A; any ready pad holds START for 180 frames to launch; empty
  slots become NPCs (`lobby.c:97-206`).

### NPCs and body swap

- L/R moves a player to the next free NPC body (`main.c:562-597`). NPCs wander their home
  room, take damage, can be healed and count toward the loss condition.
- **Bug:** swapping away from a seated body leaves the console locked to the NPC, and the
  helm's last steer value frozen, so the ship keeps turning (`main.c:794,940-942`).

## 2. Level format

- **JSON** (`levels/starting.json`, 30 x 20 grid): `{version, grid{w,h}, rooms[{id,x,y,w,h,name}],
  hallways["x,y",...], entities[{id,x,y,group}], groups{name:"#hex"}}`.
- **Compiler** (`tools/compile-levels.py:49-137`) writes a big-endian `.lvl` (magic `STLV`,
  tile bytes, entities, room names, a per-cell room id).
- **Runtime:** one tile is 20 u (2 m); walls only on the -X and -Z edges, a cutaway for the
  fixed camera. A per-cell room id drives room-scoped systems (fire).

## 3. Asset pipeline

Deterministic stdlib-only Python generators; the OBJ is the source of truth, a JSON sidecar
feeds the browser editor, and a `gen-*-c.py` bakes a C header. Ship 26 triangles, fighter 24,
bridge panel 16, weapons panel 26, character 84 (seven boxes), textures 32 x 32. A browser
level editor (with a "generate spaceship" button that joins 6-9 rooms by a minimum spanning
tree of corridors) and a three.js model editor. Asset unit scales vary (10, 14 and 30 units
per metre).

## 4. Rendering

- A fixed 3/4 camera that frames only the first two players.
- Flat per-triangle lighting with four point lights at fixed coordinates unrelated to the
  level.
- **The exterior is a 120 x 90 picture-in-picture** in the corner, with its own viewport,
  scissor, depth clear, lighting and a chase camera that follows position but not yaw.
- **Interior and exterior are decoupled:** the bridge never reflects the ship's heading, and
  the interior's stars sit on a static shell.

## 5. What worked and what was painful

**Worked:** the phased design doc with tuning tables; one power-share formula used by every
system; a thin event interface between consoles and the simulation; generators plus baked
artefacts; browser editors with autosave; CI that builds the ROM and a WASM emulator for
instant playtests; port-agnostic menus.

**Painful:** fighting the N64 renderer (stars as 8 x 8 squares, CI4 alpha loss), an unresolved
crash on real hardware with two players, vague commit messages and a 4,500-line phase dump,
a 1,594-line `main.c` with function-static state, simulation mixed with rendering, enums kept
in step by hand, mixed unit scales, and dead code.

## 6. Recommendations for Star Crew

**Carry over as design:**
- Four stations reached on foot; one seat per console; NPC bodies plus body swap so 1-4
  players all work; downed officers revived by a teammate; "all crew down" as a loss.
- Zero-sum power with proportional rebalancing; the cheap-shot and heavy-shot heat model; the
  forward arc that makes helm and weapons cooperate.
- Six shield faces chosen by the dominant local axis; only overflow bleeds through; station
  damage passes to its occupant.
- Fire that persists until crew act: an extinguisher, or a vent that trades crew safety for a
  clear room.
- The fighter state machine with randomised phases; capital burst fire; dummy targets for a
  tutorial; missions as a roster.
- Rooms with a per-cell room id for room-scoped systems (now the compartment graph); the
  generator, baked artefact and browser editor workflow; asset budgets that are trivial for a
  Pi 3.
- Off-screen threat arrows; projected HP bars.

**Do not carry over:**
- N64 specifics (packed vertices, fixed-point UVs, the 2D star blit, CI4 rules, 320 x 240 text,
  a big-endian level file).
- Timings counted in frames; use seconds and a fixed-step simulation.
- The monolithic `main.c`, singleton consoles, function-static state, and a module that both
  simulates and renders.
- Station positions and lights hard-coded in C; they belong in the layout.
- A shared player and enemy projectile pool; a camera that frames two players; mixed unit
  scales.
- A tiny picture-in-picture as the only view of space. Keep the separate-scene idea, but drive
  interior and exterior from one ship state (`openspec/changes/ship-frames`).

**Reconsider:** science as a rhythm game rather than real face balancing; the cosmetic repair
pulse; the unclamped orbit spring; revive only at full HP; seats left locked by NPC bodies.
