//! Layout types for flexbox-style TUI layout.
//!
//! Cell-based dimensions throughout. `LayoutRect` uses signed `i32` for
//! position so elements can sit above/left of the viewport (needed for
//! scroll clipping). `Size` + `Direction` + `Overflow` + `Border` +
//! `Padding` cover the CSS-like sizing model; `compute_content_area`
//! applies border + padding to derive the inner rect children lay out in.
//!
//! Note: `LayoutRect` here is different from `render::Rect` —
//! `LayoutRect` is signed (`i32` x/y) so layout can position children
//! above or left of their parent for scroll clipping; `render::Rect`
//! is unsigned (`u16`) because it names actual terminal grid cells.
//! Layout computes `LayoutRect`; paint clips + converts to `Rect`.

/// Layout rectangle with signed position (i32) and unsigned dimensions (u16).
///
/// Allows elements to be positioned above/left of the viewport (negative
/// coords) which is needed for scroll clipping — when content has scrolled
/// up, the laid-out rect has a negative `y` and only the visible portion
/// gets painted.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LayoutRect {
    pub x: i32,
    pub y: i32,
    pub width: u16,
    pub height: u16,
}

impl LayoutRect {
    pub fn new(x: i32, y: i32, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Right edge (x + width).
    pub fn right(&self) -> i32 {
        self.x + self.width as i32
    }

    /// Bottom edge (y + height).
    pub fn bottom(&self) -> i32 {
        self.y + self.height as i32
    }

    /// Check if this rect intersects another.
    pub fn intersects(&self, other: &LayoutRect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }

    /// Compute intersection of two rects. Empty if no overlap.
    pub fn intersection(&self, other: &LayoutRect) -> LayoutRect {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let r = self.right().min(other.right());
        let b = self.bottom().min(other.bottom());
        if r <= x || b <= y {
            LayoutRect::default()
        } else {
            LayoutRect::new(x, y, (r - x) as u16, (b - y) as u16)
        }
    }

    /// Zero dimensions.
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// Flexbox main-axis direction. Maps to CSS `flex-direction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Children laid out left to right (`flex-direction: row`).
    Row,
    /// Children laid out top to bottom (`flex-direction: column`).
    #[default]
    Column,
}

/// Sizing for width or height — the three CSS-like modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Size {
    /// Exact number of cells.
    Fixed(u16),
    /// Flexible: takes remaining space proportional to weight.
    /// `Flex(1)` = equal share. `Flex(2)` = double share.
    Flex(u16),
    /// Child determines its own size (default: content-driven).
    #[default]
    Auto,
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

/// Overflow behavior. Matches CSS `overflow` semantics as closely as a
/// cell grid allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    /// No clipping; content may draw outside the box.
    #[default]
    Visible,
    /// Clipped; scrollable; no scrollbar.
    Hidden,
    /// Clipped; scrollable; scrollbar always visible.
    Scroll,
    /// Clipped; scrollable; scrollbar visible only when needed.
    Auto,
}

/// Border style. Single/Rounded draws all four sides; Top/Bottom/Left/Right
/// draws only that one side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Border {
    #[default]
    None,
    Single,
    Rounded,
    Top,
    Bottom,
    Left,
    Right,
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

/// Cross-axis alignment. Maps to CSS `align-items`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

