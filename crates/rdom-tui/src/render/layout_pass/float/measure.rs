//! Floats in intrinsic sizes (CSS Sizing 3 §5.1 / §5.2): an inline
//! formatting context's lines packed beside its own floats, and a block
//! container's inline sizes with its floats beside the content that
//! follows them — on a scratch exclusion area, the container being the
//! root of its own formatting context (a float of an enclosing context is
//! not seen: DIVERGENCES). A block container's block size is its flow
//! measured as laid out (`block::measure`).

use rdom_core::{Dom, NodeId};

use super::area::ExclusionArea;
use super::lines::InlineFloats;
use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::render::inline::InlineLayout;
use crate::render::layout_pass::block::{Run, RunKind, flow_runs};

/// Whether one of `runs` holds a float of `id`'s own flow.
fn holds_floats(dom: &Dom<TuiExt>, runs: &[Run]) -> bool {
    runs.iter().any(|r| {
        r.kind == RunKind::Float || r.children.iter().any(|&c| super::is_float_item(dom, c))
    })
}

/// `block`'s inline content packed `width` cells wide beside its own
/// floats: the layout, and the rows from its top to the lowest float's
/// bottom (0 with none) — CSS 2.1 §10.6.7 for a root sized from its
/// content.
pub(in crate::render::layout_pass) fn inline_rows(
    dom: &Dom<TuiExt>,
    block: NodeId,
    width: u16,
) -> (InlineLayout, u16) {
    let mut area = ExclusionArea::default();
    let mut ex = InlineFloats::new(dom, &mut area, LayoutRect::new(0, 0, width, 0), 0);
    let layout =
        crate::render::inline::compute_inline_layout_around(dom, block, width, Some(&mut ex));
    drop(ex);
    (layout, rows_to(area.lowest()))
}

/// The inline size of the block container `id` whose flow holds floats
/// (`None` otherwise): CSS Sizing 3 §5.1 — its max-content size (`max`)
/// the widest of its runs laid out with no soft wrap, a float run's floats
/// side by side and beside the in-flow content that follows them (an
/// inline run packs its own floats as boxes in its line); its min-content
/// size the widest single piece, a float's whole. `outer(child, max)` is
/// a block-level child's outer contribution.
pub(in crate::render::layout_pass) fn block_width(
    dom: &Dom<TuiExt>,
    id: NodeId,
    max: bool,
    outer: &dyn Fn(NodeId, bool) -> u16,
) -> Option<u16> {
    let runs = flow_runs(dom, id);
    if !holds_floats(dom, &runs) {
        return None;
    }
    let available = if max { u16::MAX } else { 0 };
    let mut floats: u16 = 0;
    let mut widest: u16 = 0;
    for run in &runs {
        match run.kind {
            RunKind::Float => {
                for &f in &run.children {
                    let w = super::size::outer_contribution(dom, f, max);
                    floats = if max { floats.saturating_add(w) } else { w };
                    widest = widest.max(floats);
                }
            }
            RunKind::Inline => {
                let w = crate::render::inline::widest_run_line(dom, id, &run.children, available);
                widest = widest.max(if max { floats.saturating_add(w) } else { w });
                floats = 0;
            }
            RunKind::Block => {
                for child in run.children.iter().filter_map(|c| c.node()) {
                    let w = outer(child, max);
                    widest = widest.max(if max { floats.saturating_add(w) } else { w });
                    floats = 0;
                }
            }
        }
    }
    Some(widest)
}

/// Rows from 0 to `bottom` (0 when there is none or it is above).
fn rows_to(bottom: Option<i32>) -> u16 {
    bottom.map_or(0, |b| b.clamp(0, i32::from(u16::MAX)) as u16)
}
