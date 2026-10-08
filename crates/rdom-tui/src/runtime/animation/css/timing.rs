//! The Web Animations 1 timing model (§4) for one CSS animation: from a
//! local time to its phase, active time, current iteration and directed
//! progress — what its keyframes are sampled at and which events fire.
//!
//! A CSS animation has no effect easing (its `animation-timing-function`
//! eases each keyframe interval, CSS Animations 1 §3), an iteration start
//! of 0, an end delay of 0 and a playback rate of 1.

use crate::style::{AnimationDirection, AnimationFillMode};

/// One animation's timing (CSS Animations 1 §4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Timing {
    /// The iteration duration, in ms (`animation-duration`).
    pub duration: f64,
    /// The start delay, in ms (`animation-delay`); negative starts
    /// part-way.
    pub delay: f64,
    /// The iteration count (`animation-iteration-count`), `INFINITY` for
    /// `infinite`.
    pub iterations: f64,
    pub direction: AnimationDirection,
    pub fill: AnimationFillMode,
}

/// Web Animations 1 §4.6.4: where a local time falls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Before,
    Active,
    After,
}

/// What an animation is at one local time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Sample {
    pub phase: Phase,
    /// The directed progress its keyframes are sampled at — `None` when
    /// the animation has no effect (outside its active interval, unfilled).
    pub progress: Option<f64>,
    /// The current iteration (§4.8), when there is an active time.
    pub iteration: Option<f64>,
}

impl Timing {
    /// §4.7.2: the active duration — `0` when either factor is.
    pub(crate) fn active_duration(&self) -> f64 {
        if self.duration == 0.0 || self.iterations == 0.0 {
            0.0
        } else {
            self.duration * self.iterations
        }
    }

    /// §4.6.3: the end time (no end delay).
    fn end_time(&self) -> f64 {
        (self.delay + self.active_duration()).max(0.0)
    }

    /// §4.6.4: the phase at local time `t` (a forwards animation).
    pub(crate) fn phase(&self, t: f64) -> Phase {
        let end = self.end_time();
        let before_active = self.delay.min(end).max(0.0);
        let active_after = (self.delay + self.active_duration()).min(end).max(0.0);
        if t < before_active {
            Phase::Before
        } else if t >= active_after {
            Phase::After
        } else {
            Phase::Active
        }
    }

    /// §4.8.3: the active time at `t` in `phase`; `None` where the fill
    /// mode leaves the animation without an effect.
    fn active_time(&self, t: f64, phase: Phase) -> Option<f64> {
        match phase {
            Phase::Before => self.fill.backwards().then(|| (t - self.delay).max(0.0)),
            Phase::Active => Some(t - self.delay),
            Phase::After => self
                .fill
                .forwards()
                .then(|| (t - self.delay).min(self.active_duration()).max(0.0)),
        }
    }

    /// The animation at local time `t` (§4.8–§4.10). On a progress-based
    /// timeline (`progress_based`) the end of the active interval is still
    /// active (Web Animations 2 §4.6.4.1: a scroll at its end shows the
    /// last keyframe, not the after phase).
    pub(crate) fn sample(&self, t: f64, progress_based: bool) -> Sample {
        let mut phase = self.phase(t);
        if progress_based
            && phase == Phase::After
            && t == (self.delay + self.active_duration())
                .min(self.end_time())
                .max(0.0)
            && self.active_duration() > 0.0
        {
            phase = Phase::Active;
        }
        let Some(active) = self.active_time(t, phase) else {
            return Sample {
                phase,
                progress: None,
                iteration: None,
            };
        };
        // §4.8.3.1: the overall progress.
        let overall = if self.duration == 0.0 {
            if phase == Phase::Before {
                0.0
            } else {
                self.iterations
            }
        } else {
            active / self.duration
        };
        // §4.8.3.2: the simple iteration progress.
        let mut simple = if overall.is_infinite() {
            0.0
        } else {
            overall % 1.0
        };
        if simple == 0.0
            && matches!(phase, Phase::Active | Phase::After)
            && active == self.active_duration()
            && self.iterations != 0.0
        {
            simple = 1.0;
        }
        // §4.8.4: the current iteration.
        let iteration = if phase == Phase::After && self.iterations.is_infinite() {
            f64::INFINITY
        } else if simple == 1.0 {
            overall.floor() - 1.0
        } else {
            overall.floor()
        };
        Sample {
            phase,
            progress: Some(self.directed(simple, iteration)),
            iteration: Some(iteration),
        }
    }

