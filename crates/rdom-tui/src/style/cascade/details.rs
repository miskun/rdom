//! `::details-content` (HTML §4.11.1, §15.5.20; CSS Pseudo-Elements 4):
//! the slot of a `<details>` element holding its content — every child
//! but its first `<summary>` element child. It is the content's parent
//! for inheritance; it skips the content while the element is closed (the
//! UA's `content-visibility: hidden`, CSS Containment 2 §4: its box stays,
//! empty, and the content keeps its computed styles); and its box is a
//! node outside the document
//! that this module keeps in step with the style
//! ([`sync_content_box`]; the box tree's view of it is
//! `render::box_tree::slot`).

use std::rc::Rc;

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::style::ComputedStyle;

/// Whether `child` of `parent` is slotted into `parent`'s
/// `::details-content`: `parent` is a `<details>` and `child` is not its
/// first `<summary>` element child.
pub(crate) fn slotted(dom: &Dom<TuiExt>, parent: NodeId, child: NodeId) -> bool {
    let p = dom.node(parent);
    if p.node_type() != NodeType::Element || p.tag_name() != Some("details") {
        return false;
    }
    summary(dom, parent) != Some(child)
}

/// `id`'s parent in the box tree (`render::box_tree::slot::parent`): its
/// parent node, except that content slotted into a `::details-content`
/// box has that box, and the box has its `<details>`.
pub(crate) fn box_parent(dom: &Dom<TuiExt>, id: NodeId) -> Option<NodeId> {
    use crate::ext::ContentBoxLink;
    if let Some(ContentBoxLink::HostedBy(host)) = dom.node(id).ext().map(|e| e.content_box_link()) {
        return Some(host);
    }
    let p = dom.node(id).parent_node()?.id();
    match dom.node(p).ext().map(|e| e.content_box_link()) {
        Some(ContentBoxLink::Box(b)) if slotted(dom, p, id) => Some(b),
        _ => Some(p),
    }
}

/// The `<details>` `details`'s first `<summary>` element child: the one
/// its first slot takes (HTML §15.5.20).
pub(crate) fn summary(dom: &Dom<TuiExt>, details: NodeId) -> Option<NodeId> {
    dom.node(details)
        .child_nodes()
        .find(|c| c.node_type() == NodeType::Element && c.tag_name() == Some("summary"))
        .map(|c| c.id())
}

/// Keep the `<details>` `host`'s `::details-content` box in step with its
/// slot's style, after the cascade wrote it: created with the first style
/// (a node outside the document, linked both ways — `ContentBoxLink`),
/// given each new one (the same `Rc`), and dropped with the last. A box
/// whose `<details>` was dropped from the arena is reclaimed by
/// [`reclaim_content_boxes`].
pub(super) fn sync_content_box(dom: &mut Dom<TuiExt>, host: NodeId) {
    use crate::ext::ContentBoxLink;
    let Some(ext) = dom.node(host).ext() else {
        return;
    };
    let style = ext.computed_details_content().cloned();
    let existing = match ext.content_box_link() {
        ContentBoxLink::Box(b) if dom.contains(b) => Some(b),
        _ => None,
    };
    match (style, existing) {
        (None, None) => {}
        (None, Some(b)) => {
            unlink(dom, host);
            let _ = dom.drop_subtree(b);
        }
        (Some(style), existing) => {
            let b = existing.unwrap_or_else(|| {
                let b = dom.create_element("details-content");
                if let Some(e) = dom.node_mut(b).ext_mut() {
                    e.set_content_box_link(ContentBoxLink::HostedBy(host));
                }
                if let Some(e) = dom.node_mut(host).ext_mut() {
                    e.set_content_box_link(ContentBoxLink::Box(b));
                }
                note_content_box(dom, host, b);
                b
            });
            // The box's own transitions (it is diffed as an element) run
            // over the slot's style.
            let slot = crate::ext::StyleSlot::Host;
            if crate::style::doc_flags::is_calc_sized(&style) {
                crate::style::doc_flags::note_calc_size(dom);
            }
            if let Some(e) = dom.node_mut(b).ext_mut()
                && !e
                    .base_computed_for(slot)
                    .is_some_and(|c| Rc::ptr_eq(c, &style))
            {
                e.layout_dirty = true;
                let overlaid = e.overlay(slot, &style).map(Rc::new);
                e.set_cascaded(slot, style, overlaid);
            }
        }
    }
}

/// Give `host`'s `::details-content` box the bottom-up subtree flags
/// `host` just took (`TuiExt::tree_has_*`): the box holds the subtree
/// they describe, and walks that skip unflagged subtrees reach the
/// content through it.
pub(super) fn mirror_flags(dom: &mut Dom<TuiExt>, host: NodeId) {
    let Some(ext) = dom.node(host).ext() else {
        return;
    };
    let crate::ext::ContentBoxLink::Box(b) = ext.content_box_link() else {
        return;
    };
    let flags = (
        ext.tree_has_positioned_pseudo,
        ext.tree_has_collapse,
        ext.tree_has_relative_inline,
    );
    if let Some(e) = dom.node_mut(b).ext_mut() {
        (
            e.tree_has_positioned_pseudo,
            e.tree_has_collapse,
            e.tree_has_relative_inline,
        ) = flags;
    }
}

/// Forget `host`'s link to its box.
fn unlink(dom: &mut Dom<TuiExt>, host: NodeId) {
    if let Some(e) = dom.node_mut(host).ext_mut() {
        e.set_content_box_link(crate::ext::ContentBoxLink::None);
    }
}

/// The `::details-content` boxes of the document: `(details, box)` pairs
/// (document data).
#[derive(Debug, Default, Clone)]
struct ContentBoxes(Vec<(NodeId, NodeId)>);

fn note_content_box(dom: &mut Dom<TuiExt>, host: NodeId, b: NodeId) {
    let mut boxes = dom
        .document_data::<ContentBoxes>()
        .cloned()
        .unwrap_or_default();
    boxes.0.push((host, b));
    dom.set_document_data(boxes);
}

/// Drop the boxes whose `<details>` left the arena (`drop_subtree`, a
/// dropping removal) or no longer links them, so a dropped element's box
/// does not outlive it. Run by each cascade.
pub(super) fn reclaim_content_boxes(dom: &mut Dom<TuiExt>) {
    let Some(boxes) = dom.document_data::<ContentBoxes>() else {
        return;
    };
    let linked = |dom: &Dom<TuiExt>, (host, b): (NodeId, NodeId)| {
        dom.contains(host)
            && dom.node(host).ext().map(|e| e.content_box_link())
                == Some(crate::ext::ContentBoxLink::Box(b))
    };
    if boxes.0.iter().all(|&pair| linked(dom, pair)) {
        return;
    }
    let (keep, stale): (Vec<_>, Vec<_>) = boxes.0.iter().partition(|&&pair| linked(dom, pair));
    dom.set_document_data(ContentBoxes(keep));
    for (_, b) in stale {
        if dom.contains(b) {
            let _ = dom.drop_subtree(b);
        }
    }
}

/// The style `child` of `parent` inherits from when it is slotted: the
/// slot's.
pub(super) fn inherited_style(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    child: NodeId,
) -> Option<Rc<ComputedStyle>> {
    let slot = dom
        .node(parent)
        .ext()?
        .computed_details_content()
        .cloned()?;
    slotted(dom, parent, child).then_some(slot)
}
