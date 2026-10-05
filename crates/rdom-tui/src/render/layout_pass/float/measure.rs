//! Floats in intrinsic sizes (CSS Sizing 3 §5.1 / §5.2): a block
//! container's content height and inline sizes measured with the same
//! rules layout follows — §9.5.1 placement, shortened line boxes,
//! clearance, block formatting context roots beside floats — on a scratch
//! exclusion area, the container being the root of its own formatting
//! context (a float of an enclosing context is not seen: DIVERGENCES).

use rdom_core::{Dom, NodeId};

use super::area::ExclusionArea;
use super::lines::InlineFloats;
use crate::ext::TuiExt;
use crate::layout::{Direction, LayoutRect, Size};
use crate::render::inline::{InlineLayout, RunPseudos};
use crate::render::layout_pass::block::{Run, RunKind, flow_runs};
use crate::render::layout_pass::intrinsic::contribution;

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

/// The content height of the block container `id` laid out `width`
/// cells wide, when its flow holds floats (`None` otherwise, its children
/// then stacking as `intrinsic::children` sums them): its runs in order —
/// a float placed at the cursor, an inline run packed beside the floats,
/// a block-level child `outer(child)` rows tall below the floats its
/// `clear` names and, when it establishes a block formatting context,
/// where the floats leave it room; a block child that does not is laid
/// out in the same area, its lines beside the same floats — reaching the
/// lowest float (§10.6.7).
pub(in crate::render::layout_pass) fn block_height(
    dom: &Dom<TuiExt>,
    id: NodeId,
    width: u16,
    outer: &dyn Fn(NodeId) -> u16,
) -> Option<u16> {
    let runs = flow_runs(dom, id);
    if !holds_floats(dom, &runs) {
        return None;
    }
    let mut area = ExclusionArea::default();
    let y = flow(dom, id, &runs, &mut area, 0, 0, width, outer);
    Some(rows_to(Some(y)).max(rows_to(area.lowest())))
}

/// Lay `runs`, the flow of `id`, out from row `y` — its content box's
/// top — in `[x0, x0 + width)` against `area`: the row after its content.
#[allow(clippy::too_many_arguments)]
fn flow(
    dom: &Dom<TuiExt>,
    id: NodeId,
    runs: &[Run],
    area: &mut ExclusionArea,
    x0: i32,
    mut y: i32,
    width: u16,
    outer: &dyn Fn(NodeId) -> u16,
) -> i32 {
    let content_top = y;
    for run in runs {
        match run.kind {
            RunKind::Float => {
                for &f in &run.children {
                    let at = super::Placement {
                        y,
                        x0,
                        cb_width: width,
                        content_top,
                    };
                    super::place(dom, area, f, at);
                }
            }
            RunKind::Inline => {
                let mut ex =
                    InlineFloats::new(dom, area, LayoutRect::new(x0, y, width, 0), content_top);
                let layout = crate::render::inline::pack_run(
                    dom,
                    id,
                    &run.children,
                    RunPseudos::default(),
                    width,
                    Some(&mut ex),
                );
                y += i32::from(layout.height());
            }
            RunKind::Block => {
                for child in run.children.iter().filter_map(|c| c.node()) {
                    y = block_top(dom, area, child, x0, y, width);
                    y = match in_same_context(dom, child, width) {
                        Some(chrome) => {
                            let runs = flow_runs(dom, child);
                            let inner = flow(
                                dom,
                                child,
                                &runs,
                                area,
                                x0 + chrome.left,
                                y + chrome.top,
                                chrome.width,
                                &|c| outer_height(dom, c, chrome.width),
                            );
                            inner + chrome.bottom
                        }
                        None => y + i32::from(outer(child)),
                    };
                }
            }
        }
    }
    y
}

/// A block child laid out in its parent's context: its margin, border
/// and padding above and below its content, left of it, and its content
/// width.
struct Chrome {
    top: i32,
    bottom: i32,
    left: i32,
    width: u16,
}

/// The chrome of `child`, a block-level child of a container `width`
/// cells wide, when its content flows in its parent's formatting context
/// — a block container that is no formatting context root, sized by its
/// content (`height: auto`), with a float in it or beside it to reach
/// its lines; `None` for any other box, which counts as its outer size.
fn in_same_context(dom: &Dom<TuiExt>, child: NodeId, width: u16) -> Option<Chrome> {
    let c = dom.node(child).ext()?.computed.as_deref()?;
    if !c.flow.is_block_flow()
        || !matches!(c.height, Size::Auto)
        || crate::render::layout_pass::block::establishes_bfc(dom, child, c)
    {
        return None;
    }
    let sizer_h = crate::render::layout_pass::box_sizing::Sizer::horizontal(c, width);
    let pad = |v: &crate::layout::PaddingValue| i32::from(v.resolve(width));
    let margin = |v: &crate::layout::MarginValue| i32::from(v.resolve(width));
    let ml = margin(&c.margin.left);
    let mr = margin(&c.margin.right);
    let border_box = match c.width {
        Size::Auto => (i32::from(width) - ml - mr).max(0) as u16,
        _ => crate::render::layout_pass::intrinsic::contribution(
            dom,
            child,
            Direction::Row,
            0,
            width,
            true,
        ),
    };
    Some(Chrome {
        top: margin(&c.margin.top) + i32::from(c.border.top.cells()) + pad(&c.padding.top),
        bottom: margin(&c.margin.bottom)
            + i32::from(c.border.bottom.cells())
            + pad(&c.padding.bottom),
        left: ml + i32::from(c.border.left.cells()) + pad(&c.padding.left),
        width: border_box.saturating_sub(sizer_h.chrome()),
    })
}

/// `child`'s outer height in a container `width` cells wide: its
/// height contribution there and its vertical margins.
fn outer_height(dom: &Dom<TuiExt>, child: NodeId, width: u16) -> u16 {
    let inner = contribution(dom, child, Direction::Column, width, width, true);
    let margins = dom
        .node(child)
        .ext()
        .and_then(|e| e.computed.as_deref())
        .map_or(0, |c| {
            i32::from(c.margin.top.resolve(width)) + i32::from(c.margin.bottom.resolve(width))
        });
    (i32::from(inner) + margins).clamp(0, i32::from(u16::MAX)) as u16
}

/// Where the block-level `child` goes, its top not above `y`: below the
/// floats its `clear` names (§9.5.2), and when it establishes a block
/// formatting context, at the first top where the floats leave it its
/// min-content width (§9.5).
fn block_top(
    dom: &Dom<TuiExt>,
    area: &ExclusionArea,
    child: NodeId,
    x0: i32,
    y: i32,
    width: u16,
) -> i32 {
    if area.is_empty() {
        return y;
    }
    let y = super::clearance_floor(dom, area, crate::render::box_tree::BoxItem::Node(child), y);
    let Some(c) = dom.node(child).ext().and_then(|e| e.computed.as_deref()) else {
        return y;
    };
    if !crate::render::layout_pass::block::establishes_bfc(dom, child, c) {
        return y;
    }
    let needed = match c.width {
        Size::Auto => i32::from(contribution(dom, child, Direction::Row, 0, width, false)).max(1),
        _ => i32::from(contribution(dom, child, Direction::Row, 0, width, true)),
    };
    let rows = contribution(dom, child, Direction::Column, width, width, true);
    area.opening(x0, x0 + i32::from(width), y, rows, |b| b.width() >= needed)
        .0
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
