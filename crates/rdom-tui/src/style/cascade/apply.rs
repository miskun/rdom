//! Per-property applicators: one declaration block onto the working
//! `ComputedStyle`, for one ladder pass (the ladder itself is
//! `ladder.rs`).
//!
//! Applicators handle the `Value<T>` variants: `Specified`, and the
//! CSS-wide keywords `inherit` / `initial` / `revert` / `revert-layer`,
//! each of which
//! takes the field from a whole computed style ([`Keywords::resolve`]):
//! the parent's, `ComputedStyle::initial()` (via `Initials`, the table
//! every element's cascade starts from), or the ladder's rollback
//! state. Most properties share one generic path (`apply_value`). They
//! also honor the `important_pass` / `important_prop` pairing so normal
//! and important declarations apply in separate passes.

pub(super) use super::colors::ElementColors;
use super::colors::apply_colors;
use super::decoration::apply_decoration;
use crate::layout::Display;
use crate::style::{ComputedStyle, ImportantMask, TuiStyle, Value};

/// Where the CSS-wide keywords of one ladder pass take their values
/// from.
pub(super) struct Keywords<'a> {
    /// `inherit`: the parent's computed style (CSS Cascade 4 §7.2).
    pub parent: &'a ComputedStyle,
    /// The document's preferred color scheme (CSS Color Adjust 1
    /// §2.1), which the colors cascaded so far resolve under.
    pub preferred_scheme: rdom_style::color::ColorScheme,
    /// `initial`: the initial values (§7.1).
    pub initial: &'a Initials,
    /// `revert`: the cascade rolled back to the previous origin
    /// (§7.3), computed on first use.
    pub revert: &'a dyn Fn() -> &'a ComputedStyle,
    /// `revert-layer`: the cascade rolled back to the previous cascade
    /// layer (Cascade 5 §7.4), computed on first use.
    pub revert_layer: &'a dyn Fn() -> &'a ComputedStyle,
}

