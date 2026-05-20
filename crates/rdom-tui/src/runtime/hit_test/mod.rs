//! `HitTestExt` — point → element lookup, the foundation of mouse
//! routing.
//!
//! `Dom::hit_test(x, y)` returns the deepest element whose painted
//! area contains `(x, y)`. `hit_test_path(x, y)` returns the full
//! ancestor chain (outer → inner), matching the browser's
//! `composedPath()` for a synthetic `MouseEvent` at that point.
//!
//! ## Algorithm (spec §7.1)
//!
//! Recursive descent from root:
//!
//! 1. **Non-element** (Fragment root): recurse into element children;
//!    skip the Fragment itself (it has no layout rect).
//! 2. **Element**:
//!    - If `(x, y)` is outside this element's `layout` rect → miss;
//!      don't add to path, don't recurse.
//!    - Otherwise, add to path.
//!    - **Overflow clip**: if `overflow != Visible` and `(x, y)` is
//!      outside this element's `content_layout` (padding/border
//!      area without content) → hit stays on this element; don't
//!      recurse.
//!    - **IFC block**: look up the fragment at `(x − content.x,
//!      y − content.y)`. If found and the fragment's owner is not
//!      the IFC block itself, walk the owner's ancestor chain up to
//!      (but not including) the IFC block and append each ancestor
//!      in outer→inner order.
//!    - **Normal block**: recurse into element children in **reverse
//!      document order** (last-painted wins for stacking). First
//!      child whose descent adds to the path wins — we return
//!      immediately without trying earlier siblings.
//!
//! ## Stacking
//!
//! No `z-index` in v1. Paint order = stacking order. Reverse-document
//! iteration in step 2 mirrors the paint pass's "later-siblings paint
//! on top" behavior.
//!
//! ## `pointer-events`
//!
//! Not supported in v1. Every painted element is hittable.

use rdom_core::{Dom, NodeId, NodeType, Position};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::ext::TuiExt;
use crate::layout::{LayoutRect, Overflow};
use crate::node::TuiNodeExt;
use crate::render::inline::{InlineFragment, has_inline_layout};
use crate::runtime::selection::user_select;

/// Extension trait adding hit-test lookup to `Dom<TuiExt>`.
pub trait HitTestExt {
    /// The deepest element whose painted area contains `(x, y)`.
    /// Uses the last-painted-wins rule: when two siblings overlap,
    /// the later one wins. Returns `None` if no element covers the
    /// point (e.g., empty viewport).
    ///
    /// For IFC blocks the lookup descends into the inline layout so
    /// a point landing on text inside a `<code>` fragment returns
    /// the `<code>` element, not the enclosing `<p>`.
    fn hit_test(&self, x: u16, y: u16) -> Option<NodeId>;

    /// The full ancestor chain from root to the deepest hit, in
    /// document order (root-most first, deepest last). Suitable for
    /// event-dispatch targets or browser-style `composedPath()`
    /// walks. Empty when nothing hit.
    fn hit_test_path(&self, x: u16, y: u16) -> Vec<NodeId>;

    /// Map a screen cell `(x, y)` to a DOM text position — a
    /// `(text_node, byte_offset)` pair suitable for
    /// [`Dom::set_selection`].
    ///
    /// Returns `None` when:
    /// - `(x, y)` misses every element;
    /// - the hit lands outside any IFC block (no selectable text
    ///   at that point);
    /// - the innermost hit element or one of its ancestors has
    ///   `user-select: none` (chrome, buttons, etc. — the
    ///   selection algorithm skips these subtrees);
    /// - `(x, y)` falls in an IFC block's padding / border but
    ///   not its content area (outside all fragments).
    ///
    /// The returned `offset` is a byte offset into the text
    /// node's data — matches the `Selection` / `Range` API and
    /// Rust string slicing conventions.
    fn position_at(&self, x: u16, y: u16) -> Option<Position>;
}

impl HitTestExt for Dom<TuiExt> {
    fn hit_test(&self, x: u16, y: u16) -> Option<NodeId> {
        self.hit_test_path(x, y).last().copied()
    }

