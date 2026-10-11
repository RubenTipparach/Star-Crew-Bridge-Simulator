//! Enemy fighters (openspec/changes/enemy-fighters): the Jackal's state, flight, AI and guns. Pure: no drill, no
//! network. Workstream C builds it (docs/design/combat-2-work-plan.md); the seam is in that plan.
//!
//! A Jackal flies by the drill's one flight rule: [`steer_toward`] sets its stick and [`fly`] moves it, with its own
//! [`FlightBlock`] from `data/fighters.json` (CLAUDE.md 6.1). Its guns lead with the drill's [`lead`]. The drill
//! launches the pair, steps them every tick, turns their [`Shot`]s into bolts and calls [`hit`] when one is struck.
//!
//! The AI (enemy-fighters design 1, weapons-and-shields design 13): orbit the prey at 800 m; after 12-20 s (seeded)
//! pick one of the prey's parts, fly out to the side that part faces, run in at it firing from 1,200 m to 300 m, break
//! off and climb back out to the orbit, and repeat. It jinks while a turret bears on it (`threatened`).

use super::data::{FlightBlock, FlightFile};
use super::{fly, lead, steer_toward, HelmMode, Ship, Side};
use crate::data::{Checks, DataError, Validate};
use crate::rng::Rng;
use glam::{DQuat, DVec3};
use serde::Deserialize;

/// Where and when the fighters leave their mothership.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LaunchBlock {
    /// Seconds into Engage the drill launches them.
    pub after_engage_s: f64,
    /// Seconds between one fighter leaving and the next.
    pub spacing_s: f64,
    /// The hangar's mouth, metres, the mothership's frame.
    pub hangar_m: [f64; 3],
    /// Speed it leaves at, m/s, along the mothership's bow and on top of the mothership's velocity.
    pub exit_mps: f64,
}

/// A fighter's guns, firing together.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GunsBlock {
    /// Damage each bolt delivers, MJ.
    pub damage_mj: f64,
    /// Bolts a second, all guns together.
    pub rate_hz: f64,
    /// Bolt speed relative to the fighter, m/s.
    pub speed_mps: f64,
    /// Seconds a bolt lives.
    pub life_s: f64,
    /// It fires only while its nose is within this angle of the lead, degrees.
    pub fire_cone_deg: f64,
}

/// How a fighter attacks.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FighterAi {
    /// The orbit's radius around the prey, metres.
    pub orbit_m: f64,
    /// Speed on the orbit, m/s.
    pub orbit_speed_mps: f64,
    /// How hard it steers back to the orbit's radius: radial weight per fraction of the radius off (clamped to 1).
    pub orbit_radial_gain: f64,
    /// Seconds on the orbit before the next run, the least and the most (seeded between).
    pub run_every_s: [f64; 2],
    /// How far out from the chosen part, on the side it faces, the run starts, metres.
    pub setup_m: f64,
    /// The run starts within this of that point, metres.
    pub setup_arrive_m: f64,
    /// ... or after this long getting there, seconds.
    pub setup_timeout_s: f64,
    /// Speed on the run, m/s.
    pub run_speed_mps: f64,
    /// It opens fire inside this range of the part, metres.
    pub fire_from_m: f64,
    /// It breaks off inside this range, metres.
    pub fire_to_m: f64,
    /// It breaks off after this long on the run, seconds.
    pub run_timeout_s: f64,
    /// It is back on the orbit after this long breaking off, at the latest, seconds.
    pub break_off_timeout_s: f64,
}

/// `data/fighters.json`: the Jackal.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FighterData {
    /// `starcrew.fighters/1`.
    pub schema: String,
    /// A stable id (`jackal`).
    pub id: String,
    /// What a console calls it.
    pub name: String,
    /// Hull, MJ.
    pub hull_mj: f64,
    /// The shield bubble's capacity, MJ.
    pub shield_mj: f64,
    /// The bubble's regeneration, MJ/s.
    pub shield_regen_mj_s: f64,
    /// Bolts within this of its centre hit it, metres.
    pub hit_radius_m: f64,
    /// The launch.
    pub launch: LaunchBlock,
    /// Flight, in the ships' schema.
    pub flight: FlightBlock,
    /// Guns.
    pub guns: GunsBlock,
    /// Behaviour.
    pub ai: FighterAi,
}

