//! `aspect-ratio` (CSS Sizing 4 §5.1): the preferred ratio of a box's
//! width to its height.

/// An `aspect-ratio` ratio (CSS Sizing 4 §5.1): `<ratio>` (CSS Values 4
/// §5.7, two non-negative numbers, the second 1 when omitted) and
/// whether `auto` came with it. The `auto` keyword alone is no ratio —
/// `None` where the style holds an `Option<AspectRatio>`.
///
/// A ratio with a zero term is *degenerate* and behaves as `auto`
/// ([`AspectRatio::value`] is `None`).
///
/// The terms are private: [`AspectRatio::new`] is the only way to build
/// one, so a non-finite or negative term never reaches layout.
///
/// ```compile_fail
/// let r = rdom_style::layout::AspectRatio { numerator: f32::NAN, denominator: 1.0, auto: false };
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AspectRatio {
    numerator: f32,
    denominator: f32,
    auto: bool,
}

impl AspectRatio {
    /// `numerator / denominator`, without `auto`. `None` when a term is
    /// negative or not finite (CSS Values 4 §5.7: `<number [0,∞]>`).
    pub fn new(numerator: f32, denominator: f32) -> Option<Self> {
        let valid = |v: f32| v.is_finite() && v >= 0.0;
        (valid(numerator) && valid(denominator)).then_some(Self {
            numerator,
            denominator,
            auto: false,
        })
    }

    /// This ratio with `auto` (the `auto && <ratio>` form) or without.
    pub fn with_auto(mut self, auto: bool) -> Self {
        self.auto = auto;
        self
    }

    /// The width term.
    pub fn numerator(self) -> f32 {
        self.numerator
    }

    /// The height term.
    pub fn denominator(self) -> f32 {
        self.denominator
    }

    /// `auto && <ratio>`: a replaced element's natural ratio would win
    /// (rdom has none), and the ratio sizes the content box rather than
    /// the border box.
    pub fn auto(self) -> bool {
        self.auto
    }

    /// The ratio `numerator / denominator`, `None` when degenerate (a
    /// zero term), which behaves as `auto` (CSS Sizing 4 §5.1).
    pub fn value(self) -> Option<f32> {
        (self.numerator > 0.0 && self.denominator > 0.0).then(|| self.numerator / self.denominator)
    }

    /// The ratio as a single `f32` — `numerator / denominator` (infinite
    /// or NaN when degenerate; see [`Self::value`]).
    pub fn as_f32(self) -> f32 {
        self.numerator / self.denominator
    }
}
