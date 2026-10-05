//! Phase-2 placement of `position: absolute | fixed` elements: the
//! containing block, the placed rect (CSS 2.1 §10.3.7 / §10.6.4), then
//! the subtree laid out inside it.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{IntrinsicSize, LayoutRect, Length, Position, Size, clamp_size};
use crate::node::TuiNodeExt;
use crate::style::ComputedStyle;

use super::axis::axis_size_from_edges;
use super::*;
use crate::render::layout_pass::box_sizing::Sizer;
use crate::render::layout_pass::intrinsic::{Keywords, intrinsic_size};

/// After phase-1 flex layout completes, walk the tree in document
/// order and place every `position: absolute | fixed` element
/// against its containing block. For each placed element, re-run
/// `layout_node` on the subtree so the element's own children flow
/// inside the placed rect.
///
/// Document-order walk guarantees that an outer positioned element
/// is placed before any positioned descendants — so when a nested
/// absolute resolves its containing block, the outer's
/// `TuiExt.layout` is already populated.
pub(in crate::render::layout_pass) fn place_positioned(
    dom: &mut Dom<TuiExt>,
    viewport: LayoutRect,
) {
    let positioned = collect_positioned(dom, dom.root());
    for id in positioned {
        let cb = containing_block(dom, id, viewport);
        let computed = dom
            .node(id)
            .computed_rc()
            .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
        let placed = compute_placed_rect(dom, id, &computed, cb);
        crate::render::layout_pass::layout_node(dom, id, placed, cb.width);
    }
}

fn collect_positioned(dom: &Dom<TuiExt>, id: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk_for_positioned(dom, id, &mut out);
    out
}

fn walk_for_positioned(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<NodeId>) {
    if dom.node(id).node_type() == NodeType::Element {
        let pos = computed_position(dom, id);
        if matches!(pos, Position::Absolute | Position::Fixed) {
            out.push(id);
        }
    }
    for child in dom.node(id).child_nodes() {
        match child.node_type() {
            NodeType::Element | NodeType::Fragment => {
                walk_for_positioned(dom, child.id(), out);
            }
            _ => {}
        }
    }
}

/// Compute the placed rect for an absolute/fixed element given its
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
///     §10.6.4): the element's intrinsic size on that axis, height
///     measured at the resolved width. A tooltip positioned with
///     only `top` / `left` is therefore as wide as its text, not 0.
///
/// X / Y resolve from the offsets via [`axis_position_anchored`];
/// an axis with both insets `auto` takes `TuiExt::static_position`.
fn compute_placed_rect(
    dom: &Dom<TuiExt>,
    id: NodeId,
    c: &ComputedStyle,
    cb: LayoutRect,
) -> LayoutRect {
    use crate::layout::Direction;
    // Resolve width/height — percentage AND Calc both resolve
    // against the containing-block's matching axis. The intrinsic
    // measurement only runs when an `auto` axis is not pinned by both
    // edges (it walks the subtree).
    let width = resolve_size_axis(
        &c.width,
        Sizer::horizontal(c, cb.width),
        cb.width,
        &c.left,
        &c.right,
        cb.width,
        |keyword, available| match keyword {
            Some(k) => Keywords::new(dom, id, c, Direction::Row, cb.height, cb.width).keyword(
                k,
                Some(cb.width),
                available,
            ),
            None => intrinsic_size(dom, id, Direction::Row, cb.width, cb.width),
        },
    );
    // CSS 2.1 §10.4: the tentative width clamped by `max-width`, then
    // `min-width`, measured as `box-sizing` says (an absolutely
    // positioned box's containing block is definite).
    let kw = Keywords::new(dom, id, c, Direction::Row, cb.height, cb.width);
    let width = kw.sizer().floor(clamp_size(
        width,
        kw.min(&c.min_width, Some(cb.width), cb.width),
        kw.max(&c.max_width, Some(cb.width), cb.width),
    ));
    // A keyword height is the content height at the resolved width
    // (CSS Sizing 3 §3.1), the shrink-to-fit height.
    let height = resolve_size_axis(
        &c.height,
        Sizer::vertical(c, cb.width),
        cb.height,
        &c.top,
        &c.bottom,
        cb.height,
        |_, _| intrinsic_size(dom, id, Direction::Column, width, cb.width),
    );
    // CSS 2.1 §10.7, the same for the height.
    let kw = Keywords::new(dom, id, c, Direction::Column, width, cb.width);
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
    let static_pos = dom.node(id).ext().and_then(|e| e.static_position);

    let x = if c.left.cells(basis_w).is_some()
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
    let y = if c.top.cells(basis_h).is_some()
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

/// Resolve a positioned box's `Size` on one axis (CSS 2.1 §10.3.7 /
/// §10.6.4) against the containing block's extent: a definite size is
/// the size, measured as `box-sizing` says (`sizer`, CSS UI 3 §3.1);
/// `auto` spans between the start / end edges when both are non-auto,
/// else is the content's size. `content(None, _)` is that shrink-to-fit
/// size; `content(Some(keyword), available)` an intrinsic keyword's
/// (CSS Sizing 3 §3.1), with the span between the edges — or the
/// containing block — as its stretch-fit size. The border box is never
/// smaller than the padding and border. Shared by positioned elements
/// and positioned pseudo-elements.
pub(in crate::render::layout_pass) fn resolve_size_axis(
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
