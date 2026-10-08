//! Column measures (CSS 2.1 §17.5.2.2 steps 1–2; CSS Tables 3 "computing
//! cell measures" and "computing column measures"): each column's
//! min-content and max-content width, its percentage, and whether a
//! length constrains it — from its `table-column` / `table-column-group`
//! boxes and the cells that span it alone, then the cells that span more
//! than one column, fewest columns first, spreading what their columns
//! lack over them.
//!
//! A cell's measures are its border box's (CSS Tables 3: the outer
//! min-content width is `max(min-width, min-content)`; the outer
//! max-content width `max(min-width, width, min-content)` when a length
//! constrains it, else `max(min-width, min-content, min(max-width,
//! max-content))`), less — in the collapsing model — its borders, which
//! sit on the lines. Widths are whole cells.

use rdom_core::Dom;

use super::Model;
use super::anonymous;
use super::grid::{Grid, GridCell};
use super::lines::Lines;
use super::structure::{Cell, Structure};
use crate::ext::TuiExt;
use crate::layout::{Direction, Size};
use crate::node::TuiNodeExt;
use crate::render::layout_pass::box_sizing::Sizer;
use crate::render::layout_pass::intrinsic::{content_max_size, content_min_size};
use crate::render::layout_pass::shares::Rolling;

/// One column's measures.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct ColumnMeasure {
    /// Min-content width.
    pub(super) min: u16,
    /// Max-content width (at least `min`).
    pub(super) max: u16,
    /// Its percentage of the table's width, if it has one.
    pub(super) percent: Option<f32>,
    /// A length width constrains it (CSS Tables 3's "constrained").
    pub(super) constrained: bool,
}

/// A cell's measures.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct CellMeasure {
    pub(super) min: u16,
    pub(super) max: u16,
    pub(super) percent: Option<f32>,
    pub(super) constrained: bool,
}

/// `cell`'s measures (see the module docs), its borders taken off in the
/// collapsing model.
pub(super) fn cell_measure(dom: &Dom<TuiExt>, cell: &GridCell, model: Model) -> CellMeasure {
    let id = match &cell.cell {
        Cell::Element(id) => *id,
        Cell::Anonymous(a) => {
            let (min, max) = (
                anonymous::width(dom, a, false, 0),
                anonymous::width(dom, a, true, 0),
            );
            return CellMeasure {
                min,
                max: max.max(min),
                percent: None,
                constrained: false,
            };
        }
    };
    let Some(c) = dom.node(id).computed_rc() else {
        return CellMeasure::default();
    };
    let sizer = Sizer::horizontal(&c, 0);
    let content_min = content_min_size(dom, id, Direction::Row, 0, 0);
    let content_max = content_max_size(dom, id, Direction::Row, 0, 0);
    let min_width = c.min_width.cells(None).map_or(0, |w| sizer.outer(w));
    let max_width = c.max_width.cells(None).map(|w| sizer.outer(w));
    let (width, percent) = match &c.width {
        Size::Fixed(n) => (Some(sizer.outer(*n)), None),
        Size::Percent(p) => (None, Some(*p)),
        _ => (None, None),
    };
    let min = min_width.max(content_min);
    let max = match width {
        Some(w) => min_width.max(w).max(content_min),
        None => min_width
            .max(content_min)
            .max(content_max.min(max_width.unwrap_or(u16::MAX))),
    };
    let borders = match model {
        Model::Collapse => c.border.left.cells().saturating_add(c.border.right.cells()),
        Model::Separate { .. } => 0,
    };
    CellMeasure {
        min: min.saturating_sub(borders),
        max: max.saturating_sub(borders),
        percent,
        constrained: width.is_some(),
    }
}