impl Validate for FighterData {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.fighters/1");
        c.number("hull_mj", self.hull_mj, 0.1, 1e4);
        c.number("shield_mj", self.shield_mj, 0.0, 1e4);
        c.number("shield_regen_mj_s", self.shield_regen_mj_s, 0.0, 1e3);
        c.number("hit_radius_m", self.hit_radius_m, 0.1, 100.0);
        let l = &self.launch;
        c.number("launch.after_engage_s", l.after_engage_s, 0.0, 3600.0);
        c.number("launch.spacing_s", l.spacing_s, 0.0, 600.0);
        for (i, v) in l.hangar_m.iter().enumerate() {
            c.number(&format!("launch.hangar_m[{i}]"), *v, -1000.0, 1000.0);
        }
        c.number("launch.exit_mps", l.exit_mps, 0.0, 1000.0);
        // The flight block is the ships' schema, checked by the ships' own rule; its fields are named `flight.*` here
        // as they are in a ship's flight.json.
        FlightFile { schema: "starcrew.flight/1".into(), flight: self.flight.clone() }.validate(c);
        let g = &self.guns;
        c.number("guns.damage_mj", g.damage_mj, 0.0, 1e3);
        c.number("guns.rate_hz", g.rate_hz, 0.01, 100.0);
        c.number("guns.speed_mps", g.speed_mps, 1.0, 1e5);
        c.number("guns.life_s", g.life_s, 0.01, 60.0);
        c.number("guns.fire_cone_deg", g.fire_cone_deg, 0.0, 180.0);
        let a = &self.ai;
        c.number("ai.orbit_m", a.orbit_m, 10.0, 1e5);
        c.number("ai.orbit_speed_mps", a.orbit_speed_mps, 1.0, 5000.0);
        c.number("ai.orbit_radial_gain", a.orbit_radial_gain, 0.0, 100.0);
        c.number("ai.run_every_s[0]", a.run_every_s[0], 0.0, 3600.0);
        c.number("ai.run_every_s[1]", a.run_every_s[1], a.run_every_s[0], 3600.0);
        c.number("ai.setup_m", a.setup_m, 10.0, 1e5);
        c.number("ai.setup_arrive_m", a.setup_arrive_m, 1.0, 1e5);
        c.number("ai.setup_timeout_s", a.setup_timeout_s, 0.1, 3600.0);
        c.number("ai.run_speed_mps", a.run_speed_mps, 1.0, 5000.0);
        c.number("ai.fire_from_m", a.fire_from_m, 1.0, 1e5);
        c.number("ai.fire_to_m", a.fire_to_m, 0.0, a.fire_from_m);
        c.number("ai.run_timeout_s", a.run_timeout_s, 0.1, 3600.0);
        c.number("ai.break_off_timeout_s", a.break_off_timeout_s, 0.1, 3600.0);
    }
}

impl FighterData {
    /// Parse and validate `text`, the contents of `file` (`data/fighters.json`).
    pub fn parse(file: &str, text: &str) -> Result<Self, DataError> {
        crate::data::parse(file, text)
    }

    /// The shipped `data/fighters.json`.
    pub fn shipped() -> Self {
        Self::parse("data/fighters.json", include_str!("../../../../data/fighters.json"))
            .expect("the shipped data/fighters.json is valid")
    }
}

/// What a fighter is doing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FighterState {
    /// Still in the hangar: it leaves in `s` seconds.
    Waiting {
        /// Seconds until it leaves.
        s: f64,
    },
    /// Circling the prey; the next run starts in `s` seconds (None until the first step draws it).
    Orbit {
        /// Seconds until the next run.
        s: Option<f64>,
    },
    /// Flying out to the start of a run at `part` (an index into [`Prey::parts`]), for `s` seconds so far.
    Setup {
        /// The part.
        part: usize,
        /// Seconds so far.
        s: f64,
    },
    /// Running in at `part`, guns free inside the fire range, for `s` seconds so far.
    Run {
        /// The part.
        part: usize,
        /// Seconds so far.
        s: f64,
    },
    /// Breaking off and climbing back out to the orbit, for `s` seconds so far.
    BreakOff {
        /// Seconds so far.
        s: f64,
    },
}

