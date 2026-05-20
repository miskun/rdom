//! `<label>` click default — focus the associated control.

use crossterm::event::{
    Event as CtEvent, KeyModifiers, MouseButton, MouseEvent as CtMouseEvent, MouseEventKind,
};
use rdom_core::{ListenerOptions, NodeId};

use crate::TuiDom;
use crate::layout::Size;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::{Stylesheet, TuiStyle};

fn test_app(dom: TuiDom, sheet: Stylesheet) -> App<TestBackend> {
    let backend = TestBackend::new(40, 10);
    let terminal = Terminal::new(backend).unwrap();
    App::with_backend(dom, sheet, terminal).unwrap()
}

fn click_at(app: &mut App<TestBackend>, x: u16, y: u16) {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        app.handle_event(CtEvent::Mouse(CtMouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        }));
    }
}

// ── Explicit association via `for` ─────────────────────────────────

#[test]
fn click_on_label_with_for_focuses_target_input() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let label = dom.create_element("label");
    dom.set_attribute(label, "for", "name").unwrap();
    let t = dom.create_text_node("Name");
    dom.append_child(label, t).unwrap();
    let input = dom.create_element("input");
    dom.set_attribute(input, "id", "name").unwrap();
    dom.append_child(root, label).unwrap();
    dom.append_child(root, input).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "label",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 2, 0);
    assert_eq!(app.dom().focused(), Some(input));
}

// ── Implicit association via wrapping ──────────────────────────────

#[test]
fn click_on_label_wrapping_input_focuses_it() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let label = dom.create_element("label");
    let t = dom.create_text_node("Check:");
    dom.append_child(label, t).unwrap();
    let input = dom.create_element("input");
    dom.set_attribute(input, "type", "checkbox").unwrap();
    dom.append_child(label, input).unwrap();
    dom.append_child(root, label).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "label",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 2, 0);
    assert_eq!(app.dom().focused(), Some(input));
}

// ── Edge cases ─────────────────────────────────────────────────────

#[test]
fn click_on_label_with_for_to_missing_id_focuses_nothing() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let label = dom.create_element("label");
    dom.set_attribute(label, "for", "nope").unwrap();
    let t = dom.create_text_node("X");
    dom.append_child(label, t).unwrap();
    dom.append_child(root, label).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "label",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 0, 0);
    assert_eq!(app.dom().focused(), None);
}

#[test]
fn click_on_label_with_no_association_focuses_nothing() {
    // Bare <label> with no for attribute + no labelable child.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let label = dom.create_element("label");
    let t = dom.create_text_node("plain");
    dom.append_child(label, t).unwrap();
    dom.append_child(root, label).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "label",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 0, 0);
    assert_eq!(app.dom().focused(), None);
}

#[test]
fn click_on_non_label_is_untouched() {
    // Random <div> click shouldn't trigger the label path.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let d = dom.create_element("div");
    dom.append_child(root, d).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "div",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 0, 0);
    assert_eq!(app.dom().focused(), None);
}

#[test]
fn prevent_default_on_click_blocks_focus_transfer() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let label = dom.create_element("label");
    dom.set_attribute(label, "for", "n").unwrap();
    let input = dom.create_element("input");
    dom.set_attribute(input, "id", "n").unwrap();
    dom.append_child(root, label).unwrap();
    dom.append_child(root, input).unwrap();

    dom.add_event_listener(label, "click", ListenerOptions::default(), |ctx| {
        ctx.event.prevent_default();
    })
    .unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "label",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 2, 0);
    assert_eq!(app.dom().focused(), None);
}

#[test]
fn label_wrapping_non_labelable_element_focuses_nothing() {
    // `<label><span>…</span></label>` — span isn't labelable.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let label = dom.create_element("label");
    let span = dom.create_element("span");
    let t = dom.create_text_node("X");
    dom.append_child(span, t).unwrap();
    dom.append_child(label, span).unwrap();
    dom.append_child(root, label).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "label",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 0, 0);
    assert_eq!(app.dom().focused(), None);
}

#[test]
fn for_attribute_wins_over_implicit_wrap() {
    // If both `for` and a wrapped control exist, `for` wins.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let label = dom.create_element("label");
    dom.set_attribute(label, "for", "outside").unwrap();
    let wrapped = dom.create_element("input");
    dom.set_attribute(wrapped, "id", "wrapped").unwrap();
    dom.append_child(label, wrapped).unwrap();
    let outside = dom.create_element("input");
    dom.set_attribute(outside, "id", "outside").unwrap();
    dom.append_child(root, label).unwrap();
    dom.append_child(root, outside).unwrap();

    let sheet = Stylesheet::bare().rule_unchecked(
        "label",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(1)),
    );
    let mut app = test_app(dom, sheet);
    app.draw_if_dirty().unwrap();
    click_at(&mut app, 0, 0);
    assert_eq!(app.dom().focused(), Some(outside));
}

fn _use_nodeid(_id: NodeId) {}
