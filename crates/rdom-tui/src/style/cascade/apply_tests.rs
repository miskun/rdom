//! CSS-wide keyword tests for the cascade applicators (`apply.rs`):
//! `initial` resolves to the one table of initial values
//! (`ComputedStyle::initial()`), `inherit` to the parent's computed
//! value, for every property.

use super::*;
use crate::TuiDom;
use crate::layout::{Display, Flow};
use crate::style::{Stylesheet, TuiStyle};
use rdom_core::NodeId;
use rdom_style::property_dispatch;

/// A `<div>` under the root, styled by `sheet`; returns its computed style.
fn cascade_div(sheet: &Stylesheet) -> ComputedStyle {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let div: NodeId = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    dom.cascade(sheet);
    computed_of(&dom, div)
}

fn style_of(decls: &[(&str, &str)]) -> TuiStyle {
    let mut style = TuiStyle::new();
    for (name, value) in decls {
        property_dispatch::set(name, value, &mut style)
            .unwrap_or_else(|e| panic!("{name}: {value}: {e:?}"));
    }
    style
}

/// A non-initial value for every property that computes to a field;
/// the test below checks that each one moved its field off the initial
/// value, so a property missing here fails loudly.
const PERTURB: &[(&str, &str)] = &[
    ("color", "red"),
    ("background-color", "blue"),
    ("border-color", "green"),
    ("font-weight", "bold"),
    ("font-style", "italic"),
    ("text-decoration", "underline"),
    ("opacity", "0.5"),
    ("display", "inline-flex"),
    ("flex-direction", "column-reverse"),
    ("flex-wrap", "wrap"),
    ("justify-content", "center"),
    ("align-items", "center"),
    ("align-content", "center"),
    ("justify-items", "center"),
    ("justify-self", "center"),
    ("align-self", "center"),
    ("direction", "rtl"),
    ("writing-mode", "vertical-lr"),
    ("white-space", "pre"),
    ("word-break", "keep-all"),
    ("overflow-wrap", "anywhere"),
    ("line-break", "strict"),
    ("hyphens", "none"),
    ("tab-size", "4"),
    ("text-transform", "uppercase"),
    ("text-indent", "2"),
    ("text-align", "center"),
    ("text-align-last", "right"),
    ("text-justify", "none"),
    ("text-wrap-style", "balance"),
    ("letter-spacing", "2"),
    ("word-spacing", "1"),
    ("line-height", "2"),
    ("vertical-align", "super"),
    ("text-decoration-style", "wavy"),
    ("text-decoration-color", "red"),
    ("text-decoration-thickness", "2"),
    ("text-underline-offset", "1"),
    ("text-underline-position", "under"),
    ("text-decoration-skip-ink", "none"),
    ("font-size", "large"),
    ("font-family", "serif"),
    ("font-stretch", "condensed"),
    ("font-variant", "small-caps"),
    ("user-select", "none"),
    ("pointer-events", "none"),
    ("visibility", "hidden"),
    ("caret-color", "transparent"),
    ("caret-text-color", "red"),
    ("overflow-x", "scroll"),
    ("overflow-y", "hidden"),
    ("overflow-clip-margin", "content-box 2"),
    ("text-overflow", "ellipsis"),
    ("max-lines", "3"),
    ("block-ellipsis", "'+'"),
    ("continue", "discard"),
    ("-webkit-box-orient", "vertical"),
    ("scrollbar-gutter", "stable"),
    ("scrollbar-width", "thin"),
    ("scrollbar-color", "red blue"),
    ("overscroll-behavior", "contain none"),
    ("scroll-padding", "1 2 3 4"),
    ("scroll-margin", "1 2 3 4"),
    ("scroll-snap-type", "y mandatory"),
    ("scroll-snap-align", "start"),
    ("scroll-snap-stop", "always"),
    ("scroll-behavior", "smooth"),
    // Before `flex-shrink`, which overrides its shrink: it perturbs
    // `flex_grow` and `flex_basis`.
    ("flex", "2 0 7"),
    ("width", "10"),
    ("height", "5"),
    ("min-width", "2"),
    ("max-width", "20"),
    ("min-height", "1"),
    ("max-height", "9"),
    ("aspect-ratio", "2 / 1"),
    ("box-sizing", "border-box"),
    ("margin-trim", "block"),
    ("contain-intrinsic-size", "auto 3"),
    ("container", "card / size"),
    ("contain", "paint"),
    ("will-change", "opacity"),
    ("content-visibility", "auto"),
    ("gap", "1 2"),
    ("flex-shrink", "0"),
    ("order", "3"),
    ("padding", "1"),
    ("margin", "1"),
    ("border", "solid"),
    ("border-collapse", "collapse"),
    ("position", "relative"),
    ("top", "1"),
    ("right", "1"),
    ("bottom", "1"),
    ("left", "1"),
    ("z-index", "3"),
    ("overlay", "auto"),
    ("clear", "both"),
    ("transition-property", "color"),
    ("transition-duration", "100ms"),
    ("transition-timing-function", "ease-in"),
    ("transition-delay", "10ms"),
    ("transition-behavior", "allow-discrete"),
    ("animation-name", "slide"),
    ("animation-duration", "1s"),
    ("animation-timing-function", "linear"),
    ("animation-delay", "-1s"),
    ("animation-iteration-count", "infinite"),
    ("animation-direction", "reverse"),
    ("animation-fill-mode", "both"),
    ("animation-play-state", "paused"),
    ("animation-composition", "add"),
    ("animation-timeline", "none"),
    ("scroll-timeline-name", "--s"),
    ("scroll-timeline-axis", "x"),
    ("view-timeline-name", "--v"),
    ("view-timeline-axis", "inline"),
    ("view-timeline-inset", "1"),
    ("timeline-scope", "all"),
    ("animation-range-start", "entry"),
    ("animation-range-end", "exit"),
    ("counter-reset", "a"),
    ("counter-increment", "a"),
    ("counter-set", "a 3"),
    ("quotes", "\"<\" \">\""),
    ("list-style", "inside url(a.png) square"),
    ("marker-side", "match-parent"),
    ("color-scheme", "light"),
    ("background-clip", "content-box"),
    ("border-width", "thick"),
    ("border-radius", "1"),
    ("box-shadow", "1 1 red"),
    ("grid-template-columns", "1 2"),
    ("grid-template-rows", "repeat(2, 1fr)"),
    ("grid-template-areas", "\"a b\""),
    ("grid-auto-columns", "3"),
    ("grid-auto-rows", "1fr 2"),
    ("grid-auto-flow", "column"),
    ("grid-area", "1 / 2 / span 2 / a"),
    ("border-spacing", "1"),
    ("interpolate-size", "allow-keywords"),
    ("outline", "auto red thick"),
    ("outline-offset", "2"),
    ("cursor", "pointer"),
    ("caret", "rgb(1 2 3) bar manual"),
    ("accent-color", "red"),
    ("appearance", "none"),
    ("field-sizing", "content"),
    ("resize", "both"),
    ("table-layout", "fixed"),
    ("caption-side", "bottom"),
    ("empty-cells", "hide"),
    ("translate", "1 2"),
    ("rotate", "10deg"),
    ("scale", "2"),
    ("transform", "translateX(1) rotate(5deg)"),
    ("transform-origin", "left top"),
    ("transform-box", "content-box"),
];

