//! The longhand table: every longhand the dispatch table knows, with
//! its animation type as its specification's property definition states
//! it, and — where rdom computes a value for it — how its computed value
//! animates (`entry.rs`). Companion fields a value carries (`display`'s
//! inner type, `flex-direction`'s reversal) move with it.

use super::AnimationType::{ByComputedValue, Discrete, NotAnimatable, RepeatableList, ShadowList};
use super::entry::{
    DISPLAY, Entry, OVERLAY, Ops, Role, VISIBILITY, e, fix_border, fix_decorations, fix_flex,
    fix_font, fix_opacity, size, steps, value,
};
use super::value::{blend, discrete, interpolable};

/// Every longhand, by the order of the dispatch table's names.
pub(super) static LONGHANDS: &[Entry] = &[
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
    e("flex-grow", ByComputedValue, value!(flex_grow => fix_flex)),
    e(
        "flex-shrink",
        ByComputedValue,
        value!(flex_shrink => fix_flex),
    ),
    e("flex-basis", ByComputedValue, size!(flex_basis)),
    e("order", ByComputedValue, value!(order)),
    // CSS Grid 2
    e(
        "grid-template-columns",
        ByComputedValue,
        value!(grid_template_columns),
    ),
    e(
        "grid-template-rows",
        ByComputedValue,
        value!(grid_template_rows),
    ),
    e("grid-template-areas", Discrete, steps!(grid_template_areas)),
    e(
        "grid-auto-columns",
        ByComputedValue,
        value!(grid_auto_columns),
    ),
    e("grid-auto-rows", ByComputedValue, value!(grid_auto_rows)),
    e("grid-auto-flow", Discrete, steps!(grid_auto_flow)),
    e("grid-row-start", Discrete, steps!(grid_row_start)),
    e("grid-row-end", Discrete, steps!(grid_row_end)),
    e("grid-column-start", Discrete, steps!(grid_column_start)),
    e("grid-column-end", Discrete, steps!(grid_column_end)),
    // CSS Box 4
    e("padding-top", ByComputedValue, value!(padding.top)),
    e("padding-right", ByComputedValue, value!(padding.right)),
    e("padding-bottom", ByComputedValue, value!(padding.bottom)),
    e("padding-left", ByComputedValue, value!(padding.left)),
    e("margin-top", ByComputedValue, value!(margin.top)),
    e("margin-right", ByComputedValue, value!(margin.right)),
    e("margin-bottom", ByComputedValue, value!(margin.bottom)),
    e("margin-left", ByComputedValue, value!(margin.left)),
    e("margin-trim", Discrete, steps!(margin_trim)),
    // CSS Backgrounds 3
    e(
        "border-top-style",
        Discrete,
        steps!(border_style.top => fix_border),
    ),
    e(
        "border-right-style",
        Discrete,
        steps!(border_style.right => fix_border),
    ),
    e(
        "border-bottom-style",
        Discrete,
        steps!(border_style.bottom => fix_border),
    ),
    e(
        "border-left-style",
        Discrete,
        steps!(border_style.left => fix_border),
    ),
    e(
        "border-top-color",
        ByComputedValue,
        value!(border_color.top),
    ),
    e(
        "border-right-color",
        ByComputedValue,
        value!(border_color.right),
    ),
    e(
        "border-bottom-color",
        ByComputedValue,
        value!(border_color.bottom),
    ),
    e(
        "border-left-color",
        ByComputedValue,
        value!(border_color.left),
    ),
    e(
        "border-top-width",
        ByComputedValue,
        value!(border_width.top => fix_border),
    ),
    e(
        "border-right-width",
        ByComputedValue,
        value!(border_width.right => fix_border),
    ),
    e(
        "border-bottom-width",
        ByComputedValue,
        value!(border_width.bottom => fix_border),
    ),
    e(
        "border-left-width",
        ByComputedValue,
        value!(border_width.left => fix_border),
    ),
    e(
        "border-top-left-radius",
        ByComputedValue,
        value!(border_radius.top_left),
    ),
    e(
        "border-top-right-radius",
        ByComputedValue,
        value!(border_radius.top_right),
    ),
    e(
        "border-bottom-right-radius",
        ByComputedValue,
        value!(border_radius.bottom_right),
    ),
    e(
        "border-bottom-left-radius",
        ByComputedValue,
        value!(border_radius.bottom_left),
    ),
    e("box-shadow", ShadowList, value!(box_shadow)),
    // CSS 2.1 §17.6
    e("border-spacing", ByComputedValue, value!(border_spacing)),
    e(
        "border-collapse",
        Discrete,
        steps!(border_collapse, border_collapse_declared),
    ),
    // CSS Generated Content 3, Lists 3
    e(
        "content",
        Discrete,
        steps!(content, content_alt, content_quotes),
    ),
    e("quotes", Discrete, steps!(quotes)),
    e("list-style-type", Discrete, steps!(list_style_type)),
    e("list-style-position", Discrete, steps!(list_style_position)),
    e("list-style-image", Discrete, steps!(list_style_image)),
    e("marker-side", Discrete, steps!(marker_side)),
    // CSS Position 3, CSS 2.1 §9.9
    e("position", Discrete, steps!(position, establishes_new_bfc)),
    e("top", ByComputedValue, value!(top)),
    e("right", ByComputedValue, value!(right)),
    e("bottom", ByComputedValue, value!(bottom)),
    e("left", ByComputedValue, value!(left)),
    e("z-index", ByComputedValue, value!(z_index)),
    e("overlay", Discrete, OVERLAY),
    e("float", Discrete, steps!(float, establishes_new_bfc)),
    e("clear", Discrete, steps!(clear)),
    // CSS Transitions 1 §2
    e("transition-property", NotAnimatable, None),
    e("transition-duration", NotAnimatable, None),
    e("transition-timing-function", NotAnimatable, None),
    e("transition-delay", NotAnimatable, None),
    e("transition-behavior", NotAnimatable, None),
    // CSS Animations 1 §4, CSS Animations 2 §3: not animatable.
    e("animation-name", NotAnimatable, None),
    e("animation-duration", NotAnimatable, None),
    e("animation-timing-function", NotAnimatable, None),
    e("animation-delay", NotAnimatable, None),
    e("animation-iteration-count", NotAnimatable, None),
    e("animation-direction", NotAnimatable, None),
    e("animation-fill-mode", NotAnimatable, None),
    e("animation-play-state", NotAnimatable, None),
    e("animation-composition", NotAnimatable, None),
    e("animation-timeline", NotAnimatable, None),
    // Scroll-driven Animations 1 §2–§4: not animatable but the insets.
    e("scroll-timeline-name", NotAnimatable, None),
    e("scroll-timeline-axis", NotAnimatable, None),
    e("view-timeline-name", NotAnimatable, None),
    e("view-timeline-axis", NotAnimatable, None),
    e(
        "view-timeline-inset",
        ByComputedValue,
        value!(view_timeline_inset),
    ),
    e("timeline-scope", NotAnimatable, None),
    e("animation-range-start", NotAnimatable, None),
    e("animation-range-end", NotAnimatable, None),
    // CSS Lists 3 §4
    e("counter-reset", ByComputedValue, value!(counter_reset)),
    e(
        "counter-increment",
        ByComputedValue,
        value!(counter_increment),
    ),
    e("counter-set", ByComputedValue, value!(counter_set)),
    // CSS Color Adjust 1
    e("color-scheme", Discrete, steps!(color_scheme)),
    // CSS Text 3 / 4
    e(
        "white-space-collapse",
        Discrete,
        steps!(text.white_space_collapse),
    ),
    e("text-wrap-mode", Discrete, steps!(text.text_wrap_mode)),
    e("word-break", Discrete, steps!(text.word_break)),
    e("overflow-wrap", Discrete, steps!(text.overflow_wrap)),
    e("line-break", Discrete, steps!(text.line_break)),
    e("hyphens", Discrete, steps!(text.hyphens)),
    e("tab-size", ByComputedValue, value!(text.tab_size)),
    e("text-transform", Discrete, steps!(text.text_transform)),
    e("text-indent", ByComputedValue, value!(text.text_indent)),
    e("text-align-all", Discrete, steps!(text.text_align_all)),
    e("text-align-last", Discrete, steps!(text.text_align_last)),
    e("text-justify", Discrete, steps!(text.text_justify)),
    e("text-wrap-style", Discrete, steps!(text.text_wrap_style)),
    e(
        "letter-spacing",
        ByComputedValue,
        value!(text.letter_spacing),
    ),
    e("word-spacing", ByComputedValue, value!(text.word_spacing)),
    // CSS Inline 3
    e("line-height", ByComputedValue, value!(text.line_height)),
    e("vertical-align", ByComputedValue, value!(vertical_align)),
    // CSS Writing Modes 4
    e("direction", NotAnimatable, None),
    e("writing-mode", NotAnimatable, None),
];
