//! `height: auto` on a block-flow element, resolved from the measured
//! content extent after its children are laid out (CSS 2.1 §10.6.3).

use rdom_core::{Dom, NodeId};

use super::block;
use super::box_sizing::Sizer;
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
    if !is_content_sized(dom, id, computed) {
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
    let content_h = used_content_height(
        dom,
        id,
        computed,
        containing_block_width,
        measurement.content_height,
    );
    let sizer = Sizer::vertical(computed, containing_block_width);
    let outer_h = content_h
        .saturating_add(sizer.chrome())
        .saturating_add(gutter_rows);
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.layout.height = outer_h;
        ext.content_layout.height = content_h;
    }
}

/// Whether `id`'s height is resolved from its content here (the gating
/// above): a block-flow box with an `auto` (or keyword) height, in
/// block flow, in flow.
pub(crate) fn is_content_sized(dom: &Dom<TuiExt>, id: NodeId, computed: &ComputedStyle) -> bool {
    let parent_is_block_flow = crate::render::box_tree::box_parent(dom, id)
        .and_then(|p| {
            use crate::node::TuiNodeExt;
            dom.node(p)
                .tui_ext()
                .and_then(|e| e.computed.as_ref().map(|c| c.flow))
        })
        .is_none_or(|f| f.is_block_flow());
    let is_out_of_flow_positioned = matches!(
        computed.position,
        crate::layout::Position::Absolute | crate::layout::Position::Fixed
    );
    // An intrinsic keyword is the automatic size on the block axis (CSS
    // Sizing 3 §3.1): resolved here like `auto`.
    matches!(
        computed.height,
        crate::layout::Size::Auto | crate::layout::Size::Intrinsic(_)
    ) && computed.flow.is_block_flow()
        && parent_is_block_flow
        && !is_out_of_flow_positioned
}

/// The content-box height of a content-sized box (`is_content_sized`)
/// whose content is `content_height` rows: the content clamped by
/// `min-height` / `max-height` (CSS 2.1 §10.7).
pub(crate) fn used_content_height(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    containing_block_width: u16,
    content_height: u16,
) -> u16 {
    // `min-height` / `max-height` percentages resolve against the
    // containing block's height when it is definite (CSS 2.1 §10.7).
    let basis = block::nearest_block_ancestor_height_is_definite(dom, id)
        .then(|| {
            use crate::node::TuiNodeExt;
            crate::render::box_tree::box_parent(dom, id)
                .and_then(|p| dom.node(p).tui_ext().map(|e| e.content_layout.height))
        })
        .flatten();
    // The clamp is on the content box: `min-height` / `max-height`
    // measure the box `box-sizing` names (CSS UI 3 §3.1), so the sizer
    // takes a border-box bound's padding and border off first. Padding
    // percent / calc resolves against the containing-block width on ALL
    // four sides (CSS 2.1 §8.4) — the same basis
    // `compute_content_area_collapsed` used for this element's inset.
    let sizer = Sizer::vertical(computed, containing_block_width);
    // A keyword bound is the content height itself (CSS Sizing 3 §3.1),
    // which the measured content already is: no clamp.
    crate::layout::clamp_size(
        content_height,
        sizer.inner_opt(computed.min_height.cells(basis)),
        sizer.inner_opt(computed.max_height.cells(basis)),
    )
}
