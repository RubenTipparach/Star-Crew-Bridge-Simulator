//! A body on the decks and the eye that follows it: the ship's walk (`sc-core::walk`), its lifts, the walk grid, and
//! what W A S D, Shift, Space, E and Q do on foot.
//!
//! It lives in the client because it is the player's own body, walked and predicted here (CLAUDE.md 6.3), and it is
//! one implementation for every view on foot: the ship tour and the game (CLAUDE.md, the law: one game, played on
//! foot; 6.1). The game sends where it put the body to the server, which checks the step (`Drill::walked`).

use sc_client::platform::keys;
use sc_core::deck::{Deck, WalkDoor};
use sc_core::lift::Lift;
use sc_core::nav::NavGrid;
use sc_core::walk::{self, Body, Input, WalkData, WalkEntities, WalkWorld};
use std::collections::HashSet;

/// A body on the decks.
pub struct OnFoot {
    /// The walk's collision, ladders, hatches, doors and lifts.
    pub world: WalkWorld,
    /// `data/crew/walk.json`.
    pub data: WalkData,
    /// The body.
    pub body: Body,
    /// The eye's floor height, following the feet smoothly up a step.
    pub eye_y: f32,
    /// Each lift's car, and the floor height its room was drawn at in the deck.
    pub lifts: Vec<(Lift, f32)>,
    /// The doors, for the lift landings a call reaches.
    pub doors: Vec<WalkDoor>,
    /// The walk grid (the bots' paths, and a script's).
    pub grid: NavGrid,
    /// The decks' floor heights, lowest first (C, B, A on the Tern), metres.
    pub decks: Vec<f32>,
}

impl OnFoot {
    /// The walk world, the body at the deck's start, the lifts and the walk grid, from a compiled deck.
    pub fn from_deck(d: &Deck) -> Result<Self, String> {
        let text = std::fs::read_to_string("data/crew/walk.json").map_err(|e| format!("data/crew/walk.json: {e}"))?;
        let data: WalkData = sc_core::data::parse("data/crew/walk.json", &text).map_err(|e| e.to_string())?;
        let w = &d.index.walk;
        let ents = WalkEntities {
            ladders: w.ladders.clone(),
            hatches: w.hatches.clone(),
            doors: w.doors.clone(),
            lifts: w.lifts.clone(),
        };
        let t0 = std::time::Instant::now();
        let world = WalkWorld::new(&d.walk_triangles(), &ents, &data)?;
        println!(
            "sc-client: walk world of {} triangles built in {:.0} ms; {} ladders, {} hatches, {} doors",
            world.triangles,
            t0.elapsed().as_secs_f64() * 1000.0,
            w.ladders.len(),
            w.hatches.len(),
            w.doors.len()
        );
        let body = Body::at_start(&w.start, &data);
        let eye_y = w.start.at_m[1];
        let lifts = w.lifts.iter().map(|l| (Lift::new(&l.stops_m, l.speed_m_s, l.door_s, l.car_m), l.car_m)).collect();
        let t0 = std::time::Instant::now();
        let (mut grid, decks) = NavGrid::for_ship(&world, w, &d.walk_triangles());
        let rel = "data/ships/tern/command_suite.json";
        let suite = std::fs::read_to_string(rel).map_err(|e| format!("{rel}: {e}"))?;
        grid.add_stairs(&NavGrid::stairs_of(&suite).map_err(|e| format!("{rel}: {e}"))?);
        println!(
            "sc-client: walk grid of {} walkable cells on {} decks built in {:.0} ms",
            grid.walkable(),
            decks.len(),
            t0.elapsed().as_secs_f64() * 1000.0
        );
        Ok(Self { world, data, body, eye_y, lifts, doors: w.doors.clone(), grid, decks })
    }

    /// Put the body's feet at `at` (where the server has it: where it joined, or behind the seat it got up from), on
    /// the walk grid's floor nearest it: a seat's height is its deck's, and a station's platform stands higher; and a
    /// body put down against a chair's back or a desk would be held by it, where the grid's floor is clear of both.
    pub fn place(&mut self, at: [f32; 3]) {
        // The walk grid's floor there (never a chair's seat or a desk), else the floor under it.
        let at = self
            .grid
            .snap(at, 0.8)
            .filter(|p| (p[1] - at[1]).abs() < 1.0)
            .or_else(|| {
                self.world
                    .floor_below(at[0], at[1] + 1.0, at[2])
                    .filter(|y| (y - at[1]).abs() < 1.0)
                    .map(|y| [at[0], y, at[2]])
            })
            .unwrap_or(at);
        let y = at[1];
        self.body = Body::new(at, &self.data);
        self.eye_y = y;
    }

