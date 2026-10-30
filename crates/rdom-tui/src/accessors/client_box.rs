//! The client box `clientTop` / `clientLeft` / `clientWidth` /
//! `clientHeight` read (CSSOM View §6), for
//! [`TuiAccessors`](super::TuiAccessors): the scrollport — the padding box
//! less the scrollbar gutters layout reserved — and its offset inside the
//! border box.

use rdom_core::NodeRef;

use crate::CascadeExt;
use crate::TuiExt;
use crate::node::TuiNodeExt;

/// An element's client box, in cells.
pub(super) struct ClientBox {
    pub(super) top: i32,
    pub(super) left: i32,
    pub(super) width: i32,
    pub(super) height: i32,
}

impl ClientBox {
    const ZERO: Self = Self {
        top: 0,
        left: 0,
        width: 0,
        height: 0,
    };
}

/// `node`'s client box from the last layout; `None` for a non-element
/// other than the root Fragment, which stands for the root element.
///
/// §6: the root's width and height are the viewport's — a terminal's has
/// no scrollbar of its own — and an element with no box or an inline box
/// answers 0 on all four.
pub(super) fn client_box(node: &NodeRef<'_, TuiExt>) -> Option<ClientBox> {
    let dom = node.dom();
    if node.id() == dom.root() {
        let viewport = dom.viewport();
        return Some(ClientBox {
            width: i32::from(viewport.cols),
            height: i32::from(viewport.rows),
            ..ClientBox::ZERO
        });
    }
    let ext = node.ext()?;
    let inline = node
        .computed()
        .is_some_and(|c| c.display == crate::layout::Display::Inline && !c.is_atomic_inline());
    if inline || !super::helpers::has_box(node) {
        return Some(ClientBox::ZERO);
    }
    let border_box = ext.border_box();
    let port = crate::render::layout_pass::scrollport::scrollport(dom, node.id())?;
    Some(ClientBox {
        top: port.y - border_box.y,
        left: port.x - border_box.x,
        width: i32::from(port.width),
        height: i32::from(port.height),
    })
}
