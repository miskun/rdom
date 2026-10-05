//! Box-model values: per-side [`Padding`] / [`Margin`] with their
//! percent / `calc()` resolution against the containing block. Border
//! values live in `border`.

/// Padding value on a single side. CSS allows numeric cells and
/// percent (resolved against the containing-block width even for
/// top/bottom padding per CSS 2.1 §8.4). rdom adds `Calc` for
/// `calc()` expressions that may mix cells and percent.
///
/// Closes `CALC-PADMARG-1`: pre-2026-05-26 the parser rejected
/// percent-bearing calc at parse time because padding fields were
/// plain `u16`. Now the type carries the unresolved expression and
/// layout-pass readers call [`resolve`](Self::resolve) with the
/// containing-block width.
#[derive(Debug, Clone, PartialEq)]
pub enum PaddingValue {
    /// Concrete cell count.
    Cells(u16),
    /// `calc(...)` expression. Resolves at layout time against the
    /// containing-block width (CSS resolves both axes' padding
    /// percent against width).
    Calc(Box<crate::calc::CalcExpr>),
}

impl Default for PaddingValue {
    fn default() -> Self {
        PaddingValue::Cells(0)
    }
}

impl PaddingValue {
    /// Resolve to a concrete cell count. `cb_width` is the
    /// containing-block width (the basis for `%` units per CSS
    /// 2.1 §8.4 — vertical padding percent ALSO resolves against
    /// width, not height).
    pub fn resolve(&self, cb_width: u16) -> u16 {
        match self {
            PaddingValue::Cells(n) => *n,
            PaddingValue::Calc(expr) => {
                let v = expr.resolve(&crate::calc::ResolveCtx::new(cb_width as i32));
                v.max(0).min(u16::MAX as i32) as u16
            }
        }
    }

    /// True iff this is provably `Cells(0)`. `Calc` returns false
    /// (conservative — the resolved value depends on the
    /// containing-block width). Used by layout-pass predicates
    /// like "does this element have any padding?" where the
    /// conservative answer for Calc is "treat as non-zero."
    pub fn is_zero(&self) -> bool {
        matches!(self, PaddingValue::Cells(0))
    }
}

/// Padding (CSS order: top, right, bottom, left).
///
/// Each side is a [`PaddingValue`] so `padding-top: calc(50% + 1)`
/// round-trips through the parser. Layout-pass readers call
/// `padding.top.resolve(cb_width)` (etc.) to convert to a u16 cell
/// count.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Padding {
    pub top: PaddingValue,
    pub right: PaddingValue,
    pub bottom: PaddingValue,
    pub left: PaddingValue,
}

/// Margin value on a single side. CSS allows numeric (positive or
/// negative), the `auto` keyword, and `calc()` (rdom adds the last
/// to close `CALC-PADMARG-1`). `Auto` participates in flex
/// main-axis space absorption and absolute-element centering.
#[derive(Debug, Clone, PartialEq)]
pub enum MarginValue {
    /// `auto`. Participates in flex space distribution and absolute
    /// centering.
    Auto,
    /// Integer cells. Signed so negative margins are valid CSS.
    Cells(i16),
    /// `calc(...)`. Resolves at layout time against the
    /// containing-block width (CSS resolves percent margins against
    /// width on all four sides). Result clamped to i16.
    Calc(Box<crate::calc::CalcExpr>),
}

impl MarginValue {
    /// Resolve to a concrete cell count. `cb_width` is the
    /// containing-block width (CSS 2.1 §8.3 — percent margins
    /// resolve against width on both axes). `Auto` resolves to 0
    /// — auto-absorption is the caller's responsibility (flex
    /// distribution computes its own auto handling).
    pub fn resolve(&self, cb_width: u16) -> i16 {
        match self {
            MarginValue::Auto => 0,
            MarginValue::Cells(n) => *n,
            MarginValue::Calc(expr) => {
                let v = expr.resolve(&crate::calc::ResolveCtx::new(cb_width as i32));
                v.clamp(i16::MIN as i32, i16::MAX as i32) as i16
            }
        }
    }

    /// True iff this is `Auto`.
    pub fn is_auto(&self) -> bool {
        matches!(self, MarginValue::Auto)
    }
}

impl Default for MarginValue {
    /// CSS initial value of `margin-*` is `0` (not `auto`).
    fn default() -> Self {
        MarginValue::Cells(0)
    }
}

/// Margin (CSS order: top, right, bottom, left). Each side is a
/// [`MarginValue`] so per-side `auto` round-trips through the parser.
/// **Note:** rdom diverges from CSS by NOT collapsing adjacent
/// vertical margins between block-level boxes (CSS 2.1 §8.3.1).
/// Tracked as `M5-MARGIN-1` in `TECH_DEBT.md`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Margin {
    pub top: MarginValue,
    pub right: MarginValue,
    pub bottom: MarginValue,
    pub left: MarginValue,
}

impl Margin {
    pub fn new(
        top: MarginValue,
        right: MarginValue,
        bottom: MarginValue,
        left: MarginValue,
    ) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Convenience: same numeric cells on all four sides.
    pub fn all_cells(n: i16) -> Self {
        Self {
            top: MarginValue::Cells(n),
            right: MarginValue::Cells(n),
            bottom: MarginValue::Cells(n),
            left: MarginValue::Cells(n),
        }
    }

