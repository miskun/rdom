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

use crate::common::render;
use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent as CtMouseEvent,
    MouseEventKind,
};
use rdom_core::{ListenerOptions, NodeType};
use rdom_tui::layout::{Size, WhiteSpace};
use rdom_tui::prelude::*;
use rdom_tui::render::{Buffer, Rect, Terminal, TestBackend};
use rdom_tui::runtime::app::App;

fn key(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent::new(code, KeyModifiers::empty()))
}

fn ch(c: char) -> CtEvent {
    CtEvent::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()))
}

fn shift(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent::new(code, KeyModifiers::SHIFT))
}

/// Dispatch an event AND run the deferred redraw — the way the
/// production run loop does. Bare `app.handle_event` mutates the DOM
/// but doesn't re-cascade or re-layout, so any caret-movement /
/// hit-test / paint-dependent assertion that follows reads stale
/// layout. Use `dispatch` in any test that asserts on rendered state
/// across multiple events.
fn dispatch<B: rdom_tui::render::Backend>(app: &mut App<B>, event: CtEvent) {
    app.handle_event(event);
    let _ = app.draw_if_dirty();
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
    // True wrap assertion: 31 chars + 7 spaces in a ≈13-cell-wide
    // textarea packs to 3 lines ("ant bee cat " / "dog elk fox " /
    // "gnu hen"). Assert all three rows have visible text.
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
        total_rows_with_text >= 3,
        "8 whitespace-separated words in a ≈13-cell-wide textarea should wrap to >= 3 rows; got {total_rows_with_text}"
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
fn focused_textarea_paints_visible_caret() {
    // After Tab-focusing a textarea, a REVERSED cell must appear at
    // the caret position. This is the round-trip that proves A1
    // (inline_flow_container) + A2 (paint_caret hoist) actually let
    // the caret render in the non-IFC paint path.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let textarea = dom.create_element("textarea");
    dom.append_child(root, textarea).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "textarea",
        TuiStyle::new()
            .width(Size::Fixed(16))
            .height(Size::Fixed(3))
            .white_space(WhiteSpace::PreWrap),
    );

    let backend = TestBackend::new(40, 10);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();

    app.handle_event(key(KeyCode::Tab));
    assert_eq!(app.dom().focused(), Some(textarea));

    let sheet_for_render = Stylesheet::new().rule_unchecked(
        "textarea",
        TuiStyle::new()
            .width(Size::Fixed(16))
            .height(Size::Fixed(3))
            .white_space(WhiteSpace::PreWrap),
    );
    let buf = render(app.dom_mut(), &sheet_for_render, Rect::new(0, 0, 40, 10));

    // Walk the textarea's bounding rect and find at least one cell
    // with REVERSED modifier set — that's the caret.
    let ta_rect = Rect::new(0, 0, 40, 4);
    let mut reversed_cells = 0usize;
    for y in ta_rect.y..ta_rect.bottom() {
        for x in ta_rect.x..ta_rect.right() {
            // Caret cell detection: it has the white-fallback bg
            // (Rgb(0xFF, 0xFF, 0xFF)) since the unstyled textarea
            // has no cascaded `color` value, so Auto caret-color
            // resolves to the high-contrast default.
            if let Some(c) = buf.cell(x, y)
                && c.bg == rdom_tui::style::Color::Rgb(0xFF, 0xFF, 0xFF)
            {
                reversed_cells += 1;
            }
        }
    }
    assert!(
        reversed_cells >= 1,
        "focused textarea must paint a visible REVERSED caret cell; got {reversed_cells}"
    );
}

