//! Box-model values: border styles and the per-side [`Border`],
//! `border-collapse`, and per-side [`Padding`] / [`Margin`] with their
//! percent / `calc()` resolution against the containing block.

/// Per-side border style — the full CSS `border-style` keyword set.
///
/// Per CSS 2.1 §8.5.3, `none` and `hidden` both produce a 0-width
/// border (the substrate honors this: `cells()` returns 0 for both).
/// The difference between them is only meaningful under
/// `border-collapse: collapse`: `hidden` is CSS Tables 3 §11.5's
/// kill-switch — wherever it appears in a border conflict, that
/// edge is suppressed entirely, regardless of any other contributor.
///
/// Style ranking on style tie (CSS 2.1 §17.6.2.1): `double > solid >
/// dashed > dotted > ridge > outset > groove > inset`. Higher rank
/// wins under collapse when widths and elements tie.
///
/// **Terminal-faithful degradation:** the substrate paints `None`,
/// `Hidden`, `Solid`, and `Double` with distinct glyphs (`│─┌┐└┘` /
/// `║═╔╗╚╝`). `Dashed`, `Dotted`, `Ridge`, `Outset`, `Groove`,
/// `Inset` parse and *rank* correctly in conflict resolution — the
/// data model is faithful — but render as `Solid` because rdom has
/// no distinct glyph set for them yet. Matches CSS's "render as
/// best you can on this medium" principle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum BorderStyle {
    /// No border. Zero width, no paint, no contribution in conflict
    /// resolution. CSS default.
    #[default]
    None,
    /// Invisible border. Zero width, no paint — same as `None` for
    /// layout. **Wins absolutely** in collapse-mode conflict
    /// resolution (CSS Tables 3 §11.5 rule 1: the kill-switch).
    Hidden,
    /// Single-line border. `│─┌┐└┘` (or `╭╮╰╯` with rounded corners).
    Solid,
    /// Double-line border. `║═╔╗╚╝`.
    Double,
    /// Dashed. Parses + ranks per CSS; renders as `Solid`.
    Dashed,
    /// Dotted. Parses + ranks per CSS; renders as `Solid`.
    Dotted,
    /// 3D ridge. Parses + ranks per CSS; renders as `Solid`.
    Ridge,
    /// 3D outset. Parses + ranks per CSS; renders as `Solid`.
    Outset,
    /// 3D groove. Parses + ranks per CSS; renders as `Solid`.
    Groove,
    /// 3D inset. Parses + ranks per CSS; renders as `Solid`.
    Inset,
    /// rdom-specific terminal-only style. Each border cell paints
    /// a half-block (`▄ ▀ ▌ ▐`) or quadrant (`▗ ▖ ▝ ▘`) glyph
    /// whose filled half-or-quarter points INWARD toward the
    /// element's content. The visible color spans roughly half a
    /// cell vertically (top/bottom edges) or horizontally
    /// (left/right edges), so a half-block-bordered element reads
    /// as a "pill" rather than a hard-edged rectangle. Pairs
    /// naturally with a `background-color`-filled interior to
    /// build a primary-CTA button style. Not a CSS-spec style;
    /// see DIVERGENCES.md.
    HalfBlock,
}

impl BorderStyle {
    /// Cells reserved for this border in layout. Per CSS 2.1 §8.5.3,
    /// `None` and `Hidden` produce 0 width; every other style → 1
    /// cell in rdom's terminal model.
    pub const fn cells(self) -> u16 {
        match self {
            BorderStyle::None | BorderStyle::Hidden => 0,
            _ => 1,
        }
    }

    /// True iff this style paints a glyph. Equivalent to `cells() == 1`
    /// in rdom (a border that reserves a cell also paints in it).
    pub const fn is_visible(self) -> bool {
        !matches!(self, BorderStyle::None | BorderStyle::Hidden)
    }

    /// True iff this is the `Hidden` kill-switch — used by collapse
    /// conflict resolution to suppress an edge regardless of other
    /// contributors.
    pub const fn is_hidden(self) -> bool {
        matches!(self, BorderStyle::Hidden)
    }

    /// True iff this is `None` (no border).
    pub const fn is_none(self) -> bool {
        matches!(self, BorderStyle::None)
    }

    /// Style-ranking score for CSS Tables 3 §11.5 conflict resolution
    /// (rule 4: "narrower borders are discarded in favor of wider
    /// ones; styles tie-break in this order").
    /// Higher number wins. `None` and `Hidden` get the lowest score
    /// because they're handled at higher-priority rules (1 + 2);
    /// `rank()` is only consulted when both participants are visible.
    pub const fn rank(self) -> u8 {
        match self {
            BorderStyle::Double => 7,
            BorderStyle::Solid => 6,
            BorderStyle::HalfBlock => 6,
            BorderStyle::Dashed => 5,
            BorderStyle::Dotted => 4,
            BorderStyle::Ridge => 3,
            BorderStyle::Outset => 2,
            BorderStyle::Groove => 1,
            BorderStyle::Inset => 0,
            BorderStyle::None | BorderStyle::Hidden => 0,
        }
    }
}