    /// The keys' walk: W A S D, Shift to run, Space to jump, the use key (up) and the down key (down) to climb, as
    /// `b` binds them; `yaw` the eye's facing.
    pub fn keys_input(held: &HashSet<u32>, pressed: &HashSet<u32>, yaw: f32, b: &crate::bindings::Bindings) -> Input {
        let k = |c: u32| if held.contains(&c) { 1.0 } else { 0.0 };
        Input {
            forward: k(keys::W) - k(keys::S),
            right: k(keys::D) - k(keys::A),
            yaw,
            run: held.contains(&keys::LSHIFT),
            jump: pressed.contains(&keys::SPACE),
            use_: pressed.contains(&b.use_key) || pressed.contains(&b.down_key),
            down: pressed.contains(&b.down_key),
        }
    }

    /// One step: E and Q work a lift first (in its car, or at a landing), else they climb; the cars run; the body
    /// walks; the eye follows the feet.
    pub fn step(&mut self, input: Input, lift_keys: bool, dt: f64) {
        let lift = lift_keys && self.lift_keys(input.use_ && !input.down, input.down);
        let input = if lift { Input { use_: false, down: false, ..input } } else { input };
        self.step_lifts(dt as f32);
        self.body.step(&self.world, &self.data, &input, dt);
        self.eye_y = walk::eye_follow(self.eye_y, self.body.feet[1], dt as f32, &self.data, self.body.climbing());
    }

    /// The eye, ship frame, metres.
    pub fn eye(&self) -> [f64; 3] {
        let f = self.body.feet;
        [f64::from(f[0]), f64::from(self.eye_y) + self.data.body.eye_m, f64::from(f[2])]
    }

    /// E and Q for the lifts: in a car, up and down a deck; at a landing, E calls the car. True when a key was taken.
    pub fn lift_keys(&mut self, up: bool, down: bool) -> bool {
        if !up && !down {
            return false;
        }
        let f = self.body.feet;
        if let Some(i) = self.world.lift_index_at(f[0], f[2]) {
            let l = &mut self.lifts[i].0;
            if up {
                l.up();
            } else {
                l.down();
            }
            return true;
        }
        let reach = self.data.lift_call_m as f32;
        if let (true, Some((i, stop))) = (up, self.world.landing_near(f[0], f[1], f[2], reach, &self.doors)) {
            let l = &mut self.lifts[i].0;
            let (s, _) = l.nearest_stop(stop);
            l.send(s);
            return true;
        }
        false
    }

    /// What the use and down keys do where the body stands (ui-art-direction design 7): in a lift's car, up a deck
    /// and down a deck; at a landing, call the car. The keys are `b`'s, as the walk reads them.
    pub fn prompt(&self, b: &crate::bindings::Bindings) -> Option<crate::hud::Prompt> {
        use crate::hud::Prompt;
        use crate::look::KeyTok;
        let f = self.body.feet;
        let use_key = vec![KeyTok::Key(b.use_label.clone())];
        if self.world.lift_index_at(f[0], f[2]).is_some() {
            let mut p = Prompt::act("Lift", use_key, "Up");
            p.acts.push((vec![KeyTok::Key(b.down_label.clone())], "Down".into()));
            return Some(p);
        }
        let reach = self.data.lift_call_m as f32;
        self.world.landing_near(f[0], f[1], f[2], reach, &self.doors).map(|_| Prompt::act("Lift", use_key, "Call"))
    }

