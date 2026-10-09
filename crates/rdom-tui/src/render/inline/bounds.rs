//! The box of a non-atomic inline element, which has no rect of its own —
//! only fragments in its block's lines: their bounding box (CSS Anchor
//! Positioning 1 §2: a fragmented anchor's box is "the axis-aligned
//! bounding rectangle" of its fragments; CSSOM View §6.1
//! `getBoundingClientRect()` likewise).

use rdom_core::{Dom, NodeId};

use super::InlineLayout;
use crate::ext::TuiExt;
use crate::layout::{Display, LayoutRect};
use crate::node::TuiNodeExt;

/// The bounding box of the fragments of the inline element `id` — its
/// text, its atoms and its generated runs, where they are drawn — in its
/// block's lines; `None` when it has none (or is not inline).
pub(crate) fn inline_box_rect(dom: &Dom<TuiExt>, id: NodeId) -> Option<LayoutRect> {
    // The block whose lines hold it: the nearest ancestor that is not an
    // inline box (a `display: contents` one has no box either).
    let mut block = dom.node(id).parent_node().map(|p| p.id())?;
    while dom
        .node(block)
        .computed()
        .is_some_and(|c| matches!(c.display, Display::Inline | Display::Contents))
    {
        block = dom.node(block).parent_node().map(|p| p.id())?;
    }
    let mut bounds: Option<(i32, i32, i32, i32)> = None;
    if let Some(layout) = dom.node(block).ext().and_then(|e| e.inline_layout.as_ref())
        && let Some(inner) = super::scrolled_content_rect(dom, block)
    {
        widen(dom, id, layout, inner, &mut bounds);
    }
    for anon in crate::render::box_tree::icb::anonymous_blocks(dom, block) {
        widen(dom, id, &anon.inline_layout, anon.rect, &mut bounds);
    }
    let (left, top, right, bottom) = bounds?;
    Some(LayoutRect::new(
        left,
        top,
        u16::try_from(right - left).unwrap_or(u16::MAX),
        u16::try_from(bottom - top).unwrap_or(u16::MAX),
    ))
}

/// Widen `bounds` (`left, top, right, bottom`) by the fragments of `id`
/// and its descendants in `layout`, laid out from `inner`.
fn widen(
    dom: &Dom<TuiExt>,
    id: NodeId,
    layout: &InlineLayout,
    inner: LayoutRect,
    bounds: &mut Option<(i32, i32, i32, i32)>,
) {
    let inside = |n: NodeId| n == id || dom.node(id).contains(n);
    let mut note = |x: i32, y: i32, w: u16, h: u16| {
        let (r, b) = (x + i32::from(w), y + i32::from(h.max(1)));
        *bounds = Some(match *bounds {
            None => (x, y, r, b),
            Some((l, t, rr, bb)) => (l.min(x), t.min(y), rr.max(r), bb.max(b)),
        });
    };
    for line in &layout.lines {
        let top = inner.y + i32::from(line.top);
        for f in line.fragments.iter().filter(|f| inside(f.node)) {
            let (fx, fy) = f.drawn_at();
            note(inner.x + fx, top + fy, f.width, f.height);
        }
        for g in line
            .generated
            .iter()
            .filter(|g| g.outside.is_none() && inside(g.host))
        {
            let (dx, dy) = g.offset;
            let rows = g.atom.as_ref().map_or(1, |a| a.height);
            note(inner.x + g.x + dx, top + i32::from(g.y) + dy, g.width, rows);
        }
    }
}