    /// §4.9.1: the directed progress.
    fn directed(&self, simple: f64, iteration: f64) -> f64 {
        if self.forwards(iteration) {
            simple
        } else {
            1.0 - simple
        }
    }

    /// §4.9.1: whether iteration `iteration` runs forwards.
    pub(crate) fn forwards(&self, iteration: f64) -> bool {
        match self.direction {
            AnimationDirection::Normal => true,
            AnimationDirection::Reverse => false,
            AnimationDirection::Alternate | AnimationDirection::AlternateReverse => {
                let d = if self.direction == AnimationDirection::AlternateReverse {
                    iteration + 1.0
                } else {
                    iteration
                };
                d.is_infinite() || d % 2.0 == 0.0
            }
        }
    }

    /// CSS Animations 2 §4.2: the interval start — `elapsedTime` of an
    /// `animationstart` out of the before phase.
    pub(crate) fn interval_start(&self) -> f64 {
        (-self.delay).min(self.active_duration()).max(0.0)
    }

    /// CSS Animations 2 §4.2: the interval end — `elapsedTime` of an
    /// `animationend` into the after phase.
    pub(crate) fn interval_end(&self) -> f64 {
        (self.end_time() - self.delay)
            .min(self.active_duration())
            .max(0.0)
    }

    /// The active time at `t` whatever the fill mode, clamped to the
    /// active interval — an `animationcancel`'s `elapsedTime`.
    pub(crate) fn clamped_active_time(&self, t: f64) -> f64 {
        (t - self.delay).min(self.active_duration()).max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing(duration: f64, delay: f64, iterations: f64) -> Timing {
        Timing {
            duration,
            delay,
            iterations,
            direction: AnimationDirection::Normal,
            fill: AnimationFillMode::None,
        }
    }

    /// Web Animations 1 §4.6.4: the phases around a delay and the end.
    #[test]
    fn phases_split_at_the_delay_and_the_end() {
        let t = timing(100.0, 50.0, 1.0);
        assert_eq!(t.phase(0.0), Phase::Before);
        assert_eq!(t.phase(50.0), Phase::Active);
        assert_eq!(t.phase(149.0), Phase::Active);
        assert_eq!(t.phase(150.0), Phase::After);
    }

    /// §4.8.3.2: the end of the last iteration is progress 1 of that
    /// iteration, not 0 of the next.
    #[test]
    fn the_end_is_progress_one_of_the_last_iteration() {
        let mut t = timing(100.0, 0.0, 2.0);
        t.fill = AnimationFillMode::Forwards;
        let s = t.sample(500.0, false);
        assert_eq!((s.progress, s.iteration), (Some(1.0), Some(1.0)));
    }

    /// §4.8.3.1: a zero duration is at the iteration count in the after
    /// phase — the last keyframe under `forwards`.
    #[test]
    fn a_zero_duration_jumps_to_the_end() {
        let mut t = timing(0.0, 0.0, 1.0);
        t.fill = AnimationFillMode::Both;
        assert_eq!(t.sample(0.0, false).progress, Some(1.0));
        assert_eq!(t.phase(0.0), Phase::After);
    }

    /// §4.9.1: `alternate-reverse` starts backwards.
    #[test]
    fn alternate_reverse_starts_backwards() {
        let mut t = timing(100.0, 0.0, 2.0);
        t.direction = AnimationDirection::AlternateReverse;
        assert_eq!(t.sample(25.0, false).progress, Some(0.75));
        assert_eq!(t.sample(125.0, false).progress, Some(0.25));
    }

    /// CSS Animations 2 §4.2: the interval bounds.
    #[test]
    fn interval_bounds_follow_the_delay() {
        assert_eq!(timing(100.0, -40.0, 1.0).interval_start(), 40.0);
        assert_eq!(timing(100.0, 20.0, 2.0).interval_end(), 200.0);
        assert_eq!(timing(100.0, -300.0, 2.0).interval_end(), 200.0);
    }
}
