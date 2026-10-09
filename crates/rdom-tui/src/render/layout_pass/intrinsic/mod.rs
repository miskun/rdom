//! Intrinsic sizing — what size does an element want along
//! `direction`, given a `cross_budget` on the perpendicular axis?
//!
//! Used by the flex layout to resolve `Size::Auto`:
//!
//! - Text nodes → widest line (Row) / rows (Column), packed by the
//!   packer's measuring mode as layout packs them.
//! - A box's contribution (`contribution`, CSS Sizing 3 §5.2): its
//!   declared size when definite (a length, or a percentage of a known
//!   containing block width) through its `box-sizing`, else its content;
//!   either way clamped by its `min-*` / `max-*`.
//! - IFC blocks → inline content width on the Row axis (max-content:
//!   the unwrapped sum; min-content: the longest unbreakable word —
//!   CSS Sizing 3 §4.1 / §4.2); line count at `cross_budget` on the
//!   Column axis.
//! - Everything else → recursive fit of children +
//!   padding/border/gap costs.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::Direction;
use crate::node::TuiNodeExt;
use crate::style::ComputedStyle;

mod children;
mod content;
mod contribution;
mod inline;
mod keywords;
mod memo;
#[cfg(test)]
pub(in crate::render::layout_pass) mod memo_tests;
mod wrap;

use content::measure_content;
pub(crate) use keywords::Keywords;
pub(super) use memo::{
    baselines_memo, begin_pass, end_pass, put_baselines_memo, with_subgrids, with_tables,
};

/// Measure an element's intrinsic size along `direction`. Used to
/// resolve `Size::Auto`. `cross_budget` is the container's
/// perpendicular dimension — consulted by IFC blocks to decide how
/// many lines their content wraps to. `containing_block_width` is
/// the basis for the element's own percent padding and margins (CSS
/// 2.1 §8.3 / §8.4); pass 0 when it is not yet known (CSS Sizing 3
/// §5.2.1: a cyclic percentage contributes nothing to an intrinsic
/// size).
pub(crate) fn intrinsic_size(
    dom: &Dom<TuiExt>,
    id: NodeId,
    direction: Direction,
    cross_budget: u16,
    containing_block_width: u16,
) -> u16 {
    intrinsic_size_inner(
        dom,
        id,
        direction,
        cross_budget,
        containing_block_width,
        IntrinsicMode::BoxSize,
        Measure::MaxContent,
    )
}

/// Measure an element's **content** intrinsic size along `direction` —
/// the min-content size of its actual children/text (CSS Sizing 3
/// §4.2: text breaks at every opportunity), ignoring any explicit
/// `Size::Fixed` declared on the element itself. Used by CSS Flexbox
/// §4.5's "content size suggestion" half of the auto-min computation,
/// where we need to know how small the content can be regardless of
/// the box's declared size.
pub(super) fn content_min_size(
    dom: &Dom<TuiExt>,
    id: NodeId,
    direction: Direction,
    cross_budget: u16,
    containing_block_width: u16,
) -> u16 {
    intrinsic_size_inner(
        dom,
        id,
        direction,
        cross_budget,
        containing_block_width,
        IntrinsicMode::ContentOnly,
        Measure::MinContent,
    )
}

/// Measure an element's **content** max-content size along `direction`
/// (CSS Sizing 3 §5.1): its content unwrapped, plus padding and border,
/// ignoring the element's own declared size — the `max-content`
/// keyword's size, as [`content_min_size`] is `min-content`'s.
pub(crate) fn content_max_size(
    dom: &Dom<TuiExt>,
    id: NodeId,
    direction: Direction,
    cross_budget: u16,
    containing_block_width: u16,
) -> u16 {
    intrinsic_size_inner(
        dom,
        id,
        direction,
        cross_budget,
        containing_block_width,
        IntrinsicMode::ContentOnly,
        Measure::MaxContent,
    )
}