/// One enemy fighter.
#[derive(Clone, Debug, PartialEq)]
pub struct Fighter {
    /// Its id in every message.
    pub id: u16,
    /// Position, metres, system frame.
    pub pos: DVec3,
    /// Velocity, m/s, system frame.
    pub vel: DVec3,
    /// Attitude: craft-local to system.
    pub rot: DQuat,
    /// Hull left, MJ.
    pub hull_mj: f64,
    /// The shield bubble's charge, MJ.
    pub shield_mj: f64,
    /// Not destroyed.
    pub alive: bool,
    /// What it is doing.
    pub state: FighterState,
    /// Body rates, rad/s, craft-local (the flight rule's).
    pub rates: DVec3,
    /// Lateral and vertical speed set points, m/s: the jink.
    pub strafe: [f64; 2],
    /// Which way it circles: 1 or -1 about the prey's up (None until the first step draws it).
    pub orbit_sign: Option<f64>,
    /// Seconds until its guns may fire again.
    pub gun_cooldown_s: f64,
    /// Seconds until it throws a new jink.
    pub jink_s: f64,
}

impl Fighter {
    /// Out of the hangar and alive: in the fight, drawn and hittable.
    pub fn out(&self) -> bool {
        self.alive && !matches!(self.state, FighterState::Waiting { .. })
    }

    /// The nose, a unit vector in the system frame.
    pub fn forward(&self) -> DVec3 {
        self.rot * DVec3::Z
    }
}

/// The ship the fighters launch from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mothership {
    /// Position, metres, system frame.
    pub pos: DVec3,
    /// Attitude: ship-local to system.
    pub rot: DQuat,
    /// Velocity, m/s, system frame.
    pub vel: DVec3,
}

/// The ship the fighters attack.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Prey<'a> {
    /// Position, metres, system frame.
    pub pos: DVec3,
    /// Attitude: ship-local to system.
    pub rot: DQuat,
    /// Velocity, m/s, system frame.
    pub vel: DVec3,
    /// Aim points on its hull, metres, its own frame (a turret, a laser bank, the bridge).
    pub parts: &'a [DVec3],
}

impl Prey<'_> {
    /// Part `i`'s point, system frame (the prey's centre when it has no such part).
    pub fn part_pos(&self, i: usize) -> DVec3 {
        self.pos + self.rot * self.parts.get(i).copied().unwrap_or(DVec3::ZERO)
    }

    /// The side part `i` faces, a unit vector in the system frame: out from the centre through the part (up when the
    /// part is the centre).
    pub fn part_facing(&self, i: usize) -> DVec3 {
        let p = self.parts.get(i).copied().unwrap_or(DVec3::ZERO);
        self.rot * p.try_normalize().unwrap_or(DVec3::Y)
    }
}

/// A bolt a fighter fired, for the drill to fly and resolve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    /// The fighter's id.
    pub owner: u16,
    /// Muzzle position, metres, system frame.
    pub pos: DVec3,
    /// Velocity, m/s, system frame (the fighter's plus the gun's).
    pub vel: DVec3,
    /// Damage it delivers, MJ.
    pub damage_mj: f64,
    /// Seconds it lives.
    pub life_s: f64,
}

/// Two fighters out of `m`'s hangar: the first leaves now, the second `launch.spacing_s` later, both at full hull and
/// shield. The drill calls this `launch.after_engage_s` into Engage.
pub fn launch(m: &Mothership, d: &FighterData, ids: [u16; 2]) -> Vec<Fighter> {
    let pos = m.pos + m.rot * DVec3::from_array(d.launch.hangar_m);
    let vel = m.vel + m.rot * DVec3::Z * d.launch.exit_mps;
    ids.iter()
        .enumerate()
        .map(|(i, &id)| Fighter {
            id,
            pos,
            vel,
            rot: m.rot,
            hull_mj: d.hull_mj,
            shield_mj: d.shield_mj,
            alive: true,
            state: if i == 0 {
                FighterState::Orbit { s: None }
            } else {
                FighterState::Waiting { s: d.launch.spacing_s * i as f64 }
            },
            rates: DVec3::ZERO,
            strafe: [0.0; 2],
            orbit_sign: None,
            gun_cooldown_s: 0.0,
            jink_s: 0.0,
        })
        .collect()
}

