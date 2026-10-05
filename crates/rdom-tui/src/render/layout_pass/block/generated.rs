//! A block-level `::before` / `::after` in its host's block flow (CSS
//! 2.1 §12.1, CSS Pseudo 4 §2; `inline::generated::is_block_pseudo`):
//! the host's first / last block-level box, laid out as a block child
//! would be — its margins in the flow's collapsing accumulator, its width
//! the containing block's less its margins (or its own), `clear` moving it
//! below the floats it names (§9.5.2), its height its own or its content's
//! — and kept as a generated box among the host's anonymous boxes
//! (`AnonymousIfc::generated`), which paint and hit-testing already read.
//!
//! Its content is laid out by the box tree's generated item
//! (`items::AnonymousItem::pseudo`): its generated text packed as one
//! inline formatting context in its content box, or — a `flex` / `grid`
//! one — its one anonymous item by flex or grid layout
//! (C8G-PSEUDO-ATOMS).

use rdom_core::{Dom, NodeId};

use super::margin_collapse::MarginAccumulator;
use crate::ext::{AnonymousIfc, GeneratedBox, PseudoSlot, TuiExt};
use crate::layout::{Direction, LayoutRect, Size};
use crate::render::inline::InlineLayout;
use crate::render::layout_pass::box_sizing::Sizer;
use crate::render::layout_pass::items::AnonymousItem;
use crate::style::ComputedStyle;

/// The computed style of `host`'s `slot` pseudo-element.
fn style_of(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
) -> Option<std::rc::Rc<ComputedStyle>> {
    let ext = dom.node(host).ext()?;
    match slot {
        PseudoSlot::Before => ext.computed_before.clone(),
        PseudoSlot::After => ext.computed_after.clone(),
    }
}

/// The pseudo-element as a box of its own, which measures and lays out
/// its content (`items::AnonymousItem`): its text, or — a `flex` / `grid`
/// one — its one anonymous item by flex or grid layout.
fn item(dom: &Dom<TuiExt>, host: NodeId, slot: PseudoSlot) -> Option<AnonymousItem> {
    AnonymousItem::pseudo(dom, host, slot)
}

/// The box's border-box width in a containing block `cb_width` cells
/// wide: its own `width`, else the containing block's less its margins
/// (CSS 2.1 §10.3.3), clamped by `min-width` / `max-width`.
fn border_width(c: &ComputedStyle, cb_width: u16) -> u16 {
    let sizer = Sizer::horizontal(c, cb_width);
    let margins = c
        .margin
        .left
        .resolve(cb_width)
        .saturating_add(c.margin.right.resolve(cb_width));
    let auto = (i32::from(cb_width) - i32::from(margins)).clamp(0, i32::from(u16::MAX)) as u16;
    let own = match &c.width {
        Size::Auto | Size::Intrinsic(_) | Size::Flex(_) => None,
        size => sizer.outer_opt(size.cells(Some(cb_width))),
    };
    let min = sizer.outer_opt(c.min_width.cells(Some(cb_width)));
    let max = sizer.outer_opt(c.max_width.cells(Some(cb_width)));
    sizer.floor(crate::layout::clamp_size(own.unwrap_or(auto), min, max))
}

/// The box's border-box height for a border box `width` cells wide: its
/// own `height`, else its packed content's rows, plus its padding and
/// border, clamped by `min-height` / `max-height`.
fn border_height(
    dom: &Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
    c: &ComputedStyle,
    width: u16,
    cb_width: u16,
) -> u16 {
    let sizer = Sizer::vertical(c, cb_width);
    let own = match &c.height {
        Size::Auto | Size::Intrinsic(_) | Size::Flex(_) => None,
        size => sizer.outer_opt(size.cells(None)),
    };
    let natural = own.unwrap_or_else(|| {
        item(dom, host, slot).map_or(sizer.chrome(), |i| {
            i.content_size(dom, Direction::Column, width, true, cb_width)
        })
    });
    let min = sizer.outer_opt(c.min_height.cells(None));
    let max = sizer.outer_opt(c.max_height.cells(None));
    sizer.floor(crate::layout::clamp_size(natural, min, max))
}

/// Where the block flow is when the box is placed.
pub(super) struct GeneratedPlace<'a> {
    /// The containing block's left edge (scrolled).
    pub(super) x: i32,
    pub(super) cb_width: u16,
    pub(super) y_cursor: i32,
    pub(super) margin_acc: &'a mut MarginAccumulator,
    /// Its index in the host's box sequence.
    pub(super) index: usize,
}

/// The index of `host`'s `slot` pseudo-element in its box sequence of
/// `len` items: the first, or the last.
pub(super) fn index(slot: PseudoSlot, len: usize) -> usize {
    match slot {
        PseudoSlot::Before => 0,
        PseudoSlot::After => len.saturating_sub(1),
    }
}

