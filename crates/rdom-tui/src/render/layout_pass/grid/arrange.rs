//! Each grid item laid out in its grid area (CSS Grid 2 §6.2, §11.1
//! "Once the size of each grid area is thus established, the grid items
//! are laid out into their containing blocks"): the area from the sized
//! tracks — columns from the inline-start edge, the right one under
//! `direction: rtl` (§7.1) — and the item's margin box in it.
//!
//! Each item is aligned in its area (§10, CSS Box Alignment 3 §6):
//! `auto` margins take the free space first (§10.2); else `justify-self`
//! / `align-self` (`auto` taking the container's `justify-items` /
//! `align-items`) stretch an `auto` size — `normal` behaving as
//! `stretch`, except for an item with a preferred aspect ratio, which is
//! sized as a block-level box (§6.2) — or place the item's own size
//! (fit-content inline, its content height in the block axis) through
//! the block layout's offsets (`block::justify_offset`: `start` / `end`
//! by the writing mode, `self-*` by the item's, `left` / `right`
//! physical, `center` rounded down, `safe` keeping an overflowing item
//! at the start, §4.4).

use rdom_core::{Dom, NodeId};

use super::Grid;
use super::placement::Placed;
use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{
    Align, Alignment, Direction, LayoutRect, MarginValue, TextDirection, clamp_size,
};
use crate::render::layout_pass::block::justify_offset;
use crate::render::layout_pass::box_sizing::aspect_cross_from_main;
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
        // The area in physical cells: an `rtl` grid's columns run from
        // its right edge (CSS Writing Modes 4 §2.1).
        let x = match rtl {
            true => right - x1 as i32,
            false => container.x + x0 as i32,
        };
        let area = LayoutRect::new(x, container.y + y0 as i32, cells(x1 - x0), cells(y1 - y0));
        let mut rect = fit(dom, &p, computed, area);
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

/// `p`'s border box in its grid `area` (physical cells): sized on each
/// axis by its declared size, else stretched or `fit-content` as its
/// self-alignment says (CSS Grid 2 §6.2), then placed in the area by its
/// `auto` margins (§10.2) or its self-alignment (§10.3, §10.4).
fn fit(dom: &Dom<TuiExt>, p: &Placed, container: &ComputedStyle, area: LayoutRect) -> LayoutRect {
    let item = &p.item;
    let c = item.computed(dom);
    let cb = area.width;
    let m = super::margins(dom, p, cb);
    let fill = |extent: u16, a: i32, b: i32| {
        (i32::from(extent) - a - b).clamp(0, i32::from(u16::MAX)) as u16
    };
    // §6.2: a preferred aspect ratio keeps `normal` from stretching —
    // such an item is sized as a block-level box: its `auto` width
    // fills the area, its `auto` height follows the ratio.
    let ratio = c.aspect_ratio.filter(|r| r.value().is_some());
    let justify = self_alignment(c.justify_self, container.justify_items);
    let align = self_alignment(c.align_self, container.align_items);
    let auto = |side: &MarginValue, trimmed: bool| side.is_auto() && !trimmed;
    let (auto_left, auto_right) = (
        auto(&c.margin.left, p.trim.left),
        auto(&c.margin.right, p.trim.right),
    );
    let (auto_top, auto_bottom) = (
        auto(&c.margin.top, p.trim.top),
        auto(&c.margin.bottom, p.trim.bottom),
    );

    // The inline axis: `justify-self`, `auto` taking `justify-items`.
    let available_w = fill(area.width, m.left, m.right);
    let kw_h = item.keywords(dom, &c, Direction::Column, available_w, cb);
    let definite_h = kw_h.size(
        &c.height,
        Some(area.height),
        fill(area.height, m.top, m.bottom),
    );
    let kw = item.keywords(dom, &c, Direction::Row, area.height, cb);
    let stretch_w =
        !auto_left && !auto_right && matches!(justify.keyword, Align::Normal | Align::Stretch);
    let width = match kw.size(&c.width, Some(cb), available_w) {
        Some(w) => w,
        // A block-level box with a ratio and a definite height takes its
        // width from them (CSS Sizing 4 §5.1).
        None if ratio.is_some() && justify.keyword == Align::Normal && definite_h.is_some() => {
            ratio
                .zip(definite_h)
                .and_then(|(r, h)| aspect_cross_from_main(h, r, Direction::Column, &c, cb))
                .unwrap_or(available_w)
        }
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
    let available_h = fill(area.height, m.top, m.bottom);
    let kw = item.keywords(dom, &c, Direction::Column, width, cb);
    let stretch_h = !auto_top
        && !auto_bottom
        && match align.keyword {
            Align::Stretch => true,
            Align::Normal => ratio.is_none(),
            _ => false,
        };
    let height = match kw.size(&c.height, Some(area.height), available_h) {
        Some(h) => h,
        None if stretch_h => available_h,
        None => ratio
            .and_then(|r| aspect_cross_from_main(width, r, Direction::Row, &c, cb))
            .unwrap_or_else(|| item.intrinsic_size(dom, Direction::Column, width, cb)),
    };
    let height = kw.sizer().floor(clamp_size(
        height,
        kw.min(&c.min_height, Some(area.height), available_h),
        kw.max(&c.max_height, Some(area.height), available_h),
    ));

    let rtl = |s: &ComputedStyle| s.text_direction == TextDirection::Rtl;
    let free_x = i32::from(available_w) - i32::from(width);
    let x = auto_margin_offset(free_x, auto_left, auto_right)
        .unwrap_or_else(|| justify_offset(justify, free_x, rtl(container), rtl(&c)));
    // The block axis runs top to bottom (`horizontal-tb`) for the
    // container and the item alike.
    let free_y = i32::from(available_h) - i32::from(height);
    let y = auto_margin_offset(free_y, auto_top, auto_bottom)
        .unwrap_or_else(|| justify_offset(align, free_y, false, false));
    LayoutRect::new(area.x + m.left + x, area.y + m.top + y, width, height)
}

/// An item's self-alignment on one axis (CSS Box Alignment 3 §6.1 /
/// §6.2): its own value, or for `auto` its container's `*-items` — a
/// `legacy` keyword as the keyword it names.
fn self_alignment(own: Alignment, default: Alignment) -> Alignment {
    if own.keyword == Align::Auto {
        Alignment {
            legacy: false,
            ..default
        }
    } else {
        own
    }
}

/// CSS Grid 2 §10.2: the offset `auto` margins give an item in its area
/// when `free` cells are left — "auto margins absorb positive free space
/// prior to alignment": two share it (the leading one rounded down), one
/// takes it all. `None` with no `auto` margin on the axis, or no
/// positive free space (they are 0 then, and the alignment applies).
fn auto_margin_offset(free: i32, start: bool, end: bool) -> Option<i32> {
    match (start, end) {
        _ if free <= 0 => None,
        (true, true) => Some(free.div_euclid(2)),
        (true, false) => Some(free),
        (false, true) => Some(0),
        (false, false) => None,
    }
}