    fn hit_test_path(&self, x: u16, y: u16) -> Vec<NodeId> {
        let mut path = Vec::new();
        // M2 §12.9-12.10: try positioned elements first, in
        // reverse paint order (= reverse z-index, with reverse
        // document order as tiebreak). The first whose layout
        // rect contains (x, y) catches the click. `descend` then
        // recurses into the subtree as normal — non-positioned
        // descendants flow through, nested positioned descendants
        // are skipped (they get their own iteration of this same
        // loop).
        let positioned = collect_positioned_reverse_z(self);
        for id in positioned {
            if let Some(rect) = self.node(id).layout_rect()
                && rect_contains(rect, x, y)
                && descend(self, id, x, y, &mut path)
            {
                return path;
            }
        }
        // No positioned hit — fall back to the document-order
        // walk over in-flow content. `descend_children_reverse`
        // skips positioned children for the same reason.
        descend(self, self.root(), x, y, &mut path);
        path
    }

    fn position_at(&self, x: u16, y: u16) -> Option<Position> {
        // Find the IFC block under (x, y). `hit_test_path` already
        // handles overflow clipping and reverse-document-order
        // stacking; we just need to find the first IFC ancestor
        // on the path.
        let path = self.hit_test_path(x, y);

        // Walk path *innermost-first* — we want the deepest IFC
        // that contains the point. But IFC blocks are block
        // elements, so any ancestor of the hit element that's an
        // IFC block is a candidate; the deepest wins.
        let mut ifc_block: Option<NodeId> = None;
        for &id in path.iter().rev() {
            if has_inline_layout(self, id) {
                ifc_block = Some(id);
                break;
            }
        }
        let ifc_id = ifc_block?;

        // user-select gate: any ancestor of the hit with
        // `user-select: none` kills the position.
        if user_select::has_none_ancestor(self, *path.last()?) {
            return None;
        }

        // Find the fragment at the point inside the IFC block.
        // If no fragment covers (x, y) — common case: the user
        // dragged the mouse past a line's content — clamp to the
        // nearest valid position on the target line. Without this,
        // drag-selection past end-of-line silently misses the final
        // character (the `position_at` returns None and the drag
        // handler doesn't update the selection focus).
        let content = self.node(ifc_id).content_layout_rect()?;
        match fragment_at(self, ifc_id, content, x, y) {
            Some(fragment) => {
                let cell_offset_in_frag = (x as i32 - content.x - fragment.x as i32).max(0) as u16;
                let bytes_into_text = cells_to_bytes(&fragment.text, cell_offset_in_frag);
                Some(Position::new(
                    fragment.text_node,
                    fragment.source_byte_offset + bytes_into_text,
                ))
            }
            None => clamp_to_line(self, ifc_id, content, x, y),
        }
    }
}

/// Clamp `(x, y)` to the nearest valid position on the inline layout
/// of `ifc_id`. Used when the hit cell isn't covered by a fragment —
/// drag past end-of-line, click past last-line bottom, etc.
///
/// Rules:
/// - `y < content.y` → first line's start position.
/// - `y >= content.y + content.height` → last line's end position.
/// - In-bounds y, x past line's content → that line's end position.
/// - In-bounds y, line is empty → walk to the nearest non-empty line.
fn clamp_to_line(
    dom: &Dom<TuiExt>,
    ifc_id: NodeId,
    content: crate::layout::LayoutRect,
    x: u16,
    y: u16,
) -> Option<Position> {
    let layout = dom.node(ifc_id).ext()?.inline_layout.as_ref()?;
    if layout.lines.is_empty() {
        return None;
    }

    let line_idx = if (y as i32) < content.y {
        0
    } else {
        let raw = (y as i32 - content.y) as usize;
        raw.min(layout.lines.len() - 1)
    };

    let target_line = &layout.lines[line_idx];

    // Empty line — try walking out to find a non-empty fragment.
    // Falls back to the last line's last fragment if everything's
    // empty (shouldn't happen for a populated IFC, but defensive).
    if target_line.fragments.is_empty() {
        for line in layout.lines.iter().rev() {
            if let Some(frag) = line.fragments.last() {
                return Some(Position::new(
                    frag.text_node,
                    frag.source_byte_offset + frag.text.len(),
                ));
            }
        }
        return None;
    }

    // x past the line's last fragment → end of last fragment.
    // x before the line's first fragment → start of first fragment.
    let first = target_line.fragments.first().unwrap();
    let last = target_line.fragments.last().unwrap();
    let line_left = content.x + first.x as i32;
    let line_right = content.x + last.x as i32 + last.width as i32;

    if (x as i32) < line_left {
        Some(Position::new(first.text_node, first.source_byte_offset))
    } else if (x as i32) >= line_right {
        Some(Position::new(
            last.text_node,
            last.source_byte_offset + last.text.len(),
        ))
    } else {
        // Somewhere in the middle of the line but no fragment
        // covered the cell (gap between fragments, shouldn't be
        // common). Clamp to the last fragment's end as a fallback.
        Some(Position::new(
            last.text_node,
            last.source_byte_offset + last.text.len(),
        ))
    }
}

