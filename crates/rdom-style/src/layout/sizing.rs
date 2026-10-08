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
    /// `min-content` / `max-content` / `fit-content` /
    /// `fit-content(<length-percentage>)` (CSS Sizing 3 §3.1): a size
    /// from the box's content, which layout measures.
    Intrinsic(IntrinsicSize),
    /// Child determines its own size (default: content-driven).
    #[default]
    Auto,
}

/// An intrinsic size keyword (CSS Sizing 3 §3.1–§3.3), valid in `width`
/// / `height`, `min-*` and `max-*`. On the inline axis it is the box's
/// min-content or max-content size (§5.1), or `fit-content`'s clamp of
/// the available space between them; on the block axis every keyword
/// is the box's content height ("equivalent to its automatic size").
#[derive(Debug, Clone, PartialEq)]
pub enum IntrinsicSize {
    /// `min-content`: the min-content size — the longest unbreakable
    /// run of content.
    MinContent,
    /// `max-content`: the max-content size — the content unwrapped.
    MaxContent,
    /// `fit-content`: `min(max-content, max(min-content, stretch-fit))`,
    /// the available space clamped between the two.
    FitContent,
    /// `fit-content(<length-percentage>)`: `min(max-content,
    /// max(min-content, limit))`. The limit is cells
    /// ([`CalcExpr::Length`](crate::calc::CalcExpr::Length)), a
    /// percentage of the containing block, or a math function.
    FitContentLimit(Box<crate::calc::CalcExpr>),
}

impl IntrinsicSize {
    /// `fit-content(<cells>)`: a limit of `cells` columns (rows on the
    /// block axis).
    pub fn fit_content(cells: u16) -> Self {
        IntrinsicSize::FitContentLimit(Box::new(crate::calc::CalcExpr::Length(i32::from(cells))))
    }

    /// `fit-content(<percent>%)`: a limit of `percent` of the containing
    /// block's size on the axis.
    pub fn fit_content_percent(percent: f32) -> Self {
        IntrinsicSize::FitContentLimit(Box::new(crate::calc::CalcExpr::Percent(f64::from(percent))))
    }

    /// `fit-content()`'s limit in cells against `basis` (the containing
    /// block's size on the axis, `None` when indefinite — a percentage
    /// then has no limit, CSS Sizing 3 §3.1 treats it as `max-content`).
    /// `None` for the other keywords.
    pub fn limit_cells(&self, basis: Option<u16>) -> Option<u16> {
        let IntrinsicSize::FitContentLimit(expr) = self else {
            return None;
        };
        match basis {
            Some(b) => Some(resolve_u16(expr, b)),
            None if expr.contains_percent() => None,
            None => Some(resolve_u16(expr, 0)),
        }
    }
}

/// `p` percent of `basis` as an extent: [`Size::percent_of`] clamped to
/// `0..=u16::MAX`.
pub(super) fn percent_cells(basis: u16, p: f32) -> u16 {
    Size::percent_of(i32::from(basis), p).clamp(0, i32::from(u16::MAX)) as u16
}

impl Size {
    /// `p` percent of `basis`, rounded onto the cell grid (ties to
    /// even, the same rule `calc()` uses). The single place a
    /// percentage becomes cells, so `12.5%` of 80 is 10 everywhere.
    pub fn percent_of(basis: i32, p: f32) -> i32 {
        (f64::from(basis) * f64::from(p) / 100.0).round_ties_even() as i32
    }

    /// `width` / `height: <p>%` — `p` percent of the containing block's
    /// extent on this axis.
    pub fn percent(p: f32) -> Self {
        Size::Percent(p)
    }

    /// This size in cells: a percentage or `calc()` resolved against
    /// `basis`, the containing block's extent on this axis — `None` when
    /// it is indefinite, so a percentage (or a `calc()` holding one) is
    /// `auto` (CSS 2.1 §10.5). `None` for `auto` and a flex weight, which
    /// the caller sizes. Clamped to an extent, `0..=u16::MAX` (a negative
    /// `calc()` is 0, CSS Values 4 §10.12).
    pub fn cells(&self, basis: Option<u16>) -> Option<u16> {
        match self {
            Size::Fixed(n) => Some(*n),
            Size::Percent(p) => basis.map(|b| percent_cells(b, *p)),
            Size::Calc(expr) => match basis {
                Some(b) => Some(resolve_u16(expr, b)),
                None if expr.contains_percent() => None,
                None => Some(resolve_u16(expr, 0)),
            },
            Size::Flex(_) | Size::Auto | Size::Intrinsic(_) => None,
        }
    }