/// `P6G-APPLY-INITIALS-1`: `<property>: initial` computes to exactly
/// `ComputedStyle::initial()`'s value, for every property the dispatch
/// table knows. The regression net against the cascade's `initial`
/// and the starting style drifting apart: the destructuring below is
/// exhaustive, so a new `ComputedStyle` field fails to compile here
/// until it is covered.
#[test]
fn initial_keyword_yields_the_initial_computed_value_for_every_property() {
    let mut perturbed = style_of(PERTURB);
    // `display: inline-flex` above moves the outer and inner types; no
    // one `display` value moves those and `list-item` together.
    perturbed.list_item = Some(rdom_style::Value::Specified(true));
    let mut reset = TuiStyle::new();
    for name in property_dispatch::property_names() {
        property_dispatch::set(name, "initial", &mut reset)
            .unwrap_or_else(|e| panic!("{name}: initial: {e:?}"));
    }
    let moved = cascade_div(&Stylesheet::bare().rule_unchecked("div", perturbed.clone()));
    let got = cascade_div(
        &Stylesheet::bare()
            .rule_unchecked("div", perturbed)
            .rule_unchecked("div", reset),
    );

    let ComputedStyle {
        fg,
        bg,
        border_color,
        modifiers,
        opacity,
        background_clip,
        width,
        height,
        min_width,
        max_width,
        min_height,
        max_height,
        aspect_ratio,
        box_sizing,
        interpolate_size,
        contain_intrinsic_width,
        contain_intrinsic_height,
        container_type,
        container_name,
        contain,
        will_change,
        content_visibility,
        padding,
        margin,
        margin_trim,
        row_gap,
        column_gap,
        flex_grow,
        flex_shrink,
        flex_basis,
        order,
        grid_template_columns,
        grid_template_rows,
        grid_template_areas,
        grid_auto_columns,
        grid_auto_rows,
        grid_auto_flow,
        grid_row_start,
        grid_row_end,
        grid_column_start,
        grid_column_end,
        border,
        border_style,
        border_width,
        border_radius,
        box_shadow,
        border_spacing,
        border_collapse,
        // Not a property: set by any declared `border-collapse`.
        border_collapse_declared: _,
        direction,
        flex_reverse,
        flex_wrap,
        justify_content,
        align_items,
        align_content,
        justify_items,
        justify_self,
        align_self,
        text_direction,
        writing_mode,
        overflow_x,
        overflow_y,
        overflow_clip_margin,
        text_overflow,
        max_lines,
        block_ellipsis,
        continue_,
        webkit_box_orient,
        scrollbar_gutter,
        scrollbar_width,
        scrollbar_color,
        overscroll_behavior_x,
        overscroll_behavior_y,
        scroll_padding,
        scroll_margin,
        scroll_snap_type,
        scroll_snap_align,
        scroll_snap_stop,
        scroll_behavior,
        display,
        flow,
        list_item,
        // `-webkit-box` with PERTURB's vertical `-webkit-box-orient` and
        // `max-lines` is the legacy clamp, which would undo its `flow`:
        // covered by `display_tests` and `css_phase8::line_clamp`.
        webkit_box: _,
        // Derived at finalization from display / position / overflow.
        establishes_new_bfc: _,
        line_clamp_container: _,
        text,
        font,
        ui,
        effects,
        table,
        vertical_align,
        text_decoration,
        // Derived from `text_decoration` and the parent's (§2.1).
        applied_decorations: _,
        user_select,
        pointer_events,
        quotes,
        list_style_type,
        list_style_position,
        list_style_image,
        marker_side,
        visibility,
        caret_color,
        caret_text_color,
        // Generated content only; an element's own is always `None`.
        content: _,
        content_alt: _,
        content_quotes: _,
        position,
        top,
        right,
        bottom,
        left,
        z_index,
        overlay,
        // A floated box is blockified (CSS 2.1 §9.7), which would undo
        // PERTURB's `display`: `float` has its own test below.
        float: _,
        clear,
        transition_property,
        transition_duration,
        transition_timing_function,
        transition_delay,
        transition_behavior,
        animation_name,
        animation_duration,
        animation_timing_function,
        animation_delay,
        animation_iteration_count,
        animation_direction,
        animation_fill_mode,
        animation_play_state,
        animation_composition,
        animation_timeline,
        scroll_timeline_name,
        scroll_timeline_axis,
        view_timeline_name,
        view_timeline_axis,
        view_timeline_inset,
        timeline_scope,
        animation_range_start,
        animation_range_end,
        counter_reset,
        counter_increment,
        counter_set,
        color_scheme,
        // Custom properties, not a property value.
        vars: _,
        animated_vars: _,
    } = ComputedStyle::initial();

    macro_rules! check {
        ($($field:ident),* $(,)?) => {$(
            assert_ne!(moved.$field, $field, "PERTURB leaves `{}` at its initial value", stringify!($field));
            assert_eq!(got.$field, $field, "`initial` for `{}`", stringify!($field));
        )*};
    }
    check!(
        fg,
        bg,
        border_color,
        modifiers,
        opacity,
        background_clip,
        width,
        height,
        min_width,
        max_width,
        min_height,
        max_height,
        aspect_ratio,
        box_sizing,
        interpolate_size,
        contain_intrinsic_width,
        contain_intrinsic_height,
        container_type,
        container_name,
        contain,
        will_change,
        content_visibility,
        padding,
        margin,
        margin_trim,
        row_gap,
        column_gap,
        flex_grow,
        flex_shrink,
        flex_basis,
        order,
        grid_template_columns,
        grid_template_rows,
        grid_template_areas,
        grid_auto_columns,
        grid_auto_rows,
        grid_auto_flow,
        grid_row_start,
        grid_row_end,
        grid_column_start,
        grid_column_end,
        border,
        border_style,
        border_width,
        border_radius,
        box_shadow,
        border_spacing,
        border_collapse,
        direction,
        flex_reverse,
        flex_wrap,
        justify_content,
        align_items,
        align_content,
        justify_self,
        align_self,
        text_direction,
        writing_mode,
        overflow_x,
        overflow_y,
        overflow_clip_margin,
        text_overflow,
        max_lines,
        block_ellipsis,
        continue_,
        webkit_box_orient,
        scrollbar_gutter,
        scrollbar_width,
        scrollbar_color,
        overscroll_behavior_x,
        overscroll_behavior_y,
        scroll_padding,
        scroll_margin,
        scroll_snap_type,
        scroll_snap_align,
        scroll_snap_stop,
        scroll_behavior,
        display,
        flow,
        list_item,
        text,
        font,
        ui,
        effects,
        table,
        vertical_align,
        text_decoration,
        user_select,
        pointer_events,
        quotes,
        list_style_type,
        list_style_position,
        list_style_image,
        marker_side,
        visibility,
        caret_color,
        caret_text_color,
        position,
        top,
        right,
        bottom,
        left,
        z_index,
        overlay,
        clear,
        transition_property,
        transition_duration,
        transition_timing_function,
        transition_delay,
        transition_behavior,
        animation_name,
        animation_duration,
        animation_timing_function,
        animation_delay,
        animation_iteration_count,
        animation_direction,
        animation_fill_mode,
        animation_play_state,
        animation_composition,
        animation_timeline,
        scroll_timeline_name,
        scroll_timeline_axis,
        view_timeline_name,
        view_timeline_axis,
        view_timeline_inset,
        timeline_scope,
        animation_range_start,
        animation_range_end,
        counter_reset,
        counter_increment,
        counter_set,
        color_scheme,
    );
    // The CSS Text group, field by field: the destructuring of
    // `TextStyle` fails to compile when a field is added uncovered.
    let rdom_style::layout::TextStyle {
        white_space_collapse,
        text_wrap_mode,
        word_break,
        overflow_wrap,
        line_break,
        hyphens,
        tab_size,
        text_transform,
        text_indent,
        text_align_all,
        text_align_last,
        text_justify,
        text_wrap_style,
        letter_spacing,
        word_spacing,
        line_height,
        text_underline_offset,
        text_underline_position,
        text_decoration_skip_ink,
    } = text;
    macro_rules! check_text {
        ($($field:ident),* $(,)?) => {$(
            assert_ne!(moved.text.$field, $field, "PERTURB leaves `{}` at its initial value", stringify!($field));
            assert_eq!(got.text.$field, $field, "`initial` for `{}`", stringify!($field));
        )*};
    }
    check_text!(
        white_space_collapse,
        text_wrap_mode,
        word_break,
        overflow_wrap,
        line_break,
        hyphens,
        tab_size,
        text_transform,
        text_indent,
        text_align_all,
        text_align_last,
        text_justify,
        text_wrap_style,
        letter_spacing,
        word_spacing,
        line_height,
        text_underline_offset,
        text_underline_position,
        text_decoration_skip_ink,
    );
    // CSS Box Alignment 3 §6.2: `justify-items: initial` is `legacy`,
    // which computes to `normal` under a parent without a `legacy` value.
    assert_ne!(moved.justify_items, crate::layout::Alignment::NORMAL);
    assert_eq!(justify_items, crate::layout::Alignment::LEGACY);
    assert_eq!(got.justify_items, crate::layout::Alignment::NORMAL);
}