/// Display mode. Controls whether the element participates as a flex
/// item in its parent's block/flex context (`Block`) or flows inline
/// within its parent's inline formatting context (`Inline`).
///
/// Does not inherit (matches CSS). Default is `Block`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Display {
    /// Standalone flex item. Gets its own `LayoutRect`. Default.
    #[default]
    Block,
    /// Participates in its parent's inline formatting context. No
    /// independent layout rect; position computed during inline layout.
    Inline,
    /// Block-level box on the inside, inline atom on the outside.
    /// Sizes to intrinsic content on BOTH axes (does not stretch
    /// cross-axially under `width: Auto` like `Block` does). Carries
    /// padding / border / background / generated content. Participates
    /// in a parent's IFC as an atomic inline fragment when the parent
    /// is IFC, or as a flex item with intrinsic main + cross size when
    /// the parent is a flex container.
    InlineBlock,
    /// Not rendered at all. The element is skipped by both the
    /// layout pass (takes no space in its parent's flex flow) and
    /// the paint pass (no background, no content, no children).
    /// Matches CSS `display: none` — same semantic (and same use
    /// cases: hidden dialog, collapsed tree subtrees, closed-
    /// dropdown options, the `<colgroup>` / `<col>` metadata tags).
    None,
}

/// White-space handling for text inside an inline formatting context.
/// Matches the CSS property of the same name.
///
/// Inherits (IFC-wide behavior — a `<pre>` wrapper needs to affect
/// every inline descendant). Default is `Normal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhiteSpace {
    /// Collapse whitespace runs to a single space; trim IFC edges;
    /// allow soft wrapping at break opportunities. Default.
    #[default]
    Normal,
    /// Preserve all whitespace verbatim; `\n` forces a hard break;
    /// no soft wrapping.
    Pre,
    /// Preserve all whitespace verbatim AND allow soft wrapping at
    /// break opportunities (matches HTML `<textarea>`'s default
    /// behavior — the typed `\n` becomes a hard break, and lines
    /// that exceed the box wrap at whitespace).
    PreWrap,
    /// Collapse like `Normal`; never soft-wrap. `<br>` still hard-breaks.
    NoWrap,
}

/// CSS `caret-color` — controls the **background color** of the
/// caret cell inside editable elements. Matches the standard CSS
/// property name; in a TUI the caret is a block (one cell), so
/// `caret-color` sets the cell's bg. The glyph color above it is
/// controlled by the companion rdom property `caret-text-color`.
///
/// Variants:
/// - `Auto` — uses the underlying cell's foreground color as the
///   caret's bg, reproducing the classic "swap fg/bg" caret look
///   without relying on terminal SGR-7 reverse video.
/// - `Transparent` — caret is not painted. Authors who want focus
///   without a visible caret reach for `:focus { caret-color:
///   transparent; }`. Editing still works; only the visible
///   indicator is suppressed.
/// - `Color(c)` — caret cell bg = `c`. Pair with `caret-text-color`
///   for a fully theme-able caret.
///
/// Inherits per CSS spec (a `caret-color: transparent` on a
/// container suppresses every descendant editable's caret).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CaretColor {
    /// Default. Caret bg = underlying cell's fg.
    #[default]
    Auto,
    /// Caret is not painted.
    Transparent,
    /// Explicit caret cell background color. Stored as a `TuiColor`
    /// so `var(--accent)` style references resolve at cascade time
    /// the same way `color` / `background-color` values do.
    Color(crate::TuiColor),
}

/// rdom extension property — `caret-text-color` controls the
/// **foreground (glyph) color** of the caret cell. There is no
/// standard CSS counterpart because CSS's caret is a thin bar; in
/// a TUI the caret is a block with both fg and bg, so both need
/// independent control.
///
/// Documented in `DIVERGENCES.md` as a TUI-specific extension.
///
/// Variants:
/// - `Auto` — uses the underlying cell's background color as the
///   glyph color, reproducing the classic fg/bg swap visual.
/// - `Color(c)` — caret cell fg = `c`.
///
/// Inherits per CSS spec.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CaretTextColor {
    /// Default. Glyph color = underlying cell's bg.
    #[default]
    Auto,
    /// Explicit caret cell glyph color. Stored as a `TuiColor` for
    /// `var()` parity with other color properties.
    Color(crate::TuiColor),
}