#[test]
fn caret_color_transparent_suppresses_caret_paint() {
    // `:focus { caret-color: transparent }` is the user's stated
    // opt-out. Editing still works (Selection state is updated),
    // but the visible REVERSED cell does NOT appear.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let textarea = dom.create_element("textarea");
    dom.append_child(root, textarea).unwrap();

    let sheet = Stylesheet::new()
        .rule_unchecked(
            "textarea",
            TuiStyle::new()
                .width(Size::Fixed(16))
                .height(Size::Fixed(3))
                .white_space(WhiteSpace::PreWrap),
        )
        .rule_unchecked(
            "textarea:focus",
            TuiStyle::new().caret_color(rdom_tui::layout::CaretColor::Transparent),
        );

    let backend = TestBackend::new(40, 10);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();

    app.handle_event(key(KeyCode::Tab));
    assert_eq!(app.dom().focused(), Some(textarea));

    let sheet_for_render = Stylesheet::new()
        .rule_unchecked(
            "textarea",
            TuiStyle::new()
                .width(Size::Fixed(16))
                .height(Size::Fixed(3))
                .white_space(WhiteSpace::PreWrap),
        )
        .rule_unchecked(
            "textarea:focus",
            TuiStyle::new().caret_color(rdom_tui::layout::CaretColor::Transparent),
        );
    let buf = render(app.dom_mut(), &sheet_for_render, Rect::new(0, 0, 40, 10));

    let ta_rect = Rect::new(0, 0, 40, 4);
    let mut reversed_cells = 0usize;
    for y in ta_rect.y..ta_rect.bottom() {
        for x in ta_rect.x..ta_rect.right() {
            // Caret cell detection: it has the white-fallback bg
            // (Rgb(0xFF, 0xFF, 0xFF)) since the unstyled textarea
            // has no cascaded `color` value, so Auto caret-color
            // resolves to the high-contrast default.
            if let Some(c) = buf.cell(x, y)
                && c.bg == rdom_tui::style::Color::Rgb(0xFF, 0xFF, 0xFF)
            {
                reversed_cells += 1;
            }
        }
    }
    assert_eq!(
        reversed_cells, 0,
        "caret-color: transparent must suppress caret paint; got {reversed_cells} REVERSED cells"
    );
}

#[test]
fn arrow_down_from_end_of_long_line_clamps_to_end_of_short_line() {
    // textarea with explicit newlines:
    //   line 0: "abcdefgh" (8 chars, offsets 0..8)
    //   line 1: "abc"      (3 chars, offsets 9..12)
    //   line 2: "abcdefg"  (7 chars, offsets 13..20)
    //
    // Caret at end of line 0 (offset 8). Arrow Down must land at the
    // END of line 1 (offset 12) — the position just past the last
    // character. Bug symptom: caret doesn't move because cell (8, y+1)
    // has nothing on it.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let textarea = dom.create_element("textarea");
    dom.append_child(root, textarea).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "textarea",
        TuiStyle::new()
            .width(Size::Fixed(30))
            .height(Size::Fixed(5))
            .white_space(WhiteSpace::PreWrap),
    );

    let backend = TestBackend::new(40, 10);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();

    // Tab to focus.
    app.handle_event(key(KeyCode::Tab));

    // Type the multi-line content via keystrokes (Enter inserts \n).
    for c in "abcdefgh".chars() {
        dispatch(&mut app, ch(c));
    }
    dispatch(&mut app, key(KeyCode::Enter));
    for c in "abc".chars() {
        dispatch(&mut app, ch(c));
    }
    dispatch(&mut app, key(KeyCode::Enter));
    for c in "abcdefg".chars() {
        dispatch(&mut app, ch(c));
    }

    // Verify content (sanity).
    let value = textarea_text(app.dom(), textarea);
    assert_eq!(value, "abcdefgh\nabc\nabcdefg");

    // Move caret to end of line 0 (offset 8). The caret currently
    // sits at offset 20 (end of all content); use ArrowUp twice +
    // End to navigate to end of line 0.
    let off_before = app.dom().selection().unwrap().focus.offset;
    assert_eq!(off_before, 20, "caret should be at end of content");
    dispatch(&mut app, key(KeyCode::Up));
    let off_after_up1 = app.dom().selection().unwrap().focus.offset;
    assert_eq!(
        off_after_up1, 12,
        "Up from offset 20 should land at end of line 1 (offset 12), got {off_after_up1}"
    );
    dispatch(&mut app, key(KeyCode::Up));
    let off_after_up2 = app.dom().selection().unwrap().focus.offset;
    dispatch(&mut app, key(KeyCode::End));

    let sel = app.dom().selection().expect("caret must exist");
    assert_eq!(
        sel.focus.offset, 8,
        "End on line 0 should place caret at offset 8 (post-'h'); intermediate offsets: 20→{off_after_up1}→{off_after_up2}→{}",
        sel.focus.offset
    );

    // Now Arrow Down. Expected: caret moves to end of line 1
    // (offset 12 = 8 + 1 newline + 3 chars).
    app.handle_event(key(KeyCode::Down));
    let sel = app.dom().selection().expect("caret must still exist");
    assert_eq!(
        sel.focus.offset, 12,
        "Arrow Down from end of long line must clamp to end of short line (got offset {})",
        sel.focus.offset
    );
}

