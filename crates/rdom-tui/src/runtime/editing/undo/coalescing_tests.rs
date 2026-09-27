//! Undo grouping end-to-end (`P7-UNDO-COALESCE-1`): keystrokes through
//! `App::handle_event`, history read back through Ctrl-Z / Ctrl-Y.
//!
//! The model is Blink's `TypingCommand` (see `editor_state` module docs):
//! a typing run, a Backspace run and a forward-Delete run each group into
//! one step; a selection change, a switch between those kinds, paste, cut,
//! undo and redo close the group. No word-boundary or timer breaks.

use std::cell::RefCell;
use std::rc::Rc;

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
};
use rdom_core::{InputType, ListenerOptions, NodeId, Position, Selection};

use crate::TuiDom;
use crate::layout::{Display, Size};
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::selection::clipboard::MemoryClipboard;
use crate::style::{Stylesheet, TuiStyle};

fn app_with(text: &str, clipboard: &str) -> (App<TestBackend>, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
    dom.set_attribute(p, "contenteditable", "true").unwrap();
    let t = dom.create_text_node(text);
    dom.append_child(p, t).unwrap();
    dom.append_child(root, p).unwrap();
    let sheet = Stylesheet::bare().rule_unchecked(
        "p",
        TuiStyle::new()
            .display(Display::Block)
            .width(Size::Fixed(40)),
    );
    let terminal = Terminal::new(TestBackend::new(60, 10)).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal)
        .unwrap()
        .with_clipboard(Box::new(MemoryClipboard::with_text(clipboard)));
    app.dom_mut().set_focused(Some(p));
    let end = text.len();
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, end))));
    (app, p, t)
}

fn press(app: &mut App<TestBackend>, code: KeyCode, modifiers: KeyModifiers) {
    app.handle_event(CtEvent::Key(KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    }));
}

fn type_str(app: &mut App<TestBackend>, s: &str) {
    for ch in s.chars() {
        press(app, KeyCode::Char(ch), KeyModifiers::empty());
    }
}

fn undo(app: &mut App<TestBackend>) {
    press(app, KeyCode::Char('z'), KeyModifiers::CONTROL);
}

fn redo(app: &mut App<TestBackend>) {
    press(app, KeyCode::Char('y'), KeyModifiers::CONTROL);
}

fn text(app: &App<TestBackend>, t: NodeId) -> String {
    app.dom().node(t).node_value().unwrap_or("").to_string()
}

#[test]
fn a_typing_run_across_words_is_one_undo_step() {
    // Blink groups a whole typing run, spaces included; there is no
    // word-boundary break in `<input>` / `<textarea>` / contenteditable.
    let (mut app, _p, t) = app_with("", "");
    type_str(&mut app, "hello world");
    assert_eq!(text(&app, t), "hello world");
    undo(&mut app);
    assert_eq!(text(&app, t), "", "one step undoes the whole run");
}

#[test]
fn a_caret_move_between_typing_splits_the_group() {
    let (mut app, _p, t) = app_with("", "");
    type_str(&mut app, "ab");
    press(&mut app, KeyCode::Left, KeyModifiers::empty());
    type_str(&mut app, "X");
    assert_eq!(text(&app, t), "aXb");
    undo(&mut app);
    assert_eq!(text(&app, t), "ab");
    undo(&mut app);
    assert_eq!(text(&app, t), "");
}

#[test]
fn a_caret_round_trip_still_closes_the_group() {
    // Left then Right puts the caret back where typing stopped, but the
    // selection changed in between — Blink closes the typing command on
    // any selection change it did not make itself.
    let (mut app, _p, t) = app_with("", "");
    type_str(&mut app, "ab");
    press(&mut app, KeyCode::Left, KeyModifiers::empty());
    press(&mut app, KeyCode::Right, KeyModifiers::empty());
    type_str(&mut app, "cd");
    undo(&mut app);
    assert_eq!(text(&app, t), "ab");
}

#[test]
fn a_script_selection_change_closes_the_group() {
    let (mut app, _p, t) = app_with("", "");
    type_str(&mut app, "ab");
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 0))));
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 2))));
    type_str(&mut app, "cd");
    undo(&mut app);
    assert_eq!(text(&app, t), "ab");
}

#[test]
fn a_backspace_run_is_one_undo_step() {
    let (mut app, _p, t) = app_with("hello", "");
    for _ in 0..3 {
        press(&mut app, KeyCode::Backspace, KeyModifiers::empty());
    }
    assert_eq!(text(&app, t), "he");
    undo(&mut app);
    assert_eq!(text(&app, t), "hello");
    let sel = app.dom().selection().copied().unwrap();
    assert_eq!(
        sel.focus,
        Position::new(t, 5),
        "undo puts the caret back where the run started"
    );
}

