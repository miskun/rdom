//! Multi-click selection tests — exercise the snap semantics
//! (`expand_to_word`, `expand_to_line`) directly, plus the full
//! router-level round-trip where two fast `mousedown`s promote to
//! a word-select.

use crossterm::event::{
    Event as CtEvent, KeyModifiers, MouseButton, MouseEvent as CtMouseEvent, MouseEventKind,
};
use rdom_core::{NodeId, Position, Selection};

use crate::TuiDom;
use crate::layout::{Display, Size};
use crate::render::{LayoutExt, Rect};
use crate::runtime::router::Router;
use crate::runtime::selection::multiclick::{expand_to_line, expand_to_word};
use crate::style::{CascadeExt, Stylesheet, TuiStyle};

// ── Fixtures ────────────────────────────────────────────────────────

fn prepare(dom: &mut TuiDom, sheet: &Stylesheet, viewport: Rect) {
    dom.cascade(sheet);
    dom.layout_dom(viewport);
}

/// `<p>{text}<span/></p>` — the span makes the `<p>` an IFC.
fn paragraph(text: &str, width: u16) -> (TuiDom, NodeId, NodeId) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let p = dom.create_element("p");
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
                .width(Size::Fixed(width)),
        )
        .rule_unchecked("span", TuiStyle::new().display(Display::Inline));
    prepare(&mut dom, &sheet, Rect::new(0, 0, width + 10, 10));
    (dom, p, t)
}

fn down_at(x: u16, y: u16) -> CtMouseEvent {
    CtMouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    }
}

// ── expand_to_word ─────────────────────────────────────────────────

#[test]
fn double_click_word_selects_the_enclosing_word() {
    // "hello world" — clicking inside "hello" (offset 2) snaps to
    // the full word [0, 5).
    let (mut dom, _p, t) = paragraph("hello world", 40);
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));

    let changed = expand_to_word(&mut dom);

    assert!(changed);
    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 0));
    assert_eq!(sel.focus, Position::new(t, 5));
}

#[test]
fn double_click_on_second_word_selects_it() {
    // Click inside "world" at offset 8 → [6, 11).
    let (mut dom, _p, t) = paragraph("hello world", 40);
    dom.set_selection(Some(Selection::caret(Position::new(t, 8))));

    expand_to_word(&mut dom);

    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 6));
    assert_eq!(sel.focus, Position::new(t, 11));
}

#[test]
fn double_click_on_whitespace_selects_the_whitespace_run() {
    // Click at offset 5 (the space between words) → the space's
    // own word [5, 6). Browser-faithful.
    let (mut dom, _p, t) = paragraph("hello world", 40);
    dom.set_selection(Some(Selection::caret(Position::new(t, 5))));

    expand_to_word(&mut dom);

    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 5));
    assert_eq!(sel.focus, Position::new(t, 6));
}

#[test]
fn double_click_on_cjk_character_selects_that_character() {
    // Each CJK char is its own word per TR29.
    let (mut dom, _p, t) = paragraph("中文", 40);
    dom.set_selection(Some(Selection::caret(Position::new(t, 3))));

    expand_to_word(&mut dom);

    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 3));
    assert_eq!(sel.focus, Position::new(t, 6));
}

#[test]
fn double_click_at_end_of_text_snaps_to_last_word() {
    let (mut dom, _p, t) = paragraph("hello", 40);
    dom.set_selection(Some(Selection::caret(Position::new(t, 5))));

    expand_to_word(&mut dom);

    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 0));
    assert_eq!(sel.focus, Position::new(t, 5));
}

#[test]
fn expand_to_word_without_selection_returns_false() {
    let (mut dom, _p, _t) = paragraph("hi", 40);
    assert!(!expand_to_word(&mut dom));
}

#[test]
fn expand_to_word_is_idempotent() {
    let (mut dom, _p, t) = paragraph("hello", 40);
    dom.set_selection(Some(Selection::new(
        Position::new(t, 0),
        Position::new(t, 5),
    )));
    // Already covering the full word — second expand is a no-op.
    assert!(!expand_to_word(&mut dom));
}

// ── expand_to_line ─────────────────────────────────────────────────

