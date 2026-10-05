//! Inline-content measurement for intrinsic sizing: text and
//! generated-content widths, the rows inline content wraps to, and the
//! padding / border costs around them.

use rdom_core::{Dom, NodeId};
use unicode_width::UnicodeWidthStr;

use super::Keywords;
use super::Measure;
use crate::ext::TuiExt;
use crate::layout::{Direction, Size};
use crate::node::TuiNodeExt;
use crate::render::inline::compute_inline_layout;
use crate::render::layout_pass::box_sizing::Sizer;
use crate::style::ComputedStyle;

/// Rows `id`'s inline content wraps to when it is laid out at
/// `cross_budget` columns (its own `Fixed` width when it has one —
/// the border box `box-sizing` makes of it, the width it will actually
/// get), less its horizontal padding and border. Padding percentages
/// resolve against the containing block's width `cb_width` (CSS Box 3
/// §4.2), not the box's own. At least 1.
pub(super) fn wrapped_rows(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    cross_budget: u16,
    cb_width: u16,
) -> u16 {
    let sizer = Sizer::horizontal(computed, cb_width);
    let outer_width = match &computed.width {
        Size::Fixed(n) => sizer.outer(*n),
        // A keyword width (CSS Sizing 3 §3.1), `fit-content` against the
        // budget.
        Size::Intrinsic(k) => Keywords::new(dom, id, computed, Direction::Row, 0, cb_width)
            .keyword(k, Some(cross_budget), cross_budget),
        _ => cross_budget,
    };
    let content_width = outer_width.saturating_sub(sizer.chrome());
    let il = compute_inline_layout(dom, id, content_width);
    // A line-clamp container's lines end at its Nth (CSS Overflow 4 §4).
    let height = match computed.max_lines {
        Some(n) if computed.line_clamp_container => {
            crate::render::layout_pass::line_clamp::clamped_lines_height(&il, n)
        }
        _ => il.height(),
    };
    height.max(1)
}

/// Sum of visible cell widths of an element's `::before` and `::after`
/// generated content. Mirrors what `paint_pass/inline_paint.rs` writes
/// inline alongside the element's own content; without including this
/// here, an auto-width element with pseudo chrome (e.g. `<button>` with
/// bracketed `::before` / `::after`) would size to its text content
/// only and clip the pseudos at paint time.
///
/// A list marker counts where it is laid out: on the block whose first
/// line it rides (`inline::generated`), not on its `<li>`.
pub(super) fn pseudo_content_width(dom: &Dom<TuiExt>, id: NodeId) -> u16 {
    use crate::ext::StyleSlot;
    use crate::render::inline::generated;
    let width = |host: NodeId, slot: StyleSlot| -> u32 {
        let node = dom.node(host);
        let computed = match slot {
            StyleSlot::Before => node.computed_before(),
            _ => node.computed_after(),
        };
        computed
            .and_then(|c| c.content.as_deref())
            .map_or(0, |t| UnicodeWidthStr::width(t) as u32)
    };
    let mut acc: u32 = 0;
    for item in generated::deferred_markers(dom, id) {
        acc = acc.saturating_add(width(item, StyleSlot::Before));
    }
    if generated::marker_line_holder(dom, id).is_none() {
        acc = acc.saturating_add(width(id, StyleSlot::Before));
    }
    acc = acc.saturating_add(width(id, StyleSlot::After));
    acc.min(u16::MAX as u32) as u16
}

/// Rows of `id`'s pseudo-elements that take a line of their own, packed
/// at `content_width` as the block pass packs them.
pub(super) fn own_line_pseudo_rows(dom: &Dom<TuiExt>, id: NodeId, content_width: u16) -> u16 {
    use crate::render::inline::{RunPseudos, generated, pack_run};
    let own_line = generated::own_line_pseudos(dom, id);
    let rows = |pseudos: RunPseudos| pack_run(dom, id, &[], pseudos, content_width, None).height();
    let mut total = 0u16;
    if own_line.before {
        total = total.saturating_add(rows(RunPseudos {
            before: true,
            after: false,
        }));
    }
    if own_line.after {
        total = total.saturating_add(rows(RunPseudos {
            before: false,
            after: true,
        }));
    }
    total
}

/// True iff `id` has at least one text child (or, through a box-less
/// child, loose inline content) whose contents
/// contain a non-whitespace character. Pure-whitespace text between
/// element siblings is treated as ignorable in intrinsic measurement
/// (matches CSS anonymous-block-around-inline collapse for empty
/// inline runs).
pub(super) fn has_non_whitespace_text(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    // Through box-less children, whose text is `id`'s (CSS Display 3 §2.5).
    crate::render::box_tree::holds_loose_text(dom, id, &|t| !t.chars().all(char::is_whitespace))
}

pub(super) fn border_main_cost(computed: &ComputedStyle, direction: Direction) -> u16 {
    let b = computed.border;
    match direction {
        Direction::Row => b.left.cells() + b.right.cells(),
        Direction::Column => b.top.cells() + b.bottom.cells(),
    }
}

/// The min- or max-content inline width of `id`'s inline content (CSS
/// Sizing 3 §5.1): its widest line packed as layout packs it
/// (`inline::widest_line`) — its static `::before` / `::after` and every
/// atomic inline (`inline-block`, `inline-flex`, `inline-grid`, CSS
/// Display 3 §2.4) boxes in those lines, an atom its own max-content
/// width wide (C7G-INLINE-ATOM-MAX: the max-content width was the sum of
/// the text inside them).
pub(super) fn inline_width(dom: &Dom<TuiExt>, id: NodeId, measure: Measure) -> u16 {
    crate::render::inline::widest_line(dom, id, measure.available())
}
