//! Which `transition-*` entry covers a property (CSS Transitions 1 §2):
//! the entry of `transition-property` naming it — the last one when
//! several do — and the duration, easing and delay at that index, the
//! other lists repeated to `transition-property`'s length.

use rdom_style::animation::{Longhand, transition_longhands};

use crate::style::ComputedStyle;
use crate::style::transition::{TimingFunction, TransitionProperty};

/// The timing of one matched `transition-*` entry.
#[derive(Debug, Clone, Copy)]
pub(super) struct Rule {
    pub duration_ms: u32,
    pub timing: TimingFunction,
    pub delay_ms: u32,
}

impl Rule {
    /// Whether a change under this rule transitions: a zero duration and
    /// delay commit the value at once (CSS Transitions 1 §3, "combined
    /// duration").
    pub(super) fn runs(&self) -> bool {
        self.duration_ms > 0 || self.delay_ms > 0
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
    fn cycle<T: Copy>(list: &[T], idx: usize, default: T) -> T {
        if list.is_empty() {
            default
        } else {
            list[idx % list.len()]
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
