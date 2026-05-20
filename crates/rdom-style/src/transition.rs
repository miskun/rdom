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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TransitionRule {
    pub property: TransitionProperty,
    pub duration_ms: u32,
    pub timing: TimingFunction,
    pub delay_ms: u32,
}

/// Which property a transition rule covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransitionProperty {
    /// `transition-property: all` — every animatable property
    /// transitions on change.
    All,
    /// `transition-property: none` — disables transitions.
    None,
    /// A specific animatable property.
    Named(AnimatableProperty),
}

/// The set of `TuiStyle` properties an explicit transition rule
/// can target by name. Discrete properties (display, position,
/// content, …) aren't here — they're not directly animatable
/// individually, but they DO toggle at midpoint when covered by
/// `transition: all`.
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

/// Named timing-function keywords from CSS Transitions L1. M3
/// ships only the keyword set; `cubic-bezier(a,b,c,d)` and
/// `steps(n, position)` are deferred per spec §10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
}

impl TimingFunction {
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
        }
    }
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
