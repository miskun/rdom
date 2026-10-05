//! Block-level boxes in normal flow beside floats: clearance (CSS 2.1
//! §9.5.2) and the block formatting context roots that must not overlap
//! a float (§9.5).

use rdom_core::{Dom, NodeId};

use super::clear_sides;
use crate::ext::TuiExt;
use crate::layout::{Direction, Size, TextDirection};
use crate::style::ComputedStyle;

/// A block-level box flow layout placed: its containing block (left edge
/// `x0`, `cb_width` cells wide) and the border box it gave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render::layout_pass) struct FlowBox {
    pub(in crate::render::layout_pass) x0: i32,
    pub(in crate::render::layout_pass) cb_width: u16,
    pub(in crate::render::layout_pass) x: i32,
    pub(in crate::render::layout_pass) y: i32,
    pub(in crate::render::layout_pass) width: u16,
    pub(in crate::render::layout_pass) rows: u16,
}

/// Where the block-level box `id` (styled `c`), placed by flow layout at
/// `b`, goes among the floats of `area`, its formatting context's:
///
/// - CSS 2.1 §9.5.2: with `clear`, its top border edge below the bottom
///   outer edge of the lowest float it clears — the greater of that and
///   its hypothetical position, so clearance never moves it up;
/// - §9.5: a box that establishes a block formatting context — `flow-root`,
///   `overflow` other than `visible` / `clip`, an inline-block's block
///   counterpart, … — "must not overlap the margin box of any floats in
///   the same block formatting context": beside them where the band left
///   holds it (an `auto` width shrinks to the band, down to its
///   min-content contribution), else below them.
///
/// `b` unchanged when no float is in the way.
pub(in crate::render::layout_pass) fn beside_floats_in(
    dom: &Dom<TuiExt>,
    area: &super::ExclusionArea,
    id: NodeId,
    c: &ComputedStyle,
    b: FlowBox,
) -> FlowBox {
    {
        if area.is_empty() {
            return b;
        }
        let mut out = b;
        let (left, right) = clear_sides(dom, id, c);
        if let Some(bottom) = area.clearance(left, right) {
            out.y = out.y.max(bottom);
        }
        if !crate::render::layout_pass::block::establishes_bfc(dom, id, c) {
            return out;
        }
        let x1 = b.x0 + i32::from(b.cb_width);
        let ml = i32::from(c.margin.left.resolve(b.cb_width));
        let mr = i32::from(c.margin.right.resolve(b.cb_width));
        let auto = matches!(c.width, Size::Auto);
        let needed = if auto {
            i32::from(crate::render::layout_pass::intrinsic::contribution(
                dom,
                id,
                Direction::Row,
                0,
                b.cb_width,
                false,
            ))
            .max(1)
        } else {
            i32::from(b.width)
        };
        let (y, band) = area.opening(b.x0, x1, out.y, b.rows, |band| {
            band.width() - ml - mr >= needed
        });
        out.y = y;
        if band.start == b.x0 && band.end == x1 {
            return out;
        }
        let room = (band.width() - ml - mr).clamp(0, i32::from(u16::MAX)) as u16;
        if auto {
            out.width = room;
        }
        let rtl = crate::render::box_tree::box_parent(dom, id).and_then(|p| {
            dom.node(p)
                .ext()?
                .computed
                .as_deref()
                .map(|pc| pc.text_direction)
        }) == Some(TextDirection::Rtl);
        out.x = if rtl {
            band.end - mr - i32::from(out.width)
        } else {
            band.start + ml
        };
        out
    }
}
