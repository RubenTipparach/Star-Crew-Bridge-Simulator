//! The crew on foot (openspec/changes/coop-drill design 9): every crew member's body on the bridge, and changing
//! station as a walk: stand, follow a path to the seat at the walk speed, sit, and only then operate it.
//!
//! It lives in the core because the server runs it and the snapshot carries it (CLAUDE.md 6.3). The geometry comes
//! from outside: the caller builds a [`Bridge`] from the layout's seats and the deck's walk grid (`crate::nav`), so the
//! drill's tests can walk a bridge of straight lines with no deck file.

use crate::combat::Station;

/// Where a body can be: a seat, or its place at the muster point at the back of the bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spot {
    /// A station's seat.
    Seat(Station),
    /// The back of the bridge, where a body with no station stands.
    Muster,
}

/// What a body is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Posture {
    /// In a seat.
    Seated = 0,
    /// Standing at the muster point.
    Standing = 1,
    /// Getting up from a seat.
    Rising = 2,
    /// On its way.
    Walking = 3,
    /// Sitting down.
    Sitting = 4,
}

impl Posture {
    /// From its wire byte.
    pub fn from_u8(v: u8) -> Option<Self> {
        [Self::Seated, Self::Standing, Self::Rising, Self::Walking, Self::Sitting].into_iter().find(|p| *p as u8 == v)
    }
}

/// The bridge the drill's crew walk on: its seats, the muster point and the paths between them.
#[derive(Clone, Debug, PartialEq)]
pub struct Bridge {
    /// Each station's seat in the ship frame (metres) and the way it faces (radians about +Y, 0 to the bow).
    pub seats: Vec<(Station, [f32; 3], f32)>,
    /// The muster point at the back of the bridge.
    pub muster: [f32; 3],
    /// Between bodies standing at the muster point, metres.
    pub spacing_m: f32,
    /// The walk speed, m/s (`data/crew/walk.json`).
    pub walk_m_s: f32,
    /// Getting up, seconds.
    pub stand_s: f32,
    /// Sitting down, seconds.
    pub sit_s: f32,
    paths: Vec<(Spot, Spot, Vec<[f32; 3]>)>,
}