/// Append nodes to `path` if `(x, y)` lands inside the subtree
/// rooted at `id`. Returns `true` when at least one node was added
/// at this level or deeper (lets the caller skip trying earlier
/// siblings).
fn descend(dom: &Dom<TuiExt>, id: NodeId, x: u16, y: u16, path: &mut Vec<NodeId>) -> bool {
    let ty = dom.node(id).node_type();

    // Fragment (the default root): no box of its own, recurse into
    // element children in reverse document order.
    if ty == NodeType::Fragment {
        return descend_children_reverse(dom, id, x, y, path);
    }

    if ty != NodeType::Element {
        return false;
    }

    // Element — check containment against its outer layout rect.
    let outer = match dom.node(id).layout_rect() {
        Some(r) if rect_contains(r, x, y) => r,
        _ => return false,
    };

    path.push(id);

    // Overflow clipping: if the element clips its children, check
    // whether (x, y) is in the content area. If not, the hit stays
    // on THIS element (its padding/border) — no recurse.
    let computed = dom.node(id).computed();
    let clips_children = computed.is_some_and(|c| {
        !matches!(c.overflow_x, Overflow::Visible) || !matches!(c.overflow_y, Overflow::Visible)
    });

    let inner = dom.node(id).content_layout_rect().unwrap_or(outer);
    if clips_children && !rect_contains(inner, x, y) {
        return true; // hit on padding/border, no descent
    }

    // Inline-flow container: descend into the inline layout to find
    // the fragment's owner element. Then walk that owner's ancestor
    // chain back up, appending outer → inner.
    if has_inline_layout(dom, id) {
        if let Some(owner) = hit_fragment(dom, id, inner, x, y)
            && owner != id
        {
            append_inline_ancestors(dom, id, owner, path);
        }
        return true;
    }

    // Normal block: recurse into element children in REVERSE
    // document order. First one that hits wins (matches paint order).
    //
    // Note: when `clips_children` is true, an overflowing child
    // still shouldn't be hittable past the inner rect. Children
    // laid out *within* inner remain hittable; children that happen
    // to be positioned outside (negative scroll offset etc.) miss
    // cleanly because their layout_rect doesn't contain (x, y).
    descend_children_reverse(dom, id, x, y, path);
    true
}

/// Recurse into direct element children in reverse document order.
/// Returns `true` when any child (or its subtree) added to `path`.
///
/// M2: positioned children (`position: absolute | fixed`) are
/// skipped here — they're handled by the z-list pass at the top
/// of `hit_test_path`. This matches the paint pass, which also
/// pulls positioned children out of the document walk into a
/// global stacking context.
fn descend_children_reverse(
    dom: &Dom<TuiExt>,
    id: NodeId,
    x: u16,
    y: u16,
    path: &mut Vec<NodeId>,
) -> bool {
    let child_ids: Vec<NodeId> = dom.node(id).child_nodes().map(|n| n.id()).collect();
    for &child in child_ids.iter().rev() {
        if is_positioned(dom, child) {
            continue;
        }
        if descend(dom, child, x, y, path) {
            return true;
        }
    }
    false
}

fn is_positioned(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    dom.node(id)
        .ext()
        .and_then(|e| e.computed.as_ref())
        .map(|c| {
            matches!(
                c.position,
                crate::layout::Position::Absolute | crate::layout::Position::Fixed
            )
        })
        .unwrap_or(false)
}

/// Collect every positioned (absolute / fixed) element in the
/// tree, sorted in **reverse paint order** — highest z-index
/// first, with reverse-document-order as the tiebreaker (so the
/// last-painted element of a same-z group is tried first).
fn collect_positioned_reverse_z(dom: &Dom<TuiExt>) -> Vec<NodeId> {
    let mut list: Vec<(i16, usize, NodeId)> = Vec::new();
    let mut order: usize = 0;
    walk_for_positioned(dom, dom.root(), &mut list, &mut order);
    // Sort by (z, order) ascending, then reverse → highest z and
    // latest order are at the front (= reverse paint order).
    list.sort_by_key(|(z, ord, _)| (*z, *ord));
    list.reverse();
    list.into_iter().map(|(_, _, id)| id).collect()
}

