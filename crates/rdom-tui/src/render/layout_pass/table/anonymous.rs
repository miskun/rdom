//! An anonymous table cell (CSS 2.1 §17.2.1 rule 2.3): a box with no
//! node, no padding, border or size of its own, wrapping a run of its
//! parent's children. It is a block container: each run of inline-level
//! content is one inline formatting context — stored on the parent as an
//! `AnonymousIfc`, which paint, hit-testing, the caret and selection read
//! as they read a block container's anonymous block boxes — each
//! block-level child a block box below the one before (no margins: they
//! do not collapse through the cell, DIVERGENCES §2), and each run of
//! misparented table parts an anonymous table (rule 3.2, `stray`).

use rdom_core::{Dom, NodeId};

use super::memo;
use super::structure::AnonymousCell;
use crate::ext::{AnonymousIfc, TuiExt};
use crate::layout::{Direction, LayoutRect};
use crate::render::box_tree::BoxItem;
use crate::render::inline::{InlineLayout, RunPseudos, pack_run};
use crate::render::layout_pass::block::is_block_level;
use crate::render::layout_pass::intrinsic::{contribution, intrinsic_size};
use crate::render::layout_pass::layout_node;

/// [`Segment`], owned: what `lay_out` walks while it mutates the arena.
enum Piece {
    Inline(Vec<(usize, BoxItem)>),
    Block(NodeId),
    Table(Vec<BoxItem>),
}

/// A piece of the cell's content: an inline run (its items, with their
/// indices in the parent's item sequence), a block-level element, or a
/// run of misparented table parts — rows, row groups, columns, captions —
/// in the anonymous table CSS 2.1 §17.2.1 rule 3.2 generates around them.
enum Segment<'a> {
    Inline(&'a [(usize, BoxItem)]),
    Block(NodeId),
    Table(Vec<BoxItem>),
}

/// Whether `item` is a table part other than a cell: a proper table child
/// misparented in a cell (§17.2.1 rule 3.2).
fn is_table_part(dom: &Dom<TuiExt>, item: BoxItem) -> bool {
    use crate::layout::{Display, TablePart};
    use crate::node::TuiNodeExt;
    item.node().is_some_and(|id| {
        dom.node(id)
            .computed()
            .is_some_and(|c| matches!(c.display, Display::TablePart(p) if p != TablePart::Cell))
    })
}

/// Whether `item` is a text node of white space alone.
fn is_blank(dom: &Dom<TuiExt>, item: BoxItem) -> bool {
    item.node().is_some_and(|id| {
        let n = dom.node(id);
        n.node_type() == rdom_core::NodeType::Text
            && n.node_value().is_none_or(|t| {
                t.chars()
                    .all(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}'))
            })
    })
}

fn segments<'a>(dom: &Dom<TuiExt>, cell: &'a AnonymousCell) -> Vec<Segment<'a>> {
    let content = &cell.content;
    let mut out = Vec::new();
    let (mut start, mut i) = (0, 0);
    while i < content.len() {
        let item = content[i].1;
        if is_table_part(dom, item) {
            // The run of parts, white space between them its own
            // (§17.2.1 rules 1.4–1.5: no box).
            let (mut end, mut j) = (i + 1, i + 1);
            while j < content.len() {
                if is_table_part(dom, content[j].1) {
                    end = j + 1;
                } else if !is_blank(dom, content[j].1) {
                    break;
                }
                j += 1;
            }
            if start < i {
                out.push(Segment::Inline(&content[start..i]));
            }
            let parts = content[i..end]
                .iter()
                .map(|&(_, it)| it)
                .filter(|&it| is_table_part(dom, it))
                .collect();
            out.push(Segment::Table(parts));
            (start, i) = (end, end);
            continue;
        }
        if let BoxItem::Node(id) = item
            && dom.node(id).node_type() == rdom_core::NodeType::Element
            && is_block_level(dom, id)
        {
            if start < i {
                out.push(Segment::Inline(&content[start..i]));
            }
            out.push(Segment::Block(id));
            start = i + 1;
        }
        i += 1;
    }
    if start < content.len() {
        out.push(Segment::Inline(&content[start..]));
    }
    out
}

fn pack(
    dom: &Dom<TuiExt>,
    cell: &AnonymousCell,
    run: &[(usize, BoxItem)],
    width: u16,
) -> InlineLayout {
    #[cfg(test)]
    super::count(&super::ANONYMOUS_PACKS);
    let items: Vec<BoxItem> = run.iter().map(|&(_, item)| item).collect();
    pack_run(
        dom,
        cell.container,
        &items,
        RunPseudos::default(),
        width,
        None,
    )
}

/// The memo key of `cell`: its container and first item (`memo::cell`).
fn first_item(cell: &AnonymousCell) -> Option<BoxItem> {
    cell.content.first().map(|&(_, item)| item)
}