impl Bridge {
    /// A bridge whose paths come from `route(from, to)` (the walk grid's A*, or straight lines in tests), computed once
    /// for every pair of spots.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        seats: Vec<(Station, [f32; 3], f32)>,
        muster: [f32; 3],
        spacing_m: f32,
        walk_m_s: f32,
        stand_s: f32,
        sit_s: f32,
        mut route: impl FnMut([f32; 3], [f32; 3]) -> Vec<[f32; 3]>,
    ) -> Self {
        let mut b = Self { seats, muster, spacing_m, walk_m_s, stand_s, sit_s, paths: Vec::new() };
        let spots: Vec<Spot> = b.seats.iter().map(|s| Spot::Seat(s.0)).chain([Spot::Muster]).collect();
        for &a in &spots {
            for &z in &spots {
                if a != z {
                    let (pa, pz) = (b.at(a, None), b.at(z, None));
                    let mut p = route(pa, pz);
                    if p.first() != Some(&pa) {
                        p.insert(0, pa);
                    }
                    if p.last() != Some(&pz) {
                        p.push(pz);
                    }
                    b.paths.push((a, z, p));
                }
            }
        }
        b
    }

    /// Straight lines between every spot (the drill's tests, and a server with no compiled deck).
    #[allow(clippy::too_many_arguments)]
    pub fn straight(
        seats: Vec<(Station, [f32; 3], f32)>,
        muster: [f32; 3],
        spacing_m: f32,
        walk_m_s: f32,
        stand_s: f32,
        sit_s: f32,
    ) -> Self {
        Self::new(seats, muster, spacing_m, walk_m_s, stand_s, sit_s, |a, b| vec![a, b])
    }

    /// Where `spot` is; at the muster point, slot `slot`'s own place in the line.
    pub fn at(&self, spot: Spot, slot: Option<u8>) -> [f32; 3] {
        match spot {
            Spot::Seat(s) => self.seats.iter().find(|x| x.0 == s).map_or(self.muster, |x| x.1),
            Spot::Muster => {
                let k = f32::from(slot.unwrap_or(0) % 8);
                let off = (k - 3.5) * self.spacing_m;
                [self.muster[0] + off, self.muster[1], self.muster[2]]
            }
        }
    }

    /// Whether the bridge has a seat for `s`.
    pub fn has(&self, s: Station) -> bool {
        self.seats.iter().any(|x| x.0 == s)
    }

    fn yaw(&self, spot: Spot) -> f32 {
        match spot {
            Spot::Seat(s) => self.seats.iter().find(|x| x.0 == s).map_or(0.0, |x| x.2),
            Spot::Muster => 0.0,
        }
    }

    /// The path from one spot to another for `slot`: its own muster place at either end.
    pub fn path(&self, from: Spot, to: Spot, slot: u8) -> Vec<[f32; 3]> {
        let mut p = self.paths.iter().find(|x| x.0 == from && x.1 == to).map(|x| x.2.clone()).unwrap_or_default();
        if from == Spot::Muster {
            p.insert(0, self.at(Spot::Muster, Some(slot)));
        }
        if to == Spot::Muster {
            p.push(self.at(Spot::Muster, Some(slot)));
        }
        p
    }

    /// How long a walk takes from seat to seat: getting up, the path, sitting down (the board's and the tests' time).
    pub fn walk_time(&self, from: Spot, to: Spot, slot: u8) -> f32 {
        let p = self.path(from, to, slot);
        let len: f32 = p.windows(2).map(|w| dist(w[0], w[1])).sum();
        let up = if matches!(from, Spot::Seat(_)) { self.stand_s } else { 0.0 };
        let down = if matches!(to, Spot::Seat(_)) { self.sit_s } else { 0.0 };
        up + len / self.walk_m_s.max(0.1) + down
    }
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// One crew member's body.
#[derive(Clone, Debug, PartialEq)]
pub struct CrewBody {
    /// The player's slot.
    pub slot: u8,
    /// Where it is, ship frame, metres.
    pub pos: [f32; 3],
    /// The way it faces, radians about +Y (0 to the bow).
    pub yaw: f32,
    /// What it is doing.
    pub posture: Posture,
    /// The spot it is at, or left from while walking.
    pub at: Spot,
    /// The seat it is walking to, if any.
    pub going: Option<Spot>,
    path: Vec<[f32; 3]>,
    leg: usize,
    timer: f32,
}

impl CrewBody {
    /// A body standing at its muster place.
    pub fn at_muster(slot: u8, b: &Bridge) -> Self {
        Self {
            slot,
            pos: b.at(Spot::Muster, Some(slot)),
            yaw: 0.0,
            posture: Posture::Standing,
            at: Spot::Muster,
            going: None,
            path: Vec::new(),
            leg: 0,
            timer: 0.0,
        }
    }

    /// The station it is walking to.
    pub fn walking_to(&self) -> Option<Station> {
        match self.going {
            Some(Spot::Seat(s)) => Some(s),
            _ => None,
        }
    }

    /// On its way somewhere (getting up, walking or sitting down).
    pub fn busy(&self) -> bool {
        self.going.is_some()
    }

    /// Set off for `to`: up from a seat first.
    pub fn go(&mut self, to: Spot, b: &Bridge) {
        if self.at == to && self.going.is_none() {
            return;
        }
        self.path = b.path(self.at, to, self.slot);
        self.leg = 0;
        self.going = Some(to);
        if matches!(self.at, Spot::Seat(_)) && self.posture == Posture::Seated {
            self.posture = Posture::Rising;
            self.timer = b.stand_s;
        } else {
            self.posture = Posture::Walking;
        }
    }

