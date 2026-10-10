//! C16G-DEPTH-CAPS (Phase 16 gate decision 2): the box tree is at most
//! [`MAX_LAYOUT_DEPTH`] deep. An element that deep skips its contents —
//! no box below it is styled, laid out, painted or hit — as an engine
//! stops laying out past a fixed depth; so every recursive pass (cascade,
//! layout, paint, hit testing) is bounded by the cap, whatever the DOM.
//! The tests build a 100 000-deep DOM through the API (the parser caps
//! its own trees at 512) and run an `App` over it on a thread whose stack
//! is [`STACK`].

use rdom_core::NodeId;

use crate::render::{Terminal, TestBackend};
use crate::runtime::app::App;
use crate::{MAX_LAYOUT_DEPTH, TuiDom, TuiNodeExt};

/// Far past the cap.
const DEEP: usize = 100_000;

/// The stack the deep tests run on: a test thread's default 2 MiB, which
/// a debug build's layout down to the cap fits (C16G-DEPTH-CAPS measured
/// 10–14 KB a level for nested grids, the deepest frames).
const STACK: usize = 2 * 1024 * 1024;

fn on_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(f)
        .expect("spawn")
        .join()
        .expect("no stack overflow")
}

/// A chain of `n` `<div>`s under the root, the text `x` in the last:
/// `chain[i]` is `i + 1` deep. Built from the bottom, so each insertion's
/// ancestor check (DOM §4.2.3) is O(1).
fn chain(n: usize) -> (TuiDom, Vec<NodeId>) {
    let mut dom = TuiDom::new();
    let mut out: Vec<NodeId> = (0..n).map(|_| dom.create_element("div")).collect();
    let text = dom.create_text_node("x");
    let mut below = text;
    for &div in out.iter().rev() {
        dom.append_child(div, below).unwrap();
        below = div;
    }
    let root = dom.root();
    dom.append_child(root, below).unwrap();
    out.shrink_to_fit();
    (dom, out)
}

/// Row 0 of what `app` drew.
fn first_row(app: &App<TestBackend>) -> String {
    let mut screen = crate::render::VirtualScreen::new(20, 3);
    screen.apply(app.terminal().backend().bytes());
    (0..20)
        .map(|x| screen.cell(x, 0).expect("on screen").symbol().to_string())
        .collect()
}

/// The element at the cap is styled and laid out; the one below it is
/// neither, and the text at the bottom is not painted. A restyle deep
/// past the cap, and a hit test, stay bounded too.
#[test]
fn a_tree_past_the_layout_cap_is_cut_at_the_cap() {
    let (at_cap, below, row, hit) = on_stack(|| {
        let (dom, chain) = chain(DEEP);
        let terminal = Terminal::new(TestBackend::new(20, 3)).unwrap();
        let mut app = App::with_backend(dom, rdom_css::from_css(""), terminal).unwrap();
        app.advance(0).unwrap();
        // A mutation far below the cap restyles nothing there.
        app.dom_mut()
            .set_attribute(chain[DEEP - 1], "class", "x")
            .unwrap();
        app.advance(16).unwrap();
        // The tab order walks the whole DOM, iteratively.
        crate::runtime::focus::tabindex::focus_next(app.dom_mut());
        let dom = app.dom();
        let at_cap = chain[MAX_LAYOUT_DEPTH - 1];
        let below = chain[MAX_LAYOUT_DEPTH];
        let at = (
            dom.node(at_cap).computed().is_some(),
            dom.node(at_cap).layout_rect().is_some_and(|r| r.width > 0),
        );
        let under = (
            dom.node(below).computed().is_some(),
            dom.node(below).layout_rect().is_some_and(|r| r.width > 0),
        );
        let row = first_row(&app);
        let hit = crate::HitTestExt::hit_test(dom, 19, 0);
        (at, under, row, hit)
    });
    assert_eq!(at_cap, (true, true), "the element at the cap has its box");
    assert_eq!(
        below,
        (false, false),
        "nothing below it is styled or laid out"
    );
    assert!(
        !row.contains('x'),
        "the text past the cap is not painted: {row:?}"
    );
    assert!(hit.is_some());
}

/// A tree to the cap and no deeper renders whole: the cap cuts nothing a
/// real page holds. Nested grids, the layout with the largest frames,
/// fit a debug build's 2 MiB test thread at the cap.
#[test]
fn a_tree_to_the_cap_renders_whole() {
    let row = on_stack(|| {
        let (dom, _) = chain(MAX_LAYOUT_DEPTH - 1);
        let terminal = Terminal::new(TestBackend::new(20, 3)).unwrap();
        let sheet = rdom_css::from_css("div { display: grid }");
        let mut app = App::with_backend(dom, sheet, terminal).unwrap();
        app.advance(0).unwrap();
        first_row(&app)
    });
    assert!(row.starts_with('x'), "{row:?}");
}

/// A styled subtree moved below the cap forgets its style (it is in the
/// capped element's skipped contents), and takes one again when it moves
/// back.
#[test]
fn a_subtree_moved_past_the_cap_is_unstyled_and_back_restyled() {
    let (dom, chain) = chain(MAX_LAYOUT_DEPTH + 4);
    let terminal = Terminal::new(TestBackend::new(20, 3)).unwrap();
    let mut app = App::with_backend(dom, rdom_css::from_css(""), terminal).unwrap();
    let root = app.dom().root();
    let moved = app.dom_mut().create_element("p");
    let inner = app.dom_mut().create_element("b");
    app.dom_mut().append_child(moved, inner).unwrap();
    app.dom_mut().append_child(root, moved).unwrap();
    app.advance(0).unwrap();
    let styled = |app: &App<TestBackend>, id| app.dom().node(id).computed().is_some();
    assert!(styled(&app, moved) && styled(&app, inner));
    app.dom_mut()
        .append_child(chain[MAX_LAYOUT_DEPTH + 1], moved)
        .unwrap();
    app.advance(16).unwrap();
    assert!(!styled(&app, moved) && !styled(&app, inner), "past the cap");
    app.dom_mut().append_child(root, moved).unwrap();
    app.advance(16).unwrap();
    assert!(styled(&app, moved) && styled(&app, inner), "back above it");
}