#[test]
fn a_forward_delete_run_groups_apart_from_a_backspace_run() {
    let (mut app, _p, t) = app_with("abcdef", "");
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 3))));
    press(&mut app, KeyCode::Backspace, KeyModifiers::empty());
    press(&mut app, KeyCode::Backspace, KeyModifiers::empty());
    press(&mut app, KeyCode::Delete, KeyModifiers::empty());
    press(&mut app, KeyCode::Delete, KeyModifiers::empty());
    assert_eq!(text(&app, t), "af");
    undo(&mut app);
    assert_eq!(text(&app, t), "adef", "the Delete run is one step");
    undo(&mut app);
    assert_eq!(text(&app, t), "abcdef", "the Backspace run is one step");
}

#[test]
fn switching_between_typing_and_deleting_splits_the_group() {
    let (mut app, _p, t) = app_with("", "");
    type_str(&mut app, "abc");
    press(&mut app, KeyCode::Backspace, KeyModifiers::empty());
    type_str(&mut app, "d");
    assert_eq!(text(&app, t), "abd");
    undo(&mut app);
    assert_eq!(text(&app, t), "ab");
    undo(&mut app);
    assert_eq!(text(&app, t), "abc");
    undo(&mut app);
    assert_eq!(text(&app, t), "");
}

#[test]
fn a_paste_is_its_own_undo_step() {
    let (mut app, _p, t) = app_with("", "XY");
    type_str(&mut app, "ab");
    press(&mut app, KeyCode::Char('v'), KeyModifiers::CONTROL);
    type_str(&mut app, "cd");
    assert_eq!(text(&app, t), "abXYcd");
    undo(&mut app);
    assert_eq!(text(&app, t), "abXY", "typing after a paste opens a group");
    undo(&mut app);
    assert_eq!(text(&app, t), "ab", "the paste is one step");
    undo(&mut app);
    assert_eq!(text(&app, t), "");
}

#[test]
fn a_cut_is_its_own_undo_step() {
    let (mut app, _p, t) = app_with("hello", "");
    press(&mut app, KeyCode::Backspace, KeyModifiers::empty());
    app.dom_mut().set_selection(Some(Selection::new(
        Position::new(t, 0),
        Position::new(t, 2),
    )));
    press(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
    press(&mut app, KeyCode::Delete, KeyModifiers::empty());
    assert_eq!(text(&app, t), "l");
    undo(&mut app);
    assert_eq!(text(&app, t), "ll");
    undo(&mut app);
    assert_eq!(text(&app, t), "hell", "the cut is one step");
}

#[test]
fn typing_over_a_selection_groups_with_the_typing_that_follows() {
    // Blink's typing command deletes the selection and then takes the
    // following keystrokes; one undo brings the selected text back.
    let (mut app, _p, t) = app_with("foo", "");
    app.dom_mut().set_selection(Some(Selection::new(
        Position::new(t, 0),
        Position::new(t, 3),
    )));
    type_str(&mut app, "bar");
    assert_eq!(text(&app, t), "bar");
    undo(&mut app);
    assert_eq!(text(&app, t), "foo");
}

#[test]
fn redo_restores_the_whole_group_and_undo_closes_it() {
    let (mut app, _p, t) = app_with("", "");
    type_str(&mut app, "hey");
    undo(&mut app);
    assert_eq!(text(&app, t), "");
    redo(&mut app);
    assert_eq!(text(&app, t), "hey", "redo re-applies the whole group");
    type_str(&mut app, "!");
    undo(&mut app);
    assert_eq!(text(&app, t), "hey", "typing after redo is a new group");
}

#[test]
fn edits_report_their_input_type() {
    // Input Events §4.1: Delete is deleteContentForward, paste is
    // insertFromPaste, cut is deleteByCut.
    let (mut app, p, t) = app_with("abc", "Z");
    let seen: Rc<RefCell<Vec<InputType>>> = Rc::default();
    let log = seen.clone();
    app.dom_mut()
        .add_event_listener(p, "beforeinput", ListenerOptions::default(), move |ctx| {
            let i = ctx.event.detail.as_input().expect("typed Input detail");
            log.borrow_mut().push(i.input_type.clone());
        })
        .unwrap();
    app.dom_mut()
        .set_selection(Some(Selection::caret(Position::new(t, 0))));
    press(&mut app, KeyCode::Delete, KeyModifiers::empty());
    press(&mut app, KeyCode::Char('v'), KeyModifiers::CONTROL);
    app.dom_mut().set_selection(Some(Selection::new(
        Position::new(t, 0),
        Position::new(t, 1),
    )));
    press(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
    assert_eq!(
        *seen.borrow(),
        vec![
            InputType::DeleteContentForward,
            InputType::InsertFromPaste,
            InputType::DeleteByCut,
        ]
    );
}
