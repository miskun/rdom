//! The rect phase 2 places an absolutely or fixed positioned box at (CSS
//! 2.1 §10.3.7 / §10.6.4, CSS Box Alignment 3 §6): its insets against its
//! containing block, its size from `width` / `height` or its content
//! (shrink-to-fit, `stretch`), `auto` margins and self-alignment in the
//! inset-modified containing block — for an element and for a positioned
//! `::before` / `::after` alike ([`Placed`]).

use rdom_core::{Dom, NodeId};

use crate::ext::{PseudoSlot, StaticPosition, TuiExt};
use crate::layout::{IntrinsicSize, LayoutRect, Length, Size, clamp_size};
use crate::node::TuiNodeExt;
use crate::style::ComputedStyle;

use super::axis::axis_size_from_edges;
use super::*;
use crate::render::layout_pass::box_sizing::Sizer;
use crate::render::layout_pass::intrinsic::Keywords;
use crate::render::layout_pass::items::AnonymousItem;

/// What phase 2 places: a positioned element, or the box of a positioned
/// `::before` / `::after` (`item`, its host's `slot` pseudo-element).
#[derive(Clone, Copy)]
pub(super) enum Placed<'a> {
    Element(NodeId),
    Generated {
        host: NodeId,
        slot: PseudoSlot,
        item: &'a AnonymousItem,
    },
}

impl Placed<'_> {
    /// The box's intrinsic size keywords on one axis.
    fn keywords<'d>(
        &'d self,
        dom: &'d Dom<TuiExt>,
        c: &ComputedStyle,
        direction: crate::layout::Direction,
        cross: u16,
        cb_width: u16,
    ) -> Keywords<'d> {
        match *self {
            Placed::Element(id) => Keywords::new(dom, id, c, direction, cross, cb_width),
            Placed::Generated { item, .. } => {
                Keywords::for_run(dom, item, direction, cross, cb_width)
            }
        }
    }

    /// Its content height at the border-box width `cross` (CSS 2.1
    /// §10.6.4) — its content's, whatever its own `height` says: asked only
    /// where the height placed by is `auto`, which a position option's
    /// style may make it where the box's own is not (CSS Anchor
    /// Positioning 1 §4.1). The inline axis is shrink-to-fit
    /// ([`shrink_available`]).
    fn shrink_to_fit(
        &self,
        dom: &Dom<TuiExt>,
        direction: crate::layout::Direction,
        cross: u16,
        cb_width: u16,
    ) -> u16 {
        match *self {
            Placed::Element(id) => crate::render::layout_pass::intrinsic::content_max_size(
                dom, id, direction, cross, cb_width,
            ),
            Placed::Generated { item, .. } => {
                item.content_size(dom, direction, cross, true, cb_width)
            }
        }
    }

    /// Its static position, when it has one.
    fn static_position(&self, dom: &Dom<TuiExt>) -> Option<StaticPosition> {
        match *self {
            Placed::Element(id) => dom.node(id).ext().and_then(|e| e.static_position),
            Placed::Generated { host, slot, item } => {
                super::pseudo::static_position(dom, host, slot, item.style())
            }
        }
    }

    /// The node whose box it is laid out in — whose `direction` decides
    /// an over-constrained inset: an element's box parent, a
    /// pseudo-element's host.
    fn parent(&self, dom: &Dom<TuiExt>) -> Option<NodeId> {
        match *self {
            Placed::Element(id) => crate::render::box_tree::box_parent(dom, id),
            Placed::Generated { host, .. } => Some(host),
        }
    }
}

/// CSS 2.1 §10.3.7: the available width an `auto` width shrinks to fit —
/// the containing block's, "found by solving for 'width' after setting
/// 'left' (in case 1) or 'right' (in case 3) to 0": less the other inset
/// (the static position's offset when both are `auto`) and the margins
/// (`auto` ones 0). Shrink-to-fit is then `min(max(min-content,
/// available), max-content)`, the `fit-content` keyword (C15G-ABSPOS-CLAMP).
fn shrink_available(
    dom: &Dom<TuiExt>,
    placed: &Placed<'_>,
    c: &ComputedStyle,
    cb: LayoutRect,
) -> u16 {
    let basis = i32::from(cb.width);
    let inset = match (c.left.cells(basis), c.right.cells(basis)) {
        (Some(left), _) => left,
        (None, Some(right)) => right,
        (None, None) => match placed.static_position(dom) {
            Some(sp) if c.text_direction != crate::layout::TextDirection::Rtl => sp.x - cb.x,
            _ => 0,
        },
    };
    let margin = |m: &crate::layout::MarginValue| match m {
        crate::layout::MarginValue::Auto => 0,
        m => i32::from(m.resolve(cb.width)),
    };
    let available = basis - inset - margin(&c.margin.left) - margin(&c.margin.right);
    available.clamp(0, i32::from(u16::MAX)) as u16
}

