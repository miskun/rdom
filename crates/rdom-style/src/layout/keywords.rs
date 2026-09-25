//! Keyword-valued layout properties: `flex-direction`, `overflow`,
//! `scrollbar-gutter`, `align-items`, `display` (outer [`Display`] and
//! inner [`Flow`]), `white-space`, `caret-color`, `caret-text-color`,
//! `pointer-events`, `user-select`, `text-decoration`, `position` and
//! `z-index`.

/// Flexbox main-axis direction. Maps to CSS `flex-direction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Children laid out left to right (`flex-direction: row`).
    Row,
    /// Children laid out top to bottom (`flex-direction: column`).
    #[default]
    Column,
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

/// `z-index` value (M2). `Auto` does not establish a stacking
/// context; the positioned layer orders it by tree position (as 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZIndex {
    /// `auto`. Default. No stacking context of its own; sorts as 0.
    #[default]
    Auto,
    /// Explicit integer; negative values are valid.
    Value(i16),
}
