//! The UI's motion (openspec/changes/ui-art-direction design 8 and 18a): how far a timed moment has gone, on which
//! curve, and a slide between two positions. Every timed moment of the screens around the consoles calls this, so the
//! durations and curves in `data/ui/ui_style.json` (`motion_ms`, `motion_ease`) are the only ones; no screen keeps its
//! own arithmetic. Pure functions of time, so they are tested without a window.

use serde::Deserialize;

/// A motion's curve (design 18a): none bounces or overshoots, so k stays in 0..=1.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Ease {
    /// k = t: the fades.
    Linear,
    /// k = 1 - (1 - t)^3: leaves at once and settles (the focus bar).
    OutCubic,
    /// Slow, fast, slow (the briefing lowering, the console rising).
    InOutCubic,
}

impl Ease {
    /// The curve at `t` (clamped to 0..=1).
    pub fn at(self, t: f32) -> f32 {
        let t = if t.is_finite() { t.clamp(0.0, 1.0) } else { 1.0 };
        match self {
            Ease::Linear => t,
            Ease::OutCubic => 1.0 - (1.0 - t).powi(3),
            Ease::InOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
        }
    }
}

/// How far a moment of `ms` milliseconds has gone after `elapsed_s` seconds, 0..=1 and linear (no duration: done).
pub fn progress(elapsed_s: f64, ms: f32) -> f32 {
    if ms <= 0.0 || !elapsed_s.is_finite() {
        return 1.0;
    }
    (elapsed_s * 1000.0 / f64::from(ms)).clamp(0.0, 1.0) as f32
}

/// How far a moment has gone on its curve: `progress` through `ease`.
pub fn eased(elapsed_s: f64, ms: f32, ease: Ease) -> f32 {
    ease.at(progress(elapsed_s, ms))
}

/// A value that slides to each new target (the focus bar's top edge): from wherever it is drawn when the target
/// changes, over `ms` on `ease`. The first target is taken at once (nothing to slide from).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Slide {
    from: f32,
    to: f32,
    since: f64,
    set: bool,
}

impl Slide {
    /// Where it is drawn at `now` (seconds), heading for `target`.
    pub fn at(&mut self, target: f32, now: f64, ms: f32, ease: Ease) -> f32 {
        if !self.set {
            *self = Slide { from: target, to: target, since: now, set: true };
        } else if target != self.to {
            // A new target mid-slide starts from where the bar is, never from the old item.
            let here = self.value(now, ms, ease);
            *self = Slide { from: here, to: target, since: now, set: true };
        }
        self.value(now, ms, ease)
    }

    fn value(&self, now: f64, ms: f32, ease: Ease) -> f32 {
        let k = eased(now - self.since, ms, ease);
        self.from + (self.to - self.from) * k
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_curve_starts_at_zero_ends_at_one_and_never_overshoots() {
        for e in [Ease::Linear, Ease::OutCubic, Ease::InOutCubic] {
            assert_eq!(e.at(0.0), 0.0, "{e:?}");
            assert_eq!(e.at(1.0), 1.0, "{e:?}");
            let mut last = 0.0;
            for i in 0..=100 {
                let k = e.at(i as f32 / 100.0);
                assert!((0.0..=1.0).contains(&k), "{e:?} at {i} is {k}: no bounce or overshoot (design 8)");
                assert!(k >= last, "{e:?} never goes back");
                last = k;
            }
        }
    }

    #[test]
    fn the_focus_curve_settles_and_the_briefing_curve_eases_both_ends() {
        assert!(Ease::OutCubic.at(0.5) > 0.85, "the focus bar is most of the way there half way through");
        assert!((Ease::InOutCubic.at(0.5) - 0.5).abs() < 1e-6, "in and out is symmetric");
        assert!(Ease::InOutCubic.at(0.1) < 0.01, "the briefing starts gently");
        assert!(Ease::InOutCubic.at(0.9) > 0.99, "and lands gently");
    }

    #[test]
    fn progress_runs_over_the_duration_and_clamps() {
        assert_eq!(progress(0.0, 90.0), 0.0);
        assert!((progress(0.045, 90.0) - 0.5).abs() < 1e-6);
        assert_eq!(progress(1.0, 90.0), 1.0);
        assert_eq!(progress(-1.0, 90.0), 0.0, "before it starts");
        assert_eq!(progress(0.01, 0.0), 1.0, "no duration: already done");
        assert_eq!(progress(f64::NAN, 90.0), 1.0, "a bad clock never leaves a moment half done");
    }

    #[test]
    fn a_slide_goes_from_the_old_target_to_the_new_one_over_its_duration() {
        let mut s = Slide::default();
        assert_eq!(s.at(100.0, 0.0, 90.0, Ease::OutCubic), 100.0, "the first target is taken at once");
        assert_eq!(s.at(172.0, 1.0, 90.0, Ease::OutCubic), 100.0, "a new target starts where the bar is");
        let mid = s.at(172.0, 1.045, 90.0, Ease::OutCubic);
        assert!(mid > 100.0 && mid < 172.0, "between the two half way: {mid}");
        assert_eq!(s.at(172.0, 1.09, 90.0, Ease::OutCubic), 172.0, "there at 90 ms");
    }

    #[test]
    fn a_slide_turned_mid_way_starts_from_where_it_is() {
        let mut s = Slide::default();
        s.at(0.0, 0.0, 100.0, Ease::Linear);
        s.at(100.0, 1.0, 100.0, Ease::Linear);
        let here = s.at(100.0, 1.05, 100.0, Ease::Linear);
        assert!((here - 50.0).abs() < 1e-3);
        assert!((s.at(0.0, 1.05, 100.0, Ease::Linear) - 50.0).abs() < 1e-3, "no jump back to the old item");
        assert!((s.at(0.0, 1.1, 100.0, Ease::Linear) - 25.0).abs() < 1e-3);
    }
}
