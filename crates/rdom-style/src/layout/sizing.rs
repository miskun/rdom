//! Sizing values: `width` / `height` ([`Size`]), `min-*` ([`MinSize`]),
//! `max-*` ([`MaxSize`]),
//! `aspect-ratio`, `gap` and the positioning offsets ([`Length`]),
//! each with its resolution against a basis.

/// Sizing for width or height — CSS-like sizing modes.
///
/// **Not `Copy`** — the `Calc` variant carries a boxed expression
/// tree. The simple variants (`Fixed` / `Flex` / `Percent` /
/// `Auto`) clone in O(1); `Calc` clones the AST. Move boundaries
/// where the previous `Copy` was implicit need `.clone()`.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Size {
    /// Exact number of cells.
    Fixed(u16),
    /// Flexible: takes remaining space proportional to weight.
    /// `Flex(1.0)` = equal share. `Flex(2.0)` = double share. A
    /// fractional weight is a `<number>` as CSS allows (`flex: 0.5`);
    /// weights summing below one share only that fraction of the free
    /// space (CSS Flexbox §9.7).
    Flex(f32),
    /// Percentage of the parent's content-area dimension on the
    /// matching axis (`width: 50%` ⇒ half of parent's content
    /// width). Carries the fraction (`12.5%` is `12.5`); resolves at
    /// layout time once the parent dimension is known, through
    /// [`Size::percent_of`], which rounds onto the cell grid once.
    /// Matches CSS `<percentage>` semantics for sizing properties.
    Percent(f32),
    /// `calc(<expr>)` — arithmetic over lengths + percentages
    /// (`+ - * /`). Resolves at layout time against the parent's
    /// matching-axis content dimension (`width` → parent width,
    /// `height` → parent height). See [`crate::calc::CalcExpr`].
    /// Negative results clamp to 0; positive results clamp to
    /// `u16::MAX`.
    Calc(Box<crate::calc::CalcExpr>),
    /// Child determines its own size (default: content-driven).
    #[default]
    Auto,
}

impl Size {
    /// `p` percent of `basis`, rounded onto the cell grid (ties to
    /// even, the same rule `calc()` uses). The single place a
    /// percentage becomes cells, so `12.5%` of 80 is 10 everywhere.
    pub fn percent_of(basis: i32, p: f32) -> i32 {
        (f64::from(basis) * f64::from(p) / 100.0).round_ties_even() as i32
    }

    /// This size in cells, a percentage or `calc()` resolved against
    /// `basis` (the containing block's extent on this axis); `None` for
    /// `auto` and a flex weight, which the caller sizes. Unclamped: a
    /// `calc()` can be negative.
    pub fn cells(&self, basis: i32) -> Option<i32> {
        match self {
            Size::Fixed(n) => Some(i32::from(*n)),
            Size::Percent(p) => Some(Size::percent_of(basis, *p)),
            Size::Calc(expr) => Some(expr.resolve(&crate::calc::ResolveCtx::new(basis))),
            Size::Flex(_) | Size::Auto => None,
        }
    }

    /// [`cells`](Self::cells) clamped to a box's extent, `0..=u16::MAX`
    /// (a negative size is 0).
    pub fn cells_u16(&self, basis: i32) -> Option<u16> {
        self.cells(basis)
            .map(|v| v.clamp(0, i32::from(u16::MAX)) as u16)
    }

    /// Resolve `Calc` to `Fixed`, leaving other variants unchanged.
    /// Pass the parent's content dimension on the relevant axis as
    /// `basis`. Used by layout sites that prefer to flatten before
    /// matching.
    pub fn resolve_calc(self, basis: i32) -> Size {
        match self {
            Size::Calc(expr) => {
                let v = expr.resolve(&crate::calc::ResolveCtx::new(basis));
                let clamped = v.max(0).min(u16::MAX as i32) as u16;
                Size::Fixed(clamped)
            }
            other => other,
        }
    }
}

/// Value of `min-width` / `min-height`. CSS-faithful: `auto` resolves
/// to intrinsic min-content for flex items (decision 4 from the M5
/// pre-prep), `Cells(n)` is the explicit cell count, `Calc` a
/// percentage or a percent-bearing math function resolved against the
/// containing block on the same axis (CSS Sizing 3 §5.2).
///
/// `From<u16>` returns `Cells(n)` so the fluent setter (`.min_width(10)`)
/// keeps working unchanged.
#[derive(Debug, Clone, PartialEq)]
pub enum MinSize {
    /// `auto` — flex items resolve to their intrinsic min-content
    /// size; non-flex items resolve to 0. The `overflow: hidden →
    /// auto = 0` CSS exception is deferred (`M5-MIN-AUTO-1`).
    Auto,
    /// Explicit cell count.
    Cells(u16),
    /// `<percentage>` or a math function holding one. Resolves at
    /// layout time against the containing block's size on the same
    /// axis.
    Calc(Box<crate::calc::CalcExpr>),
}

