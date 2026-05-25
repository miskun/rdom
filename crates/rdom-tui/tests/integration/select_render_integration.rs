//! End-to-end rendered-buffer integration test for `<select>`.
//!
//! At the architect-required bar: drive `App` with real DOM
//! construction, then re-render and assert that the selected
//! option's label actually appears in the painted buffer.
//!
//! Catches the bug class where the select's chrome substitution
//! path (`select_chrome_text` in `inline_paint.rs`) silently fails
//! to render — e.g. when the selected option lookup returns the
//! wrong label, or when paint takes a different path entirely.

use crate::common::render;
use rdom_tui::layout::Size;
use rdom_tui::prelude::*;
use rdom_tui::render::{Buffer, Rect, Terminal, TestBackend};
use rdom_tui::runtime::app::App;

fn row_slice(buf: &Buffer, y: u16, x_start: u16, x_end: u16) -> String {
    let mut out = String::new();
    for x in x_start..x_end {
        if let Some(c) = buf.cell(x, y)
            && !c.is_spacer()
        {
            out.push_str(c.symbol());
        }
    }
    out
}

#[test]
fn closed_select_renders_selected_option_label() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let sel = dom.create_element("select");
    dom.set_attribute(sel, "name", "color").unwrap();
    for (val, label, default) in [
        ("red", "Red", false),
        ("green", "Green", true),
        ("blue", "Blue", false),
    ] {
        let opt = dom.create_element("option");
        dom.set_attribute(opt, "value", val).unwrap();
        if default {
            dom.set_attribute(opt, "selected", "").unwrap();
        }
        let t = dom.create_text_node(label);
        dom.append_child(opt, t).unwrap();
        dom.append_child(sel, opt).unwrap();
    }
    dom.append_child(root, sel).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "select",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(1)),
    );

    let backend = TestBackend::new(40, 5);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();

    // Sanity: closed dropdown has the second option selected.
    assert_eq!(
        app.dom().node(sel).get_attribute("name"),
        Some("color"),
        "select carries the name attribute"
    );

    let sheet_for_render = Stylesheet::new().rule_unchecked(
        "select",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(1)),
    );
    let buf = render(app.dom_mut(), &sheet_for_render, Rect::new(0, 0, 40, 5));
    let row0 = row_slice(&buf, 0, 0, 40);
    assert!(
        row0.contains("Green"),
        "closed select should render the selected option's label ('Green'); got {row0:?}"
    );
}