/// Every column's measures.
pub(super) fn measures(
    dom: &Dom<TuiExt>,
    structure: &Structure,
    grid: &Grid,
    lines: &Lines,
    model: Model,
) -> Vec<ColumnMeasure> {
    let mut columns = vec![ColumnMeasure::default(); grid.columns];
    // CSS Tables 3: a column (group) box's outer min-content width is
    // `max(min-width, width)` — a length width is a floor — and its
    // max-content width the same; a percentage makes a percent column.
    for (c, source) in structure.columns.iter().enumerate() {
        for id in [source.group, source.column].into_iter().flatten() {
            let Some(s) = dom.node(id).computed() else {
                continue;
            };
            let col = &mut columns[c];
            match &s.width {
                Size::Fixed(w) => {
                    col.min = col.min.max(*w);
                    col.max = col.max.max(*w);
                    col.constrained = true;
                }
                Size::Percent(p) => col.percent = Some(col.percent.map_or(*p, |q| q.max(*p))),
                _ => {}
            }
        }
    }
    let mut spanning = Vec::new();
    for cell in &grid.cells {
        let m = cell_measure(dom, cell, model);
        if cell.columns == 1 {
            let col = &mut columns[cell.column];
            col.min = col.min.max(m.min);
            col.max = col.max.max(m.max);
            col.constrained |= m.constrained;
            if let Some(p) = m.percent {
                col.percent = Some(col.percent.map_or(p, |q| q.max(p)));
            }
        } else {
            spanning.push((cell, m));
        }
    }
    // Spanning cells, fewest columns first (CSS Tables 3: "cells of span
    // up to N"), each in tree order.
    spanning.sort_by_key(|(cell, _)| cell.columns);
    for (cell, m) in spanning {
        let range = cell.column..cell.column_end().min(columns.len());
        let inner = lines.inner_vertical(cell.column, cell.column_end());
        spread(
            &mut columns[range.clone()],
            m.min.saturating_sub(inner),
            Field::Min,
        );
        spread(
            &mut columns[range.clone()],
            m.max.saturating_sub(inner),
            Field::Max,
        );
        if let Some(p) = m.percent {
            spread_percent(&mut columns[range], p);
        }
    }
    for c in &mut columns {
        c.max = c.max.max(c.min);
    }
    columns
}

#[derive(Clone, Copy, PartialEq)]
enum Field {
    Min,
    Max,
}

/// Grow `columns` so that their `field` sums to at least `need`: the
/// shortfall spread in whole cells — the min-content shortfall over the
/// columns by how far each could still grow (max − min), else by their
/// max-content widths, else equally; the max-content shortfall by their
/// max-content widths, else equally (CSS Tables 3 distributes a spanning
/// cell's excess this way, its percent and constrained refinements
/// aside).
fn spread(columns: &mut [ColumnMeasure], need: u16, field: Field) {
    let get = |c: &ColumnMeasure| match field {
        Field::Min => c.min,
        Field::Max => c.max,
    };
    let have: u32 = columns.iter().map(|c| u32::from(get(c))).sum();
    let short = u32::from(need).saturating_sub(have);
    if short == 0 || columns.is_empty() {
        return;
    }
    let growth: Vec<f64> = columns
        .iter()
        .map(|c| f64::from(c.max.saturating_sub(c.min)))
        .collect();
    let maxes: Vec<f64> = columns.iter().map(|c| f64::from(c.max)).collect();
    let weights = if field == Field::Min && growth.iter().any(|&w| w > 0.0) {
        growth
    } else if maxes.iter().any(|&w| w > 0.0) {
        maxes
    } else {
        vec![1.0; columns.len()]
    };
    let total: f64 = weights.iter().sum();
    let mut shares = Rolling::new(f64::from(short), total);
    for (c, w) in columns.iter_mut().zip(weights) {
        let add = shares.share(w).min(u32::from(u16::MAX)) as u16;
        match field {
            Field::Min => {
                c.min = c.min.saturating_add(add);
                c.max = c.max.max(c.min);
            }
            Field::Max => c.max = c.max.saturating_add(add),
        }
    }
}

/// A spanning cell's percentage `p` beyond its columns' own goes to its
/// columns without one, by their max-content widths (else equally).
fn spread_percent(columns: &mut [ColumnMeasure], p: f32) {
    let have: f32 = columns.iter().filter_map(|c| c.percent).sum();
    let free: Vec<usize> = (0..columns.len())
        .filter(|&i| columns[i].percent.is_none())
        .collect();
    if p <= have || free.is_empty() {
        return;
    }
    let weights: Vec<f32> = free.iter().map(|&i| f32::from(columns[i].max)).collect();
    let total: f32 = weights.iter().sum();
    for (k, &i) in free.iter().enumerate() {
        let share = if total > 0.0 {
            weights[k] / total
        } else {
            1.0 / free.len() as f32
        };
        columns[i].percent = Some((p - have) * share);
    }
}
