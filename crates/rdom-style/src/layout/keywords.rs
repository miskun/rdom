//! Keyword-valued layout properties: `flex-direction` (the axis and the
//! one-value [`FlexDirection`]), `flex-wrap`,
//! `scroll-behavior`, `box-sizing`, `direction`,
//! `writing-mode`, `display` (outer [`Display`] and inner [`Flow`]),
//! `white-space`, `caret-color`, `caret-text-color`, `pointer-events`,
//! `visibility`, `user-select`, `text-decoration`, `position` and
//! `z-index`. The Box Alignment keywords are in `alignment`.

/// Flexbox main-axis direction. Maps to CSS `flex-direction`; the
/// default is its initial value, `row` (CSS Flexbox §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Children laid out left to right (`flex-direction: row`).
    #[default]
    Row,
    /// Children laid out top to bottom (`flex-direction: column`).
    Column,
}

/// `flex-direction` as one value (CSS Flexbox §5.1): an axis and whether
/// main-start and main-end swap. rdom stores it as two fields — the axis
/// (`direction`) and `flex_reverse` — which
/// `ComputedStyle::flex_direction` and `TuiStyle::flex_direction` join.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexDirection {
    /// `row`, the initial value.
    #[default]
    Row,
    /// `row-reverse`.
    RowReverse,
    /// `column`.
    Column,
    /// `column-reverse`.
    ColumnReverse,
}

impl FlexDirection {
    /// The value for `axis`, reversed or not.
    pub const fn new(axis: Direction, reverse: bool) -> Self {
        match (axis, reverse) {
            (Direction::Row, false) => FlexDirection::Row,
            (Direction::Row, true) => FlexDirection::RowReverse,
            (Direction::Column, false) => FlexDirection::Column,
            (Direction::Column, true) => FlexDirection::ColumnReverse,
        }
    }

    /// The main axis.
    pub const fn axis(self) -> Direction {
        match self {
            FlexDirection::Row | FlexDirection::RowReverse => Direction::Row,
            FlexDirection::Column | FlexDirection::ColumnReverse => Direction::Column,
        }
    }

    /// Whether main-start and main-end swap (`*-reverse`).
    pub const fn is_reversed(self) -> bool {
        matches!(
            self,
            FlexDirection::RowReverse | FlexDirection::ColumnReverse
        )
    }
}

/// `flex-wrap` (CSS Flexbox §5.2): whether a flex container is
/// single-line or multi-line, and which way its lines stack. Not
/// inherited; initial `NoWrap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlexWrap {
    /// `nowrap`: one line; the items overflow (or shrink) on it.
    #[default]
    NoWrap,
    /// `wrap`: items break onto lines, stacked from cross-start.
    Wrap,
    /// `wrap-reverse`: as `wrap`, with cross-start and cross-end swapped
    /// — the first line at the cross-end edge.
    WrapReverse,
}

/// CSS `scroll-behavior` (CSSOM View §12.1): how a programmatic scroll
/// of this scroll container moves when its caller asks for behavior
/// `auto` — `element.scrollTo(…)`, `scrollTop = n`, `scrollIntoView()`,
/// and keyboard scrolling. User wheel and scrollbar drags are always
/// instant. Applies to scroll containers.
///
/// Does not inherit (matches CSS). Initial value: `Auto`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum ScrollBehavior {
    /// The scroll is instant. CSS default.
    #[default]
    Auto,
    /// The scroll animates over a user-agent-defined duration.
    Smooth,
}

/// CSS `box-sizing` (CSS UI 3 §3.1, now CSS Sizing 3 "Box Edges for
/// Sizing"): which box `width` / `height` and their `min-*` / `max-*`
/// measure. Not inherited; initial `content-box`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BoxSizing {
    /// The sizes measure the content box; padding and border lie
    /// outside them. The CSS initial value.
    #[default]
    ContentBox,
    /// The sizes measure the border box; the content box is what is
    /// left after padding and border, floored at zero.
    BorderBox,
}

