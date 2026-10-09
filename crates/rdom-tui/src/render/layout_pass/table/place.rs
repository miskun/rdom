//! Writing a solved table's boxes (CSS 2.1 §17.4, §17.5): the captions
//! above and below the table box (`caption-side`, §17.4.1), the table
//! box between them with its rows grown to a taller box (§17.5.3), and
//! every row group, row, column (group) and cell at its grid area — a
//! cell's content laid out in it, an anonymous cell's kept on the box
//! whose children it wraps.
//!
//! A box's edge on a line (CSS 2.1 §17.6): in the separated model it
//! stops at the line (the spacing is no box's); in the collapsing model
//! it covers the line when it has a border on that side — sharing the
//! line's cell with its neighbour's border — and stops at it otherwise.

use std::collections::HashMap;

use rdom_core::{Dom, NodeId};

use super::grid::GridCell;
use super::structure::{Cell, Structure};
use super::{Model, Solved, TableBox, align, anonymous};
use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{Border, CaptionSide, Direction, Display, LayoutRect, TablePart};
use crate::node::TuiNodeExt;
use crate::render::layout_pass::intrinsic::{contribution, intrinsic_size};
use crate::render::layout_pass::{layout_node, positioning, tree};

/// Whether `id` is a `table-column` or `table-column-group` box: a box
/// with a rect (its background paints under its column's cells, §17.5.1)
/// that is no hit-test target.
pub(crate) fn is_column_box(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    dom.node(id).computed().is_some_and(|c| {
        matches!(
            c.display,
            Display::TablePart(TablePart::Column | TablePart::ColumnGroup)
        )
    })
}

/// A caption's side, margins and border-box height at `width`.
struct CaptionBox {
    id: NodeId,
    side: CaptionSide,
    margin_top: u16,
    margin_bottom: u16,
    margin_left: u16,
    margin_right: u16,
    height: u16,
}

fn caption_boxes(dom: &Dom<TuiExt>, structure: &Structure, width: u16) -> Vec<CaptionBox> {
    structure
        .captions
        .iter()
        .filter_map(|&id| {
            let c = dom.node(id).computed_rc()?;
            let m = |v: &crate::layout::MarginValue| v.resolve(width).max(0) as u16;
            let (ml, mr) = (m(&c.margin.left), m(&c.margin.right));
            let inner = width.saturating_sub(ml.saturating_add(mr));
            Some(CaptionBox {
                id,
                side: c.table.caption_side,
                margin_top: m(&c.margin.top),
                margin_bottom: m(&c.margin.bottom),
                margin_left: ml,
                margin_right: mr,
                height: intrinsic_size(dom, id, Direction::Column, inner, width),
            })
        })
        .collect()
}

/// The captions' heights above and below the table box at `width`, their
/// vertical margins included (a caption's margins collapse with nothing,
/// DIVERGENCES §2).
pub(super) fn caption_heights(dom: &Dom<TuiExt>, structure: &Structure, width: u16) -> (u16, u16) {
    caption_boxes(dom, structure, width)
        .iter()
        .fold((0u16, 0u16), |(top, bottom), c| {
            let h = c
                .height
                .saturating_add(c.margin_top)
                .saturating_add(c.margin_bottom);
            match c.side {
                CaptionSide::Top => (top.saturating_add(h), bottom),
                CaptionSide::Bottom => (top, bottom.saturating_add(h)),
            }
        })
}

/// §17.5.2.2's CAPMIN: the widest caption's min-content contribution,
/// margins included.
pub(super) fn caption_min(dom: &Dom<TuiExt>, structure: &Structure, cb: u16) -> u16 {
    structure
        .captions
        .iter()
        .map(|&id| {
            let margins = dom.node(id).computed().map_or(0, |c| {
                (c.margin.left.resolve(cb).max(0) + c.margin.right.resolve(cb).max(0)) as u16
            });
            contribution(dom, id, Direction::Row, 0, cb, false).saturating_add(margins)
        })
        .max()
        .unwrap_or(0)
}

/// The positions of a solved table's lines and tracks on one axis, from
/// `origin`, left to right (top to bottom): `line[k]` where physical line
/// `k` starts, `track[k]` where physical track `k` starts (after line
/// `k`). Callers name tracks and lines in column / row order; an `rtl`
/// table's columns (`mirrored`, CSS 2.1 §17.5) are laid right to left, so
/// its column `k` is physical track `n - 1 - k`.
struct Axis {
    line: Vec<i32>,
    track: Vec<i32>,
    widths: Vec<u16>,
    mirrored: bool,
}

