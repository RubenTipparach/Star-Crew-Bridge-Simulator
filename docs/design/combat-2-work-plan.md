# Combat 2: who builds what (2026-10-10)

The owner, 2026-10-10: "seperate this work out, and coordinate work with other sessions", "you are in charge",
"delegate work out so we get more done". Pi 1's session coordinates (CLAUDE.md 13: messages carry the ask, git carries
the results, the plan lives here).

The design is five changes, written up and validated on `feat/combat-2`:
`laser-banks`, `armour-and-missiles`, `enemy-fighters`, `ship-damage`, `objectives-tab`. Read the one you build, and
the owner's words in `laser-banks`' proposal and `ship-damage`'s.

## Workstreams

| # | Who | Branch (from `feat/combat-2`) | Builds | Owns these files |
| --- | --- | --- | --- | --- |
| A | Pi 1 (coordinator) | `feat/combat-2` | `laser-banks` and `armour-and-missiles` in the core; the new stations and commands; the Hound's AI; automation and bots; the protocol; the server's seats; integrating B and C; the recordings | `sc-core/src/combat/mod.rs`, `automation.rs`, `data.rs`; `sc-net/*`; `sc-server/*`; `sc-client/src/seat.rs`, `drill.rs` |
| B | The cloud session | `feat/ship-damage` | `ship-damage` design 1-5 and 7 as a pure module: the hull test, armour by weapon (`armour-and-missiles` 1), the march, the cascade, capability, fires and the two teams; the Hound's layout | `sc-core/src/combat/damage.rs` (new), `data/enemies.json` (`layout` block), `data/ships/tern/damage.json` (status) |
| C | Pi 2 | `feat/fighters` | `enemy-fighters` design 1 and 4 as a pure module: the Jackal's state, flight, AI and guns; its data | `sc-core/src/combat/fighters.rs` (new), `data/enemies.json` (`fighter` block) |
| D | The Mac | `feat/combat-consoles` | The consoles: mockups first (`docs/mockups/consoles.html`) for LASER PORT and LASER STBD, the GUNNER sight, Tactical's DECOY and 20-missile magazine, and the OBJECTIVES tab on every station; then the engine consoles at parity; the objectives' kinds in the mission file | `sc-client/src/console/*` (new modules and the view structs), `docs/mockups/consoles.html`, `tools/consoles/*`, `data/missions/*.json` |

Nobody edits another stream's files. A stream that needs a change in someone else's file says so in its commit message
and in a message to Pi 1.

## The seams

### B: `combat::damage` (pure: no `Drill`, no network)

```rust
/// A ship's insides, loaded once: hull sections, rooms (brush prisms), loads (system, centre, node), nodes, conduits.
pub struct ShipLayout { /* from layout.json + power.json for the Tern; enemies.json `layout` for the Hound */ }
impl ShipLayout { pub fn tern(root: &Path) -> Result<Self, String>; pub fn from_enemy(data: &EnemyLayout) -> Result<Self, String>; }
/// The live state: 54 (or the Hound's) armour sections, each load's integrity, node health, conduits, fires, teams.
pub struct DamageState { .. }
impl DamageState { pub fn new(l: &ShipLayout) -> Self; }
pub enum Weapon { Cannon, Laser, Missile }
/// Ship frame throughout: `p0 -> p1` is the shot's path (a bolt's step, a beam's ray, a blast point and the centre).
pub fn hull_entry(l: &ShipLayout, p0: DVec3, p1: DVec3) -> Option<DVec3>;
/// What passes the shield arrives here; returns what happened for events and the structural hull.
pub fn resolve_hit(l: &ShipLayout, s: &mut DamageState, cfg: &DamageData, entry: DVec3, dir: DVec3,
                   energy_mj: f64, weapon: Weapon, hit_id: u64, seed: u64) -> HitOutcome;
pub struct HitOutcome { pub section: (u8, u8), pub absorbed_mj: f64, pub passed_mj: f64,
                        pub systems: Vec<(usize, f64)>, pub bursts: Vec<(DVec3, f64)>, pub fires: Vec<usize> }
/// Capability 0-1 of a system by its layout id (the drill reads "impulse_drive", "shield_generator", ...).
pub fn capability(l: &ShipLayout, s: &DamageState, system: &str) -> f64;
/// Fires burn, teams travel and work; `priority` is the captain's raised system, if any.
pub fn tick(l: &ShipLayout, s: &mut DamageState, cfg: &DamageData, dt: f64, priority: Option<&str>) -> Vec<DamageEvent>;
```

Pi 1 calls `hull_entry` and `resolve_hit` from the drill's hits, after the shield, and `capability` where the table in
`ship-damage` 4 says. The tests read as sentences and include the owner's cascade.

### C: `combat::fighters` (pure)

```rust
pub struct FighterData { .. }               // enemies.json `fighter`
pub struct Fighter { pub id: u16, pub pos: DVec3, pub vel: DVec3, pub rot: DQuat, pub hull_mj: f64,
                     pub shield_mj: f64, pub alive: bool, pub state: FighterState, .. }
pub struct Mothership { pub pos: DVec3, pub rot: DQuat, pub vel: DVec3 }      // where they launch
pub struct Prey<'a> { pub pos: DVec3, pub rot: DQuat, pub vel: DVec3, pub parts: &'a [DVec3] } // ship-frame aim points
pub struct Shot { pub owner: u16, pub pos: DVec3, pub vel: DVec3, pub damage_mj: f64, pub life_s: f64 }
pub fn launch(m: &Mothership, d: &FighterData, ids: [u16; 2]) -> Vec<Fighter>;
pub fn step(f: &mut [Fighter], d: &FighterData, prey: &Prey, threatened: &[bool], dt: f64, rng: &mut Rng) -> Vec<Shot>;
pub fn hit(f: &mut Fighter, d: &FighterData, dmg_mj: f64) -> bool; // true when destroyed
```

### D: the consoles' views (in `console/mod.rs`, D owns the structs; A fills them in `seat.rs`)

```rust
pub struct LaserView { pub bank: String, pub charge: f32, pub secs_to_full: f32, pub min_charge: f32,
                       pub bearing: bool, pub hit_chance: f32, pub spot_face: String, pub spot_shield: f32,
                       pub spot_armour: f32, pub capability: f32 }
pub struct GunnerView { pub aim: [f32; 2], pub targets: Vec<GunTarget>, pub chosen: Option<usize>,
                        pub capacitor: f32, pub heat: f32, pub trigger: bool }
pub struct GunTarget { pub id: u16, pub kind: String, pub rel: [f32; 3], pub lead: [f32; 3], pub hull: f32, pub shield: f32 }
pub struct ObjectiveView { pub title: String, pub clock_s: f32, pub rows: Vec<ObjectiveRow> }
pub struct ObjectiveRow { pub words: String, pub state: String, pub number: Option<String> }
// Tactical gains: magazine: u32 (of 20), decoys: u32 (of 5), inbound: Vec<f32> (seconds to impact).
```

Until A's simulation lands, D fills these from fixtures (`--console-fixture`) as the console parity check does.

## Order

1. Now: A, B, C and D start from `feat/combat-2`.
2. B and C push their branches as they go; A merges them into `feat/combat-2` and wires them in.
3. D's mockups go to the owner's survey for approval; the engine consoles follow.
4. A records a whole fight on the Pi when B and C are wired: lasers, missiles, decoys, fighters, a part breaking, a
   fire, a repair.

Reports go in each change's `tasks.md` and in commits; a number that lives only in a message is not a result.
