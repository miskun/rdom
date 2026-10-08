//! Transition data model — parsed `transition-*` declarations
//! shared between `TuiStyle` (cascade input) and the runtime
//! animation engine (M3 Part B).
//!
//! A transition rule says "when property `P` changes on this
//! element, animate it over `duration` with `timing` after a
//! `delay`." The cascade carries the parsed rules into
//! `ComputedStyle.transitions`; the runtime's animation engine
//! consults that list when it observes a property change to
//! decide whether to interpolate.

/// One parsed transition rule.
#[derive(Debug, Clone, PartialEq)]
pub struct TransitionRule {
    pub property: TransitionProperty,
    pub duration_ms: u32,
    pub timing: TimingFunction,
    /// Negative: the transition starts part-way (CSS Transitions 1 §2.4).
    pub delay_ms: i32,
}

/// Which property a transition rule covers (CSS Transitions 1 §2.1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransitionProperty {
    /// `transition-property: all` — every animatable property
    /// transitions on change.
    All,
    /// `transition-property: none` — disables transitions.
    None,
    /// A property the dispatch table knows ([`PropertyName`]): a
    /// longhand, a shorthand (its longhands transition) or a
    /// flow-relative property (its physical twin's). A discrete or
    /// not-animatable one starts no transition unless the
    /// [`animation`](crate::animation) type allows it.
    Named(PropertyName),
    /// Any other `<custom-ident>` — a custom property (`--x`, which
    /// transitions when registered) or a name rdom does not know. Valid
    /// CSS (Transitions 1 §2.1) and kept for its place in the list.
    Other(String),
}

impl TransitionProperty {
    /// The entry naming `name`: [`Named`](Self::Named) for a property the
    /// dispatch table knows (ASCII case-insensitive), else
    /// [`Other`](Self::Other). `all` and `none` are their keywords.
    pub fn named(name: &str) -> TransitionProperty {
        if name.starts_with("--") {
            return TransitionProperty::Other(name.to_string());
        }
        let lower = name.to_ascii_lowercase();
        match lower.as_str() {
            "all" => TransitionProperty::All,
            "none" => TransitionProperty::None,
            _ => match PropertyName::new(&lower) {
                Some(n) => TransitionProperty::Named(n),
                None => TransitionProperty::Other(lower),
            },
        }
    }

    /// The name as written in the value (`all`, `none`, a property).
    pub fn name(&self) -> &str {
        match self {
            TransitionProperty::All => "all",
            TransitionProperty::None => "none",
            TransitionProperty::Named(n) => n.as_str(),
            TransitionProperty::Other(n) => n,
        }
    }
}

/// The canonical name of a property the dispatch table knows
/// ([`property_names`](crate::property_dispatch::property_names)) — what
/// a [`TransitionProperty::Named`] entry holds. Built only from a known
/// name, so a misspelt one (`colour`) cannot become a rule that never
/// fires: [`TransitionProperty::named`] keeps it in `Other`, as CSS keeps
/// an unknown `<custom-ident>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PropertyName(&'static str);

impl PropertyName {
    /// The property `name` names, ASCII case-insensitively; `None` for a
    /// custom property or a name rdom does not know.
    pub fn new(name: &str) -> Option<PropertyName> {
        let lower = name.to_ascii_lowercase();
        crate::property_dispatch::property_names()
            .iter()
            .find(|n| **n == lower)
            .map(|n| PropertyName(n))
    }

    /// The canonical (lower-case) name.
    pub fn as_str(&self) -> &'static str {
        self.0
    }
}

impl std::fmt::Display for PropertyName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

/// `transition-behavior` (CSS Transitions 2 §3.1): whether a property
/// whose values do not interpolate (a discrete one, or a pair that does
/// not interpolate) transitions. Closed (DESIGN): the grammar's two values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TransitionBehavior {
    /// Only interpolable values transition (the initial value).
    #[default]
    Normal,
    /// Discrete values transition too, stepping at the midpoint —
    /// `display` and `visibility` by their own rules.
    AllowDiscrete,
}

