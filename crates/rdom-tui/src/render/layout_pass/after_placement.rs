//! What runs after phase 2 places the positioned boxes
//! (C16G-ANCHOR-AFTER-MOVE).
//!
//! Phase 2 places each positioned box against the boxes as phase 1 laid
//! them out. Then the post-placement passes move boxes that are already
//! laid out ([`post_moves`]), each move journalled for a scroll update to
//! take back (`scroll_update::journal`): sticky boxes stick, relative and
//! sticky `::before` / `::after` move in their flows, relatively
//! positioned inline boxes move, and an open `<select>`'s picker flips —
//! after the inline moves, so a select inside a moved inline box flips
//! where it is.
//!
//! An anchor is read where those passes put it: the anchor box is the box
//! as laid out (CSS Anchor Positioning 1 §2), and a sticky box's stick
//! (CSS Position 3 §3.4) and a relative box's offset (CSS 2.1 §9.4.3) are
//! layout. So once they ran, every anchor-positioned box is placed again
//! against the anchors they moved, in tree order
//! (`scroll_update::reanchor`), and one that moved is moved with its
//! content. One whose size changed — an inset or `anchor-size()` reading a
//! moved anchor against an edge that stayed — needs its content laid out
//! at that size: the moves are taken back and phase 2 runs again with the
//! box given its rect, then the passes again; a dependent box resized by
//! that takes another round, at most [`MAX_RESIZE_ROUNDS`].
//!
//! A scroll update runs the same passes in the same order, so it leaves
//! what this leaves (`scroll_update`); a resize there falls back to a
//! layout. The positioned boxes' reach into their scroll containers
//! (`positioned_overflow`) is measured where phase 2 placed them.

use rdom_core::{Dom, NodeId};

use super::positioning::Given;
use crate::ext::TuiExt;
use crate::layout::LayoutRect;

/// The most times phase 2 runs again for boxes resized by their moved
/// anchors in one layout; a box still resizing after them keeps the size
/// the last run gave it.
pub(super) const MAX_RESIZE_ROUNDS: usize = 3;

/// The post-placement passes, then the anchored boxes placed against what
/// they moved (module doc).
pub(super) fn run(dom: &mut Dom<TuiExt>, viewport: LayoutRect) {
    let sticky = super::sticky::collect_sticky(dom);
    super::scroll_update::set_sticky(dom, &sticky);
    let mut given = Given::new();
    for round in 0..=MAX_RESIZE_ROUNDS {
        post_moves(dom, viewport, &sticky);
        let resized = super::scroll_update::reanchor(dom, viewport);
        if resized.is_empty() || round == MAX_RESIZE_ROUNDS {
            return;
        }
        super::scroll_update::undo_moves(dom);
        given.extend(resized);
        super::positioning::place_positioned(dom, viewport, &given);
    }
}

/// The passes that move laid-out boxes after phase 2, in their order
/// (module doc); `sticky` the document's sticky elements.
pub(super) fn post_moves(dom: &mut Dom<TuiExt>, viewport: LayoutRect, sticky: &[NodeId]) {
    super::sticky::place_sticky_in(dom, sticky);
    // Relatively positioned and sticky `::before` / `::after`, moved from
    // their in-flow places (phase 2 placed the absolute and fixed ones
    // with the elements).
    super::positioning::offset_in_flow_pseudos(dom);
    // … and relatively positioned inline boxes (ACID-FIX-5).
    super::positioning::offset_relative_inlines(dom);
    // An open `<select>` picker flips above its field where the screen
    // ends.
    super::picker::place_pickers(dom, viewport);
}