/// Controls whether the user can select text inside the element.
/// Matches the CSS `user-select` property. Inherits (so a chrome
/// subtree can be marked unselectable with a single rule on the
/// wrapper). Default is `Auto`.
///
/// Variants:
/// - `Auto` — the selection algorithm decides based on element
///   type: text-bearing elements are selectable, UA-default
///   chrome (`<button>`) isn't.
/// - `Text` — always selectable.
/// - `None` — not selectable. Drag-select skips this subtree;
///   its edges are clamped to the nearest selectable ancestor.
///   Use for UI chrome (sidebars, status bars, buttons).
/// - `All` — click anywhere inside selects the entire element as
///   one unit (one-tap-to-copy tokens, URLs, code snippets).
/// - `Contain` — selection cannot cross this element's boundary.
///   Drag into the element clamps to its outer edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UserSelect {
    /// Default. Selectable when the element carries text.
    #[default]
    Auto,
    /// Always selectable.
    Text,
    /// Not selectable. Drag-select skips this subtree.
    None,
    /// Single-unit selection: click anywhere → whole element
    /// selected.
    All,
    /// Selection cannot cross this element's boundary.
    Contain,
}

/// CSS `text-decoration` property (subset). The CSS shorthand
/// accepts `<line> <style> <color>` triples (`underline dotted
/// red`); rdom 0.1.0 ships the `<line>` axis only, since
/// terminals don't render decoration styles or independent
/// decoration colors. The line value drives a single SGR
/// modifier bit: `Underline` → `Modifier::UNDERLINED` (SGR-4),
/// `LineThrough` → `Modifier::CROSSED_OUT` (SGR-9). `None`
/// clears both. (`Overline` is an HTML/CSS thing terminals
/// don't support cleanly; deferred.)
///
/// Does NOT inherit per CSS spec (each element sets its own
/// decoration). Initial value: `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextDecoration {
    /// No underline, no line-through. Initial value.
    #[default]
    None,
    /// Single underline. SGR-4.
    Underline,
    /// Strikethrough. SGR-9.
    LineThrough,
}

/// Padding (CSS order: top, right, bottom, left).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Padding {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

/// Margin value on a single side. CSS allows numeric (positive or
/// negative) and the `auto` keyword. `Auto` participates in flex
/// main-axis space absorption and absolute-element centering
/// (M5.3b). Until M5.3b lights up auto-absorption, layout treats
/// `Auto` as `Cells(0)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarginValue {
    /// `auto`. Participates in flex space distribution and absolute
    /// centering (M5.3b).
    Auto,
    /// Integer cells. Signed so negative margins are valid CSS.
    Cells(i16),
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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
        let v = MarginValue::Cells(n);
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
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

/// CSS `position` property (M2). Determines whether and how an
/// element is removed from normal flow and how it accepts
/// `top` / `right` / `bottom` / `left` offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    /// Default. In normal flow; `top/right/bottom/left` ignored.
    #[default]
    Static,
    /// In flow + still takes space; paint+hit-test rect shifted
    /// by `top/left`. Establishes a containing block.
    Relative,
    /// Removed from flow; positioned against nearest positioned
    /// ancestor (or the viewport).
    Absolute,
    /// Removed from flow; positioned against the viewport always.
    Fixed,
    /// In flow until the nearest scrollable ancestor would scroll
    /// the element past its threshold (`top` / `bottom` / `left` /
    /// `right` insets), at which point the element pins to that
    /// edge within its containing block. When the containing block
    /// itself scrolls past, the sticky element scrolls with it
    /// (the "post-stick" phase). M5.4.
    Sticky,
}

/// Offset value for `top` / `right` / `bottom` / `left` (M2). M5+
/// may grow `Percent(...)` once the layout primitives bundle adds
/// percentage resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Length {
    /// `auto`. Resolution depends on context — phase-2 placement.
    #[default]
    Auto,
    /// Integer cells. Signed so negative offsets are valid CSS.
    Cells(i16),
}

