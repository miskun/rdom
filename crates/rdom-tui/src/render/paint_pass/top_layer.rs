//! The document's top layer (CSS Position 4 "top layer"; HTML's modal
//! dialogs and showing popovers, `Dom::top_layer`): painted after the
//! whole document, bottom to top — each element's `::backdrop` over the
//! viewport, then the element as a stacking context of its own, clipped
//! by the viewport only. The stacking walk leaves these elements out
//! (`stacking::collect_layers`), so each paints once.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::render::{Buffer, Rect};
use crate::style::ComputedStyle;

/// Paint every rendered top-layer element, in top-layer order, with its
/// `::backdrop` beneath it (CSS Pseudo-Elements 4 §4: one per element
/// rendered in the top layer).
pub(super) fn paint_top_layer(dom: &Dom<TuiExt>, buf: &mut Buffer, clip: Rect) {
    for &id in dom.top_layer() {
        if !is_rendered(dom, id) {
            continue;
        }
        if let Some(backdrop) = dom.node(id).ext().and_then(|e| e.computed_backdrop()) {
            fill_backdrop(buf, clip, backdrop);
        }
        super::paint_stacking_context(dom, id, buf, clip, clip);
    }
}

/// Whether the top-layer element `id` generates a box: it and every
/// ancestor were cascaded to a `display` other than `none`.
pub(crate) fn is_rendered(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    dom.node(id).ext().is_some_and(|e| e.computed.is_some()) && crate::node::is_rendered(dom, id)
}

/// Fill every cell of `clip` with the backdrop's bg (and optional
/// fg). Uses `Buffer::cell_mut` so the pre-existing symbols are
/// preserved underneath — apps that want a solid wipe set an
/// explicit `content: " "` override on `::backdrop`.
fn fill_backdrop(buf: &mut Buffer, clip: Rect, style: &ComputedStyle) {
    // A translucent backdrop (`rgb(0 0 0 / 50%)`, the common web dim)
    // composites over the page (C3-ALPHA).
    buf.tint(clip, style.bg);
    buf.tint_glyphs(clip, style.fg);
}
