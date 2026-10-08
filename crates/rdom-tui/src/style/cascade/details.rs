//! `::details-content` (HTML §4.11.1, §15.5.20; CSS Pseudo-Elements 4):
//! the slot of a `<details>` element holding its content — every child
//! but its first `<summary>` element child. The slot is no box in rdom
//! (DIVERGENCES §2); it is the content's parent for inheritance, and it
//! hides the content while the element is closed (the UA's
//! `content-visibility: hidden`) or while the slot is `display: none`.

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
    let summary = p
        .child_nodes()
        .find(|c| c.node_type() == NodeType::Element && c.tag_name() == Some("summary"))
        .map(|c| c.id());
    summary != Some(child)
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

/// Whether the `<details>` `parent`'s slot hides its content: the element
/// is closed, or the slot is `display: none`.
pub(crate) fn hides(dom: &Dom<TuiExt>, parent: NodeId) -> bool {
    let node = dom.node(parent);
    let Some(slot) = node
        .ext()
        .and_then(|e| e.computed_details_content().map(|s| &**s))
    else {
        return false;
    };
    !node.has_attribute("open") || slot.display == crate::layout::Display::None
}

/// Whether `child` of `parent` is content the slot hides.
pub(crate) fn hidden(dom: &Dom<TuiExt>, parent: NodeId, child: NodeId) -> bool {
    slotted(dom, parent, child) && hides(dom, parent)
}
