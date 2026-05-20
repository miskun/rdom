//! B.2 tests — `perform_edit` + `insert_at_selection` +
//! end-to-end character insert via `App::handle_event`.
//!
//! Covers the full contract: beforeinput fires, handlers can
//! prevent, mutation happens, input fires, caret moves,
//! selection-aware replace works.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
};
use rdom_core::{ListenerOptions, NodeId, Position, Selection};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::TuiDom;
use crate::layout::{Display, Size};
use crate::render::{LayoutExt, Rect, Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::editing::{Edit, EditOutcome, insert_at_selection, perform_edit};
use crate::style::{CascadeExt, Stylesheet, TuiStyle};

// ── Fixtures ────────────────────────────────────────────────────────

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

// ── perform_edit — pure API ─────────────────────────────────────────

#[test]
fn perform_edit_inserts_text_and_moves_caret() {
    let (mut dom, _p, t) = editable_paragraph("hello");

    let outcome = perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 5..5,
            text: "!".to_string(),
        },
    );

    assert_eq!(outcome, EditOutcome::Applied);
    assert_eq!(dom.node(t).node_value(), Some("hello!"));
    // Caret ends at byte 6 (end of insert).
    let sel = dom.selection().unwrap();
    assert!(sel.is_collapsed());
    assert_eq!(sel.focus, Position::new(t, 6));
}

#[test]
fn perform_edit_replaces_range_with_text() {
    let (mut dom, _p, t) = editable_paragraph("hello");

    perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 1..4,
            text: "EY".to_string(),
        },
    );

    assert_eq!(dom.node(t).node_value(), Some("hEYo"));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 3)); // h + EY = 3 bytes
}

#[test]
fn perform_edit_empty_replacement_deletes_range() {
    let (mut dom, _p, t) = editable_paragraph("hello");

    perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 1..4,
            text: String::new(),
        },
    );

    assert_eq!(dom.node(t).node_value(), Some("ho"));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 1));
}

#[test]
fn perform_edit_fires_beforeinput_and_input_events_in_order() {
    let (mut dom, p, t) = editable_paragraph("ab");
    let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

    for event_type in ["beforeinput", "input"] {
        let log = log.clone();
        let ty = event_type.to_string();
        dom.add_event_listener(p, event_type, ListenerOptions::default(), move |ctx| {
            let data = ctx
                .event
                .detail
                .as_input()
                .and_then(|i| i.data.as_deref())
                .unwrap_or("");
            log.borrow_mut().push(format!("{ty}:{data}"));
        })
        .unwrap();
    }

    perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 2..2,
            text: "c".to_string(),
        },
    );

    assert_eq!(
        *log.borrow(),
        vec!["beforeinput:c".to_string(), "input:c".to_string()]
    );
}

// Tuple captured per typed-input-detail test: (input_type, data).
type CapturedInputDetail = Rc<RefCell<Option<(rdom_core::InputType, Option<String>)>>>;
// Log of (event_type, input_type, data) tuples.
type CapturedInputLog = Rc<RefCell<Vec<(String, rdom_core::InputType, Option<String>)>>>;

#[test]
fn perform_edit_input_event_carries_typed_input_detail() {
    // Canonical step-4 failing test: an insertion edit produces
    // `beforeinput` and `input` events with `EventDetail::Input`
    // carrying `InputType::InsertText` + `data = Some("c")`.
    use rdom_core::InputType;
    let (mut dom, p, t) = editable_paragraph("ab");

    let captured: CapturedInputLog = Rc::new(RefCell::new(Vec::new()));
    for event_type in ["beforeinput", "input"] {
        let captured = captured.clone();
        let ty = event_type.to_string();
        dom.add_event_listener(p, event_type, ListenerOptions::default(), move |ctx| {
            let i = ctx
                .event
                .detail
                .as_input()
                .expect("input/beforeinput must carry EventDetail::Input");
            captured
                .borrow_mut()
                .push((ty.clone(), i.input_type.clone(), i.data.clone()));
        })
        .unwrap();
    }

    perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 2..2,
            text: "c".to_string(),
        },
    );

    let log = captured.borrow();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].0, "beforeinput");
    assert_eq!(log[0].1, InputType::InsertText);
    assert_eq!(log[0].2.as_deref(), Some("c"));
    assert_eq!(log[1].0, "input");
    assert_eq!(log[1].1, InputType::InsertText);
    assert_eq!(log[1].2.as_deref(), Some("c"));
}