/// CSS `direction` (CSS Writing Modes 4 §2.1): the inline base
/// direction — which edge is inline-start. Inherited; initial `ltr`.
/// (The Rust name avoids [`Direction`], rdom's `flex-direction`.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextDirection {
    /// Left to right: inline-start is the left edge.
    #[default]
    Ltr,
    /// Right to left: inline-start is the right edge.
    Rtl,
}

/// CSS `writing-mode` (CSS Writing Modes 4 §3.1). Inherited; initial
/// `horizontal-tb`. rdom computes every value but lays boxes out as
/// `horizontal-tb` (DIVERGENCES).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WritingMode {
    /// Horizontal lines, stacked top to bottom.
    #[default]
    HorizontalTb,
    /// Vertical lines, stacked right to left.
    VerticalRl,
    /// Vertical lines, stacked left to right.
    VerticalLr,
    /// Vertical lines, right to left, glyphs set sideways.
    SidewaysRl,
    /// Vertical lines, left to right, glyphs set sideways.
    SidewaysLr,
}

/// Outer display type (CSS Display 3 §2.1), and the box keywords
/// `contents` / `none` (§2.5): whether the element is block-level
/// (`Block`), inline-level (`Inline`, or the atomic `InlineBlock`), or
/// generates no box of its own (`Contents`) or none at all (`None`).
/// [`Flow`] is the inner display type; the mapping table is there.
///
/// Does not inherit (matches CSS). Default is `Block`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Display {
    /// `block`: a block-level box (CSS Display 3 §2.1) — in block flow it
    /// stacks on its parent's block axis, in a flex or grid container it
    /// is a flex or grid item, as every child box there is (blockified,
    /// §2.7). Gets its own `LayoutRect`. Default.
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
    /// `display: contents` (CSS Display 3 §2.5): the element generates
    /// no box — no padding, border, background or rect of its own — and
    /// its children and `::before` / `::after` take part in its parent's
    /// formatting context as if they were the parent's. It stays in the
    /// DOM: it inherits and passes inherited properties on, matches
    /// selectors, is on hit-test paths and can be focused. On a
    /// replaced element or form control it computes to `None`
    /// (Appendix B).
    Contents,
}

/// **Inner display** — how an element lays out its own children.
/// Pairs with [`Display`] (the "outer display" — how the element
/// participates in its parent).
///
/// CSS3 Display Module models display as a two-value property
/// `<outer> <inner>`:
///
/// | `display: <…>` (CSS Display 3 §2)        | outer `Display` | inner `Flow` |
/// |------------------------------------------|-----------------|--------------|
/// | `block` = `block flow` = `flow` (default) | `Block`         | `Block`      |
/// | `flow-root` = `block flow-root`           | `Block`         | `FlowRoot`   |
/// | `inline` = `inline flow`                  | `Inline`        | `Block`      |
/// | `inline-block` = `inline flow-root`       | `InlineBlock`   | `Block`      |
/// | `flex` = `block flex`                     | `Block`         | `Flex`       |
/// | `inline-flex` = `inline flex`             | `Inline`        | `Flex`       |
/// | `grid` = `block grid`                     | `Block`         | `Grid`       |
/// | `inline-grid` = `inline grid`             | `Inline`        | `Grid`       |
/// | `contents`                                | `Contents`      | `Block`      |
/// | `none`                                    | `None`          | `Block`      |
///
/// An inline block always establishes a block formatting context, so
/// its flow-root inner type is `Block` with the outer `InlineBlock`.
/// `list-item` (§2.3) is a flag beside the pair
/// (`TuiStyle::list_item`): `list-item` is `block flow` with it,
/// `inline list-item` `inline flow` with it.
///
/// Default is `Block` — rdom's block layout pass walks children
/// in document order, stacking at natural heights per CSS 2.1 §10.
/// Authors opt into flex distribution via `display: flex` (or
/// `display: inline-flex` for inline-level flex containers), and into
/// grid layout via `display: grid` / `inline-grid`.
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
    /// `flow-root` (CSS Display 3 §2.2): block flow, like `Block`, in a
    /// new block formatting context (CSS 2.1 §9.4.1) — no margin
    /// collapses through its edges.
    FlowRoot,
    /// `grid` (CSS Display 3 §2.2, CSS Grid 2 §5.1): the children are
    /// grid items, placed in the container's grid and sized by its
    /// tracks. Establishes an independent formatting context.
    Grid,
}

