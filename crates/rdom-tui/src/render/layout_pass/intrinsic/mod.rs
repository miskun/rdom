//! Intrinsic sizing — what size does an element want along
//! `direction`, given a `cross_budget` on the perpendicular axis?
//!
//! Used by the flex layout to resolve `Size::Auto`:
//!
//! - Text nodes → widest line (Row) / line count (Column), via
//!   `unicode-width`.
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
use unicode_width::UnicodeWidthStr;

use crate::ext::TuiExt;
use crate::layout::Direction;
use crate::node::TuiNodeExt;
use crate::style::ComputedStyle;

mod children;
mod contribution;
mod inline;
mod keywords;
mod memo;
#[cfg(test)]
pub(in crate::render::layout_pass) mod memo_tests;
mod wrap;

use super::ifc::is_ifc_block;
use inline::{
    border_main_cost, has_non_whitespace_text, inline_width, pseudo_content_width, wrapped_rows,
};
pub(crate) use keywords::Keywords;
pub(super) use memo::{begin_pass, end_pass};

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
pub(in crate::render::layout_pass) fn contribution(
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
    let key = (
        id,
        direction == Direction::Row,
        measure == Measure::MaxContent,
        cross_budget,
        containing_block_width,
    );
    if let Some(v) = memo::get(dom, key) {
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
    memo::put(dom, key, value);
    value
}

/// [`content_size`] measured, not looked up.
fn measure_content(
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
    let (gutter_col, gutter_row) = super::gutter_axes(computed, false, false);
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
                wrapped_rows(dom, id, computed, cross_budget, containing_block_width)
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
    //
    // A flex container measures its flex items (CSS Flexbox §4): its
    // elements, its pseudo-elements and an anonymous item per run of text
    // — so its own `::before` / `::after` are no inline content beside
    // them — in order-modified document order (§5.4: its first and last
    // items, for `margin-trim`).
    if computed.flow == crate::layout::Flow::Flex {
        let mut items = super::items::items_of(dom, id);
        super::items::sort_by_order(dom, &mut items);
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
        let content = super::grid::content_size(
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
            return content
                .saturating_add(pseudo_beside_inline)
                .saturating_add(pad_main)
                .saturating_add(border_main);
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
        &super::items::elements(&children),
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
