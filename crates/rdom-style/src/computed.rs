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
pub type VarMap = Rc<std::collections::HashMap<String, String>>;

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
    pub border_fg: Color,
    /// All modifier bits collapsed — bold, dim, italic, underlined,
    /// reversed. Cascade sets these from the individual `bold`/`dim`/...
    /// properties on `TuiStyle`.
    pub modifiers: Modifier,
    /// CSS `opacity` in `[0.0, 1.0]`. Cascade clamps; paint
    /// alpha-blends `fg` / `bg` / `border_fg` against the resolved
    /// parent bg. Truecolor-only — opacity only blends `Color::Rgb`
    /// values (a `Color::Reset` opacity is a no-op since the
    /// terminal default bg is unknowable). Default `1.0`.
    pub opacity: f32,

    // ── Layout ────────────────────────────────────────────────────────
    pub width: Size,
    pub height: Size,
    pub min_width: Option<crate::layout::MinSize>,
    pub max_width: Option<u16>,
    pub min_height: Option<crate::layout::MinSize>,
    pub max_height: Option<u16>,
    /// `aspect-ratio: <w> / <h>`. When set and one axis is explicit
    /// while the other is auto, the flex resolver computes the
    /// dependent axis (half-to-even rounded to integer cells). Both
    /// axes explicit → ignored. Preserved as `(numerator, denominator)`
    /// integers so the CSS round-trip recovers the original form.
    pub aspect_ratio: Option<crate::layout::AspectRatio>,
    pub padding: Padding,
    /// Resolved margin. CSS does NOT collapse adjacent block margins
    /// (rdom divergence — `M5-MARGIN-1`).
    pub margin: crate::layout::Margin,
    pub gap: u16,
    /// CSS `flex-shrink`. Default `1` (CSS spec). When total
    /// declared sizes exceed the parent's main-axis budget, items
    /// shrink proportional to `flex_shrink * basis`. `0` opts out.
    pub flex_shrink: u16,
    pub border: Border,
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

    // ── Inline formatting ────────────────────────────────────────────
    /// Outer display — how this element participates in its parent's
    /// formatting context. Default `Block`. See also `flow` for the
    /// inner display (how this element lays out its OWN children).
    pub display: Display,
    /// Inner display — how this element lays out its children.
    /// Default `Block` (children stack at natural heights, CSS 2.1
    /// §10 block flow). `display: flex` flips this to `Flex`. See
    /// [`Flow`] for the full table.
    pub flow: crate::layout::Flow,
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

    /// Custom-property values in scope. Populated from parent + own
    /// Custom-property (`--var-name: value;`) map in scope for this
    /// element. Populated from the stylesheet's `root_vars` during
    /// cascade; `Rc`-cloned into every `ComputedStyle` so references
    /// are cheap.
    pub vars: VarMap,
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
            border_fg: Color::Reset,
            modifiers: Modifier::empty(),
            opacity: 1.0,
            width: Size::Auto,
            height: Size::Auto,
            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            aspect_ratio: None,
            padding: Padding::default(),
            margin: crate::layout::Margin::default(),
            gap: 0,
            flex_shrink: 1,
            border: Border::none(),
            border_collapse: crate::layout::BorderCollapse::Separate,
            border_collapse_declared: false,
            direction: Direction::Column,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
            scrollbar_gutter: crate::layout::ScrollbarGutter::Auto,
            display: Display::Block,
            flow: crate::layout::Flow::Block,
            establishes_new_bfc: false,
            white_space: WhiteSpace::Normal,
            user_select: UserSelect::Auto,
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
            vars: Rc::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self::initial()
    }
}

/// Content for `::before` / `::after` — literal, variable reference,
/// attribute reference, or concatenation.
///
/// - `Content::Str("▾")` — straight string.
/// - `Content::Var("arrow")` — looked up in `ComputedStyle.vars`
///   (CSS custom properties: `content: var(--arrow)`).
/// - `Content::Attr("label")` — looked up in the host element's
///   attributes (CSS `content: attr(label)`).
/// - `Content::Concat(vec![Content::Str("▾ "), Content::Var("label")])` —
///   concatenation. Nests arbitrarily.
/// - `Content::None` — explicit "no content"; the pseudo-element does
///   not render at all (matches CSS `content: none;`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Content {
    Str(String),
    Var(String),
    Attr(String),
    Concat(Vec<Content>),
    None,
}

