//! A table's boxes (CSS 2.1 §17.2, §17.2.1 "anonymous table objects";
//! CSS Tables 3 §3.2's fixup): its captions, its columns, its row groups
//! in display order, their rows and the rows' cells.
//!
//! The box tree is read through `box_tree::item_sequence` (box-less
//! children unwrapped, `::before` / `::after` as items). Of a table's
//! children, captions, column groups, columns, row groups and rows are
//! proper table children; a run of anything else — cells, text, other
//! boxes — is wrapped in an anonymous row (§17.2.1 rule 2.1). Of a row
//! group's, rows are proper and any other run makes an anonymous row (rule
//! 2.2); of a row's, cells are proper and any other run makes an
//! anonymous cell (rule 2.3). White space alone between proper children
//! generates no box (rule 1.3–1.5), and a `table-column-group`'s other
//! children are ignored (rule 1.2). `display: none` and out-of-flow boxes
//! take no part.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Display, Position, TablePart};
use crate::node::TuiNodeExt;
use crate::render::box_tree::BoxItem;

/// A table's boxes.
#[derive(Debug, Default)]
pub(super) struct Structure {
    /// Its `table-caption` children, in tree order.
    pub(super) captions: Vec<NodeId>,
    /// One per column its `table-column` / `table-column-group` boxes
    /// make, in order: the column box and its group.
    pub(super) columns: Vec<ColumnSource>,
    /// Its column and column-group boxes, each once.
    pub(super) column_boxes: Vec<NodeId>,
    /// Its row groups in display order; consecutive rows that are not in
    /// a row group (and anonymous rows) form one group with no box.
    pub(super) groups: Vec<Group>,
}

/// The boxes one column comes from.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct ColumnSource {
    pub(super) column: Option<NodeId>,
    pub(super) group: Option<NodeId>,
}

/// A row group: its box (none for loose rows) and its rows.
#[derive(Debug, Default)]
pub(super) struct Group {
    pub(super) element: Option<NodeId>,
    pub(super) rows: Vec<Row>,
}

/// A row: its box (none for an anonymous row) and its cells.
#[derive(Debug, Default)]
pub(super) struct Row {
    pub(super) element: Option<NodeId>,
    pub(super) cells: Vec<Cell>,
}

/// A cell: a `table-cell` element, or an anonymous cell wrapping a run
/// of its parent's box items.
#[derive(Debug, Clone)]
pub(super) enum Cell {
    Element(NodeId),
    Anonymous(AnonymousCell),
}

/// An anonymous cell (§17.2.1 rule 2.3): a run of `container`'s item
/// sequence, each item with its index there.
#[derive(Debug, Clone)]
pub(super) struct AnonymousCell {
    pub(super) container: NodeId,
    pub(super) content: Vec<(usize, BoxItem)>,
}

/// What a box item is to the table model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Part(TablePart),
    /// White space alone: no box between proper children.
    Space,
    /// Anything else in flow: wrapped in anonymous boxes.
    Other,
    /// No box in the table: `display: none`, out of flow, a comment.
    Skip,
}

fn kind(dom: &Dom<TuiExt>, item: BoxItem) -> Kind {
    let id = match item {
        BoxItem::Generated(..) => return Kind::Other,
        BoxItem::Node(id) => id,
    };
    let node = dom.node(id);
    match node.node_type() {
        NodeType::Text => {
            let blank = node.node_value().is_none_or(|t| {
                t.chars()
                    .all(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}'))
            });
            if blank { Kind::Space } else { Kind::Other }
        }
        NodeType::Element => {
            let Some(c) = node.computed() else {
                return Kind::Other;
            };
            if c.display == Display::None
                || matches!(c.position, Position::Absolute | Position::Fixed)
            {
                return Kind::Skip;
            }
            match c.display {
                Display::TablePart(p) => Kind::Part(p),
                _ => Kind::Other,
            }
        }
        _ => Kind::Skip,
    }
}

/// `container`'s box items with their kinds, those taking no part left out.
fn items(dom: &Dom<TuiExt>, container: NodeId) -> Vec<(usize, BoxItem, Kind)> {
    crate::render::box_tree::item_sequence(dom, container)
        .into_iter()
        .enumerate()
        .map(|(i, item)| (i, item, kind(dom, item)))
        .filter(|&(_, _, k)| k != Kind::Skip)
        .collect()
}