    /// Convenience: `margin: auto` on all four sides. Useful for
    /// modal centering when combined with `position: absolute; top:
    /// 0; left: 0; right: 0; bottom: 0`.
    pub fn all_auto() -> Self {
        Self {
            top: MarginValue::Auto,
            right: MarginValue::Auto,
            bottom: MarginValue::Auto,
            left: MarginValue::Auto,
        }
    }
}

/// The four sides as [`Sides`](super::Sides) — the shape of the
/// `margin-*` longhands, one per side.
impl From<Margin> for super::Sides<MarginValue> {
    fn from(m: Margin) -> Self {
        super::Sides::new(m.top, m.right, m.bottom, m.left)
    }
}

impl From<super::Sides<MarginValue>> for Margin {
    fn from(s: super::Sides<MarginValue>) -> Self {
        Self::new(s.top, s.right, s.bottom, s.left)
    }
}

/// The four sides as [`Sides`](super::Sides) — the shape of the
/// `padding-*` longhands, one per side.
impl From<Padding> for super::Sides<PaddingValue> {
    fn from(p: Padding) -> Self {
        super::Sides::new(p.top, p.right, p.bottom, p.left)
    }
}

impl From<super::Sides<PaddingValue>> for Padding {
    fn from(s: super::Sides<PaddingValue>) -> Self {
        Self {
            top: s.top,
            right: s.right,
            bottom: s.bottom,
            left: s.left,
        }
    }
}

/// `.margin(2)` shortcut — applies `n` cells to all four sides.
/// Mirrors the ergonomic that `MinSize::From<u16>` provides for
/// `.min_width(10)`.
impl From<i16> for Margin {
    fn from(n: i16) -> Self {
        Self::all_cells(n)
    }
}

/// `.margin_top(2)` shortcut — `n` cells on one side.
impl From<i16> for MarginValue {
    fn from(n: i16) -> Self {
        MarginValue::Cells(n)
    }
}

/// `.padding(2)` shortcut — `n` cells on all four sides, as
/// `From<i16> for Margin` is for `.margin(2)`.
impl From<u16> for Padding {
    fn from(n: u16) -> Self {
        Self::all(n)
    }
}

/// `.padding_top(2)` shortcut — `n` cells on one side.
impl From<u16> for PaddingValue {
    fn from(n: u16) -> Self {
        PaddingValue::Cells(n)
    }
}

impl Padding {
    pub fn new(top: u16, right: u16, bottom: u16, left: u16) -> Self {
        Self {
            top: PaddingValue::Cells(top),
            right: PaddingValue::Cells(right),
            bottom: PaddingValue::Cells(bottom),
            left: PaddingValue::Cells(left),
        }
    }

    /// Same horizontal (left/right) and vertical (top/bottom).
    pub fn symmetric(h: u16, v: u16) -> Self {
        Self::new(v, h, v, h)
    }

    /// Left + right in cells, percentages against `cb_width` (CSS Box 3
    /// §4.2), saturating at `u16::MAX`: two `u16` sides can sum past it.
    pub fn horizontal(&self, cb_width: u16) -> u16 {
        self.left
            .resolve(cb_width)
            .saturating_add(self.right.resolve(cb_width))
    }

    /// Top + bottom in cells, percentages against `cb_width` (vertical
    /// padding percentages use the width too, CSS Box 3 §4.2), saturating
    /// at `u16::MAX`.
    pub fn vertical(&self, cb_width: u16) -> u16 {
        self.top
            .resolve(cb_width)
            .saturating_add(self.bottom.resolve(cb_width))
    }

    /// Same on all sides.
    pub fn all(n: u16) -> Self {
        Self::new(n, n, n, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Padding ──────────────────────────────────────────────────────

    #[test]
    fn padding_all_uniform() {
        let p = Padding::all(3);
        assert_eq!(p, Padding::new(3, 3, 3, 3));
    }

    #[test]
    fn padding_symmetric_hv() {
        let p = Padding::symmetric(4, 2);
        assert_eq!(p.left, PaddingValue::Cells(4));
        assert_eq!(p.right, PaddingValue::Cells(4));
        assert_eq!(p.top, PaddingValue::Cells(2));
        assert_eq!(p.bottom, PaddingValue::Cells(2));
    }
}

/// `margin-trim` (CSS Box 4 §3): which of a container's content edges
/// truncate to zero the margins of the children adjoining them, by
/// logical side. Not inherited; the initial value `none` trims nothing.
///
/// A block container trims on the block axis only; a flex container
/// trims its first / last item's main-axis margins and every item's
/// cross-axis margins (§3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MarginTrim {
    /// `block-start` (in `block`).
    pub block_start: bool,
    /// `inline-start` (in `inline`).
    pub inline_start: bool,
    /// `block-end` (in `block`).
    pub block_end: bool,
    /// `inline-end` (in `inline`).
    pub inline_end: bool,
}

impl MarginTrim {
    /// `margin-trim: none`.
    pub const NONE: MarginTrim = MarginTrim {
        block_start: false,
        inline_start: false,
        block_end: false,
        inline_end: false,
    };

    /// `block`: both block-axis edges.
    pub const BLOCK: MarginTrim = MarginTrim {
        block_start: true,
        inline_start: false,
        block_end: true,
        inline_end: false,
    };

    /// `inline`: both inline-axis edges.
    pub const INLINE: MarginTrim = MarginTrim {
        block_start: false,
        inline_start: true,
        block_end: false,
        inline_end: true,
    };

    /// Whether any edge trims.
    pub fn any(self) -> bool {
        self != Self::NONE
    }
}
