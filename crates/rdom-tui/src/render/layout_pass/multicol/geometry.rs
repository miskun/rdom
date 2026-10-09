//! The column boxes' count and width (CSS Multi-column 1 §3.4's
//! pseudo-algorithm), in whole cells.

use crate::style::ComputedStyle;

/// A multi-column container's columns: how many, how wide, and the gap
/// between two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Columns {
    pub(crate) count: u16,
    pub(crate) width: u16,
    pub(crate) gap: u16,
}

impl Columns {
    /// The distance from one column's left edge to the next's.
    pub(crate) fn pitch(self) -> i32 {
        i32::from(self.width) + i32::from(self.gap)
    }
}

/// The used `column-gap` of a multi-column container whose content box is
/// `available` cells wide: `normal` is 1em (§3.5) — one cell, keeping two
/// columns' words apart (DIVERGENCES) — and a percentage is of the content
/// box.
pub(crate) fn gap(computed: &ComputedStyle, available: u16) -> u16 {
    match computed.column_gap {
        crate::layout::GapValue::Normal => 1,
        ref g => g.resolve(available),
    }
}

/// §3.4: the count `N` and width `W` of the columns in a content box `U`
/// (`available`) cells wide. Whole cells: `W` is floored, so the columns
/// may leave a few cells at the inline end (DIVERGENCES); never less than
/// one column of one cell, and never more columns than one cell wide fit.
pub(crate) fn columns(computed: &ComputedStyle, available: u16) -> Columns {
    use crate::layout::ColumnCount;
    let gap = gap(computed, available);
    let (u, g) = (i64::from(available), i64::from(gap));
    let width = computed.multicol.column_width.cells();
    let count = match computed.multicol.column_count {
        ColumnCount::Auto => None,
        ColumnCount::Count(n) => Some(i64::from(n.min(u32::from(u16::MAX)))),
    };
    // (03) / (04): as many columns of at least `column-width` as fit, at
    // most `column-count`.
    let fit = |w: i64| ((u + g) / (w + g).max(1)).max(1);
    let n = match (width, count) {
        (None, Some(n)) => n,
        (Some(w), None) => fit(i64::from(w)),
        (Some(w), Some(n)) => n.min(fit(i64::from(w))),
        (None, None) => 1,
    }
    .max(1)
    // A column is at least a cell: no more columns than one-cell columns
    // fit (`column-count: 65535` in 40 cells is 20 with a one-cell gap —
    // C15G-MULTICOL-COST).
    .min(fit(1));
    let w = match width {
        // (02): the count shares the width out.
        None => (u - (n - 1) * g) / n,
        // (03) / (04): the columns widen to fill it.
        Some(_) => (u + g) / n - g,
    };
    Columns {
        count: n.clamp(1, i64::from(u16::MAX)) as u16,
        width: w.clamp(1, i64::from(u16::MAX)) as u16,
        gap,
    }
}

/// Whether `computed`'s box is a multi-column container (§2,
/// [`ComputedStyle::is_multicol_container`]).
pub(crate) fn is_multicol(computed: &ComputedStyle) -> bool {
    computed.is_multicol_container()
}