#[test]
fn perform_edit_delete_carries_delete_input_type_and_null_data() {
    // Pure-delete edits (empty `text`, non-empty `range`) emit
    // InputType::DeleteContentBackward + data: None, per the web
    // DOM `InputEvent` convention.
    use rdom_core::InputType;
    let (mut dom, p, t) = editable_paragraph("hello");

    let captured: CapturedInputDetail = Rc::new(RefCell::new(None));
    {
        let captured = captured.clone();
        dom.add_event_listener(p, "input", ListenerOptions::default(), move |ctx| {
            let i = ctx.event.detail.as_input().expect("typed Input detail");
            *captured.borrow_mut() = Some((i.input_type.clone(), i.data.clone()));
        })
        .unwrap();
    }

    perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 1..3,
            text: String::new(),
        },
    );

    let r = captured.borrow();
    let (input_type, data) = r.as_ref().expect("listener fired");
    assert_eq!(input_type, &InputType::DeleteContentBackward);
    assert!(data.is_none());
}

#[test]
fn perform_edit_replace_carries_insert_replacement_text_input_type() {
    // Non-empty range + non-empty text = replace; reports
    // `InsertReplacementText`. Data carries the new text.
    use rdom_core::InputType;
    let (mut dom, p, t) = editable_paragraph("hello");

    let captured: CapturedInputDetail = Rc::new(RefCell::new(None));
    {
        let captured = captured.clone();
        dom.add_event_listener(p, "input", ListenerOptions::default(), move |ctx| {
            let i = ctx.event.detail.as_input().expect("typed Input detail");
            *captured.borrow_mut() = Some((i.input_type.clone(), i.data.clone()));
        })
        .unwrap();
    }

    perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 1..4,
            text: "X".to_string(),
        },
    );

    let r = captured.borrow();
    let (input_type, data) = r.as_ref().expect("listener fired");
    assert_eq!(input_type, &InputType::InsertReplacementText);
    assert_eq!(data.as_deref(), Some("X"));
}

#[test]
fn perform_edit_beforeinput_prevent_default_blocks_mutation() {
    let (mut dom, p, t) = editable_paragraph("ab");
    dom.add_event_listener(p, "beforeinput", ListenerOptions::default(), |ctx| {
        ctx.event.prevent_default();
    })
    .unwrap();

    let input_fired = Rc::new(Cell::new(false));
    let fl = input_fired.clone();
    dom.add_event_listener(p, "input", ListenerOptions::default(), move |_| {
        fl.set(true);
    })
    .unwrap();

    let outcome = perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 2..2,
            text: "x".to_string(),
        },
    );

    assert_eq!(outcome, EditOutcome::Prevented);
    // DOM unchanged.
    assert_eq!(dom.node(t).node_value(), Some("ab"));
    // Input did NOT fire.
    assert!(!input_fired.get());
}

#[test]
fn perform_edit_no_editable_target_is_noop() {
    // A text node outside any contenteditable subtree.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p"); // NOT contenteditable
    let t = dom.create_text_node("hello");
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();

    let outcome = perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 0..0,
            text: "x".to_string(),
        },
    );

    assert_eq!(outcome, EditOutcome::NoEditableTarget);
    assert_eq!(dom.node(t).node_value(), Some("hello"));
}

// ── insert_at_selection ────────────────────────────────────────────

