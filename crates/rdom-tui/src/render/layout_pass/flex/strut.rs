//! `visibility: collapse` on flex items — CSS Flexbox §4.4 and §9.4
//! step 10.
//!
//! "If any flex items have `visibility: collapse`, note the cross size of
//! the line they're in as the item's strut size, and restart layout from
//! the beginning. In this second layout round, when collecting items
//! into lines, treat the collapsed items as having zero main size. For
//! the rest of the algorithm following that step, ignore the collapsed
//! items entirely (as if they were `display: none`) except that after
//! calculating the cross size of the lines, if any line's cross size is
//! less than the largest strut size among all the collapsed items in the
//! line, set its cross size to that strut size."
//!
//! The first round needs only the lines and their cross sizes, so it
//! is not a whole layout: [`make_struts`] breaks the uncollapsed items
//! into lines and sizes each line holding a collapsed item from its
//! items' hypothetical main sizes — a single-line container's line is
//! its cross size when that is definite — then turns each collapsed item
//! into a strut (`ChildMain::make_strut`), which the second round (the
//! ordinary algorithm, `ChildMain::strut` read where it ignores an
//! item) lays out.

use rdom_core::Dom;

use super::lines::{LineFrame, break_lines, line_cross_size};
use super::main_axis::{ChildMain, MainBudgets};
use crate::ext::TuiExt;
use crate::style::ComputedStyle;

/// Note each collapsed item's strut size and make it a strut. `items`
/// are the container's items as gathered (uncollapsed), `frame` the
/// line frame the container measures its items in, `gap` the main-axis
/// gap between items; `multi_line` whether the container wraps.
pub(super) fn make_struts(
    dom: &Dom<TuiExt>,
    container: &ComputedStyle,
    items: &mut [ChildMain],
    budgets: MainBudgets,
    gap: u16,
    frame: LineFrame,
    multi_line: bool,
) {
    if !items.iter().any(|ci| ci.item.is_collapsed(dom)) {
        return;
    }
    let direction = frame.direction;
    let lines = if multi_line {
        break_lines(dom, items, direction, budgets, gap)
    } else {
        std::iter::once(0..items.len()).collect()
    };
    for range in lines {
        let line = &items[range.clone()];
        if !line.iter().any(|ci| ci.item.is_collapsed(dom)) {
            continue;
        }
        // A single-line container's line is its cross size (§9.4 step 8)
        // when that is known; otherwise the line is as large as its
        // items' outer hypothetical cross sizes, each measured at its
        // hypothetical main size.
        let size = match frame.space.container {
            Some(cross) if !multi_line => cross,
            _ => {
                let main: Vec<u16> = line
                    .iter()
                    .map(|ci| ci.hypothetical(dom, direction, budgets))
                    .collect();
                line_cross_size(dom, container, line, &main, frame, true).0
            }
        };
        for ci in &mut items[range] {
            if ci.item.is_collapsed(dom) {
                ci.make_strut(size);
            }
        }
    }
}
