//! `ComputedStyle` — the post-cascade result. No `Option`, no `Value<_>`,
//! no `Inherit` / `Initial` keywords. Every field is a concrete value
//! that the layout and paint passes can read directly.
//!
//! Populated by `Dom::cascade()` and cached on `TuiExt`.

use std::rc::Rc;

use crate::layout::{
    Border, CaretColor, CaretTextColor, Direction, Display, Overflow, Padding, Size, UserSelect,
    WhiteSpace,
};
use crate::{Color, Modifier};

/// Resolved `var()` map. Copied by reference through inheritance so
/// child elements share their parent's vars without allocation.
pub type VarMap = Rc<std::collections::HashMap<String, crate::CustomValue>>;

/// Fully-concrete style. One per element + one each for `::before` /
/// `::after` (the pseudo-element variants live in `TuiExt::computed_before`
/// / `computed_after`).
// `Eq` is intentionally omitted: `opacity: f32` blocks it. PartialEq
// is sufficient for cascade diff comparisons and tests.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    // ── Paint ─────────────────────────────────────────────────────────
    pub fg: Color,
    pub bg: Color,
    /// `border-top-color` … `border-left-color`, resolved (an
    /// undeclared side is the element's `color`, `currentcolor`).
    pub border_color: crate::layout::Sides<Color>,
    /// All modifier bits collapsed — bold, dim, italic, underlined,
    /// reversed. Cascade sets these from the individual `bold`/`dim`/...
    /// properties on `TuiStyle`.
    pub modifiers: Modifier,
    /// CSS `opacity` in `[0.0, 1.0]`. Cascade clamps; paint
    /// alpha-blends `fg` / `bg` / `border_color` against the resolved
    /// parent bg. Truecolor-only — opacity only blends `Color::Rgb`
    /// values (a `Color::Reset` opacity is a no-op since the
    /// terminal default bg is unknowable). Default `1.0`.
    pub opacity: f32,
    /// `background-clip` of the final background layer — the box the
    /// background color is painted in (CSS Backgrounds 3 §3.2, §3.8).
    /// Initial `border-box`: under the border too.
    pub background_clip: crate::layout::VisualBox,

    // ── Layout ────────────────────────────────────────────────────────
    pub width: Size,
    pub height: Size,
    /// `min-width`; initial `auto` (CSS Sizing 3 §5.2).
    pub min_width: crate::layout::MinSize,
    pub max_width: crate::layout::MaxSize,
    /// `min-height`; initial `auto`.
    pub min_height: crate::layout::MinSize,
    pub max_height: crate::layout::MaxSize,
    /// `box-sizing` (CSS UI 3 §3.1): which box `width` / `height` and
    /// `min-*` / `max-*` measure. Initial `content-box`.
    pub box_sizing: crate::layout::BoxSizing,
    /// `contain-intrinsic-width` / `-height` (CSS Sizing 4 §6.1), viewport
    /// units resolved. Initial `none`. Used under size containment (C14).
    pub contain_intrinsic_width: crate::layout::ContainIntrinsicSize,
    pub contain_intrinsic_height: crate::layout::ContainIntrinsicSize,
    /// `aspect-ratio: <w> / <h>`. When set and one axis is explicit
    /// while the other is auto, the flex resolver computes the
    /// dependent axis (half-to-even rounded to integer cells). Both
    /// axes explicit → ignored. Preserved as `(numerator, denominator)`
    /// integers so the CSS round-trip recovers the original form.
    pub aspect_ratio: Option<crate::layout::AspectRatio>,
    pub padding: Padding,
    /// Resolved margin. Vertical margins collapse in block flow
    /// (CSS 2.1 §8.3.1) at layout time; this is the element's own value.
    pub margin: crate::layout::Margin,
    /// `margin-trim` (CSS Box 4 §3): which content edges trim the
    /// adjoining children's margins. Initial `none`.
    pub margin_trim: crate::layout::MarginTrim,
    /// `row-gap` / `column-gap` (CSS Box Alignment 3 §8.1), initial
    /// `normal`. A row flex container's items are `column-gap` apart, a
    /// column's `row-gap`; rdom also spaces a block container's block
    /// children by `row-gap` (DIVERGENCES).
    pub row_gap: crate::layout::GapValue,
    pub column_gap: crate::layout::GapValue,
    /// CSS `flex-shrink`. Default `1` (CSS spec). When total
    /// declared sizes exceed the parent's main-axis budget, items
    /// shrink proportional to `flex_shrink * basis`. `0` opts out.
    pub flex_shrink: f32,
    /// `order` (CSS Flexbox §5.4): the item's place in order-modified
    /// document order, which flex layout and paint use. Initial `0`.
    pub order: i32,
    /// `flex-grow` (CSS Flexbox §7.3.1): the item's share of positive
    /// free space. Initial `0`.
    pub flex_grow: f32,
    /// `flex-basis` (CSS Flexbox §7.3.3): the flex base size (§9.2
    /// step 3). Initial `auto`.
    pub flex_basis: crate::layout::FlexBasis,
    /// The used border: [`border_style`](Self::border_style) with every
    /// zero-width side `none` (CSS Backgrounds 3 §4.3) — what layout
    /// reserves cells for and paint draws.
    pub border: Border,
    /// The cascaded `border-*-style`s, which `inherit` / `revert` copy.
    pub border_style: Border,
    /// The cascaded `border-*-width`s, viewport units resolved.
    pub border_width: crate::layout::Sides<crate::layout::BorderWidth>,
    /// The `border-*-radius`es (CSS Backgrounds 3 §5.1), viewport units
    /// resolved; a percentage stays for paint, which knows the box.
    pub border_radius: crate::layout::Corners<crate::layout::BorderRadius>,
    /// `box-shadow` (CSS Backgrounds 3 §6.1), front to back: colors
    /// resolved (`currentcolor` against the element's `color`), viewport
    /// units resolved.
    pub box_shadow: Vec<crate::layout::BoxShadow<Color>>,
    /// `border-spacing` (CSS 2.1 §17.6.1), viewport units resolved.
    /// Inherited. Not laid out yet (C13-TFC).
    pub border_spacing: crate::layout::BorderSpacing,
    /// `border-collapse: separate | collapse`. CSS-faithful name,
    /// extended to apply to any flex container (rdom divergence).
    /// **Inherits** — the cascade propagates parent's value to
    /// children. Default `Separate`.
    pub border_collapse: crate::layout::BorderCollapse,
    /// True iff this element was assigned `border-collapse` by a
    /// cascade rule that specified a concrete value (`collapse` or
    /// `separate`) — not via inheritance and not via
    /// `border-collapse: inherit`. Identifies the element as a
    /// **collapse-root**: the boundary of its own collapse group,
    /// equivalent to a `<table>` in the CSS table model. Used by
    /// the flex/block layout's transparent-intermediate propagation
    /// (`has_effective_border_on_edge`) to seal nested collapse-
    /// groups from each other, matching CSS 2.1 §17.6.2.1's table-
    /// equals-boundary rule extended to rdom's non-table elements.
    pub border_collapse_declared: bool,
    pub direction: Direction,
    /// `row-reverse` / `column-reverse` (CSS Flexbox §5.1): main-start
    /// and main-end of `direction`'s axis swap. Initial `false`.
    pub flex_reverse: bool,
    /// CSS `direction` (CSS Writing Modes 4 §2.1): which edge is
    /// inline-start. Inherited; initial `ltr`. (`direction` above is
    /// `flex-direction`.)
    pub text_direction: crate::layout::TextDirection,
    /// CSS `writing-mode` (CSS Writing Modes 4 §3.1). Inherited; layout
    /// is `horizontal-tb` whatever it computes to (DIVERGENCES).
    pub writing_mode: crate::layout::WritingMode,
    /// Per-axis overflow. Resolved after the cross-axis rule from
    /// CSS Overflow Level 3: if one axis is not `Visible` and the
    /// other is `Visible`, the `Visible` side behaves as `Auto`.
    /// The cascade normalizes both axes to a consistent pair.
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    /// CSS `scrollbar-gutter` — controls whether `Overflow::Auto`
    /// reserves gutter cells when no scrollbar is actually showing.
    /// `Auto` (default) reserves only when overflow occurs (TUI
    /// approximation: never pre-reserve for `Auto`; reserve for
    /// `Scroll`). `Stable` always reserves to prevent reflow when
    /// a scrollbar appears.
    pub scrollbar_gutter: crate::layout::ScrollbarGutter,
    /// CSS `scroll-behavior` — whether a programmatic scroll of this
    /// scroll container animates (`Smooth`) or jumps (`Auto`, default).
    /// Read by the runtime's scroll paths, not by layout.
    pub scroll_behavior: crate::layout::ScrollBehavior,

    // ── Inline formatting ────────────────────────────────────────────
    /// Outer display — how this element participates in its parent's
    /// formatting context. Default `Block`. See also `flow` for the
    /// inner display (how this element lays out its OWN children).
    pub display: Display,
    /// Inner display — how this element lays out its children.
    /// Default `Block` (children stack at natural heights, CSS 2.1
    /// §10 block flow). `display: flex` flips this to `Flex`. See
    /// [`Flow`](crate::layout::Flow) for the full table.
    pub flow: crate::layout::Flow,
    /// Whether `display` names `list-item` (CSS Display 3 §2.3). Not
    /// inherited; default `false`. rdom has no `::marker` yet: a list
    /// item lays out as its outer and inner types say (DIVERGENCES).
    pub list_item: bool,
    /// True when this element establishes a new **block formatting
    /// context** per CSS 2.1 §9.4.1. Triggers: root element, flex
    /// containers, inline-blocks, absolute/fixed positioning,
    /// non-visible overflow on either axis. Margin collapsing
    /// crosses a parent-child boundary only when the parent does
    /// NOT establish a new BFC. Computed at cascade finalization
    /// (last pass over the property bag).
    pub establishes_new_bfc: bool,
    /// Whitespace handling inside an inline formatting context.
    /// Inherits. Default `Normal`.
    pub white_space: WhiteSpace,
    /// Whether text inside this element is selectable by the user.
    /// Inherits. Default `Auto`.
    pub user_select: UserSelect,
    /// CSS `pointer-events`. Inherited; initial `auto`.
    pub pointer_events: crate::layout::PointerEvents,
    /// CSS `visibility` (CSS Display 3 §4). Inherited; initial
    /// `visible`.
    pub visibility: crate::layout::Visibility,
    /// Whether the caret is visible. `Auto` paints the caret as a
    /// REVERSED cell; `Transparent` suppresses caret paint. Inherits.
    /// Default `Auto`.
    pub caret_color: CaretColor,
    /// Glyph color of the caret cell. Inherits. Default `Auto`.
    pub caret_text_color: CaretTextColor,

    // ── Content (pseudo-elements) ─────────────────────────────────────
    /// Resolved `content:` value for this element or pseudo-element.
    /// `None` when `content: none;` or no content was specified.
    pub content: Option<String>,

    // ── Positioning (M2) ─────────────────────────────────────────────
    /// `position` keyword. Default `Static`. Non-inheriting.
    pub position: crate::layout::Position,
    /// `top` / `right` / `bottom` / `left` offsets. Default `Auto`.
    /// Non-inheriting.
    pub top: crate::layout::Length,
    pub right: crate::layout::Length,
    pub bottom: crate::layout::Length,
    pub left: crate::layout::Length,
    /// `z-index`. Default `Auto`. Non-inheriting.
    pub z_index: crate::layout::ZIndex,

    // ── Transitions (M3) ─────────────────────────────────────────────
    /// Resolved `transition-*` longhand lists. Empty when no
    /// transition rules apply. Engine reads them by index per the
    /// CSS L1 reconciliation rule (shorter lists cycle).
    pub transition_property: Vec<crate::transition::TransitionProperty>,
    pub transition_duration: Vec<u32>,
    pub transition_timing_function: Vec<crate::transition::TimingFunction>,
    pub transition_delay: Vec<u32>,

    /// `counter-reset` / `counter-increment` (CSS Lists 3 §3.1).
    /// Non-inheriting; the cascade applies them to its counter state
    /// in tree order.
    pub counter_reset: Vec<crate::counters::CounterOp>,
    pub counter_increment: Vec<crate::counters::CounterOp>,

    /// `color-scheme` (CSS Color Adjust 1 §2). Inherits; initial
    /// `normal`. The used scheme is
    /// [`ColorSchemeList::used`](crate::color::ColorSchemeList::used)
    /// of the document's preferred one.
    pub color_scheme: crate::color::ColorSchemeList,

    /// Custom-property values in scope. Populated from parent + own
    /// Custom-property (`--var-name: value;`) map in scope for this
    /// element. Populated from the stylesheet's `root_vars` during
    /// cascade; `Rc`-cloned into every `ComputedStyle` so references
    /// are cheap.
    pub vars: VarMap,
    /// `vars` with the running transitions of registered custom
    /// properties applied (CSS Properties and Values API 1 §6.2) — on
    /// this element or inherited — or `None` when none applies. `var()`
    /// substitution reads it; transitions compare `vars`, the cascaded
    /// values.
    pub animated_vars: Option<VarMap>,
}

