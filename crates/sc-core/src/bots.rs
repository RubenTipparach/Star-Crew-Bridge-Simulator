//! The bot crew at work, first version (openspec/changes/crew-npcs, design 7): each bot walks from place to place in
//! its department's rooms, by a path on the walk grid (`crate::nav`), and stays at each for a while.
//!
//! It lives in the core because it is a rule the server will run: a bot's body is a `walk::Body` stepped by the same
//! code as a player's, steered by the same `Input` a player's keys make (CLAUDE.md 6.1), and every choice is drawn
//! from the bot's own seeded stream (CLAUDE.md 6.4), so a replay walks the same crew the same way.

use crate::crew::{CompanyData, CrewMember};
use crate::nav::{NavGrid, Waypoint};
use crate::rng::Rng;
use crate::walk::{Body, Input, WalkData, WalkWorld};

/// How near a point counts as reached, metres.
const REACH_M: f32 = 0.35;
/// A bot that has moved less than this in `STUCK_S` seconds while it should be walking is stuck, metres.
const STUCK_M: f32 = 0.3;
const STUCK_S: f32 = 3.0;

/// One bot.
pub struct Bot {
    /// Who it is.
    pub member: CrewMember,
    /// Its body.
    pub body: Body,
    /// The way it faces in the plan, radians (for drawing).
    pub yaw: f32,
    rng: Rng,
    plan: Vec<Waypoint>,
    goal: Option<[f32; 3]>,
    wait_s: f32,
    stuck_from: ([f32; 3], f32),
    replans: u32,
}

/// The bot crew.
pub struct Crew {
    /// The bots, in slot order.
    pub bots: Vec<Bot>,
    /// Each department's places: walkable floor points in its rooms.
    places: Vec<Vec<[f32; 3]>>,
    stay_s: [f32; 2],
}

impl Crew {
    /// The crew `members` of `company`, each placed at one of its department's places. `rooms` are the compartments'
    /// ids and floor middles (the deck file's `floor_m`); a room's place is the walkable cell nearest its middle.
    pub fn new(
        company: &CompanyData,
        members: Vec<CrewMember>,
        rooms: &[(String, [f32; 3])],
        grid: &NavGrid,
        session_seed: u64,
        d: &WalkData,
    ) -> Self {
        let places: Vec<Vec<[f32; 3]>> = company
            .departments
            .iter()
            .map(|dep| {
                dep.rooms
                    .iter()
                    .filter_map(|r| rooms.iter().find(|(id, _)| id == r))
                    .filter_map(|(_, p)| grid.snap(*p, 4.0))
                    .collect()
            })
            .collect();
        let bots = members
            .into_iter()
            .filter(|m| !places[m.department].is_empty())
            .map(|m| {
                let mut rng = Rng::for_purpose(session_seed, m.slot, "bot_places");
                let ps = &places[m.department];
                let at = ps[(rng.next_u64() % ps.len() as u64) as usize];
                // Spread bots that start in one room round its middle.
                let jitter = [(rng.next_f64() as f32 - 0.5) * 1.5, (rng.next_f64() as f32 - 0.5) * 1.5];
                let feet = grid.snap([at[0] + jitter[0], at[1], at[2] + jitter[1]], 1.5).unwrap_or(at);
                let wait_s = 1.0 + rng.next_f64() as f32 * 6.0;
                Bot {
                    body: Body::new(feet, d),
                    yaw: rng.next_f64() as f32 * std::f32::consts::TAU,
                    member: m,
                    rng,
                    plan: Vec::new(),
                    goal: None,
                    wait_s,
                    stuck_from: (feet, 0.0),
                    replans: 0,
                }
            })
            .collect();
        Self { bots, places, stay_s: [company.stay_s[0] as f32, company.stay_s[1] as f32] }
    }

    /// Step every bot `dt` seconds.
    pub fn step(&mut self, world: &WalkWorld, grid: &NavGrid, d: &WalkData, dt: f32) {
        for b in &mut self.bots {
            let input = b.think(world, grid, &self.places, self.stay_s, dt);
            b.body.step(world, d, &input, dt as f64);
        }
    }
}

