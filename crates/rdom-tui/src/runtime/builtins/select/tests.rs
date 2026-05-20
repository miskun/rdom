//! `<select>` listbox + multi-select tests.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
};
use rdom_core::{ListenerOptions, NodeId};
use std::cell::Cell;
use std::rc::Rc;

use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::runtime::builtins::{form, select};
use crate::style::Stylesheet;

fn test_app(dom: TuiDom) -> App<TestBackend> {
    let backend = TestBackend::new(40, 8);
    let terminal = Terminal::new(backend).unwrap();
    App::with_backend(dom, Stylesheet::new(), terminal).unwrap()
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> CtEvent {
    CtEvent::Key(KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

/// Build `<select>` with N options. Returns (app, select_id, option_ids).
fn select_fixture(multi: bool, labels: &[&str]) -> (App<TestBackend>, NodeId, Vec<NodeId>) {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let sel = dom.create_element("select");
    if multi {
        dom.set_attribute(sel, "multiple", "").unwrap();
    }
    let mut opts = Vec::new();
    for label in labels {
        let opt = dom.create_element("option");
        dom.set_attribute(opt, "value", label).unwrap();
        let t = dom.create_text_node(label);
        dom.append_child(opt, t).unwrap();
        dom.append_child(sel, opt).unwrap();
        opts.push(opt);
    }
    dom.append_child(root, sel).unwrap();
    // Reset the type-ahead thread-local before every test so
    // state from a previous test running on the same thread
    // doesn't leak in. Side effect is harmless for tests that
    // don't exercise type-ahead.
    select::reset_typeahead_buffer_for_tests();
    let app = test_app(dom);
    (app, sel, opts)
}

// ── value() helper ────────────────────────────────────────────────

#[test]
fn value_returns_empty_when_nothing_selected() {
    let (app, sel, _) = select_fixture(false, &["a", "b", "c"]);
    assert_eq!(select::value(app.dom(), sel), "");
}

#[test]
fn value_returns_single_selected_option_value() {
    let (mut app, sel, opts) = select_fixture(false, &["a", "b", "c"]);
    app.dom_mut()
        .set_attribute(opts[1], "selected", "")
        .unwrap();
    assert_eq!(select::value(app.dom(), sel), "b");
}

#[test]
fn value_returns_space_separated_for_multi_select() {
    let (mut app, sel, opts) = select_fixture(true, &["a", "b", "c"]);
    app.dom_mut()
        .set_attribute(opts[0], "selected", "")
        .unwrap();
    app.dom_mut()
        .set_attribute(opts[2], "selected", "")
        .unwrap();
    assert_eq!(select::value(app.dom(), sel), "a c");
}

#[test]
fn option_value_falls_back_to_text_content_when_no_value_attribute() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let sel = dom.create_element("select");
    let opt = dom.create_element("option");
    let t = dom.create_text_node("Hello");
    dom.append_child(opt, t).unwrap();
    dom.append_child(sel, opt).unwrap();
    dom.append_child(root, sel).unwrap();
    assert_eq!(select::option_value(&dom, opt), "Hello");
}

#[test]
fn options_descend_into_optgroup() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let sel = dom.create_element("select");
    let grp = dom.create_element("optgroup");
    let o1 = dom.create_element("option");
    let o2 = dom.create_element("option");
    dom.append_child(grp, o1).unwrap();
    dom.append_child(grp, o2).unwrap();
    dom.append_child(sel, grp).unwrap();
    dom.append_child(root, sel).unwrap();
    let list = select::options(&dom, sel);
    assert_eq!(list, vec![o1, o2]);
}

// ── Single-select keyboard ────────────────────────────────────────

#[test]
fn down_arrow_selects_next_option_in_single_select() {
    let (mut app, sel, opts) = select_fixture(false, &["a", "b", "c"]);
    app.dom_mut()
        .set_attribute(opts[0], "selected", "")
        .unwrap();
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "b");
    assert!(!app.dom().node(opts[0]).has_attribute("selected"));
}

#[test]
fn down_arrow_starts_from_first_enabled_when_no_selection() {
    let (mut app, sel, opts) = select_fixture(false, &["a", "b", "c"]);
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Down, KeyModifiers::empty()));
    // Highlight starts at the first option (a), Down moves to b.
    assert_eq!(select::value(app.dom(), sel), "b");
    assert!(app.dom().node(opts[1]).has_attribute("selected"));
}

