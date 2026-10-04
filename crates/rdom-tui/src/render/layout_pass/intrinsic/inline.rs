//! Inline-content measurement for intrinsic sizing: text and
//! generated-content widths, the rows inline content wraps to, and the
//! padding / border costs around them.

use rdom_core::{Dom, NodeId, NodeType};
use unicode_width::UnicodeWidthStr;

use super::Measure;
use crate::ext::TuiExt;
use crate::layout::{Direction, Size};
use crate::node::TuiNodeExt;
use crate::render::inline::compute_inline_layout;
use crate::style::ComputedStyle;

/// Rows `id`'s inline content wraps to when it is laid out at
/// `cross_budget` columns (its own `Fixed` width when it has one —
/// the width it will actually get), less its horizontal padding and
/// border. Padding percentages resolve against the containing block's
/// width `cb_width` (CSS Box 3 §4.2), not the box's own. At least 1.
pub(super) fn wrapped_rows(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    cross_budget: u16,
    cb_width: u16,
) -> u16 {
    let outer_width = match &computed.width {
        Size::Fixed(n) => *n,
        _ => cross_budget,
    };
    let row_pad =
        computed.padding.left.resolve(cb_width) + computed.padding.right.resolve(cb_width);
    let content_width = outer_width
        .saturating_sub(row_pad)
        .saturating_sub(border_main_cost(computed, Direction::Row));
    compute_inline_layout(dom, id, content_width)
        .height()
        .max(1)
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
    let rows = |pseudos: RunPseudos| pack_run(dom, id, &[], pseudos, content_width).height();
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
    let b = computed.border;
    match direction {
        Direction::Row => b.left.cells() + b.right.cells(),
        Direction::Column => b.top.cells() + b.bottom.cells(),
    }
}

/// Inline content width of `id` on the Row axis for `measure`.
pub(super) fn inline_width(dom: &Dom<TuiExt>, id: NodeId, measure: Measure) -> u16 {
    match measure {
        Measure::MaxContent => inline_content_width(dom, id),
        Measure::MinContent => min_content_inline_width(dom, id),
    }
}

/// Min-content inline width (CSS Sizing 3 §4.2): the widest line when
/// the content breaks at every soft-wrap opportunity. Packing at a
/// zero content width puts each unbreakable word on its own line
/// with exactly the packer's break rules (`white-space`, hyphens),
/// so this cannot drift from what layout wraps.
fn min_content_inline_width(dom: &Dom<TuiExt>, id: NodeId) -> u16 {
    compute_inline_layout(dom, id, 0)
        .lines
        .iter()
        .map(|line| line.width)
        .max()
        .unwrap_or(0)
}

/// Sum of visible cell widths of all text in an IFC block's inline
/// subtree, its inline descendants' static pseudo-elements included
/// (`id`'s own are added by the caller). Walks text nodes and descends
/// into inline element children. Used as the intrinsic max-content
/// width for IFC blocks.
fn inline_content_width(dom: &Dom<TuiExt>, id: NodeId) -> u16 {
    fn walk(dom: &Dom<TuiExt>, id: NodeId, acc: &mut u32) {
        use crate::ext::StyleSlot;
        use crate::layout::{Display, Position};
        use crate::render::inline::generated;
        for child in dom.node(id).child_nodes() {
            match child.node_type() {
                NodeType::Text => {
                    let text = child.node_value().unwrap_or("");
                    *acc = acc.saturating_add(UnicodeWidthStr::width(text) as u32);
                }
                NodeType::Element => {
                    // Out-of-flow descendants (`display: none`,
                    // `position: absolute|fixed`) generate no in-flow box
                    // and so add nothing to their ancestor's max-content
                    // inline width. Skip them — otherwise a text-leaf with
                    // an absolutely-positioned child (e.g. a chip with an
                    // absolute dropdown) inflates its intrinsic width by
                    // the hidden child's text. Mirrors the same filter in
                    // `intrinsic_element` and the IFC walk in
                    // `render::inline::walk_subtree`.
                    let (display, position) = child
                        .ext()
                        .and_then(|e| e.computed.as_ref())
                        .map(|c| (c.display, c.position))
                        .unwrap_or((Display::Block, Position::Static));
                    if display == Display::None
                        || matches!(position, Position::Absolute | Position::Fixed)
                    {
                        continue;
                    }
                    // The element's static `::before` / `::after` are its
                    // first / last inline children (CSS 2.1 §12.1), packed
                    // with the text (`render::inline::walk_inline_box`).
                    for slot in [StyleSlot::Before, StyleSlot::After] {
                        let text = generated::static_pseudo_text(dom, child.id(), slot);
                        *acc = acc
                            .saturating_add(text.map_or(0, |t| UnicodeWidthStr::width(t) as u32));
                    }
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
