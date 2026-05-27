//! Sidebar keyboard navigation pipeline tests.
//!
//! User-reported (2026-05-25): the showcase booted with nothing
//! focused, and ArrowDown after Tab-Tab seemed to highlight every
//! other demo only — items in between rendered with no focus
//! indicator. Root cause: the sidebar didn't set `flex-shrink: 0`
//! on its `<summary>` / `<li>` children, and on a typical small
//! terminal the nav was taller than the viewport. Flex shrink
//! dropped some entries to zero height (per `M5-MIN-CONTENT-1`),
//! stacking multiple items at the same paint row — so focus IS
//! advancing one li per ArrowDown, but the now-focused row was
//! often a 0-height entry painted under a visible sibling.
//!
//! These tests pin three things:
//!
//! 1. On app start, the first sidebar `<li>` is focused via
//!    `autofocus` so the user can press arrow keys immediately.
//! 2. After each ArrowDown the focused element is the next
//!    demo `<li>` in document order (handler correctness — also
//!    covered by `nav::tests::next_demo_li_*`).
//! 3. Every sidebar entry that should be visible paints at a
//!    UNIQUE y row — no zero-height squish, no overlap. Drives
//!    the App through the full cascade + layout + paint pipeline
//!    against a fixed viewport.

use crossterm::event::{Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use rdom_showcase::{
    DEMOS, ShowcaseState, build_shell, mount_demo, shell::base_stylesheet, wire_sidebar_keys,
};
use rdom_tui::render::{Buffer, PaintExt, Rect, Terminal, TestBackend};
use rdom_tui::{App, CascadeExt, LayoutExt, NodeId, TuiDom};

fn key_press(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::empty(),
    })
}

fn focused_li_slug(app: &App<TestBackend>) -> Option<String> {
    let dom = app.dom();
    let focused = dom.focused()?;
    let node = dom.node(focused);
    if node.tag_name() != Some("li") {
        return None;
    }
    node.get_attribute("data-demo-slug").map(|s| s.to_string())
}

fn build_app(viewport: Rect) -> App<TestBackend> {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);

    let state = std::rc::Rc::new(std::cell::RefCell::new(ShowcaseState::from_handles(
        &handles,
    )));
    mount_demo(&mut state.borrow_mut(), &mut dom, 0);
    wire_sidebar_keys(&mut dom, handles.sidebar, std::rc::Rc::clone(&state));

    let backend = TestBackend::new(viewport.width, viewport.height);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, base_stylesheet(), terminal).unwrap();
    for demo in DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }
    app.draw_if_dirty().unwrap();
    app
}

#[test]
fn first_demo_li_is_focused_on_startup() {
    let app = build_app(Rect::new(0, 0, 80, 24));
    assert_eq!(
        focused_li_slug(&app).as_deref(),
        Some(DEMOS[0].slug()),
        "the first sidebar <li> should have autofocus so the keyboard nav is usable on first paint"
    );
}

#[test]
fn arrow_down_advances_one_demo_li_per_press_in_document_order() {
    let mut app = build_app(Rect::new(0, 0, 80, 24));

    // Sidebar groups demos by Category, so document order is
    // category-first, then registry order within each category.
    // Re-derive it here rather than reusing registry order
    // directly — registry order interleaves categories.
    let mut expected: Vec<&'static str> = Vec::new();
    let mut seen_cats: Vec<rdom_showcase::Category> = Vec::new();
    for d in DEMOS {
        if !seen_cats.contains(&d.category()) {
            seen_cats.push(d.category());
        }
    }
    for cat in &seen_cats {
        for d in DEMOS.iter().filter(|d| d.category() == *cat) {
            expected.push(d.slug());
        }
    }

    let mut actual: Vec<String> = Vec::new();
    actual.push(focused_li_slug(&app).unwrap_or_default());
    for _ in 1..expected.len() {
        app.handle_event(key_press(KeyCode::Down));
        app.draw_if_dirty().unwrap();
        actual.push(focused_li_slug(&app).unwrap_or_default());
    }
    let actual_strs: Vec<&str> = actual.iter().map(String::as_str).collect();
    assert_eq!(
        actual_strs, expected,
        "ArrowDown should advance to each next demo li in sidebar document order"
    );
}