impl Structure {
    /// The structure of `table`.
    pub(super) fn of(dom: &Dom<TuiExt>, table: super::TableBox<'_>) -> Self {
        let (table, entries) = match table {
            super::TableBox::Element(id) => (id, items(dom, id)),
            super::TableBox::Anonymous { parent, items } => (
                parent,
                items
                    .iter()
                    .enumerate()
                    .map(|(i, &item)| (i, item, kind(dom, item)))
                    .filter(|&(_, _, k)| k != Kind::Skip)
                    .collect(),
            ),
        };
        let mut s = Structure::default();
        let mut loose = Group::default();
        let mut run: Vec<(usize, BoxItem, Kind)> = Vec::new();
        let (mut header, mut footer): (Option<Group>, Option<Group>) = (None, None);
        let mut middle: Vec<Group> = Vec::new();
        for entry in entries {
            let (_, item, k) = entry;
            let part = match k {
                Kind::Part(p) if p != TablePart::Cell => p,
                _ => {
                    run.push(entry);
                    continue;
                }
            };
            if let Some(row) = anonymous_row(table, std::mem::take(&mut run)) {
                loose.rows.push(row);
            }
            let id = item.node().expect("a table part is an element");
            match part {
                TablePart::Caption => s.captions.push(id),
                TablePart::Column => s.push_column(dom, id, None),
                TablePart::ColumnGroup => s.push_column_group(dom, id),
                TablePart::Row => loose.rows.push(row_of(dom, id)),
                TablePart::RowGroup | TablePart::HeaderGroup | TablePart::FooterGroup => {
                    if !loose.rows.is_empty() {
                        middle.push(std::mem::take(&mut loose));
                    }
                    let group = group_of(dom, id);
                    // CSS 2.1 §17.2: the first header group's rows come
                    // before every other row, the first footer group's
                    // after; any further one is a row group.
                    match part {
                        TablePart::HeaderGroup if header.is_none() => header = Some(group),
                        TablePart::FooterGroup if footer.is_none() => footer = Some(group),
                        _ => middle.push(group),
                    }
                }
                TablePart::Cell => unreachable!("cells join the run"),
            }
        }
        if let Some(row) = anonymous_row(table, run) {
            loose.rows.push(row);
        }
        if !loose.rows.is_empty() {
            middle.push(loose);
        }
        s.groups = header.into_iter().chain(middle).chain(footer).collect();
        s
    }

    /// `column`, a `table-column` box, in `group`: as many columns as its
    /// `span` (HTML §4.9.4, for `<col>`; one otherwise).
    fn push_column(&mut self, dom: &Dom<TuiExt>, column: NodeId, group: Option<NodeId>) {
        self.column_boxes.push(column);
        let span = rdom_core::table::column_span_of(dom, column);
        for _ in 0..span {
            self.columns.push(ColumnSource {
                column: Some(column),
                group,
            });
        }
    }

    /// A `table-column-group` box: its `table-column` children's columns,
    /// or — with none — as many as its `span` (HTML §4.9.3, for
    /// `<colgroup>`; one otherwise). §17.2.1 rule 1.2: its other children
    /// are ignored.
    fn push_column_group(&mut self, dom: &Dom<TuiExt>, group: NodeId) {
        self.column_boxes.push(group);
        let columns: Vec<NodeId> = items(dom, group)
            .into_iter()
            .filter(|&(_, _, k)| k == Kind::Part(TablePart::Column))
            .filter_map(|(_, item, _)| item.node())
            .collect();
        if columns.is_empty() {
            for _ in 0..rdom_core::table::column_span_of(dom, group) {
                self.columns.push(ColumnSource {
                    column: None,
                    group: Some(group),
                });
            }
        }
        for column in columns {
            self.push_column(dom, column, Some(group));
        }
    }
}

/// A row group box: its row children, and an anonymous row per run of
/// anything else (§17.2.1 rule 2.2).
fn group_of(dom: &Dom<TuiExt>, group: NodeId) -> Group {
    let mut out = Group {
        element: Some(group),
        rows: Vec::new(),
    };
    let mut run = Vec::new();
    for entry in items(dom, group) {
        match entry {
            (_, BoxItem::Node(row), Kind::Part(TablePart::Row)) => {
                if let Some(r) = anonymous_row(group, std::mem::take(&mut run)) {
                    out.rows.push(r);
                }
                out.rows.push(row_of(dom, row));
            }
            other => run.push(other),
        }
    }
    if let Some(r) = anonymous_row(group, run) {
        out.rows.push(r);
    }
    out
}

/// A row box and its cells.
fn row_of(dom: &Dom<TuiExt>, row: NodeId) -> Row {
    Row {
        element: Some(row),
        cells: cells_of(row, items(dom, row)),
    }
}

/// The anonymous row wrapping `run`, a run of `container`'s items that
/// are no proper children of it — `None` when the run is white space
/// alone (§17.2.1 rule 1.4–1.5).
fn anonymous_row(container: NodeId, run: Vec<(usize, BoxItem, Kind)>) -> Option<Row> {
    if run.iter().all(|&(_, _, k)| k == Kind::Space) {
        return None;
    }
    Some(Row {
        element: None,
        cells: cells_of(container, run),
    })
}

/// The cells of a row whose items (of `container`'s sequence) are
/// `entries`: each `table-cell` element, and an anonymous cell per run of
/// anything else holding more than white space (§17.2.1 rule 2.3).
fn cells_of(container: NodeId, entries: Vec<(usize, BoxItem, Kind)>) -> Vec<Cell> {
    let mut cells = Vec::new();
    let mut run: Vec<(usize, BoxItem, Kind)> = Vec::new();
    let flush = |run: &mut Vec<(usize, BoxItem, Kind)>, cells: &mut Vec<Cell>| {
        let taken = std::mem::take(run);
        if taken.iter().all(|&(_, _, k)| k == Kind::Space) {
            return;
        }
        cells.push(Cell::Anonymous(AnonymousCell {
            container,
            content: taken.into_iter().map(|(i, item, _)| (i, item)).collect(),
        }));
    };
    for entry in entries {
        match entry {
            (_, BoxItem::Node(cell), Kind::Part(TablePart::Cell)) => {
                flush(&mut run, &mut cells);
                cells.push(Cell::Element(cell));
            }
            other => run.push(other),
        }
    }
    flush(&mut run, &mut cells);
    cells
}