    /// Step `dt` seconds. Returns the seat it sat down in this step, if any.
    pub fn step(&mut self, b: &Bridge, dt: f32) -> Option<Station> {
        match self.posture {
            Posture::Seated | Posture::Standing => None,
            Posture::Rising => {
                self.timer -= dt;
                if self.timer <= 0.0 {
                    self.posture = Posture::Walking;
                }
                None
            }
            Posture::Walking => {
                let mut left = b.walk_m_s * dt;
                while left > 0.0 {
                    let Some(&next) = self.path.get(self.leg + 1) else { break };
                    let d = dist(self.pos, next);
                    if d > 1e-4 {
                        self.yaw = (next[0] - self.pos[0]).atan2(next[2] - self.pos[2]);
                    }
                    if d <= left {
                        self.pos = next;
                        self.leg += 1;
                        left -= d;
                    } else {
                        let k = left / d;
                        for (p, n) in self.pos.iter_mut().zip(next) {
                            *p += (n - *p) * k;
                        }
                        left = 0.0;
                    }
                }
                if self.leg + 1 >= self.path.len() {
                    let to = self.going.unwrap_or(Spot::Muster);
                    self.at = to;
                    self.pos = b.at(to, Some(self.slot));
                    match to {
                        Spot::Seat(_) => {
                            self.posture = Posture::Sitting;
                            self.timer = b.sit_s;
                        }
                        Spot::Muster => {
                            self.posture = Posture::Standing;
                            self.going = None;
                            self.yaw = 0.0;
                        }
                    }
                }
                None
            }
            Posture::Sitting => {
                self.timer -= dt;
                self.yaw = b.yaw(self.at);
                if self.timer <= 0.0 {
                    self.posture = Posture::Seated;
                    self.going = None;
                    if let Spot::Seat(s) = self.at {
                        return Some(s);
                    }
                }
                None
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The Tern's bridge seats (layout.json) and muster point, straight paths.
    pub fn bridge() -> Bridge {
        Bridge::straight(
            vec![(Station::Helm, [1.8, 3.5, 28.2], 0.0), (Station::Tactical, [-1.8, 3.5, 28.2], 0.0)],
            [0.0, 3.5, 21.2],
            0.9,
            2.4,
            0.4,
            0.4,
        )
    }

    #[test]
    fn a_body_walks_from_the_muster_point_to_a_seat_and_sits() {
        let b = bridge();
        let mut body = CrewBody::at_muster(3, &b);
        body.go(Spot::Seat(Station::Helm), &b);
        let want = b.walk_time(Spot::Muster, Spot::Seat(Station::Helm), 3);
        let mut t = 0.0;
        let mut sat = None;
        while sat.is_none() && t < 20.0 {
            sat = body.step(&b, 1.0 / 30.0);
            t += 1.0 / 30.0;
        }
        assert_eq!(sat, Some(Station::Helm));
        assert!((t - want).abs() < 0.1, "the walk takes the time the bridge says: {t} against {want}");
        assert!(want < 6.0, "a bridge seat is a few seconds away: {want}");
        assert_eq!(body.pos, [1.8, 3.5, 28.2]);
        assert_eq!(body.posture, Posture::Seated);
    }

    #[test]
    fn getting_up_comes_before_the_walk() {
        let b = bridge();
        let mut body = CrewBody::at_muster(0, &b);
        body.go(Spot::Seat(Station::Tactical), &b);
        while body.step(&b, 0.05).is_none() {}
        body.go(Spot::Seat(Station::Helm), &b);
        assert_eq!(body.posture, Posture::Rising);
        body.step(&b, 0.2);
        assert_eq!(body.pos, [-1.8, 3.5, 28.2], "still in the chair while it gets up");
        body.step(&b, 0.3);
        assert_eq!(body.posture, Posture::Walking);
    }
}
