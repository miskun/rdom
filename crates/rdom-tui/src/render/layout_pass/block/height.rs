//! CSS 2.1 §10.5 / §10.6.3 height resolution for a block-level box:
//! explicit, percentage (against a *definite* containing block only)
//! or intrinsic content height, clamped by min / max.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{Direction, Size, clamp_size};
use crate::render::layout_pass::intrinsic::Keywords;
use crate::render::layout_pass::intrinsic::intrinsic_size;
use crate::style::ComputedStyle;

/// CSS 2.1 §10.6.3 — block-level non-replaced element height. For
/// phase 2 this is the simple version: `Auto` → intrinsic content
/// height; `Fixed` → declared; `Percent` → percent of container
/// (the "percent needs definite parent" rule lands in phase 6).
///
/// Two separate budgets:
/// - `resolved_width` is the child's content width — fed to
///   `intrinsic_size` as the cross-axis budget so descendant text
///   wraps against the box that will hold it.
/// - `container_height` is the containing-block's content height —
///   used as the base for `height: <pct>%` resolution and for
///   `calc()` with percent terms.
pub(super) fn resolve_block_height(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    resolved_width: u16,
    container_height: u16,
    containing_block_width: u16,
) -> u16 {
    // CSS 2.1 §10.5 — `height: <percent>` only resolves against
    // the containing block's height when that height is *definite*
    // (Fixed, or Percent-of-definite, or otherwise pinned). When
    // the containing block's own height is `auto` (indefinite),
    // the percent falls back to `auto` — i.e. intrinsic content.
    // Same rule applies to `Calc` expressions with percent terms.
    let parent_height_definite = nearest_block_ancestor_height_is_definite(dom, id);

    // A `Fixed` height is definite; a percentage or a `calc()` (which
    // may hold one) is only when the parent's height is — otherwise it
    // falls through to intrinsic, as `auto`. A `calc()` without a
    // percentage would resolve without the basis, but treating every
    // `calc()` alike never hurts. `Flex` here means the shorthand was
    // used in a non-flex context: `auto`.
    // The declared size measures the box `box-sizing` names (CSS UI 3
    // §3.1); the sizer turns it into the border-box height stored. An
    // intrinsic keyword is the automatic size on this axis (CSS Sizing 3
    // §3.1), so it falls through to the content height like `auto`.
    let kw = Keywords::new(
        dom,
        id,
        computed,
        Direction::Column,
        resolved_width,
        containing_block_width,
    );
    let sizer = kw.sizer();
    let definite = match &computed.height {
        Size::Fixed(n) => Some(*n),
        Size::Percent(_) | Size::Calc(_) if parent_height_definite => {
            computed.height.cells(Some(container_height))
        }
        _ => None,
    }
    .map(|h| sizer.outer(h));
    // Otherwise the intrinsic content height — a walk of the child's
    // subtree. The cross budget passed to `intrinsic_size` is the WIDTH
    // descendants are laid out into (`Direction::Column` queries height;
    // the cross axis is the row): the child's own resolved width, which
    // text wraps to.
    let raw = definite.unwrap_or_else(|| {
        intrinsic_size(
            dom,
            id,
            Direction::Column,
            resolved_width,
            containing_block_width,
        )
    });

    // Clamp by min-height / max-height. Min:auto on block elements
    // resolves to 0 per CSS 2.1 (block boxes have no content-min
    // floor — that's a flex-only concept from Flexbox §4.5).
    // Percentages resolve against the containing block's height when it
    // is definite (CSS 2.1 §10.7: else `0` / `none`).
    let basis = parent_height_definite.then_some(container_height);
    let min_cells = kw.min(&computed.min_height, basis, raw);
    let max_cells = kw.max(&computed.max_height, basis, raw);
    sizer.floor(clamp_size(raw, min_cells, max_cells))
}

/// CSS 2.1 §10.5 — walk up to find the nearest block-flow ancestor
/// whose height is *definite* (resolves to a concrete value
/// independent of content measurement). Used to gate `Size::Percent`
/// height resolution.
///
/// A height is definite when:
/// - The element's `computed.height` is `Size::Fixed(_)`, OR
/// - It's `Size::Percent` or `Size::Calc` AND the ancestor chain
///   resolves to a definite ancestor (recursive), OR
/// - `min-height` pins the box to a concrete value (Cells(>0)), OR
/// - The element is absolutely-positioned with both top + bottom
///   (height is `cb_height - top - bottom`, definite by extension
///   when the CB is definite — for v1 we assume CB-of-absolute is
///   definite since it traces to the viewport).
///
/// Flex layout asks it too, for its items' `min-height` / `max-height`
/// percentages (a column's main axis, a row's cross axis): it reads the
/// container's height the same way. A flex item's definite post-flex
/// size (§9.8) is the `Size::Flex` case below.
pub(crate) fn nearest_block_ancestor_height_is_definite(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    // The box parent: a `display: contents` ancestor has no box.
    height_is_definite_below(dom, crate::render::box_tree::box_parent(dom, id))
}

