//! An anonymous table cell (CSS 2.1 §17.2.1 rule 2.3): a box with no
//! node, no padding, border or size of its own, wrapping a run of its
//! parent's children. It is a block container: each run of inline-level
//! content is one inline formatting context — stored on the parent as an
//! `AnonymousIfc`, which paint, hit-testing, the caret and selection read
//! as they read a block container's anonymous block boxes — and each
//! block-level child a block box below the one before (no margins: they
//! do not collapse through the cell, DIVERGENCES §2).

use rdom_core::{Dom, NodeId};

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
}

/// A piece of the cell's content: an inline run (its items, with their
/// indices in the parent's item sequence) or a block-level element.
enum Segment<'a> {
    Inline(&'a [(usize, BoxItem)]),
    Block(NodeId),
}

fn segments<'a>(dom: &Dom<TuiExt>, cell: &'a AnonymousCell) -> Vec<Segment<'a>> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, &(_, item)) in cell.content.iter().enumerate() {
        if let BoxItem::Node(id) = item
            && dom.node(id).node_type() == rdom_core::NodeType::Element
            && is_block_level(dom, id)
        {
            if start < i {
                out.push(Segment::Inline(&cell.content[start..i]));
            }
            out.push(Segment::Block(id));
            start = i + 1;
        }
    }
    if start < cell.content.len() {
        out.push(Segment::Inline(&cell.content[start..]));
    }
    out
}

fn pack(
    dom: &Dom<TuiExt>,
    cell: &AnonymousCell,
    run: &[(usize, BoxItem)],
    width: u16,
) -> InlineLayout {
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

/// The cell's min-content (`max_content` false) or max-content width
/// (CSS Sizing 3 §5.1): its widest inline run packed at that constraint
/// or block-level child's contribution.
pub(super) fn width(dom: &Dom<TuiExt>, cell: &AnonymousCell, max_content: bool, cb: u16) -> u16 {
    segments(dom, cell)
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
        })
        .max()
        .unwrap_or(0)
}

/// The cell's height at `width`: its pieces stacked.
pub(super) fn height(dom: &Dom<TuiExt>, cell: &AnonymousCell, width: u16, cb: u16) -> u16 {
    segments(dom, cell)
        .into_iter()
        .map(|s| match s {
            Segment::Inline(run) => pack(dom, cell, run, width).height(),
            Segment::Block(id) => intrinsic_size(dom, id, Direction::Column, width, cb),
        })
        .fold(0u16, u16::saturating_add)
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
        }
    }
    boxes
}