/// Where a `steps()` easing jumps (CSS Easing 1 §2.3). `start` /
/// `end` are the `jump-start` / `jump-end` aliases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StepPosition {
    /// `jump-start` / `start`: the first jump happens at 0.
    Start,
    /// `jump-end` / `end` (the default): the last jump happens at 1.
    #[default]
    End,
    /// `jump-none`: no jump at either end; `n - 1` jumps inside.
    JumpNone,
    /// `jump-both`: jumps at both ends; `n + 1` jumps in total.
    JumpBoth,
}

/// One point of a `linear()` easing (CSS Easing 2 §2.1): at input
/// progress `input` the output progress is `output`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearStop {
    pub output: f32,
    pub input: f32,
}

impl LinearStop {
    /// The point with output progress `output` at input progress `input`
    /// — CSS's order, `linear(<output> <input>%)`: `linear(0, 0.25 75%,
    /// 1)`'s middle stop is `LinearStop::new(0.25, 0.75)`.
    pub const fn new(output: f32, input: f32) -> Self {
        LinearStop { output, input }
    }
}

/// `<easing-function>` (CSS Easing 1 / 2): the keyword curves,
/// `linear(<stops>)`, `cubic-bezier(x1, y1, x2, y2)` and
/// `steps(n, <position>)`. `PartialEq` only — the curves carry `f32`s.
/// Not `Copy`: `linear()`'s points are shared behind an `Arc`.
///
/// `#[non_exhaustive]` (DESIGN): CSS Easing keeps adding easing
/// functions, and an easing is evaluated here ([`ease`](Self::ease)), so
/// no consumer matches one to animate.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum TimingFunction {
    /// Identity — `t` linearly maps to itself.
    Linear,
    /// CSS default — `cubic-bezier(0.25, 0.1, 0.25, 1.0)`.
    #[default]
    Ease,
    /// `cubic-bezier(0.42, 0, 1.0, 1.0)`.
    EaseIn,
    /// `cubic-bezier(0, 0, 0.58, 1.0)`.
    EaseOut,
    /// `cubic-bezier(0.42, 0, 0.58, 1.0)`.
    EaseInOut,
    /// `cubic-bezier(x1, y1, x2, y2)`; `x1`, `x2` ∈ [0, 1] (the
    /// parser rejects anything else), `y` unbounded.
    CubicBezier { x1: f32, y1: f32, x2: f32, y2: f32 },
    /// `steps(count, position)`; `count ≥ 1` (`≥ 2` for `jump-none`).
    Steps { count: u32, position: StepPosition },
    /// `linear(<linear-stop-list>)` (CSS Easing 2 §2.1): a piecewise
    /// linear curve through these points, canonical — at least two,
    /// inputs non-decreasing.
    LinearStops(std::sync::Arc<[LinearStop]>),
}

impl TimingFunction {
    /// `step-start` = `steps(1, jump-start)`.
    pub const STEP_START: TimingFunction = TimingFunction::Steps {
        count: 1,
        position: StepPosition::Start,
    };
    /// `step-end` = `steps(1, jump-end)`.
    pub const STEP_END: TimingFunction = TimingFunction::Steps {
        count: 1,
        position: StepPosition::End,
    };

    /// Map normalized linear progress `t` ∈ [0, 1] to eased
    /// progress. Cubic-bezier evaluation via Newton's method —
    /// 5 iterations gives <1% error which is well below cell-
    /// grid quantization noise.
    pub fn ease(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match *self {
            TimingFunction::Linear => t,
            TimingFunction::Ease => bezier(0.25, 0.1, 0.25, 1.0, t),
            TimingFunction::EaseIn => bezier(0.42, 0.0, 1.0, 1.0, t),
            TimingFunction::EaseOut => bezier(0.0, 0.0, 0.58, 1.0, t),
            TimingFunction::EaseInOut => bezier(0.42, 0.0, 0.58, 1.0, t),
            TimingFunction::CubicBezier { x1, y1, x2, y2 } => bezier(x1, y1, x2, y2, t),
            TimingFunction::Steps { count, position } => steps(count, position, t, false),
            TimingFunction::LinearStops(ref stops) => linear(stops, t),
        }
    }

