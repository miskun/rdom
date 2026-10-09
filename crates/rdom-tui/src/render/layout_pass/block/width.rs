//! CSS 2.1 §10.3.3 width resolution for a block-level box: the
//! `margin-left + width + margin-right = containing block` equation
//! with auto absorption, min / max clamping, and the content width a
//! block's children resolve percentages against.

use crate::layout::{Direction, LayoutRect, MarginValue, compute_content_area_collapsed};
use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::render::layout_pass::intrinsic::Keywords;
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
/// where `W_outer` is the child's border-box width — rdom stores outer
/// rects in `LayoutRect` — and `CB` is the containing block's content
/// width. A declared `width` / `min-width` / `max-width` measures the
/// box `box-sizing` names, or is an intrinsic keyword measured from
/// the content (CSS Sizing 3 §3.1, `fit-content` against the space the
/// margins leave); [`Keywords`] turns it into that border-box width.
///
/// Auto values absorb leftover space; over-constrained widths override
/// the inline-end margin (`margin-right` under `ltr`, `margin-left`
/// under `rtl`) to make the equation balance. The
/// content width is never negative (§10.3.3), so the border-box width
/// is at least the padding plus border, whatever the containing block.
pub(super) fn resolve_block_width(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    (containing_block_width, container_height): (u16, Option<u16>),
) -> ResolvedWidth {
    let cb = containing_block_width as i32;

    // Declared width and margins in their raw forms.
    let width_decl = &computed.width;
    let ml_decl = &computed.margin.left;
    let mr_decl = &computed.margin.right;

    // Resolve the declared width to a concrete cell count when
    // possible. `Auto` stays "needs computation" — we drive it
    // from the leftover after margins.
    // `Flex` in block context is treated as `Auto` (`Size::cells`): flex
    // factors only mean something inside a flex container, and a block
    // child of one resolves its size from `flex-basis` (0% for the
    // `flex: <N>` shorthand).

    // CSS 2.1 §10.3.3: the over-constrained equation drops the margin on
    // the containing block's inline-end side.
    let rtl = crate::render::box_tree::box_parent(dom, id).and_then(|p| {
        dom.node(p)
            .ext()
            .and_then(|e| e.computed.as_ref())
            .map(|c| c.text_direction)
    }) == Some(crate::layout::TextDirection::Rtl);
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

    // The stretch-fit width `fit-content` clamps: what the margins leave.
    let available = (cb - ml_cells - mr_cells).clamp(0, i32::from(u16::MAX)) as u16;
    let kw = Keywords::new(dom, id, computed, Direction::Row, 0, containing_block_width);
    let sizer = kw.sizer();
    let declared_width: Option<i32> = kw
        .size(width_decl, Some(containing_block_width), available)
        .map(i32::from)
        // CSS 2.1 §17.5.2.2: an `auto`-width table is as wide as its
        // content asks, within the space (shrink-to-fit).
        .or_else(|| {
            (computed.flow == crate::layout::Flow::Table).then(|| {
                i32::from(kw.keyword(
                    &crate::layout::IntrinsicSize::FitContent,
                    Some(containing_block_width),
                    available,
                ))
            })
        })
        // CSS Sizing 4 §5.1: with a preferred aspect ratio and a definite
        // height, the automatic width is the transferred size, not the
        // stretch fit.
        .or_else(|| {
            ratio_width(
                dom,
                id,
                computed,
                (containing_block_width, container_height),
            )
            .map(i32::from)
        });

    // CSS Box Alignment 3 §6.1: a `justify-self` other than `normal` /
    // `stretch` sizes an `auto` width as `fit-content` and places the box
    // in the containing block — unless an `auto` margin takes the space
    // (CSS 2.1 §10.3.3 then distributes it as before).
    let justify = super::align::justify_self_of(dom, id, computed);
    if super::align::aligns(justify) && !ml_auto && !mr_auto {
        let width = declared_width.unwrap_or_else(|| {
            i32::from(kw.keyword(
                &crate::layout::IntrinsicSize::FitContent,
                Some(containing_block_width),
                available,
            ))
        });
        let width = i32::from(sizer.floor(
            clamp_width(width, computed, cb, &kw, available).clamp(0, i32::from(u16::MAX)) as u16,
        ));
        let free = cb - ml_cells - mr_cells - width;
        let offset =
            super::align::justify_offset(justify, free, rtl, super::align::is_rtl(computed));
        return ResolvedWidth {
            margin_left: (ml_cells + offset).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
            width: width.clamp(0, i32::from(u16::MAX)) as u16,
        };
    }

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
        // Over-constrained: the margin on the containing block's
        // inline-end side is overridden so the equation balances — the
        // right one under `ltr`, the left one under `rtl`.
        (Some(w), false, false) if rtl => {
            let ml = cb - w - mr_cells;
            (ml, w, mr_cells)
        }
        (Some(w), false, false) => {
            let mr = cb - w - ml_cells;
            (ml_cells, w, mr)
        }
    };

    // Apply min/max-width clamp. CSS 2.1 §10.4: clamp the resolved
    // width by max-width first, then min-width (min wins over max).
    // After clamping, if the width changed, re-distribute the
    // leftover to whichever margins were auto.
    let clamped_width = i32::from(sizer.floor(
        clamp_width(width_final, computed, cb, &kw, available).clamp(0, i32::from(u16::MAX)) as u16,
    ));
    let ml_clamped = if clamped_width != width_final {
        let leftover = cb - clamped_width;
        // Only the left margin positions the box; under `ltr` the right
        // margin is whatever balances the equation, under `rtl` the left.
        match (ml_auto, mr_auto) {
            (true, true) => leftover.div_euclid(2),
            (true, false) => leftover - mr_cells,
            (false, false) if rtl => leftover - mr_cells,
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

/// The width `computed`'s `aspect-ratio` transfers from a definite height
/// (CSS Sizing 4 §5.1): a declared length, or a percentage or `calc()` of
/// a definite containing block's height (CSS 2.1 §10.5). `None` without a
/// ratio, for a table, or when the height is not definite.
fn ratio_width(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    (cb, container_height): (u16, Option<u16>),
) -> Option<u16> {
    use crate::layout::Size;
    let ratio = computed.aspect_ratio?;
    if computed.flow == crate::layout::Flow::Table {
        return None;
    }
    // The containing block's height, when it is definite (CSS 2.1 §10.5):
    // the one the caller lays the box out in, or — asked before layout —
    // the box parent's content height, the viewport's for a box of the
    // initial containing block (CSS 2.1 §10.1), which has no Ext.
    let basis = super::height::nearest_block_ancestor_height_is_definite(dom, id).then(|| {
        container_height.unwrap_or_else(|| {
            match crate::render::box_tree::box_parent(dom, id).and_then(|p| dom.node(p).ext()) {
                Some(e) => e.content_layout.height,
                None => crate::style::cascade::document_viewport(dom).rows,
            }
        })
    });
    let height = match &computed.height {
        Size::Fixed(n) => Some(*n),
        Size::Percent(_) | Size::Calc(_) => basis.and_then(|b| computed.height.cells(Some(b))),
        _ => None,
    }?;
    // CSS Sizing 4 §5.1: the size transferred is the definite height as
    // `min-height` / `max-height` clamp it.
    let kw = crate::render::layout_pass::intrinsic::Keywords::new(
        dom,
        id,
        computed,
        Direction::Column,
        cb,
        cb,
    );
    let sizer = kw.sizer();
    let outer = sizer.outer(height);
    let outer = sizer.floor(crate::layout::clamp_size(
        outer,
        kw.min(&computed.min_height, basis, outer),
        kw.max(&computed.max_height, basis, outer),
    ));
    crate::render::layout_pass::box_sizing::aspect_cross_from_main(
        outer,
        ratio,
        Direction::Column,
        computed,
        cb,
    )
}

/// `width` clamped by `max-width`, then `min-width` (CSS 2.1 §10.4:
/// min wins over max), their percentages resolved against the
/// containing block's width `cb` (CSS Sizing 3 §5.2), measured as
/// `box-sizing` says or from the content for a keyword (`kw`, with
/// `available` as the stretch-fit width). `min-width: auto` floors at 0
/// for a block box.
fn clamp_width(
    width: i32,
    computed: &ComputedStyle,
    cb: i32,
    kw: &Keywords<'_>,
    available: u16,
) -> i32 {
    let basis = Some(cb.clamp(0, i32::from(u16::MAX)) as u16);
    let min_cells = kw.min(&computed.min_width, basis, available);
    let max_cells = kw.max(&computed.max_width, basis, available);
    let after_max = match max_cells {
        Some(m) => width.min(i32::from(m)),
        None => width,
    };
    let clamped = after_max.max(min_cells.map_or(0, i32::from));
    // CSS 2.1 §17.5.2.2: a table is never narrower than its columns'
    // min-content widths and its captions (MIN, CAPMIN).
    if computed.flow == crate::layout::Flow::Table {
        let min = kw.keyword(&crate::layout::IntrinsicSize::MinContent, basis, available);
        return clamped.max(i32::from(min));
    }
    clamped
}

/// The content width a block's children resolve percentages against,
/// computable before the block is laid out: its used width (CSS 2.1
/// §10.3.3) minus its own padding and border.
pub(super) fn block_content_width(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    containing_block_width: u16,
) -> u16 {
    let width = resolve_block_width(dom, id, computed, (containing_block_width, None)).width;
    compute_content_area_collapsed(
        LayoutRect::new(0, 0, width, 0),
        computed.padding.clone(),
        computed.border,
        computed.border_collapse,
        containing_block_width,
    )
    .width
}