impl Axis {
    fn new(origin: i32, lines: &[u16], tracks: &[u16]) -> Self {
        Self::laid(origin, lines.iter().copied(), tracks.iter().copied(), false)
    }

    /// The columns of `lines`: right to left when `lines.rtl`.
    fn columns(origin: i32, lines: &super::lines::Lines, tracks: &[u16]) -> Self {
        let (l, t) = (lines.vertical.iter().copied(), tracks.iter().copied());
        if lines.rtl {
            Self::laid(origin, l.rev(), t.rev(), true)
        } else {
            Self::laid(origin, l, t, false)
        }
    }

    fn laid(
        origin: i32,
        lines: impl ExactSizeIterator<Item = u16>,
        tracks: impl Iterator<Item = u16>,
        mirrored: bool,
    ) -> Self {
        let mut at = origin;
        let (mut line, mut track) = (Vec::with_capacity(lines.len()), Vec::new());
        let mut widths = Vec::with_capacity(lines.len());
        let mut tracks = tracks;
        for l in lines {
            line.push(at);
            widths.push(l);
            at += i32::from(l);
            if let Some(t) = tracks.next() {
                track.push(at);
                at += i32::from(t);
            }
        }
        Axis {
            line,
            track,
            widths,
            mirrored,
        }
    }

    /// Each track's cells, from `origin`, in column / row order.
    fn tracks(&self, origin: i32) -> Vec<std::ops::Range<i32>> {
        let n = self.track.len();
        (0..n)
            .map(|k| {
                let p = if self.mirrored { n - 1 - k } else { k };
                self.track[p] - origin..self.line[p + 1] - origin
            })
            .collect()
    }

    /// The extent of tracks `from..to` (column / row order) of a box
    /// whose physical start and end sides — left and right, top and
    /// bottom — have a border (`borders`; see the module docs).
    fn span(&self, from: usize, to: usize, model: Model, borders: (bool, bool)) -> (i32, u16) {
        let n = self.track.len();
        let (from, to) = if self.mirrored {
            (n.saturating_sub(to), n.saturating_sub(from))
        } else {
            (from, to)
        };
        let collapse = model == Model::Collapse;
        let start = if from >= to {
            self.line.get(from).copied().unwrap_or(0)
        } else if collapse && borders.0 {
            self.line[from]
        } else {
            self.track[from]
        };
        let end = if collapse && borders.1 {
            self.line[to] + i32::from(self.widths[to])
        } else {
            self.line[to]
        };
        (start, (end - start).clamp(0, i32::from(u16::MAX)) as u16)
    }
}

fn borders(dom: &Dom<TuiExt>, id: Option<NodeId>) -> Border {
    id.and_then(|id| dom.node(id).computed().map(|c| c.border))
        .unwrap_or_else(Border::none)
}

/// The rect of a box over columns `cols` and rows `rows` with border `b`.
fn area(
    xs: &Axis,
    ys: &Axis,
    model: Model,
    cols: (usize, usize),
    rows: (usize, usize),
    b: Border,
) -> LayoutRect {
    let (x, w) = xs.span(
        cols.0,
        cols.1,
        model,
        (!b.left.is_none(), !b.right.is_none()),
    );
    let (y, h) = ys.span(
        rows.0,
        rows.1,
        model,
        (!b.top.is_none(), !b.bottom.is_none()),
    );
    LayoutRect::new(x, y, w, h)
}

/// Give `id` — a box this layout places, not `layout_node` — its rect,
/// and lay its `display: none` children's geometry to zero.
fn set_rect(dom: &mut Dom<TuiExt>, id: NodeId, rect: LayoutRect) {
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        tree::clear_box_state(ext, rect);
    }
    tree::collapse_hidden_children(dom, id, rect);
    for n in positioning::out_of_flow_positioned_children(dom, id) {
        positioning::record_static_position(dom, n, rect.x, rect.y);
    }
}

