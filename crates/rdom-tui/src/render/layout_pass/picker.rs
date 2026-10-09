//! A drop-down `<select>`'s picker placed against its select (HTML's
//! `::picker(select)`, whose UA style anchors it to the select with
//! `position-try-fallbacks: flip-block`; CSS Anchor Positioning 1 §4):
//! rdom's picker is the select's option children overflowing its one
//! in-flow row, in the top layer (`TopLayerKind::Picker`). When that list
//! would run past the viewport's bottom it is tried flipped — mirrored
//! about the field's row, its last option there — and used if it fits;
//! when neither fits the base stays (§4.3: "Return current styles")
//! (C15G-SELECT-FLIP).

use rdom_core::{Dom, NodeId, TopLayerKind};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;

/// Flip each open picker that overflows `viewport` below and fits above.
pub(super) fn place_pickers(dom: &mut Dom<TuiExt>, viewport: LayoutRect) {
    let pickers: Vec<NodeId> = dom
        .top_layer()
        .iter()
        .copied()
        .filter(|&id| dom.top_layer_kind(id) == Some(TopLayerKind::Picker))
        .collect();
    for select in pickers {
        let Some(field) = dom.node(select).ext().map(|e| e.layout) else {
            continue;
        };
        let rows = super::element_children_of(dom, select)
            .into_iter()
            .filter_map(|o| dom.node(o).ext().map(|e| e.layout))
            .filter(|r| r.height > 0)
            .fold(None, |acc: Option<(i32, i32)>, r| {
                let (top, bottom) = (r.y, r.y + i32::from(r.height));
                Some(acc.map_or((top, bottom), |(t, b)| (t.min(top), b.max(bottom))))
            });
        let Some((top, bottom)) = rows else {
            continue;
        };
        let view_bottom = viewport.y + i32::from(viewport.height);
        if bottom <= view_bottom {
            continue;
        }
        // Mirrored about the field: the list ends where the field does.
        let dy = (field.y + i32::from(field.height)) - bottom;
        if dy >= 0 || top + dy < viewport.y {
            continue;
        }
        for option in super::element_children_of(dom, select) {
            super::tree::shift_box(dom, option, 0, dy);
        }
    }
}