/// A declared value, resolved for one pass: the specified value, or
/// the computed style a CSS-wide keyword copies the field from.
pub(super) enum Resolved<'v, 'a, T> {
    Specified(&'v T),
    From(&'a ComputedStyle),
}

impl<'a> Keywords<'a> {
    pub(super) fn resolve<'v, T>(&self, value: &'v Value<T>) -> Resolved<'v, 'a, T> {
        match value {
            Value::Specified(x) => Resolved::Specified(x),
            Value::Inherit => Resolved::From(self.parent),
            Value::Initial => Resolved::From(self.initial.get()),
            Value::Revert => Resolved::From((self.revert)()),
            Value::RevertLayer => Resolved::From((self.revert_layer)()),
        }
    }
}

/// Compute `establishes_new_bfc` from the working style + parent
/// context. Runs after the cascade ladder so all source properties
/// are at their final values. Per CSS 2.1 §9.4.1 + Flexbox §3:
///
/// An element establishes a new block formatting context when:
/// - It's a flex or grid container (`flow: Flex` / `Grid`) — they form
///   independent formatting contexts for their items (CSS Flexbox §3,
///   CSS Grid 2 §5.1).
/// - It's an inline-block — establishes a new BFC for its content
///   (which then lays out as block).
/// - It is a scroll container (`overflow` `hidden` / `scroll` / `auto`
///   on an axis). `overflow: clip` is not one and forms no BFC (CSS
///   Overflow 3 §3.1).
/// - It's absolutely or fixed positioned — out-of-flow boxes form
///   their own BFCs.
/// - (Root element is also a BFC — handled implicitly because
///   layout starts at root regardless.)
///
/// Margin collapsing checks this predicate: parent-child margin
/// collapse happens only when the parent does NOT establish a new
/// BFC.
pub(super) fn finalize_bfc_formation(working: &mut ComputedStyle) {
    use crate::layout::{Flow, Position};
    working.establishes_new_bfc = matches!(working.flow, Flow::Flex | Flow::Grid | Flow::FlowRoot | Flow::Table)
        || matches!(working.display, Display::InlineBlock)
        || working.is_scroll_container()
        || matches!(working.position, Position::Absolute | Position::Fixed)
        // CSS 2.1 §9.4.1: floats establish a new block formatting context.
        || working.float != crate::layout::Float::None
        // CSS Box Alignment 3 §5.1: a block container whose
        // `align-content` is not `normal` is an independent formatting
        // context.
        || (working.flow.is_block_flow()
            && working.align_content.keyword != crate::layout::Align::Normal)
        // CSS Containment 2 §3.2, §3.4: layout and paint containment make
        // an independent formatting context.
        || crate::style::containment::layout(working)
        || crate::style::containment::paint(working);
}

/// CSS Box Alignment 3 §6.2: `justify-items: legacy` (its initial value)
/// computes to the parent's value when that is `legacy` with a side
/// (`legacy center`, …), and to `normal` otherwise — so a `legacy` value
/// reaches the descendants that do not set `justify-items`.
pub(super) fn finalize_justify_items(working: &mut ComputedStyle, parent: &ComputedStyle) {
    use crate::layout::{Align, Alignment};
    if working.justify_items == Alignment::LEGACY {
        working.justify_items =
            if parent.justify_items.legacy && parent.justify_items.keyword != Align::Normal {
                parent.justify_items
            } else {
                Alignment::NORMAL
            };
    }
}

/// CSS Display 3 Appendix B: `display: contents` on a replaced element
/// or a form control — whose children are not its rendering — behaves
/// as `display: none`. rdom computes it so, so layout, paint, hit
/// testing and focus all see no box.
pub(super) fn finalize_unusual_contents(working: &mut ComputedStyle, tag: Option<&str>) {
    const NO_CONTENTS: &[&str] = &[
        "br", "wbr", "meter", "progress", "canvas", "embed", "object", "audio", "iframe", "img",
        "video", "frame", "frameset", "input", "textarea", "select",
    ];
    if working.display == Display::Contents && tag.is_some_and(|t| NO_CONTENTS.contains(&t)) {
        working.display = Display::None;
    }
}

/// Apply one `TuiStyle` to `working`, for one ladder pass. Paints +
/// layout + display + text all in one pass.
pub(super) fn apply_style(
    working: &mut ComputedStyle,
    colors: &mut ElementColors,
    style: &TuiStyle,
    important_pass: bool,
    kw: &Keywords<'_>,
) {
    // One declared value → one `ComputedStyle` field of the same type:
    // specified as written, `inherit` the parent's field, `initial` the
    // field of `ComputedStyle::initial()`.
    macro_rules! value {
        ($($($field:ident).+: $mask:ident),* $(,)?) => {$(
            apply_value(
                &mut working.$($field).+,
                &style.$($field).+,
                style.important.contains(ImportantMask::$mask),
                important_pass,
                kw,
                |c| &c.$($field).+,
            );
        )*};
    }

    // Paint properties (`colors.rs`, `decoration.rs`).
    apply_colors(working, colors, style, important_pass, kw);
    apply_decoration(working, style, important_pass, kw);

    // The font properties (`font.rs`).
    super::font::apply_font(working, style, important_pass, kw);
    // The `text-decoration` longhands (`text_decoration.rs`; the color
    // with the other colors).
    super::text_decoration::apply_text_decoration(working, style, important_pass, kw);
    apply_opacity(
        working,
        &style.opacity,
        style.important.contains(ImportantMask::OPACITY),
        important_pass,
        kw,
    );

    // Layout properties.
    value!(width: WIDTH, height: HEIGHT);
    value!(min_width: MIN_WIDTH, min_height: MIN_HEIGHT);
    // `max-*`: the declared value is the computed `Option` itself
    // (`none` is `None`).
    value!(max_width: MAX_WIDTH, max_height: MAX_HEIGHT);
    value!(box_sizing: BOX_SIZING);
    value!(interpolate_size: INTERPOLATE_SIZE);
    value!(
        contain_intrinsic_width: CONTAIN_INTRINSIC_WIDTH,
        contain_intrinsic_height: CONTAIN_INTRINSIC_HEIGHT,
    );
    // CSS Conditional 5 §6.1–§6.2, CSS Containment 2 §2, CSS Will Change
    // 1 §2; not inherited.
    value!(
        container_type: CONTAINER_TYPE,
        container_name: CONTAINER_NAME,
        contain: CONTAIN,
        will_change: WILL_CHANGE,
        content_visibility: CONTENT_VISIBILITY,
    );
    // `aspect-ratio`: the declared value is the computed `Option` itself
    // (`auto` alone is `None`).
    value!(aspect_ratio: ASPECT_RATIO);
    // The `padding-*` / `margin-*` longhands, one declaration per side
    // (CSS Box 3 §3.2 / §4.2).
    value!(
        padding.top: PADDING_TOP,
        padding.right: PADDING_RIGHT,
        padding.bottom: PADDING_BOTTOM,
        padding.left: PADDING_LEFT,
        margin.top: MARGIN_TOP,
        margin.right: MARGIN_RIGHT,
        margin.bottom: MARGIN_BOTTOM,
        margin.left: MARGIN_LEFT,
    );
    value!(
        margin_trim: MARGIN_TRIM,
        row_gap: ROW_GAP,
        column_gap: COLUMN_GAP,
        flex_grow: FLEX_GROW,
        flex_shrink: FLEX_SHRINK,
        flex_basis: FLEX_BASIS,
        order: ORDER,
        grid_template_areas: GRID_TEMPLATE_AREAS,
        grid_auto_flow: GRID_AUTO_FLOW,
    );
    apply_grid(working, style, important_pass, kw);
    super::text::apply_text(working, style, important_pass, kw);
    apply_border_collapse(
        &mut working.border_collapse,
        &mut working.border_collapse_declared,
        &style.border_collapse,
        style.important.contains(ImportantMask::BORDER_COLLAPSE),
        important_pass,
        kw,
    );
    value!(
        direction: FLEX_DIRECTION,
        flex_reverse: FLEX_REVERSE,
        flex_wrap: FLEX_WRAP,
        justify_content: JUSTIFY_CONTENT,
        align_items: ALIGN_ITEMS,
        align_content: ALIGN_CONTENT,
        justify_items: JUSTIFY_ITEMS,
        justify_self: JUSTIFY_SELF,
        align_self: ALIGN_SELF,
        text_direction: TEXT_DIRECTION,
        writing_mode: WRITING_MODE,
        overflow_x: OVERFLOW_X,
        overflow_y: OVERFLOW_Y,
        overflow_clip_margin: OVERFLOW_CLIP_MARGIN,
        text_overflow: TEXT_OVERFLOW,
        max_lines: MAX_LINES,
        block_ellipsis: BLOCK_ELLIPSIS,
        continue_: CONTINUE,
        webkit_box_orient: WEBKIT_BOX_ORIENT,
        scrollbar_gutter: SCROLLBAR_GUTTER,
        scrollbar_width: SCROLLBAR_WIDTH,
        scrollbar_color: SCROLLBAR_COLOR,
        overscroll_behavior_x: OVERSCROLL_BEHAVIOR_X,
        overscroll_behavior_y: OVERSCROLL_BEHAVIOR_Y,
        scroll_padding.top: SCROLL_PADDING_TOP,
        scroll_padding.right: SCROLL_PADDING_RIGHT,
        scroll_padding.bottom: SCROLL_PADDING_BOTTOM,
        scroll_padding.left: SCROLL_PADDING_LEFT,
        scroll_margin.top: SCROLL_MARGIN_TOP,
        scroll_margin.right: SCROLL_MARGIN_RIGHT,
        scroll_margin.bottom: SCROLL_MARGIN_BOTTOM,
        scroll_margin.left: SCROLL_MARGIN_LEFT,
        scroll_snap_type: SCROLL_SNAP_TYPE,
        scroll_snap_align: SCROLL_SNAP_ALIGN,
        scroll_snap_stop: SCROLL_SNAP_STOP,
        scroll_behavior: SCROLL_BEHAVIOR,
        // `display` owns both halves: `display: inherit` takes the
        // parent's outer and inner display.
        display: DISPLAY,
        flow: FLOW,
        list_item: LIST_ITEM,
        webkit_box: WEBKIT_BOX,
        user_select: USER_SELECT,
        pointer_events: POINTER_EVENTS,
        quotes: QUOTES,
        list_style_type: LIST_STYLE_TYPE,
        list_style_position: LIST_STYLE_POSITION,
        list_style_image: LIST_STYLE_IMAGE,
        marker_side: MARKER_SIDE,
        visibility: VISIBILITY,
        caret_color: CARET_COLOR,
        caret_text_color: CARET_TEXT_COLOR,
        // CSS UI 4 §5: the outline; none inherit.
        ui.outline_style: OUTLINE_STYLE,
        ui.outline_width: OUTLINE_WIDTH,
        ui.outline_color: OUTLINE_COLOR,
        ui.outline_offset: OUTLINE_OFFSET,
        // CSS UI 4 §4.1; inherits.
        ui.cursor: CURSOR,
        // §6.2; inherit.
        ui.caret_shape: CARET_SHAPE,
        ui.caret_animation: CARET_ANIMATION,
        // §6.3; inherits.
        ui.accent_color: ACCENT_COLOR,
        // §7.1; not inherited.
        ui.appearance: APPEARANCE,
        // §7.2; not inherited.
        ui.field_sizing: FIELD_SIZING,
        // §4.2; not inherited.
        ui.resize: RESIZE,
        // CSS 2.1 §17.5.2: not inherited; §17.4.1: inherits.
        table.table_layout: TABLE_LAYOUT,
        table.caption_side: CAPTION_SIDE,
        table.empty_cells: EMPTY_CELLS,
        // CSS Transforms 1 §5–§7, Transforms 2 §6; none inherit.
        effects.translate: TRANSLATE,
        effects.rotate: ROTATE,
        effects.scale: SCALE,
        effects.transform: TRANSFORM,
        effects.transform_origin: TRANSFORM_ORIGIN,
        effects.transform_box: TRANSFORM_BOX,
        // Compositing and Blending 1 §3.2, §3.4, §5.2; none inherit.
        effects.mix_blend_mode: MIX_BLEND_MODE,
        effects.isolation: ISOLATION,
        effects.background_blend_mode: BACKGROUND_BLEND_MODE,
        // CSS Masking 1 §5.1; not inherited.
        effects.clip_path: CLIP_PATH,
    );
    // Positioning (M2), transitions (M3; latest list wins), animations
    // (CSS Animations 1 §4) and counters
    // (CSS Lists 3 §3.1). None inherit by default.
    value!(
        position: POSITION,
        top: TOP,
        right: RIGHT,
        bottom: BOTTOM,
        left: LEFT,
        z_index: Z_INDEX,
        overlay: OVERLAY,
        float: FLOAT,
        clear: CLEAR,
        transition_property: TRANSITION_PROPERTY,
        transition_duration: TRANSITION_DURATION,
        transition_timing_function: TRANSITION_TIMING_FUNCTION,
        transition_delay: TRANSITION_DELAY,
        transition_behavior: TRANSITION_BEHAVIOR,
        animation_name: ANIMATION_NAME,
        animation_duration: ANIMATION_DURATION,
        animation_timing_function: ANIMATION_TIMING_FUNCTION,
        animation_delay: ANIMATION_DELAY,
        animation_iteration_count: ANIMATION_ITERATION_COUNT,
        animation_direction: ANIMATION_DIRECTION,
        animation_fill_mode: ANIMATION_FILL_MODE,
        animation_play_state: ANIMATION_PLAY_STATE,
        animation_composition: ANIMATION_COMPOSITION,
        animation_timeline: ANIMATION_TIMELINE,
        scroll_timeline_name: SCROLL_TIMELINE_NAME,
        scroll_timeline_axis: SCROLL_TIMELINE_AXIS,
        view_timeline_name: VIEW_TIMELINE_NAME,
        view_timeline_axis: VIEW_TIMELINE_AXIS,
        view_timeline_inset: VIEW_TIMELINE_INSET,
        timeline_scope: TIMELINE_SCOPE,
        animation_range_start: ANIMATION_RANGE_START,
        animation_range_end: ANIMATION_RANGE_END,
        counter_reset: COUNTER_RESET,
        counter_increment: COUNTER_INCREMENT,
        counter_set: COUNTER_SET,
        // Inherits; `light-dark()` picks by it (CSS Color Adjust 1 §2).
        color_scheme: COLOR_SCHEME,
        // Inherits (CSS 2.1 §17.6.1); laid out with C13-TFC.
        border_spacing: BORDER_SPACING,
    );
}

// ─── Applicators ────────────────────────────────────────────────────

/// Should this declaration actually apply during the current pass?
/// Normal pass applies normal declarations; important pass applies
/// important ones.
#[inline]
pub(super) fn matches_pass(important_prop: bool, important_pass: bool) -> bool {
    important_prop == important_pass
}

/// The initial values `initial` resolves to: `ComputedStyle::initial()`,
/// the same table every element's cascade starts from, so the two
/// cannot drift (`P6G-APPLY-INITIALS-1`). Built on the first `initial`
/// keyword an element's cascade meets; most elements never build it.
#[derive(Default)]
pub(super) struct Initials(std::cell::OnceCell<ComputedStyle>);

impl Initials {
    pub(super) fn get(&self) -> &ComputedStyle {
        self.0.get_or_init(ComputedStyle::initial)
    }
}

/// The CSS-wide keyword resolution every property shares: specified
/// as written, a keyword the `field` of its source style
/// ([`Keywords::resolve`]) — `inherit` the parent's computed value
/// (inherited property or not — CSS Cascade 4 §7.2), `initial` the
/// property's initial value, `revert` the rolled-back cascade's.
/// The grid longhands whose `TuiStyle` value a consumer can write
/// outside the grammar (the fields are public; the builders check):
/// such a declaration is ignored, as a CSS parser drops an invalid one
/// (CSS Syntax 3 §8.1), so layout reads only valid values
/// (C7G-TRACK-VALIDITY). A declared `grid-auto-*` list becomes the
/// computed one, owned; the initial `auto` stays borrowed
/// (C7G-INITIAL-ALLOC).
fn apply_grid(
    working: &mut ComputedStyle,
    style: &TuiStyle,
    important_pass: bool,
    kw: &Keywords<'_>,
) {
    use crate::layout::{GridLine, GridTemplate, TrackSize};
    let template = |v: &GridTemplate| v.is_valid().then(|| v.clone());
    let auto = |v: &Vec<TrackSize>| {
        TrackSize::is_valid_list(v).then(|| std::borrow::Cow::Owned(v.clone()))
    };
    let line = |v: &GridLine| v.is_valid().then(|| v.clone());
    let important = |mask| style.important.contains(mask);
    macro_rules! grid {
        ($($field:ident: $mask:ident => $to:expr),* $(,)?) => {$(
            apply_converted(
                &mut working.$field,
                &style.$field,
                important(ImportantMask::$mask),
                important_pass,
                kw,
                |c| &c.$field,
                $to,
            );
        )*};
    }
    grid!(
        grid_template_columns: GRID_TEMPLATE_COLUMNS => template,
        grid_template_rows: GRID_TEMPLATE_ROWS => template,
        grid_auto_columns: GRID_AUTO_COLUMNS => auto,
        grid_auto_rows: GRID_AUTO_ROWS => auto,
        grid_row_start: GRID_ROW_START => line,
        grid_row_end: GRID_ROW_END => line,
        grid_column_start: GRID_COLUMN_START => line,
        grid_column_end: GRID_COLUMN_END => line,
    );
}

pub(super) fn apply_value<T: Clone>(
    target: &mut T,
    value: &Option<Value<T>>,
    important_prop: bool,
    important_pass: bool,
    kw: &Keywords<'_>,
    field: fn(&ComputedStyle) -> &T,
) {
    apply_converted(
        target,
        value,
        important_prop,
        important_pass,
        kw,
        field,
        |v| Some(v.clone()),
    );
}

/// [`apply_value`] for a property whose declared form `S` differs from
/// its computed one `T`, or may lie outside its grammar: `to` computes a
/// declared value, `None` ignoring the declaration (the value stays).
fn apply_converted<S, T: Clone>(
    target: &mut T,
    value: &Option<Value<S>>,
    important_prop: bool,
    important_pass: bool,
    kw: &Keywords<'_>,
    field: fn(&ComputedStyle) -> &T,
    to: impl Fn(&S) -> Option<T>,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        match kw.resolve(v) {
            Resolved::Specified(x) => {
                if let Some(x) = to(x) {
                    *target = x;
                }
            }
            Resolved::From(source) => *target = field(source).clone(),
        }
    }
}

