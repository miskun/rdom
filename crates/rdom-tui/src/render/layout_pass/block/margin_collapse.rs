//! CSS 2.1 §8.3.1 vertical margin collapsing for block flow: the
//! accumulator that merges adjoining margins, the parent–child and
//! collapse-through predicates, and the outer-margin chain walkers
//! that surface a nested first / last child's margin at its
//! ancestor's edge (each level resolving percentages against its own
//! containing block).

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{MarginChainMemo, TuiExt};
use crate::layout::{MarginValue, Size};
use crate::style::ComputedStyle;

use super::super::is_in_flow;
use super::width::block_content_width;

/// CSS 2.1 §8.3.1 — does this block's top and bottom margins meet
/// directly (i.e. is the block "collapse-through")? True when there
/// is nothing between the top and bottom edges that could separate
/// the margins: no height, no padding, no border, no element /
/// non-whitespace text children, and no min-height pinning the box
/// open.
pub(super) fn is_empty_collapse_through(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    resolved_height: u16,
) -> bool {
    if resolved_height != 0 {
        return false;
    }
    if !computed.padding.top.is_zero() || !computed.padding.bottom.is_zero() {
        return false;
    }
    if computed.border.top.is_visible() || computed.border.bottom.is_visible() {
        return false;
    }
    // An explicit `min-height: Cells(n)` with n > 0 keeps the box
    // open even when content is empty — that opens a separator
    // between top and bottom margins, so collapse-through doesn't
    // apply.
    if let Some(crate::layout::MinSize::Cells(n)) = computed.min_height
        && n > 0
    {
        return false;
    }
    // Any in-flow element child OR any non-whitespace text child
    // counts as content that separates the margins. Out-of-flow
    // children don't count (they don't take space in normal flow).
    for child in dom.node(id).child_nodes() {
        match child.node_type() {
            NodeType::Element => {
                let c = child.ext().and_then(|e| e.computed.as_ref());
                let in_flow = c
                    .map(|s| {
                        s.display != crate::layout::Display::None
                            && !matches!(
                                s.position,
                                crate::layout::Position::Absolute | crate::layout::Position::Fixed
                            )
                    })
                    .unwrap_or(true);
                if in_flow {
                    return false;
                }
            }
            NodeType::Text => {
                if let Some(t) = child.node_value()
                    && !t.chars().all(char::is_whitespace)
                {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

/// CSS 2.1 §8.3.1 — predicate for "this container's `margin-top`
/// collapses through to its first in-flow block child's
/// `margin-top`." All conditions must hold: no top padding, no top
/// border, no clearance (always true in v1 — `clear` isn't a
/// property we model), and the container doesn't establish a new
/// block formatting context.
pub(super) fn parent_collapses_top_with_first_child(parent: &ComputedStyle) -> bool {
    parent.padding.top.is_zero() && parent.border.top.is_none() && !parent.establishes_new_bfc
}

/// Symmetric to `parent_collapses_top_with_first_child` — for the
/// bottom edge.
pub(super) fn parent_collapses_bottom_with_last_child(parent: &ComputedStyle) -> bool {
    parent.padding.bottom.is_zero() && parent.border.bottom.is_none() && !parent.establishes_new_bfc
}

/// CSS 2.1 §8.3.1 vertical-margin collapse accumulator.
///
/// Maintains the running set of margins to be collapsed between two
/// block boundaries: the largest positive (or zero) and the most
/// negative (or zero). Resolution:
///
/// `final = positive_max + negative_min`
///
/// - Both positive (negative_min == 0): max.
/// - Both negative (positive_max == 0): min (most negative).
/// - Mixed: largest positive plus most negative — partial
///   cancellation per spec.
/// - Empty (initial state, zero contributions): 0.
///
/// Used by `layout_block_children` to track the unresolved margin
/// between the last placed block and the next block to be placed,
/// supporting any number of intervening empty-collapse-through
/// blocks (Phase 5.3) and absorbing the parent-child collapse
/// boundary (Phase 5.2).
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct MarginAccumulator {
    pub(super) positive_max: i16,
    pub(super) negative_min: i16,
}

impl MarginAccumulator {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn add(&mut self, margin: i16) {
        if margin > self.positive_max {
            self.positive_max = margin;
        }
        if margin < self.negative_min {
            self.negative_min = margin;
        }
    }

    pub(super) fn resolved(&self) -> i16 {
        self.positive_max + self.negative_min
    }

    /// Fold another accumulator's contributions into this one.
    /// Used by Phase 5.3 (collapse-through children fold their
    /// outer bottom into the running gap accumulator).
    pub(super) fn merge(&mut self, other: Self) {
        if other.positive_max > self.positive_max {
            self.positive_max = other.positive_max;
        }
        if other.negative_min < self.negative_min {
            self.negative_min = other.negative_min;
        }
    }
}

/// One chain result the walkers hand back for memoization:
/// `(block, containing-block width, top chain?, bottom chain?)`.
type ChainEntry = (
    NodeId,
    u16,
    Option<MarginAccumulator>,
    Option<MarginAccumulator>,
);

/// Write the walkers' chain results onto their blocks
/// (`TuiExt::margin_chain`), merging an entry for the other edge
/// computed against the same width and replacing one for another.
pub(super) fn store_margin_chain_memo(dom: &mut Dom<TuiExt>, entries: &[ChainEntry]) {
    for &(id, cb, top, bottom) in entries {
        let mut node = dom.node_mut(id);
        let Some(ext) = node.ext_mut() else {
            continue;
        };
        let mut memo = match ext.margin_chain {
            Some(m) if m.containing_block_width == cb => m,
            _ => MarginChainMemo {
                containing_block_width: cb,
                outer_top: None,
                outer_bottom: None,
            },
        };
        if let Some(t) = top {
            memo.outer_top = Some((t.positive_max, t.negative_min));
        }
        if let Some(b) = bottom {
            memo.outer_bottom = Some((b.positive_max, b.negative_min));
        }
        ext.margin_chain = Some(memo);
    }
}

/// A memoized chain of `id` for `containing_block_width`, if an
/// ancestor's placement computed it in this pass.
fn memoized_chain(
    dom: &Dom<TuiExt>,
    id: NodeId,
    containing_block_width: u16,
    pick: impl Fn(&MarginChainMemo) -> Option<(i16, i16)>,
) -> Option<MarginAccumulator> {
    let memo = dom.node(id).ext()?.margin_chain?;
    if memo.containing_block_width != containing_block_width {
        return None;
    }
    pick(&memo).map(|(positive_max, negative_min)| MarginAccumulator {
        positive_max,
        negative_min,
    })
}

/// CSS 2.1 §8.3.1 — the margins that surface at `id`'s **outer top
/// edge**: its own `margin-top` merged with the parent-first-child
/// collapse chain (its first in-flow block child, that one's first
/// block child, …, plus the margins of empty collapse-through
/// siblings), stopping at the first block that blocks the chain (top
/// padding / top border / new BFC / non-block-level first child).
/// Closes `BFC1-MARGIN-COLLAPSE-UPWARD-1`: the grandparent sees the
/// merged margin, not just `parent.margin-top`.
///
/// Every level walked is pushed onto `memo` so the caller can store it
/// (`BFC1-PERF-MARGIN-CHAIN-1`); a level whose result is already
/// memoized for this width is taken as is.
pub(super) fn outer_top_margin(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    containing_block_width: u16,
    memo: &mut Vec<ChainEntry>,
) -> MarginAccumulator {
    if let Some(acc) = memoized_chain(dom, id, containing_block_width, |m| m.outer_top) {
        return acc;
    }
    let mut acc = MarginAccumulator::new();
    acc.add(vertical_margin(
        &computed.margin.top,
        containing_block_width,
    ));
    if parent_collapses_top_with_first_child(computed) {
        // The children's margins resolve against `id`'s content width
        // (CSS 2.1 §8.3), known from its own width resolution before it
        // is laid out.
        let child_cb = block_content_width(computed, containing_block_width);
        // Walk in-flow children left-to-right. The first one that
        // contributes a top margin determines where the chain stops.
        // Empty-collapse-through children fold BOTH their margins and
        // we continue to the next sibling.
        for child in dom.node(id).child_nodes() {
            if !is_in_flow(dom, child.id()) {
                continue;
            }
            match child.node_type() {
                NodeType::Element => {
                    let child_computed = child
                        .ext()
                        .and_then(|e| e.computed.clone())
                        .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
                    use crate::layout::Display;
                    if matches!(
                        child_computed.display,
                        Display::Inline | Display::InlineBlock
                    ) {
                        // Anonymous block box wraps this inline run.
                        // Anon-box content (visible glyphs or zero-
                        // sized atom rects) sits between the parent's
                        // top and any subsequent block, blocking the
                        // chain.
                        break;
                    }
                    acc.merge(outer_top_margin(
                        dom,
                        child.id(),
                        &child_computed,
                        child_cb,
                        memo,
                    ));
                    if is_statically_empty_collapse_through(dom, child.id(), &child_computed) {
                        acc.add(vertical_margin(&child_computed.margin.bottom, child_cb));
                        continue;
                    }
                    break;
                }
                NodeType::Text => {
                    if let Some(t) = child.node_value()
                        && !t.chars().all(char::is_whitespace)
                    {
                        break; // anon-box content blocks chain
                    }
                }
                _ => {}
            }
        }
    }
    memo.push((id, containing_block_width, Some(acc), None));
    acc
}

/// Symmetric to [`outer_top_margin`] — the margins that surface at
/// `id`'s **outer bottom edge** through the last-block-child collapse
/// chain.
pub(super) fn outer_bottom_margin(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    containing_block_width: u16,
    memo: &mut Vec<ChainEntry>,
) -> MarginAccumulator {
    if let Some(acc) = memoized_chain(dom, id, containing_block_width, |m| m.outer_bottom) {
        return acc;
    }
    let mut acc = MarginAccumulator::new();
    acc.add(vertical_margin(
        &computed.margin.bottom,
        containing_block_width,
    ));
    if parent_collapses_bottom_with_last_child(computed) {
        let child_cb = block_content_width(computed, containing_block_width);
        let children: Vec<_> = dom.node(id).child_nodes().collect();
        for child in children.into_iter().rev() {
            if !is_in_flow(dom, child.id()) {
                continue;
            }
            match child.node_type() {
                NodeType::Element => {
                    let child_computed = child
                        .ext()
                        .and_then(|e| e.computed.clone())
                        .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
                    use crate::layout::Display;
                    if matches!(
                        child_computed.display,
                        Display::Inline | Display::InlineBlock
                    ) {
                        break;
                    }
                    acc.merge(outer_bottom_margin(
                        dom,
                        child.id(),
                        &child_computed,
                        child_cb,
                        memo,
                    ));
                    if is_statically_empty_collapse_through(dom, child.id(), &child_computed) {
                        acc.add(vertical_margin(&child_computed.margin.top, child_cb));
                        continue;
                    }
                    break;
                }
                NodeType::Text => {
                    if let Some(t) = child.node_value()
                        && !t.chars().all(char::is_whitespace)
                    {
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    memo.push((id, containing_block_width, None, Some(acc)));
    acc
}

/// Static (no resolved-height-required) variant of
/// `is_empty_collapse_through` for the outer-margin peek paths.
/// A box qualifies when it has no top/bottom padding, no top/
/// bottom border, no min-height pinning, declared height is
/// `Auto` or `Fixed(0)`, and no in-flow non-whitespace content.
fn is_statically_empty_collapse_through(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
) -> bool {
    if !computed.padding.top.is_zero() || !computed.padding.bottom.is_zero() {
        return false;
    }
    if computed.border.top.is_visible() || computed.border.bottom.is_visible() {
        return false;
    }
    match computed.height {
        Size::Fixed(0) | Size::Auto => {}
        _ => return false,
    }
    if let Some(crate::layout::MinSize::Cells(n)) = computed.min_height
        && n > 0
    {
        return false;
    }
    for child in dom.node(id).child_nodes() {
        match child.node_type() {
            NodeType::Element => {
                let c = child.ext().and_then(|e| e.computed.as_ref());
                let in_flow = c
                    .map(|s| {
                        s.display != crate::layout::Display::None
                            && !matches!(
                                s.position,
                                crate::layout::Position::Absolute | crate::layout::Position::Fixed
                            )
                    })
                    .unwrap_or(true);
                if in_flow {
                    return false;
                }
            }
            NodeType::Text => {
                if let Some(t) = child.node_value()
                    && !t.chars().all(char::is_whitespace)
                {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
}

/// Convert a `MarginValue` to its effective cell contribution on
/// the BLOCK axis (top/bottom). `Auto` on the block axis collapses
/// to 0 per CSS 2.1 §8.3 — only inline-axis auto margins absorb
/// leftover space; block-axis autos don't. `Calc` resolves against
/// the containing-block width (which is the basis for percent
/// margins on all four sides per CSS 2.1 §8.3).
fn vertical_margin(m: &MarginValue, cb_width: u16) -> i16 {
    match m {
        MarginValue::Auto => 0,
        MarginValue::Cells(n) => *n,
        MarginValue::Calc(_) => m.resolve(cb_width),
    }
}
