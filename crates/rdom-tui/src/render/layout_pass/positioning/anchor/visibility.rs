//! `position-visibility` (CSS Anchor Positioning 1 §5): an
//! anchor-positioned box hidden — strongly, its subtree with it, as
//! `visibility: hidden` that no descendant can undo — when its default
//! anchor is clipped out of view (`anchors-visible`, the initial value),
//! when an anchor it references is missing (`anchors-valid`), or when it
//! overflows whatever it tries (`no-overflow`). Layout records the boxes
//! it hides each placement pass (document data); paint, hit-testing and
//! focus ask [`hidden`] through `render::visibility::shows`.

use rdom_core::{Dom, NodeId};

use crate::ext::{PseudoSlot, TuiExt};
use crate::layout::LayoutRect;
use crate::node::TuiNodeExt;

/// The boxes the last placement pass hid: an element (`None`) with its
/// subtree, or a host's positioned pseudo-element.
#[derive(Debug, Default)]
struct AnchorHidden(Vec<(NodeId, Option<PseudoSlot>)>);

/// Forget the last pass's hidden boxes.
pub(in crate::render::layout_pass) fn begin(dom: &mut Dom<TuiExt>) {
    if let Some(h) = dom.document_data_mut::<AnchorHidden>() {
        h.0.clear();
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
        h.0.push((id, slot));
    }
}

/// Whether `position-visibility` hides `id`'s `slot` box: `id` or an
/// ancestor is hidden, or that pseudo-element is. Free with nothing
/// hidden.
pub(crate) fn hidden(dom: &Dom<TuiExt>, id: NodeId, slot: Option<PseudoSlot>) -> bool {
    let Some(h) = dom
        .document_data::<AnchorHidden>()
        .filter(|h| !h.0.is_empty())
    else {
        return false;
    };
    if slot.is_some() && h.0.contains(&(id, slot)) {
        return true;
    }
    let mut cur = Some(id);
    while let Some(n) = cur {
        if h.0.contains(&(n, None)) {
            return true;
        }
        cur = crate::render::box_tree::slot::parent(dom, n);
    }
    false
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
