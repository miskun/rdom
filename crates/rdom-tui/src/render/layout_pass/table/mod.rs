//! The table formatting context — CSS 2.1 §17, with CSS Tables 3 where it
//! is more precise.
//!
//! A table box (`display: table` / `inline-table`, or the anonymous table
//! around a run of stray table parts) lays its children out in a grid:
//!
//! 1. [`structure`] — the box tree's table boxes (§17.2.1): captions,
//!    columns from `table-column` / `table-column-group` boxes, row groups
//!    in display order (the first header group first, the first footer
//!    group last), rows and cells, with anonymous rows and cells wrapping
//!    whatever is not a proper table child.
//! 2. [`grid`] — each cell in its slots (`rdom_core::table::assign_slots`,
//!    HTML §4.9.12.1 / CSS Tables 3 §3.3: `colspan`, `rowspan`).
//! 3. [`lines`] — the space between and around the columns and rows: the
//!    separated model's `border-spacing` (§17.6.1), or the collapsing
//!    model's shared border lines (§17.6.2), each a whole cell wide or none.
//! 4. [`columns`] — each column's min-content and max-content width, its
//!    percentage and whether a length constrains it, from its cells and
//!    `table-column` boxes; spanning cells distributed over their columns
//!    (CSS Tables 3 "computing column measures").
//! 5. [`width`] — the table's width distributed over the columns: the
//!    automatic algorithm (§17.5.2.2, CSS Tables 3 "distributing width to
//!    the columns") or the fixed one (§17.5.2.1).
//! 6. [`rows`] — row heights: the tallest cell of each row, rowspanning
//!    cells' excess spread over their rows, a taller table's height over
//!    every row (§17.5.3).
//! 7. [`place`] — every box written: captions above and below
//!    (`caption-side`, §17.4.1), row groups, rows, columns and cells at
//!    their grid areas, each cell's content laid out in it; `visibility:
//!    collapse` rows and columns taking no space (§17.5.5).
//!
//! [`solve`] runs steps 1–6 for a table border box of a given width — the
//! layout and the table's intrinsic block size share it — and
//! [`content_size`] answers the table's min- / max-content width
//! (§17.5.2.2's MIN and MAX, captions' CAPMIN) without distributing. Cells
//! are measured through `intrinsic`, whose per-pass memo keys them by
//! width, so a pass measures each once per width it is asked at.
//!
//! Whole cells throughout: a distribution hands out whole cells with the
//! rolling floor flex and grid use (`shares::Rolling`, DIVERGENCES §1).

mod anonymous;
mod columns;
#[cfg(test)]
mod cost_tests;
mod grid;
mod lines;
mod place;
mod rows;
mod structure;
mod width;

use rdom_core::{Dom, NodeId};

use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{BorderCollapse, Direction, LayoutRect, Size, TableLayout};
use crate::style::ComputedStyle;

use super::intrinsic::Measure;

pub(crate) use place::is_column_box;

/// The table's two border models (CSS 2.1 §17.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Model {
    /// `border-collapse: separate` with its `border-spacing` in cells
    /// (§17.6.1): each cell keeps its own border.
    Separate { h: u16, v: u16 },
    /// `border-collapse: collapse` (§17.6.2): neighbours share one border
    /// line, the table has no padding.
    Collapse,
}

impl Model {
    fn of(computed: &ComputedStyle) -> Self {
        match computed.border_collapse {
            BorderCollapse::Collapse => Model::Collapse,
            // Whole cells, no percentages (C4-SPACING): no basis needed.
            BorderCollapse::Separate => Model::Separate {
                h: computed.border_spacing.horizontal.resolve(0),
                v: computed.border_spacing.vertical.resolve(0),
            },
        }
    }
}

/// The table box's own chrome on each side: its border and padding in
/// the separated model, nothing in the collapsing one (its border is on
/// the grid's outer lines, §17.6.2, and it has no padding).
#[derive(Debug, Clone, Copy, Default)]
struct Chrome {
    top: u16,
    right: u16,
    bottom: u16,
    left: u16,
}

