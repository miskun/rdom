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
    /// width). Resolves at layout time once the parent dimension
    /// is known. Matches CSS `<percentage>` semantics for sizing
    /// properties; clamped to `u16::MAX` cells after multiplication.
    Percent(u16),
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

/// CSS `scrollbar-gutter` — controls whether a scrollable element
/// reserves space for its scrollbar even when not actively
/// showing one. CSS spec default is `Auto`: reserve nothing
/// until the scrollbar actually appears (content reflows when
/// it does). `Stable` always reserves so content never reflows.
///
/// rdom uses this to gate `reserve_scrollbar_gutter` in the
/// layout pass. With `Auto`, an `overflow: auto` element doesn't
/// give up cells for a scrollbar gutter that may never be needed
/// — important for single-row affordances like a closed
/// `<details>` summary. With `Stable`, the cell is reserved even
/// at rest, useful for live-updating content where mid-frame
/// reflow would be visually disruptive.
///
/// Does not inherit (matches CSS). Initial value: `Auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollbarGutter {
    /// Reserve gutter cells only when the scrollbar actually
    /// shows (i.e. `Overflow::Scroll` always reserves; `Auto`
    /// only when content overflows). CSS default.
    #[default]
    Auto,
    /// Always reserve a gutter for any axis with `Scroll` or
    /// `Auto` overflow — even when content fits. Content never
    /// reflows when a scrollbar appears.
    Stable,
}

/// Per-side border state. CSS lets authors enable any combination
/// of `border-top` / `border-right` / `border-bottom` / `border-left`
/// independently. `corner_style` only matters when all 4 sides are
/// drawn — the rounded-corner glyphs `╭╮╰╯` need both sides at a
/// corner to share a cell.
///
/// The `border` shorthand and the per-side longhands all write into
/// this struct via the cascade. `Border::default()` is "no border."
///
/// Backward-compat factory methods (`Border::single()`, `Border::top()`,
/// etc.) match the names of the old `Border` enum variants so the
/// cascade + paint sites that used to switch on those variants now
/// read field combinations instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Border {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
    pub corner_style: CornerStyle,
}

impl Border {
    /// All sides off. Same as `Default`.
    pub const fn none() -> Self {
        Self {
            top: false,
            right: false,
            bottom: false,
            left: false,
            corner_style: CornerStyle::Square,
        }
    }
    /// All four sides on, square corners. `border: solid` shorthand.
    pub const fn single() -> Self {
        Self {
            top: true,
            right: true,
            bottom: true,
            left: true,
            corner_style: CornerStyle::Square,
        }
    }
    /// All four sides on, rounded corners. `border: rounded` shorthand.
    pub const fn rounded() -> Self {
        Self {
            top: true,
            right: true,
            bottom: true,
            left: true,
            corner_style: CornerStyle::Rounded,
        }
    }
    /// Top side only. `border-top: solid` longhand without others.
    pub const fn top() -> Self {
        Self {
            top: true,
            right: false,
            bottom: false,
            left: false,
            corner_style: CornerStyle::Square,
        }
    }
    pub const fn bottom() -> Self {
        Self {
            top: false,
            right: false,
            bottom: true,
            left: false,
            corner_style: CornerStyle::Square,
        }
    }
    pub const fn left() -> Self {
        Self {
            top: false,
            right: false,
            bottom: false,
            left: true,
            corner_style: CornerStyle::Square,
        }
    }
    pub const fn right() -> Self {
        Self {
            top: false,
            right: true,
            bottom: false,
            left: false,
            corner_style: CornerStyle::Square,
        }
    }