/// `z-index` value (M2). `Auto` does not establish a stacking
/// context; the M2 flat-sort model treats it as 0 for sort order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZIndex {
    /// `auto`. Default. Sorts as 0 in M2's flat z-list.
    #[default]
    Auto,
    /// Explicit integer; negative values are valid.
    Value(i16),
}

impl Padding {
    pub fn new(top: u16, right: u16, bottom: u16, left: u16) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Same horizontal (left/right) and vertical (top/bottom).
    pub fn symmetric(h: u16, v: u16) -> Self {
        Self {
            top: v,
            right: h,
            bottom: v,
            left: h,
        }
    }

    /// Same on all sides.
    pub fn all(n: u16) -> Self {
        Self {
            top: n,
            right: n,
            bottom: n,
            left: n,
        }
    }
}

/// Compute the inner "content" rectangle that children lay out in, given
/// an outer rectangle plus the element's padding and border. Shrinks the
/// outer rect by both. Use [`compute_content_area_collapsed`] when an
/// element under `border-collapse: collapse` needs the border-overlap
/// special case (M5.5b).
pub fn compute_content_area(area: LayoutRect, padding: Padding, border: Border) -> LayoutRect {
    compute_content_area_collapsed(area, padding, border, BorderCollapse::Separate)
}

/// Same as [`compute_content_area`] but aware of `border-collapse`.
/// Under `BorderCollapse::Collapse`, when the element has a border,
/// the parent's content area **includes** its own border-ring cells —
/// children's outer edges coincide with the parent's border cells.
/// Padding still insets normally.
///
/// This is decision 2 from the M5 pre-prep: concentrate the box-model
/// special case in this one function so every other layout consumer
/// stays unchanged.
pub fn compute_content_area_collapsed(
    area: LayoutRect,
    padding: Padding,
    border: Border,
    collapse: BorderCollapse,
) -> LayoutRect {
    // Collapse + border present → the parent's border ring is shared
    // with children's outer edges. Treat the parent as having no
    // border for content-area purposes; the border still paints
    // (paint pass renders it), but children's rects extend into
    // those cells.
    let effective_border = if collapse == BorderCollapse::Collapse && border != Border::None {
        Border::None
    } else {
        border
    };
    let border_left = matches!(
        effective_border,
        Border::Left | Border::Single | Border::Rounded
    ) as u16;
    let border_top = matches!(
        effective_border,
        Border::Top | Border::Single | Border::Rounded
    ) as u16;
    let border_h = match effective_border {
        Border::Left | Border::Right => 1,
        Border::Single | Border::Rounded => 2,
        _ => 0,
    };
    let border_v = match effective_border {
        Border::Top | Border::Bottom => 1,
        Border::Single | Border::Rounded => 2,
        _ => 0,
    };

    let inset_x = padding.left + border_left;
    let inset_y = padding.top + border_top;
    let total_h = padding.left + padding.right + border_h;
    let total_v = padding.top + padding.bottom + border_v;

    LayoutRect::new(
        area.x + inset_x as i32,
        area.y + inset_y as i32,
        area.width.saturating_sub(total_h),
        area.height.saturating_sub(total_v),
    )
}

