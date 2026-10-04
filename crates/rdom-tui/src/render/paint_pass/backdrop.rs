//! The `::backdrop` of open modal dialogs: painted over the whole
//! viewport after the main pass, then the dialog painted again on top
//! of it — which works without the top layer as a stacking concept.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::render::{Buffer, Rect};
use crate::style::ComputedStyle;

/// Find every open modal `<dialog>` (any element with both `open`
/// and `data-rdom-modal` attributes), overlay its `::backdrop`
/// style across the viewport, and re-paint the dialog subtree on
/// top. Works without z-index support by running as a post-pass.
pub(super) fn paint_modal_backdrops(dom: &Dom<TuiExt>, buf: &mut Buffer, clip: Rect) {
    let mut modals: Vec<NodeId> = Vec::new();
    collect_modal_dialogs(dom, dom.root(), &mut modals);
    for dialog_id in modals {
        let Some(backdrop_style) = dom
            .node(dialog_id)
            .ext()
            .and_then(|e| e.computed_backdrop.clone())
        else {
            continue;
        };
        fill_backdrop(buf, clip, &backdrop_style);
        super::paint_stacking_context(dom, dialog_id, buf, clip, clip);
    }
}

fn collect_modal_dialogs(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<NodeId>) {
    let node = dom.node(id);
    if node.tag_name() == Some("dialog")
        && node.has_attribute("open")
        && node.has_attribute("data-rdom-modal")
    {
        out.push(id);
    }
    for child in node.child_nodes() {
        collect_modal_dialogs(dom, child.id(), out);
    }
}

/// Fill every cell of `clip` with the backdrop's bg (and optional
/// fg). Uses `Buffer::cell_mut` so the pre-existing symbols are
/// preserved underneath — apps that want a solid wipe set an
/// explicit `content: " "` override on `dialog::backdrop`.
fn fill_backdrop(buf: &mut Buffer, clip: Rect, style: &ComputedStyle) {
    // A translucent backdrop (`rgb(0 0 0 / 50%)`, the common web dim)
    // composites over the page (C3-ALPHA).
    buf.tint(clip, style.bg);
    buf.tint_glyphs(clip, style.fg);
}
