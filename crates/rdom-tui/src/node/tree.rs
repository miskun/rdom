//! Tree queries over `TuiExt` documents: text controls and editable
//! scopes, rendered-ness, child text, text descendants.

use rdom_core::NodeType;

use crate::ext::TuiExt;
use crate::style::content_visibility::{SkippedFor, skips_contents_for};

use super::TuiNodeExt;

/// Whether `id` is a text-family `<input>`: its `type` state
/// (`Dom::input_type_state` — case-insensitive, Text when missing or
/// invalid) is one whose value is a single line of text the user edits
/// (HTML §4.10.5.1: text, search, tel, url, email, password). Toggles,
/// buttons, hidden, range and the unshipped date / color / file types
/// are not.
///
/// `number` is included: it edits as text but with a numeric
/// `beforeinput` filter installed by `runtime::builtins::number`.
///
/// The one text-family predicate: editing, seeding, `set_value`,
/// implicit submission and validation all ask it.
pub(crate) fn is_text_input<Ext>(dom: &rdom_core::Dom<Ext>, id: rdom_core::NodeId) -> bool {
    use rdom_core::InputTypeState as T;
    matches!(
        dom.input_type_state(id),
        Some(T::Text | T::Search | T::Tel | T::Url | T::Email | T::Password | T::Number)
    )
}

/// Whether `id` generates a box as far as `display` goes: neither it nor
/// any ancestor box has a computed `display: none` (CSS Display 3 §2.5 —
/// the element and its descendants generate no boxes), and no ancestor
/// box skips its contents (CSS Containment 2 §4). `display` does not
/// inherit, so the ancestors must be walked; the cascade still computes
/// styles inside such a subtree and layout leaves their rects zeroed.
/// O(depth). Elements never cascaded do not count as `none`.
///
/// The one skip-aware "in the box tree" answer for a single node; a walk
/// down the DOM prunes with [`in_skipped_contents`] instead.
pub(crate) fn is_rendered(dom: &crate::TuiDom, id: rdom_core::NodeId) -> bool {
    rendered_for(dom, id, SkippedFor::Rendering)
}

/// [`is_rendered`] as the user-agent features read it
/// ([`SkippedFor::Features`]): an off-screen `content-visibility: auto`
/// element's skipped contents count — they stay focusable and selectable
/// (CSS Containment 2 §4).
pub(crate) fn is_available(dom: &crate::TuiDom, id: rdom_core::NodeId) -> bool {
    rendered_for(dom, id, SkippedFor::Features)
}

fn rendered_for(dom: &crate::TuiDom, id: rdom_core::NodeId, who: SkippedFor) -> bool {
    let none = |n: rdom_core::NodeId| {
        dom.node(n)
            .computed()
            .is_some_and(|c| c.display == crate::layout::Display::None)
    };
    if none(id) {
        return false;
    }
    // Up the box tree: an ancestor box that is `display: none` or skips
    // its contents (`content-visibility`, CSS Containment 2 §4) — a closed
    // `<details>`'s slot among them.
    let mut cur = crate::render::box_tree::box_parent(dom, id);
    while let Some(n) = cur {
        if none(n) || skips_contents_for(dom, n, who) {
            return false;
        }
        cur = crate::render::box_tree::box_parent(dom, n);
    }
    true
}

/// Whether `child`, a child node of `parent`, is in skipped contents for
/// `who` (CSS Containment 2 §4): `parent` skips its contents, or a box
/// between them does — the `::details-content` slot of a closed
/// `<details>` (HTML §15.5.20), which holds every child but the summary.
/// The prune of a walk down `child_nodes()` that must leave out what the
/// box tree leaves out — Tab order, copy, tree guides — while keeping
/// tree order. O(1): at most the slot lies between.
pub(crate) fn in_skipped_contents(
    dom: &crate::TuiDom,
    parent: rdom_core::NodeId,
    child: rdom_core::NodeId,
    who: SkippedFor,
) -> bool {
    let mut cur = child;
    while let Some(p) = crate::render::box_tree::slot::parent(dom, cur) {
        if skips_contents_for(dom, p, who) {
            return true;
        }
        if p == parent {
            return false;
        }
        cur = p;
    }
    false
}

/// The DOM Standard's "child text content" (§4.2): the concatenated
/// data of `id`'s Text children, in tree order — not of deeper
/// descendants. A `<style>` element's sheet, a `<textarea>`'s value, an
/// `<input>`'s value (its text child) and an `<option>`'s label all read
/// it.
pub(crate) fn child_text<Ext>(dom: &rdom_core::Dom<Ext>, id: rdom_core::NodeId) -> String {
    let mut out = String::new();
    for child in dom.node(id).child_nodes() {
        if child.node_type() == NodeType::Text
            && let Some(s) = child.node_value()
        {
            out.push_str(s);
        }
    }
    out
}