impl MinSize {
    /// `min-* : <p>%` — `p` percent of the containing block's extent on
    /// this axis (CSS Sizing 3 §5.2), as the parser stores it.
    pub fn percent(p: f64) -> Self {
        MinSize::Calc(Box::new(crate::calc::CalcExpr::Percent(p)))
    }

    /// The floor in cells, `None` for `auto`. `basis` is the
    /// containing block's size on the property's axis, `None` when it
    /// is indefinite — a percentage against an indefinite basis is
    /// treated as `0` (CSS 2.1 §10.7).
    pub fn cells(&self, basis: Option<u16>) -> Option<u16> {
        match self {
            MinSize::Auto => None,
            MinSize::Cells(n) => Some(*n),
            MinSize::Calc(expr) => Some(match basis {
                Some(b) => resolve_u16(expr, b),
                None if expr.contains_percent() => 0,
                None => resolve_u16(expr, 0),
            }),
        }
    }
}

impl From<u16> for MinSize {
    fn from(n: u16) -> Self {
        MinSize::Cells(n)
    }
}

/// Value of `max-width` / `max-height` (`none` is the absent value,
/// `Option::None` on the style). `Calc` holds a percentage or a
/// percent-bearing math function, resolved against the containing
/// block on the same axis (CSS Sizing 3 §5.2).
#[derive(Debug, Clone, PartialEq)]
pub enum MaxSize {
    /// Explicit cell count.
    Cells(u16),
    /// `<percentage>` or a math function holding one.
    Calc(Box<crate::calc::CalcExpr>),
}

impl MaxSize {
    /// `max-* : <p>%` — `p` percent of the containing block's extent on
    /// this axis (CSS Sizing 3 §5.2), as the parser stores it.
    pub fn percent(p: f64) -> Self {
        MaxSize::Calc(Box::new(crate::calc::CalcExpr::Percent(p)))
    }

    /// The limit in cells. `basis` is the containing block's size on
    /// the property's axis, `None` when indefinite — a percentage
    /// against an indefinite basis is treated as `none` (CSS 2.1
    /// §10.7), so no limit.
    pub fn cells(&self, basis: Option<u16>) -> Option<u16> {
        match self {
            MaxSize::Cells(n) => Some(*n),
            MaxSize::Calc(expr) => match basis {
                Some(b) => Some(resolve_u16(expr, b)),
                None if expr.contains_percent() => None,
                None => Some(resolve_u16(expr, 0)),
            },
        }
    }
}

impl From<u16> for MaxSize {
    fn from(n: u16) -> Self {
        MaxSize::Cells(n)
    }
}

impl Size {
    /// This size with a flex weight kept in `<number [0,∞]>` (CSS
    /// Flexbox §7.1), for a value built in Rust: a negative, zero or
    /// NaN weight is no grow (`Size::Auto`, as `flex: 0` parses), an
    /// infinite one the largest finite weight. Other sizes are as given.
    /// The style setters apply it.
    pub fn validated(self) -> Size {
        match self {
            Size::Flex(w) if w > 0.0 => Size::Flex(w.min(f32::MAX)),
            Size::Flex(_) => Size::Auto,
            other => other,
        }
    }
}

/// A `flex-shrink` factor kept in `<number [0,∞]>` (CSS Flexbox §7.1),
/// for a value built in Rust: negative or NaN is 0, infinite the largest
/// finite factor. The style setters apply it.
pub fn valid_flex_factor(v: f32) -> f32 {
    if v > 0.0 { v.min(f32::MAX) } else { 0.0 }
}

/// Value of `flex-basis` (CSS Flexbox §7.3.3: `content | <'width'>`),
/// set by the `flex` shorthand (§7.2). Stored and cascaded; the layout
/// pass does not read it yet — a growing item's basis is 0 and a
/// non-growing one's is its `width` / `height` (C6-FLEX-LONGHANDS,
/// DIVERGENCES).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum FlexBasis {
    /// `auto`: the item's main size property. The initial value.
    #[default]
    Auto,
    /// `content`: the item's content size.
    Content,
    /// Explicit cell count.
    Cells(u16),
    /// `<percentage>` or a math function holding one, against the flex
    /// container's inner main size.
    Calc(Box<crate::calc::CalcExpr>),
}

