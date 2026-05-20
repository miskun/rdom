//! Integration tests — undo/redo end-to-end through `perform_edit`
//! and `App::handle_event`.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
};
use rdom_core::{NodeId, Position, Selection};

use crate::TuiDom;
use crate::layout::{Display, Size};
use crate::render::{LayoutExt, Rect, Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::editing::{Edit, EditOutcome, UndoOutcome, perform_edit, redo_last, undo_last};
use crate::style::{CascadeExt, Stylesheet, TuiStyle};

fn editable_paragraph(text: &str) -> (TuiDom, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "contenteditable", "true").unwrap();
    let t = dom.create_text_node(text);
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(40)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    dom.cascade(&sheet);
    dom.layout_dom(Rect::new(0, 0, 60, 10));
    (dom, p, t)
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }
}

// ── Undo/redo API directly ────────────────────────────────────────

#[test]
fn undo_reverses_last_insert() {
    let (mut dom, p, t) = editable_paragraph("hello");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, 5))));

    perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 5..5,
            text: "!".to_string(),
        },
    );
    assert_eq!(dom.node(t).node_value(), Some("hello!"));

    let outcome = undo_last(&mut dom);
    assert_eq!(outcome, UndoOutcome::Applied);
    assert_eq!(dom.node(t).node_value(), Some("hello"));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 5));
}

#[test]
fn redo_reapplies_reversed_insert() {
    let (mut dom, p, t) = editable_paragraph("hello");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, 5))));

    perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 5..5,
            text: "!".to_string(),
        },
    );
    undo_last(&mut dom);
    assert_eq!(dom.node(t).node_value(), Some("hello"));

    let outcome = redo_last(&mut dom);
    assert_eq!(outcome, UndoOutcome::Applied);
    assert_eq!(dom.node(t).node_value(), Some("hello!"));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 6));
}

#[test]
fn undo_reverses_delete() {
    let (mut dom, p, t) = editable_paragraph("hello");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, 5))));

    perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 4..5,
            text: String::new(),
        },
    );
    assert_eq!(dom.node(t).node_value(), Some("hell"));

    undo_last(&mut dom);
    assert_eq!(dom.node(t).node_value(), Some("hello"));
}

#[test]
fn undo_reverses_replace() {
    let (mut dom, p, t) = editable_paragraph("hello");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::new(
        Position::new(t, 0),
        Position::new(t, 5),
    )));

    perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 0..5,
            text: "WORLD".to_string(),
        },
    );
    assert_eq!(dom.node(t).node_value(), Some("WORLD"));

    undo_last(&mut dom);
    assert_eq!(dom.node(t).node_value(), Some("hello"));
}

#[test]
fn multiple_undos_pop_entries_in_reverse_order() {
    let (mut dom, p, t) = editable_paragraph("");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, 0))));

    // Three separate inserts with deletes between them so nothing
    // coalesces.
    perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 0..0,
            text: "a".to_string(),
        },
    );
    perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 0..1,
            text: "b".to_string(),
        }, // Replace
    );
    perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 0..1,
            text: "c".to_string(),
        }, // Replace
    );
    assert_eq!(dom.node(t).node_value(), Some("c"));

    undo_last(&mut dom);
    assert_eq!(dom.node(t).node_value(), Some("b"));
    undo_last(&mut dom);
    assert_eq!(dom.node(t).node_value(), Some("a"));
    undo_last(&mut dom);
    assert_eq!(dom.node(t).node_value(), Some(""));

    let outcome = undo_last(&mut dom);
    assert_eq!(outcome, UndoOutcome::Noop, "empty stack is a no-op");
}

#[test]
fn new_edit_after_undo_clears_redo_stack() {
    let (mut dom, p, t) = editable_paragraph("hello");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, 5))));

    perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 5..5,
            text: "!".to_string(),
        },
    );
    undo_last(&mut dom); // redo stack now has the "!" entry

    // Perform a new edit — should discard the redo'd future.
    perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 5..5,
            text: "?".to_string(),
        },
    );
    assert_eq!(dom.node(t).node_value(), Some("hello?"));

    // Redo should do nothing — the old future was wiped.
    let outcome = redo_last(&mut dom);
    assert_eq!(outcome, UndoOutcome::Noop);
}

#[test]
fn undo_without_editable_focus_is_noop() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    let t = dom.create_text_node("hello");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();
    // NOT contenteditable.
    dom.set_focused(Some(p));

    let outcome = undo_last(&mut dom);
    assert_eq!(outcome, UndoOutcome::Noop);
}

// ── Undo/redo fires `input` but not `beforeinput` ────────────────

