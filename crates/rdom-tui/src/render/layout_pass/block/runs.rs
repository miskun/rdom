//! Partitioning a block container's children into block-level and
//! inline-level runs (CSS 2.1 §9.2.1.1 anonymous block boxes).

use rdom_core::{Dom, NodeId, NodeType};

use super::*;
use crate::render::box_tree::BoxItem;

/// One run of consecutive children sharing a level (block-level or
/// inline-level). Block runs get per-child block layout; inline
/// runs fold into one anonymous block per CSS 2.1 §9.2.1.1.
pub(super) struct Run {
    pub(super) kind: RunKind,
    /// The parent's box items in document order (its child nodes, and
    /// the generated items of a box-less child that holds a block,
    /// `box_tree::box_sequence`).
    pub(super) children: Vec<BoxItem>,
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
        let holds_line = run.kind == RunKind::Block
            || run.children.iter().any(|&c| match c {
                BoxItem::Node(c) => bears_line(dom, id, c),
                BoxItem::Generated(..) => true,
            })
            || (i == 0 && pseudos.before)
            || (i == last && pseudos.after);
        if !holds_line {
            for c in run.children.iter().filter_map(|c| c.node()) {
                carried.extend(static_before.remove(&c).unwrap_or_default());
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
pub(super) enum RunKind {
    Block,
    Inline,
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
    let BoxItem::Node(id) = item else {
        return RunKind::Inline;
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
