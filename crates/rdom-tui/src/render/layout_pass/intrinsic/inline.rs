//! Inline-content measurement for intrinsic sizing: text and
//! generated-content widths, the rows inline content wraps to, and the
//! padding / border costs around them.

use rdom_core::{Dom, NodeId};

use super::Keywords;
use super::Measure;
use crate::ext::TuiExt;
use crate::layout::{Direction, Size};
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
    // Its lines beside its own floats, and the floats' rows (CSS 2.1
    // §10.6.7: a root sized from its content reaches its lowest float).
    let (il, floats) =
        crate::render::layout_pass::float::measure::inline_rows(dom, id, content_width);
    // A line-clamp container's lines end at its Nth (CSS Overflow 4 §4).
    let height = match computed.max_lines {
        Some(n) if computed.line_clamp_container => {
            crate::render::layout_pass::line_clamp::clamped_lines_height(&il, n)
        }
        _ => il.height().max(floats),
    };
    height.max(1)
}

/// Sum of visible cell widths of an element's `::before` and `::after`
/// generated content under `measure`. Mirrors what `paint_pass/inline_paint.rs` writes
/// inline alongside the element's own content; without including this
/// here, an auto-width element with pseudo chrome (e.g. `<button>` with
/// bracketed `::before` / `::after`) would size to its text content
/// only and clip the pseudos at paint time.
///
/// A list marker counts where it is laid out, on the block whose first
/// line it rides (`inline::markers`), packed as layout packs it — an
/// inside one as text, an outside one taking no room (C10-LIST-ITEM).
pub(super) fn pseudo_content_width(dom: &Dom<TuiExt>, id: NodeId, measure: Measure) -> u16 {
    use crate::ext::{PseudoSlot, StyleSlot};
    use crate::render::inline::generated;
    // The host's own inline pseudo-elements: their text (with the markers
    // riding this line, which the packer takes in with the `::before`), an
    // atom's box (its max-content width), a float's margin box — none for
    // a block-level or `display: none` one.
    let own = |slot: StyleSlot, pslot: PseudoSlot| -> u32 {
        match generated::inline_pseudo(dom, id, slot) {
            // Packed as layout packs it: transformed, collapsed, at its
            // tab stops, letter-spaced (C9G-MISC-CORRECTNESS).
            Some(generated::InlinePseudo::Text(_)) => u32::from(
                crate::render::inline::widest_pseudo_line(dom, id, pslot, measure.available()),
            ),
            Some(generated::InlinePseudo::Atom) => {
                crate::render::layout_pass::generated_atoms::measure(dom, id, pslot, 0, Some(true))
                    .map_or(0, |(w, _)| u32::from(w))
            }
            Some(generated::InlinePseudo::Float) => {
                u32::from(crate::render::layout_pass::float::size::outer_contribution(
                    dom,
                    crate::render::box_tree::BoxItem::Generated(id, pslot),
                    true,
                ))
            }
            None => 0,
        }
    };
    let before_is_text = matches!(
        generated::inline_pseudo(dom, id, StyleSlot::Before),
        Some(generated::InlinePseudo::Text(_))
    );
    let markers = if before_is_text {
        0
    } else {
        u32::from(crate::render::inline::widest_marker_line(
            dom,
            id,
            measure.available(),
        ))
    };
    let acc = markers
        .saturating_add(own(StyleSlot::Before, PseudoSlot::Before))
        .saturating_add(own(StyleSlot::After, PseudoSlot::After));
    acc.min(u16::MAX as u32) as u16
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
/// Display 3 §2.4) boxes in those lines, an atom its contribution under
/// the same constraint wide (C7G-INLINE-ATOM-MAX, C8G-FLOAT-MEASURE).
pub(super) fn inline_width(dom: &Dom<TuiExt>, id: NodeId, measure: Measure) -> u16 {
    crate::render::inline::widest_line(dom, id, measure.available())
}
