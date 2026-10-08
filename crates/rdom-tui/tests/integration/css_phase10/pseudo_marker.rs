//! C10G-PSEUDO-MARKER — CSS Pseudo-Elements 4 §4 / CSS Lists 3 §3.1: a
//! `::before` or `::after` whose `display` is `list-item` is a list item,
//! so it has a `::marker` of its own — `::before::marker` /
//! `::after::marker` — and increments the `list-item` counter (§4.6) as
//! an element list item does. The marker rides the pseudo-element's first
//! line: inside, as its first inline box; outside, hung beside its box.

use crossterm::event::{Event as CtEvent, KeyModifiers, MouseEvent, MouseEventKind};
use rdom_tui::ext::PseudoSlot;
use rdom_tui::prelude::*;
use rdom_tui::render::{Terminal, TestBackend};
use rdom_tui::{App, Color, HitTestExt};

use super::{el, paint, paint_tree, rows, text_el};

const RED: Color = Color::Rgb(255, 0, 0);

/// A `.l` holding two `.h` blocks, `a` and `b`, under the root.
fn two_hosts(dom: &mut TuiDom, root: NodeId) -> (NodeId, NodeId) {
    let l = el(dom, root, "div", "l");
    let a = text_el(dom, l, "div", "h", "a");
    let b = text_el(dom, l, "div", "h", "b");
    (a, b)
}

/// CSS Lists 3 §3.1 / §4.6: a `display: list-item` `::before` gets a
/// marker, numbered by the `list-item` counter it increments itself — 1,
/// 2 across two hosts under one `counter-reset: list-item` — placed
/// `inside`, as the first inline box of the `::before`'s line.
#[test]
fn a_list_item_before_has_a_marker_inside_its_line() {
    let rows = paint_tree(
        ".l { counter-reset: list-item } \
         .h::before { display: list-item; content: 'x'; \
         list-style-type: decimal; list-style-position: inside }",
        6,
        4,
        |dom, root| {
            two_hosts(dom, root);
        },
    );
    assert_eq!(rows, ["1. x  ", "a     ", "2. x  ", "b     "]);
}

/// CSS Lists 3 §3.5: an `outside` marker (the initial position) hangs
/// beside the `::before`'s box, its end at the box's inline-start border
/// edge — in the host's padding here — taking no room in the line.
#[test]
fn an_outside_nested_marker_hangs_beside_the_pseudo_elements_box() {
    let rows = paint_tree(
        ".l { counter-reset: list-item } .h { padding-left: 3 } \
         .h::before { display: list-item; content: 'x'; list-style-type: decimal }",
        6,
        4,
        |dom, root| {
            two_hosts(dom, root);
        },
    );
    assert_eq!(rows, ["1. x  ", "   a  ", "2. x  ", "   b  "]);
}

/// `::after::marker`: a list-item `::after` after its host's content,
/// its marker a `list-style-type` string (CSS Lists 3 §3.4).
#[test]
fn a_list_item_after_has_a_marker_too() {
    let rows = paint_tree(
        ".h::after { display: list-item; content: 'y'; \
         list-style-type: '- '; list-style-position: inside }",
        6,
        2,
        |dom, root| {
            text_el(dom, root, "div", "h", "a");
        },
    );
    assert_eq!(rows, ["a     ", "- y   "]);
}

/// `::before::marker` rules style the nested marker (CSS Lists 3 §3.2:
/// its `color` and `content`), not the `::before`'s text.
#[test]
fn nested_marker_rules_style_the_marker() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    text_el(&mut dom, root, "div", "h", "a");
    let buf = paint(
        &mut dom,
        ".h::before { display: list-item; content: 'x'; list-style-position: inside } \
         .h::before::marker { content: '→ '; color: red }",
        6,
        2,
    );
    assert_eq!(rows(&buf, 6, 2), ["→ x   ", "a     "]);
    assert_eq!(buf.cell(0, 0).unwrap().fg, RED);
    assert_ne!(buf.cell(2, 0).unwrap().fg, RED, "the ::before's own text");
}

/// A `::before` that is no list item has no marker: a `::before::marker`
/// rule styles nothing (CSS Lists 3 §3.1).
#[test]
fn a_before_that_is_no_list_item_has_no_marker() {
    let rows = paint_tree(
        ".h::before { content: 'x' } .h::before::marker { content: '!' }",
        6,
        1,
        |dom, root| {
            text_el(dom, root, "div", "h", "a");
        },
    );
    assert_eq!(rows, ["xa    "]);
}

/// A list-item `::before` laid out as a box of its own anywhere — a flex
/// or grid item (CSS Flexbox §4, CSS Grid 2 §6.1), a float (CSS 2.1 §9.5), an absolutely
/// positioned box (CSS Position 3) — carries its marker on its own lines.
#[test]
fn every_box_a_before_makes_carries_its_marker() {
    let marker = "display: list-item; content: 'x'; \
                  list-style-type: decimal; list-style-position: inside";
    for (host, before, expected) in [
        ("display: flex", "", "1. xa "),
        (
            "display: grid; grid-template-columns: max-content max-content",
            "",
            "1. xa ",
        ),
        ("", "float: left;", "1. xa "),
        (
            "position: relative",
            "position: absolute; top: 0; right: 0;",
            "a 1. x",
        ),
    ] {
        let css = format!(".h {{ {host} }} .h::before {{ {before} {marker} }}");
        let rows = paint_tree(&css, 6, 1, |dom, root| {
            text_el(dom, root, "div", "h", "a");
        });
        assert_eq!(rows, [expected], "{host} / {before}");
    }
}

/// Selectors 4 §3.6.3: `::before::marker:hover` matches while the pointer
/// is over the nested marker, which the pseudo hit test names — not over
/// the `::before`'s own text.
#[test]
fn a_nested_marker_is_hit_and_hovered() {
    let mut dom = TuiDom::new();
    let root = dom.root();
    let h = text_el(&mut dom, root, "div", "h", "a");
    let sheet = rdom_css::from_css_strict(
        ".h::before { display: list-item; content: 'x'; list-style-position: inside } \
         .h::before::marker:hover { color: red }",
    )
    .unwrap();
    let terminal = Terminal::new(TestBackend::new(6, 2)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    app.advance(0).unwrap();
    assert_eq!(
        app.dom().hit_test_pseudo(0, 0),
        Some((h, PseudoSlot::BeforeMarker))
    );
    assert_eq!(
        app.dom().hit_test_pseudo(2, 0),
        Some((h, PseudoSlot::Before))
    );
    let fg = |app: &App<TestBackend>| {
        app.dom()
            .node(h)
            .computed_pseudo(PseudoSlot::BeforeMarker)
            .map(|c| c.fg)
    };
    assert_ne!(fg(&app), Some(RED));
    app.handle_event(CtEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Moved,
        column: 0,
        row: 0,
        modifiers: KeyModifiers::empty(),
    }));
    app.advance(0).unwrap();
    assert_eq!(fg(&app), Some(RED));
}