impl Content {
    /// Resolve `Var(...)` against `vars` and `Attr(...)` against the
    /// `attr_lookup` closure; join `Concat` parts. Returns `None` if
    /// the result should cause the pseudo-element to be skipped
    /// entirely (`Content::None`).
    ///
    /// Unresolved vars and missing attrs yield empty strings rather
    /// than failing — matches CSS's permissive behavior (browsers
    /// render nothing for `attr(missing)`).
    pub fn resolve<F>(
        &self,
        vars: &std::collections::HashMap<String, String>,
        attr_lookup: &F,
    ) -> Option<String>
    where
        F: Fn(&str) -> Option<String>,
    {
        match self {
            Content::None => None,
            Content::Str(s) => Some(s.clone()),
            Content::Var(name) => Some(vars.get(name).cloned().unwrap_or_default()),
            Content::Attr(name) => Some(attr_lookup(name).unwrap_or_default()),
            Content::Concat(parts) => {
                let mut out = String::new();
                for p in parts {
                    if let Some(s) = p.resolve(vars, attr_lookup) {
                        out.push_str(&s);
                    }
                    // Content::None inside a concat contributes nothing.
                }
                Some(out)
            }
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
        assert_eq!(s.gap, 0);
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

    /// Attribute lookup that returns None for every name — used by
    /// tests that don't exercise `Content::Attr`.
    fn no_attrs(_: &str) -> Option<String> {
        None
    }

    #[test]
    fn content_str_resolves_to_itself() {
        let vars = HashMap::new();
        assert_eq!(
            Content::Str("→".into()).resolve(&vars, &no_attrs),
            Some("→".into())
        );
    }

    #[test]
    fn content_var_looks_up() {
        let mut vars = HashMap::new();
        vars.insert("arrow".into(), "▾".into());
        assert_eq!(
            Content::Var("arrow".into()).resolve(&vars, &no_attrs),
            Some("▾".into())
        );
    }

    #[test]
    fn content_var_unresolved_empty_string() {
        let vars = HashMap::new();
        assert_eq!(
            Content::Var("nope".into()).resolve(&vars, &no_attrs),
            Some(String::new())
        );
    }

    #[test]
    fn content_attr_looks_up_via_closure() {
        let vars = HashMap::new();
        let lookup = |name: &str| match name {
            "label" => Some("Fruit".to_string()),
            _ => None,
        };
        assert_eq!(
            Content::Attr("label".into()).resolve(&vars, &lookup),
            Some("Fruit".into())
        );
    }

    #[test]
    fn content_attr_missing_yields_empty_string() {
        let vars = HashMap::new();
        let lookup = |_: &str| None;
        assert_eq!(
            Content::Attr("label".into()).resolve(&vars, &lookup),
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
        assert_eq!(c.resolve(&vars, &no_attrs), Some("FOO BAR BAZ".into()));
    }

    #[test]
    fn content_concat_mixes_var_and_attr() {
        let mut vars = HashMap::new();
        vars.insert("sep".into(), " · ".into());
        let lookup = |name: &str| match name {
            "label" => Some("Group".to_string()),
            _ => None,
        };
        let c = Content::Concat(vec![
            Content::Attr("label".into()),
            Content::Var("sep".into()),
            Content::Str("end".into()),
        ]);
        assert_eq!(c.resolve(&vars, &lookup), Some("Group · end".into()));
    }

    #[test]
    fn content_none_returns_none() {
        let vars = HashMap::new();
        assert_eq!(Content::None.resolve(&vars, &no_attrs), None);
    }

    #[test]
    fn content_none_inside_concat_contributes_nothing() {
        let vars = HashMap::new();
        let c = Content::Concat(vec![
            Content::Str("A".into()),
            Content::None,
            Content::Str("B".into()),
        ]);
        assert_eq!(c.resolve(&vars, &no_attrs), Some("AB".into()));
    }
}