#[test]
fn insert_at_selection_uses_caret_when_collapsed() {
    let (mut dom, _p, t) = editable_paragraph("hello");
    dom.set_selection(Some(Selection::caret(Position::new(t, 3))));

    let outcome = insert_at_selection(&mut dom, "-");
    assert_eq!(outcome, EditOutcome::Applied);
    assert_eq!(dom.node(t).node_value(), Some("hel-lo"));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 4));
}

#[test]
fn insert_at_selection_replaces_range_when_selection_non_collapsed() {
    let (mut dom, _p, t) = editable_paragraph("hello world");
    dom.set_selection(Some(Selection::new(
        Position::new(t, 6),
        Position::new(t, 11),
    )));

    insert_at_selection(&mut dom, "there");
    assert_eq!(dom.node(t).node_value(), Some("hello there"));
    assert_eq!(dom.selection().unwrap().focus, Position::new(t, 11));
}

#[test]
fn insert_at_selection_backward_selection_still_replaces() {
    // anchor > focus (backward drag). Should still replace the
    // ordered range.
    let (mut dom, _p, t) = editable_paragraph("hello world");
    dom.set_selection(Some(Selection::new(
        Position::new(t, 11),
        Position::new(t, 6),
    )));

    insert_at_selection(&mut dom, "there");
    assert_eq!(dom.node(t).node_value(), Some("hello there"));
}

#[test]
fn insert_at_selection_applies_across_text_nodes_in_same_editable() {
    // C1: cross-text-node selections inside a single contenteditable
    // host commit through `perform_cross_node_edit`. The covered
    // range from both nodes is deleted and the replacement lands at
    // the document-order earlier endpoint.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "contenteditable", "true").unwrap();
    let t1 = dom.create_text_node("hello ");
    dom.append_child(p, t1).unwrap();
    let code = dom.create_element("code");
    let t2 = dom.create_text_node("world");
    dom.append_child(code, t2).unwrap();
    dom.append_child(p, code).unwrap();
    dom.append_child(root, p).unwrap();
    dom.set_selection(Some(Selection::new(
        Position::new(t1, 2),
        Position::new(t2, 3),
    )));

    let outcome = insert_at_selection(&mut dom, "x");
    assert_eq!(outcome, EditOutcome::Applied);
    // t1: keep "he", drop " " (offsets 2..6), append "x" → "hex".
    // t2: drop "wor" (offsets 0..3) → "ld".
    assert_eq!(dom.node(t1).node_value(), Some("hex"));
    assert_eq!(dom.node(t2).node_value(), Some("ld"));
    // Caret collapses at end-of-insertion in t1.
    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t1, 3));
    assert!(sel.is_collapsed());
}

// ── End-to-end via App::handle_event ────────────────────────────────

fn test_app(dom: TuiDom, sheet: Stylesheet, viewport: Rect) -> App<TestBackend> {
    let backend = TestBackend::new(viewport.width, viewport.height);
    let terminal = Terminal::new(backend).unwrap();
    App::with_backend(dom, sheet, terminal).unwrap()
}

