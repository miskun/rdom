//! What the last layout did after placing the positioned boxes, kept so
//! a scroll update can take it back (document data): the passes that run
//! after phase 2 — sticky placement, the picker flip, the relative and
//! sticky `::before` / `::after` offsets — each move boxes that are
//! already laid out, and the sticky and picker ones depend on the scroll
//! offsets. A scroll update undoes them, moves the scrolled content, and
//! runs them again ([`super`]).

use rdom_core::{Dom, NodeId};

use crate::ext::TuiExt;
use crate::layout::LayoutRect;
use crate::render::box_tree::BoxItem;

/// One move a post-placement pass made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render::layout_pass) enum PostMove {
    /// A sticky element's stick: its subtree moved, `fixed` boxes whose
    /// containing block is outside it kept (`tree::shift_subtree`).
    Subtree { id: NodeId, dx: i32, dy: i32 },
    /// A box and its whole subtree moved (`tree::shift_box`): a flipped
    /// picker's option.
    Box { id: NodeId, dx: i32, dy: i32 },
    /// A relative or sticky pseudo-element moved in its owner's flow.
    Pseudo(crate::render::layout_pass::positioning::PseudoMove),
}

/// Where phase 2 placed a positioned box: the rect placement gave it, and
/// where its border box was once laid out there — so a later move of the
/// box (a scroll moving it with its containing block) is the difference
/// between where it is and `at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render::layout_pass) struct PlacedRecord {
    pub(in crate::render::layout_pass) item: BoxItem,
    pub(in crate::render::layout_pass) rect: LayoutRect,
    pub(in crate::render::layout_pass) at: (i32, i32),
}

/// The last layout's record (document data).
#[derive(Debug)]
pub(in crate::render::layout_pass) struct Journal {
    /// The viewport it laid out against.
    pub(super) viewport: LayoutRect,
    /// The positioned boxes phase 2 placed, in document order, each with
    /// where it was placed.
    pub(super) positioned: Vec<PlacedRecord>,
    /// The sticky elements, in document order.
    pub(super) sticky: Vec<NodeId>,
    /// The post-placement moves, in the order they were made.
    pub(super) moves: Vec<PostMove>,
}

impl Journal {
    /// Whether every node the record names is still in the tree — the
    /// caller vouches nothing but scroll offsets changed, and a removed
    /// node would make the record stale.
    pub(super) fn is_live(&self, dom: &Dom<TuiExt>) -> bool {
        let live = |id: NodeId| dom.contains(id) && dom.node(id).is_connected();
        self.positioned.iter().all(|p| match p.item {
            BoxItem::Node(id) | BoxItem::Generated(id, _) => live(id),
        }) && self.sticky.iter().all(|&id| live(id))
            && self.moves.iter().all(|m| match *m {
                PostMove::Subtree { id, .. } | PostMove::Box { id, .. } => live(id),
                PostMove::Pseudo(p) => live(p.owner()),
            })
    }
}

/// Start the record of a layout against `viewport`.
pub(in crate::render::layout_pass) fn begin(dom: &mut Dom<TuiExt>, viewport: LayoutRect) {
    dom.set_document_data(Journal {
        viewport,
        positioned: Vec::new(),
        sticky: Vec::new(),
        moves: Vec::new(),
    });
}

/// Start the record of a run of phase 2 (each run of phases 1–2 places
/// every positioned box again).
pub(in crate::render::layout_pass) fn begin_placement(dom: &mut Dom<TuiExt>) {
    if let Some(j) = dom.document_data_mut::<Journal>() {
        j.positioned.clear();
    }
}

/// Record where phase 2 placed a positioned box.
pub(in crate::render::layout_pass) fn note_placed(dom: &mut Dom<TuiExt>, placed: PlacedRecord) {
    if let Some(j) = dom.document_data_mut::<Journal>() {
        j.positioned.push(placed);
    }
}

/// Record the sticky elements.
pub(in crate::render::layout_pass) fn set_sticky(dom: &mut Dom<TuiExt>, sticky: &[NodeId]) {
    if let Some(j) = dom.document_data_mut::<Journal>() {
        j.sticky.clear();
        j.sticky.extend_from_slice(sticky);
    }
}

/// Record a post-placement move.
pub(in crate::render::layout_pass) fn record(dom: &mut Dom<TuiExt>, m: PostMove) {
    if let Some(j) = dom.document_data_mut::<Journal>() {
        j.moves.push(m);
    }
}

/// Take the record out of the document: a scroll update consumes it and
/// starts a new one.
pub(super) fn take(dom: &mut Dom<TuiExt>) -> Option<Journal> {
    dom.remove_document_data::<Journal>()
}

/// Put `j` back as the document's record.
pub(super) fn restore(dom: &mut Dom<TuiExt>, j: Journal) {
    dom.set_document_data(j);
}

/// Take back `moves`, last first: the boxes return to where phase 2 left
/// them.
pub(super) fn undo(dom: &mut Dom<TuiExt>, moves: &[PostMove]) {
    for m in moves.iter().rev() {
        match *m {
            PostMove::Subtree { id, dx, dy } => {
                crate::render::layout_pass::tree::shift_subtree(dom, id, -dx, -dy);
            }
            PostMove::Box { id, dx, dy } => {
                crate::render::layout_pass::tree::shift_box(dom, id, -dx, -dy);
            }
            PostMove::Pseudo(p) => p.undo(dom),
        }
    }
}