impl ComputedStyle {
    /// Spec initial values: what every property starts as before any
    /// cascade input is applied. `Color::Reset` means "use the terminal
    /// default"; size/layout defaults match the legacy Element defaults
    /// for continuity.
    pub fn initial() -> Self {
        Self {
            fg: Color::Reset,
            bg: Color::Reset,
            border_color: crate::layout::Sides::all(Color::Reset),
            modifiers: Modifier::empty(),
            opacity: 1.0,
            background_clip: crate::layout::VisualBox::BorderBox,
            width: Size::Auto,
            height: Size::Auto,
            min_width: crate::layout::MinSize::Auto,
            max_width: crate::layout::MaxSize::None,
            min_height: crate::layout::MinSize::Auto,
            max_height: crate::layout::MaxSize::None,
            box_sizing: crate::layout::BoxSizing::ContentBox,
            contain_intrinsic_width: crate::layout::ContainIntrinsicSize::default(),
            contain_intrinsic_height: crate::layout::ContainIntrinsicSize::default(),
            aspect_ratio: None,
            padding: Padding::default(),
            margin: crate::layout::Margin::default(),
            margin_trim: crate::layout::MarginTrim::NONE,
            row_gap: crate::layout::GapValue::Normal,
            column_gap: crate::layout::GapValue::Normal,
            flex_shrink: 1.0,
            order: 0,
            flex_grow: 0.0,
            flex_basis: crate::layout::FlexBasis::Auto,
            border: Border::none(),
            border_style: Border::none(),
            border_width: crate::layout::Sides::default(),
            border_radius: crate::layout::Corners::default(),
            box_shadow: Vec::new(),
            border_spacing: crate::layout::BorderSpacing::default(),
            border_collapse: crate::layout::BorderCollapse::Separate,
            border_collapse_declared: false,
            direction: Direction::Column,
            flex_reverse: false,
            text_direction: crate::layout::TextDirection::Ltr,
            writing_mode: crate::layout::WritingMode::HorizontalTb,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
            scrollbar_gutter: crate::layout::ScrollbarGutter::Auto,
            scroll_behavior: crate::layout::ScrollBehavior::Auto,
            display: Display::Block,
            flow: crate::layout::Flow::Block,
            list_item: false,
            establishes_new_bfc: false,
            white_space: WhiteSpace::Normal,
            user_select: UserSelect::Auto,
            pointer_events: crate::layout::PointerEvents::Auto,
            visibility: crate::layout::Visibility::Visible,
            caret_color: CaretColor::Auto,
            caret_text_color: CaretTextColor::Auto,
            content: None,
            position: crate::layout::Position::Static,
            top: crate::layout::Length::Auto,
            right: crate::layout::Length::Auto,
            bottom: crate::layout::Length::Auto,
            left: crate::layout::Length::Auto,
            z_index: crate::layout::ZIndex::Auto,
            transition_property: Vec::new(),
            transition_duration: Vec::new(),
            transition_timing_function: Vec::new(),
            transition_delay: Vec::new(),
            counter_reset: Vec::new(),
            counter_increment: Vec::new(),
            color_scheme: crate::color::ColorSchemeList::normal(),
            vars: Rc::new(std::collections::HashMap::new()),
            animated_vars: None,
        }
    }
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self::initial()
    }
}

