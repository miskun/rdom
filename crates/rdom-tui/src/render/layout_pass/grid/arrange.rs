//! Each grid item laid out in its grid area (CSS Grid 2 §6.2, §11.1
//! "Once the size of each grid area is thus established, the grid items
//! are laid out into their containing blocks"): the area from the sized
//! tracks — columns from the inline-start edge, the right one under
//! `direction: rtl` (§7.1) — and the item's margin box in it.
//!
//! Self-alignment is `normal` here: an item whose size on an axis is
//! `auto` (and its margins not `auto`) stretches to fill its area,
//! `normal` behaving as `stretch` (CSS Box Alignment 3 §6.1); any other
//! item takes its own size — fit-content on the inline axis, its content
//! height on the block axis — at the area's start. The other alignment
//! values are C7-GRID-ALIGN's.

use rdom_core::{Dom, NodeId};

use super::Grid;
use super::placement::Placed;
use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{Align, Direction, LayoutRect, clamp_size};
use crate::render::layout_pass::gutter::scroll_offset;
use crate::render::layout_pass::items::Item;
use crate::render::layout_pass::layout_node;
use crate::style::ComputedStyle;

/// Lay out `grid`'s items in the grid container `id`'s content box
/// `container`. Returns the anonymous items' boxes, in placement order.
pub(super) fn arrange(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    grid: Grid,
    container: LayoutRect,
) -> Vec<AnonymousIfc> {
    let columns = grid.columns.extents();
    let rows = grid.rows.as_ref().map(|r| r.extents()).unwrap_or_default();
    let rtl = crate::render::layout_pass::margin_trim::inline_reversed(computed);
    // The tracks' edges, absolute (unscrolled), for §9.1: an `rtl` grid's
    // columns start at their right edge.
    let mut lines = grid.lines;
    let right = container.x + i32::from(container.width);
    lines.columns.edges = columns
        .iter()
        .map(|&(a, b)| match rtl {
            true => (right - a as i32, right - b as i32),
            false => (container.x + a as i32, container.x + b as i32),
        })
        .collect();
    lines.rows.edges = rows
        .iter()
        .map(|&(a, b)| (container.y + a as i32, container.y + b as i32))
        .collect();
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.grid_lines = Some(Box::new(lines));
    }
    let scroll_x = scroll_offset(dom, id, Direction::Row);
    let scroll_y = scroll_offset(dom, id, Direction::Column);
    let mut anonymous = Vec::new();
    for p in grid.placed {
        let (x0, x1) = (columns[p.columns.start].0, columns[p.columns.end - 1].1);
        let (y0, y1) = rows
            .get(p.rows.start)
            .zip(rows.get(p.rows.end - 1))
            .map_or((0, 0), |(a, b)| (a.0, b.1));
        let area = LayoutRect::new(
            container.x + x0 as i32,
            container.y + y0 as i32,
            cells(x1 - x0),
            cells(y1 - y0),
        );
        let mut rect = fit(dom, &p, computed, area, rtl);
        if rtl {
            // The column axis runs from the right edge (CSS Writing Modes
            // 4 §2.1): mirror the item across the content box.
            let from_start = rect.x - container.x;
            rect.x = container.x + i32::from(container.width) - from_start - i32::from(rect.width);
        }
        rect.x -= scroll_x;
        rect.y -= scroll_y;
        match &p.item {
            Item::Element(child) => layout_node(dom, *child, rect, area.width),
            Item::Anonymous(anon) => anonymous.push(anon.lay_out(dom, rect, area.width)),
        }
    }
    anonymous
}

/// Cells as an extent.
fn cells(n: u32) -> u16 {
    n.min(u32::from(u16::MAX)) as u16
}

/// `item`'s border box in its grid `area` (in a left-to-right frame: an
/// `rtl` container's inline-start margin is the item's right one, and
/// the caller mirrors the box).
fn fit(
    dom: &Dom<TuiExt>,
    p: &Placed,
    container: &ComputedStyle,
    area: LayoutRect,
    rtl: bool,
) -> LayoutRect {
    let item = &p.item;
    let c = item.computed(dom);
    let cb = area.width;
    let m = super::margins(dom, p, cb);
    let (left, right, ml, mr) = if rtl {
        (&c.margin.right, &c.margin.left, m.right, m.left)
    } else {
        (&c.margin.left, &c.margin.right, m.left, m.right)
    };
    let (mt, mb) = (m.top, m.bottom);
    let fill = |extent: u16, a: i32, b: i32| {
        (i32::from(extent) - a - b).clamp(0, i32::from(u16::MAX)) as u16
    };

    // The inline axis: `justify-self`, `auto` taking the container's
    // `justify-items`.
    let available_w = fill(area.width, ml, mr);
    let kw = item.keywords(dom, &c, Direction::Row, area.height, cb);
    let stretch_w = stretches(c.justify_self.keyword, container.justify_items.keyword)
        && !left.is_auto()
        && !right.is_auto();
    let width = match kw.size(&c.width, Some(cb), available_w) {
        Some(w) => w,
        None if stretch_w => available_w,
        // Fit-content: the available space clamped between the min- and
        // max-content sizes (CSS Sizing 3 §3.1).
        None => {
            let min = item.content_extreme(dom, Direction::Row, area.height, cb, false);
            let max = item.content_extreme(dom, Direction::Row, area.height, cb, true);
            max.min(min.max(available_w))
        }
    };
    let width = kw.sizer().floor(clamp_size(
        width,
        kw.min(&c.min_width, Some(cb), available_w),
        kw.max(&c.max_width, Some(cb), available_w),
    ));

    // The block axis: `align-self`, `auto` taking `align-items`.
    let available_h = fill(area.height, mt, mb);
    let kw = item.keywords(dom, &c, Direction::Column, width, cb);
    let stretch_h = stretches(c.align_self.keyword, container.align_items.keyword)
        && !c.margin.top.is_auto()
        && !c.margin.bottom.is_auto();
    let height = match kw.size(&c.height, Some(area.height), available_h) {
        Some(h) => h,
        None if stretch_h => available_h,
        None => item.intrinsic_size(dom, Direction::Column, width, cb),
    };
    let height = kw.sizer().floor(clamp_size(
        height,
        kw.min(&c.min_height, Some(area.height), available_h),
        kw.max(&c.max_height, Some(area.height), available_h),
    ));
    LayoutRect::new(area.x + ml, area.y + mt, width, height)
}

/// Whether a self-alignment value (`own`, `auto` taking the container's
/// `default`) stretches the item: `normal` or `stretch` (CSS Box
/// Alignment 3 §6.1 — `normal` behaves as `stretch` for a grid item
/// without an aspect ratio).
fn stretches(own: Align, default: Align) -> bool {
    let keyword = if own == Align::Auto { default } else { own };
    matches!(keyword, Align::Normal | Align::Stretch)
}
