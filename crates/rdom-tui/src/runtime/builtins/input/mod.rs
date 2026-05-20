//! `<input>` (text-family) value model.
//!
//! ## The `value`-attribute model
//!
//! HTML's `<input>` carries its current state in the `value`
//! attribute (technically `defaultValue` per spec, but this v1
//! collapses both into one). Editing infrastructure (Phase B)
//! operates on text nodes — so an `<input>` needs a text-node child
//! that mirrors the `value` attribute.
//!
//! Two integration points keep them in lockstep:
//!
//! - [`seed_all`] — walks the DOM once and gives every `<input>` a
//!   single text-child reflecting its `value` attribute (or empty
//!   string when absent). Called from `App::build` so parsed pages
//!   and direct API users land in the same shape.
//! - [`mirror_to_attribute`] — called after a successful edit on an
//!   `<input>` text child, copies the new text content back into the
//!   `value` attribute so `dom.node(input).get_attribute("value")`
//!   stays accurate.
//!
//! ## Limitations (v1)
//!
//! - Programmatic `set_attribute(input, "value", "x")` AFTER seed
//!   does NOT update the text content. Use [`set_value`] instead,
//!   which writes both the attribute and the text child.
//! - Only the text-family `type` values participate (text, password,
//!   email, url, tel, search, default). `type="checkbox"` etc. land
//!   in C.4b and use a different model.

use rdom_core::NodeId;

use crate::TuiDom;

/// Read the live value of an `<input>`. Reads the text content of
/// the input's first text-node child (i.e., what the editing
/// pipeline has written). Returns `""` when the input has no text
/// child (typically a programmatically-created `<input>` that
/// wasn't routed through [`seed_all`] or [`set_value`]).
pub fn value(dom: &TuiDom, input: NodeId) -> String {
    let mut out = String::new();
    for child in dom.node(input).child_nodes() {
        if child.node_type() == rdom_core::NodeType::Text
            && let Some(s) = child.node_value()
        {
            out.push_str(s);
        }
    }
    out
}

/// Write the value of an `<input>` programmatically. Sets the
/// `value` attribute AND replaces the text-child contents so the
/// editing pipeline + paint pass agree.
///
/// Errors silently when `input` isn't an element node — matches the
/// generally-forgiving builder-chain style used elsewhere in
/// rdom-tui's helper API.
pub fn set_value(dom: &mut TuiDom, input: NodeId, new_value: &str) {
    let _ = dom.set_attribute(input, "value", new_value);
    // Errors discarded at the boundary — see the function-level
    // docstring. The canonical helper (`crate::node::install_text_content`)
    // propagates, callers that want the forgiving style swallow here.
    let _ = crate::node::install_text_content(dom, input, new_value);
}

/// Walk the DOM under `root` and ensure every `<input>` has a
/// single text-node child whose content matches its `value`
/// attribute (or `""` if none). Re-seeding an already-seeded input
/// is idempotent — the text child gets rewritten only when the
/// attribute and text content disagree, or when no text child
/// exists at all (caret needs a Text node to live on).
///
/// Called from `App::build` so parsed templates (`<input value="x">`
/// with no text child) and direct-API users (who may forget to
/// append a text child) both work.
pub fn seed_all(dom: &mut TuiDom) {
    let inputs: Vec<NodeId> = collect_inputs(dom, dom.root());
    for id in inputs {
        // Only text-family inputs participate in the seed: a
        // checkbox / radio / submit button has no editable text
        // surface — its `value` attribute is what gets submitted,
        // not what's displayed (the glyph comes from a UA
        // `::before` content rule).
        if !is_text_family_input(dom, id) {
            continue;
        }
        let want = dom
            .node(id)
            .get_attribute("value")
            .unwrap_or("")
            .to_string();
        let have = value(dom, id);
        let has_text_child = dom
            .node(id)
            .child_nodes()
            .any(|c| c.node_type() == rdom_core::NodeType::Text);
        if !has_text_child || want != have {
            let _ = crate::node::install_text_content(dom, id, &want);
        }
    }

    // Textareas need an editable text child too. Unlike `<input>`,
    // a `<textarea>`'s initial content is its existing text child
    // (no `value` attribute), so we only seed when there isn't one.
    let textareas: Vec<NodeId> = collect_textareas(dom, dom.root());
    for id in textareas {
        let has_text_child = dom
            .node(id)
            .child_nodes()
            .any(|c| c.node_type() == rdom_core::NodeType::Text);
        if !has_text_child {
            let _ = crate::node::install_text_content(dom, id, "");
        }
    }
}

/// Same text-family list as `node::is_text_input_type`. Lives
/// here too because `node` keeps it private. Both must stay in
/// sync — adding a new text-family type means updating both.
fn is_text_family_input(dom: &TuiDom, id: NodeId) -> bool {
    matches!(
        dom.node(id).get_attribute("type"),
        None | Some("text")
            | Some("password")
            | Some("email")
            | Some("url")
            | Some("tel")
            | Some("search")
            | Some("number")
    )
}

/// Mirror the input's current text content into its `value`
/// attribute. Called from `perform_edit` after a successful edit
/// commits, so apps reading `get_attribute("value")` always see
/// the live value.
///
/// `editable` is the editable element id from `perform_edit` (i.e.,
/// the `<input>` itself, since it IS the editable). No-op for any
/// non-`<input>` editable.
pub fn mirror_to_attribute(dom: &mut TuiDom, editable: NodeId) {
    if dom.node(editable).tag_name() != Some("input") {
        return;
    }
    let live = value(dom, editable);
    let _ = dom.set_attribute(editable, "value", &live);
}

// ── Internals ──────────────────────────────────────────────────────

/// Recursively collect every `<input>` element id under `root`
/// (inclusive). Used by `seed_all`.
fn collect_inputs(dom: &TuiDom, root: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk_by_tag(dom, root, "input", &mut out);
    out
}

/// Recursively collect every `<textarea>` element id under `root`
/// (inclusive). Used by `seed_all`.
fn collect_textareas(dom: &TuiDom, root: NodeId) -> Vec<NodeId> {
    let mut out = Vec::new();
    walk_by_tag(dom, root, "textarea", &mut out);
    out
}

fn walk_by_tag(dom: &TuiDom, id: NodeId, tag: &str, out: &mut Vec<NodeId>) {
    if dom.node(id).tag_name() == Some(tag) {
        out.push(id);
    }
    for child in dom.node(id).child_nodes() {
        walk_by_tag(dom, child.id(), tag, out);
    }
}

#[cfg(test)]
mod tests;
