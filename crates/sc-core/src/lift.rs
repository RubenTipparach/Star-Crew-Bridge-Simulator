//! A lift: one car in a shaft, its stops, its doors (openspec/changes/deck-pipeline, design section 13b; the
//! deck-access change's lift, crew-on-deck section 5's doors).
//!
//! It lives in the core because a car's position is a gameplay fact: the server moves it, every client draws it,
//! and a body in the shaft stands on its floor. Players and tests send it with the same calls (CLAUDE.md 6.1).
//!
//! The car waits at a stop with its doors open. Sent elsewhere, it closes its doors (`door_s`), travels at
//! `speed_m_s`, opens them again (`door_s`) and waits. A call while it is busy is kept and served next; a call to
//! where it already waits only reopens its doors.

/// What the car is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Waiting at a stop, doors open.
    Idle,
    /// Closing its doors before leaving.
    Closing,
    /// Travelling to its target stop.
    Moving,
    /// Opening its doors at the stop it reached.
    Opening,
}

/// A lift's car.
#[derive(Debug, Clone)]
pub struct Lift {
    stops: Vec<f32>,
    speed_m_s: f32,
    door_s: f32,
    /// The car's floor, ship coordinates, metres.
    pub car_y: f32,
    /// What it is doing.
    pub phase: Phase,
    t: f32,
    at: usize,
    target: usize,
    next: Option<usize>,
}

impl Lift {
    /// A car waiting, doors open, at the stop nearest `car_y`. `stops` are floor heights in metres, in any order.
    ///
    /// # Panics
    /// With no stops, or a speed that is not positive.
    pub fn new(stops: &[f32], speed_m_s: f32, door_s: f32, car_y: f32) -> Self {
        assert!(!stops.is_empty() && speed_m_s > 0.0, "a lift needs a stop and a speed");
        let mut stops = stops.to_vec();
        stops.sort_by(f32::total_cmp);
        let at = nearest(&stops, car_y);
        Self {
            car_y: stops[at],
            stops,
            speed_m_s,
            door_s: door_s.max(0.0),
            phase: Phase::Idle,
            t: 0.0,
            at,
            target: at,
            next: None,
        }
    }

    /// The stops, lowest first, metres.
    pub fn stops(&self) -> &[f32] {
        &self.stops
    }

    /// The stop the car is at or last left (lowest is 0).
    pub fn stop(&self) -> usize {
        self.at
    }

    /// The stop it is going to.
    pub fn target(&self) -> usize {
        self.target
    }

    /// Send the car to `stop` (an index, lowest 0). Out of range does nothing.
    pub fn send(&mut self, stop: usize) {
        if stop >= self.stops.len() {
            return;
        }
        match self.phase {
            Phase::Idle if stop == self.at => {}
            Phase::Idle => {
                self.target = stop;
                self.phase = Phase::Closing;
                self.t = 0.0;
            }
            Phase::Closing if stop == self.at => {
                // Called back to where it stands while closing: open again from where the doors are.
                self.phase = Phase::Opening;
                self.t = self.door_s - self.t;
                self.target = stop;
            }
            Phase::Closing => self.target = stop,
            Phase::Moving | Phase::Opening => {
                if stop != self.target {
                    self.next = Some(stop);
                }
            }
        }
    }

    /// Send the car one stop up from the one it is at or going to; at the top, nothing.
    pub fn up(&mut self) {
        let from = if self.phase == Phase::Idle { self.at } else { self.target };
        if from + 1 < self.stops.len() {
            self.send(from + 1);
        }
    }

    /// Send the car one stop down; at the bottom, nothing.
    pub fn down(&mut self) {
        let from = if self.phase == Phase::Idle { self.at } else { self.target };
        if from > 0 {
            self.send(from - 1);
        }
    }

    /// The nearest stop to a floor height `y`, and how far it is, metres.
    pub fn nearest_stop(&self, y: f32) -> (usize, f32) {
        let i = nearest(&self.stops, y);
        (i, (self.stops[i] - y).abs())
    }