#[test]
fn caret_visible_immediately_after_enter() {
    // After Enter inserts `\n`, the caret moves to the start of the
    // freshly-created empty line. It must be VISIBLE there — not
    // hidden until the user types a character on that line.
    //
    // Bug symptom: pressing Enter moves the caret offset correctly,
    // but the REVERSED cell isn't painted until a char arrives.
    // Root cause: `cell_of_position` returns None when the position
    // is at the end of content past a `\n` because there's no
    // fragment covering it.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let textarea = dom.create_element("textarea");
    dom.append_child(root, textarea).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "textarea",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(4))
            .white_space(WhiteSpace::PreWrap),
    );

    let backend = TestBackend::new(40, 10);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();

    // Tab to focus + type "abc" + Enter.
    app.handle_event(key(KeyCode::Tab));
    for c in "abc".chars() {
        app.handle_event(ch(c));
    }
    app.handle_event(key(KeyCode::Enter));

    // Sanity: content is "abc\n", caret is at offset 4.
    let value = textarea_text(app.dom(), textarea);
    assert_eq!(value, "abc\n");
    let sel = app.dom().selection().expect("caret must exist after Enter");
    assert_eq!(sel.focus.offset, 4, "caret must sit just past the newline");

    // Render and assert there's a REVERSED cell painted (the caret).
    let sheet_for_render = Stylesheet::new().rule_unchecked(
        "textarea",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(4))
            .white_space(WhiteSpace::PreWrap),
    );
    let buf = render(app.dom_mut(), &sheet_for_render, Rect::new(0, 0, 40, 10));

    let ta_rect = Rect::new(0, 0, 40, 5);
    let mut reversed_cells = 0usize;
    for y in ta_rect.y..ta_rect.bottom() {
        for x in ta_rect.x..ta_rect.right() {
            // Caret cell detection: it has the white-fallback bg
            // (Rgb(0xFF, 0xFF, 0xFF)) since the unstyled textarea
            // has no cascaded `color` value, so Auto caret-color
            // resolves to the high-contrast default.
            if let Some(c) = buf.cell(x, y)
                && c.bg == rdom_tui::style::Color::Rgb(0xFF, 0xFF, 0xFF)
            {
                reversed_cells += 1;
            }
        }
    }
    assert!(
        reversed_cells >= 1,
        "caret must be visible immediately after Enter inserts a newline; got {reversed_cells} REVERSED cells"
    );
}

#[test]
fn up_at_top_of_content_moves_to_line_start() {
    // Browser behavior: Arrow Up at the top line of an editable
    // moves the caret to the start of that line (offset 0), it does
    // NOT stay put. rdom's `caret_up` previously returned None when
    // `y == 0`, leaving the caret where it was.
    let mut app = build_app_with_three_lines();

    // Caret should currently be at end of all content. Navigate to
    // offset 5 of line 0 (between 'e' and 'f').
    dispatch(&mut app, key(KeyCode::Up));
    dispatch(&mut app, key(KeyCode::Up));
    dispatch(&mut app, key(KeyCode::Home));
    for _ in 0..5 {
        dispatch(&mut app, key(KeyCode::Right));
    }
    assert_eq!(app.dom().selection().unwrap().focus.offset, 5);

    // Up at top → line start.
    dispatch(&mut app, key(KeyCode::Up));
    let sel = app.dom().selection().unwrap();
    assert_eq!(
        sel.focus.offset, 0,
        "Arrow Up at top-of-content must move caret to line start, got {}",
        sel.focus.offset
    );
}

#[test]
fn down_at_bottom_of_content_moves_to_line_end() {
    // Browser behavior: Arrow Down at the last line of an editable
    // moves the caret to the end of that line. Symmetric to Up-at-top.
    let mut app = build_app_with_three_lines();

    // Caret is at end of all content (offset 19). Move to offset 13
    // (last line, between 'a' and 'b') so Down has a place to go to.
    dispatch(&mut app, key(KeyCode::Home));
    for _ in 0..1 {
        dispatch(&mut app, key(KeyCode::Right));
    }
    let off = app.dom().selection().unwrap().focus.offset;
    assert_eq!(off, 13, "caret should be at offset 13 (line 2 column 1)");

    // Down at bottom → end of last line.
    dispatch(&mut app, key(KeyCode::Down));
    let sel = app.dom().selection().unwrap();
    assert_eq!(
        sel.focus.offset, 19,
        "Arrow Down at bottom-of-content must move caret to line end, got {}",
        sel.focus.offset
    );
}