/// Content for `::before` / `::after` — literal, variable reference,
/// counter, or concatenation.
///
/// - `Content::Str("▾")` — straight string.
/// - `Content::Var("arrow")` — looked up in `ComputedStyle.vars`
///   (CSS custom properties: `content: var(--arrow)`).
/// - `Content::Concat(vec![Content::Str("▾ "), Content::Var("label")])` —
///   concatenation. Nests arbitrarily.
/// - `Content::None` — explicit "no content"; the pseudo-element does
///   not render at all (matches CSS `content: none;`).
///
/// Resolution reads the element's context through [`ContentContext`].
/// CSS `content: attr(label)` is an `attr()` substitution (CSS Values 5
/// §8.7), which the cascade makes before parsing the value: the host's
/// attribute arrives as a string, so there is no attribute variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Content {
    Str(String),
    Var(String),
    /// `counter(name[, style])` — the innermost counter of that name
    /// in scope at the pseudo-element (CSS Lists 3 §3.2).
    Counter {
        name: String,
        style: crate::counters::CounterStyle,
    },
    Concat(Vec<Content>),
    None,
}

/// What `content` resolution needs from the element it is generated
/// for: its custom properties and its counters. The
/// cascade implements this over its working state; a bare
/// `HashMap<String, CustomValue>` (a [`VarMap`]'s map) implements it as "variables only" for
/// callers without an element.
pub trait ContentContext {
    /// The value of custom property `--name` (without the dashes).
    fn var(&self, name: &str) -> Option<String>;
    /// The innermost counter `name` in scope (0 when there is none).
    fn counter(&self, name: &str) -> i32;
}