/// Clamp a cell count to min/max constraints. Matches CSS: when `min >
/// max`, `min` wins.
pub fn clamp_size(value: u16, min: Option<u16>, max: Option<u16>) -> u16 {
    let mut result = value;
    if let Some(max) = max {
        result = result.min(max);
    }
    if let Some(min) = min {
        result = result.max(min);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── LayoutRect ───────────────────────────────────────────────────

    #[test]
    fn right_and_bottom_edges() {
        let r = LayoutRect::new(10, 20, 5, 6);
        assert_eq!(r.right(), 15);
        assert_eq!(r.bottom(), 26);
    }

    #[test]
    fn intersects_overlapping() {
        let a = LayoutRect::new(0, 0, 10, 10);
        let b = LayoutRect::new(5, 5, 10, 10);
        assert!(a.intersects(&b));
    }

    #[test]
    fn intersects_touching_is_false() {
        // Right edge touches left edge — CSS box model: no overlap.
        let a = LayoutRect::new(0, 0, 10, 10);
        let b = LayoutRect::new(10, 0, 10, 10);
        assert!(!a.intersects(&b));
    }

    #[test]
    fn intersection_basic() {
        let a = LayoutRect::new(-5, 0, 10, 10);
        let b = LayoutRect::new(0, 0, 20, 20);
        assert_eq!(a.intersection(&b), LayoutRect::new(0, 0, 5, 10));
    }

    #[test]
    fn intersection_no_overlap_is_empty() {
        let a = LayoutRect::new(-10, 0, 5, 5);
        let b = LayoutRect::new(0, 0, 20, 20);
        assert!(a.intersection(&b).is_empty());
    }

    #[test]
    fn is_empty_zero_dim() {
        assert!(LayoutRect::new(0, 0, 0, 5).is_empty());
        assert!(LayoutRect::new(0, 0, 5, 0).is_empty());
        assert!(!LayoutRect::new(0, 0, 5, 5).is_empty());
    }

    // ── Padding ──────────────────────────────────────────────────────

    #[test]
    fn padding_all_uniform() {
        let p = Padding::all(3);
        assert_eq!(p, Padding::new(3, 3, 3, 3));
    }

    #[test]
    fn padding_symmetric_hv() {
        let p = Padding::symmetric(4, 2);
        assert_eq!(p.left, 4);
        assert_eq!(p.right, 4);
        assert_eq!(p.top, 2);
        assert_eq!(p.bottom, 2);
    }

    // ── compute_content_area ─────────────────────────────────────────

    #[test]
    fn content_area_none_padding_no_border() {
        let area = LayoutRect::new(0, 0, 80, 24);
        assert_eq!(
            compute_content_area(area, Padding::default(), Border::None),
            area
        );
    }

    #[test]
    fn content_area_with_padding() {
        let area = LayoutRect::new(0, 0, 80, 24);
        let content = compute_content_area(area, Padding::symmetric(2, 1), Border::None);
        assert_eq!(content, LayoutRect::new(2, 1, 76, 22));
    }

    #[test]
    fn content_area_with_asymmetric_padding() {
        let area = LayoutRect::new(0, 0, 80, 24);
        // top=1, right=3, bottom=2, left=5 → x+5, y+1, w-8, h-3
        let content = compute_content_area(area, Padding::new(1, 3, 2, 5), Border::None);
        assert_eq!(content, LayoutRect::new(5, 1, 72, 21));
    }

    #[test]
    fn content_area_with_single_border() {
        let area = LayoutRect::new(0, 0, 80, 24);
        let content = compute_content_area(area, Padding::default(), Border::Single);
        assert_eq!(content, LayoutRect::new(1, 1, 78, 22));
    }

    #[test]
    fn content_area_with_top_only_border() {
        let area = LayoutRect::new(0, 0, 80, 24);
        let content = compute_content_area(area, Padding::default(), Border::Top);
        // border_top=1, border_left=0, border_v=1, border_h=0
        assert_eq!(content, LayoutRect::new(0, 1, 80, 23));
    }

    // ── clamp_size ───────────────────────────────────────────────────

    #[test]
    fn clamp_within_range_unchanged() {
        assert_eq!(clamp_size(10, Some(0), Some(20)), 10);
    }

    #[test]
    fn clamp_below_min() {
        assert_eq!(clamp_size(3, Some(5), Some(20)), 5);
    }

    #[test]
    fn clamp_above_max() {
        assert_eq!(clamp_size(30, Some(5), Some(20)), 20);
    }

    #[test]
    fn clamp_min_wins_over_max() {
        // min=10, max=5 → min wins → 10
        assert_eq!(clamp_size(7, Some(10), Some(5)), 10);
    }

    #[test]
    fn clamp_no_constraints() {
        assert_eq!(clamp_size(42, None, None), 42);
    }
}
