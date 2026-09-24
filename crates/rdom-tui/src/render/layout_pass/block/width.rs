//! CSS 2.1 §10.3.3 width resolution for a block-level box: the
//! `margin-left + width + margin-right = containing block` equation
//! with auto absorption, min / max clamping, and the content width a
//! block's children resolve percentages against.

use crate::layout::{LayoutRect, MarginValue, Size, compute_content_area_collapsed};
use crate::style::ComputedStyle;

/// Result of CSS 2.1 §10.3.3 width resolution for a single block
/// child: the resolved `margin-left`, `width`, and `margin-right`
/// in cells, with auto values resolved and over-constrained cases
/// normalized.
#[derive(Debug, Clone, Copy)]
pub(super) struct ResolvedWidth {
    pub(super) margin_left: i16,
    pub(super) width: u16,
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
pub(super) fn resolve_block_width(
    computed: &ComputedStyle,
    containing_block_width: u16,
) -> ResolvedWidth {
    let cb = containing_block_width as i32;

    // Declared width and margins in their raw forms.
    let width_decl = &computed.width;
    let ml_decl = &computed.margin.left;
    let mr_decl = &computed.margin.right;

    // Resolve the declared width to a concrete cell count when
    // possible. `Auto` stays "needs computation" — we drive it
    // from the leftover after margins.
    let declared_width: Option<i32> = resolve_size_to_cells(width_decl, cb);

    let ml_auto = matches!(ml_decl, MarginValue::Auto);
    let mr_auto = matches!(mr_decl, MarginValue::Auto);
    // Resolve cells (including Calc-with-percent) against the
    // containing-block width per CSS 2.1 §8.3.
    let cb_width_u16 = containing_block_width;
    let ml_cells = if ml_auto {
        0i32
    } else {
        ml_decl.resolve(cb_width_u16) as i32
    };
    let mr_cells = if mr_auto {
        0i32
    } else {
        mr_decl.resolve(cb_width_u16) as i32
    };

    let (ml_final, width_final, _): (i32, i32, i32) = match (declared_width, ml_auto, mr_auto) {
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
    let ml_clamped = if clamped_width != width_final {
        let leftover = cb - clamped_width;
        // Only the left margin positions the box (LTR); the right
        // margin is whatever balances the equation.
        match (ml_auto, mr_auto) {
            (true, true) => leftover.div_euclid(2),
            (true, false) => leftover - mr_cells,
            (false, true) | (false, false) => ml_cells,
        }
    } else {
        ml_final
    };

    ResolvedWidth {
        margin_left: ml_clamped.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
        width: clamped_width.max(0).min(u16::MAX as i32) as u16,
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
        Size::Percent(p) => Some(Size::percent_of(basis, *p)),
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

/// The content width a block's children resolve percentages against,
/// computable before the block is laid out: its used width (CSS 2.1
/// §10.3.3) minus its own padding and border.
pub(super) fn block_content_width(computed: &ComputedStyle, containing_block_width: u16) -> u16 {
    let width = resolve_block_width(computed, containing_block_width).width;
    compute_content_area_collapsed(
        LayoutRect::new(0, 0, width, 0),
        computed.padding.clone(),
        computed.border,
        computed.border_collapse,
        containing_block_width,
    )
    .width
}
