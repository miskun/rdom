//! End-to-end integration test for `<textarea>` behavior.
//!
//! Pins the contract the user-visible textarea is expected to honor:
//!
//! 1. Tab focuses the textarea (it's implicitly focusable).
//! 2. Typing into a focused textarea appends to its text content.
//! 3. Text longer than the textarea's content width wraps onto
//!    multiple visual rows.
//! 4. Enter inserts a `\n` into the textarea value (HTML default;
//!    Enter inside a `<textarea>` does NOT submit the surrounding
//!    form).
//! 5. Shift+Enter likewise inserts a `\n`.
//!
//! These mirror the user-visible behavior of HTML's `<textarea>` and
//! are the bar 0.1.0's textarea ships with.

use std::cell::Cell;
use std::rc::Rc;

use crossterm::event::{Event as CtEvent, KeyCode, KeyEvent, KeyModifiers};
use rdom_core::{ListenerOptions, NodeType};
use rdom_tui::layout::{Size, WhiteSpace};
use rdom_tui::prelude::*;
use rdom_tui::render::{Buffer, Rect, Terminal, TestBackend};
use rdom_tui::runtime::app::App;

mod common;
use common::render;

fn key(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent::new(code, KeyModifiers::empty()))
}

fn ch(c: char) -> CtEvent {
    CtEvent::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()))
}

fn shift(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent::new(code, KeyModifiers::SHIFT))
}

/// Extract the text content of a textarea by walking its text-node
/// children. Mirrors what `runtime::builtins::input::value` would
/// return if it covered textareas.
fn textarea_text(dom: &TuiDom, id: rdom_tui::NodeId) -> String {
    let mut out = String::new();
    for child in dom.node(id).child_nodes() {
        if child.node_type() == NodeType::Text
            && let Some(v) = child.node_value()
        {
            out.push_str(v);
        }
    }
    out
}

/// Count the number of rows in `buf` where any cell within the
/// element's `bounding_rect` contains `target` as a symbol. A row
/// counts at most once.
fn rows_containing_glyph(buf: &Buffer, rect: Rect, target: &str) -> usize {
    let mut hits = 0usize;
    for y in rect.y..rect.bottom() {
        let mut found = false;
        for x in rect.x..rect.right() {
            if let Some(c) = buf.cell(x, y)
                && !c.is_spacer()
                && c.symbol() == target
            {
                found = true;
                break;
            }
        }
        if found {
            hits += 1;
        }
    }
    hits
}

