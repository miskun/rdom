//! CSS 2.1 §8.3.1 vertical margin collapsing for block flow: the
//! accumulator that merges adjoining margins, the parent–child and
//! collapse-through predicates, and the outer-margin chain walkers
//! that surface a nested first / last child's margin at its
//! ancestor's edge (each level resolving percentages against its own
//! containing block).

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::{MarginChainMemo, TuiExt};
use crate::layout::{MarginValue, Size};
use crate::render::inline::generated::{inline_content_at_edge, own_line_pseudos};
use crate::render::layout_pass::margin_trim::trimmed_edges;
use crate::style::ComputedStyle;

use super::super::is_in_flow;
use super::width::block_content_width;

/// CSS 2.1 §8.3.1 — does this block's top and bottom margins meet
/// directly (i.e. is the block "collapse-through")? True when there
/// is nothing between the top and bottom edges that could separate
/// the margins: no height, no padding, no border, no element /
/// non-whitespace text children, and no min-height pinning the box
/// open. Takes the resolved height (placement knows it).
pub(super) fn is_empty_collapse_through(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    resolved_height: u16,
) -> bool {
    resolved_height == 0 && is_collapse_through_shape(dom, id, computed)
}

/// [`is_empty_collapse_through`] before the height is resolved (the
/// outer-margin chain walkers peek ahead): the declared height must be
/// `auto` or `0`. A `height: 0%` / `calc()` box therefore reads as
/// non-empty here and empty at placement — the walk stops one level
/// early, which only costs a re-walk, never a wrong margin.
fn is_statically_empty_collapse_through(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
) -> bool {
    matches!(
        computed.height,
        Size::Fixed(0) | Size::Auto | Size::Intrinsic(_)
    ) && is_collapse_through_shape(dom, id, computed)
}

