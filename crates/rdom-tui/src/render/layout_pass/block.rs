//! Block layout pass — CSS 2.1 §10 normal flow.
//!
//! Given a block container (`flow: Block`) and its in-flow children,
//! stacks the children vertically in document order at their natural
//! heights. No distribution, no shrink-to-fit; container overflows
//! below its content box if children don't fit.
//!
//! Width resolution follows CSS 2.1 §10.3.3 — the seven-term sum
//! `margin-left + border-left + padding-left + width + padding-right
//! + border-right + margin-right` must equal the containing-block
//! width. Auto margins absorb leftover horizontal space (the
//! `margin: 0 auto` centering pattern).
//!
//! Height: each child takes its declared `height` (`Fixed`), its
//! resolved percentage (`Percent`), or its intrinsic content height
//! (`Auto`). Min/max clamping applies after computing the size.
//!
//! **Scope of phase 2** (this module, first commit):
//! - Width formula + auto margins + min/max clamp.
//! - Plain vertical stacking (no margin collapse — phase 5).
//! - No anonymous box generation around inline-level children — phase 3.
//! - No dispatch from `layout_children` — phase 4 wires it in.
//! - Strict percent-height-needs-definite-parent — phase 6 will
//!   tighten this; for now percent resolves against the container.
//!
//! The function is exposed via `pub(super)` so phase 4's dispatch
//! in `mod.rs` can call it; phases 2 & 3 ship it dormant.
//!
//! `#![allow(dead_code)]` because phases 2 & 3 build infrastructure
//! that's not on the live dispatch path yet. The dead-code lint
//! returns automatically once phase 4 wires `layout_children`'s
//! `match flow` arm.

#![allow(dead_code)]

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::{
    Direction, LayoutRect, MarginValue, Size, clamp_size, compute_content_area_collapsed,
};
use crate::node::TuiNodeExt;
use crate::render::layout_pass::intrinsic::intrinsic_size;
use crate::style::ComputedStyle;

use super::{element_children_of, layout_node};

/// Lay out `id`'s in-flow element children as block-level boxes
/// stacked vertically in `container`. `parent_computed` is the
/// container's cascaded style (not used directly for sizing yet —
/// kept for parity with `layout_flex_children` and for the
/// margin-collapse pass in phase 5).
pub(super) fn layout_block_children(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    container: LayoutRect,
    _parent_computed: &ComputedStyle,
) {
    // In-flow element children. `position: absolute` / `fixed` are
    // skipped (the positioning pass places them); `display: none`
    // takes no space.
    let children: Vec<NodeId> = element_children_of(dom, id)
        .into_iter()
        .filter(|&c| is_in_flow(dom, c))
        .collect();
    if children.is_empty() {
        return;
    }

    let containing_block_width = container.width;
    // Cursor walks the y axis as we place children. Adjacent
    // margins simply add for now (phase 5 introduces collapse).
    let mut y_cursor: i32 = container.y;

    for &child in &children {
        let computed = dom
            .node(child)
            .computed()
            .cloned()
            .unwrap_or_else(ComputedStyle::initial);

        // Width side per CSS 2.1 §10.3.3.
        let resolved = resolve_block_width(&computed, containing_block_width);

        // Height side. Auto = intrinsic content; Fixed/Percent/Calc
        // = resolved; min/max clamping last. The "percent height
        // needs definite parent" CSS rule is deferred to phase 6;
        // for now percent resolves against the container's height,
        // which matches the prior flex behavior.
        let height = resolve_block_height(dom, child, &computed, container.height);

        // Vertical margins. Negative-cell margins are allowed in CSS
        // and rdom carries them as `i16`. The cursor advances by
        // (top_margin + height + bottom_margin); the child's own
        // outer rect doesn't include its margins.
        let top_margin = vertical_margin(&computed.margin.top);
        let bottom_margin = vertical_margin(&computed.margin.bottom);

        let outer_x = container.x + resolved.margin_left as i32;
        let outer_y = y_cursor + top_margin as i32;
        let outer_rect = LayoutRect::new(outer_x, outer_y, resolved.width, height);

        // Lay out the child — `layout_node` writes its rect, recurses
        // into descendants, and handles padding/border insets.
        layout_node(dom, child, outer_rect);

        // Advance: outer rect's bottom + bottom margin.
        y_cursor = outer_rect.bottom() + bottom_margin as i32;
    }
}