/// `id`'s intrinsic size contribution along `direction` as a border box
/// (CSS Sizing 3 §5.2): its min-content contribution, or with
/// `max_content` its max-content one — its declared size when definite,
/// else its content, either way clamped by its `min-*` / `max-*`. What a
/// container's content size sums or takes the largest of
/// (`children::children_size`), and what a grid track sizes to.
pub(crate) fn contribution(
    dom: &Dom<TuiExt>,
    id: NodeId,
    direction: Direction,
    cross_budget: u16,
    containing_block_width: u16,
    max_content: bool,
) -> u16 {
    intrinsic_size_inner(
        dom,
        id,
        direction,
        cross_budget,
        containing_block_width,
        IntrinsicMode::BoxSize,
        if max_content {
            Measure::MaxContent
        } else {
            Measure::MinContent
        },
    )
}

/// Which intrinsic size text contributes (CSS Sizing 3 §4.1 / §4.2).
/// Threaded through the recursion: a min-content measurement asks
/// every descendant for its min-content contribution.
#[derive(Copy, Clone, PartialEq, Eq)]
pub(super) enum Measure {
    /// Text unwrapped.
    MaxContent,
    /// Text broken at every soft-wrap opportunity: the widest
    /// unbreakable word (the whole text under `white-space: nowrap`
    /// / `pre`).
    MinContent,
}

impl Measure {
    /// The width inline content is packed at for this size: unbounded
    /// (no soft wrap taken) or none (every one taken).
    pub(super) fn available(self) -> u16 {
        match self {
            Measure::MaxContent => u16::MAX,
            Measure::MinContent => 0,
        }
    }
}

/// How `intrinsic_size_inner` interprets the element's declared
/// size.
///
/// Two callers in the layout pass need subtly different things:
///
/// 1. **Flex layout asking "how big does this child want to be on
///    the main axis?"** — wants the box's declared size when set
///    (`width: 30` means "I want 30"). Pick `BoxSize`. Used by
///    `Size::Auto` resolution in `layout_flex_children`'s natural-
///    size computation and by intrinsic measurement of grow-
///    children's cross-axis suggestions.
///
/// 2. **Flex layout computing CSS Flexbox §4.5 content size
///    suggestion** — wants the min-content of the actual content
///    (text + children), even when the element has a declared
///    size that's larger or smaller. Pick `ContentOnly`. Without
///    this, an empty `<a width=100 max-width=30>` would report
///    intrinsic = 100 (from the short-circuit) instead of 0
///    (its actual content), and `max-width: 30` would never get
///    a chance to clamp the box down.
///
/// Recursive descent always uses `BoxSize` for children — the
/// content-only mode only skips the short-circuit at the TOP of
/// the call stack. The auto-min rule applies to the box being
/// measured, not its descendants.
#[derive(Copy, Clone, PartialEq, Eq)]
pub(super) enum IntrinsicMode {
    /// Honor an explicit `Size::Fixed` on the element (short-
    /// circuit to that value). "Size of the box as it wants to
    /// appear in layout."
    BoxSize,
    /// Ignore any declared `Size::Fixed`; always measure children
    /// plus text. "Size of the content irrespective of the box's
    /// declaration." CSS Flexbox §4.5's content size suggestion.
    ContentOnly,
}

fn intrinsic_size_inner(
    dom: &Dom<TuiExt>,
    id: NodeId,
    direction: Direction,
    cross_budget: u16,
    containing_block_width: u16,
    mode: IntrinsicMode,
    measure: Measure,
) -> u16 {
    let kind = dom.node(id).node_type();
    match kind {
        NodeType::Text => intrinsic_text(dom, id, direction, cross_budget, measure),
        NodeType::Element | NodeType::Fragment => intrinsic_element(
            dom,
            id,
            direction,
            cross_budget,
            containing_block_width,
            mode,
            measure,
        ),
        // Comments and any later non-rendered node kind (`NodeType` is
        // `#[non_exhaustive]`): only elements and text render.
        _ => 0,
    }
}