impl Chrome {
    fn of(computed: &ComputedStyle, model: Model, cb_width: u16) -> Self {
        if model == Model::Collapse {
            return Chrome::default();
        }
        let (p, b) = (&computed.padding, &computed.border);
        Chrome {
            top: p.top.resolve(cb_width).saturating_add(b.top.cells()),
            right: p.right.resolve(cb_width).saturating_add(b.right.cells()),
            bottom: p.bottom.resolve(cb_width).saturating_add(b.bottom.cells()),
            left: p.left.resolve(cb_width).saturating_add(b.left.cells()),
        }
    }

    fn horizontal(self) -> u16 {
        self.left.saturating_add(self.right)
    }

    fn vertical(self) -> u16 {
        self.top.saturating_add(self.bottom)
    }
}

/// A table laid out at one width: its structure, grid, lines and the
/// sizes of its columns and rows.
struct Solved {
    structure: structure::Structure,
    grid: grid::Grid,
    lines: lines::Lines,
    model: Model,
    chrome: Chrome,
    /// Each column's width, between the lines.
    columns: Vec<u16>,
    /// Each row's height, between the lines.
    rows: Vec<u16>,
    /// The width cells resolve percentages against: the grid's.
    cb: u16,
}

impl Solved {
    /// The table box's border-box height: its chrome, lines and rows.
    fn box_height(&self) -> u16 {
        self.chrome
            .vertical()
            .saturating_add(self.lines.total_rows(&self.grid, &self.rows))
    }
}

