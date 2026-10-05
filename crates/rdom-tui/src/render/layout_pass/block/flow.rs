//! A block container's flow (CSS 2.1 §9.4.1, §10.6.3): its in-flow box
//! items partitioned into runs (`runs`), then placed in order — a float
//! run's floats at the cursor (§9.5.1), a block run's block-level boxes
//! with their margins collapsing (§8.3.1, `margin_collapse`), an inline
//! run in an anonymous block box (§9.2.1.1) — and its content height.
//!
//! The one block-flow model: layout (`layout_block_children`) and
//! intrinsic measurement (`measure`) run this same loop and differ only in
//! what a piece *is* placed as ([`FlowSink`]): laid out in the document,
//! or measured against a scratch exclusion area. Every arithmetic — the
//! margin accumulator, the parent / child collapse predicates, `margin-trim`,
//! gaps, collapsed-border overlap, clearance and formatting context roots
//! beside floats (`place`), block-level pseudo-elements (`generated`) — is
//! here or behind it, once.

use std::collections::HashMap;

use rdom_core::{Dom, NodeId};

use super::generated::{self, GeneratedPlace};
use super::inline_run::RunPlace;
use super::margin_collapse::{
    MarginAccumulator, parent_collapses_bottom_with_last_child,
    parent_collapses_top_with_first_child,
};
use super::place::BlockPlace;
use super::runs::{
    Run, RunKind, drop_lineless_runs, first_flow_run, is_float, last_flow_run, partition,
};
use super::{BlockMeasurement, is_in_flow};
use crate::ext::{PseudoSlot, TuiExt};
use crate::layout::{Direction, LayoutRect};
use crate::render::box_tree::BoxItem;
use crate::render::inline::RunPseudos;
use crate::render::layout_pass::float::Placement;
use crate::style::ComputedStyle;

