//! Sizing values: `width` / `height` ([`Size`]), `min-*` ([`MinSize`]),
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
    /// `Flex(1)` = equal share. `Flex(2)` = double share.
    Flex(u16),
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
        crate::calc::round_half_to_even(f64::from(basis) * f64::from(p) / 100.0)
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
/// pre-prep), `Cells(n)` is the explicit cell count.
///
/// `From<u16>` returns `Cells(n)` so the fluent setter (`.min_width(10)`)
/// keeps working unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinSize {
    /// `auto` — flex items resolve to their intrinsic min-content
    /// size; non-flex items resolve to 0. The `overflow: hidden →
    /// auto = 0` CSS exception is deferred (`M5-MIN-AUTO-1`).
    Auto,
    /// Explicit cell count.
    Cells(u16),
}

impl From<u16> for MinSize {
    fn from(n: u16) -> Self {
        MinSize::Cells(n)
    }
}

/// `aspect-ratio: <w> / <h>` — preserved as the original integer
/// numerator/denominator pair so the CSS round-trip (`set → serialize
/// → set`) recovers the same value. Use [`AspectRatio::as_f32`] when
/// you need the ratio as a float (e.g. for size resolution in the
/// flex layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AspectRatio {
    pub numerator: u16,
    pub denominator: u16,
}

impl AspectRatio {
    /// Construct from numerator/denominator. Both must be positive.
    /// Returns `None` if either is zero.
    pub fn new(numerator: u16, denominator: u16) -> Option<Self> {
        if numerator == 0 || denominator == 0 {
            None
        } else {
            Some(Self {
                numerator,
                denominator,
            })
        }
    }

    /// The ratio as a single `f32` — `numerator / denominator`.
    pub fn as_f32(self) -> f32 {
        (self.numerator as f32) / (self.denominator as f32)
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
    /// Resolve `Calc` to `Cells`, leaving other variants unchanged.
    pub fn resolve_calc(self, basis: i32) -> Length {
        match self {
            Length::Calc(expr) => Length::Cells(expr.resolve(&crate::calc::ResolveCtx::new(basis))),
            other => other,
        }
    }
}