#[test]
fn undo_fires_input_but_not_beforeinput() {
    use rdom_core::ListenerOptions;
    use std::cell::RefCell;
    use std::rc::Rc;

    let (mut dom, p, t) = editable_paragraph("hello");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, 5))));

    let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    for ty in ["beforeinput", "input"] {
        let log = log.clone();
        let ty_str = ty.to_string();
        dom.add_event_listener(p, ty, ListenerOptions::default(), move |_| {
            log.borrow_mut().push(ty_str.clone());
        })
        .unwrap();
    }

    perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 5..5,
            text: "!".to_string(),
        },
    );
    log.borrow_mut().clear();

    undo_last(&mut dom);
    let events: Vec<String> = log.borrow().clone();
    assert_eq!(
        events,
        vec!["input".to_string()],
        "undo fires `input` but NOT `beforeinput`"
    );
}

// ── Coalescing end-to-end ─────────────────────────────────────────

#[test]
fn rapid_character_inserts_undo_as_single_chunk() {
    let (mut dom, p, t) = editable_paragraph("");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, 0))));

    // Three char inserts in quick succession — should coalesce
    // because each `perform_edit` call happens well under 500ms
    // after the previous in a synchronous test.
    for (i, ch) in ['h', 'e', 'y'].iter().enumerate() {
        perform_edit(
            &mut dom,
            Edit {
                node: t,
                range: i..i,
                text: ch.to_string(),
            },
        );
    }
    assert_eq!(dom.node(t).node_value(), Some("hey"));

    undo_last(&mut dom);
    assert_eq!(
        dom.node(t).node_value(),
        Some(""),
        "single undo should erase all three coalesced chars"
    );
}

// ── End-to-end via App::handle_event (Ctrl-Z / Ctrl-Y) ────────────

fn test_app(dom: TuiDom, sheet: Stylesheet, viewport: Rect) -> App<TestBackend> {
    let backend = TestBackend::new(viewport.width, viewport.height);
    let terminal = Terminal::new(backend).unwrap();
    App::with_backend(dom, sheet, terminal).unwrap()
}

#[test]
fn ctrl_z_undoes_last_edit_via_app() {
    let (mut dom, p, t) = editable_paragraph("hello");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, 5))));

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(40)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    let mut app = test_app(dom, sheet, Rect::new(0, 0, 60, 10));

    app.handle_event(CtEvent::Key(key(KeyCode::Char('!'), KeyModifiers::empty())));
    assert_eq!(app.dom().node(t).node_value(), Some("hello!"));

    app.handle_event(CtEvent::Key(key(KeyCode::Char('z'), KeyModifiers::CONTROL)));
    assert_eq!(app.dom().node(t).node_value(), Some("hello"));
}

#[test]
fn ctrl_y_redoes_via_app() {
    let (mut dom, p, t) = editable_paragraph("hello");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, 5))));

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(40)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    let mut app = test_app(dom, sheet, Rect::new(0, 0, 60, 10));

    app.handle_event(CtEvent::Key(key(KeyCode::Char('!'), KeyModifiers::empty())));
    app.handle_event(CtEvent::Key(key(KeyCode::Char('z'), KeyModifiers::CONTROL)));
    assert_eq!(app.dom().node(t).node_value(), Some("hello"));

    app.handle_event(CtEvent::Key(key(KeyCode::Char('y'), KeyModifiers::CONTROL)));
    assert_eq!(app.dom().node(t).node_value(), Some("hello!"));
}

#[test]
fn ctrl_shift_z_redoes_via_app() {
    // Mac-style redo: Cmd-Shift-Z. We accept Ctrl-Shift-Z too.
    let (mut dom, p, t) = editable_paragraph("hello");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, 5))));

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(40)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    let mut app = test_app(dom, sheet, Rect::new(0, 0, 60, 10));

    app.handle_event(CtEvent::Key(key(KeyCode::Char('!'), KeyModifiers::empty())));
    app.handle_event(CtEvent::Key(key(KeyCode::Char('z'), KeyModifiers::CONTROL)));

    app.handle_event(CtEvent::Key(key(
        KeyCode::Char('z'),
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    )));
    assert_eq!(app.dom().node(t).node_value(), Some("hello!"));
}

#[test]
fn bare_z_still_types_when_editable_focused() {
    // Sanity check that Ctrl gating didn't accidentally swallow
    // the bare letter.
    let (mut dom, p, t) = editable_paragraph("");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, 0))));

    let sheet = Stylesheet::bare()
        .rule_unchecked(
            "p",
            TuiStyle::new()
                .display(Display::Block)
                .width(Size::Fixed(40)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    let mut app = test_app(dom, sheet, Rect::new(0, 0, 60, 10));

    app.handle_event(CtEvent::Key(key(KeyCode::Char('z'), KeyModifiers::empty())));
    assert_eq!(app.dom().node(t).node_value(), Some("z"));
}

#[test]
fn perform_edit_passes_outcome_after_edit() {
    // Making sure the returned EditOutcome still surfaces as
    // `Applied` after B.4's extra record-on-state work.
    let (mut dom, p, t) = editable_paragraph("a");
    dom.set_focused(Some(p));
    dom.set_selection(Some(Selection::caret(Position::new(t, 1))));

    let outcome = perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 1..1,
            text: "b".to_string(),
        },
    );
    assert_eq!(outcome, EditOutcome::Applied);
}
