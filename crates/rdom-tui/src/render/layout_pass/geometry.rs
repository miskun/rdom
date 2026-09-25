//! Box geometry for the layout pass: the content area children lay out
//! in (border + padding inset, `border-collapse` aware), the padding
//! box (the scrollport), and CSS min / max clamping.
//!
//! These apply rdom-style's layout values to rectangles; they moved
//! here from `rdom_style::layout` (`P6G-LAYOUT-RS-SPLIT-1`) because
//! the layout pass and the runtime's hit / scroll geometry are their
//! only callers. Public at `rdom_tui::layout::*`.

use crate::layout::{Border, BorderCollapse, LayoutRect, Padding};

/// Compute the inner "content" rectangle that children lay out in, given
/// an outer rectangle plus the element's padding and border. Shrinks the
/// outer rect by both. Use [`compute_content_area_collapsed`] when an
/// element under `border-collapse: collapse` needs the border-overlap
/// special case (M5.5b).
pub fn compute_content_area(area: LayoutRect, padding: Padding, border: Border) -> LayoutRect {
    compute_content_area_collapsed(area, padding, border, BorderCollapse::Separate, area.width)
}

/// Same as [`compute_content_area`] but aware of `border-collapse`.
/// Under `BorderCollapse::Collapse`, when the element has a border,
/// the parent's content area **includes** its own border-ring cells —
/// children's outer edges coincide with the parent's border cells.
/// Padding still insets normally.
///
/// This is decision 2 from the M5 pre-prep: concentrate the box-model
/// special case in this one function so every other layout consumer
/// stays unchanged.
///
/// `containing_block_width` is the basis for percent / `calc()`
/// padding on all four sides (CSS 2.1 §8.4). [`compute_content_area`]
/// passes the element's own width, which is right only when the two
/// coincide; layout passes the real containing block.
pub fn compute_content_area_collapsed(
    area: LayoutRect,
    padding: Padding,
    border: Border,
    collapse: BorderCollapse,
    containing_block_width: u16,
) -> LayoutRect {
    // Collapse + border present → the parent's border ring is shared
    // with children's outer edges. Treat the parent as having no
    // border for content-area purposes; the border still paints
    // (paint pass renders it), but children's rects extend into
    // those cells.
    let effective_border = if collapse == BorderCollapse::Collapse && !border.is_empty() {
        Border::none()
    } else {
        border
    };
    let border_left = effective_border.left.cells();
    let border_top = effective_border.top.cells();
    let border_h = border_left + effective_border.right.cells();
    let border_v = border_top + effective_border.bottom.cells();

    // Percent / calc padding resolves against the containing-block
    // width on ALL four sides (CSS 2.1 §8.4 — vertical padding
    // percent also uses width).
    let cb_w = containing_block_width;
    let pad_l = padding.left.resolve(cb_w);
    let pad_r = padding.right.resolve(cb_w);
    let pad_t = padding.top.resolve(cb_w);
    let pad_b = padding.bottom.resolve(cb_w);

    let inset_x = pad_l + border_left;
    let inset_y = pad_t + border_top;
    let total_h = pad_l + pad_r + border_h;
    let total_v = pad_t + pad_b + border_v;

    LayoutRect::new(
        area.x + inset_x as i32,
        area.y + inset_y as i32,
        area.width.saturating_sub(total_h),
        area.height.saturating_sub(total_v),
    )
}

/// Compute the **padding-box** edge for an element with `outer` (border-box)
/// rect and the given `border`. This is the CSS Box Model 3 §1 padding edge:
/// `border-box ∸ border` on each side.
///
/// CSS Overflow 3 §3 names this rect the **scrollport** of a scroll container:
/// the region inside which overflow content is clipped, where the scrollbar
/// gutter lives, and what `position: sticky` pins against.
///
/// Independent of `border-collapse`. The M5.5b layout-time expansion in
/// [`compute_content_area_collapsed`] widens `content_layout` into the border
/// ring so children with their own borders can position on the shared edge —
/// that's a *child-positioning* concern, not a paint-clipping one. CSS Overflow
/// 3 §3 places the scrollport at the padding-box for every scroll container,
/// no table/collapse exception, so paint clipping reads this rect even when
/// the layout-side content rect was expanded.
///
/// Saturating math throughout — a degenerate `outer` smaller than the border
/// insets yields a zero-size rect at the inset origin, never a panic.
pub fn compute_padding_box(outer: LayoutRect, border: Border) -> LayoutRect {
    let bl = border.left.cells();
    let bt = border.top.cells();
    let bh = bl + border.right.cells();
    let bv = bt + border.bottom.cells();
    LayoutRect::new(
        outer.x + bl as i32,
        outer.y + bt as i32,
        outer.width.saturating_sub(bh),
        outer.height.saturating_sub(bv),
    )
}

