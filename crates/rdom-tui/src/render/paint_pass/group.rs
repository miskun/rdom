//! `opacity` group rendering (OPACITY-1): an element with
//! `opacity < 1` paints its stacking context into a layer at full
//! opacity, and the layer composites back onto the frame at the
//! element's alpha (`Buffer::composite_group`).
//!
//! The layer covers only the rows the subtree can paint
//! ([`layer_region`]), across the full frame width, so a translucent
//! element costs O(W · its height) rather than O(W · H). Full width
//! keeps every buffer-edge rule the paint pass applies at the frame's
//! left / right edges (the border off-buffer filter, the wide-glyph
//! right-edge ellipsis) identical; the one-row margin above and below
//! keeps the top / bottom rules identical, because no paint of the
//! subtree lands on a margin row that is not also a frame edge.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Display, LayoutRect};
use crate::render::{Buffer, Rect};

/// Paint `root`'s stacking context at `alpha` through a bounded layer.
/// `paint` paints the context into the buffer it is given.
pub(super) fn paint_group(
    dom: &Dom<TuiExt>,
    root: NodeId,
    buf: &mut Buffer,
    alpha: f32,
    paint: impl Fn(&mut Buffer),
) {
    let region = layer_region(dom, root, buf.area);
    let mut layer = buf.copy_region(region);
    paint(&mut layer);
    #[cfg(test)]
    let unbounded = {
        // Every paint test doubles as a check that the bound is exact:
        // the same group painted through a full-frame layer composites
        // to the same frame.
        let mut full = buf.clone();
        let mut full_layer = buf.clone();
        paint(&mut full_layer);
        full.composite_group(&full_layer, alpha);
        full
    };
    buf.composite_group(&layer, alpha);
    #[cfg(test)]
    {
        assert_eq!(*buf, unbounded, "bounded layer diverged for {root:?}");
        assert_eq!(buf.border_dirs, unbounded.border_dirs);
        assert_eq!(buf.half_block_quads, unbounded.half_block_quads);
    }
}

/// The frame region an `opacity` group rooted at `root` can paint:
/// the full width of `area`, over the rows spanned by every box in the
/// subtree (element boxes, anonymous block boxes, pseudo-element
/// boxes, and each inline layout's lines), widened by one row above
/// and below and clamped to `area`.
pub(super) fn layer_region(dom: &Dom<TuiExt>, root: NodeId, area: Rect) -> Rect {
    let mut rows: Option<(i64, i64)> = None;
    collect_rows(dom, root, &mut rows);
    let Some((top, bottom)) = rows else {
        return Rect::new(area.x, area.y, area.width, 0);
    };
    let top = (top - 1).max(i64::from(area.y));
    let bottom = (bottom + 1).min(i64::from(area.bottom()));
    if bottom <= top {
        return Rect::new(area.x, area.y, area.width, 0);
    }
    Rect::new(area.x, top as u16, area.width, (bottom - top) as u16)
}

fn extend(rows: &mut Option<(i64, i64)>, top: i64, bottom: i64) {
    if bottom <= top {
        return;
    }
    *rows = Some(match *rows {
        None => (top, bottom),
        Some((t, b)) => (t.min(top), b.max(bottom)),
    });
}

fn extend_rect(rows: &mut Option<(i64, i64)>, r: LayoutRect) {
    extend(rows, i64::from(r.y), i64::from(r.y) + i64::from(r.height));
}

fn collect_rows(dom: &Dom<TuiExt>, id: NodeId, rows: &mut Option<(i64, i64)>) {
    let node = dom.node(id);
    if node.node_type() == NodeType::Element
        && let Some(ext) = node.ext()
    {
        if ext
            .computed
            .as_ref()
            .is_some_and(|c| c.display == Display::None)
        {
            return;
        }
        extend_rect(rows, ext.layout);
        for anon in &ext.anonymous_blocks {
            extend_rect(rows, anon.rect);
        }
        for pseudo in [ext.before_layout, ext.after_layout].into_iter().flatten() {
            extend_rect(rows, pseudo.rect);
        }
        if let Some(layout) = &ext.inline_layout {
            // Lines past the box (overflowing text, anchor tagging)
            // still address rows below its content origin.
            let y = i64::from(ext.content_layout.y);
            extend(rows, y, y + layout.lines.len() as i64);
        }
    }
    for child in node.child_nodes() {
        if matches!(child.node_type(), NodeType::Element | NodeType::Fragment) {
            collect_rows(dom, child.id(), rows);
        }
    }
}
