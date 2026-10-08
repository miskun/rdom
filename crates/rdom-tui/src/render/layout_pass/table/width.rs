//! The table's width over its columns.
//!
//! **Automatic layout** (CSS 2.1 §17.5.2.2; CSS Tables 3 "distributing
//! width to the columns"): the space between the lines is compared with
//! four guesses — every column at its min-content width; percent columns
//! at their percentage of the space (never below their min-content);
//! columns a length constrains, also at their max-content width; every
//! column at its max-content width — and each column's width interpolated
//! between the two guesses the space lies between. Below the first, every
//! column keeps its min-content width (the table overflows its box, which
//! `layout_table` widens to it); past the last, the extra goes to the
//! unconstrained columns by their max-content widths (equally when they
//! have none), else to the constrained ones, else to the percent ones.
//!
//! **Fixed layout** (§17.5.2.1): a column's width is its `table-column`
//! box's `width`, else its share of the first row's cell with a `width`;
//! the columns left share what remains equally. The cells' content plays
//! no part.
//!
//! A `visibility: collapse` column is 0 wide (§17.5.5). Whole cells.

use rdom_core::Dom;

use super::Model;
use super::columns::ColumnMeasure;
use super::grid::Grid;
use super::lines::Lines;
use super::structure::{Cell, Structure};
use crate::ext::TuiExt;
use crate::layout::Size;
use crate::node::TuiNodeExt;
use crate::render::layout_pass::box_sizing::Sizer;
use crate::render::layout_pass::shares::{Rolling, floor_cells};

/// A percent column's width of `space`: its percentage, at least its
/// min-content width.
fn percent_width(c: &ColumnMeasure, p: f32, space: u16) -> u16 {
    let w = floor_cells(f64::from(space) * f64::from(p) / 100.0).min(u32::from(u16::MAX)) as u16;
    w.max(c.min)
}

/// The four guesses (see the module docs) for `space` cells.
fn guesses(columns: &[ColumnMeasure], collapsed: &[bool], space: u16) -> [Vec<u16>; 4] {
    let live = |i: usize| !collapsed.get(i).copied().unwrap_or(false);
    let min: Vec<u16> = columns
        .iter()
        .enumerate()
        .map(|(i, c)| if live(i) { c.min } else { 0 })
        .collect();
    let pct = |i: usize, c: &ColumnMeasure, other: u16| match c.percent {
        Some(p) if live(i) => percent_width(c, p, space),
        _ => other,
    };
    let percent: Vec<u16> = columns
        .iter()
        .enumerate()
        .map(|(i, c)| pct(i, c, min[i]))
        .collect();
    let specified: Vec<u16> = columns
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let own = if c.constrained && live(i) {
                c.max
            } else {
                min[i]
            };
            pct(i, c, own)
        })
        .collect();
    let max: Vec<u16> = columns
        .iter()
        .enumerate()
        .map(|(i, c)| pct(i, c, if live(i) { c.max } else { 0 }))
        .collect();
    [min, percent, specified, max]
}

fn sum(widths: &[u16]) -> u32 {
    widths.iter().map(|&w| u32::from(w)).sum()
}

/// Distribute `space` cells over `columns` (automatic layout).
pub(super) fn distribute(columns: &[ColumnMeasure], collapsed: &[bool], space: u16) -> Vec<u16> {
    let guesses = guesses(columns, collapsed, space);
    let space32 = u32::from(space);
    if space32 <= sum(&guesses[0]) {
        return guesses[0].clone();
    }
    for pair in guesses.windows(2) {
        let (lo, hi) = (&pair[0], &pair[1]);
        let (a, b) = (sum(lo), sum(hi));
        if space32 <= b {
            let mut shares = Rolling::new(f64::from(space32 - a), f64::from(b - a));
            return lo
                .iter()
                .zip(hi)
                .map(|(&l, &h)| l.saturating_add(shares.share(f64::from(h - l)) as u16))
                .collect();
        }
    }
    // Past every guess: the extra to the unconstrained columns, else the
    // constrained ones, else the percent ones.
    let widths = guesses[3].clone();
    let extra = space32 - sum(&widths);
    let live = |i: usize| !collapsed.get(i).copied().unwrap_or(false);
    // 0: unconstrained, 1: no percentage, 2: any live column.
    let in_group = |group: u8, i: usize, c: &ColumnMeasure| {
        live(i)
            && match group {
                0 => c.percent.is_none() && !c.constrained,
                1 => c.percent.is_none(),
                _ => true,
            }
    };
    for group in 0..3u8 {
        let members: Vec<usize> = (0..columns.len())
            .filter(|&i| in_group(group, i, &columns[i]))
            .collect();
        if members.is_empty() {
            continue;
        }
        let weights: Vec<f64> = members.iter().map(|&i| f64::from(columns[i].max)).collect();
        let equal = weights.iter().all(|&w| w == 0.0);
        let total = if equal {
            members.len() as f64
        } else {
            weights.iter().sum()
        };
        let mut shares = Rolling::new(f64::from(extra), total);
        let mut out = widths;
        for (k, &i) in members.iter().enumerate() {
            let w = if equal { 1.0 } else { weights[k] };
            out[i] = out[i].saturating_add(shares.share(w).min(u32::from(u16::MAX)) as u16);
        }
        return out;
    }
    widths
}

