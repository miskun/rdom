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
    pub delay_ms: u32,
}

/// Which property a transition rule covers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TransitionProperty {
    /// `transition-property: all` — every animatable property
    /// transitions on change.
    All,
    /// `transition-property: none` — disables transitions.
    None,
    /// A specific animatable property.
    Named(AnimatableProperty),
    /// Any other `<custom-ident>` — a non-animatable property
    /// (`display`, `position`, …) or an unknown name. Valid CSS
    /// (Transitions 1 §2.1); it never starts a transition because
    /// discrete properties only transition under `transition-behavior:
    /// allow-discrete` (Transitions 2), which rdom does not ship.
    Discrete(String),
}

/// The set of `TuiStyle` properties a transition can animate.
/// Discrete properties (display, position, content, …) aren't here:
/// they never transition (CSS Transitions 1; `transition-behavior:
/// allow-discrete` is Level 2 and not shipped), so `transition: all`
/// covers exactly this set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimatableProperty {
    /// `color` (= TuiStyle.fg)
    Color,
    /// `background-color`
    BackgroundColor,
    /// `border-color`
    BorderColor,
    /// `width`
    Width,
    /// `height`
    Height,
    /// `padding` (uniform — per-side longhands not yet exposed
    /// as a transition target; spec-faithful for M3, can grow)
    Padding,
    /// `gap`
    Gap,
    /// `top`
    Top,
    /// `right`
    Right,
    /// `bottom`
    Bottom,
    /// `left`
    Left,
    /// `z-index`
    ZIndex,
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

/// `<easing-function>` (CSS Easing 1): the keyword curves,
/// `cubic-bezier(x1, y1, x2, y2)` and `steps(n, <position>)`.
/// `PartialEq` only — `CubicBezier` carries `f32`s.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
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
    pub fn ease(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            TimingFunction::Linear => t,
            TimingFunction::Ease => bezier(0.25, 0.1, 0.25, 1.0, t),
            TimingFunction::EaseIn => bezier(0.42, 0.0, 1.0, 1.0, t),
            TimingFunction::EaseOut => bezier(0.0, 0.0, 0.58, 1.0, t),
            TimingFunction::EaseInOut => bezier(0.42, 0.0, 0.58, 1.0, t),
            TimingFunction::CubicBezier { x1, y1, x2, y2 } => bezier(x1, y1, x2, y2, t),
            TimingFunction::Steps { count, position } => steps(count, position, t),
        }
    }
}

/// CSS Easing 1 §2.3.1, the step easing algorithm (without the
/// "before flag", which only matters for animations in their delay
/// phase).
fn steps(count: u32, position: StepPosition, t: f32) -> f32 {
    let count = count.max(1) as f32;
    let mut current = (t * count).floor();
    if matches!(position, StepPosition::Start | StepPosition::JumpBoth) {
        current += 1.0;
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
