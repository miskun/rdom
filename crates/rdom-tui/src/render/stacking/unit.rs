//! The phases of a paint unit that gathers them when it paints — a float
//! or an atomic box (an inline block, an inline flex or grid container, a
//! flex or grid item), which paints "as if it created a new stacking
//! context, but any positioned descendants and descendants which actually
//! create a new stacking context [are] part of the parent stacking
//! context" (CSS 2.1 Appendix E, 7.2.1.4.1.1; §9.5 for floats; Flexbox
//! §5.4, Grid 2 §6.5 for items): its in-flow block-level boxes (step 4)
//! and its floats (step 5), in tree order. A walk of its subtree that
//! stops at positioned boxes and stacking contexts (the enclosing
//! context's layers hold them), at nested atomic boxes and at floats
//! (paint units of their own), so each box is visited by one unit only.

use rdom_core::{Dom, NodeId, NodeType};

use super::{
    BoxEntry, children_clip, creates_stacking_context, is_float, is_layered, is_positioned,
    is_z_indexed_item, paints_atomically,
};
use crate::ext::{PseudoSlot, TuiExt};
use crate::layout::Display;
use crate::render::Rect;

/// A float of a paint unit: an element, or the `k`-th floated
/// pseudo-element `id`'s formatting context run placed (`generated`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnitFloat {
    pub id: NodeId,
    pub generated: Option<usize>,
    /// The clip it paints into.
    pub clip: Rect,
}

/// The phases of the unit rooted at `root`, whose content paints into
/// `content_clip`: `f` gets each in-flow block-level box in tree order
/// (the background phase, painted as it is met), `floats` each float.
/// Allocates nothing unless a flex item is reordered
/// (`paint_order_children`) or the unit holds a float.
pub(crate) fn for_each_unit_box(
    dom: &Dom<TuiExt>,
    root: NodeId,
    content_clip: Rect,
    f: &mut impl FnMut(BoxEntry),
    floats: &mut Vec<UnitFloat>,
) {
    walk(dom, root, root, content_clip, f, floats);
}

fn walk(
    dom: &Dom<TuiExt>,
    id: NodeId,
    box_parent: NodeId,
    clip: Rect,
    f: &mut impl FnMut(BoxEntry),
    floats: &mut Vec<UnitFloat>,
) {
    if id == box_parent {
        generated(dom, id, clip, PseudoSlot::Before, f, floats);
    }
    let kids = if id == box_parent {
        crate::render::box_tree::paint_order_children(dom, id)
    } else {
        crate::render::box_tree::PaintOrder::tree(dom, id)
    };
    for cid in kids {
        super::visit();
        let child = dom.node(cid);
        match child.node_type() {
            NodeType::Fragment => {
                walk(dom, cid, box_parent, clip, f, floats);
                continue;
            }
            NodeType::Element if crate::render::box_tree::is_contents(dom, cid) => {
                walk(dom, cid, box_parent, clip, f, floats);
                continue;
            }
            NodeType::Element => {}
            _ => continue,
        }
        let Some(c) = child.ext().and_then(|e| e.computed.as_ref()) else {
            box_entry(dom, box_parent, cid, clip, f);
            walk(dom, cid, cid, clip, f, floats);
            continue;
        };
        if c.display == Display::None {
            continue;
        }
        if !is_positioned(c) && !is_z_indexed_item(dom, box_parent, c) && is_float(dom, cid) {
            floats.push(UnitFloat {
                id: cid,
                generated: None,
                clip,
            });
            continue;
        }
        if is_layered(dom, cid, box_parent, c)
            || creates_stacking_context(dom, box_parent, c)
            || paints_atomically(dom, box_parent, c)
        {
            continue;
        }
        box_entry(dom, box_parent, cid, clip, f);
        walk(dom, cid, cid, children_clip(dom, cid, c, clip), f, floats);
    }
    if id == box_parent {
        generated(dom, id, clip, PseudoSlot::After, f, floats);
    }
}

/// The in-flow, non-atomic element `id` in the background phase, when
/// `box_parent`'s content paint draws it as a box.
fn box_entry(
    dom: &Dom<TuiExt>,
    box_parent: NodeId,
    id: NodeId,
    clip: Rect,
    f: &mut impl FnMut(BoxEntry),
) {
    if crate::render::paint_pass::paints_child_box(dom, box_parent, id) {
        f(BoxEntry {
            id,
            generated: None,
            clip,
            unit: 0,
        });
    }
}

/// `id`'s block-level `slot` pseudo-element in the background phase, and
/// its run's floated pseudo-elements on that side of its children
/// (`collect::Walk::generated`'s order).
fn generated(
    dom: &Dom<TuiExt>,
    id: NodeId,
    clip: Rect,
    slot: PseudoSlot,
    f: &mut impl FnMut(BoxEntry),
    floats: &mut Vec<UnitFloat>,
) {
    let Some(ext) = dom.node(id).ext() else {
        return;
    };
    if !crate::render::box_tree::is_flex_or_grid_container(dom, id) {
        for (k, anon) in ext.anonymous_blocks.iter().enumerate() {
            if anon.generated.is_some_and(|g| g.slot == slot) {
                f(BoxEntry {
                    id,
                    generated: Some(k),
                    clip,
                    unit: 0,
                });
            }
        }
    }
    let leading = slot == PseudoSlot::Before;
    for (k, g) in ext.floated_pseudos().iter().enumerate() {
        let own_before = g
            .generated
            .is_some_and(|g| g.host == id && g.slot == PseudoSlot::Before);
        if own_before == leading {
            floats.push(UnitFloat {
                id,
                generated: Some(k),
                clip,
            });
        }
    }
}
