//! Sidebar keyboard navigation pipeline tests (TREE-2).
//!
//! The sidebar is now the native ARIA tree built-in
//! (`runtime::builtins::tree`), not a bespoke roving-focus handler.
//! The `[role=tree]` container is the single tab stop and holds
//! focus; an internal `data-rdom-active` marker is the cursor.
//! These tests pin the showcase-level contract:
//!
//! 1. On app start the tree container is focused (via `autofocus`)
//!    and the active-descendant cursor is seeded on the mounted
//!    demo's leaf — keyboard nav is live and visibly anchored on
//!    first paint, no Tab required.
//! 2. ArrowDown advances the cursor to the next visible treeitem in
//!    document order (the built-in's job — pinned end-to-end through
//!    the App's keyboard router here).
//! 3. Enter on a demo leaf mounts that demo (the built-in dispatches
//!    a bubbling `click`; `wire_sidebar_click` turns it into a mount).
//! 4. Every visible sidebar treeitem paints at a UNIQUE y row — no
//!    zero-height squish, no overlap (the original "highlight
//!    disappears" regression, re-pinned for the tree structure).

use crossterm::event::{Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use rdom_showcase::{
    DEMOS, ShellHandles, ShowcaseState, build_shell, mount_demo, seed_tree_cursor,
    shell::base_stylesheet, wire_sidebar_click,
};
use rdom_tui::render::{Buffer, PaintExt, Rect, Terminal, TestBackend};
use rdom_tui::{App, CascadeExt, LayoutExt, NodeId, TuiDom};
use std::cell::RefCell;
use std::rc::Rc;

fn key_press(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::empty(),
    })
}

/// Build the showcase app exactly as `main::run` does: mount demo 0,
/// seed the tree cursor, wire the single click handler. Returns the
/// App, the shared nav state (for asserting which demo is mounted),
/// and the shell handles.
fn build_app(viewport: Rect) -> (App<TestBackend>, Rc<RefCell<ShowcaseState>>, ShellHandles) {
    let mut dom: TuiDom = TuiDom::new();
    let handles = build_shell(&mut dom);

    let state = Rc::new(RefCell::new(ShowcaseState::from_handles(&handles)));
    mount_demo(&mut state.borrow_mut(), &mut dom, 0);
    seed_tree_cursor(&mut dom, handles.sidebar, 0);
    wire_sidebar_click(&mut dom, handles.sidebar, Rc::clone(&state));

    let backend = TestBackend::new(viewport.width, viewport.height);
    let terminal = Terminal::new(backend).unwrap();
    let mut app = App::with_backend(dom, base_stylesheet(), terminal).unwrap();
    for demo in DEMOS {
        app.push_stylesheet(demo.stylesheet());
    }
    app.draw_if_dirty().unwrap();
    (app, state, handles)
}

/// The `<ul role=tree>` navigator under `sidebar`.
fn nav_tree(dom: &TuiDom, sidebar: NodeId) -> NodeId {
    fn walk(dom: &TuiDom, id: NodeId) -> Option<NodeId> {
        let n = dom.node(id);
        if n.tag_name() == Some("ul") && n.get_attribute("role") == Some("tree") {
            return Some(id);
        }
        for c in n.child_nodes() {
            if let Some(f) = walk(dom, c.id()) {
                return Some(f);
            }
        }
        None
    }
    walk(dom, sidebar).expect("sidebar has a <ul role=tree>")
}

/// The node carrying the active-descendant cursor under `root`.
fn active_descendant(dom: &TuiDom, root: NodeId) -> Option<NodeId> {
    for c in dom.node(root).child_nodes() {
        if c.has_attribute("data-rdom-active") {
            return Some(c.id());
        }
        if let Some(f) = active_descendant(dom, c.id()) {
            return Some(f);
        }
    }
    None
}

