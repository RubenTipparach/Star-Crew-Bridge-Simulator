//! The fixed-step clock: simulation time in seconds and whole ticks, never frames (CLAUDE.md 6.4).
//!
//! It lives in the core because every rule steps on it, and a replay or a server must step it the
//! same way whatever the display rate. The caller feeds it the real seconds that passed; it says how
//! many ticks to run, keeping the remainder for the next call.

/// A fixed-step clock: `tick_hz` ticks a second, at most `max_ticks_per_advance` ticks per advance.
#[derive(Clone, Debug, PartialEq)]
pub struct FixedClock {
    tick_s: f64,
    max_ticks_per_advance: u32,
    tick: u64,
    carry_s: f64,
}

impl FixedClock {
    /// A clock at `tick_hz` ticks a second (30 for the server, design section 6) that runs at most
    /// `max_ticks_per_advance` ticks for one advance, so a long stall does not spiral.
    ///
    /// # Panics
    /// When `tick_hz` is not finite and positive or `max_ticks_per_advance` is zero: a clock without
    /// a step is a programming error, not data.
    pub fn new(tick_hz: f64, max_ticks_per_advance: u32) -> Self {
        assert!(tick_hz.is_finite() && tick_hz > 0.0, "tick rate must be finite and positive, got {tick_hz}");
        assert!(max_ticks_per_advance > 0, "a clock must be allowed at least one tick an advance");
        Self { tick_s: 1.0 / tick_hz, max_ticks_per_advance, tick: 0, carry_s: 0.0 }
    }

    /// The length of one tick in seconds.
    pub fn tick_s(&self) -> f64 {
        self.tick_s
    }

    /// Ticks run since the clock started.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Simulation time in seconds: whole ticks times the tick length (no drift from summing).
    pub fn time_s(&self) -> f64 {
        self.tick as f64 * self.tick_s
    }

    /// How far between the last tick and the next the real time is, 0 to 1, for interpolation.
    pub fn alpha(&self) -> f64 {
        self.carry_s / self.tick_s
    }

    /// Add `elapsed_s` real seconds and return the ticks to run now. Non-finite or negative input
    /// adds nothing (a guard at the edge, CLAUDE.md 6.6). Time beyond the cap is dropped.
    pub fn advance(&mut self, elapsed_s: f64) -> u32 {
        if elapsed_s.is_finite() && elapsed_s > 0.0 {
            self.carry_s += elapsed_s;
        }
        let mut n = 0;
        while self.carry_s >= self.tick_s && n < self.max_ticks_per_advance {
            self.carry_s -= self.tick_s;
            self.tick += 1;
            n += 1;
        }
        if n == self.max_ticks_per_advance && self.carry_s >= self.tick_s {
            self.carry_s %= self.tick_s;
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_at_thirty_hertz_is_thirty_ticks_whatever_the_frame_rate() {
        for fps in [24.0, 30.0, 60.0, 144.0] {
            let mut c = FixedClock::new(30.0, 8);
            let frames = fps as usize;
            let ticks: u32 = (0..frames).map(|_| c.advance(1.0 / fps)).sum();
            assert!((29..=30).contains(&ticks), "{fps} fps gave {ticks} ticks in a second");
        }
    }

    #[test]
    fn time_is_whole_ticks_with_no_drift() {
        let mut c = FixedClock::new(30.0, 1000);
        for _ in 0..30_000 {
            c.advance(1.0 / 30.0);
        }
        assert!((c.time_s() - c.tick() as f64 / 30.0).abs() < 1e-12, "time is ticks times the step");
    }

    #[test]
    fn a_long_stall_runs_at_most_the_cap() {
        let mut c = FixedClock::new(30.0, 4);
        assert_eq!(c.advance(10.0), 4, "a ten second stall must not run 300 ticks in one frame");
        assert!(c.alpha() < 1.0, "the dropped time is not carried");
    }

    #[test]
    fn bad_elapsed_time_adds_nothing() {
        let mut c = FixedClock::new(30.0, 4);
        assert_eq!(c.advance(f64::NAN), 0);
        assert_eq!(c.advance(-1.0), 0);
        assert_eq!(c.advance(f64::INFINITY), 0);
        assert_eq!(c.tick(), 0);
    }
}