#[test]
fn up_arrow_stops_at_first_option() {
    let (mut app, sel, opts) = select_fixture(false, &["a", "b", "c"]);
    app.dom_mut()
        .set_attribute(opts[0], "selected", "")
        .unwrap();
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Up, KeyModifiers::empty()));
    // Already at first — no-op (no wrap).
    assert_eq!(select::value(app.dom(), sel), "a");
}

#[test]
fn down_arrow_skips_disabled_options() {
    let (mut app, sel, opts) = select_fixture(false, &["a", "b", "c"]);
    app.dom_mut()
        .set_attribute(opts[0], "selected", "")
        .unwrap();
    app.dom_mut()
        .set_attribute(opts[1], "disabled", "")
        .unwrap();
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "c");
}

#[test]
fn home_jumps_to_first_option() {
    let (mut app, sel, opts) = select_fixture(false, &["a", "b", "c"]);
    app.dom_mut()
        .set_attribute(opts[2], "selected", "")
        .unwrap();
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Home, KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "a");
}

#[test]
fn end_jumps_to_last_option() {
    let (mut app, sel, opts) = select_fixture(false, &["a", "b", "c"]);
    app.dom_mut()
        .set_attribute(opts[0], "selected", "")
        .unwrap();
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::End, KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "c");
}

#[test]
fn disabled_select_ignores_keyboard() {
    let (mut app, sel, opts) = select_fixture(false, &["a", "b", "c"]);
    app.dom_mut().set_attribute(sel, "disabled", "").unwrap();
    app.dom_mut()
        .set_attribute(opts[0], "selected", "")
        .unwrap();
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "a");
}

// ── Multi-select keyboard ─────────────────────────────────────────

#[test]
fn arrow_in_multi_select_moves_highlight_without_changing_selection() {
    let (mut app, sel, opts) = select_fixture(true, &["a", "b", "c"]);
    app.dom_mut()
        .set_attribute(opts[0], "selected", "")
        .unwrap();
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Down, KeyModifiers::empty()));
    // Selection unchanged; highlight moved to b.
    assert_eq!(select::value(app.dom(), sel), "a");
    assert!(app.dom().node(opts[1]).has_attribute("data-rdom-highlight"));
}

#[test]
fn space_toggles_highlighted_option_in_multi_select() {
    let (mut app, sel, opts) = select_fixture(true, &["a", "b", "c"]);
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Down, KeyModifiers::empty()));
    // Highlight is now on b.
    app.handle_event(key(KeyCode::Char(' '), KeyModifiers::empty()));
    assert!(app.dom().node(opts[1]).has_attribute("selected"));
    // Space again toggles off.
    app.handle_event(key(KeyCode::Char(' '), KeyModifiers::empty()));
    assert!(!app.dom().node(opts[1]).has_attribute("selected"));
}

#[test]
fn shift_down_extends_multi_select_from_anchor() {
    let (mut app, sel, opts) = select_fixture(true, &["a", "b", "c", "d"]);
    app.dom_mut()
        .set_attribute(opts[0], "selected", "")
        .unwrap();
    app.dom_mut().set_focused(Some(sel));
    // Down (no shift) moves highlight to b. No anchor yet.
    app.handle_event(key(KeyCode::Down, KeyModifiers::empty()));
    // Shift+Down moves highlight to c AND extends from b → c.
    app.handle_event(key(KeyCode::Down, KeyModifiers::SHIFT));
    assert!(app.dom().node(opts[1]).has_attribute("selected"));
    assert!(app.dom().node(opts[2]).has_attribute("selected"));
    assert!(!app.dom().node(opts[0]).has_attribute("selected"));
    assert!(!app.dom().node(opts[3]).has_attribute("selected"));
}

#[test]
fn ctrl_a_selects_all_options_in_multi_select() {
    let (mut app, sel, opts) = select_fixture(true, &["a", "b", "c"]);
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Char('a'), KeyModifiers::CONTROL));
    assert!(
        opts.iter()
            .all(|&o| app.dom().node(o).has_attribute("selected"))
    );
}