    /// How open the doors are at the car's stop: 0 shut, 1 open (always 0 while moving).
    pub fn doors_open(&self) -> f32 {
        let k = if self.door_s > 0.0 { (self.t / self.door_s).clamp(0.0, 1.0) } else { 1.0 };
        match self.phase {
            Phase::Idle => 1.0,
            Phase::Closing => 1.0 - k,
            Phase::Moving => 0.0,
            Phase::Opening => k,
        }
    }

    /// The landing at floor height `y` can be walked through: the car stands there with its doors open.
    pub fn open_at(&self, y: f32) -> bool {
        (self.car_y - y).abs() < 0.05 && self.doors_open() > 0.9
    }

    /// Advance `dt` seconds.
    pub fn step(&mut self, dt: f32) {
        if dt.is_nan() || dt <= 0.0 {
            return;
        }
        self.t += dt;
        match self.phase {
            Phase::Idle => {
                if let Some(n) = self.next.take() {
                    self.send(n);
                }
            }
            Phase::Closing => {
                if self.t >= self.door_s {
                    self.phase = Phase::Moving;
                    self.t = 0.0;
                }
            }
            Phase::Moving => {
                let goal = self.stops[self.target];
                let d = goal - self.car_y;
                let move_m = self.speed_m_s * dt;
                if d.abs() <= move_m {
                    self.car_y = goal;
                    self.at = self.target;
                    self.phase = Phase::Opening;
                    self.t = 0.0;
                } else {
                    self.car_y += move_m * d.signum();
                }
            }
            Phase::Opening => {
                if self.t >= self.door_s {
                    self.phase = Phase::Idle;
                    self.t = 0.0;
                    if let Some(n) = self.next.take() {
                        self.send(n);
                    }
                }
            }
        }
    }
}

fn nearest(stops: &[f32], y: f32) -> usize {
    let mut best = 0;
    for (i, s) in stops.iter().enumerate() {
        if (s - y).abs() < (stops[best] - y).abs() {
            best = i;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    const STOPS: [f32; 3] = [-3.5, 0.0, 3.5];

    fn run(l: &mut Lift, seconds: f32) {
        let mut t = 0.0;
        while t < seconds {
            l.step(1.0 / 60.0);
            t += 1.0 / 60.0;
        }
    }

    #[test]
    fn a_car_sent_down_one_deck_arrives_after_its_doors_and_the_ride() {
        let mut l = Lift::new(&STOPS, 1.5, 2.0, 3.5);
        l.down();
        run(&mut l, 2.0 + 3.5 / 1.5 + 0.1);
        assert!((l.car_y - 0.0).abs() < 1e-4, "the car rides 3.5 m at 1.5 m/s after closing for 2 s");
        assert_eq!(l.phase, Phase::Opening);
        run(&mut l, 2.0);
        assert_eq!(l.phase, Phase::Idle);
        assert!(l.open_at(0.0));
    }

    #[test]
    fn a_landing_is_shut_while_the_car_is_away_or_moving() {
        let mut l = Lift::new(&STOPS, 1.5, 2.0, 3.5);
        assert!(l.open_at(3.5) && !l.open_at(0.0), "waiting at A, only A's landing opens");
        l.send(0);
        run(&mut l, 2.5);
        assert!(!l.open_at(3.5), "moving away, A's landing is shut");
    }

    #[test]
    fn up_at_the_top_and_down_at_the_bottom_do_nothing() {
        let mut l = Lift::new(&STOPS, 1.5, 2.0, 3.5);
        l.up();
        assert_eq!(l.phase, Phase::Idle);
        let mut b = Lift::new(&STOPS, 1.5, 2.0, -3.5);
        b.down();
        assert_eq!(b.phase, Phase::Idle);
    }

    #[test]
    fn a_call_while_moving_is_served_next() {
        let mut l = Lift::new(&STOPS, 1.5, 0.5, 3.5);
        l.send(1);
        run(&mut l, 1.0);
        l.send(0);
        run(&mut l, 30.0);
        assert!((l.car_y + 3.5).abs() < 1e-4, "the car goes on to the bottom after stopping at B");
    }
}