impl Flow {
    /// Block flow — `flow` or `flow-root` (CSS Display 3 §2.2): the
    /// children stack in normal flow (CSS 2.1 §9.4.1).
    pub const fn is_block_flow(self) -> bool {
        matches!(self, Flow::Block | Flow::FlowRoot)
    }

    /// A flex or grid container (CSS Display 3 §2.2): its in-flow
    /// children are items — blockified (§2.7), reordered by `order`,
    /// painted atomically — and each run of its child text is an
    /// anonymous item (CSS Flexbox §4, CSS Grid 2 §6.1).
    pub const fn is_flex_or_grid(self) -> bool {
        matches!(self, Flow::Flex | Flow::Grid)
    }
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

/// CSS `pointer-events` — the subset that means something in a cell
/// grid (`auto` | `none`). Inherited, like the web. `none` makes the
/// element transparent to hit-testing: pointer input falls through to
/// whatever is beneath, and a descendant that sets `auto` is a target
/// again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PointerEvents {
    /// Default: the element is a hit target.
    #[default]
    Auto,
    /// Transparent to the pointer.
    None,
}

/// CSS `visibility` (CSS Display 3 §4): whether the box is drawn.
/// Inherited; initial `visible`. A `hidden` box keeps its place and
/// size but draws nothing and is neither hit nor focusable; a
/// descendant may set `visible` again. `collapse` is `hidden`, except
/// on a flex item (Flexbox §4.4: removed, leaving a cross-size strut)
/// and a table row (CSS 2.1 §17.5.5: removed, its cells still sizing
/// the columns).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Visibility {
    #[default]
    Visible,
    Hidden,
    Collapse,
}

impl Visibility {
    /// `visible`: the box is drawn.
    pub const fn is_visible(self) -> bool {
        matches!(self, Visibility::Visible)
    }
}

/// Controls whether the user can select text inside the element.
/// Matches the CSS `user-select` property (CSS UI 4 §6.1). **Not
/// inherited** — an element without a declaration computes `Auto` —
/// but the *used* value of `Auto` follows the parent's used value
/// where that is `None` or `All`, so one rule on a wrapper still marks
/// a chrome subtree unselectable. The renderer resolves the used
/// value. Default is `Auto`.
///
/// Variants:
/// - `Auto` — used value `Contain` on an editable element; otherwise
///   `None` / `All` when the parent's used value is that, else `Text`.
/// - `Text` — selectable; stops a `None` / `All` parent from reaching
///   its descendants.
/// - `None` — not selectable. Drag-select skips this subtree (except
///   descendants that declare `Text` / `Contain` / `All`).
///   Use for UI chrome (sidebars, status bars, buttons).
/// - `All` — click anywhere inside selects the entire element as
///   one unit (one-tap-to-copy tokens, URLs, code snippets).
/// - `Contain` — a selection started inside cannot leave this
///   element. Does not propagate: a nested `Contain` is its own host.
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

/// `z-index: auto | <integer>` (CSS 2.1 §9.9.1). `Auto` does not
/// establish a stacking context; the positioned layer orders it by tree
/// position (as 0). The integer is any `i32`: a larger literal clamps to
/// the range (CSS Values 4 §5.1), as in every engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZIndex {
    /// `auto`. Default. No stacking context of its own; sorts as 0.
    #[default]
    Auto,
    /// Explicit integer; negative values are valid.
    Value(i32),
}