fn key_press(c: char) -> CtEvent {
    CtEvent::Key(KeyEvent {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

#[test]
fn typing_printable_char_on_focused_editable_inserts_at_caret() {
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

    app.handle_event(key_press('!'));

    assert_eq!(app.dom().node(t).node_value(), Some("hello!"));
}

#[test]
fn typing_on_non_editable_focus_does_not_mutate_dom() {
    // Focused but NOT contenteditable. Typing should not insert.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "tabindex", "0").unwrap();
    let t = dom.create_text_node("hello");
    dom.append_child(p, t).unwrap();
    let span = dom.create_element("span");
    dom.append_child(p, span).unwrap();
    dom.append_child(root, p).unwrap();
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

    app.handle_event(key_press('!'));
    assert_eq!(app.dom().node(t).node_value(), Some("hello"));
}

#[test]
fn ctrl_combo_is_not_intercepted_as_character_insert() {
    // Ctrl-A (select-all) should NOT insert an 'a' — it hits the
    // selection-keyboard path first.
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

    app.handle_event(CtEvent::Key(KeyEvent {
        code: KeyCode::Char('a'),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }));

    // Text unchanged.
    assert_eq!(app.dom().node(t).node_value(), Some("hello"));
    // Selection is now the whole text (Ctrl-A did its job).
    let sel = app.dom().selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 0));
    assert_eq!(sel.focus, Position::new(t, 5));
}

// ── C.4a: readonly + value-attribute mirror ───────────────────────

/// Build a parented `<input>` (with seeded text child) ready for
/// edit-pipeline tests. Returns (dom, input_id, text_id).
fn editable_input(initial: &str) -> (TuiDom, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.set_attribute(input, "value", initial).unwrap();
    dom.append_child(root, input).unwrap();
    crate::runtime::builtins::input::seed_all(&mut dom);
    let t = dom
        .node(input)
        .child_nodes()
        .next()
        .map(|c| c.id())
        .unwrap();
    (dom, input, t)
}

#[test]
fn readonly_input_blocks_perform_edit_with_prevented_outcome() {
    let (mut dom, input, t) = editable_input("hello");
    dom.set_attribute(input, "readonly", "").unwrap();

    let outcome = perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 5..5,
            text: "!".to_string(),
        },
    );
    assert_eq!(outcome, EditOutcome::Prevented);
    assert_eq!(dom.node(t).node_value(), Some("hello"));
    assert_eq!(dom.node(input).get_attribute("value"), Some("hello"));
}

#[test]
fn readonly_fires_beforeinput_then_ua_cancels_without_input_event() {
    // Per UI Events / Input Events Level 2: `readonly` form fields
    // still dispatch `beforeinput` (cancelable). The UA's default
    // action cancels the edit, so no mutation lands and no `input`
    // event fires. Listeners can observe the attempted edit
    // (useful for analytics, validation feedback, etc.) — the
    // earlier short-circuit hid that signal entirely.
    let (mut dom, input, t) = editable_input("x");
    dom.set_attribute(input, "readonly", "").unwrap();

    let fires = Rc::new(RefCell::new(Vec::<String>::new()));
    for ty in ["beforeinput", "input"] {
        let f = fires.clone();
        let label = ty.to_string();
        dom.add_event_listener(input, ty, ListenerOptions::default(), move |_| {
            f.borrow_mut().push(label.clone());
        })
        .unwrap();
    }

    let outcome = perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 1..1,
            text: "y".to_string(),
        },
    );

    let calls = fires.borrow().clone();
    assert_eq!(
        calls,
        vec!["beforeinput".to_string()],
        "readonly must fire beforeinput exactly once (no input event); got {:?}",
        calls
    );
    assert_eq!(outcome, EditOutcome::Prevented);
    assert_eq!(dom.node(t).node_value(), Some("x"));
}

#[test]
fn readonly_beforeinput_listener_can_observe_input_type_and_data() {
    // Sanity check: the beforeinput we fire on readonly carries
    // the same InputType + data as a regular edit, so listeners
    // can introspect the rejected attempt.
    let (mut dom, input, t) = editable_input("x");
    dom.set_attribute(input, "readonly", "").unwrap();

    let captured = Rc::new(RefCell::new(None::<(rdom_core::InputType, Option<String>)>));
    let c = captured.clone();
    dom.add_event_listener(
        input,
        "beforeinput",
        ListenerOptions::default(),
        move |ctx| {
            if let rdom_core::EventDetail::Input(detail) = &ctx.event.detail {
                *c.borrow_mut() = Some((detail.input_type.clone(), detail.data.clone()));
            }
        },
    )
    .unwrap();

    let _ = perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 1..1,
            text: "y".to_string(),
        },
    );

    let captured = captured.borrow().clone();
    assert_eq!(
        captured,
        Some((rdom_core::InputType::InsertText, Some("y".to_string())))
    );
}

