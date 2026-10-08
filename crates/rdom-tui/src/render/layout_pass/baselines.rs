//! A box's first and last baselines (CSS 2.1 §10.8.1: an inline-block's
//! baseline is "the baseline of its last line box in the normal flow";
//! CSS Box Alignment 3 §9.1: a block container's first and last baselines
//! are its first and last in-flow line boxes'), as the rows its text sits
//! on — each line box's glyph row, wherever its `line-height` and the
//! `vertical-align` of its content put it.
//!
//! One model: the lines are the ones layout packs, measured by the block
//! flow's own measurement (`block::measure::baselines`, the flow run with
//! each inline run packed and each block-level child's baselines taken
//! in turn) — the inline-block's in its line (`inline::vertical`), a flex
//! or grid item's for baseline alignment (`items::baseline`). A flex or
//! grid container's are its first and last content rows (C6-ALIGN).
//! Memoized for the layout pass, as the content sizes are.

use rdom_core::{Dom, NodeId};

use super::box_sizing::Sizer;
use super::dispatch::ChildrenLayout;
use crate::ext::TuiExt;
use crate::layout::Direction;
use crate::style::ComputedStyle;

#[cfg(test)]
thread_local! {
    /// Baseline measurements computed, not served from the memo (cost
    /// tests).
    pub(in crate::render::layout_pass) static WALKS: std::cell::Cell<usize> =
        const { std::cell::Cell::new(0) };
}

/// The rows of `id`'s first and last baselines, its border box `width`
/// cells wide in a containing block `cb_width` wide, counted from its
/// border-box top; `None` with no line box in it.
pub(crate) fn content_rows(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    width: u16,
    cb_width: u16,
) -> Option<(u16, u16)> {
    let key = (id, width, cb_width);
    if let Some(rows) = super::intrinsic::baselines_memo(dom, key) {
        return rows;
    }
    #[cfg(test)]
    WALKS.with(|c| c.set(c.get() + 1));
    let rows = measure(dom, id, computed, width, cb_width);
    super::intrinsic::put_baselines_memo(dom, key, rows);
    rows
}

/// [`content_rows`] measured, not looked up.
fn measure(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    width: u16,
    cb_width: u16,
) -> Option<(u16, u16)> {
    let top = computed
        .border
        .top
        .cells()
        .saturating_add(computed.padding.top.resolve(cb_width));
    let (first, last) = match super::dispatch::children_layout(dom, id, computed) {
        // CSS 2.1 §17.5.3: a table's baselines are its first and last
        // rows', from its border-box top (captions included).
        ChildrenLayout::Table => {
            return super::table::baselines(dom, id, computed, width, cb_width);
        }
        ChildrenLayout::Flex | ChildrenLayout::Grid => {
            let chrome = Sizer::vertical(computed, cb_width).chrome();
            let rows =
                super::intrinsic::content_max_size(dom, id, Direction::Column, width, cb_width)
                    .saturating_sub(chrome);
            if rows == 0 {
                return None;
            }
            (0, rows - 1)
        }
        ChildrenLayout::Inline | ChildrenLayout::TextLeaf | ChildrenLayout::Block => {
            let content = width.saturating_sub(Sizer::horizontal(computed, cb_width).chrome());
            let (first, last) = super::block::measure::baselines(dom, id, computed, content)?;
            let row = |r: i32| r.clamp(0, i32::from(u16::MAX)) as u16;
            (row(first), row(last))
        }
    };
    Some((top.saturating_add(first), top.saturating_add(last)))
}
