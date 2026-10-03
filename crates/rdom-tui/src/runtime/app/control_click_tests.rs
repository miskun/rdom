//! `EDIT-CLICK-IN-CONTROL-1`: a press inside a text control (or any
//! editing host) puts the caret inside it, and a drag that starts there
//! stays there. Browsers never let a press in a text control start a
//! selection in the page around it — the control's selection is its own.
//!
//! The DOM mirrors the showcase's tab form: a `<label>` beside a control
//! in a flex row. The repro mounted the control after `App::build`, so at
//! click time it had no text node and no inline layout, and the press
//! snapped to the nearest prose — the label — and the next keystroke was
//! lost.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton,
    MouseEvent, MouseEventKind,
};
use rdom_core::{NodeId, NodeType, Position};

use crate::TuiDom;
use crate::layout::{Direction, Flow, Size};
use crate::node::{TuiNodeExt, is_descendant_or_self};
use crate::render::inline::cell_of_position;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::builtins::input;
use crate::style::{Stylesheet, TuiStyle};

fn mouse(app: &mut App<TestBackend>, kind: MouseEventKind, column: u16, row: u16) {
    app.handle_event(CtEvent::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::empty(),
    }));
    app.advance(0).unwrap();
}

fn click(app: &mut App<TestBackend>, column: u16, row: u16) {
    mouse(app, MouseEventKind::Down(MouseButton::Left), column, row);
    mouse(app, MouseEventKind::Up(MouseButton::Left), column, row);
}

fn type_char(app: &mut App<TestBackend>, c: char) {
    app.handle_event(CtEvent::Key(KeyEvent {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }));
    app.advance(0).unwrap();
}

/// Rows are flex rows; a label is 9 cells wide, the control takes the
/// rest (`flex: 1`), a textarea is 3 rows tall.
fn sheet() -> Stylesheet {
    Stylesheet::new()
        .rule_unchecked(
            "div",
            TuiStyle::new().flow(Flow::Flex).direction(Direction::Row),
        )
        .rule_unchecked("label", TuiStyle::new().width(Size::Fixed(9)))
        .rule_unchecked("input", TuiStyle::new().width(Size::Flex(1)))
        .rule_unchecked(
            "textarea",
            TuiStyle::new().width(Size::Flex(1)).height(Size::Fixed(3)),
        )
}

/// A `<div>` row holding `<label>  Name: </label>`, appended to the root.
/// Returns the row.
fn label_row(dom: &mut TuiDom) -> NodeId {
    let root = dom.root();
    let row = dom.create_element("div");
    let label = dom.create_element("label");
    let text = dom.create_text_node("  Name: ");
    dom.append_child(label, text).unwrap();
    dom.append_child(row, label).unwrap();
    dom.append_child(root, row).unwrap();
    row
}

fn app_of(dom: TuiDom) -> App<TestBackend> {
    let terminal = Terminal::new(TestBackend::new(40, 8)).unwrap();
    let mut app = App::with_backend(dom, sheet(), terminal).unwrap();
    app.advance(0).unwrap();
    app
}

/// Label + control in a row; the control is mounted after `App::build`
/// (as a switched-in demo's is) and a frame is drawn.
fn app_with_late_control(tag: &str) -> (App<TestBackend>, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let row = label_row(&mut dom);
    let mut app = app_of(dom);
    let control = app.dom_mut().create_element(tag);
    app.dom_mut().append_child(row, control).unwrap();
    app.advance(0).unwrap();
    (app, control)
}

/// Label + control in a row, present when `App::build` seeds controls.
fn app_with_mounted_input() -> (App<TestBackend>, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let row = label_row(&mut dom);
    let control = dom.create_element("input");
    dom.append_child(row, control).unwrap();
    (app_of(dom), control)
}

/// A cell inside `id`'s border box, `dx` cells from its left edge.
fn cell_in(app: &App<TestBackend>, id: NodeId, dx: u16) -> (u16, u16) {
    let r = app.dom().node(id).layout_rect().unwrap();
    assert!(r.width > dx, "control too narrow for the test: {r:?}");
    (r.x as u16 + dx, r.y as u16)
}

/// The selection is a caret on a text node inside `control`, and the
/// caret's cell lies inside `control`'s box.
fn assert_caret_inside(app: &App<TestBackend>, control: NodeId) -> Position {
    let dom = app.dom();
    let sel = *dom.selection().expect("the press sets a selection");
    assert!(sel.is_collapsed(), "a click leaves a caret: {sel:?}");
    let node = sel.focus.node;
    assert_eq!(dom.node(node).node_type(), NodeType::Text);
    assert!(
        is_descendant_or_self(dom, node, control),
        "the caret must be in the control's own text, not the label's: {sel:?}"
    );
    let (x, y) = cell_of_position(dom, sel.focus).expect("the caret has a cell");
    let r = dom.node(control).layout_rect().unwrap();
    assert!(
        (x as i32) >= r.x
            && (x as i32) < r.x + r.width as i32
            && (y as i32) >= r.y
            && (y as i32) < r.y + r.height as i32,
        "the caret cell ({x}, {y}) must lie inside the control's box {r:?}"
    );
    sel.focus
}

#[test]
fn a_click_into_an_input_mounted_after_build_puts_the_caret_inside_it() {
    let (mut app, control) = app_with_late_control("input");
    let (x, y) = cell_in(&app, control, 2);
    click(&mut app, x, y);
    assert_eq!(app.dom().focused(), Some(control));
    assert_caret_inside(&app, control);
    type_char(&mut app, 'Z');
    assert_eq!(input::value(app.dom(), control), "Z", "the keystroke lands");
}

#[test]
fn a_click_into_an_input_seeded_at_build_puts_the_caret_inside_it() {
    let (mut app, control) = app_with_mounted_input();
    let (x, y) = cell_in(&app, control, 2);
    click(&mut app, x, y);
    assert_eq!(app.dom().focused(), Some(control));
    assert_caret_inside(&app, control);
    type_char(&mut app, 'Z');
    assert_eq!(input::value(app.dom(), control), "Z");
}

#[test]
fn a_click_into_a_textarea_mounted_after_build_puts_the_caret_inside_it() {
    let (mut app, control) = app_with_late_control("textarea");
    let (x, y) = cell_in(&app, control, 2);
    // The middle row of the 3-row textarea, below its (empty) first line.
    click(&mut app, x, y + 1);
    assert_eq!(app.dom().focused(), Some(control));
    assert_caret_inside(&app, control);
    type_char(&mut app, 'Z');
    assert_eq!(input::value(app.dom(), control), "Z");
}

#[test]
fn a_drag_from_an_input_over_its_label_stays_in_the_input() {
    let (mut app, control) = app_with_late_control("input");
    let (x, y) = cell_in(&app, control, 2);
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), x, y);
    // Over the label's text, left of the input.
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), 3, y);
    let sel = *app.dom().selection().expect("a selection");
    for end in [sel.anchor, sel.focus] {
        assert!(
            is_descendant_or_self(app.dom(), end.node, control),
            "both ends stay in the input: {sel:?}"
        );
    }
    mouse(&mut app, MouseEventKind::Up(MouseButton::Left), 3, y);
    let sel = *app.dom().selection().expect("a selection");
    assert!(is_descendant_or_self(app.dom(), sel.focus.node, control));
}
