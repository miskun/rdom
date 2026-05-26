//! Intrinsic sizing — what size does an element want along
//! `direction`, given a `cross_budget` on the perpendicular axis?
//!
//! Used by the flex layout to resolve `Size::Auto`:
//!
//! - Text nodes → widest line (Row) / line count (Column), via
//!   `unicode-width`.
//! - Elements with explicit `Size::Fixed(n)` → `n` (short-circuit).
//! - IFC blocks → inline content width (unwrapped sum) on the Row
//!   axis; line count at `cross_budget` on the Column axis.
//! - Everything else → recursive fit of children +
//!   padding/border/gap costs.

use rdom_core::{Dom, NodeId, NodeType};
use unicode_width::UnicodeWidthStr;

use crate::ext::TuiExt;
use crate::layout::{Border, Direction, Size};
use crate::node::TuiNodeExt;
use crate::render::inline::compute_inline_layout;
use crate::style::ComputedStyle;

use super::ifc::is_ifc_block;

/// Measure an element's intrinsic size along `direction`. Used to
/// resolve `Size::Auto`. `cross_budget` is the container's
/// perpendicular dimension — consulted by IFC blocks to decide how
/// many lines their content wraps to.
pub(crate) fn intrinsic_size(
    dom: &Dom<TuiExt>,
    id: NodeId,
    direction: Direction,
    cross_budget: u16,
) -> u16 {
    intrinsic_size_inner(dom, id, direction, cross_budget, IntrinsicMode::BoxSize)
}

/// Measure an element's **content** intrinsic size along `direction` —
/// i.e. the min-content size of its actual children/text, ignoring
/// any explicit `Size::Fixed` declared on the element itself. Used
/// by CSS Flexbox §4.5's "content size suggestion" half of the
/// auto-min computation, where we need to know how small the content
/// can be regardless of the box's declared size.
pub(super) fn content_min_size(
    dom: &Dom<TuiExt>,
    id: NodeId,
    direction: Direction,
    cross_budget: u16,
) -> u16 {
    intrinsic_size_inner(dom, id, direction, cross_budget, IntrinsicMode::ContentOnly)
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
    mode: IntrinsicMode,
) -> u16 {
    let kind = dom.node(id).node_type();
    match kind {
        NodeType::Text => intrinsic_text(dom, id, direction),
        NodeType::Element | NodeType::Fragment => {
            intrinsic_element(dom, id, direction, cross_budget, mode)
        }
        NodeType::Comment => 0,
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
    mode: IntrinsicMode,
) -> u16 {
    let computed = dom
        .node(id)
        .computed()
        .cloned()
        .unwrap_or_else(ComputedStyle::initial);

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
        let declared = match direction {
            Direction::Row => &computed.width,
            Direction::Column => &computed.height,
        };
        if let Size::Fixed(n) = declared {
            return *n;
        }
    }

    // Padding + border cost on the main axis.
    let pad_main = match direction {
        Direction::Row => computed.padding.left + computed.padding.right,
        Direction::Column => computed.padding.top + computed.padding.bottom,
    };
    let border_main = border_main_cost(&computed, direction);

    // ::before / ::after generated content (Row only) — paints
    // inline alongside the children's first / last line. Contributes
    // to the row's intrinsic width sum but not to column height
    // (single-line pseudo content joins the existing inline row).
    let pseudo_main = match direction {
        Direction::Row => pseudo_content_width(dom, id),
        Direction::Column => 0,
    };

    // IFC block: inline content. Width = max-content (unwrapped sum
    // of text widths). Height = line count at the available content
    // width.
    if is_ifc_block(dom, id) {
        let content = match direction {
            Direction::Row => inline_content_width(dom, id),
            Direction::Column => {
                // We're asked for height given cross_budget (= width
                // the container can give us). Subtract this block's
                // own padding/border on the width axis to get the
                // content width inline layout should pack against.
                //
                // BUT: if the block has an explicit Fixed width,
                // THAT's the width we'll actually be laid out at —
                // not the container's cross_budget. Respect it, or
                // height will be computed for a wrong width and the
                // wrap-count will lie. Caught by hit-test tests
                // that set narrow Fixed widths on IFC blocks.
                let outer_width = match &computed.width {
                    Size::Fixed(n) => *n,
                    _ => cross_budget,
                };
                let row_pad = computed.padding.left + computed.padding.right;
                let row_border = border_main_cost(&computed, Direction::Row);
                let content_width = outer_width
                    .saturating_sub(row_pad)
                    .saturating_sub(row_border);
                let layout = compute_inline_layout(dom, id, content_width);
                layout.height().max(1)
            }
        };
        return content
            .saturating_add(pseudo_main)
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
    use crate::layout::{Display, Position};
    let children: Vec<NodeId> = super::element_children_of(dom, id)
        .into_iter()
        .filter(|&c| {
            let c_computed = dom.node(c).ext().and_then(|e| e.computed.as_ref());
            match c_computed {
                Some(s) => {
                    s.display != Display::None
                        && !matches!(s.position, Position::Absolute | Position::Fixed)
                }
                None => true,
            }
        })
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
                Direction::Row => inline_content_width(dom, id),
                Direction::Column => {
                    let outer_width = match &computed.width {
                        Size::Fixed(n) => *n,
                        _ => cross_budget,
                    };
                    let row_pad = computed.padding.left + computed.padding.right;
                    let row_border = border_main_cost(&computed, Direction::Row);
                    let content_width = outer_width
                        .saturating_sub(row_pad)
                        .saturating_sub(row_border);
                    let layout = compute_inline_layout(dom, id, content_width);
                    layout.height().max(1)
                }
            };
            return content
                .saturating_add(pseudo_main)
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
    let child_cross_budget = match direction {
        Direction::Row => cross_budget.saturating_sub(
            (computed.padding.top + computed.padding.bottom)
                .saturating_add(border_main_cost(&computed, Direction::Column)),
        ),
        Direction::Column => cross_budget.saturating_sub(
            (computed.padding.left + computed.padding.right)
                .saturating_add(border_main_cost(&computed, Direction::Row)),
        ),
    };

    let intrinsic_children: u16 = if computed.direction == direction {
        // Children flow along the queried axis — sum their main
        // sizes plus gaps.
        let gap_total = computed
            .gap
            .saturating_mul((children.len() as u16).saturating_sub(1));
        let children_main: u16 = children
            .iter()
            .map(|&c| intrinsic_size(dom, c, direction, child_cross_budget))
            .fold(0u16, |acc, n| acc.saturating_add(n));
        children_main.saturating_add(gap_total)
    } else {
        // Children stack across the queried axis — take the max.
        children
            .iter()
            .map(|&c| intrinsic_size(dom, c, direction, child_cross_budget))
            .max()
            .unwrap_or(0)
    };

    intrinsic_children
        .saturating_add(pseudo_main)
        .saturating_add(pad_main)
        .saturating_add(border_main)
}

