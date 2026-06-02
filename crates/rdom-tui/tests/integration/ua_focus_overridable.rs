//! Focus-indicator semantics (`UA-FOCUS-OVERRIDABLE-1` → `FOCUS-VOCAB-1`).
//!
//! 1. The focus background tint is non-important, so authors can override
//!    it (was unoverridable when `!important`).
//! 2. The tint is **scoped to atomic controls** (button/input/textarea/
//!    select/summary/a). A focused container — `<canvas>`, `<div>`, table,
//!    scroll region — gets **no** background fill (the web shows those an
//!    outline, which rdom can't fill-substitute; containers express focus
//!    via the scrollbar thumb / an internal cursor / the consumer's CSS).
//!    This replaced the old generic `:focus` tint + its `canvas:focus`
//!    opt-out hack.
//! 3. The text-field family keeps the tint over their own field background.

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
fn focused_container_gets_no_tint() {
    // The tint is scoped to atomic controls — a focused container (div,
    // table, canvas, …) is NOT flooded with the focus background. It's the
    // same clean result canvas got via its old opt-out hack, now the default
    // for every non-control.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let div = dom.create_element("div");
    dom.append_child(root, div).unwrap();

    assert_eq!(
        cascade_focused(&mut dom, &Stylesheet::new(), div),
        Color::Reset,
        "a focused container must not be flooded with the focus tint"
    );
}

#[test]
fn focused_control_gets_the_tint() {
    // The affordance is intact where it belongs: a focused atomic control
    // (here a <button>) shows the focus background.
    let mut dom = TuiDom::new();
    let root = dom.root();
    let button = dom.create_element("button");
    dom.append_child(root, button).unwrap();

    assert_eq!(
        cascade_focused(&mut dom, &Stylesheet::new(), button),
        FOCUS_BG,
        "a focused control keeps the focus tint"
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
fn author_can_express_focus_on_a_container() {
    // Containers get no default focus fill, but an author can express focus
    // however they like — here a `div:focus` background.
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