    /// The intrinsic keyword this size is, if any.
    pub fn intrinsic(&self) -> Option<&IntrinsicSize> {
        match self {
            Size::Intrinsic(k) => Some(k),
            _ => None,
        }
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
/// pre-prep), `Cells(n)` is the explicit cell count, `Percent` a
/// percentage and `Calc` a math function, both resolved against the
/// containing block on the same axis (CSS Sizing 3 §5.2). The default is
/// `auto`, the initial value.
///
/// `From<u16>` returns `Cells(n)` so the fluent setter (`.min_width(10)`)
/// keeps working unchanged.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum MinSize {
    /// `auto` — flex items resolve to their intrinsic min-content
    /// size; non-flex items resolve to 0. The `overflow: hidden →
    /// auto = 0` CSS exception is deferred (`M5-MIN-AUTO-1`). The
    /// initial value (CSS Sizing 3 §5.2).
    #[default]
    Auto,
    /// Explicit cell count.
    Cells(u16),
    /// `<percentage>` of the containing block's size on the same axis,
    /// as [`Size::Percent`] holds it (`12.5%` is `12.5`).
    Percent(f32),
    /// A math function. Resolves at layout time against the containing
    /// block's size on the same axis.
    Calc(Box<crate::calc::CalcExpr>),
    /// An intrinsic keyword (CSS Sizing 3 §3.2), which layout measures.
    Intrinsic(IntrinsicSize),
}

impl From<u16> for Size {
    fn from(n: u16) -> Self {
        Size::Fixed(n)
    }
}

impl From<IntrinsicSize> for Size {
    fn from(k: IntrinsicSize) -> Self {
        Size::Intrinsic(k)
    }
}

impl MinSize {
    /// The intrinsic keyword this bound is, if any.
    pub fn intrinsic(&self) -> Option<&IntrinsicSize> {
        match self {
            MinSize::Intrinsic(k) => Some(k),
            _ => None,
        }
    }

    /// `min-* : <p>%` — `p` percent of the containing block's extent on
    /// this axis (CSS Sizing 3 §5.2), as the parser stores it: the same
    /// shape as [`Size::percent`] and [`MaxSize::percent`].
    pub fn percent(p: f32) -> Self {
        MinSize::Percent(p)
    }

