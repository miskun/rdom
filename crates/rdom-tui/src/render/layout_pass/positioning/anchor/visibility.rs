//! `position-visibility` (CSS Anchor Positioning 1 §5): an
//! anchor-positioned box hidden — strongly, its subtree with it, as
//! `visibility: hidden` that no descendant can undo — when its default
//! anchor is clipped out of view (`anchors-visible`, the initial value),
//! when an anchor it references is missing (`anchors-valid`), or when it
//! overflows whatever it tries (`no-overflow`). Layout records the boxes
//! it hides each placement pass (document data); paint, hit-testing and
//! focus ask [`hidden`] through `render::visibility::shows`.

use std::collections::HashSet;

use rdom_core::{Dom, NodeId};

use crate::ext::{PseudoSlot, TuiExt};
use crate::layout::LayoutRect;
use crate::node::TuiNodeExt;

#[cfg(test)]
thread_local! {
    /// Hidden-box comparisons `hidden` made (cost tests).
    pub(in crate::render::layout_pass) static HIDDEN_WORK: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

/// The boxes the last placement pass hid: an element (`None`) with its
/// subtree, or a host's positioned pseudo-element — and, built on the first
/// query after the pass, every node they hide, so a query is one lookup,
/// not an ancestor walk against each hidden box (C15G-ANCHOR-COST).
#[derive(Debug, Default)]
struct AnchorHidden {
    roots: Vec<(NodeId, Option<PseudoSlot>)>,
    expanded: std::cell::OnceCell<Expanded>,
}

/// The nodes a hidden element hides (it and its box-tree descendants), and
/// the hidden pseudo-elements.
#[derive(Debug, Default)]
struct Expanded {
    nodes: HashSet<NodeId>,
    pseudos: HashSet<(NodeId, PseudoSlot)>,
}

/// Forget the last pass's hidden boxes.
pub(in crate::render::layout_pass) fn begin(dom: &mut Dom<TuiExt>) {
    if let Some(h) = dom.document_data_mut::<AnchorHidden>() {
        h.roots.clear();
        h.expanded.take();
    }
}

/// Hide `id` (or its `slot` pseudo-element) until the next pass.
pub(in crate::render::layout_pass) fn hide(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    slot: Option<PseudoSlot>,
) {
    if dom.document_data::<AnchorHidden>().is_none() {
        dom.set_document_data(AnchorHidden::default());
    }
    if let Some(h) = dom.document_data_mut::<AnchorHidden>() {
        h.roots.push((id, slot));
        h.expanded.take();
    }
}

/// Whether `position-visibility` hides `id`'s `slot` box: `id` or an
/// ancestor is hidden, or that pseudo-element is. Free with nothing
/// hidden; one lookup otherwise.
pub(crate) fn hidden(dom: &Dom<TuiExt>, id: NodeId, slot: Option<PseudoSlot>) -> bool {
    let Some(h) = dom
        .document_data::<AnchorHidden>()
        .filter(|h| !h.roots.is_empty())
    else {
        return false;
    };
    let e = h.expanded.get_or_init(|| expand(dom, &h.roots));
    #[cfg(test)]
    HIDDEN_WORK.with(|c| c.set(c.get() + 1));
    e.nodes.contains(&id) || slot.is_some_and(|s| e.pseudos.contains(&(id, s)))
}

/// Every node `roots` hide: `O(hidden subtrees)`, once per pass.
fn expand(dom: &Dom<TuiExt>, roots: &[(NodeId, Option<PseudoSlot>)]) -> Expanded {
    let mut out = Expanded::default();
    let mut stack: Vec<NodeId> = Vec::new();
    for &(id, slot) in roots {
        match slot {
            Some(s) => {
                out.pseudos.insert((id, s));
            }
            None => stack.push(id),
        }
    }
    // The DOM subtree, and a `<details>`'s `::details-content` box (whose
    // children are the details' own: `slot::parent`'s view).
    for root in stack {
        for n in std::iter::once(root).chain(dom.descendants(root)) {
            #[cfg(test)]
            HIDDEN_WORK.with(|c| c.set(c.get() + 1));
            out.nodes.insert(n);
            if let Some(crate::ext::ContentBoxLink::Box(b)) =
                dom.node(n).ext().map(|e| e.content_box_link())
            {
                out.nodes.insert(b);
            }
        }
    }
    out
}

/// Whether the anchor `anchor`, its box `rect`, is clipped out of view by
/// a box between it and the root that clips its overflow (§5
/// `anchors-visible`: "clipped by intervening boxes").
pub(in crate::render::layout_pass) fn clipped_out(
    dom: &Dom<TuiExt>,
    anchor: NodeId,
    rect: LayoutRect,
) -> bool {
    if rect.width == 0 || rect.height == 0 {
        return true;
    }
    let mut cur = crate::render::box_tree::box_parent(dom, anchor);
    while let Some(p) = cur {
        if let Some(ext) = dom.node(p).tui_ext()
            && let Some(c) = ext.computed.as_deref()
            && c.clips_overflow()
        {
            let port = crate::render::layout_pass::scrollport_of(ext, c);
            let apart = rect.x >= port.x + i32::from(port.width)
                || port.x >= rect.x + i32::from(rect.width)
                || rect.y >= port.y + i32::from(port.height)
                || port.y >= rect.y + i32::from(rect.height);
            if apart {
                return true;
            }
        }
        cur = crate::render::box_tree::box_parent(dom, p);
    }
    false
}
