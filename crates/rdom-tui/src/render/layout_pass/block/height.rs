//! CSS 2.1 §10.5 / §10.6.3 height resolution for a block-level box:
//! explicit, percentage (against a *definite* containing block only)
//! or intrinsic content height, clamped by min / max.

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{Direction, Size, clamp_size};
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

    let raw = match &computed.height {
        Size::Auto | Size::Flex(_) => {
            // Intrinsic content height — walk the child's subtree.
            // Block items aren't "flex items" in this pass; Flex
            // here means the shorthand was used in a non-flex
            // context, treated as Auto.
            //
            // cross_budget passed to intrinsic_size = the WIDTH
            // descendants will be laid out into (Direction::Column
            // queries height; cross axis is row/width). That's the
            // child's own resolved width — text wraps to it.
            intrinsic_size(
                dom,
                id,
                Direction::Column,
                resolved_width,
                containing_block_width,
            )
        }
        Size::Fixed(n) => *n,
        Size::Percent(p) => {
            if parent_height_definite {
                Size::percent_of(container_height as i32, *p).clamp(0, u16::MAX as i32) as u16
            } else {
                // Fall through to intrinsic — same as Auto.
                intrinsic_size(
                    dom,
                    id,
                    Direction::Column,
                    resolved_width,
                    containing_block_width,
                )
            }
        }
        Size::Calc(expr) => {
            // Calc with percent terms needs a definite basis too.
            // For simplicity, treat all Calc the same as Percent:
            // definite parent → resolve; indefinite → fall back to
            // intrinsic. Calc without percent terms still resolves
            // correctly (the basis isn't used) so the fallback path
            // never hurts.
            if parent_height_definite {
                let v = expr.resolve(&rdom_style::calc::ResolveCtx::new(container_height as i32));
                v.max(0).min(u16::MAX as i32) as u16
            } else {
                intrinsic_size(
                    dom,
                    id,
                    Direction::Column,
                    resolved_width,
                    containing_block_width,
                )
            }
        }
    };

    // Clamp by min-height / max-height. Min:auto on block elements
    // resolves to 0 per CSS 2.1 (block boxes have no content-min
    // floor — that's a flex-only concept from Flexbox §4.5).
    let min_cells: Option<u16> = match computed.min_height {
        Some(crate::layout::MinSize::Cells(n)) => Some(n),
        Some(crate::layout::MinSize::Auto) | None => None,
    };
    clamp_size(raw, min_cells, computed.max_height)
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
/// Flex items have a definite cross-axis size after distribution,
/// but in this codepath we're only consulted when walking up a
/// `Flow::Block` chain from a Block child — flex contexts are
/// outside that.
fn nearest_block_ancestor_height_is_definite(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    use crate::layout::{MinSize, Position};
    // Iterative walk so a pathological `<div height="50%">` nest
    // can't blow the stack. Each step looks at THE PARENT — `id`
    // is the descendant whose percent we're trying to resolve, so
    // the first iteration consults `id.parent`, the next consults
    // *that* parent's parent, etc.
    let mut cur = id;
    loop {
        let node = dom.node(cur);
        let Some(parent) = node.parent_node() else {
            // No parent — `cur` is root. The viewport is definite
            // by construction (layout_dom passes viewport rect).
            return true;
        };
        let parent_id = parent.id();
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
        if let Some(MinSize::Cells(n)) = parent_computed.min_height
            && n > 0
        {
            return true;
        }
        match parent_computed.height {
            Size::Fixed(_) => return true,
            // Plain `auto` height tracks content → indefinite
            // (CSS 2.1 §10.5). True for a block-flow box AND for a
            // column flex item with no grow (its main size is its
            // content). The narrower `auto`-cross-stretch case (a row
            // flex item) is conservatively left indefinite too; see
            // DIVERGENCES.md "Percentage height".
            Size::Auto => return false,
            Size::Flex(_) => {
                // `flex: …` shorthand on the height ⇒ a growing /
                // flexing item. CSS Flexbox §9.8: a flex item in a
                // flex container with a definite main size has a
                // DEFINITE post-flexing main size, so percentages of
                // its content resolve against it — even though its own
                // `height` isn't `Fixed`. It's definite iff the flex
                // container is, so chain up and re-test the container.
                use crate::layout::Flow;
                match parent.parent_node() {
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
                        Some(Flow::Flex) => cur = parent_id,
                        // A `flex`-height value under a block-flow
                        // parent is a non-flex context (the shorthand
                        // was used outside a flex container) → treated
                        // as `auto` → indefinite.
                        Some(Flow::Block) => return false,
                    },
                }
            }
            Size::Percent(_) | Size::Calc(_) => {
                // Chain up — re-test against the GRANDparent.
                cur = parent_id;
            }
        }
    }
}