/// The cell's min-content (`max_content` false) or max-content width
/// (CSS Sizing 3 §5.1): its widest inline run packed at that constraint
/// or block-level child's contribution. Measured once a pass.
pub(super) fn width(dom: &Dom<TuiExt>, cell: &AnonymousCell, max_content: bool, cb: u16) -> u16 {
    let measure = || {
        let w = segments(dom, cell)
            .into_iter()
            .map(|s| match s {
                Segment::Inline(run) => {
                    let at = if max_content { u16::MAX } else { 0 };
                    pack(dom, cell, run, at)
                        .lines
                        .iter()
                        .map(|l| l.width)
                        .max()
                        .unwrap_or(0)
                }
                Segment::Block(id) => contribution(dom, id, Direction::Row, 0, cb, max_content),
                Segment::Table(parts) => {
                    super::anonymous_width(dom, cell.container, &parts, max_content)
                }
            })
            .max()
            .unwrap_or(0);
        (w, 0)
    };
    let Some(first) = first_item(cell) else {
        return 0;
    };
    let query = memo::CellQuery::Width { max_content };
    memo::cell(dom, cell.container, first, query, cb, measure).0
}

/// The cell's height at `width` — its pieces stacked — and the row of its
/// first line box's text from its top (§17.5.3: its baseline; its height
/// when it holds none): one packing of each run, once a pass a width.
fn at_width(dom: &Dom<TuiExt>, cell: &AnonymousCell, width: u16, cb: u16) -> (u16, u16) {
    let measure = || {
        let (mut height, mut baseline) = (0u16, None);
        for s in segments(dom, cell) {
            let h = match s {
                Segment::Inline(run) => {
                    let il = pack(dom, cell, run, width);
                    if baseline.is_none()
                        && let Some((first, _)) = il.baselines()
                    {
                        baseline = Some(height.saturating_add(first));
                    }
                    il.height()
                }
                Segment::Block(id) => intrinsic_size(dom, id, Direction::Column, width, cb),
                Segment::Table(parts) => {
                    super::anonymous_height(dom, cell.container, &parts, width)
                }
            };
            height = height.saturating_add(h);
        }
        (height, baseline.unwrap_or(height))
    };
    let Some(first) = first_item(cell) else {
        return (0, 0);
    };
    memo::cell(
        dom,
        cell.container,
        first,
        memo::CellQuery::At(width),
        cb,
        measure,
    )
}

/// The cell's height at `width`: its pieces stacked.
pub(super) fn height(dom: &Dom<TuiExt>, cell: &AnonymousCell, width: u16, cb: u16) -> u16 {
    at_width(dom, cell, width, cb).0
}

/// Lay the cell out with its content box at `rect`: its inline runs
/// packed (their atoms laid out at their fragments) and returned as the
/// parent's anonymous boxes, its block-level children laid out below one
/// another.
pub(super) fn lay_out(
    dom: &mut Dom<TuiExt>,
    cell: &AnonymousCell,
    rect: LayoutRect,
    cb: u16,
) -> Vec<AnonymousIfc> {
    let mut boxes = Vec::new();
    let mut y = rect.y;
    let pieces: Vec<Piece> = segments(dom, cell)
        .into_iter()
        .map(|s| match s {
            Segment::Inline(run) => Piece::Inline(run.to_vec()),
            Segment::Block(id) => Piece::Block(id),
            Segment::Table(parts) => Piece::Table(parts),
        })
        .collect();
    for piece in pieces {
        match piece {
            Piece::Inline(run) => {
                let mut il = pack(dom, cell, &run, rect.width);
                crate::render::layout_pass::generated_atoms::lay_out(dom, &mut il);
                let at = LayoutRect::new(rect.x, y, rect.width, il.height());
                for (atom, place) in crate::render::inline::atomic_placements(&il, at) {
                    layout_node(dom, atom, place, rect.width);
                }
                y += i32::from(at.height);
                let range = (
                    run.first().map_or(0, |e| e.0),
                    run.last().map_or(0, |e| e.0 + 1),
                );
                boxes.push(AnonymousIfc::new(at, il, range, None));
            }
            Piece::Block(id) => {
                let h = intrinsic_size(dom, id, Direction::Column, rect.width, cb);
                layout_node(dom, id, LayoutRect::new(rect.x, y, rect.width, h), cb);
                let got = dom.node(id).ext().map_or(h, |e| e.layout.height);
                y += i32::from(got);
            }
            Piece::Table(parts) => {
                let at = LayoutRect::new(rect.x, y, rect.width, 0);
                y += i32::from(super::layout_anonymous(dom, cell.container, &parts, at));
            }
        }
    }
    boxes
}

/// The row of the cell's first line box's text from its top at `width`
/// (§17.5.3: its baseline), or its height when it holds none.
pub(super) fn first_baseline(dom: &Dom<TuiExt>, cell: &AnonymousCell, width: u16, cb: u16) -> u16 {
    at_width(dom, cell, width, cb).1
}