/// CSS Sizing 4 §3.1 `stretch` on an absolutely positioned box: its
/// inset-modified containing block (an `auto` inset 0) less its margins
/// (`auto` 0) on `axis`, as a border box (C15G-STRETCH).
fn stretch_fit(c: &ComputedStyle, cb: LayoutRect, axis: crate::layout::Direction) -> u16 {
    let basis = i32::from(cb.width);
    let margin = |m: &crate::layout::MarginValue| match m {
        crate::layout::MarginValue::Auto => 0,
        m => i32::from(m.resolve(cb.width)),
    };
    let (extent, insets, margins) = match axis {
        crate::layout::Direction::Row => (
            i32::from(cb.width),
            c.left.cells(basis).unwrap_or(0) + c.right.cells(basis).unwrap_or(0),
            margin(&c.margin.left) + margin(&c.margin.right),
        ),
        crate::layout::Direction::Column => {
            let basis = i32::from(cb.height);
            (
                basis,
                c.top.cells(basis).unwrap_or(0) + c.bottom.cells(basis).unwrap_or(0),
                margin(&c.margin.top) + margin(&c.margin.bottom),
            )
        }
    };
    (extent - insets - margins).clamp(0, i32::from(u16::MAX)) as u16
}

/// Compute the placed rect for an absolute/fixed box given its
/// computed style and resolved containing block.
///
/// Width / height resolve in this order:
/// - `Size::Fixed(n)` → `n`.
/// - `Size::Flex(_)` → fills the containing block on that axis.
/// - `Size::Percent(p)` → `cb_axis * p / 100`. Resolves against the
///   *containing block* — for absolute/fixed positioning, that's the
///   nearest positioned ancestor (or the viewport for `fixed`).
/// - `Size::Auto`:
///   - When both edges of the axis are `Cells`, derive from
///     `cb_axis - left - right` (or `cb_axis - top - bottom`).
///   - Otherwise shrink-to-fit the content (CSS 2.1 §10.3.7 /
///     §10.6.4): the width `min(max(min-content, available),
///     max-content)` ([`shrink_available`]), the height the content's
///     at that width. A tooltip positioned with only `top` / `left` is
///     therefore as wide as its text, not 0, and wraps where the
///     containing block ends.
///
/// X / Y resolve from the offsets via [`axis_position_anchored`];
/// an axis with both insets `auto` takes the box's static position.
pub(super) fn compute_placed_rect(
    dom: &Dom<TuiExt>,
    placed: Placed<'_>,
    c: &ComputedStyle,
    cb: LayoutRect,
) -> LayoutRect {
    use crate::layout::Direction;
    // CSS Box Alignment 3 §6.1 / §6.2: an absolutely positioned box is
    // aligned in its inset-modified containing block (`self_align`); an
    // aligned `auto` size is `fit-content` there, not stretched.
    let justify = self_align(
        c.justify_self,
        &c.left,
        &c.right,
        &c.margin.left,
        &c.margin.right,
    );
    let align = self_align(
        c.align_self,
        &c.top,
        &c.bottom,
        &c.margin.top,
        &c.margin.bottom,
    );
    let fit = |size: &Size, aligned: bool| match size {
        Size::Auto if aligned => Size::Intrinsic(IntrinsicSize::FitContent),
        other => other.clone(),
    };
    let (width_size, height_size) = (
        fit(&c.width, justify.is_some()),
        fit(&c.height, align.is_some()),
    );
    // Resolve width/height — percentage AND Calc both resolve
    // against the containing-block's matching axis. The intrinsic
    // measurement only runs when an `auto` axis is not pinned by both
    // edges (it walks the subtree).
    let width = resolve_size_axis(
        &width_size,
        Sizer::horizontal(c, cb.width),
        cb.width,
        &c.left,
        &c.right,
        cb.width,
        |keyword, available| match keyword {
            Some(IntrinsicSize::Stretch) => stretch_fit(c, cb, Direction::Row),
            Some(k) => placed
                .keywords(dom, c, Direction::Row, cb.height, cb.width)
                .keyword(k, Some(cb.width), available),
            None => placed
                .keywords(dom, c, Direction::Row, cb.height, cb.width)
                .keyword(
                    &IntrinsicSize::FitContent,
                    Some(cb.width),
                    shrink_available(dom, &placed, c, cb),
                ),
        },
    );
    // CSS 2.1 §10.4: the tentative width clamped by `max-width`, then
    // `min-width`, measured as `box-sizing` says (an absolutely
    // positioned box's containing block is definite).
    let kw = placed.keywords(dom, c, Direction::Row, cb.height, cb.width);
    let width = kw.sizer().floor(clamp_size(
        width,
        kw.min(&c.min_width, Some(cb.width), cb.width),
        kw.max(&c.max_width, Some(cb.width), cb.width),
    ));
    // A keyword height is the content height at the resolved width
    // (CSS Sizing 3 §3.1), the shrink-to-fit height.
    let height = resolve_size_axis(
        &height_size,
        Sizer::vertical(c, cb.width),
        cb.height,
        &c.top,
        &c.bottom,
        cb.height,
        |keyword, _| match keyword {
            Some(IntrinsicSize::Stretch) => stretch_fit(c, cb, Direction::Column),
            _ => placed.shrink_to_fit(dom, Direction::Column, width, cb.width),
        },
    );
    // CSS 2.1 §10.7, the same for the height.
    let kw = placed.keywords(dom, c, Direction::Column, width, cb.width);
    let height = kw.sizer().floor(clamp_size(
        height,
        kw.min(&c.min_height, Some(cb.height), cb.height),
        kw.max(&c.max_height, Some(cb.height), cb.height),
    ));

    // M5.3b — absolute element centering via `margin: auto` between
    // resolved insets. CSS rule: when both axis insets are `Cells`
    // (non-auto) AND the corresponding axis margins are both `Auto`,
    // distribute remaining space equally to both margins — i.e.
    // center the element between the insets.
    use crate::layout::MarginValue;
    let (cx_left, cx_right) = (c.margin.left.clone(), c.margin.right.clone());
    let (cy_top, cy_bottom) = (c.margin.top.clone(), c.margin.bottom.clone());
    // Margin percent / calc resolves against the containing-block
    // width on ALL four sides (CSS 2.1 §8.3).
    let margin_cb_w = cb.width;

    let basis_w = cb.width as i32;
    let basis_h = cb.height as i32;

    // An axis with both insets `auto` starts at the static position
    // phase 1 recorded (CSS 2.1 §10.3.7 / §10.6.4). It is `None` only
    // for an element whose parent has not been laid out yet; the
    // containing block's start stands in then.
    let static_pos = placed.static_position(dom);

    let cb_rtl = placed
        .parent(dom)
        .and_then(|p| dom.node(p).computed().map(|pc| pc.text_direction))
        .unwrap_or(c.text_direction)
        == crate::layout::TextDirection::Rtl;
    let self_rtl = c.text_direction == crate::layout::TextDirection::Rtl;
    let x = if let Some(value) = justify {
        let (start, extent) = inset_modified(&c.left, &c.right, cb.x, cb.width);
        let (ml, mr) = (
            c.margin.left.resolve(margin_cb_w),
            c.margin.right.resolve(margin_cb_w),
        );
        let free = extent - i32::from(width) - i32::from(ml) - i32::from(mr);
        start
            + i32::from(ml)
            + crate::render::layout_pass::block::justify_offset(value, free, cb_rtl, self_rtl)
    } else if c.left.cells(basis_w).is_some()
        && c.right.cells(basis_w).is_some()
        && matches!(cx_left, MarginValue::Auto)
        && matches!(cx_right, MarginValue::Auto)
    {
        // Center horizontally between left + right insets.
        let left = c.left.cells(basis_w).unwrap_or(0);
        let right = c.right.cells(basis_w).unwrap_or(0);
        let span = basis_w.saturating_sub(left + right);
        let extra = span.saturating_sub(width as i32).max(0);
        cb.x + left + extra / 2
    } else {
        let base = match static_pos {
            Some(sp) if matches!((&c.left, &c.right), (Length::Auto, Length::Auto)) => sp.x,
            // CSS 2.1 §10.3.7: over-constrained, the inline-end inset is
            // ignored — `right` under `ltr`, `left` under `rtl` (the
            // box's own `direction`, DIVERGENCES).
            _ if c.text_direction == crate::layout::TextDirection::Rtl
                && c.left.cells(basis_w).is_some()
                && c.right.cells(basis_w).is_some() =>
            {
                axis_position_anchored(&Length::Auto, &c.right, cb.x, cb.width, width)
            }
            _ => axis_position_anchored(&c.left, &c.right, cb.x, cb.width, width),
        };
        let start_margin = match &cx_left {
            MarginValue::Cells(n) => *n as i32,
            MarginValue::Auto => 0,
            MarginValue::Calc(_) => cx_left.resolve(margin_cb_w) as i32,
        };
        base + start_margin
    };
    let y = if let Some(value) = align {
        let (start, extent) = inset_modified(&c.top, &c.bottom, cb.y, cb.height);
        let (mt, mb) = (
            c.margin.top.resolve(margin_cb_w),
            c.margin.bottom.resolve(margin_cb_w),
        );
        let free = extent - i32::from(height) - i32::from(mt) - i32::from(mb);
        // The block axis runs top to bottom (`horizontal-tb`), for the
        // containing block and the box alike.
        start
            + i32::from(mt)
            + crate::render::layout_pass::block::justify_offset(value, free, false, false)
    } else if c.top.cells(basis_h).is_some()
        && c.bottom.cells(basis_h).is_some()
        && matches!(cy_top, MarginValue::Auto)
        && matches!(cy_bottom, MarginValue::Auto)
    {
        let top = c.top.cells(basis_h).unwrap_or(0);
        let bottom = c.bottom.cells(basis_h).unwrap_or(0);
        let span = basis_h.saturating_sub(top + bottom);
        let extra = span.saturating_sub(height as i32).max(0);
        cb.y + top + extra / 2
    } else {
        let base = match static_pos {
            Some(sp) if matches!((&c.top, &c.bottom), (Length::Auto, Length::Auto)) => sp.y,
            _ => axis_position_anchored(&c.top, &c.bottom, cb.y, cb.height, height),
        };
        let start_margin = match &cy_top {
            MarginValue::Cells(n) => *n as i32,
            MarginValue::Auto => 0,
            MarginValue::Calc(_) => cy_top.resolve(margin_cb_w) as i32,
        };
        base + start_margin
    };
    LayoutRect::new(x, y, width, height)
}