    /// True if no side is enabled.
    pub const fn is_empty(&self) -> bool {
        !self.top && !self.right && !self.bottom && !self.left
    }
    /// True if exactly all four sides are enabled.
    pub const fn is_box(&self) -> bool {
        self.top && self.right && self.bottom && self.left
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

/// **Inner display** — how an element lays out its own children.
/// Pairs with [`Display`] (the "outer display" — how the element
/// participates in its parent).
///
/// CSS3 Display Module models display as a two-value property
/// `<outer> <inner>`:
///
/// | `display: <…>`     | outer `Display`   | inner `Flow` |
/// |--------------------|-------------------|--------------|
/// | `block` (default)  | `Block`           | `Block`      |
/// | `flex`             | `Block`           | `Flex`       |
/// | `inline`           | `Inline`          | n/a          |
/// | `inline-block`     | `InlineBlock`     | `Block`      |
/// | `inline-flex`      | `Inline`          | `Flex`       |
/// | `none`             | `None`            | n/a          |
///
/// Default is `Block` — rdom's block layout pass walks children
/// in document order, stacking at natural heights per CSS 2.1 §10.
/// Authors opt into flex distribution via `display: flex` (or
/// `display: inline-flex` for inline-level flex containers).
///
/// Does not inherit. Computed at cascade time alongside `Display`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Flow {
    /// Children stack vertically in document order at natural
    /// heights (CSS 2.1 §10). The default — matches CSS `display:
    /// block` inner. No distribution, no shrink-to-fit; container
    /// overflows below its content if too short. Vertical margins
    /// between adjacent block children collapse per CSS 2.1 §8.3.1.
    #[default]
    Block,
    /// Children participate in flex distribution along the
    /// container's `direction` axis (`Row` / `Column`). Grow, shrink,
    /// gap, justify-content semantics per CSS Flexible Box L1.
    /// Container forms a new BFC.
    Flex,
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

/// Offset value for `top` / `right` / `bottom` / `left`.
///
/// **Not `Copy`** — the `Calc` variant carries a boxed expression
/// tree. The simple variants clone in O(1); `Calc` clones the AST.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Length {
    /// `auto`. Resolution depends on context — phase-2 placement.
    #[default]
    Auto,
    /// Integer cells. Signed so negative offsets are valid CSS.
    Cells(i16),
    /// `calc(<expr>)`. Resolves at layout time against the
    /// parent's matching-axis content dimension (`top`/`bottom` →
    /// height, `left`/`right` → width). Result clamped to the
    /// `i16` range.
    Calc(Box<crate::calc::CalcExpr>),
}

impl Length {
    /// Resolve `Calc` to `Cells`, leaving other variants unchanged.
    pub fn resolve_calc(self, basis: i32) -> Length {
        match self {
            Length::Calc(expr) => {
                let v = expr.resolve(&crate::calc::ResolveCtx::new(basis));
                let clamped = v.max(i16::MIN as i32).min(i16::MAX as i32) as i16;
                Length::Cells(clamped)
            }
            other => other,
        }
    }
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
    let effective_border = if collapse == BorderCollapse::Collapse && !border.is_empty() {
        Border::none()
    } else {
        border
    };
    let border_left = effective_border.left as u16;
    let border_top = effective_border.top as u16;
    let border_h = border_left + effective_border.right as u16;
    let border_v = border_top + effective_border.bottom as u16;

    // Percent / calc padding resolves against the containing-block
    // width on ALL four sides (CSS 2.1 §8.4 — vertical padding
    // percent also uses width). `area.width` here is the element's
    // outer width, which under the standard box model equals
    // the containing-block width minus any position offsets.
    let cb_w = area.width;
    let pad_l = padding.left.resolve(cb_w);
    let pad_r = padding.right.resolve(cb_w);
    let pad_t = padding.top.resolve(cb_w);
    let pad_b = padding.bottom.resolve(cb_w);

    let inset_x = pad_l + border_left;
    let inset_y = pad_t + border_top;
    let total_h = pad_l + pad_r + border_h;
    let total_v = pad_t + pad_b + border_v;