    /// [`ease`](Self::ease) in the before phase (a transition's delay):
    /// CSS Easing 1 §2.3.1's "before flag" holds a step easing's jump at
    /// 0 back, so the start value shows until the delay ends.
    pub fn ease_before(&self, t: f32) -> f32 {
        match *self {
            TimingFunction::Steps { count, position } => {
                steps(count, position, t.clamp(0.0, 1.0), true)
            }
            _ => self.ease(t),
        }
    }
}

/// CSS Easing 2 §2.1.2, "calculate linear easing output progress": point
/// A is the last point whose input is at most `t` (the first when none
/// is, the one before the last when it is the last), point B the next;
/// equal inputs give B's output, else the line through A and B.
fn linear(stops: &[LinearStop], t: f32) -> f32 {
    let n = stops.len();
    if n < 2 {
        return stops.first().map_or(t, |s| s.output);
    }
    let mut a = stops.iter().rposition(|s| s.input <= t).unwrap_or(0);
    if a == n - 1 {
        a -= 1;
    }
    let (a, b) = (stops[a], stops[a + 1]);
    if a.input == b.input {
        return b.output;
    }
    a.output + (b.output - a.output) * (t - a.input) / (b.input - a.input)
}

/// CSS Easing 1 §2.3.1, the step easing algorithm, with its "before
/// flag" (`before`: the input is in the before phase).
fn steps(count: u32, position: StepPosition, t: f32, before: bool) -> f32 {
    let count = count.max(1) as f32;
    let mut current = (t * count).floor();
    if matches!(position, StepPosition::Start | StepPosition::JumpBoth) {
        current += 1.0;
    }
    // The before flag, at a step boundary: the jump has not happened.
    if before && (t * count).fract() == 0.0 {
        current -= 1.0;
    }
    let jumps = match position {
        StepPosition::Start | StepPosition::End => count,
        StepPosition::JumpNone => (count - 1.0).max(1.0),
        StepPosition::JumpBoth => count + 1.0,
    };
    // Clamp as the spec does for inputs in [0, 1].
    (current.max(0.0).min(jumps)) / jumps
}

/// Cubic-bezier with control points `(0,0), (x1,y1), (x2,y2), (1,1)`.
/// `t` is the input progress; the function returns the eased
/// y-coordinate at that x. Newton's method on
/// `f(u) = bezier_x(u) - t` to find `u`, then read `bezier_y(u)`.
fn bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    if t == 0.0 || t == 1.0 {
        return t;
    }
    // Find u ∈ [0, 1] such that bezier_x(u) ≈ t.
    let mut u = t;
    for _ in 0..5 {
        let cx = bezier_axis(x1, x2, u);
        let dx = bezier_axis_derivative(x1, x2, u);
        if dx.abs() < 1e-6 {
            break;
        }
        u -= (cx - t) / dx;
        u = u.clamp(0.0, 1.0);
    }
    bezier_axis(y1, y2, u)
}

#[inline]
fn bezier_axis(p1: f32, p2: f32, u: f32) -> f32 {
    // Standard cubic Bezier: B(u) = 3(1-u)^2 u p1 + 3(1-u) u^2 p2 + u^3
    // (with p0 = 0 and p3 = 1 fixed).
    let one_minus = 1.0 - u;
    3.0 * one_minus * one_minus * u * p1 + 3.0 * one_minus * u * u * p2 + u * u * u
}