/// Write the table `id`'s boxes: its border box `outer` (its wrapper box,
/// captions included, at its used width), `solved` at that width. Returns
/// the anonymous boxes `id` holds.
pub(super) fn place(
    dom: &mut Dom<TuiExt>,
    table: TableBox<'_>,
    outer: LayoutRect,
    mut solved: Solved,
) -> Vec<AnonymousIfc> {
    let element = match table {
        TableBox::Element(id) => Some(id),
        TableBox::Anonymous { .. } => None,
    };
    let width = outer.width;
    let captions = caption_boxes(dom, &solved.skeleton.structure, width);
    let (top, bottom) = caption_heights(dom, &solved.skeleton.structure, width);
    // §17.5.3: a table box taller than its rows gives them the rest.
    let given = outer.height.saturating_sub(top.saturating_add(bottom));
    let need = solved.box_height();
    if given > need {
        let live: Vec<usize> = (0..solved.rows.len())
            .filter(|&r| !solved.skeleton.grid.rows[r].collapsed)
            .collect();
        let total = solved.rows.iter().fold(0u16, |a, &r| a.saturating_add(r));
        let want = total.saturating_add(given - need);
        super::rows::spread(&mut solved.rows, &live, want);
    }
    let box_height = given.max(solved.box_height());
    let table_box = LayoutRect::new(outer.x, outer.y + i32::from(top), width, box_height);
    let chrome = solved.chrome;
    let content = LayoutRect::new(
        table_box.x + i32::from(chrome.left),
        table_box.y + i32::from(chrome.top),
        width.saturating_sub(chrome.horizontal()),
        box_height.saturating_sub(chrome.vertical()),
    );
    // The grid scrolls with the table's offsets (a scroll container).
    let (sx, sy) = element
        .and_then(|id| dom.node(id).ext())
        .map_or((0, 0), |e| (e.scroll_x, e.scroll_y));
    let xs = Axis::columns(content.x - sx, &solved.skeleton.lines, &solved.columns);
    let ys = Axis::new(
        content.y - sy,
        &solved.skeleton.lines.horizontal,
        &solved.rows,
    );

    // Captions (§17.4.1): above and below the table box, in tree order.
    let (mut above, mut below) = (outer.y, table_box.y + i32::from(box_height));
    for c in &captions {
        let y = match c.side {
            CaptionSide::Top => &mut above,
            CaptionSide::Bottom => &mut below,
        };
        *y += i32::from(c.margin_top);
        let rect = LayoutRect::new(
            outer.x + i32::from(c.margin_left),
            *y,
            width.saturating_sub(c.margin_left.saturating_add(c.margin_right)),
            c.height,
        );
        layout_node(dom, c.id, rect, width);
        let got = dom.node(c.id).ext().map_or(c.height, |e| e.layout.height);
        *y += i32::from(got) + i32::from(c.margin_bottom);
    }

    let skeleton = std::rc::Rc::clone(&solved.skeleton);
    let (structure, grid) = (&skeleton.structure, &skeleton.grid);
    let (n, m) = (grid.columns, grid.rows.len());
    let model = skeleton.model;
    // Columns and column groups (§17.5.1): their tracks over every row.
    for col in &structure.column_boxes {
        super::count_column_scan();
        let rect = if col.start < col.end && col.end <= n {
            area(
                &xs,
                &ys,
                model,
                (col.start, col.end),
                (0, m),
                borders(dom, Some(col.id)),
            )
        } else {
            LayoutRect::new(content.x, content.y, 0, 0)
        };
        set_rect(dom, col.id, rect);
    }
    // Row groups and rows: every column.
    for group in grid.groups.iter().filter(|g| g.start < g.end) {
        #[cfg(test)]
        super::count(&super::GROUP_SCANS);
        let b = borders(dom, Some(group.element));
        let rect = area(&xs, &ys, model, (0, n), (group.start, group.end), b);
        set_rect(dom, group.element, rect);
    }
    for (r, row) in grid.rows.iter().enumerate() {
        if let Some(e) = row.element {
            let rect = area(&xs, &ys, model, (0, n), (r, r + 1), borders(dom, Some(e)));
            set_rect(dom, e, rect);
        }
    }
    // Cells past the grid's cap: no box.
    for cell in &grid.beyond {
        if let Cell::Element(e) = cell {
            tree::collapse_subtree_geometry(dom, *e);
        }
    }
    // Cells, their content laid out in them.
    let cb = solved.cb;
    let mut anonymous: HashMap<NodeId, Vec<AnonymousIfc>> = HashMap::new();
    for cell in &grid.cells {
        let hidden = (cell.row..cell.row_end()).all(|r| grid.rows[r].collapsed)
            || (cell.column..cell.column_end().min(n)).all(|c| grid.collapsed_columns[c]);
        let rect = cell_rect(dom, cell, &xs, &ys, model);
        // §17.5.3: its content where its `vertical-align` puts it.
        let lines = &skeleton.lines;
        let row_baseline = solved.baselines.get(cell.row).copied().flatten();
        let dy = (!hidden).then(|| {
            align::offset(
                dom,
                cell,
                lines,
                model,
                (rect.width, rect.height),
                row_baseline,
                cb,
            )
        });
        match &cell.cell {
            Cell::Element(e) => {
                if hidden {
                    tree::collapse_subtree_geometry(dom, *e);
                } else {
                    layout_node(dom, *e, rect, cb);
                    tree::shift_cell_content(dom, *e, i32::from(dy.unwrap_or(0)));
                }
            }
            Cell::Anonymous(a) => {
                if hidden {
                    continue;
                }
                let at = LayoutRect {
                    y: rect.y + i32::from(dy.unwrap_or(0)),
                    ..rect
                };
                let boxes = anonymous::lay_out(dom, a, at, cb);
                anonymous.entry(a.container).or_default().extend(boxes);
            }
        }
    }
    let own = element
        .and_then(|id| anonymous.remove(&id))
        .unwrap_or_default();
    for (container, mut boxes) in anonymous {
        boxes.sort_by_key(|b| b.child_range.0);
        if let Some(ext) = dom.node_mut(container).ext_mut() {
            ext.anonymous_blocks = boxes;
        }
    }
    // CSS Transforms 1 §3: a translated row group or row moves with its
    // cells, after they are laid out, as `layout_node` moves any other box
    // (C15G-TRANSLATE-GAPS).
    let parts = grid.groups.iter().map(|g| g.element);
    for e in parts.chain(grid.rows.iter().filter_map(|r| r.element)) {
        let node = dom.node(e);
        let shift = node.ext().zip(node.computed()).map_or((0, 0), |(x, c)| {
            crate::style::effects::translation(c, x.layout, x.content_layout)
        });
        if shift != (0, 0) {
            tree::shift_box(dom, e, shift.0, shift.1);
        }
    }
    let Some(id) = element else {
        return own;
    };
    for n in positioning::out_of_flow_positioned_children(dom, id) {
        positioning::record_static_position(dom, n, content.x - sx, content.y - sy);
    }
    let wrapper = LayoutRect::new(
        outer.x,
        outer.y,
        width,
        top.saturating_add(box_height).saturating_add(bottom),
    );
    if let Some(ext) = dom.node_mut(id).ext_mut() {
        ext.layout = wrapper;
        ext.content_layout = content;
        // §17.4: the table box between the captions — the box its border,
        // background, scrollport and clip are on (`TuiExt::border_box`) —
        // and its tracks from its content edge, unscrolled
        // (`TuiAccessors::table_tracks`).
        let origin = (content.x - sx, content.y - sy);
        ext.kept = Some(Box::new(crate::ext::KeptLayout::Table(
            crate::ext::TableKept {
                insets: crate::ext::TableInsets {
                    above: top,
                    below: bottom,
                },
                columns: xs.tracks(origin.0),
                rows: ys.tracks(origin.1),
            },
        )));
    }
    own
}