#[test]
fn vertical_caret_remembers_sticky_x_across_short_lines() {
    // The user's exact scenario:
    //   line 0: "abcdefg"  (7 chars, offsets 0..7)
    //   line 1: "abc"      (3 chars, offsets 8..11)
    //   line 2: "abcdefg"  (7 chars, offsets 12..19)
    //
    // Start at offset 5 of line 0 (over 'f', column 5).
    // - Up at top → line start (offset 0). Sticky-x = 5 preserved.
    // - Down → clamped to end of line 1 (offset 11, column 3).
    //   Sticky-x = 5 NOT updated.
    // - Down → line 2 at sticky column 5 = offset 17 (over 'f').
    //
    // This is the canonical sticky-x test. Without it, the second
    // Down would land at column 3 (the last clamped column), not 5.
    let mut app = build_app_with_three_lines();

    // Navigate to offset 5 of line 0.
    dispatch(&mut app, key(KeyCode::Up));
    dispatch(&mut app, key(KeyCode::Up));
    dispatch(&mut app, key(KeyCode::Home));
    for _ in 0..5 {
        dispatch(&mut app, key(KeyCode::Right));
    }
    assert_eq!(app.dom().selection().unwrap().focus.offset, 5);

    // Up at top → line start.
    dispatch(&mut app, key(KeyCode::Up));
    assert_eq!(app.dom().selection().unwrap().focus.offset, 0);

    // Down → end of line 1 (sticky column 5 clamped to 3).
    dispatch(&mut app, key(KeyCode::Down));
    assert_eq!(
        app.dom().selection().unwrap().focus.offset,
        11,
        "Down should clamp to end of short line 1 (offset 11)"
    );

    // Down → line 2 at sticky column 5. Offset = 12 (line 2 start) + 5 = 17.
    dispatch(&mut app, key(KeyCode::Down));
    assert_eq!(
        app.dom().selection().unwrap().focus.offset,
        17,
        "Down must restore sticky-x to column 5 on the wider line 2 (offset 17)"
    );
}

#[test]
fn sticky_x_resets_on_horizontal_motion() {
    // Sticky-x persists across vertical motions but RESETS on any
    // other action (typing, Left/Right, Home/End, mouse).
    let mut app = build_app_with_three_lines();

    // Caret at offset 5 (over 'f' on line 0).
    dispatch(&mut app, key(KeyCode::Up));
    dispatch(&mut app, key(KeyCode::Up));
    dispatch(&mut app, key(KeyCode::Home));
    for _ in 0..5 {
        dispatch(&mut app, key(KeyCode::Right));
    }

    // Down → line 1 clamped to col 3 (offset 11). Sticky-x = 5.
    dispatch(&mut app, key(KeyCode::Down));
    assert_eq!(app.dom().selection().unwrap().focus.offset, 11);

    // Left → reset sticky-x. Caret moves to offset 10.
    dispatch(&mut app, key(KeyCode::Left));
    assert_eq!(app.dom().selection().unwrap().focus.offset, 10);

    // Down → line 2 at column 2 (the current column after Left),
    // NOT column 5. Offset = 12 + 2 = 14.
    dispatch(&mut app, key(KeyCode::Down));
    assert_eq!(
        app.dom().selection().unwrap().focus.offset,
        14,
        "Down after Left must use current column (2), not stale sticky-x (5)"
    );
}

/// Build an App with a textarea containing three lines:
///   line 0: "abcdefg"  offsets 0..7
///   line 1: "abc"      offsets 8..11
///   line 2: "abcdefg"  offsets 12..19
/// Tab-focused. Caret at offset 19 (end of all content).
fn build_app_with_three_lines() -> App<TestBackend> {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let textarea = dom.create_element("textarea");
    dom.append_child(root, textarea).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "textarea",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(5))
            .white_space(WhiteSpace::PreWrap),
    );

    let backend = TestBackend::new(40, 10);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();

    dispatch(&mut app, key(KeyCode::Tab));
    for c in "abcdefg".chars() {
        dispatch(&mut app, ch(c));
    }
    dispatch(&mut app, key(KeyCode::Enter));
    for c in "abc".chars() {
        dispatch(&mut app, ch(c));
    }
    dispatch(&mut app, key(KeyCode::Enter));
    for c in "abcdefg".chars() {
        dispatch(&mut app, ch(c));
    }

    app
}

