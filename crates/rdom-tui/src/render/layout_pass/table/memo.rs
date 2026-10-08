//! What one layout pass keeps of each table (C13G-TABLE-COST). A table is
//! asked its min- and max-content widths, its height at a width (its
//! parent measuring it), its baselines and its layout — four or five
//! questions a pass, each of which used to build its structure, grid and
//! lines and measure its columns again. All of that is pure while a pass
//! runs (the cascaded styles and the tree do not change, as the intrinsic
//! memo assumes), so the pass keeps it: a table's [`Skeleton`] once, its
//! [`Sizes`] once per width it is solved at, and each anonymous cell's
//! measures once per width (an anonymous cell has no node for the
//! intrinsic memo to key it by). Outside a pass nothing is kept.
//!
//! The memo lives in the pass's document data (`intrinsic::memo`) and is
//! borrowed only to read or write an entry — never while measuring, which
//! may reach a nested table.

use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::Rc;

use rdom_core::{Dom, NodeId};

use super::columns::{self, ColumnMeasure};
use super::grid::Grid;
use super::lines::Lines;
use super::structure::Structure;
use super::{Model, TableBox};
use crate::ext::TuiExt;
use crate::render::box_tree::BoxItem;
use crate::render::layout_pass::intrinsic::with_tables;
use crate::style::ComputedStyle;

/// Which table: an element, or the anonymous table around a run of a
/// parent's box items, named by its first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TableKey {
    Element(NodeId),
    Anonymous(NodeId, Option<BoxItem>),
}

impl TableKey {
    fn of(table: TableBox<'_>) -> Self {
        match table {
            TableBox::Element(id) => TableKey::Element(id),
            TableBox::Anonymous { parent, items } => {
                TableKey::Anonymous(parent, items.first().copied())
            }
        }
    }
}

/// A table's width-independent shape: its boxes, its grid, its lines and
/// — when asked — its column measures.
#[derive(Debug)]
pub(super) struct Skeleton {
    pub(super) structure: Structure,
    pub(super) grid: Grid,
    pub(super) lines: Lines,
    pub(super) model: Model,
    measures: OnceCell<Vec<ColumnMeasure>>,
}

impl Skeleton {
    fn build(dom: &Dom<TuiExt>, table: TableBox<'_>, computed: &ComputedStyle) -> Self {
        let model = Model::of(computed);
        let structure = Structure::of(dom, table);
        let grid = Grid::of(dom, &structure);
        let lines = Lines::of(dom, computed, &structure, &grid, model);
        Skeleton {
            structure,
            grid,
            lines,
            model,
            measures: OnceCell::new(),
        }
    }

    /// Its columns' measures (`columns::measures`), measured once.
    pub(super) fn measures(&self, dom: &Dom<TuiExt>) -> &[ColumnMeasure] {
        self.measures.get_or_init(|| {
            columns::measures(dom, &self.structure, &self.grid, &self.lines, self.model)
        })
    }
}

/// A table solved at one border-box width: its columns' widths, its rows'
/// heights and their baselines.
#[derive(Debug)]
pub(super) struct Sizes {
    pub(super) columns: Vec<u16>,
    pub(super) rows: Vec<u16>,
    pub(super) baselines: Vec<Option<u16>>,
}

/// What an anonymous cell is asked (`anonymous`): its min- or max-content
/// width, or its height and first baseline at a width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum CellQuery {
    Width { max_content: bool },
    At(u16),
}

/// The pass's tables.
#[derive(Debug, Default)]
pub(in crate::render::layout_pass) struct TableMemo {
    skeletons: HashMap<TableKey, Rc<Skeleton>>,
    sizes: HashMap<(TableKey, u16, u16), Rc<Sizes>>,
    /// By the cell's container, its first item, the question and its
    /// containing block's width: (a width or height, a first baseline).
    cells: HashMap<(NodeId, BoxItem, CellQuery, u16), (u16, u16)>,
}

/// `table`'s skeleton (styled `computed`), built once a pass.
pub(super) fn skeleton(
    dom: &Dom<TuiExt>,
    table: TableBox<'_>,
    computed: &ComputedStyle,
) -> Rc<Skeleton> {
    let key = TableKey::of(table);
    if let Some(s) = with_tables(dom, |m| m.skeletons.get(&key).cloned()).flatten() {
        return s;
    }
    let s = Rc::new(Skeleton::build(dom, table, computed));
    with_tables(dom, |m| m.skeletons.insert(key, s.clone()));
    s
}

/// `table`'s sizes at border-box width `width` in a containing block `cb`
/// wide: `solve` once a pass per width.
pub(super) fn sizes(
    dom: &Dom<TuiExt>,
    table: TableBox<'_>,
    width: u16,
    cb: u16,
    solve: impl FnOnce() -> Sizes,
) -> Rc<Sizes> {
    let key = (TableKey::of(table), width, cb);
    if let Some(s) = with_tables(dom, |m| m.sizes.get(&key).cloned()).flatten() {
        return s;
    }
    let s = Rc::new(solve());
    with_tables(dom, |m| m.sizes.insert(key, s.clone()));
    s
}

/// An anonymous cell's answer to `query` (the cell named by its
/// `container` and first item), measured once a pass.
pub(super) fn cell(
    dom: &Dom<TuiExt>,
    container: NodeId,
    first: BoxItem,
    query: CellQuery,
    cb: u16,
    measure: impl FnOnce() -> (u16, u16),
) -> (u16, u16) {
    let key = (container, first, query, cb);
    if let Some(v) = with_tables(dom, |m| m.cells.get(&key).copied()).flatten() {
        return v;
    }
    let v = measure();
    with_tables(dom, |m| m.cells.insert(key, v));
    v
}