#[test]
fn textarea_wraps_long_input_and_enter_inserts_newline() {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let textarea = dom.create_element("textarea");
    dom.set_attribute(textarea, "name", "notes").unwrap();
    dom.append_child(root, textarea).unwrap();

    // Author CSS narrows the textarea so typing a few dozen chars
    // pushes content past the content-box and forces a wrap.
    // Width 16 with `padding: 0 1` (UA default) and a 1-cell
    // scrollbar gutter leaves ~13 cells of content width.
    let sheet = Stylesheet::new()
        .rule_unchecked(
            "textarea",
            TuiStyle::new()
                .width(Size::Fixed(16))
                .height(Size::Fixed(5))
                .white_space(WhiteSpace::PreWrap),
        )
        .rule_unchecked(
            "root",
            TuiStyle::new()
                .width(Size::Fixed(40))
                .height(Size::Fixed(10)),
        );

    let backend = TestBackend::new(40, 10);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();

    // ── Step 1: Tab focuses the textarea.
    app.handle_event(key(KeyCode::Tab));
    assert_eq!(
        app.dom().focused(),
        Some(textarea),
        "Tab should focus the textarea"
    );

    // The focus_node helper must seed a caret so typing has somewhere
    // to land. Mirrors the assertion from `tab_form_integration.rs`.
    assert!(
        app.dom().selection().is_some(),
        "focus_node should seed a caret for editable textarea"
    );

    // ── Step 2: type long content with spaces. PreWrap wraps at
    // whitespace, so multi-word content longer than the content
    // width must wrap onto multiple rows.
    //
    // Textarea is 16 cells wide, minus padding (1L + 1R) minus
    // scrollbar gutter (1R) = ~13 cells of content width. The phrase
    // "ant bee cat dog elk fox gnu hen" is 31 chars with 7 spaces,
    // so the packer wraps it to at least 3 lines (each line ≤ 13).
    let long_text = "ant bee cat dog elk fox gnu hen";
    for c in long_text.chars() {
        app.handle_event(ch(c));
    }

    // DOM-level assertion: all chars in the value, including spaces.
    let value = textarea_text(app.dom(), textarea);
    assert_eq!(
        value, long_text,
        "all typed chars must be in the textarea's text content"
    );

    // Visual assertion: re-render and count rows containing 'a'.
    // 'a' appears in "ant" (line 1), so check 'b' for line 2 ("bee"
    // wraps to line 2 if line 1 holds "ant bee" = 7 cells; otherwise
    // earlier). The bag-of-glyphs check is brittle; use 'a' as a
    // canary that wrap put SOMETHING on row 0 and 'h' that something
    // got to a later row.
    let viewport = Rect::new(0, 0, 40, 10);
    let sheet_for_render = Stylesheet::new()
        .rule_unchecked(
            "textarea",
            TuiStyle::new()
                .width(Size::Fixed(16))
                .height(Size::Fixed(5))
                .white_space(WhiteSpace::PreWrap),
        )
        .rule_unchecked(
            "root",
            TuiStyle::new()
                .width(Size::Fixed(40))
                .height(Size::Fixed(10)),
        );
    let buf = render(app.dom_mut(), &sheet_for_render, viewport);

    let ta_rect = Rect::new(0, 0, 40, 6);
    let row_hits_a = rows_containing_glyph(&buf, ta_rect, "a"); // appears in "ant" / "cat"
    let row_hits_h = rows_containing_glyph(&buf, ta_rect, "h"); // appears in "hen" (last word)
    assert!(
        row_hits_a >= 1 && row_hits_h >= 1,
        "wrapped output should have 'a' (in ant/cat) and 'h' (in hen) visible; got a={row_hits_a}, h={row_hits_h}"
    );
    // True wrap assertion: total rows containing any of the words'
    // glyphs must be ≥ 2.
    let mut total_rows_with_text = 0usize;
    for y in ta_rect.y..ta_rect.bottom() {
        let mut found = false;
        for x in ta_rect.x..ta_rect.right() {
            if let Some(c) = buf.cell(x, y)
                && !c.is_spacer()
                && c.symbol()
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_alphabetic())
            {
                found = true;
                break;
            }
        }
        if found {
            total_rows_with_text += 1;
        }
    }
    assert!(
        total_rows_with_text >= 2,
        "long whitespace-separated text in a 13-cell-wide textarea should wrap to >= 2 rows; got {total_rows_with_text}"
    );

    // ── Step 3: Enter inserts a newline (HTML default for textarea).
    app.handle_event(key(KeyCode::Enter));
    let value = textarea_text(app.dom(), textarea);
    assert!(
        value.ends_with('\n'),
        "Enter inside a <textarea> must insert a newline, not submit the form. Value: {value:?}"
    );

    // ── Step 4: Shift+Enter also inserts a newline.
    app.handle_event(shift(KeyCode::Enter));
    let value = textarea_text(app.dom(), textarea);
    assert!(
        value.ends_with("\n\n"),
        "Shift+Enter must also insert a newline. Value: {value:?}"
    );
}

#[test]
fn enter_inside_form_textarea_inserts_newline_and_does_not_submit() {
    // HTML behavior: Enter inside a <textarea> inserts a newline. It
    // does NOT submit the surrounding form (that's <input>'s job).
    // Pin this with an explicit form + submit listener so the contract
    // can't silently regress.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();

    let form_el = dom.create_element("form");
    let textarea = dom.create_element("textarea");
    dom.set_attribute(textarea, "name", "notes").unwrap();
    dom.append_child(form_el, textarea).unwrap();
    dom.append_child(root, form_el).unwrap();

    // Track submit fires. Must stay at 0 throughout the test.
    let submit_count = Rc::new(Cell::new(0u32));
    let submit_count_clone = submit_count.clone();
    dom.add_event_listener(form_el, "submit", ListenerOptions::default(), move |_ctx| {
        submit_count_clone.set(submit_count_clone.get() + 1);
    })
    .unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "textarea",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(3))
            .white_space(WhiteSpace::PreWrap),
    );

    let backend = TestBackend::new(40, 10);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();

    app.handle_event(key(KeyCode::Tab));
    assert_eq!(
        app.dom().focused(),
        Some(textarea),
        "Tab should focus the textarea"
    );

    for c in "abc".chars() {
        app.handle_event(ch(c));
    }
    app.handle_event(key(KeyCode::Enter));
    for c in "def".chars() {
        app.handle_event(ch(c));
    }

    // Newline was inserted.
    let value = textarea_text(app.dom(), textarea);
    assert_eq!(
        value, "abc\ndef",
        "Enter must insert a newline into textarea, not submit the form. Got: {value:?}"
    );

    // And critically: submit did NOT fire.
    assert_eq!(
        submit_count.get(),
        0,
        "Enter inside a form <textarea> must NOT submit the form"
    );
}