/// True iff the child participates in normal flow. Mirrors the
/// filter used by `layout_flex_children` (will be DRY'd via the
/// pending `is_in_flow` helper in `DRY-1`).
fn is_in_flow(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let Some(c) = dom.node(id).ext().and_then(|e| e.computed.as_ref()) else {
        return true;
    };
    use crate::layout::{Display, Position};
    c.display != Display::None && !matches!(c.position, Position::Absolute | Position::Fixed)
}

/// Result of CSS 2.1 §10.3.3 width resolution for a single block
/// child: the resolved `margin-left`, `width`, and `margin-right`
/// in cells, with auto values resolved and over-constrained cases
/// normalized.
#[derive(Debug, Clone, Copy)]
struct ResolvedWidth {
    margin_left: i16,
    width: u16,
    #[allow(dead_code)] // phase 2: cursor doesn't use the right margin
    // because horizontal block placement is left-anchored; phase 5
    // (margin collapse) doesn't touch horizontal margins either.
    // Kept on the struct for symmetry + future use (right-anchored
    // direction support).
    margin_right: i16,
}

/// CSS 2.1 §10.3.3 — "Block-level, non-replaced elements in normal
/// flow." Resolves the width-and-horizontal-margin equation:
///
/// ```text
///   ML + W_outer + MR = CB
/// ```
///
/// where `W_outer` is the child's border-box width (the
/// `Size::Fixed(N)` value used here, NOT the CSS-strict content
/// width — rdom stores outer rects in `LayoutRect`, matching the
/// flex layout pass's convention). `CB` is the containing block's
/// content width.
///
/// Auto values absorb leftover space; over-constrained widths
/// (LTR) override `margin-right` to make the equation balance.
///
/// **Divergence note.** CSS 2.1 strict defines `width` as the
/// content-box size, with padding + border added on top. rdom
/// follows the flex pass's `width = outer` convention so authors
/// see one definition of `width` across both layout modes.
/// Documented in `DIVERGENCES.md` under "Values" — `box-sizing:
/// border-box` is the implicit default.
fn resolve_block_width(computed: &ComputedStyle, containing_block_width: u16) -> ResolvedWidth {
    let cb = containing_block_width as i32;

    // Declared width and margins in their raw forms.
    let width_decl = &computed.width;
    let ml_decl = computed.margin.left;
    let mr_decl = computed.margin.right;

    // Resolve the declared width to a concrete cell count when
    // possible. `Auto` stays "needs computation" — we drive it
    // from the leftover after margins.
    let declared_width: Option<i32> = resolve_size_to_cells(width_decl, cb);

    let ml_auto = matches!(ml_decl, MarginValue::Auto);
    let mr_auto = matches!(mr_decl, MarginValue::Auto);
    let ml_cells = match ml_decl {
        MarginValue::Auto => 0i32,
        MarginValue::Cells(n) => n as i32,
    };
    let mr_cells = match mr_decl {
        MarginValue::Auto => 0i32,
        MarginValue::Cells(n) => n as i32,
    };

    let (ml_final, width_final, mr_final): (i32, i32, i32) =
        match (declared_width, ml_auto, mr_auto) {
            // Width auto — any auto margins resolve to 0; width absorbs
            // leftover. (Note: width here is outer/border-box, NOT
            // CSS-strict content width.)
            (None, _, _) => {
                let w = cb - ml_cells - mr_cells;
                (ml_cells, w.max(0), mr_cells)
            }
            // Width fixed, both margins auto → center.
            (Some(w), true, true) => {
                let leftover = cb - w;
                let half = leftover.div_euclid(2);
                // The odd cell goes to the right margin — matches the
                // common browser behavior for odd-leftover centering.
                (half, w, leftover - half)
            }
            // Width fixed, only ML auto → ML absorbs leftover.
            (Some(w), true, false) => {
                let ml = cb - w - mr_cells;
                (ml, w, mr_cells)
            }
            // Width fixed, only MR auto → MR absorbs leftover.
            (Some(w), false, true) => {
                let mr = cb - w - ml_cells;
                (ml_cells, w, mr)
            }
            // Over-constrained (LTR): the declared MR is silently
            // overridden so the equation balances.
            (Some(w), false, false) => {
                let mr = cb - w - ml_cells;
                (ml_cells, w, mr)
            }
        };

    // Apply min/max-width clamp. CSS 2.1 §10.4: clamp the resolved
    // width by max-width first, then min-width (min wins over max).
    // After clamping, if the width changed, re-distribute the
    // leftover to whichever margins were auto.
    let clamped_width = clamp_width(width_final, &computed.min_width, computed.max_width, cb);
    let (ml_clamped, mr_clamped) = if clamped_width != width_final {
        let leftover = cb - clamped_width;
        match (ml_auto, mr_auto) {
            (true, true) => {
                let half = leftover.div_euclid(2);
                (half, leftover - half)
            }
            (true, false) => (leftover - mr_cells, mr_cells),
            (false, true) => (ml_cells, leftover - ml_cells),
            (false, false) => (ml_cells, leftover - ml_cells),
        }
    } else {
        (ml_final, mr_final)
    };

    ResolvedWidth {
        margin_left: ml_clamped.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        width: clamped_width.max(0).min(u16::MAX as i32) as u16,
        margin_right: mr_clamped.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
    }
}