    /// The floor in cells, `None` for `auto` and an intrinsic keyword
    /// (layout measures it). `basis` is the
    /// containing block's size on the property's axis, `None` when it
    /// is indefinite — a percentage against an indefinite basis is
    /// treated as `0` (CSS 2.1 §10.7).
    pub fn cells(&self, basis: Option<u16>) -> Option<u16> {
        match self {
            MinSize::Auto | MinSize::Intrinsic(_) => None,
            MinSize::Cells(n) => Some(*n),
            MinSize::Percent(p) => Some(basis.map_or(0, |b| percent_cells(b, *p))),
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

impl From<IntrinsicSize> for MinSize {
    fn from(k: IntrinsicSize) -> Self {
        MinSize::Intrinsic(k)
    }
}

/// Value of `max-width` / `max-height`: `none | <length-percentage>`
/// (CSS Sizing 3 §5.2). `Percent` and `Calc` (a math function) resolve
/// against the containing block on the same axis.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum MaxSize {
    /// `none` — no limit. The initial value.
    #[default]
    None,
    /// Explicit cell count.
    Cells(u16),
    /// `<percentage>` of the containing block's size on the same axis,
    /// as [`Size::Percent`] holds it.
    Percent(f32),
    /// A math function.
    Calc(Box<crate::calc::CalcExpr>),
    /// An intrinsic keyword (CSS Sizing 3 §3.3), which layout measures.
    Intrinsic(IntrinsicSize),
}

impl MaxSize {
    /// The intrinsic keyword this bound is, if any.
    pub fn intrinsic(&self) -> Option<&IntrinsicSize> {
        match self {
            MaxSize::Intrinsic(k) => Some(k),
            _ => None,
        }
    }

    /// `max-* : <p>%` — `p` percent of the containing block's extent on
    /// this axis (CSS Sizing 3 §5.2), as the parser stores it: the same
    /// shape as [`Size::percent`] and [`MinSize::percent`].
    pub fn percent(p: f32) -> Self {
        MaxSize::Percent(p)
    }

    /// The limit in cells, `None` for `none` and an intrinsic keyword
    /// (layout measures it). `basis` is the containing
    /// block's size on the property's axis, `None` when indefinite — a
    /// percentage against an indefinite basis is treated as `none` (CSS
    /// 2.1 §10.7), so no limit.
    pub fn cells(&self, basis: Option<u16>) -> Option<u16> {
        match self {
            MaxSize::None | MaxSize::Intrinsic(_) => None,
            MaxSize::Cells(n) => Some(*n),
            MaxSize::Percent(p) => basis.map(|b| percent_cells(b, *p)),
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

impl From<IntrinsicSize> for MaxSize {
    fn from(k: IntrinsicSize) -> Self {
        MaxSize::Intrinsic(k)
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
/// set by the `flex-basis` longhand and the `flex` shorthand (§7.2):
/// the flex base size of the item (§9.2 step 3).
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
    /// An intrinsic size keyword (CSS Sizing 3 §3.1): `min-content`,
    /// `max-content`, `fit-content`, `fit-content(<l>)`.
    Intrinsic(IntrinsicSize),
}

/// One axis of `contain-intrinsic-size` (CSS Sizing 4 §6.1): `auto?
/// [ none | <length [0,∞]> ]` — the size a box under size containment
/// takes as its content's, and with `auto` the last size it was laid out
/// at, once it has one. Initial `none`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ContainIntrinsicSize {
    /// `auto`: remember the box's last laid-out size.
    pub auto: bool,
    /// The `<length>`, `None` for `none`. Cells are
    /// [`CalcExpr::Length`](crate::calc::CalcExpr::Length); a math
    /// function or a viewport-percentage length stays an expression
    /// (no percentages: the grammar has none).
    pub length: Option<crate::calc::CalcExpr>,
}

impl ContainIntrinsicSize {
    /// The length in cells, `None` for `none`.
    pub fn cells(&self) -> Option<u16> {
        self.length.as_ref().map(|e| resolve_u16(e, 0))
    }
}

/// `expr` against `basis`, clamped to `0..=u16::MAX`.
pub(super) fn resolve_u16(expr: &crate::calc::CalcExpr, basis: u16) -> u16 {
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

/// A `row-gap` / `column-gap` value (CSS Box Alignment 3 §8.1), also
/// `border-spacing`'s: whole cells, or a `calc()` / percentage that
/// resolves at layout time against the container's content size on the
/// gap's axis (indefinite → 0; `CALC-GAP-1`), or `normal` — the initial
/// value, 0 in flex (and grid) layout.
#[derive(Debug, Clone, PartialEq)]
pub enum GapValue {
    Cells(u16),
    Calc(Box<crate::calc::CalcExpr>),
    /// `normal`: 0 in flex layout (§8.1; a multi-column `1em`, which
    /// rdom has no layout for yet).
    Normal,
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
            GapValue::Normal => 0,
        }
    }

    /// The value as whole cells when it needs no basis.
    pub fn as_cells(&self) -> Option<u16> {
        match self {
            GapValue::Cells(n) => Some(*n),
            GapValue::Calc(_) => None,
            GapValue::Normal => Some(0),
        }
    }
}

/// Offset value for `top` / `right` / `bottom` / `left`, and
/// `text-indent`'s length.
///
/// **Not `Copy`** — the `Calc` variant carries an expression tree,
/// shared behind an `Arc`: every variant clones in O(1) without
/// allocating, so an inherited `calc()` `text-indent` costs its
/// descendants a reference count, not a copy (C9-CARRY-INDENT).
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
    Calc(std::sync::Arc<crate::calc::CalcExpr>),
}

impl Length {
    /// `calc(<expr>)`: [`Length::Calc`] of `expr`.
    pub fn calc(expr: crate::calc::CalcExpr) -> Self {
        Length::Calc(std::sync::Arc::new(expr))
    }

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