#[test]
fn caret_color_blue_paints_caret_cell_with_blue_bg() {
    // `caret-color: blue` must paint the caret cell's bg with the
    // resolved color value. No `REVERSED` modifier — the caret is
    // a pure fg/bg swap (or override) at the painted cell.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let textarea = dom.create_element("textarea");
    dom.append_child(root, textarea).unwrap();

    let sheet = Stylesheet::new()
        .rule_unchecked(
            "textarea",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .height(Size::Fixed(3))
                .white_space(WhiteSpace::PreWrap),
        )
        .rule_unchecked(
            "textarea:focus",
            TuiStyle::new().caret_color(rdom_tui::layout::CaretColor::Color(
                rdom_tui::TuiColor::Literal(rdom_tui::style::Color::Rgb(0x1E, 0x90, 0xFF)),
            )),
        );

    let backend = TestBackend::new(40, 10);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();

    dispatch(&mut app, key(KeyCode::Tab));

    let sheet_for_render = Stylesheet::new()
        .rule_unchecked(
            "textarea",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .height(Size::Fixed(3))
                .white_space(WhiteSpace::PreWrap),
        )
        .rule_unchecked(
            "textarea:focus",
            TuiStyle::new().caret_color(rdom_tui::layout::CaretColor::Color(
                rdom_tui::TuiColor::Literal(rdom_tui::style::Color::Rgb(0x1E, 0x90, 0xFF)),
            )),
        );
    let buf = render(app.dom_mut(), &sheet_for_render, Rect::new(0, 0, 40, 10));

    // Find a cell with bg = Rgb(0x1E, 0x90, 0xFF). That's the caret.
    let mut found_blue_bg = false;
    let ta_rect = Rect::new(0, 0, 40, 4);
    for y in ta_rect.y..ta_rect.bottom() {
        for x in ta_rect.x..ta_rect.right() {
            if let Some(c) = buf.cell(x, y)
                && c.bg == rdom_tui::style::Color::Rgb(0x1E, 0x90, 0xFF)
            {
                found_blue_bg = true;
            }
        }
    }
    assert!(
        found_blue_bg,
        "caret-color: blue should paint a cell with bg = Rgb(0x1E, 0x90, 0xFF)"
    );
}

#[test]
fn caret_text_color_paints_glyph_with_specified_fg() {
    // `caret-text-color: <color>` must paint the caret cell's fg
    // (the glyph color) with the resolved value. Pair with a
    // caret-color so we can deterministically identify the caret
    // cell.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let textarea = dom.create_element("textarea");
    dom.append_child(root, textarea).unwrap();

    let sheet = Stylesheet::new()
        .rule_unchecked(
            "textarea",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .height(Size::Fixed(3))
                .white_space(WhiteSpace::PreWrap),
        )
        .rule_unchecked(
            "textarea:focus",
            TuiStyle::new()
                .caret_color(rdom_tui::layout::CaretColor::Color(
                    rdom_tui::TuiColor::Literal(rdom_tui::style::Color::Rgb(0x1E, 0x90, 0xFF)),
                ))
                .caret_text_color(rdom_tui::layout::CaretTextColor::Color(
                    rdom_tui::TuiColor::Literal(rdom_tui::style::Color::Rgb(0xFF, 0xFF, 0x00)),
                )),
        );

    let backend = TestBackend::new(40, 10);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();

    dispatch(&mut app, key(KeyCode::Tab));

    let sheet_for_render = Stylesheet::new()
        .rule_unchecked(
            "textarea",
            TuiStyle::new()
                .width(Size::Fixed(20))
                .height(Size::Fixed(3))
                .white_space(WhiteSpace::PreWrap),
        )
        .rule_unchecked(
            "textarea:focus",
            TuiStyle::new()
                .caret_color(rdom_tui::layout::CaretColor::Color(
                    rdom_tui::TuiColor::Literal(rdom_tui::style::Color::Rgb(0x1E, 0x90, 0xFF)),
                ))
                .caret_text_color(rdom_tui::layout::CaretTextColor::Color(
                    rdom_tui::TuiColor::Literal(rdom_tui::style::Color::Rgb(0xFF, 0xFF, 0x00)),
                )),
        );
    let buf = render(app.dom_mut(), &sheet_for_render, Rect::new(0, 0, 40, 10));

    // The caret cell must have bg=blue AND fg=yellow.
    let mut found = false;
    let ta_rect = Rect::new(0, 0, 40, 4);
    for y in ta_rect.y..ta_rect.bottom() {
        for x in ta_rect.x..ta_rect.right() {
            if let Some(c) = buf.cell(x, y)
                && c.bg == rdom_tui::style::Color::Rgb(0x1E, 0x90, 0xFF)
                && c.fg == rdom_tui::style::Color::Rgb(0xFF, 0xFF, 0x00)
            {
                found = true;
            }
        }
    }
    assert!(
        found,
        "caret-color: blue + caret-text-color: yellow should produce a cell with bg=blue, fg=yellow"
    );
}

