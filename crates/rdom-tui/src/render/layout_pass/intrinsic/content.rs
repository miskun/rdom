//! An element's content size measured (`intrinsic::content_size` looks
//! it up first): a block container's block size by its block flow
//! (`block::measure`), an inline formatting context's or a text leaf's by
//! its lines, a flex container's by its items, a grid container's by its
//! grid, a box with element children by their contributions — with its
//! padding, border and permanent scrollbar gutter, and its block-level
//! `::before` / `::after` on its inline axis.

use rdom_core::{Dom, NodeId};

use super::inline::{
    border_main_cost, has_non_whitespace_text, inline_width, pseudo_content_width, wrapped_rows,
};
use super::{Measure, children};
use crate::ext::TuiExt;
use crate::layout::Direction;
use crate::render::layout_pass::ifc::is_ifc_block;
use crate::style::ComputedStyle;

/// [`content_size`] measured, not looked up: its flow content's, with
/// its block-level `::before` / `::after` boxes (`block::generated`)
/// stacked on its block axis. A block container's block size is its block
/// flow measured as layout lays it out (`block::measure`), those boxes in
/// it.
pub(super) fn measure_content(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    direction: Direction,
    cross_budget: u16,
    containing_block_width: u16,
    measure: Measure,
) -> u16 {
    if direction == Direction::Column
        && super::super::dispatch::children_layout(dom, id, computed)
            == super::super::dispatch::ChildrenLayout::Block
    {
        return block_content_height(dom, id, computed, cross_budget, containing_block_width);
    }
    let flow = measure_flow_content(
        dom,
        id,
        computed,
        direction,
        cross_budget,
        containing_block_width,
        measure,
    );
    if direction == Direction::Column {
        return flow;
    }
    let chrome =
        crate::render::layout_pass::box_sizing::Sizer::horizontal(computed, containing_block_width)
            .chrome();
    match crate::render::layout_pass::block::generated::intrinsic_width(
        dom,
        id,
        measure == Measure::MaxContent,
    ) {
        Some(wide) => flow.max(wide.saturating_add(chrome)),
        None => flow,
    }
}

/// The block size of the block container `id` (a border box
/// `cross_budget` wide): its flow's content height (`block::measure`, the
/// block layout's model) plus its vertical padding, border and permanent
/// horizontal scrollbar gutter.
fn block_content_height(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    cross_budget: u16,
    containing_block_width: u16,
) -> u16 {
    use crate::render::layout_pass::box_sizing::Sizer;
    let g = super::super::gutters(computed, false, false);
    let width = cross_budget
        .saturating_sub(Sizer::horizontal(computed, containing_block_width).chrome())
        .saturating_sub(g.columns());
    super::super::block::measure::content_height(dom, id, computed, width)
        .saturating_add(Sizer::vertical(computed, containing_block_width).chrome())
        .saturating_add(g.bottom)
}

/// [`measure_content`] for the host's flow content alone.
fn measure_flow_content(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    direction: Direction,
    cross_budget: u16,
    containing_block_width: u16,
    measure: Measure,
) -> u16 {
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
    let border_main = border_main_cost(computed, direction);
    // A permanent scrollbar gutter (`overflow: scroll`, `scrollbar-gutter:
    // stable`) is part of the box: the vertical bar costs a column, the
    // horizontal bar a row. An `auto` gutter that only appears on overflow
    // is settled by `layout_node`'s second pass instead.
    let g = super::super::gutters(computed, false, false);
    let gutter_main = match direction {
        Direction::Row => g.columns(),
        Direction::Column => g.bottom,
    };
    let border_main = border_main.saturating_add(gutter_main);

    // ::before / ::after generated content (Row only) — paints
    // inline alongside the children's first / last line. Contributes
    // to the row's intrinsic width sum but not to column height
    // (single-line pseudo content joins the existing inline row).
    let pseudo_main = match direction {
        Direction::Row => pseudo_content_width(dom, id, measure),
        Direction::Column => 0,
    };

    // IFC block: inline content. Width = its widest line packed at the
    // measurement's constraint (`inline_width`). Height = line count at
    // the available content width.
    if is_ifc_block(dom, id) {
        let content = match direction {
            Direction::Row => inline_width(dom, id, measure),
            Direction::Column => {
                wrapped_rows(dom, id, computed, cross_budget, containing_block_width)
            }
        };
        return content.saturating_add(pad_main).saturating_add(border_main);
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
    //
    // A flex container measures its flex items (CSS Flexbox §4): its
    // elements, its pseudo-elements and an anonymous item per run of text
    // — so its own `::before` / `::after` are no inline content beside
    // them — in order-modified document order (§5.4: its first and last
    // items, for `margin-trim`).
    if computed.flow == crate::layout::Flow::Flex {
        let mut items = super::super::items::items_of(dom, id);
        super::super::items::sort_by_order(dom, &mut items);
        let content = if items.is_empty() {
            0
        } else {
            children::children_size(
                dom,
                id,
                computed,
                &items,
                direction,
                cross_budget,
                containing_block_width,
                measure,
            )
        };
        return content.saturating_add(pad_main).saturating_add(border_main);
    }
    // A grid container's content size is its grid's (CSS Grid 2 §5.2):
    // its tracks sized under the measurement's constraint.
    if computed.flow == crate::layout::Flow::Grid {
        let content = super::super::grid::content_size(
            dom,
            id,
            computed,
            direction,
            cross_budget,
            containing_block_width,
            measure,
        );
        return content.saturating_add(pad_main).saturating_add(border_main);
    }
    let children: Vec<NodeId> = super::super::element_children_of(dom, id)
        .into_iter()
        .filter(|&c| super::super::is_in_flow(dom, c))
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
        //     `::after` plus padding/border: their widths on the Row
        //     axis, on the Column axis the rows they pack to — they are
        //     the box's content (CSS 2.1 §12.1), a line box of their own.
        if has_non_whitespace_text(dom, id) {
            let content = match direction {
                Direction::Row => inline_width(dom, id, measure),
                Direction::Column => {
                    wrapped_rows(dom, id, computed, cross_budget, containing_block_width)
                }
            };
            return content.saturating_add(pad_main).saturating_add(border_main);
        }
        let pseudo_rows = || {
            let p = crate::render::inline::generated::visible_inline_pseudos(dom, id);
            if p.before || p.after {
                wrapped_rows(dom, id, computed, cross_budget, containing_block_width)
            } else {
                0
            }
        };
        let generated = match direction {
            Direction::Row => pseudo_main,
            Direction::Column => pseudo_rows(),
        };
        return generated
            .saturating_add(pad_main)
            .saturating_add(border_main);
    }

    let with_text = children::children_size(
        dom,
        id,
        computed,
        &super::super::items::elements(&children),
        direction,
        cross_budget,
        containing_block_width,
        measure,
    );

    with_text
        .saturating_add(pseudo_main)
        .saturating_add(pad_main)
        .saturating_add(border_main)
}
