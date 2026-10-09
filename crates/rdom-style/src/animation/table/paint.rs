//! The first half of the longhand table (`table`): the colors, fonts,
//! decorations, display, UI, overflow, sizing, effects, alignment,
//! multi-column, fragmentation and anchor longhands.

use super::super::AnimationType::{ByComputedValue, Discrete, NotAnimatable, RepeatableList};
use super::super::entry::{
    CONTENT_VISIBILITY, DISPLAY, Entry, Ops, Role, VISIBILITY, e, fix_decorations, fix_flex,
    fix_font, fix_opacity, size, steps, value,
};
use super::super::value::{blend, discrete, interpolable};

/// The longhands up to the grid ones, in the dispatch table's order.
pub(super) const PAINT: &[Entry] = &[
    // CSS Color 4 / Backgrounds 3
    e("color", ByComputedValue, value!(fg)),
    Entry {
        role: Role::Background,
        ..e("background-color", ByComputedValue, value!(bg))
    },
    e("background-image", Discrete, None),
    e("background-position", RepeatableList, None),
    e("background-size", RepeatableList, None),
    e("background-repeat", Discrete, None),
    e("background-attachment", Discrete, None),
    e("background-origin", RepeatableList, None),
    e("background-clip", RepeatableList, steps!(background_clip)),
    // CSS Fonts 4
    e(
        "font-weight",
        ByComputedValue,
        value!(font.weight => fix_font),
    ),
    e(
        "font-style",
        ByComputedValue,
        value!(font.style => fix_font),
    ),
    e("font-size", ByComputedValue, value!(font.size)),
    e("font-family", Discrete, steps!(font.family)),
    e("font-stretch", ByComputedValue, value!(font.stretch)),
    e("font-variant", Discrete, steps!(font.variant)),
    // CSS Text Decoration 4
    e(
        "text-decoration-line",
        Discrete,
        steps!(text_decoration.line, applied_decorations),
    ),
    e(
        "text-decoration-style",
        Discrete,
        steps!(text_decoration.style => fix_decorations),
    ),
    e(
        "text-decoration-color",
        ByComputedValue,
        value!(text_decoration.color => fix_decorations),
    ),
    e(
        "text-decoration-thickness",
        ByComputedValue,
        value!(text_decoration.thickness),
    ),
    e(
        "text-underline-offset",
        ByComputedValue,
        value!(text.text_underline_offset),
    ),
    e(
        "text-underline-position",
        Discrete,
        steps!(text.text_underline_position),
    ),
    e(
        "text-decoration-skip-ink",
        Discrete,
        steps!(text.text_decoration_skip_ink),
    ),
    // CSS Color 4 §13
    e("opacity", ByComputedValue, value!(opacity => fix_opacity)),
    // CSS Display 3 / Flexbox / Box Alignment
    e("display", Discrete, DISPLAY),
    e("flex-direction", Discrete, steps!(direction, flex_reverse)),
    e("flex-wrap", Discrete, steps!(flex_wrap)),
    e("justify-content", Discrete, steps!(justify_content)),
    e("align-content", Discrete, steps!(align_content)),
    e("align-items", Discrete, steps!(align_items)),
    e("align-self", Discrete, steps!(align_self)),
    e("justify-items", Discrete, steps!(justify_items)),
    e("justify-self", Discrete, steps!(justify_self)),
    // CSS UI 4 / Display 3
    e("user-select", Discrete, steps!(user_select)),
    e("pointer-events", Discrete, steps!(pointer_events)),
    e("visibility", Discrete, VISIBILITY),
    e("caret-color", ByComputedValue, value!(caret_color)),
    e(
        "caret-text-color",
        ByComputedValue,
        value!(caret_text_color),
    ),
    // CSS UI 4 §6.2
    e("caret-shape", Discrete, steps!(ui.caret_shape)),
    e("caret-animation", Discrete, steps!(ui.caret_animation)),
    // CSS UI 4 §6.3
    e("accent-color", ByComputedValue, value!(ui.accent_color)),
    // CSS UI 4 §7.1
    e("appearance", Discrete, steps!(ui.appearance)),
    // §7.2
    e("field-sizing", Discrete, steps!(ui.field_sizing)),
    // §4.2
    e("resize", Discrete, steps!(ui.resize)),
    // CSS 2.1 §17.5.2, §17.4.1
    e("table-layout", Discrete, steps!(table.table_layout)),
    e("caption-side", Discrete, steps!(table.caption_side)),
    e("empty-cells", Discrete, steps!(table.empty_cells)),
    // CSS Overflow 3 / 4, Scrollbars 1, Overscroll 1, Scroll Snap 1
    e(
        "overflow-x",
        Discrete,
        steps!(overflow_x, establishes_new_bfc),
    ),
    e(
        "overflow-y",
        Discrete,
        steps!(overflow_y, establishes_new_bfc),
    ),
    e(
        "overflow-clip-margin",
        ByComputedValue,
        value!(overflow_clip_margin),
    ),
    e("text-overflow", Discrete, steps!(text_overflow)),
    e("max-lines", ByComputedValue, value!(max_lines)),
    e("block-ellipsis", Discrete, steps!(block_ellipsis)),
    e(
        "continue",
        Discrete,
        steps!(continue_, line_clamp_container, flow),
    ),
    e("-webkit-box-orient", Discrete, steps!(webkit_box_orient)),
    e("scrollbar-gutter", Discrete, steps!(scrollbar_gutter)),
    e("scrollbar-width", Discrete, steps!(scrollbar_width)),
    e("scrollbar-color", ByComputedValue, value!(scrollbar_color)),
    // CSS UI 4 §5
    e("outline-style", Discrete, steps!(ui.outline_style)),
    e("outline-width", ByComputedValue, value!(ui.outline_width)),
    e("outline-color", ByComputedValue, value!(ui.outline_color)),
    e("outline-offset", ByComputedValue, value!(ui.outline_offset)),
    e("cursor", Discrete, steps!(ui.cursor)),
    e(
        "overscroll-behavior-x",
        Discrete,
        steps!(overscroll_behavior_x),
    ),
    e(
        "overscroll-behavior-y",
        Discrete,
        steps!(overscroll_behavior_y),
    ),
    e(
        "scroll-padding-top",
        ByComputedValue,
        value!(scroll_padding.top),
    ),
    e(
        "scroll-padding-right",
        ByComputedValue,
        value!(scroll_padding.right),
    ),
    e(
        "scroll-padding-bottom",
        ByComputedValue,
        value!(scroll_padding.bottom),
    ),
    e(
        "scroll-padding-left",
        ByComputedValue,
        value!(scroll_padding.left),
    ),
    e(
        "scroll-margin-top",
        ByComputedValue,
        value!(scroll_margin.top),
    ),
    e(
        "scroll-margin-right",
        ByComputedValue,
        value!(scroll_margin.right),
    ),
    e(
        "scroll-margin-bottom",
        ByComputedValue,
        value!(scroll_margin.bottom),
    ),
    e(
        "scroll-margin-left",
        ByComputedValue,
        value!(scroll_margin.left),
    ),
    e("scroll-snap-type", Discrete, steps!(scroll_snap_type)),
    e("scroll-snap-align", Discrete, steps!(scroll_snap_align)),
    e("scroll-snap-stop", Discrete, steps!(scroll_snap_stop)),
    e("scroll-behavior", NotAnimatable, None),
    // CSS Sizing 3 / 4
    e("width", ByComputedValue, size!(width)),
    e("height", ByComputedValue, size!(height)),
    e("min-width", ByComputedValue, size!(min_width)),
    e("max-width", ByComputedValue, size!(max_width)),
    e("min-height", ByComputedValue, size!(min_height)),
    e("max-height", ByComputedValue, size!(max_height)),
    e("aspect-ratio", ByComputedValue, value!(aspect_ratio)),
    e("box-sizing", Discrete, steps!(box_sizing)),
    e("interpolate-size", NotAnimatable, None),
    // CSS Containment 2 §2, CSS Will Change 1 §2: not animatable.
    e("contain", NotAnimatable, None),
    e("will-change", NotAnimatable, None),
    // CSS Transforms 2 §6.1–§6.3, §12; Transforms 1 §5–§7
    e("translate", ByComputedValue, value!(effects.translate)),
    e("rotate", ByComputedValue, value!(effects.rotate)),
    e("scale", ByComputedValue, value!(effects.scale)),
    e("transform", ByComputedValue, value!(effects.transform)),
    e(
        "transform-origin",
        ByComputedValue,
        value!(effects.transform_origin),
    ),
    e("transform-box", Discrete, steps!(effects.transform_box)),
    // Filter Effects 1 §5, §14; Filter Effects 2 §3
    e("filter", ByComputedValue, value!(effects.filter)),
    e(
        "backdrop-filter",
        ByComputedValue,
        value!(effects.backdrop_filter),
    ),
    // Compositing and Blending 1 §3.2, §3.4, §5.2: not animatable.
    e("mix-blend-mode", NotAnimatable, None),
    e("isolation", NotAnimatable, None),
    e("background-blend-mode", NotAnimatable, None),
    // CSS Masking 1 §5.1: by computed value (basic shapes of one kind).
    e("clip-path", ByComputedValue, value!(effects.clip_path)),
    // CSS 2.1 §11.1.2, Masking 1 §6.1: by computed value, as a rectangle.
    e("clip", ByComputedValue, value!(effects.clip)),
    // §6–§7: kept as text, no computed value (a cell has no alpha).
    e("mask-image", Discrete, None),
    e("mask-mode", Discrete, None),
    e("mask-repeat", Discrete, None),
    e("mask-position", RepeatableList, None),
    e("mask-clip", Discrete, None),
    e("mask-origin", Discrete, None),
    e("mask-size", RepeatableList, None),
    e("mask-composite", Discrete, None),
    e("mask-type", Discrete, None),
    e("mask-border-source", Discrete, None),
    e("mask-border-slice", ByComputedValue, None),
    e("mask-border-width", ByComputedValue, None),
    e("mask-border-outset", ByComputedValue, None),
    e("mask-border-repeat", Discrete, None),
    e("mask-border-mode", Discrete, None),
    // §4: discrete, `hidden` shown only at its end.
    e("content-visibility", Discrete, CONTENT_VISIBILITY),
    // CSS Conditional 5 §6.1–§6.2: not animatable.
    e("container-type", NotAnimatable, None),
    e("container-name", NotAnimatable, None),
    e(
        "contain-intrinsic-width",
        ByComputedValue,
        value!(contain_intrinsic_width),
    ),
    e(
        "contain-intrinsic-height",
        ByComputedValue,
        value!(contain_intrinsic_height),
    ),
    // CSS Box Alignment 3 §8, Flexbox §7, §5.4
    e("row-gap", ByComputedValue, value!(row_gap)),
    e("column-gap", ByComputedValue, value!(column_gap)),
    // CSS Multi-column 1 §3–§7.
    e(
        "column-count",
        ByComputedValue,
        value!(multicol.column_count),
    ),
    e(
        "column-width",
        ByComputedValue,
        value!(multicol.column_width),
    ),
    e(
        "column-rule-style",
        Discrete,
        steps!(multicol.column_rule_style),
    ),
    e(
        "column-rule-width",
        ByComputedValue,
        value!(multicol.column_rule_width),
    ),
    e(
        "column-rule-color",
        ByComputedValue,
        value!(multicol.column_rule_color),
    ),
    e("column-span", Discrete, steps!(multicol.column_span)),
    e("column-fill", Discrete, steps!(multicol.column_fill)),
    // CSS Fragmentation 3 §3, §5.4.
    e("break-before", Discrete, steps!(fragmentation.break_before)),
    e("break-after", Discrete, steps!(fragmentation.break_after)),
    e("break-inside", Discrete, steps!(fragmentation.break_inside)),
    e("orphans", ByComputedValue, value!(fragmentation.orphans)),
    e("widows", ByComputedValue, value!(fragmentation.widows)),
    e(
        "box-decoration-break",
        Discrete,
        steps!(fragmentation.box_decoration_break),
    ),
    // CSS Anchor Positioning 1 §2–§5: discrete.
    e("anchor-name", Discrete, steps!(anchor.anchor_name)),
    e("anchor-scope", Discrete, steps!(anchor.anchor_scope)),
    e("position-anchor", Discrete, steps!(anchor.position_anchor)),
    e("position-area", Discrete, steps!(anchor.position_area)),
    e(
        "position-try-fallbacks",
        Discrete,
        steps!(anchor.position_try_fallbacks),
    ),
    e(
        "position-try-order",
        Discrete,
        steps!(anchor.position_try_order),
    ),
    e(
        "position-visibility",
        Discrete,
        steps!(anchor.position_visibility),
    ),
    e("flex-grow", ByComputedValue, value!(flex_grow => fix_flex)),
    e(
        "flex-shrink",
        ByComputedValue,
        value!(flex_shrink => fix_flex),
    ),
    e("flex-basis", ByComputedValue, size!(flex_basis)),
    e("order", ByComputedValue, value!(order)),
];
