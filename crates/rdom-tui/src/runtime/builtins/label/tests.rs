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

// ── Toggle re-dispatch ─────────────────────────────────────────────
// Per the module contract: clicking a label associated with a
// checkbox or radio input ALSO synthesizes a click on the control
// so it toggles. (Other labelables — button, select, textarea, etc.
// — get focus only.) Tests pin both the activation and the focus
// transfer, plus the re-entrancy guard that keeps the re-dispatched
// click from looping back through the label listener.

#[test]
fn click_on_label_wrapping_radio_toggles_checked() {
    // Radio under a wrapping label. Click lands on the label's
    // text area (x=2), not on the input glyph itself, so the only
    // path to toggling the radio is the label re-dispatch.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let label = dom.create_element("label");
    let t = dom.create_text_node("Pick me");
    dom.append_child(label, t).unwrap();
    let input = dom.create_element("input");
    dom.set_attribute(input, "type", "radio").unwrap();
    dom.set_attribute(input, "name", "g").unwrap();
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
    assert!(
        app.dom().node(input).has_attribute("checked"),
        "clicking the label text should toggle the wrapped radio"
    );
    assert_eq!(
        app.dom().focused(),
        Some(input),
        "label click also moves focus"
    );
}

#[test]
fn click_on_label_wrapping_checkbox_toggles_checked() {
    // Checkbox flips on each click (vs. radio which cannot be
    // unchecked by re-clicking). Two clicks = two flips.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let label = dom.create_element("label");
    let t = dom.create_text_node("Agree");
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
    assert!(
        app.dom().node(input).has_attribute("checked"),
        "first click toggles checkbox on"
    );
    click_at(&mut app, 2, 0);
    assert!(
        !app.dom().node(input).has_attribute("checked"),
        "second click toggles checkbox off"
    );
}

#[test]
fn label_text_click_re_dispatches_exactly_once_to_input() {
    // Loop-prevention regression: the label's re-dispatched click
    // bubbles back up through the label (the input is a
    // descendant), so the label listener has a `target != control`
    // skip. If that skip ever regresses, the input's click
    // listener fires N times instead of once and either the radio
    // ends up in the wrong state or the dispatch never terminates.
    // Recording listener on the input must see exactly one click.
    //
    // (Note: every click in rdom is marked `is_synthetic = true`
    // by the mouse router — that flag can't distinguish original
    // from re-dispatched, which is why the loop breaker is
    // `target != control`, not the synthetic flag.)
    use std::cell::Cell;
    use std::rc::Rc;

    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let label = dom.create_element("label");
    let t = dom.create_text_node("Pick me");
    dom.append_child(label, t).unwrap();
    let input = dom.create_element("input");
    dom.set_attribute(input, "type", "radio").unwrap();
    dom.set_attribute(input, "name", "g").unwrap();
    dom.append_child(label, input).unwrap();
    dom.append_child(root, label).unwrap();

    let click_count = Rc::new(Cell::new(0u32));
    let counter = Rc::clone(&click_count);
    dom.add_event_listener(input, "click", ListenerOptions::default(), move |_ctx| {
        counter.set(counter.get() + 1);
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
    assert_eq!(
        click_count.get(),
        1,
        "label text click should re-dispatch exactly one click on the input"
    );
}