impl Bot {
    fn think(
        &mut self,
        world: &WalkWorld,
        grid: &NavGrid,
        places: &[Vec<[f32; 3]>],
        stay_s: [f32; 2],
        dt: f32,
    ) -> Input {
        let still = Input { yaw: self.yaw, ..Input::default() };
        if self.body.climbing() {
            self.stuck_from = (self.body.feet, 0.0);
            return still;
        }
        let f = self.body.feet;
        if self.plan.is_empty() {
            if self.goal.take().is_some() {
                // Arrived: stay a while.
                self.wait_s = stay_s[0] + self.rng.next_f64() as f32 * (stay_s[1] - stay_s[0]);
            }
            self.wait_s -= dt;
            if self.wait_s > 0.0 {
                return still;
            }
            let ps = &places[self.member.department];
            let next = ps[(self.rng.next_u64() % ps.len() as u64) as usize];
            self.replans = 0;
            self.go(world, grid, next);
            return still;
        }
        // Stuck: re-plan to the same goal, then give it up for another.
        self.stuck_from.1 += dt;
        if self.stuck_from.1 > STUCK_S {
            let moved = (f[0] - self.stuck_from.0[0]).hypot(f[2] - self.stuck_from.0[2]);
            self.stuck_from = (f, 0.0);
            if moved < STUCK_M {
                self.replans += 1;
                match (self.goal, self.replans) {
                    (Some(g), r) if r <= 2 => self.go(world, grid, g),
                    _ => {
                        self.plan.clear();
                        self.goal = None;
                        self.wait_s = 1.0;
                    }
                }
                return still;
            }
        }
        match self.plan[0] {
            Waypoint::Walk(p) => {
                let (dx, dz) = (p[0] - f[0], p[2] - f[2]);
                if dx.hypot(dz) < REACH_M && (p[1] - f[1]).abs() < 1.0 {
                    self.plan.remove(0);
                    return still;
                }
                self.turn_to(dx.atan2(dz), dt);
                Input { forward: 1.0, yaw: dx.atan2(dz), ..Input::default() }
            }
            Waypoint::Climb { at, down, .. } => {
                let (dx, dz) = (at[0] - f[0], at[2] - f[2]);
                if dx.hypot(dz) > REACH_M {
                    self.turn_to(dx.atan2(dz), dt);
                    return Input { forward: 1.0, yaw: dx.atan2(dz), ..Input::default() };
                }
                // At the ladder: Use, as a player presses it.
                self.plan.remove(0);
                Input { use_: true, down, yaw: self.yaw, ..Input::default() }
            }
        }
    }

    fn go(&mut self, world: &WalkWorld, grid: &NavGrid, to: [f32; 3]) {
        self.stuck_from = (self.body.feet, 0.0);
        match grid.path(world, self.body.feet, to) {
            Some(p) => {
                self.plan = p;
                self.goal = Some(to);
            }
            None => {
                self.plan.clear();
                self.goal = None;
                self.wait_s = 2.0;
            }
        }
    }

    fn turn_to(&mut self, yaw: f32, dt: f32) {
        let d = (yaw - self.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        self.yaw += d * (dt * 10.0).min(1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crew::Department;
    use crate::walk::WalkEntities;

    fn quad(t: &mut Vec<[f32; 9]>, a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3]) {
        t.push([a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2]]);
        t.push([a[0], a[1], a[2], c[0], c[1], c[2], d[0], d[1], d[2]]);
    }

    #[test]
    fn a_bot_walks_through_the_door_to_its_other_room_and_stays() {
        let d: WalkData = crate::data::parse("w", include_str!("../../../data/crew/walk.json")).expect("walk data");
        let mut t = Vec::new();
        quad(&mut t, [-10.0, 0.0, 0.0], [-10.0, 0.0, 10.0], [10.0, 0.0, 10.0], [10.0, 0.0, 0.0]);
        quad(&mut t, [0.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 3.0, 7.0], [0.0, 0.0, 7.0]);
        quad(&mut t, [0.0, 0.0, 8.2], [0.0, 3.0, 8.2], [0.0, 3.0, 10.0], [0.0, 0.0, 10.0]);
        let w = WalkWorld::new(&t, &WalkEntities::default(), &d).expect("world");
        let grid = NavGrid::build(&w, &[0.0], [-10.0, 0.0, 10.0, 10.0], &[], &[]);
        let company = CompanyData {
            schema: "starcrew.company/1".into(),
            berths: 2,
            stay_s: [1.0, 1.0],
            departments: vec![Department {
                id: "engineering".into(),
                name: "Engineering".into(),
                count: 1,
                colour_srgb: [0.5; 3],
                rooms: vec!["west".into(), "east".into()],
            }],
        };
        let rooms = vec![("west".to_string(), [-5.0, 0.0, 3.0]), ("east".to_string(), [5.0, 0.0, 3.0])];
        let member = CrewMember { slot: 2, name: "Test".into(), department: 0 };
        let mut crew = Crew::new(&company, vec![member], &rooms, &grid, 3, &d);
        let start_east = crew.bots[0].body.feet[0] > 0.0;
        let mut crossed = false;
        for _ in 0..(60 * 60) {
            crew.step(&w, &grid, &d, 1.0 / 60.0);
            let x = crew.bots[0].body.feet[0];
            assert!(x.abs() > 0.05 || (7.0..8.2).contains(&crew.bots[0].body.feet[2]), "never through the wall");
            if (x > 0.0) != start_east {
                crossed = true;
            }
        }
        assert!(crossed, "in a minute, a bot with two rooms works in both");
        assert!(crew.bots[0].body.feet[1].abs() < 0.05, "on its feet on the floor");
    }
}
