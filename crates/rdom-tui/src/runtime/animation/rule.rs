//! Which `transition-*` entry covers a property (CSS Transitions 1 §2):
//! the entry of `transition-property` naming it — the last one when
//! several do — and the duration, easing and delay at that index, the
//! other lists repeated to `transition-property`'s length.

use std::time::{Duration, Instant};

use rdom_style::animation::{Longhand, transition_longhands};

use crate::style::ComputedStyle;
use crate::style::transition::{TimingFunction, TransitionProperty};

/// The timing of one matched `transition-*` entry.
#[derive(Debug, Clone)]
pub(super) struct Rule {
    pub duration_ms: u32,
    pub timing: TimingFunction,
    /// Negative: the transition starts part-way (CSS Transitions 1 §2.4).
    pub delay_ms: i32,
}

/// When a transition registered at `now` under a rule runs.
#[derive(Debug, Clone, Copy)]
pub(super) struct Clock {
    /// When its delay began — before `now` by the part a negative delay
    /// skips.
    pub started_at: Instant,
    /// The delay still to wait: zero for a negative one.
    pub delay: Duration,
    /// The part of the duration a negative delay skipped:
    /// `min(max(-delay, 0), duration)`, the `elapsedTime` of its
    /// `transitionrun` and `transitionstart` (§6).
    pub skipped: Duration,
    pub duration: Duration,
}

impl Rule {
    /// Whether a change under this rule transitions: its combined
    /// duration — `max(duration, 0) + delay` — is positive (CSS
    /// Transitions 1 §3); otherwise the value changes at once.
    pub(super) fn runs(&self) -> bool {
        i64::from(self.duration_ms) + i64::from(self.delay_ms) > 0
    }

    /// The clock of a transition this rule starts at `now`.
    pub(super) fn clock(&self, now: Instant) -> Clock {
        let duration = Duration::from_millis(u64::from(self.duration_ms));
        let delay = Duration::from_millis(u64::from(self.delay_ms.max(0).unsigned_abs()));
        let skipped =
            Duration::from_millis(u64::from(self.delay_ms.min(0).unsigned_abs())).min(duration);
        Clock {
            started_at: now.checked_sub(skipped).unwrap_or(now),
            delay,
            skipped,
            duration,
        }
    }
}

/// Whether any entry of `style` could start a transition — the diff's
/// fast path for the elements that declare none.
pub(super) fn any(style: &ComputedStyle) -> bool {
    !style.transition_property.is_empty()
        && (style.transition_duration.iter().any(|d| *d > 0)
            || style.transition_delay.iter().any(|d| *d > 0))
}

/// The entry at `idx`, the other lists cycled (§2: "the values are
/// repeated as necessary").
fn at(style: &ComputedStyle, idx: usize) -> Rule {
    fn cycle<T: Clone>(list: &[T], idx: usize, default: T) -> T {
        if list.is_empty() {
            default
        } else {
            list[idx % list.len()].clone()
        }
    }
    Rule {
        duration_ms: cycle(&style.transition_duration, idx, 0),
        timing: cycle(&style.transition_timing_function, idx, TimingFunction::Ease),
        delay_ms: cycle(&style.transition_delay, idx, 0),
    }
}

/// Which `transition-property` entry covers each longhand of a style.
pub(super) struct Coverage(Vec<Option<u16>>);

impl Coverage {
    /// The coverage of `style`'s `transition-property`: a later entry
    /// naming a longhand wins (§2: "the last occurrence"); `none` covers
    /// nothing; `all` covers every animatable longhand.
    pub(super) fn of(style: &ComputedStyle) -> Coverage {
        let mut slots = vec![None; Longhand::all().count()];
        let direction = style.text_direction;
        for (i, p) in style.transition_property.iter().enumerate() {
            let i = Some(i as u16);
            match p {
                TransitionProperty::All => {
                    for l in Longhand::all().filter(|l| l.is_animatable()) {
                        slots[l.index()] = i;
                    }
                }
                TransitionProperty::Named(name) => {
                    for l in transition_longhands(name, direction) {
                        slots[l.index()] = i;
                    }
                }
                TransitionProperty::None | TransitionProperty::Other(_) => {}
            }
        }
        Coverage(slots)
    }

    /// The rule covering `l` in `style`, if any.
    pub(super) fn rule(&self, style: &ComputedStyle, l: Longhand) -> Option<Rule> {
        self.0[l.index()].map(|i| at(style, usize::from(i)))
    }
}

/// The rule covering the custom property `--name` (`all` or the name
/// itself).
pub(super) fn custom(style: &ComputedStyle, name: &str) -> Option<Rule> {
    let idx = style.transition_property.iter().rposition(|p| match p {
        TransitionProperty::All => true,
        TransitionProperty::Other(n) => n.strip_prefix("--") == Some(name),
        _ => false,
    })?;
    Some(at(style, idx))
}