#[test]
fn default_caret_inverts_underlying_cell_colors() {
    // No `caret-color` / `caret-text-color` set. The default Auto
    // behavior must paint the caret cell with bg = underlying cell's
    // fg, fg = underlying cell's bg. This reproduces the visual of
    // the old REVERSED modifier without using SGR-7.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let textarea = dom.create_element("textarea");
    dom.append_child(root, textarea).unwrap();

    // Author rule: textarea has bg = dark gray (custom), fg = white.
    // With default caret-color (Auto), caret cell should be
    // bg=white (was fg), fg=dark-gray (was bg).
    let sheet = Stylesheet::new().rule_unchecked(
        "textarea",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(2))
            .white_space(WhiteSpace::PreWrap)
            .bg(rdom_tui::style::Color::Rgb(0x10, 0x10, 0x10))
            .fg(rdom_tui::style::Color::Rgb(0xEE, 0xEE, 0xEE)),
    );

    let backend = TestBackend::new(40, 10);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();

    dispatch(&mut app, key(KeyCode::Tab));

    let sheet_for_render = Stylesheet::new().rule_unchecked(
        "textarea",
        TuiStyle::new()
            .width(Size::Fixed(10))
            .height(Size::Fixed(2))
            .white_space(WhiteSpace::PreWrap)
            .bg(rdom_tui::style::Color::Rgb(0x10, 0x10, 0x10))
            .fg(rdom_tui::style::Color::Rgb(0xEE, 0xEE, 0xEE)),
    );
    let buf = render(app.dom_mut(), &sheet_for_render, Rect::new(0, 0, 40, 10));

    // Author sets fg=#EEEEEE on textarea. The UA's
    // `:focus { background-color: #2d2f31 !important }` wins over
    // the author's bg when focused, so the focused textarea's
    // CASCADED values are: fg=#EEEEEE (author), bg=#2d2f31 (UA).
    // Auto caret resolves to: caret_bg=cascaded_fg=#EEEEEE,
    // caret_fg=cascaded_bg=#2d2f31. That's the inversion.
    let mut found_inverted = false;
    let ta_rect = Rect::new(0, 0, 40, 3);
    for y in ta_rect.y..ta_rect.bottom() {
        for x in ta_rect.x..ta_rect.right() {
            if let Some(c) = buf.cell(x, y)
                && c.bg == rdom_tui::style::Color::Rgb(0xEE, 0xEE, 0xEE)
                && c.fg == rdom_tui::style::Color::Rgb(0x2D, 0x2F, 0x31)
            {
                found_inverted = true;
            }
        }
    }
    assert!(
        found_inverted,
        "default caret (caret-color: auto) must produce a cell whose bg = cascaded fg and fg = cascaded bg"
    );
}

fn mouse_down(x: u16, y: u16) -> CtEvent {
    CtEvent::Mouse(CtMouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    })
}

fn mouse_drag(x: u16, y: u16) -> CtEvent {
    CtEvent::Mouse(CtMouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    })
}

fn mouse_up(x: u16, y: u16) -> CtEvent {
    CtEvent::Mouse(CtMouseEvent {
        kind: MouseEventKind::Up(MouseButton::Left),
        column: x,
        row: y,
        modifiers: KeyModifiers::empty(),
    })
}