/// A text node's size, packed alone by its parent's CSS Text values as
/// layout packs it (transformed, collapsed, at its tab stops,
/// letter-spaced): its widest line under `measure` on the Row axis, the
/// rows it packs to at `cross_budget` on the Column axis (at least one).
fn intrinsic_text(
    dom: &Dom<TuiExt>,
    id: NodeId,
    direction: Direction,
    cross_budget: u16,
    measure: Measure,
) -> u16 {
    match direction {
        Direction::Row => crate::render::inline::text_node_extent(dom, id, measure.available()).0,
        Direction::Column => crate::render::inline::text_node_extent(dom, id, cross_budget)
            .1
            .max(1),
    }
}

fn intrinsic_element(
    dom: &Dom<TuiExt>,
    id: NodeId,
    direction: Direction,
    cross_budget: u16,
    containing_block_width: u16,
    mode: IntrinsicMode,
    measure: Measure,
) -> u16 {
    let computed = dom
        .node(id)
        .computed_rc()
        .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));

    // BoxSize mode: the box's contribution — its declared size, else its
    // content, with its `min-*` / `max-*` applied (CSS Sizing 3 §5.2,
    // `contribution`). ContentOnly mode: the content alone — the CSS
    // Flexbox §4.5 "content size suggestion" needs the size of the
    // actual content, not the declared box size, so the auto-min floor
    // doesn't mistake a `width: 100` declaration for "this box must be
    // 100 cells of content" — empty boxes need to be allowed to shrink
    // toward 0 to honor a smaller `max-width`.
    if mode == IntrinsicMode::BoxSize {
        return contribution::box_contribution(
            dom,
            id,
            &computed,
            direction,
            cross_budget,
            containing_block_width,
            measure,
        );
    }
    content_size(
        dom,
        id,
        &computed,
        direction,
        cross_budget,
        containing_block_width,
        measure,
    )
}

/// The size of `id`'s content along `direction` plus its padding,
/// border and permanent scrollbar gutter, ignoring its declared size:
/// text and inline content, or its in-flow children's contributions.
fn content_size(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    direction: Direction,
    cross_budget: u16,
    containing_block_width: u16,
    measure: Measure,
) -> u16 {
    // Both axes' sizes are memoized for the layout pass (`memo`): a
    // Column measurement of an inline formatting context packs its atoms,
    // each measured on the Column axis again (its height and its
    // baseline), so unmemoized nested atoms cost a walk per level per
    // enclosing level.
    // A subgrid's size takes its parent's laid-out tracks, which are
    // written during the pass: measured each time, never memoized.
    // Size containment (CSS Containment 2 §3.1): on a contained axis the
    // content counts as nothing — `contain-intrinsic-size` where given —
    // and only the box's own padding, border and gutter remain.
    if super::containment::contains(computed, direction) {
        return contained_content_size(computed, direction, containing_block_width);
    }
    let key = (!super::grid::reads_parent_lines(computed)).then_some((
        id,
        direction == Direction::Row,
        measure == Measure::MaxContent,
        cross_budget,
        containing_block_width,
    ));
    if let Some(v) = key.and_then(|key| memo::get(dom, key)) {
        return v;
    }
    #[cfg(test)]
    match direction {
        Direction::Row => memo_tests::ROW_WALKS.with(|c| c.set(c.get() + 1)),
        Direction::Column => memo_tests::COLUMN_WALKS.with(|c| c.set(c.get() + 1)),
    }
    let value = measure_content(
        dom,
        id,
        computed,
        direction,
        cross_budget,
        containing_block_width,
        measure,
    );
    if let Some(key) = key {
        memo::put(dom, key, value);
    }
    value
}

/// A size-contained box's size on `direction`'s axis: its
/// `contain-intrinsic-*` content size plus its padding, border and
/// permanent scrollbar gutter.
fn contained_content_size(
    computed: &ComputedStyle,
    direction: Direction,
    containing_block_width: u16,
) -> u16 {
    use super::box_sizing::Sizer;
    let g = super::gutters(computed, false, false);
    let (chrome, gutter) = match direction {
        Direction::Row => (
            Sizer::horizontal(computed, containing_block_width).chrome(),
            g.columns(),
        ),
        Direction::Column => (
            Sizer::vertical(computed, containing_block_width).chrome(),
            g.bottom,
        ),
    };
    super::containment::contained_size(computed, direction)
        .saturating_add(chrome)
        .saturating_add(gutter)
}