#[test]
fn ctrl_a_skips_disabled_options() {
    let (mut app, sel, opts) = select_fixture(true, &["a", "b", "c"]);
    app.dom_mut()
        .set_attribute(opts[1], "disabled", "")
        .unwrap();
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Char('a'), KeyModifiers::CONTROL));
    assert!(app.dom().node(opts[0]).has_attribute("selected"));
    assert!(!app.dom().node(opts[1]).has_attribute("selected"));
    assert!(app.dom().node(opts[2]).has_attribute("selected"));
}

// ── change event firing ──────────────────────────────────────────

#[test]
fn keyboard_navigation_fires_change_event() {
    let (mut app, sel, _opts) = select_fixture(false, &["a", "b", "c"]);
    let fired = Rc::new(Cell::new(0u32));
    let f = fired.clone();
    app.dom_mut()
        .add_event_listener(sel, "change", ListenerOptions::default(), move |_| {
            f.set(f.get() + 1);
        })
        .unwrap();
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(fired.get(), 1);
}

// ── Form integration ─────────────────────────────────────────────

#[test]
fn form_collect_includes_select_value() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form_el = dom.create_element("form");
    let sel = dom.create_element("select");
    dom.set_attribute(sel, "name", "pet").unwrap();
    let dog = dom.create_element("option");
    dom.set_attribute(dog, "value", "dog").unwrap();
    dom.set_attribute(dog, "selected", "").unwrap();
    let cat = dom.create_element("option");
    dom.set_attribute(cat, "value", "cat").unwrap();
    dom.append_child(sel, dog).unwrap();
    dom.append_child(sel, cat).unwrap();
    dom.append_child(form_el, sel).unwrap();
    dom.append_child(root, form_el).unwrap();

    let app = test_app(dom);
    assert_eq!(
        form::collect(app.dom(), form_el),
        vec![("pet".to_string(), "dog".to_string())]
    );
}

#[test]
fn form_collect_emits_multi_select_values_as_separate_pairs() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form_el = dom.create_element("form");
    let sel = dom.create_element("select");
    dom.set_attribute(sel, "name", "toppings").unwrap();
    dom.set_attribute(sel, "multiple", "").unwrap();
    for v in ["olives", "mushrooms", "peppers"] {
        let opt = dom.create_element("option");
        dom.set_attribute(opt, "value", v).unwrap();
        if v != "mushrooms" {
            dom.set_attribute(opt, "selected", "").unwrap();
        }
        dom.append_child(sel, opt).unwrap();
    }
    dom.append_child(form_el, sel).unwrap();
    dom.append_child(root, form_el).unwrap();

    let app = test_app(dom);
    assert_eq!(
        form::collect(app.dom(), form_el),
        vec![
            ("toppings".to_string(), "olives".to_string()),
            ("toppings".to_string(), "peppers".to_string()),
        ]
    );
}

// ── Click path ────────────────────────────────────────────────────

/// Dispatch a synthetic click directly to an option — drives the
/// select's root click listener without depending on layout
/// geometry (which the inline-block options don't fully lay out
/// in test backends anyway).
fn dispatch_click(app: &mut App<TestBackend>, target: NodeId) {
    use crate::TuiDispatchExt;
    use crossterm::event::{MouseButton, MouseEvent as CtMouseEvent, MouseEventKind};
    let fake = CtMouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: 0,
        row: 0,
        modifiers: KeyModifiers::empty(),
    };
    let mut click = crate::TuiEvent::click(fake);
    let _ = app.dom_mut().dispatch_tui_event(target, &mut click);
}

#[test]
fn click_on_option_in_single_select_replaces_selection() {
    let (mut app, _sel, opts) = select_fixture(false, &["a", "b", "c"]);
    app.dom_mut()
        .set_attribute(opts[0], "selected", "")
        .unwrap();
    dispatch_click(&mut app, opts[2]);
    assert!(!app.dom().node(opts[0]).has_attribute("selected"));
    assert!(app.dom().node(opts[2]).has_attribute("selected"));
}

#[test]
fn click_on_option_in_multi_select_toggles() {
    let (mut app, _sel, opts) = select_fixture(true, &["a", "b", "c"]);
    dispatch_click(&mut app, opts[1]);
    assert!(app.dom().node(opts[1]).has_attribute("selected"));
    dispatch_click(&mut app, opts[1]);
    assert!(!app.dom().node(opts[1]).has_attribute("selected"));
}