/// A number in `[lo, hi)`.
fn between(rng: &mut Rng, lo: f64, hi: f64) -> f64 {
    lo + (hi - lo) * rng.next_f64()
}

/// The flight rule's craft for one tick: the fighter's motion state in a [`Ship`], so it flies by [`fly`].
fn craft(f: &Fighter, stick: [f64; 3], speed_set: f64) -> Ship {
    Ship {
        id: f.id,
        side: Side::Enemy,
        active: true,
        alive: true,
        pos: f.pos,
        rot: f.rot,
        vel: f.vel,
        rates: f.rates,
        speed_set,
        strafe: f.strafe,
        stick,
        helm_mode: HelmMode::Hold,
        order: None,
        faces: [0.0; 6],
        preset: 0,
        hull: f.hull_mj,
        turrets: Vec::new(),
        tubes: Vec::new(),
        magazine: 0,
        lock_target: None,
        lock_s: 0.0,
    }
}

/// One tick for every fighter: the AI picks where to fly, the flight rule flies there, the shield regenerates and the
/// guns fire on a run. `threatened[i]` is true while a turret bears on fighter `i` (it jinks). Every random choice
/// comes from `rng`, drawn in the fighters' order, so a seed gives the same fight.
pub fn step(f: &mut [Fighter], d: &FighterData, prey: &Prey, threatened: &[bool], dt: f64, rng: &mut Rng) -> Vec<Shot> {
    let mut shots = Vec::new();
    let ai = &d.ai;
    let up = prey.rot * DVec3::Y;
    for (i, fi) in f.iter_mut().enumerate() {
        if !fi.alive {
            continue;
        }
        if let FighterState::Waiting { s } = fi.state {
            // In the hangar it keeps the velocity it will leave with.
            fi.pos += fi.vel * dt;
            fi.state =
                if s - dt <= 0.0 { FighterState::Orbit { s: None } } else { FighterState::Waiting { s: s - dt } };
            continue;
        }
        let sign = *fi.orbit_sign.get_or_insert_with(|| if rng.next_f64() < 0.5 { 1.0 } else { -1.0 });
        let r = prey.pos - fi.pos;
        let range = r.length().max(1.0);
        let rh = r / range;

        // The state machine: each state says where to fly and how fast, and when it hands over.
        let (desired, speed, firing_at) = match fi.state {
            FighterState::Waiting { .. } => unreachable!("handled above"),
            FighterState::Orbit { s } => {
                let left = s.unwrap_or_else(|| between(rng, ai.run_every_s[0], ai.run_every_s[1])) - dt;
                fi.state = if left <= 0.0 && !prey.parts.is_empty() {
                    let part = (rng.next_u64() % prey.parts.len() as u64) as usize;
                    FighterState::Setup { part, s: 0.0 }
                } else {
                    FighterState::Orbit { s: Some(left.max(0.0)) }
                };
                let tangent = (rh.cross(up) * sign).try_normalize().unwrap_or(DVec3::X);
                let radial = ((range - ai.orbit_m) / ai.orbit_m * ai.orbit_radial_gain).clamp(-1.0, 1.0);
                ((tangent + rh * radial).normalize_or_zero(), ai.orbit_speed_mps, None)
            }
            FighterState::Setup { part, s } => {
                let start = prey.part_pos(part) + prey.part_facing(part) * ai.setup_m;
                let to = start - fi.pos;
                fi.state = if to.length() < ai.setup_arrive_m || s + dt >= ai.setup_timeout_s {
                    FighterState::Run { part, s: 0.0 }
                } else {
                    FighterState::Setup { part, s: s + dt }
                };
                (to.normalize_or_zero(), d.flight.speed_max_mps, None)
            }
            FighterState::Run { part, s } => {
                let to = prey.part_pos(part) - fi.pos;
                let dist = to.length();
                fi.state = if dist < ai.fire_to_m || s + dt >= ai.run_timeout_s {
                    FighterState::BreakOff { s: 0.0 }
                } else {
                    FighterState::Run { part, s: s + dt }
                };
                let fire = dist <= ai.fire_from_m && dist >= ai.fire_to_m;
                (to.normalize_or_zero(), ai.run_speed_mps, fire.then_some(part))
            }
            FighterState::BreakOff { s } => {
                fi.state = if range >= ai.orbit_m || s + dt >= ai.break_off_timeout_s {
                    FighterState::Orbit { s: None }
                } else {
                    FighterState::BreakOff { s: s + dt }
                };
                ((up - rh).normalize_or_zero(), d.flight.speed_max_mps, None)
            }
        };

        // The jink: new strafe set points every period while a turret bears, none otherwise.
        if threatened.get(i).copied().unwrap_or(false) {
            fi.jink_s -= dt;
            if fi.jink_s <= 0.0 {
                let j = d.flight.jink_mps.min(d.flight.strafe_max_mps);
                fi.strafe = [between(rng, -j, j), between(rng, -j, j)];
                fi.jink_s = d.flight.jink_period_s;
            }
        } else {
            fi.strafe = [0.0; 2];
            fi.jink_s = 0.0;
        }

        let stick =
            if desired == DVec3::ZERO { [0.0; 3] } else { steer_toward(fi.rot, fi.rates, &d.flight, desired, 1.0) };
        let mut s = craft(fi, stick, speed);
        fly(&mut s, &d.flight, dt);
        (fi.pos, fi.vel, fi.rot, fi.rates) = (s.pos, s.vel, s.rot, s.rates);
        fi.shield_mj = (fi.shield_mj + d.shield_regen_mj_s * dt).min(d.shield_mj);

        // The guns: led at the part's point on the hull, fired while the nose is on the lead.
        fi.gun_cooldown_s = (fi.gun_cooldown_s - dt).max(0.0);
        if let Some(part) = firing_at {
            let rel = prey.part_pos(part) - fi.pos;
            if let Some(aim) = lead(rel, prey.vel - fi.vel, d.guns.speed_mps) {
                let off = fi.forward().angle_between(aim.dir).to_degrees();
                if off <= d.guns.fire_cone_deg && fi.gun_cooldown_s <= 0.0 {
                    shots.push(Shot {
                        owner: fi.id,
                        pos: fi.pos + fi.forward() * d.hit_radius_m,
                        vel: fi.vel + aim.dir * d.guns.speed_mps,
                        damage_mj: d.guns.damage_mj,
                        life_s: d.guns.life_s,
                    });
                    fi.gun_cooldown_s = 1.0 / d.guns.rate_hz;
                }
            }
        }
    }
    shots
}