#[cfg(test)]
thread_local! {
    /// Tables [`solve`]d (cost tests).
    pub(super) static SOLVES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Solve the table `id` (styled `computed`, in a containing block `cb`
/// wide) at a border-box width of `width`: its columns sized into it (or
/// past it, when its content needs more) and its rows to their content.
fn solve(dom: &Dom<TuiExt>, id: NodeId, computed: &ComputedStyle, width: u16, cb: u16) -> Solved {
    #[cfg(test)]
    SOLVES.with(|c| c.set(c.get() + 1));
    let model = Model::of(computed);
    let chrome = Chrome::of(computed, model, cb);
    let structure = structure::Structure::of(dom, id);
    let grid = grid::Grid::of(dom, &structure);
    let lines = lines::Lines::of(dom, computed, &structure, &grid, model);
    let grid_width = width.saturating_sub(chrome.horizontal());
    let assignable = grid_width.saturating_sub(lines.total_columns(&grid));
    let cb = grid_width;
    let columns = if uses_fixed_layout(computed) {
        width::fixed(dom, &structure, &grid, &lines, model, assignable)
    } else {
        let measures = columns::measures(dom, &structure, &grid, &lines, model);
        width::distribute(&measures, &grid.collapsed_columns, assignable)
    };
    let rows = rows::heights(dom, &grid, &lines, model, &columns, cb);
    Solved {
        structure,
        grid,
        lines,
        model,
        chrome,
        columns,
        rows,
        cb,
    }
}

/// CSS 2.1 §17.5.2.1 / CSS Tables 3: the fixed algorithm runs for
/// `table-layout: fixed` when the table's width is not `auto` (an `auto`
/// width has nothing to fix the columns against).
fn uses_fixed_layout(computed: &ComputedStyle) -> bool {
    computed.table.table_layout == TableLayout::Fixed
        && !matches!(
            computed.width,
            Size::Auto | Size::Intrinsic(_) | Size::Flex(_) | Size::CalcSize(_)
        )
}

/// The table `id`'s border-box size on `direction` as its content size
/// (`intrinsic::content_size` for a table): on the inline axis its
/// min-content or max-content width — the columns' plus the lines and its
/// chrome (§17.5.2.2's MIN / MAX), at least its widest caption's (CAPMIN);
/// on the block axis its height at the border-box width `cross_budget`,
/// captions included.
pub(super) fn content_size(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    direction: Direction,
    cross_budget: u16,
    cb_width: u16,
    measure: Measure,
) -> u16 {
    match direction {
        Direction::Row => {
            let model = Model::of(computed);
            let chrome = Chrome::of(computed, model, cb_width);
            let structure = structure::Structure::of(dom, id);
            let grid = grid::Grid::of(dom, &structure);
            let lines = lines::Lines::of(dom, computed, &structure, &grid, model);
            let columns = if uses_fixed_layout(computed) {
                let fixed = width::fixed(dom, &structure, &grid, &lines, model, 0);
                fixed.iter().map(|&w| u32::from(w)).sum::<u32>()
            } else {
                let measures = columns::measures(dom, &structure, &grid, &lines, model);
                match measure {
                    Measure::MinContent => width::grid_min(&measures, &grid.collapsed_columns),
                    Measure::MaxContent => width::grid_max(&measures, &grid.collapsed_columns),
                }
            };
            let table = columns
                .saturating_add(u32::from(lines.total_columns(&grid)))
                .saturating_add(u32::from(chrome.horizontal()))
                .min(u32::from(u16::MAX)) as u16;
            table.max(place::caption_min(dom, &structure, cb_width))
        }
        Direction::Column => {
            let solved = solve(dom, id, computed, cross_budget, cb_width);
            let (top, bottom) = place::caption_heights(dom, &solved.structure, cross_budget);
            top.saturating_add(solved.box_height())
                .saturating_add(bottom)
        }
    }
}

/// The table's used width floor (CSS 2.1 §17.5.2.2: "the used width is
/// the greater of W, CAPMIN, and MIN"): its min-content border-box width.
pub(super) fn min_width(dom: &Dom<TuiExt>, id: NodeId, computed: &ComputedStyle, cb: u16) -> u16 {
    content_size(
        dom,
        id,
        computed,
        Direction::Row,
        0,
        cb,
        Measure::MinContent,
    )
}

/// Lay the table `id` (styled `computed`) out in its box: `layout_node`
/// gave it its border box (`TuiExt::layout`, captions included); this
/// places the captions, the table box between them and the grid in it,
/// grows the box to its content where that needs more, and returns the
/// anonymous cells whose content `id` holds.
pub(super) fn layout_table(
    dom: &mut Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
) -> Vec<AnonymousIfc> {
    let Some(outer) = dom.node(id).ext().map(|e| e.layout) else {
        return Vec::new();
    };
    // Its containing block: the box it is laid out in.
    let cb = crate::render::box_tree::box_parent(dom, id)
        .and_then(|p| dom.node(p).ext().map(|e| e.content_layout.width))
        .unwrap_or(outer.width);
    // CSS 2.1 §17.5.2.2: never narrower than its content allows.
    let width = outer.width.max(min_width(dom, id, computed, cb));
    let solved = solve(dom, id, computed, width, cb);
    place::place(dom, id, LayoutRect { width, ..outer }, solved)
}

/// The table box of the laid-out table `id` (CSS 2.1 §17.4: the box its
/// border and background paint on, between its captions): its content box
/// grown by its chrome. `None` for any other box.
pub(crate) fn table_box(dom: &Dom<TuiExt>, id: NodeId) -> Option<LayoutRect> {
    let node = dom.node(id);
    let computed = node.ext()?.computed.as_deref()?;
    if computed.flow != crate::layout::Flow::Table {
        return None;
    }
    let content = node.ext()?.content_layout;
    let cb = crate::render::box_tree::box_parent(dom, id)
        .and_then(|p| dom.node(p).ext().map(|e| e.content_layout.width))
        .unwrap_or(content.width);
    let chrome = Chrome::of(computed, Model::of(computed), cb);
    Some(LayoutRect::new(
        content.x - i32::from(chrome.left),
        content.y - i32::from(chrome.top),
        content.width.saturating_add(chrome.horizontal()),
        content.height.saturating_add(chrome.vertical()),
    ))
}

/// The rows of the table `id`'s first and last baselines from its
/// border-box top, `width` cells wide (CSS 2.1 §17.5.3: an inline table's
/// baseline is its first row's).
pub(super) fn baselines(
    dom: &Dom<TuiExt>,
    id: NodeId,
    computed: &ComputedStyle,
    width: u16,
    cb: u16,
) -> Option<(u16, u16)> {
    let solved = solve(dom, id, computed, width, cb);
    let (top, _) = place::caption_heights(dom, &solved.structure, width);
    place::row_baselines(dom, &solved).map(|(first, last)| {
        let at = top.saturating_add(solved.chrome.top);
        (at.saturating_add(first), at.saturating_add(last))
    })
}
