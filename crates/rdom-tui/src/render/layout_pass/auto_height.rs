//! `height: auto` on a block-flow element, resolved from the measured
//! content extent after its children are laid out (CSS 2.1 §10.6.3).

use rdom_core::{Dom, NodeId};

use super::block;
use crate::ext::TuiExt;
use crate::style::ComputedStyle;

/// CSS 2.1 §10.6.3: resolve `height: Auto` on a block-flow element
/// from the measured content extent. `gutter_rows` is what the
/// scrollbar reservation took off the content area's height; it
/// belongs to the box, so the outer height counts it.
///
/// Gating:
/// - the element's own `flow == Block` (a flex container's height is
///   already final from its parent's distribution / declared size);
/// - the parent's `flow` is also `Block` — Auto height on a flex
///   *item* means "stretch to the cross axis" (CSS Flexbox §7.5), and
///   the parent's flex pass already wrote that height;
/// - no explicit `Fixed` / `Percent` / `Calc` height;
/// - not `absolute` / `fixed`: `compute_placed_rect` owns that height
///   (auto there means "derive from `top` / `bottom` against the
///   containing block").
pub(crate) fn resolve_auto_height(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    containing_block_width: u16,
    measurement: Option<block::BlockMeasurement>,
    gutter_rows: u16,
) {
    let parent_is_block_flow = dom
        .node(id)
        .parent_node()
        .and_then(|p| {
            use crate::node::TuiNodeExt;
            p.tui_ext()
                .and_then(|e| e.computed.as_ref().map(|c| c.flow))
        })
        .map(|f| matches!(f, crate::layout::Flow::Block))
        .unwrap_or(true);
    let is_out_of_flow_positioned = matches!(
        computed.position,
        crate::layout::Position::Absolute | crate::layout::Position::Fixed
    );
    if !matches!(computed.height, crate::layout::Size::Auto)
        || !matches!(computed.flow, crate::layout::Flow::Block)
        || !parent_is_block_flow
        || is_out_of_flow_positioned
    {
        return;
    }
    // An IFC block or pure-text leaf has no `BlockMeasurement`; its
    // content extent is its packed line count (at least the one row
    // an empty editing host keeps for the caret).
    let Some(measurement) = measurement.or_else(|| {
        dom.node(id)
            .ext()
            .and_then(|e| e.inline_layout.as_ref())
            .map(|il| block::BlockMeasurement {
                content_height: il.height().max(1),
            })
    }) else {
        return;
    };
    // `min-height` / `max-height` percentages resolve against the
    // containing block's height when it is definite (CSS 2.1 §10.7).
    let basis = block::nearest_block_ancestor_height_is_definite(dom, id)
        .then(|| {
            use crate::node::TuiNodeExt;
            dom.node(id)
                .parent_node()
                .and_then(|p| p.tui_ext().map(|e| e.content_layout.height))
        })
        .flatten();
    let content_h = crate::layout::clamp_size(
        measurement.content_height,
        match &computed.min_height {
            Some(m @ crate::layout::MinSize::Calc(_))
            | Some(m @ crate::layout::MinSize::Cells(_)) => m.cells(basis),
            _ => None,
        },
        computed.max_height.as_ref().and_then(|m| m.cells(basis)),
    );
    // Padding percent / calc resolves against the containing-block
    // width on ALL four sides (CSS 2.1 §8.4) — the same basis
    // `compute_content_area_collapsed` used for this element's inset.
    let pad = computed.padding.top.resolve(containing_block_width)
        + computed.padding.bottom.resolve(containing_block_width);
    let border = computed.border.top.cells() + computed.border.bottom.cells();
    let outer_h = content_h
        .saturating_add(pad)
        .saturating_add(border)
        .saturating_add(gutter_rows);
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.layout.height = outer_h;
        ext.content_layout.height = content_h;
    }
}