#[test]
fn drag_selection_past_end_of_line_includes_last_char() {
    // Browser behavior: dragging the mouse from the start of a line
    // to PAST the last character selects the entire line including
    // the final character. rdom's hit-test returns None for cells
    // past the line's content, so a naive drag handler leaves the
    // selection's focus stuck at the last-char position (one short
    // of the line's end). Same class of bug as the Down-arrow
    // clamping fix — needs an end-of-line fallback.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let textarea = dom.create_element("textarea");
    dom.append_child(root, textarea).unwrap();

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

    // Focus + type "abcdefg" (7 chars). Caret ends at offset 7.
    dispatch(&mut app, key(KeyCode::Tab));
    for c in "abcdefg".chars() {
        dispatch(&mut app, ch(c));
    }
    assert_eq!(textarea_text(app.dom(), textarea), "abcdefg");

    // Mouse: press at the textarea's first content cell, drag well
    // past the last char, release. The textarea sits at (0, 0)
    // with a Fixed(20) width; content starts at column 1 (padding).
    // "abcdefg" occupies columns 1..8 (cells 1, 2, ..., 7).
    // Cell 8 is past the last char — column 7 holds 'g'.
    //
    // Drag to column 30 = way past the text — emulates the user
    // dragging beyond the line's end.
    dispatch(&mut app, mouse_down(1, 0));
    dispatch(&mut app, mouse_drag(30, 0));
    dispatch(&mut app, mouse_up(30, 0));

    // Selection should span offset 0 to offset 7 (the full text).
    // Selection model: anchor=mouse-down, focus=mouse-up. Both offsets
    // are checked since the order depends on drag direction (here:
    // forward, so anchor < focus).
    let sel = app.dom().selection().expect("selection must exist");
    let (lo, hi) = if sel.anchor.offset <= sel.focus.offset {
        (sel.anchor.offset, sel.focus.offset)
    } else {
        (sel.focus.offset, sel.anchor.offset)
    };
    assert_eq!(
        (lo, hi),
        (0, 7),
        "dragging past end-of-line must include the last character (selection should be 0..7); got ({}, {})",
        lo,
        hi
    );
}

#[test]
fn shift_end_extends_selection_to_line_end() {
    // Browser: Shift+End from a caret in the middle of a line
    // extends the selection focus to the end of that line. Anchor
    // stays put.
    let mut app = build_app_with_three_lines();

    // Caret currently at offset 19 (end of all content). Move it
    // to line 1 (middle line, "abc") at offset 9 (between 'a' and 'b').
    dispatch(&mut app, key(KeyCode::Up));
    dispatch(&mut app, key(KeyCode::Home));
    dispatch(&mut app, key(KeyCode::Right));
    let off = app.dom().selection().unwrap().focus.offset;
    assert_eq!(off, 9, "caret should be at line 1, offset 9 (after 'a')");

    // Shift+End → extend to end of line 1 (offset 11, after 'c').
    dispatch(&mut app, shift(KeyCode::End));
    let sel = app.dom().selection().unwrap();
    assert_eq!(sel.anchor.offset, 9, "anchor stays put after Shift+End");
    assert_eq!(
        sel.focus.offset, 11,
        "Shift+End extends focus to end of current line"
    );
}

#[test]
fn shift_home_extends_selection_to_line_start() {
    // Browser: Shift+Home extends selection focus to start of line.
    let mut app = build_app_with_three_lines();

    // Move caret to line 1 offset 10 (between 'b' and 'c').
    dispatch(&mut app, key(KeyCode::Up));
    dispatch(&mut app, key(KeyCode::Home));
    dispatch(&mut app, key(KeyCode::Right));
    dispatch(&mut app, key(KeyCode::Right));
    let off = app.dom().selection().unwrap().focus.offset;
    assert_eq!(off, 10, "caret should be at line 1, offset 10 (after 'b')");

    // Shift+Home → extend to start of line 1 (offset 8).
    dispatch(&mut app, shift(KeyCode::Home));
    let sel = app.dom().selection().unwrap();
    assert_eq!(sel.anchor.offset, 10, "anchor stays put after Shift+Home");
    assert_eq!(
        sel.focus.offset, 8,
        "Shift+Home extends focus to start of current line"
    );
}

#[test]
fn shift_down_extends_selection_with_sticky_x() {
    // Browser: Shift+Down extends selection focus to the next line
    // at the same column (sticky-x), clamped to that line's width.
    // Anchor stays. Subsequent Shift+Down preserves sticky-x.
    let mut app = build_app_with_three_lines();

    // Place caret at line 0 offset 5 (over 'f').
    dispatch(&mut app, key(KeyCode::Up));
    dispatch(&mut app, key(KeyCode::Up));
    dispatch(&mut app, key(KeyCode::Home));
    for _ in 0..5 {
        dispatch(&mut app, key(KeyCode::Right));
    }
    assert_eq!(app.dom().selection().unwrap().focus.offset, 5);

    // Shift+Down → focus moves to line 1, sticky-x = 5, clamped to
    // line 1's width (3) → end of line 1 = offset 11.
    dispatch(&mut app, shift(KeyCode::Down));
    let sel = app.dom().selection().unwrap();
    assert_eq!(sel.anchor.offset, 5, "anchor stays");
    assert_eq!(
        sel.focus.offset, 11,
        "Shift+Down clamps focus to end of short line, sticky-x preserved"
    );

    // Another Shift+Down → line 2 at sticky col 5 → offset 17.
    dispatch(&mut app, shift(KeyCode::Down));
    let sel = app.dom().selection().unwrap();
    assert_eq!(sel.anchor.offset, 5);
    assert_eq!(
        sel.focus.offset, 17,
        "second Shift+Down restores sticky-x to col 5 on wider line"
    );
}