#[inline]
fn bezier_axis_derivative(p1: f32, p2: f32, u: f32) -> f32 {
    let one_minus = 1.0 - u;
    3.0 * one_minus * one_minus * p1 + 6.0 * one_minus * u * (p2 - p1) + 3.0 * u * u * (1.0 - p2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_is_identity() {
        assert_eq!(TimingFunction::Linear.ease(0.0), 0.0);
        assert_eq!(TimingFunction::Linear.ease(0.5), 0.5);
        assert_eq!(TimingFunction::Linear.ease(1.0), 1.0);
    }

    #[test]
    fn endpoints_are_pinned_for_named_curves() {
        for f in [
            TimingFunction::Ease,
            TimingFunction::EaseIn,
            TimingFunction::EaseOut,
            TimingFunction::EaseInOut,
        ] {
            assert!((f.ease(0.0) - 0.0).abs() < 0.01);
            assert!((f.ease(1.0) - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn ease_curves_in_expected_direction() {
        // ease-in: progress lags then catches up — at t=0.5,
        // y < 0.5.
        assert!(TimingFunction::EaseIn.ease(0.5) < 0.5);
        // ease-out: fast start, slow end — at t=0.5, y > 0.5.
        assert!(TimingFunction::EaseOut.ease(0.5) > 0.5);
        // ease (CSS default): symmetric-ish, at t=0.5 ≈ 0.802
        // (the canonical reference value).
        let mid = TimingFunction::Ease.ease(0.5);
        assert!((mid - 0.8).abs() < 0.05, "ease at 0.5 = {mid}");
    }
}

#[cfg(test)]
mod name_tests {
    use super::{LinearStop, PropertyName, TransitionProperty};

    /// CSS Transitions 1 §2.1: a `transition-property` entry naming a
    /// property rdom knows is `Named`, built only from a known name
    /// (ASCII case-insensitive, kept canonical); a misspelling is not a
    /// property and is kept as written in `Other`, as CSS keeps it.
    #[test]
    fn a_named_entry_holds_only_a_known_property() {
        assert_eq!(
            PropertyName::new("Color").map(|n| n.as_str()),
            Some("color")
        );
        assert_eq!(PropertyName::new("colour"), None);
        assert_eq!(PropertyName::new("--x"), None);
        let color = PropertyName::new("color").unwrap();
        assert_eq!(
            TransitionProperty::named("COLOR"),
            TransitionProperty::Named(color)
        );
        assert_eq!(
            TransitionProperty::named("colour"),
            TransitionProperty::Other("colour".into())
        );
        assert_eq!(TransitionProperty::Named(color).name(), "color");
    }

    /// CSS Easing 2 §2.1: `linear(<output> <input>%)` — the constructor
    /// takes its arguments in CSS's order.
    #[test]
    fn a_linear_stop_is_built_output_first() {
        let stop = LinearStop::new(0.25, 0.75);
        assert_eq!((stop.output, stop.input), (0.25, 0.75));
    }
}

#[cfg(test)]
mod easing_tests {
    use super::{StepPosition, TimingFunction};

    /// CSS Easing 1 §2.3.1 worked examples for `steps(4, …)` at 0.3, plus
    /// the endpoints, and a parameterized bezier equal to a keyword one.
    #[test]
    fn steps_and_cubic_bezier_ease() {
        let s4 = |position| TimingFunction::Steps { count: 4, position };
        assert_eq!(s4(StepPosition::End).ease(0.3), 0.25);
        assert_eq!(s4(StepPosition::Start).ease(0.3), 0.5);
        assert!((s4(StepPosition::JumpNone).ease(0.3) - 1.0 / 3.0).abs() < 1e-6);
        assert_eq!(s4(StepPosition::JumpBoth).ease(0.3), 0.4);
        assert_eq!(s4(StepPosition::End).ease(0.0), 0.0);
        assert_eq!(s4(StepPosition::End).ease(1.0), 1.0);
        assert_eq!(s4(StepPosition::Start).ease(0.0), 0.25);
        assert_eq!(TimingFunction::STEP_END.ease(0.99), 0.0);
        assert_eq!(TimingFunction::STEP_START.ease(0.01), 1.0);
        let ease_in = TimingFunction::CubicBezier {
            x1: 0.42,
            y1: 0.0,
            x2: 1.0,
            y2: 1.0,
        };
        assert_eq!(ease_in.ease(0.5), TimingFunction::EaseIn.ease(0.5));
    }
}

#[cfg(test)]
mod timing_tests;
