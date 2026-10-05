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

use std::rc::Rc;

use rdom_core::Dom;

use super::baseline::Shim;
use super::placement::Placed;
use super::subgrid::SubAxes;
use super::{Grid, GridBox, content};
use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{
    Align, Alignment, AspectRatio, Direction, LayoutRect, MarginValue, Sides, Size, TextDirection,
    clamp_size,
};
use crate::render::layout_pass::block::justify_offset;
use crate::render::layout_pass::box_sizing::aspect_cross_from_main;
use crate::render::layout_pass::gutter::scroll_offset;
use crate::render::layout_pass::items::Item;
use crate::render::layout_pass::layout_node;
use crate::style::ComputedStyle;

/// Lay out `grid`'s items in the grid container `owner`'s content box
/// `container`. Returns the anonymous items' boxes, in placement order.
/// An element container keeps its lines (§9.1) and scrolls its items; a
/// generated one has no absolutely positioned boxes and does not scroll.
pub(super) fn arrange(
    dom: &mut Dom<TuiExt>,
    owner: GridBox<'_>,
    computed: &ComputedStyle,
    grid: Grid,
    container: LayoutRect,
) -> Vec<AnonymousIfc> {
    let rtl = crate::render::layout_pass::margin_trim::inline_reversed(computed);
    // §10.5: the tracks distributed in the content box by
    // `justify-content` / `align-content` — a subgridded axis's are its
    // parent's, already placed (§9).
    let columns = grid.inherited_columns.clone().unwrap_or_else(|| {
        content::distribute(
            &grid.columns,
            container.width,
            computed.justify_content,
            content::inline_ends(rtl),
        )
    });
    let rows = grid.inherited_rows.clone().unwrap_or_else(|| {
        grid.rows.as_ref().map_or_else(Vec::new, |r| {
            content::distribute(
                r,
                container.height,
                computed.align_content,
                content::BLOCK_ENDS,
            )
        })
    });
    // The tracks' edges, for §9.1 and the subgrids (§9): offsets from
    // the content box's inline-start (an `rtl` grid's right) and top
    // edges, which `distribute` already gives.
    debug_assert!(
        grid.lines.is_some(),
        "a laid-out grid was sized with its lines"
    );
    if let Some(mut lines) = grid.lines {
        lines.columns.edges.clone_from(&columns);
        lines.rows.edges.clone_from(&rows);
        lines.rtl = rtl;
        for (p, &sub) in grid.placed.iter().zip(&grid.subgrids) {
            super::subgrid::record(&mut lines, p, sub);
        }
        if let Some(id) = owner.element()
            && let Some(ext) = dom.node_mut(id).ext_mut()
        {
            ext.grid_lines = Some(Box::new(lines));
        }
    }
    let right = container.x + i32::from(container.width);
    let scroll = |axis| owner.element().map_or(0, |id| scroll_offset(dom, id, axis));
    let (scroll_x, scroll_y) = (scroll(Direction::Row), scroll(Direction::Column));
    let mut anonymous = Vec::new();
    for (k, p) in grid.placed.iter().enumerate() {
        let (x0, x1) = (columns[p.columns.start].0, columns[p.columns.end - 1].1);
        let (y0, y1) = rows
            .get(p.rows.start)
            .zip(rows.get(p.rows.end - 1))
            .map_or((0, 0), |(a, b)| (a.0, b.1));
        // The area in physical cells: an `rtl` grid's columns run from
        // its right edge (CSS Writing Modes 4 §2.1).
        let x = match rtl {
            true => right - x1,
            false => container.x + x0,
        };
        let area = LayoutRect::new(x, container.y + y0, cells(x1 - x0), cells(y1 - y0));
        let mut rect = fit(
            dom,
            p,
            computed,
            area,
            grid.baselines.get(k).and_then(Option::as_ref),
            grid.subgrids.get(k).copied().unwrap_or_default(),
        );
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
fn cells(n: i32) -> u16 {
    n.clamp(0, i32::from(u16::MAX)) as u16
}

/// `p`'s border box in its grid `area` (physical cells): sized on each
/// axis by its declared size, else stretched or `fit-content` as its
/// self-alignment says (CSS Grid 2 §6.2), then placed in the area by its
/// `auto` margins (§10.2), its baseline group's shim (`shim`, §10.4 with
/// Box Alignment 3 §9.3) or its self-alignment (§10.3, §10.4).
#[allow(clippy::too_many_arguments)]
fn fit(
    dom: &Dom<TuiExt>,
    p: &Placed,
    container: &ComputedStyle,
    area: LayoutRect,
    shim: Option<&Shim>,
    sub: SubAxes,
) -> LayoutRect {
    let f = ItemFit::new(dom, p, container, area.width);
    let available_w = fill(area.width, f.m.left, f.m.right);
    // §9: "The subgrid is always stretched in its subgridded
    // dimension(s)": its self-alignment and sizes are ignored there.
    let width = match sub.columns {
        true => available_w,
        false => f.width(dom, Some(area.height)),
    };
    let available_h = fill(area.height, f.m.top, f.m.bottom);
    let stretch_h = !f.auto.top
        && !f.auto.bottom
        && match f.align.keyword {
            Align::Stretch => true,
            Align::Normal => f.ratio.is_none(),
            _ => false,
        };
    let height = match (shim, sub.rows) {
        (_, true) => available_h,
        (Some(s), false) => s.height,
        (None, false) => f.height(dom, width, Some(area.height), stretch_h),
    };

    let rtl = |s: &ComputedStyle| s.text_direction == TextDirection::Rtl;
    let free_x = i32::from(available_w) - i32::from(width);
    let x = match sub.columns {
        true => 0,
        false => auto_margin_offset(free_x, f.auto.left, f.auto.right)
            .unwrap_or_else(|| justify_offset(f.justify, free_x, rtl(container), rtl(&f.c))),
    };
    // The block axis runs top to bottom (`horizontal-tb`) for the
    // container and the item alike.
    let free_y = i32::from(available_h) - i32::from(height);
    let y = match shim {
        _ if sub.rows => 0,
        Some(s) if s.last => free_y - s.offset,
        Some(s) => s.offset,
        None => auto_margin_offset(free_y, f.auto.top, f.auto.bottom)
            .unwrap_or_else(|| justify_offset(f.align, free_y, false, false)),
    };
    LayoutRect::new(area.x + f.m.left + x, area.y + f.m.top + y, width, height)
}

/// The border-box width `p`'s preferred aspect ratio gives it from a
/// definite height (CSS Sizing 4 §5.1) in an area `area_h` tall (`None`
/// before the rows are sized) — what it contributes to its columns
/// (CSS Grid 2 §11.5), clamped by `min-width` / `max-width`. `None` when
/// its width does not come from the ratio.
pub(super) fn ratio_width(
    dom: &Dom<TuiExt>,
    p: &Placed,
    container: &ComputedStyle,
    area_h: Option<u16>,
) -> Option<u16> {
    // Its columns are not sized yet: percentages against them are cyclic
    // (CSS Sizing 3 §5.2.1), so 0.
    let f = ItemFit::new(dom, p, container, 0);
    let width = f.transferred_width(dom, area_h)?;
    let kw = p
        .item
        .keywords(dom, &f.c, Direction::Row, area_h.unwrap_or(0), 0);
    Some(kw.sizer().floor(clamp_size(
        width,
        kw.min(&f.c.min_width, Some(0), 0),
        kw.max(&f.c.max_width, Some(0), 0),
    )))
}

/// The size of a baseline-aligned item `p` in a grid area `area_width`
/// wide whose height is not known yet (its row is being sized): its
/// border box — the width it will have, the height it has unstretched —
/// and its top and bottom margins.
pub(super) fn baseline_size(
    dom: &Dom<TuiExt>,
    p: &Placed,
    container: &ComputedStyle,
    area_width: u16,
) -> ((u16, u16), (i32, i32)) {
    let f = ItemFit::new(dom, p, container, area_width);
    let width = f.width(dom, None);
    let height = f.height(dom, width, None, false);
    ((width, height), (f.m.top, f.m.bottom))
}

/// `extent` less the margins `a` and `b`, at least 0.
fn fill(extent: u16, a: i32, b: i32) -> u16 {
    (i32::from(extent) - a - b).clamp(0, i32::from(u16::MAX)) as u16
}

/// What sizes an item in its area: its style, its margins (against the
/// area's width), which of them are `auto`, its preferred aspect ratio
/// and its self-alignment on each axis.
struct ItemFit<'a> {
    item: &'a Item,
    c: Rc<ComputedStyle>,
    /// The area's width: the containing block of the item's percentages.
    cb: u16,
    m: Sides<i32>,
    auto: Sides<bool>,
    /// §6.2: a preferred aspect ratio keeps `normal` from stretching —
    /// such an item is sized as a block-level box: its `auto` width
    /// fills the area, its `auto` height follows the ratio.
    ratio: Option<AspectRatio>,
    justify: Alignment,
    align: Alignment,
}

impl<'a> ItemFit<'a> {
    fn new(dom: &Dom<TuiExt>, p: &'a Placed, container: &ComputedStyle, cb: u16) -> Self {
        let c = p.item.computed(dom);
        let auto = |side: &MarginValue, trimmed: bool| side.is_auto() && !trimmed;
        Self {
            item: &p.item,
            cb,
            m: super::margins(dom, p, cb),
            auto: Sides {
                top: auto(&c.margin.top, p.trim.top),
                right: auto(&c.margin.right, p.trim.right),
                bottom: auto(&c.margin.bottom, p.trim.bottom),
                left: auto(&c.margin.left, p.trim.left),
            },
            ratio: c.aspect_ratio.filter(|r| r.value().is_some()),
            justify: self_alignment(c.justify_self, container.justify_items),
            align: self_alignment(c.align_self, container.align_items),
            c,
        }
    }

    /// The border-box width in its area, `area_h` tall
    /// (`None` while the rows are being sized): the declared width, else
    /// stretched under `normal` / `stretch` (a ratio item's from a
    /// definite height), else `fit-content`; clamped by `min-width` /
    /// `max-width`.
    fn width(&self, dom: &Dom<TuiExt>, area_h: Option<u16>) -> u16 {
        let (c, item, cb) = (&self.c, self.item, self.cb);
        let available_w = fill(cb, self.m.left, self.m.right);
        let budget = area_h.unwrap_or(0);
        let kw = item.keywords(dom, c, Direction::Row, budget, cb);
        let stretch = !self.auto.left
            && !self.auto.right
            && matches!(self.justify.keyword, Align::Normal | Align::Stretch);
        let width = match kw.size(&c.width, Some(cb), available_w) {
            Some(w) => w,
            None => match self.transferred_width(dom, area_h) {
                Some(w) => w,
                None if stretch => available_w,
                // Fit-content: the available space clamped between the
                // min- and max-content sizes (CSS Sizing 3 §3.1).
                None => {
                    let min = item.content_extreme(dom, Direction::Row, budget, cb, false);
                    let max = item.content_extreme(dom, Direction::Row, budget, cb, true);
                    max.min(min.max(available_w))
                }
            },
        };
        kw.sizer().floor(clamp_size(
            width,
            kw.min(&c.min_width, Some(cb), available_w),
            kw.max(&c.max_width, Some(cb), available_w),
        ))
    }

    /// The width an `auto`-width item's preferred aspect ratio gives it
    /// from a definite height (CSS Sizing 4 §5.1) in an area `area_h`
    /// tall (`None` while the rows are being sized): its declared height,
    /// or its stretched one under `align-self: stretch` once the area is
    /// known. `None` without a ratio or such a height, or when
    /// `justify-self: stretch` makes the width definite itself (the
    /// ratio then has nothing to size).
    fn transferred_width(&self, dom: &Dom<TuiExt>, area_h: Option<u16>) -> Option<u16> {
        let (c, item, cb) = (&self.c, self.item, self.cb);
        let ratio = self.ratio?;
        if !matches!(c.width, Size::Auto) || self.justify.keyword == Align::Stretch {
            return None;
        }
        let available_h = fill(area_h.unwrap_or(0), self.m.top, self.m.bottom);
        let available_w = fill(cb, self.m.left, self.m.right);
        let stretched = area_h.is_some()
            && self.align.keyword == Align::Stretch
            && !self.auto.top
            && !self.auto.bottom;
        let height = item
            .keywords(dom, c, Direction::Column, available_w, cb)
            .size(&c.height, area_h, available_h)
            .or(stretched.then_some(available_h))?;
        aspect_cross_from_main(height, ratio, Direction::Column, c, cb)
    }

    /// The border-box height at `width` in an area `area_h` tall (`None`
    /// while the rows are being sized): the declared height, else the
    /// area's less the margins when `stretch`, else from the ratio, else
    /// the content's; clamped by `min-height` / `max-height`.
    fn height(&self, dom: &Dom<TuiExt>, width: u16, area_h: Option<u16>, stretch: bool) -> u16 {
        let (c, item, cb) = (&self.c, self.item, self.cb);
        let available_h = fill(area_h.unwrap_or(0), self.m.top, self.m.bottom);
        let kw = item.keywords(dom, c, Direction::Column, width, cb);
        let height = match kw.size(&c.height, area_h, available_h) {
            Some(h) => h,
            None if stretch => available_h,
            None => self
                .ratio
                .and_then(|r| aspect_cross_from_main(width, r, Direction::Row, c, cb))
                .unwrap_or_else(|| item.intrinsic_size(dom, Direction::Column, width, cb)),
        };
        kw.sizer().floor(clamp_size(
            height,
            kw.min(&c.min_height, area_h, available_h),
            kw.max(&c.max_height, area_h, available_h),
        ))
    }
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