#[test]
fn click_on_disabled_option_is_ignored() {
    let (mut app, _sel, opts) = select_fixture(false, &["a", "b", "c"]);
    app.dom_mut()
        .set_attribute(opts[1], "disabled", "")
        .unwrap();
    dispatch_click(&mut app, opts[1]);
    assert!(!app.dom().node(opts[1]).has_attribute("selected"));
}

// ── C.7b: dropdown open/close ──────────────────────────────────────

#[test]
fn select_without_multiple_or_size_is_dropdown() {
    let (app, sel, _) = select_fixture(false, &["a", "b"]);
    assert!(select::is_dropdown(app.dom(), sel));
    assert!(!select::is_open(app.dom(), sel));
}

#[test]
fn select_with_multiple_is_not_dropdown() {
    let (app, sel, _) = select_fixture(true, &["a", "b"]);
    assert!(!select::is_dropdown(app.dom(), sel));
}

#[test]
fn select_with_size_is_not_dropdown() {
    let (mut app, sel, _) = select_fixture(false, &["a", "b", "c"]);
    app.dom_mut().set_attribute(sel, "size", "3").unwrap();
    assert!(!select::is_dropdown(app.dom(), sel));
}

#[test]
fn open_sets_the_open_marker_on_dropdown() {
    let (mut app, sel, _) = select_fixture(false, &["a"]);
    select::open(app.dom_mut(), sel);
    assert!(select::is_open(app.dom(), sel));
}

#[test]
fn close_clears_the_open_marker() {
    let (mut app, sel, _) = select_fixture(false, &["a"]);
    select::open(app.dom_mut(), sel);
    select::close(app.dom_mut(), sel);
    assert!(!select::is_open(app.dom(), sel));
}

#[test]
fn open_on_listbox_is_noop() {
    let (mut app, sel, _) = select_fixture(true, &["a"]);
    select::open(app.dom_mut(), sel);
    assert!(!app.dom().node(sel).has_attribute("data-rdom-open"));
}

#[test]
fn click_on_closed_dropdown_chrome_opens_it() {
    let (mut app, sel, _) = select_fixture(false, &["a", "b"]);
    // Click the select itself (not an option).
    dispatch_click(&mut app, sel);
    assert!(select::is_open(app.dom(), sel));
}

#[test]
fn click_on_open_dropdown_chrome_closes_it() {
    let (mut app, sel, _) = select_fixture(false, &["a", "b"]);
    select::open(app.dom_mut(), sel);
    dispatch_click(&mut app, sel);
    assert!(!select::is_open(app.dom(), sel));
}

#[test]
fn click_on_option_in_open_dropdown_selects_and_closes() {
    let (mut app, sel, opts) = select_fixture(false, &["a", "b", "c"]);
    select::open(app.dom_mut(), sel);
    dispatch_click(&mut app, opts[1]);
    assert!(app.dom().node(opts[1]).has_attribute("selected"));
    assert!(!select::is_open(app.dom(), sel));
}

#[test]
fn clicking_an_option_on_a_listbox_does_not_toggle_open_marker() {
    let (mut app, sel, opts) = select_fixture(true, &["a", "b"]);
    dispatch_click(&mut app, opts[0]);
    // Listbox has no "open" concept — attribute never set.
    assert!(!app.dom().node(sel).has_attribute("data-rdom-open"));
    assert!(app.dom().node(opts[0]).has_attribute("selected"));
}

#[test]
fn esc_closes_an_open_dropdown() {
    let (mut app, sel, _) = select_fixture(false, &["a"]);
    select::open(app.dom_mut(), sel);
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Esc, KeyModifiers::empty()));
    assert!(!select::is_open(app.dom(), sel));
}

#[test]
fn esc_on_closed_dropdown_is_noop() {
    let (mut app, sel, _) = select_fixture(false, &["a"]);
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Esc, KeyModifiers::empty()));
    assert!(!select::is_open(app.dom(), sel));
}

#[test]
fn enter_on_closed_dropdown_opens_it() {
    let (mut app, sel, _) = select_fixture(false, &["a"]);
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Enter, KeyModifiers::empty()));
    assert!(select::is_open(app.dom(), sel));
}

#[test]
fn enter_on_open_dropdown_closes_it() {
    let (mut app, sel, _) = select_fixture(false, &["a"]);
    select::open(app.dom_mut(), sel);
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Enter, KeyModifiers::empty()));
    assert!(!select::is_open(app.dom(), sel));
}

