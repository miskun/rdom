//! The scrolls into view that wait for an `App`'s next layout: asked for
//! when its layout could not be brought up to date (`LayoutFlush::Stale`
//! lists when), done
//! after the frame's layout, in the order they were asked for — a later
//! one wins on a container both move, as it would done at once.

use rdom_core::NodeId;

use crate::TuiDom;
use crate::node::TuiNodeExt;
use crate::runtime::smooth_scroll::ScrollIntoViewOptions;

/// Document data: an `App` lays this document out. A scroll asked for on
/// it waits for that layout when the layout cannot be made current; on
/// any other document — a bare one, or a second document scrolled from
/// inside an `App`'s handler — it is done at once, against the last
/// layout (C9G-MISC-CORRECTNESS).
struct LaidOutByApp;

/// Mark `dom` as one an `App` lays out (`App`'s construction).
pub(crate) fn laid_out_by_app(dom: &mut TuiDom) {
    dom.set_document_data(LaidOutByApp);
}

/// A scroll into view waiting for the next layout.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Scroll {
    /// HTML's focusing steps: done only if `id` still has the focus.
    Focus(NodeId),
    /// `Element.scrollIntoView(options)`.
    IntoView(NodeId, ScrollIntoViewOptions),
}

impl Scroll {
    fn target(self) -> NodeId {
        match self {
            Scroll::Focus(id) | Scroll::IntoView(id, _) => id,
        }
    }
}

/// The waiting scrolls, in the order asked for (document data).
struct Pending(Vec<Scroll>);

/// Scroll now, against a current layout — or, on a document an `App`
/// lays out whose layout cannot be made current, at its next layout.
pub(crate) fn scroll_now_or_at_layout(dom: &mut TuiDom, scroll: Scroll) {
    if dom.document_data::<LaidOutByApp>().is_some() && !super::flush_layout(dom).is_current() {
        match dom.document_data_mut::<Pending>() {
            Some(p) => p.0.push(scroll),
            None => {
                dom.set_document_data(Pending(vec![scroll]));
            }
        }
        return;
    }
    // The flush ran listeners (`derived_state`): one may have moved the
    // focus on, or removed the element.
    if still_wanted(dom, scroll) {
        run(dom, scroll);
    }
}

/// Whether `scroll` is still to be done: its element is in the document
/// and, for a focus's, still has the focus (HTML scrolls the element
/// focused).
fn still_wanted(dom: &TuiDom, scroll: Scroll) -> bool {
    let id = scroll.target();
    dom.contains(id) && (!matches!(scroll, Scroll::Focus(_)) || dom.focused() == Some(id))
}

/// After a layout: do the scrolls waiting for it. Whether a scroll
/// offset moved — the caller brings the layout up to the offsets.
pub(crate) fn service(dom: &mut TuiDom) -> bool {
    let Some(Pending(scrolls)) = dom.remove_document_data::<Pending>() else {
        return false;
    };
    let mut moved = false;
    for scroll in scrolls {
        if !still_wanted(dom, scroll) {
            continue;
        }
        let id = scroll.target();
        let before = offsets(dom, id);
        run(dom, scroll);
        moved |= offsets(dom, id) != before;
    }
    moved
}

fn run(dom: &mut TuiDom, scroll: Scroll) {
    use crate::runtime::smooth_scroll::ScrollLogicalPosition;
    let (id, options) = match scroll {
        // Browsers reveal a focused element `nearest` on both axes.
        Scroll::Focus(id) => (
            id,
            ScrollIntoViewOptions::new()
                .block(ScrollLogicalPosition::Nearest)
                .inline(ScrollLogicalPosition::Nearest),
        ),
        Scroll::IntoView(id, options) => (id, options),
    };
    crate::runtime::scrollbar::scroll_element_into_view(dom, id, options);
}

/// The scroll offsets of `id`'s ancestors.
fn offsets(dom: &TuiDom, id: NodeId) -> Vec<(i32, i32)> {
    std::iter::successors(dom.node(id).parent_node(), |n| n.parent_node())
        .filter_map(|n| n.tui_ext().map(|e| (e.scroll_x, e.scroll_y)))
        .collect()
}