    /// Run the cars `dt` seconds and tell the walk world where they are.
    pub fn step_lifts(&mut self, dt: f32) {
        for (i, (l, _)) in self.lifts.iter_mut().enumerate() {
            l.step(dt);
            let open = (l.doors_open() > 0.9).then(|| l.stops()[l.stop()]);
            self.world.set_lift(i, l.car_y, open);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OnFoot;
    use sc_core::combat::bodies::seat_in_reach;
    use sc_core::combat::data::DrillData;
    use sc_core::combat::Station;
    use sc_core::walk::Input;

    /// The law (CLAUDE.md): a player walks to a station to sit at it, so every station on the bridge must be reachable
    /// by a body on its own feet: from the corridor outside, the walk grid's path, walked by the body's own controller
    /// (`bots::steer`, as the playtest and the bot crew walk), brings the seat within reach. Needs the compiled deck
    /// (`sc-tools deckc`); skipped without one.
    #[test]
    fn every_bridge_station_is_reached_on_foot_from_the_corridor() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let Ok(bytes) = std::fs::read(root.join("compiled/tern.deck")) else {
            eprintln!("no compiled deck: skipped");
            return;
        };
        let Ok(d) = sc_core::deck::read(&bytes) else {
            eprintln!("a compiled deck of another version: skipped (rebuild it with sc-tools deckc)");
            return;
        };
        std::env::set_current_dir(&root).expect("the repository");
        let mut f = OnFoot::from_deck(&d).expect("the walk");
        let data = DrillData::shipped();
        let stations: Vec<Station> = data.mission.stations.iter().filter_map(|s| Station::from_id(s)).collect();
        let seats = sc_server::bridge::seats(&root, &data, &stations).expect("the seats");
        let (reach, dy) = (data.stations.seats.reach_m as f32, data.stations.seats.reach_dy_m as f32);
        let corridor =
            data.tern_layout.room("a_corridor").map(|k| data.tern_layout.rooms[k].floor.as_vec3().to_array());
        let start = f.grid.snap(corridor.expect("deck A's corridor"), 6.0).expect("floor in the corridor");
        let mut failed = Vec::new();
        for (st, p, yaw) in seats.iter().filter(|s| s.1[1] > 3.0) {
            f.place(start);
            let behind = [p[0] - yaw.sin() * 0.8, p[1], p[2] - yaw.cos() * 0.8];
            // Behind the seat on a floor joined to the corridor; or, where the seat stands up a ladder (a turret pod), the
            // seat itself, which the grid reaches by the ladder.
            let goal = f.grid.snap_joined(behind, 1.0, start).unwrap_or(*p);
            let mut plan = f.grid.path(&f.world, start, goal).unwrap_or_default();
            let mut yaw_now = 0.0f32;
            let mut reached = false;
            for _ in 0..(30 * 40) {
                if seat_in_reach(&seats, f.body.feet, reach, dy) == Some(*st) {
                    reached = true;
                    break;
                }
                if plan.is_empty() {
                    plan.push(sc_core::nav::Waypoint::Walk(goal));
                }
                let input: Input =
                    sc_core::bots::steer(&mut plan, f.body.feet, &mut yaw_now, f.world.ladder_reach_m, 1.0 / 30.0);
                f.step(input, false, 1.0 / 30.0);
            }
            if !reached {
                failed.push(format!("{} (the body stopped at {:.2?})", st.name(), f.body.feet));
            }
        }
        assert!(failed.is_empty(), "stations a body cannot walk to: {}", failed.join("; "));
    }

    /// A player who gets up from a seat stands where the server puts them (`Bridge::stand_point`) and walks away on
    /// their own feet: a body put down inside a chair or a desk cannot move, and the game would hold them there. Needs
    /// the compiled deck; skipped without one.
    #[test]
    fn a_player_who_stands_up_walks_away_from_every_seat() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let Ok(bytes) = std::fs::read(root.join("compiled/tern.deck")) else {
            eprintln!("no compiled deck: skipped");
            return;
        };
        let Ok(d) = sc_core::deck::read(&bytes) else {
            eprintln!("a compiled deck of another version: skipped (rebuild it with sc-tools deckc)");
            return;
        };
        std::env::set_current_dir(&root).expect("the repository");
        let mut f = OnFoot::from_deck(&d).expect("the walk");
        let data = DrillData::shipped();
        let stations: Vec<Station> = data.mission.stations.iter().filter_map(|s| Station::from_id(s)).collect();
        let (bridge, _) =
            sc_server::bridge::load(&data, &root, Some(&root.join("compiled/tern.deck"))).expect("bridge");
        let mut failed = Vec::new();
        for st in stations {
            let up = bridge.stand_point(st);
            // The best of four ways out, each facing four ways, in a second and a half: a body free to walk gets well
            // clear of where it stood.
            let mut best = 0.0f32;
            for (forward, right) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
                for k in 0..4 {
                    let yaw = k as f32 * std::f32::consts::FRAC_PI_2;
                    f.place(up);
                    let at = f.body.feet;
                    for _ in 0..45 {
                        f.step(Input { forward, right, yaw, ..Input::default() }, false, 1.0 / 30.0);
                    }
                    best = best.max((f.body.feet[0] - at[0]).hypot(f.body.feet[2] - at[2]));
                }
            }
            if best < 0.5 {
                failed.push(format!("{} (stood at {:.2?}, moved {best:.2} m at most)", st.name(), up));
            }
        }
        assert!(failed.is_empty(), "seats a body cannot walk away from: {}", failed.join("; "));
    }
}
