//! Pin the contract: the showcase status bar (after `build_shell` +
//! `seed_default_hints`) must paint hints with proper whitespace
//! AND a mouse-position slot reachable from `wire_mouse_position_indicator`.
//!
//! User report: in the running binary they see "↑↓navigate·
//! Enterselect" — whitespace between key and label spans missing,
//! mouse coords don't appear at all. This file builds the exact
//! showcase setup, paints, and verifies.

use rdom_showcase::{build_shell, shell::base_stylesheet, wire_mouse_position_indicator};
use rdom_tui::node::TuiNodeExt;
use rdom_tui::render::{Buffer, Rect};
use rdom_tui::{CascadeExt, LayoutExt, PaintExt, Stylesheet, TuiDom};

fn build_and_paint(width: u16, height: u16) -> (TuiDom, Buffer) {
    let mut dom: TuiDom = TuiDom::new();
    let _ = build_shell(&mut dom);
    let base = base_stylesheet();
    // Combine with the UA stylesheet (which `App::new` would
    // normally seed). `Stylesheet::new()` IS the UA sheet, so we
    // hand both to cascade_all in stack order.
    let ua = Stylesheet::new();
    let sheets: Vec<&Stylesheet> = vec![&ua, &base];
    dom.cascade_all(&sheets);
    dom.layout_dom(Rect::new(0, 0, width, height));
    let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
    dom.paint_dom(&mut buf, Rect::new(0, 0, width, height));
    (dom, buf)
}

fn last_row_text(buf: &Buffer) -> String {
    let y = buf.area.height - 1;
    let mut s = String::new();
    for x in 0..buf.area.width {
        if let Some(c) = buf.cell(x, y) {
            if c.is_spacer() {
                continue;
            }
            s.push_str(c.symbol());
        }
    }
    s
}

#[test]
fn status_bar_hints_with_flex_row_demo_mounted_using_real_app() {
    // EXACT reproduction of the running showcase: build_shell +
    // mount a demo + use the real `App::with_backend` pipeline +
    // push every demo stylesheet (just like `main.rs` does) + use
    // App's `draw_if_dirty` (NOT `cascade_all` directly) so the
    // incremental cascade-via-dirty-tracker path runs.
    //
    // User report: "↑↓navigate· Enterselect" — whitespace
    // missing between key and label spans. If my other tests
    // pass but this one fails, the difference is the
    // app-pipeline path vs. the manual cascade+layout path.
    use rdom_showcase::{DEMOS, ShowcaseState, mount_demo};
    use rdom_tui::App;
    use rdom_tui::render::{Terminal, TestBackend};

    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    let mut state = ShowcaseState::from_handles(&handles);
    let flex_row_idx = DEMOS
        .iter()
        .position(|d| d.slug() == "layout/flex-row")
        .expect("flex-row demo registered");
    mount_demo(&mut state, &mut dom, flex_row_idx);

    let backend = TestBackend::new(123, 26);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, base_stylesheet(), terminal).unwrap();
    for demo in DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }
    app.draw_if_dirty().unwrap();

    // Locate the status bar's hints slot via the handle exposed
    // by build_shell. Find its painted row.
    let hints_rect = app
        .dom()
        .node(handles.status_bar_hints)
        .tui_ext()
        .unwrap()
        .layout;
    eprintln!("DBG hints_rect = {hints_rect:?}");
    let main_rect = app.dom().node(handles.status_bar).tui_ext().unwrap().layout;
    eprintln!("DBG status_bar rect = {main_rect:?}");

    // Paint the actual last row of the buffer.
    let buf_bytes = app.terminal().backend().bytes();
    let painted = String::from_utf8_lossy(buf_bytes).to_string();
    eprintln!("DBG painted (raw byte stream): {painted:?}");

    // Also: paint via dom.paint_dom into a fresh buffer for
    // direct inspection.
    use rdom_tui::PaintExt;
    let mut buf = rdom_tui::render::Buffer::empty(Rect::new(0, 0, 123, 26));
    app.dom().paint_dom(&mut buf, Rect::new(0, 0, 123, 26));
    let last = last_row_text(&buf);
    eprintln!("DBG via direct paint_dom last row: {last:?}");
    assert!(
        last.contains("↑↓ navigate"),
        "showcase status-bar must paint whitespace between key and label \
         (real-app path); got {last:?}"
    );
}

