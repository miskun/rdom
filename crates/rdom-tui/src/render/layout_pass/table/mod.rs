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
//!    their grid areas, each cell's content laid out in it — moved by its
//!    `vertical-align` ([`align`], §17.5.3); `visibility: collapse` rows
//!    and columns taking no space (§17.5.5).
//!
//! [`stray`] wraps table parts outside a table in an anonymous table
//! (§17.2.1 rule 3), which block flow lays out as one block-level box.
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

mod align;
mod anonymous;
mod columns;
#[cfg(test)]
mod cost_tests;
mod grid;
mod lines;
mod memo;
mod place;
mod rows;
mod stray;
mod structure;
mod width;

use rdom_core::{Dom, NodeId};

use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{BorderCollapse, Direction, LayoutRect, Size, TableLayout};
use crate::render::box_tree::BoxItem;
use crate::style::ComputedStyle;

use super::intrinsic::Measure;

pub(in crate::render::layout_pass) use memo::TableMemo;
pub(crate) use place::is_column_box;
pub(super) use stray::{anonymous_height, anonymous_width, layout_anonymous};

/// A table box: a `display: table` / `inline-table` element, or the
/// anonymous table wrapping a run of `parent`'s box items that are table
/// parts outside a table (CSS 2.1 §17.2.1 rule 3).
#[derive(Debug, Clone, Copy)]
pub(super) enum TableBox<'a> {
    Element(NodeId),
    Anonymous {
        parent: NodeId,
        items: &'a [BoxItem],
    },
}

/// The widest grid a table lays out: 65 535 columns. HTML caps each
/// `colspan` and `span` at 1000 (§4.9.11, §4.9.3) but not their sum; a
/// terminal cell offset is a `u16`, so a column past this could never
/// show — and every per-column list a pass builds stays bounded, whatever
/// the attributes say (DIVERGENCES §2).
const MAX_COLUMNS: usize = u16::MAX as usize;

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

/// A table laid out at one width: its skeleton (structure, grid, lines,
/// kept for the pass) and the sizes of its columns and rows.
struct Solved {
    skeleton: std::rc::Rc<memo::Skeleton>,
    chrome: Chrome,
    /// Each column's width, between the lines.
    columns: Vec<u16>,
    /// Each row's height, between the lines.
    rows: Vec<u16>,
    /// Each row's baseline from its track's top (§17.5.3).
    baselines: Vec<Option<u16>>,
    /// The width cells resolve percentages against: the grid's.
    cb: u16,
}

impl Solved {
    /// The table box's border-box height: its chrome, lines and rows.
    fn box_height(&self) -> u16 {
        let s = &self.skeleton;
        self.chrome
            .vertical()
            .saturating_add(s.lines.total_rows(&s.grid, &self.rows))
    }
}

