//! Outlines of inline boxes (CSS UI 4 §5, C12G-OUTLINE-INLINE): an
//! inline element with an outline is ringed on each line it has a
//! fragment on — one rectangle per line box around the cells its
//! fragments (its text, its `::before` / `::after` runs, the atoms in it)
//! take there, its rows those fragments span. §5.1 lets the outline of an
//! inline box broken across lines be one non-rectangular shape or one per
//! fragment; rdom draws one per fragment (DIVERGENCES §1).
//!
//! The rings are recorded for the stacking context's end
//! ([`defer`](super::super::outline::defer)), as a block's are. The cost
//! where nothing has an outline: one style read per fragment, whose
//! element is the flow's block for text directly in it.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{Display, LayoutRect};
use crate::render::inline::InlineLayout;
use crate::render::{Buffer, Rect};

/// One inline element's extent on one line: its cells `left..right`, its
/// rows `top..bottom`.
struct Extent {
    element: NodeId,
    left: i32,
    right: i32,
    top: i32,
    bottom: i32,
}

/// Record the outline rings of the inline elements of `layout`, laid out
/// in the content area `inner`, painted into `clip`.
pub(super) fn defer_inline_outlines(
    dom: &Dom<TuiExt>,
    layout: &InlineLayout,
    inner: LayoutRect,
    buf: &mut Buffer,
    clip: Rect,
) {
    for line in &layout.lines {
        let top = inner.y + i32::from(line.top);
        let mut extents: Vec<Extent> = Vec::new();
        for f in &line.fragments {
            // An atom's own outline is its box's; it widens its inline
            // ancestors' fragments.
            let start = if f.atomic {
                dom.node(f.node).parent_node().map(|p| p.id())
            } else {
                Some(f.node)
            };
            // Text where it is drawn: moved by the relatively positioned
            // inline boxes around it (ACID-FIX-5).
            let (fx, fy) = f.drawn_at();
            let x = inner.x + fx;
            let y = top + fy;
            let cells = (x, x + i32::from(f.width), y, y + i32::from(f.height.max(1)));
            note(dom, start, cells, &mut extents);
        }
        for g in &line.generated {
            if g.outside.is_some() {
                continue;
            }
            let (dx, dy) = g.offset;
            let x = inner.x + g.x + dx;
            let y = top + i32::from(g.y) + dy;
            let rows = g.atom.as_ref().map_or(1, |a| a.height.max(1));
            let cells = (x, x + i32::from(g.width), y, y + i32::from(rows));
            note(dom, Some(g.host), cells, &mut extents);
        }
        for e in extents {
            let Some(style) = dom.node(e.element).ext().and_then(|x| x.computed.clone()) else {
                continue;
            };
            let (Ok(width), Ok(height)) = (
                u16::try_from(e.right - e.left),
                u16::try_from(e.bottom - e.top),
            ) else {
                continue;
            };
            if width == 0 {
                continue;
            }
            let outer = LayoutRect::new(e.left, e.top, width, height);
            super::super::outline::defer(dom, buf, &style, outer, clip);
        }
    }
}

/// Widen the extents of the inline elements from `start` up — through
/// `display: contents` ones, which have no box — that have an outline,
/// by `cells` (`left, right, top, bottom`).
fn note(
    dom: &Dom<TuiExt>,
    start: Option<NodeId>,
    (left, right, top, bottom): (i32, i32, i32, i32),
    extents: &mut Vec<Extent>,
) {
    let mut cur = start;
    while let Some(id) = cur {
        let Some(style) = dom.node(id).ext().and_then(|e| e.computed.as_deref()) else {
            return;
        };
        match style.display {
            Display::Inline => {}
            Display::Contents => {
                cur = dom.node(id).parent_node().map(|p| p.id());
                continue;
            }
            _ => return,
        }
        if style.ui.outline_style.line().is_some() {
            match extents.iter_mut().find(|e| e.element == id) {
                Some(e) => {
                    e.left = e.left.min(left);
                    e.right = e.right.max(right);
                    e.top = e.top.min(top);
                    e.bottom = e.bottom.max(bottom);
                }
                None => extents.push(Extent {
                    element: id,
                    left,
                    right,
                    top,
                    bottom,
                }),
            }
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
}
