//! Vertical alignment in cells (CSS 2.1 §17.5.3): a cell's content at the
//! top of its box (`top`), centred in it (`middle` — the upper of two
//! middles when the free rows are odd), at its bottom (`bottom`), or with
//! its first line's baseline on its row's (`baseline`, and every other
//! `vertical-align` value, which on a cell behaves as `baseline`). A
//! row's baseline is the lowest baseline of its `baseline` cells — a
//! cell's is its first line box's (C9G-ONE-BASELINE's `content_rows`), or
//! the bottom of its content edge when it has none — and the row is tall
//! enough to hold them aligned.

use rdom_core::Dom;

use super::Model;
use super::anonymous;
use super::grid::{Grid, GridCell};
use super::lines::{Lines, cell_border};
use super::structure::Cell;
use crate::ext::TuiExt;
use crate::layout::{Direction, VerticalAlign};
use crate::node::TuiNodeExt;
use crate::render::layout_pass::box_sizing::Sizer;
use crate::render::layout_pass::intrinsic::content_max_size;

/// How a cell places its content (§17.5.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CellAlign {
    Top,
    Middle,
    Bottom,
    Baseline,
}

/// `cell`'s alignment: its `vertical-align` (an anonymous cell's is the
/// initial `baseline`).
pub(super) fn of(dom: &Dom<TuiExt>, cell: &GridCell) -> CellAlign {
    let va = cell
        .element()
        .and_then(|id| dom.node(id).computed().map(|c| c.vertical_align.clone()));
    match va {
        Some(VerticalAlign::Top) => CellAlign::Top,
        Some(VerticalAlign::Middle) => CellAlign::Middle,
        Some(VerticalAlign::Bottom) => CellAlign::Bottom,
        _ => CellAlign::Baseline,
    }
}

/// The rows from the top of `cell`'s first row's track to its baseline,
/// its border box `width` cells wide: its first line box's text row (from
/// its border-box top, `baselines::content_rows`), else the bottom of its
/// content edge — less, in the collapsing model, a top border that sits on
/// the line above the track.
pub(super) fn baseline(
    dom: &Dom<TuiExt>,
    cell: &GridCell,
    lines: &Lines,
    model: Model,
    width: u16,
    cb: u16,
) -> u16 {
    let from_box = match &cell.cell {
        Cell::Anonymous(a) => anonymous::first_baseline(dom, a, width, cb),
        Cell::Element(id) => {
            let Some(c) = dom.node(*id).computed_rc() else {
                return 0;
            };
            crate::render::layout_pass::baselines::content_rows(dom, *id, &c, width, cb)
                .map(|(first, _)| first)
                .unwrap_or_else(|| {
                    let content = content_max_size(dom, *id, Direction::Column, width, cb);
                    let bottom_chrome = c
                        .border
                        .bottom
                        .cells()
                        .saturating_add(c.padding.bottom.resolve(cb));
                    content.saturating_sub(bottom_chrome)
                })
        }
    };
    let on_line = match model {
        Model::Collapse if !cell_border(dom, cell).top.is_none() => {
            lines.horizontal.get(cell.row).copied().unwrap_or(0)
        }
        _ => 0,
    };
    from_box.saturating_sub(on_line)
}

/// Each row's baseline from the top of its track: the lowest of its
/// `baseline` cells' (those starting in it); `None` for a row with none.
pub(super) fn row_baselines(
    dom: &Dom<TuiExt>,
    grid: &Grid,
    lines: &Lines,
    model: Model,
    widths: &[u16],
    cb: u16,
) -> Vec<Option<u16>> {
    let mut out = vec![None; grid.rows.len()];
    for (cell, &w) in grid.cells.iter().zip(widths) {
        if of(dom, cell) != CellAlign::Baseline {
            continue;
        }
        let b = baseline(dom, cell, lines, model, w, cb);
        let row = &mut out[cell.row];
        *row = Some(row.map_or(b, |r: u16| r.max(b)));
    }
    out
}

/// How far down `cell`'s content moves in its border box of `height`
/// rows and `width` cells (§17.5.3), its row's baseline at `row_baseline`
/// rows below its track's top.
pub(super) fn offset(
    dom: &Dom<TuiExt>,
    cell: &GridCell,
    lines: &Lines,
    model: Model,
    (width, height): (u16, u16),
    row_baseline: Option<u16>,
    cb: u16,
) -> u16 {
    let (chrome, content) = match &cell.cell {
        Cell::Anonymous(a) => (0, anonymous::height(dom, a, width, cb)),
        Cell::Element(id) => {
            let Some(c) = dom.node(*id).computed_rc() else {
                return 0;
            };
            let chrome = Sizer::vertical(&c, cb).chrome();
            let content = content_max_size(dom, *id, Direction::Column, width, cb);
            (chrome, content.saturating_sub(chrome))
        }
    };
    let free = height.saturating_sub(chrome).saturating_sub(content);
    match of(dom, cell) {
        CellAlign::Top => 0,
        CellAlign::Middle => free / 2,
        CellAlign::Bottom => free,
        CellAlign::Baseline => row_baseline
            .map_or(0, |b| {
                b.saturating_sub(baseline(dom, cell, lines, model, width, cb))
            })
            .min(free),
    }
}