/// The self-alignment that places an absolutely positioned box on one
/// axis (CSS Box Alignment 3 §6.1 / §6.2), or `None` where the CSS 2.1
/// placement stands: `auto` is `normal` for such a box (§6.1), and
/// `normal` / `stretch` keep §10.3.7's / §10.6.4's sizes and offsets;
/// both insets `auto` place it at its static position (the
/// static-position rectangle alignment of CSS Position 3 §4.1 is not
/// modeled, DIVERGENCES §4); and `auto` margins on both sides between
/// two insets center it, winning over the alignment (§6.1).
fn self_align(
    value: crate::layout::Alignment,
    start: &Length,
    end: &Length,
    margin_start: &crate::layout::MarginValue,
    margin_end: &crate::layout::MarginValue,
) -> Option<crate::layout::Alignment> {
    let both_auto = matches!((start, end), (Length::Auto, Length::Auto));
    let auto_margins = margin_start.is_auto() && margin_end.is_auto();
    (crate::render::layout_pass::block::aligns(value) && !both_auto && !auto_margins)
        .then_some(value)
}

/// The inset-modified containing block on one axis (CSS Position 3
/// §4.1): the containing block (`cb_start`, `cb_extent`) less the
/// insets, an `auto` one counting as 0 when the other is not. Returns
/// its start and extent (negative when the insets overlap).
fn inset_modified(start: &Length, end: &Length, cb_start: i32, cb_extent: u16) -> (i32, i32) {
    let basis = i32::from(cb_extent);
    let s = start.cells(basis).unwrap_or(0);
    let e = end.cells(basis).unwrap_or(0);
    (cb_start + s, basis - s - e)
}