/// The height-independent half of collapse-through: no vertical
/// padding or border, no `min-height` pinning, and no in-flow content
/// (an in-flow element child, a non-whitespace text child or a visible
/// `::before` / `::after` — a line box, CSS 2.1 §8.3.1 — separates the
/// margins; out-of-flow children take no space in normal flow).
fn is_collapse_through_shape(dom: &Dom<TuiExt>, id: NodeId, computed: &ComputedStyle) -> bool {
    if !computed.padding.top.is_zero() || !computed.padding.bottom.is_zero() {
        return false;
    }
    if computed.border.top.is_visible() || computed.border.bottom.is_visible() {
        return false;
    }
    if let crate::layout::MinSize::Cells(n) = computed.min_height
        && n > 0
    {
        return false;
    }
    let pseudos = crate::render::inline::generated::visible_inline_pseudos(dom, id);
    if pseudos.before || pseudos.after {
        return false;
    }
    for item in crate::render::box_tree::box_sequence(dom, id) {
        // A generated item of a box-less child is a line's content.
        let Some(child) = item.node() else {
            return false;
        };
        let child = dom.node(child);
        match child.node_type() {
            NodeType::Element if is_in_flow(dom, child.id()) => return false,
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
/// border, the child has no clearance ([`first_child_has_clearance`]),
/// the container doesn't establish a new
/// block formatting context, and no line box comes first — neither
/// inline content ahead of the first block child (text, an inline
/// box: an anonymous block with a line, CSS 2.1 §9.2.1.1) nor a
/// `::before` on a line of its own separates the margins.
pub(super) fn parent_collapses_top_with_first_child(
    dom: &Dom<TuiExt>,
    id: NodeId,
    parent: &ComputedStyle,
) -> bool {
    parent.padding.top.is_zero()
        && parent.border.top.is_none()
        // A trimmed margin is gone (CSS Box 4 §3): nothing collapses out.
        && !trimmed_edges(parent).top
        && !establishes_independent_formatting_context(dom, id, parent)
        && !inline_content_at_edge(dom, id, false)
        && !own_line_pseudos(dom, id).before
        // A block-level `::before` keeps its margins inside the host
        // (DIVERGENCES §2).
        && !crate::render::inline::generated::block_pseudos(dom, id).before
        && !first_child_has_clearance(dom, id)
}

/// §8.3.1's "the child has no clearance", decided as §9.5.2 asks from the
/// child's hypothetical position — with its top margin collapsed through
/// `id`, at `id`'s own top. A float ahead of it among `id`'s children — an
/// element or a floated `::before` — is placed at that top too, so a first
/// in-flow block child whose `clear` names the side of such a float has
/// clearance. Decided only where a float may be (`may_hold_floats`, which
/// builds nothing): most blocks hold none. (A float from outside `id` that
/// reaches below `id`'s top is not weighed: DIVERGENCES, margin
/// collapsing.)
fn first_child_has_clearance(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    use crate::layout::FloatSide;
    use crate::render::box_tree::BoxItem;
    use crate::render::layout_pass::float::{clear_sides, float_side_of};
    if !crate::render::layout_pass::float::measure::may_hold_floats(dom, id) {
        return false;
    }
    #[cfg(test)]
    super::CLEARANCE_SCANS.with(|c| c.set(c.get() + 1));
    let (mut left, mut right) = (false, false);
    for item in crate::render::box_tree::box_sequence(dom, id) {
        match float_side_of(dom, item) {
            Some(FloatSide::Left) => left = true,
            Some(FloatSide::Right) => right = true,
            // A block-level `::before` comes first: it is no float and
            // holds no `clear` of the child's.
            None if matches!(item, BoxItem::Generated(..)) => return false,
            None => {
                let Some(child) = item.node() else {
                    return false;
                };
                if dom.node(child).node_type() != NodeType::Element || !is_in_flow(dom, child) {
                    continue;
                }
                let Some(c) = dom.node(child).ext().and_then(|e| e.computed.as_deref()) else {
                    return false;
                };
                let (clears_left, clears_right) = clear_sides(dom, child, c);
                return (clears_left && left) || (clears_right && right);
            }
        }
    }
    false
}

/// Symmetric to `parent_collapses_top_with_first_child` — for the
/// bottom edge (inline content after the last block child, or an
/// `::after` on a line of its own, separates).
pub(super) fn parent_collapses_bottom_with_last_child(
    dom: &Dom<TuiExt>,
    id: NodeId,
    parent: &ComputedStyle,
) -> bool {
    parent.padding.bottom.is_zero()
        && parent.border.bottom.is_none()
        && !trimmed_edges(parent).bottom
        && !establishes_independent_formatting_context(dom, id, parent)
        && !inline_content_at_edge(dom, id, true)
        && !own_line_pseudos(dom, id).after
        && !crate::render::inline::generated::block_pseudos(dom, id).after
}

/// Does `id` establish an independent formatting context for its
/// children, so their margins never collapse with its own? Either its
/// own style makes it one (`establishes_new_bfc`: flex or grid container,
/// inline-block, non-visible overflow, absolute / fixed — CSS 2.1
/// §9.4.1), or its place in the tree does:
///
/// - the root (CSS 2.1 §8.3.1: "margins of the root element's box do
///   not collapse"; §9.4.1: the root element establishes a BFC) — an
///   element root, or a root fragment, the initial containing block,
///   whose children are its block flow (`layout_pass::icb`): their
///   margins collapse with each other's and their children's, as
///   `<body>`'s children's do, but not through the ICB;
/// - a flex or grid item (Flexbox §4: "A flex item establishes an
///   independent formatting context for its contents", Grid 2 §6.1 the
///   same of a grid item).
///
/// Fragments between an element and its layout parent are transparent
/// (`element_children_of` unwraps them).
pub(in crate::render::layout_pass) fn establishes_independent_formatting_context(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
) -> bool {
    if computed.establishes_new_bfc || id == dom.root() {
        return true;
    }
    let up = |p: rdom_core::NodeRef<'_, TuiExt>| {
        crate::render::box_tree::slot::parent(dom, p.id()).map(|n| dom.node(n))
    };
    let mut parent = up(dom.node(id));
    while let Some(p) = parent {
        match p.node_type() {
            // The initial containing block lays its children out in block
            // flow (`layout_pass::icb`).
            NodeType::Fragment if p.id() == dom.root() => return false,
            NodeType::Fragment => parent = up(p),
            // A box-less element is transparent too (CSS Display 3 §2.5).
            NodeType::Element if crate::render::box_tree::is_contents(dom, p.id()) => {
                parent = up(p)
            }
            _ => {
                return p
                    .ext()
                    .and_then(|e| e.computed.as_ref())
                    .is_some_and(|c| c.flow.is_flex_or_grid());
            }
        }
    }
    false
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
pub(super) type ChainEntry = (
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

/// Which outer edge a chain walk surfaces margins at.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Edge {
    Top,
    Bottom,
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
    outer_edge_margin(dom, id, computed, containing_block_width, memo, Edge::Top)
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
    outer_edge_margin(
        dom,
        id,
        computed,
        containing_block_width,
        memo,
        Edge::Bottom,
    )
}

fn outer_edge_margin(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    containing_block_width: u16,
    memo: &mut Vec<ChainEntry>,
    edge: Edge,
) -> MarginAccumulator {
    let pick = |m: &MarginChainMemo| match edge {
        Edge::Top => m.outer_top,
        Edge::Bottom => m.outer_bottom,
    };
    if let Some(acc) = memoized_chain(dom, id, containing_block_width, pick) {
        return acc;
    }
    let (own, collapses) = match edge {
        Edge::Top => (
            &computed.margin.top,
            parent_collapses_top_with_first_child(dom, id, computed),
        ),
        Edge::Bottom => (
            &computed.margin.bottom,
            parent_collapses_bottom_with_last_child(dom, id, computed),
        ),
    };
    let mut acc = MarginAccumulator::new();
    acc.add(vertical_margin(own, containing_block_width));
    if collapses {
        // The children's margins resolve against `id`'s content width
        // (CSS 2.1 §8.3), known from its own width resolution before it
        // is laid out.
        let child_cb = block_content_width(dom, id, computed, containing_block_width);
        // Walk the in-flow children from the edge inward. The first one
        // that contributes a margin at that edge determines where the
        // chain stops; empty collapse-through children fold BOTH their
        // margins and the walk continues to the next sibling.
        // In box-tree order: a box-less child holding a block box gives
        // its own children (CSS Display 3 §2.5).
        let children = crate::render::box_tree::box_sequence(dom, id);
        let ordered: Box<dyn Iterator<Item = _>> = match edge {
            Edge::Top => Box::new(children.into_iter()),
            Edge::Bottom => Box::new(children.into_iter().rev()),
        };
        for item in ordered {
            // A floated `::before` / `::after` is out of flow, as a
            // floated element is (CSS 2.1 §9.5); any other generated item
            // holds a line or a box, which blocks the chain.
            let Some(child) = item.node() else {
                if crate::render::layout_pass::float::float_side_of(dom, item).is_some() {
                    continue;
                }
                break;
            };
            let child = dom.node(child);
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
                        Display::Inline | Display::InlineBlock | Display::Contents
                    ) {
                        // An anonymous block box wraps this inline run;
                        // its content (visible glyphs or zero-sized atom
                        // rects) blocks the chain.
                        break;
                    }
                    acc.merge(outer_edge_margin(
                        dom,
                        child.id(),
                        &child_computed,
                        child_cb,
                        memo,
                        edge,
                    ));
                    if is_statically_empty_collapse_through(dom, child.id(), &child_computed) {
                        let other = match edge {
                            Edge::Top => &child_computed.margin.bottom,
                            Edge::Bottom => &child_computed.margin.top,
                        };
                        acc.add(vertical_margin(other, child_cb));
                        continue;
                    }
                    break;
                }
                NodeType::Text => {
                    if let Some(t) = child.node_value()
                        && !t.chars().all(char::is_whitespace)
                    {
                        break; // anon-box content blocks the chain
                    }
                }
                _ => {}
            }
        }
    }
    let (top, bottom) = match edge {
        Edge::Top => (Some(acc), None),
        Edge::Bottom => (None, Some(acc)),
    };
    memo.push((id, containing_block_width, top, bottom));
    acc
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

/// Debug check for the memo invariant: every block a chain walk
/// visited is laid out later in the same pass and clears its entry,
/// so nothing survives `layout_dom`.
#[cfg(debug_assertions)]
pub(crate) fn debug_assert_no_margin_chain_memo(dom: &Dom<TuiExt>, id: NodeId) {
    if let Some(ext) = dom.node(id).ext() {
        debug_assert!(
            ext.margin_chain.is_none(),
            "margin-chain memo survived the layout pass on {id:?}"
        );
    }
    for child in crate::render::box_tree::children(dom, id) {
        debug_assert_no_margin_chain_memo(dom, child);
    }
}
