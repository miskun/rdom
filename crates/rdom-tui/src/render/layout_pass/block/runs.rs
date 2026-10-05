//! Partitioning a block container's children into block-level and
//! inline-level runs (CSS 2.1 §9.2.1.1 anonymous block boxes).

use rdom_core::{Dom, NodeId, NodeType};

use super::*;
use crate::render::box_tree::BoxItem;
use crate::render::layout_pass::float;

/// One run of consecutive children sharing a level (block-level or
/// inline-level). Block runs get per-child block layout; inline
/// runs fold into one anonymous block per CSS 2.1 §9.2.1.1.
pub(in crate::render::layout_pass) struct Run {
    pub(in crate::render::layout_pass) kind: RunKind,
    /// The parent's box items in document order (its child nodes, and
    /// the generated items of a box-less child that holds a block,
    /// `box_tree::box_sequence`).
    pub(in crate::render::layout_pass) children: Vec<BoxItem>,
    /// Indices into the parent's box sequence, as `[start, end)`.
    /// Stored on the resulting `AnonymousIfc` so paint / hit-test can
    /// map back to surrounding context.
    pub(super) child_range: (usize, usize),
}

impl Run {
    /// The anonymous block box of a pseudo-element with no inline run to
    /// join: no children, an empty child range at `at`.
    pub(super) fn pseudo_only(at: usize) -> Self {
        Run {
            kind: RunKind::Inline,
            children: Vec::new(),
            child_range: (at, at),
        }
    }
}

/// Partition a block container's in-flow box items — `(index in its box
/// sequence, item)`, floats included — into runs. A run is a contiguous
/// sequence of children that share a level (block or inline); when the
/// level flips, the run closes and a new one opens. Comments and
/// fragments are inline-level (no effect on layout beyond breaking
/// adjacency). A float (CSS 2.1 §9.5) joins an open inline run — the
/// packer places it beside the run's lines — and otherwise stands in a
/// float run of its own, placed at the flow's cursor (`RunKind::Float`).
pub(in crate::render::layout_pass) fn partition(
    dom: &Dom<TuiExt>,
    in_flow: &[(usize, BoxItem)],
) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for (orig_idx, child_id) in in_flow {
        let floated = crate::render::layout_pass::float::is_float_item(dom, *child_id);
        let kind = match runs.last() {
            _ if !floated => child_level(dom, *child_id),
            Some(last) if last.kind == RunKind::Inline => RunKind::Inline,
            _ => RunKind::Float,
        };
        match runs.last_mut() {
            Some(last) if last.kind == kind => {
                last.children.push(*child_id);
                last.child_range.1 = orig_idx + 1;
            }
            _ => runs.push(Run {
                kind,
                children: vec![*child_id],
                child_range: (*orig_idx, orig_idx + 1),
            }),
        }
    }
    runs
}

/// Whether the child `id` floats (CSS 2.1 §9.5): out of flow, but laid
/// out by this pass where it occurs.
pub(in crate::render::layout_pass) fn is_float(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    crate::render::layout_pass::float::float_side(dom, id).is_some()
}

/// The first run that is not a float run: the first that holds a line or
/// a block-level box.
pub(super) fn first_flow_run(runs: &[Run]) -> Option<&Run> {
    runs.iter().find(|r| r.kind != RunKind::Float)
}

/// The last run that is not a float run.
pub(super) fn last_flow_run(runs: &[Run]) -> Option<&Run> {
    runs.iter().rev().find(|r| r.kind != RunKind::Float)
}

/// `id`'s in-flow box items and floats partitioned into runs, as
/// `layout_block_children` partitions them (before it drops the runs that
/// hold no line) — for intrinsic sizing to measure what layout lays out.
pub(in crate::render::layout_pass) fn flow_runs(dom: &Dom<TuiExt>, id: NodeId) -> Vec<Run> {
    #[cfg(test)]
    FLOW_RUNS.with(|c| c.set(c.get() + 1));
    let items: Vec<(usize, BoxItem)> = crate::render::box_tree::box_sequence(dom, id)
        .into_iter()
        .enumerate()
        .filter(|(_, c)| {
            c.node()
                .is_none_or(|c| is_in_flow(dom, c) || is_float(dom, c))
        })
        .collect();
    partition(dom, &items)
}