#[test]
fn shift_up_extends_selection_with_sticky_x() {
    // Symmetric to Shift+Down. Caret at line 2 offset 17 (over 'f').
    // Shift+Up → line 1 clamped to 11. Shift+Up → line 0 col 5 = offset 5.
    let mut app = build_app_with_three_lines();

    // Move to line 2 offset 17.
    dispatch(&mut app, key(KeyCode::Home));
    for _ in 0..5 {
        dispatch(&mut app, key(KeyCode::Right));
    }
    assert_eq!(app.dom().selection().unwrap().focus.offset, 17);

    dispatch(&mut app, shift(KeyCode::Up));
    let sel = app.dom().selection().unwrap();
    assert_eq!(sel.anchor.offset, 17);
    assert_eq!(
        sel.focus.offset, 11,
        "Shift+Up clamps focus to end of short line, sticky-x preserved"
    );

    dispatch(&mut app, shift(KeyCode::Up));
    let sel = app.dom().selection().unwrap();
    assert_eq!(sel.anchor.offset, 17);
    assert_eq!(
        sel.focus.offset, 5,
        "second Shift+Up restores sticky-x to col 5 on wider line"
    );
}

#[test]
fn drag_select_inside_enabled_input_creates_selection() {
    // Positive control for the next test (`disabled` blocks drag).
    // Demonstrates that drag inside an enabled input DOES create
    // a selection — without this, the disabled test could be a
    // false positive (passing because the drag plumbing never
    // works at all on inputs).
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.set_attribute(input, "type", "text").unwrap();
    dom.set_attribute(input, "value", "hello").unwrap();
    dom.append_child(root, input).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "input",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(1)),
    );

    let backend = TestBackend::new(40, 5);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    // Force the first layout pass so inline_layout is populated
    // before any mouse event — otherwise the initial mouse_down's
    // hit-test happens against an empty layout and `position_at`
    // returns None. Production run loops draw before reading
    // input; the integration helper has to do the same.
    let _ = app.draw_if_dirty();

    dispatch(&mut app, mouse_down(2, 0));
    dispatch(&mut app, mouse_drag(6, 0));
    dispatch(&mut app, mouse_up(6, 0));

    let sel = app.dom().selection();
    assert!(
        sel.is_some(),
        "drag inside ENABLED input must create a selection"
    );
    let sel = sel.unwrap();
    assert!(
        sel.anchor.offset != sel.focus.offset,
        "selection must be a range (not collapsed)"
    );
}

#[test]
fn drag_select_inside_disabled_input_does_not_select_or_focus() {
    // Browser behavior: a disabled <input> doesn't accept focus and
    // doesn't allow text selection. Mouse-down inside a disabled
    // input is effectively a no-op for both focus and selection.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let input = dom.create_element("input");
    dom.set_attribute(input, "type", "text").unwrap();
    dom.set_attribute(input, "value", "hello").unwrap();
    dom.set_attribute(input, "disabled", "").unwrap();
    dom.append_child(root, input).unwrap();

    let sheet = Stylesheet::new().rule_unchecked(
        "input",
        TuiStyle::new()
            .width(Size::Fixed(20))
            .height(Size::Fixed(1)),
    );

    let backend = TestBackend::new(40, 5);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, sheet, terminal).unwrap();
    // Force the initial layout so `position_at` would resolve a
    // cell to a text-node Position if it were going to. Without
    // this, the test passes for the wrong reason (no layout
    // ever ran, so position_at would have returned None even on
    // an enabled input).
    let _ = app.draw_if_dirty();

    // Mouse-down + drag across the disabled input's content. Must
    // NOT produce a selection.
    dispatch(&mut app, mouse_down(2, 0));
    dispatch(&mut app, mouse_drag(8, 0));
    dispatch(&mut app, mouse_up(8, 0));

    assert!(
        app.dom().selection().is_none(),
        "disabled input must not produce a selection on drag; got {:?}",
        app.dom().selection()
    );
    assert_ne!(
        app.dom().focused(),
        Some(input),
        "disabled input must not receive focus on click"
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