/// What a flow's pieces are placed as: laid out, or measured.
pub(super) trait FlowSink {
    /// The document, read.
    fn dom(&self) -> &Dom<TuiExt>;
    /// The out-of-flow boxes whose static position is just before
    /// `child` take `(x, y)` (CSS 2.1 §10.3.7 / §10.6.4) — layout only.
    fn anchor(&mut self, _child: NodeId, _x: i32, _y: i32) {}
    /// A float of a float run, its top not above `at.y`.
    fn float(&mut self, item: BoxItem, at: Placement);
    /// A block-level child at `place`: the cursor after it.
    fn block(&mut self, child: NodeId, place: BlockPlace<'_>) -> i32;
    /// A block-level `::before` / `::after` at `at`: the cursor after it.
    fn generated(&mut self, host: NodeId, slot: PseudoSlot, at: GeneratedPlace<'_>) -> Option<i32>;
    /// An inline run in an anonymous block box at `place`: its height.
    fn inline_run(&mut self, run: &Run, pseudos: RunPseudos, place: RunPlace) -> u16;
}

/// The out-of-flow positioned children of a flow, by the in-flow item
/// their static position is taken before, and those after its last.
#[derive(Default)]
pub(super) struct Statics {
    pub(super) before: HashMap<NodeId, Vec<NodeId>>,
    pub(super) trailing: Vec<NodeId>,
}

/// A block container's flow, partitioned: its runs, the length of its box
/// sequence, and its first and last in-flow element children.
pub(super) struct Prepared {
    pub(super) raw_len: usize,
    pub(super) runs: Vec<Run>,
    edges: (Option<NodeId>, Option<NodeId>),
}

/// Partition `id`'s box sequence (`box_tree::box_sequence`) into runs
/// (CSS 2.1 §9.2.1.1, `runs::partition`): the floats in float runs, the
/// runs that hold no line dropped, the host's `::before` / `::after` in an
/// anonymous block box of their own beside a block-level edge child.
/// `statics`, when asked for, gets the static-position anchors of `id`'s
/// out-of-flow positioned children. `None` when nothing is in flow and no
/// pseudo-element makes a line.
pub(super) fn prepare(
    dom: &Dom<TuiExt>,
    id: NodeId,
    statics: Option<&mut Statics>,
) -> Option<Prepared> {
    // Collect ALL direct child nodes (text + element), in box-tree
    // order (`box_sequence`: a `display: contents` child holding a
    // block box gives its own children and generated items). Block
    // layout distinguishes inline-level (text + Display::Inline/
    // InlineBlock elements) from block-level (Display::Block elements)
    // — text nodes are inline-level participants in an anonymous block
    // per CSS 2.1 §9.2.1.1 rule 2.
    let raw_children: Vec<BoxItem> = crate::render::box_tree::box_sequence(dom, id);
    // CSS 2.1 §12.1: `::before` / `::after` are the host's first / last
    // children, so a host whose only content is its generated text still
    // has an inline run — the pseudo-elements alone — and a line box.
    let pseudos = crate::render::inline::generated::visible_inline_pseudos(dom, id);
    let has_pseudos = pseudos.before || pseudos.after;
    let mut scratch = Statics::default();
    let statics = match statics {
        Some(s) => {
            // `D-M2-2`: out-of-flow positioned children take their static
            // position from the flow cursor at the point where their
            // hypothetical box would have gone — recorded just before the
            // in-flow sibling that follows them is placed.
            let raw_nodes: Vec<NodeId> = raw_children.iter().filter_map(|c| c.node()).collect();
            (s.before, s.trailing) =
                crate::render::layout_pass::positioning::static_anchors(dom, &raw_nodes);
            s
        }
        None => &mut scratch,
    };
    // Filter out-of-flow elements; text nodes are always in flow. Each
    // entry keeps its index into `raw_children` (the box sequence), and
    // the runs' `child_range`s are those RAW indices — not positions in
    // this filtered list — so an anonymous box's range can be matched
    // against a child's index in its parent's box sequence
    // (`inline_flow_for_text`, TREE-BFC-PSEUDO-1). Floats stay: out of
    // flow, but placed where they occur (CSS 2.1 §9.5.1).
    let in_flow: Vec<(usize, BoxItem)> = raw_children
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, c)| {
            c.node()
                .is_none_or(|c| is_in_flow(dom, c) || is_float(dom, c))
        })
        .collect();
    if in_flow.is_empty() && !has_pseudos {
        return None;
    }
    let edges = {
        let mut ids = in_flow
            .iter()
            .filter_map(|(_, c)| c.node())
            .filter(|&c| !is_float(dom, c));
        let first = ids.next();
        (first, ids.next_back().or(first))
    };
    let runs = partition(dom, &in_flow);
    let mut runs = drop_lineless_runs(dom, id, runs, &mut statics.before, &mut statics.trailing);
    if runs.is_empty() && has_pseudos {
        // No in-flow child holds a line: the pseudo-elements are the
        // whole inline content, in one anonymous block box.
        runs.push(Run::pseudo_only(raw_children.len()));
    }
    // CSS 2.1 §9.2.1.1: a `::before` (`::after`) whose host starts
    // (ends) with a block-level child is an inline box with no inline
    // run to join — it gets an anonymous block box of its own: an
    // empty inline run the placement loop packs with the pseudo alone.
    // (A leading / trailing whitespace run already carries it.)
    let own_line = crate::render::inline::generated::own_line_pseudos(dom, id);
    if own_line.before && first_flow_run(&runs).is_some_and(|r| r.kind == RunKind::Block) {
        let at = runs[0].child_range.0;
        runs.insert(0, Run::pseudo_only(at));
    }
    if own_line.after && last_flow_run(&runs).is_some_and(|r| r.kind == RunKind::Block) {
        let at = runs[runs.len() - 1].child_range.1;
        runs.push(Run::pseudo_only(at));
    }
    Some(Prepared {
        raw_len: raw_children.len(),
        runs,
        edges,
    })
}