fn apply_border_collapse(
    target: &mut crate::layout::BorderCollapse,
    declared: &mut bool,
    value: &Option<Value<crate::layout::BorderCollapse>>,
    important_prop: bool,
    important_pass: bool,
    kw: &Keywords<'_>,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        match v {
            Value::Specified(x) => {
                *target = *x;
                // Author wrote `border-collapse: collapse | separate;`
                // on THIS element — it becomes a collapse-root (the
                // boundary of its own group, equivalent to a CSS
                // `<table>`). See `ComputedStyle::border_collapse_declared`.
                *declared = true;
            }
            Value::Inherit => {
                // Explicit `inherit` keyword: semantically "use my
                // parent's value." Not a collapse-root declaration —
                // leave `declared` as-is (initial: false; never set
                // by inheritance).
                *target = kw.parent.border_collapse;
            }
            Value::Initial => {
                *target = kw.initial.get().border_collapse;
                // `initial` resets to the property's initial value
                // (`separate`); the author IS declaring something on
                // this element, so it's a (trivial) collapse-root.
                *declared = true;
            }
            Value::Revert | Value::RevertLayer => {
                // Whatever the rolled-back cascade had, collapse-root
                // flag included.
                let source = if matches!(v, Value::Revert) {
                    (kw.revert)()
                } else {
                    (kw.revert_layer)()
                };
                *target = source.border_collapse;
                *declared = source.border_collapse_declared;
            }
        }
    }
}

/// Apply CSS `opacity` to the working `ComputedStyle`. Does not
/// inherit by default, but an explicit `inherit` takes the parent's
/// computed opacity. Clamped to `[0.0, 1.0]` defensively at cascade
/// time even though the `.opacity(f)` setter already clamps.
fn apply_opacity(
    working: &mut ComputedStyle,
    value: &Option<Value<f32>>,
    important_prop: bool,
    important_pass: bool,
    kw: &Keywords<'_>,
) {
    if let Some(v) = value
        && matches_pass(important_prop, important_pass)
    {
        working.opacity = match kw.resolve(v) {
            Resolved::Specified(v) => v.clamp(0.0, 1.0),
            Resolved::From(source) => source.opacity,
        };
    }
}
