//! `ComputedStyle` — the post-cascade result. No `Option`, no `Value<_>`,
//! no `Inherit` / `Initial` keywords. Every field is a concrete value
//! that the layout and paint passes can read directly.
//!
//! Populated by `Dom::cascade()` and cached on `TuiExt`.

use std::rc::Rc;

use crate::layout::{
    Border, CaretColor, CaretTextColor, Direction, Display, Overflow, Padding, Size, UserSelect,
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
    /// The modifier bits the font draws: bold (`font-weight` from 600)
    /// and italic (`font-style`), derived by the cascade from
    /// [`font`](Self::font). The decorations are
    /// [`applied_decorations`](Self::applied_decorations).
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
    /// `grid-template-columns` (CSS Grid 2 §7.2): the explicit columns of
    /// a grid container, viewport units resolved. Initial `none`.
    pub grid_template_columns: crate::layout::GridTemplate,
    /// `grid-template-rows` (§7.2): the explicit rows. Initial `none`.
    pub grid_template_rows: crate::layout::GridTemplate,
    /// `grid-template-areas` (CSS Grid 2 §7.3): the explicit grid's
    /// named areas. Initial `none`.
    pub grid_template_areas: crate::layout::GridTemplateAreas,
    /// `grid-auto-columns` (CSS Grid 2 §7.6): the implicit columns'
    /// sizes, repeated as a pattern; never empty. Initial `auto`, the
    /// shared [`TrackSize::AUTO_LIST`](crate::layout::TrackSize::AUTO_LIST)
    /// borrowed — every element starts from the initial style, so it
    /// allocates nothing (C7G-INITIAL-ALLOC); a declared list is owned.
    pub grid_auto_columns: std::borrow::Cow<'static, [crate::layout::TrackSize]>,
    /// `grid-auto-rows` (§7.6): the implicit rows' sizes. Initial `auto`,
    /// borrowed as `grid_auto_columns`'.
    pub grid_auto_rows: std::borrow::Cow<'static, [crate::layout::TrackSize]>,
    /// `grid-auto-flow` (CSS Grid 2 §7.7). Initial `row`.
    pub grid_auto_flow: crate::layout::GridAutoFlow,
    /// `grid-row-start` (CSS Grid 2 §8.3): where the item's grid area
    /// starts among the rows. Initial `auto`, as are the next three.
    pub grid_row_start: crate::layout::GridLine,
    /// `grid-row-end` (§8.3).
    pub grid_row_end: crate::layout::GridLine,
    /// `grid-column-start` (§8.3).
    pub grid_column_start: crate::layout::GridLine,
    /// `grid-column-end` (§8.3).
    pub grid_column_end: crate::layout::GridLine,
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
    /// `flex-direction`'s axis (CSS Flexbox §5.1); initial `row`. Read
    /// by flex containers only: a block container's children stack on
    /// its block axis whatever this says.
    pub direction: Direction,
    /// `row-reverse` / `column-reverse` (CSS Flexbox §5.1): main-start
    /// and main-end of `direction`'s axis swap. Initial `false`.
    pub flex_reverse: bool,
    /// `flex-wrap` (CSS Flexbox §5.2): single- or multi-line. Initial
    /// `nowrap`; not inherited.
    pub flex_wrap: crate::layout::FlexWrap,
    /// `justify-content` (CSS Box Alignment 3 §5.2): the main-axis
    /// alignment of a flex container's lines' items. Initial `normal`;
    /// not inherited.
    pub justify_content: crate::layout::Alignment,
    /// `align-items` (CSS Box Alignment 3 §6.3): the default cross-axis
    /// alignment of a flex container's items. Initial `normal`.
    pub align_items: crate::layout::Alignment,
    /// `align-content` (CSS Box Alignment 3 §5.1): how a multi-line flex
    /// container's lines share its cross axis. Initial `normal`.
    pub align_content: crate::layout::Alignment,
    /// `align-self` (§6.1): the item's own cross-axis alignment; `auto`
    /// (the initial value) takes its container's `align-items`.
    pub align_self: crate::layout::Alignment,
    /// `justify-items` (CSS Box Alignment 3 §6.2): the default
    /// `justify-self` of the box's children. Initial `legacy`, which
    /// computes to the parent's value when that is `legacy …`, else to
    /// `normal` (the cascade does this, so a computed style holds
    /// `legacy` only with a side).
    pub justify_items: crate::layout::Alignment,
    /// `justify-self` (§6.1): a block-level box's inline-axis alignment
    /// in its containing block; `auto` (initial) takes the parent's
    /// `justify-items`. Ignored in flex layout.
    pub justify_self: crate::layout::Alignment,
    /// CSS `direction` (CSS Writing Modes 4 §2.1): which edge is
    /// inline-start. Inherited; initial `ltr`. (`direction` above is
    /// `flex-direction`.)
    pub text_direction: crate::layout::TextDirection,
    /// CSS `writing-mode` (CSS Writing Modes 4 §3.1). Inherited; layout
    /// is `horizontal-tb` whatever it computes to (DIVERGENCES).
    pub writing_mode: crate::layout::WritingMode,
    /// Per-axis overflow, after CSS Overflow 3 §3.1's computed-value
    /// rule: when one axis is a scroll container's (`hidden`, `scroll`,
    /// `auto`), a `visible` other axis computes to `auto` and a `clip`
    /// one to `hidden`.
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    /// `overflow-clip-margin` (CSS Overflow 3 §3.2): a `clip` axis's
    /// overflow clip edge.
    pub overflow_clip_margin: crate::layout::OverflowClipMargin,
    /// `text-overflow` (CSS Overflow 4 §3); not inherited.
    pub text_overflow: crate::layout::TextOverflow,
    /// `max-lines` (CSS Overflow 4 §4.2); `None` is `none`.
    pub max_lines: Option<u32>,
    /// `block-ellipsis` (CSS Overflow 4 §4.3); inherited.
    pub block_ellipsis: crate::layout::BlockEllipsis,
    /// `continue` (CSS Overflow 4 §4.4).
    pub continue_: crate::layout::Continue,
    /// `-webkit-box-orient`: the legacy clamp's condition.
    pub webkit_box_orient: crate::layout::BoxOrient,
    /// Derived at the cascade's end: whether the box is a line-clamp
    /// container (CSS Overflow 4 §4) — a block container with
    /// `max-lines` and `continue: collapse` / `discard`, or the
    /// `-webkit-legacy` form on a `-webkit-box` with a vertical
    /// `-webkit-box-orient`, which then lays out as a block container.
    pub line_clamp_container: bool,
    /// CSS `scrollbar-gutter` — controls whether `Overflow::Auto`
    /// reserves gutter cells when no scrollbar is actually showing.
    /// `Auto` (default) reserves only when overflow occurs (TUI
    /// approximation: never pre-reserve for `Auto`; reserve for
    /// `Scroll`). `Stable` always reserves to prevent reflow when
    /// a scrollbar appears.
    pub scrollbar_gutter: crate::layout::ScrollbarGutter,
    /// `scrollbar-width` (CSS Scrollbars 1 §3). Not inherited.
    pub scrollbar_width: crate::layout::ScrollbarWidth,
    /// `scrollbar-color` (CSS Scrollbars 1 §2), its colors as specified.
    /// Inherited.
    pub scrollbar_color: crate::layout::ScrollbarColor,
    /// `overscroll-behavior-x` / `-y` (CSS Overscroll Behavior 1 §3).
    /// Not inherited.
    pub overscroll_behavior_x: crate::layout::OverscrollBehavior,
    pub overscroll_behavior_y: crate::layout::OverscrollBehavior,
    /// `scroll-padding-*` (CSS Scroll Snap 1 §4.1), the scroll container's
    /// optimal viewing region's insets. Not inherited.
    pub scroll_padding: crate::layout::Sides<crate::layout::ScrollPadding>,
    /// `scroll-margin-*` (CSS Scroll Snap 1 §4.2), the box's scroll snap
    /// area's outsets, in cells. Not inherited.
    pub scroll_margin: crate::layout::Sides<i16>,
    /// `scroll-snap-type` / `-align` / `-stop` (CSS Scroll Snap 1 §5–§6).
    /// Not inherited.
    pub scroll_snap_type: crate::layout::ScrollSnapType,
    pub scroll_snap_align: crate::layout::ScrollSnapAlign,
    pub scroll_snap_stop: crate::layout::ScrollSnapStop,
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
    /// Whether `display` is `-webkit-box` / `-webkit-inline-box`
    /// (Compat Standard §5; `TuiStyle::webkit_box`). Not inherited;
    /// default `false`.
    pub webkit_box: bool,
    /// True when this element establishes a new **block formatting
    /// context** per CSS 2.1 §9.4.1. Triggers: root element, flex
    /// containers, inline-blocks, absolute/fixed positioning,
    /// non-visible overflow on either axis. Margin collapsing
    /// crosses a parent-child boundary only when the parent does
    /// NOT establish a new BFC. Computed at cascade finalization
    /// (last pass over the property bag).
    pub establishes_new_bfc: bool,
    /// The CSS Text properties — white-space processing, wrapping —
    /// which all inherit (CSS Text 3 / 4).
    pub text: crate::layout::TextStyle,
    /// The font properties (CSS Fonts 4), the weight computed to a
    /// number. All inherit; `modifiers` carries the bold and italic they
    /// draw.
    pub font: crate::layout::Font,
    /// `vertical-align` (CSS 2.1 §10.8.1), a percentage resolved against
    /// the line height. Not inherited; initial `baseline`.
    pub vertical_align: crate::layout::VerticalAlign,
    /// The `text-decoration` longhands (CSS Text Decoration 4 §2), the
    /// color resolved. Not inherited.
    pub text_decoration: crate::layout::TextDecorations,
    /// The decorations drawn on the element's text: its own and those its
    /// ancestors propagate to it (§2.1) — what paint reads. Derived by
    /// the cascade; not a property.
    pub applied_decorations: crate::layout::AppliedDecorations,
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
    /// `float` (CSS 2.1 §9.5.1): `none` on an absolutely positioned box
    /// (§9.7). Not inherited.
    pub float: crate::layout::Float,
    /// `clear` (CSS 2.1 §9.5.2). Not inherited.
    pub clear: crate::layout::Clear,

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
    /// `flex-direction` as one value (CSS Flexbox §5.1): the axis
    /// ([`direction`](Self::direction)) and [`flex_reverse`](Self::flex_reverse).
    pub fn flex_direction(&self) -> crate::layout::FlexDirection {
        crate::layout::FlexDirection::new(self.direction, self.flex_reverse)
    }

    /// Whether the box is a scroll container (CSS Overflow 3 §3.1):
    /// `overflow` `hidden`, `scroll` or `auto` on an axis. A `clip` box
    /// clips without being one.
    pub fn is_scroll_container(&self) -> bool {
        self.overflow_x.is_scrollable() || self.overflow_y.is_scrollable()
    }

    /// Whether the box clips its content on either axis — a scroll
    /// container or an `overflow: clip` axis.
    pub fn clips_overflow(&self) -> bool {
        self.overflow_x.clips() || self.overflow_y.clips()
    }

    /// CSS Overflow 3 §3.1's computed value: beside an axis that makes a
    /// scroll container, `visible` computes to `auto` and `clip` to
    /// `hidden`; otherwise both stay as specified.
    #[deny(clippy::wildcard_enum_match_arm)]
    pub fn normalize_overflow(&mut self) {
        use crate::layout::Overflow;
        if !self.is_scroll_container() {
            return;
        }
        for axis in [&mut self.overflow_x, &mut self.overflow_y] {
            *axis = match *axis {
                Overflow::Visible => Overflow::Auto,
                Overflow::Clip => Overflow::Hidden,
                other @ (Overflow::Hidden | Overflow::Scroll | Overflow::Auto) => other,
            };
        }
    }

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
            grid_template_columns: crate::layout::GridTemplate::None,
            grid_template_rows: crate::layout::GridTemplate::None,
            grid_template_areas: crate::layout::GridTemplateAreas::NONE,
            grid_auto_columns: std::borrow::Cow::Borrowed(crate::layout::TrackSize::AUTO_LIST),
            grid_auto_rows: std::borrow::Cow::Borrowed(crate::layout::TrackSize::AUTO_LIST),
            grid_auto_flow: crate::layout::GridAutoFlow::ROW,
            grid_row_start: crate::layout::GridLine::Auto,
            grid_row_end: crate::layout::GridLine::Auto,
            grid_column_start: crate::layout::GridLine::Auto,
            grid_column_end: crate::layout::GridLine::Auto,
            border: Border::none(),
            border_style: Border::none(),
            border_width: crate::layout::Sides::default(),
            border_radius: crate::layout::Corners::default(),
            box_shadow: Vec::new(),
            border_spacing: crate::layout::BorderSpacing::default(),
            border_collapse: crate::layout::BorderCollapse::Separate,
            border_collapse_declared: false,
            direction: Direction::Row,
            flex_reverse: false,
            flex_wrap: crate::layout::FlexWrap::NoWrap,
            justify_content: crate::layout::Alignment::NORMAL,
            align_items: crate::layout::Alignment::NORMAL,
            align_content: crate::layout::Alignment::NORMAL,
            align_self: crate::layout::Alignment::AUTO,
            justify_items: crate::layout::Alignment::LEGACY,
            justify_self: crate::layout::Alignment::AUTO,
            text_direction: crate::layout::TextDirection::Ltr,
            writing_mode: crate::layout::WritingMode::HorizontalTb,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,
            overflow_clip_margin: crate::layout::OverflowClipMargin::default(),
            text_overflow: crate::layout::TextOverflow::default(),
            max_lines: None,
            block_ellipsis: crate::layout::BlockEllipsis::NoEllipsis,
            continue_: crate::layout::Continue::Auto,
            webkit_box_orient: crate::layout::BoxOrient::InlineAxis,
            line_clamp_container: false,
            scrollbar_gutter: crate::layout::ScrollbarGutter::Auto,
            scrollbar_width: crate::layout::ScrollbarWidth::Auto,
            scrollbar_color: crate::layout::ScrollbarColor::Auto,
            overscroll_behavior_x: crate::layout::OverscrollBehavior::Auto,
            overscroll_behavior_y: crate::layout::OverscrollBehavior::Auto,
            scroll_padding: crate::layout::Sides::new(
                crate::layout::ScrollPadding::Auto,
                crate::layout::ScrollPadding::Auto,
                crate::layout::ScrollPadding::Auto,
                crate::layout::ScrollPadding::Auto,
            ),
            scroll_margin: crate::layout::Sides::new(0, 0, 0, 0),
            scroll_snap_type: crate::layout::ScrollSnapType::None,
            scroll_snap_align: crate::layout::ScrollSnapAlign {
                block: crate::layout::SnapAlign::None,
                inline: crate::layout::SnapAlign::None,
            },
            scroll_snap_stop: crate::layout::ScrollSnapStop::Normal,
            scroll_behavior: crate::layout::ScrollBehavior::Auto,
            display: Display::Block,
            flow: crate::layout::Flow::Block,
            list_item: false,
            webkit_box: false,
            establishes_new_bfc: false,
            text: crate::layout::TextStyle::default(),
            font: crate::layout::Font {
                weight: crate::layout::FontWeight::Number(400.0),
                ..crate::layout::Font::default()
            },
            vertical_align: crate::layout::VerticalAlign::Baseline,
            text_decoration: crate::layout::TextDecorations {
                line: crate::layout::TextDecorationLine::NONE,
                style: crate::layout::TextDecorationStyle::Solid,
                color: Color::Reset,
                thickness: crate::layout::TextDecorationThickness::Auto,
            },
            applied_decorations: crate::layout::AppliedDecorations::NONE,
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
            float: crate::layout::Float::None,
            clear: crate::layout::Clear::None,
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

#[cfg(test)]
#[path = "computed_tests.rs"]
mod tests;
