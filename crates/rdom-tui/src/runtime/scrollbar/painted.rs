//! Scroll offsets as painted (`P7-SCROLL-REPAINT-1`).
//!
//! A scroll offset change repaints on the web whoever made it (CSSOM
//! View §4.1 "perform a scroll" updates the rendering; HTML §8.1.7.3
//! "update the rendering" paints it). Scroll offsets are not DOM
//! mutations, so the dirty tracker never sees them: the
//! [`App`](crate::runtime::App) compares each element's offsets with
//! the ones its last frame painted instead. That covers every writer
//! alike — the programmatic scroll API from a listener, a timer or an
//! `AppHandle::inject` closure, a smooth-scroll step, and a consumer
//! writing `TuiExt::scroll_x` / `scroll_y` directly — and leaves an
//! offset written back to its painted value with nothing to draw.
//!
//! Only elements in the tree count: a detached subtree is not painted,
//! and re-inserting it is a tree mutation that repaints anyway.

use rdom_core::NodeId;

use crate::TuiDom;
use crate::node::TuiNodeExt;

/// Whether some connected element's scroll offset differs from the one
/// the last painted frame recorded ([`note_painted`]).
pub(crate) fn moved_since_paint(dom: &TuiDom) -> bool {
    let mut stack = vec![dom.root()];
    while let Some(id) = stack.pop() {
        let node = dom.node(id);
        if node
            .tui_ext()
            .is_some_and(|e| (e.scroll_x, e.scroll_y) != e.painted_scroll)
        {
            return true;
        }
        stack.extend(node.child_nodes().map(|c| c.id()));
    }
    false
}

/// Record every connected element's scroll offsets as painted. The App
/// calls this once a frame is drawn — after layout's own clamp and the
/// caret reveal, so the recorded offsets are the ones on screen.
pub(crate) fn note_painted(dom: &mut TuiDom) {
    let mut stack: Vec<NodeId> = vec![dom.root()];
    while let Some(id) = stack.pop() {
        stack.extend(dom.node(id).child_nodes().map(|c| c.id()));
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.painted_scroll = (ext.scroll_x, ext.scroll_y);
        }
    }
}
