//! `UA-FOCUS-OVERRIDABLE-1`: focus-indicator semantics.
//!
//! 1. The generic UA `:focus` background tint is non-important, so
//!    authors can override it on any element (was unoverridable when
//!    `!important`).
//! 2. A focused `<canvas>` gets **no** tint by default — it's a
//!    replaced/content element the app paints, so the focus background
//!    must not paint over it (the web focuses a canvas with a
//!    non-destructive outline; rdom opts canvas out of the bg tint).
//! 3. Non-canvas focusable elements still get the tint, and the
//!    text-field family keeps it over their own field background.

use rdom_tui::prelude::*;
use rdom_tui::style::Color;

const FOCUS_BG: Color = Color::Rgb(0x2d, 0x2f, 0x31);

fn cascade_focused(dom: &mut TuiDom, sheet: &Stylesheet, focus: NodeId) -> Color {
    dom.set_focused(Some(focus));
    dom.cascade(sheet);
    dom.node(focus).computed().cloned().unwrap().bg
}

#[test]
fn focused_canvas_is_clean_by_default() {
    // No author rule, no inline style — a focused canvas must NOT be
    // tinted. This is the behavior a web dev expects (focus never
    // repaints a canvas), with zero consumer effort.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let canvas = dom.create_element("canvas");
    dom.append_child(root, canvas).unwrap();

    assert_eq!(
        cascade_focused(&mut dom, &Stylesheet::new(), canvas),
        Color::Reset,
        "a focused <canvas> must keep its (transparent) background by default"
    );
}

#[test]
fn focused_non_canvas_still_gets_the_tint() {
    // The exemption is scoped to canvas — a focusable div still shows
    // the generic focus indicator (no regression of the affordance).
    let mut dom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    assert_eq!(
        cascade_focused(&mut dom, &Stylesheet::new(), div),
        FOCUS_BG,
        "non-canvas focusable elements keep the generic focus tint"
    );
}

#[test]
fn author_can_paint_a_focused_canvas_if_it_wants() {
    // Canvas opts out by default, but an app that *wants* a focused-
    // canvas background can still set one (the exemption isn't a lock).
    let mut dom = TuiDom::new();
    let root = dom.root();
    let canvas = dom.create_element("canvas");
    dom.append_child(root, canvas).unwrap();
    let sheet = Stylesheet::new()
        .rule("canvas:focus", TuiStyle::new().bg(Color::Rgb(0, 0, 255)))
        .unwrap();
    assert_eq!(
        cascade_focused(&mut dom, &sheet, canvas),
        Color::Rgb(0, 0, 255),
    );
}

#[test]
fn author_focus_rule_overrides_generic_tint_on_a_div() {
    // The generic tint is overridable (non-important) on any element.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();
    let sheet = Stylesheet::new()
        .rule("div:focus", TuiStyle::new().bg(Color::Rgb(0, 0, 255)))
        .unwrap();
    assert_eq!(
        cascade_focused(&mut dom, &sheet, div),
        Color::Rgb(0, 0, 255)
    );
}

#[test]
fn text_input_still_shows_focus_tint() {
    // Regression guard: the field-bg chain (0,7,1) must NOT hide the
    // focus tint — the scoped `input:focus { … !important }` keeps it.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.append_child(root, input).unwrap();

    assert_eq!(
        cascade_focused(&mut dom, &Stylesheet::new(), input),
        FOCUS_BG,
        "text input must still show the focus tint over its field background"
    );
}
