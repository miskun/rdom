//! ARIA tree behavior tests — keyboard navigation, expand/collapse,
//! active-descendant cursor, pointer, and `preventDefault` override.

use crossterm::event::{
    Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton,
    MouseEvent as CtMouseEvent, MouseEventKind,
};
use rdom_core::{ListenerOptions, NodeId};
use std::cell::Cell;
use std::rc::Rc;

use crate::TuiDom;
use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::style::Stylesheet;

const ACTIVE: &str = "data-rdom-active";

fn test_app(dom: TuiDom) -> App<TestBackend> {
    let backend = TestBackend::new(40, 12);
    let terminal = Terminal::new(backend).unwrap();
    // Real UA stylesheet so layout + roles behave as shipped.
    App::with_backend(dom, Stylesheet::new(), terminal).unwrap()
}

fn ti(dom: &mut TuiDom, label: &str, attrs: &[(&str, &str)]) -> NodeId {
    let li = dom.create_element("li");
    dom.set_attribute(li, "role", "treeitem").unwrap();
    for (k, v) in attrs {
        dom.set_attribute(li, k, v).unwrap();
    }
    let t = dom.create_text_node(label);
    dom.append_child(li, t).unwrap();
    li
}

fn group(dom: &mut TuiDom, parent: NodeId) -> NodeId {
    let g = dom.create_element("ul");
    dom.set_attribute(g, "role", "group").unwrap();
    dom.append_child(parent, g).unwrap();
    g
}

struct Fix {
    app: App<TestBackend>,
    tree: NodeId,
    cluster: NodeId,
    pods: NodeId,
    nodes: NodeId,
    node1: NodeId,
    node2: NodeId,
}

/// ```text
/// <ul role=tree>
///   <li treeitem aria-expanded=true> Cluster
///     <ul group>
///       <li treeitem> Pods
///       <li treeitem aria-expanded=true> Nodes
///         <ul group>
///           <li treeitem> node-1
///           <li treeitem> node-2
/// ```
fn fixture() -> Fix {
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let tree = dom.create_element("ul");
    dom.set_attribute(tree, "role", "tree").unwrap();
    dom.append_child(root, tree).unwrap();

    let cluster = ti(&mut dom, "Cluster", &[("aria-expanded", "true")]);
    dom.append_child(tree, cluster).unwrap();
    let cg = group(&mut dom, cluster);
    let pods = ti(&mut dom, "Pods", &[]);
    dom.append_child(cg, pods).unwrap();
    let nodes = ti(&mut dom, "Nodes", &[("aria-expanded", "true")]);
    dom.append_child(cg, nodes).unwrap();
    let ng = group(&mut dom, nodes);
    let node1 = ti(&mut dom, "node-1", &[]);
    let node2 = ti(&mut dom, "node-2", &[]);
    dom.append_child(ng, node1).unwrap();
    dom.append_child(ng, node2).unwrap();

    let mut app = test_app(dom);
    app.dom_mut().set_focused(Some(tree));
    app.draw_if_dirty().unwrap();
    Fix {
        app,
        tree,
        cluster,
        pods,
        nodes,
        node1,
        node2,
    }
}

fn key(code: KeyCode) -> CtEvent {
    CtEvent::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: KeyEventState::empty(),
    })
}

fn active(app: &App<TestBackend>, tree: NodeId) -> Option<NodeId> {
    fn walk(app: &App<TestBackend>, id: NodeId) -> Option<NodeId> {
        for child in app.dom().node(id).child_nodes() {
            if child.has_attribute(ACTIVE) {
                return Some(child.id());
            }
            if let Some(f) = walk(app, child.id()) {
                return Some(f);
            }
        }
        None
    }
    walk(app, tree)
}

// ── Cursor navigation ───────────────────────────────────────────

#[test]
fn arrow_down_seeds_then_advances_cursor() {
    let mut f = fixture();
    f.app.handle_event(key(KeyCode::Down));
    assert_eq!(
        active(&f.app, f.tree),
        Some(f.cluster),
        "first Down → first item"
    );
    f.app.handle_event(key(KeyCode::Down));
    assert_eq!(
        active(&f.app, f.tree),
        Some(f.pods),
        "second Down → next visible"
    );
}

#[test]
fn arrow_up_clamps_at_top() {
    let mut f = fixture();
    f.app.handle_event(key(KeyCode::Down)); // cluster
    f.app.handle_event(key(KeyCode::Up)); // clamp
    assert_eq!(active(&f.app, f.tree), Some(f.cluster));
}

#[test]
fn home_and_end_jump() {
    let mut f = fixture();
    f.app.handle_event(key(KeyCode::End));
    assert_eq!(active(&f.app, f.tree), Some(f.node2), "End → last visible");
    f.app.handle_event(key(KeyCode::Home));
    assert_eq!(active(&f.app, f.tree), Some(f.cluster), "Home → first");
}

#[test]
fn down_skips_collapsed_subtree() {
    let mut f = fixture();
    // Collapse Cluster — Pods/Nodes/node-* drop out of the visible list.
    f.app
        .dom_mut()
        .set_attribute(f.cluster, "aria-expanded", "false")
        .unwrap();
    f.app.handle_event(key(KeyCode::Down)); // cluster
    f.app.handle_event(key(KeyCode::Down)); // clamp — nothing below
    assert_eq!(active(&f.app, f.tree), Some(f.cluster));
}

// ── Expand / collapse ───────────────────────────────────────────

