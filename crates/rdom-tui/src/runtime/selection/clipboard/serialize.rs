//! Selection → plain-text serializer for copy / cut.
//!
//! Walks the DOM in document order between `range.start` and
//! `range.end`, concatenating the text-node data inside the range.
//!
//! ## Scope (v1)
//!
//! - Concatenates text as-is, without whitespace collapsing or
//!   `<br>` → `\n` expansion. A follow-up phase will mirror the
//!   visual normalization ("clipboard text matches what the user
//!   sees").
//! - Elements with `user-select: none` are skipped so the chrome
//!   inside a selection range doesn't leak into the clipboard.
//! - Comments and element nodes contribute nothing on their own;
//!   their text descendants are what get concatenated.

use rdom_core::{Dom, NodeId, NodeType, Range};

use crate::ext::TuiExt;
use crate::layout::UserSelect;

/// Concatenate the text content of all text nodes within `range`.
/// Empty when the range is collapsed or the traversal finds no
/// selectable text.
pub fn serialize_selection(dom: &Dom<TuiExt>, range: &Range) -> String {
    let mut out = String::new();
    let mut state = WalkState::Before;
    visit(dom, dom.root(), range, &mut state, &mut out);
    out
}

/// Traversal state. Before reaching `range.start` we skip everything;
/// between start and end we accumulate; once past `range.end` we stop.
enum WalkState {
    Before,
    Inside,
    Done,
}

fn visit(dom: &Dom<TuiExt>, id: NodeId, range: &Range, state: &mut WalkState, out: &mut String) {
    if matches!(state, WalkState::Done) {
        return;
    }
    // Skip user-select:none subtrees entirely.
    if has_user_select_none(dom, id) {
        return;
    }

    match dom.node(id).node_type() {
        NodeType::Text => {
            append_text(dom, id, range, state, out);
        }
        _ => {
            // Element / Comment / Fragment: recurse into children.
            let children: Vec<NodeId> = dom.node(id).child_nodes().map(|c| c.id()).collect();
            for child in children {
                visit(dom, child, range, state, out);
                if matches!(state, WalkState::Done) {
                    return;
                }
            }
        }
    }
}

fn append_text(
    dom: &Dom<TuiExt>,
    id: NodeId,
    range: &Range,
    state: &mut WalkState,
    out: &mut String,
) {
    let Some(data) = dom.node(id).node_value() else {
        return;
    };

    let is_start = range.start.node == id;
    let is_end = range.end.node == id;

    match (is_start, is_end, &*state) {
        (true, true, _) => {
            // Selection starts and ends in this text node.
            let slice = slice_bytes(data, range.start.offset, range.end.offset);
            out.push_str(slice);
            *state = WalkState::Done;
        }
        (true, false, _) => {
            // Enter the selection here; consume to end of text.
            let slice = &data[range.start.offset.min(data.len())..];
            out.push_str(slice);
            *state = WalkState::Inside;
        }
        (false, true, WalkState::Inside) => {
            // Exit the selection at offset.
            let slice = &data[..range.end.offset.min(data.len())];
            out.push_str(slice);
            *state = WalkState::Done;
        }
        (false, false, WalkState::Inside) => {
            // Whole text node is inside.
            out.push_str(data);
        }
        _ => {
            // Before the start, or after the end in some edge case —
            // contribute nothing.
        }
    }
}

/// Byte-safe sub-slice with start/end clamped to `data.len()`. Used
/// only for the same-node case; the offsets are already byte-accurate
/// thanks to `Position`'s byte-offset contract.
fn slice_bytes(data: &str, start: usize, end: usize) -> &str {
    let s = start.min(data.len());
    let e = end.min(data.len()).max(s);
    &data[s..e]
}

/// `true` when `id` (or any ancestor) has `user-select: none`.
/// Matches the drag / position_at gating so serialization and
/// interaction stay consistent.
fn has_user_select_none(dom: &Dom<TuiExt>, id: NodeId) -> bool {
    let mut cur = Some(id);
    while let Some(n) = cur {
        let ext = dom.node(n).ext();
        if let Some(e) = ext
            && let Some(c) = &e.computed
            && c.user_select == UserSelect::None
        {
            return true;
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    false
}