/// Walk up from `node_id` (inclusive) to the nearest editing host or
/// text control ([`TuiNodeExt::is_editable`]). Returns its id, or
/// `None` when `node_id` is not editable: nothing on the way up is
/// editable, or the walk first meets a `contenteditable` element in
/// the false state — a non-editable island inside an editing host
/// (HTML §6.8.1: the false state stops inheritance).
/// Used by runtime paths that need to route an edit or caret action
/// to the enclosing editable scope.
pub fn nearest_editable_ancestor(
    dom: &rdom_core::Dom<TuiExt>,
    node_id: rdom_core::NodeId,
) -> Option<rdom_core::NodeId> {
    let mut cur = Some(node_id);
    while let Some(id) = cur {
        if dom.node(id).is_editable() {
            return Some(id);
        }
        if dom.content_editable_state(id) == Some(rdom_core::ContentEditableState::False) {
            return None;
        }
        cur = dom.node(id).parent_node().map(|p| p.id());
    }
    None
}

/// True when `node` is a descendant of `ancestor`, or `node ==
/// ancestor`. Used by user-select / focus / selection clamp logic
/// to decide whether a candidate target is inside a host's subtree.
pub fn is_descendant_or_self(
    dom: &rdom_core::Dom<TuiExt>,
    node: rdom_core::NodeId,
    ancestor: rdom_core::NodeId,
) -> bool {
    let mut cur = Some(node);
    while let Some(n) = cur {
        if n == ancestor {
            return true;
        }
        cur = dom.node(n).parent_node().map(|p| p.id());
    }
    false
}

/// First text-node descendant of `root` (inclusive) in document
/// order. Returns `None` when no text node exists in the subtree.
/// Used by selection-extension logic that needs the host's
/// leading text position.
pub fn first_text_descendant(
    dom: &rdom_core::Dom<TuiExt>,
    root: rdom_core::NodeId,
) -> Option<rdom_core::NodeId> {
    // Iterative: a DOM may be any depth (C16G-DEPTH-CAPS).
    std::iter::once(root)
        .chain(dom.descendants(root))
        .find(|&n| dom.node(n).node_type() == NodeType::Text)
}

/// Last text-node descendant of `root` (inclusive) in document
/// order. Returns `None` when no text node exists in the subtree.
/// Used by selection-extension logic that needs the host's
/// trailing text position.
pub fn last_text_descendant(
    dom: &rdom_core::Dom<TuiExt>,
    root: rdom_core::NodeId,
) -> Option<rdom_core::NodeId> {
    // Iterative: a DOM may be any depth (C16G-DEPTH-CAPS).
    std::iter::once(root)
        .chain(dom.descendants(root))
        .filter(|&n| dom.node(n).node_type() == NodeType::Text)
        .last()
}

/// Byte length of a text node's data. Returns `0` for non-text
/// nodes (use as a position-clamp helper, not a node-type check).
pub fn text_len(dom: &rdom_core::Dom<TuiExt>, id: rdom_core::NodeId) -> usize {
    dom.node(id).node_value().map(|s| s.len()).unwrap_or(0)
}

/// Replace all children of `id` with a single text node holding
/// `text`. The canonical "set this element's text content" mutation
/// — used by `<input>` / `<textarea>` value seeding + the
/// `set_value` / form-control setters in `TuiAccessorsMut`. Lives
/// here (substrate-neutral) so the value-installation logic lives
/// in one place rather than being re-implemented per call site.
///
/// Returns `Err` if any of the underlying `remove_child` /
/// `append_child` calls fail. Callers that prefer the
/// generally-forgiving builder-chain style (e.g. the public
/// `runtime::builtins::input::set_value`) ignore the result with
/// `let _ = …` at the call.
pub(crate) fn install_text_content(
    dom: &mut rdom_core::Dom<TuiExt>,
    id: rdom_core::NodeId,
    text: &str,
) -> crate::Result<()> {
    let existing: Vec<rdom_core::NodeId> = dom.node(id).child_nodes().map(|c| c.id()).collect();
    for child in existing {
        dom.remove_child(id, child)?;
    }
    let text_node = dom.create_text_node(text);
    dom.append_child(id, text_node)?;
    Ok(())
}
