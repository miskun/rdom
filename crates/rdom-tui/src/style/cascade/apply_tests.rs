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
    ("flex-direction", "row"),
    ("white-space", "pre"),
    ("user-select", "none"),
    ("pointer-events", "none"),
    ("caret-color", "transparent"),
    ("caret-text-color", "red"),
    ("overflow-x", "scroll"),
    ("overflow-y", "hidden"),
    ("scrollbar-gutter", "stable"),
    ("width", "10"),
    ("height", "5"),
    ("min-width", "2"),
    ("max-width", "20"),
    ("min-height", "1"),
    ("max-height", "9"),
    ("aspect-ratio", "2 / 1"),
    ("gap", "1"),
    ("flex-shrink", "0"),
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
    ("transition-property", "color"),
    ("transition-duration", "100ms"),
    ("transition-timing-function", "ease-in"),
    ("transition-delay", "10ms"),
    ("counter-reset", "a"),
    ("counter-increment", "a"),
];

/// `P6G-APPLY-INITIALS-1`: `<property>: initial` computes to exactly
/// `ComputedStyle::initial()`'s value, for every property the dispatch
/// table knows. The regression net against the cascade's `initial`
/// and the starting style drifting apart: the destructuring below is
/// exhaustive, so a new `ComputedStyle` field fails to compile here
/// until it is covered.
#[test]
fn initial_keyword_yields_the_initial_computed_value_for_every_property() {
    let perturbed = style_of(PERTURB);
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
        border_fg,
        modifiers,
        opacity,
        width,
        height,
        min_width,
        max_width,
        min_height,
        max_height,
        aspect_ratio,
        padding,
        margin,
        gap,
        flex_shrink,
        border,
        border_collapse,
        // Not a property: set by any declared `border-collapse`.
        border_collapse_declared: _,
        direction,
        overflow_x,
        overflow_y,
        scrollbar_gutter,
        display,
        flow,
        // Derived at finalization from display / position / overflow.
        establishes_new_bfc: _,
        white_space,
        user_select,
        pointer_events,
        caret_color,
        caret_text_color,
        // Generated content only; an element's own is always `None`.
        content: _,
        position,
        top,
        right,
        bottom,
        left,
        z_index,
        transition_property,
        transition_duration,
        transition_timing_function,
        transition_delay,
        counter_reset,
        counter_increment,
        // Custom properties, not a property value.
        vars: _,
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
        border_fg,
        modifiers,
        opacity,
        width,
        height,
        min_width,
        max_width,
        min_height,
        max_height,
        aspect_ratio,
        padding,
        margin,
        gap,
        flex_shrink,
        border,
        border_collapse,
        direction,
        overflow_x,
        overflow_y,
        scrollbar_gutter,
        display,
        flow,
        white_space,
        user_select,
        pointer_events,
        caret_color,
        caret_text_color,
        position,
        top,
        right,
        bottom,
        left,
        z_index,
        transition_property,
        transition_duration,
        transition_timing_function,
        transition_delay,
        counter_reset,
        counter_increment,
    );
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