/// Visible treeitems of `tree` in document order — mirrors the
/// built-in's `visible_items`: descend a branch only when expanded.
fn visible_items(dom: &TuiDom, tree: NodeId) -> Vec<NodeId> {
    fn treeitem_children(dom: &TuiDom, container: NodeId) -> Vec<NodeId> {
        dom.node(container)
            .child_nodes()
            .filter(|n| n.get_attribute("role") == Some("treeitem"))
            .map(|n| n.id())
            .collect()
    }
    fn child_group(dom: &TuiDom, item: NodeId) -> Option<NodeId> {
        dom.node(item)
            .child_nodes()
            .find(|n| n.get_attribute("role") == Some("group"))
            .map(|n| n.id())
    }
    fn collect(dom: &TuiDom, container: NodeId, out: &mut Vec<NodeId>) {
        for item in treeitem_children(dom, container) {
            out.push(item);
            let expanded = dom.node(item).get_attribute("aria-expanded") == Some("true");
            if expanded && let Some(group) = child_group(dom, item) {
                collect(dom, group, out);
            }
        }
    }
    let mut out = Vec::new();
    collect(dom, tree, &mut out);
    out
}

#[test]
fn boot_focuses_the_tree_and_seeds_cursor_on_mounted_demo() {
    let (app, _state, handles) = build_app(Rect::new(0, 0, 80, 24));
    let dom = app.dom();

    let focused = dom.focused().expect("something is focused on boot");
    assert_eq!(
        dom.node(focused).get_attribute("role"),
        Some("tree"),
        "the tree container holds focus on boot (autofocus), so arrows work immediately"
    );

    let tree = nav_tree(dom, handles.sidebar);
    let active = active_descendant(dom, tree).expect("a leaf carries the cursor");
    assert_eq!(
        dom.node(active).get_attribute("data-demo-slug"),
        Some(DEMOS[0].slug()),
        "the active-descendant cursor is seeded on the mounted demo's leaf"
    );
}

#[test]
fn arrowing_past_the_fold_keeps_the_cursor_in_view() {
    // Keyboard nav must scroll the sidebar to keep the active-
    // descendant cursor visible — the ARIA listbox/tree pattern and
    // browser focus scroll-into-view. At a short viewport the sidebar
    // overflows; arrowing the cursor down past the fold must scroll
    // the sidebar so the cursor row stays within the scrollport
    // instead of disappearing below it.
    let (mut app, _state, handles) = build_app(Rect::new(0, 0, 80, 12));
    let tree = nav_tree(app.dom(), handles.sidebar);
    let total = visible_items(app.dom(), tree).len();

    for step in 1..total {
        app.handle_event(key_press(KeyCode::Down));
        app.draw_if_dirty().unwrap();

        let cursor = active_descendant(app.dom(), tree).expect("cursor exists");
        let crect = app
            .dom()
            .node(cursor)
            .ext()
            .map(|e| e.layout)
            .unwrap_or_default();
        // The sidebar's visible content region (scrollport ≈ content box).
        let port = app
            .dom()
            .node(handles.sidebar)
            .ext()
            .map(|e| e.content_layout)
            .unwrap_or_default();
        assert!(
            crect.y >= port.y && crect.y < port.y + port.height as i32,
            "after ArrowDown #{step}, cursor row (y={}) must stay within the sidebar \
             scrollport [{}, {}) — scroll-into-view should follow the cursor",
            crect.y,
            port.y,
            port.y + port.height as i32
        );
    }
}

#[test]
fn end_then_home_scrolls_the_cursor_into_view_both_directions() {
    // End jumps to the last item (scrolls down); Home jumps back to
    // the first (scrolls up). Both must land the cursor in view.
    let (mut app, _state, handles) = build_app(Rect::new(0, 0, 80, 12));
    let tree = nav_tree(app.dom(), handles.sidebar);

    let in_view = |app: &App<TestBackend>| {
        let cursor = active_descendant(app.dom(), tree).unwrap();
        let c = app
            .dom()
            .node(cursor)
            .ext()
            .map(|e| e.layout)
            .unwrap_or_default();
        let port = app
            .dom()
            .node(handles.sidebar)
            .ext()
            .map(|e| e.content_layout)
            .unwrap_or_default();
        c.y >= port.y && c.y < port.y + port.height as i32
    };

    app.handle_event(key_press(KeyCode::End));
    app.draw_if_dirty().unwrap();
    assert!(in_view(&app), "End must scroll the last row into view");

    app.handle_event(key_press(KeyCode::Home));
    app.draw_if_dirty().unwrap();
    assert!(in_view(&app), "Home must scroll back up to the first row");
}