/// Clamp a cell count to min/max constraints. Matches CSS: when `min >
/// max`, `min` wins.
pub fn clamp_size(value: u16, min: Option<u16>, max: Option<u16>) -> u16 {
    let mut result = value;
    if let Some(max) = max {
        result = result.min(max);
    }
    if let Some(min) = min {
        result = result.max(min);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::BorderStyle;

    // ── compute_content_area ─────────────────────────────────────────

    #[test]
    fn content_area_none_padding_no_border() {
        let area = LayoutRect::new(0, 0, 80, 24);
        assert_eq!(
            compute_content_area(area, Padding::default(), Border::none()),
            area
        );
    }

    #[test]
    fn content_area_with_padding() {
        let area = LayoutRect::new(0, 0, 80, 24);
        let content = compute_content_area(area, Padding::symmetric(2, 1), Border::none());
        assert_eq!(content, LayoutRect::new(2, 1, 76, 22));
    }

    #[test]
    fn content_area_with_asymmetric_padding() {
        let area = LayoutRect::new(0, 0, 80, 24);
        // top=1, right=3, bottom=2, left=5 → x+5, y+1, w-8, h-3
        let content = compute_content_area(area, Padding::new(1, 3, 2, 5), Border::none());
        assert_eq!(content, LayoutRect::new(5, 1, 72, 21));
    }

    #[test]
    fn content_area_with_single_border() {
        let area = LayoutRect::new(0, 0, 80, 24);
        let content = compute_content_area(area, Padding::default(), Border::single());
        assert_eq!(content, LayoutRect::new(1, 1, 78, 22));
    }

    #[test]
    fn content_area_with_top_only_border() {
        let area = LayoutRect::new(0, 0, 80, 24);
        let content = compute_content_area(area, Padding::default(), Border::top());
        // border_top=1, border_left=0, border_v=1, border_h=0
        assert_eq!(content, LayoutRect::new(0, 1, 80, 23));
    }

    // ── clamp_size ───────────────────────────────────────────────────

    #[test]
    fn clamp_within_range_unchanged() {
        assert_eq!(clamp_size(10, Some(0), Some(20)), 10);
    }

    #[test]
    fn clamp_below_min() {
        assert_eq!(clamp_size(3, Some(5), Some(20)), 5);
    }

    #[test]
    fn clamp_above_max() {
        assert_eq!(clamp_size(30, Some(5), Some(20)), 20);
    }

    #[test]
    fn clamp_min_wins_over_max() {
        // min=10, max=5 → min wins → 10
        assert_eq!(clamp_size(7, Some(10), Some(5)), 10);
    }

    #[test]
    fn clamp_no_constraints() {
        assert_eq!(clamp_size(42, None, None), 42);
    }

    // ── compute_padding_box (CSS Box Model 3 §1, Overflow 3 §3) ────

    #[test]
    fn padding_box_subtracts_border_on_all_sides() {
        // CSS Box Model 3: padding-box edge = border-box ∸ border.
        // Inset by 1 on every side reduces a 10×10 outer to an 8×8
        // padding-box offset by (1, 1).
        let outer = LayoutRect::new(0, 0, 10, 10);
        let border = Border::single(); // all four sides = 1 cell
        let pb = compute_padding_box(outer, border);
        assert_eq!(pb, LayoutRect::new(1, 1, 8, 8));
    }

    #[test]
    fn padding_box_independent_of_border_collapse() {
        // `compute_padding_box` ignores border-collapse entirely.
        // M5.5b's layout-time expansion of `content_layout` into the
        // border ring is a child-positioning concern, not a paint-
        // clipping one. CSS Overflow 3 §3: the scrollport is the
        // padding-box, full stop — no table/collapse exception.
        let outer = LayoutRect::new(5, 10, 20, 15);
        let border = Border::single();
        // Function signature takes no BorderCollapse parameter — the
        // semantics are independent by construction. The assertion
        // here is the result equals what we'd get for any collapse
        // mode (the same single rect).
        let pb = compute_padding_box(outer, border);
        assert_eq!(pb, LayoutRect::new(6, 11, 18, 13));
    }

    #[test]
    fn padding_box_saturates_when_border_exceeds_outer() {
        // Defensive: a degenerate outer rect smaller than the
        // border insets must not panic. Should saturate to a
        // zero-size rect at the inset origin.
        let outer = LayoutRect::new(0, 0, 1, 1);
        let border = Border::single();
        let pb = compute_padding_box(outer, border);
        // After inset by (1, 1) on a 1×1: x=1, y=1, w=0, h=0.
        assert_eq!(pb.x, 1);
        assert_eq!(pb.y, 1);
        assert_eq!(pb.width, 0);
        assert_eq!(pb.height, 0);
    }

    #[test]
    fn padding_box_with_no_border_equals_outer() {
        // No border → padding-box = border-box. The non-bordered
        // path doesn't shrink the rect.
        let outer = LayoutRect::new(3, 7, 50, 40);
        let pb = compute_padding_box(outer, Border::none());
        assert_eq!(pb, outer);
    }

    #[test]
    fn padding_box_per_side_border_only_top() {
        // The source-disclosure shape: `border-top: solid`, no other
        // sides. Padding-box drops only the top row.
        let outer = LayoutRect::new(0, 0, 20, 10);
        let mut border = Border::none();
        border.top = BorderStyle::Solid;
        let pb = compute_padding_box(outer, border);
        assert_eq!(pb, LayoutRect::new(0, 1, 20, 9));
    }
}