#[test]
fn enter_on_listbox_does_not_toggle_open_marker() {
    let (mut app, sel, _) = select_fixture(true, &["a"]);
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Enter, KeyModifiers::empty()));
    assert!(!app.dom().node(sel).has_attribute("data-rdom-open"));
}

// ── C.7c: type-ahead search ────────────────────────────────────────

#[test]
fn typing_letter_jumps_to_option_with_matching_label() {
    let (mut app, sel, opts) = select_fixture(false, &["Apple", "Banana", "Cherry"]);
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Char('b'), KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "Banana");
    assert!(app.dom().node(opts[1]).has_attribute("data-rdom-highlight"));
}

#[test]
fn type_ahead_is_case_insensitive() {
    let (mut app, sel, _opts) = select_fixture(false, &["Apple", "Banana"]);
    app.dom_mut().set_focused(Some(sel));
    // Uppercase typed, lowercase label — still matches.
    app.handle_event(key(KeyCode::Char('A'), KeyModifiers::SHIFT));
    assert_eq!(select::value(app.dom(), sel), "Apple");
}

#[test]
fn type_ahead_wraps_around_at_end() {
    let (mut app, sel, opts) = select_fixture(false, &["Apple", "Banana", "Cherry"]);
    // Highlight starts at cherry (last); typing 'a' should wrap
    // back to apple, not stay at cherry.
    app.dom_mut()
        .set_attribute(opts[2], "data-rdom-highlight", "")
        .unwrap();
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Char('a'), KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "Apple");
}

#[test]
fn type_ahead_skips_disabled_options() {
    let (mut app, sel, opts) = select_fixture(false, &["Apple", "Apricot", "Cherry"]);
    app.dom_mut()
        .set_attribute(opts[0], "disabled", "")
        .unwrap();
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Char('a'), KeyModifiers::empty()));
    // Apple is disabled — match Apricot instead.
    assert_eq!(select::value(app.dom(), sel), "Apricot");
}

#[test]
fn type_ahead_with_no_match_leaves_state_unchanged() {
    let (mut app, sel, opts) = select_fixture(false, &["Apple", "Banana"]);
    app.dom_mut()
        .set_attribute(opts[0], "selected", "")
        .unwrap();
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Char('z'), KeyModifiers::empty()));
    // Selection + highlight stay put.
    assert_eq!(select::value(app.dom(), sel), "Apple");
}

#[test]
fn type_ahead_in_multi_select_moves_highlight_only() {
    let (mut app, sel, opts) = select_fixture(true, &["Apple", "Banana"]);
    app.dom_mut()
        .set_attribute(opts[0], "selected", "")
        .unwrap();
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Char('b'), KeyModifiers::empty()));
    // Highlight on banana, but selection unchanged (still apple).
    assert!(app.dom().node(opts[1]).has_attribute("data-rdom-highlight"));
    assert_eq!(select::value(app.dom(), sel), "Apple");
}