/// Resolve a `Size` to a definite cell count when possible. Returns
/// `None` for `Size::Auto` (caller chooses the fallback). `Flex`
/// in block context is treated as `Auto` — flex factors only mean
/// something inside a flex container, and the spec says block-level
/// flex children resolve their main size from `flex-basis` (which
/// for the `flex: <N>` shorthand is 0%).
fn resolve_size_to_cells(size: &Size, basis: i32) -> Option<i32> {
    match size {
        Size::Auto | Size::Flex(_) => None,
        Size::Fixed(n) => Some(*n as i32),
        Size::Percent(p) => Some((basis * *p as i32) / 100),
        Size::Calc(expr) => {
            let v = expr.resolve(&rdom_style::calc::ResolveCtx::new(basis));
            Some(v)
        }
    }
}

fn clamp_width(
    width: i32,
    min: &Option<crate::layout::MinSize>,
    max: Option<u16>,
    _basis: i32, // reserved for percent-min/max in a later phase
) -> i32 {
    let min_cells: Option<i32> = match min {
        Some(crate::layout::MinSize::Cells(n)) => Some(*n as i32),
        Some(crate::layout::MinSize::Auto) | None => None, // phase 2: Auto floors are spec-correctly 0 for block; phase 5/6 may revisit
    };
    let max_cells = max.map(|n| n as i32);
    let after_max = match max_cells {
        Some(m) => width.min(m),
        None => width,
    };
    match min_cells {
        Some(m) => after_max.max(m),
        None => after_max.max(0),
    }
}

/// CSS 2.1 §10.6.3 — block-level non-replaced element height. For
/// phase 2 this is the simple version: `Auto` → intrinsic content
/// height; `Fixed` → declared; `Percent` → percent of container
/// (the "percent needs definite parent" rule lands in phase 6).
fn resolve_block_height(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    container_height: u16,
) -> u16 {
    let raw = match &computed.height {
        Size::Auto | Size::Flex(_) => {
            // Intrinsic content height — walk the child's subtree.
            // Block items aren't "flex items" in this pass; Flex
            // here means the shorthand was used in a non-flex
            // context, treated as Auto.
            intrinsic_size(dom, id, Direction::Column, container_height)
        }
        Size::Fixed(n) => *n,
        Size::Percent(p) => {
            ((container_height as u32 * *p as u32) / 100).min(u16::MAX as u32) as u16
        }
        Size::Calc(expr) => {
            let v = expr.resolve(&rdom_style::calc::ResolveCtx::new(container_height as i32));
            v.max(0).min(u16::MAX as i32) as u16
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

/// Convert a `MarginValue` to its effective cell contribution on
/// the BLOCK axis (top/bottom). `Auto` on the block axis collapses
/// to 0 per CSS 2.1 §8.3 — only inline-axis auto margins absorb
/// leftover space; block-axis autos don't.
fn vertical_margin(m: &MarginValue) -> i16 {
    match m {
        MarginValue::Auto => 0,
        MarginValue::Cells(n) => *n,
    }
}

/// Convenience helper for `compute_content_area_collapsed` style
/// access — currently unused in this module but retained for
/// symmetry with `flex.rs` and to give phase 5 a single import.
#[allow(dead_code)]
fn content_area(outer: LayoutRect, computed: &ComputedStyle) -> LayoutRect {
    compute_content_area_collapsed(
        outer,
        computed.padding,
        computed.border,
        computed.border_collapse,
    )
}