#[test]
fn triple_click_line_selects_the_whole_ifc_line() {
    // Single-line paragraph: whole text node becomes the selection.
    let (mut dom, _p, t) = paragraph("hello world", 40);
    dom.set_selection(Some(Selection::caret(Position::new(t, 3))));

    let changed = expand_to_line(&mut dom);

    assert!(changed);
    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 0));
    // End offset covers the last rendered fragment's bytes —
    // trailing trim applies at IFC boundary, so for a single
    // "hello world" with no trailing space the focus ends at 11.
    assert_eq!(sel.focus, Position::new(t, 11));
}

#[test]
fn triple_click_on_wrapped_second_line_selects_only_that_line() {
    // Width 6 forces "hello world" to wrap → line 0: "hello",
    // line 1: "world". Clicking inside "world" selects only "world".
    let (mut dom, _p, t) = paragraph("hello world", 6);
    dom.set_selection(Some(Selection::caret(Position::new(t, 8))));

    let changed = expand_to_line(&mut dom);

    assert!(changed);
    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 6));
    assert_eq!(sel.focus, Position::new(t, 11));
}

#[test]
fn triple_click_first_line_of_wrapped_paragraph_selects_that_line() {
    let (mut dom, _p, t) = paragraph("hello world", 6);
    dom.set_selection(Some(Selection::caret(Position::new(t, 2))));

    expand_to_line(&mut dom);

    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 0));
    assert_eq!(sel.focus, Position::new(t, 5));
}

#[test]
fn expand_to_line_without_selection_returns_false() {
    let (mut dom, _p, _t) = paragraph("hi", 40);
    assert!(!expand_to_line(&mut dom));
}

// ── Router integration: fast successive clicks promote count ───────

#[test]
fn two_fast_downs_promote_selection_to_word() {
    let (mut dom, _p, t) = paragraph("hello world", 40);
    let mut router = Router::new();

    // First click inside "hello".
    router.route(&mut dom, CtEvent::Mouse(down_at(2, 0)));
    let first = dom.selection().unwrap();
    assert!(first.is_collapsed()); // caret only

    // Second click at same place — no sleep, well within threshold.
    router.route(&mut dom, CtEvent::Mouse(down_at(2, 0)));

    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 0));
    assert_eq!(sel.focus, Position::new(t, 5));
}

#[test]
fn three_fast_downs_promote_selection_to_line() {
    let (mut dom, _p, t) = paragraph("hello world", 40);
    let mut router = Router::new();

    router.route(&mut dom, CtEvent::Mouse(down_at(2, 0)));
    router.route(&mut dom, CtEvent::Mouse(down_at(2, 0)));
    router.route(&mut dom, CtEvent::Mouse(down_at(2, 0)));

    let sel = dom.selection().unwrap();
    assert_eq!(sel.anchor, Position::new(t, 0));
    assert_eq!(sel.focus, Position::new(t, 11));
}

#[test]
fn fourth_click_resets_to_caret_selection() {
    // 3 expands to line. A 4th click in the same window should wrap
    // back to count=1 — a single caret click. This keeps repeat
    // double-click-like gestures fluid.
    let (mut dom, _p, _t) = paragraph("hello world", 40);
    let mut router = Router::new();

    router.route(&mut dom, CtEvent::Mouse(down_at(2, 0)));
    router.route(&mut dom, CtEvent::Mouse(down_at(2, 0)));
    router.route(&mut dom, CtEvent::Mouse(down_at(2, 0)));
    router.route(&mut dom, CtEvent::Mouse(down_at(2, 0)));

    // Final click is count=1 → collapsed caret.
    assert!(dom.selection().unwrap().is_collapsed());
}

#[test]
fn clicks_on_different_positions_do_not_promote() {
    // Two clicks within threshold but >2 cells apart should stay
    // at count 1 each — no promotion.
    let (mut dom, _p, _t) = paragraph("hello world", 40);
    let mut router = Router::new();

    router.route(&mut dom, CtEvent::Mouse(down_at(2, 0)));
    router.route(&mut dom, CtEvent::Mouse(down_at(8, 0))); // far away

    assert!(dom.selection().unwrap().is_collapsed());
}

#[test]
fn reset_clears_multi_click_state() {
    let (mut dom, _p, _t) = paragraph("hello world", 40);
    let mut router = Router::new();

    router.route(&mut dom, CtEvent::Mouse(down_at(2, 0)));
    router.reset();
    // After reset, a second click at the same place starts over
    // at count 1 — not promoted.
    router.route(&mut dom, CtEvent::Mouse(down_at(2, 0)));

    assert!(dom.selection().unwrap().is_collapsed());
}