#[test]
fn type_ahead_advances_past_current_highlight_on_repeat() {
    // Multiple options start with same letter — repeated keystroke
    // cycles through them.
    let (mut app, sel, _opts) = select_fixture(false, &["Apple", "Apricot", "Avocado"]);
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Char('a'), KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "Apple");
    app.handle_event(key(KeyCode::Char('a'), KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "Apricot");
    app.handle_event(key(KeyCode::Char('a'), KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "Avocado");
    // Wrap back to Apple.
    app.handle_event(key(KeyCode::Char('a'), KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "Apple");
}

#[test]
fn ctrl_letter_is_not_type_ahead() {
    let (mut app, sel, opts) = select_fixture(false, &["Apple", "Banana"]);
    app.dom_mut()
        .set_attribute(opts[0], "selected", "")
        .unwrap();
    app.dom_mut().set_focused(Some(sel));
    // Ctrl+B: Ctrl modifier kills type-ahead — no jump.
    app.handle_event(key(KeyCode::Char('b'), KeyModifiers::CONTROL));
    assert_eq!(select::value(app.dom(), sel), "Apple");
}

// ── Polish #6: multi-keystroke prefix matching ─────────────────────

#[test]
fn type_ahead_multi_keystroke_matches_prefix() {
    // "Ap" should land on Apple even when Apricot / Avocado are
    // also present — a single 'a' would match Apple, but adding
    // 'p' refines to the `ap…` prefix.
    let (mut app, sel, _opts) = select_fixture(false, &["Avocado", "Apple", "Apricot"]);
    app.dom_mut().set_focused(Some(sel));
    // First 'a' lands on Avocado (first in list).
    app.handle_event(key(KeyCode::Char('a'), KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "Avocado");
    // 'p' within the timeout: buffer becomes "ap" — Apple is the
    // first option starting with "ap".
    app.handle_event(key(KeyCode::Char('p'), KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "Apple");
    // Another 'p': buffer becomes "app" — Apple still matches
    // (Apricot doesn't).
    app.handle_event(key(KeyCode::Char('p'), KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "Apple");
    // Type 'r': buffer becomes "appr" — no match, highlight stays.
    app.handle_event(key(KeyCode::Char('r'), KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "Apple");
}

#[test]
fn type_ahead_buffer_resets_on_focus_change() {
    // Two selects side by side. Typing "a" on the first should
    // not leak into the second's buffer.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let s1 = dom.create_element("select");
    let o1 = dom.create_element("option");
    let t1 = dom.create_text_node("Apple");
    dom.append_child(o1, t1).unwrap();
    dom.append_child(s1, o1).unwrap();
    let s2 = dom.create_element("select");
    let o2 = dom.create_element("option");
    let t2 = dom.create_text_node("Avocado");
    dom.append_child(o2, t2).unwrap();
    let o3 = dom.create_element("option");
    let t3 = dom.create_text_node("Banana");
    dom.append_child(o3, t3).unwrap();
    dom.append_child(s2, o2).unwrap();
    dom.append_child(s2, o3).unwrap();
    dom.append_child(root, s1).unwrap();
    dom.append_child(root, s2).unwrap();

    select::reset_typeahead_buffer_for_tests();
    let backend = TestBackend::new(40, 8);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, Stylesheet::new(), terminal).unwrap();

    app.dom_mut().set_focused(Some(s1));
    app.handle_event(key(KeyCode::Char('a'), KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), s1), "Apple");

    // Switch focus to s2 — the "a" buffer from s1 should reset
    // when the same 'b' keystroke lands on s2. Typing 'b' should
    // match Banana (not "ab" with stale buffer + 'b').
    app.dom_mut().set_focused(Some(s2));
    app.handle_event(key(KeyCode::Char('b'), KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), s2), "Banana");
}

#[test]
fn type_ahead_different_letters_build_multi_char_prefix() {
    // Typing 'c' then 'h' should jump to "Cherry" (prefix "ch"),
    // not cycle through single-char 'c' matches.
    let (mut app, sel, _opts) =
        select_fixture(false, &["Cabbage", "Cauliflower", "Cherry", "Carrot"]);
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Char('c'), KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "Cabbage");
    app.handle_event(key(KeyCode::Char('h'), KeyModifiers::empty()));
    // "ch" matches Cherry.
    assert_eq!(select::value(app.dom(), sel), "Cherry");
}

#[test]
fn space_in_single_select_does_not_trigger_type_ahead() {
    // Space is also not a common label-starting character; in
    // single-select it's currently a no-op (polish: dropdown
    // open on space). Confirm it's not accidentally treated as
    // a search char.
    let (mut app, sel, opts) = select_fixture(false, &[" Apple", "Banana"]);
    app.dom_mut()
        .set_attribute(opts[1], "selected", "")
        .unwrap();
    app.dom_mut().set_focused(Some(sel));
    app.handle_event(key(KeyCode::Char(' '), KeyModifiers::empty()));
    assert_eq!(select::value(app.dom(), sel), "Banana");
}

#[test]
fn form_collect_uses_text_content_when_option_has_no_value() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let form_el = dom.create_element("form");
    let sel = dom.create_element("select");
    dom.set_attribute(sel, "name", "x").unwrap();
    let opt = dom.create_element("option");
    dom.set_attribute(opt, "selected", "").unwrap();
    let t = dom.create_text_node("FallbackText");
    dom.append_child(opt, t).unwrap();
    dom.append_child(sel, opt).unwrap();
    dom.append_child(form_el, sel).unwrap();
    dom.append_child(root, form_el).unwrap();

    let app = test_app(dom);
    assert_eq!(
        form::collect(app.dom(), form_el),
        vec![("x".to_string(), "FallbackText".to_string())]
    );
}