/// Lay `host`'s block-level `slot` pseudo-element out in its block flow:
/// the generated box to keep among the host's anonymous boxes, and the
/// new flow cursor (its bottom border edge; its bottom margin is left in
/// `margin_acc`, as a block child's is).
pub(super) fn lay_out(
    dom: &mut Dom<TuiExt>,
    host: NodeId,
    slot: PseudoSlot,
    at: GeneratedPlace<'_>,
) -> Option<(AnonymousIfc, i32)> {
    let c = style_of(dom, host, slot)?;
    let cb = at.cb_width;
    let mut top = MarginAccumulator::new();
    top.add(vertical_margin(&c.margin.top, cb));
    at.margin_acc.merge(top);
    let mut y = at.y_cursor + i32::from(at.margin_acc.resolved());
    // CSS 2.1 §9.5.2: below the floats its `clear` names.
    let rtl = dom
        .node(host)
        .ext()
        .and_then(|e| e.computed.as_deref())
        .is_some_and(|hc| hc.text_direction == crate::layout::TextDirection::Rtl);
    let (left, right) = c.clear.sides(rtl);
    if left || right {
        let floats = crate::render::layout_pass::float::with_area(dom, |_, area| {
            area.clearance(left, right)
        });
        if let Some(bottom) = floats {
            y = y.max(bottom);
        }
    }
    let width = border_width(&c, cb);
    let height = border_height(dom, host, slot, &c, width, cb);
    let x = at.x + i32::from(c.margin.left.resolve(cb));
    let rect = LayoutRect::new(x, y, width, height);
    let (h, v) = (Sizer::horizontal(&c, cb), Sizer::vertical(&c, cb));
    let pl = c
        .padding
        .left
        .resolve(cb)
        .saturating_add(c.border.left.cells());
    let pt = c
        .padding
        .top
        .resolve(cb)
        .saturating_add(c.border.top.cells());
    let content = LayoutRect::new(
        x + i32::from(pl),
        y + i32::from(pt),
        width.saturating_sub(h.chrome()),
        height.saturating_sub(v.chrome()),
    );
    let (lines_at, lines) = match item(dom, host, slot) {
        Some(i) => i.lay_out_content(dom, content),
        None => (
            content,
            InlineLayout {
                lines: Vec::new(),
                content_width: content.width,
            },
        ),
    };
    let mut bottom = MarginAccumulator::new();
    bottom.add(vertical_margin(&c.margin.bottom, cb));
    *at.margin_acc = bottom;
    Some((
        AnonymousIfc::new(
            lines_at,
            lines,
            (at.index, at.index + 1),
            Some(GeneratedBox::new(host, slot, rect)),
        ),
        y + i32::from(height),
    ))
}

/// A block-axis margin in cells (`auto` is 0, CSS 2.1 §10.6.3).
fn vertical_margin(m: &crate::layout::MarginValue, cb_width: u16) -> i16 {
    m.resolve(cb_width)
}

/// `host`'s block-level pseudo-elements' contribution to its intrinsic
/// size (`intrinsic::measure_content`): `(widest margin box, rows)` — the
/// rows their margin boxes stack, packed at `content_width` — or `None`
/// when it has none.
pub(in crate::render::layout_pass) fn intrinsic(
    dom: &Dom<TuiExt>,
    host: NodeId,
    content_width: u16,
    max_content: bool,
) -> Option<(u16, u16)> {
    let own = crate::render::inline::generated::block_pseudos(dom, host);
    let mut out: Option<(u16, u16)> = None;
    for (on, slot) in [
        (own.before, PseudoSlot::Before),
        (own.after, PseudoSlot::After),
    ] {
        if !on {
            continue;
        }
        let Some(c) = style_of(dom, host, slot) else {
            continue;
        };
        let cb = content_width;
        let margins_x = c
            .margin
            .left
            .resolve(cb)
            .saturating_add(c.margin.right.resolve(cb))
            .max(0) as u16;
        let natural = || {
            item(dom, host, slot).map_or(0, |i| {
                i.content_size(dom, Direction::Row, 0, max_content, cb)
            })
        };
        let wide = match &c.width {
            Size::Auto | Size::Intrinsic(_) | Size::Flex(_) => natural(),
            _ => border_width(&c, cb),
        }
        .saturating_add(margins_x);
        let width = border_width(&c, cb);
        let margins_y = vertical_margin(&c.margin.top, cb)
            .saturating_add(vertical_margin(&c.margin.bottom, cb))
            .max(0) as u16;
        let rows = border_height(dom, host, slot, &c, width, cb).saturating_add(margins_y);
        let (w, h) = out.unwrap_or((0, 0));
        out = Some((w.max(wide), h.saturating_add(rows)));
    }
    out
}
