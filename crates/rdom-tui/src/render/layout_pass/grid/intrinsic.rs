//! A grid container's content size (CSS Grid 2 §5.2): "the sum of the
//! grid container's track sizes (including gutters) in the appropriate
//! axis, when the grid is sized under a max-content constraint
//! (min-content constraint)" — its columns for its width; for its height
//! at a given width, its rows sized to their content at the columns that
//! width gives.

use rdom_core::{Dom, NodeId};

use super::subgrid::Inherit;
use super::template::Bounds;
use super::{AxisContext, Dimension, content_bounds, laid_out_axis, size_grid, stretches};
use crate::ext::TuiExt;
use crate::layout::Direction;
use crate::render::layout_pass::box_sizing::Sizer;
use crate::render::layout_pass::intrinsic::Measure;
use crate::style::ComputedStyle;

use super::sizing::Space;

/// The content size of the grid container `id` along `direction` — its
/// grid, without its padding, border or scrollbar gutter, which the
/// caller adds: on the inline axis its columns sized under `measure`'s
/// constraint; on the block axis its rows at the width `cross_budget`
/// (its border box) leaves its content. `cb_width` is its containing
/// block's width, the basis of its padding percentages.
pub(in crate::render::layout_pass) fn content_size(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    direction: Direction,
    cross_budget: u16,
    cb_width: u16,
    measure: Measure,
) -> u16 {
    // A subgrid measured as its parent arranges it takes the parent's
    // laid-out tracks (§9).
    let inherit = super::subgrid::from_parent(dom, id, computed);
    content_size_with(
        dom,
        id,
        computed,
        direction,
        cross_budget,
        cb_width,
        measure,
        &inherit,
    )
}

/// [`content_size`] for a grid that takes the axes `inherit` names from
/// its parent.
#[allow(clippy::too_many_arguments)]
pub(super) fn content_size_with(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    direction: Direction,
    cross_budget: u16,
    cb_width: u16,
    measure: Measure,
    inherit: &Inherit,
) -> u16 {
    let total = match direction {
        Direction::Row => {
            let (min, max) = content_bounds(computed, Dimension::Columns);
            // A percentage gap has no basis: 0 (CSS Box Alignment 3 §8.3).
            let gap = computed.column_gap.resolve(0);
            let columns = AxisContext {
                space: match measure {
                    Measure::MinContent => Space::MinContent,
                    Measure::MaxContent => Space::MaxContent,
                },
                bounds: Bounds {
                    size: None,
                    max,
                    min,
                    gap,
                },
                stretch: stretches(computed, Dimension::Columns),
            };
            size_grid(dom, id, computed, columns, None, inherit, false)
                .columns
                .total()
        }
        Direction::Column => {
            let chrome = Sizer::horizontal(computed, cb_width).chrome();
            let gutter_columns =
                crate::render::layout_pass::gutters(computed, false, false).columns();
            let width = cross_budget
                .saturating_sub(chrome)
                .saturating_sub(gutter_columns);
            let columns = laid_out_axis(computed, Dimension::Columns, Some(width));
            let rows = laid_out_axis(computed, Dimension::Rows, None);
            size_grid(dom, id, computed, columns, Some(rows), inherit, false)
                .rows
                .map_or(0, |r| r.total())
        }
    };
    total.min(u32::from(u16::MAX)) as u16
}
