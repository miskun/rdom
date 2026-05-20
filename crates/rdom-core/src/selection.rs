//! `Selection`, `Range`, `Position` — DOM-native text-selection
//! primitives, browser-faithful.
//!
//! A `Position` is an `(node, offset)` pair. The node is typically
//! a Text node and the offset is a byte index into the text's
//! data. Positions with an Element node are conceptually valid
//! (offset = "N children in") but v1 selection lives inside text.
//!
//! A `Range` is an ordered pair of positions — `start` precedes or
//! equals `end` in document order. Used for paint + copy.
//!
//! A `Selection` is the document-level interaction state: an
//! `anchor` (where the user started selecting) and a `focus`
//! (where the cursor is now). The pair **preserves direction** —
//! `anchor` may come after `focus` — so shrinking a selection by
//! dragging backward works.
//!
//! ## Why node+offset, not screen coordinates?
//!
//! Layout changes (resize, content insertion, scroll) invalidate
//! screen-coordinate selections on the next frame. Node+offset
//! survives re-layout: a selection between byte 3 and byte 15 of
//! a given text node stays at "bytes 3..15" regardless of where
//! those bytes land on screen. This matches the browser's
//! `Selection` API and is the reason browsers use this model.
//!
//! The runtime derives screen rectangles for paint from
//! node+offset via the `InlineLayout` fragments on each IFC block.

use crate::node_id::NodeId;

/// A position within the DOM's text content.
///
/// - `node`: the Text node the position falls inside. v1 restricts
///   positions to Text nodes; future work may generalize to
///   Element positions (before/after a child).
/// - `offset`: byte offset into the text node's `data` string.
///   **Bytes** (not codepoints, not graphemes) — consistent with
///   the browser `Range` API and with how slicing works in Rust.
///   Callers that need grapheme-level indexing convert via
///   `unicode-segmentation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub node: NodeId,
    pub offset: usize,
}

impl Position {
    /// Construct a position at `offset` bytes into `node`.
    pub fn new(node: NodeId, offset: usize) -> Self {
        Self { node, offset }
    }
}

/// A range of text in document order — `start` precedes or equals
/// `end`. Always normalized on construction via [`Range::new`] to
/// accept any two positions and sort them.
///
/// Use [`Range::is_collapsed`] to detect zero-length ranges (the
/// "caret" case).
///
/// v1 note: sorting requires document-order comparison, which
/// needs the Dom. If you already know your two positions are in
/// order, use the struct literal directly (or
/// [`Range::ordered_unchecked`]); otherwise call
/// [`Dom::selection_range`](crate::Dom::selection_range) to get
/// the normalized range for the current `Selection`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    /// Construct a range assuming the caller already knows the
    /// positions are in document order. Never fails; if you pass
    /// positions in the wrong order the resulting `Range` is
    /// unusable for paint / copy but will still compile.
    ///
    /// For a DOM-ordered normalization, use
    /// [`Dom::selection_range`](crate::Dom::selection_range).
    pub fn ordered_unchecked(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// `true` iff `start == end` — zero-length range (caret
    /// position; no highlighted text).
    pub fn is_collapsed(&self) -> bool {
        self.start == self.end
    }
}

/// Document-level selection. Two positions — `anchor` at the
/// start of the interaction (e.g., mousedown, Shift+Click origin)
/// and `focus` at the current cursor position.
///
/// **Direction-preserving**: `anchor` may come before OR after
/// `focus`. Dragging backward shrinks / inverts the selection
/// without losing the original anchor.
///
/// Use [`Selection::caret`] for a collapsed selection (cursor
/// between two graphemes, no text highlighted).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Selection {
    pub anchor: Position,
    pub focus: Position,
}

impl Selection {
    /// Construct with explicit anchor + focus. Can be inverted
    /// (focus before anchor in document order).
    pub fn new(anchor: Position, focus: Position) -> Self {
        Self { anchor, focus }
    }

    /// Construct a collapsed selection (caret) at `pos`.
    /// `anchor == focus == pos`.
    pub fn caret(pos: Position) -> Self {
        Self {
            anchor: pos,
            focus: pos,
        }
    }

    /// `true` iff `anchor == focus` — no text is highlighted; the
    /// selection represents a caret position only.
    pub fn is_collapsed(&self) -> bool {
        self.anchor == self.focus
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Dom;

    fn node() -> (Dom, NodeId) {
        let mut dom: Dom = Dom::new();
        let t = dom.create_text_node("hello world");
        (dom, t)
    }

    #[test]
    fn position_construct() {
        let (_, n) = node();
        let p = Position::new(n, 3);
        assert_eq!(p.node, n);
        assert_eq!(p.offset, 3);
    }

    #[test]
    fn selection_caret_is_collapsed() {
        let (_, n) = node();
        let sel = Selection::caret(Position::new(n, 5));
        assert!(sel.is_collapsed());
        assert_eq!(sel.anchor, sel.focus);
    }

    #[test]
    fn selection_with_different_anchor_focus_is_not_collapsed() {
        let (_, n) = node();
        let sel = Selection::new(Position::new(n, 2), Position::new(n, 7));
        assert!(!sel.is_collapsed());
    }

    #[test]
    fn selection_preserves_direction() {
        let (_, n) = node();
        // Inverted: focus before anchor.
        let sel = Selection::new(Position::new(n, 8), Position::new(n, 2));
        assert_eq!(sel.anchor.offset, 8);
        assert_eq!(sel.focus.offset, 2);
        assert!(!sel.is_collapsed());
    }

    #[test]
    fn range_is_collapsed_when_start_eq_end() {
        let (_, n) = node();
        let p = Position::new(n, 4);
        let r = Range::ordered_unchecked(p, p);
        assert!(r.is_collapsed());
    }
}