/// A cell's border box in the grid.
fn cell_rect(dom: &Dom<TuiExt>, cell: &GridCell, xs: &Axis, ys: &Axis, model: Model) -> LayoutRect {
    area(
        xs,
        ys,
        model,
        (cell.column, cell.column_end().min(xs.track.len())),
        (cell.row, cell.row_end()),
        super::lines::cell_border(dom, cell),
    )
}

/// The rows of a solved table's first and last row baselines, from the
/// grid's top (CSS 2.1 §17.5.3: a table's baseline is its first row's — the
/// lowest of its `baseline` cells' first lines, else the bottom of the
/// row).
pub(super) fn row_baselines(solved: &Solved) -> Option<(u16, u16)> {
    let ys = Axis::new(0, &solved.skeleton.lines.horizontal, &solved.rows);
    let baseline = |r: usize| -> u16 {
        let below_top = solved.baselines[r].unwrap_or_else(|| solved.rows[r].saturating_sub(1));
        (ys.track[r] + i32::from(below_top)).clamp(0, i32::from(u16::MAX)) as u16
    };
    let live: Vec<usize> = (0..solved.rows.len())
        .filter(|&r| !solved.skeleton.grid.rows[r].collapsed)
        .collect();
    let first = baseline(*live.first()?);
    let last = baseline(*live.last()?);
    Some((first, last))
}
