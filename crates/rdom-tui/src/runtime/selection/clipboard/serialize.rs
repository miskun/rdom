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
//! - Text whose used `user-select` is `none` is skipped so the chrome
//!   inside a selection range doesn't leak into the clipboard. The
//!   walk carries the used value down (CSS UI 4 §6.1), so an explicit
//!   `text` descendant of a `none` element is still copied.
//! - Comments and element nodes contribute nothing on their own;
//!   their text descendants are what get concatenated.

use rdom_core::{Dom, NodeId, NodeType, Range};

use crate::layout::UserSelect;

use crate::ext::TuiExt;
use crate::runtime::selection::user_select;

/// Concatenate the text content of all text nodes within `range`.
/// Empty when the range is collapsed or the traversal finds no
/// selectable text.
pub fn serialize_selection(dom: &Dom<TuiExt>, range: &Range) -> String {
    let mut out = String::new();
    let mut state = WalkState::Before;
    let root = dom.root();
    let used = user_select::used_value(dom, root);
    visit(dom, root, used, range, &mut state, &mut out);
    out
}

/// Traversal state. Before reaching `range.start` we skip everything;
/// between start and end we accumulate; once past `range.end` we stop.
enum WalkState {
    Before,
    Inside,
    Done,
}

/// `used` is the used `user-select` of `id` (for a non-element, its
/// parent element's).
fn visit(
    dom: &Dom<TuiExt>,
    id: NodeId,
    used: UserSelect,
    range: &Range,
    state: &mut WalkState,
    out: &mut String,
) {
    if matches!(state, WalkState::Done) {
        return;
    }

    match dom.node(id).node_type() {
        NodeType::Text => {
            if used != UserSelect::None {
                append_text(dom, id, range, state, out);
            } else {
                // Unselectable text still moves the walk past the
                // range's boundaries.
                advance_past(id, range, state);
            }
        }
        _ => {
            // Element / Comment / Fragment: recurse into children.
            let children: Vec<NodeId> = dom.node(id).child_nodes().map(|c| c.id()).collect();
            for child in children {
                let child_used = if dom.node(child).node_type() == NodeType::Element {
                    user_select::resolve_child(dom, child, used)
                } else {
                    used
                };
                visit(dom, child, child_used, range, state, out);
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

/// State transition for an unselectable text node: it contributes no
/// text, but a range boundary inside it still opens or closes the
/// selection (otherwise an end inside `none` chrome would copy on to
/// the end of the document).
fn advance_past(id: NodeId, range: &Range, state: &mut WalkState) {
    if range.end.node == id && (range.start.node == id || matches!(state, WalkState::Inside)) {
        *state = WalkState::Done;
    } else if range.start.node == id {
        *state = WalkState::Inside;
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