    LayoutRect::new(
        area.x + inset_x as i32,
        area.y + inset_y as i32,
        area.width.saturating_sub(total_h),
        area.height.saturating_sub(total_v),
    )
}

/// Compute the **padding-box** edge for an element with `outer` (border-box)
/// rect and the given `border`. This is the CSS Box Model 3 §1 padding edge:
/// `border-box ∸ border` on each side.
///
/// CSS Overflow 3 §3 names this rect the **scrollport** of a scroll container:
/// the region inside which overflow content is clipped, where the scrollbar
/// gutter lives, and what `position: sticky` pins against.
///
/// Independent of `border-collapse`. The M5.5b layout-time expansion in
/// [`compute_content_area_collapsed`] widens `content_layout` into the border
/// ring so children with their own borders can position on the shared edge —
/// that's a *child-positioning* concern, not a paint-clipping one. CSS Overflow
/// 3 §3 places the scrollport at the padding-box for every scroll container,
/// no table/collapse exception, so paint clipping reads this rect even when
/// the layout-side content rect was expanded.
///
/// Saturating math throughout — a degenerate `outer` smaller than the border
/// insets yields a zero-size rect at the inset origin, never a panic.
pub fn compute_padding_box(outer: LayoutRect, border: Border) -> LayoutRect {
    let bl = border.left as u16;
    let bt = border.top as u16;
    let bh = bl + border.right as u16;
    let bv = bt + border.bottom as u16;
    LayoutRect::new(
        outer.x + bl as i32,
        outer.y + bt as i32,
        outer.width.saturating_sub(bh),
        outer.height.saturating_sub(bv),
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
        assert_eq!(p.left, PaddingValue::Cells(4));
        assert_eq!(p.right, PaddingValue::Cells(4));
        assert_eq!(p.top, PaddingValue::Cells(2));
        assert_eq!(p.bottom, PaddingValue::Cells(2));
    }

    // ── compute_content_area ─────────────────────────────────────────

    #[test]
    fn content_area_none_padding_no_border() {
        let area = LayoutRect::new(0, 0, 80, 24);
        assert_eq!(
            compute_content_area(area, Padding::default(), Border::none()),
            area
        );
    }

    #[test]
    fn content_area_with_padding() {
        let area = LayoutRect::new(0, 0, 80, 24);
        let content = compute_content_area(area, Padding::symmetric(2, 1), Border::none());
        assert_eq!(content, LayoutRect::new(2, 1, 76, 22));
    }

    #[test]
    fn content_area_with_asymmetric_padding() {
        let area = LayoutRect::new(0, 0, 80, 24);
        // top=1, right=3, bottom=2, left=5 → x+5, y+1, w-8, h-3
        let content = compute_content_area(area, Padding::new(1, 3, 2, 5), Border::none());
        assert_eq!(content, LayoutRect::new(5, 1, 72, 21));
    }

    #[test]
    fn content_area_with_single_border() {
        let area = LayoutRect::new(0, 0, 80, 24);
        let content = compute_content_area(area, Padding::default(), Border::single());
        assert_eq!(content, LayoutRect::new(1, 1, 78, 22));
    }

    #[test]
    fn content_area_with_top_only_border() {
        let area = LayoutRect::new(0, 0, 80, 24);
        let content = compute_content_area(area, Padding::default(), Border::top());
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

    // ── compute_padding_box (CSS Box Model 3 §1, Overflow 3 §3) ────

    #[test]
    fn padding_box_subtracts_border_on_all_sides() {
        // CSS Box Model 3: padding-box edge = border-box ∸ border.
        // Inset by 1 on every side reduces a 10×10 outer to an 8×8
        // padding-box offset by (1, 1).
        let outer = LayoutRect::new(0, 0, 10, 10);
        let border = Border::single(); // all four sides = 1 cell
        let pb = compute_padding_box(outer, border);
        assert_eq!(pb, LayoutRect::new(1, 1, 8, 8));
    }

    #[test]
    fn padding_box_independent_of_border_collapse() {
        // `compute_padding_box` ignores border-collapse entirely.
        // M5.5b's layout-time expansion of `content_layout` into the
        // border ring is a child-positioning concern, not a paint-
        // clipping one. CSS Overflow 3 §3: the scrollport is the
        // padding-box, full stop — no table/collapse exception.
        let outer = LayoutRect::new(5, 10, 20, 15);
        let border = Border::single();
        // Function signature takes no BorderCollapse parameter — the
        // semantics are independent by construction. The assertion
        // here is the result equals what we'd get for any collapse
        // mode (the same single rect).
        let pb = compute_padding_box(outer, border);
        assert_eq!(pb, LayoutRect::new(6, 11, 18, 13));
    }

    #[test]
    fn padding_box_saturates_when_border_exceeds_outer() {
        // Defensive: a degenerate outer rect smaller than the
        // border insets must not panic. Should saturate to a
        // zero-size rect at the inset origin.
        let outer = LayoutRect::new(0, 0, 1, 1);
        let border = Border::single();
        let pb = compute_padding_box(outer, border);
        // After inset by (1, 1) on a 1×1: x=1, y=1, w=0, h=0.
        assert_eq!(pb.x, 1);
        assert_eq!(pb.y, 1);
        assert_eq!(pb.width, 0);
        assert_eq!(pb.height, 0);
    }

    #[test]
    fn padding_box_with_no_border_equals_outer() {
        // No border → padding-box = border-box. The non-bordered
        // path doesn't shrink the rect.
        let outer = LayoutRect::new(3, 7, 50, 40);
        let pb = compute_padding_box(outer, Border::none());
        assert_eq!(pb, outer);
    }

    #[test]
    fn padding_box_per_side_border_only_top() {
        // The source-disclosure shape: `border-top: solid`, no other
        // sides. Padding-box drops only the top row.
        let outer = LayoutRect::new(0, 0, 20, 10);
        let mut border = Border::none();
        border.top = true;
        let pb = compute_padding_box(outer, border);
        assert_eq!(pb, LayoutRect::new(0, 1, 20, 9));
    }
}