/// `dmg_mj` reaches fighter `f`: the bubble takes it first, the hull the rest. True when this hit destroyed it; a
/// destroyed fighter stays destroyed and takes no more.
pub fn hit(f: &mut Fighter, d: &FighterData, dmg_mj: f64) -> bool {
    if !f.alive || !dmg_mj.is_finite() || dmg_mj <= 0.0 {
        return false;
    }
    // `d` is the seam's: the Jackal has no armour yet, so nothing of it changes a hit.
    let _ = d;
    let absorbed = dmg_mj.min(f.shield_mj);
    f.shield_mj -= absorbed;
    f.hull_mj -= dmg_mj - absorbed;
    if f.hull_mj <= 0.0 {
        f.hull_mj = 0.0;
        f.alive = false;
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::DT;

    const PARTS: [DVec3; 3] = [DVec3::new(0.0, 4.0, 10.0), DVec3::new(8.0, 0.0, 0.0), DVec3::new(-8.0, 0.0, 0.0)];

    fn hound() -> Mothership {
        Mothership { pos: DVec3::new(0.0, 0.0, 2000.0), rot: DQuat::IDENTITY, vel: DVec3::new(0.0, 0.0, -50.0) }
    }

    fn tern() -> Prey<'static> {
        Prey { pos: DVec3::ZERO, rot: DQuat::IDENTITY, vel: DVec3::new(0.0, 0.0, 60.0), parts: &PARTS }
    }

    /// Fly `secs` with the prey coasting, returning every shot and the state each tick.
    fn run(f: &mut [Fighter], d: &FighterData, secs: f64, seed: u64) -> (Vec<Shot>, Vec<Vec<FighterState>>) {
        let mut rng = Rng::for_purpose(seed, 0, "fighters");
        let mut prey = tern();
        let (mut shots, mut states) = (Vec::new(), Vec::new());
        for _ in 0..(secs / DT) as usize {
            shots.extend(step(f, d, &prey, &[false, false], DT, &mut rng));
            states.push(f.iter().map(|x| x.state).collect());
            prey.pos += prey.vel * DT;
        }
        (shots, states)
    }

    #[test]
    fn the_shipped_jackal_is_the_designs() {
        let d = FighterData::shipped();
        assert_eq!((d.hull_mj, d.shield_mj, d.shield_regen_mj_s, d.hit_radius_m), (3.0, 4.0, 0.5, 4.0));
        assert_eq!(
            (d.flight.speed_max_mps, d.flight.accel_fwd_mps2, d.flight.rate_limit_deg_s),
            (300.0, 60.0, [90.0; 3])
        );
        assert_eq!((d.guns.damage_mj, d.guns.rate_hz, d.guns.speed_mps, d.guns.life_s), (1.0, 6.0, 1200.0, 1.5));
        assert_eq!(
            (d.ai.orbit_m, d.ai.fire_from_m, d.ai.fire_to_m, d.ai.run_every_s),
            (800.0, 1200.0, 300.0, [12.0, 20.0])
        );
    }

    #[test]
    fn an_unknown_key_in_the_fighters_file_is_refused() {
        let text = include_str!("../../../../data/fighters.json").replacen("\"hull_mj\"", "\"hul_mj\"", 1);
        let e = FighterData::parse("data/fighters.json", &text).unwrap_err();
        assert!(e.message.contains("hul_mj"), "the error names the misspelt key: {e}");
    }

    #[test]
    fn a_bad_flight_number_is_refused_by_the_ships_rule() {
        let text = include_str!("../../../../data/fighters.json").replacen("\"tau_v_s\": 0.5", "\"tau_v_s\": 0", 1);
        let e = FighterData::parse("data/fighters.json", &text).unwrap_err();
        assert_eq!(e.field, "flight.tau_v_s");
    }

    #[test]
    fn two_fighters_launch_from_the_hangar_the_second_after_the_spacing() {
        let d = FighterData::shipped();
        let mut f = launch(&hound(), &d, [10, 11]);
        assert_eq!(f.len(), 2);
        assert_eq!((f[0].id, f[1].id), (10, 11));
        assert!(f[0].out() && !f[1].out(), "the first is out at once, the second still in the hangar");
        let mouth = hound().pos + DVec3::from_array(d.launch.hangar_m);
        assert!(f.iter().all(|x| x.pos.distance(mouth) < 1e-9 && x.shield_mj == d.shield_mj && x.hull_mj == d.hull_mj));
        run(&mut f, &d, d.launch.spacing_s + 0.1, 1);
        assert!(f[1].out(), "the second is out after the spacing");
    }

    #[test]
    fn a_fighter_orbits_then_runs_at_a_part_and_fires_inside_the_fire_range() {
        let d = FighterData::shipped();
        let mut f = launch(&hound(), &d, [10, 11]);
        let mut rng = Rng::for_purpose(3, 0, "fighters");
        let mut prey = tern();
        let mut fired = Vec::new();
        let mut ran = false;
        for _ in 0..(90.0 / DT) as usize {
            for s in step(&mut f, &d, &prey, &[false, false], DT, &mut rng) {
                let me = f.iter().find(|x| x.id == s.owner).unwrap();
                let FighterState::Run { part, .. } = me.state else { panic!("fired outside a run: {:?}", me.state) };
                let dist = prey.part_pos(part).distance(me.pos);
                fired.push(dist);
                // The bolt is led at the part: it passes within a few metres of the part's point, flown on.
                let rel_v = s.vel - prey.vel;
                let p = s.pos - prey.part_pos(part);
                let t = (-p.dot(rel_v) / rel_v.length_squared()).max(0.0);
                assert!(
                    (p + rel_v * t).length() < 6.0,
                    "a bolt aimed at its part misses by {}",
                    (p + rel_v * t).length()
                );
            }
            ran |= f.iter().any(|x| matches!(x.state, FighterState::Run { .. }));
            prey.pos += prey.vel * DT;
        }
        assert!(ran, "within 90 s a fighter makes a run");
        assert!(!fired.is_empty(), "a run fires");
        assert!(
            fired.iter().all(|&r| (d.ai.fire_to_m - 30.0..=d.ai.fire_from_m + 30.0).contains(&r)),
            "every bolt is fired from 1,200 m to 300 m (allowing a tick's flight): {fired:?}"
        );
    }

    #[test]
    fn after_a_run_it_breaks_off_and_returns_to_the_orbit() {
        let d = FighterData::shipped();
        let mut f = launch(&hound(), &d, [10, 11]);
        let (_, states) = run(&mut f, &d, 120.0, 5);
        let seq: Vec<u8> = states
            .iter()
            .map(|s| match s[0] {
                FighterState::Waiting { .. } => 0,
                FighterState::Orbit { .. } => 1,
                FighterState::Setup { .. } => 2,
                FighterState::Run { .. } => 3,
                FighterState::BreakOff { .. } => 4,
            })
            .collect();
        let mut changes: Vec<u8> = seq.clone();
        changes.dedup();
        let cycle = [1, 2, 3, 4, 1];
        assert!(changes.windows(5).any(|w| w == cycle), "orbit, set-up, run, break off, orbit: {changes:?}");
    }

    #[test]
    fn its_flight_keeps_to_300_mps() {
        let d = FighterData::shipped();
        let mut f = launch(&hound(), &d, [10, 11]);
        let mut rng = Rng::for_purpose(9, 0, "fighters");
        for _ in 0..(60.0 / DT) as usize {
            step(&mut f, &d, &tern(), &[true, true], DT, &mut rng);
            for x in &f {
                let fwd = x.vel.dot(x.forward());
                assert!(fwd <= d.flight.speed_max_mps + 1.0, "forward speed {fwd} over the top speed");
            }
        }
    }

    #[test]
    fn it_jinks_while_threatened_and_flies_level_otherwise() {
        let d = FighterData::shipped();
        let mut f = launch(&hound(), &d, [10, 11]);
        let mut rng = Rng::for_purpose(1, 0, "fighters");
        step(&mut f, &d, &tern(), &[true, false], DT, &mut rng);
        assert_ne!(f[0].strafe, [0.0; 2], "a threatened fighter jinks");
        step(&mut f, &d, &tern(), &[false, false], DT, &mut rng);
        assert_eq!(f[0].strafe, [0.0; 2], "and stops when nothing bears");
    }

    #[test]
    fn the_bubble_takes_a_hit_first_and_regenerates() {
        let d = FighterData::shipped();
        let mut f = launch(&hound(), &d, [10, 11]);
        assert!(!hit(&mut f[0], &d, 3.0));
        assert_eq!((f[0].shield_mj, f[0].hull_mj), (1.0, 3.0), "the bubble took it all");
        run(&mut f, &d, 2.0, 1);
        assert!((f[0].shield_mj - 2.0).abs() < 0.05, "0.5 MJ/s for 2 s: {}", f[0].shield_mj);
        run(&mut f, &d, 10.0, 1);
        assert_eq!(f[0].shield_mj, d.shield_mj, "it stops at full");
    }

    #[test]
    fn a_destroyed_fighter_stays_destroyed() {
        let d = FighterData::shipped();
        let mut f = launch(&hound(), &d, [10, 11]);
        assert!(hit(&mut f[0], &d, 7.5), "4 MJ of bubble and 3 MJ of hull: 7.5 MJ destroys it");
        assert!(!f[0].alive && !f[0].out());
        assert!(!hit(&mut f[0], &d, 5.0), "a second hit does not destroy it again");
        let at = f[0].pos;
        let (shots, _) = run(&mut f, &d, 60.0, 2);
        assert!(!f[0].alive && f[0].pos == at, "it neither moves nor comes back");
        assert!(shots.iter().all(|s| s.owner != 10), "and fires nothing");
    }

    #[test]
    fn step_is_deterministic_for_a_seed() {
        let d = FighterData::shipped();
        let (mut a, mut b, mut c) =
            (launch(&hound(), &d, [10, 11]), launch(&hound(), &d, [10, 11]), launch(&hound(), &d, [10, 11]));
        let (sa, _) = run(&mut a, &d, 60.0, 42);
        let (sb, _) = run(&mut b, &d, 60.0, 42);
        let (sc, _) = run(&mut c, &d, 60.0, 43);
        assert_eq!((a.clone(), sa.clone()), (b, sb), "the same seed flies the same fight");
        assert!(a != c || sa != sc, "another seed flies another");
    }
}