#[test]
fn status_bar_hints_have_whitespace_between_key_and_label() {
    // `seed_default_hints` populates the hints slot with
    // `↑↓ navigate · Enter select`. After paint, the bottom row of
    // the buffer must contain at least "↑↓ navigate" (with the space).
    let (_dom, buf) = build_and_paint(80, 24);
    let row = last_row_text(&buf);
    eprintln!("DBG status-bar row: {row:?}");
    assert!(
        row.contains("↑↓ navigate"),
        "status bar must paint whitespace between key and label spans; got {row:?}"
    );
    assert!(
        row.contains("Enter select"),
        "status bar must paint whitespace in the second hint too; got {row:?}"
    );
}

#[test]
fn status_bar_mouse_pos_slot_is_paintable() {
    // Mount the mouse-position listener and synthesize a node-value
    // write directly to the slot to verify the slot itself can hold
    // and render text. (Synthesizing a real `mousemove` event takes
    // a router setup; we just test the rendering path here.)
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);
    wire_mouse_position_indicator(&mut dom, handles.status_bar_mouse_pos);

    // Directly write a text node into the mouse-pos slot. This is
    // what the listener does when a mousemove event fires.
    let t = dom.create_text_node("X: 42 Y: 7");
    dom.append_child(handles.status_bar_mouse_pos, t).unwrap();

    let ua = Stylesheet::new();
    let base = base_stylesheet();
    let sheets: Vec<&Stylesheet> = vec![&ua, &base];
    dom.cascade_all(&sheets);
    dom.layout_dom(Rect::new(0, 0, 80, 24));
    let mut buf = Buffer::empty(Rect::new(0, 0, 80, 24));
    dom.paint_dom(&mut buf, Rect::new(0, 0, 80, 24));

    let row = last_row_text(&buf);
    eprintln!("DBG status-bar row: {row:?}");
    let mp = dom
        .node(handles.status_bar_mouse_pos)
        .tui_ext()
        .unwrap()
        .layout;
    eprintln!("DBG mouse-pos slot layout: {mp:?}");
    assert!(
        row.contains("X: 42 Y: 7"),
        "mouse-pos slot must render the text we wrote into it; got {row:?}"
    );
}

#[test]
fn appending_text_to_empty_slot_reflows_flex_distribution() {
    // The EXACT showcase scenario: at startup the mouse-pos slot
    // is empty, so the flex distribution gives all width to the
    // hints slot (`flex: 1` left + intrinsic-width right means
    // left grabs all space, right gets 0). When the mousemove
    // listener writes "X: 42 Y: 7" into the empty slot, the
    // mutation must mark the parent flex container dirty so the
    // next cascade-and-layout reallocates space and gives the
    // slot non-zero width. Pinned at the substrate level by
    // `crates/rdom-tui/tests/integration/append_text_reflows_layout.rs`;
    // this end-to-end test verifies the showcase wiring picks it
    // up via the actual cascade pipeline.
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);

    let base = base_stylesheet();
    let ua = Stylesheet::new();
    let sheets: Vec<&Stylesheet> = vec![&ua, &base];
    dom.cascade_all(&sheets);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    let initial = dom
        .node(handles.status_bar_mouse_pos)
        .tui_ext()
        .unwrap()
        .layout;
    eprintln!("DBG initial slot rect: {initial:?}");
    assert_eq!(
        initial.width, 0,
        "empty slot starts with width 0 (flex distributes to hints); got {initial:?}"
    );

    // Append text to the empty slot — what the mousemove listener
    // does in production.
    let t = dom.create_text_node("X: 42 Y: 7");
    dom.append_child(handles.status_bar_mouse_pos, t).unwrap();

    // Re-cascade. With the dirty-tracker fix, ChildListChanged
    // marked the parent flex container dirty; cascade_all
    // re-flows the flex distribution and the slot picks up its
    // intrinsic width.
    dom.cascade_all(&sheets);
    dom.layout_dom(Rect::new(0, 0, 80, 24));

    let after = dom
        .node(handles.status_bar_mouse_pos)
        .tui_ext()
        .unwrap()
        .layout;
    eprintln!("DBG slot rect after text append: {after:?}");
    assert!(
        after.width > 0,
        "after appending 'X: 42 Y: 7' to the empty slot, the flex container \
         must re-flow and give the slot a non-zero width; got {after:?}"
    );
}