#[test]
fn input_value_attribute_mirrors_text_after_edit() {
    let (mut dom, input, t) = editable_input("ab");

    let outcome = perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 2..2,
            text: "c".to_string(),
        },
    );
    assert_eq!(outcome, EditOutcome::Applied);
    assert_eq!(dom.node(input).get_attribute("value"), Some("abc"));
}

// ── C1: cross-text-node edits in contenteditable ───────────────

/// Build a contenteditable `<p>` containing two text nodes
/// straddling a `<b>` boundary: `<p>abc<b>xyz</b></p>`. Returns
/// (dom, p, t1, t2).
fn contenteditable_with_two_text_nodes() -> (TuiDom, NodeId, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "contenteditable", "true").unwrap();
    let t1 = dom.create_text_node("abc");
    dom.append_child(p, t1).unwrap();
    let b = dom.create_element("b");
    let t2 = dom.create_text_node("xyz");
    dom.append_child(b, t2).unwrap();
    dom.append_child(p, b).unwrap();
    dom.append_child(root, p).unwrap();
    (dom, p, t1, t2)
}

#[test]
fn cross_node_typing_replaces_selection_across_two_text_nodes() {
    // Browser behavior: selecting from one text node to another
    // (here, across a <b> boundary) and typing should delete the
    // covered range AND insert the replacement at the anchor.
    // The pre-C1 MVP silently dropped these keystrokes.
    let (mut dom, _p, t1, t2) = contenteditable_with_two_text_nodes();

    // Select from t1[1] to t2[2]: covers "bc" + "xy".
    dom.set_selection(Some(Selection::new(
        Position::new(t1, 1),
        Position::new(t2, 2),
    )));

    let outcome = insert_at_selection(&mut dom, "Q");
    assert_eq!(outcome, EditOutcome::Applied);

    // t1's tail removed + replacement appended; t2's head removed.
    assert_eq!(dom.node(t1).node_value(), Some("aQ"));
    assert_eq!(dom.node(t2).node_value(), Some("z"));

    // Caret collapses to end-of-insertion in the anchor node.
    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t1, 2));
    assert!(sel.is_collapsed());
}

#[test]
fn cross_node_pure_delete_via_empty_replacement() {
    // Same setup; insert "" — verifies the delete-only path.
    let (mut dom, _p, t1, t2) = contenteditable_with_two_text_nodes();
    dom.set_selection(Some(Selection::new(
        Position::new(t1, 1),
        Position::new(t2, 2),
    )));

    let outcome = insert_at_selection(&mut dom, "");
    assert_eq!(outcome, EditOutcome::Applied);
    assert_eq!(dom.node(t1).node_value(), Some("a"));
    assert_eq!(dom.node(t2).node_value(), Some("z"));

    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t1, 1));
    assert!(sel.is_collapsed());
}

#[test]
fn cross_node_edit_fires_beforeinput_once_then_input_once() {
    // Single beforeinput + single input per logical edit, even
    // when multiple text nodes are mutated under the hood.
    let (mut dom, p, t1, t2) = contenteditable_with_two_text_nodes();
    dom.set_selection(Some(Selection::new(
        Position::new(t1, 1),
        Position::new(t2, 2),
    )));

    let fires = Rc::new(RefCell::new(Vec::<String>::new()));
    for ty in ["beforeinput", "input"] {
        let f = fires.clone();
        let label = ty.to_string();
        dom.add_event_listener(p, ty, ListenerOptions::default(), move |_| {
            f.borrow_mut().push(label.clone());
        })
        .unwrap();
    }

    let _ = insert_at_selection(&mut dom, "Q");
    assert_eq!(
        fires.borrow().as_slice(),
        ["beforeinput".to_string(), "input".to_string()],
    );
}