/// The columns' min-content total (§17.5.2.2's MIN, lines aside).
pub(super) fn grid_min(columns: &[ColumnMeasure], collapsed: &[bool]) -> u32 {
    sum(&guesses(columns, collapsed, 0)[0])
}

/// The columns' max-content total (§17.5.2.2's MAX, lines aside): the
/// max-content widths, made wide enough that each percent column's
/// max-content width is its percentage of the whole and the other
/// columns fit in what the percentages leave (CSS Tables 3's
/// max-content width of a table with percent columns).
pub(super) fn grid_max(columns: &[ColumnMeasure], collapsed: &[bool]) -> u32 {
    let live = |i: usize| !collapsed.get(i).copied().unwrap_or(false);
    let plain: u32 = columns
        .iter()
        .enumerate()
        .filter(|&(i, c)| live(i) && c.percent.is_none())
        .map(|(_, c)| u32::from(c.max))
        .sum();
    let all: u32 = columns
        .iter()
        .enumerate()
        .filter(|&(i, _)| live(i))
        .map(|(_, c)| u32::from(c.max))
        .sum();
    let percent: f64 = columns
        .iter()
        .enumerate()
        .filter(|&(i, _)| live(i))
        .filter_map(|(_, c)| c.percent)
        .map(f64::from)
        .sum::<f64>()
        .min(100.0);
    let mut need = f64::from(all);
    for (i, c) in columns.iter().enumerate() {
        if let Some(p) = c.percent.filter(|&p| p > 0.0 && live(i)) {
            need = need.max(f64::from(c.max) * 100.0 / f64::from(p));
        }
    }
    if percent > 0.0 && percent < 100.0 {
        need = need.max(f64::from(plain) * 100.0 / (100.0 - percent));
    }
    floor_cells(need.ceil())
}

/// Fixed layout (§17.5.2.1): the columns' widths in `space` cells.
pub(super) fn fixed(
    dom: &Dom<TuiExt>,
    structure: &Structure,
    grid: &Grid,
    lines: &Lines,
    model: Model,
    space: u16,
) -> Vec<u16> {
    let n = grid.columns;
    let mut widths: Vec<Option<u16>> = vec![None; n];
    let of_space = |p: f32| floor_cells(f64::from(space) * f64::from(p) / 100.0) as u16;
    // A column box with a width fixes its column.
    for (c, source) in structure.columns.iter().enumerate().take(n) {
        for id in [source.column, source.group].into_iter().flatten() {
            if widths[c].is_some() {
                break;
            }
            widths[c] = match dom.node(id).computed().map(|s| s.width.clone()) {
                Some(Size::Fixed(w)) => Some(w),
                Some(Size::Percent(p)) => Some(of_space(p)),
                _ => None,
            };
        }
    }
    // Else the first row's cell with a width, shared by its columns.
    for cell in grid.cells.iter().filter(|c| c.row == 0) {
        let Cell::Element(id) = &cell.cell else {
            continue;
        };
        let Some(c) = dom.node(*id).computed_rc() else {
            continue;
        };
        let border_box = match &c.width {
            Size::Fixed(w) => Sizer::horizontal(&c, 0).outer(*w),
            Size::Percent(p) => of_space(*p),
            _ => continue,
        };
        let borders = match model {
            Model::Collapse => c.border.left.cells().saturating_add(c.border.right.cells()),
            Model::Separate { .. } => 0,
        };
        let range = cell.column..cell.column_end().min(n);
        let open: Vec<usize> = range.filter(|&i| widths[i].is_none()).collect();
        let total = border_box
            .saturating_sub(borders)
            .saturating_sub(lines.inner_vertical(cell.column, cell.column_end()));
        let mut shares = Rolling::new(f64::from(total), open.len() as f64);
        for i in open {
            widths[i] = Some(shares.share(1.0) as u16);
        }
    }
    let used: u32 = widths.iter().flatten().map(|&w| u32::from(w)).sum();
    let open = widths.iter().filter(|w| w.is_none()).count();
    let mut shares = Rolling::new(
        f64::from(u32::from(space).saturating_sub(used)),
        open as f64,
    );
    widths
        .into_iter()
        .enumerate()
        .map(|(i, w)| {
            if grid.collapsed_columns.get(i).copied().unwrap_or(false) {
                return 0;
            }
            w.unwrap_or_else(|| shares.share(1.0) as u16)
        })
        .collect()
}