/// Resolve a positioned box's `Size` on one axis (CSS 2.1 §10.3.7 /
/// §10.6.4) against the containing block's extent: a definite size is
/// the size, measured as `box-sizing` says (`sizer`, CSS UI 3 §3.1);
/// `auto` spans between the start / end edges when both are non-auto,
/// else is the content's size. `content(None, _)` is that shrink-to-fit
/// size; `content(Some(keyword), available)` an intrinsic keyword's
/// (CSS Sizing 3 §3.1), with the span between the edges — or the
/// containing block — as its stretch-fit size. The border box is never
/// smaller than the padding and border.
fn resolve_size_axis(
    size: &Size,
    sizer: Sizer,
    cb_extent: u16,
    start: &Length,
    end: &Length,
    edges_basis: u16,
    content: impl FnOnce(Option<&IntrinsicSize>, u16) -> u16,
) -> u16 {
    let both_edges =
        start.cells(edges_basis as i32).is_some() && end.cells(edges_basis as i32).is_some();
    sizer.floor(match (size, size.cells(Some(cb_extent))) {
        (_, Some(cells)) => sizer.outer(cells),
        (Size::Flex(_), _) => cb_extent,
        (Size::Intrinsic(k), _) => {
            let available = if both_edges {
                axis_size_from_edges(start, end, edges_basis, 0)
            } else {
                cb_extent
            };
            content(Some(k), available)
        }
        _ if both_edges => axis_size_from_edges(start, end, edges_basis, 0),
        _ => content(None, 0),
    })
}