impl ContentContext for std::collections::HashMap<String, crate::CustomValue> {
    fn var(&self, name: &str) -> Option<String> {
        self.get(name).map(|v| v.as_str().to_string())
    }
    fn counter(&self, _name: &str) -> i32 {
        0
    }
}

impl Content {
    /// Resolve `Var(...)` and `Counter { .. }` against `ctx`; join
    /// `Concat` parts. Returns `None` if the result should cause the
    /// pseudo-element to be skipped entirely (`Content::None`).
    ///
    /// An unresolved var yields the empty string rather than failing.
    pub fn resolve(&self, ctx: &impl ContentContext) -> Option<String> {
        match self {
            Content::None => None,
            Content::Str(s) => Some(s.clone()),
            Content::Var(name) => Some(ctx.var(name).unwrap_or_default()),
            Content::Counter { name, style } => Some(style.format(ctx.counter(name))),
            Content::Concat(parts) => {
                let mut out = String::new();
                for p in parts {
                    if let Some(s) = p.resolve(ctx) {
                        out.push_str(&s);
                    }
                    // Content::None inside a concat contributes nothing.
                }
                Some(out)
            }
        }
    }

    /// Does this value (or any nested part) read a counter?
    pub fn uses_counters(&self) -> bool {
        match self {
            Content::Counter { .. } => true,
            Content::Concat(parts) => parts.iter().any(Content::uses_counters),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn initial_is_safe_defaults() {
        let s = ComputedStyle::initial();
        assert_eq!(s.fg, Color::Reset);
        assert_eq!(s.bg, Color::Reset);
        assert_eq!(s.modifiers, Modifier::empty());
        assert_eq!(s.width, Size::Auto);
        assert_eq!(s.height, Size::Auto);
        assert_eq!(s.direction, Direction::Column);
        assert_eq!(s.overflow_x, Overflow::Visible);
        assert_eq!(s.overflow_y, Overflow::Visible);
        assert_eq!(s.border, Border::none());
        assert_eq!(s.row_gap, crate::layout::GapValue::Normal);
        assert_eq!(s.column_gap, crate::layout::GapValue::Normal);
        assert_eq!(s.padding, Padding::default());
        assert_eq!(s.display, Display::Block);
        assert_eq!(s.flow, crate::layout::Flow::Block);
        assert!(!s.establishes_new_bfc);
        assert_eq!(s.white_space, WhiteSpace::Normal);
        assert_eq!(s.user_select, UserSelect::Auto);
        assert!(s.content.is_none());
        assert!(s.vars.is_empty());
    }

    #[test]
    fn default_matches_initial() {
        assert_eq!(ComputedStyle::default(), ComputedStyle::initial());
    }

    #[test]
    fn content_str_resolves_to_itself() {
        let vars = HashMap::new();
        assert_eq!(Content::Str("→".into()).resolve(&vars), Some("→".into()));
    }

    #[test]
    fn content_var_looks_up() {
        let mut vars = HashMap::new();
        vars.insert("arrow".into(), "▾".into());
        assert_eq!(
            Content::Var("arrow".into()).resolve(&vars),
            Some("▾".into())
        );
    }

    #[test]
    fn content_var_unresolved_empty_string() {
        let vars = HashMap::new();
        assert_eq!(
            Content::Var("nope".into()).resolve(&vars),
            Some(String::new())
        );
    }

    #[test]
    fn content_concat_joins() {
        let mut vars = HashMap::new();
        vars.insert("x".into(), "BAR".into());
        let c = Content::Concat(vec![
            Content::Str("FOO ".into()),
            Content::Var("x".into()),
            Content::Str(" BAZ".into()),
        ]);
        assert_eq!(c.resolve(&vars), Some("FOO BAR BAZ".into()));
    }

    #[test]
    fn content_none_returns_none() {
        let vars = HashMap::new();
        assert_eq!(Content::None.resolve(&vars), None);
    }

    #[test]
    fn content_none_inside_concat_contributes_nothing() {
        let vars = HashMap::new();
        let c = Content::Concat(vec![
            Content::Str("A".into()),
            Content::None,
            Content::Str("B".into()),
        ]);
        assert_eq!(c.resolve(&vars), Some("AB".into()));
    }
}
