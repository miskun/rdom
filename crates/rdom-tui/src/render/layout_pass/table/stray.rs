//! The anonymous table around table parts outside a table (CSS 2.1
//! §17.2.1 rule 3): block flow partitions a run of them as one table run
//! (`block::RunKind::Table`) and lays it out, or measures it, here — a
//! table whose style is an anonymous box's that is a `table`.

use rdom_core::{Dom, NodeId};

use super::{Measure, TableBox, place, size_of, solve};
use crate::ext::TuiExt;
use crate::layout::{Direction, LayoutRect};
use crate::render::box_tree::BoxItem;
use crate::style::ComputedStyle;

/// The anonymous table's style: an anonymous box's (inheriting from
/// `parent`, every other property initial) that is a `table`.
fn anonymous_style(dom: &Dom<TuiExt>, parent: NodeId) -> ComputedStyle {
    let parent = dom
        .node(parent)
        .ext()
        .and_then(|e| e.computed.clone())
        .unwrap_or_else(|| std::rc::Rc::new(ComputedStyle::initial()));
    let mut style = crate::style::cascade::anonymous_box_style(&parent);
    style.display = crate::layout::Display::Block;
    style.flow = crate::layout::Flow::Table;
    style
}

/// The min-content (`max_content` false) or max-content width of the
/// anonymous table around `items`, a run of `parent`'s box items.
pub(in crate::render::layout_pass) fn anonymous_width(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    items: &[BoxItem],
    max_content: bool,
) -> u16 {
    let measure = if max_content {
        Measure::MaxContent
    } else {
        Measure::MinContent
    };
    let style = anonymous_style(dom, parent);
    let table = TableBox::Anonymous { parent, items };
    size_of(dom, table, &style, Direction::Row, 0, 0, measure)
}

/// The anonymous table's used width in `available` cells: as an `auto`
/// table's, shrink-to-fit (§17.5.2.2).
fn anonymous_used_width(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    items: &[BoxItem],
    available: u16,
) -> u16 {
    let min = anonymous_width(dom, parent, items, false);
    let max = anonymous_width(dom, parent, items, true);
    max.min(available.max(min))
}

/// The height of the anonymous table around `items` in a containing block
/// `available` cells wide.
pub(in crate::render::layout_pass) fn anonymous_height(
    dom: &Dom<TuiExt>,
    parent: NodeId,
    items: &[BoxItem],
    available: u16,
) -> u16 {
    let width = anonymous_used_width(dom, parent, items, available);
    let style = anonymous_style(dom, parent);
    let table = TableBox::Anonymous { parent, items };
    size_of(
        dom,
        table,
        &style,
        Direction::Column,
        width,
        available,
        Measure::MaxContent,
    )
}

/// Lay the anonymous table around `items` out at `at` — its top-left
/// corner and the containing block's width — and return its height.
pub(in crate::render::layout_pass) fn layout_anonymous(
    dom: &mut Dom<TuiExt>,
    parent: NodeId,
    items: &[BoxItem],
    at: LayoutRect,
) -> u16 {
    let width = anonymous_used_width(dom, parent, items, at.width);
    let style = anonymous_style(dom, parent);
    let table = TableBox::Anonymous { parent, items };
    let solved = solve(dom, table, &style, width, at.width);
    let (top, bottom) = place::caption_heights(dom, &solved.structure, width);
    let height = top
        .saturating_add(solved.box_height())
        .saturating_add(bottom);
    let rect = LayoutRect::new(at.x, at.y, width, height);
    let boxes = place::place(dom, table, rect, solved);
    debug_assert!(
        boxes.is_empty(),
        "a run of table parts wraps no text of its parent"
    );
    height
}