/// `container`, `id`'s content area, less the parent-child
/// border-collapse insets (CSS 2.1 §17.6.3 + BFC-1 invariant): under
/// `border-collapse: collapse` with its own border, `layout_node`
/// expanded the content area into the border ring — right only when the
/// first / last child has a border of its own to share the cell with; a
/// content-bearing child would land on the painted border row. The same
/// per-edge inset flex applies, so the two modes agree.
pub(super) fn inset(
    dom: &Dom<TuiExt>,
    prepared: &Prepared,
    computed: &ComputedStyle,
    container: LayoutRect,
) -> LayoutRect {
    let (top, bottom, left, right) =
        crate::render::layout_pass::border_collapse::collapse_parent_edge_insets(
            dom,
            prepared.edges,
            computed,
        );
    LayoutRect::new(
        container.x + i32::from(left),
        container.y + i32::from(top),
        container.width.saturating_sub(left + right),
        container.height.saturating_sub(top + bottom),
    )
}

/// Where the flow starts: its content area (inset), and its scroll offsets.
#[derive(Clone, Copy)]
pub(super) struct FlowAt {
    pub(super) container: LayoutRect,
    pub(super) scroll_x: i32,
    pub(super) scroll_y: i32,
}

/// Where the flow ended: the cursor after its last in-flow content, the
/// bottom margin still buffered, whether that margin escapes through
/// the container (§8.3.1), and the content height.
pub(super) struct FlowEnd {
    pub(super) y_cursor: i32,
    pub(super) margin_acc: MarginAccumulator,
    pub(super) suppress_last_bottom: bool,
    pub(super) measurement: BlockMeasurement,
}

