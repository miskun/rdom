//! The clamp point of a line-clamp container (CSS Overflow 4 §4): the
//! row after its Nth in-flow line box — its own lines, its anonymous
//! block boxes' and its block-level descendants' in the same block
//! formatting context, in block order. Layout cuts the container's
//! automatic height there; paint hides what follows it and marks the Nth
//! line with the `block-ellipsis`. A container whose content ends at its
//! Nth line has no clamp point: nothing is hidden or marked.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::Display;

/// Where a line-clamp container is cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClampPoint {
    /// The first row (viewport coordinates) after the Nth line box.
    pub(crate) bottom: i32,
    /// The flow holding the Nth line: an element's own lines, or one of
    /// its anonymous block boxes (`Some(index)`).
    pub(crate) owner: NodeId,
    pub(crate) anon: Option<usize>,
    /// The line's index in that flow.
    pub(crate) line: usize,
}

/// One line box of the walk, in viewport rows.
struct Line {
    top: i32,
    bottom: i32,
    owner: NodeId,
    anon: Option<usize>,
    index: usize,
}

/// `id`'s clamp point, `None` when it is not a line-clamp container
/// (`ComputedStyle::line_clamp_container`) or its content ends by its
/// Nth line box.
pub(crate) fn clamp_point(dom: &Dom<TuiExt>, id: NodeId) -> Option<ClampPoint> {
    let c = dom.node(id).ext()?.computed.as_deref()?;
    if !c.line_clamp_container {
        return None;
    }
    let n = usize::try_from(c.max_lines?).ok()?;
    let mut lines = Vec::new();
    let mut content_bottom = i32::MIN;
    collect(dom, id, true, &mut lines, &mut content_bottom);
    // Block flow stacks its lines top to bottom.
    lines.sort_by_key(|l| l.top);
    let nth = lines.get(n.checked_sub(1)?)?;
    let truncated = lines.len() > n || content_bottom > nth.bottom;
    truncated.then_some(ClampPoint {
        bottom: nth.bottom,
        owner: nth.owner,
        anon: nth.anon,
        line: nth.index,
    })
}

/// Gather `id`'s line boxes into `lines` and the bottom of every box of
/// its block-level descendants into `bottom`. A descendant that is an
/// independent formatting context (or a flex / grid container) counts
/// as a box whose lines are its own; an out-of-flow one not at all.
fn collect(dom: &Dom<TuiExt>, id: NodeId, root: bool, lines: &mut Vec<Line>, bottom: &mut i32) {
    let Some(ext) = dom.node(id).ext() else {
        return;
    };
    let box_less = crate::render::box_tree::is_contents(dom, id);
    if !root && !box_less {
        *bottom = (*bottom).max(ext.layout.bottom());
    }
    if let (Some(il), Some(origin)) = (
        ext.inline_layout.as_ref(),
        crate::render::inline::scrolled_content_rect(dom, id),
    ) {
        for (index, l) in il.lines.iter().enumerate() {
            lines.push(Line {
                top: origin.y + i32::from(l.top),
                bottom: origin.y + i32::from(l.bottom()),
                owner: id,
                anon: None,
                index,
            });
        }
    }
    for (k, anon) in ext.anonymous_blocks.iter().enumerate() {
        for (index, l) in anon.inline_layout.lines.iter().enumerate() {
            lines.push(Line {
                top: anon.rect.y + i32::from(l.top),
                bottom: anon.rect.y + i32::from(l.bottom()),
                owner: id,
                anon: Some(k),
                index,
            });
        }
    }
    for child in super::element_children_of(dom, id) {
        if !super::is_in_flow(dom, child) {
            continue;
        }
        let Some(cc) = dom.node(child).ext().and_then(|e| e.computed.as_deref()) else {
            continue;
        };
        // An inline-level child sits in the lines already counted.
        if cc.display != Display::Block {
            continue;
        }
        if cc.establishes_new_bfc || !cc.flow.is_block_flow() {
            if let Some(e) = dom.node(child).ext() {
                *bottom = (*bottom).max(e.layout.bottom());
            }
            continue;
        }
        collect(dom, child, false, lines, bottom);
    }
}

/// `measurement` of `id`'s content laid out in `inner`, cut at its clamp
/// point when it has one (§4.4 `collapse`: the box's automatic height
/// ends at the clamp point).
pub(super) fn clamped(
    dom: &Dom<TuiExt>,
    id: NodeId,
    inner: crate::layout::LayoutRect,
    measurement: Option<super::block::BlockMeasurement>,
) -> Option<super::block::BlockMeasurement> {
    let Some(point) = clamp_point(dom, id) else {
        return measurement;
    };
    let scroll_y = dom.node(id).ext().map_or(0, |e| e.scroll_y);
    let rows = point.bottom + scroll_y - inner.y;
    Some(super::block::BlockMeasurement {
        content_height: rows.clamp(0, i32::from(u16::MAX)) as u16,
    })
}

/// The height of an inline layout clamped to `max_lines` line boxes —
/// the intrinsic block size of a line-clamp container whose lines are
/// its own (`intrinsic::wrapped_rows`).
pub(crate) fn clamped_lines_height(
    il: &crate::render::inline::InlineLayout,
    max_lines: u32,
) -> u16 {
    let n = usize::try_from(max_lines).unwrap_or(usize::MAX);
    match il.lines.get(n.saturating_sub(1)) {
        Some(line) if il.lines.len() > n => line.bottom(),
        _ => il.height(),
    }
}
