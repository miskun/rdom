//! Baseline self-alignment in a grid's rows (CSS Grid 2 §10.4, CSS Box
//! Alignment 3 §9): the items whose `align-self` is `baseline` /
//! `last baseline` form baseline-sharing groups — one per row and
//! preference, a spanning item joining its first row's first-baseline
//! group or its last row's last-baseline group (§9.3) — and each item is
//! shimmed so the group's baselines meet, flush with the row's start
//! (`last`: end). The shims are part of the items' row contributions
//! (Grid §11.5 step 1), so the rows hold the aligned groups.
//!
//! An item with an `auto` block-axis margin does not take part (the
//! margin wins, §10.2). The columns have no baseline alignment: a
//! horizontal item has no baseline on the inline axis, and
//! `justify-self: baseline` falls back to `safe self-start` (§4.2,
//! `block::justify_offset`).

use std::collections::BTreeMap;

use rdom_core::Dom;

use super::placement::Placed;
use crate::ext::TuiExt;
use crate::layout::Align;
use crate::render::layout_pass::items::BaselineBox;
use crate::style::ComputedStyle;

/// A baseline-aligned item's place in its group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Shim {
    /// It aligns its last baseline, from the area's end.
    pub(super) last: bool,
    /// The rows between its margin box and the area's start (`last`:
    /// end) that bring its baseline to the group's.
    pub(super) offset: i32,
    /// Its border-box height, measured for the alignment.
    pub(super) height: u16,
}

/// Each of `placed`'s items' shim (`None` when it is not baseline-aligned,
/// or `aligns(k)` is false for the `k`th — not measured at all), the
/// items in grid areas of `widths` (each area's width, by item) of the
/// grid container styled `container`.
pub(super) fn shims(
    dom: &Dom<TuiExt>,
    container: &ComputedStyle,
    placed: &[Placed],
    widths: &[u16],
    aligns: impl Fn(usize) -> bool,
) -> Vec<Option<Shim>> {
    let measured: Vec<Option<(bool, usize, BaselineBox)>> = placed
        .iter()
        .zip(widths)
        .enumerate()
        .map(|(k, (p, &width))| {
            if !aligns(k) {
                return None;
            }
            let c = p.item.computed(dom);
            let value = if c.align_self.keyword == Align::Auto {
                container.align_items.keyword
            } else {
                c.align_self.keyword
            };
            let last = match value {
                Align::Baseline => false,
                Align::LastBaseline => true,
                _ => return None,
            };
            let auto_margin = (c.margin.top.is_auto() && !p.trim.top)
                || (c.margin.bottom.is_auto() && !p.trim.bottom);
            if auto_margin {
                return None;
            }
            let (size, margins) = super::arrange::baseline_size(dom, p, container, width);
            let b = BaselineBox::measure(dom, &p.item, &c, size, margins, width);
            let row = if last { p.rows.end - 1 } else { p.rows.start };
            Some((last, row, b))
        })
        .collect();
    // Per group: how far its items reach above their first baselines
    // (first), or below their last ones (last).
    let mut reach: BTreeMap<(bool, usize), i32> = BTreeMap::new();
    for &(last, row, b) in measured.iter().flatten() {
        let r = if last {
            b.below_last()
        } else {
            b.above_first()
        };
        let e = reach.entry((last, row)).or_insert(r);
        *e = (*e).max(r);
    }
    measured
        .into_iter()
        .map(|m| {
            m.map(|(last, row, b)| {
                let own = if last {
                    b.below_last()
                } else {
                    b.above_first()
                };
                Shim {
                    last,
                    offset: reach[&(last, row)] - own,
                    height: b.height,
                }
            })
        })
        .collect()
}