/// Sum of visible cell widths of an element's `::before` and `::after`
/// generated content. Mirrors what `paint_pass/inline_paint.rs` writes
/// inline alongside the element's own content; without including this
/// here, an auto-width element with pseudo chrome (e.g. `<button>` with
/// bracketed `::before` / `::after`) would size to its text content
/// only and clip the pseudos at paint time.
fn pseudo_content_width(dom: &Dom<TuiExt>, id: NodeId) -> u16 {
    let mut acc: u32 = 0;
    if let Some(before) = dom.node(id).computed_before()
        && let Some(text) = before.content.as_deref()
    {
        acc = acc.saturating_add(UnicodeWidthStr::width(text) as u32);
    }
    if let Some(after) = dom.node(id).computed_after()
        && let Some(text) = after.content.as_deref()
    {
        acc = acc.saturating_add(UnicodeWidthStr::width(text) as u32);
    }
    acc.min(u16::MAX as u32) as u16
}

/// True iff `id` has at least one direct text child whose contents
/// contain a non-whitespace character. Pure-whitespace text between
/// element siblings is treated as ignorable in intrinsic measurement
/// (matches CSS anonymous-block-around-inline collapse for empty
/// inline runs).
pub(super) fn has_non_whitespace_text(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    for child in dom.node(id).child_nodes() {
        if child.node_type() == NodeType::Text
            && let Some(text) = child.node_value()
            && !text.chars().all(char::is_whitespace)
        {
            return true;
        }
    }
    false
}

pub(super) fn border_main_cost(computed: &ComputedStyle, direction: Direction) -> u16 {
    match (direction, computed.border) {
        (Direction::Row, Border::Left | Border::Right) => 1,
        (Direction::Column, Border::Top | Border::Bottom) => 1,
        (_, Border::Single | Border::Rounded) => 2,
        _ => 0,
    }
}

/// Sum of visible cell widths of all text in an IFC block's inline
/// subtree. Walks text nodes and descends into inline element
/// children. Used as the intrinsic max-content width for IFC blocks.
pub(super) fn inline_content_width(dom: &Dom<TuiExt>, id: NodeId) -> u16 {
    fn walk(dom: &Dom<TuiExt>, id: NodeId, acc: &mut u32) {
        for child in dom.node(id).child_nodes() {
            match child.node_type() {
                NodeType::Text => {
                    let text = child.node_value().unwrap_or("");
                    *acc = acc.saturating_add(UnicodeWidthStr::width(text) as u32);
                }
                NodeType::Element => {
                    walk(dom, child.id(), acc);
                }
                _ => {}
            }
        }
    }
    let mut acc: u32 = 0;
    walk(dom, id, &mut acc);
    acc.min(u16::MAX as u32) as u16
}