/// CSS Cascade 4 §7.2: `inherit` takes the parent's computed value,
/// inherited property or not. `display` owns the inner display
/// (`Flow`) too, so `display: inherit` under a flex container is a
/// flex container.
#[test]
fn display_inherit_takes_the_parents_inner_display_too() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let parent = dom.create_element("section");
    let child = dom.create_element("div");
    dom.append_child(parent, child).unwrap();
    dom.append_child(root, parent).unwrap();
    let sheet = Stylesheet::bare()
        .rule_unchecked("section", style_of(&[("display", "flex")]))
        .rule_unchecked("div", style_of(&[("display", "inherit")]));
    dom.cascade(&sheet);
    let c = computed_of(&dom, child);
    assert_eq!((c.display, c.flow), (Display::Block, Flow::Flex));
}

// ─── Blockification (C6G-BLOCKIFY) ──────────────────────────────────

/// CSS Display 3 §2.7 through a `contents` child (§2.5): a restyle that
/// turns the flex container back into a block must un-blockify the
/// `contents` element's children, though the `contents` element's own
/// computed style does not change — its subtree is not kept.
#[test]
fn a_restyle_unblockifies_the_children_of_a_contents_item() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let f = dom.create_element("div");
    let c = dom.create_element("span");
    let s = dom.create_element("span");
    dom.append_child(root, f).unwrap();
    dom.append_child(f, c).unwrap();
    dom.append_child(c, s).unwrap();
    dom.node_mut(c)
        .ext_mut()
        .unwrap()
        .set_inline_style(TuiStyle::new().display(Display::Contents));
    dom.node_mut(s)
        .ext_mut()
        .unwrap()
        .set_inline_style(TuiStyle::new().display(Display::Inline));
    dom.node_mut(f)
        .ext_mut()
        .unwrap()
        .set_inline_style(TuiStyle::new().flow(Flow::Flex));
    let sheet = Stylesheet::bare();
    dom.cascade(&sheet);
    assert_eq!(computed_of(&dom, s).display, Display::Block);

    dom.node_mut(f)
        .ext_mut()
        .unwrap()
        .set_inline_style(TuiStyle::new());
    let sheets = [&sheet];
    let registry = Rc::new(PropertyRegistry::new(&sheets));
    restyle_vars(&mut dom, &sheets, registry, &[f]);
    assert_eq!(computed_of(&dom, f).flow, Flow::Block);
    assert_eq!(computed_of(&dom, s).display, Display::Inline);
}

/// `float: initial` computes to `none` (CSS 2.1 §9.5.1), and the box it
/// had blockified is inline again (§9.7) — kept apart from
/// [`initial_keyword_yields_the_initial_computed_value_for_every_property`],
/// whose `display` a float would blockify.
#[test]
fn float_initial_is_none_and_unblockifies() {
    let floated = style_of(&[("float", "left"), ("display", "inline")]);
    let moved = cascade_div(&Stylesheet::bare().rule_unchecked("div", floated.clone()));
    assert_eq!(
        (moved.float, moved.display),
        (crate::layout::Float::Left, crate::layout::Display::Block)
    );
    let got = cascade_div(
        &Stylesheet::bare()
            .rule_unchecked("div", floated)
            .rule_unchecked("div", style_of(&[("float", "initial")])),
    );
    assert_eq!(
        (got.float, got.display),
        (crate::layout::Float::None, crate::layout::Display::Inline)
    );
}
