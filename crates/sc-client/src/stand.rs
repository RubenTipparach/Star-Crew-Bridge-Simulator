//! Getting up from a seat (openspec/changes/leave-station design 2 and 4): the stand key held `seats.stand_hold_s`,
//! counted only from a press made in the seat, or a click on LEAVE STATION.
//!
//! It is the client's because it reads keys; the rule it ends in, the core's `Drill::stand`, is the server's. A
//! seated player's hold here is what LEAVE STATION's ring shows.

/// The stand key's hold.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StandHold {
    /// Seconds the key has been held since a press made in the seat.
    held_s: f64,
    /// A press made in the seat counts: the key was up at some moment while seated.
    armed: bool,
    /// Stood already on this hold: nothing more until the key is let go.
    spent: bool,
}

impl StandHold {
    /// A frame: `seated` (in a seat, the menu shut), the key `down`, `dt` seconds, `hold_s` to stand. Returns whether
    /// to stand now.
    pub fn step(&mut self, seated: bool, down: bool, dt: f64, hold_s: f64) -> bool {
        if !seated {
            *self = Self::default();
            return false;
        }
        if !down {
            self.armed = true;
            self.spent = false;
            self.held_s = 0.0;
            return false;
        }
        if !self.armed || self.spent {
            return false;
        }
        self.held_s += dt;
        if self.held_s >= hold_s {
            self.spent = true;
            self.held_s = 0.0;
            return true;
        }
        false
    }

    /// How much of the hold has passed, 0..1: LEAVE STATION's ring.
    pub fn share(&self, hold_s: f64) -> f64 {
        if hold_s <= 0.0 {
            return 0.0;
        }
        (self.held_s / hold_s).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOLD_S: f64 = 0.5;
    const DT: f64 = 1.0 / 60.0;

    /// Hold the key `secs` while seated; whether it stood.
    fn hold(h: &mut StandHold, secs: f64) -> bool {
        let mut stood = false;
        let mut t = 0.0;
        while t < secs - 1e-9 {
            stood |= h.step(true, true, DT, HOLD_S);
            t += DT;
        }
        stood
    }

    #[test]
    fn holding_the_key_half_a_second_in_the_seat_stands() {
        let mut h = StandHold::default();
        h.step(true, false, DT, HOLD_S);
        assert!(!hold(&mut h, 0.3), "a short hold does nothing");
        assert!(h.share(HOLD_S) > 0.5, "the ring shows the hold");
        h.step(true, false, DT, HOLD_S);
        assert_eq!(h.share(HOLD_S), 0.0, "let go, the ring empties");
        assert!(hold(&mut h, 0.55), "a full hold stands");
    }

    #[test]
    fn the_key_that_sat_you_down_does_not_stand_you_up() {
        // Regression (leave-station 2): E held from sitting stood the body straight back up after half a second.
        let mut h = StandHold::default();
        h.step(false, true, DT, HOLD_S);
        assert!(!hold(&mut h, 2.0), "still held from the walk: not a press in the seat");
        h.step(true, false, DT, HOLD_S);
        assert!(hold(&mut h, 0.55), "let go and pressed again: it stands");
    }

    #[test]
    fn one_hold_stands_once() {
        let mut h = StandHold::default();
        h.step(true, false, DT, HOLD_S);
        assert!(hold(&mut h, 0.55));
        assert!(!hold(&mut h, 2.0), "kept held after standing, nothing more");
    }
}
