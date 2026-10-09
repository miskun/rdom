//! Whether a box is drawn (CSS Display 3 §4 `visibility`): the one
//! answer paint, hit-testing and focus share — the computed value, which
//! holds a running transition's value.

use rdom_core::{Dom, NodeId};

use crate::ext::{StyleSlot, TuiExt};
use crate::layout::Visibility;

/// `id`'s `slot` box's used `visibility`: the computed one (a running
/// transition's value while one runs). `Visible` for a node with no
/// computed style (never cascaded, or not an element).
pub(crate) fn visibility_of(dom: &Dom<TuiExt>, id: NodeId, slot: StyleSlot) -> Visibility {
    let node = dom.node(id);
    let Some(ext) = node.ext() else {
        return Visibility::Visible;
    };
    let computed = ext.computed_for(slot);
    computed.map_or(Visibility::Visible, |c| c.visibility)
}

/// `id`'s `slot` box is drawn: visible, and not hidden by an
/// anchor-positioned ancestor's `position-visibility` (CSS Anchor
/// Positioning 1 §5) — which a descendant's `visibility` cannot undo.
pub(crate) fn shows(dom: &Dom<TuiExt>, id: NodeId, slot: StyleSlot) -> bool {
    visibility_of(dom, id, slot).is_visible()
        && !crate::render::layout_pass::position_hidden(
            dom,
            id,
            match slot {
                StyleSlot::Before => Some(crate::ext::PseudoSlot::Before),
                StyleSlot::After => Some(crate::ext::PseudoSlot::After),
                _ => None,
            },
        )
}