/// Place `prepared`, the flow of `id` (styled `computed`), from `at`
/// through `sink`, in order (CSS 2.1 §9.4.1).
pub(super) fn run<S: FlowSink>(
    sink: &mut S,
    id: NodeId,
    computed: &ComputedStyle,
    prepared: &Prepared,
    at: FlowAt,
) -> FlowEnd {
    let runs = &prepared.runs;
    let container = at.container;
    let containing_block_width = container.width;
    let content_x = container.x - at.scroll_x;
    let content_top = container.y - at.scroll_y;
    let mut y_cursor: i32 = content_top;
    // CSS 2.1 §8.3.1 — the unresolved margins between the last placed
    // block (or the container's top) and the next: adjacent in-flow
    // block siblings' collapse into one, `max(positives) +
    // min(negatives)`. Anonymous block boxes have no margins; they
    // resolve it. Parent–first / last-child collapse and empty
    // collapse-through blocks build on it.
    let mut margin_acc = MarginAccumulator::new();
    // §8.3.1: the first (last) in-flow block child's top (bottom) margin
    // collapses through the container when nothing separates them — it
    // surfaces at the container's outer edge, not inside it.
    let suppress_first_top = parent_collapses_top_with_first_child(sink.dom(), id, computed);
    let suppress_last_bottom = parent_collapses_bottom_with_last_child(sink.dom(), id, computed);
    // CSS Box 4 §3 `margin-trim`: a block-level child adjoining a trimmed
    // block-start / block-end content edge contributes no margin there.
    let trim = crate::render::layout_pass::margin_trim::trimmed_edges(computed);
    let trim_first_top = trim.top && first_flow_run(runs).is_some_and(|r| r.kind == RunKind::Block);
    let trim_last_bottom =
        trim.bottom && last_flow_run(runs).is_some_and(|r| r.kind == RunKind::Block);
    // The runs that hold the host's `::before` / `::after`: its first and
    // last in-flow ones (a float run holds no line).
    let first_line_run = runs.iter().position(|r| r.kind != RunKind::Float);
    let last_line_run = runs.iter().rposition(|r| r.kind != RunKind::Float);
    let last_block_run = runs.iter().rposition(|r| r.kind == RunKind::Block);
    // CSS Box Alignment 3 §8.1: `row-gap` between adjacent in-flow
    // block-level element children — not around anonymous block boxes
    // (whitespace runs between blocks would multiply it).
    let row_gap = crate::render::layout_pass::resolve_gap(computed, container, Direction::Column);
    let mut placed_blocks: usize = 0;
    // BORDER-MODEL-1 (M6): the previous block sibling, for the
    // collapsed-border overlap; an anonymous block box between breaks it.
    let mut prev_block: Option<NodeId> = None;
    for (run_idx, run) in runs.iter().enumerate() {
        match run.kind {
            RunKind::Float => {
                // A float between block-level boxes: its top where the
                // next box's would be, past the margins collapsed so far
                // (CSS 2.1 §9.5.1 rules 4–6).
                for &f in &run.children {
                    let y = y_cursor + i32::from(margin_acc.resolved());
                    if let Some(n) = f.node() {
                        sink.anchor(n, content_x, y);
                    }
                    sink.float(
                        f,
                        Placement {
                            y,
                            x0: content_x,
                            cb_width: containing_block_width,
                            content_top,
                        },
                    );
                }
            }
            RunKind::Block => {
                let is_last_block_run = Some(run_idx) == last_block_run;
                let last_child = run.children.len() - 1;
                for (i, item) in run.children.iter().enumerate() {
                    let child = match *item {
                        BoxItem::Node(child) => child,
                        BoxItem::Generated(host, slot) => {
                            let at = GeneratedPlace {
                                x: content_x,
                                cb_width: containing_block_width,
                                y_cursor,
                                margin_acc: &mut margin_acc,
                                index: generated::index(slot, prepared.raw_len),
                            };
                            if let Some(bottom) = sink.generated(host, slot, at) {
                                (y_cursor, prev_block) = (bottom, None);
                                placed_blocks += 1;
                            }
                            continue;
                        }
                    };
                    // The hypothetical box has zero margins: it collapses
                    // through whatever is buffered.
                    sink.anchor(
                        child,
                        content_x,
                        y_cursor + i32::from(margin_acc.resolved()),
                    );
                    let first = placed_blocks == 0;
                    let last = is_last_block_run && i == last_child;
                    if !first && row_gap > 0 {
                        y_cursor += i32::from(row_gap);
                    }
                    // BORDER-MODEL-1 (M6): under `border-collapse:
                    // collapse` with no gap, adjacent bordered blocks
                    // share their border row (paint's mask-OR draws the
                    // junction).
                    if row_gap == 0
                        && let Some(prev) = prev_block
                        && super::place::borders_overlap(sink.dom(), computed, prev, child)
                    {
                        y_cursor -= 1;
                    }
                    y_cursor = sink.block(
                        child,
                        BlockPlace {
                            container: LayoutRect::new(
                                content_x,
                                container.y,
                                container.width,
                                container.height,
                            ),
                            containing_block_width,
                            y_cursor,
                            margin_acc: &mut margin_acc,
                            suppress_top_margin: first && (suppress_first_top || trim_first_top),
                            suppress_bottom_margin: last
                                && (suppress_last_bottom || trim_last_bottom),
                        },
                    );
                    placed_blocks += 1;
                    prev_block = Some(child);
                }
            }
            RunKind::Inline => {
                // Runs cover the in-flow children in order, so the host's
                // `::before` / `::after` belong to the first / last run.
                let pseudos = RunPseudos {
                    before: Some(run_idx) == first_line_run,
                    after: Some(run_idx) == last_line_run,
                };
                // An anonymous block box has no margins: the buffered
                // ones resolve above it.
                let top = y_cursor + i32::from(margin_acc.resolved());
                margin_acc = MarginAccumulator::new();
                let height = sink.inline_run(
                    run,
                    pseudos,
                    RunPlace {
                        at: LayoutRect::new(content_x, top, containing_block_width, 0),
                        content_top,
                    },
                );
                y_cursor = top + i32::from(height);
                prev_block = None;
            }
        }
    }
    // CSS 2.1 §10.6.3 — the content height: from the first cursor to the
    // last in-flow content, with the bottom margin left in the
    // accumulator unless it escapes through the container (§8.3.1; a
    // formatting context root traps it).
    let mut content_height = (y_cursor - content_top).max(0);
    if !suppress_last_bottom {
        content_height = (content_height + i32::from(margin_acc.resolved())).max(0);
    }
    FlowEnd {
        y_cursor,
        margin_acc,
        suppress_last_bottom,
        measurement: BlockMeasurement {
            content_height: content_height.min(i32::from(u16::MAX)) as u16,
        },
    }
}
