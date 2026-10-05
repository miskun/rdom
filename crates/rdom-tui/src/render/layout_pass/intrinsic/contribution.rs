//! A box's intrinsic size contribution (CSS Sizing 3 §5.2): what a box
//! adds to its container's min- / max-content size — its declared size
//! when that is definite, else its content, and either way clamped by
//! its `min-*` / `max-*` ("with … its min and max sizes applied"). Every
//! declared size goes through [`Keywords`], so through the box's
//! [`Sizer`](crate::render::layout_pass::box_sizing::Sizer) (CSS UI 3
//! §3.1, `box-sizing`).

use rdom_core::{Dom, NodeId};

use super::{Keywords, Measure, content_size};
use crate::ext::TuiExt;
use crate::layout::{Direction, IntrinsicSize, Size};
use crate::style::ComputedStyle;

/// `id`'s contribution along `direction`, measured as `measure` asks.
/// `cb_width` is its containing block's width, 0 while that is the very
/// thing being measured (a cyclic percentage, which then behaves as
/// `auto`, CSS Sizing 3 §5.2.1).
pub(super) fn box_contribution(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    direction: Direction,
    cross_budget: u16,
    cb_width: u16,
    measure: Measure,
) -> u16 {
    // TABLE-COLSYNC-1: a table cell's resolved column width is its used
    // main size (a width → Row axis), so a table measures to its laid-out
    // column widths (e.g. when it's a flex item being sized by a scroll
    // wrapper) — final, like a definite size.
    if direction == Direction::Row
        && let Some(w) = dom.node(id).ext().and_then(|e| e.table_used_width)
    {
        return w;
    }
    let kw = Keywords::new(dom, id, computed, direction, cross_budget, cb_width);
    // A percentage resolves against a definite containing block: the
    // width when known; a height's basis is indefinite here.
    let basis = match direction {
        Direction::Row => (cb_width > 0).then_some(cb_width),
        Direction::Column => None,
    };
    // `fit-content` (in a min / max) is the min-content size in a
    // min-content measurement, the max-content size in a max-content one.
    let available = match measure {
        Measure::MinContent => 0,
        Measure::MaxContent => u16::MAX,
    };
    let (declared, min, max) = match direction {
        Direction::Row => (&computed.width, &computed.min_width, &computed.max_width),
        Direction::Column => (&computed.height, &computed.min_height, &computed.max_height),
    };
    let content = |m| content_size(dom, id, computed, direction, cross_budget, cb_width, m);
    let size = match declared {
        // An inline-axis keyword contributes its own content size (CSS
        // Sizing 3 §5.1): `min-content` / `max-content` whatever is
        // being measured, `fit-content` as the measurement goes (its
        // stretch-fit size is unknown here), a `fit-content()` limit
        // capping a max-content contribution.
        Size::Intrinsic(k) if direction == Direction::Row => match (k, measure) {
            (IntrinsicSize::MinContent, _) | (_, Measure::MinContent) => {
                content(Measure::MinContent)
            }
            (IntrinsicSize::MaxContent | IntrinsicSize::FitContent, _) => {
                content(Measure::MaxContent)
            }
            (IntrinsicSize::FitContentLimit(_), Measure::MaxContent) => kw.keyword(k, None, 0),
        },
        // A length, or a percentage / `calc()` against a definite basis,
        // is the border box `box-sizing` makes of it (CSS UI 3 §3.1).
        Size::Fixed(_) | Size::Percent(_) | Size::Calc(_) => kw
            .size(declared, basis, available)
            .unwrap_or_else(|| content(measure)),
        _ => content(measure),
    };
    // CSS 2.1 §10.4: `max-*` caps, then `min-*` floors (it wins).
    let size = kw.max(max, basis, available).map_or(size, |m| size.min(m));
    kw.min(min, basis, available).map_or(size, |m| size.max(m))
}
