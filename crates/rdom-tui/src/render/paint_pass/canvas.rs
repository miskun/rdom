//! The canvas background (CSS Backgrounds 3 §2.11.2): the root element's
//! background paints the whole canvas — every cell of the viewport,
//! beneath everything — not only its own box. The root element is the
//! tree's root: the root fragment (C14G-ROOT-ELEMENT, styled by
//! `style::cascade::root`, `:root { background: … }`), or an element root
//! (`Dom::with_root_tag`). When its background is transparent, an HTML
//! document's propagates (§2.11.2, "for documents whose root element is an
//! HTML `HTML` element"): for an `<html>` element root, its first `body`
//! child's; under the root fragment, which holds the document's element
//! tree, its first `html` element child's — or that `<html>`'s `body`'s —
//! else its first `body` element child's. The element whose background
//! propagates paints none on its own box: its used background is
//! transparent. Any other top-level element paints its own box only.
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

/// The node whose background is the canvas's (module doc): the root when
/// its background paints, else the propagating `<html>` or `<body>`
/// element, if its background paints.
pub(super) fn source(dom: &Dom<TuiExt>) -> Option<NodeId> {
    let root = dom.root();
    match dom.node(root).node_type() {
        NodeType::Fragment => {
            if paints(dom, root) {
                return Some(root);
            }
            if let Some(html) = child_named(dom, root, "html") {
                if !has_box(dom, html) {
                    return None;
                }
                if paints(dom, html) {
                    return Some(html);
                }
                return body_of(dom, html);
            }
            body_of(dom, root)
        }
        NodeType::Element => {
            if !has_box(dom, root) {
                return None;
            }
            if paints(dom, root) {
                return Some(root);
            }
            if !is_named(dom, root, "html") {
                return None;
            }
            body_of(dom, root)
        }
        _ => None,
    }
}

/// Whether the element `id` is the canvas's source, so paints no
/// background of its own: only the root, an `<html>` or a `<body>` can
/// be — any other element answers without looking further.
pub(super) fn takes_background(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    (id == dom.root() || is_named(dom, id, "html") || is_named(dom, id, "body"))
        && source(dom) == Some(id)
}

/// `parent`'s first `body` element child, when it has a box and its
/// background paints.
fn body_of(dom: &Dom<TuiExt>, parent: NodeId) -> Option<NodeId> {
    let body = child_named(dom, parent, "body")?;
    (has_box(dom, body) && paints(dom, body)).then_some(body)
}

/// `parent`'s first element child named `tag` (ASCII case-insensitively).
fn child_named(dom: &Dom<TuiExt>, parent: NodeId, tag: &str) -> Option<NodeId> {
    dom.node(parent)
        .child_nodes()
        .find(|c| c.node_type() == NodeType::Element && is_named(dom, c.id(), tag))
        .map(|c| c.id())
}

fn is_named(dom: &Dom<TuiExt>, id: NodeId, tag: &str) -> bool {
    dom.node(id)
        .tag_name()
        .is_some_and(|t| t.eq_ignore_ascii_case(tag))
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
