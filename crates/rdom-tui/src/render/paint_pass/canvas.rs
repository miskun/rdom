//! The canvas background (CSS Backgrounds 3 §2.11.2): the root element's
//! background paints the whole canvas — every cell of the viewport,
//! beneath everything — not only its own box, whose height is its
//! content's (`layout_pass::icb`). The root element is the document
//! element: an element root, or a root fragment's first element child.
//! In an HTML document whose `html` element's background is transparent,
//! `body`'s propagates instead (§2.11.2, "for documents whose root
//! element is an HTML `HTML` element"). The element whose background
//! propagates paints none on its own box: its used background is
//! transparent.
//!
//! rdom's backgrounds are colours, so the propagated background is the
//! `background-color` (with its `background-clip` moot: the canvas has no
//! border or padding).

use rdom_core::{Dom, NodeId, NodeType};

use super::{background, fills};
use crate::ext::TuiExt;
use crate::layout::Display;
use crate::node::TuiNodeExt;
use crate::render::{Buffer, Rect};

/// The element whose background is the canvas's: the root element
/// ([`root_element`]) when its background is not transparent, else its
/// first `body` child when the root element is `html`, if either paints
/// a background.
pub(super) fn source(dom: &Dom<TuiExt>) -> Option<NodeId> {
    let root = dom.node(root_element(dom)?);
    if paints(dom, root.id()) {
        return Some(root.id());
    }
    if !root
        .tag_name()
        .is_some_and(|t| t.eq_ignore_ascii_case("html"))
    {
        return None;
    }
    let body = root.child_nodes().find(|c| {
        c.node_type() == NodeType::Element
            && c.tag_name().is_some_and(|t| t.eq_ignore_ascii_case("body"))
    })?;
    (has_box(dom, body.id()) && paints(dom, body.id())).then(|| body.id())
}

/// The document's root element, if it generates a box: the document
/// element (`Dom::document_element` — an element root, or a root
/// fragment's first element child; DIVERGENCES).
fn root_element(dom: &Dom<TuiExt>) -> Option<NodeId> {
    let root = dom.document_element();
    (root.node_type() == NodeType::Element && has_box(dom, root.id())).then(|| root.id())
}

/// Paint the canvas background over `clip`, the viewport.
pub(super) fn paint(dom: &Dom<TuiExt>, buf: &mut Buffer, clip: Rect) {
    let Some(bg) = source(dom).and_then(|id| dom.node(id).computed().map(|c| c.bg)) else {
        return;
    };
    if bg.is_translucent() {
        let alpha = f32::from(bg.alpha()) / 255.0;
        let opaque = bg.opaque();
        buf.paint_translucent(clip, alpha, |layer| {
            background::fill_bg(layer, clip, opaque)
        });
    } else {
        background::fill_bg(buf, clip, bg);
    }
}

/// Whether `id` generates a box (CSS Display 3 §2.5: `none` and
/// `contents` do not).
fn has_box(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    dom.node(id)
        .computed()
        .is_some_and(|c| !matches!(c.display, Display::None | Display::Contents))
}

/// Whether `id`'s background color paints (`fills`).
fn paints(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    dom.node(id).computed().is_some_and(|c| fills(c.bg))
}
