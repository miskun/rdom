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

// ── C12-CONTROLS: appearance (§7.1) ───────────────────────────────

/// §7.1 `appearance: none`: the control is drawn without its native
/// chrome — rdom's toggle marks, button brackets and the select's `▾` are
/// the UA's `::before` / `::after` rules, which it no longer takes — so
/// author CSS draws it; an author `::before` still applies. `auto`, `base`
/// and the compat keywords keep the chrome; `-webkit-appearance` is the
/// legacy name.
#[test]
fn appearance_none_strips_the_pseudo_element_chrome() {
    let render = |css: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        input(&mut dom, root, "checkbox", true);
        let b = dom.create_element("button");
        let t = dom.create_text_node("OK");
        dom.append_child(b, t).unwrap();
        dom.append_child(root, b).unwrap();
        let s = dom.create_element("select");
        let o = dom.create_element("option");
        dom.set_attribute(o, "selected", "").unwrap();
        let ot = dom.create_text_node("one");
        dom.append_child(o, ot).unwrap();
        dom.append_child(s, o).unwrap();
        dom.append_child(root, s).unwrap();
        let buf = paint(
            &mut dom,
            &format!("input, button {{ display: block }} {css}"),
            30,
            4,
        );
        rows(&buf)
    };
    let chrome = render("");
    assert_eq!(chrome[..2], ["[x]", "[ OK ]"]);
    assert!(chrome[2].contains('▾'), "{chrome:?}");
    for css in [
        "input, button, select { appearance: none }",
        "input, button, select { -webkit-appearance: none }",
    ] {
        let bare = render(css);
        assert_eq!(bare[..2], ["", "OK"], "{css}");
        assert!(!bare[2].contains('▾'), "{css}: {bare:?}");
    }
    for css in [
        "input, button, select { appearance: auto }",
        "input, button, select { appearance: base }",
        "input { appearance: checkbox } button { appearance: button }",
    ] {
        assert_eq!(render(css), chrome, "{css}");
    }
    let authored = render("input { appearance: none } input::before { content: '✓' }");
    assert_eq!(authored[0], "✓");
}

/// §7.1: a progress bar's and a range slider's chrome is their painted
/// bar and track; under `appearance: none` they draw nothing.
#[test]
fn appearance_none_strips_painted_chrome() {
    let render = |css: &str| {
        let mut dom = TuiDom::new();
        let root = dom.root();
        let p = dom.create_element("progress");
        dom.set_attribute(p, "value", "1").unwrap();
        dom.append_child(root, p).unwrap();
        input(&mut dom, root, "range", false);
        rdom_tui::runtime::builtins::range::attach_all(&mut dom);
        let buf = paint(
            &mut dom,
            &format!("progress, input {{ width: 6 }} {css}"),
            8,
            2,
        );
        rows(&buf)
    };
    let chrome = render("");
    assert!(!chrome[0].is_empty() && !chrome[1].is_empty(), "{chrome:?}");
    assert_eq!(render("progress, input { appearance: none }"), ["", ""]);
}

// ── C12-CONTROLS: field-sizing (§7.2) ─────────────────────────────

/// A text-family `<input>` (or a `<textarea>`) holding `value` — its
/// value lives in a text child.
fn field(dom: &mut TuiDom, tag: &str, value: &str) -> NodeId {
    let root = dom.root();
    let id = dom.create_element(tag);
    let t = dom.create_text_node(value);
    dom.append_child(id, t).unwrap();
    let wrap = dom.create_element("div");
    dom.append_child(wrap, id).unwrap();
    dom.append_child(root, wrap).unwrap();
    id
}

fn size_of(dom: &TuiDom, id: NodeId) -> (u16, u16) {
    use rdom_tui::TuiNodeExt;
    let r = dom.node(id).layout_rect().expect("laid out");
    (r.width, r.height)
}

/// §7.2 `field-sizing: content`: a text field's size follows its content
/// — the UA's fixed 20-cell field (rdom's stand-in for a browser's
/// `size` / `cols` / `rows` intrinsic size) no longer applies — padding
/// included; `fixed` (the initial value) keeps it, and an author `width`
/// / `height` still wins over the content.
#[test]
fn field_sizing_content_sizes_a_field_to_its_value() {
    let mut dom = TuiDom::new();
    let a = field(&mut dom, "input", "hello");
    let b = field(&mut dom, "input", "hello");
    let c = field(&mut dom, "input", "hello");
    let t = field(&mut dom, "textarea", "ab\ncdef");
    for (id, class) in [(a, "c"), (b, "f"), (c, "w"), (t, "c")] {
        dom.set_attribute(id, "class", class).unwrap();
    }
    paint(
        &mut dom,
        ".c { field-sizing: content } .f { field-sizing: fixed } \
         .w { field-sizing: content; width: 10 }",
        30,
        12,
    );
    assert_eq!(size_of(&dom, a), (7, 1), "content: 5 + padding");
    // The UA field: 20 cells of content box, plus padding.
    assert_eq!(size_of(&dom, b), (22, 1), "fixed: the UA field");
    assert_eq!(size_of(&dom, c), (12, 1), "an author width wins");
    assert_eq!(size_of(&dom, t), (6, 2), "textarea: widest line, two rows");
}