#[test]
fn arrow_down_advances_the_cursor_one_visible_item_at_a_time() {
    let (mut app, _state, handles) = build_app(Rect::new(0, 0, 80, 24));

    let tree = nav_tree(app.dom(), handles.sidebar);
    let order = visible_items(app.dom(), tree);
    let start = {
        let cur = active_descendant(app.dom(), tree).unwrap();
        order.iter().position(|&id| id == cur).unwrap()
    };

    // ArrowDown through the remaining visible items; each press
    // should land on the next item in document order.
    for step in 1..(order.len() - start) {
        app.handle_event(key_press(KeyCode::Down));
        app.draw_if_dirty().unwrap();
        let cur = active_descendant(app.dom(), tree).unwrap();
        assert_eq!(
            cur,
            order[start + step],
            "ArrowDown #{step} should move the cursor to the next visible treeitem"
        );
    }
}

#[test]
fn enter_on_a_demo_leaf_mounts_that_demo() {
    let (mut app, state, handles) = build_app(Rect::new(0, 0, 80, 24));
    assert_eq!(state.borrow().current_idx, 0, "boots on demo 0");

    // From the seeded cursor (demo 0's leaf), ArrowDown lands on the
    // next visible item. The first category (Layout) has several
    // demos, so the next item is a sibling leaf, not a branch.
    let tree = nav_tree(app.dom(), handles.sidebar);
    app.handle_event(key_press(KeyCode::Down));
    app.draw_if_dirty().unwrap();

    let cur = active_descendant(app.dom(), tree).unwrap();
    let slug = app
        .dom()
        .node(cur)
        .get_attribute("data-demo-slug")
        .expect("next visible item is a demo leaf")
        .to_string();
    let expected_idx = DEMOS.iter().position(|d| d.slug() == slug).unwrap();
    assert_ne!(
        expected_idx, 0,
        "cursor moved off the initially-mounted demo"
    );

    app.handle_event(key_press(KeyCode::Enter));
    app.draw_if_dirty().unwrap();
    assert_eq!(
        state.borrow().current_idx,
        expected_idx,
        "Enter on a demo leaf mounts that demo"
    );
}

#[test]
fn enter_on_a_category_branch_toggles_it_without_mounting() {
    let (mut app, state, handles) = build_app(Rect::new(0, 0, 80, 24));
    assert_eq!(state.borrow().current_idx, 0);

    // ArrowUp from the seeded leaf lands on its branch parent; Enter
    // there must collapse the branch, NOT mount a demo (branches
    // carry no data-demo-slug).
    let tree = nav_tree(app.dom(), handles.sidebar);
    app.handle_event(key_press(KeyCode::Up));
    app.draw_if_dirty().unwrap();

    let cur = active_descendant(app.dom(), tree).unwrap();
    assert!(
        app.dom().node(cur).has_attribute("aria-expanded"),
        "ArrowUp from the first leaf lands on its branch parent"
    );
    assert!(
        app.dom()
            .node(cur)
            .get_attribute("data-demo-slug")
            .is_none()
    );
    let was_expanded = app.dom().node(cur).get_attribute("aria-expanded") == Some("true");

    app.handle_event(key_press(KeyCode::Enter));
    app.draw_if_dirty().unwrap();

    assert_eq!(
        state.borrow().current_idx,
        0,
        "Enter on a category branch must not change the mounted demo"
    );
    let now_expanded = app.dom().node(cur).get_attribute("aria-expanded") == Some("true");
    assert_ne!(
        now_expanded, was_expanded,
        "Enter on a branch toggles its expanded state"
    );
}

