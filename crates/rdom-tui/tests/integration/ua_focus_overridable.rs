//! `UA-FOCUS-OVERRIDABLE-1`: the generic UA `:focus` background tint is
//! non-important, so authors can override it on any element (a
//! `<canvas>` or app-painted container that owns its background). The
//! text-field family keeps an `!important` focus tint so it still beats
//! its own high-specificity field background.

use rdom_tui::prelude::*;
use rdom_tui::style::Color;

const FOCUS_BG: Color = Color::Rgb(0x2d, 0x2f, 0x31);

fn cascade_focused(dom: &mut TuiDom, sheet: &Stylesheet, focus: NodeId) -> Color {
    dom.set_focused(Some(focus));
    dom.cascade(sheet);
    dom.node(focus).computed().cloned().unwrap().bg
}

#[test]
fn author_focus_rule_overrides_generic_focus_tint() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let canvas = dom.create_element("canvas");
    dom.append_child(root, canvas).unwrap();

    // Higher-specificity author rule (canvas:focus = 0,1,1) beats the
    // generic UA `:focus` (0,1,0) — both non-important now.
    let sheet = Stylesheet::new()
        .rule("canvas:focus", TuiStyle::new().bg(Color::Rgb(0, 0, 255)))
        .unwrap();

    assert_eq!(
        cascade_focused(&mut dom, &sheet, canvas),
        Color::Rgb(0, 0, 255),
        "author canvas:focus rule must override the UA focus tint (was unoverridable when !important)"
    );
}

#[test]
fn inline_style_overrides_generic_focus_tint() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let canvas = dom.create_element("canvas");
    // Inline (normal) beats UA-normal in the cascade ladder.
    dom.node_mut(canvas)
        .set_inline_style(TuiStyle::new().bg(Color::Reset));
    dom.append_child(root, canvas).unwrap();

    assert_eq!(
        cascade_focused(&mut dom, &Stylesheet::new(), canvas),
        Color::Reset,
        "an inline background must reclaim a focused canvas"
    );
}

#[test]
fn unfocused_canvas_has_no_focus_tint() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let canvas = dom.create_element("canvas");
    dom.append_child(root, canvas).unwrap();
    dom.cascade(&Stylesheet::new());
    assert_eq!(dom.node(canvas).computed().unwrap().bg, Color::Reset);
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