#[cfg(test)]
thread_local! {
    /// Tables [`solve`]d (cost tests).
    pub(super) static SOLVES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Column sources examined to find a column box's columns (cost
    /// tests).
    pub(super) static COLUMN_SCANS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Tables' structures built (cost tests).
    pub(super) static STRUCTURES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Rows examined to find a row group's rows (cost tests).
    pub(super) static GROUP_SCANS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Anonymous cells' inline runs packed (cost tests).
    pub(super) static ANONYMOUS_PACKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Count one more of `counter` (cost tests).
#[cfg(test)]
fn count(counter: &'static std::thread::LocalKey<std::cell::Cell<usize>>) {
    counter.with(|c| c.set(c.get() + 1));
}

/// Count one column source examined (cost tests).
fn count_column_scan() {
    #[cfg(test)]
    COLUMN_SCANS.with(|c| c.set(c.get() + 1));
}

/// Solve the table `table` (styled `computed`, in a containing block `cb`
/// wide) at a border-box width of `width`: its columns sized into it (or
/// past it, when its content needs more) and its rows to their content.
/// A pass solves a table once per width (`memo`).
fn solve(
    dom: &Dom<TuiExt>,
    table: TableBox<'_>,
    computed: &ComputedStyle,
    width: u16,
    cb: u16,
) -> Solved {
    let skeleton = memo::skeleton(dom, table, computed);
    let s = &*skeleton;
    let chrome = Chrome::of(computed, s.model, cb);
    let grid_width = width.saturating_sub(chrome.horizontal());
    let sizes = memo::sizes(dom, table, width, cb, || {
        #[cfg(test)]
        count(&SOLVES);
        let assignable = grid_width.saturating_sub(s.lines.total_columns(&s.grid));
        let columns = if uses_fixed_layout(computed) {
            width::fixed(dom, &s.structure, &s.grid, &s.lines, s.model, assignable)
        } else {
            width::distribute(s.measures(dom), &s.grid.collapsed_columns, assignable)
        };
        let rows::Rows { heights, baselines } =
            rows::heights(dom, &s.grid, &s.lines, s.model, &columns, grid_width);
        memo::Sizes {
            columns,
            rows: heights,
            baselines,
        }
    });
    Solved {
        chrome,
        columns: sizes.columns.clone(),
        rows: sizes.rows.clone(),
        baselines: sizes.baselines.clone(),
        cb: grid_width,
        skeleton,
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
    size_of(
        dom,
        TableBox::Element(id),
        computed,
        direction,
        cross_budget,
        cb_width,
        measure,
    )
}

/// [`content_size`] for any table box.
fn size_of(
    dom: &Dom<TuiExt>,
    table: TableBox<'_>,
    computed: &ComputedStyle,
    direction: Direction,
    cross_budget: u16,
    cb_width: u16,
    measure: Measure,
) -> u16 {
    match direction {
        Direction::Row => {
            let s = memo::skeleton(dom, table, computed);
            let chrome = Chrome::of(computed, s.model, cb_width);
            let columns = if uses_fixed_layout(computed) {
                let fixed = width::fixed(dom, &s.structure, &s.grid, &s.lines, s.model, 0);
                fixed.iter().map(|&w| u32::from(w)).sum::<u32>()
            } else {
                let measures = s.measures(dom);
                match measure {
                    Measure::MinContent => width::grid_min(measures, &s.grid.collapsed_columns),
                    Measure::MaxContent => width::grid_max(measures, &s.grid.collapsed_columns),
                }
            };
            let table = columns
                .saturating_add(u32::from(s.lines.total_columns(&s.grid)))
                .saturating_add(u32::from(chrome.horizontal()))
                .min(u32::from(u16::MAX)) as u16;
            table.max(place::caption_min(dom, &s.structure, cb_width))
        }
        Direction::Column => {
            let solved = solve(dom, table, computed, cross_budget, cb_width);
            let (top, bottom) =
                place::caption_heights(dom, &solved.skeleton.structure, cross_budget);
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
    let solved = solve(dom, TableBox::Element(id), computed, width, cb);
    place::place(
        dom,
        TableBox::Element(id),
        LayoutRect { width, ..outer },
        solved,
    )
}

/// CSS 2.1 §17.6.1.1: whether the cell `id` draws no border or
/// background — `empty-cells: hide` in the separated model (its table's
/// `border-collapse: separate`; an anonymous table's always) on a cell with
/// no content: no in-flow box, no text but white space, no `::before` /
/// `::after`.
pub(crate) fn hides_empty_cell(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    use crate::ext::PseudoSlot;
    use crate::layout::{Display, EmptyCells, TablePart};
    let node = dom.node(id);
    let Some(c) = node.ext().and_then(|e| e.computed.as_deref()) else {
        return false;
    };
    if c.display != Display::TablePart(TablePart::Cell) || c.table.empty_cells != EmptyCells::Hide {
        return false;
    }
    let mut up = crate::render::box_tree::box_parent(dom, id);
    while let Some(p) = up {
        let pc = dom.node(p).ext().and_then(|e| e.computed.as_deref());
        match pc {
            Some(pc) if pc.flow == crate::layout::Flow::Table => {
                if pc.border_collapse == BorderCollapse::Collapse {
                    return false;
                }
                break;
            }
            Some(pc) if matches!(pc.display, Display::TablePart(_)) => {
                up = crate::render::box_tree::box_parent(dom, p);
            }
            _ => break,
        }
    }
    let empty_children = crate::render::box_tree::children(dom, id).all(|child| {
        let n = dom.node(child);
        match n.node_type() {
            rdom_core::NodeType::Text => n.node_value().is_none_or(|t| {
                t.chars()
                    .all(|ch| matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{c}'))
            }),
            rdom_core::NodeType::Element => {
                !super::is_in_flow(dom, child)
                    || n.ext()
                        .and_then(|e| e.computed.as_deref())
                        .is_some_and(|s| s.display == Display::None)
            }
            _ => true,
        }
    });
    empty_children
        && crate::render::box_tree::generated_text(dom, id, PseudoSlot::Before).is_none()
        && crate::render::box_tree::generated_text(dom, id, PseudoSlot::After).is_none()
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
    let solved = solve(dom, TableBox::Element(id), computed, width, cb);
    let (top, _) = place::caption_heights(dom, &solved.skeleton.structure, width);
    place::row_baselines(&solved).map(|(first, last)| {
        let at = top.saturating_add(solved.chrome.top);
        (at.saturating_add(first), at.saturating_add(last))
    })
}
