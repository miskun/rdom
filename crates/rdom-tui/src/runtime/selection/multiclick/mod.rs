//! Double-click → word-select. Triple-click → line-select.
//!
//! Called as a default action from
//! `router/mouse/handle_down` after `drag::begin` has set the
//! initial caret selection. A handler can suppress this by calling
//! `event.prevent_default()` on the `mousedown` event (same switch
//! that gates drag + focus-on-click).
//!
//! ## Click counting lives on `Router`
//!
//! The multi-click gesture depends on timing between successive
//! `mousedown` events — so `Router::register_click` records a
//! `(time, column, row, count)` tuple on each down. Two downs
//! within 500 ms and 2 cells → count 2. Three → count 3. After 3
//! we stop — a fourth click inside the window resets to 1 (matches
//! browser behavior: quadruple-click doesn't over-select).
//!
//! ## Snap semantics
//!
//! - **Word**: uses `UnicodeSegmentation::split_word_bound_indices`
//!   (TR29). The clicked position falls inside exactly one word —
//!   the selection becomes that word's `[start, end)`. Clicking on
//!   whitespace selects the whitespace run (browser-faithful).
//! - **Line**: uses the IFC block's `inline_layout` line data. The
//!   line that contains the anchor's fragment is selected end to
//!   end, spanning whatever text nodes / inline elements make it
//!   up.

use unicode_segmentation::UnicodeSegmentation;

use rdom_core::{NodeId, NodeType, Position, Selection};

use crate::TuiDom;
use crate::render::inline::{InlineLayout, inline_flow_container};

/// Expand the current selection to the word containing
/// `selection.anchor`. Returns `true` when the selection actually
/// changed. No-op when there's no selection, the anchor isn't in a
/// text node, or the anchor lands in a zero-width word (shouldn't
/// happen with a normal text node).
pub(crate) fn expand_to_word(dom: &mut TuiDom) -> bool {
    let Some(sel) = dom.selection().copied() else {
        return false;
    };
    let Some(text) = text_of(dom, sel.anchor.node) else {
        return false;
    };
    let (start, end) = word_bounds(&text, sel.anchor.offset);
    if start == end {
        return false;
    }
    let next = Selection::new(
        Position::new(sel.anchor.node, start),
        Position::new(sel.anchor.node, end),
    );
    if dom.selection() == Some(&next) {
        return false;
    }
    dom.set_selection(Some(next));
    true
}

/// Expand the current selection to the full IFC line containing
/// `selection.anchor`. Walks the anchor's IFC block's
/// `inline_layout` to find the matching line, then takes the
/// start of the first fragment and the end of the last fragment
/// on that line.
pub(crate) fn expand_to_line(dom: &mut TuiDom) -> bool {
    let Some(sel) = dom.selection().copied() else {
        return false;
    };
    let Some(ifc) = ifc_block_of(dom, sel.anchor.node) else {
        return false;
    };
    let Some(layout) = inline_layout_of(dom, ifc) else {
        return false;
    };
    let Some(line_idx) = line_containing(layout, sel.anchor.node, sel.anchor.offset) else {
        return false;
    };
    let line = &layout.lines[line_idx];
    let Some(first) = line.fragments.first() else {
        return false;
    };
    let Some(last) = line.fragments.last() else {
        return false;
    };
    let next = Selection::new(
        Position::new(first.text_node, first.source_byte_offset),
        Position::new(last.text_node, last.source_byte_offset + last.text.len()),
    );
    if dom.selection() == Some(&next) {
        return false;
    }
    dom.set_selection(Some(next));
    true
}

// ── Word / line lookups ─────────────────────────────────────────────

/// Find the word bounds `[start, end)` such that `offset` falls in
/// that range. At end-of-text (`offset == text.len()`), returns the
/// last word's bounds. Empty text returns `(0, 0)`.
fn word_bounds(text: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(text.len());
    let words: Vec<(usize, &str)> = text.split_word_bound_indices().collect();
    for (start, word) in &words {
        let end = start + word.len();
        if *start <= offset && offset < end {
            return (*start, end);
        }
    }
    // Exactly at end of text — snap to the last word.
    if let Some((start, word)) = words.last() {
        return (*start, start + word.len());
    }
    (0, 0)
}

/// Find the line index in `layout` that contains the fragment
/// whose `text_node` + byte range covers `offset`. Returns `None`
/// when no fragment matches (e.g., the text node isn't part of
/// this IFC's inline content).
fn line_containing(layout: &InlineLayout, text_node: NodeId, offset: usize) -> Option<usize> {
    for (idx, line) in layout.lines.iter().enumerate() {
        for frag in &line.fragments {
            if frag.text_node != text_node {
                continue;
            }
            let frag_end = frag.source_byte_offset + frag.text.len();
            if frag.source_byte_offset <= offset && offset <= frag_end {
                return Some(idx);
            }
        }
    }
    None
}

// ── DOM / layout accessors ──────────────────────────────────────────

fn text_of(dom: &TuiDom, id: NodeId) -> Option<String> {
    if dom.node(id).node_type() != NodeType::Text {
        return None;
    }
    dom.node(id).node_value().map(|s| s.to_string())
}

fn ifc_block_of(dom: &TuiDom, node_id: NodeId) -> Option<NodeId> {
    let parent = dom.node(node_id).parent_node().map(|p| p.id())?;
    inline_flow_container(dom, parent)
}

fn inline_layout_of(dom: &TuiDom, id: NodeId) -> Option<&InlineLayout> {
    dom.node(id).ext()?.inline_layout.as_ref()
}

#[cfg(test)]
mod tests;