#[test]
fn cursor_row_highlight_fills_the_cell_left_of_the_content() {
    // The selected/cursor row's highlight must extend one cell to the
    // LEFT of the tree content — filling the gap between the panel
    // border and the first glyph — so it reads as a full-width
    // selection bar. The 1-cell inset rides on the tree's padding
    // (`.sidebar-tree`), and the highlight fills the tree's padding
    // box, so that cell carries the bg while content stays put.
    let (mut app, _state, handles) = build_app(Rect::new(0, 0, 80, 24));
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

    let dom = app.dom();
    let tree = nav_tree(dom, handles.sidebar);
    let cursor = active_descendant(dom, tree).expect("a row is the cursor");

    // The cursor row's y, and the tree's border-box left column (the
    // padding cell the highlight should now reach).
    let cursor_rect = dom.node(cursor).ext().map(|e| e.layout).unwrap_or_default();
    let tree_rect = dom.node(tree).ext().map(|e| e.layout).unwrap_or_default();
    let content_rect = dom
        .node(cursor)
        .ext()
        .map(|e| e.content_layout)
        .unwrap_or_default();

    let hl = rdom_tui::Color::Rgb(0x2d, 0x2f, 0x31);
    let left_col = tree_rect.x as u16;
    let y = cursor_rect.y as u16;

    assert!(
        (left_col as i32) < content_rect.x,
        "the tree's left edge (col {left_col}) must sit left of the content (col {}) — \
         that's the inset cell the highlight should fill",
        content_rect.x
    );
    assert_eq!(
        buf.cell(left_col, y).map(|c| c.bg),
        Some(hl),
        "cursor-row highlight must fill the inset cell at the tree's left edge (col {left_col}, row {y})"
    );
}

#[test]
fn every_visible_sidebar_treeitem_paints_at_a_unique_row() {
    // Regression for the user-reported "highlight disappears"
    // visual: when the sidebar exceeds the viewport height,
    // flex-shrink (default 1) used to drop some entries to zero
    // height, stacking them under siblings at the same y. Every
    // visible treeitem (branch + leaf) must own its own row.
    let (mut app, _state, handles) = build_app(Rect::new(0, 0, 80, 24));
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

    // Collect every sidebar treeitem (branch or leaf) and its
    // painted y row.
    fn collect(dom: &TuiDom, id: NodeId, out: &mut Vec<NodeId>) {
        let n = dom.node(id);
        if n.tag_name() == Some("li") && n.get_attribute("role") == Some("treeitem") {
            out.push(id);
        }
        for c in n.child_nodes() {
            collect(dom, c.id(), out);
        }
    }
    let mut entries = Vec::new();
    let dom_ref = app.dom();
    collect(dom_ref, handles.sidebar, &mut entries);

    let viewport_h = viewport.height as i32;
    let mut visible_rows: Vec<(i32, String)> = Vec::new();
    for id in &entries {
        let ext = dom_ref.node(*id).ext();
        let layout = ext.map(|e| e.layout).unwrap_or_default();
        if layout.height == 0 {
            let label = dom_ref.text_content(*id);
            panic!(
                "treeitem {label:?} has zero painted height — flex-shrink squish is back. \
                 See M5-MIN-CONTENT-1.",
            );
        }
        if layout.y < 0 || layout.y >= viewport_h {
            continue;
        }
        visible_rows.push((layout.y, dom_ref.text_content(*id)));
    }

    let mut by_row: std::collections::BTreeMap<i32, Vec<String>> = Default::default();
    for (y, label) in &visible_rows {
        by_row.entry(*y).or_default().push(label.trim().to_string());
    }
    let collisions: Vec<_> = by_row.iter().filter(|(_, v)| v.len() > 1).collect();
    assert!(
        collisions.is_empty(),
        "multiple sidebar treeitems painted at the same y row: {collisions:#?}"
    );
}