fn walk_for_positioned(
    dom: &Dom<TuiExt>,
    id: NodeId,
    out: &mut Vec<(i16, usize, NodeId)>,
    order: &mut usize,
) {
    if let Some(computed) = dom.node(id).ext().and_then(|e| e.computed.as_ref())
        && matches!(
            computed.position,
            crate::layout::Position::Absolute | crate::layout::Position::Fixed
        )
    {
        let z = match computed.z_index {
            crate::layout::ZIndex::Auto => 0,
            crate::layout::ZIndex::Value(n) => n,
        };
        out.push((z, *order, id));
        *order += 1;
    }
    for child in dom.node(id).child_nodes() {
        walk_for_positioned(dom, child.id(), out, order);
    }
}

/// Look up the inline fragment under `(x, y)` inside an IFC block's
/// content area. Returns the fragment's owner element (the direct
/// element parent of the underlying text — typically `<code>`, `<b>`,
/// or the IFC block itself when the text is a direct child).
fn hit_fragment(
    dom: &Dom<TuiExt>,
    ifc_block: NodeId,
    content: LayoutRect,
    x: u16,
    y: u16,
) -> Option<NodeId> {
    let ext = dom.node(ifc_block).ext()?;
    let layout = ext.inline_layout.as_ref()?;

    // Line index is the y-offset within content.
    let line_index = y as i32 - content.y;
    if line_index < 0 || line_index as usize >= layout.lines.len() {
        return None;
    }
    let line = &layout.lines[line_index as usize];

    // Local x within content.
    let x_local_i = x as i32 - content.x;
    if x_local_i < 0 {
        return None;
    }
    let x_local = x_local_i as u16;

    for fragment in &line.fragments {
        if x_local >= fragment.x && x_local < fragment.x + fragment.width {
            return Some(fragment.node);
        }
    }
    None
}

/// Walk the ancestor chain from `owner` up to (but not including)
/// `ifc_block`. Append each to `path` in outer → inner order so the
/// final path stays document-ordered.
fn append_inline_ancestors(
    dom: &Dom<TuiExt>,
    ifc_block: NodeId,
    owner: NodeId,
    path: &mut Vec<NodeId>,
) {
    // Collect inner → outer first, then reverse.
    let mut chain = Vec::new();
    let mut cur = owner;
    while cur != ifc_block {
        chain.push(cur);
        match dom.node(cur).parent_node() {
            Some(parent) => cur = parent.id(),
            None => break, // defensive — should never trigger in a well-formed tree
        }
    }
    chain.reverse();
    path.extend(chain);
}

#[inline]
fn rect_contains(r: LayoutRect, x: u16, y: u16) -> bool {
    let x = x as i32;
    let y = y as i32;
    x >= r.x && x < r.x + r.width as i32 && y >= r.y && y < r.y + r.height as i32
}

/// Look up the `InlineFragment` under `(x, y)` inside an IFC
/// block's content area. Returns a reference into the block's
/// stored `InlineLayout` — the caller extracts whatever info it
/// needs (owner, text_node, source offset) without cloning.
fn fragment_at(
    dom: &Dom<TuiExt>,
    ifc_block: NodeId,
    content: LayoutRect,
    x: u16,
    y: u16,
) -> Option<&InlineFragment> {
    let ext = dom.node(ifc_block).ext()?;
    let layout = ext.inline_layout.as_ref()?;

    let line_index = y as i32 - content.y;
    if line_index < 0 || line_index as usize >= layout.lines.len() {
        return None;
    }
    let line = &layout.lines[line_index as usize];

    let x_local_i = x as i32 - content.x;
    if x_local_i < 0 {
        return None;
    }
    let x_local = x_local_i as u16;

    line.fragments
        .iter()
        .find(|&fragment| x_local >= fragment.x && x_local < fragment.x + fragment.width)
        .map(|v| v as _)
}

/// Walk graphemes of `text` counting cell widths; return the byte
/// offset of the grapheme whose cell range contains `target_cells`.
///
/// Cell grain is per-grapheme (1 for ASCII, 2 for CJK, etc.), not
/// byte length. If `target_cells` falls inside a wide grapheme, the
/// returned offset is the grapheme's *start* byte — the click snaps
/// to the left edge of the character. If `target_cells` overshoots
/// the text's total cell width, returns `text.len()`.
fn cells_to_bytes(text: &str, target_cells: u16) -> usize {
    let mut consumed_cells: u16 = 0;
    for (idx, g) in text.grapheme_indices(true) {
        let w = UnicodeWidthStr::width(g) as u16;
        if target_cells < consumed_cells.saturating_add(w) {
            return idx;
        }
        consumed_cells = consumed_cells.saturating_add(w);
    }
    text.len()
}

#[cfg(test)]
mod tests;
