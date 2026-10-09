//! Relatively positioned inline boxes (CSS 2.1 §9.4.3, CSS Position 3
//! §3.4): an inline box laid out in its line, then moved by its insets
//! "without affecting the layout of surrounding boxes" — its line keeps
//! the cells it took (ACID-FIX-5).
//!
//! A non-atomic inline box has no rect of its own: it is the fragments of
//! the lines it spans. Once the document is laid out, each text fragment
//! records the move of the relatively positioned inline boxes it is
//! inside, summed (`InlineFragment::offset`) — paint and hit-testing draw
//! and find it there; an atom inside such a box moves as a box, its
//! element's rects shifted and the shift journalled, as a moved
//! pseudo-element's box is (`pseudo_offsets`). Each box moves against the
//! containing block a relative box has: the content box of the block
//! container whose flow holds it (`relative::relative_offset`).
//!
//! The text offsets are set, not added, so the pass is idempotent: a
//! scroll update runs it again after taking the journalled box moves back.

use rdom_core::{Dom, NodeId, NodeType};

use crate::ext::TuiExt;
use crate::layout::{Display, LayoutRect, Position};
use crate::node::TuiNodeExt;
use crate::render::inline::InlineLayout;

/// Move every relatively positioned inline box of the document from its
/// in-flow place. Walks only the subtrees whose cascade found one
/// (`TuiExt::tree_has_relative_inline`).
pub(in crate::render::layout_pass) fn offset_relative_inlines(dom: &mut Dom<TuiExt>) {
    let mut owners = Vec::new();
    collect(dom, dom.root(), &mut owners);
    for owner in owners {
        offset_owner(dom, owner);
    }
}

/// The elements under `id` (itself included) whose flows may hold a moved
/// inline box: those in a flagged subtree, outside any `display: none` one.
fn collect(dom: &Dom<TuiExt>, id: NodeId, out: &mut Vec<NodeId>) {
    for child in crate::render::box_tree::children(dom, id) {
        let child = dom.node(child);
        match child.node_type() {
            NodeType::Fragment => collect(dom, child.id(), out),
            NodeType::Element => {
                let Some(ext) = child.ext() else {
                    continue;
                };
                let hidden = ext
                    .computed
                    .as_ref()
                    .is_some_and(|c| c.display == Display::None);
                if !ext.tree_has_relative_inline || hidden {
                    continue;
                }
                out.push(child.id());
                collect(dom, child.id(), out);
            }
            _ => {}
        }
    }
}

/// Set the moves of the fragments in `owner`'s flows.
fn offset_owner(dom: &mut Dom<TuiExt>, owner: NodeId) {
    let Some(ext) = dom.node(owner).ext() else {
        return;
    };
    if ext.inline_layout.is_none() && ext.anonymous_blocks.is_empty() {
        return;
    }
    let mover = Mover {
        cb: ext.content_layout,
        definite: dom
            .node(owner)
            .computed()
            .is_some_and(|c| c.height.cells(None).is_some()),
        owner,
    };
    // The moves, worked out on the laid-out tree, then written.
    let mut texts: Vec<TextMove> = Vec::new();
    let mut atoms: Vec<(NodeId, (i32, i32))> = Vec::new();
    let flows = ext.inline_layout.iter().map(|il| (None, il)).chain(
        ext.anonymous_blocks
            .iter()
            .enumerate()
            .filter(|(_, a)| a.generated.is_none())
            .map(|(k, a)| (Some(k), &a.inline_layout)),
    );
    for (flow, il) in flows {
        mover.flow(dom, il, flow, &mut texts, &mut atoms);
    }
    if let Some(ext) = dom.node_mut(owner).ext_mut() {
        for &(flow, line, f, by) in &texts {
            let il = match flow {
                None => ext.inline_layout.as_mut(),
                Some(k) => ext
                    .anonymous_blocks
                    .get_mut(k)
                    .map(|a| &mut a.inline_layout),
            };
            if let Some(fragment) = il
                .and_then(|il| il.lines.get_mut(line))
                .and_then(|l| l.fragments.get_mut(f))
            {
                fragment.offset = by;
            }
        }
    }
    for (atom, (dx, dy)) in atoms {
        crate::render::layout_pass::tree::shift_box(dom, atom, dx, dy);
        crate::render::layout_pass::scroll_update::record(
            dom,
            crate::render::layout_pass::scroll_update::PostMove::Box { id: atom, dx, dy },
        );
    }
}

/// A text fragment's move: its flow (`None` the owner's own, `Some(k)` its
/// `k`-th anonymous block box's), line, index in the line, and the move.
type TextMove = (Option<usize>, usize, usize, (i32, i32));

/// What the moves of one owner's flows are computed against.
struct Mover {
    /// The containing block of a relative box in these flows.
    cb: LayoutRect,
    /// Whether its height is definite (CSS 2.1 §9.3.2).
    definite: bool,
    owner: NodeId,
}

impl Mover {
    /// The moves of `il`'s fragments: a text fragment's to set as its
    /// offset (every one, `(0, 0)` included, so a box that stopped moving
    /// is put back), an atom's to shift its box by.
    fn flow(
        &self,
        dom: &Dom<TuiExt>,
        il: &InlineLayout,
        flow: Option<usize>,
        texts: &mut Vec<TextMove>,
        atoms: &mut Vec<(NodeId, (i32, i32))>,
    ) {
        for (l, line) in il.lines.iter().enumerate() {
            for (f, fragment) in line.fragments.iter().enumerate() {
                if fragment.atomic {
                    // The atom's own `position: relative` moved it in
                    // layout; the boxes around it move it here.
                    let parent = dom.node(fragment.node).parent_node().map(|p| p.id());
                    let by = self.moved_by(dom, parent);
                    if by != (0, 0) {
                        atoms.push((fragment.node, by));
                    }
                } else {
                    let by = self.moved_by(dom, Some(fragment.node));
                    if by != (0, 0) || fragment.offset != (0, 0) {
                        texts.push((flow, l, f, by));
                    }
                }
            }
        }
    }

    /// The summed move of the relatively positioned inline boxes from
    /// `from` up to the owner, exclusive.
    fn moved_by(&self, dom: &Dom<TuiExt>, from: Option<NodeId>) -> (i32, i32) {
        let (mut dx, mut dy) = (0, 0);
        let mut cur = from;
        while let Some(id) = cur.filter(|&id| id != self.owner) {
            let node = dom.node(id);
            if let Some(c) = node.computed()
                && c.position == Position::Relative
                && c.display == Display::Inline
            {
                let (x, y) = super::relative_offset(c, self.cb, self.definite);
                dx += x;
                dy += y;
            }
            cur = node.parent_node().map(|p| p.id());
        }
        (dx, dy)
    }
}
