//! Phase-2 placement of `position: absolute | fixed` boxes — elements
//! and `::before` / `::after` alike (CSS Pseudo 4 §2): the containing
//! block, the placed rect (`rect`, CSS 2.1 §10.3.7 / §10.6.4), then the
//! box's content laid out inside it.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{PseudoSlot, TuiExt};
use crate::layout::{LayoutRect, Position};
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;
use crate::style::ComputedStyle;

use super::rect::Placed;
use super::*;

/// After phase-1 flex layout completes, walk the tree in document
/// order and place every `position: absolute | fixed` box against its
/// containing block: an element, its subtree then laid out inside the
/// placed rect by `layout_node`; a `::before` / `::after`, its content
/// laid out inside it (`pseudo::place`).
///
/// Returns the boxes it placed, in document order (a host's `::before`
/// right after the host, its `::after` after the host's descendants).
///
/// Document-order walk guarantees that an outer positioned element
/// is placed before any positioned descendants — so when a nested
/// absolute resolves its containing block, the outer's
/// `TuiExt.layout` is already populated.
pub(in crate::render::layout_pass) fn place_positioned(
    dom: &mut Dom<TuiExt>,
    viewport: LayoutRect,
) -> Vec<BoxItem> {
    let positioned = collect_positioned(dom);
    // The anchor names, found on the first anchor-positioned box; the boxes
    // `position-visibility` hides, found again.
    let anchors = super::anchor::AnchorIndex::default();
    super::anchor::visibility::begin(dom);
    crate::render::layout_pass::scroll_update::begin_placement(dom);
    for &item in &positioned {
        match item {
            BoxItem::Node(id) => {
                let cb = containing_block(dom, id, viewport);
                let computed = dom
                    .node(id)
                    .computed_rc()
                    .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
                let querying = super::anchor::Querying {
                    node: id,
                    pseudo: false,
                };
                let placed = super::anchor::placed_rect(
                    dom,
                    &anchors,
                    Placed::Element(id),
                    querying,
                    &computed,
                    cb,
                );
                if placed.hidden {
                    super::anchor::visibility::hide(dom, id, None);
                }
                crate::render::layout_pass::layout_node(dom, id, placed.rect, cb.width);
                let at = dom
                    .node(id)
                    .ext()
                    .map_or((0, 0), |e| (e.layout.x, e.layout.y));
                crate::render::layout_pass::scroll_update::note_placed(
                    dom,
                    crate::render::layout_pass::scroll_update::PlacedRecord {
                        item,
                        rect: placed.rect,
                        at,
                    },
                );
            }
            BoxItem::Generated(host, slot) => {
                super::pseudo::place(dom, &anchors, host, slot, viewport)
            }
        }
    }
    positioned
}

/// The positioned boxes phase 2 places, in document order; each
/// element's positioned pseudo-elements from the last placement are
/// dropped on the way (`TuiExt::positioned_pseudos`), as phase 1 drops a
/// box's floated ones.
fn collect_positioned(dom: &mut Dom<TuiExt>) -> Vec<BoxItem> {
    let mut out = Vec::new();
    walk_for_positioned(dom, dom.root(), false, &mut out);
    out
}

/// `hidden`: under a `display: none` ancestor, where no box is generated
/// (CSS Display 3 §2.5) — its pseudo-elements get none.
fn walk_for_positioned(dom: &mut Dom<TuiExt>, id: NodeId, hidden: bool, out: &mut Vec<BoxItem>) {
    let mut hidden = hidden;
    let element = dom.node(id).node_type() == NodeType::Element;
    if element {
        let pos = computed_position(dom, id);
        if matches!(pos, Position::Absolute | Position::Fixed) {
            out.push(BoxItem::Node(id));
        }
        hidden |= dom
            .node(id)
            .computed()
            .is_some_and(|c| c.display == crate::layout::Display::None);
        if let Some(ext) = dom.node_mut(id).ext_mut() {
            ext.positioned_pseudos = None;
        }
    }
    let pseudo = |dom: &Dom<TuiExt>, slot| {
        (element && !hidden && super::pseudo::is_positioned_box(dom, id, slot))
            .then_some(BoxItem::Generated(id, slot))
    };
    out.extend(pseudo(dom, PseudoSlot::Before));
    let children: Vec<NodeId> = crate::render::box_tree::children(dom, id).collect();
    for c in children {
        if matches!(
            dom.node(c).node_type(),
            NodeType::Element | NodeType::Fragment
        ) {
            walk_for_positioned(dom, c, hidden, out);
        }
    }
    out.extend(pseudo(dom, PseudoSlot::After));
}
