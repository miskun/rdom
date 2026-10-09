//! The second half of the longhand table (`table`): the grid, box,
//! border, table, generated content, positioning, transition, animation,
//! timeline, counter and text longhands.

use super::super::AnimationType::{ByComputedValue, Discrete, NotAnimatable, ShadowList};
use super::super::entry::{Entry, OVERLAY, Ops, e, fix_border, steps, value};
use super::super::value::{blend, discrete, interpolable};

/// The longhands from the grid ones on, in the dispatch table's order.
pub(super) const FLOW: &[Entry] = &[
    // CSS Grid 2
    e(
        "grid-template-columns",
        ByComputedValue,
        value!(grid.grid_template_columns),
    ),
    e(
        "grid-template-rows",
        ByComputedValue,
        value!(grid.grid_template_rows),
    ),
    e(
        "grid-template-areas",
        Discrete,
        steps!(grid.grid_template_areas),
    ),
    e(
        "grid-auto-columns",
        ByComputedValue,
        value!(grid.grid_auto_columns),
    ),
    e(
        "grid-auto-rows",
        ByComputedValue,
        value!(grid.grid_auto_rows),
    ),
    e("grid-auto-flow", Discrete, steps!(grid.grid_auto_flow)),
    e("grid-row-start", Discrete, steps!(grid.grid_row_start)),
    e("grid-row-end", Discrete, steps!(grid.grid_row_end)),
    e(
        "grid-column-start",
        Discrete,
        steps!(grid.grid_column_start),
    ),
    e("grid-column-end", Discrete, steps!(grid.grid_column_end)),
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
        value!(motion.view_timeline_inset),
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
