//! C12-CONTROLS — CSS UI 4 §6.3 `accent-color`, §7.1 `appearance`, §7.2
//! `field-sizing`, §4.2 `resize`: the form controls' UA chrome under the
//! user-interface properties.

use super::{cell, el, paint, rows};
use rdom_tui::{Color, NodeId, TuiDom};

const RED: Color = Color::Rgb(255, 0, 0);

fn input(dom: &mut TuiDom, parent: NodeId, ty: &str, checked: bool) -> NodeId {
    let id = dom.create_element("input");
    dom.set_attribute(id, "type", ty).unwrap();
    if checked {
        dom.set_attribute(id, "checked", "").unwrap();
    }
    dom.append_child(parent, id).unwrap();
    id
}

// ── C12-CONTROLS: accent-color (§6.3) ─────────────────────────────

/// §6.3: `accent-color` tints the checked state of a checkbox and a radio
/// — the mark — and is inherited, so a form sets it for its controls; an
/// unchecked control and `auto` keep the text color.
#[test]
fn accent_color_tints_checked_toggles() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let form = el(&mut dom, root, "f", "");
    input(&mut dom, form, "checkbox", true);
    input(&mut dom, form, "checkbox", false);
    let r = el(&mut dom, root, "r", "");
    input(&mut dom, r, "radio", true);
    let buf = paint(
        &mut dom,
        ".f, .r { accent-color: red } input { display: block }",
        12,
        4,
    );
    assert_eq!(rows(&buf)[..3], ["[x]", "[ ]", "(•)"]);
    assert_eq!(cell(&buf, 1, 0).fg, RED, "checked checkbox");
    assert_ne!(cell(&buf, 1, 1).fg, RED, "unchecked");
    assert_eq!(cell(&buf, 1, 2).fg, RED, "checked radio");
    let mut dom = TuiDom::new();
    let root = dom.root();
    input(&mut dom, root, "checkbox", true);
    let buf = paint(&mut dom, "input { color: blue }", 12, 2);
    assert_eq!(cell(&buf, 1, 0).fg, Color::Rgb(0, 0, 255), "auto");
}

/// §6.3: a progress bar and a range slider draw in the accent color; a
/// meter's bar keeps its own (CSS UI 4 lists checkbox, radio, range and
/// progress).
#[test]
fn accent_color_tints_progress_and_range_not_meter() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("progress");
    dom.set_attribute(p, "value", "1").unwrap();
    dom.append_child(root, p).unwrap();
    let m = dom.create_element("meter");
    dom.set_attribute(m, "value", "1").unwrap();
    dom.append_child(root, m).unwrap();
    input(&mut dom, root, "range", false);
    // The slider's paint callback (an `App` attaches it at build).
    rdom_tui::runtime::builtins::range::attach_all(&mut dom);
    let buf = paint(
        &mut dom,
        "progress, meter, input { width: 6; accent-color: red }",
        12,
        4,
    );
    assert_eq!(cell(&buf, 0, 0).fg, RED, "progress");
    assert_ne!(cell(&buf, 0, 1).fg, RED, "meter");
    assert_eq!(cell(&buf, 0, 2).fg, RED, "range track");
}