#[test]
fn right_expands_collapsed_branch_and_fires_toggle() {
    let mut f = fixture();
    f.app
        .dom_mut()
        .set_attribute(f.cluster, "aria-expanded", "false")
        .unwrap();
    let fires = Rc::new(Cell::new(0u32));
    let fc = fires.clone();
    f.app
        .dom_mut()
        .add_event_listener(f.cluster, "toggle", ListenerOptions::default(), move |_| {
            fc.set(fc.get() + 1)
        })
        .unwrap();
    f.app.handle_event(key(KeyCode::Down)); // cursor → cluster
    f.app.handle_event(key(KeyCode::Right)); // expand
    assert_eq!(
        f.app.dom().node(f.cluster).get_attribute("aria-expanded"),
        Some("true")
    );
    assert_eq!(fires.get(), 1, "expand fires toggle");
}

#[test]
fn right_on_expanded_branch_descends_to_first_child() {
    let mut f = fixture();
    f.app.handle_event(key(KeyCode::Down)); // cluster (expanded)
    f.app.handle_event(key(KeyCode::Right)); // descend
    assert_eq!(active(&f.app, f.tree), Some(f.pods));
}

#[test]
fn left_collapses_then_ascends_to_parent() {
    let mut f = fixture();
    // Cursor onto Nodes (expanded branch, depth 1).
    f.app.handle_event(key(KeyCode::End)); // node-2
    f.app.handle_event(key(KeyCode::Up)); // node-1
    assert_eq!(active(&f.app, f.tree), Some(f.node1));
    f.app.handle_event(key(KeyCode::Up)); // nodes
    assert_eq!(active(&f.app, f.tree), Some(f.nodes));
    f.app.handle_event(key(KeyCode::Left)); // collapse Nodes
    assert_eq!(
        f.app.dom().node(f.nodes).get_attribute("aria-expanded"),
        Some("false")
    );
    f.app.handle_event(key(KeyCode::Left)); // ascend to Cluster
    assert_eq!(active(&f.app, f.tree), Some(f.cluster));
}

#[test]
fn right_on_unloaded_branch_fires_toggle_for_lazy_load() {
    // A branch declared `aria-expanded=false` with NO children yet —
    // Right must still fire `toggle` (the app's load hook), not no-op.
    let mut dom: TuiDom = TuiDom::new();
    let root = dom.root();
    let tree = dom.create_element("ul");
    dom.set_attribute(tree, "role", "tree").unwrap();
    dom.append_child(root, tree).unwrap();
    let lazy = ti(&mut dom, "Lazy", &[("aria-expanded", "false")]);
    dom.append_child(tree, lazy).unwrap();

    let mut app = test_app(dom);
    app.dom_mut().set_focused(Some(tree));
    app.draw_if_dirty().unwrap();
    let fires = Rc::new(Cell::new(0u32));
    let fc = fires.clone();
    app.dom_mut()
        .add_event_listener(lazy, "toggle", ListenerOptions::default(), move |_| {
            fc.set(fc.get() + 1)
        })
        .unwrap();
    app.handle_event(key(KeyCode::Down));
    app.handle_event(key(KeyCode::Right));
    assert_eq!(
        fires.get(),
        1,
        "expanding an unloaded branch fires the load hook"
    );
}

// ── Activation ──────────────────────────────────────────────────

#[test]
fn enter_activates_and_fires_click() {
    let mut f = fixture();
    f.app.handle_event(key(KeyCode::Down)); // cluster
    f.app.handle_event(key(KeyCode::Down)); // pods (leaf)
    let fires = Rc::new(Cell::new(0u32));
    let fc = fires.clone();
    f.app
        .dom_mut()
        .add_event_listener(f.pods, "click", ListenerOptions::default(), move |_| {
            fc.set(fc.get() + 1)
        })
        .unwrap();
    f.app.handle_event(key(KeyCode::Enter));
    assert_eq!(fires.get(), 1, "Enter fires click on the active item");
}

// ── Pointer ─────────────────────────────────────────────────────

fn click_at(app: &mut App<TestBackend>, x: u16, y: u16) {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        app.handle_event(CtEvent::Mouse(CtMouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        }));
    }
}

#[test]
fn click_on_label_sets_cursor_without_toggling() {
    let mut f = fixture();
    // Pods is on row 1 (Cluster row 0). Click its label (well right of
    // the gutter) — sets the cursor, does NOT toggle anything.
    click_at(&mut f.app, 8, 1);
    assert_eq!(active(&f.app, f.tree), Some(f.pods));
    // Cluster (a branch) stays expanded — label clicks don't toggle.
    assert_eq!(
        f.app.dom().node(f.cluster).get_attribute("aria-expanded"),
        Some("true")
    );
}

#[test]
fn click_in_twisty_gutter_toggles_branch() {
    let mut f = fixture();
    // Cluster's chevron sits at the far left of row 0.
    click_at(&mut f.app, 0, 0);
    assert_eq!(
        f.app.dom().node(f.cluster).get_attribute("aria-expanded"),
        Some("false"),
        "gutter click collapses the branch"
    );
}

// ── preventDefault override ─────────────────────────────────────

#[test]
fn prevent_default_suppresses_builtin_navigation() {
    let mut f = fixture();
    // App listener on the tree consumes the keydown before the
    // root-level builtin default action runs.
    f.app
        .dom_mut()
        .add_event_listener(f.tree, "keydown", ListenerOptions::default(), move |ctx| {
            ctx.event.prevent_default();
        })
        .unwrap();
    f.app.handle_event(key(KeyCode::Down));
    assert_eq!(
        active(&f.app, f.tree),
        None,
        "no cursor moved — default prevented"
    );
}