/// Per-side border state. CSS lets authors enable any combination
/// of `border-top` / `border-right` / `border-bottom` / `border-left`
/// independently — each side carries its own [`BorderStyle`].
/// `corner_style` only matters when all 4 sides paint — the
/// rounded-corner glyphs `╭╮╰╯` need both sides at a corner to share
/// a cell.
///
/// The `border` shorthand and the per-side longhands all write into
/// this struct via the cascade. `Border::default()` is "no border"
/// (all sides `BorderStyle::None`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Border {
    pub top: BorderStyle,
    pub right: BorderStyle,
    pub bottom: BorderStyle,
    pub left: BorderStyle,
    pub corner_style: CornerStyle,
}

impl Border {
    /// All sides off (`BorderStyle::None`). Same as `Default`.
    pub const fn none() -> Self {
        Self {
            top: BorderStyle::None,
            right: BorderStyle::None,
            bottom: BorderStyle::None,
            left: BorderStyle::None,
            corner_style: CornerStyle::Square,
        }
    }
    /// All four sides solid, square corners. `border: solid`.
    pub const fn single() -> Self {
        Self::ring(BorderStyle::Solid)
    }
    /// All four sides solid, rounded corners. `border: rounded`.
    pub const fn rounded() -> Self {
        Self {
            top: BorderStyle::Solid,
            right: BorderStyle::Solid,
            bottom: BorderStyle::Solid,
            left: BorderStyle::Solid,
            corner_style: CornerStyle::Rounded,
        }
    }
    /// All four sides set to the same style, square corners.
    pub const fn ring(style: BorderStyle) -> Self {
        Self {
            top: style,
            right: style,
            bottom: style,
            left: style,
            corner_style: CornerStyle::Square,
        }
    }
    /// Top side only (solid). `border-top: solid` longhand without others.
    pub const fn top() -> Self {
        Self {
            top: BorderStyle::Solid,
            right: BorderStyle::None,
            bottom: BorderStyle::None,
            left: BorderStyle::None,
            corner_style: CornerStyle::Square,
        }
    }
    pub const fn bottom() -> Self {
        Self {
            top: BorderStyle::None,
            right: BorderStyle::None,
            bottom: BorderStyle::Solid,
            left: BorderStyle::None,
            corner_style: CornerStyle::Square,
        }
    }
    pub const fn left() -> Self {
        Self {
            top: BorderStyle::None,
            right: BorderStyle::None,
            bottom: BorderStyle::None,
            left: BorderStyle::Solid,
            corner_style: CornerStyle::Square,
        }
    }
    pub const fn right() -> Self {
        Self {
            top: BorderStyle::None,
            right: BorderStyle::Solid,
            bottom: BorderStyle::None,
            left: BorderStyle::None,
            corner_style: CornerStyle::Square,
        }
    }

    /// True iff every side is `None` (no border at all).
    pub const fn is_empty(&self) -> bool {
        self.top.is_none() && self.right.is_none() && self.bottom.is_none() && self.left.is_none()
    }
    /// True iff every side paints a visible glyph. (Used to gate
    /// rounded-corner rendering — corners only round when all four
    /// sides participate.)
    pub const fn is_box(&self) -> bool {
        self.top.is_visible()
            && self.right.is_visible()
            && self.bottom.is_visible()
            && self.left.is_visible()
    }
}

/// Corner glyph style — applies when all 4 sides are drawn (per-side
/// borders don't form corners). `Square` uses `┌┐└┘`; `Rounded` uses
/// `╭╮╰╯`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CornerStyle {
    #[default]
    Square,
    Rounded,
}

/// CSS `border-collapse` (M5.5). Default is `Separate` — every box
/// draws its own border ring. `Collapse` makes adjacent borders
/// share their cells: parent + child meeting at an edge use **one**
/// cell of border, not two; sibling flex children sharing an edge
/// also share **one** cell. The paint pass walks the buffer after
/// element-by-element border painting and rewrites junction glyphs
/// (`├ ┤ ┬ ┴ ┼`) based on 4-neighbor connectivity.
///
/// **Deliberate divergence from CSS:** the spec restricts
/// `border-collapse: collapse` to `<table>` boxes only. rdom extends
/// it to any flex container — TUI grid layouts are too dominant an
/// idiom to gate behind table semantics.
///
/// Style-conflict resolution (when parent + child borders share an
/// edge with different `border-style`): "outermost wins" — parent's
/// style at the shared edge defeats the child's. Simplification of
/// CSS's full hidden > double > solid > … cascade. Tracked as
/// `M5-COLLAPSE-1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderCollapse {
    /// CSS initial value. Each box's border ring is independent.
    #[default]
    Separate,
    /// Adjacent borders share cells; paint joiner rewrites junction
    /// glyphs.
    Collapse,
}

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

/// `.margin(2)` shortcut — applies `n` cells to all four sides.
/// Mirrors the ergonomic that `MinSize::From<u16>` provides for
/// `.min_width(10)`.
impl From<i16> for Margin {
    fn from(n: i16) -> Self {
        Self::all_cells(n)
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
