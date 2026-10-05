//! Intrinsic sizing — what size does an element want along
//! `direction`, given a `cross_budget` on the perpendicular axis?
//!
//! Used by the flex layout to resolve `Size::Auto`:
//!
//! - Text nodes → widest line (Row) / line count (Column), via
//!   `unicode-width`.
//! - Elements with explicit `Size::Fixed(n)` → `n` (short-circuit).
//! - IFC blocks → inline content width on the Row axis (max-content:
//!   the unwrapped sum; min-content: the longest unbreakable word —
//!   CSS Sizing 3 §4.1 / §4.2); line count at `cross_budget` on the
//!   Column axis.
//! - Everything else → recursive fit of children +
//!   padding/border/gap costs.

use rdom_core::{Dom, NodeId, NodeType};
use unicode_width::UnicodeWidthStr;

use crate::ext::TuiExt;
use crate::layout::{Direction, Size};
use crate::node::TuiNodeExt;
use crate::style::ComputedStyle;

mod inline;

use super::box_sizing::Sizer;
use super::ifc::is_ifc_block;
use inline::{
    border_main_cost, has_non_whitespace_text, inline_width, own_line_pseudo_rows,
    pseudo_content_width, wrapped_rows,
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
        NodeType::Text => intrinsic_text(dom, id, direction),
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

fn intrinsic_text(dom: &Dom<TuiExt>, id: NodeId, direction: Direction) -> u16 {
    let text = dom.text_content(id);
    match direction {
        Direction::Row => {
            // Widest line (in case text has newlines).
            text.lines()
                .map(|line| UnicodeWidthStr::width(line) as u16)
                .max()
                .unwrap_or(0)
        }
        Direction::Column => text.lines().count().max(1) as u16,
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

    // BoxSize mode: if the element has an explicit Fixed size along
    // `direction`, that wins over child measurement — matches CSS
    // min-content + explicit width.
    //
    // ContentOnly mode: skip the short-circuit. The CSS Flexbox §4.5
    // "content size suggestion" needs the size of the actual content,
    // not the declared box size, so the auto-min floor doesn't
    // mistake a `width: 100` declaration for "this box must be 100
    // cells of content" — empty boxes need to be allowed to shrink
    // toward 0 to honor a smaller `max-width`.
    if mode == IntrinsicMode::BoxSize {
        // TABLE-COLSYNC-1: a table cell's resolved column width is its used
        // main size (a width → Row axis), so a table measures to its laid-out
        // column widths (e.g. when it's a flex item being sized by a scroll
        // wrapper) — same short-circuit as an explicit `Fixed`.
        if direction == Direction::Row
            && let Some(w) = dom.node(id).ext().and_then(|e| e.table_used_width)
        {
            return w;
        }
        let declared = match direction {
            Direction::Row => &computed.width,
            Direction::Column => &computed.height,
        };
        // A declared size measures the box `box-sizing` names (CSS UI
        // 3 §3.1): the contribution is the border box it makes.
        if let Size::Fixed(n) = declared {
            return Sizer::along(&computed, direction, containing_block_width).outer(*n);
        }
    }

    // Padding + border cost on the main axis. Padding-with-percent
    // resolves against the containing-block width on BOTH axes per
    // CSS 2.1 §8.4. Calc that mixes percent with cells resolves at
    // this point in the layout pass; constant calcs were resolved at
    // parse time.
    let cb_w_for_pad = containing_block_width;
    let pad_main = match direction {
        Direction::Row => computed.padding.horizontal(cb_w_for_pad),
        Direction::Column => computed.padding.vertical(cb_w_for_pad),
    };
    let border_main = border_main_cost(&computed, direction);
    // A permanent scrollbar gutter (`overflow: scroll`, `scrollbar-gutter:
    // stable`) is part of the box: the vertical bar costs a column, the
    // horizontal bar a row. An `auto` gutter that only appears on overflow
    // is settled by `layout_node`'s second pass instead.
    let (gutter_col, gutter_row) = super::gutter_axes(&computed, false, false);
    let gutter_main = match direction {
        Direction::Row => u16::from(gutter_col),
        Direction::Column => u16::from(gutter_row),
    };
    let border_main = border_main.saturating_add(gutter_main);

    // ::before / ::after generated content (Row only) — paints
    // inline alongside the children's first / last line. Contributes
    // to the row's intrinsic width sum but not to column height
    // (single-line pseudo content joins the existing inline row).
    let pseudo_main = match direction {
        Direction::Row => pseudo_content_width(dom, id),
        Direction::Column => 0,
    };
    // For an inline flow of its own (IFC block, pure-text leaf) the
    // packer lays the static pseudos out with the text, so a
    // min-content pack already holds them; the max-content sum of text
    // widths does not.
    let pseudo_beside_inline = match measure {
        Measure::MinContent => 0,
        Measure::MaxContent => pseudo_main,
    };

    // IFC block: inline content. Width = max-content (unwrapped sum
    // of text widths). Height = line count at the available content
    // width.
    if is_ifc_block(dom, id) {
        let content = match direction {
            Direction::Row => inline_width(dom, id, measure),
            Direction::Column => {
                wrapped_rows(dom, id, &computed, cross_budget, containing_block_width)
            }
        };
        return content
            .saturating_add(pseudo_beside_inline)
            .saturating_add(pad_main)
            .saturating_add(border_main);
    }

    // Recursive fit of children. We walk **element** children only —
    // the same set `flex::layout_children` actually distributes
    // space over. Walking all child_nodes would double-count text
    // between block siblings (a single `"\n    "` between two
    // `<card>` items would summate as 2 rows on the Column axis),
    // making intrinsic measurement disagree with layout about what
    // counts as a "child."
    //
    // CSS 2.1 §9.3 + §9.5: out-of-flow children (`display: none`,
    // `position: absolute|fixed`) take no space in their parent's
    // in-flow content extent. They MUST be filtered here too —
    // otherwise the parent's intrinsic includes hidden / floated
    // content that doesn't actually occupy any cells, inflating
    // it. Specifically caught the chrome bug where a closed
    // `<details>` element's hidden `<pre>` body inflated the
    // intrinsic from ~1 row (summary) to ~15, starving the
    // sibling `flex: 1` panel of its share of the main axis.
    let children: Vec<NodeId> = super::element_children_of(dom, id)
        .into_iter()
        .filter(|&c| super::is_in_flow(dom, c))
        .collect();

    if children.is_empty() {
        // No element children. Two cases:
        //
        // (a) The element has non-whitespace text content (e.g.
        //     `<note>only text</note>`). It's not IFC per the
        //     predicate (see `ifc.rs` for why pure-text blocks stay
        //     non-IFC: paint routing for `::before`/`::after`), but
        //     its intrinsic main-axis size still depends on how the
        //     text wraps. Measure via `compute_inline_layout` at
        //     `cross_budget` so wrap is respected — matching what
        //     paint sees when `paint_inline_content` renders the
        //     same text.
        //
        // (b) No text (or whitespace-only text). Just `::before` /
        //     `::after` chrome on the Row axis plus padding/border.
        if has_non_whitespace_text(dom, id) {
            let content = match direction {
                Direction::Row => inline_width(dom, id, measure),
                Direction::Column => {
                    wrapped_rows(dom, id, &computed, cross_budget, containing_block_width)
                }
            };
            return content
                .saturating_add(pseudo_beside_inline)
                .saturating_add(pad_main)
                .saturating_add(border_main);
        }
        return pseudo_main
            .saturating_add(pad_main)
            .saturating_add(border_main);
    }

    // Cross budget to forward to children. They'll be laid out
    // inside our content area; for IFC measurement at the child
    // level this is what determines wrap.
    // Padding percentages resolve against the containing block's width
    // on both axes (CSS Box 3 §4.2), not against the budget.
    let child_cross_budget = match direction {
        Direction::Row => cross_budget.saturating_sub(
            computed
                .padding
                .vertical(cb_w_for_pad)
                .saturating_add(border_main_cost(&computed, Direction::Column)),
        ),
        Direction::Column => cross_budget.saturating_sub(
            computed
                .padding
                .horizontal(cb_w_for_pad)
                .saturating_add(border_main_cost(&computed, Direction::Row)),
        ),
    };

    // The children's containing block is this element's content box.
    // Measuring along the Row axis, that width is the very thing being
    // computed — a cyclic percentage, which CSS Sizing 3 §5.2.1 treats
    // as zero for the intrinsic contribution. Along the Column axis it
    // is the content width the children will wrap to.
    let child_cb_width = match direction {
        Direction::Row => 0,
        Direction::Column => child_cross_budget,
    };

    // Flexbox §9.9 / §4.5: an item's contribution is its outer size —
    // margins on the queried axis included.
    let outer = |c: NodeId| {
        let inner = intrinsic_size_inner(
            dom,
            c,
            direction,
            child_cross_budget,
            child_cb_width,
            IntrinsicMode::BoxSize,
            measure,
        );
        let margins = dom
            .node(c)
            .ext()
            .and_then(|e| e.computed.as_ref())
            .map(|cs| {
                let (a, b) = match direction {
                    Direction::Row => (&cs.margin.left, &cs.margin.right),
                    Direction::Column => (&cs.margin.top, &cs.margin.bottom),
                };
                i32::from(a.resolve(child_cb_width)) + i32::from(b.resolve(child_cb_width))
            })
            .unwrap_or(0);
        (i32::from(inner) + margins).clamp(0, i32::from(u16::MAX)) as u16
    };
    let intrinsic_children: u16 = if computed.direction == direction {
        // Children flow along the queried axis — sum their outer main
        // sizes plus gaps. Intrinsic sizing has no container size:
        // percent gaps are 0.
        let gap_total = computed
            .gap
            .resolve(0)
            .saturating_mul((children.len() as u16).saturating_sub(1));
        let children_main: u16 = children
            .iter()
            .map(|&c| outer(c))
            .fold(0u16, |acc, n| acc.saturating_add(n));
        children_main.saturating_add(gap_total)
    } else {
        // Children stack across the queried axis — the largest outer size.
        children.iter().map(|&c| outer(c)).max().unwrap_or(0)
    };

    // Direct text runs next to element children become anonymous
    // block boxes (CSS 2.1 §9.2.1.1): a row each on the Column axis
    // (unwrapped estimate; block layout measures the real wrap), the
    // widest run on the Row axis.
    let text_runs = dom
        .node(id)
        .child_nodes()
        .filter(|c| {
            c.node_type() == NodeType::Text
                && c.node_value()
                    .is_some_and(|t| !t.chars().all(char::is_whitespace))
        })
        .map(|c| intrinsic_text(dom, c.id(), direction));
    let with_text = match direction {
        Direction::Column => text_runs.fold(intrinsic_children, |acc, n| acc.saturating_add(n)),
        Direction::Row => text_runs.fold(intrinsic_children, |acc, n| acc.max(n)),
    };
    // A `::before` / `::after` beside a block-level edge child is a line
    // box of its own (CSS 2.1 §9.2.1.1) — its rows add on the Column axis.
    let with_text = match direction {
        Direction::Column => {
            with_text.saturating_add(own_line_pseudo_rows(dom, id, child_cross_budget))
        }
        Direction::Row => with_text,
    };

    with_text
        .saturating_add(pseudo_main)
        .saturating_add(pad_main)
        .saturating_add(border_main)
}