#[test]
fn every_sidebar_entry_paints_at_a_unique_row() {
    // Regression for the user-reported "highlight disappears"
    // visual: when the sidebar exceeds the viewport height,
    // flex-shrink (default 1) used to drop some entries to zero
    // height, stacking them under siblings at the same y. Now
    // sidebar children have `flex-shrink: 0` + the sidebar uses
    // `overflow-y: auto`, so every entry that fits in the visible
    // window owns its own row.
    let mut app = build_app(Rect::new(0, 0, 80, 24));
    // Re-cascade + relayout + paint to inspect cells (TestBackend
    // captures ANSI bytes; we want addressable cells).
    let viewport = Rect::new(0, 0, 80, 24);
    let mut buf = Buffer::empty(viewport);
    let sheet = base_stylesheet();
    let mut all_sheets = vec![sheet];
    for demo in DEMOS {
        all_sheets.push(demo.stylesheet());
    }
    let dom = app.dom_mut();
    let refs: Vec<&_> = all_sheets.iter().collect();
    dom.cascade_all(&refs);
    dom.layout_dom(viewport);
    dom.paint_dom(&mut buf, viewport);

    // Collect every sidebar `<li>` + `<summary>` and its painted
    // y row (from the layout rect). Items entirely outside the
    // viewport are allowed to overlap (scrolled out of view);
    // visible items must each have a unique row.
    fn collect(dom: &TuiDom, id: NodeId, out: &mut Vec<(NodeId, &'static str)>) {
        let n = dom.node(id);
        match n.tag_name() {
            Some("li") if n.get_attribute("data-demo-slug").is_some() => {
                out.push((id, "li"));
            }
            Some("summary") => out.push((id, "summary")),
            _ => {}
        }
        for c in n.child_nodes() {
            collect(dom, c.id(), out);
        }
    }
    let mut entries = Vec::new();
    let dom_ref = app.dom();
    let handles_sidebar = {
        // Re-derive sidebar id by walking from root: first `<aside>`.
        fn first_aside(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
            let n = dom.node(id);
            if n.tag_name() == Some("aside") {
                return Some(id);
            }
            for c in n.child_nodes() {
                if let Some(found) = first_aside(dom, c.id()) {
                    return Some(found);
                }
            }
            None
        }
        first_aside(dom_ref, dom_ref.root()).expect("sidebar exists")
    };
    collect(dom_ref, handles_sidebar, &mut entries);

    let viewport_h = viewport.height as i32;
    let mut visible_rows: Vec<(i32, NodeId, &'static str, String)> = Vec::new();
    for (id, kind) in &entries {
        let ext = dom_ref.node(*id).ext();
        let layout = ext.map(|e| e.layout).unwrap_or_default();
        // Skip entries with zero or off-viewport rect; we only
        // care about what the user actually sees.
        if layout.height == 0 {
            // Zero-height = the bug we're trying to prevent.
            let label = dom_ref.text_content(*id);
            panic!(
                "{kind} {label:?} has zero painted height — flex-shrink squish is back. \
                 See M5-MIN-CONTENT-1.",
            );
        }
        if layout.y < 0 || layout.y >= viewport_h {
            continue;
        }
        let label = dom_ref.text_content(*id);
        visible_rows.push((layout.y, *id, kind, label));
    }

    // Check uniqueness of y among visible entries.
    let mut by_row: std::collections::BTreeMap<i32, Vec<String>> = Default::default();
    for (y, _, kind, label) in &visible_rows {
        by_row
            .entry(*y)
            .or_default()
            .push(format!("{kind}:{}", label.trim()));
    }
    let collisions: Vec<_> = by_row.iter().filter(|(_, v)| v.len() > 1).collect();
    assert!(
        collisions.is_empty(),
        "multiple sidebar entries painted at the same y row: {:#?}",
        collisions
    );
}
