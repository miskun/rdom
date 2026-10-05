//! The intrinsic cross size of a multi-line flex container, or of a
//! single-line row (CSS Flexbox §9.9.2): where its items break into
//! lines, for `flex::lines_cross_size` to lay the lines out.

use rdom_core::{Dom, NodeId};

use super::{Keywords, Measure, contribution::box_contribution};
use crate::ext::TuiExt;
use crate::layout::Direction;
use crate::render::layout_pass::box_sizing::Sizer;
use crate::render::layout_pass::flex::item::FlexItem;
use crate::style::ComputedStyle;

/// The cross size along `query` of the flex container `id` — multi-line,
/// or a single-line row (`query` the column axis), whose one line holds
/// every item —
/// whose in-flow items are `children`. `cross_budget` is the extent the
/// container is measured against on its other axis (a row's border-box
/// width), `cb_width` its containing block's width.
///
/// The items break at the container's inner main size: a row's content
/// width (its declared width, else `cross_budget`); a column's content
/// height — its declared height or, when that is `auto`, its content
/// height clamped by `max-height`, which a column whose height is its
/// content's never exceeds, so it is one line (§9.3: an indefinite main
/// size uses the max main size). A column's items are measured
/// unwrapped: its width is what is being measured.
pub(super) fn wrapped_cross_size(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    children: &[FlexItem],
    query: Direction,
    cross_budget: u16,
    cb_width: u16,
) -> u16 {
    let (gutter_col, gutter_row) = crate::render::layout_pass::gutter_axes(computed, false, false);
    match query {
        // A row's lines stack vertically.
        Direction::Column => {
            let kw = Keywords::new(dom, id, computed, Direction::Row, 0, cb_width);
            let basis = (cb_width > 0).then_some(cb_width);
            let outer = kw
                .size(&computed.width, basis, cross_budget)
                .unwrap_or(cross_budget);
            let outer = kw
                .max(&computed.max_width, basis, cross_budget)
                .map_or(outer, |m| outer.min(m));
            let outer = kw
                .min(&computed.min_width, basis, cross_budget)
                .map_or(outer, |m| outer.max(m));
            let main = outer
                .saturating_sub(kw.sizer().chrome())
                .saturating_sub(u16::from(gutter_col));
            crate::render::layout_pass::flex::lines_cross_size(dom, id, children, main, 0, main)
        }
        // A column's lines sit side by side.
        Direction::Row => {
            let outer = box_contribution(
                dom,
                id,
                computed,
                Direction::Column,
                u16::MAX,
                cb_width,
                Measure::MaxContent,
            );
            let main = outer
                .saturating_sub(Sizer::vertical(computed, cb_width).chrome())
                .saturating_sub(u16::from(gutter_row));
            crate::render::layout_pass::flex::lines_cross_size(dom, id, children, main, u16::MAX, 0)
        }
    }
}