#[cfg(test)]
thread_local! {
    /// [`flow_runs`] calls (tests only: the idle-cost pin).
    pub(in crate::render::layout_pass) static FLOW_RUNS: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

/// CSS 2.1 §9.2.1.1 / §16.6.1: white space that the `white-space`
/// property collapses away generates no inline box, so an inline run
/// holding nothing else — collapsible whitespace-only text, comments —
/// generates no anonymous block box: no zero-height box between block
/// siblings to break their margin collapsing, and none for a nested
/// scroll container to report. A run is kept when it carries the
/// host's visible `::before` (as the first run) or `::after` (as the
/// last); the generated text holds a line.
///
/// The out-of-flow boxes anchored on a dropped run's children take
/// their static position from what follows instead: the next kept
/// run's first child, else the trailing position.
pub(super) fn drop_lineless_runs(
    dom: &Dom<TuiExt>,
    id: NodeId,
    runs: Vec<Run>,
    static_before: &mut HashMap<NodeId, Vec<NodeId>>,
    static_trailing: &mut Vec<NodeId>,
) -> Vec<Run> {
    use crate::render::inline::generated::{bears_line, visible_inline_pseudos};
    let pseudos = visible_inline_pseudos(dom, id);
    let last = runs.len().saturating_sub(1);
    let mut carried: Vec<NodeId> = Vec::new();
    let mut kept = Vec::with_capacity(runs.len());
    for (i, run) in runs.into_iter().enumerate() {
        let holds_line = run.kind != RunKind::Inline
            || run.children.iter().any(|&c| match c {
                BoxItem::Node(c) => bears_line(dom, id, c),
                // A floated pseudo-element holds no line (CSS 2.1 §9.5).
                BoxItem::Generated(..) => !float::is_float_item(dom, c),
            })
            || (i == 0 && pseudos.before)
            || (i == last && pseudos.after);
        if !holds_line {
            for c in run.children.iter().filter_map(|c| c.node()) {
                carried.extend(static_before.remove(&c).unwrap_or_default());
            }
            // Its floats still take their place, at the cursor.
            let floats: Vec<BoxItem> = run
                .children
                .iter()
                .copied()
                .filter(|&c| float::is_float_item(dom, c))
                .collect();
            if !floats.is_empty() {
                kept.push(Run {
                    kind: RunKind::Float,
                    children: floats,
                    child_range: run.child_range,
                });
            }
            continue;
        }
        if !carried.is_empty()
            && let Some(first) = run.children.first().and_then(|c| c.node())
        {
            let mut anchored = std::mem::take(&mut carried);
            anchored.extend(static_before.remove(&first).unwrap_or_default());
            static_before.insert(first, anchored);
        }
        kept.push(run);
    }
    if !carried.is_empty() {
        carried.append(static_trailing);
        *static_trailing = carried;
    }
    kept
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render::layout_pass) enum RunKind {
    Block,
    Inline,
    /// Floats with no inline run to join (CSS 2.1 §9.5): placed at the
    /// flow's cursor, holding no line.
    Float,
}

/// Classify a box item as block-level vs inline-level. Text
/// nodes are always inline-level; element children depend on their
/// `Display`. Per CSS 2.1 §9.2: only `Block` elements are
/// block-level; `Inline` and `InlineBlock` are inline-level (the
/// inline-block participates in IFC as an atomic box per phase
/// 3.5's planned inline-block-in-IFC packing). A box-less element in
/// the sequence holds no block box (`box_sequence`), so it is an
/// inline-level participant, and so are generated items.
pub(super) fn child_level(dom: &Dom<TuiExt>, item: BoxItem) -> RunKind {
    let id = match item {
        BoxItem::Node(id) => id,
        // A block-level pseudo-element of the container itself.
        BoxItem::Generated(host, slot)
            if crate::render::inline::generated::is_block_pseudo(dom, host, slot.into()) =>
        {
            return RunKind::Block;
        }
        BoxItem::Generated(..) => return RunKind::Inline,
    };
    let node = dom.node(id);
    match node.node_type() {
        NodeType::Text => RunKind::Inline,
        NodeType::Element => {
            let display = node
                .ext()
                .and_then(|e| e.computed.as_ref())
                .map(|c| c.display)
                .unwrap_or(crate::layout::Display::Block);
            match display {
                crate::layout::Display::Inline
                | crate::layout::Display::InlineBlock
                | crate::layout::Display::Contents => RunKind::Inline,
                crate::layout::Display::Block | crate::layout::Display::None => RunKind::Block,
            }
        }
        // Comments, fragments — treat as inline-level (effectively
        // invisible; they don't break runs).
        _ => RunKind::Inline,
    }
}

/// The inline-level runs of `id`'s in-flow box items (CSS 2.1 §9.2.1.1:
/// each the content of one anonymous block box, packed as one inline
/// formatting context), in order, cut at its block-level children — the
/// partition `layout_block_children` makes, for intrinsic sizing to
/// measure what layout packs.
pub(in crate::render::layout_pass) fn inline_runs(
    dom: &Dom<TuiExt>,
    id: NodeId,
) -> Vec<Vec<BoxItem>> {
    let mut runs: Vec<Vec<BoxItem>> = Vec::new();
    let mut open = false;
    for item in crate::render::box_tree::box_sequence(dom, id) {
        // Out of flow: positioned and floated boxes, a floated
        // pseudo-element too.
        if item
            .node()
            .map_or_else(|| float::is_float_item(dom, item), |c| !is_in_flow(dom, c))
        {
            continue;
        }
        match child_level(dom, item) {
            RunKind::Inline if open => runs.last_mut().expect("an open run").push(item),
            RunKind::Inline => {
                runs.push(vec![item]);
                open = true;
            }
            // `child_level` never gives `Float`: floats are filtered above.
            RunKind::Block | RunKind::Float => open = false,
        }
    }
    runs
}

/// Whether the element `id` is block-level in its parent's flow (it
/// ends an inline run, [`inline_runs`]).
pub(in crate::render::layout_pass) fn is_block_level(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    child_level(dom, BoxItem::Node(id)) == RunKind::Block
}