/// [`nearest_block_ancestor_height_is_definite`] for a box whose box
/// parent is `parent` (`None`: the viewport) — an anonymous flex item,
/// which has no node to start from.
pub(crate) fn height_is_definite_below(dom: &Dom<TuiExt>, parent: Option<NodeId>) -> bool {
    use crate::layout::{MinSize, Position};
    // Iterative walk so a pathological `<div height="50%">` nest
    // can't blow the stack. Each step looks at the next box parent up:
    // first the box whose children's percentages are being resolved,
    // then *its* box parent, etc.
    let mut next = parent;
    loop {
        let Some(parent_id) = next else {
            // No parent: the viewport is definite by construction
            // (layout_dom passes viewport rect).
            return true;
        };
        let parent = dom.node(parent_id);
        let Some(parent_computed) = parent.ext().and_then(|e| e.computed.as_ref()) else {
            return true; // fragment root etc.
        };
        if matches!(
            parent_computed.position,
            Position::Absolute | Position::Fixed
        ) {
            // Absolute / fixed: `compute_placed_rect` sets a
            // concrete height (CB height when both top + bottom
            // are `Cells`, declared otherwise). Conservative
            // simplification: treat as definite.
            return true;
        }
        if let MinSize::Cells(n) = parent_computed.min_height
            && n > 0
        {
            return true;
        }
        match parent_computed.height {
            Size::Fixed(_) => return true,
            // Plain `auto` height tracks content → indefinite (CSS 2.1
            // §10.5) for a block-flow box. A flex item's is not: CSS
            // Flexbox §9.8 makes its post-flexing main size (a column
            // item) and its stretched cross size (a row item — rdom
            // stretches every item without an `auto` cross margin)
            // definite when its flex container's size is, so chain up
            // to the container. An intrinsic keyword is the content
            // height too (CSS Sizing 3 §3.1).
            //
            // The document root's children are items of rdom's viewport
            // column (DIVERGENCES): only one that grows has a size the
            // viewport fixes — definite, as a `<n>fr` one is below.
            Size::Auto | Size::Intrinsic(_) => {
                match crate::render::box_tree::box_parent(dom, parent_id) {
                    Some(gp) if crate::render::box_tree::is_flex_container(dom, gp) => {
                        next = crate::render::box_tree::box_parent(dom, parent_id);
                    }
                    Some(gp)
                        if dom.node(gp).node_type() == rdom_core::NodeType::Fragment
                            && parent_computed.flex_grow > 0.0 =>
                    {
                        return true;
                    }
                    _ => return false,
                }
            }
            Size::Flex(_) => {
                // `<n>fr` on the height ⇒ a growing / flexing item.
                // CSS Flexbox §9.8: a flex item in a
                // flex container with a definite main size has a
                // DEFINITE post-flexing main size, so percentages of
                // its content resolve against it — even though its own
                // `height` isn't `Fixed`. It's definite iff the flex
                // container is, so chain up and re-test the container.
                use crate::layout::Flow;
                match crate::render::box_tree::box_parent(dom, parent_id).map(|gp| dom.node(gp)) {
                    // No grandparent: `parent` is the top-level box,
                    // flexed against the viewport `layout_dom` passes
                    // in — definite.
                    None => return true,
                    Some(gp) => match gp.ext().and_then(|e| e.computed.as_ref()).map(|c| c.flow) {
                        // Fragment / document root lays children out as
                        // a definite-size column flex container (the
                        // viewport), so a flexing child is definite.
                        None => return true,
                        // Flex container: chain up to test its size.
                        Some(Flow::Flex) => {
                            next = crate::render::box_tree::box_parent(dom, parent_id)
                        }
                        // A `flex`-height value under a block-flow
                        // parent is a non-flex context (the shorthand
                        // was used outside a flex container) → treated
                        // as `auto` → indefinite.
                        Some(Flow::Block | Flow::FlowRoot) => return false,
                    },
                }
            }
            Size::Percent(_) | Size::Calc(_) => {
                // Chain up — re-test against the GRANDparent.
                next = crate::render::box_tree::box_parent(dom, parent_id);
            }
        }
    }
}