#[test]
fn cross_node_edit_three_node_selection_clears_intermediate_text_nodes() {
    // Selection spans `<p>{abc}<b>{xyz}</b>{end}</p>` from t1[1]
    // to t3[2]: t1's tail and t3's head are partial-trimmed, t2
    // is fully covered and must be cleared.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "contenteditable", "true").unwrap();
    let t1 = dom.create_text_node("abc");
    dom.append_child(p, t1).unwrap();
    let b = dom.create_element("b");
    let t2 = dom.create_text_node("xyz");
    dom.append_child(b, t2).unwrap();
    dom.append_child(p, b).unwrap();
    let t3 = dom.create_text_node("end");
    dom.append_child(p, t3).unwrap();
    dom.append_child(root, p).unwrap();

    dom.set_selection(Some(Selection::new(
        Position::new(t1, 1),
        Position::new(t3, 2),
    )));

    let outcome = insert_at_selection(&mut dom, "Q");
    assert_eq!(outcome, EditOutcome::Applied);
    assert_eq!(dom.node(t1).node_value(), Some("aQ"));
    assert_eq!(
        dom.node(t2).node_value(),
        Some(""),
        "intermediate text node must be cleared"
    );
    assert_eq!(dom.node(t3).node_value(), Some("d"));

    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t1, 2));
}

#[test]
fn cross_node_edit_with_reverse_order_anchor_after_focus() {
    // Browsers preserve direction in Selection (anchor / focus aren't
    // necessarily in document order). A cross-node edit must order
    // the endpoints by document position internally and apply the
    // replacement at the document-order earlier endpoint regardless
    // of which one the user "anchored."
    let (mut dom, _p, t1, t2) = contenteditable_with_two_text_nodes();

    // Anchor in t2 (later in doc order), focus in t1.
    dom.set_selection(Some(Selection::new(
        Position::new(t2, 2),
        Position::new(t1, 1),
    )));

    let outcome = insert_at_selection(&mut dom, "Q");
    assert_eq!(outcome, EditOutcome::Applied);
    // Result identical to forward-ordered selection:
    // t1 = "aQ", t2 = "z".
    assert_eq!(dom.node(t1).node_value(), Some("aQ"));
    assert_eq!(dom.node(t2).node_value(), Some("z"));

    // Caret lands at end-of-insertion in t1 (document-order start).
    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t1, 2));
}

#[test]
fn cross_node_edit_respects_beforeinput_prevent_default() {
    // A handler that cancels beforeinput suppresses the entire
    // cross-node edit — nothing mutates, outcome is Prevented.
    let (mut dom, p, t1, t2) = contenteditable_with_two_text_nodes();
    dom.set_selection(Some(Selection::new(
        Position::new(t1, 1),
        Position::new(t2, 2),
    )));

    dom.add_event_listener(p, "beforeinput", ListenerOptions::default(), |ctx| {
        ctx.event.prevent_default();
    })
    .unwrap();

    let outcome = insert_at_selection(&mut dom, "Q");
    assert_eq!(outcome, EditOutcome::Prevented);
    assert_eq!(dom.node(t1).node_value(), Some("abc"));
    assert_eq!(dom.node(t2).node_value(), Some("xyz"));
}

#[test]
fn textarea_does_not_get_value_attribute_written() {
    // Textarea's value is its text content — there's no `value`
    // attribute to mirror. Verifying we don't accidentally start
    // writing one.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let ta = dom.create_element("textarea");
    let t = dom.create_text_node("hello");
    dom.append_child(ta, t).unwrap();
    dom.append_child(root, ta).unwrap();

    let _ = perform_edit(
        &mut dom,
        Edit {
            node: t,
            range: 5..5,
            text: "!".to_string(),
        },
    );

    assert_eq!(dom.node(t).node_value(), Some("hello!"));
    assert_eq!(dom.node(ta).get_attribute("value"), None);
}