/// `expr` against `basis`, clamped to `0..=u16::MAX`.
fn resolve_u16(expr: &crate::calc::CalcExpr, basis: u16) -> u16 {
    let v = expr.resolve(&crate::calc::ResolveCtx::new(i32::from(basis)));
    v.clamp(0, i32::from(u16::MAX)) as u16
}

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

/// `gap` value: whole cells, or a `calc()` / percentage that resolves
/// at layout time against the container's content size on the gap's
/// axis (CSS Box Alignment 3 §8: indefinite → 0). `CALC-GAP-1`.
#[derive(Debug, Clone, PartialEq)]
pub enum GapValue {
    Cells(u16),
    Calc(Box<crate::calc::CalcExpr>),
}

impl Default for GapValue {
    fn default() -> Self {
        GapValue::Cells(0)
    }
}

impl From<u16> for GapValue {
    fn from(cells: u16) -> Self {
        GapValue::Cells(cells)
    }
}

impl From<crate::calc::CalcExpr> for GapValue {
    fn from(expr: crate::calc::CalcExpr) -> Self {
        GapValue::Calc(Box::new(expr))
    }
}

impl GapValue {
    /// Resolve against `basis` (the container's content size on the
    /// gap's axis; `0` when that size is indefinite).
    pub fn resolve(&self, basis: u16) -> u16 {
        match self {
            GapValue::Cells(n) => *n,
            GapValue::Calc(expr) => {
                let v = expr.resolve(&crate::calc::ResolveCtx::new(i32::from(basis)));
                v.clamp(0, i32::from(u16::MAX)) as u16
            }
        }
    }

    /// The value as whole cells when it needs no basis.
    pub fn as_cells(&self) -> Option<u16> {
        match self {
            GapValue::Cells(n) => Some(*n),
            GapValue::Calc(_) => None,
        }
    }
}

/// Offset value for `top` / `right` / `bottom` / `left`.
///
/// **Not `Copy`** — the `Calc` variant carries a boxed expression
/// tree. The simple variants clone in O(1); `Calc` clones the AST.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Length {
    /// `auto`. Resolution depends on context — phase-2 placement.
    #[default]
    Auto,
    /// Integer cells. Signed so negative offsets are valid CSS; `i32`
    /// so a virtualized surface can position past ±32 k cells
    /// (`SUB-4`).
    Cells(i32),
    /// `calc(<expr>)`. Resolves at layout time against the
    /// parent's matching-axis content dimension (`top`/`bottom` →
    /// height, `left`/`right` → width).
    Calc(Box<crate::calc::CalcExpr>),
}

impl Length {
    /// This length in cells, a `calc()` resolved against `basis` (the
    /// containing block's extent on this axis); `None` for `auto`.
    pub fn cells(&self, basis: i32) -> Option<i32> {
        match self {
            Length::Auto => None,
            Length::Cells(n) => Some(*n),
            Length::Calc(expr) => Some(expr.resolve(&crate::calc::ResolveCtx::new(basis))),
        }
    }

    /// Resolve `Calc` to `Cells`, leaving other variants unchanged.
    pub fn resolve_calc(self, basis: i32) -> Length {
        match self {
            Length::Calc(expr) => Length::Cells(expr.resolve(&crate::calc::ResolveCtx::new(basis))),
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::{CalcExpr, CalcOp};

    fn calc(lhs: CalcExpr, rhs: CalcExpr) -> Box<CalcExpr> {
        Box::new(CalcExpr::binary(CalcOp::Sub, lhs, rhs))
    }

    /// `C2G-CELLS-CONVERSIONS`: the one conversion of a size or an
    /// inset to cells — percentages through `Size::percent_of`, `calc()`
    /// against the same basis, `auto` (and a flex weight) left to the
    /// caller.
    #[test]
    fn sizes_and_lengths_to_cells() {
        assert_eq!(Size::Fixed(7).cells(80), Some(7));
        assert_eq!(Size::Percent(12.5).cells(80), Some(10));
        let minus = calc(CalcExpr::Percent(50.0), CalcExpr::Number(50.0));
        assert_eq!(Size::Calc(minus.clone()).cells(80), Some(-10));
        assert_eq!(Size::Calc(minus.clone()).cells_u16(80), Some(0));
        assert_eq!(Size::Percent(200.0).cells_u16(40_000), Some(u16::MAX));
        assert_eq!(Size::Auto.cells(80), None);
        assert_eq!(Size::Flex(1.0).cells(80), None);
        assert_eq!(Length::Cells(-3).cells(80), Some(-3));
        assert_eq!(Length::Calc(minus).cells(80), Some(-10));
        assert_eq!(Length::Auto.cells(80), None);
    }
}
